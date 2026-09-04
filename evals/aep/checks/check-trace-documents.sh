#!/usr/bin/env bash
# task:agent-eval-trace-documents — T1 … T8.
#
# Checked with `aep observe trace check` against purpose-made transcripts in `transcripts/`: no API
# call, and no dependency on `run-agents.sh` existing. Those transcripts are this check's inputs,
# hand-written here in `establish_verifiers`; the *committed* fixtures under `eval/fixtures/` are a
# different thing, produced by the live run, and nothing in this file reads them.
#
# The two documents are **not** in this repository and never were: they are AEP's own eval
# corpus, `conformance/eval/<case>/expectations.trace.yaml`, resolved by `charter_spec` in `lib.sh`.
# That is the one source of truth — the subject's gate replays them, this eval's gate never could.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

DEC="$(charter_spec decomposer)"
REV="$(charter_spec plan-reviewer)"

declare_row T1 "both documents are accepted by aep observe trace check, exit 0, one row per expectation"
declare_row T2 "every R12 row is present in the right document, by id, and gating — and the document names no gating row the contract omits"
declare_row T3 "a decomposer transcript carrying \`artifact move\` turns the decomposer's move absence red"
declare_row T4 "a reviewer transcript carrying \`artifact new\` turns the reviewer's create absence red"
declare_row T5 "against a transcript with no tool calls, each document reports at least one red row"
declare_row T6 "the plan-reviewer document contains no tool.absent over Write or Edit"
declare_row T7 "every tool.absent is paired with a tool.called over the same tool in the same document"
declare_row T8 "neither document's expectations reference the agent's own charter file under agents/"

have aep || { red_all "the \`aep\` CLI is not on PATH"; finish; exit; }
REASON="$(charter_specs_missing)" && { red_all "$REASON"; finish; exit; }

WORK="$(scratch)"
check_against() { # check_against <doc> <transcript> -> report on stdout, exit status preserved
  aep observe trace check --spec "$1" --transcript "$2" 2>&1
}

# ---- T1 -----------------------------------------------------------------------------------------
# A document that parses, evaluated against a transcript that holds its bounds. The row count is
# part of the claim: a report with fewer rows than the document has expectations is a document
# whose tail nobody evaluated.
R=0
for pair in "decomposer:$DEC" "plan-reviewer:$REV"; do
  case_name="${pair%%:*}"; doc="${pair#*:}"
  clean="$TRANSCRIPTS/$case_name-clean.jsonl"
  if [ ! -f "$clean" ]; then
    R=1; why "no input transcript at ${clean#"$REPO"/}"; continue
  fi
  out="$WORK/t1-$case_name.txt"
  check_against "$doc" "$clean" > "$out"; status=$?
  declared="$(grep -c '^[[:space:]]*-[[:space:]]*id:' "$doc")"
  rows="$(trace_rows "$out")"
  [ "$status" -eq 0 ] || { R=1; why "$case_name: exit $status against its clean transcript"; }
  [ "$rows" -eq "$declared" ] \
    || { R=1; why "$case_name: $rows verdict row(s) for $declared declared expectation(s)"; }
  [ "$status" -eq 0 ] || grep -E '^[[:space:]]*(gap|unk) ' "$out" | head -4 | while IFS= read -r l; do
    why "  $l"
  done
done
row T1 "$R"

# ---- T2 -----------------------------------------------------------------------------------------
# Read from `contracts/trace-expectations.txt`, which is where the ids R12 does not give are fixed.
# Three claims per row: the id exists, the kind and matcher under it are the ones R12 names, and the
# expectation is gating — because an advisory charter bound is a bound that never fails.
R=0
while IFS=$'\t' read -r stage id kind tool contains severity; do
  doc="$(charter_spec "$stage")" || { R=1; why "unknown stage \`$stage\` in trace-expectations.txt"; continue; }
  # The expectation's own block: from its `- id:` line to the next one.
  block="$(awk -v id="$id" '
    $0 ~ "^[[:space:]]*-[[:space:]]*id:[[:space:]]*"id"[[:space:]]*$" { inside = 1; print; next }
    inside && /^[[:space:]]*-[[:space:]]*id:/ { inside = 0 }
    inside { print }' "$doc")"
  if [ -z "$block" ]; then
    R=1; why "$stage declares no expectation with id \`$id\`"; continue
  fi
  grep -q "$kind:" <<< "$block" || { R=1; why "$stage/$id is not a \`$kind\` expectation"; }
  [ "$tool" = "-" ] || grep -Eq "tool:[[:space:]]*$tool([[:space:]]|$)" <<< "$block" \
    || { R=1; why "$stage/$id does not name tool \`$tool\`"; }
  [ "$contains" = "-" ] || grep -qF "$contains" <<< "$block" \
    || { R=1; why "$stage/$id does not carry the matcher \`$contains\`"; }
  if [ "$severity" = "gate" ] && grep -q 'severity:[[:space:]]*advisory' <<< "$block"; then
    R=1; why "$stage/$id is advisory; R12 requires it to gate"
  fi
done < <(contract_lines trace-expectations.txt)

# The other direction, and the half that was not being checked. `contracts/trace-expectations.txt`
# is hand-maintained, and a hand-maintained list that only ever gets read one way goes stale in
# silence: the loop above is green when the contract names a subset of the document, so a gating
# bound the subject *adds* is a bound nothing in this repository reads. Enumerated from the document
# itself, so the list cannot fall behind without a red row. Advisory rows are excluded on purpose —
# the contract records gates, and R12 asks for gates.
for stage in decomposer plan-reviewer; do
  doc="$(charter_spec "$stage")"
  NAMED="$(contract_lines trace-expectations.txt | awk -F'\t' -v s="$stage" '$1 == s { print $2 }')"
  while IFS= read -r id; do
    [ -z "$id" ] && continue
    grep -qxF -- "$id" <<< "$NAMED" \
      || { R=1; why "$stage/$id gates in ${doc##*/} and contracts/trace-expectations.txt does not name it"; }
  done < <(gating_ids "$doc")
done
row T2 "$R"

# ---- T3, T4 -------------------------------------------------------------------------------------
# A named row, red, against a transcript that violates exactly that bound. Not "the run failed":
# a document can fail for a reason that has nothing to do with the row under test.
named_row_red() { # named_row_red <doc> <transcript> <expectation-id>
  local out="$WORK/named-$3.txt"
  check_against "$1" "$2" > "$out"
  local verdict; verdict="$(trace_verdict "$out" "$3")"
  case "$verdict" in
    gap) return 0 ;;
    ok)  why "$3 stayed green against ${2##*/} — the bound does not discriminate"; return 1 ;;
    unk) why "$3 was undecidable against ${2##*/} — the adapter could not read the event"; return 1 ;;
    *)   why "$3 produced no row at all against ${2##*/}"; return 1 ;;
  esac
}
# The id is looked up by the bound it carries, never written here as a literal: an id spelled in a
# check and again in the contract drifts the moment the document renames one, and T2 already holds
# the contract against the document.
T="$TRANSCRIPTS/decomposer-ran-a-move.jsonl"
ID="$(contract_expectation_id decomposer tool.absent move)"
if [ -z "$ID" ]; then
  why "contracts/trace-expectations.txt names no decomposer tool.absent bound over \`artifact move\`"; row T3 1
elif [ -f "$T" ]; then named_row_red "$DEC" "$T" "$ID"; row T3 $?
else why "no input transcript at ${T#"$REPO"/}"; row T3 1; fi

T="$TRANSCRIPTS/plan-reviewer-created-an-artifact.jsonl"
ID="$(contract_expectation_id plan-reviewer tool.absent new)"
if [ -z "$ID" ]; then
  why "contracts/trace-expectations.txt names no plan-reviewer tool.absent bound over \`artifact new\`"; row T4 1
elif [ -f "$T" ]; then named_row_red "$REV" "$T" "$ID"; row T4 $?
else why "no input transcript at ${T#"$REPO"/}"; row T4 1; fi

# ---- T5 -----------------------------------------------------------------------------------------
# The positive control, fired. This is the row that separates "the agent stayed inside its charter"
# from "the harness surfaced none of the agent's calls" — and R13 says the second must fail loudly
# rather than report a green wall of vacuous absences. Shown per document, as T5 requires.
R=0
EMPTY="$TRANSCRIPTS/no-tool-calls.jsonl"
if [ ! -f "$EMPTY" ]; then
  R=1; why "no input transcript at ${EMPTY#"$REPO"/}"
else
  for pair in "decomposer:$DEC" "plan-reviewer:$REV"; do
    case_name="${pair%%:*}"; doc="${pair#*:}"
    out="$WORK/t5-$case_name.txt"
    check_against "$doc" "$EMPTY" > "$out"
    reds="$(grep -cE '^[[:space:]]*(gap|unk) ' "$out")"
    if [ "$reds" -eq 0 ]; then
      R=1; why "$case_name: every row green against a transcript with no tool calls at all"
    else
      why "$case_name: $reds red row(s) — the control fires"
    fi
  done
fi
row T5 "$R"

# ---- T6 -----------------------------------------------------------------------------------------
# R14. The reviewer's grant is `[Read, Grep, Glob, Bash]`, so an absence over `Write` or `Edit` is
# true of every possible run — indistinguishable from a check that was switched off.
R=0
OFFEND="$(awk '
  /tool\.absent:/ { inside = 1; block = ""; next }
  inside && /^[[:space:]]*-[[:space:]]*id:/ { inside = 0 }
  inside { if ($0 ~ /tool:[[:space:]]*(Write|Edit)[[:space:]]*$/) print NR": "$0 }' "$REV")"
[ -z "$OFFEND" ] || { R=1; why "${REV#"$REPO"/}: $OFFEND"; }
row T6 "$R"

# ---- T7 -----------------------------------------------------------------------------------------
# R13, enumerated pair by pair, one line each. Read the pairing rule before reading a red row here:
# it is literal, and `contracts/interface.md` does not soften it.
R=0
for pair in "decomposer:$DEC" "plan-reviewer:$REV"; do
  case_name="${pair%%:*}"; doc="${pair#*:}"
  ABSENT="$(awk '/tool\.absent:/ { inside = 1; next }
    inside && /^[[:space:]]*-[[:space:]]*id:/ { inside = 0 }
    inside && /tool:/ { sub(/^.*tool:[[:space:]]*/, ""); print $1; inside = 0 }' "$doc" | sort -u)"
  CALLED="$(awk '/tool\.called:/ { inside = 1; next }
    inside && /^[[:space:]]*-[[:space:]]*id:/ { inside = 0 }
    inside && /tool:/ { sub(/^.*tool:[[:space:]]*/, ""); print $1; inside = 0 }' "$doc" | sort -u)"
  [ -z "$ABSENT" ] && { R=1; why "$case_name: no tool.absent bound at all — R12 requires at least one"; }
  while IFS= read -r t; do
    [ -z "$t" ] && continue
    if grep -qx "$t" <<< "$CALLED"; then
      why "$case_name: absent($t) ↔ called($t)"
    else
      R=1
      why "$case_name: absent($t) has no tool.called over $t — the bound is green on a transcript carrying none of the agent's calls"
    fi
  done <<< "$ABSENT"
done
row T7 "$R"

# ---- T8 -----------------------------------------------------------------------------------------
# The specification's first invariant. A document that cites the charter file asserts the sentence
# is still written, which is the failure mode the story exists to remove.
#
# Read over the document's **assertions**, not its prose: `yaml_body` drops `#` comments first,
# because the canonical decomposer case opens by stating this very invariant in words ("No row here
# reads `agents/decomposer.md`") and a grep over the raw bytes reads the disclaimer as the breach.
# The path is matched wherever it now lives — `integrations/claude-code/agents/` before the plugins
# moved to agentplugins, `plugins/aep-plan/agents/` after — because what the invariant forbids is
# reading the agent's own markdown, not one spelling of where it sits.
R=0
for pair in "decomposer:$DEC" "plan-reviewer:$REV"; do
  case_name="${pair%%:*}"; doc="${pair#*:}"
  HIT="$(yaml_body "$doc" | grep -n 'agents/')"
  [ -z "$HIT" ] || { R=1; while IFS= read -r l; do why "$case_name: $l"; done <<< "$HIT"; }
done
row T8 "$R"

finish

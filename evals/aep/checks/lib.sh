#!/usr/bin/env bash
# Shared vocabulary for the W4.1 verifiers.
#
# Sourced by every `check-*.sh`. It carries no assertion of its own — what it carries is the two
# rules the specification states as invariants, made mechanical so no individual check can forget
# them:
#
#   * **A vacuous check is a failed check.** Every id a check declares must be reported. `finish`
#     fails the check if one was not, so a row that fell out of a branch is a red row and not an
#     absent one.
#   * **The verdict table prints on every path, including failure.** Nothing here sets `-e`, and no
#     helper exits early. A check that dies before its rows print is indistinguishable from a check
#     that had nothing to say.
#
# Deliberately *not* `set -e`: an assertion that aborts the script takes the report with it.
set -uo pipefail

CHECKS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EVAL_DIR="$(cd "$CHECKS_DIR/.." && pwd)"
# Migrated: this eval lives in the metaharness repository; the subject checkout comes from AEP_REPO.
REPO="${AEP_REPO:-$HOME/beyond10x/aep}"
AGENTPLUGINS_REPO="${AGENTPLUGINS_REPO:-$HOME/beyond10x/agentplugins}"
PLUGIN_DIR="$AGENTPLUGINS_REPO/plugins/aep-plan"
CONTRACTS="$CHECKS_DIR/contracts"
TRANSCRIPTS="$CHECKS_DIR/transcripts"
RUNNER="$EVAL_DIR/run-agents.sh"
FIXTURE_SRC="$REPO/examples/planning-passkeys"

# ---- the two charter expectation documents ------------------------------------------------------
# **One source of truth, and it is not here.** The decomposer and plan-reviewer charter cases are
# AEP's own eval corpus: `conformance/eval/<case>/expectations.trace.yaml`, replayed by
# `crates/edge/aep-cli/tests/eval_corpus.rs` in *that* repository's `task check`. A copy under
# `evals/aep/` would be a second copy nothing gates — nothing under `evals/` runs in this
# repository's gate (AGENTS.md invariant 5) — and that is exactly how `contracts/trace-expectations.txt`
# came to name ids and a CLI spelling no document carries. So these checks read the canonical files.
#
#   AEP_REPO                       the subject checkout (the default for everything else here too)
#   EVAL_CHARTER_SPEC_DIR          the corpus directory inside it, if the corpus ever moves
#   EVAL_CHARTER_SPEC_DECOMPOSER   one document, by path, overriding both of the above
#   EVAL_CHARTER_SPEC_REVIEWER     the same, for the plan-reviewer
#
# Deliberately *not* `EVAL_SPEC_DECOMPOSER`/`EVAL_SPEC_REVIEWER`: those are the runner's own
# per-stage override (`contracts/interface.md`), and `check-runner-verdict.sh` R4 sets them to an
# emptied document on purpose. Sharing the name would hand that emptied file to every other row.
CHARTER_SPEC_DIR="${EVAL_CHARTER_SPEC_DIR:-$REPO/conformance/eval}"
CHARTER_SPEC_DECOMPOSER="${EVAL_CHARTER_SPEC_DECOMPOSER:-$CHARTER_SPEC_DIR/decomposer-charter/expectations.trace.yaml}"
CHARTER_SPEC_REVIEWER="${EVAL_CHARTER_SPEC_REVIEWER:-$CHARTER_SPEC_DIR/plan-reviewer-charter/expectations.trace.yaml}"

# charter_spec <decomposer|plan-reviewer>  — the document's path. The stage names are the keys
# `contracts/trace-expectations.txt` uses, so a contract row and a document resolve through one name.
charter_spec() {
  case "$1" in
    decomposer)    printf '%s' "$CHARTER_SPEC_DECOMPOSER" ;;
    plan-reviewer) printf '%s' "$CHARTER_SPEC_REVIEWER" ;;
    *)             return 1 ;;
  esac
}

# charter_specs_missing  — prints why the documents cannot be read and returns 0, or returns 1.
#
# The reason is the whole value of this helper: "missing document" sends a reader to `evals/aep/`,
# where the file has never been, instead of to the checkout that owns it.
charter_specs_missing() {
  local stage path missing=""
  for stage in decomposer plan-reviewer; do
    path="$(charter_spec "$stage")"
    [ -f "$path" ] || missing="$missing $path"
  done
  [ -z "$missing" ] && return 1
  printf 'the canonical charter document(s) are not readable:%s' "$missing"
  if [ -z "${AEP_REPO:-}" ] && [ -z "${EVAL_CHARTER_SPEC_DIR:-}" ]; then
    printf ' — AEP_REPO is unset, so the default %s was used' "$HOME/beyond10x/aep"
  fi
  printf '. They are AEP'"'"'s own eval corpus (conformance/eval/<case>/expectations.trace.yaml, 0.51.0 or later); set AEP_REPO to a checkout that carries it, or name each file with EVAL_CHARTER_SPEC_DECOMPOSER / EVAL_CHARTER_SPEC_REVIEWER.'
  return 0
}

# ---- rows ---------------------------------------------------------------------------------------
# A check declares its ids and their statements up front, then reports each one exactly once.

declare -A STATEMENT=()
declare -A REPORTED=()
ROW_IDS=()
FAILED=0

# declare_row <id> <statement>
declare_row() {
  STATEMENT["$1"]="$2"
  ROW_IDS+=("$1")
}

# row <id> <exit-status>   — 0 is a pass, anything else is a failure.
row() {
  local id="$1" code="$2"
  if [ -n "${REPORTED[$id]:-}" ]; then
    printf 'FAIL  %-4s reported twice — the check is confused about its own rows\n' "$id"
    FAILED=$((FAILED + 1))
    return
  fi
  REPORTED["$id"]=1
  if [ "$code" -eq 0 ]; then
    printf 'PASS  %-4s %s\n' "$id" "${STATEMENT[$id]:-<undeclared row>}"
  else
    printf 'FAIL  %-4s %s\n' "$id" "${STATEMENT[$id]:-<undeclared row>}"
    FAILED=$((FAILED + 1))
  fi
}

# why <text…>  — the reason under the row it belongs to. Printed, never counted.
why() { printf '        ↳ %s\n' "$*"; }

# red_all <reason>  — every not-yet-reported row goes red for one shared reason.
#
# This is what a missing deliverable looks like. It is emphatically not a skip: the rows are in the
# table, they are red, and the reason is under them. A check that quietly reported nothing when its
# subject did not exist would go green in `run-checks.sh` for having no failures.
red_all() {
  local reason="$1" id
  for id in "${ROW_IDS[@]}"; do
    [ -n "${REPORTED[$id]:-}" ] && continue
    row "$id" 1
    why "$reason"
  done
}

# finish  — the check's exit status, and the last enforcement of the no-silent-row rule.
finish() {
  local id missing=0
  for id in "${ROW_IDS[@]}"; do
    if [ -z "${REPORTED[$id]:-}" ]; then
      printf 'FAIL  %-4s never reported — a row that did not run is not a row that passed\n' "$id"
      missing=$((missing + 1))
    fi
  done
  [ "$((FAILED + missing))" -eq 0 ]
}

# ---- preconditions ------------------------------------------------------------------------------

# have <command>  — is the tool on PATH.
have() { command -v "$1" >/dev/null 2>&1; }

# runner_present  — the subject of most of these checks.
runner_present() { [ -f "$RUNNER" ]; }

# runner <args…>  — invoke it through `bash`, so a missing execute bit is not a false red.
runner() { bash "$RUNNER" "$@"; }

# ---- scratch ------------------------------------------------------------------------------------
# Never `/tmp`: this machine's tmpfs drops writes under pressure. Same rule the two sibling evals
# follow, and the same fallback.

scratch() {
  local base="${TMPDIR:-$HOME/.cache/claude-tmp}"
  mkdir -p "$base" || return 1
  mktemp -d "$base/agent-eval-check.XXXXXX"
}

# under_allowed_base <path>  — is it under $TMPDIR or the documented fallback (F1's other half).
under_allowed_base() {
  local path="$1" base="${TMPDIR:-}" fallback="$HOME/.cache/claude-tmp"
  case "$path" in
    /tmp/*) return 1 ;;
  esac
  [ -n "$base" ] && case "$path" in "$base"/*) return 0 ;; esac
  case "$path" in "$fallback"/*) return 0 ;; esac
  return 1
}

# ---- contracts ----------------------------------------------------------------------------------

# contract_lines <file>  — the file's meaningful lines: no comments, no blanks.
contract_lines() {
  grep -v '^[[:space:]]*#' "$CONTRACTS/$1" 2>/dev/null | grep -v '^[[:space:]]*$'
}

# pre_task_blob <revision> <path>  — the file's bytes before W4.1 touched it.
pre_task_blob() { git -C "$REPO" cat-file blob "$1:$2" 2>/dev/null; }

# contract_expectation_id <stage> <kind> <verb>  — the id of the contract row whose matcher ends in
# <verb>. A check that wants "the bound over `artifact move`" asks for it by what it bounds; the id
# is spelled once, in `contracts/trace-expectations.txt`, and nowhere else. An id written into a
# check as a literal is an id that drifts silently when the document renames it, which is the defect
# this whole story is an instance of.
contract_expectation_id() {
  contract_lines trace-expectations.txt \
    | awk -F'\t' -v s="$1" -v k="$2" -v v="$3" '$1 == s && $3 == k && $5 ~ (v "$") { print $2; exit }'
}

# gating_ids <document>  — every expectation id in a `trace-spec/1` document that is **not**
# advisory. The reverse of what the contract file lists, and the half nobody was checking: a gating
# bound the subject adds and this repository never names is a bound no check here reads.
gating_ids() {
  awk '
    /^[[:space:]]*-[[:space:]]*id:[[:space:]]*/ {
      if (id != "" && !adv) print id
      id = $0
      sub(/^[[:space:]]*-[[:space:]]*id:[[:space:]]*/, "", id)
      sub(/[[:space:]]*$/, "", id)
      adv = 0
      next
    }
    id != "" && /severity:[[:space:]]*advisory/ { adv = 1 }
    END { if (id != "" && !adv) print id }
  ' "$1" 2>/dev/null
}

# yaml_body <document>  — the document with `#` comment text removed.
#
# A row that asks whether a document *references* something must read what the document asserts, not
# what its prose says about itself: the canonical decomposer case opens with the sentence "No row
# here reads `agents/decomposer.md`", and a grep over the raw file reads that disclaimer as the
# violation it disclaims.
yaml_body() { sed 's/[[:space:]]*#.*$//' "$1" 2>/dev/null; }

# ---- reading a verdict table --------------------------------------------------------------------
# The runner's table, by row id. `interface.md` fixes the two accepted verdict words per shape; a
# row this cannot parse is *absent*, and an absent row is a failure at every call site below.

# table_verdict <file> <id>  — prints `pass`, `fail`, `note` or nothing at all.
table_verdict() {
  awk -v want="$2" '
    { verdict = $1; id = $2 }
    id != want { next }
    verdict == "PASS" || verdict == "ok"  { print "pass"; found = 1; exit }
    verdict == "FAIL" || verdict == "gap" || verdict == "unk" { print "fail"; found = 1; exit }
    verdict == "note" { print "note"; found = 1; exit }
  ' "$1" 2>/dev/null
}

# table_has_row <file> <id>
table_has_row() { [ -n "$(table_verdict "$1" "$2")" ]; }

# ---- reading an `aep observe trace check` report ------------------------------------------------
# `report_to_text` writes `  <status> <id>  <statement>`; the status is `ok`, `gap` or `unk`, with
# ` (adv)` appended for an advisory row.

# trace_verdict <report-file> <expectation-id>  — prints `ok`, `gap`, `unk` or nothing.
trace_verdict() {
  awk -v want="$2" '
    { status = $1; rest = $2 }
    status != "ok" && status != "gap" && status != "unk" { next }
    rest == "(adv)" { rest = $3 }
    rest == want { print status; exit }
  ' "$1" 2>/dev/null
}

# trace_rows <report-file>  — how many verdict rows the report carried. Zero rows is R15's failure.
trace_rows() {
  awk '$1 == "ok" || $1 == "gap" || $1 == "unk" { n++ } END { print n + 0 }' "$1" 2>/dev/null
}

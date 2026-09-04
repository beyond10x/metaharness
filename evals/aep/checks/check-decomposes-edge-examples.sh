#!/usr/bin/env bash
# task:decomposes-edge-examples — E1 … E4.
#
# The one task in the set with no dependency on anything else, and the only one whose subject
# already exists — which is why E1–E3 can be red for a real reason today rather than for absence.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

DECOMPOSER="$PLUGIN_DIR/agents/decomposer.md"
SKILL="$PLUGIN_DIR/skills/planning/SKILL.md"

declare_row E1 "neither file contains derived_from:epic: in an `artifact new` example"
declare_row E2 "both files contain decomposes:epic: in the example that previously read derived_from"
declare_row E3 "the diff against the pre-task revision is the relation token and nothing else"
declare_row E4 "the corrected command creates a story carrying decomposes: epic:…, and validate exits 0"

have git || { red_all "\`git\` is not on PATH"; finish; exit; }
for f in "$DECOMPOSER" "$SKILL"; do
  [ -f "$f" ] || { red_all "$f does not exist"; finish; exit; }
done

# story_example_epic_edges <file…>  — the relation of every `--relate <rel>:epic:` token taught
# inside an `artifact new … story` example, one per line, deduplicated.
#
# **Scoped to that example on purpose.** The task these rows decide is about the edge a *story*
# takes from its epic. A grep over the whole file answers a different question — "does any epic edge
# appear anywhere" — and agentplugins 0.7.0 legitimately teaches a second one:
# `agents/decomposer.md:93` files a `decision-blocker` with `--relate blocks:epic:…`, which is a
# blocker stopping an area of an epic and is exactly what the plugin should teach. A check that
# reddened on it would be reporting the file's vocabulary, not the story example's edge.
#
# An invocation runs from the line naming it to the first line that does not end in a continuation
# backslash, which is how every example in both files is written.
story_example_epic_edges() {
  awk '
    # recorded-under-this-name: this pattern is read against the `agentplugins` checkout,
    # whose examples still spell the first level flat. It follows that file, not this one.
    /aep artifact new[[:space:]]+story([[:space:]]|$)/ { in_story = 1 }
    in_story && match($0, /--relate[[:space:]]+[a-z_]+:epic:/) {
      tok = substr($0, RSTART, RLENGTH)
      sub(/^--relate[[:space:]]+/, "", tok)
      sub(/:epic:$/, "", tok)
      print tok
    }
    in_story && !/\\[[:space:]]*$/ { in_story = 0 }
  ' "$@" | sort -u
}

# ---- E1 -----------------------------------------------------------------------------------------
# Scoped to an `artifact new` example, not to the whole file: `derived_from` is a legitimate
# relation, and a rule that forbade the word would forbid the vocabulary.
R=0
for f in "$DECOMPOSER" "$SKILL"; do
  HIT="$(grep -n -- '--relate derived_from:epic:' "$f")"
  # Relative to the plugin, not to `$REPO`: these two files moved to the `agentplugins` checkout.
  [ -z "$HIT" ] || { R=1; why "${f#"$PLUGIN_DIR"/}: $HIT"; }
done
row E1 "$R"

# ---- E2 -----------------------------------------------------------------------------------------
# Read through the same extractor E4 uses, so both rows are claims about the same text: the token
# has to be in the `artifact new … story` example, not merely somewhere in the file.
R=0
for f in "$DECOMPOSER" "$SKILL"; do
  EDGES="$(story_example_epic_edges "$f")"
  if [ -z "$EDGES" ]; then
    R=1; why "${f#"$PLUGIN_DIR"/} teaches no epic edge in an \`artifact new … story\` example"
  elif ! grep -qx 'decomposes' <<< "$EDGES"; then
    R=1; why "${f#"$PLUGIN_DIR"/}: the new-story example takes $(tr '\n' ' ' <<< "$EDGES")from its epic, not \`decomposes\`"
  fi
done
row E2 "$R"

# ---- E3 -----------------------------------------------------------------------------------------
# "only the relation token on those lines — no surrounding prose is rewritten", made exact: undo
# the substitution on the current file and it must be byte-identical to the pre-task blob. Any other
# edit anywhere in either file survives the undo and shows up here.
R=0
while IFS=$'\t' read -r mode rev path; do
  [ "$mode" = "token-only" ] || continue
  BEFORE="$(pre_task_blob "$rev" "$path")"
  if [ -z "$BEFORE" ]; then
    R=1; why "cannot read $path at $rev — the pinned pre-task revision is unreachable"
    continue
  fi
  UNDONE="$(sed 's/decomposes:epic:/derived_from:epic:/g' "$REPO/$path")"
  if [ "$UNDONE" != "$BEFORE" ]; then
    R=1
    why "$path differs from $rev by more than the relation token:"
    diff <(printf '%s\n' "$BEFORE") <(printf '%s\n' "$UNDONE") | head -8 | while IFS= read -r l; do
      why "  $l"
    done
  fi
done < <(contract_lines pre-task-blobs.txt)
row E3 "$R"

# ---- E4 -----------------------------------------------------------------------------------------
# The token the file teaches, run for real against a scratch store. Not the whole example line: the
# example names `epic:passkey-login`, which is not the epic the passkeys fixture actually carries,
# and a check that failed on that would be reporting the example's slug rather than its edge.
R=0
if ! have aep; then
  R=1; why "the \`aep\` CLI is not on PATH"
else
  WORK="$(scratch)/e4"
  mkdir -p "$WORK"
  cp -R "$FIXTURE_SRC/." "$WORK/"
  mkdir -p "$WORK/artifacts"
  cp -R "$REPO/artifacts/lifecycles" "$WORK/artifacts/lifecycles"
  cp -R "$REPO/artifacts/templates" "$WORK/artifacts/templates"
  STORE="$WORK/.engineering/planning"

  # The edge the `artifact new … story` example teaches — and only that example. What this row
  # decides is that a story takes `decomposes` from its epic, not that no other `*:epic:` token
  # exists anywhere in the two files; a `blocks:epic:` on a decision-blocker is a different, correct
  # lesson and reddening on it would be the check judging the wrong sentence.
  EDGES="$(story_example_epic_edges "$DECOMPOSER" "$SKILL")"
  if [ -z "$EDGES" ]; then
    R=1; why "neither file teaches an epic edge inside an \`artifact new … story\` example"
  elif [ "$EDGES" != "decomposes" ]; then
    R=1; why "the new-story example takes $(tr '\n' ' ' <<< "$EDGES")from its epic, not \`decomposes\` alone"
  else
    REL="$EDGES"
    OUT="$(cd "$WORK" && aep plan artifact new story e4-probe --store "$STORE" \
      --title "E4 probe" --relate "$REL:epic:passkey-sign-in" 2>&1)" || {
      R=1; why "the taught command was refused: $OUT"
    }
    FILE="$STORE/story/e4-probe.md"
    if [ -f "$FILE" ]; then
      grep -Eq '^[[:space:]]*-[[:space:]]*decomposes:[[:space:]]*epic:passkey-sign-in' "$FILE" \
        || { R=1; why "the created story carries no \`decomposes: epic:passkey-sign-in\` edge"; }
    else
      R=1; why "no story was created at $FILE"
    fi
    VOUT="$(cd "$WORK" && aep plan artifact validate --store "$STORE" 2>&1)" \
      || { R=1; why "validate exited non-zero after the taught command: $VOUT"; }
  fi
fi
row E4 "$R"

finish

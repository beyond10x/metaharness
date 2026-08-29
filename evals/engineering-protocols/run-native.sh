#!/usr/bin/env bash
# The **native walk** of the same work `run-driven.sh` drives: `b10x-harness workflow run` walks the
# flow `protocol workflow flow` projects from `adp/default/2` with the eval's own step map, and
# `protocol drive transition` — the engine as a program — governs every section boundary through the
# loop's `transition` hook (atlas ADR 0004; engineering-protocols `story:drive-transition-verb`).
#
# It is the other side of the comparison `harness/README.md` § "What stays outside: the governor"
# describes: a native walk moves the sequencer, so it is a *different experiment* from the driven
# arm, and where it is measured against that arm it is measured as tokens, turns, wall-time and —
# with a rate card — cost, under the **same** governor program. It is not a conformance claim.
#
# ## What one run does, end to end
#
#   1. checks the binaries: `protocol` on PATH must know `drive transition`; `b10x-harness` must be
#      newer than the harness repository's newest commit (the same refusal `run-driven.sh` makes);
#   2. assembles the same hermetic scratch project `run-driven.sh` builds: a copy of the subject's
#      document tree, an EMPTY planning store under `ws_project/`, the task, the plugin;
#   3. projects the flow — `protocol workflow flow --id adp/default --map driven.steps.yaml` — and
#      writes a hooks file declaring the governor at `transition` and store integrity at
#      `before-call`;
#   4. `b10x-harness workflow plan` — the shape, free of charge. **This is where a run without
#      `--spend` stops**, printing the one command that would spend;
#   5. with `--spend`: re-execs under `systemd-run --user --scope` (substrate's cgroup probe needs
#      the calling process inside the root), walks the flow with `--json` into the scratch
#      directory, keeps its sessions there, and prints a census: sections entered and left, every
#      `transition-refused` with the engine's reason, hook decisions, steps ran/failed/skipped,
#      retreats, and the sessions with what they spent.
#
# This spends real money on a real model. It is not part of `task check` and must never be.
#
# Environment (defaults match `run-driven.sh`'s b10x arm, so the two walks are comparable):
#   EP_REPO                 the engineering-protocols checkout (default: ~/beyond10x/engineering-protocols)
#   HARNESS_REPO            the harness checkout, for the freshness check (default: ~/beyond10x/harness)
#   EVAL_B10X_BINARY        (default: ~/.local/bin/b10x-harness)
#   EVAL_B10X_ENDPOINT      (default: https://api.anthropic.com/v1)
#   EVAL_B10X_MODEL         (default: claude-haiku-4-5-20251001)
#   EVAL_B10X_WIRE          (default: anthropic-messages)
#   EVAL_B10X_TOKEN_FILE    (default: ~/.claude/.credentials.json)
#   EVAL_B10X_TOKEN_POINTER (default: /claudeAiOauth/accessToken)
#   EVAL_B10X_CGROUP_ROOT   (default: the user's systemd slice)
#   EVAL_MAX_ATTEMPTS       the retreat bound on every section (default 3)
#   EVAL_PRICES             a rate card for `--prices`; without one the run reports tokens, no price
#
# Usage:  bash evals/engineering-protocols/run-native.sh            # everything free, then stop
#         bash evals/engineering-protocols/run-native.sh --spend    # and walk it
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO="${EP_REPO:-$HOME/beyond10x/engineering-protocols}"
HARNESS_REPO="${HARNESS_REPO:-$HOME/beyond10x/harness}"
B10X_BINARY="${EVAL_B10X_BINARY:-$HOME/.local/bin/b10x-harness}"
B10X_ENDPOINT="${EVAL_B10X_ENDPOINT:-https://api.anthropic.com/v1}"
B10X_MODEL="${EVAL_B10X_MODEL:-claude-haiku-4-5-20251001}"
B10X_WIRE="${EVAL_B10X_WIRE:-anthropic-messages}"
B10X_TOKEN_FILE="${EVAL_B10X_TOKEN_FILE:-$HOME/.claude/.credentials.json}"
B10X_TOKEN_POINTER="${EVAL_B10X_TOKEN_POINTER:-/claudeAiOauth/accessToken}"
B10X_CGROUP_ROOT="${EVAL_B10X_CGROUP_ROOT:-/sys/fs/cgroup/user.slice/user-$(id -u).slice/user@$(id -u).service}"
MAX_ATTEMPTS="${EVAL_MAX_ATTEMPTS:-3}"
PRICES="${EVAL_PRICES:-}"
TASK_ID="NATIVE-1"
SPEND=0
[[ "${1:-}" == "--spend" ]] && SPEND=1

say() { printf '%s\n' "$*"; }

# Under `--spend`, the walk must run from inside the user's cgroup root — substrate's probe reads
# the calling process's own cgroup, and from a login shell the machine reports no exec facts and
# the loop publishes no `run`. Re-exec first, so one scratch directory serves the whole run.
if [ "$SPEND" -eq 1 ] && [ -z "${EVAL_B10X_SCOPED:-}" ] && [ -d "$B10X_CGROUP_ROOT" ]; then
  if ! grep -q "$(printf '%s' "$B10X_CGROUP_ROOT" | sed 's#^/sys/fs/cgroup##')" /proc/self/cgroup; then
    say "re-executing under systemd-run --user --scope so substrate's cgroup probe sees this process inside $B10X_CGROUP_ROOT"
    export EVAL_B10X_SCOPED=1
    exec systemd-run --user --scope --quiet -- bash "$0" --spend
  fi
fi

# --- 1. the binaries ---------------------------------------------------------------------------
command -v protocol >/dev/null || { say "FAIL: no \`protocol\` on PATH (cargo install --path $REPO/crates/protocol-cli)"; exit 1; }
if ! protocol drive transition --help >/dev/null 2>&1; then
  say "FAIL: this \`protocol\` has no \`drive transition\` — the governor this walk needs; reinstall from engineering-protocols >= 90906c7"
  exit 1
fi
[ -x "$B10X_BINARY" ] || { say "FAIL: no b10x-harness at $B10X_BINARY"; exit 1; }
if [ -d "$HARNESS_REPO/.git" ]; then
  NEWEST_COMMIT="$(git -C "$HARNESS_REPO" log -1 --format=%ct)"
  BINARY_BUILT="$(stat -c %Y "$B10X_BINARY")"
  if [ "$BINARY_BUILT" -lt "$NEWEST_COMMIT" ]; then
    say "FAIL: $B10X_BINARY was built $(date -d "@$BINARY_BUILT" '+%F %T'), older than harness's newest commit"
    say "      $(date -d "@$NEWEST_COMMIT" '+%F %T'); install from a clean checkout of that commit first"
    exit 1
  fi
fi
say "protocol:     $(command -v protocol) ($(protocol --version 2>/dev/null | head -1))"
say "b10x-harness: $B10X_BINARY (built $(date -d "@$(stat -c %Y "$B10X_BINARY")" '+%F %T'))"

# --- 2. the scratch project, exactly as run-driven.sh builds it ------------------------------
SCRATCH_BASE="${TMPDIR:-$HOME/.cache/claude-tmp}"
mkdir -p "$SCRATCH_BASE"
WORK="$(mktemp -d "$SCRATCH_BASE/native-eval.XXXXXX")"
say "scratch directory: $WORK"
TREE="$WORK/tree"
mkdir -p "$TREE/artifacts"
for directory in protocols principles workflows profiles drivers; do
  cp -R "$REPO/$directory" "$TREE/$directory"
done
cp -R "$REPO/artifacts/lifecycles" "$TREE/artifacts/lifecycles"
cp -R "$REPO/artifacts/templates" "$TREE/artifacts/templates"
PROJECT="$WORK/ws_project"
mkdir -p "$PROJECT/.engineering/planning"
cat > "$PROJECT/.engineering/project.yaml" <<YAML
version: aep.project/1
protocol: adp/1
profile: development.driven
protocols: ../../tree
summary: >-
  The native eval's scratch project: an empty planning store, the subject repository's document
  tree copied in, and a task governed by \`development.driven\` — walked by b10x-harness, governed
  by protocol drive transition.
YAML
cat > "$PROJECT/.engineering/task.yaml" <<YAML
id: $TASK_ID
kind: feature
objective: add-passkey-login
protocol: adp/1
profile: development.driven
constraints:
  facts:
    change.public_contract: false
    change.architectural: false
  notes:
    - Existing password sign-in must keep working through the rollout.
YAML
PLUGIN_SRC="$REPO/integrations/claude-code"
mkdir -p "$WORK/plugin"
(cd "$PLUGIN_SRC" && tar -cf - .) | (cd "$WORK/plugin" && tar -xf -)
[ -f "$WORK/plugin/skills/planning/SKILL.md" ] || { say "FAIL: the copied plugin has no planning skill"; exit 1; }
MAP="$SCRIPT_DIR/driven.steps.yaml"

# --- 3. the flow and the hooks ---------------------------------------------------------------
FLOW="$WORK/flow.yaml"
(cd "$TREE" && protocol workflow flow --id adp/default --root "$TREE" --map "$MAP" --max-attempts "$MAX_ATTEMPTS" --out "$FLOW" >/dev/null)
say "flow: $FLOW ($(grep -c '^\s*- id:' "$FLOW") node(s))"
HOOKS="$WORK/hooks.json"
python3 - "$HOOKS" "$PROJECT" "$TREE" "$MAP" <<'PY'
import json, sys
hooks, project, tree, step_map = sys.argv[1:5]
governor = ["protocol", "drive", "transition",
            "--project", project, "--root", tree,
            "--task", f"{project}/.engineering/task.yaml", "--map", step_map]
json.dump({"version": 1, "hooks": [
    {"on": "transition", "command": governor},
    {"on": "before-call", "tools": ["file_write", "file_edit"], "command": ["protocol", "drive", "hook"]},
]}, open(hooks, "w"), indent=2)
PY
say "hooks: $HOOKS (transition -> protocol drive transition; before-call -> protocol drive hook)"

# The governor, consulted by hand once before anything is spent: the same document the loop will
# send at the first boundary. Exit 0 or 2 are both answers; anything else means it cannot answer.
PROBE="$(printf '{"hook":"transition","flow":"adp/default","path":"root.receive","moment":"enter","attempt":1,"of":%s,"workspace":"%s"}' "$MAX_ATTEMPTS" "$PROJECT")"
set +e
PROBE_OUT="$(printf '%s' "$PROBE" | protocol drive transition --project "$PROJECT" --root "$TREE" --task "$PROJECT/.engineering/task.yaml" --map "$MAP" 2>&1)"
PROBE_EXIT=$?
set -e
case "$PROBE_EXIT" in
  0) say "governor: answers (enter root.receive -> proceed)" ;;
  2) say "governor: answers (enter root.receive -> refused: $PROBE_OUT)" ;;
  *) say "FAIL: the governor cannot answer (exit $PROBE_EXIT): $PROBE_OUT"; exit 1 ;;
esac
LEAVE="$(printf '{"hook":"transition","flow":"adp/default","path":"root.receive","moment":"leave","attempt":1,"of":%s,"failed":false,"handoff":{},"workspace":"%s"}' "$MAX_ATTEMPTS" "$PROJECT")"
set +e
LEAVE_OUT="$(printf '%s' "$LEAVE" | protocol drive transition --project "$PROJECT" --root "$TREE" --task "$PROJECT/.engineering/task.yaml" --map "$MAP" 2>&1)"
LEAVE_EXIT=$?
set -e
case "$LEAVE_EXIT" in
  0) say "governor: leave root.receive over an EMPTY store -> proceed (nothing is owed there)" ;;
  2) say "governor: leave root.receive over an EMPTY store -> refused, in the engine's words: $LEAVE_OUT" ;;
  *) say "FAIL: the governor cannot answer leave (exit $LEAVE_EXIT): $LEAVE_OUT"; exit 1 ;;
esac

# --- 4. the shape, free ----------------------------------------------------------------------
"$B10X_BINARY" workflow plan --flow "$FLOW" > "$WORK/plan.txt"
say "plan: $WORK/plan.txt ($(grep -cE '^\s+[0-9]+\.' "$WORK/plan.txt") step(s))"

INPUT="$(cat "$PROJECT/.engineering/task.yaml")"
declare -a RUN=(
  "$B10X_BINARY" workflow run
  --flow "$FLOW" --input "$INPUT" --hooks "$HOOKS" --max-attempts "$MAX_ATTEMPTS"
  --base-url "$B10X_ENDPOINT" --model "$B10X_MODEL" --wire "$B10X_WIRE"
  --oauth-token-file "$B10X_TOKEN_FILE" --oauth-token-pointer "$B10X_TOKEN_POINTER"
  --workspace "$PROJECT" --substrate-embedded --cgroup-root "$B10X_CGROUP_ROOT"
  --allow-program protocol --plugin-dir "$WORK/plugin"
  --session-dir "$WORK/sessions" --json
)
[ -n "$PRICES" ] && RUN+=(--prices "$PRICES")

if [ "$SPEND" -eq 0 ]; then
  say
  say "Everything free has run. The walk itself spends money and is not started."
  say "To walk it (a fresh scratch directory of the same shape, same flow, same governor):"
  say
  say "  bash $SCRIPT_DIR/run-native.sh --spend"
  say
  say "or by hand, from inside the user's cgroup scope:"
  say "  systemd-run --user --scope --quiet -- $(printf '%q ' "${RUN[@]}")"
  exit 0
fi

# --- 5. the walk -----------------------------------------------------------------------------
[ -r "$B10X_TOKEN_FILE" ] || { say "FAIL: no credential at $B10X_TOKEN_FILE (set EVAL_B10X_TOKEN_FILE)"; exit 1; }
say "walking (arm native, $MAX_ATTEMPTS attempt(s) per section) …"
STARTED=$(date +%s)
set +e
"${RUN[@]}" > "$WORK/native.jsonl" 2> "$WORK/native.err"
WALK_EXIT=$?
set -e
ELAPSED=$(( $(date +%s) - STARTED ))
say "walk exit: $WALK_EXIT after ${ELAPSED}s (0 clean · 2 finished and did not · 1 refused or aborted)"

# --- the census ------------------------------------------------------------------------------
say
say "== census ($WORK/native.jsonl)"
python3 - "$WORK/native.jsonl" <<'PY'
import json, sys, collections
kinds = collections.Counter(); refused = []; hooks = collections.Counter(); finished = None
for line in open(sys.argv[1]):
    line = line.strip()
    if not line: continue
    try: e = json.loads(line)
    except json.JSONDecodeError: continue
    k = e.get("kind"); kinds[k] += 1
    if k == "transition-refused": refused.append(e)
    if k == "hook-ran": hooks[str(e.get("decision"))] += 1
    if k == "flow-finished": finished = e
print("events:", ", ".join(f"{k} {n}" for k, n in sorted(kinds.items())))
print(f"transition-refused: {len(refused)}")
for r in refused:
    print(f"  - {r.get('path')} {r.get('moment')} attempt {r.get('attempt')}: {r.get('reason')}")
print("hook decisions:", dict(hooks) or "none")
print("flow-finished:", {k: finished[k] for k in ("ran","failed","skipped","retreats","clean")} if finished else "absent — the walk did not finish")
PY
say
say "== sessions ($WORK/sessions)"
"$B10X_BINARY" sessions --session-dir "$WORK/sessions" || true
say
say "store after the walk:"
(cd "$PROJECT" && protocol artifact list 2>/dev/null | sed 's/^/  /' || true)
say
say "record: $WORK  (flow.yaml, hooks.json, plan.txt, native.jsonl, native.err, sessions/, ws_project/)"
exit "$WALK_EXIT"

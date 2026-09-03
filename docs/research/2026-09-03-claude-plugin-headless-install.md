# Claude Code's headless plugin install, and the scratch-config-home layout

**Date:** 2026-09-03 · **Binary:** `claude` **2.1.258**, at `~/.local/bin/claude` ·
**Method:** `--help` on three verbs, and reading the operator's own `~/.claude` tree. **Nothing
here was spent** — no session was started, no plugin was installed, no network was reached by
anything this note ran.

Written before `--plugin` was built, on `AGENTS.md` § *Where work is tracked* (*"investigations
behind a design"*) and because `story:vendor-plugin-in-scratch-home` names the headless form as
`inferable` and says to establish it here first.

---

## 1. What the CLI offers, verbatim

```console
$ claude plugin --help
Commands:
  install|i [options] <plugin>         Install a plugin from available marketplaces (use
                                       plugin@marketplace for specific marketplace)
  list [options]                       List installed plugins
  marketplace                          Manage Claude Code marketplaces
  uninstall|remove [options] <plugin>  Uninstall an installed plugin
  update [options] <plugin>            Update a plugin to the latest version (restart required)
  validate [options] <path>            Validate a plugin or marketplace manifest, …
```

```console
$ claude plugin marketplace --help
Commands:
  add [options] <source>      Add a marketplace from a URL, path, or GitHub repo
  list [options]              List all configured marketplaces
  remove|rm [options] <name>  Remove a configured marketplace
  update [options] [name]     Update marketplace(s) from their source

$ claude plugin marketplace add --help
Usage: claude plugin marketplace add [options] <source>
Options:
  --scope <scope>      Where to declare the marketplace: user (default), project, or local
  --sparse <paths...>  Limit checkout to specific directories via git sparse-checkout

$ claude plugin install --help
Usage: claude plugin install|i [options] <plugin>
Options:
  --config <key=value>  Set a userConfig option declared in the plugin's manifest (repeatable)
  -s, --scope <scope>   Installation scope: user, project, or local (default: "user")
  -y, --yes             Accept the displayed marketplace-declared command without the
                        confirmation prompt … (required when stdin or stdout is not a TTY)
```

### F1 — the headless form exists, and it is two commands

**Established.** A non-interactive install is:

```console
$ claude plugin marketplace add <owner>/<repo> --scope user
$ claude plugin install <name>@<marketplace> --scope user --yes
```

`--yes` is the headless half and its own help says so: *"required when stdin or stdout is not a
TTY"*. Both verbs honour `CLAUDE_CONFIG_DIR`, on the same reasoning the rest of the adapter
already rests on — the config home is where the two registry documents in § 2 live.

### F2 — **there is no pin**

**Established, and it is the finding that decides `--plugin`.** Neither verb takes a ref, a tag, a
branch or a commit. `marketplace add` takes a *source* and two options, neither of which is a
revision; `install` takes `<plugin>` — the help's own spelling is `plugin@marketplace`, never
`plugin@marketplace@version` — and three options, none of which is a revision. `update` exists and
means *move to latest*, which is the opposite of a pin.

So a run that installed through this CLI would install **whatever the remote said at launch
time**: two arms of one bench could get two different plugins and both would report the same
`--plugin` argument. That is not a comparison.

### F3 — the install reaches the network, at launch

**Established from the verbs' own descriptions** (*"from a URL, path, or GitHub repo"*,
*"from their source"*) and from § 2's `known_marketplaces.json`, which records a `github` source
and a `lastUpdated` timestamp per marketplace. A launch that ran either verb would reach out from
inside the boundary § 8 of the protocol design exists to draw.

---

## 2. The scratch-config-home layout, read from a real one

Read from `~/.claude` at 2.1.258. **This is the layout, not a guess about it** — every path and
every field below was read out of a file on this machine.

```
<config home>/plugins/known_marketplaces.json
<config home>/plugins/installed_plugins.json
<config home>/plugins/marketplaces/<marketplace>/          the fetched marketplace repository
<config home>/plugins/marketplaces/<marketplace>/.claude-plugin/marketplace.json
<config home>/plugins/cache/<marketplace>/<name>/<version>/   the plugin tree itself
<config home>/plugins/cache/<marketplace>/<name>/<version>/.claude-plugin/plugin.json
```

### `known_marketplaces.json`

```json
{
  "beyond10x": {
    "source": { "source": "github", "repo": "beyond10x/agentplugins" },
    "installLocation": "/home/…/.claude/plugins/marketplaces/beyond10x",
    "lastUpdated": "2026-09-02T21:13:36.335Z"
  }
}
```

Keyed by the **marketplace name**, which is the `name` field of the repository's
`.claude-plugin/marketplace.json` and is not derivable from the repo spelling
(`beyond10x/agentplugins` → `beyond10x`). This file is therefore the only offline way to get from
the repo a caller names to the marketplace name the plugin id needs.

### `installed_plugins.json`

```json
{
  "version": 2,
  "plugins": {
    "aep-planning@beyond10x": [
      { "scope": "user",
        "installPath": "/home/…/.claude/plugins/cache/beyond10x/aep-planning/0.4.0",
        "version": "0.4.0",
        "installedAt": "2026-09-02T11:00:58.973Z",
        "lastUpdated": "2026-09-02T21:13:37.085Z",
        "gitCommitSha": "21147b7667dfaefcfa45a094e9542891b1783541" }
    ]
  }
}
```

**`gitCommitSha` is present and is the pin the CLI will not let a caller ask for.** It is written
by the installer after the fetch; a value the operator can read afterwards and cannot request
beforehand. `version` is the plugin manifest's own `version` and is also the cache directory's
name, so the same entry is addressable two ways.

An entry is a **list**, because one plugin can be installed at `user`, `project` and `local` scope
at once, each with its own `installPath` — a reader that took the first element would be picking a
scope by accident.

### The plugin tree

An ordinary directory: `.claude-plugin/plugin.json` (`name`, `version`, `description`, …) beside
`skills/`, `agents/`, `commands/`, `hooks/`. Identical in shape to what `--plugin-dir` already
takes, which is why the copy-and-digest machinery this repository already has needs no second
implementation.

---

## 3. What `--plugin` does, given F1–F3

Decided in `docs/design/runs-side-by-side-v0.1.md` § 3; recorded here as what the evidence
supports:

* **The run runs neither verb.** F2 says a launch-time install cannot be pinned and F3 says it
  reaches the network. `--plugin` resolves against a marketplace the operator has **already
  fetched** with the two commands in F1, copies the tree, and digests it before the copy.
* **The pin is matched against `version` *or* `gitCommitSha`**, both of which § 2 shows are
  recorded per install. An unpinned `--plugin repo@name` is refused by name.
* **The placement is the § 2 layout, rewritten into the scratch home** with `installLocation` and
  `installPath` pointing inside it, so nothing in the assembled home names a path outside it.
* **The refusal names the two commands.** An operator whose marketplace is not fetched gets the
  exact `claude plugin marketplace add …` and `claude plugin install …` to run once, deliberately,
  outside a run.

---

## 4. Open probes — what a paid run would close

| id | question | why reading cannot answer it |
|---|---|---|
| **Q19** — **closed 2026-09-03, answer: no** | does a session launched with `CLAUDE_CONFIG_DIR` pointing at a home **metaharness assembled** actually load the plugin, and what `source` string does the opening record report for it? | Driven once, paid: `aep eval run` over the golden-path case with `--plugin-dir aep-planning --plugin …@adp@0.6.0 --plugin …@ess-schema@0.6.0` (metaharness 0.6.0, claude 2.1.259). `session.started.plugins` = `[{"name":"aep-planning","source":"aep-planning@inline","version":"0.6.0"}]`; `skills` held only `aep-planning:*`; the run later found the ess-schema tree under the *operator's* cache by hand and read the wave skill from a checkout. The registry was placed and not loaded, because enablement lives in the user settings source that `--setting-sources ""` removes. Since 0.6.1 the launch also passes `--plugin-dir <config home>/plugins/cache/<mkt>/<name>/<pin>` for each pin; Q21 stays open (the `source` string for a registry-loaded plugin was never produced) |
| **Q20** | is there any headless spelling that pins? | `--help` on three verbs shows none at 2.1.258. A future version could add one; this note is pinned to the version it was read from |
| **Q21** | does a marketplace-installed plugin's `source` differ from a `--plugin-dir` one's (`<name>@inline` in the golden fixture)? | same reason as Q19 — it is a field in an opening record nobody has produced for this path |

Q19 closed the day this note was written, in the direction the note allowed for: the assembled
registry is *read* and not *loaded*. `InstalledPlugin::loaded_by` for a `--plugin` install now says
both placements and names this probe; the 0.6.0 `CHANGELOG` entry that said *not driven* stands as
the record of what was known when it shipped (`AGENTS.md` invariant 4).

//! Claude Code's plugin registry: where it lives, what it says, and how a pinned plugin is placed
//! into a scratch config home.
//!
//! **Everything vendor-specific about a marketplace plugin is here**, on invariant 2. The neutral
//! half — the `<repo>@<name>@<pin>` spelling and the refusal an unpinned one gets — is
//! [`metaharness_protocol::MarketplacePlugin`].
//!
//! # Read first, driven once, and the difference is stated
//!
//! Every path and every field below was **read out of a real config home** at Claude Code 2.1.258
//! and is recorded verbatim in `docs/research/2026-09-03-claude-plugin-headless-install.md`. The
//! note's probe **Q19** — does a session launched against a config home *metaharness* assembled
//! load what is placed there — was driven on 2026-09-03 and answered **no**: a session declaring two
//! pinned plugins opened with only its `--plugin-dir` plugin in `session.started.plugins`. The
//! registry is enabled through the user settings source, and `--setting-sources ""` (H2) switches
//! that source off. So the launch places the copy here **and** names it to the vendor with
//! `--plugin-dir` (`launch::build_args`); the registry documents keep the vendor's own bookkeeping
//! coherent, and [`metaharness_protocol::InstalledPlugin::loaded_by`] says which of the two loads.
//!
//! # The run reaches no network
//!
//! `claude plugin marketplace add` and `claude plugin install` both fetch, and neither takes a
//! ref, a tag or a commit at 2.1.258 — so a launch-time install would be both unpinnable and a
//! network reach from inside the hermetic boundary. [`resolve_marketplace`] therefore resolves
//! against what the operator has **already** fetched, and every refusal names the two commands
//! that fetch it, to be run once, deliberately, outside a run.

use std::fmt;

use metaharness_protocol::MarketplacePlugin;
use serde_json::{Map, Value};

/// The directory Claude Code keeps its plugin bookkeeping in, under the config home.
pub const PLUGIN_REGISTRY_HOME: &str = "plugins";

/// The marketplace registry, relative to the config home.
pub const KNOWN_MARKETPLACES: &str = "plugins/known_marketplaces.json";

/// The installed-plugin registry, relative to the config home.
pub const INSTALLED_PLUGINS: &str = "plugins/installed_plugins.json";

/// Where a fetched marketplace's checkout sits, relative to the config home.
pub const MARKETPLACES_HOME: &str = "plugins/marketplaces";

/// Where an installed plugin's tree sits, relative to the config home.
pub const PLUGIN_CACHE_HOME: &str = "plugins/cache";

/// The `version` field of `installed_plugins.json`, as written by 2.1.258.
const REGISTRY_VERSION: u64 = 2;

/// One resolution: where the operator's copy of a pinned plugin is, and what it is called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketplaceMatch {
    /// The marketplace's **name**, which is not the repo spelling and is only knowable from the
    /// registry.
    pub marketplace: String,
    /// The operator's own directory holding the plugin tree.
    pub install_path: String,
    /// The version that entry records.
    pub version: String,
    /// The commit the installer recorded, where it recorded one.
    pub commit: Option<String>,
}

/// Why a declared marketplace plugin could not be resolved offline.
///
/// Each one names what to run **once, outside a run**, because that is where the fetch belongs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketplaceRefusal {
    /// No fetched marketplace has this source repository.
    MarketplaceNotFetched {
        /// What was asked for.
        plugin: MarketplacePlugin,
    },
    /// The marketplace is fetched and this plugin is not installed from it.
    PluginNotInstalled {
        /// What was asked for.
        plugin: MarketplacePlugin,
        /// The marketplace it was looked for in.
        marketplace: String,
    },
    /// The plugin is installed and not at this pin.
    PinNotInstalled {
        /// What was asked for.
        plugin: MarketplacePlugin,
        /// The marketplace it was looked for in.
        marketplace: String,
        /// Every pin that *is* installed — versions and commits both, so the reader can copy one.
        available: Vec<String>,
    },
    /// A registry document was there and did not have the shape 2.1.258 writes.
    RegistryMisshapen {
        /// Which document.
        document: &'static str,
        /// What was wrong with it.
        detail: String,
    },
}

impl fmt::Display for MarketplaceRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarketplaceRefusal::MarketplaceNotFetched { plugin } => write!(
                f,
                "--plugin {plugin}: no marketplace in this machine's `{KNOWN_MARKETPLACES}` has \
                 the source repository `{}`. A run fetches nothing — neither `marketplace add` \
                 nor `install` takes a pin at 2.1.258, so a launch-time fetch would be \
                 unreproducible. Fetch it once, deliberately, outside a run:\n  \
                 claude plugin marketplace add {}\n  claude plugin install {}@<marketplace>",
                plugin.repo, plugin.repo, plugin.name
            ),
            MarketplaceRefusal::PluginNotInstalled {
                plugin,
                marketplace,
            } => write!(
                f,
                "--plugin {plugin}: the marketplace `{marketplace}` is fetched and `{}` is not \
                 installed from it. Install it once, outside a run:\n  \
                 claude plugin install {}@{marketplace} --scope user --yes",
                plugin.name, plugin.name
            ),
            MarketplaceRefusal::PinNotInstalled {
                plugin,
                marketplace,
                available,
            } => write!(
                f,
                "--plugin {plugin}: `{}@{marketplace}` is installed and not at `{}`. Installed \
                 pins: {}. The pin is matched against the entry's `version` or its `gitCommitSha`, \
                 and it is required rather than defaulted — a run that took whatever was latest \
                 would not be the run somebody reproduces",
                plugin.name,
                plugin.pin,
                if available.is_empty() {
                    "none".to_string()
                } else {
                    available.join(", ")
                }
            ),
            MarketplaceRefusal::RegistryMisshapen { document, detail } => write!(
                f,
                "{document} is not the shape Claude Code 2.1.258 writes: {detail}. Refused rather \
                 than guessed — a registry this build misread would place a plugin somewhere the \
                 vendor does not look, and the run would report a plugin nothing loaded"
            ),
        }
    }
}

impl std::error::Error for MarketplaceRefusal {}

/// Resolve one declared plugin against the operator's two registry documents.
///
/// Pure: it reads two already-parsed values and no filesystem, so the whole resolution is a value
/// a test decides about — the same division `plan_launch` draws for everything else.
///
/// # Errors
///
/// [`MarketplaceRefusal`], one variant per way the operator's machine does not have what the run
/// named, each naming what to run once to fix it.
pub fn resolve_marketplace(
    plugin: &MarketplacePlugin,
    known_marketplaces: &Value,
    installed_plugins: &Value,
) -> Result<MarketplaceMatch, MarketplaceRefusal> {
    let known =
        known_marketplaces
            .as_object()
            .ok_or_else(|| MarketplaceRefusal::RegistryMisshapen {
                document: KNOWN_MARKETPLACES,
                detail: "the document is not an object keyed by marketplace name".to_string(),
            })?;

    // The repo spelling is the only thing a caller can be expected to know; the marketplace's
    // *name* comes out of its own manifest and is only readable here.
    let marketplace = known
        .iter()
        .find(|(_, entry)| {
            entry.pointer("/source/repo").and_then(Value::as_str) == Some(plugin.repo.as_str())
        })
        .map(|(name, _)| name.clone())
        .ok_or_else(|| MarketplaceRefusal::MarketplaceNotFetched {
            plugin: plugin.clone(),
        })?;

    let id = format!("{}@{marketplace}", plugin.name);
    let entries = installed_plugins
        .pointer("/plugins")
        .and_then(Value::as_object)
        .ok_or_else(|| MarketplaceRefusal::RegistryMisshapen {
            document: INSTALLED_PLUGINS,
            detail: "no `plugins` object".to_string(),
        })?;
    // A **list**, because one plugin can be installed at `user`, `project` and `local` scope at
    // once, each with its own path. Taking the first would be picking a scope by accident.
    let installs = entries.get(&id).and_then(Value::as_array).ok_or_else(|| {
        MarketplaceRefusal::PluginNotInstalled {
            plugin: plugin.clone(),
            marketplace: marketplace.clone(),
        }
    })?;

    let mut available = Vec::new();
    for install in installs {
        let version = install.get("version").and_then(Value::as_str);
        let commit = install.get("gitCommitSha").and_then(Value::as_str);
        for pin in [version, commit].into_iter().flatten() {
            if !available.contains(&pin.to_string()) {
                available.push(pin.to_string());
            }
        }
        if version == Some(plugin.pin.as_str()) || commit == Some(plugin.pin.as_str()) {
            let install_path = install
                .get("installPath")
                .and_then(Value::as_str)
                .ok_or_else(|| MarketplaceRefusal::RegistryMisshapen {
                    document: INSTALLED_PLUGINS,
                    detail: format!("the entry for `{id}` has no `installPath`"),
                })?;
            return Ok(MarketplaceMatch {
                marketplace,
                install_path: install_path.to_string(),
                version: version.unwrap_or(&plugin.pin).to_string(),
                commit: commit.map(ToString::to_string),
            });
        }
    }

    Err(MarketplaceRefusal::PinNotInstalled {
        plugin: plugin.clone(),
        marketplace,
        available,
    })
}

/// One plugin, as the scratch registry records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScratchEntry {
    /// The marketplace's name.
    pub marketplace: String,
    /// Its source repository, so the assembled registry says where it came from.
    pub repo: String,
    /// The plugin's name.
    pub name: String,
    /// The version the tree is.
    pub version: String,
    /// The commit, where the operator's installer recorded one.
    pub commit: Option<String>,
    /// Where the tree sits **inside the scratch config home**.
    pub installed_at: String,
    /// Where the marketplace checkout sits inside the scratch config home.
    pub marketplace_at: String,
}

/// The two registry documents a scratch config home needs, in the layout 2.1.258 writes.
///
/// Deterministic: both are `serde_json` objects, which are `BTreeMap`s in this workspace, and the
/// entries are inserted from an ordered slice.
///
/// **Nothing in either document names a path outside the scratch home.** A registry that pointed
/// at the operator's own cache would make the copy pointless — the child would read the live tree,
/// which is the very thing the digest exists to pin against.
#[must_use]
pub fn scratch_registry(entries: &[ScratchEntry]) -> (Value, Value) {
    let mut marketplaces = Map::new();
    let mut plugins = Map::new();

    for entry in entries {
        marketplaces.insert(
            entry.marketplace.clone(),
            serde_json::json!({
                "source": {"source": "github", "repo": entry.repo},
                "installLocation": entry.marketplace_at,
            }),
        );
        let mut install = Map::new();
        install.insert("scope".to_string(), Value::from("user"));
        install.insert(
            "installPath".to_string(),
            Value::from(entry.installed_at.clone()),
        );
        install.insert("version".to_string(), Value::from(entry.version.clone()));
        if let Some(commit) = &entry.commit {
            install.insert("gitCommitSha".to_string(), Value::from(commit.clone()));
        }
        plugins.insert(
            format!("{}@{}", entry.name, entry.marketplace),
            Value::Array(vec![Value::Object(install)]),
        );
    }

    // **No `lastUpdated`, and no `installedAt`.** A real registry carries both; metaharness reads
    // no clock (design D2), and a timestamp invented here would make the same run's scratch home
    // differ from itself between two launches.
    (
        Value::Object(marketplaces),
        serde_json::json!({"version": REGISTRY_VERSION, "plugins": Value::Object(plugins)}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known() -> Value {
        serde_json::json!({
            "beyond10x": {
                "source": {"source": "github", "repo": "beyond10x/agentplugins"},
                "installLocation": "/operator/.claude/plugins/marketplaces/beyond10x"
            }
        })
    }

    fn installed() -> Value {
        serde_json::json!({
            "version": 2,
            "plugins": {
                "aep-planning@beyond10x": [
                    {"scope": "user", "installPath": "/operator/cache/0.3.7", "version": "0.3.7"},
                    {"scope": "local", "installPath": "/operator/cache/0.4.0", "version": "0.4.0",
                     "gitCommitSha": "21147b7"}
                ]
            }
        })
    }

    fn asked(text: &str) -> MarketplacePlugin {
        text.parse().expect("parses")
    }

    /// A plugin installed at two scopes has two entries, and the pin picks which — not the order.
    #[test]
    fn the_pin_picks_the_entry_and_not_the_first_scope_in_the_list() {
        let found = resolve_marketplace(
            &asked("beyond10x/agentplugins@aep-planning@0.4.0"),
            &known(),
            &installed(),
        )
        .expect("resolves");
        assert_eq!(found.install_path, "/operator/cache/0.4.0");
        assert_eq!(found.commit.as_deref(), Some("21147b7"));
    }

    #[test]
    fn an_unknown_pin_lists_every_pin_that_is_installed_under_both_spellings() {
        let refusal = resolve_marketplace(
            &asked("beyond10x/agentplugins@aep-planning@9.9.9"),
            &known(),
            &installed(),
        )
        .expect_err("not installed");
        let said = refusal.to_string();
        for pin in ["0.3.7", "0.4.0", "21147b7"] {
            assert!(said.contains(pin), "{said}");
        }
    }

    /// The registry is assembled from values and carries no clock.
    #[test]
    fn the_assembled_registry_carries_no_timestamp() {
        let (marketplaces, plugins) = scratch_registry(&[ScratchEntry {
            marketplace: "beyond10x".to_string(),
            repo: "beyond10x/agentplugins".to_string(),
            name: "aep-planning".to_string(),
            version: "0.4.0".to_string(),
            commit: None,
            installed_at: "/scratch/claude-home/plugins/cache/beyond10x/aep-planning/0.4.0"
                .to_string(),
            marketplace_at: "/scratch/claude-home/plugins/marketplaces/beyond10x".to_string(),
        }]);
        let rendered = format!("{marketplaces}{plugins}");
        assert!(!rendered.contains("lastUpdated"), "{rendered}");
        assert!(!rendered.contains("installedAt"), "{rendered}");
        assert!(rendered.contains("aep-planning@beyond10x"));
    }

    #[test]
    fn an_empty_registry_is_two_empty_documents_and_never_an_absent_one() {
        let (marketplaces, plugins) = scratch_registry(&[]);
        assert_eq!(marketplaces, serde_json::json!({}));
        assert_eq!(plugins, serde_json::json!({"version": 2, "plugins": {}}));
    }
}

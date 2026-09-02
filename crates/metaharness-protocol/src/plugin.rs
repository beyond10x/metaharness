//! Plugin injection: what a run installed, where it put it, and what it digests to.
//!
//! The neutral half of crossing #4. Nothing here names a vendor — **where** a plugin has to sit
//! for a particular binary to load it is that adapter's own named constant, and each adapter says
//! how strong its evidence for that placement is.
//!
//! Two rules make the rest of it work:
//!
//! * **The caller reads the directory; the launch plan only decides.** [`PluginTree`] is what a
//!   caller found on disk, handed to a pure `plan_launch` the same way the ancestor walk and the
//!   inputs digest are, so the copy list and the digest are values a test reads **before** a
//!   process exists (design § 8.4 O7).
//! * **The digest is over paths and contents together** ([`tree_digest`]). A digest over contents
//!   alone would not move when a file was renamed, and renaming is how a plugin's `SKILL.md`
//!   stops being loaded while every byte in the tree stays the same.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::frame::Digest;

/// The digest of a directory: its file paths **and** their contents, in one canonical order.
///
/// The canonical form is one line per file — `<relative path> <sha256 of its bytes>\n`, in byte
/// order of the path — and the digest is [`Digest::of`] over that text. Stated here in full
/// because two processes have to agree on it: metaharness computes it at launch and a consumer
/// recomputing it from the same tree must get the same string, or the attestation cites a number
/// nobody else can arrive at.
///
/// The per-file digests are the caller's, so a large plugin never has to be held in memory at
/// once.
#[must_use]
pub fn tree_digest(files: &BTreeMap<String, Digest>) -> Digest {
    let mut canonical = String::new();
    for (path, digest) in files {
        let _ = writeln!(canonical, "{path} {digest}");
    }
    Digest::of(canonical.as_bytes())
}

/// What a caller found when it read one declared plugin directory.
///
/// A value and not a `Result`, because the two ways of being unusable are refused **by the launch
/// plan**, by name, beside every other launch refusal — and a caller that had already turned them
/// into its own error would have taken that refusal away from the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginTree {
    /// The directory the run named, as it named it.
    pub source: PathBuf,
    /// What is in it.
    pub content: PluginContent,
}

impl PluginTree {
    /// The plugin's name: the directory's own last component.
    ///
    /// The name the vendor's opening record is expected to report, so H1a's *"loaded exactly the
    /// declared set"* has something to compare against.
    #[must_use]
    pub fn name(&self) -> String {
        self.source
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    }
}

/// What one declared plugin directory holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginContent {
    /// This many regular files, digesting to this.
    Files {
        /// How many files were read.
        count: usize,
        /// [`tree_digest`] over them.
        digest: Digest,
    },
    /// It is a directory and it holds no file at all.
    ///
    /// Its own case rather than `Files { count: 0 }`, because an empty directory is the shape a
    /// mistyped path produces after somebody "fixed" it by creating the directory, and a run that
    /// installed nothing and reported an installed plugin is exactly the silent failure the
    /// declaration exists to prevent.
    Empty,
    /// It could not be read, or is not a directory.
    Unreadable {
        /// What the filesystem said, verbatim.
        detail: String,
    },
}

/// One plugin directory to copy into a run's scratch tree before the child starts.
///
/// The copy is performed **once, at launch**, unlike a credential copy — which happens before
/// every spawn because a token ages out (H6, Q13). A plugin does not age; what it must not do is
/// change under a run that has already digested it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInstall {
    /// The operator's directory.
    pub from: PathBuf,
    /// Where it goes in the run's scratch tree — the adapter's placement.
    pub to: PathBuf,
    /// [`tree_digest`] over `from`, computed before the copy.
    pub digest: Digest,
}

/// One installed plugin, as the attestation states it.
///
/// Strings rather than paths because this is a record somebody reads and a consumer parses, and
/// because a path that is not UTF-8 must degrade to a lossy name rather than fail to serialize
/// the whole attestation.
///
/// **The attestation is not evidence** (design § 8.3). This block says what metaharness copied
/// and where; whether the vendor then loaded it is asserted from the vendor's own opening record,
/// which is what H1a compares against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPlugin {
    /// The plugin's name — the source directory's last component.
    pub name: String,
    /// Where it came from, as the run named it.
    pub source: String,
    /// Where it was put, which is inside the run's own scratch tree.
    pub installed_at: String,
    /// [`tree_digest`] over the source, computed before the copy.
    pub digest: Digest,
    /// **How this vendor is told the plugin is there, and how strong that claim is.**
    ///
    /// Carried per install rather than left to the adapter's documentation, because the two
    /// adapters do not know it equally well: one names the directory in the argv with the
    /// vendor's own flag, and the other puts it at a path read from strings in a binary and
    /// driven by nobody. A reader of the record must be able to tell those apart without
    /// leaving it.
    pub loaded_by: String,
}

/// A plugin named by **marketplace coordinates and a pin**: `<repo>@<name>@<version-or-commit>`.
///
/// The neutral half of amendment a16. Nothing here knows how a particular vendor stores a
/// marketplace on disk — that is the adapter's, on invariant 2 — and nothing here fetches: this is
/// the *spelling* a caller uses and the refusal it gets when the spelling names something that
/// cannot be reproduced.
///
/// **Three segments, and the third is not optional.** An unpinned plugin names something that can
/// change between two runs that both claim to have used it, which makes a bench's two arms
/// incomparable and the run unreproducible. It is refused by name rather than warned about, on
/// design § 7.1's rule: a control that reports and proceeds has already stopped controlling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplacePlugin {
    /// The marketplace's source repository, as the caller named it — `owner/repo`.
    ///
    /// Never the marketplace's *name*: the two differ (`beyond10x/agentplugins` is the marketplace
    /// `beyond10x`) and only the adapter's registry can get from one to the other.
    pub repo: String,
    /// The plugin's own name inside that marketplace.
    pub name: String,
    /// The pin: a version, or a commit. Which of the two it is, is the adapter's to decide when it
    /// resolves — a caller who knows only the version must not have to look up a sha.
    pub pin: String,
}

impl std::fmt::Display for MarketplacePlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}@{}", self.repo, self.name, self.pin)
    }
}

/// Why a `--plugin` spelling was refused, before anything was resolved or spawned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketplacePluginError {
    /// Fewer than three segments: no pin.
    Unpinned {
        /// What was written.
        given: String,
    },
    /// A segment that is there and empty.
    EmptySegment {
        /// Which one.
        segment: &'static str,
        /// What was written.
        given: String,
    },
}

impl std::fmt::Display for MarketplacePluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketplacePluginError::Unpinned { given } => write!(
                f,
                "`{given}` names no pin. Write `<repo>@<name>@<version-or-commit>`: an unpinned \
                 plugin can change between two runs that both claim to have used it, which makes \
                 the two arms of a comparison incomparable and neither of them reproducible"
            ),
            MarketplacePluginError::EmptySegment { segment, given } => write!(
                f,
                "`{given}` has an empty {segment}. Write `<repo>@<name>@<version-or-commit>`; a \
                 blank segment names nothing and would be resolved against everything"
            ),
        }
    }
}

impl std::error::Error for MarketplacePluginError {}

impl std::str::FromStr for MarketplacePlugin {
    type Err = MarketplacePluginError;

    /// Split on the **last two** `@`, because a repository spelling never contains one and a
    /// plugin name never does, while a pin might one day.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (head, pin) =
            text.rsplit_once('@')
                .ok_or_else(|| MarketplacePluginError::Unpinned {
                    given: text.to_string(),
                })?;
        let (repo, name) =
            head.rsplit_once('@')
                .ok_or_else(|| MarketplacePluginError::Unpinned {
                    given: text.to_string(),
                })?;
        for (segment, value) in [("repo", repo), ("name", name), ("pin", pin)] {
            if value.is_empty() {
                return Err(MarketplacePluginError::EmptySegment {
                    segment: match segment {
                        "repo" => "repo",
                        "name" => "name",
                        _ => "pin",
                    },
                    given: text.to_string(),
                });
            }
        }
        Ok(Self {
            repo: repo.to_string(),
            name: name.to_string(),
            pin: pin.to_string(),
        })
    }
}

/// One declared marketplace plugin, **resolved by the caller** against an already-fetched
/// marketplace and read off disk.
///
/// The same division as [`PluginTree`] and the ancestor walk: the caller reads, the launch plan
/// decides. A declared plugin with no resolution here is a caller that forgot to look, and it is
/// refused rather than silently planned without.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMarketplacePlugin {
    /// What the run asked for.
    pub requested: MarketplacePlugin,
    /// The marketplace's **name**, which is what the plugin's id is spelled with.
    pub marketplace: String,
    /// The version the resolution landed on, whichever spelling of the pin was given.
    pub version: String,
    /// The commit the installer recorded, where it recorded one.
    pub commit: Option<String>,
    /// The source tree, as the caller read it.
    pub tree: PluginTree,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(files: [(&str, &[u8]); 2]) -> BTreeMap<String, Digest> {
        files
            .into_iter()
            .map(|(path, bytes)| (path.to_string(), Digest::of(bytes)))
            .collect()
    }

    /// One byte, in one file, and the whole tree digests differently. A digest a mutation cannot
    /// move is decoration, not a pin.
    #[test]
    fn one_edited_byte_in_one_file_changes_the_trees_digest() {
        let before = tree_digest(&tree([
            (".claude-plugin/plugin.json", b"{\"name\":\"x\"}"),
            ("skills/one/SKILL.md", b"do the thing"),
        ]));
        let after = tree_digest(&tree([
            (".claude-plugin/plugin.json", b"{\"name\":\"x\"}"),
            ("skills/one/SKILL.md", b"do the thang"),
        ]));
        assert_ne!(before, after);
    }

    /// The paths are in the digest too: a file renamed is a plugin whose skill stopped loading
    /// while every byte in the tree stayed the same.
    #[test]
    fn a_renamed_file_changes_the_digest_although_no_content_did() {
        let before = tree_digest(&tree([
            (".claude-plugin/plugin.json", b"{}"),
            ("skills/one/SKILL.md", b"body"),
        ]));
        let after = tree_digest(&tree([
            (".claude-plugin/plugin.json", b"{}"),
            ("skills/one/SKILL.MD", b"body"),
        ]));
        assert_ne!(before, after);
    }

    /// The same tree read twice is the same string, whatever order the caller happened to walk
    /// it in — a `BTreeMap` is the ordering, said once.
    #[test]
    fn the_same_tree_digests_to_the_same_string_twice() {
        let first = tree_digest(&tree([("b", b"2"), ("a", b"1")]));
        let second = tree_digest(&tree([("a", b"1"), ("b", b"2")]));
        assert_eq!(first, second);
        assert_eq!(first.as_str().len(), 64);
    }

    #[test]
    fn an_empty_tree_still_has_a_digest_and_it_is_not_a_files_digest() {
        let empty = tree_digest(&BTreeMap::new());
        assert_eq!(empty, Digest::of(b""));
    }

    #[test]
    fn a_plugin_tree_names_itself_after_its_directory() {
        let tree = PluginTree {
            source: PathBuf::from("/operator/integrations/claude-code"),
            content: PluginContent::Empty,
        };
        assert_eq!(tree.name(), "claude-code");
    }
}

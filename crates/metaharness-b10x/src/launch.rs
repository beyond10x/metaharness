//! What metaharness spawns, and what it does not have to.
//!
//! # A launch this short is the point
//!
//! The other adapters spend two thousand lines each making a vendor hermetic: a scratch `HOME`, a
//! copied plugin tree, a hook channel, a retained transcript pulled out of the vendor's own store,
//! a credential kept away from the ambient environment.
//!
//! Almost none of that applies here, and not because this adapter is unfinished:
//!
//! | what a vendor needs | why `b10x-harness` does not |
//! |---|---|
//! | a scratch home, so ambient config cannot leak in | it reads no config file at all |
//! | a copied plugin tree | it has no plugin mechanism |
//! | a hook channel | its decisions are in-process, and this adapter makes none |
//! | transcript retrieval | `--json` on stdout **is** the record |
//! | credential isolation | it reads no credential it was not pointed at, by construction |
//!
//! What is left is an argv and the attestation that says so.

use std::path::Path;

/// One launch of `b10x-harness run`.
#[derive(Debug, Clone)]
pub struct B10xLaunch {
    /// The binary, on `PATH` or by path.
    pub program: String,
    /// The endpoint origin plus API prefix.
    pub base_url: String,
    /// The model the endpoint serves.
    pub model: String,
    /// The file holding the bearer credential. Named, never ambient.
    pub api_key_file: String,
    /// The tree the read-only tools may see.
    pub workspace: String,
    /// The substrate socket, where the run may write and execute.
    pub substrate: Option<String>,
    /// Programs `run` may start. Empty publishes no `run`.
    pub allow_program: Vec<String>,
    /// Ceiling on model turns.
    pub max_turns: Option<u32>,
    /// The request.
    pub input: String,
}

impl B10xLaunch {
    /// A launch against one endpoint, with the read-only toolset only.
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key_file: impl AsRef<Path>,
        workspace: impl AsRef<Path>,
        input: impl Into<String>,
    ) -> Self {
        Self {
            program: "b10x-harness".to_owned(),
            base_url: base_url.into(),
            model: model.into(),
            api_key_file: api_key_file.as_ref().display().to_string(),
            workspace: workspace.as_ref().display().to_string(),
            substrate: None,
            allow_program: Vec::new(),
            max_turns: None,
            input: input.into(),
        }
    }

    /// The same launch, with a confined workspace and the programs it may start.
    #[must_use]
    pub fn confined(
        mut self,
        socket: impl AsRef<Path>,
        programs: impl IntoIterator<Item = String>,
    ) -> Self {
        self.substrate = Some(socket.as_ref().display().to_string());
        self.allow_program = programs.into_iter().collect();
        self
    }

    /// A ceiling on model turns.
    #[must_use]
    pub fn with_max_turns(mut self, turns: u32) -> Self {
        self.max_turns = Some(turns);
        self
    }
}

/// The argv this launch spawns.
///
/// `--json` is not optional and is not a caller's choice: the whole adapter reads that record, and
/// a launch without it would produce a run nothing could observe.
#[must_use]
pub fn argv(launch: &B10xLaunch) -> Vec<String> {
    let mut argv = vec![
        launch.program.clone(),
        "run".to_owned(),
        "--base-url".to_owned(),
        launch.base_url.clone(),
        "--model".to_owned(),
        launch.model.clone(),
        "--api-key-file".to_owned(),
        launch.api_key_file.clone(),
        "--workspace".to_owned(),
        launch.workspace.clone(),
        "--json".to_owned(),
    ];
    if let Some(socket) = &launch.substrate {
        argv.push("--substrate".to_owned());
        argv.push(socket.clone());
    }
    for program in &launch.allow_program {
        argv.push("--allow-program".to_owned());
        argv.push(program.clone());
    }
    if let Some(turns) = launch.max_turns {
        argv.push("--max-turns".to_owned());
        argv.push(turns.to_string());
    }
    // Last, because it is the one argument that can look like a flag and every other argument is
    // positional-free. `-p` on the vendor adapters is here for the same reason.
    argv.push("--input".to_owned());
    argv.push(launch.input.clone());
    argv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch() -> B10xLaunch {
        B10xLaunch::new(
            "https://gw.example/v1",
            "gpt-5.6-sol",
            "/run/secrets/key",
            "/work",
            "do the thing",
        )
    }

    #[test]
    fn a_launch_is_always_observable_because_the_record_is_not_optional() {
        let argv = argv(&launch());
        assert!(argv.contains(&"--json".to_owned()), "{argv:?}");
    }

    #[test]
    fn the_credential_is_named_and_never_left_to_the_environment() {
        let argv = argv(&launch());
        let at = argv.iter().position(|word| word == "--api-key-file").expect("named");
        assert_eq!(argv[at + 1], "/run/secrets/key");
        assert!(
            !argv.iter().any(|word| word == "--api-key-env"),
            "a launch that could fall back to the environment is one whose credential depends on \
             where it was started: {argv:?}"
        );
    }

    #[test]
    fn a_read_only_launch_names_no_socket_and_no_program() {
        let argv = argv(&launch());
        assert!(!argv.iter().any(|word| word == "--substrate"), "{argv:?}");
        assert!(!argv.iter().any(|word| word == "--allow-program"), "{argv:?}");
    }

    #[test]
    fn a_confined_launch_names_the_socket_and_every_program_separately() {
        let argv = argv(&launch().confined(
            "/run/substrate.sock",
            ["cargo".to_owned(), "protocol".to_owned()],
        ));
        let at = argv.iter().position(|word| word == "--substrate").expect("named");
        assert_eq!(argv[at + 1], "/run/substrate.sock");
        let programs: Vec<&String> = argv
            .iter()
            .enumerate()
            .filter(|(index, word)| *word == "--allow-program" && *index + 1 < argv.len())
            .map(|(index, _)| &argv[index + 1])
            .collect();
        assert_eq!(programs, vec!["cargo", "protocol"]);
    }

    #[test]
    fn the_request_is_last_because_it_is_the_one_argument_that_can_look_like_a_flag() {
        let argv = argv(&launch().with_max_turns(8));
        assert_eq!(argv[argv.len() - 2], "--input");
        assert_eq!(argv[argv.len() - 1], "do the thing");
    }
}

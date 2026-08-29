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

use std::path::{Path, PathBuf};

/// The `PATH` a b10x child is given, before `HOME`'s own bin directory is prepended.
///
/// The child's environment is **constructed, never inherited** (design § 8.1 H3), so without this
/// there is no `PATH` at all and a bare program name resolves to nothing. That is not hypothetical:
/// it is what the arm did — `env_clear()` and an empty map — and every launch died with
/// `No such file or directory` naming nothing.
const BASE_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// The `PATH` a spawned child gets, given the `HOME` it will run with.
///
/// Public for the same reason the Claude adapter's is: `doctor` must resolve the binary **the way
/// the spawn will** (CT-3). Until now it read the *operator's* `PATH` for this kind while the spawn
/// used none, so a pre-flight could bless a binary the run could not even find.
#[must_use]
pub fn child_path(home: Option<&str>) -> String {
    match home {
        Some(home) => format!("{home}/.local/bin:{BASE_PATH}"),
        None => BASE_PATH.to_owned(),
    }
}

/// The first executable named `program` on `path`, or `None`.
///
/// Resolved at plan time so the launch record names the **file that ran** rather than a word that
/// was looked up later. A machine holding two installs resolves them differently on two different
/// `PATH`s, and a record naming only `b10x-harness` cannot tell a reader which one answered.
///
/// A program that is already a path is returned as it stands: the caller named a file, and
/// searching for it would be second-guessing them.
#[must_use]
pub fn resolve_program(program: &str, path: &str) -> Option<PathBuf> {
    if program.contains('/') {
        return Some(PathBuf::from(program));
    }
    path.split(':')
        .filter(|directory| !directory.is_empty())
        .map(|directory| Path::new(directory).join(program))
        .find(|candidate| candidate.is_file())
}

/// Where the run's writes and executions are confined, if anywhere.
///
/// Without one the catalogue behind the three verbs is read-only. That is not a setting the loop
/// chooses: what appears is what the machine can confine, so a run with no confinement has no
/// write entry to publish and an arm launched that way cannot attempt a task that changes a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confinement {
    /// `--substrate <socket>`: substrate's daemon, which authenticates a peer.
    Daemon(String),
    /// `--substrate-embedded`: the driver in the loop's own process.
    ///
    /// The root is the workspace's **parent**, because the workspace is adopted rather than
    /// created — and the loop derives it from `--workspace` itself, so the flag carries no value.
    /// It was passed one until 2026-08-29, which the loop then rejected as an unexpected
    /// positional argument: every confined launch died on argument parsing before reaching a
    /// model, with `error: unexpected argument '<root>' found` on stderr and no terminal record.
    /// The root is still held here, unsent, because the launch record has to say which tree was
    /// served and the argv no longer does.
    Embedded(String),
}

/// Which model API the loop is told to speak.
///
/// Named rather than defaulted, because the two are different endpoints under the same
/// `--base-url` and a launch that guessed wrong reaches a 404 that names nothing. [`None`] on a
/// launch leaves the flag off entirely, which is the loop's own default and not this adapter's
/// opinion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    /// `POST {base-url}/responses`.
    OpenAiResponses,
    /// `POST {base-url}/messages`.
    AnthropicMessages,
}

impl Wire {
    /// The word the loop's `--wire` takes.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAiResponses => "openai-responses",
            Self::AnthropicMessages => "anthropic-messages",
        }
    }

    /// The same word, back, so a caller reading a config file need not spell the match out.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "openai-responses" => Some(Self::OpenAiResponses),
            "anthropic-messages" => Some(Self::AnthropicMessages),
            _ => None,
        }
    }
}

/// Where the loop is told to read its bearer.
///
/// Four variants and no fifth: the loop refuses to pick a credential up from anywhere it was not
/// pointed at, so there is nothing to express beyond *this file* and *this variable* — twice, once
/// for a key issued to a program and once for a subscription token, because the two travel under
/// **different header names** and the loop cannot tell them apart from the value.
///
/// A pointer accompanies the subscription variants and not the key ones for the same reason: a
/// subscription store is a JSON document with a token somewhere inside it, and which field is that
/// store's business rather than something this adapter knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credential {
    /// `--api-key-file <path>`.
    File(String),
    /// `--api-key-env <name>`. The name, never the value — metaharness passes a variable's name
    /// into an argv and the secret itself never enters this process.
    Environment(String),
    /// `--oauth-token-file <path>`, with an optional `--oauth-token-pointer`.
    SubscriptionFile {
        /// The document holding the token.
        path: String,
        /// A JSON pointer to the token inside it. [`None`] means the file *is* the token.
        pointer: Option<String>,
    },
    /// `--oauth-token-env <name>`, with an optional `--oauth-token-pointer`. The name, never the
    /// value.
    SubscriptionEnvironment {
        /// The variable holding the token.
        name: String,
        /// A JSON pointer to the token inside it. [`None`] means the variable *is* the token.
        pointer: Option<String>,
    },
}

/// One launch of `b10x-harness run`.
#[derive(Debug, Clone)]
pub struct B10xLaunch {
    /// The binary, on `PATH` or by path.
    pub program: String,
    /// The endpoint origin plus API prefix.
    pub base_url: String,
    /// The model the endpoint serves.
    pub model: String,
    /// Where the loop reads its bearer.
    ///
    /// [`None`] launches it with no credential at all, which is what a run declared
    /// `--credentials none` is: the request goes out with no `authorization` header and the far
    /// end decides. Right for a gateway on this machine, and right for a run whose first request
    /// is meant to be refused.
    ///
    /// Either way the credential is **named and never ambient**, and metaharness never holds it:
    /// this is a path or a variable name in an argv, not a secret passing through this process.
    pub credential: Option<Credential>,
    /// Which model API to speak. [`None`] leaves the loop on its own default.
    pub wire: Option<Wire>,
    /// The tree the read-only tools may see.
    pub workspace: String,
    /// Where the run may write and execute. [`None`] leaves the catalogue read-only.
    pub confinement: Option<Confinement>,
    /// A delegated cgroup subtree, without which no `run` entry is published.
    pub cgroup_root: Option<String>,
    /// Programs `run` may start. Empty publishes no `run`.
    pub allow_program: Vec<String>,
    /// The operator's hook file, consulted before every call the loop is about to make.
    pub hooks: Option<String>,
    /// Ceiling on model turns.
    pub max_turns: Option<u32>,
    /// A build toolchain the run may read, admitted read-only.
    pub toolchain: Option<String>,
    /// A program on the host the confined run may execute, staged and mounted read-only.
    ///
    /// Separate from [`Self::allow_program`] because they answer different questions: the
    /// allow-list says what a `run` may *name*, and this says what the sandbox *contains*. A path
    /// on the allow-list that the sandbox does not hold is admitted and then dies at `ENOENT`,
    /// which reads to a model as a wrong command rather than a missing file. The loop adds the
    /// mounted path to its own allow-list, so this is the whole declaration.
    pub driver: Option<String>,
    /// Where the run may write, ordered, as `<glob>=<allowed|partial-only|denied>`.
    pub write_scope: Vec<String>,
    /// Files the run is given before it starts, instead of discovering them.
    pub context: Vec<String>,
    /// Bind the tools to the scope without stating it in the instruction.
    pub scope_silent: bool,
    /// A rate card, so the run's record states what it cost.
    ///
    /// The one thing on this launch that has no counterpart on the vendor adapters, and the
    /// asymmetry is real rather than an omission: Claude Code and codex are priced by a catalogue
    /// their own service delivers, and this loop is priced by rates somebody declared. Both
    /// figures are the harness's own; only one of them needs a file.
    pub prices: Option<String>,
    /// The request.
    pub input: String,
}

impl B10xLaunch {
    /// A launch against one endpoint, with the read-only toolset only and no credential.
    ///
    /// The credential is added by [`Self::authenticated`] rather than taken here, so that a
    /// launch reaching an endpoint unauthenticated is something the caller wrote down.
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        workspace: impl AsRef<Path>,
        input: impl Into<String>,
    ) -> Self {
        Self {
            program: "b10x-harness".to_owned(),
            base_url: base_url.into(),
            model: model.into(),
            credential: None,
            wire: None,
            workspace: workspace.as_ref().display().to_string(),
            confinement: None,
            cgroup_root: None,
            allow_program: Vec::new(),
            hooks: None,
            max_turns: None,
            toolchain: None,
            driver: None,
            write_scope: Vec::new(),
            context: Vec::new(),
            scope_silent: false,
            prices: None,
            input: input.into(),
        }
    }

    /// The same launch, reading its bearer from this file.
    #[must_use]
    pub fn authenticated(mut self, api_key_file: impl AsRef<Path>) -> Self {
        self.credential = Some(Credential::File(
            api_key_file.as_ref().display().to_string(),
        ));
        self
    }

    /// The same launch, reading its bearer from this environment variable.
    #[must_use]
    pub fn from_environment(mut self, variable: impl Into<String>) -> Self {
        self.credential = Some(Credential::Environment(variable.into()));
        self
    }

    /// The same launch, reading a subscription token from this file, optionally at a pointer.
    ///
    /// Distinct from [`Self::authenticated`] rather than a flag on it: an API key and a
    /// subscription token are presented under different header names, and a launch that confused
    /// them produces a 401 naming authentication and not the header, which is the hardest kind of
    /// failure to read.
    #[must_use]
    pub fn with_subscription_file(
        mut self,
        path: impl AsRef<Path>,
        pointer: Option<String>,
    ) -> Self {
        self.credential = Some(Credential::SubscriptionFile {
            path: path.as_ref().display().to_string(),
            pointer,
        });
        self
    }

    /// The same launch, reading a subscription token from this variable, optionally at a pointer.
    #[must_use]
    pub fn with_subscription_env(
        mut self,
        name: impl Into<String>,
        pointer: Option<String>,
    ) -> Self {
        self.credential = Some(Credential::SubscriptionEnvironment {
            name: name.into(),
            pointer,
        });
        self
    }

    /// The same launch, speaking this model API.
    #[must_use]
    pub fn speaking(mut self, wire: Wire) -> Self {
        self.wire = Some(wire);
        self
    }

    /// The same launch, with a build toolchain admitted read-only.
    #[must_use]
    pub fn with_toolchain(mut self, name: impl Into<String>) -> Self {
        self.toolchain = Some(name.into());
        self
    }

    /// The same launch, with one more rule about where it may write.
    ///
    /// Ordered: the first rule whose glob matches decides, so the order they are added in is the
    /// order they are read in.
    #[must_use]
    pub fn with_write_scope(mut self, rule: impl Into<String>) -> Self {
        self.write_scope.push(rule.into());
        self
    }

    /// The same launch, binding the tools to its scope without stating it in the instruction.
    #[must_use]
    pub fn with_scope_silent(mut self) -> Self {
        self.scope_silent = true;
        self
    }

    /// The same launch, with one more file it is given rather than has to find.
    #[must_use]
    pub fn with_context(mut self, file: impl AsRef<Path>) -> Self {
        self.context.push(file.as_ref().display().to_string());
        self
    }

    /// The same launch, priced at the rates in this card.
    #[must_use]
    pub fn with_prices(mut self, card: impl AsRef<Path>) -> Self {
        self.prices = Some(card.as_ref().display().to_string());
        self
    }

    /// The same launch, confined by substrate's daemon, with the programs it may start.
    #[must_use]
    pub fn confined(
        mut self,
        socket: impl AsRef<Path>,
        programs: impl IntoIterator<Item = String>,
    ) -> Self {
        self.confinement = Some(Confinement::Daemon(socket.as_ref().display().to_string()));
        self.allow_program = programs.into_iter().collect();
        self
    }

    /// The same launch, confined by a driver in the loop's own process.
    #[must_use]
    pub fn confined_in_process(
        mut self,
        root: impl AsRef<Path>,
        programs: impl IntoIterator<Item = String>,
    ) -> Self {
        self.confinement = Some(Confinement::Embedded(root.as_ref().display().to_string()));
        self.allow_program = programs.into_iter().collect();
        self
    }

    /// The cgroup subtree a confined run may start a process inside.
    #[must_use]
    pub fn with_cgroup_root(mut self, root: impl AsRef<Path>) -> Self {
        self.cgroup_root = Some(root.as_ref().display().to_string());
        self
    }

    /// The operator's hook file, consulted before every call.
    #[must_use]
    pub fn with_hooks(mut self, path: impl Into<String>) -> Self {
        self.hooks = Some(path.into());
        self
    }

    /// The same launch, with one host program staged so a confined `run` can start it.
    #[must_use]
    pub fn with_driver(mut self, path: impl Into<String>) -> Self {
        self.driver = Some(path.into());
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
/// Every long flag [`argv`] can emit, and whether it carries a value.
///
/// **Derived from a launch with every option set, never listed by hand.** A hand-written list is a
/// second statement of the same fact, and the two drift the first time a flag is added to `argv`
/// alone — which is exactly the failure this exists to detect, arrived at from the other side.
///
/// The point of it is that [`crate::PINNED_VERSIONS`] is not a pin. `b10x-harness --version`
/// answered `0.1.0` on 2026-08-29 both before and after `--substrate-embedded` stopped taking a
/// value, so doctor blessed a binary that rejected the adapter's own argv. A version string the
/// producer never bumps proves nothing about the interface; the flags do.
#[must_use]
pub fn emitted_flags() -> Vec<(String, bool)> {
    // Values chosen so none of them can be mistaken for a flag by the walk below.
    let mut launch = B10xLaunch::new("https://endpoint.invalid/v1", "a-model", "/w", "a request")
        .with_subscription_file("/a/token.json", Some("/at/a/pointer".to_owned()))
        .speaking(Wire::AnthropicMessages)
        .with_toolchain("a-toolchain")
        .with_write_scope("a/glob=allowed")
        .with_scope_silent()
        .with_context("a/file")
        .with_prices("a/card.toml")
        .with_max_turns(1)
        .confined_in_process("/root", ["a-program".to_owned()])
        .with_cgroup_root("/a/subtree");
    // The daemon socket and the API-key routes are the alternatives to what is set above, and a
    // single launch cannot hold both halves of a choice. Their flags are collected from a second
    // launch and merged, so the surface is every flag `argv` *can* emit rather than every flag one
    // particular launch does.
    let other = B10xLaunch::new("https://endpoint.invalid/v1", "a-model", "/w", "a request")
        .from_environment("A_VARIABLE")
        .confined("/a/socket", ["a-program".to_owned()]);
    let mut flags: Vec<(String, bool)> = Vec::new();
    for argv in [argv(&launch), argv(&other), {
        launch = launch.authenticated("/a/key");
        argv(&launch)
    }] {
        let mut it = argv.iter().peekable();
        while let Some(word) = it.next() {
            let Some(name) = word.strip_prefix("--").filter(|rest| !rest.is_empty()) else {
                continue;
            };
            let takes_value = it.peek().is_some_and(|next| !next.starts_with("--"));
            let name = format!("--{name}");
            if !flags.iter().any(|(seen, _)| seen == &name) {
                flags.push((name, takes_value));
            }
        }
    }
    flags.sort();
    flags
}

/// The pointer flag, which the loop accepts only alongside a subscription source.
fn push_pointer(argv: &mut Vec<String>, pointer: Option<&String>) {
    if let Some(pointer) = pointer {
        argv.push("--oauth-token-pointer".to_owned());
        argv.push(pointer.clone());
    }
}

#[must_use]
pub fn argv(launch: &B10xLaunch) -> Vec<String> {
    let mut argv = vec![
        launch.program.clone(),
        "run".to_owned(),
        "--base-url".to_owned(),
        launch.base_url.clone(),
        "--model".to_owned(),
        launch.model.clone(),
        "--workspace".to_owned(),
        launch.workspace.clone(),
        "--json".to_owned(),
        // **No session file.** The loop's own flag documentation writes this arm's case out: *"for
        // an evaluation arm that must leave nothing on the machine it ran on … an arm whose runs
        // must be identically reproducible from their flags cannot have one of them silently pick
        // up the previous one's state."* A driven step is exactly that — one launch per step, and
        // the comparison between arms is worthless if step two starts from step one's leftovers.
        //
        // It is also what makes the launch survive a child environment with no `HOME`: the loop
        // keeps sessions under `XDG_STATE_HOME` or `HOME`, and this adapter constructs the child's
        // environment rather than inheriting one (H3), so a run that wanted a session directory
        // refused before reaching a model — `neither XDG_STATE_HOME nor HOME is set`, with no
        // terminal record, which the driver above reads as `metaharness exited 3`.
        "--no-session".to_owned(),
        // **The ceiling, not `--yes`, and the difference is a whole tier of enforcement.**
        //
        // This said `--yes` — approve everything — and the reason given was that there is "no
        // decision seam on this arm for a ceiling to consult". That was true and is not: the loop
        // consults hook programs before each call, and `--hooks` below carries the operator's. So
        // the arm now has what the vendor arms have, and `--yes` would throw it away: `--yes`
        // approves every call *including* the destructive ones above `high`, and it does not
        // combine with a ceiling, so it silently made the ceiling moot.
        //
        // `high` is chosen and not maximal: `file_write` and `file_edit` are `medium` and `run` is
        // `high`, so every entry a driven step legitimately uses runs unasked and nothing stalls
        // waiting for a terminal this child does not have. Above `high` there is only the
        // destructive class, which no driven step should reach — and if one does, the default
        // approver has no terminal, denies, and says so to the model rather than doing it.
        "--approve-up-to".to_owned(),
        "high".to_owned(),
    ];
    // Named and never discovered, which is the loop's own rule: a hook found in the workspace would
    // be a program the repository runs on this machine.
    if let Some(hooks) = &launch.hooks {
        argv.push("--hooks".to_owned());
        argv.push(hooks.clone());
    }
    match &launch.credential {
        Some(Credential::File(path)) => {
            argv.push("--api-key-file".to_owned());
            argv.push(path.clone());
        }
        Some(Credential::Environment(variable)) => {
            argv.push("--api-key-env".to_owned());
            argv.push(variable.clone());
        }
        Some(Credential::SubscriptionFile { path, pointer }) => {
            argv.push("--oauth-token-file".to_owned());
            argv.push(path.clone());
            push_pointer(&mut argv, pointer.as_ref());
        }
        Some(Credential::SubscriptionEnvironment { name, pointer }) => {
            argv.push("--oauth-token-env".to_owned());
            argv.push(name.clone());
            push_pointer(&mut argv, pointer.as_ref());
        }
        None => {}
    }
    if let Some(wire) = launch.wire {
        argv.push("--wire".to_owned());
        argv.push(wire.as_str().to_owned());
    }
    if let Some(name) = &launch.toolchain {
        argv.push("--toolchain".to_owned());
        argv.push(name.clone());
    }
    if let Some(program) = &launch.driver {
        argv.push("--driver".to_owned());
        argv.push(program.clone());
    }
    for rule in &launch.write_scope {
        argv.push("--write-scope".to_owned());
        argv.push(rule.clone());
    }
    if launch.scope_silent {
        argv.push("--scope-announce".to_owned());
        argv.push("silent".to_owned());
    }
    for file in &launch.context {
        argv.push("--context".to_owned());
        argv.push(file.clone());
    }
    if let Some(card) = &launch.prices {
        argv.push("--prices".to_owned());
        argv.push(card.clone());
    }
    match &launch.confinement {
        Some(Confinement::Daemon(socket)) => {
            argv.push("--substrate".to_owned());
            argv.push(socket.clone());
        }
        // The root is deliberately not pushed: the loop takes this flag bare and serves the
        // workspace's parent, which is the same tree.
        Some(Confinement::Embedded(_)) => argv.push("--substrate-embedded".to_owned()),
        None => {}
    }
    if let Some(root) = &launch.cgroup_root {
        argv.push("--cgroup-root".to_owned());
        argv.push(root.clone());
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
            "/work",
            "do the thing",
        )
        .authenticated("/run/secrets/key")
    }

    fn value_after(argv: &[String], flag: &str) -> Option<String> {
        argv.iter()
            .position(|word| word == flag)
            .and_then(|at| argv.get(at + 1))
            .cloned()
    }

    #[test]
    fn a_launch_is_always_observable_because_the_record_is_not_optional() {
        let argv = argv(&launch());
        assert!(argv.contains(&"--json".to_owned()), "{argv:?}");
    }

    #[test]
    fn the_credential_is_named_and_never_left_to_the_environment() {
        let argv = argv(&launch());
        let at = argv
            .iter()
            .position(|word| word == "--api-key-file")
            .expect("named");
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
        assert!(
            !argv.iter().any(|word| word == "--allow-program"),
            "{argv:?}"
        );
    }

    #[test]
    fn a_confined_launch_names_the_socket_and_every_program_separately() {
        let argv = argv(&launch().confined(
            "/run/substrate.sock",
            ["cargo".to_owned(), "protocol".to_owned()],
        ));
        let at = argv
            .iter()
            .position(|word| word == "--substrate")
            .expect("named");
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
    fn a_driver_travels_as_a_mount_and_not_as_one_more_allow_listed_name() {
        // The two are different questions and were confused for a whole eval run: the allow-list
        // says what a `run` may *name*, and only a mount says what the sandbox *contains*. A
        // driven run allow-listed its CLI by absolute host path, was admitted, found nothing —
        // the sandbox binds `/usr`, `/bin`, `/lib`, `/lib64` and the workspace — and hand-wrote
        // the planning store instead of using the CLI it could not start.
        let argv = argv(&launch().with_driver("/home/op/src/target/release/protocol"));
        let at = argv
            .iter()
            .position(|word| word == "--driver")
            .expect("the driver is declared");
        assert_eq!(argv[at + 1], "/home/op/src/target/release/protocol");
        assert!(
            !argv.iter().any(|word| word == "--allow-program"),
            "declaring a driver is not declaring a program: the loop adds the mounted path to its \
             own allowlist, and a caller doing it again would be naming the host path, which is \
             the spelling that does not resolve inside: {argv:?}"
        );
    }

    #[test]
    fn the_request_is_last_because_it_is_the_one_argument_that_can_look_like_a_flag() {
        let argv = argv(&launch().with_max_turns(8));
        assert_eq!(argv[argv.len() - 2], "--input");
        assert_eq!(argv[argv.len() - 1], "do the thing");
    }

    #[test]
    fn a_launch_with_no_credential_names_no_key_file_rather_than_an_empty_one() {
        // `--credentials none`: the request goes out unauthenticated and the far end decides. An
        // empty `--api-key-file ""` would be refused by the loop itself, which is a different
        // outcome from the one the operator asked for.
        let argv = argv(&B10xLaunch::new(
            "https://gw.example/v1",
            "m",
            "/work",
            "hi",
        ));
        assert!(
            !argv.iter().any(|word| word == "--api-key-file"),
            "{argv:?}"
        );
    }

    #[test]
    fn a_priced_launch_carries_the_card_so_the_record_states_what_the_run_cost() {
        // Without this the b10x arm reaches the matrix with `total_cost_usd: null` beside arms
        // that state a figure, and the one axis an evaluation programme compares on is missing
        // for exactly one cell.
        let priced = argv(&launch().with_prices("/etc/rates.json"));
        assert_eq!(
            value_after(&priced, "--prices"),
            Some("/etc/rates.json".to_owned())
        );
        assert!(
            !argv(&launch()).iter().any(|word| word == "--prices"),
            "and a launch nobody priced names no card"
        );
    }

    #[test]
    fn a_write_scope_reaches_the_argv_in_the_order_it_was_declared() {
        let scoped = argv(
            &launch()
                .with_write_scope(".engineering/planning/**=partial-only")
                .with_write_scope("**=allowed"),
        );
        let rules: Vec<&String> = scoped
            .iter()
            .zip(scoped.iter().skip(1))
            .filter(|(flag, _)| *flag == "--write-scope")
            .map(|(_, rule)| rule)
            .collect();
        // First match wins downstream, so the order is the declaration and not a set.
        assert_eq!(
            rules,
            vec![".engineering/planning/**=partial-only", "**=allowed"]
        );
        assert!(
            !argv(&launch()).iter().any(|word| word == "--write-scope"),
            "a scope nobody declared bounds nothing"
        );
    }

    #[test]
    fn the_experiment_control_reaches_the_argv_only_when_it_was_asked_for() {
        let silent = argv(&launch().with_scope_silent());
        assert_eq!(
            value_after(&silent, "--scope-announce"),
            Some("silent".to_owned())
        );
        assert!(
            !argv(&launch())
                .iter()
                .any(|word| word == "--scope-announce"),
            "a real run states its scope, because being refused costs a call"
        );
    }

    #[test]
    fn every_context_file_reaches_the_argv_because_a_partial_context_is_not_reproducible() {
        let seeded = argv(
            &launch()
                .with_context("/a/SKILL.md")
                .with_context("/b/api.rs"),
        );
        let files: Vec<&String> = seeded
            .iter()
            .zip(seeded.iter().skip(1))
            .filter(|(flag, _)| *flag == "--context")
            .map(|(_, file)| file)
            .collect();
        assert_eq!(files, vec!["/a/SKILL.md", "/b/api.rs"]);
    }
}

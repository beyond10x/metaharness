# Prompted native read-only scoper, 2026-10-03

The opt-in Rust fixture `actual_b10x_prompted_read_only_scoper_preserves_role_and_file_evidence`
extends the owned native b10x Responses fixture. It runs the actual installed
0.13.3 through `metaharness run b10x --strict-version`, with a plain prompt naming
an AEP read-only scoper role. The first actual provider request must retain that
role instruction; both requests must offer exactly file_read, dir_list, find and
search. A real native file_read must return the fixture's owned bytes and replay
them to the provider. The normalized call remains observe-only and the terminal
and native process status must indicate success, with observed version 0.13.3.

This is a prompted role observation. It proves neither named-agent/delegate
loading nor governed AEP artifact handling, authorization, state transitions or
phase admission. The fixture supplies deterministic model responses and synthetic
usage; it does not observe hosted model judgment or real monetary consumption.
Fresh private HOME/workspace, no credentials, and a loopback-only network
namespace are inherited from the native fixture. Strict admission binds this
observation to the production driver's selected compatibility pin. The source
release/revision pairing must be verified independently by the release coordinator.

Use the existing native fixture's absolute binary/evidence environment variables
and select this test with `--ignored --exact`. It requires the combined production
CLI carrying the deliberately qualified 0.13.3 pin; the earlier 0.12.1-pinned
binary correctly refuses it before any request. The source commit precedes the
native run, so no unexecuted scenario is claimed green here.

The adjacent containment fixture's refusal branch requires a withholding item
specifically naming `run` with a nonblank reason. An unrelated withheld tool is
insufficient evidence for process withholding.

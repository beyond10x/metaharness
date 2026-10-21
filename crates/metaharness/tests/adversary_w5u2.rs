//! Adversary pass 1, wave 5 unit 2 (`story:hook-program-quotes-channel-paths`, amendment a37).
//!
//! a37 § 3, as reworded after this pass (coordinator decision F2): the refusal names the path
//! through Rust's `escape_ascii`. Printable ASCII is written as it is, except `'`, `"` and `\`,
//! which carry a backslash in front; `\t`, `\r` and `\n` are written as those escapes; every other
//! byte is `\xNN`. Escaping `\` keeps the rendering unambiguous. No process is started.
#![cfg(unix)]

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

use metaharness::{Refusal, SCRATCH_PATH_NOT_UTF8};

#[test]
fn adversary_w5u2_the_refusal_names_the_path_through_escape_ascii() {
    let path = PathBuf::from(OsStr::from_bytes(b"/s/it's \"q\" a\\b\t-\xff/.tmpAbC123"));
    let message = Refusal::ScratchPathNotUtf8 { path }.to_string();
    assert!(message.starts_with(SCRATCH_PATH_NOT_UTF8), "{message}");
    assert!(
        message.contains(r#"/s/it\'s \"q\" a\\b\t-\xff/.tmpAbC123"#),
        "the path is not rendered by escape_ascii's rules: {message}"
    );
}

//! The web app behaves like the desktop app: every command the UI invokes is run by diffity-server, or is one
//! of the few about the server's desktop (`NOT_IN_WEB`). When upstream adds a command, this fails until it is
//! listed in src/server/handler.rs.
//!
//! Run with `cargo test --no-default-features --features server --test web_commands`.
#![cfg(not(feature = "app"))]

use std::collections::BTreeSet;
use std::path::Path;

use diffity_desktop_lib::server::{command_names, NOT_IN_WEB};

#[test]
fn every_command_the_ui_invokes_runs_on_the_server_or_is_left_out_on_purpose() {
    let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src");
    let invoked = invoked_commands(&ui);
    assert!(invoked.len() > 100, "found only {} commands in {}: is the pattern stale?", invoked.len(), ui.display());

    let served: BTreeSet<&str> = command_names().into_iter().collect();
    let missing: Vec<&String> = invoked.iter().filter(|name| !served.contains(name.as_str()) && !NOT_IN_WEB.contains(&name.as_str())).collect();
    assert!(missing.is_empty(), "the UI invokes commands diffity-server does not run; add them to src/server/handler.rs: {missing:?}");

    let both: Vec<&&str> = NOT_IN_WEB.iter().filter(|name| served.contains(**name)).collect();
    assert!(both.is_empty(), "served and listed as not in the web at once: {both:?}");
}

/// `invoke('name'` and `invoke<Type>('name'` in the UI's TypeScript.
fn invoked_commands(dir: &Path) -> BTreeSet<String> {
    let pattern = regex::Regex::new(r"invoke(?:<[^(]*>)?\(\s*'([a-z_]+)'").unwrap();
    let mut names = BTreeSet::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            names.extend(invoked_commands(&path));
        } else if path.extension().is_some_and(|ext| ext == "ts" || ext == "tsx") {
            let text = std::fs::read_to_string(&path).unwrap();
            names.extend(pattern.captures_iter(&text).map(|c| c[1].to_string()));
        }
    }
    names
}

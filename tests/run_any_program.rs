// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Pass 2 is any program the programmer names (the user's ruling,
//! 2026-10-12, 0.8.0): `hrs -- <program> [args]` transpiles, then runs the
//! program in the Rust project, `export/` (`target/src/` before 0.9.0),
//! found as a shell finds it. Harsh arbitrates no
//! conflict between two programs of one name -- Dioxus's `dx` and Deno's --
//! it shows them: `hrs which`, and a note when PATH holds several.
//! `hrs dx` is retired.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn hrs() -> &'static str {
    env!("CARGO_BIN_EXE_hrs")
}

/// A scratch project, and two fake programs named `dx` in two folders:
/// the first says it is Deno's, the second Dioxus's.
fn setup(tag: &str) -> (PathBuf, String) {
    let d = std::env::temp_dir().join(format!("harsh-run-any-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    for (dir, who) in [("a", "deno 2.0"), ("b", "dioxus-cli 0.7.10")] {
        std::fs::create_dir_all(d.join(dir)).unwrap();
        let p = d.join(dir).join("dx");
        std::fs::write(&p, format!("#!/bin/sh\necho \"{who}\"\necho \"ran: $* in $(pwd)\"\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    assert!(Command::new(hrs()).args(["new", "app"]).current_dir(&d).output().unwrap().status.success());
    let path = format!("{}:{}:{}", d.join("a").display(), d.join("b").display(), std::env::var("PATH").unwrap());
    (d, path)
}

fn run(dir: &Path, path: &str, args: &[&str]) -> (bool, String, String) {
    let o = Command::new(hrs()).args(args).current_dir(dir).env("PATH", path).output().unwrap();
    (o.status.success(), String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into())
}

#[test]
fn a_program_runs_in_the_generated_project_as_the_shell_finds_it() {
    let (d, path) = setup("dashdash");
    let app = d.join("app");
    let (ok, out, err) = run(&app, &path, &["--", "dx", "serve", "--version"]);
    assert!(ok, "{err}");
    // The first on PATH runs, as in a shell; its arguments are its own, `--version` included.
    assert!(out.contains("deno 2.0") && out.contains("ran: serve --version in") && out.contains("app/export"), "{out}");
    // Two programs of the name: said, never decided.
    assert!(err.contains("2 programs named `dx` on PATH; running") && err.contains("hrs which dx"), "{err}");
    // A path is the programmer's choice, resolved from where `hrs` runs.
    let (ok, out, _) = run(&app, &path, &["--", "../b/dx", "serve"]);
    assert!(ok && out.contains("dioxus-cli 0.7.10") && out.contains("app/export"), "{out}");
}

#[test]
fn which_lists_every_program_of_the_name_in_order() {
    let (d, path) = setup("which");
    let (ok, out, _) = run(&d, &path, &["which", "dx"]);
    assert!(ok, "{out}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert!(lines[0].contains("/a/dx") && lines[0].contains("deno 2.0") && lines[0].contains("<- runs"), "{out}");
    assert!(lines[1].contains("/b/dx") && lines[1].contains("dioxus-cli 0.7.10") && !lines[1].contains("<- runs"), "{out}");
}

#[test]
fn hrs_dx_is_retired_and_names_its_replacement() {
    let (d, path) = setup("retired");
    let (ok, out, err) = run(&d.join("app"), &path, &["dx", "serve"]);
    assert!(!ok && out.is_empty(), "{out}");
    assert!(err.contains("`hrs dx` is retired") && err.contains("hrs -- dx serve"), "{err}");
}

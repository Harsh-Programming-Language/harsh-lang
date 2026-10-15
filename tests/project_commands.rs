// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The project commands decided 2026-10-04: `hrs new [--lib]`, `hrs new
//! --help`, `hrs publish` (checks, then stops until the registry opens).

use std::path::PathBuf;
use std::process::Command;

fn hrs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hrs"))
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("harsh-projcmd-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn new_help_prints_usage_and_creates_nothing() {
    let d = scratch("help");
    let out = hrs().args(["new", "--help"]).current_dir(&d).output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("usage: hrs new [--lib] <name>"));
    assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0, "a project was created");
    let out = hrs().args(["new", "-x"]).current_dir(&d).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot start with `-`"));
    assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
}

#[test]
fn new_lib_makes_a_library_either_way_round() {
    for (args, name) in [(["new", "--lib", "geo"], "geo"), (["new", "geo2", "--lib"], "geo2")] {
        let d = scratch(name);
        assert!(hrs().args(args).current_dir(&d).status().unwrap().success());
        let root = d.join(name);
        let cargo = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        // 0.9.0: no target paths; cargo finds `export/src/lib.rs` itself.
        assert!(!cargo.contains("path =") && !cargo.contains("[[bin]]"), "{cargo}");
        let lib = std::fs::read_to_string(root.join("src/lib.hrs")).unwrap();
        assert!(lib.contains("pub fn add") && lib.contains("#[test]"), "{lib}");
        assert!(!root.join("src/main.hrs").exists());
        assert!(root.join("Hrs.toml").exists());
    }
}

#[test]
fn publish_checks_the_package_then_stops() {
    let d = scratch("publish");
    assert!(hrs().args(["new", "--lib", "geo"]).current_dir(&d).status().unwrap().success());
    let root = d.join("geo");
    std::fs::write(root.join("Hrs.toml"), "[dependencies]\nhrs_std = \"0.1\"\nmine = { path = \"../mine\" }\n").unwrap();
    let out = hrs().arg("publish").current_dir(&root).output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(err.contains("no `description`") && err.contains("no `license`"), "{err}");
    assert!(err.contains("`mine` is a path"), "{err}");
    assert!(err.contains("registry is not open yet"), "{err}");
}

/// `hrs` at a workspace's root, without a `[package]` (2026-10-13; it was
/// refused as a project with no target under `target/src/`): `check`,
/// `test` and `fmt` run in each member in `members`' order -- a `*` expanded,
/// `exclude` left out -- a Harsh member by `hrs`, a Rust member by cargo,
/// which `fmt` skips; `run` acts on one program and is refused, naming them.
#[test]
fn a_workspace_root_runs_each_member() {
    let d = scratch("workspace");
    assert!(hrs().args(["new", "app"]).current_dir(&d).status().unwrap().success());
    std::fs::create_dir_all(d.join("crates")).unwrap();
    assert!(hrs().args(["new", "--lib", "util"]).current_dir(d.join("crates")).status().unwrap().success());
    assert!(hrs().args(["new", "--lib", "old"]).current_dir(d.join("crates")).status().unwrap().success());
    let rusty = d.join("rusty");
    std::fs::create_dir_all(rusty.join("src")).unwrap();
    std::fs::write(rusty.join("Cargo.toml"), "[package]\nname = \"rusty\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::write(rusty.join("src/lib.rs"), "pub fn one() -> i32 { 1 }\n").unwrap();
    std::fs::write(
        d.join("Cargo.toml"),
        "[workspace]\nmembers = [\n    \"app\",\n    \"crates/*\",\n    \"rusty\",\n]\nexclude = [\"./crates/old/\"]  # a comment, \"#\" in a string: [workspace]\nresolver = \"2\"\n",
    )
    .unwrap();
    let members = harsh_lang::driver::workspace_members(&d).unwrap();
    let names: Vec<_> = members.iter().map(|m| m.strip_prefix(&d).unwrap().display().to_string()).collect();
    assert_eq!(names, ["app", "crates/util", "rusty"]);
    assert!(harsh_lang::driver::workspace_members(&d.join("app")).is_none());

    let out = hrs().arg("test").current_dir(&d).output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{err}");
    let order: Vec<_> = err.lines().filter(|l| l.starts_with("hrs: workspace member")).collect();
    assert_eq!(order, ["hrs: workspace member app", "hrs: workspace member crates/util", "hrs: workspace member rusty (Rust, by cargo)"], "{err}");
    assert!(!d.join("crates/old/export").exists() && !d.join("export/crates/old").exists(), "an excluded member was built");
    // 0.9.0, the user's option A (2026-10-14): one `export/` at the root is
    // the Rust workspace -- each Harsh member transpiled at its place, the
    // Rust member mirrored, one `target/` -- and `hrs`'s files in `.hrs/`.
    let ex = d.join("export");
    assert!(ex.join("Cargo.toml").is_file() && ex.join("target").is_dir());
    assert!(ex.join("app/src/main.rs").is_file() && ex.join("crates/util/src/lib.rs").is_file());
    assert!(ex.join("rusty/src/lib.rs").is_file() && !ex.join("crates/old").exists());
    assert!(!d.join("app/export").exists() && !d.join("app/target").exists());
    assert!(d.join(".hrs/members/app/maps/main.map.json").is_file());
    let out = hrs().args(["-c", "build", "--quiet"]).current_dir(&d).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    let out = hrs().args(["fmt", "--check"]).current_dir(&d).output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success() && !err.contains("rusty"), "{err}");

    let out = hrs().arg("run").current_dir(&d).output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success() && err.contains("run it in a member's folder (app, crates/util, rusty)"), "{err}");
}

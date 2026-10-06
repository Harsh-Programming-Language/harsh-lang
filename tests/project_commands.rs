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
        assert!(cargo.contains("[lib]\npath = \"target/src/lib.rs\"") && !cargo.contains("[[bin]]"), "{cargo}");
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

//! 0.9.0's layout (the user's design, 2026-10-14): `export/` at the
//! project's root is the whole Rust project -- its `Cargo.toml`, `src/`
//! transpiled, the project's other files mirrored, cargo's `target/` --
//! and `hrs`'s own files are in `.hrs/`. `hrs -c <cmd>` is
//! `cd export && cargo <cmd>`; `hrs cargo` and `hrs export` are retired;
//! `hrs migrate` moves an older project.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("harsh-export-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn hrs(args: &[&str], dir: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_hrs")).args(args).current_dir(dir).output().unwrap();
    (out.status.success(), format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)))
}

/// `hrs new` names no generated path, and ignores what `hrs` writes; a run
/// leaves the Rust in `export/`, the map in `.hrs/`, and no `target/` at
/// the root.
#[test]
fn a_new_project_builds_into_export() {
    let d = scratch("new");
    assert!(hrs(&["new", "app"], &d).0);
    let app = d.join("app");
    let cargo = fs::read_to_string(app.join("Cargo.toml")).unwrap();
    assert!(!cargo.contains("path =") && !cargo.contains("target/"), "{cargo}");
    assert_eq!(fs::read_to_string(app.join(".gitignore")).unwrap(), "/target\n/export\n/.hrs\n");
    let (ok, out) = hrs(&["run"], &app);
    assert!(ok && out.contains("Hello"), "{out}");
    assert!(app.join("export/Cargo.toml").is_file() && app.join("export/src/main.rs").is_file());
    assert!(app.join("export/target").is_dir() && !app.join("target").exists());
    assert!(app.join(".hrs/maps/main.map.json").is_file() && !app.join("export/src/main.map.json").exists());
    let m = fs::read_to_string(app.join("export/Cargo.toml")).unwrap();
    assert!(m.trim_end().ends_with("[workspace]") && !m.contains("Harsh"), "{m}");
    let _ = fs::remove_dir_all(&d);
}

/// The project's other files are mirrored into `export/` -- hidden ones too,
/// but not `.git` -- and so are `src/`'s files that are not Harsh, beside
/// the transpiled ones, so an `include_str!` path means in `export/src/`
/// what it means in `src/`.
#[test]
fn the_rest_of_the_project_is_mirrored() {
    let d = scratch("mirror");
    assert!(hrs(&["new", "app"], &d).0);
    let app = d.join("app");
    fs::create_dir_all(app.join("assets")).unwrap();
    fs::create_dir_all(app.join(".git")).unwrap();
    fs::write(app.join("assets/a.txt"), "from assets").unwrap();
    fs::write(app.join(".env"), "X=1").unwrap();
    fs::write(app.join("src/b.txt"), " and src").unwrap();
    fs::write(app.join("src/main.hrs"), "fn main$:\n    print! \"{}{}\" (include_str! \"../assets/a.txt\") (include_str! \"b.txt\")\n").unwrap();
    let (ok, out) = hrs(&["run"], &app);
    assert!(ok && out.contains("from assets and src"), "{out}");
    let ex = app.join("export");
    assert!(ex.join("assets/a.txt").is_file() && ex.join(".env").is_file() && ex.join("src/b.txt").is_file());
    assert!(!ex.join(".git").exists() && !ex.join(".hrs").exists() && !ex.join("src/main.hrs").exists());
    let rust = fs::read_to_string(ex.join("src/main.rs")).unwrap();
    assert!(rust.contains("include_str!(\"../assets/a.txt\")") && rust.contains("include_str!(\"b.txt\")"), "{rust}");
    let _ = fs::remove_dir_all(&d);
}

/// `hrs -c <cmd>` is cargo in `export/`, after pass 1; alone, it says what
/// it wants. The retired commands name their replacements.
#[test]
fn dash_c_is_cargo_in_export_and_the_old_commands_are_retired() {
    let d = scratch("dashc");
    assert!(hrs(&["new", "app"], &d).0);
    let app = d.join("app");
    let (ok, out) = hrs(&["-c", "read-manifest"], &app);
    assert!(ok && (out.contains("export/Cargo.toml") || out.contains("export\\\\Cargo.toml")), "{out}");
    let (ok, out) = hrs(&["-c"], &app);
    assert!(!ok && out.contains("hrs -c"), "{out}");
    let (ok, out) = hrs(&["cargo", "tree"], &app);
    assert!(!ok && out.contains("retired (0.9.0)") && out.contains("hrs -c tree"), "{out}");
    let (ok, out) = hrs(&["export", "out"], &app);
    assert!(!ok && out.contains("retired (0.9.0)") && out.contains("hrs -c publish"), "{out}");
    let _ = fs::remove_dir_all(&d);
}

/// `hrs migrate` moves a 0.8 project: its targets out of `target/src/`,
/// `/export` and `/.hrs` into its `.gitignore`, the old `target/` gone;
/// until then `hrs` refuses it, naming the command. At a workspace's root
/// it does each Harsh member.
#[test]
fn migrate_moves_a_project_and_a_workspace() {
    let d = scratch("migrate");
    let old = |dir: &Path| {
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("target/src")).unwrap();
        fs::write(dir.join("target/src/.hrs-stamp"), "0.8.5").unwrap();
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"a\"\npath = \"target/src/main.rs\"\n").unwrap();
        fs::write(dir.join("src/main.hrs"), "fn main$:\n    println! \"moved\"\n").unwrap();
        fs::write(dir.join(".gitignore"), "/target\n").unwrap();
    };
    let a = d.join("a");
    old(&a);
    let (ok, out) = hrs(&["build"], &a);
    assert!(!ok && out.contains("hrs migrate"), "{out}");
    let (ok, out) = hrs(&["migrate"], &a);
    assert!(ok, "{out}");
    assert!(fs::read_to_string(a.join("Cargo.toml")).unwrap().contains("path = \"src/main.rs\""));
    assert_eq!(fs::read_to_string(a.join(".gitignore")).unwrap(), "/target\n/export\n/.hrs\n");
    assert!(!a.join("target").exists());
    let (ok, out) = hrs(&["run"], &a);
    assert!(ok && out.contains("moved"), "{out}");

    let w = d.join("ws");
    old(&w.join("m"));
    fs::write(w.join("Cargo.toml"), "[workspace]\nmembers = [\"m\"]\n").unwrap();
    fs::write(w.join(".gitignore"), "/target\n").unwrap();
    let (ok, out) = hrs(&["migrate"], &w);
    assert!(ok && out.contains("workspace member m"), "{out}");
    assert!(!w.join(crate_hrs_toml()).exists(), "a workspace's root has no Hrs.toml");
    assert!(fs::read_to_string(w.join(".gitignore")).unwrap().contains("/export"));
    let (ok, out) = hrs(&["build"], &w);
    assert!(ok, "{out}");
    assert!(w.join("export/m/src/main.rs").is_file() && w.join("export/target").is_dir());
    let _ = fs::remove_dir_all(&d);
}

fn crate_hrs_toml() -> &'static str {
    harsh_lang::driver::HRS_MANIFEST
}

/// `hrs_std` folded in: the transpiler writes `crate::hrs_std::`, and the
/// source map is shifted with each insertion, so a column after one still
/// lands on its Harsh token.
#[test]
fn folding_hrs_std_keeps_the_columns() {
    let d = scratch("fold");
    let hrs_file = d.join("main.hrs");
    let src = "fn main$:\n    let a = (m~ [1 2]) + oops\n";
    fs::write(&hrs_file, src).unwrap();
    let (rs, map) = (d.join("main.rs"), d.join("main.map.json"));
    harsh_lang::driver::set_fold_std(true);
    harsh_lang::driver::transpile_one(src, &hrs_file, &rs, &map).unwrap();
    harsh_lang::driver::set_fold_std(false);
    let rust = fs::read_to_string(&rs).unwrap();
    assert!(rust.contains("crate::hrs_std::Matrix") && !rust.contains(" hrs_std::"), "{rust}");
    let line = rust.lines().position(|l| l.contains("oops")).unwrap();
    let col = rust.lines().nth(line).unwrap().find("oops").unwrap() + 1;
    let m = harsh_lang::remap::SourceMap::load(map.to_str().unwrap()).unwrap();
    assert_eq!(m.locate(line + 1, col), Some((2, 26)));
    let _ = fs::remove_dir_all(&d);
}

/// `hrs run` runs the program where the user is, as `cargo run` does --
/// not in `export/`, where cargo builds it: a relative path is the
/// project's. (`hrs -c run` is cargo in `export/`, literally.)
#[test]
fn a_program_runs_where_the_user_is() {
    let d = scratch("cwd");
    assert!(hrs(&["new", "app"], &d).0);
    let app = d.join("app");
    fs::write(app.join("src/main.hrs"), "fn main$:\n    println! \"cwd={}\" (std.env.current_dir$ <- unwrap$ <- display$)\n").unwrap();
    let (ok, out) = hrs(&["run"], &app);
    let here = app.canonicalize().unwrap();
    assert!(ok && out.contains(&format!("cwd={}\n", here.display())), "{out}");
    let (ok, out) = hrs(&["-c", "run", "--quiet"], &app);
    assert!(ok && out.contains(&format!("cwd={}\n", here.join("export").display())), "{out}");
    let _ = fs::remove_dir_all(&d);
}

/// The folded `hrs_std` leaves with its last use: its folder goes, and so
/// does `mod hrs_std;` from a crate root that was not transpiled again,
/// which would name a folder no longer there. While it is used, nothing of
/// Harsh is in `export/` -- `hrs_std`'s notes on Harsh stay behind (found
/// by the developer book's 3rd edition).
#[test]
fn the_folded_module_goes_with_its_last_use() {
    let d = scratch("unfold");
    assert!(hrs(&["new", "app"], &d).0);
    let app = d.join("app");
    fs::write(app.join("Hrs.toml"), "[dependencies]\nhrs_std = \"0.1\"\n").unwrap();
    fs::write(app.join("src/main.hrs"), "mod util\n\nfn main$:\n    util.show$\n").unwrap();
    fs::write(app.join("src/util.hrs"), "pub fn show$:\n    println! \"{}\" (v~ [1, 2])\n").unwrap();
    let (ok, out) = hrs(&["-c", "read-manifest"], &app);
    assert!(ok, "{out}");
    let main = fs::read_to_string(app.join("export/src/main.rs")).unwrap();
    assert!(main.contains("mod hrs_std;") && app.join("export/src/hrs_std/mod.rs").is_file(), "{main}");
    for e in fs::read_dir(app.join("export/src/hrs_std")).unwrap() {
        let t = fs::read_to_string(e.unwrap().path()).unwrap();
        assert!(!t.contains("Harsh") && !t.contains("//!"), "{t}");
    }
    // Only util.hrs changes; main.rs is not transpiled again.
    fs::write(app.join("src/util.hrs"), "pub fn show$:\n    println! \"{}\" 3\n").unwrap();
    let (ok, out) = hrs(&["-c", "read-manifest"], &app);
    assert!(ok && out.contains("transpiled 1 of 2"), "{out}");
    let main = fs::read_to_string(app.join("export/src/main.rs")).unwrap();
    assert!(!main.contains("hrs_std") && !app.join("export/src/hrs_std").exists(), "{main}");
    let _ = fs::remove_dir_all(&d);
}

/// A workspace whose root is a Rust package, with a Harsh member: the
/// root's own manifest and `src/` are in `export/` too, and the workspace
/// builds there.
#[test]
fn a_rust_root_package_with_a_harsh_member() {
    let w = scratch("rustroot");
    fs::create_dir_all(w.join("src")).unwrap();
    fs::write(w.join("Cargo.toml"), "[package]\nname = \"top\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\nmembers = [\"util\"]\n\n[dependencies]\nutil = { path = \"util\" }\n").unwrap();
    fs::write(w.join("src/main.rs"), "fn main() { println!(\"sum={}\", util::add(2, 3)); }\n").unwrap();
    assert!(hrs(&["new", "--lib", "util"], &w).0);
    let (ok, out) = hrs(&["build"], &w.join("util"));
    assert!(ok, "{out}");
    assert!(w.join("export/Cargo.toml").is_file() && w.join("export/src/main.rs").is_file() && w.join("export/util/src/lib.rs").is_file());
    let out = Command::new("cargo").args(["run", "--quiet"]).current_dir(w.join("export")).output().unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("sum=5"), "{}", String::from_utf8_lossy(&out.stderr));
    let _ = fs::remove_dir_all(&w);
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `hrs` — the Harsh transpiler and build driver.

use harsh_lang::driver::{self, Project};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
usage:
  hrs build [cargo args]     transpile into export/, then cargo build there
  hrs run   [cargo args]     the same, then cargo run (from where you are)
  hrs test  [cargo args]     the same, then cargo test
  hrs check [cargo args]     the same, then cargo check
  hrs lint  [cargo args]     the same, then cargo clippy
  hrs watch [subcommand]     rebuild on every save (default: check)
  hrs -- <program> [args]    transpile, then run any program in export/,
                             found as your shell finds it (hrs -- dx serve)
  hrs -c <command> [args]    transpile, then cargo <command> in export/ --
                             hrs -c publish publishes the Rust to crates.io
  hrs publish                publish to Harsh's registry: the .hrs sources,
                             not export/ (not open yet: checks the package
                             and says how to share today)
  hrs which <program>        every program of that name on PATH, in order,
                             with its --version: the first one is what runs
  hrs new   [--lib] <name>   create a project laid out for Harsh -- a program,
                             or with --lib a library
  hrs add   <crate>..        add Harsh crates to Hrs.toml -- hrs_std and the
                             rest of Harsh's distribution; a Rust crate is
                             added to Cargo.toml with cargo add
  hrs migrate                bring a project up to date: Harsh's crates from
                             Cargo.toml to Hrs.toml (0.3.0); the generated
                             Rust from target/src/ to export/ (0.9.0)
  hrs fmt [--check] [files]  reformat .hrs files in place (default: src/**.hrs);
                             --check lists the files that would change, exit 1
  hrs dist  <dir>            write Harsh's standard distribution (hrs_std,
                             hrs_proc_macro, hrs_quote, hrs_syn) into <dir>,
                             for building outside a Harsh project
  hrs expand <file.hrs>      print the file with its Harsh macros unfolded,
                             as Harsh

  hrs <input.hrs> [-o out.rs] [--map out.map.json]
                             transpile a single file
  hrs [--check] <a.hrs> <b.hrs>..
                             transpile several files together, each beside
                             its .hrs (Harsh files in a Rust crate); --check
                             writes nothing and fails when a .rs is stale

Sources live in src/**.hrs. export/ beside them is the whole Rust project
they make -- its Cargo.toml, src/**.rs, the project's other files, and
cargo's target/ -- for building, for reading, and for crates.io. hrs's own
working files are in .hrs/.

A workspace has one export/, at its root: the whole workspace as Rust.
There (a [workspace] with no [package]), build, check, test, lint and fmt
go through the members in order -- a Harsh member by hrs, a Rust member by
cargo -- and -c is cargo in export/, for the whole workspace.";

fn main() -> ExitCode {
    // `--version` is `hrs`'s own only before `--`: after it, the arguments
    // are the program's (`hrs -- dx --version`).
    if std::env::args().skip(1).take_while(|a| a != "--").any(|a| a == "--version" || a == "-V") {
        println!("hrs {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(first) = args.first() else {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    };
    let rest: Vec<String> = args[1..].to_vec();

    // At a workspace's root, without a `[package]`: each member in turn
    // (2026-10-13; it was "no target under target/src/").
    if let Some(code) = at_workspace_root(first, &rest) {
        return code;
    }

    match first.as_str() {
        "-h" | "--help" | "help" => {
            println!("{}", USAGE);
            ExitCode::SUCCESS
        }
        "build" | "run" | "test" | "check" => cargo_cmd(first, &rest),
        "lint" => cargo_cmd("clippy", &rest),
        // `hrs -c <command> [args]`: transpile, then `cargo <command>` in
        // `export/` -- `hrs -c publish` for crates.io (the user, 2026-10-14).
        "-c" if !rest.is_empty() => c_cmd(&rest),
        "-c" => {
            eprintln!("hrs: `hrs -c` runs cargo in export/ after transpiling: hrs -c <command> [args], e.g. hrs -c publish");
            ExitCode::from(2)
        }
        "cargo" => {
            eprintln!("hrs: `hrs cargo` is retired (0.9.0) -- write `hrs -c {}`: cargo, run in export/", rest.join(" "));
            ExitCode::from(2)
        }
        // Pass 2 is any program the programmer names, found as the shell
        // finds it: Harsh arbitrates no conflict between two programs of
        // one name (the user's ruling, 2026-10-12; `hrs dx` is retired).
        "--" if !rest.is_empty() => tool_cmd(&rest[0], &rest[1..]),
        "--" => {
            eprintln!("hrs: `hrs --` runs a program after transpiling: hrs -- <program> [args], e.g. hrs -- dx serve");
            ExitCode::from(2)
        }
        "dx" => {
            eprintln!("hrs: `hrs dx` is retired (0.8.0) -- Harsh runs any program after transpiling, found as your shell finds it: write `hrs -- dx {}`", rest.join(" "));
            ExitCode::from(2)
        }
        "which" if rest.len() == 1 => which(&rest[0]),
        "which" => {
            eprintln!("hrs: hrs which <program> -- every program of that name on PATH, in order");
            ExitCode::from(2)
        }
        "watch" => watch(&rest),
        "new" => new_project(&rest),
        "publish" => publish(&rest),
        "add" => add(&rest),
        "migrate" => migrate(),
        "export" => {
            eprintln!("hrs: `hrs export` is retired (0.9.0) -- every `hrs build` writes export/, the whole Rust project; to publish it to crates.io: hrs -c publish");
            ExitCode::from(2)
        }
        "fmt" => fmt(&rest),
        "expand" => expand(&rest),
        "dist" => dist(&rest),
        _ => single_file(&args),
    }
}

/// `hrs build`, `check`, `test`, `lint` and `fmt` at a workspace's root run
/// in each member, in `members`' order: a Harsh member by `hrs` in its
/// folder, a Rust member by cargo in its place in the workspace's `export/`
/// (`fmt`, Harsh's only, skips it). The first failure stops the run, as
/// cargo's would. `hrs -c …` is cargo in `export/`, for the whole workspace. A command that
/// acts on one program -- `run`, `--`, `watch`, `publish`, `add`
/// -- is refused, naming the members. `None` when the folder is not a
/// workspace's root, or the command is another.
fn at_workspace_root(first: &str, rest: &[String]) -> Option<ExitCode> {
    let per_member = matches!(first, "build" | "check" | "test" | "lint")
        || (first == "-c" && !rest.is_empty())
        || (first == "fmt" && rest.iter().all(|a| a == "--check"));
    let one_program = matches!(first, "run" | "--" | "watch" | "publish" | "add");
    if !per_member && !one_program {
        return None;
    }
    let root = Project::find().ok()?.root;
    let members = driver::workspace_members(&root)?;
    let rel = |m: &PathBuf| m.strip_prefix(&root).unwrap_or(m).display().to_string();
    if one_program {
        let names: Vec<String> = members.iter().map(rel).collect();
        eprintln!(
            "hrs: `hrs {first}` acts on one program, and {} is a workspace's root: run it in a member's folder ({})",
            root.display(),
            names.join(", ")
        );
        return Some(ExitCode::from(2));
    }
    if members.is_empty() {
        eprintln!("hrs: {} is a workspace's root with no members", root.display());
        return Some(ExitCode::FAILURE);
    }
    // 0.9.0 (option A): the workspace's `export/` is the Rust workspace.
    // Cargo, run anywhere in it, reads every member, so it is written
    // whole first; `hrs -c` is cargo there, once, for the workspace.
    if first != "fmt" {
        if let Err(e) = driver::prepare_workspace_root(&root) {
            eprintln!("{e}");
            return Some(ExitCode::FAILURE);
        }
    }
    let export = root.join(driver::EXPORT_DIR);
    // Harsh's distribution, patched in for the crates of it the workspace
    // names -- as for a member's own cargo -- and none for plain Rust.
    let patches = if first == "fmt" {
        Vec::new()
    } else {
        let mut manifests = vec![export.join("Cargo.toml")];
        manifests.extend(driver::harsh_members(&root).into_iter().map(|m| Project { root: m }.export_dir().join("Cargo.toml")));
        match driver::distribution(&root, &manifests) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("hrs: {e}");
                return Some(ExitCode::FAILURE);
            }
        }
    };
    if first == "-c" {
        let status = std::process::Command::new("cargo").args(&patches).args(rest).current_dir(&export).status();
        let lock = export.join("Cargo.lock");
        if lock.is_file() {
            let _ = std::fs::copy(&lock, root.join("Cargo.lock"));
        }
        return Some(match status {
            Ok(s) if s.success() => ExitCode::SUCCESS,
            Ok(_) => ExitCode::FAILURE,
            Err(e) => {
                eprintln!("hrs: cargo: {e}");
                ExitCode::FAILURE
            }
        });
    }
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("hrs"));
    for m in &members {
        let harsh = driver::is_harsh_member(m);
        if !harsh && first == "fmt" {
            continue;
        }
        eprintln!("hrs: workspace member {}{}", rel(m), if harsh { "" } else { " (Rust, by cargo)" });
        let status = if harsh {
            std::process::Command::new(&exe).arg(first).args(rest).current_dir(m).status()
        } else {
            let (sub, args) = match first {
                "lint" => ("clippy", rest),
                s => (s, rest),
            };
            // In its place in `export/`, mirrored there.
            let at = export.join(m.strip_prefix(&root).unwrap_or(m));
            std::process::Command::new("cargo").args(&patches).arg(sub).args(args).current_dir(&at).status()
        };
        match status {
            Ok(s) if s.success() => {}
            Ok(_) => return Some(ExitCode::FAILURE),
            Err(e) => {
                eprintln!("hrs: {}: {e}", rel(m));
                return Some(ExitCode::FAILURE);
            }
        }
    }
    Some(ExitCode::SUCCESS)
}

/// `hrs add <crate>..`: Harsh crates into the project's Hrs.toml (made if
/// missing).
fn add(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("hrs add <crate>..  (Harsh's crates: {})", driver::DISTRIBUTION.join(", "));
        return ExitCode::from(2);
    }
    let p = match Project::find() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("hrs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    // `hrs_std` goes to Hrs.toml; the procedural-macro crates to Cargo.toml.
    let to_cargo = args.iter().all(|a| driver::DISTRIBUTION.contains(&a.as_str()) && !driver::HRS_TOML_CRATES.contains(&a.as_str()));
    let path = if to_cargo { p.root.join("Cargo.toml") } else { p.root.join(driver::HRS_MANIFEST) };
    let mut manifest = std::fs::read_to_string(&path).unwrap_or_else(|_| NEW_HRS_TOML.to_string());
    let mut ok = true;
    for name in args {
        match driver::add_dependency(&manifest, name) {
            Ok((text, msg)) => {
                manifest = text;
                eprintln!("hrs: {msg}");
            }
            Err(e) => {
                eprintln!("hrs: {e}");
                ok = false;
            }
        }
    }
    if std::fs::write(&path, &manifest).is_err() {
        eprintln!("hrs: cannot write {}", path.display());
        return ExitCode::FAILURE;
    }
    if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// A new project's `.gitignore` (0.9.0): what `hrs` generates. `/target`
/// covers `export/target/` too, the copy of this file there being read from
/// `export/`.
const NEW_GITIGNORE: &str = "/target\n/export\n/.hrs\n";

/// A new project's `Hrs.toml`: Harsh's dependencies, beside Cargo.toml.
const NEW_HRS_TOML: &str = "# Hrs.toml -- Harsh's dependencies; Cargo.toml beside it keeps Rust's\n[package]\ncargo = \"Cargo.toml\"        # optional: the Cargo.toml beside this file is the default\n\n[dependencies]\n";

/// `/export` and `/.hrs` added to the project's `.gitignore` -- when it has
/// one, or is a git repository -- and what was done.
fn ignore_generated(root: &std::path::Path) -> Option<String> {
    let ignore = root.join(".gitignore");
    let text = std::fs::read_to_string(&ignore).unwrap_or_default();
    let missing: Vec<&str> = ["/export", "/.hrs"].into_iter().filter(|e| !text.lines().any(|l| l.trim() == *e || l.trim() == &e[1..])).collect();
    if missing.is_empty() || !(ignore.exists() || root.join(".git").exists()) {
        return None;
    }
    let mut t = text.clone();
    if !t.is_empty() && !t.ends_with('\n') {
        t.push('\n');
    }
    for e in &missing {
        t.push_str(e);
        t.push('\n');
    }
    std::fs::write(&ignore, t).ok()?;
    Some(format!("added {} to .gitignore", missing.join(" and ")))
}

/// `hrs migrate`: Harsh's crates moved from Cargo.toml to Hrs.toml (0.3.0);
/// the targets' paths from `target/src/` to `src/`, read in `export/`, the
/// `.gitignore` given `/export` and `/.hrs`, and the old generated tree and
/// builds removed (0.9.0). At a workspace's root, each Harsh member.
fn migrate() -> ExitCode {
    // The nearest folder, here or above, with a Cargo.toml.
    let mut dir = std::env::current_dir().ok();
    let mut root = None;
    while let Some(d) = dir {
        if d.join("Cargo.toml").is_file() {
            root = Some(d);
            break;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    let Some(root) = root else {
        eprintln!("hrs: no Cargo.toml here or above");
        return ExitCode::FAILURE;
    };
    // A workspace's root (0.9.0): its `.gitignore`, then each Harsh member
    // migrated in its folder. Cargo builds in the root's `export/target/`.
    if let Some(members) = driver::workspace_members(&root) {
        if let Some(l) = ignore_generated(&root) {
            eprintln!("hrs: {l}");
        }
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("hrs"));
        for m in members.iter().filter(|m| driver::is_harsh_member(m)) {
            eprintln!("hrs: workspace member {}", m.strip_prefix(&root).unwrap_or(m).display());
            match std::process::Command::new(&exe).arg("migrate").current_dir(m).status() {
                Ok(s) if s.success() => {}
                _ => return ExitCode::FAILURE,
            }
        }
        return ExitCode::SUCCESS;
    }
    let cargo_path = root.join("Cargo.toml");
    let hrs_path = root.join(driver::HRS_MANIFEST);
    let Ok(cargo) = std::fs::read_to_string(&cargo_path) else {
        eprintln!("hrs: cannot read {}", cargo_path.display());
        return ExitCode::FAILURE;
    };
    let hrs = std::fs::read_to_string(&hrs_path).ok();
    let (cargo2, hrs2, moved) = driver::migrate(&cargo, hrs.as_deref().or(Some(NEW_HRS_TOML)));
    // 0.9.0: what `hrs` generates is ignored, and the old places go -- the
    // generated tree `target/src/` and the builds in `target/`, all `hrs`'s
    // output since 0.3.0 (cargo builds into `export/target/` now).
    let mut layout: Vec<String> = ignore_generated(&root).into_iter().collect();
    let old = root.join("target");
    if old.join("src").join(".hrs-stamp").is_file() || moved.iter().any(|n| n == driver::EXPORT_DIR) {
        if old.is_dir() && std::fs::remove_dir_all(&old).is_ok() {
            layout.push("removed target/ (the old generated tree and builds; cargo builds in export/target/ now)".to_string());
        }
    }
    if moved.is_empty() && layout.is_empty() {
        eprintln!("hrs: nothing to migrate; the project is up to date");
        return ExitCode::SUCCESS;
    }
    for l in &layout {
        eprintln!("hrs: {l}");
    }
    if moved.is_empty() {
        return ExitCode::SUCCESS;
    }
    if std::fs::write(&cargo_path, cargo2).is_err() || std::fs::write(&hrs_path, hrs2).is_err() {
        eprintln!("hrs: cannot write the manifests");
        return ExitCode::FAILURE;
    }
    // The generated tree's new place is a path update, not a move.
    if moved.iter().any(|n| n == driver::EXPORT_DIR) {
        eprintln!("hrs: Cargo.toml's targets now say src/, read in export/, the Rust project (0.9.0; it was target/src/)");
    }
    let crates: Vec<&String> = moved.iter().filter(|n| *n != driver::EXPORT_DIR).collect();
    if !crates.is_empty() {
        eprintln!("hrs: moved {} from Cargo.toml to Hrs.toml", crates.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", "));
    }
    ExitCode::SUCCESS
}

/// `hrs dist <dir>`: Harsh's standard distribution, written into `dir` --
/// `dir/hrs_std` and the other three -- for what builds outside a Harsh
/// project and still wants `hrs_std`: the Jupyter kernel gives it to evcxr.
fn dist(args: &[String]) -> ExitCode {
    let Some(dir) = args.first() else {
        eprintln!("hrs dist <dir>");
        return ExitCode::from(2);
    };
    let dir = PathBuf::from(dir);
    match driver::write_distribution(&dir) {
        Ok(()) => {
            eprintln!("hrs: wrote Harsh's standard distribution to {}", dir.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("hrs: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// `hrs expand <file.hrs>`: the file with its Harsh macros unfolded, as Harsh
/// -- the middle step between what you wrote and the Rust it becomes. What
/// the transpiler goes on to read; useful when a macro does not do what you
/// meant, since a mistake in a macro is a mistake in the Harsh it produced.
fn expand(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("hrs expand <file.hrs>");
        return ExitCode::from(2);
    };
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("hrs: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let (work, _zones) = harsh_lang::rawzone::prepare(&src);
    let (work, regs) = match harsh_lang::procmac::prepare(&work) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e.msg);
            return ExitCode::from(1);
        }
    };
    let toks = match harsh_lang::procmac::lex(&work, &regs) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {}", e.msg);
            return ExitCode::from(1);
        }
    };
    // The project the file belongs to decides which proc macros exist: its
    // runner is built as `hrs build` builds it. A file outside any project,
    // or in one that uses none, expands without them.
    let dir = std::path::Path::new(path)
        .canonicalize()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let runner = match driver::Project::find_from(dir).map(|p| p.proc_runner()) {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            eprintln!("hrs: {e}");
            return ExitCode::from(1);
        }
        Err(_) => None,
    };
    let procs: &dyn harsh_lang::mac::ProcMacros = match &runner {
        Some(r) => r,
        None => &harsh_lang::mac::NoProcMacros,
    };
    let taken = harsh_lang::layout::names_in_scope(&toks);
    match harsh_lang::mac::expand_all_with(toks, &taken, procs) {
        Ok((out, _)) => {
            println!("{}", harsh_lang::mac::render_file(&out));
            // A call nothing defined is printed as written; say so, rather
            // than let it pass for an expansion.
            for name in harsh_lang::mac::unexpanded(&out) {
                eprintln!(
                    "note: `{name}~` is left as written: it is not a `macro_rules~` of this file or the prelude, \
                     nor a proc macro of this project (`proc-macros` under [package.metadata.harsh])"
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {}", e.msg);
            ExitCode::from(1)
        }
    }
}

/// `hrs fmt [--check] [files]`: the formatter (`fmt.rs`) over the given
/// files, or every `.hrs` under `src/` of the project found from here.
fn fmt(args: &[String]) -> ExitCode {
    let check = args.iter().any(|a| a == "--check");
    let mut files: Vec<PathBuf> = args.iter().filter(|a| *a != "--check").map(PathBuf::from).collect();
    if files.is_empty() {
        match Project::find() {
            Ok(p) => {
                fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
                    if let Ok(rd) = std::fs::read_dir(dir) {
                        for e in rd.flatten() {
                            let path = e.path();
                            if path.is_dir() {
                                walk(&path, out);
                            } else if path.extension().map_or(false, |x| x == "hrs") {
                                out.push(path);
                            }
                        }
                    }
                }
                walk(&p.root.join("src"), &mut files);
                files.sort();
            }
            Err(e) => {
                eprintln!("hrs: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }
    let mut changed = 0usize;
    let mut skipped = 0usize;
    for f in &files {
        let src = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("hrs: {}: {}", f.display(), e);
                return ExitCode::FAILURE;
            }
        };
        let (out, backed_out) = harsh_lang::fmt::format_checked(&src);
        if backed_out {
            // Rule 0 refused the formatter's own result: never silent, and
            // never counted as formatted (S4, 2026-10-05).
            skipped += 1;
            eprintln!(
                "hrs fmt: {}: left as written -- its formatted form would not transpile to the same Rust; this is a formatter bug, please report the file",
                f.display()
            );
            continue;
        }
        if out != src {
            changed += 1;
            if check {
                println!("{}", f.display());
            } else if let Err(e) = std::fs::write(f, &out) {
                eprintln!("hrs: {}: {}", f.display(), e);
                return ExitCode::FAILURE;
            }
        }
    }
    if check {
        if changed > 0 || skipped > 0 {
            eprintln!("hrs fmt: {} of {} files would change{}", changed, files.len(), if skipped > 0 { format!(", {skipped} could not be formatted") } else { String::new() });
            return ExitCode::FAILURE;
        }
        eprintln!("hrs fmt: {} files, all formatted", files.len());
    } else {
        eprintln!("hrs fmt: {} of {} files reformatted{}", changed, files.len(), if skipped > 0 { format!(", {skipped} left as written") } else { String::new() });
        if skipped > 0 {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// `hrs -c <command> [args]`: pass 1, then `cargo <command>` in `export/`,
/// the Rust project (the user, 2026-10-14). For crates.io, `hrs -c publish`:
/// a Harsh macro crate is refused -- its macros exist only in Harsh, and it
/// has no Rust meaning -- and a missing `description` or `license`, which
/// crates.io requires, is said before cargo says it.
fn c_cmd(args: &[String]) -> ExitCode {
    let p = match Project::find() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("hrs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    if args[0] == "publish" || args[0] == "package" {
        if p.meta().map_or(false, |m| m.proc_macro) {
            eprintln!("hrs: a Harsh macro crate is not published to crates.io: its macros exist only in Harsh, expanded where they are called -- it goes to Harsh's registry, hrs publish");
            return ExitCode::FAILURE;
        }
        let toml = std::fs::read_to_string(p.root.join("Cargo.toml")).unwrap_or_default();
        let package = toml.split("\n[").next().unwrap_or("");
        let has = |k: &str| package.lines().any(|l| { let t = l.trim_start(); t.starts_with(&format!("{k} ")) || t.starts_with(&format!("{k}=")) || (k == "license" && t.starts_with("license-file")) });
        for k in ["description", "license"] {
            if !has(k) {
                eprintln!("hrs: warning: Cargo.toml has no `{k}` in [package] -- crates.io requires it");
            }
        }
    }
    if let Err(e) = build_tree(&p) {
        eprintln!("{}", e);
        return ExitCode::FAILURE;
    }
    if let Some(msg) = driver::missing_std(&p.root) {
        eprintln!("hrs: {msg}");
        return ExitCode::FAILURE;
    }
    match p.tool("cargo", args) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("hrs: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Say what the transpile step did, so a no-op is never silent: a build that
/// prints only cargo's `Finished` line is indistinguishable from one that
/// skipped a file it should have rebuilt.
fn report(rebuilt: usize, total: usize) {
    if rebuilt > 0 {
        eprintln!("hrs: transpiled {} of {} file(s)", rebuilt, total);
    } else {
        eprintln!("hrs: {} file(s) up to date", total);
    }
}

fn build_tree(p: &Project) -> Result<Vec<PathBuf>, String> {
    let (rebuilt, maps) = p.transpile(false)?;
    report(rebuilt.len(), maps.len());
    Ok(maps)
}

/// Every executable named `name` on PATH, in PATH's order: the first is the
/// one a shell -- and `hrs --` -- runs.
fn on_path(name: &str) -> Vec<PathBuf> {
    let Some(path) = std::env::var_os("PATH") else { return Vec::new() };
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT".into()).split(';').map(|e| e.to_lowercase()).collect()
    } else {
        vec![String::new()]
    };
    let mut out: Vec<PathBuf> = Vec::new();
    for dir in std::env::split_paths(&path) {
        for ext in &exts {
            let p = dir.join(format!("{name}{ext}"));
            if is_executable(&p) && !out.iter().any(|q| q == &p) {
                out.push(p);
            }
        }
    }
    out
}

fn is_executable(p: &std::path::Path) -> bool {
    let Ok(m) = std::fs::metadata(p) else { return false };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        m.is_file() && m.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        m.is_file()
    }
}

/// `hrs which <program>`: every program of that name on PATH, in order, with
/// the first line its `--version` prints -- so two programs sharing a name
/// (Dioxus's `dx` and Deno's) are seen at once. Harsh decides nothing.
fn which(name: &str) -> ExitCode {
    let found = on_path(name);
    if found.is_empty() {
        eprintln!("hrs: no program named `{name}` on PATH");
        return ExitCode::FAILURE;
    }
    for (n, p) in found.iter().enumerate() {
        let version = version_of(p);
        let mark = if n == 0 { "   <- runs" } else { "" };
        println!("{}    {}{}", p.display(), version, mark);
    }
    ExitCode::SUCCESS
}

/// The first line a program prints for `--version`, waiting at most two
/// seconds; empty when it prints nothing or does not finish.
fn version_of(p: &std::path::Path) -> String {
    use std::process::{Command, Stdio};
    let Ok(mut child) = Command::new(p).arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() else {
        return String::new();
    };
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < std::time::Duration::from_secs(2) => std::thread::sleep(std::time::Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                return String::new();
            }
        }
    }
    let Ok(out) = child.wait_with_output() else { return String::new() };
    let text = if out.stdout.is_empty() { out.stderr } else { out.stdout };
    String::from_utf8_lossy(&text).lines().next().unwrap_or("").trim().to_string()
}

/// `hrs -- <program> [args]`: pass 1, then the program in the generated
/// project. The program is found as a shell finds it -- a name on PATH, a
/// path as written, relative to where `hrs` was run -- and when PATH holds
/// more than one program of that name, a note says which runs (2026-10-12).
fn tool_cmd(prog: &str, args: &[String]) -> ExitCode {
    // A path is the caller's: resolved from where `hrs` runs, not from
    // `export/`, where the program will.
    let prog: String = if prog.contains('/') || prog.contains('\\') {
        std::fs::canonicalize(prog).map(|p| p.display().to_string()).unwrap_or_else(|_| prog.to_string())
    } else {
        let found = on_path(prog);
        if found.len() > 1 {
            eprintln!("hrs: {} programs named `{prog}` on PATH; running {} (`hrs which {prog}` lists them)", found.len(), found[0].display());
        }
        prog.to_string()
    };
    let prog = prog.as_str();
    let p = match Project::find() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("hrs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = build_tree(&p) {
        eprintln!("{}", e);
        return ExitCode::FAILURE;
    }
    if let Some(msg) = driver::missing_std(&p.root) {
        eprintln!("hrs: {msg}");
        return ExitCode::FAILURE;
    }
    match p.tool(prog, args) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("hrs: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn cargo_cmd(sub: &str, args: &[String]) -> ExitCode {
    let p = match Project::find() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("hrs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let maps = match build_tree(&p) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };
    if let Some(msg) = driver::missing_std(&p.root) {
        eprintln!("hrs: {msg}");
        return ExitCode::FAILURE;
    }
    match p.cargo(sub, args, &maps) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("hrs: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn watch(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("check").to_string();
    let sub = if sub == "lint" { "clippy".to_string() } else { sub };
    let rest: Vec<String> = args.iter().skip(1).cloned().collect();
    let p = match Project::find() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("hrs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    eprintln!("hrs: watching src/ — cargo {}", sub);
    let r = driver::watch(&p, || {
        eprint!("\x1b[2J\x1b[H");
        match p.transpile(false) {
            Ok((rebuilt, maps)) => {
                report(rebuilt.len(), maps.len());
                let _ = p.cargo(&sub, &rest, &maps);
            }
            Err(e) => eprintln!("{}", e),
        }
    });
    if let Err(e) = r {
        eprintln!("hrs: {}", e);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// A new library's source: one public function and a test, so `hrs test`
/// works at once and shows where tests go.
const NEW_LIB_HRS: &str = "/// Adds two numbers.\npub fn add (a: i32) (b: i32) -> i32: a + b\n\n#[cfg test]\nmod tests\n    use super.*\n\n    #[test]\n    fn adds$:\n        assert_eq! (add 2 3) 5\n";

/// `hrs publish` (decided 2026-10-04): checks the package as a registry
/// would -- `name`, `version`, `description`, `license` in `Cargo.toml`, and
/// every Harsh dependency in `Hrs.toml` by version, not by path -- then stops,
/// since Harsh's registry is not open yet, naming the two ways to share.
fn publish(_args: &[String]) -> ExitCode {
    let cargo = std::fs::read_to_string("Cargo.toml").unwrap_or_default();
    let hrs = std::fs::read_to_string(driver::HRS_MANIFEST).unwrap_or_default();
    if cargo.is_empty() {
        eprintln!("hrs publish: no Cargo.toml here -- run it at the root of a Harsh project");
        return ExitCode::from(2);
    }
    let mut problems = Vec::new();
    let package: String = cargo.split("\n[").next().unwrap_or("").to_string();
    for key in ["name", "version", "description"] {
        if !package.lines().any(|l| l.trim_start().starts_with(&format!("{key} ")) || l.trim_start().starts_with(&format!("{key}="))) {
            problems.push(format!("Cargo.toml has no `{key}` in [package]"));
        }
    }
    if !package.lines().any(|l| { let t = l.trim_start(); t.starts_with("license ") || t.starts_with("license=") || t.starts_with("license-file") }) {
        problems.push("Cargo.toml has no `license` (or `license-file`) in [package]".to_string());
    }
    let mut in_deps = false;
    for l in hrs.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]";
            continue;
        }
        if in_deps && t.contains("path") && !t.contains("version") && !t.starts_with('#') {
            let dep = t.split('=').next().unwrap_or("").trim();
            problems.push(format!("Hrs.toml: `{dep}` is a path -- a published crate names its dependencies by version"));
        }
    }
    if problems.is_empty() {
        println!("hrs publish: the package is ready.");
    } else {
        for p in &problems {
            eprintln!("hrs publish: {p}");
        }
    }
    eprintln!("hrs publish: Harsh's registry is not open yet. To share as Harsh today, list the crate by path in Hrs.toml; to publish the Rust to crates.io: hrs -c publish.");
    ExitCode::FAILURE
}

fn new_project(args: &[String]) -> ExitCode {
    const USAGE: &str = "usage: hrs new [--lib] <name>\n  creates <name>/ laid out for Harsh: Cargo.toml, Hrs.toml, and src/main.hrs -- or, with --lib, a library: src/lib.hrs";
    // A flag is not a name (found 2026-10-04: `hrs new --help` created a
    // project called `--help`).
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    // `--lib`, before or after the name, as `cargo new` takes it (decided
    // 2026-10-04): a library for intent #1 or #2.
    let lib = args.iter().any(|a| a == "--lib");
    let names: Vec<&String> = args.iter().filter(|a| a.as_str() != "--lib").collect();
    let Some(name) = names.first().copied() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    if name.starts_with('-') {
        eprintln!("hrs: `{name}` is not a project name -- a name cannot start with `-`\n{USAGE}");
        return ExitCode::from(2);
    }
    let root = PathBuf::from(name);
    if root.exists() {
        eprintln!("hrs: {} already exists", root.display());
        return ExitCode::FAILURE;
    }
    let mk = |p: PathBuf, body: &str| std::fs::write(p, body);
    if std::fs::create_dir_all(root.join("src")).is_err() {
        eprintln!("hrs: cannot create {}", root.display());
        return ExitCode::FAILURE;
    }
    // No target paths (0.9.0): `export/`, where cargo runs, has `src/main.rs`
    // or `src/lib.rs`, where cargo looks unasked.
    let _ = mk(
        root.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             # Harsh sources are in src/**.hrs; `hrs build` writes the Rust project to export/.\n\
             [dependencies]\n"
        ),
    );
    let _ = mk(root.join(driver::HRS_MANIFEST), NEW_HRS_TOML);
    if lib {
        let _ = mk(root.join("src/lib.hrs"), NEW_LIB_HRS);
        let _ = mk(root.join(".gitignore"), NEW_GITIGNORE);
        println!("created {} (a library)\n  cd {} && hrs test", root.display(), root.display());
        return ExitCode::SUCCESS;
    }
    let _ = mk(
        root.join("src/main.hrs"),
        "fn main$:\n    println! \"Hello from Harsh\"\n",
    );
    let _ = mk(root.join(".gitignore"), NEW_GITIGNORE);
    println!("created {}\n  cd {} && hrs run", root.display(), root.display());
    ExitCode::SUCCESS
}

/// `hrs in.hrs [-o out.rs] [--map m.json]`, `hrs a.hrs b.hrs ..`, and
/// `--check`: Harsh files in a Rust project (intent #3, 2026-10-03). Several
/// files share their functions' parameter counts, as a project's do, and
/// each is written beside its `.hrs`. A written file starts with a header
/// naming its source; `--check` writes nothing and fails if a `.rs` is not
/// what its `.hrs` makes -- the drift check, for CI.
fn single_file(args: &[String]) -> ExitCode {
    // The language server's completion transpiles a project's file on the
    // side, and asks for it as the project writes it: `hrs_std` folded in.
    if std::env::var("HRS_FOLD_STD").map_or(false, |v| v == "1") {
        driver::set_fold_std(true);
    }
    let (mut inputs, mut output, mut map, mut check) = (Vec::new(), None, None, false);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                output = args.get(i).map(PathBuf::from);
            }
            "--map" => {
                i += 1;
                map = args.get(i).map(PathBuf::from);
            }
            "--check" => check = true,
            s if s.ends_with(".hrs") => inputs.push(PathBuf::from(s)),
            _ => {
                eprintln!("{}", USAGE);
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    if inputs.is_empty() {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    }
    if inputs.len() > 1 && (output.is_some() || map.is_some()) {
        eprintln!("hrs: `-o` and `--map` name one file; several are each written beside their .hrs");
        return ExitCode::from(2);
    }
    // Every file's parameter counts, for `$` and partial application across
    // the files given together.
    let mut sources = Vec::new();
    let mut arities = std::collections::HashMap::new();
    for input in &inputs {
        match std::fs::read_to_string(input) {
            Ok(s) => {
                if let Ok(toks) = harsh_lang::lex::lex(&s) {
                    arities.extend(harsh_lang::juxt::collect_arities(&toks));
                }
                sources.push(s);
            }
            Err(e) => {
                eprintln!("hrs: cannot read {}: {}", input.display(), e);
                return ExitCode::FAILURE;
            }
        }
    }
    let mut drifted = Vec::new();
    for (n, (input, src)) in inputs.iter().zip(&sources).enumerate() {
        let target = if inputs.len() == 1 { output.clone() } else { Some(input.with_extension("rs")) };
        let target = if check && target.is_none() { Some(input.with_extension("rs")) } else { target };
        let tmp = std::env::temp_dir().join(format!("hrs-one-{}-{n}.rs", std::process::id()));
        let tmp_map = tmp.with_extension("map");
        if let Err(e) = driver::transpile_one_with(src, input, &tmp, &tmp_map, &arities) {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
        let body = std::fs::read_to_string(&tmp).unwrap_or_default();
        let _ = std::fs::remove_file(&tmp);
        let name = input.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        let header = format!("// Generated from {name} by hrs; edit that file, not this one.\n");
        let text = if target.is_some() { format!("{header}{body}") } else { body };
        if check {
            let target = target.unwrap();
            if std::fs::read_to_string(&target).ok().as_deref() != Some(text.as_str()) {
                drifted.push(target);
            }
            let _ = std::fs::remove_file(&tmp_map);
            continue;
        }
        match &target {
            Some(t) => {
                if std::fs::write(t, &text).is_err() {
                    eprintln!("hrs: cannot write {}", t.display());
                    return ExitCode::FAILURE;
                }
            }
            None => print!("{text}"),
        }
        // The map's generated offsets move by the header's length.
        if let Some(m) = &map {
            let shift = if target.is_some() { header.len() as u64 } else { 0 };
            let shifted = std::fs::read_to_string(&tmp_map)
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .map(|mut v| {
                    // The map names the file written, not the scratch file
                    // the transpiler wrote first.
                    if let Some(t) = &target {
                        v["generated"] = serde_json::json!(t.display().to_string());
                    }
                    if let Some(es) = v["entries"].as_array_mut() {
                        for e in es.iter_mut() {
                            for k in 0..2 {
                                if let Some(x) = e[k].as_u64() {
                                    e[k] = serde_json::json!(x + shift);
                                }
                            }
                        }
                    }
                    v.to_string()
                });
            if let Some(t) = shifted {
                let _ = std::fs::write(m, t);
            }
        }
        let _ = std::fs::remove_file(&tmp_map);
    }
    if !drifted.is_empty() {
        for d in &drifted {
            eprintln!("hrs: {} is not what its .hrs makes: run hrs on it again", d.display());
        }
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `hrs` — the Harsh transpiler and build driver.

use harsh_lang::driver::{self, Project};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
usage:
  hrs build [cargo args]     transpile, then cargo build
  hrs run   [cargo args]     transpile, then cargo run
  hrs test  [cargo args]     transpile, then cargo test
  hrs check [cargo args]     transpile, then cargo check
  hrs lint  [cargo args]     transpile, then cargo clippy
  hrs cargo <sub> [args]     transpile, then any cargo subcommand (cargo leptos build, ..)
  hrs watch [subcommand]     rebuild on every save (default: check)
  hrs new   <name>           create a project laid out for Harsh
  hrs add   <crate>..        add Harsh crates to Hrs.toml -- hrs_std and the
                             rest of Harsh's distribution; a Rust crate is
                             added to Cargo.toml with cargo add
  hrs migrate                move Harsh's crates from Cargo.toml to Hrs.toml
                             (0.3.0: Hrs.toml lists Harsh's dependencies)
  hrs export [dir]           write the project as a plain Rust crate,
                             formatted with cargo fmt (default: target/export)
  hrs fmt   [--check] [files] reformat .hrs files in place (default: src/**.hrs);
                             --check lists the files that would change, exit 1
  hrs dist  <dir>             write Harsh's standard distribution (hrs_std,
                             hrs_proc_macro, hrs_quote, hrs_syn) into <dir>,
                             for building outside a Harsh project

  hrs <input.hrs> [-o out.rs] [--map out.map.json]
                             transpile a single file

Sources live in src/**.hrs; generated Rust goes to target/hrs/, and
Cargo.toml points its targets at that tree.";

fn main() -> ExitCode {
    if std::env::args().skip(1).any(|a| a == "--version" || a == "-V") {
        println!("hrs {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(first) = args.first() else {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    };
    let rest: Vec<String> = args[1..].to_vec();

    match first.as_str() {
        "-h" | "--help" | "help" => {
            println!("{}", USAGE);
            ExitCode::SUCCESS
        }
        "build" | "run" | "test" | "check" => cargo_cmd(first, &rest),
        "lint" => cargo_cmd("clippy", &rest),
        // `hrs cargo <subcommand> [args]`: transpile, then any cargo
        // subcommand -- `hrs cargo leptos build`, `hrs cargo doc`.
        "cargo" if !rest.is_empty() => cargo_cmd(&rest[0], &rest[1..]),
        "watch" => watch(&rest),
        "new" => new_project(&rest),
        "add" => add(&rest),
        "migrate" => migrate(),
        "export" => export(&rest),
        "fmt" => fmt(&rest),
        "expand" => expand(&rest),
        "dist" => dist(&rest),
        _ => single_file(&args),
    }
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

/// A new project's `Hrs.toml`: Harsh's dependencies, beside Cargo.toml.
const NEW_HRS_TOML: &str = "# Hrs.toml -- Harsh's dependencies; Cargo.toml beside it keeps Rust's\n[package]\ncargo = \"Cargo.toml\"        # optional: the Cargo.toml beside this file is the default\n\n[dependencies]\n";

/// `hrs migrate`: Harsh's crates moved from Cargo.toml to Hrs.toml (0.3.0).
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
    let cargo_path = root.join("Cargo.toml");
    let hrs_path = root.join(driver::HRS_MANIFEST);
    let Ok(cargo) = std::fs::read_to_string(&cargo_path) else {
        eprintln!("hrs: cannot read {}", cargo_path.display());
        return ExitCode::FAILURE;
    };
    let hrs = std::fs::read_to_string(&hrs_path).ok();
    let (cargo2, hrs2, moved) = driver::migrate(&cargo, hrs.as_deref().or(Some(NEW_HRS_TOML)));
    if moved.is_empty() {
        eprintln!("hrs: nothing to move; Cargo.toml names none of Harsh's crates");
        return ExitCode::SUCCESS;
    }
    if std::fs::write(&cargo_path, cargo2).is_err() || std::fs::write(&hrs_path, hrs2).is_err() {
        eprintln!("hrs: cannot write the manifests");
        return ExitCode::FAILURE;
    }
    eprintln!("hrs: moved {} from Cargo.toml to Hrs.toml", moved.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", "));
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
    for f in &files {
        let src = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("hrs: {}: {}", f.display(), e);
                return ExitCode::FAILURE;
            }
        };
        let out = harsh_lang::fmt::format(&src);
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
        if changed > 0 {
            eprintln!("hrs fmt: {} of {} files would change", changed, files.len());
            return ExitCode::FAILURE;
        }
        eprintln!("hrs fmt: {} files, all formatted", files.len());
    } else {
        eprintln!("hrs fmt: {} of {} files reformatted", changed, files.len());
    }
    ExitCode::SUCCESS
}

fn export(args: &[String]) -> ExitCode {
    let dir = args.first().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("target/export"));
    let r = Project::find().and_then(|p| {
        let dir = if dir.is_absolute() { dir.clone() } else { p.root.join(&dir) };
        p.export(&dir).map(|e| (dir, e))
    });
    match r {
        Ok((dir, e)) => {
            eprintln!(
                "hrs: exported {} file(s) to {}{}",
                e.files.len(),
                dir.display(),
                if e.formatted { ", formatted with cargo fmt" } else { " (cargo fmt not available; left unformatted)" }
            );
            ExitCode::SUCCESS
        }
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

fn new_project(args: &[String]) -> ExitCode {
    let Some(name) = args.first() else {
        eprintln!("usage: hrs new <name>");
        return ExitCode::from(2);
    };
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
    let _ = mk(
        root.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             # Harsh sources are in src/**.hrs; `hrs build` generates this tree.\n\
             [[bin]]\nname = \"{name}\"\npath = \"target/hrs/main.rs\"\n\n[dependencies]\n"
        ),
    );
    let _ = mk(root.join(driver::HRS_MANIFEST), NEW_HRS_TOML);
    let _ = mk(
        root.join("src/main.hrs"),
        "fn main$:\n    println! \"Hello from Harsh\"\n",
    );
    let _ = mk(root.join(".gitignore"), "/target\n");
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

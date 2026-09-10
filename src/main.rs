// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `hrs` — the Harsh transpiler and build driver.

use hrust::driver::{self, Project};
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
  hrs export [dir]           write the project as a plain Rust crate,
                             formatted with cargo fmt (default: target/export)
  hrs fmt   [--check] [files] reformat .hrs files in place (default: src/**.hrs);
                             --check lists the files that would change, exit 1

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
        "export" => export(&rest),
        "fmt" => fmt(&rest),
        _ => single_file(&args),
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
        let out = hrust::fmt::format(&src);
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
    let _ = mk(
        root.join("src/main.hrs"),
        "fn main$:\n    println! \"Hello from Harsh\"\n",
    );
    let _ = mk(root.join(".gitignore"), "/target\n");
    println!("created {}\n  cd {} && hrs run", root.display(), root.display());
    ExitCode::SUCCESS
}

fn single_file(args: &[String]) -> ExitCode {
    let (mut input, mut output, mut map) = (None, None, None);
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
            s if input.is_none() => input = Some(PathBuf::from(s)),
            _ => {
                eprintln!("{}", USAGE);
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    let Some(input) = input else {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    };
    let src = match std::fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("hrs: cannot read {}: {}", input.display(), e);
            return ExitCode::FAILURE;
        }
    };
    let rust = output.clone().unwrap_or_else(|| PathBuf::from("/dev/stdout"));
    let mapp = map.unwrap_or_else(|| PathBuf::from("/dev/null"));
    match driver::transpile_one(&src, &input, &rust, &mapp) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", e);
            ExitCode::FAILURE
        }
    }
}

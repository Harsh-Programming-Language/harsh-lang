// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The build driver.
//!
//! Layout: Harsh sources live in `src/**.hrs`; generated Rust and its source
//! maps go to `target/hrs/`, and `Cargo.toml` points its targets at that tree:
//!
//! ```toml
//! [[bin]]
//! name = "myapp"
//! path = "target/hrs/main.rs"
//! ```
//!
//! One manifest, dependencies untouched, and nothing extra to gitignore since
//! Cargo already ignores `/target`. Plain `cargo build` works once the tree has
//! been transpiled; `hrs build` does both and remaps the diagnostics.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

pub const GEN_DIR: &str = "target/hrs";
pub const SRC_DIR: &str = "src";

pub struct Project {
    pub root: PathBuf,
}

/// What `Project::export` wrote.
pub struct Export {
    pub files: Vec<PathBuf>,
    /// Whether `cargo fmt` ran successfully over the exported tree.
    pub formatted: bool,
}

#[derive(Debug)]
pub struct Unit {
    pub source: PathBuf,
    pub rust: PathBuf,
    pub map: PathBuf,
}

impl Project {
    /// Walk up from the current directory looking for a `Cargo.toml`.
    pub fn find() -> Result<Self, String> {
        let mut dir = std::env::current_dir().map_err(|e| e.to_string())?;
        loop {
            if dir.join("Cargo.toml").is_file() {
                return Ok(Project { root: dir });
            }
            if !dir.pop() {
                return Err("no Cargo.toml found in this directory or any parent".into());
            }
        }
    }

    /// Refuse to build unless `Cargo.toml` points a target at the generated
    /// tree. Without that, cargo auto-discovers `src/main.rs` and quietly builds
    /// the wrong program -- a success that runs something else is worse than
    /// an error.
    pub fn check_manifest(&self) -> Result<(), String> {
        let path = self.root.join("Cargo.toml");
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
        if text.contains(GEN_DIR) {
            return Ok(());
        }
        Err(format!(
            "{} has no target under {}/; cargo would build src/main.rs instead.\n\
             Add one, or run `hrs new` for a project laid out for Harsh:\n\n\
             [[bin]]\nname = \"<name>\"\npath = \"{}/main.rs\"",
            path.display(),
            GEN_DIR,
            GEN_DIR
        ))
    }

    pub fn units(&self) -> Result<Vec<Unit>, String> {
        let src = self.root.join(SRC_DIR);
        if !src.is_dir() {
            return Err(format!("{} does not exist", src.display()));
        }
        let mut out = Vec::new();
        collect(&src, &src, &self.root.join(GEN_DIR), &mut out)?;
        out.sort_by(|a, b| a.source.cmp(&b.source));
        Ok(out)
    }

    /// Transpile everything whose source is newer than its output.
    ///
    /// Returns the units that were rebuilt, and every unit's map path so
    /// diagnostics can be remapped whether or not it was rebuilt this time.
    pub fn transpile(&self, force: bool) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
        self.check_manifest()?;
        let units = self.units()?;
        // Arities of every `fn` in the project, so `$` works across modules.
        // A partial application is a whole-project question, so a change to
        // any file's signatures means every file is checked again.
        let mut arities = std::collections::HashMap::new();
        let mut sources = Vec::new();
        for u in &units {
            let src = fs::read_to_string(&u.source)
                .map_err(|e| format!("{}: {}", u.source.display(), e))?;
            if let Ok(toks) = crate::lex::lex(&src) {
                arities.extend(crate::juxt::collect_arities(&toks));
            }
            sources.push(src);
        }
        let mut rebuilt = Vec::new();
        let mut maps = Vec::new();
        for (u, src) in units.iter().zip(sources) {
            maps.push(u.map.clone());
            if !force && !is_stale(&u.source, &u.rust) {
                continue;
            }
            let rust = transpile_one_with(&src, &u.source, &u.rust, &u.map, &arities)?;
            rebuilt.push(rust);
        }
        Ok((rebuilt, maps))
    }

    /// Write the project as a plain Rust crate under `dir`: `Cargo.toml` with
    /// its targets pointed at `src/`, every `.hrs` transpiled to `src/**.rs`,
    /// no source maps, and `cargo fmt` run over the result when rustfmt is
    /// installed. This is the hand-off artifact -- what a Rust reader, a
    /// reviewer, or a crates.io upload sees. `target/hrs/` is not for
    /// reading: it keeps the source layout so diagnostics map back, and
    /// formatting it would break that map.
    pub fn export(&self, dir: &Path) -> Result<Export, String> {
        self.check_manifest()?;
        let units = self.units()?;
        let mut arities = std::collections::HashMap::new();
        let mut sources = Vec::new();
        for u in &units {
            let src = fs::read_to_string(&u.source)
                .map_err(|e| format!("{}: {}", u.source.display(), e))?;
            if let Ok(toks) = crate::lex::lex(&src) {
                arities.extend(crate::juxt::collect_arities(&toks));
            }
            sources.push(src);
        }
        let src_root = self.root.join(SRC_DIR);
        let out_src = dir.join(SRC_DIR);
        fs::create_dir_all(&out_src).map_err(|e| format!("{}: {}", out_src.display(), e))?;
        let mut files = Vec::new();
        for (u, src) in units.iter().zip(sources) {
            let rel = u.source.strip_prefix(&src_root).unwrap_or(&u.source);
            let rust = out_src.join(rel.with_extension("rs"));
            let toks = crate::lex::lex(&src)
                .map_err(|e| render_error(&u.source, &src, e.span, &e.msg))?;
            let tree = crate::layout::build_with(toks, &arities)
                .map_err(|e| render_error(&u.source, &src, e.span, &e.msg))?;
            let mut em = crate::emit::Emitter::new(&src);
            em.program(&tree);
            // The exported crate is built by cargo alone, doc tests included,
            // so its doc examples are translated here as well.
            crate::docex::to_rust_in(&mut em.out, &mut em.map, &src)
                .map_err(|(span, msg)| render_error(&u.source, &src, span, &msg))?;
            if let Some(d) = rust.parent() {
                fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            fs::write(&rust, &em.out).map_err(|e| format!("{}: {}", rust.display(), e))?;
            files.push(rust);
        }
        // The manifest: the same file with its targets under `src/`, which
        // is also where cargo would find them unaided.
        let manifest = self.root.join("Cargo.toml");
        let text = fs::read_to_string(&manifest).map_err(|e| format!("{}: {}", manifest.display(), e))?;
        let text = text.replace(&format!("{}/", GEN_DIR), &format!("{}/", SRC_DIR));
        fs::write(dir.join("Cargo.toml"), text).map_err(|e| e.to_string())?;
        for extra in ["Cargo.lock", "README.md", "LICENSE"] {
            let from = self.root.join(extra);
            if from.is_file() {
                let _ = fs::copy(&from, dir.join(extra));
            }
        }
        let formatted = Command::new("cargo")
            .arg("fmt")
            .arg("--manifest-path")
            .arg(dir.join("Cargo.toml"))
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        Ok(Export { files, formatted })
    }

    /// Run a cargo subcommand, remapping its diagnostics onto Harsh source.
    pub fn cargo(&self, sub: &str, args: &[String], maps: &[PathBuf]) -> Result<i32, String> {
        // Only cargo's own build commands speak `--message-format`; an
        // external subcommand (`cargo leptos`, `cargo doc`'s cousins) is run
        // plainly, its output passed through unmapped.
        if !matches!(sub, "build" | "run" | "test" | "check" | "clippy" | "bench" | "doc") {
            let status = Command::new("cargo")
                .arg(sub)
                .args(args)
                .current_dir(&self.root)
                .status()
                .map_err(|e| format!("cargo: {}", e))?;
            return Ok(status.code().unwrap_or(1));
        }
        let loaded: Vec<crate::remap::SourceMap> = maps
            .iter()
            .filter(|m| m.is_file())
            .filter_map(|m| crate::remap::SourceMap::load(&m.display().to_string()).ok())
            .collect();

        let mut cmd = Command::new("cargo");
        // Cargo draws its progress bar on stderr with carriage returns; our
        // remapped diagnostics go to stdout of the same terminal, so the first
        // line of an error would land on top of a half-erased bar.
        cmd.env("CARGO_TERM_PROGRESS_WHEN", "never");
        cmd.arg(sub)
            .arg("--message-format=json-diagnostic-rendered-ansi")
            .args(args)
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = cmd.spawn().map_err(|e| format!("cargo: {}", e))?;
        let stdout = child.stdout.take().unwrap();

        let out = std::io::stdout();
        let mut w = out.lock();
        let (mut errors, mut warnings) = (0usize, 0usize);
        // `cargo run` and `cargo test` write program output to the child's
        // stderr and stdout; only the JSON lines are ours to interpret.
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            // Cargo's messages are JSON objects with a `reason`. Anything else
            // is the program's own output -- including a line that happens to
            // parse as JSON, such as `5050` or `true` -- and is passed through.
            let cargo_msg = line.starts_with('{')
                && serde_json::from_str::<serde_json::Value>(&line).map_or(false, |v| v.get("reason").is_some());
            if !cargo_msg {
                let _ = writeln!(w, "{}", line);
                let _ = w.flush();
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            if v["reason"].as_str() != Some("compiler-message") {
                continue;
            }
            let msg = &v["message"];
            let level = msg["level"].as_str().unwrap_or("");
            if level == "failure-note" {
                continue;
            }
            // rustc's own tallies ("1 warning emitted", "aborting due to N
            // previous errors") are messages with no span and no code; they
            // are neither shown nor counted -- the line below is the tally.
            let bare = msg["spans"].as_array().map_or(true, |a| a.is_empty()) && msg["code"].is_null();
            if bare {
                continue;
            }
            let mut shown = false;
            for m in &loaded {
                if crate::remap::render(m, msg, &mut w).unwrap_or(false) {
                    shown = true;
                    break;
                }
            }
            if !shown {
                // Not from generated code -- show cargo's own rendering.
                if let Some(r) = msg["rendered"].as_str() {
                    let _ = write!(w, "{}", r);
                }
            }
            match level {
                "error" => errors += 1,
                "warning" => warnings += 1,
                _ => {}
            }
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        if errors > 0 || warnings > 0 {
            let _ = writeln!(w, "hrs: {} error(s), {} warning(s)", errors, warnings);
        }
        let _ = w.flush();
        Ok(status.code().unwrap_or(1))
    }
}

fn collect(dir: &Path, src_root: &Path, gen_root: &Path, out: &mut Vec<Unit>) -> Result<(), String> {
    for e in fs::read_dir(dir).map_err(|e| format!("{}: {}", dir.display(), e))? {
        let p = e.map_err(|e| e.to_string())?.path();
        if p.is_dir() {
            collect(&p, src_root, gen_root, out)?;
        } else if p.extension().map(|x| x == "hrs").unwrap_or(false) {
            let rel = p.strip_prefix(src_root).unwrap_or(&p).to_path_buf();
            let rust = gen_root.join(rel.with_extension("rs"));
            let map = gen_root.join(rel.with_extension("map.json"));
            out.push(Unit { source: p, rust, map });
        }
    }
    Ok(())
}

/// Equal mtimes count as stale. A source saved within the same clock tick as
/// the previous transpile would otherwise be skipped forever; the cost of the
/// tie going the other way is one redundant transpile.
/// A generated file is also stale when the transpiler itself is newer than
/// it: a `cargo install` of a new `hrs` must not leave the Rust the old one
/// wrote in place. The executable's own mtime is the version stamp.
fn is_stale(source: &Path, rust: &Path) -> bool {
    let m = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
    let exe = std::env::current_exe().ok().and_then(|p| m(&p));
    match (m(source), m(rust)) {
        (Some(a), Some(b)) => a >= b || exe.map_or(false, |e| e >= b),
        _ => true,
    }
}

/// Transpile one file, writing the Rust and its source map.
pub fn transpile_one(
    src: &str,
    from: &Path,
    rust: &Path,
    map: &Path,
) -> Result<PathBuf, String> {
    let toks = crate::lex::lex(src).map_err(|e| render_error(from, src, e.span, &e.msg))?;
    let arities = crate::juxt::collect_arities(&toks);
    transpile_tokens(toks, src, from, rust, map, &arities)
}

/// Transpile one file with arities known from the whole project.
pub fn transpile_one_with(
    src: &str,
    from: &Path,
    rust: &Path,
    map: &Path,
    arities: &std::collections::HashMap<String, usize>,
) -> Result<PathBuf, String> {
    let toks = crate::lex::lex(src).map_err(|e| render_error(from, src, e.span, &e.msg))?;
    transpile_tokens(toks, src, from, rust, map, arities)
}

fn transpile_tokens(
    toks: Vec<crate::lex::Token>,
    src: &str,
    from: &Path,
    rust: &Path,
    map: &Path,
    arities: &std::collections::HashMap<String, usize>,
) -> Result<PathBuf, String> {
    let tree = crate::layout::build_with(toks, arities)
        .map_err(|e| render_error(from, src, e.span, &e.msg))?;
    let mut em = crate::emit::Emitter::new(src);
    // `include_str!` paths are relative to the source file; the Rust is
    // written elsewhere, so they are re-based from there.
    if let (Some(fd), Some(rd)) = (from.parent(), rust.parent()) {
        em.set_include_base(fd, rd);
    }
    em.program(&tree);
    // A fenced doc example is Harsh; rustdoc reads Rust. Rewriting the
    // generated text (and shifting the map by what it changes) keeps the
    // comment a comment everywhere else in the transpiler.
    crate::docex::to_rust_in(&mut em.out, &mut em.map, src)
        .map_err(|(span, msg)| render_error(from, src, span, &msg))?;

    if let Some(d) = rust.parent() {
        fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    fs::write(rust, &em.out).map_err(|e| format!("{}: {}", rust.display(), e))?;

    let mut j = String::from("{\"version\":1,\"source\":");
    json_str(&mut j, &from.display().to_string());
    j.push_str(",\"generated\":");
    json_str(&mut j, &rust.display().to_string());
    j.push_str(",\"entries\":[");
    for (k, e) in em.map.iter().enumerate() {
        if k > 0 {
            j.push(',');
        }
        j.push_str(&format!("[{},{},{},{}]", e.gen_lo, e.gen_hi, e.src_lo, e.src_hi));
    }
    j.push_str("]}");
    fs::write(map, j).map_err(|e| format!("{}: {}", map.display(), e))?;
    Ok(rust.to_path_buf())
}

fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A transpile error, rendered against the Harsh source in rustc's style.
pub fn render_error(path: &Path, src: &str, span: crate::lex::Span, msg: &str) -> String {
    let off = span.lo as usize;
    let (mut line, mut start) = (1usize, 0usize);
    for (i, c) in src.char_indices() {
        if i >= off {
            break;
        }
        if c == '\n' {
            line += 1;
            start = i + 1;
        }
    }
    let end = src[start..].find('\n').map(|k| start + k).unwrap_or(src.len());
    let col = src[start..off.min(end)].chars().count() + 1;
    let width = (span.hi - span.lo).max(1) as usize;
    format!(
        "error: {}\n  --> {}:{}:{}\n   |\n{:>3}| {}\n   | {}{}",
        msg,
        path.display(),
        line,
        col,
        line,
        &src[start..end],
        " ".repeat(col.saturating_sub(1)),
        "^".repeat(width)
    )
}

/// Poll the source tree for changes. Polling rather than filesystem events:
/// no dependency, no platform differences, and editors write partial files and
/// fire several events per save anyway.
pub fn watch<F: FnMut()>(project: &Project, mut on_change: F) -> Result<(), String> {
    let snapshot = |p: &Project| -> BTreeMap<PathBuf, SystemTime> {
        let mut m = BTreeMap::new();
        if let Ok(units) = p.units() {
            for u in units {
                if let Ok(t) = fs::metadata(&u.source).and_then(|x| x.modified()) {
                    m.insert(u.source, t);
                }
            }
        }
        m
    };
    let mut seen: BTreeMap<PathBuf, SystemTime> = snapshot(project);
    on_change();
    loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
        let now = snapshot(project);
        if now != seen {
            // Settle: editors write in stages, so wait for quiet.
            std::thread::sleep(std::time::Duration::from_millis(120));
            seen = snapshot(project);
            let _ = now;
            on_change();
        }
    }
}

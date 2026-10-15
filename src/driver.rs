// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The build driver.
//!
//! Layout (0.9.0, the user's design of 2026-10-14): Harsh sources live in
//! `src/**.hrs`, and `export/` beside them is the whole Rust project they
//! make -- `export/Cargo.toml`, written from the project's `Cargo.toml` and
//! `Hrs.toml`; `export/src/**.rs`, the transpilation; the project's other
//! files (`assets/`, `Dioxus.toml`, `.gitignore`, …) mirrored in; and
//! `export/target/`, cargo's. The project's root has no `target/`. `export/`
//! holds Rust only: `hrs`'s own working files -- the source maps, the stamp,
//! the proc-macro runner and the distribution it builds with -- live in
//! `.hrs/` at the project's root, as git's do in `.git/` (the user,
//! 2026-10-14).
//!
//! `hrs build` is two passes: pass 1 transpiles the project and its Harsh
//! crates and writes `export/` (`Project::prepare`); pass 2 is Cargo, run in
//! `export/`, its diagnostics remapped onto the Harsh. `export/` is also
//! what crates.io receives: `hrs -c publish` is `cargo publish` there.

use std::collections::BTreeMap;
use std::fs;
#[cfg(feature = "remap")]
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(feature = "remap")]
use std::process::Stdio;
use std::time::SystemTime;

/// The generated Rust project (0.9.0; `target/src/` before).
pub const EXPORT_DIR: &str = "export";
/// Where the transpiled files go, mirroring `src/`.
pub const GEN_DIR: &str = "export/src";
/// `hrs`'s own working files: source maps, the stamp, the proc-macro
/// runner, the distribution it builds with -- Harsh's, not the Rust's, so
/// outside `export/` (the user, 2026-10-14).
pub const WORK_DIR: &str = ".hrs";
pub const SRC_DIR: &str = "src";

pub struct Project {
    pub root: PathBuf,
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
        Self::find_from(std::env::current_dir().map_err(|e| e.to_string())?)
    }

    /// The project `dir` belongs to: the nearest directory, from `dir`
    /// upwards, holding a `Cargo.toml`.
    pub fn find_from(mut dir: PathBuf) -> Result<Self, String> {
        loop {
            if dir.join("Cargo.toml").is_file() {
                return Ok(Project { root: dir });
            }
            if !dir.pop() {
                return Err("no Cargo.toml found in this directory or any parent".into());
            }
        }
    }

    /// Refuse a manifest an older `hrs` laid out: its targets under
    /// `target/hrs/` (before 0.7.0) or `target/src/` (before 0.9.0), where
    /// nothing is generated any more -- `hrs migrate` updates it. And a Harsh
    /// crate named in `Cargo.toml` (it belongs in `Hrs.toml`, 0.3.0).
    pub fn check_manifest(&self) -> Result<(), String> {
        let path = self.root.join("Cargo.toml");
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
        for (old, until) in [("target/hrs/", "0.7.0"), ("target/src/", "0.9.0")] {
            if text.lines().any(|l| !l.trim_start().starts_with('#') && l.contains(old)) {
                return Err(format!(
                    "{}: points at `{old}`, where `hrs` generated the Rust before {until}; it is `{GEN_DIR}/` now -- run `hrs migrate` once to update the project",
                    path.display()
                ));
            }
        }
        #[cfg(feature = "remap")]
        {
            let found = harsh_crates_in_cargo(&text);
            if !found.is_empty() {
                return Err(format!(
                    "{}: {} is Harsh's, and since 0.3.0 belongs in Hrs.toml beside it -- run `hrs migrate` to move it",
                    path.display(),
                    found.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", ")
                ));
            }
        }
        Ok(())
    }

    /// The workspace this project is a member of -- or whose root it is,
    /// with a `[package]` of its own -- if any.
    pub fn workspace(&self) -> Option<PathBuf> {
        workspace_of(&self.root)
    }

    /// Where this project stands in its workspace: `crates/util`; empty at
    /// the root, or outside a workspace.
    fn in_workspace(&self) -> Option<(PathBuf, PathBuf)> {
        let w = self.workspace()?;
        let rel = self.root.strip_prefix(&w).map(Path::to_path_buf).unwrap_or_default();
        Some((w, rel))
    }

    /// The Rust project's folder: `export/`, or in a workspace (0.9.0,
    /// the user's option A, 2026-10-14) the member's place in the
    /// workspace root's `export/` -- `<root>/export/crates/util/`.
    pub fn export_dir(&self) -> PathBuf {
        match self.in_workspace() {
            Some((w, rel)) => w.join(EXPORT_DIR).join(rel),
            None => self.root.join(EXPORT_DIR),
        }
    }

    /// Where the transpiled files go: `src/` in the Rust project.
    pub fn gen_dir(&self) -> PathBuf {
        self.export_dir().join(SRC_DIR)
    }

    /// `hrs`'s own working files: `.hrs/`, or a member's under its
    /// workspace root's, `.hrs/members/crates/util/`.
    pub fn work_dir(&self) -> PathBuf {
        match self.in_workspace() {
            Some((w, rel)) if !rel.as_os_str().is_empty() => w.join(WORK_DIR).join("members").join(rel),
            _ => self.root.join(WORK_DIR),
        }
    }

    /// The folder cargo works from, holding `Cargo.lock` and `target/`:
    /// the `export/` of the workspace, or of the project.
    pub fn cargo_root(&self) -> PathBuf {
        self.lock_root().join(EXPORT_DIR)
    }

    /// The folder whose `Cargo.lock` is the user's: the workspace root, or
    /// the project's.
    pub fn lock_root(&self) -> PathBuf {
        self.workspace().unwrap_or_else(|| self.root.clone())
    }

    /// The source map of a generated file, `export/src/a/b.rs` ->
    /// `.hrs/maps/a/b.map.json`; `None` for a file outside `export/src/`.
    pub fn map_of(&self, rust: &Path) -> Option<PathBuf> {
        let rel = rust.strip_prefix(self.gen_dir()).ok()?;
        Some(self.work_dir().join("maps").join(rel.with_extension("map.json")))
    }

    pub fn units(&self) -> Result<Vec<Unit>, String> {
        let src = self.root.join(SRC_DIR);
        if !src.is_dir() {
            return Err(format!("{} does not exist", src.display()));
        }
        let mut out = Vec::new();
        collect(&src, &src, &self.gen_dir(), &self.work_dir().join("maps"), &mut out)?;
        out.sort_by(|a, b| a.source.cmp(&b.source));
        Ok(out)
    }

    /// Transpile everything whose source is newer than its output.
    ///
    /// Returns the units that were rebuilt, and every unit's map path so
    /// diagnostics can be remapped whether or not it was rebuilt this time.
    pub fn transpile(&self, force: bool) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
        // The Harsh libraries this project depends on by path are transpiled
        // first, recursively, so cargo finds their Rust: a Harsh app using a
        // Harsh library, `add_one = { path = "../add_one" }` (found
        // 2026-09-25: it never worked, though the Book's chapter 17 says `hrs`
        // walks each member).
        self.transpile_path_dependencies(&mut vec![self.root.canonicalize().unwrap_or(self.root.clone())])?;
        self.transpile_only(force)
    }

    /// Transpile this project's own files (its path dependencies are the
    /// caller's business).
    pub fn transpile_only(&self, force: bool) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
        self.check_manifest()?;
        let units = self.units()?;
        // Arities of every `fn` in the project, so `$` works across modules.
        // A partial application is a whole-project question, so a change to
        // any file's signatures means every file is checked again.
        crate::juxt::reset_constructors();
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
        check_hrs_std(&self.root, &units, &sources)?;
        // Harsh proc macros (ruling 17): this crate's own registrations are
        // checked against its manifest, and the crates it uses are built
        // into a runner before any of its files is expanded.
        self.registrations()?;
        let runner = self.proc_runner()?;
        let procs: &dyn crate::mac::ProcMacros = match &runner {
            Some(r) => r,
            None => &crate::mac::NoProcMacros,
        };
        // A runner rebuilt after a file was transpiled may expand that
        // file's calls differently: the file is stale.
        let runner_time = runner.as_ref().and_then(|r| fs::metadata(&r.bin).and_then(|m| m.modified()).ok());
        // A different `hrs` may transpile a file differently: every file is
        // stale once when the stamp of the `hrs` that last transpiled the
        // project differs from this one's (found 2026-09-24: after an
        // upgrade, a project kept the old version's Rust until its sources
        // changed). A stamp, not the files' times: `hrs` rewrites a generated
        // file only when its content changes, so a file the new `hrs` writes
        // identically would stay "older" for ever.
        // Before 0.7.0 the generated tree was `target/hrs/`. Left behind, it
        // fooled Cargo: a path package is the same package wherever its
        // manifest lies, and the old build's recorded source -- the stale
        // `target/hrs/main.rs`, unchanged -- made every later build look
        // fresh, so edits were silently ignored (found 2026-10-06, by
        // Harshlings' shared project). It is `hrs`'s own output: removed.
        let old_gen = self.root.join("target").join("hrs");
        if old_gen.is_dir() {
            let _ = fs::remove_dir_all(&old_gen);
        }
        // So was `target/src/`, from 0.7.0 to 0.8.5 (its stamp says so).
        let old_src = self.root.join("target").join("src");
        if old_src.join(".hrs-stamp").is_file() {
            let _ = fs::remove_dir_all(&old_src);
        }
        let stamp_path = self.work_dir().join("stamp");
        // Whether `hrs_std` is folded into the crate is part of what the Rust
        // is: a project that starts or stops listing it is transpiled afresh.
        let fold = lists_hrs_std(&self.root);
        set_fold_std(fold);
        let stamp = format!("{}{}", hrs_stamp(), if fold { " fold" } else { "" });
        let new_hrs = fs::read_to_string(&stamp_path).ok().as_deref() != Some(stamp.as_str());
        // A `use` naming a Harsh macro crate -- a re-export,
        // `pub use hello_macro_derive.HelloMacro` -- has no Rust meaning: it
        // is blanked, byte for byte, before the file is transpiled (the user,
        // 2026-09-24, option (a); `PACKAGING.md` section 5).
        let macro_libs = self.macro_crate_libs()?;
        let mut rebuilt = Vec::new();
        let mut maps = Vec::new();
        for (u, src) in units.iter().zip(sources) {
            let src = blank_macro_uses(&src, &macro_libs);
            maps.push(u.map.clone());
            // (A named function, not a closure over lines: the converter's
            // reflow split this closure's call across lines in a form the
            // transpiler refuses -- found by the self-host, 2026-09-24.)
            let older_than_runner = runner_time.map_or(false, |t| older_than(&u.rust, t));
            if !force && !older_than_runner && !new_hrs && !is_stale(&u.source, &u.rust) && u.map.is_file() {
                continue;
            }
            let rust = transpile_one_procs(&src, &u.source, &u.rust, &u.map, &arities, procs)?;
            rebuilt.push(rust);
        }
        if new_hrs {
            let _ = fs::create_dir_all(self.work_dir());
            let _ = fs::write(&stamp_path, &stamp);
        }
        Ok((rebuilt, maps))
    }

    fn manifest_text(&self) -> Result<String, String> {
        let path = self.root.join("Cargo.toml");
        fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))
    }

    /// `[package.metadata.harsh]`, read from this project's manifest.
    pub fn meta(&self) -> Result<crate::procmac::Meta, String> {
        crate::procmac::read_meta(&self.manifest_text()?)
            .map_err(|e| format!("{}: {e}", self.root.join("Cargo.toml").display()))
    }

    /// The proc macros this crate registers, checked against its manifest:
    /// a crate marked `proc-macro = true` registers at least one, in its
    /// root (`src/lib.hrs` -- the runner calls `crate::name`), and uses none
    /// itself; a crate not marked registers none.
    pub fn registrations(&self) -> Result<Vec<crate::procmac::Registration>, String> {
        let meta = self.meta()?;
        let manifest = self.root.join("Cargo.toml");
        let root_file = self.root.join(SRC_DIR).join("lib.hrs");
        let mut names = Vec::new();
        for u in self.units()? {
            let src = fs::read_to_string(&u.source).map_err(|e| format!("{}: {}", u.source.display(), e))?;
            let (work1, _) = crate::rawzone::prepare(&src);
            let (work, _) = crate::dslzone::prepare(&work1);
            let (_, regs) = crate::procmac::prepare(&work).map_err(|e| render_error(&u.source, &src, e.span, &e.msg))?;
            let Some(first) = regs.first() else { continue };
            if !meta.proc_macro {
                return Err(render_error(
                    &u.source,
                    &src,
                    first.span,
                    &format!(
                        "`#[proc_macro~]` in a crate that is not a proc-macro crate: add\n\n    \
                         [package.metadata.harsh]\n    proc-macro = true\n\nto {}",
                        manifest.display()
                    ),
                ));
            }
            if u.source != root_file {
                return Err(render_error(
                    &u.source,
                    &src,
                    first.span,
                    "a Harsh proc macro is registered in the crate's root, `src/lib.hrs`",
                ));
            }
            names.extend(regs);
        }
        if meta.proc_macro && names.is_empty() {
            return Err(format!(
                "{}: `proc-macro = true`, but no `pub fn` in {} is marked `#[proc_macro~]`",
                manifest.display(),
                root_file.display()
            ));
        }
        if meta.proc_macro && !meta.proc_macros.is_empty() {
            return Err(format!(
                "{}: a proc-macro crate does not use Harsh proc macros itself (not in this version)",
                manifest.display()
            ));
        }
        Ok(names)
    }

    /// Transpile each Harsh project this one depends on by path, deepest
    /// first; `seen` guards against visiting one twice.
    fn transpile_path_dependencies(&self, seen: &mut Vec<PathBuf>) -> Result<(), String> {
        let Ok(text) = self.manifest_text() else { return Ok(()) };
        for path in path_dependencies(&text) {
            let root = self.root.join(&path);
            let canon = root.canonicalize().unwrap_or(root.clone());
            if seen.contains(&canon) {
                continue;
            }
            seen.push(canon.clone());
            // By its own name, not through this one's: `../crates/util`.
            let dep = Project { root: canon };
            // Harsh when it has `.hrs` sources; a Rust crate is cargo's.
            if dep.units().map_or(true, |u| u.is_empty()) {
                continue;
            }
            dep.transpile_path_dependencies(seen)?;
            dep.transpile_only(false).map_err(|e| format!("in the Harsh library `{path}`:\n{e}"))?;
        }
        Ok(())
    }

    /// The library names of the Harsh macro crates this project lists.
    pub fn macro_crate_libs(&self) -> Result<Vec<String>, String> {
        let mut libs = Vec::new();
        for rel in &self.meta()?.proc_macros {
            let dep = Project { root: self.root.join(rel) };
            if let Ok(text) = dep.manifest_text() {
                if let Some(p) = crate::procmac::package_name(&text) {
                    libs.push(p.replace('-', "_"));
                }
            }
        }
        Ok(libs)
    }

    /// Harsh macros re-exported by this project's path dependencies:
    /// `(macro crate's directory, the names re-exported)` -- a library that
    /// lists a macro crate under `proc-macros` and says
    /// `pub use hello_macro_derive.HelloMacro` hands `HelloMacro` to every
    /// crate that depends on it, as a Rust re-export does.
    fn reexported_macros(&self) -> Result<Vec<(PathBuf, Vec<String>)>, String> {
        let mut out: Vec<(PathBuf, Vec<String>)> = Vec::new();
        for path in path_dependencies(&self.manifest_text()?) {
            let dep = Project { root: self.root.join(&path) };
            let Ok(meta) = dep.meta() else { continue };
            for rel in &meta.proc_macros {
                let mroot = dep.root.join(rel);
                let Ok(text) = (Project { root: mroot.clone() }).manifest_text() else { continue };
                let Some(pkg) = crate::procmac::package_name(&text) else { continue };
                let lib = pkg.replace('-', "_");
                let mut names = Vec::new();
                for u in dep.units().unwrap_or_default() {
                    let src = fs::read_to_string(&u.source).unwrap_or_default();
                    names.extend(reexports_of(&src, &lib));
                }
                if !names.is_empty() {
                    let mroot = mroot.canonicalize().unwrap_or(mroot);
                    out.push((mroot, names));
                }
            }
        }
        Ok(out)
    }

    /// Build the runner of the proc macros this project uses, if it uses
    /// any: each listed crate transpiled, the runner's sources written under
    /// `.hrs/proc-macros/` when they change, and cargo run over it --
    /// cargo's own freshness check is the cache (decision P4).
    pub fn proc_runner(&self) -> Result<Option<crate::procmac::Runner>, String> {
        let meta = self.meta()?;
        // The macro crates to build the runner from: those this project lists
        // (every macro), and those its path dependencies re-export (the names
        // re-exported).
        let mut wanted: Vec<(String, PathBuf, Option<Vec<String>>)> = Vec::new();
        for rel in &meta.proc_macros {
            wanted.push((rel.clone(), self.root.join(rel), None));
        }
        for (root, names) in self.reexported_macros()? {
            if !wanted.iter().any(|(_, r, _)| r.canonicalize().ok().as_ref() == Some(&root)) {
                wanted.push((root.display().to_string(), root, Some(names)));
            }
        }
        if wanted.is_empty() {
            return Ok(None);
        }
        let mut crates: Vec<crate::procmac::MacroCrate> = Vec::new();
        let mut runtime: Option<String> = None;
        for (rel, root, only) in &wanted {
            let rel = rel.as_str();
            let root = root.clone();
            let root = root.canonicalize().map_err(|_| {
                format!("{}: proc-macros: `{rel}` is not a directory", self.root.join("Cargo.toml").display())
            })?;
            let dep = Project { root: root.clone() };
            let text = dep.manifest_text()?;
            if !dep.meta()?.proc_macro {
                return Err(format!(
                    "{}: proc-macros: `{rel}` is not a Harsh proc-macro crate (its manifest has no `proc-macro = true` under [package.metadata.harsh])",
                    self.root.join("Cargo.toml").display()
                ));
            }
            dep.transpile(false).map_err(|e| format!("in the proc-macro crate `{rel}`:\n{e}"))?;
            let mut macros = dep.registrations()?;
            if let Some(names) = only {
                macros.retain(|m| names.contains(&m.name));
            }
            for m in &macros {
                if let Some(other) = crates.iter().find(|c| c.macros.iter().any(|o| o.name == m.name)) {
                    return Err(format!("the proc macro `{}` is defined by both `{}` and `{rel}`", m.name, other.package));
                }
            }
            let package = crate::procmac::package_name(&text).ok_or_else(|| format!("{}: no [package] name", root.display()))?;
            if runtime.is_none() {
                runtime = crate::procmac::runtime_dependency(&text, &root);
            }
            // The runner builds the crate's Rust: its `export/`.
            #[cfg(feature = "remap")]
            let root = {
                dep.prepare(&mut Vec::new())?;
                dep.export_dir()
            };
            crates.push(crate::procmac::MacroCrate { lib: package.replace('-', "_"), package, root, macros });
        }
        let runtime = runtime.ok_or_else(|| {
            "a Harsh proc-macro crate depends on `hrs_proc_macro`; none of the listed crates does".to_string()
        })?;
        let dir = self.work_dir().join("proc-macros");
        fs::create_dir_all(&dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
        let (manifest, main) = crate::procmac::runner_sources(&crates, &runtime);
        for (name, text) in [("Cargo.toml", &manifest), ("main.rs", &main)] {
            let path = dir.join(name);
            if fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
                fs::write(&path, text).map_err(|e| format!("{}: {}", path.display(), e))?;
            }
        }
        let target = dir.join("target");
        #[cfg(feature = "remap")]
        let patches = {
            let mut manifests = vec![dir.join("Cargo.toml")];
            manifests.extend(crates.iter().map(|c| c.root.join("Cargo.toml")));
            distribution(&self.root, &manifests)?
        };
        #[cfg(not(feature = "remap"))]
        let patches: Vec<String> = Vec::new();
        let status = Command::new("cargo")
            .env("CARGO_TERM_PROGRESS_WHEN", "never")
            .args(&patches)
            .arg("build")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(dir.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target)
            .status()
            .map_err(|e| format!("cargo: {e}"))?;
        if !status.success() {
            return Err(format!("the proc-macro runner did not build ({})", dir.display()));
        }
        let bin = target.join("debug").join(format!("hrs-proc-runner{}", std::env::consts::EXE_SUFFIX));
        let macros = crates.into_iter().flat_map(|c| c.macros).collect();
        Ok(Some(crate::procmac::Runner { bin, macros }))
    }

    /// Pass 1's second half (0.9.0, the user's design of 2026-10-14):
    /// write `export/` as the whole Rust project -- its `Cargo.toml`, from
    /// the project's `Cargo.toml` and `Hrs.toml`; `hrs_std` folded into
    /// `export/src/hrs_std/` when the code uses it; the project's other files
    /// mirrored in; the lockfile copied in. Each Harsh crate `Hrs.toml`
    /// names by path is transpiled and prepared first, and named in
    /// `export/Cargo.toml` by its own `export/` and its version. The path of
    /// `export/Cargo.toml`.
    ///
    /// In a workspace, the whole workspace is prepared: cargo, run in any
    /// member, reads every member's manifest.
    #[cfg(feature = "remap")]
    pub fn prepare(&self, seen: &mut Vec<PathBuf>) -> Result<PathBuf, String> {
        if let Some(w) = self.workspace() {
            prepare_workspace(&w, seen)?;
            return Ok(self.export_dir().join("Cargo.toml"));
        }
        self.prepare_own(seen)
    }

    /// `prepare` for this project alone.
    #[cfg(feature = "remap")]
    fn prepare_own(&self, seen: &mut Vec<PathBuf>) -> Result<PathBuf, String> {
        let export = self.export_dir();
        let ws = self.workspace();
        // A path inside the workspace is that place in its `export/`: a
        // member transpiled there, or the rest mirrored.
        let in_ws_export = |dir: &Path| -> Option<PathBuf> {
            let w = ws.as_ref()?;
            let (cw, cd) = (w.canonicalize().ok()?, dir.canonicalize().ok()?);
            cd.strip_prefix(&cw).ok().map(|rel| w.join(EXPORT_DIR).join(rel))
        };
        // The Cargo.toml `Hrs.toml` names (`[package] cargo`), else the one
        // beside it; its relative paths are read from its own folder.
        let hrs_manifest = read_hrs_manifest(&self.root)?;
        let manifest_path = hrs_manifest.as_ref().map_or_else(|| self.root.join("Cargo.toml"), |m| m.cargo.clone());
        let cargo_dir = manifest_path.parent().unwrap_or(&self.root).to_path_buf();
        let cargo_text = fs::read_to_string(&manifest_path).map_err(|e| format!("{}: {}", manifest_path.display(), e))?;
        let mut extra = Vec::new();
        if let Some(m) = hrs_manifest {
            for (name, value) in &m.deps {
                // `hrs_std` is folded into the crate below; it is no
                // dependency of the Rust.
                if name == "hrs_std" {
                    continue;
                }
                let mut value = value.clone();
                if let Some(i) = value.find("path = \"") {
                    let at = i + 8;
                    let Some(len) = value[at..].find('"') else {
                        return Err(format!("{}: `{name}`: an unclosed path", HRS_MANIFEST));
                    };
                    let rel = value[at..at + len].to_string();
                    // Harsh's own Rust crates -- `hrs_proc_macro` from a
                    // checkout -- passed through, the path made absolute.
                    if DISTRIBUTION.contains(&name.as_str()) {
                        value.replace_range(at..at + len, &manifest_path_str(&self.root.join(&rel)));
                        extra.push(format!("{name} = {value}"));
                        continue;
                    }
                    let dir = self.root.join(&rel);
                    let dir = dir
                        .canonicalize()
                        .map_err(|e| format!("{}: the Harsh crate `{name}` at {}: {e}", HRS_MANIFEST, dir.display()))?;
                    let dep = Project { root: dir.clone() };
                    if !seen.contains(&dir) {
                        seen.push(dir.clone());
                        dep.transpile(false)?;
                        dep.prepare(seen)?;
                    }
                    // Named by its own `export/`, with its version: the path
                    // while building, the version once it is on crates.io
                    // (as cargo's own workspaces are published).
                    // In the workspace, the path as written finds it there.
                    if in_ws_export(&dir).is_none() {
                        value.replace_range(at..at + len, &manifest_path_str(&dep.export_dir()));
                    }
                    if !value.contains("version") {
                        if let Some(v) = dep.manifest_text().ok().as_deref().and_then(package_version) {
                            value = value.replacen('{', &format!("{{ version = \"{v}\","), 1);
                        }
                    }
                }
                extra.push(format!("{name} = {value}"));
            }
        }
        // Outside a workspace `export/Cargo.toml` is its own, so inherited
        // fields are written out; in one, cargo reads the workspace's.
        let cargo_text = if ws.is_some() { cargo_text } else { resolve_workspace(&cargo_text, &cargo_dir) };
        // A Harsh crate belongs in Hrs.toml only (0.3.0): named in Cargo.toml
        // too, it would be a duplicate key in the generated manifest. Said
        // plainly, rather than merged (ShopBill's S3, 2026-10-06).
        let in_cargo = cargo_dependency_names(&cargo_text);
        for line in &extra {
            let name = line.split('=').next().unwrap_or("").trim();
            if in_cargo.iter().any(|n| n == name) {
                return Err(format!("`{name}` is in both Cargo.toml and {HRS_MANIFEST}: a Harsh crate belongs in {HRS_MANIFEST} only -- remove it from Cargo.toml"));
            }
        }
        // `hrs_std`, folded: the items the code reaches, as the crate's own
        // module, and its Rust dependencies in the manifest.
        // A project that stopped listing it keeps no copy behind.
        let folded = if lists_hrs_std(&self.root) {
            self.fold_hrs_std()?
        } else {
            // (Unless `src/hrs_std/` is the project's own module.)
            if !self.root.join(SRC_DIR).join("hrs_std").exists() {
                let _ = fs::remove_dir_all(self.gen_dir().join("hrs_std"));
            }
            false
        };
        if folded {
            extra.extend(hrs_std_dependencies());
        }
        // Plain Rust: no Harsh table, nor a comment about the Harsh sources,
        // which `export/` does not have.
        let plain: String = without_harsh_table(&cargo_text)
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                !(t.starts_with('#') && !t.starts_with("#[") && speaks_of_harsh(t))
            })
            .map(|l| format!("{l}\n"))
            .collect();
        let mirror_root = ws.clone().unwrap_or_else(|| self.root.clone());
        // A Cargo.toml path dependency on a Harsh project -- a library beside
        // it with `.hrs` sources -- is that project's `export/`.
        let text = generated_manifest(&plain, &cargo_dir, &extra, ws.is_none(), &mirror_root, &mut |dir: &Path| {
            // In the workspace: mirrored or transpiled at the same place.
            if in_ws_export(dir).is_some() {
                return None;
            }
            let dep = Project { root: dir.to_path_buf() };
            if dep.units().map_or(true, |u| u.is_empty()) {
                return None;
            }
            let canon = dir.canonicalize().unwrap_or(dir.to_path_buf());
            if !seen.contains(&canon) {
                seen.push(canon);
                let _ = dep.prepare(seen);
            }
            Some(dep.export_dir())
        })?;
        let gen = export.join("Cargo.toml");
        fs::create_dir_all(&export).map_err(|e| format!("{}: {e}", export.display()))?;
        if fs::read_to_string(&gen).ok().as_deref() != Some(text.as_str()) {
            fs::write(&gen, &text).map_err(|e| format!("{}: {e}", gen.display()))?;
        }
        let skip = ws.as_deref().map(harsh_members).unwrap_or_default();
        link_project_dirs(&self.root, &export, &skip);
        mirror_src_extras(&self.root.join(SRC_DIR), &self.gen_dir());
        if ws.is_none() {
            export_extras(&self.root);
        }
        Ok(gen)
    }

    /// Fold `hrs_std` into `export/src/hrs_std/` when the transpiled code
    /// names it (the files already say `crate::hrs_std::`, written so by the
    /// transpiler when the project lists `hrs_std`): only the items the code
    /// reaches, closed over what each uses (`shake`), and a `mod hrs_std;`
    /// at the end of each crate root, where it moves no line of the map.
    /// Whether it was used.
    #[cfg(feature = "remap")]
    fn fold_hrs_std(&self) -> Result<bool, String> {
        let gen = self.gen_dir();
        let module = gen.join("hrs_std");
        let units = self.units()?;
        let macros = hrs_std_macros();
        let uses = |t: &str| t.contains("crate::hrs_std::") || macros.iter().any(|m| t.contains(&format!("crate::{m}!")));
        let files: Vec<PathBuf> = units.iter().map(|u| u.rust.clone()).filter(|f| fs::read_to_string(f).map_or(false, |t| uses(&t))).collect();
        // Each crate root declares the module, at its end: `main.rs`,
        // `lib.rs`, and each binary of `src/bin/`, which reaches it by path.
        let mut roots_files: Vec<(PathBuf, String)> = Vec::new();
        for root in ["main.rs", "lib.rs"] {
            roots_files.push((gen.join(root), "mod hrs_std;".into()));
        }
        if let Ok(rd) = fs::read_dir(gen.join("bin")) {
            for e in rd.flatten() {
                if e.path().extension().map_or(false, |x| x == "rs") {
                    roots_files.push((e.path(), "#[path = \"../hrs_std/mod.rs\"]\nmod hrs_std;".into()));
                }
            }
        }
        let decl_of = |decl: &str| format!("\n/// `hrs_std`: the parts of it this crate uses.\n#[allow(dead_code, unused_imports, unused_macros)]\n{decl}\n");
        if files.is_empty() {
            if module.is_dir() {
                let _ = fs::remove_dir_all(&module);
            }
            // A root not transpiled again since it last declared the module
            // would name a folder that is gone.
            for (f, decl) in &roots_files {
                let Ok(t) = fs::read_to_string(f) else { continue };
                let d = decl_of(decl);
                if let Some(at) = t.rfind(&d) {
                    let mut t2 = t[..at].to_string();
                    t2.push_str(&t[at + d.len()..]);
                    fs::write(f, t2).map_err(|e| format!("{}: {e}", f.display()))?;
                }
            }
            return Ok(false);
        }
        let mut vendored: Vec<(String, String)> = Vec::new();
        for (path, text) in crate::dist::FILES {
            let Some(name) = path.strip_prefix("hrs_std/src/") else { continue };
            let target = if name == "lib.rs" { module.join("mod.rs") } else { module.join(name) };
            let text = strip_test_modules(&vendor_paths(&without_harsh_notes(text), "crate::", "hrs_std"));
            vendored.push((target.display().to_string(), text));
        }
        let mut roots = std::collections::HashSet::new();
        for u in &units {
            if let Ok(t) = fs::read_to_string(&u.rust) {
                if let Ok(toks) = crate::lex::lex_rust(&t) {
                    roots.extend(toks.into_iter().filter(|x| x.kind == crate::lex::Tk::Ident).map(|x| x.text));
                }
            }
        }
        fs::create_dir_all(&module).map_err(|e| format!("{}: {e}", module.display()))?;
        for (target, text) in crate::shake::shake(&vendored, &roots) {
            if fs::read_to_string(&target).ok().as_deref() != Some(text.as_str()) {
                fs::write(&target, text).map_err(|e| format!("{target}: {e}"))?;
            }
        }
        for (f, decl) in roots_files {
            let Ok(t) = fs::read_to_string(&f) else { continue };
            let decl = decl_of(&decl);
            if !t.contains(&decl) {
                let t = format!("{}{decl}", t.trim_end_matches('\n').to_string() + "\n");
                fs::write(&f, t).map_err(|e| format!("{}: {e}", f.display()))?;
            }
        }
        Ok(true)
    }

    /// Run another tool -- `dx`, Dioxus's; `cargo`, for `hrs -c` -- on the
    /// Rust project after pass 1, in `export/`, where its `Cargo.toml` and
    /// the project's other files are.
    #[cfg(feature = "remap")]
    pub fn tool(&self, prog: &str, args: &[String]) -> Result<i32, String> {
        let generated = self.prepare(&mut Vec::new())?;
        let dir = generated.parent().unwrap_or(&self.root).to_path_buf();
        let mut cmd = Command::new(prog);
        if prog == "cargo" {
            cmd.args(distribution(&self.root, &[generated.clone()])?);
        }
        let status = cmd
            .args(args)
            .current_dir(&dir)
            .status()
            .map_err(|e| format!("cannot run `{prog}`: {e} -- is it installed?"))?;
        self.lock_back();
        Ok(status.code().unwrap_or(1))
    }

    /// Run a cargo subcommand, remapping its diagnostics onto Harsh source.
    ///
    /// Needs the `remap` feature: rustc's JSON diagnostics are read and
    /// remapped onto the Harsh sources. Without it the library is the pure
    /// language -- lexer, layout, expander, emitter -- with no dependencies,
    /// which is what `hrs_proc_macro` builds on.
    #[cfg(feature = "remap")]
    pub fn cargo(&self, sub: &str, args: &[String], maps: &[PathBuf]) -> Result<i32, String> {
        let mut seen = Vec::new();
        let generated = self.prepare(&mut seen)?;
        // The maps of the Harsh libraries it builds by path too: an error in
        // one is reported at its `.hrs` line.
        let mut maps = maps.to_vec();
        for dir in &seen {
            if let Ok(units) = (Project { root: dir.clone() }).units() {
                maps.extend(units.into_iter().map(|u| u.map).filter(|m| !maps.contains(m)).collect::<Vec<_>>());
            }
        }
        let result = self.cargo_run(sub, args, &maps, &generated);
        self.lock_back();
        result
    }

    /// The lockfile cargo wrote in `export/`, back where the user keeps it.
    #[cfg(feature = "remap")]
    fn lock_back(&self) {
        let back = self.cargo_root().join("Cargo.lock");
        if back.is_file() {
            let _ = fs::copy(&back, self.lock_root().join("Cargo.lock"));
        }
    }

    #[cfg(feature = "remap")]
    fn cargo_run(&self, sub: &str, args: &[String], maps: &[PathBuf], generated: &Path) -> Result<i32, String> {
        // Cargo runs in `export/`, the Rust project, and builds into its
        // `target/` -- or where the user chose (`CARGO_TARGET_DIR`, a
        // `.cargo/config.toml`), which cargo reads unasked.
        let manifest = generated.to_path_buf();
        let mut dir = generated.parent().unwrap_or(&self.root).to_path_buf();
        let mut at: Vec<String> = Vec::new();
        // The program runs where the user is, as `cargo run` runs it: its
        // relative paths are the project's, not the mirror's (a file it
        // writes into a mirrored folder would go at the next pass 1).
        if sub == "run" {
            at = vec!["--manifest-path".into(), manifest.display().to_string()];
            dir = std::env::current_dir().unwrap_or(dir);
        }
        // Only cargo's own build commands speak `--message-format`; an
        // external subcommand (`cargo leptos`, `cargo doc`'s cousins) is run
        // plainly, its output passed through unmapped.
        if !matches!(sub, "build" | "run" | "test" | "check" | "clippy" | "bench" | "doc") {
            // A patch only for a crate of Harsh's distribution the manifest
            // names -- a Harsh macro crate's `hrs_proc_macro`; none for
            // ordinary Rust, whose cargo runs plain.
            let status = Command::new("cargo")
                .args(distribution(&self.root, &[manifest.clone()])?)
                .arg(sub)
                .args(args)
                .current_dir(&dir)
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
        cmd.args(distribution(&self.root, &[manifest.clone()])?);
        cmd.arg(sub)
            .arg("--message-format=json-diagnostic-rendered-ansi")
            .args(&at)
            .args(args)
            .current_dir(&dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        // A program that panics names the generated file,
        // `src/main.rs:4:20`, from `export/`. For `run` and `test` its stderr is read
        // here and every such place is put back on the Harsh line,
        // `src/main.hrs:4:9`, through the same maps as the compiler's
        // errors (2026-09-21). Cargo keeps its colours when the terminal
        // would have shown them.
        let rewrite_panics = matches!(sub, "run" | "test");
        if rewrite_panics {
            use std::io::IsTerminal;
            if std::io::stderr().is_terminal() {
                cmd.env("CARGO_TERM_COLOR", "always");
            }
            cmd.stderr(Stdio::piped());
        }
        let mut child = cmd.spawn().map_err(|e| format!("cargo: {}", e))?;
        let stdout = child.stdout.take().unwrap();
        let stderr_thread = child.stderr.take().map(|stderr| {
            let root = self.root.clone();
            let maps: Vec<PathBuf> = maps.to_vec();
            std::thread::spawn(move || {
                let places = PanicPlaces::load(&root, &maps);
                let err = std::io::stderr();
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    let mut e = err.lock();
                    let _ = writeln!(e, "{}", places.rewrite(&line));
                }
            })
        });

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
        if let Some(t) = stderr_thread {
            let _ = t.join();
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        if errors > 0 || warnings > 0 {
            let _ = writeln!(w, "hrs: {} error(s), {} warning(s)", errors, warnings);
        }
        let _ = w.flush();
        Ok(status.code().unwrap_or(1))
    }
}

/// The generated files of a project, each with its map, for putting a
/// panic's `src/main.rs:4:20` (from `export/`) back on its Harsh line.
#[cfg(feature = "remap")]
pub struct PanicPlaces {
    files: Vec<(String, String, crate::remap::SourceMap)>, // (generated, source), relative to the root
}

#[cfg(feature = "remap")]
impl PanicPlaces {
    pub fn load(root: &Path, maps: &[PathBuf]) -> Self {
        // A panic names the generated file as cargo compiled it, from
        // `export/`: `src/main.rs:4:20`; the Harsh file is named from the
        // project's root, `src/main.hrs:4:9`.
        // In a workspace, cargo names a member's file from the workspace's
        // `export/`: `app/src/main.rs`.
        let export = Project { root: root.to_path_buf() }.cargo_root();
        let rel = |p: &str, from: &Path| -> String {
            let p = Path::new(p);
            p.strip_prefix(from).unwrap_or(p).display().to_string()
        };
        let files = maps
            .iter()
            .filter_map(|m| crate::remap::SourceMap::load(&m.display().to_string()).ok())
            .map(|m| (rel(m.generated(), &export), rel(m.source(), root), m))
            .collect();
        PanicPlaces { files }
    }

    /// Every `GENERATED:LINE:COL` in `line` rewritten to `SOURCE:LINE:COL`;
    /// anything the maps do not cover is left as it was.
    pub fn rewrite(&self, line: &str) -> String {
        let mut out = line.to_string();
        for (generated, source, map) in &self.files {
            let needle = format!("{generated}:");
            let mut from = 0;
            while let Some(at) = out[from..].find(&needle).map(|i| i + from) {
                // `src/lib.rs:` is also the tail of a dependency's path,
                // `…/nalgebra-0.33/src/lib.rs:123`: only a path that starts
                // here is the crate's own.
                // Rust's own `./src/main.rs` is the crate's.
                let path_char = |c: char| c.is_alphanumeric() || matches!(c, '/' | '\\' | '_' | '-' | '.');
                let before = out[..at].strip_suffix("./").unwrap_or(&out[..at]);
                if before.chars().next_back().map_or(false, path_char) {
                    from = at + needle.len();
                    continue;
                }
                let rest = &out[at + needle.len()..];
                let l: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                let after_l = &rest[l.len()..];
                let c: String = after_l.strip_prefix(':').map_or(String::new(), |s| s.chars().take_while(|c| c.is_ascii_digit()).collect());
                let (Ok(gl), Ok(gc)) = (l.parse::<usize>(), c.parse::<usize>()) else {
                    from = at + needle.len();
                    continue;
                };
                let end = at + needle.len() + l.len() + 1 + c.len();
                match map.locate(gl, gc) {
                    Some((sl, sc)) => {
                        let place = format!("{source}:{sl}:{sc}");
                        out.replace_range(at..end, &place);
                        from = at + place.len();
                    }
                    None => from = end,
                }
            }
        }
        out
    }
}

/// The prelude's `m~` and `v~` expand to types in the `hrs_std` crate, so a
/// project that calls them needs it among its dependencies. Rather than let
/// rustc fail on an unresolved crate somewhere in generated code, say so here,
/// with the line to add (2026-09-21). `hrs new` deliberately does not add it:
/// a first program should build without the network or a numeric library.
/// A file that defines its own `m` or `v` has shadowed the prelude's, and
/// needs nothing.
pub fn check_hrs_std(root: &Path, units: &[Unit], sources: &[String]) -> Result<(), String> {
    let manifest = fs::read_to_string(root.join(HRS_MANIFEST)).unwrap_or_default();
    let declared = manifest.lines().any(|l| {
        let l = l.trim_start();
        l.starts_with("hrs_std") && l[7..].trim_start().starts_with(['=', '.'])
    });
    if declared {
        return Ok(());
    }
    for (u, src) in units.iter().zip(sources) {
        if let Some((name, line)) = uses_hrs_std(src) {
            return Err(format!(
                "{}:{}: `{name}` needs the `hrs_std` crate, which this project does not depend on.\n\
                 Add it with `hrs add hrs_std`, or by hand in Hrs.toml:\n\n    [dependencies]\n    hrs_std = \"0.1\"\n",
                u.source.display(),
                line
            ));
        }
    }
    Ok(())
}

/// The first thing in a file that needs `hrs_std`, as it is spelled, with its
/// line: a call of the prelude's `m~` or `v~` -- unless the file defines a
/// macro of that name, which shadows ours -- a dotted operator (`.*`), or a
/// function marked `<>`. The last two are written to Rust as `hrs_std::DOT`
/// and `hrs_std::each!`, so without the crate rustc would name something the
/// user never wrote.
pub fn uses_hrs_std(src: &str) -> Option<(String, usize)> {
    use crate::lex::Tk;
    let toks = crate::lex::lex(src).ok()?;
    let own = |n: &str| {
        toks.windows(3).any(|w| w[0].is_kw("macro_rules") && w[1].text == "~" && w[2].text == n)
    };
    // A `use` line's `.*` is a glob, as `juxt::rewrite_dotted` knows.
    let is_use = |line: usize| toks.iter().filter(|t| t.line == line).take(6).any(|t| t.is_kw("use"));
    for k in 0..toks.len().saturating_sub(1) {
        let (t, n) = (&toks[k], &toks[k + 1]);
        let tight = t.span.hi == n.span.lo;
        if t.kind == Tk::Ident && matches!(t.text.as_str(), "m" | "v") && n.text == "~" && tight && !own(&t.text) {
            return Some((format!("{}~", t.text), t.line));
        }
        if t.kind == Tk::Dot && n.kind == Tk::Punct && tight && matches!(n.text.as_str(), "*" | "+" | "-" | "/") && !is_use(t.line) {
            return Some((format!(".{}", n.text), t.line));
        }
        if crate::juxt::is_each_mark(&toks, k) {
            return Some((format!("{}<>", toks[k - 1].text), t.line));
        }
    }
    None
}

fn collect(dir: &Path, src_root: &Path, gen_root: &Path, map_root: &Path, out: &mut Vec<Unit>) -> Result<(), String> {
    for e in fs::read_dir(dir).map_err(|e| format!("{}: {}", dir.display(), e))? {
        let p = e.map_err(|e| e.to_string())?.path();
        if p.is_dir() {
            collect(&p, src_root, gen_root, map_root, out)?;
        } else if p.extension().map(|x| x == "hrs").unwrap_or(false) {
            let rel = p.strip_prefix(src_root).unwrap_or(&p).to_path_buf();
            let rust = gen_root.join(rel.with_extension("rs"));
            // The map beside nothing a reader or crates.io sees.
            let map = map_root.join(rel.with_extension("map.json"));
            out.push(Unit { source: p, rust, map });
        }
    }
    Ok(())
}

/// The paths of a manifest's path dependencies: each `name = { path = "…" }`
/// entry under `[dependencies]`, read line by line.
fn path_dependencies(manifest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]";
            continue;
        }
        if in_deps {
            if let Some(i) = t.find("path") {
                let rest = t[i + 4..].trim_start();
                if let Some(r) = rest.strip_prefix('=') {
                    if let Some(p) = r.trim_start().strip_prefix('"').and_then(|r| r.split('"').next()) {
                        out.push(p.to_string());
                    }
                }
            }
        }
    }
    out
}

/// The first path segment of a `use` line, if the line is one.
fn use_root(line: &str) -> Option<&str> {
    let t = line.trim_start();
    let t = t.strip_prefix("pub(crate) ").or_else(|| t.strip_prefix("pub ")).unwrap_or(t);
    let rest = t.strip_prefix("use ")?.trim_start();
    Some(rest.split(|c: char| c == '.' || c == '{' || c.is_whitespace() || c == ';').next().unwrap_or(""))
}

/// Blank, byte for byte, every `use` line whose path starts with one of
/// `libs`: a Harsh macro crate, which has no Rust meaning.
fn blank_macro_uses(src: &str, libs: &[String]) -> String {
    if libs.is_empty() {
        return src.to_string();
    }
    src.split_inclusive('\n')
        .map(|l| match use_root(l) {
            Some(r) if libs.iter().any(|x| x == r) => l.chars().map(|c| if c == '\n' { '\n' } else { ' ' }).collect(),
            _ => l.to_string(),
        })
        .collect()
}

/// The names `src` re-exports from the macro crate `lib`:
/// `pub use lib.Name` or `pub use lib.{A, B}`.
fn reexports_of(src: &str, lib: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix("pub use ") else { continue };
        let Some(after) = rest.trim_start().strip_prefix(lib).and_then(|r| r.strip_prefix('.')) else { continue };
        let after = after.trim_end().trim_end_matches(';');
        let items = after.strip_prefix('{').and_then(|r| r.strip_suffix('}')).unwrap_or(after);
        out.extend(items.split(',').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()));
    }
    out
}

/// The crates of Harsh's standard distribution, which ship inside `hrs`
/// (the user, 2026-09-25): named in a `Cargo.toml` as Rust names `syn` or
/// `quote` -- `hrs_quote = "0.1"` -- and never fetched: `hrs` writes them
/// under `.hrs/dist/` and patches them into the proc-macro runner's build
/// (only a Harsh macro crate uses them, and it is never exported);
/// `hrs_std`, since 0.9.0, is folded into the crate instead.
#[cfg(feature = "remap")]
pub const DISTRIBUTION: [&str; 4] = ["hrs_std", "hrs_proc_macro", "hrs_quote", "hrs_syn"];

/// Harsh's own manifest, beside `Cargo.toml` (the user's design,
/// 2026-10-03): its `[dependencies]` are Harsh's -- `hrs_std`, Harsh crates
/// -- and `Cargo.toml` keeps Rust's. `hrs` handles Harsh's at the Harsh
/// level, then writes a generated `Cargo.toml` for Cargo.
pub const HRS_MANIFEST: &str = "Hrs.toml";

/// Harsh's crates that are Harsh dependencies, and so belong in `Hrs.toml`.
/// The procedural-macro crates (`hrs_proc_macro`, `hrs_quote`, `hrs_syn`)
/// are compile-time tooling, as `syn` and `quote` are Rust's, and stay in
/// `Cargo.toml` (the user's principle: "except the macro crates").
pub const HRS_TOML_CRATES: [&str; 1] = ["hrs_std"];

/// What `Hrs.toml` says: the `Cargo.toml` it belongs to (`[package] cargo`,
/// relative to its folder; the one beside it by default) and its
/// dependencies, each `name = <value as written>`.
pub struct HrsManifest {
    pub cargo: PathBuf,
    pub deps: Vec<(String, String)>,
}

pub fn read_hrs_manifest(root: &Path) -> Result<Option<HrsManifest>, String> {
    let path = root.join(HRS_MANIFEST);
    let Ok(text) = fs::read_to_string(&path) else { return Ok(None) };
    let mut table = String::new();
    let mut cargo = root.join("Cargo.toml");
    let mut deps = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        // A comment runs from a `#` outside a quoted string to the line's end.
        let mut quoted = false;
        let cut = raw
            .char_indices()
            .find(|&(_, c)| {
                if c == '"' {
                    quoted = !quoted;
                }
                c == '#' && !quoted
            })
            .map_or(raw.len(), |(i, _)| i);
        let line = raw[..cut].trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            table = line.trim_matches(|c| c == '[' || c == ']').trim().to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("{}:{}: expected `name = value`", path.display(), n + 1));
        };
        let (key, value) = (key.trim().to_string(), value.trim().to_string());
        match table.as_str() {
            "package" if key == "cargo" => cargo = root.join(value.trim_matches('"')),
            "package" => {}
            "dependencies" => deps.push((key, value)),
            other => return Err(format!("{}:{}: `[{other}]` is not a table of Hrs.toml; it has `[package]` and `[dependencies]`", path.display(), n + 1)),
        }
    }
    Ok(Some(HrsManifest { cargo, deps }))
}

/// Harsh's crates named as dependencies in a `Cargo.toml` -- since 0.3.0
/// they belong in `Hrs.toml`.
#[cfg(feature = "remap")]
pub fn harsh_crates_in_cargo(text: &str) -> Vec<String> {
    let mut table = String::new();
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            table = line.to_string();
            continue;
        }
        if !table.contains("dependencies") {
            continue;
        }
        let key = line.split(['=', '.']).next().unwrap_or("").trim();
        if HRS_TOML_CRATES.contains(&key) && !out.iter().any(|k: &String| k == key) {
            out.push(key.to_string());
        }
    }
    out
}

/// `hrs migrate`: Harsh's crates moved from `Cargo.toml`'s dependency tables
/// into `Hrs.toml`'s `[dependencies]` -- the two texts after, and what moved.
#[cfg(feature = "remap")]
pub fn migrate(cargo: &str, hrs: Option<&str>) -> (String, String, Vec<String>) {
    let mut table = String::new();
    let mut kept = Vec::new();
    let mut moved: Vec<String> = Vec::new();
    let mut names = Vec::new();
    for raw in cargo.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            table = line.to_string();
        } else if table.contains("dependencies") {
            let key = line.split(['=', '.']).next().unwrap_or("").trim();
            if HRS_TOML_CRATES.contains(&key) {
                moved.push(line.to_string());
                names.push(key.to_string());
                continue;
            }
        }
        // The generated tree's old places: `target/hrs/` (to 0.6.x) and
        // `target/src/` (0.7.0 to 0.8.x). Since 0.9.0 the Rust project is
        // `export/`, whose `src/` the target's path names as cargo would.
        if !raw.trim_start().starts_with('#') && (raw.contains("target/hrs/") || raw.contains("target/src/")) {
            names.push(EXPORT_DIR.to_string());
            kept.push(raw.replace("target/hrs/", "src/").replace("target/src/", "src/"));
            continue;
        }
        kept.push(raw.to_string());
    }
    names.dedup();
    let mut hrs = hrs
        .map(|t| t.to_string())
        .unwrap_or_else(|| "# Hrs.toml -- Harsh's dependencies; Cargo.toml beside it keeps Rust's\n[package]\ncargo = \"Cargo.toml\"\n\n[dependencies]\n".to_string());
    for line in &moved {
        let name = line.split(['=', '.']).next().unwrap_or("").trim().to_string();
        if let Ok((t, _)) = add_dependency_line(&hrs, &name, line) {
            hrs = t;
        }
    }
    (kept.join("\n") + "\n", hrs, names)
}

/// A workspace's root, without a `[package]` of its own: its members'
/// folders, in the order `members` lists them, a trailing `*` expanded to
/// the folders beneath (sorted) that hold a `Cargo.toml`, `exclude` left out.
/// `None` when `root`'s `Cargo.toml` is a package, or is no workspace
/// (2026-10-13: `hrs` at a workspace's root was not a command).
pub fn workspace_members(root: &Path) -> Option<Vec<PathBuf>> {
    match workspace_listing(root)? {
        (members, false) => Some(members),
        _ => None,
    }
}

/// A `[workspace]`'s members, as `workspace_members` reads them, and
/// whether the root is a package too.
pub fn workspace_listing(root: &Path) -> Option<(Vec<PathBuf>, bool)> {
    let text = fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let mut section = String::new();
    let mut is_workspace = false;
    let mut has_package = false;
    let mut key = String::new();
    let mut lists: std::collections::HashMap<String, Vec<String>> = Default::default();
    let mut open = false;
    for raw in text.lines() {
        // A `#` starts a comment only outside a string.
        let mut in_str = false;
        let cut = raw.char_indices().find(|&(_, c)| {
            if c == '"' {
                in_str = !in_str;
            }
            c == '#' && !in_str
        });
        let line = cut.map_or(raw, |(i, _)| &raw[..i]).trim();
        if line.is_empty() {
            continue;
        }
        if !open && line.starts_with('[') {
            section = line.replace(' ', "");
            is_workspace |= section == "[workspace]";
            has_package |= section == "[package]";
            continue;
        }
        let mut rest = line;
        if !open {
            if section != "[workspace]" {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            key = k.trim().to_string();
            if key != "members" && key != "exclude" {
                continue;
            }
            rest = v.trim();
            let Some(r) = rest.strip_prefix('[') else { continue };
            rest = r;
            open = true;
        }
        let (items, closed) = match rest.split_once(']') {
            Some((a, _)) => (a, true),
            None => (rest, false),
        };
        for item in items.split(',') {
            let item = item.trim().trim_matches('"').trim_matches('\'');
            if !item.is_empty() {
                lists.entry(key.clone()).or_default().push(item.to_string());
            }
        }
        if closed {
            open = false;
        }
    }
    if !is_workspace {
        return None;
    }
    // `./crates/old/` and `crates/old` name one folder.
    let clean = |p: &str| p.trim_start_matches("./").trim_end_matches('/').to_string();
    let exclude: Vec<PathBuf> = lists.get("exclude").into_iter().flatten().map(|e| root.join(clean(e))).collect();
    let mut out = Vec::new();
    for m in lists.get("members").into_iter().flatten() {
        if m.trim().is_empty() {
            continue;
        }
        let m = clean(m);
        let found: Vec<PathBuf> = if let Some(base) = m.strip_suffix("/*") {
            let mut v: Vec<PathBuf> = fs::read_dir(root.join(base))
                .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.join("Cargo.toml").is_file()).collect())
                .unwrap_or_default();
            v.sort();
            v
        } else {
            vec![root.join(m)]
        };
        for f in found {
            if !exclude.contains(&f) && !out.contains(&f) {
                out.push(f);
            }
        }
    }
    Some((out, has_package))
}

/// The workspace `root` belongs to: the nearest folder, from `root`
/// upwards, whose `[workspace]` lists it -- or `root` itself when it is a
/// package with a `[workspace]` of its own.
pub fn workspace_of(root: &Path) -> Option<PathBuf> {
    let me = root.canonicalize().ok()?;
    for w in root.ancestors() {
        let Some((members, has_package)) = workspace_listing(w) else { continue };
        if w == root {
            if has_package {
                return Some(w.to_path_buf());
            }
            continue;
        }
        if members.iter().any(|m| m.canonicalize().ok().as_ref() == Some(&me)) {
            return Some(w.to_path_buf());
        }
        // The nearest workspace is the one cargo takes.
        return None;
    }
    None
}

/// A workspace's Harsh members, canonical.
pub fn harsh_members(w: &Path) -> Vec<PathBuf> {
    let Some((members, has_package)) = workspace_listing(w) else { return Vec::new() };
    let mut out: Vec<PathBuf> = members.iter().filter(|m| is_harsh_member(m)).filter_map(|m| m.canonicalize().ok()).collect();
    if has_package && is_harsh_member(w) {
        if let Ok(c) = w.canonicalize() {
            out.push(c);
        }
    }
    out
}

/// A workspace's `export/` (0.9.0, the user's option A, 2026-10-14): the
/// whole Rust workspace -- its `Cargo.toml`, each Harsh member transpiled
/// at its place, the rest of the workspace (its Rust members among it)
/// mirrored -- with one `Cargo.lock` and one `target/`. Once per run.
#[cfg(feature = "remap")]
fn prepare_workspace(w: &Path, seen: &mut Vec<PathBuf>) -> Result<(), String> {
    let marker = w.canonicalize().unwrap_or(w.to_path_buf()).join("[workspace]");
    if seen.contains(&marker) {
        return Ok(());
    }
    seen.push(marker);
    let Some((_, has_package)) = workspace_listing(w) else { return Ok(()) };
    let export = w.join(EXPORT_DIR);
    let harsh = harsh_members(w);
    // The root's own manifest: a virtual root's, or a Rust package's -- a
    // Harsh root package writes its own below, as a member does.
    let rust_root = has_package && !is_harsh_member(w);
    if !has_package || rust_root {
        let path = w.join("Cargo.toml");
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let plain: String = without_harsh_table(&text)
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                !(t.starts_with('#') && !t.starts_with("#[") && speaks_of_harsh(t))
            })
            .map(|l| format!("{l}\n"))
            .collect();
        let cw = w.canonicalize().unwrap_or(w.to_path_buf());
        let text = generated_manifest(&plain, w, &[], false, w, &mut |dir: &Path| {
            // Outside the workspace, a Harsh library is its own `export/`.
            let cd = dir.canonicalize().ok()?;
            if cd.starts_with(&cw) {
                return None;
            }
            let dep = Project { root: dir.to_path_buf() };
            (!dep.units().map_or(true, |u| u.is_empty())).then(|| dep.export_dir())
        })?;
        fs::create_dir_all(&export).map_err(|e| format!("{}: {e}", export.display()))?;
        let gen = export.join("Cargo.toml");
        if fs::read_to_string(&gen).ok().as_deref() != Some(text.as_str()) {
            fs::write(&gen, &text).map_err(|e| format!("{}: {e}", gen.display()))?;
        }
        link_project_dirs(w, &export, &harsh);
        if rust_root {
            // Its Rust is its own: `src/` as it is.
            mirror_entry(&w.join(SRC_DIR), &export.join(SRC_DIR), &[]);
        }
    }
    for m in &harsh {
        let p = Project { root: m.clone() };
        if !seen.contains(m) {
            seen.push(m.clone());
        }
        let name = m.strip_prefix(w.canonicalize().unwrap_or(w.to_path_buf())).unwrap_or(m).display().to_string();
        let name = if name.is_empty() { ".".to_string() } else { name };
        p.transpile(false).map_err(|e| format!("in the workspace member `{name}`:\n{e}"))?;
        p.prepare_own(seen)?;
    }
    export_extras(w);
    Ok(())
}

/// `prepare_workspace`, for `hrs` at a workspace's root: before cargo runs
/// a Rust member, in its place in `export/`.
#[cfg(feature = "remap")]
pub fn prepare_workspace_root(w: &Path) -> Result<(), String> {
    prepare_workspace(w, &mut Vec::new())
}

/// What `export/` gets beside the Rust: a `.gitignore` for its `target/`
/// when the project has none -- cargo leaves its `target/` alone when the
/// project says so (`cargo leptos watch` watched its own builds without
/// it, 0.8.5) -- and the lockfile, the user's, copied in (back after
/// cargo).
#[cfg(feature = "remap")]
fn export_extras(root: &Path) {
    let export = root.join(EXPORT_DIR);
    let ignore = export.join(".gitignore");
    if !root.join(".gitignore").exists() && !ignore.exists() {
        let _ = fs::write(&ignore, "/target\n");
    }
    let lock = root.join("Cargo.lock");
    if lock.is_file() {
        let _ = fs::copy(&lock, export.join("Cargo.lock"));
    }
}

/// A member `hrs` builds: one with an `Hrs.toml`, or with `.hrs` sources
/// in `src/`. Any other member is Rust's, built by cargo.
pub fn is_harsh_member(dir: &Path) -> bool {
    dir.join(HRS_MANIFEST).is_file() || Project { root: dir.to_path_buf() }.units().map_or(false, |u| !u.is_empty())
}

/// A workspace member's inherited fields written out (2026-10-03): the
/// generated manifest is its own workspace, so `version.workspace = true`
/// and `serde = { workspace = true, features = [..] }` take their values
/// from the nearest `[workspace]` above `cargo_dir` -- `[workspace.package]`
/// and `[workspace.dependencies]`, a path there made absolute from the
/// workspace's root. Without a workspace above, the text is unchanged.
pub fn resolve_workspace(cargo_text: &str, cargo_dir: &Path) -> String {
    if !cargo_text.contains("workspace = true") {
        return cargo_text.to_string();
    }
    let mut ws: Option<(PathBuf, String)> = None;
    let mut d = cargo_dir.parent();
    while let Some(dir) = d {
        if let Ok(t) = fs::read_to_string(dir.join("Cargo.toml")) {
            if t.lines().any(|l| l.trim() == "[workspace]") {
                ws = Some((dir.to_path_buf(), t));
                break;
            }
        }
        d = dir.parent();
    }
    let Some((ws_dir, ws_text)) = ws else { return cargo_text.to_string() };
    // The workspace's tables, `name = value` as written.
    let mut package = std::collections::HashMap::new();
    let mut deps = std::collections::HashMap::new();
    let mut table = String::new();
    for raw in ws_text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            table = line.to_string();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, mut v) = (k.trim().to_string(), v.trim().to_string());
        if let Some(i) = v.find("path = \"") {
            let at = i + 8;
            if let Some(len) = v[at..].find('"') {
                let rel = v[at..at + len].to_string();
                if !Path::new(&rel).is_absolute() {
                    v.replace_range(at..at + len, &manifest_path_str(&ws_dir.join(&rel)));
                }
            }
        }
        match table.as_str() {
            "[workspace.package]" => {
                package.insert(k, v);
            }
            "[workspace.dependencies]" => {
                deps.insert(k, v);
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    table.clear();
    for raw in cargo_text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            table = line.to_string();
            out.push(raw.to_string());
            continue;
        }
        let in_deps = table.contains("dependencies");
        // `key.workspace = true`
        if let Some(key) = line.strip_suffix(".workspace = true") {
            let key = key.trim();
            let found = if in_deps { deps.get(key) } else { package.get(key) };
            if let Some(v) = found {
                out.push(format!("{key} = {v}"));
                continue;
            }
        }
        // `key = { workspace = true, more.. }`
        if let Some((key, value)) = line.split_once('=') {
            let (key, value) = (key.trim(), value.trim());
            if value.starts_with('{') && value.contains("workspace = true") {
                let found = if in_deps { deps.get(key) } else { package.get(key) };
                if let Some(v) = found {
                    let more: Vec<&str> = value
                        .trim_matches(|c| c == '{' || c == '}')
                        .split(',')
                        .map(str::trim)
                        .filter(|p| !p.is_empty() && *p != "workspace = true")
                        .collect();
                    let base = if v.starts_with('{') {
                        v.trim_matches(|c| c == '{' || c == '}').trim().to_string()
                    } else {
                        format!("version = {v}")
                    };
                    let all: Vec<String> = std::iter::once(base).chain(more.iter().map(|m| m.to_string())).collect();
                    out.push(format!("{key} = {{ {} }}", all.join(", ")));
                    continue;
                }
            }
        }
        out.push(raw.to_string());
    }
    out.join("\n") + "\n"
}

/// A path as a TOML string's contents: forward slashes, which Cargo reads
/// on every platform.
pub fn manifest_path_str(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

#[cfg(feature = "remap")]
/// The names under `[dependencies]` (and the dev / build tables) of a
/// manifest's text.
fn cargo_dependency_names(text: &str) -> Vec<String> {
    let mut table = String::new();
    let mut names = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            table = line.to_string();
        } else if table.contains("dependencies") && !line.is_empty() && !line.starts_with('#') {
            let key = line.split(['=', '.']).next().unwrap_or("").trim();
            if !key.is_empty() {
                names.push(key.to_string());
            }
        }
    }
    names
}

#[cfg(feature = "remap")]
/// The project's top level, mirrored into `export/`, the Rust project (S3,
/// 2026-10-06): Cargo builds there, so `CARGO_MANIFEST_DIR` is `export/`,
/// and `asset!("/assets/main.css")` or `env!("CARGO_MANIFEST_DIR")` paths
/// must find the project's `assets/`, `migrations/`, … Every top-level
/// entry, folders and files alike (Docreview's H12, 0.7.3: `Dioxus.toml` was
/// missing), hidden ones included since 0.9.0 (`cargo leptos` reads the
/// `.gitignore` of the folder it runs in) -- but `src` (transpiled instead),
/// `export` itself, `.hrs` (`hrs`'s own), a `target` left by an older `hrs`, a version-control
/// folder (`.git`, `.hg`, `.svn`), and the manifests `hrs` writes there
/// itself (`Cargo.toml`, `Cargo.lock`, `Hrs.toml`).
///
/// A mirror, not a symbolic link (ShopBill's S7, 0.8.2): Dioxus's `asset!`
/// resolves a path through its links and requires the file to sit inside
/// `CARGO_MANIFEST_DIR`, and a link led back out of the generated project to
/// the project. Each folder is a real folder there and each file a hard link to
/// the project's -- the same file, no copy -- or a copy where a link cannot
/// be made (another disk). Run at every pass 1: a file added, changed or
/// replaced (an editor that saves by writing a new file breaks a hard link)
/// is linked again, one removed is removed. A symbolic link left by an
/// earlier `hrs` is replaced.
fn link_project_dirs(root: &Path, gen: &Path, skip: &[PathBuf]) {
    let Ok(entries) = fs::read_dir(root) else { return };
    for e in entries.flatten() {
        let name = e.file_name();
        let n = name.to_string_lossy();
        if [SRC_DIR, EXPORT_DIR, WORK_DIR, "target", ".git", ".hg", ".svn", "Cargo.toml", "Cargo.lock", HRS_MANIFEST].contains(&n.as_ref()) {
            continue;
        }
        mirror_entry(&e.path(), &gen.join(&name), skip);
    }
}

/// Whether `gen` is `src`'s place in `export/`: `<root>/export/<rel>` for
/// `<root>/<rel>`.
fn mirrors(src: &Path, gen: &Path) -> bool {
    src.ancestors().skip(1).any(|a| src.strip_prefix(a).map_or(false, |rel| a.join(EXPORT_DIR).join(rel) == gen))
}

/// The files in `src/` that are not Harsh -- a data file an `include_str!`
/// reads, a Rust module written in Rust -- mirrored into `export/src/`
/// beside the transpiled ones, so `export/` builds and publishes alone and
/// a relative path means there what it means in `src/`.
#[cfg(feature = "remap")]
fn mirror_src_extras(src: &Path, gen: &Path) {
    let Ok(entries) = fs::read_dir(src) else { return };
    for e in entries.flatten() {
        let path = e.path();
        let to = gen.join(e.file_name());
        let Ok(meta) = fs::symlink_metadata(&path) else { continue };
        if meta.is_dir() {
            let _ = fs::create_dir_all(&to);
            mirror_src_extras(&path, &to);
        } else if path.extension().map_or(false, |x| x == "hrs") {
            continue;
        } else if path.extension().map_or(false, |x| x == "rs") && path.with_extension("hrs").is_file() {
            // The transpiled file's name: the `.hrs` wins.
            continue;
        } else {
            mirror_entry(&path, &to, &[]);
        }
    }
}

#[cfg(feature = "remap")]
fn mirror_entry(src: &Path, dst: &Path, skip: &[PathBuf]) {
    let Ok(meta) = fs::symlink_metadata(src) else { return };
    // A Harsh member of the workspace: its own `prepare` writes its place.
    if meta.is_dir() && !skip.is_empty() && src.canonicalize().map_or(false, |c| skip.contains(&c)) {
        return;
    }
    // Any other Harsh project beneath -- one the workspace excludes -- is
    // no Rust: `export/` holds only Rust.
    if meta.is_dir() && src.join("Cargo.toml").is_file() && is_harsh_member(src) {
        remove_any(dst);
        return;
    }
    if meta.file_type().is_symlink() {
        // A link in the project stays a link: it is the project's choice.
        if let Ok(to) = fs::read_link(src) {
            if fs::read_link(dst).ok().as_deref() != Some(to.as_path()) {
                remove_any(dst);
                #[cfg(unix)]
                let _ = std::os::unix::fs::symlink(&to, dst);
                #[cfg(windows)]
                let _ = if src.is_dir() { std::os::windows::fs::symlink_dir(&to, dst) } else { std::os::windows::fs::symlink_file(&to, dst) };
            }
        }
        return;
    }
    if meta.is_dir() {
        mirror_dir(src, dst, skip);
    } else {
        mirror_file(src, &meta, dst);
    }
}

#[cfg(feature = "remap")]
fn mirror_dir(src: &Path, dst: &Path, skip: &[PathBuf]) {
    let real_dir = fs::symlink_metadata(dst).map_or(false, |m| m.is_dir() && !m.file_type().is_symlink());
    if !real_dir {
        remove_any(dst);
        if fs::create_dir_all(dst).is_err() {
            return;
        }
    }
    let mut names = std::collections::HashSet::new();
    // A Rust crate's own `target/`, and version control, are not the project.
    let is_crate = src.join("Cargo.toml").is_file();
    if let Ok(entries) = fs::read_dir(src) {
        for e in entries.flatten() {
            let n = e.file_name();
            if (is_crate && n == "target") || [".git", ".hg", ".svn"].iter().any(|v| n == *v) {
                continue;
            }
            names.insert(n);
            mirror_entry(&e.path(), &dst.join(e.file_name()), skip);
        }
    }
    // What the project no longer has goes from the mirror too.
    if let Ok(entries) = fs::read_dir(dst) {
        for e in entries.flatten() {
            if !names.contains(&e.file_name()) {
                remove_any(&e.path());
            }
        }
    }
}

#[cfg(feature = "remap")]
fn mirror_file(src: &Path, meta: &fs::Metadata, dst: &Path) {
    // A hard link shares its file's length and time; a copy is given the
    // file's time. Either way, equal means up to date.
    if let Ok(d) = fs::symlink_metadata(dst) {
        if d.is_file() && d.len() == meta.len() && d.modified().ok() == meta.modified().ok() {
            return;
        }
    }
    remove_any(dst);
    if fs::hard_link(src, dst).is_ok() {
        return;
    }
    if fs::copy(src, dst).is_ok() {
        if let (Ok(t), Ok(f)) = (meta.modified(), fs::OpenOptions::new().write(true).open(dst)) {
            let _ = f.set_modified(t);
        }
    }
}

#[cfg(feature = "remap")]
fn remove_any(p: &Path) {
    match fs::symlink_metadata(p) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {
            let _ = fs::remove_dir_all(p);
        }
        Ok(_) => {
            // A file, or a link (to a folder on Windows: `remove_dir`).
            if fs::remove_file(p).is_err() {
                let _ = fs::remove_dir(p);
            }
        }
        Err(_) => {}
    }
}

/// `export/Cargo.toml`: the user's, with
/// - a target's `path` and the package's `build` left relative, since
///   `export/` holds `src/` transpiled and the rest of the project mirrored;
/// - every other relative path made absolute (the file moved), and a path
///   dependency on a Harsh project -- `harsh_dep` says which, and prepares
///   it -- named by that project's `export/`;
/// - its own `[workspace]` when it stands alone (`standalone`), so a
///   workspace above it does not claim it -- a member's belongs to its
///   workspace's `export/`;
/// - a relative path inside `mirror_root`, the folder `export/` mirrors,
///   left as written;
/// - `extra` dependency lines added to `[dependencies]`.
pub fn generated_manifest(
    cargo_text: &str,
    root: &Path,
    extra: &[String],
    standalone: bool,
    mirror_root: &Path,
    harsh_dep: &mut dyn FnMut(&Path) -> Option<PathBuf>,
) -> Result<String, String> {
    // Inside the folder `export/` mirrors, a relative path means the same
    // there: kept as written, so `export/` stays movable and publishable.
    let mirrored = mirror_root.canonicalize().ok();
    let inside = |p: &Path| match (&mirrored, p.canonicalize()) {
        (Some(m), Ok(c)) => c.starts_with(m),
        _ => false,
    };
    let mut out = Vec::new();
    let mut has_deps = false;
    let mut has_workspace = false;
    let mut table = String::new();
    for raw in cargo_text.lines() {
        let mut line = raw.to_string();
        let t0 = line.trim();
        if t0.starts_with('[') {
            table = t0.trim_matches(|c| c == '[' || c == ']').trim().to_string();
        }
        let is_target = matches!(table.as_str(), "lib" | "bin" | "test" | "example" | "bench");
        let is_deps = table.ends_with("dependencies");
        for key in ["path", "build"] {
            if is_target || (key == "build" && table == "package") {
                continue;
            }
            let mut search = 0;
            while let Some(i) = line[search..].find(&format!("{key} = \"")) {
                let at = search + i + key.len() + 4;
                let Some(len) = line[at..].find('"') else { break };
                let value = line[at..at + len].to_string();
                let relative = !Path::new(&value).is_absolute();
                let abs = if relative { root.join(&value) } else { PathBuf::from(&value) };
                let dep = if is_deps && key == "path" { harsh_dep(&abs) } else { None };
                let abs = match dep {
                    Some(d) => manifest_path_str(&d),
                    None if relative && inside(&abs) => value.clone(),
                    None => manifest_path_str(&abs),
                };
                line.replace_range(at..at + len, &abs);
                search = at + abs.len();
            }
        }
        let t = line.trim().to_string();
        out.push(line);
        if t == "[dependencies]" {
            has_deps = true;
            out.extend(extra.iter().cloned());
        }
        if t == "[workspace]" {
            has_workspace = true;
        }
    }
    if !has_deps && !extra.is_empty() {
        out.push(String::new());
        out.push("[dependencies]".into());
        out.extend(extra.iter().cloned());
    }
    if standalone && !has_workspace {
        out.push(String::new());
        out.push("# Its own workspace root. Written by hrs from the project's Cargo.toml.".into());
        out.push("[workspace]".into());
    }
    Ok(out.join("\n") + "\n")
}

/// A package's `version`, from its manifest's `[package]` table.
#[cfg(feature = "remap")]
fn package_version(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for l in manifest.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
            continue;
        }
        if in_package {
            if let Some(v) = t.strip_prefix("version") {
                let v = v.trim_start().strip_prefix('=')?.trim();
                return Some(v.trim_matches('"').to_string());
            }
        }
    }
    None
}

/// Whether `Hrs.toml` lists `hrs_std`: then the transpiler writes its paths
/// `crate::hrs_std::…` and `prepare` folds it into the crate.
pub fn lists_hrs_std(root: &Path) -> bool {
    std::fs::read_to_string(root.join(HRS_MANIFEST)).map_or(false, |m| {
        m.lines().any(|l| {
            let t = l.trim_start();
            t.starts_with("hrs_std") && t[7..].trim_start().starts_with(['=', '.'])
        })
    })
}

/// The macros `hrs_std` exports (`#[macro_export]`): at the crate's root
/// once folded, so `hrs_std::each!` is written `crate::each!`.
#[cfg(feature = "remap")]
fn hrs_std_macros() -> Vec<String> {
    let mut out = Vec::new();
    for (path, text) in crate::dist::FILES {
        if !path.starts_with("hrs_std/src/") {
            continue;
        }
        let mut exported = false;
        for l in text.lines() {
            let t = l.trim();
            if t == "#[macro_export]" {
                exported = true;
            } else if let Some(r) = t.strip_prefix("macro_rules!") {
                if exported {
                    out.push(r.trim().trim_end_matches('{').trim().to_string());
                }
                exported = false;
            } else if !t.starts_with("#[") && !t.starts_with("///") && !t.is_empty() {
                exported = false;
            }
        }
    }
    out
}

/// `hrs_std`'s own Rust dependencies, for `export/Cargo.toml` once it is
/// folded into the crate.
#[cfg(feature = "remap")]
fn hrs_std_dependencies() -> Vec<String> {
    let std_manifest = crate::dist::FILES.iter().find(|(p, _)| *p == "hrs_std/Cargo.toml").map(|(_, t)| *t).unwrap_or("");
    std_manifest
        .lines()
        .skip_while(|l| l.trim() != "[dependencies]")
        .skip(1)
        .take_while(|l| !l.trim_start().starts_with('['))
        .filter(|l| !l.trim().is_empty())
        .map(String::from)
        .collect()
}

thread_local! {
    /// Whether the transpiler writes `hrs_std`'s paths into the crate's own
    /// module (`Project::transpile_only` sets it from `Hrs.toml`).
    static FOLD_STD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn set_fold_std(on: bool) {
    FOLD_STD.with(|f| f.set(on));
}

/// `hrs_std::Matrix` written `crate::hrs_std::Matrix`, and `hrs_std::each!`
/// written `crate::each!` (an exported macro sits at the crate's root),
/// every map entry after each change shifted by what it added, so rustc's
/// errors still land on the Harsh line and column. A path already
/// `crate::hrs_std::` (or `$crate::…`) is left; strings and comments are
/// not read (the Rust lexer skips them).
#[cfg(feature = "remap")]
pub fn fold_std_paths(out: &mut String, map: &mut [crate::emit::MapEntry]) {
    use crate::lex::Tk;
    let Ok(toks) = crate::lex::lex_rust(out) else { return };
    let mut edits: Vec<(usize, usize, String)> = Vec::new(); // (at, removed, inserted)
    for k in 0..toks.len() {
        let t = &toks[k];
        if !(t.kind == Tk::Ident && t.text == "hrs_std") {
            continue;
        }
        if toks.get(k + 1).map_or(true, |n| n.kind != Tk::PathSep) {
            continue;
        }
        if k > 0 && toks[k - 1].kind == Tk::PathSep {
            continue;
        }
        let at = t.span.lo as usize;
        let is_macro = toks.get(k + 2).map_or(false, |n| n.kind == Tk::Ident)
            && toks.get(k + 3).map_or(false, |n| n.text == "!");
        if is_macro {
            // `hrs_std::` → `crate::`
            let end = toks[k + 1].span.hi as usize;
            edits.push((at, end - at, "crate::".into()));
        } else {
            edits.push((at, 0, "crate::".into()));
        }
    }
    for (at, removed, inserted) in edits.into_iter().rev() {
        out.replace_range(at..at + removed, &inserted);
        let delta = inserted.len() as i64 - removed as i64;
        for e in map.iter_mut() {
            if e.gen_lo as usize > at || (removed == 0 && e.gen_lo as usize == at) {
                e.gen_lo = (e.gen_lo as i64 + delta).max(at as i64) as u32;
            }
            if e.gen_hi as usize > at {
                e.gen_hi = (e.gen_hi as i64 + delta).max(at as i64) as u32;
            }
        }
    }
}

/// A distribution crate's version as a dependency line gives it, `"0.1"`:
/// major and minor, from its manifest embedded in `hrs`.
#[cfg(feature = "remap")]
pub fn dist_version(name: &str) -> Option<String> {
    let path = format!("{name}/Cargo.toml");
    let (_, text) = crate::dist::FILES.iter().find(|(p, _)| *p == path)?;
    let v = text.lines().find_map(|l| l.trim().strip_prefix("version = \""))?.split('"').next()?;
    let mut parts = v.split('.');
    Some(format!("{}.{}", parts.next()?, parts.next()?))
}

/// Insert `line` for `name` into a manifest's `[dependencies]`, unless
/// `name` is there already.
#[cfg(feature = "remap")]
pub fn add_dependency_line(manifest: &str, name: &str, line: &str) -> Result<(String, String), String> {
    let lines: Vec<&str> = manifest.lines().collect();
    let key = |l: &str| l.split(['=', '.']).next().unwrap_or("").trim().to_string();
    let table = lines.iter().position(|l| l.trim() == "[dependencies]");
    let text = match table {
        Some(s) => {
            let end = (s + 1..lines.len()).find(|&k| lines[k].trim_start().starts_with('[')).unwrap_or(lines.len());
            if lines[s + 1..end].iter().any(|l| key(l) == name) {
                return Ok((manifest.to_string(), format!("`{name}` is already in Hrs.toml")));
            }
            let mut at = end;
            while at > s + 1 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
            out.insert(at, line.to_string());
            out.join("\n") + "\n"
        }
        None => format!("{}\n\n[dependencies]\n{line}\n", manifest.trim_end()),
    };
    Ok((text, format!("added `{line}` to Hrs.toml")))
}

/// `hrs add name`: `manifest` with the Harsh crate `name` added to its
/// `[dependencies]`, and what to tell the user. Only Harsh's own crates --
/// the four of the distribution now, the registry's when it is built; a Rust
/// crate is `cargo add`'s (the user's rule, 2026-09-29: one command per
/// registry, so a name is never resolved against the wrong one).
#[cfg(feature = "remap")]
pub fn add_dependency(manifest: &str, name: &str) -> Result<(String, String), String> {
    if !DISTRIBUTION.contains(&name) {
        return Err(format!("`{name}` is not a Harsh crate; a Rust crate is added with `cargo add {name}`"));
    }
    let version = dist_version(name).ok_or_else(|| format!("no version for `{name}` in this hrs"))?;
    let line = format!("{name} = \"{version}\"");
    let lines: Vec<&str> = manifest.lines().collect();
    let key = |l: &str| l.split(['=', '.']).next().unwrap_or("").trim().to_string();
    let table = lines.iter().position(|l| l.trim() == "[dependencies]");
    let text = match table {
        Some(s) => {
            let end = (s + 1..lines.len()).find(|&k| lines[k].trim_start().starts_with('[')).unwrap_or(lines.len());
            if lines[s + 1..end].iter().any(|l| key(l) == name) {
                return Ok((manifest.to_string(), format!("`{name}` is already in Hrs.toml")));
            }
            let mut at = end;
            while at > s + 1 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
            out.insert(at, line.clone());
            out.join("\n") + "\n"
        }
        None => format!("{}\n\n[dependencies]\n{line}\n", manifest.trim_end()),
    };
    Ok((text, format!("added `{line}` to Hrs.toml")))
}

/// A project whose Rust names `hrs_std` -- `m~`, `v~`, the matrices -- but
/// whose manifest does not list it: the message to print instead of rustc's
/// "use of undeclared crate `hrs_std`".
#[cfg(feature = "remap")]
pub fn missing_std(root: &Path) -> Option<String> {
    let manifest = std::fs::read_to_string(root.join(HRS_MANIFEST)).unwrap_or_default();
    let listed = manifest.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("hrs_std") && t[7..].trim_start().starts_with(['=', '.'])
    });
    if listed {
        return None;
    }
    let mut stack = vec![Project { root: root.to_path_buf() }.gen_dir()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().map_or(true, |n| n != "hrs_std") {
                    stack.push(p);
                }
            } else if p.extension().map_or(false, |x| x == "rs")
                && std::fs::read_to_string(&p).map_or(false, |t| t.contains("hrs_std::"))
            {
                return Some("this project uses `hrs_std` (`m~`, `v~`, the matrices), which Hrs.toml does not list: run `hrs add hrs_std`".into());
            }
        }
    }
    None
}

/// Write the standard distribution under `.hrs/dist/` (a file
/// only when its text differs, so cargo rebuilds nothing needlessly), and
/// return the `--config` arguments that patch crates.io's requests for its
/// crates to those copies -- only those the `manifests` name, and what they
/// need in turn, since cargo warns of every patch its graph does not use. `hrs_proc_macro` builds on the transpiler's own library:
/// patched to its source where that is on disk (the tree `hrs` was built
/// from, or cargo's copy of the installed `harsh-lang`), else taken from
/// crates.io at this version.
#[cfg(feature = "remap")]
pub fn distribution(root: &Path, manifests: &[PathBuf]) -> Result<Vec<String>, String> {
    // In a workspace cargo reads every member's manifest, wherever it runs:
    // the patches are the whole workspace's, written once, at its root.
    let mut manifests = manifests.to_vec();
    let dist = match workspace_of(root) {
        Some(w) => {
            manifests.push(w.join(EXPORT_DIR).join("Cargo.toml"));
            for m in harsh_members(&w) {
                manifests.push(Project { root: m }.export_dir().join("Cargo.toml"));
            }
            w.join(WORK_DIR).join("dist")
        }
        None => Project { root: root.to_path_buf() }.work_dir().join("dist"),
    };
    let manifests = &manifests[..];
    let toml_path = |p: &Path| p.display().to_string().replace('\\', "\\\\").replace('"', "\\\"");
    // Which crates the manifests name, and what those need: `hrs_syn` needs
    // `hrs_quote`, which needs `hrs_proc_macro`, which needs `harsh-lang`.
    let named = |c: &str| {
        manifests.iter().any(|m| {
            fs::read_to_string(m).map_or(false, |t| {
                // By version only: a `path` dependency -- someone working on
                // a crate of the distribution -- is theirs, and a patch it
                // did not use would draw cargo's warning.
                t.lines().any(|l| {
                    let l = l.trim_start();
                    let names = l.strip_prefix(c).map_or(false, |r| r.trim_start().starts_with('=') || r.starts_with('.'));
                    names && !l.contains("path")
                })
            })
        })
    };
    // A version the distribution does not satisfy would send cargo to
    // crates.io, whose answer -- "candidate versions found which didn't
    // match" -- misleads; say what this `hrs` ships instead (found
    // 2026-09-25: the Book's `proc_hello` still asked for 0.1).
    for c in DISTRIBUTION {
        let shipped = crate::dist::FILES
            .iter()
            .find(|(p, _)| *p == format!("{c}/Cargo.toml"))
            .and_then(|(_, t)| t.lines().find_map(|l| l.strip_prefix("version = \"")?.split('"').next()))
            .unwrap_or("");
        for m in manifests {
            let Ok(t) = fs::read_to_string(m) else { continue };
            for l in t.lines() {
                let l = l.trim_start();
                let Some(r) = l.strip_prefix(c) else { continue };
                if !r.trim_start().starts_with('=') || l.contains("path") {
                    continue;
                }
                let asked = r.split('"').nth(1).unwrap_or("");
                if !asked.is_empty() && !caret_allows(asked, shipped) {
                    return Err(format!(
                        "{}: `{c} = \"{asked}\"` -- this hrs ships {c} {shipped}, in Harsh's standard distribution; ask for \"{}\"",
                        m.display(),
                        shipped.rsplitn(2, '.').nth(1).unwrap_or(shipped)
                    ));
                }
            }
        }
    }
    let syn = named("hrs_syn");
    let quote = syn || named("hrs_quote");
    let procs = quote || named("hrs_proc_macro");
    let wanted = [("hrs_std", named("hrs_std")), ("hrs_proc_macro", procs), ("hrs_quote", quote), ("hrs_syn", syn)];
    // Written only when something uses it: an ordinary project's cargo runs
    // plain, with nothing of Harsh's beside it.
    if wanted.iter().any(|(_, w)| *w) {
        write_distribution(&dist)?;
    }
    let mut args = Vec::new();
    for (c, want) in wanted {
        if want {
            args.push("--config".to_string());
            args.push(format!("patch.crates-io.{c}.path=\"{}\"", toml_path(&dist.join(c))));
        }
    }
    let own = Path::new(env!("CARGO_MANIFEST_DIR"));
    if procs && own.join("src/lib.rs").is_file() && own.join("Cargo.toml").is_file() {
        args.push("--config".to_string());
        args.push(format!("patch.crates-io.harsh-lang.path=\"{}\"", toml_path(own)));
    }
    Ok(args)
}

/// Write Harsh's standard distribution -- `hrs_std`, `hrs_proc_macro`,
/// `hrs_quote`, `hrs_syn` -- into `dir`, one folder per crate, as `hrs build`
/// writes it under a project's `.hrs/dist/`: a file only when its text
/// differs, so cargo rebuilds nothing needlessly. Also `hrs dist <dir>`, for
/// what builds outside a Harsh project and still wants `hrs_std` -- the
/// Jupyter kernel, whose evcxr takes it by `:dep hrs_std = { path = … }`
/// (the user, 2026-09-26).
#[cfg(feature = "remap")]
pub fn write_distribution(dir: &Path) -> Result<(), String> {
    for (path, text) in crate::dist::FILES {
        let file = dir.join(path);
        let text = if *path == "hrs_proc_macro/Cargo.toml" {
            text.replace(", path = \"..\"", "")
        } else {
            text.to_string()
        };
        if fs::read_to_string(&file).ok().as_deref() != Some(text.as_str()) {
            if let Some(parent) = file.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("{}: {}", parent.display(), e))?;
            }
            fs::write(&file, text).map_err(|e| format!("{}: {}", file.display(), e))?;
        }
    }
    Ok(())
}

/// Whether cargo's default (caret) requirement `asked` admits `shipped`:
/// `0.1` admits 0.1.x; `1.2` admits 1.x from 1.2.
#[cfg(feature = "remap")]
fn caret_allows(asked: &str, shipped: &str) -> bool {
    let num = |s: &str| s.trim_start_matches(['^', '=', '~']).split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    let (a, s) = (num(asked), num(shipped));
    let at = |v: &Vec<u64>, i: usize| v.get(i).copied().unwrap_or(0);
    if at(&a, 0) != at(&s, 0) {
        return false;
    }
    if at(&a, 0) == 0 {
        return at(&a, 1) == at(&s, 1) && (a.len() < 3 || at(&s, 2) >= at(&a, 2));
    }
    (at(&s, 1), at(&s, 2)) >= (at(&a, 1), at(&a, 2))
}

/// `prefix` (`hrs_std::` in the exported code, `crate::` in `hrs_std`'s own)
/// rewritten to `crate::<module>::`, where `hrs_std` now lives -- `hrs_std`,
/// in a project's `export/src/` and in a single file (`single_file`) alike
/// -- except before a
/// macro, `each!`, which `#[macro_export]` puts at the crate's root:
/// `crate::each!`. `$crate::` likewise, for the macros' own bodies.
fn vendor_paths(text: &str, prefix: &str, module: &str) -> String {
    let mut out = String::with_capacity(text.len() + 64);
    let mut i = 0;
    let b = text.as_bytes();
    while i < text.len() {
        let at_word_start = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
        let dollar = text[i..].starts_with("$crate::");
        let plain = !dollar && at_word_start && text[i..].starts_with(prefix);
        if dollar || plain {
            let skip = if dollar { "$crate::".len() } else { prefix.len() };
            let rest = &text[i + skip..];
            let name_len = rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len());
            let is_macro = rest[name_len..].starts_with('!') && name_len > 0;
            out.push_str(if dollar { "$crate::" } else { "crate::" });
            if !is_macro {
                out.push_str(module);
                out.push_str("::");
            }
            i += skip;
            continue;
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// `#[cfg(test)] mod name { … }` removed: the export carries the code, not
/// `hrs_std`'s own tests. Found with the Rust lexer, so a brace in a string
/// is text.
fn strip_test_modules(text: &str) -> String {
    let Ok(toks) = crate::lex::lex_rust(text) else { return text.to_string() };
    let sig: Vec<&crate::lex::Token> = toks.iter().filter(|t| !t.is_comment()).collect();
    let mut cuts: Vec<(usize, usize)> = Vec::new();
    let mut k = 0;
    while k + 8 < sig.len() {
        let is_cfg_test = sig[k].text == "#" && sig[k + 1].text == "[" && sig[k + 2].text == "cfg" && sig[k + 3].text == "("
            && sig[k + 4].text == "test" && sig[k + 5].text == ")" && sig[k + 6].text == "]" && sig[k + 7].text == "mod";
        if is_cfg_test && sig.get(k + 9).map_or(false, |t| t.text == "{") {
            let mut d = 0i32;
            let mut j = k + 9;
            while j < sig.len() {
                match sig[j].text.as_str() {
                    "{" => d += 1,
                    "}" => {
                        d -= 1;
                        if d == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            if j < sig.len() {
                cuts.push((sig[k].span.lo as usize, sig[j].span.hi as usize));
                k = j + 1;
                continue;
            }
        }
        k += 1;
    }
    let mut out = text.to_string();
    for (lo, hi) in cuts.into_iter().rev() {
        out.replace_range(lo..hi, "");
    }
    out
}

/// The Rust of a Harsh program as one file, for a place that takes one file
/// and has no `hrs_std`: the Rust Playground, which the website's Playground
/// sends its code to (the user, 2026-09-26). When the code names `hrs_std`,
/// the items of `hrs_std` it reaches follow it as an inline module,
/// `mod hrs_std { … }`, shaken as a build folds them (`shake`), its
/// own modules nested inside. The code comes first, its lines where they
/// were, so rustc's line numbers for it stand; only its `hrs_std::` paths
/// change, to `crate::hrs_std::`. Code that does not name `hrs_std` is
/// returned as it is. `hrs_std`'s dependencies, nalgebra and num-traits, are
/// the receiver's to provide -- the Rust Playground has both.
pub fn single_file(rust: &str) -> String {
    let Ok(toks) = crate::lex::lex_rust(rust) else { return rust.to_string() };
    let roots: std::collections::HashSet<String> =
        toks.into_iter().filter(|x| x.kind == crate::lex::Tk::Ident).map(|x| x.text).collect();
    if !roots.contains("hrs_std") {
        return rust.to_string();
    }
    let code = vendor_paths(rust, "hrs_std::", "hrs_std");
    let mut vendored: Vec<(String, String)> = Vec::new();
    for (path, text) in crate::dist::FILES {
        let Some(name) = path.strip_prefix("hrs_std/src/") else { continue };
        let text = without_harsh_notes(text);
        vendored.push((name.to_string(), strip_test_modules(&vendor_paths(&text, "crate::", "hrs_std"))));
    }
    let shaken = crate::shake::shake(&vendored, &roots);
    let mut lib = shaken.iter().find(|(n, _)| n == "lib.rs").map(|(_, t)| t.clone()).unwrap_or_default();
    for (name, text) in &shaken {
        let Some(module) = name.strip_suffix(".rs") else { continue };
        if module == "lib" {
            continue;
        }
        let decl = format!("mod {module};");
        let inline = format!("mod {module} {{\n{}\n}}", text.trim_end());
        lib = lib.replacen(&decl, &inline, 1);
    }
    format!(
        "{}\n\n/// Harsh's matrices and vectors, as much of them as this program uses.\n#[allow(dead_code, unused_imports, unused_macros)]\nmod hrs_std {{\n{}\n}}\n",
        code.trim_end(),
        lib.trim_end()
    )
}

/// `hrs_std`'s text without its notes on Harsh's workings -- its `//!`
/// lines and the comments that speak of Harsh -- as the Rust it is folded
/// into, or sent with, carries the code and nothing of Harsh.
fn without_harsh_notes(text: &str) -> String {
    text.split_inclusive('\n')
        .filter(|l| {
            let t = l.trim_start();
            !(t.starts_with("//!") || (t.starts_with("//") && speaks_of_harsh(t)))
        })
        .collect()
}

/// Whether a comment is about the Harsh world -- the language, its tool,
/// its sources -- which `export/`, plain Rust, does not carry.
fn speaks_of_harsh(comment: &str) -> bool {
    comment.contains("Harsh") || comment.contains("harsh") || comment.contains("hrs ") || comment.contains(".hrs") || comment.contains("`hrs`")
}

/// A manifest without its `[package.metadata.harsh]` table, for `export/`.
#[cfg(feature = "remap")]
fn without_harsh_table(manifest: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in manifest.split_inclusive('\n') {
        let t = line.trim();
        if t.starts_with('[') {
            skipping = t == "[package.metadata.harsh]";
        }
        if !skipping {
            out.push_str(line);
        }
    }
    out.trim_end().to_string() + "\n"
}

/// Which `hrs` this is: its version and its binary's modification time, so
/// an upgrade -- or a rebuild of `hrs` itself -- is seen.
fn hrs_stamp() -> String {
    let built = std::env::current_exe()
        .and_then(fs::metadata)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    format!("{} {}", env!("CARGO_PKG_VERSION"), built)
}

/// Whether the generated file `rust` is no newer than `time` (a missing one
/// counts as older).
fn older_than(rust: &Path, time: std::time::SystemTime) -> bool {
    fs::metadata(rust).and_then(|m| m.modified()).map_or(true, |r| r <= time)
}

/// Whether a generated file is older than its source. (A newer `hrs` is
/// the stamp's business, `.hrs/stamp`, in `Project::transpile_only`: comparing the binary's
/// time with the file's, as this did, retranspiled for ever after an
/// upgrade, since a file written identically keeps its old time.)
fn is_stale(source: &Path, rust: &Path) -> bool {
    let m = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
    match (m(source), m(rust)) {
        (Some(a), Some(b)) => a >= b,
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
    // A Rust `macro_rules!` is a zone of Rust: blanked out here, put back
    // verbatim after the emitter has run (`rawzone`).
    let (work1, zs) = crate::rawzone::prepare(src);
    let (work, dz) = crate::dslzone::prepare(&work1);
    let (work, regs) = proc_marks(&work, from, src)?;
    let toks = crate::procmac::lex(&work, &regs).map_err(|e| render_error(from, src, e.span, &e.msg))?;
    let (toks, expansions) = expand_harsh_macros(toks, from, src, &crate::mac::NoProcMacros)?;
    if let Some(text) = reread_expansion(&toks, src, &work1, &zs, &dz) {
        // The map of the re-read text points into that text; carry it back
        // to the file the user wrote (an open item since 0.1.26, closed
        // 2026-10-04).
        let anchors = reread_anchors(&toks, &work1, &dz);
        let out = transpile_one(&text, from, rust, map)?;
        remap_reread(map, &anchors);
        return Ok(out);
    }
    let arities = crate::juxt::collect_arities(&toks);
    transpile_tokens(toks, &work, from, rust, map, &arities, &zs, src, &expansions, (&work1, &dz))
}

/// `#[proc_macro~]` marks blanked from the text before it is lexed
/// (`procmac`): the mark is Harsh's and never reaches rustc. The attribute
/// and derive `~` forms are refused here until they are built.
fn proc_marks(work: &str, from: &Path, src: &str) -> Result<(String, Vec<crate::procmac::Registration>), String> {
    crate::procmac::prepare(work).map_err(|e| render_error(from, src, e.span, &e.msg))
}

/// Harsh's own macros unfold here, into Harsh, before the file is read as a
/// program: what follows is ordinary Harsh and the emitted Rust holds no macro
/// of ours (`src/mac.rs`). Only this path expands -- `hrs fmt` and the editor
/// must show the author their macro, not its expansion.
fn expand_harsh_macros(
    toks: Vec<crate::lex::Token>,
    from: &Path,
    src: &str,
    procs: &dyn crate::mac::ProcMacros,
) -> Result<(Vec<crate::lex::Token>, Vec<crate::mac::Expansion>), String> {
    let taken = crate::layout::names_in_scope(&toks);
    crate::mac::expand_all_with(toks, &taken, procs).map_err(|e| render_error(from, src, e.span, &e.msg))
}

/// Rust to Harsh, in memory: what `hrs-from` writes. The converter, then the
/// formatter -- only when the result transpiles, since a converter gap left
/// as the converter wrote it is easier to see than one reflowed -- then the
/// examples in doc comments, converted like the rest. `hrs-from` and the
/// website's Converter both call this, so they cannot disagree (the user,
/// 2026-09-27).
pub fn convert_str(rust: &str) -> Result<String, String> {
    convert_str_warned(rust).map(|(out, _)| out)
}

/// `convert_str`, with the conversion's warnings: each stretch of
/// Rust-shaped text in a macro's body left as written for want of a binding
/// (`bindings`, 2026-10-12).
pub fn convert_str_warned(rust: &str) -> Result<(String, Vec<String>), String> {
    let converted = crate::unbrace::convert(rust)?;
    let warnings = crate::unbrace::take_warnings();
    let mut out = format_if_sound(converted);
    crate::docex::to_harsh_in(&mut out);
    Ok((out, warnings))
}

/// The converter's output through the formatter -- when it transpiles. When
/// it does not (a gap in the converter), it is returned as the converter
/// wrote it: the formatter, guarded only against changing a *working* file,
/// would otherwise reflow the gap away from where it was found.
pub fn format_if_sound(harsh: String) -> String {
    if transpile_str(&harsh).is_ok() {
        crate::fmt::format(&harsh)
    } else {
        harsh
    }
}

/// A whole Harsh source to Rust, in memory: the driver's path without files
/// or a source map. Used for the holes of a Rust macro's brace body, which
/// may hold brace bodies with holes of their own.
pub fn transpile_str(src: &str) -> Result<String, String> {
    crate::juxt::reset_constructors();
    let (work1, zs) = crate::rawzone::prepare(src);
    let (work, dz) = crate::dslzone::prepare(&work1);
    let (work, regs) = crate::procmac::prepare(&work).map_err(|e| e.msg)?;
    let toks = crate::procmac::lex(&work, &regs).map_err(|e| e.msg)?;
    let taken = crate::layout::names_in_scope(&toks);
    let toks = crate::mac::expand_all(toks, &taken).map_err(|e| e.msg)?;
    if let Some(text) = reread_expansion(&toks, src, &work1, &zs, &dz) {
        return transpile_str(&text);
    }
    let arities = crate::juxt::collect_arities(&toks);
    let tree = crate::layout::build_with(toks, &arities).map_err(|e| e.msg)?;
    crate::layout::check_top_level(&tree).map_err(|e| e.msg)?;
    let mut em = crate::emit::Emitter::new(&work);
    em.program(&tree);
    crate::dslzone::restore(&mut em.out, &work1, &dz, &hole, &mut em.map)?;
    crate::rawzone::restore_with_map(&mut em.out, src, &zs, &mut em.map);
    Ok(em.out)
}

/// The text of an expansion that must be read as source: when a `~` macro's
/// expansion produced a Rust macro's brace body, that body is Rust with Harsh
/// only in holes, as a typed one is (the user's rule, 2026-09-23: the macro's
/// author expands to valid Harsh, holes included; the transpiler reads it as
/// it reads a file, and anything else is an error). Bodies are set aside from
/// text, so the expanded file is rendered and read again. `None` when no
/// expansion produced one -- the usual case, which takes the usual path.
/// The rendered text holds the file's own set-asides as markers -- typed
/// bodies as `{/*Zn*/}`, `macro_rules!` zones as comments -- which are put
/// back first, so nothing typed is lost to the re-reading.
fn reread_expansion(
    toks: &[crate::lex::Token],
    src: &str,
    work1: &str,
    zs: &[crate::rawzone::Zone],
    dz: &[crate::dslzone::Zone],
) -> Option<String> {
    let made = toks.windows(3).any(|w| {
        w[1].ctx != 0 && w[1].text == "!" && w[0].kind == crate::lex::Tk::Ident && !w[1].tilde && w[2].kind == crate::lex::Tk::Open('{')
    });
    if !made {
        return None;
    }
    let mut text = crate::mac::render_file(toks);
    for (n, z) in dz.iter().enumerate() {
        let ph = format!("{{/*Z{n}*/}}");
        if let Some(at) = text.find(&ph) {
            text.replace_range(at..at + ph.len(), &work1[z.open..=z.close]);
        }
    }
    crate::rawzone::restore(&mut text, src, zs);
    Some(text)
}

/// Anchors from the re-read text back to the source: each rendered token
/// found in order (its own span, a macro call's for an expansion), and each
/// DSL body put back in place of its placeholder. `(text_lo, src_lo, len)`.
fn reread_anchors(toks: &[crate::lex::Token], work1: &str, dz: &[crate::dslzone::Zone]) -> Vec<(u32, u32, u32)> {
    let rendered = crate::mac::render_file(toks);
    let mut raw = Vec::new();
    let mut cur = 0usize;
    for t in toks {
        if t.text.is_empty() {
            continue;
        }
        if let Some(p) = rendered[cur..].find(t.text.as_str()) {
            let at = cur + p;
            raw.push((at, t.span.lo as usize, t.text.len()));
            cur = at + t.text.len();
        }
    }
    // The placeholders `{/*Zn*/}` become the bodies: shift what follows, and
    // anchor each body at its own place in the source.
    let mut shifts: Vec<(usize, isize, usize, usize)> = Vec::new(); // at, delta, src, len
    for (n, z) in dz.iter().enumerate() {
        let ph = format!("{{/*Z{n}*/}}");
        if let Some(at) = rendered.find(&ph) {
            let body = z.close + 1 - z.open;
            shifts.push((at, body as isize - ph.len() as isize, z.open, body));
        }
    }
    let shift_of = |pos: usize| -> isize { shifts.iter().filter(|s| s.0 < pos).map(|s| s.1).sum() };
    // A token's offset is the blanked text's, where a placeholder longer than
    // its body's first line pushed what follows along (`Zone::extra`): back
    // to the file's own offset.
    let mut work_opens = Vec::new();
    let mut cum = 0usize;
    for z in dz {
        work_opens.push((z.open + cum, z.extra));
        cum += z.extra;
    }
    let to_source = |w: usize| -> usize { w - work_opens.iter().filter(|&&(o, _)| w > o).map(|&(_, x)| x).sum::<usize>() };
    let mut out: Vec<(u32, u32, u32)> = raw
        .into_iter()
        // A token inside a placeholder is the placeholder's, whatever its
        // length: `{/*Z1*/}` is 8 bytes, `{/*Z10*/}` 9 (it was taken as 9
        // always, dropping the token right after a short one -- found writing
        // the developer book, 0.7.3).
        .filter(|&(at, _, _)| !shifts.iter().any(|s| at >= s.0 && at < s.0 + (s.3 as isize - s.1) as usize))
        .map(|(at, src, len)| (((at as isize) + shift_of(at)) as u32, to_source(src) as u32, len as u32))
        .collect();
    let _ = work1;
    for s in &shifts {
        out.push((((s.0 as isize) + shift_of(s.0)) as u32, s.2 as u32, s.3 as u32));
    }
    out.sort();
    out
}

/// Rewrite a map written for re-read text so its source offsets are the
/// user's file's, through the anchors. The map is ours (written just above
/// by `transpile_tokens`), so its JSON is edited as text: no JSON crate is
/// needed where `harsh-lang` is a library.
fn remap_reread(map: &Path, anchors: &[(u32, u32, u32)]) {
    let Ok(raw) = fs::read_to_string(map) else { return };
    let tr = |off: u64| -> u64 {
        let off = off as u32;
        match anchors.iter().rev().find(|a| a.0 <= off) {
            Some(&(t, s, len)) => (s + (off - t).min(len)) as u64,
            None => off as u64,
        }
    };
    // Each `[n,n,n,n,n]` in "entries", and the two numbers after the name in
    // each `[ctx,"name",lo,hi]` of "expansions".
    let (Some(e0), Some(x0)) = (raw.find("\"entries\":["), raw.find("],\"expansions\":[")) else { return };
    let head = &raw[..e0 + 11];
    let entries = &raw[e0 + 11..x0];
    let tail = &raw[x0..];
    let mut out = String::from(head);
    for (k, item) in entries.split("],[").enumerate() {
        let inner = item.trim_start_matches('[').trim_end_matches(']');
        let mut n: Vec<u64> = inner.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        if n.len() == 5 {
            let lo = tr(n[2]);
            let hi = tr(n[3].saturating_sub(1)) + 1;
            n[2] = lo;
            n[3] = hi.max(lo + 1);
        }
        if k > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&n.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
        out.push(']');
    }
    let xs = &tail[16..tail.len().saturating_sub(2)];
    out.push_str("],\"expansions\":[");
    for (k, item) in xs.split("],[").enumerate().filter(|(_, it)| !it.is_empty()) {
        let inner = item.trim_start_matches('[').trim_end_matches(']');
        let parts: Vec<&str> = inner.rsplitn(3, ',').collect();
        if k > 0 {
            out.push(',');
        }
        if parts.len() == 3 {
            let hi: u64 = parts[0].trim().parse().unwrap_or(0);
            let lo: u64 = parts[1].trim().parse().unwrap_or(0);
            out.push_str(&format!("[{},{},{}]", parts[2], tr(lo), tr(hi.saturating_sub(1)) + 1));
        } else {
            out.push('[');
            out.push_str(inner);
            out.push(']');
        }
    }
    out.push_str("]}");
    let _ = fs::write(map, out);
}

/// One hole of a brace body, Harsh to Rust.
fn hole(code: &str) -> Result<String, String> {
    crate::dslzone::transpile_hole(code, &transpile_str)
}

/// Transpile one file with arities known from the whole project.
pub fn transpile_one_with(
    src: &str,
    from: &Path,
    rust: &Path,
    map: &Path,
    arities: &std::collections::HashMap<String, usize>,
) -> Result<PathBuf, String> {
    transpile_one_procs(src, from, rust, map, arities, &crate::mac::NoProcMacros)
}

/// `transpile_one_with`, with the project's Harsh proc macros callable.
pub fn transpile_one_procs(
    src: &str,
    from: &Path,
    rust: &Path,
    map: &Path,
    arities: &std::collections::HashMap<String, usize>,
    procs: &dyn crate::mac::ProcMacros,
) -> Result<PathBuf, String> {
    let (work1, zs) = crate::rawzone::prepare(src);
    let (work, dz) = crate::dslzone::prepare(&work1);
    let (work, regs) = proc_marks(&work, from, src)?;
    let toks = crate::procmac::lex(&work, &regs).map_err(|e| render_error(from, src, e.span, &e.msg))?;
    let (toks, expansions) = expand_harsh_macros(toks, from, src, procs)?;
    if let Some(text) = reread_expansion(&toks, src, &work1, &zs, &dz) {
        // Back to the user's file, as in `transpile_one`.
        let anchors = reread_anchors(&toks, &work1, &dz);
        let out = transpile_one_procs(&text, from, rust, map, arities, procs)?;
        remap_reread(map, &anchors);
        return Ok(out);
    }
    transpile_tokens(toks, &work, from, rust, map, arities, &zs, src, &expansions, (&work1, &dz))
}

fn transpile_tokens(
    toks: Vec<crate::lex::Token>,
    src: &str,
    from: &Path,
    rust: &Path,
    map: &Path,
    arities: &std::collections::HashMap<String, usize>,
    zones: &[crate::rawzone::Zone],
    original: &str,
    expansions: &[crate::mac::Expansion],
    dsl: (&str, &[crate::dslzone::Zone]),
) -> Result<PathBuf, String> {
    let tree = crate::layout::build_with(toks, arities)
        .map_err(|e| render_error(from, src, crate::mac::at_call(e.span, expansions), &e.msg))?;
    crate::layout::check_top_level(&tree)
        .map_err(|e| render_error(from, src, crate::mac::at_call(e.span, expansions), &e.msg))?;
    // What a derive removed -- its `#[derive~ …]`, the helper attributes it
    // consumed -- is not copied back through a gap (`mac::erase`).
    let emitted = crate::mac::erase(src, expansions);
    let mut em = crate::emit::Emitter::new(&emitted);
    // `include_str!` paths are relative to the source file; the Rust is
    // written elsewhere, so they are re-based from there.
    // Not when the Rust is in `export/src/`, where `src/`'s other files are
    // mirrored: there a path means what it means in `src/`.
    if let (Some(fd), Some(rd)) = (from.parent(), rust.parent()) {
        if !mirrors(fd, rd) {
            em.set_include_base(fd, rd);
        }
    }
    em.program(&tree);
    // A proc macro's tokens sit in a region of their own past the file
    // (`mac::PROC_BASE`); their map entries go to the call that produced
    // them, before the restores below shift anything. `ctx` stays, so the
    // remapper still says which macro it was.
    for e in em.map.iter_mut() {
        let s = crate::mac::at_call(crate::lex::Span { lo: e.src_lo, hi: e.src_hi }, expansions);
        e.src_lo = s.lo;
        e.src_hi = s.hi;
    }
    // A fenced doc example is Harsh; rustdoc reads Rust. Rewriting the
    // generated text (and shifting the map by what it changes) keeps the
    // comment a comment everywhere else in the transpiler.
    crate::docex::to_rust_in(&mut em.out, &mut em.map, src)
        .map_err(|(span, msg)| render_error(from, src, span, &msg))?;
    // A Rust macro's brace bodies go back with their holes transpiled, then
    // the Rust zones, byte for byte.
    crate::dslzone::restore(&mut em.out, dsl.0, dsl.1, &hole, &mut em.map)
        .map_err(|m| format!("{}: {m}", from.display()))?;
    crate::rawzone::restore_with_map(&mut em.out, original, zones, &mut em.map);
    // `hrs_std` folded into the crate (0.9.0): its paths are the crate's own.
    #[cfg(feature = "remap")]
    if FOLD_STD.with(|f| f.get()) {
        fold_std_paths(&mut em.out, &mut em.map);
    }

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
        j.push_str(&format!("[{},{},{},{},{}]", e.gen_lo, e.gen_hi, e.src_lo, e.src_hi, e.ctx));
    }
    j.push_str("],\"expansions\":[");
    for (k, x) in expansions.iter().enumerate() {
        if k > 0 {
            j.push(',');
        }
        j.push_str(&format!("[{},", x.ctx));
        json_str(&mut j, &x.name);
        j.push_str(&format!(",{},{}]", x.call_lo, x.call_hi));
    }
    j.push_str("]}");
    if let Some(d) = map.parent() {
        fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
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

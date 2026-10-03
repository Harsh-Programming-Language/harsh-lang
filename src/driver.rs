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
#[cfg(feature = "remap")]
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(feature = "remap")]
use std::process::Stdio;
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

    /// Refuse to build unless `Cargo.toml` points a target at the generated
    /// tree. Without that, cargo auto-discovers `src/main.rs` and quietly builds
    /// the wrong program -- a success that runs something else is worse than
    /// an error.
    pub fn check_manifest(&self) -> Result<(), String> {
        let path = self.root.join("Cargo.toml");
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
        if text.contains(GEN_DIR) {
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
        let stamp_path = self.root.join(GEN_DIR).join(".hrs-stamp");
        let stamp = hrs_stamp();
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
            if !force && !older_than_runner && !new_hrs && !is_stale(&u.source, &u.rust) {
                continue;
            }
            let rust = transpile_one_procs(&src, &u.source, &u.rust, &u.map, &arities, procs)?;
            rebuilt.push(rust);
        }
        if new_hrs {
            let _ = fs::create_dir_all(self.root.join(GEN_DIR));
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

    /// Build the runner of the proc macros this project uses, if it uses
    /// any: each listed crate transpiled, the runner's sources written under
    /// `target/hrs/proc-macros/` when they change, and cargo run over it --
    /// cargo's own freshness check is the cache (decision P4).
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
            seen.push(canon);
            let dep = Project { root };
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
            crates.push(crate::procmac::MacroCrate { lib: package.replace('-', "_"), package, root, macros });
        }
        let runtime = runtime.ok_or_else(|| {
            "a Harsh proc-macro crate depends on `hrs_proc_macro`; none of the listed crates does".to_string()
        })?;
        let dir = self.root.join(GEN_DIR).join("proc-macros");
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

    /// Write the project as a plain Rust crate under `dir`: `Cargo.toml` with
    /// its targets pointed at `src/`, every `.hrs` transpiled to `src/**.rs`,
    /// no source maps, and `cargo fmt` run over the result when rustfmt is
    /// installed. This is the hand-off artifact -- what a Rust reader, a
    /// reviewer, or a crates.io upload sees. `target/hrs/` is not for
    /// reading: it keeps the source layout so diagnostics map back, and
    /// formatting it would break that map.
    pub fn export(&self, dir: &Path) -> Result<Export, String> {
        self.check_manifest()?;
        // `export` is pure Rust, for reading what the Harsh becomes (the
        // user, 2026-09-25; `docs/dev/PACKAGING.md`): a Harsh macro crate has
        // no Rust meaning, and is refused.
        if self.meta()?.proc_macro {
            return Err(format!(
                "{}: a Harsh macro crate is not exported: its macros exist only in Harsh, expanded where they are called, and have no Rust meaning",
                self.root.join("Cargo.toml").display()
            ));
        }
        let units = self.units()?;
        let runner = self.proc_runner()?;
        let procs: &dyn crate::mac::ProcMacros = match &runner {
            Some(r) => r,
            None => &crate::mac::NoProcMacros,
        };
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
            // An expansion that produced a brace body is read again as source
            // (`reread_expansion`), before anything else.
            let src: String = {
                let (w1, z) = crate::rawzone::prepare(&src);
                let (w, d) = crate::dslzone::prepare(&w1);
                let (w, r) = proc_marks(&w, &u.source, &src)?;
                let t = crate::procmac::lex(&w, &r).map_err(|e| render_error(&u.source, &src, e.span, &e.msg))?;
                let (t, _) = expand_harsh_macros(t, &u.source, &src, procs)?;
                reread_expansion(&t, &src, &w1, &z, &d).unwrap_or_else(|| src.to_string())
            };
            let (work1, zs) = crate::rawzone::prepare(&src);
            let (work, dz) = crate::dslzone::prepare(&work1);
            let (work, regs) = proc_marks(&work, &u.source, &src)?;
            let toks = crate::procmac::lex(&work, &regs)
                .map_err(|e| render_error(&u.source, &src, e.span, &e.msg))?;
            let (toks, expansions) = expand_harsh_macros(toks, &u.source, &src, procs)?;
            let tree = crate::layout::build_with(toks, &arities)
                .map_err(|e| render_error(&u.source, &src, crate::mac::at_call(e.span, &expansions), &e.msg))?;
            crate::layout::check_top_level(&tree)
                .map_err(|e| render_error(&u.source, &src, crate::mac::at_call(e.span, &expansions), &e.msg))?;
            // What a derive removed is not copied back through a gap.
            let emitted = crate::mac::erase(&work, &expansions);
            let mut em = crate::emit::Emitter::new(&emitted);
            em.program(&tree);
            // The exported crate is built by cargo alone, doc tests included,
            // so its doc examples are translated here as well.
            crate::docex::to_rust_in(&mut em.out, &mut em.map, &work)
                .map_err(|(span, msg)| render_error(&u.source, &src, span, &msg))?;
            crate::dslzone::restore(&mut em.out, &work1, &dz, &hole, &mut em.map)
                .map_err(|m| format!("{}: {m}", u.source.display()))?;
            crate::rawzone::restore_with_map(&mut em.out, &src, &zs, &mut em.map);
            if let Some(d) = rust.parent() {
                fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            fs::write(&rust, &em.out).map_err(|e| format!("{}: {}", rust.display(), e))?;
            files.push(rust);
        }
        // `hrs_std` is a Harsh crate, and the export carries none (the user,
        // 2026-09-25; `PACKAGING.md` section 4): when the code uses it, its
        // sources become the crate's own module, `matrix`, and its Rust
        // dependencies replace it in the manifest. Module-level for now --
        // the whole of `hrs_std` when any of it is used; item-level shaking
        // is the next step.
        // (A named function and `cfg!`, not a nested closure under an
        // attributed `let`: the converter misread that -- the self-host,
        // 2026-09-25.)
        let uses_std = cfg!(feature = "remap") && exports_name_hrs_std(&files);
        #[cfg(feature = "remap")]
        if uses_std {
            for f in &files {
                let t = fs::read_to_string(f).map_err(|e| e.to_string())?;
                fs::write(f, vendor_paths(&t, "hrs_std::", "matrix")).map_err(|e| e.to_string())?;
            }
            let module = out_src.join("matrix");
            fs::create_dir_all(&module).map_err(|e| e.to_string())?;
            let mut vendored: Vec<(String, String)> = Vec::new();
            for (path, text) in crate::dist::FILES {
                let Some(name) = path.strip_prefix("hrs_std/src/") else { continue };
                let target = if name == "lib.rs" { module.join("mod.rs") } else { module.join(name) };
                // Its `//!` notes describe Harsh's own workings; the export is
                // plain Rust, and carries the code only.
                let text: String = text
                    .split_inclusive('\n')
                    .filter(|l| {
                        let t = l.trim_start();
                        !(t.starts_with("//!") || (t.starts_with("//") && speaks_of_harsh(t)))
                    })
                    .collect();
                let text = strip_test_modules(&vendor_paths(&text, "crate::", "matrix"));
                vendored.push((target.display().to_string(), text));
            }
            // Item level: only what the exported code reaches, closed over
            // what each kept item uses (`shake`). The roots are every name the
            // exported code writes -- generous on purpose.
            let mut roots = std::collections::HashSet::new();
            for f in &files {
                if let Ok(t) = fs::read_to_string(f) {
                    if let Ok(toks) = crate::lex::lex_rust(&t) {
                        roots.extend(toks.into_iter().filter(|x| x.kind == crate::lex::Tk::Ident).map(|x| x.text));
                    }
                }
            }
            for (target, text) in crate::shake::shake(&vendored, &roots) {
                fs::write(&target, text).map_err(|e| format!("{target}: {e}"))?;
            }
            for root in ["main.rs", "lib.rs"] {
                let f = out_src.join(root);
                if let Ok(t) = fs::read_to_string(&f) {
                    let decl = "/// Matrices, vectors and element-wise operations.\n#[allow(dead_code, unused_imports, unused_macros)]\nmod matrix;\n\n";
                    fs::write(&f, format!("{decl}{t}")).map_err(|e| e.to_string())?;
                }
            }
        }
        // The manifest: the same file with its targets under `src/`, which
        // is also where cargo would find them unaided.
        let manifest = self.root.join("Cargo.toml");
        let text = fs::read_to_string(&manifest).map_err(|e| format!("{}: {}", manifest.display(), e))?;
        let text = text.replace(&format!("{}/", GEN_DIR), &format!("{}/", SRC_DIR));
        // No Harsh table in pure Rust: `[package.metadata.harsh]` stays in
        // the Harsh project.
        let text = without_harsh_table(&text);
        // No `hrs_std`: its own dependencies instead, when the code used it.
        let text = {
            // Nor a manifest comment about the Harsh sources, which the export
            // does not have.
            #[cfg_attr(not(feature = "remap"), allow(unused_mut))]
            let mut lines: Vec<String> = text
                .lines()
                .filter(|l| {
                    let t = l.trim_start();
                    !t.starts_with("hrs_std") && !(t.starts_with('#') && !t.starts_with("#[") && speaks_of_harsh(t))
                })
                .map(String::from)
                .collect();
            #[cfg(feature = "remap")]
            if uses_std {
                let std_manifest = crate::dist::FILES.iter().find(|(p, _)| *p == "hrs_std/Cargo.toml").map(|(_, t)| *t).unwrap_or("");
                let deps: Vec<&str> = std_manifest
                    .lines()
                    .skip_while(|l| l.trim() != "[dependencies]")
                    .skip(1)
                    .take_while(|l| !l.trim_start().starts_with('['))
                    .filter(|l| !l.trim().is_empty())
                    .collect();
                match lines.iter().position(|l| l.trim() == "[dependencies]") {
                    Some(i) => {
                        for (k, d) in deps.iter().enumerate() {
                            lines.insert(i + 1 + k, d.to_string());
                        }
                    }
                    None => {
                        lines.push(String::new());
                        lines.push("[dependencies]".into());
                        lines.extend(deps.iter().map(|d| d.to_string()));
                    }
                }
            }
            lines.join("\n") + "\n"
        };
        fs::write(dir.join("Cargo.toml"), text).map_err(|e| e.to_string())?;
        for extra in ["Cargo.lock", "README.md", "LICENSE"] {
            // A lock naming `hrs_std`, which the export no longer has, is
            // left for cargo to write afresh.
            if extra == "Cargo.lock" && uses_std {
                continue;
            }
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
    ///
    /// Needs the `remap` feature: rustc's JSON diagnostics are read and
    /// remapped onto the Harsh sources. Without it the library is the pure
    /// language -- lexer, layout, expander, emitter -- with no dependencies,
    /// which is what `hrs_proc_macro` builds on.
    #[cfg(feature = "remap")]
    /// Before Cargo, the Harsh level (the user's design, 2026-10-03): with
    /// an `Hrs.toml`, each Harsh crate it names by path is transpiled and
    /// prepared in turn (its own Harsh crates first), and the generated
    /// `target/hrs/Cargo.toml` -- the user's `Cargo.toml` plus the Harsh
    /// dependencies -- is written. Its path, for `--manifest-path`; `None`
    /// for a project without `Hrs.toml`, built from its `Cargo.toml` as ever.
    #[cfg(feature = "remap")]
    pub fn prepare(&self, seen: &mut Vec<PathBuf>) -> Result<Option<PathBuf>, String> {
        let Some(m) = read_hrs_manifest(&self.root)? else { return Ok(None) };
        let mut cargo_text = fs::read_to_string(&m.cargo).map_err(|e| format!("{}: {}", m.cargo.display(), e))?;
        let cargo_dir = m.cargo.parent().unwrap_or(&self.root).to_path_buf();
        // Cargo finds a `build.rs` beside its manifest unasked; the generated
        // manifest is elsewhere, so it is named.
        let script = cargo_dir.join("build.rs");
        if script.is_file() && !cargo_text.lines().any(|l| l.trim_start().starts_with("build =")) {
            cargo_text = cargo_text.replacen("[package]", &format!("[package]\nbuild = \"{}\"", manifest_path_str(&script)), 1);
        }
        let mut extra = Vec::new();
        for (name, value) in &m.deps {
            let mut value = value.clone();
            if let Some(i) = value.find("path = \"") {
                let at = i + 8;
                let Some(len) = value[at..].find('"') else {
                    return Err(format!("{}: `{name}`: an unclosed path", HRS_MANIFEST));
                };
                let rel = value[at..at + len].to_string();
                // Harsh's own crates -- `hrs_std` from a checkout -- are
                // Rust: passed through, the path made absolute.
                if DISTRIBUTION.contains(&name.as_str()) {
                    value.replace_range(at..at + len, &manifest_path_str(&self.root.join(&rel)));
                    extra.push(format!("{name} = {value}"));
                    continue;
                }
                let dir = self.root.join(&rel);
                let dir = dir
                    .canonicalize()
                    .map_err(|e| format!("{}: the Harsh crate `{name}` at {}: {e}", HRS_MANIFEST, dir.display()))?;
                let has_own = dir.join(HRS_MANIFEST).is_file();
                if !seen.contains(&dir) {
                    seen.push(dir.clone());
                    let dep = Project { root: dir.clone() };
                    dep.transpile(false)?;
                    dep.prepare(seen)?;
                }
                // A Harsh crate with its own Hrs.toml is built from its
                // generated manifest; one without, from its own Cargo.toml.
                let at_dir = if has_own { dir.join(GEN_DIR) } else { dir.clone() };
                value.replace_range(at..at + len, &manifest_path_str(&at_dir));
            }
            extra.push(format!("{name} = {value}"));
        }
        let text = generated_manifest(&cargo_text, &cargo_dir, &extra);
        let gen = self.root.join(GEN_DIR).join("Cargo.toml");
        fs::create_dir_all(gen.parent().unwrap()).map_err(|e| format!("{}: {e}", gen.display()))?;
        if fs::read_to_string(&gen).ok().as_deref() != Some(text.as_str()) {
            fs::write(&gen, &text).map_err(|e| format!("{}: {e}", gen.display()))?;
        }
        // The lockfile stays the project's: copied in here, back after Cargo.
        let lock = cargo_dir.join("Cargo.lock");
        if lock.is_file() {
            let _ = fs::copy(&lock, gen.with_file_name("Cargo.lock"));
        }
        Ok(Some(gen))
    }

    #[cfg(feature = "remap")]
    pub fn cargo(&self, sub: &str, args: &[String], maps: &[PathBuf]) -> Result<i32, String> {
        #[cfg(feature = "remap")]
        let generated = self.prepare(&mut Vec::new())?;
        #[cfg(not(feature = "remap"))]
        let generated: Option<PathBuf> = None;
        let result = self.cargo_run(sub, args, maps, generated.as_deref());
        if let Some(g) = &generated {
            let back = g.with_file_name("Cargo.lock");
            if back.is_file() {
                let _ = fs::copy(&back, self.root.join("Cargo.lock"));
            }
        }
        result
    }

    #[cfg(feature = "remap")]
    fn cargo_run(&self, sub: &str, args: &[String], maps: &[PathBuf], generated: Option<&Path>) -> Result<i32, String> {
        let manifest = generated.map(|g| g.to_path_buf()).unwrap_or_else(|| self.root.join("Cargo.toml"));
        // The generated manifest, with the project's own `target/` -- unless
        // the user chose a target directory (`CARGO_TARGET_DIR`, or
        // `target-dir` in a `.cargo/config.toml`), which then wins.
        let chosen = std::env::var_os("CARGO_TARGET_DIR").is_some()
            || ["config.toml", "config"].iter().any(|f| {
                fs::read_to_string(self.root.join(".cargo").join(f)).map_or(false, |t| t.contains("target-dir"))
            });
        let at: Vec<String> = match generated {
            Some(g) if chosen => vec!["--manifest-path".into(), g.display().to_string()],
            Some(g) => vec![
                "--manifest-path".into(),
                g.display().to_string(),
                "--target-dir".into(),
                self.root.join("target").display().to_string(),
            ],
            None => Vec::new(),
        };
        // Only cargo's own build commands speak `--message-format`; an
        // external subcommand (`cargo leptos`, `cargo doc`'s cousins) is run
        // plainly, its output passed through unmapped.
        if !matches!(sub, "build" | "run" | "test" | "check" | "clippy" | "bench" | "doc") {
            let status = Command::new("cargo")
                .args(distribution(&self.root, &[manifest.clone()])?)
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
        cmd.args(distribution(&self.root, &[manifest.clone()])?);
        cmd.arg(sub)
            .arg("--message-format=json-diagnostic-rendered-ansi")
            .args(&at)
            .args(args)
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        // A program that panics names the generated file,
        // `target/hrs/main.rs:4:20`. For `run` and `test` its stderr is read
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
/// panic's `target/hrs/main.rs:4:20` back on its Harsh line.
#[cfg(feature = "remap")]
pub struct PanicPlaces {
    files: Vec<(String, String, crate::remap::SourceMap)>, // (generated, source), relative to the root
}

#[cfg(feature = "remap")]
impl PanicPlaces {
    pub fn load(root: &Path, maps: &[PathBuf]) -> Self {
        let rel = |p: &str| -> String {
            let p = Path::new(p);
            p.strip_prefix(root).unwrap_or(p).display().to_string()
        };
        let files = maps
            .iter()
            .filter_map(|m| crate::remap::SourceMap::load(&m.display().to_string()).ok())
            .map(|m| (rel(m.generated()), rel(m.source()), m))
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
/// `name = { path = "…" }` entries under `[dependencies]`: the paths.
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
/// under `target/hrs/dist/` and patches them in.
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
        kept.push(raw.to_string());
    }
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

/// The generated `Cargo.toml` (`target/hrs/Cargo.toml`): the user's, with
/// every relative `path`/`build` made absolute (the file moved), its own
/// `[workspace]` (so a workspace above it does not claim it), and `extra`
/// dependency lines added to `[dependencies]`.
/// A path as a TOML string's contents: forward slashes, which Cargo reads
/// on every platform.
pub fn manifest_path_str(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

pub fn generated_manifest(cargo_text: &str, cargo_dir: &Path, extra: &[String]) -> String {
    let mut out = Vec::new();
    let mut has_deps = false;
    let mut has_workspace = false;
    for raw in cargo_text.lines() {
        let mut line = raw.to_string();
        for key in ["path", "build"] {
            let mut search = 0;
            while let Some(i) = line[search..].find(&format!("{key} = \"")) {
                let at = search + i + key.len() + 4;
                let Some(len) = line[at..].find('"') else { break };
                let value = line[at..at + len].to_string();
                let abs = if Path::new(&value).is_absolute() { value.clone() } else { manifest_path_str(&cargo_dir.join(&value)) };
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
    if !has_workspace {
        out.push(String::new());
        out.push("# Its own workspace root: generated by hrs from Cargo.toml and Hrs.toml.".into());
        out.push("[workspace]".into());
    }
    out.join("\n") + "\n"
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

/// `hrs add name`: `manifest` with the Harsh crate `name` added to its
/// `[dependencies]`, and what to tell the user. Only Harsh's own crates --
/// the four of the distribution now, the registry's when it is built; a Rust
/// crate is `cargo add`'s (the user's rule, 2026-09-29: one command per
/// registry, so a name is never resolved against the wrong one).
#[cfg(feature = "remap")]
/// Insert `line` for `name` into a manifest's `[dependencies]`, unless
/// `name` is there already.
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
    let mut stack = vec![root.join("target").join("hrs")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().map_or(true, |n| n != "dist" && n != ".complete") {
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

/// Write the standard distribution under `root/target/hrs/dist/` (a file
/// only when its text differs, so cargo rebuilds nothing needlessly), and
/// return the `--config` arguments that patch crates.io's requests for its
/// crates to those copies -- only those the `manifests` name, and what they
/// need in turn, since cargo warns of every patch its graph does not use. `hrs_proc_macro` builds on the transpiler's own library:
/// patched to its source where that is on disk (the tree `hrs` was built
/// from, or cargo's copy of the installed `harsh-lang`), else taken from
/// crates.io at this version.
#[cfg(feature = "remap")]
pub fn distribution(root: &Path, manifests: &[PathBuf]) -> Result<Vec<String>, String> {
    let dist = root.join(GEN_DIR).join("dist");
    write_distribution(&dist)?;
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
/// writes it under a project's `target/hrs/dist/`: a file only when its text
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
/// rewritten to `crate::<module>::`, where `hrs_std` now lives -- `matrix` in
/// an export, `hrs_std` in a single file (`single_file`) -- except before a
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
/// `mod hrs_std { … }`, shaken as `hrs export` shakes them (`shake`), its
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
        // As in an export: the code, not the notes on Harsh's workings.
        let text: String = text
            .split_inclusive('\n')
            .filter(|l| {
                let t = l.trim_start();
                !(t.starts_with("//!") || (t.starts_with("//") && speaks_of_harsh(t)))
            })
            .collect();
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

/// Whether any exported file names `hrs_std`.
fn exports_name_hrs_std(files: &[PathBuf]) -> bool {
    for f in files {
        if let Ok(t) = fs::read_to_string(f) {
            if t.contains("hrs_std::") {
                return true;
            }
        }
    }
    false
}

/// Whether a comment is about the Harsh world -- the language, its tool,
/// its sources -- which the export, plain Rust, does not carry.
fn speaks_of_harsh(comment: &str) -> bool {
    comment.contains("Harsh") || comment.contains("harsh") || comment.contains("hrs ") || comment.contains(".hrs") || comment.contains("`hrs`")
}

/// A manifest without its `[package.metadata.harsh]` table, for `export`.
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
/// `.hrs-stamp`'s business, in `Project::transpile`: comparing the binary's
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
        return transpile_one(&text, from, rust, map);
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
    let mut out = format_if_sound(crate::unbrace::convert(rust)?);
    crate::docex::to_harsh_in(&mut out);
    Ok(out)
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
        return transpile_one_procs(&text, from, rust, map, arities, procs);
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
    if let (Some(fd), Some(rd)) = (from.parent(), rust.parent()) {
        em.set_include_base(fd, rd);
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

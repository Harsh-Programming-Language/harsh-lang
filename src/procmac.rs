// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Harsh's own procedural macros: the registration mark (ruling 17,
//! `docs/dev/PROC-MACRO-PLAN.md`).
//!
//! A Harsh proc macro is an ordinary `pub fn` marked `#[proc_macro~]`. The
//! mark is not Rust, so it must not reach rustc: this pass finds it, records
//! the name of the function it marks, and **blanks its bytes** in the text
//! the rest of the pipeline reads -- the shape `rawzone` uses for
//! `macro_rules!` zones. Blanking rather than removing tokens keeps every
//! offset, line and source-map entry where it was, and the emitter, which
//! copies source whitespace between tokens, finds only spaces there.
//!
//! **Every `~` expands in Harsh and none reaches Rust** (the user's rule,
//! 2026-09-24). A derive call, `#[derive~ Name]`, and an attribute-like
//! macro's, `#[name~ arguments]`, are left here for the expander, which runs
//! them on the item beneath (`mac::expand_derive`, `mac::expand_attribute`);
//! passed through, they would reach rustc as `#[name!(…)]`, Rust nobody wrote.

use crate::lex::{Span, Tk, Token};
use crate::mac::Error;

/// One `#[proc_macro~]`: the function it marks, and where the mark stood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    /// The name a call uses: the function's for `name~ stream`, the derive's
    /// for `#[derive~ Name]` (Rust's `#[proc_macro_derive(Name)]`).
    pub name: String,
    /// The `pub fn` the mark stands on, which the runner calls.
    pub func: String,
    pub kind: Kind,
    pub span: Span,
    /// The mark's line, and its column when it began that line: a mark
    /// written inline, `#[proc_macro~] pub fn f …`, leaves the item first on
    /// its line once blanked, and the item takes this column back (`lex`).
    pub line: usize,
    pub col: Option<usize>,
}

/// What a registered proc macro is called as, mirroring Rust's
/// `#[proc_macro]` and `#[proc_macro_derive(Name, attributes(…))]` (the
/// user's rule, 2026-09-24: for all Harsh proc macros, mirror Rust's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Function,
    /// `helpers` are the attributes the derive declares for the item it is
    /// on (`#[show skip]`); they are the derive's to read and are removed
    /// from the item once it has run.
    Derive { helpers: Vec<String> },
    /// Rust's `#[proc_macro_attribute]`: the attribute's arguments and the
    /// item, the item replaced by what it returns.
    Attribute,
}

const MARK: &str = "proc_macro";
const DERIVE_MARK: &str = "proc_macro_derive";
const ATTRIBUTE_MARK: &str = "proc_macro_attribute";

/// Blank every `#[proc_macro~]` in `src`, returning the text the pipeline
/// reads next and the functions registered, in source order. The text is
/// `src` with the attributes' bytes replaced by spaces: same length, same
/// lines. A file that does not lex is returned untouched -- the lexer
/// reports it on the next read, with its own message.
pub fn prepare(src: &str) -> Result<(String, Vec<Registration>), Error> {
    let Ok(toks) = crate::lex::lex(src) else {
        return Ok((src.to_string(), Vec::new()));
    };
    let mut regs = Vec::new();
    let mut blank: Vec<Span> = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        // `#[` or `#![`, the `#` tight against what follows.
        let Some(open) = attribute_open(&toks, i) else {
            i += 1;
            continue;
        };
        let Some(close) = matching_close(&toks, open) else {
            i += 1;
            continue;
        };
        let head = open + 1;
        let marked = head + 1 < close
            && toks[head].kind == Tk::Ident
            && toks[head + 1].text == "~"
            && toks[head].span.hi == toks[head + 1].span.lo;
        if !marked {
            i = close + 1;
            continue;
        }
        let head_name = toks[head].text.clone();
        let at = Span::new(toks[i].span.lo as usize, toks[close].span.hi as usize);
        match head_name.as_str() {
            // A derive *call*: the expander's business, with the item beneath.
            "derive" => {
                i = close + 1;
                continue;
            }
            MARK | DERIVE_MARK | ATTRIBUTE_MARK => {}
            // An attribute-like macro's *call*, `#[route~ GET "/"]`: the
            // expander's business, with the item beneath -- as a derive's.
            _ => {
                i = close + 1;
                continue;
            }
        }
        if toks[i + 1].text == "!" {
            let msg = format!("`#[{head_name}~ …]` marks a function; it is not an inner attribute");
            return Err(Error { span: at, msg });
        }
        let (kind, derive) = if head_name == MARK || head_name == ATTRIBUTE_MARK {
            if head + 2 != close {
                let msg = format!("`#[{head_name}~]` takes no arguments");
                return Err(Error { span: at, msg });
            }
            // (A binding, not an `if` inside the tuple: the converter's gap
            // with `if` expressions as arguments -- ROADMAP, known bugs.)
            let kind = if head_name == MARK { Kind::Function } else { Kind::Attribute };
            (kind, None)
        } else {
            let (derive, helpers) = derive_args(&toks[head + 2..close]).ok_or_else(|| Error {
                span: at,
                msg: "`#[proc_macro_derive~ Name]` names its derive, and may declare the helper attributes it reads: \
                      `#[proc_macro_derive~ Name (attributes helper other)]`"
                    .into(),
            })?;
            (Kind::Derive { helpers }, Some(derive))
        };
        let func = marked_fn(&toks, close + 1).ok_or_else(|| Error {
            span: at,
            msg: format!("`#[{head_name}~ …]` marks a `pub fn`, and none follows it"),
        })?;
        // As rustc: an attribute macro takes two streams, the others one.
        let wanted = if kind == Kind::Attribute { 2 } else { 1 };
        let got = marked_params(&toks, close + 1);
        if got != Some(wanted) {
            // (Bindings, not `if` expressions as `format!` arguments, which
            // `hrs-from` cannot write yet -- ROADMAP, known bugs.)
            let which = if kind == Kind::Attribute { "attribute" } else { "this" };
            let takes = if wanted == 2 {
                "two streams, `(attr: TokenStream) (item: TokenStream)`"
            } else {
                "one stream, `(input: TokenStream)`"
            };
            let msg = format!("{which} proc macro has incorrect signature: its `pub fn` takes {takes}");
            return Err(Error { span: at, msg });
        }
        let name = derive.unwrap_or_else(|| func.clone());
        regs.push(Registration { name, func, kind, span: at, line: toks[i].line, col: toks[i].line_start });
        blank.push(at);
        i = close + 1;
    }
    let mut out = src.to_string();
    for s in &blank {
        let (lo, hi) = (s.lo as usize, s.hi as usize);
        // The attribute is one line of ASCII by construction (`#[proc_macro~]`,
        // spacing aside); a multi-line one keeps its newlines.
        let spaces: String = src[lo..hi].chars().map(|c| if c == '\n' { '\n' } else { ' ' }).collect();
        out.replace_range(lo..hi, &spaces);
    }
    Ok((out, regs))
}

/// Lex text `prepare` blanked. A token that followed an inline mark on the
/// mark's line is now first on that line, further right than the item ever
/// was; it takes the mark's column back, so the item stands where the mark
/// stood (found 2026-09-24: `#[proc_macro~] pub fn f` inline was refused as
/// over-indented).
pub fn lex(text: &str, regs: &[Registration]) -> Result<Vec<Token>, crate::lex::LexError> {
    let mut toks = crate::lex::lex(text)?;
    for r in regs {
        let Some(col) = r.col else { continue };
        if let Some(t) = toks.iter_mut().find(|t| t.span.lo >= r.span.hi) {
            if t.line == r.line && t.line_start.is_some() {
                t.line_start = Some(col);
            }
        }
    }
    Ok(toks)
}

/// `Name` or `Name (attributes a b)` -- Rust's `Name` and
/// `Name, attributes(a, b)` in Harsh's attribute spelling (juxtaposed, the
/// group isolated). The name and the helpers, or `None` for any other shape.
fn derive_args(toks: &[Token]) -> Option<(String, Vec<String>)> {
    let (first, rest) = toks.split_first()?;
    if first.kind != Tk::Ident {
        return None;
    }
    if rest.is_empty() {
        return Some((first.text.clone(), Vec::new()));
    }
    let [open, word, helpers @ .., close] = rest else { return None };
    let shaped = open.kind == Tk::Open('(')
        && close.kind == Tk::Close(')')
        && word.text == "attributes"
        && !helpers.is_empty()
        && helpers.iter().all(|t| t.kind == Tk::Ident);
    shaped.then(|| (first.text.clone(), helpers.iter().map(|t| t.text.clone()).collect()))
}

/// The index of the `[` when `toks[i]` begins an attribute, `#[` or `#![`.
fn attribute_open(toks: &[Token], i: usize) -> Option<usize> {
    if toks[i].kind != Tk::Hash {
        return None;
    }
    let mut k = i + 1;
    if toks.get(k).map_or(false, |t| t.text == "!") {
        k += 1;
    }
    (toks.get(k)?.kind == Tk::Open('[')).then_some(k)
}

fn matching_close(toks: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (k, t) in toks.iter().enumerate().skip(open) {
        match t.kind {
            Tk::Open(_) => depth += 1,
            Tk::Close(_) => {
                depth -= 1;
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

/// The name of the `pub fn` after a `#[proc_macro~]`, skipping comments and
/// other attributes (`#[inline]`, doc comments) as Rust would. `pub(crate)`
/// is not `pub`: the runner calls the function from another crate.
fn marked_fn(toks: &[Token], mut k: usize) -> Option<String> {
    loop {
        let t = toks.get(k)?;
        if t.is_comment() {
            k += 1;
        } else if let Some(open) = attribute_open(toks, k) {
            k = matching_close(toks, open)? + 1;
        } else {
            break;
        }
    }
    let public = toks.get(k)?.is_kw("pub") && toks.get(k + 1)?.is_kw("fn");
    let name = toks.get(k + 2)?;
    (public && name.kind == Tk::Ident).then(|| name.text.clone())
}

/// How many parameter groups the marked `pub fn` has: `(attr: T) (item: T)`
/// is two.
fn marked_params(toks: &[Token], mut k: usize) -> Option<usize> {
    loop {
        let t = toks.get(k)?;
        if t.is_comment() {
            k += 1;
        } else if let Some(open) = attribute_open(toks, k) {
            k = matching_close(toks, open)? + 1;
        } else {
            break;
        }
    }
    k += 3; // `pub fn name`
    let mut n = 0;
    while toks.get(k).map_or(false, |t| t.kind == Tk::Open('(')) {
        k = matching_close(toks, k)? + 1;
        n += 1;
    }
    Some(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO: &str = "use hrs_proc_macro.TokenStream\n\n#[proc_macro~]\npub fn hello_macro (input: TokenStream) -> TokenStream:\n    input\n";

    #[test]
    fn a_registration_is_recorded_and_blanked() {
        let (out, regs) = prepare(HELLO).unwrap();
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].name, "hello_macro");
        assert_eq!(out.len(), HELLO.len());
        assert!(!out.contains("proc_macro~"));
        assert_eq!(out.lines().nth(2), Some("              "));
    }

    #[test]
    fn other_attributes_and_comments_may_sit_between() {
        let src = "#[proc_macro~]\n/// Says hello.\n#[inline]\npub fn hi (i: TokenStream) -> TokenStream:\n    i\n";
        assert_eq!(prepare(src).unwrap().1[0].name, "hi");
    }

    #[test]
    fn the_mark_needs_a_pub_fn() {
        for src in [
            "#[proc_macro~]\nfn hi (i: T) -> T:\n    i\n",
            "#[proc_macro~]\npub(crate) fn hi (i: T) -> T:\n    i\n",
            "#[proc_macro~]\npub struct S\n",
            "#[proc_macro~]\n",
        ] {
            let e = prepare(src).unwrap_err();
            assert!(e.msg.contains("marks a `pub fn`"), "{src}: {}", e.msg);
        }
    }

    #[test]
    fn the_mark_takes_no_arguments_and_is_not_inner() {
        assert!(prepare("#[proc_macro~ (x)]\npub fn f (i: T) -> T:\n    i\n").unwrap_err().msg.contains("no arguments"));
        assert!(prepare("#![proc_macro~]\n").unwrap_err().msg.contains("inner attribute"));
    }

    #[test]
    fn rust_attributes_are_not_touched() {
        let src = "#[derive(Debug)]\n#[proc_macro]\n#[tokio.main]\n#[doc = concat! \"a\" \"b\"]\npub fn f$:\n    ()\n";
        let (out, regs) = prepare(src).unwrap();
        assert_eq!(out, src);
        assert!(regs.is_empty());
    }

    /// Rust's `#[proc_macro_derive(Describe, attributes(describe))]`, in
    /// Harsh's attribute spelling: the derive's name is what calls use, the
    /// function is what the runner calls.
    #[test]
    fn a_derive_registers_its_name_its_function_and_its_helpers() {
        let src = "#[proc_macro_derive~ Describe]\npub fn describe (i: TokenStream) -> TokenStream:\n    i\n\n\
                   #[proc_macro_derive~ Show (attributes show hide)]\npub fn show (i: TokenStream) -> TokenStream:\n    i\n";
        let (out, regs) = prepare(src).unwrap();
        assert!(!out.contains("proc_macro_derive~"));
        let got: Vec<(&str, &str, &Kind)> = regs.iter().map(|r| (r.name.as_str(), r.func.as_str(), &r.kind)).collect();
        // (Short bindings: in its width-reflowed layout the converter splits
        // a struct literal holding a `vec!` in a form the transpiler refuses
        // -- ROADMAP, known bugs, not isolated.)
        let none = Kind::Derive { helpers: Vec::new() };
        let two = Kind::Derive { helpers: vec!["show".to_string(), "hide".to_string()] };
        assert_eq!(got, [("Describe", "describe", &none), ("Show", "show", &two)]);
    }

    #[test]
    fn a_derive_mark_has_one_shape() {
        for args in ["", "(Describe)", "Describe, Other", "Describe (attrs x)", "Describe (attributes)", "Describe (attributes (x))"] {
            let src = format!("#[proc_macro_derive~ {args}]\npub fn d (i: T) -> T:\n    i\n");
            let e = prepare(&src).unwrap_err();
            assert!(e.msg.contains("names its derive"), "{args}: {}", e.msg);
        }
    }

    /// A derive *call* is the expander's, with the item beneath it; `prepare`
    /// leaves it in the text.
    #[test]
    fn a_derive_call_is_left_for_the_expander() {
        let src = "#[derive~ Describe]\nstruct P\n    x: i32\n";
        assert_eq!(prepare(src).unwrap().0, src);
    }

    #[test]
    fn a_mark_in_a_string_or_comment_is_text() {
        let src = "// #[proc_macro~]\nfn f$:\n    \"#[derive~ X]\"\n";
        assert_eq!(prepare(src).unwrap().0, src);
    }
}

// ---------------------------------------------------------------------------
// The manifest: `[package.metadata.harsh]`
// ---------------------------------------------------------------------------

/// What a project's `Cargo.toml` says about Harsh proc macros, under
/// `[package.metadata.harsh]`: whether the crate *is* a proc-macro crate, and
/// which proc-macro crates it *uses* (paths relative to its root).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Meta {
    pub proc_macro: bool,
    pub proc_macros: Vec<String>,
}

/// Read `[package.metadata.harsh]` from a manifest's text. Lines are scanned
/// as `driver::check_hrs_std` does -- no TOML crate. The table holds two keys
/// and nothing else, so a typo is an error, not a silently ignored line.
pub fn read_meta(manifest: &str) -> Result<Meta, String> {
    let mut meta = Meta::default();
    let mut inside = false;
    let mut pending: Option<String> = None;
    for raw in manifest.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if let Some(acc) = pending.as_mut() {
            acc.push(' ');
            acc.push_str(line);
            if line.contains(']') {
                let text = pending.take().unwrap();
                meta.proc_macros = string_array(&text)?;
            }
            continue;
        }
        if line.starts_with('[') {
            inside = line == "[package.metadata.harsh]";
            continue;
        }
        if !inside || line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("[package.metadata.harsh]: `{line}` is not `key = value`"));
        };
        let value = value.trim();
        match key.trim() {
            "proc-macro" => {
                meta.proc_macro = match value {
                    "true" => true,
                    "false" => false,
                    _ => return Err(format!("[package.metadata.harsh]: `proc-macro` is `true` or `false`, not `{value}`")),
                };
            }
            "proc-macros" => {
                if value.contains(']') {
                    meta.proc_macros = string_array(value)?;
                } else {
                    pending = Some(value.to_string());
                }
            }
            other => {
                return Err(format!(
                    "[package.metadata.harsh]: unknown key `{other}`; the keys are `proc-macro = true` \
                     (this crate defines Harsh proc macros) and `proc-macros = [\"../path\"]` (it uses them)"
                ))
            }
        }
    }
    if pending.is_some() {
        return Err("[package.metadata.harsh]: `proc-macros = [` is never closed".into());
    }
    Ok(meta)
}

/// `["a", "b"]` -> `a`, `b`.
fn string_array(text: &str) -> Result<Vec<String>, String> {
    let inner = text
        .trim()
        .strip_prefix('[')
        .and_then(|t| t.trim_end().strip_suffix(']'))
        .ok_or_else(|| format!("[package.metadata.harsh]: `proc-macros` is a list of paths, `[\"../macros\"]`, not `{}`", text.trim()))?;
    inner
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .map(str::to_string)
                .ok_or_else(|| format!("[package.metadata.harsh]: `{s}` in `proc-macros` is not a quoted path"))
        })
        .collect()
}

/// `[package] name`, as cargo names the library: hyphens become underscores.
pub fn crate_name(manifest: &str) -> Option<String> {
    package_name(manifest).map(|n| n.replace('-', "_"))
}

/// `[package] name`, as written.
pub fn package_name(manifest: &str) -> Option<String> {
    let mut inside = false;
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            inside = line == "[package]";
            continue;
        }
        if inside {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == "name" {
                    return Some(v.trim().trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

/// The `hrs_proc_macro = …` dependency line of a macro crate, for the runner
/// to depend on the very same crate (one `TokenStream` type). A `path` in it
/// is relative to that crate, so it is made absolute from `root`.
pub fn runtime_dependency(manifest: &str, root: &std::path::Path) -> Option<String> {
    let line = manifest
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("hrs_proc_macro") && l["hrs_proc_macro".len()..].trim_start().starts_with('='))?;
    let Some(at) = line.find("path") else {
        return Some(line.to_string());
    };
    let rest = &line[at..];
    let open = rest.find('"')? + 1;
    let close = open + rest[open..].find('"')?;
    let rel = &rest[open..close];
    let abs = root.join(rel);
    let abs = abs.canonicalize().unwrap_or(abs);
    Some(format!("{}{}{}", &line[..at + open], abs.display(), &rest[close..]))
}

// ---------------------------------------------------------------------------
// The runner
// ---------------------------------------------------------------------------

/// One proc-macro crate a project uses: its cargo package name, the name its
/// library is known by in Rust, its root, and the macros it registers.
#[derive(Debug, Clone)]
pub struct MacroCrate {
    pub package: String,
    pub lib: String,
    pub root: std::path::PathBuf,
    pub macros: Vec<Registration>,
}

/// The runner's `Cargo.toml` and `main.rs` (decision P4: generated per
/// consumer, under `target/src/proc-macros/`). `runtime` is the macro
/// crates' own `hrs_proc_macro` dependency line, so the runner and the
/// macros share one `TokenStream` type. The empty `[workspace]` keeps the
/// runner out of any workspace the consumer belongs to.
pub fn runner_sources(crates: &[MacroCrate], runtime: &str) -> (String, String) {
    let mut manifest = String::from(
        "# Generated by hrs: the runner of this project's Harsh proc macros.\n\
         [package]\nname = \"hrs-proc-runner\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n\
         [[bin]]\nname = \"hrs-proc-runner\"\npath = \"main.rs\"\n\n[dependencies]\n",
    );
    for c in crates {
        manifest.push_str(&format!(
            "{} = {{ path = \"{}\", package = \"{}\" }}\n",
            c.lib,
            c.root.display(),
            c.package
        ));
    }
    manifest.push_str(runtime);
    manifest.push_str("\n\n[workspace]\n");
    let mut main = String::from("// Generated by hrs: dispatches one request to the named macro.\nfn main() {\n    hrs_proc_macro::serve(&[\n");
    // Each call name to its function: a derive is called by its own name
    // (`Describe`) and implemented by the function the mark stands on.
    for c in crates {
        for m in &c.macros {
            let kind = if m.kind == Kind::Attribute { "Attribute" } else { "Function" };
            main.push_str(&format!("        (\"{}\", hrs_proc_macro::Registered::{kind}({}::{})),\n", m.name, c.lib, m.func));
        }
    }
    main.push_str("    ])\n}\n");
    (manifest, main)
}

/// The built runner, as the expander calls it.
pub struct Runner {
    pub bin: std::path::PathBuf,
    pub macros: Vec<Registration>,
}

impl crate::mac::ProcMacros for Runner {
    fn has(&self, name: &str) -> bool {
        self.macros.iter().any(|m| m.name == name && m.kind == Kind::Function)
    }

    fn derive_helpers(&self, name: &str) -> Option<Vec<String>> {
        self.macros.iter().find(|m| m.name == name).and_then(|m| match &m.kind {
            Kind::Derive { helpers } => Some(helpers.clone()),
            _ => None,
        })
    }

    fn is_attribute(&self, name: &str) -> bool {
        self.macros.iter().any(|m| m.name == name && m.kind == Kind::Attribute)
    }

    /// Decision P2: `name`, a newline and the stream on stdin; the expansion
    /// on stdout. The runner reads all of stdin before it writes, so writing
    /// the request and then waiting cannot deadlock.
    fn expand(&self, name: &str, input: &str) -> Result<String, String> {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut child = Command::new(&self.bin)
            .env("RUST_BACKTRACE", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("the proc-macro runner did not start ({}): {e}", self.bin.display()))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(format!("{name}\n{input}").as_bytes());
        }
        let out = child.wait_with_output().map_err(|e| format!("the proc-macro runner: {e}"))?;
        if out.status.success() {
            return String::from_utf8(out.stdout).map_err(|_| "its expansion is not UTF-8".to_string());
        }
        Err(runner_error(&String::from_utf8_lossy(&out.stderr)))
    }
}

/// What a failed run said, readably: a panic as `panicked: MESSAGE` without
/// the generated file's location or the backtrace note (both about the
/// runner's Rust, not the macro's Harsh); any other message as it is.
pub fn runner_error(stderr: &str) -> String {
    let lines: Vec<&str> = stderr.lines().filter(|l| !l.starts_with("note: run with `RUST_BACKTRACE")).collect();
    if let Some(k) = lines.iter().position(|l| l.starts_with("thread '") && l.contains("panicked at")) {
        let head = lines[k];
        // Rust before 1.73: `panicked at 'MESSAGE', file:line:col`.
        if let Some(q) = head.find("panicked at '") {
            let rest = &head[q + "panicked at '".len()..];
            let msg = rest.rfind("', ").map_or(rest, |e| &rest[..e]);
            return format!("panicked: {msg}");
        }
        // Since 1.73: `panicked at file:line:col:` and the message beneath.
        let msg = lines[k + 1..].join("\n");
        return format!("panicked: {}", msg.trim());
    }
    let all = lines.join("\n");
    let all = all.trim();
    if all.is_empty() { "it failed without a message".to_string() } else { all.to_string() }
}

#[cfg(test)]
mod runner_tests {
    use super::*;

    #[test]
    fn the_runner_dispatches_every_registered_macro() {
        let reg = |name: &str, func: &str, kind: Kind| Registration {
            name: name.into(),
            func: func.into(),
            kind,
            span: Span::new(0, 0),
            line: 1,
            col: Some(0),
        };
        let c = MacroCrate {
            package: "hello-macros".into(),
            lib: "hello_macros".into(),
            root: "/p/hello".into(),
            macros: vec![
                reg("hello_macro", "hello_macro", Kind::Function),
                reg("twice", "twice", Kind::Function),
                reg("Describe", "describe", Kind::Derive { helpers: vec![] }),
            ],
        };
        let (manifest, main) = runner_sources(&[c], "hrs_proc_macro = \"0.1\"");
        assert!(manifest.contains("hello_macros = { path = \"/p/hello\", package = \"hello-macros\" }\nhrs_proc_macro = \"0.1\"\n\n[workspace]\n"), "{manifest}");
        assert!(main.contains("(\"hello_macro\", hrs_proc_macro::Registered::Function(hello_macros::hello_macro)),"), "{main}");
        assert!(main.contains("(\"twice\", hrs_proc_macro::Registered::Function(hello_macros::twice)),"), "{main}");
        assert!(main.contains("(\"Describe\", hrs_proc_macro::Registered::Function(hello_macros::describe)),"), "{main}");
    }

    #[test]
    fn a_panic_reads_as_the_macros_message() {
        let new = "thread 'main' panicked at target/src/lib.rs:7:5:\nnot yet\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n";
        assert_eq!(runner_error(new), "panicked: not yet");
        let old = "thread 'main' panicked at 'not yet', target/src/lib.rs:7:5\nnote: run with `RUST_BACKTRACE=1` …\n";
        assert_eq!(runner_error(old), "panicked: not yet");
        assert_eq!(runner_error("no proc macro `x` in this crate\n"), "no proc macro `x` in this crate");
        assert_eq!(runner_error(""), "it failed without a message");
    }
}

#[cfg(test)]
mod meta_tests {
    use super::*;

    #[test]
    fn reads_both_keys_and_nothing_else() {
        let m = read_meta("[package]\nname = \"x\"\n\n[package.metadata.harsh]\nproc-macro = true # a comment\n").unwrap();
        assert_eq!(m, Meta { proc_macro: true, proc_macros: vec![] });
        let m = read_meta("[package.metadata.harsh]\nproc-macros = [\"../hello\", \"../more\"]\n[dependencies]\n").unwrap();
        assert_eq!(m.proc_macros, ["../hello", "../more"]);
        let m = read_meta("[package.metadata.harsh]\nproc-macros = [\n    \"../a\",\n    \"../b\",\n]\n").unwrap();
        assert_eq!(m.proc_macros, ["../a", "../b"]);
        assert_eq!(read_meta("[dependencies]\nproc-macro = 1\n").unwrap(), Meta::default());
    }

    #[test]
    fn refuses_what_it_does_not_know() {
        assert!(read_meta("[package.metadata.harsh]\nproc_macro = true\n").unwrap_err().contains("unknown key `proc_macro`"));
        assert!(read_meta("[package.metadata.harsh]\nproc-macro = yes\n").unwrap_err().contains("`true` or `false`"));
        assert!(read_meta("[package.metadata.harsh]\nproc-macros = \"../a\"\n").unwrap_err().contains("never closed"));
        assert!(read_meta("[package.metadata.harsh]\nproc-macros = [../a]\n").unwrap_err().contains("not a quoted path"));
    }

    #[test]
    fn names_and_the_runtime_line() {
        assert_eq!(crate_name("[package]\nname = \"hello-macros\"\n").as_deref(), Some("hello_macros"));
        let root = std::path::Path::new("/nowhere/hello");
        let line = runtime_dependency("[dependencies]\nhrs_proc_macro = { path = \"../rt\" }\n", root).unwrap();
        assert_eq!(line, "hrs_proc_macro = { path = \"/nowhere/hello/../rt\" }");
        let line = runtime_dependency("[dependencies]\nhrs_proc_macro = \"0.1\"\n", root).unwrap();
        assert_eq!(line, "hrs_proc_macro = \"0.1\"");
    }
}

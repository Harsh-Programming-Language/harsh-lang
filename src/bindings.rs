// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The binding table `hrs-from` decides holes by (the user's ruling,
//! 2026-10-12).
//!
//! **A hole is a Rust expression in a macro's body that calls on something
//! bound outside the DSL's own language -- a variable, a function, a name
//! the DSL binds -- or uses an operator, which is a function too.** A single
//! word is exempt: `{count}` reads the same in both languages. The converter
//! finds candidates by position (after `:` or `=`, in a brace pair, a format
//! slot, a splice, the DSL's control flow, an arm); this module decides.
//!
//! Nothing is guessed from a name's case or layout: a name counts when the
//! table holds it. The table is the standard prelude, the file's items, the
//! names its `use`s import, and the bindings in scope at the macro call --
//! the enclosing function's parameters, its `let`s, `for`s and patterns
//! before the call -- plus the names the DSL binds itself (`let:item`,
//! `for item in …`, an arm's `Some(v)`) and those the expression binds
//! (its closures' parameters). A glob import, `use leptos::prelude::*`,
//! brings names nobody can list without the crate; they are not in the
//! table, and an expression built only from them is left as written, with
//! a warning (the user's choice, 2026-10-12).
//!
//! The transpiler keeps no such table: Harsh's layout never depends on what
//! a name means (`docs/GOVERNANCE.md`). This one serves the converter alone.

use crate::lex::{Tk, Token};
use std::collections::HashSet;

/// The names every Rust file can use without importing them: the standard
/// prelude's types, traits, functions and macros.
const PRELUDE: &[&str] = &[
    "Some", "None", "Ok", "Err", "Option", "Result", "Vec", "String", "Box", "ToString", "ToOwned", "Clone", "Copy", "Default",
    "Iterator", "IntoIterator", "Extend", "FromIterator", "DoubleEndedIterator", "ExactSizeIterator", "Into", "From", "TryFrom",
    "TryInto", "AsRef", "AsMut", "Drop", "Fn", "FnMut", "FnOnce", "PartialEq", "Eq", "PartialOrd", "Ord", "Send", "Sync", "Sized",
    "Unpin", "drop", "std", "core", "alloc", "crate", "self", "Self", "super", "println", "print", "eprintln", "eprint", "format",
    "vec", "panic", "assert", "assert_eq", "assert_ne", "debug_assert", "debug_assert_eq", "debug_assert_ne", "write", "writeln",
    "matches", "todo", "unimplemented", "unreachable", "dbg", "concat", "stringify", "include_str", "include_bytes", "env",
    "option_env", "line", "column", "file", "module_path", "cfg", "format_args", "char", "str", "bool", "u8", "u16", "u32", "u64",
    "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32", "f64",
];

/// Rust's keywords: never a binding.
fn keyword(s: &str) -> bool {
    matches!(
        s,
        "as" | "async" | "await" | "break" | "const" | "continue" | "dyn" | "else" | "enum" | "extern" | "false" | "fn" | "for"
            | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub" | "ref" | "return" | "static"
            | "struct" | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while"
    )
}

/// An operator: a function by another name (`+` is `Add::add`), and the
/// access operators `.` and `::` (the user, 2026-10-12): `item.name`,
/// `Duration::ZERO`. `=` is not one -- assignment has no trait -- and
/// neither is `=>`, `:` or `,`.
fn operator(t: &Token) -> bool {
    matches!(
        t.text.as_str(),
        "+" | "-" | "*" | "/" | "%" | "^" | "!" | "&" | "|" | "&&" | "||" | "<<" | ">>" | "==" | "!=" | "<" | ">" | "<=" | ">="
            | "+=" | "-=" | "*=" | "/=" | "%=" | "^=" | "&=" | "|=" | "<<=" | ">>=" | "?" | ".." | "..=" | "." | "::"
    )
}

/// The file's own names: its items, its imports and the prelude.
pub fn file_names(toks: &[Token]) -> HashSet<String> {
    let mut set: HashSet<String> = PRELUDE.iter().map(|s| s.to_string()).collect();
    let sig: Vec<&Token> = toks.iter().filter(|t| !t.is_comment()).collect();
    let mut i = 0;
    while i < sig.len() {
        let t = sig[i];
        // An item's name: `fn f`, `struct S`, `macro_rules! m`, …
        if matches!(t.text.as_str(), "fn" | "struct" | "enum" | "const" | "static" | "type" | "trait" | "mod" | "union") {
            let mut k = i + 1;
            if sig.get(k).map_or(false, |n| n.text == "mut") {
                k += 1;
            }
            if let Some(n) = sig.get(k).filter(|n| n.kind == Tk::Ident && !keyword(&n.text)) {
                set.insert(n.text.clone());
            }
        }
        if t.text == "macro_rules" && sig.get(i + 1).map_or(false, |b| b.text == "!") {
            if let Some(n) = sig.get(i + 2) {
                set.insert(n.text.clone());
            }
        }
        // An enum's variants: the names one level inside its braces.
        if t.text == "enum" {
            if let Some(o) = (i + 1..sig.len()).find(|&k| sig[k].kind == Tk::Open('{')) {
                let (mut d, mut expect, mut k) = (0i32, true, o);
                while k < sig.len() {
                    match sig[k].kind {
                        Tk::Open(_) => d += 1,
                        Tk::Close(_) => {
                            d -= 1;
                            if d == 0 {
                                break;
                            }
                        }
                        Tk::Comma if d == 1 => expect = true,
                        Tk::Ident if d == 1 && expect => {
                            set.insert(sig[k].text.clone());
                            expect = false;
                        }
                        _ => {}
                    }
                    k += 1;
                }
            }
        }
        // `use a::b::{c, d as e, f::*}`: `c`, `e` (a glob brings nothing
        // nameable); `a::b::{self}` names `b`.
        if t.text == "use" {
            let end = (i + 1..sig.len()).find(|&k| sig[k].kind == Tk::Semi).unwrap_or(sig.len());
            let mut k = i + 1;
            while k < end {
                let n = sig[k];
                if n.kind == Tk::Ident && !keyword(&n.text) || n.text == "self" {
                    let next = sig.get(k + 1).map(|x| x.text.as_str());
                    let leaf = !matches!(next, Some("::"));
                    if leaf {
                        if next == Some("as") {
                            if let Some(a) = sig.get(k + 2) {
                                set.insert(a.text.clone());
                            }
                            k += 3;
                            continue;
                        }
                        if n.text == "self" {
                            // `b::{self}`: the module before the group.
                            if let Some(m) = (i + 1..k).rev().find(|&j| sig[j].kind == Tk::Ident && sig[j].text != "self") {
                                set.insert(sig[m].text.clone());
                            }
                        } else {
                            set.insert(n.text.clone());
                        }
                    }
                }
                k += 1;
            }
            i = end;
        }
        i += 1;
    }
    set
}

/// The names a pattern binds: each name in it that is not a path, a
/// constructor applied (`Some(..)`, `P { .. }`), a macro, or a constant the
/// table already names.
pub fn pattern_names(pat: &[&Token], known: &HashSet<String>) -> Vec<String> {
    let mut out = Vec::new();
    for (k, t) in pat.iter().enumerate() {
        if t.kind != Tk::Ident || keyword(&t.text) || t.text == "_" {
            continue;
        }
        let next = pat.get(k + 1).map(|n| n.text.as_str());
        let prev = if k > 0 { Some(pat[k - 1].text.as_str()) } else { None };
        if matches!(next, Some("(") | Some("{") | Some("::") | Some("!")) || prev == Some("::") {
            continue;
        }
        // `field: pat` in a struct pattern: the field is no binding.
        if next == Some(":") && prev.map_or(false, |p| p == "{" || p == ",") && pat.iter().take(k).any(|t| t.text == "{") {
            continue;
        }
        if known.contains(&t.text) {
            continue;
        }
        out.push(t.text.clone());
    }
    out
}

/// The bindings in scope at `pos`: the enclosing function's parameters and
/// every `let`, `for`, `if let`/`while let`, match arm and closure pattern
/// from its start to `pos` (a block's end is not tracked: a name bound in a
/// closed block before the call counts, which errs towards a hole).
pub fn locals_before(toks: &[Token], pos: usize, known: &HashSet<String>) -> HashSet<String> {
    let sig: Vec<(usize, &Token)> = toks.iter().enumerate().filter(|(_, t)| !t.is_comment()).collect();
    // The innermost `fn` whose body is open at `pos`.
    let mut fn_starts: Vec<(usize, i32)> = Vec::new(); // (sig index of `fn`, depth of its body)
    let mut d = 0i32;
    let mut pending: Option<usize> = None;
    for (n, &(i, t)) in sig.iter().enumerate() {
        if i >= pos {
            break;
        }
        match t.kind {
            Tk::Open('{') => {
                d += 1;
                if let Some(f) = pending.take() {
                    fn_starts.push((f, d));
                }
            }
            Tk::Close('}') => {
                while fn_starts.last().map_or(false, |&(_, fd)| fd == d) {
                    fn_starts.pop();
                }
                d -= 1;
            }
            Tk::Semi => pending = None,
            _ if t.text == "fn" => pending = Some(n),
            _ => {}
        }
    }
    let mut out = HashSet::new();
    let Some(&(start, _)) = fn_starts.last() else { return out };
    let span: Vec<&Token> = sig[start..].iter().take_while(|(i, _)| *i < pos).map(|(_, t)| *t).collect();
    // Parameters: `name: Type` at the parameter list's depth.
    if let Some(o) = span.iter().position(|t| t.text == "(") {
        let mut d = 0i32;
        let mut seg: Vec<&Token> = Vec::new();
        for t in &span[o..] {
            match t.kind {
                Tk::Open(_) => {
                    d += 1;
                    if d == 1 {
                        continue;
                    }
                }
                Tk::Close(_) => {
                    d -= 1;
                    if d == 0 {
                        break;
                    }
                }
                _ => {}
            }
            if d == 1 && t.kind == Tk::Comma {
                seg.clear();
                continue;
            }
            if d == 1 && t.kind == Tk::Colon && !seg.is_empty() {
                out.extend(pattern_names(&seg, known));
                seg.clear();
                seg.push(t); // the type follows: nothing more to bind
                continue;
            }
            if seg.first().map_or(true, |s| s.kind != Tk::Colon) {
                seg.push(t);
            }
        }
    }
    // `let PAT [: T] =`, `for PAT in`, `if let PAT =`, `PAT =>`, `|PAT|`.
    let mut k = 0;
    while k < span.len() {
        let t = span[k];
        if t.text == "let" || t.text == "for" {
            let stop = |u: &Token| matches!(u.text.as_str(), "=" | "in" | ":" | ";");
            let mut d = 0i32;
            let mut j = k + 1;
            let mut pat: Vec<&Token> = Vec::new();
            while j < span.len() {
                let u = span[j];
                if d == 0 && stop(u) {
                    break;
                }
                match u.kind {
                    Tk::Open(_) => d += 1,
                    Tk::Close(_) => d -= 1,
                    _ => {}
                }
                pat.push(u);
                j += 1;
            }
            out.extend(pattern_names(&pat, known));
            k = j;
            continue;
        }
        if t.text == "=>" {
            // The arm's pattern: back to the previous `,`, `{` or `}` at its depth.
            let mut d = 0i32;
            let mut j = k;
            let mut pat: Vec<&Token> = Vec::new();
            while j > 0 {
                j -= 1;
                let u = span[j];
                match u.kind {
                    Tk::Close(_) => d += 1,
                    Tk::Open(_) => {
                        if d == 0 {
                            break;
                        }
                        d -= 1;
                    }
                    Tk::Comma if d == 0 => break,
                    _ => {}
                }
                if d == 0 && u.text == "}" {
                    break;
                }
                pat.insert(0, u);
            }
            if let Some(g) = pat.iter().position(|u| u.text == "if") {
                pat.truncate(g);
            }
            out.extend(pattern_names(&pat, known));
        }
        if let Some(params) = closure_params(&span, k) {
            out.extend(pattern_names(&params.0, known));
            k = params.1;
            continue;
        }
        k += 1;
    }
    out
}

/// At `k`, a closure's parameter list `|a, (b, c): T|`: its tokens (types
/// dropped) and the index after its closing bar. `||` binds nothing.
fn closure_params<'a>(span: &[&'a Token], k: usize) -> Option<(Vec<&'a Token>, usize)> {
    let t = span[k];
    if t.text != "|" {
        return None;
    }
    let opens = k == 0 || {
        let p = span[k - 1];
        matches!(p.kind, Tk::Open(_) | Tk::Comma | Tk::Eq | Tk::FatArrow | Tk::Semi) || p.text == "move" || p.text == "return" || (operator(p) && p.text != "|")
    };
    if !opens {
        return None;
    }
    let mut j = k + 1;
    let mut d = 0i32;
    let mut out: Vec<&Token> = Vec::new();
    let mut in_type = false;
    while j < span.len() {
        let u = span[j];
        match u.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            _ => {}
        }
        if d == 0 && u.text == "|" {
            return Some((out, j + 1));
        }
        if d == 0 && u.kind == Tk::Comma {
            in_type = false;
        } else if d == 0 && u.kind == Tk::Colon {
            in_type = true;
        } else if !in_type {
            out.push(u);
        }
        j += 1;
    }
    None
}

/// The names a DSL body binds itself, from `open` to `close`: Leptos's
/// `let:item`, a `for PAT in` (or Maud's `@for`), Maud's `@let PAT =`, and an
/// arm's pattern, `PAT = future =>` (`select!`).
pub fn dsl_bound(toks: &[Token], open: usize, close: usize, known: &HashSet<String>) -> HashSet<String> {
    let body: Vec<&Token> = toks[open + 1..close].iter().filter(|t| !t.is_comment()).collect();
    let mut out = HashSet::new();
    let mut k = 0;
    while k < body.len() {
        let t = body[k];
        if t.text == "let" && body.get(k + 1).map_or(false, |c| c.kind == Tk::Colon) {
            if let Some(n) = body.get(k + 2).filter(|n| n.kind == Tk::Ident) {
                out.insert(n.text.clone());
            }
        }
        if t.text == "for" || (t.text == "let" && k > 0 && body[k - 1].text == "@") {
            let mut j = k + 1;
            let mut pat = Vec::new();
            while j < body.len() && !matches!(body[j].text.as_str(), "in" | "=" | "{" | ";") {
                pat.push(body[j]);
                j += 1;
            }
            out.extend(pattern_names(&pat, known));
        }
        // `PAT = fut =>`: a top-level `=` with a `=>` after it before the arm ends.
        if t.kind == Tk::Eq {
            let ends_arm = body[k + 1..].iter().take_while(|u| u.kind != Tk::Comma).any(|u| u.kind == Tk::FatArrow);
            if ends_arm {
                let mut j = k;
                let mut pat = Vec::new();
                let mut d = 0i32;
                while j > 0 {
                    j -= 1;
                    let u = body[j];
                    match u.kind {
                        Tk::Close(_) => d += 1,
                        Tk::Open(_) => {
                            if d == 0 {
                                break;
                            }
                            d -= 1;
                        }
                        Tk::Comma if d == 0 => break,
                        _ => {}
                    }
                    if d == 0 && u.text == "}" {
                        break;
                    }
                    pat.insert(0, u);
                }
                out.extend(pattern_names(&pat, known));
            }
        }
        k += 1;
    }
    out
}

/// What a candidate is.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// A single word, a literal: the same in both languages, no hole.
    Atom,
    /// Calls on a binding or uses an operator: a hole.
    Hole,
    /// Names nothing the table holds and uses no operator: left as written
    /// (a glob's names, or the DSL's own words).
    Unbound,
}

/// Decide a candidate stretch of Rust, `frag`, against the names bound
/// outside the DSL's language at its macro call.
pub fn decide(frag: &str, bound: &HashSet<String>) -> Verdict {
    let Ok(toks) = crate::lex::lex_rust(frag) else { return Verdict::Unbound };
    let sig: Vec<&Token> = toks.iter().filter(|t| !t.is_comment()).collect();
    if sig.len() == 1 && matches!(sig[0].kind, Tk::Ident | Tk::Int | Tk::Float | Tk::Str | Tk::Char) {
        return Verdict::Atom;
    }
    // The expression's own bindings: its closures' parameters and its `let`s.
    let mut own: HashSet<String> = HashSet::new();
    let mut bars: Vec<(usize, usize)> = Vec::new();
    let mut k = 0;
    while k < sig.len() {
        if let Some((params, after)) = closure_params(&sig, k) {
            own.extend(pattern_names(&params, bound));
            bars.push((k, after));
            k = after;
            continue;
        }
        if sig[k].text == "let" {
            let mut j = k + 1;
            let mut pat = Vec::new();
            while j < sig.len() && !matches!(sig[j].text.as_str(), "=" | ":" | ";") {
                pat.push(sig[j]);
                j += 1;
            }
            own.extend(pattern_names(&pat, bound));
        }
        k += 1;
    }
    // A nested macro's brace body, `view! { <p>"odd"</p> }`, is that DSL's
    // own text: its tokens neither bind nor operate here.
    let mut k = 0;
    while k + 2 < sig.len() {
        if sig[k].kind == Tk::Ident && sig[k + 1].text == "!" && sig[k + 2].kind == Tk::Open('{') {
            let mut d = 0i32;
            let mut j = k + 2;
            while j < sig.len() {
                match sig[j].kind {
                    Tk::Open(_) => d += 1,
                    Tk::Close(_) => {
                        d -= 1;
                        if d == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            bars.push((k + 2, j + 1));
            k = j;
        }
        k += 1;
    }
    let in_bars = |k: usize| bars.iter().any(|&(a, b)| k >= a && k < b);
    // A turbofish's `<…>` holds types, not comparisons.
    let mut generic = 0i32;
    for (k, t) in sig.iter().enumerate() {
        if in_bars(k) {
            continue;
        }
        if t.kind == Tk::Lt && k > 0 && sig[k - 1].text == "::" {
            generic += 1;
            continue;
        }
        if generic > 0 {
            match t.text.as_str() {
                "<" => generic += 1,
                ">" => generic -= 1,
                ">>" => generic -= 2,
                _ => {}
            }
            continue;
        }
        // `||` opening a closure of no parameters is no operator.
        if t.text == "||" && (k == 0 || matches!(sig[k - 1].kind, Tk::Open(_) | Tk::Comma | Tk::Eq | Tk::FatArrow) || sig[k - 1].text == "move") {
            continue;
        }
        if operator(t) {
            // `!` after a name is a macro's bang, not a negation.
            let bang = t.text == "!" && k > 0 && sig[k - 1].kind == Tk::Ident && sig[k - 1].span.hi == t.span.lo;
            if !bang {
                return Verdict::Hole;
            }
        }
        if t.kind == Tk::Ident && !keyword(&t.text) {
            // A method or a field after `.`, a path's later segment: resolved
            // by what precedes them.
            let after_dot = k > 0 && matches!(sig[k - 1].text.as_str(), "." | "::");
            if !after_dot && (bound.contains(&t.text) || own.contains(&t.text)) {
                return Verdict::Hole;
            }
        }
    }
    Verdict::Unbound
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Rust -> Harsh.
//!
//! Brace matching is free here: the lexer already tracks bracket depth, so
//! every `{` has a known partner without any parsing. The whole problem
//! reduces to one classification asked once per brace -- does this brace
//! become indentation, or does it stay a brace?
//!
//! Two decisions keep the translation safe rather than clever:
//!
//!   * Only braces at paren/bracket depth zero are considered. A `{` inside a
//!     call is a closure body or a struct literal in argument position, and
//!     Harsh suppresses layout inside brackets anyway, so it must stay a brace.
//!   * When classification is not clear-cut, the braces are left alone. Braces
//!     are legal everywhere in Harsh, so the fallback is always valid output --
//!     just less idiomatic.

use crate::lex::{Tk, Token};
use crate::rules::BLOCK_KEYWORDS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Brace {
    /// `{` becomes `:` plus indentation.
    Block(Kind),
    /// `use a::{b, c}` becomes `use a.(b, c)`.
    UseGroup,
    /// Struct literal, closure body, macro body: left verbatim.
    Verbatim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Separated by `;` in the source, one statement per line in the output.
    Stmts,
    /// Separated by `,`: struct fields, enum variants, match arms.
    Commas,
    /// impl/trait/mod bodies: items separated by `;` or by nested braces.
    Items,
}

pub fn convert(src: &str) -> Result<String, String> {
    // A file `hrs` wrote starts with a header naming its Harsh source
    // (0.3.1); it says where the Rust came from, and is no part of the
    // program -- dropped, so a round trip does not stack them.
    let src = match src.split_once('\n') {
        Some((first, rest)) if first.starts_with("// Generated from ") && first.contains(" by hrs;") => rest,
        _ => src,
    };
    // A Rust `macro_rules!` is Rust on both sides of the boundary (2026-09-17,
    // two worlds): it is copied into the Harsh file verbatim rather than
    // converted, so the round trip is byte-exact for free. Harsh's own macros
    // are `macro_rules~`, which this converter never writes.
    let zones = crate::rawzone::zones(&crate::lex::lex_rust(src).map_err(|e| e.msg.clone())?);
    if !zones.is_empty() {
        let work = crate::rawzone::blank_out(src, &zones);
        let mut out = convert_inner(&work)?;
        crate::rawzone::restore(&mut out, src, &zones);
        return Ok(out);
    }
    convert_inner(src)
}

fn convert_inner(src: &str) -> Result<String, String> {
    let toks = crate::lex::lex_rust(src).map_err(|e| e.msg)?;
    // Rust's empty generic list -- `f::<>(x)`, `P<>`, `impl<>` -- means
    // nothing, and in Harsh `f<>` is *apply to each*. It is dropped here, from
    // the text, before anything is converted (2026-09-21).
    let empty: Vec<(usize, usize)> = (0..toks.len().saturating_sub(1))
        .filter(|&k| toks[k].kind == Tk::Lt && toks[k + 1].kind == Tk::Gt)
        .map(|k| {
            let from = if k > 0 && toks[k - 1].kind == Tk::PathSep { k - 1 } else { k };
            (toks[from].span.lo as usize, toks[k + 1].span.hi as usize)
        })
        .collect();
    if !empty.is_empty() {
        let mut cleaned = String::with_capacity(src.len());
        let mut at = 0;
        for (lo, hi) in empty {
            cleaned.push_str(&src[at..lo]);
            at = hi;
        }
        cleaned.push_str(&src[at..]);
        return convert_inner(&cleaned);
    }
    let classes = classify_all(&toks);
    let pc = param_commas(&toks);
    let ep = empty_params(&toks);
    let mut w = Writer { out: String::new(), level: 0, at_line_start: true, param_commas: pc, empty_params: ep, src, brace_depth: 0, in_enum_body: false, errors: Vec::new() };
    w.emit(&toks, &classes, 0, toks.len(), Kind::Items);
    // A piece of Rust in a DSL body that could not be written as Harsh stops
    // the conversion (the user's ruling, 2026-09-23: no silent failure).
    if !w.errors.is_empty() {
        return Err(w.errors.join("\n"));
    }
    let mut s = w.out;
    while s.ends_with('\n') {
        s.pop();
    }
    s.push('\n');
    Ok(s)
}

/// Index of each brace's partner, plus how each `{` is classified.
struct Classes {
    partner: Vec<usize>,
    brace: Vec<Brace>,
}


fn classify_all(toks: &[Token]) -> Classes {
    let n = toks.len();
    let mut partner = vec![usize::MAX; n];
    let mut brace = vec![Brace::Verbatim; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut depth_at = vec![0usize; n];
    // Depth of `(` and `[` only. A `{` nested inside those can never become
    // indentation, because Harsh suppresses layout there.
    let mut paren_depth = 0usize;

    for i in 0..n {
        match toks[i].kind {
            Tk::Open('(') | Tk::Open('[') => paren_depth += 1,
            Tk::Close(')') | Tk::Close(']') => paren_depth = paren_depth.saturating_sub(1),
            Tk::Open('{') => {
                stack.push(i);
                depth_at[i] = paren_depth;
            }
            Tk::Close('}') => {
                if let Some(o) = stack.pop() {
                    partner[o] = i;
                    partner[i] = o;
                }
            }
            _ => {}
        }
    }
    for i in 0..n {
        if toks[i].kind != Tk::Open('{') {
            continue;
        }
        let close = partner[i];
        // An empty block has no body to indent, so it keeps its braces. Harsh
        // rejects a block opener with nothing under it.
        let empty = close == usize::MAX
            || toks[i + 1..close.min(n)].iter().all(|t| t.is_comment());
        // An empty *function* body is written `()` -- the unit, with any
        // comment inside as lines above it -- so it needs no braces; an
        // empty struct, impl or match keeps `{ }` on one line.
        let empty_fn_body = empty && close != usize::MAX && {
            let mut k = i;
            let mut d = 0i32;
            let mut is_fn = false;
            while k > 0 {
                k -= 1;
                match toks[k].kind {
                    Tk::Close(_) => d += 1,
                    Tk::Open(_) => {
                        if d == 0 {
                            break;
                        }
                        d -= 1;
                    }
                    Tk::Semi if d == 0 => break,
                    Tk::Ident if d == 0 && toks[k].is_kw("fn") => {
                        is_fn = true;
                        break;
                    }
                    Tk::Ident if d == 0 && matches!(toks[k].text.as_str(), "struct" | "enum" | "impl" | "trait" | "mod" | "match") => break,
                    _ => {}
                }
            }
            is_fn
        };
        let empty = empty && !empty_fn_body;
        // Parens are transparent to layout, so a closure body inside an
        // argument group is a block like any other: `(|item|:` with the
        // body beneath. Other braces inside brackets stay Rust's.
        let closure_body = depth_at[i] > 0 && paren_depth_only(toks, i, &brace) && i > 0 && closure_before_brace(toks, i);
        // A record variant inside an enum's body: `Record { .. }` is a
        // field list, not a literal -- its enclosing brace is an enum's.
        let record_variant = i > 0
            && toks[i - 1].kind == Tk::Ident
            && enclosing_open(toks, i).map_or(false, |o| {
                matches!(brace[o], Brace::Block(Kind::Commas)) && {
                    // the enclosing header names an enum: walk back from
                    // its `{` to the previous `;`, `{` or `}`
                    let mut k = o;
                    let mut d = 0i32;
                    while k > 0 {
                        k -= 1;
                        match toks[k].kind {
                            Tk::Close(_) => d += 1,
                            Tk::Open(_) => {
                                if d == 0 {
                                    k += 1;
                                    break;
                                }
                                d -= 1;
                            }
                            Tk::Semi if d == 0 => {
                                k += 1;
                                break;
                            }
                            _ => {}
                        }
                    }
                    toks[k..o].iter().any(|t| t.is_kw("enum"))
                }
            });
        // Parens are transparent to layout: inside parens only, a brace is
        // classified as at depth zero -- a `match`, an `if`, a closure body
        // all become blocks. Inside `[ .. ]` the layout is off.
        let transparent = depth_at[i] == 0 || paren_depth_only(toks, i, &brace);
        brace[i] = if record_variant {
            Brace::Block(Kind::Commas)
        } else if !transparent || empty {
            Brace::Verbatim
        } else if closure_body {
            Brace::Block(Kind::Stmts)
        } else {
            classify_brace(toks, i, &brace)
        };
    }
    // Layout is suppressed inside braces, so a block nested in a verbatim
    // brace must stay verbatim too -- otherwise the converter emits an
    // indented block that the forward direction cannot reproduce.
    for i in 0..n {
        if toks[i].kind != Tk::Open('{') || brace[i] != Brace::Verbatim {
            continue;
        }
        let close = partner[i];
        if close == usize::MAX {
            continue;
        }
        // A macro's brace body (`view! { .. }`, `rsx! { .. }`) is Harsh on
        // the way out -- its holes are laid out -- so what is nested inside
        // it is classified on its own terms, not forced verbatim.
        let macro_body = i > 0 && toks[i - 1].kind == Tk::Punct && toks[i - 1].text == "!";
        let hole = enclosing_open(toks, i).map_or(false, |o| o > 0 && toks[o - 1].kind == Tk::Punct && toks[o - 1].text == "!");
        let transcriber = i > 0 && toks[i - 1].kind == Tk::FatArrow;
        if macro_body || hole || transcriber {
            continue;
        }
        for j in (i + 1)..close {
            if toks[j].kind == Tk::Open('{') {
                brace[j] = Brace::Verbatim;
            }
        }
    }
    Classes { partner, brace }
}

/// Does a closure prototype end right before the `{` at `i` -- `|x| {`, or
/// with a return type, `|x| -> T {`?
fn closure_before_brace(toks: &[Token], i: usize) -> bool {
    // `||` is a prototype only where an expression can start; after an
    // expression (`a || { .. }`) it is the boolean or.
    let is_proto = |k: usize| -> bool {
        let t = &toks[k];
        if !(t.kind == Tk::Punct && t.text.ends_with('|')) {
            return false;
        }
        if t.text == "||" {
            return k == 0
                || matches!(toks[k - 1].kind, Tk::Open(_) | Tk::Comma | Tk::Eq | Tk::FatArrow | Tk::Semi)
                || toks[k - 1].is_kw("move")
                || toks[k - 1].is_kw("return")
                || toks[k - 1].kind == Tk::Open('{');
        }
        true
    };
    if is_proto(i - 1) {
        return true;
    }
    let mut k = i;
    while k > 0 {
        k -= 1;
        let t = &toks[k];
        if t.kind == Tk::Punct && t.text == "->" {
            return k > 0 && is_proto(k - 1);
        }
        let type_tok = matches!(t.kind, Tk::Ident | Tk::PathSep | Tk::Lt | Tk::Gt | Tk::Open('[') | Tk::Close(']') | Tk::Open('(') | Tk::Close(')') | Tk::Comma | Tk::Lifetime)
            || (t.kind == Tk::Punct && matches!(t.text.as_str(), "&" | "*" | "+" | "'" | ">>"));
        // (`>>` closes two generic lists at once, `Option<Vec<u8>>`: it was
        // missing here, and such a closure's body was isolated as if it were
        // an argument -- `(do: …)`, which then transpiled to a turbofish in the
        // return type. Found 2026-09-23 in the converter's own source.)
        if !type_tok {
            return false;
        }
    }
    false
}

/// The index of the innermost `{` still open at `i`, if any.
fn enclosing_open(toks: &[Token], i: usize) -> Option<usize> {
    let mut stack: Vec<usize> = Vec::new();
    for (k, t) in toks[..i].iter().enumerate() {
        match t.kind {
            Tk::Open('{') => stack.push(k),
            Tk::Close('}') => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.last().copied()
}

/// A binary operator a block can follow as an operand: `a && { .. }`.
/// The line with every string and character literal's content replaced by
/// `_`, so a scan for brackets or words sees only code. A lifetime's `'` has
/// no partner within three characters and is left alone.
fn mask_literals(line: &str) -> String {
    let cs: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c == '"' {
            out.push('"');
            i += 1;
            while i < cs.len() && cs[i] != '"' {
                if cs[i] == '\\' {
                    i += 1;
                }
                out.push('_');
                i += 1;
            }
            out.push('"');
            i += 1;
            continue;
        }
        if c == '\'' {
            let len = if cs.get(i + 1) == Some(&'\\') { 3 } else { 2 };
            if cs.get(i + len) == Some(&'\'') {
                out.push_str("'_'");
                i += len + 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// One piece of Rust code from a DSL body, as Harsh: converted as the body of
/// a function, then transpiled back and compared token for token with the
/// original. Anything else is an error, never a silent Rust fallback.
fn hole_of(frag: &str) -> Result<String, String> {
    // A hole is a whole expression: one that ends expecting more is a
    // misread extent, and an error rather than a hole.
    let toks: Vec<Token> = crate::lex::lex_rust(frag).map_err(|e| e.msg)?.into_iter().filter(|t| !t.is_comment()).collect();
    let dangling = |t: &Token| {
        matches!(t.text.as_str(), "if" | "match" | "while" | "for" | "in" | "else" | "move" | "as" | "return" | "=" | "==" | "!=" | "->" | "=>" | "." | "," | "+" | "-" | "*" | "/" | "%" | "&&" | "||" | "<" | "<=" | ">=" | "&" | "|")
    };
    if toks.last().map_or(true, dangling) {
        return Err("the expression is incomplete where the scan ended".into());
    }
    let wrapped = format!("fn __hrs_hole() {{\n{}\n}}\n", frag.trim());
    let harsh = convert(&wrapped).map_err(|e| e.lines().next().unwrap_or("").to_string())?;
    let mut lines = harsh.lines().skip_while(|l| !l.starts_with("fn __hrs_hole"));
    lines.next();
    let body: Vec<&str> = lines.take_while(|l| l.starts_with(' ') || l.is_empty()).filter(|l| !l.trim().is_empty()).collect();
    let min = body.iter().map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    let code: String = body.iter().map(|l| &l[min..]).collect::<Vec<_>>().join("\n");
    if code.is_empty() {
        return Err("nothing came out".into());
    }
    let back = crate::dslzone::transpile_hole(&code, &crate::driver::transpile_str)?;
    if same_rust(frag, &back) {
        Ok(code)
    } else {
        Err(format!("its Harsh comes back as `{}`", back.split_whitespace().collect::<Vec<_>>().join(" ")))
    }
}

/// Two pieces of Rust are the same program text: the same tokens, comments
/// aside, a macro call's delimiter aside (it means nothing to the macro), and
/// a trailing comma before a closer aside.
fn same_rust(a: &str, b: &str) -> bool {
    // (A nested function: a closure with the return type `Option<Vec<…>>`
    // did not survive `hrs-from` when this was written -- the `>>`, fixed the
    // same day in `closure_before_brace`.)
    fn norm(s: &str) -> Option<Vec<String>> {
        let mut out: Vec<String> = crate::lex::lex_rust(s).ok()?.into_iter().filter(|t| !t.is_comment()).map(|t| t.text).collect();
        let mut stack: Vec<bool> = Vec::new();
        for i in 0..out.len() {
            if matches!(out[i].as_str(), "(" | "[" | "{") {
                let bang = i >= 2 && out[i - 1] == "!" && out[i - 2].chars().next().map_or(false, |c| c.is_alphanumeric() || c == '_');
                stack.push(bang);
                if bang {
                    out[i] = "(".into();
                }
            } else if matches!(out[i].as_str(), ")" | "]" | "}") && stack.pop() == Some(true) {
                out[i] = ")".into();
            }
        }
        let mut v: Vec<String> = Vec::new();
        for t in out {
            if matches!(t.as_str(), ")" | "]" | "}") && v.last().map_or(false, |l| l == ",") {
                v.pop();
            }
            v.push(t);
        }
        Some(v)
    }
    matches!((norm(a), norm(b)), (Some(x), Some(y)) if x == y)
}

fn is_binary_operator(t: &Token) -> bool {
    t.kind == Tk::Punct && matches!(t.text.as_str(), "&&" | "||" | "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<=" | ">=" | "^" | "<<")
}

/// The innermost bracket left open at `i`.
fn innermost_open_char(toks: &[Token], i: usize) -> Option<char> {
    let mut stack: Vec<char> = Vec::new();
    for t in &toks[..i] {
        match t.kind {
            Tk::Open(c) => stack.push(c),
            Tk::Close(_) => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.last().copied()
}

/// Is the innermost bracket left open at `i` a `[`?
/// Is the innermost group still open at `i` a paren holding a top-level
/// comma -- a tuple, `(Point { x: 1 }, msg)`? An inline literal there must be
/// isolated: its field list would otherwise read to the group's `)` and take
/// the tuple's other elements as shorthand fields, which is a valid literal
/// and so a silent wrong program rather than an error.
fn innermost_open_is_tuple(toks: &[Token], i: usize) -> bool {
    let mut stack: Vec<(char, usize)> = Vec::new();
    for (k, t) in toks[..i].iter().enumerate() {
        match t.kind {
            Tk::Open(c) => stack.push((c, k)),
            Tk::Close(_) => {
                stack.pop();
            }
            _ => {}
        }
    }
    let Some(&('(', open)) = stack.last() else { return false };
    // Not a call's argument list: there each argument is isolated already,
    // and a second pair of parens would only be noise.
    // (A keyword before the paren -- `let (..)`, `in (..)`, `return (..)` --
    // is not a call: the paren is a tuple's, 2026-10-03.)
    if open > 0
        && matches!(
            toks[open - 1].kind,
            Tk::Ident | Tk::Close(')') | Tk::Close(']') | Tk::Gt
        )
        && !(toks[open - 1].kind == Tk::Ident && crate::rules::is_keyword(&toks[open - 1].text))
    {
        return false;
    }
    let mut d = 0i32;
    for t in &toks[open + 1..] {
        match t.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) if d == 0 => return false,
            Tk::Close(_) => d -= 1,
            Tk::Comma if d == 0 => return true,
            _ => {}
        }
    }
    false
}

fn innermost_open_is_bracket(toks: &[Token], i: usize) -> bool {
    let mut stack: Vec<char> = Vec::new();
    for t in &toks[..i] {
        match t.kind {
            Tk::Open(c) => stack.push(c),
            Tk::Close(_) => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.last() == Some(&'[')
}

/// Are all the brackets left open at `i` parens? Inside `[ .. ]` or `{ .. }`
/// the layout is off, so a closure body there keeps its braces.
fn paren_depth_only(toks: &[Token], i: usize, brace: &[Brace]) -> bool {
    let mut stack: Vec<(char, usize)> = Vec::new();
    for (k, t) in toks[..i].iter().enumerate() {
        match t.kind {
            Tk::Open(c) => stack.push((c, k)),
            Tk::Close(_) => {
                stack.pop();
            }
            _ => {}
        }
    }
    // A brace that is itself a layout block is transparent, and so is a
    // hole in a macro's markup (`{ .. }` right inside `view! { .. }`); only
    // a `[` or a Rust-kept `{` suspends the layout.
    stack.iter().enumerate().all(|(n, &(c, k))| {
        c == '('
            || (c == '{' && matches!(brace[k], Brace::Block(_)))
            || (c == '{' && k > 0 && toks[k - 1].kind == Tk::Punct && toks[k - 1].text == "!")
            || (c == '{' && n > 0 && {
                let (pc, pk) = stack[n - 1];
                pc == '{' && pk > 0 && toks[pk - 1].kind == Tk::Punct && toks[pk - 1].text == "!"
            })
    })
}

/// Whether `toks` holds nothing but comments and whole attributes, `#[…]`
/// or `#![…]` -- so a statement has not begun yet.
fn only_attributes(toks: &[Token]) -> bool {
    let mut k = 0;
    while k < toks.len() {
        if toks[k].is_comment() {
            k += 1;
            continue;
        }
        if toks[k].kind != Tk::Hash {
            return false;
        }
        let mut open = k + 1;
        if toks.get(open).map_or(false, |t| t.text == "!") {
            open += 1;
        }
        if toks.get(open).map_or(true, |t| t.kind != Tk::Open('[')) {
            return false;
        }
        let mut d = 0i32;
        let mut end = None;
        for (j, t) in toks.iter().enumerate().skip(open) {
            match t.kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => {
                    d -= 1;
                    if d == 0 {
                        end = Some(j);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(e) = end else { return false };
        k = e + 1;
    }
    true
}

/// Walk back from a `{` to the start of its segment -- the run of tokens since
/// the last `;`, `{` or `}` at the same depth -- and decide what the brace is.
fn classify_brace(toks: &[Token], open: usize, brace: &[Brace]) -> Brace {
    // A brace right after `(` or a `,` inside parens is an argument. Data
    // -- the object literal of `json!({ .. })`, `"key": value` pairs -- is
    // Rust's, kept; a statement block (a `let`, a `;` at depth one) is a
    // block, written `(do:` .. `)`.
    if open > 0 && matches!(toks[open - 1].kind, Tk::Open('(') | Tk::Comma) {
        let mut d = 0i32;
        let mut k = open + 1;
        let mut is_block = false;
        while k < toks.len() {
            match toks[k].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => {
                    if d == 0 {
                        break;
                    }
                    d -= 1;
                }
                Tk::Semi if d == 0 => is_block = true,
                Tk::Ident if d == 0 && k == open + 1 && matches!(toks[k].text.as_str(), "let" | "if" | "match" | "for" | "while" | "loop" | "return" | "use") => is_block = true,
                _ => {}
            }
            k += 1;
        }
        return if is_block { Brace::Block(Kind::Stmts) } else { Brace::Verbatim };
    }
    // `use a::{b, c}`: the only construct where `::` immediately precedes `{`.
    if open > 0 && toks[open - 1].kind == Tk::PathSep {
        return Brace::UseGroup;
    }

    let mut start = open;
    let mut depth = 0i32;
    while start > 0 {
        let t = &toks[start - 1];
        match t.kind {
            // A `}` at depth zero closes the previous item: the segment ends
            // -- unless that brace was a literal or a pattern (verbatim),
            // which is part of this very line: `if let E::G { .. } = &m {`.
            // A `}` that ends its line ends the item before, verbatim or not:
            // an empty `struct Empty {}` keeps its braces (verbatim), and the
            // `fn` on the next line is not part of its line (found 2026-09-27:
            // `struct Empty { } fn g$`). A pattern's `}` inside a condition,
            // `if let E::G { .. } = &m {`, is followed on its own line.
            Tk::Close('}') if depth == 0 && start < open && toks[start].line > t.line => break,
            Tk::Close('}') if depth == 0 => {
                let is_verbatim = {
                    // find the partner `{` by walking back
                    let mut k = start - 1;
                    let mut dd = 0i32;
                    let mut found = None;
                    while k > 0 {
                        k -= 1;
                        match toks[k].kind {
                            Tk::Close('}') => dd += 1,
                            Tk::Open('{') => {
                                if dd == 0 {
                                    found = Some(k);
                                    break;
                                }
                                dd -= 1;
                            }
                            _ => {}
                        }
                    }
                    found.map_or(false, |o| matches!(brace.get(o), Some(Brace::Verbatim)))
                };
                if !is_verbatim {
                    break;
                }
                depth += 1;
            }
            Tk::Close(_) => depth += 1,
            Tk::Open(_) if depth == 0 => break,
            Tk::Open(_) => depth -= 1,
            Tk::Semi if depth == 0 => break,
            // A match arm's `=>` begins the arm's value: a brace inside it
            // belongs to what follows the arrow, not to the arms before --
            // whose guard's `if` made `Some(_) => match g(h) {` read as an
            // `if` block (found 2026-09-24). An arm whose value is the
            // brace itself, `=> {`, keeps the walk.
            Tk::FatArrow if depth == 0 && start < open => break,
            _ => {}
        }
        start -= 1;
    }
    let seg = &toks[start..open];

    // `foo! { .. }` and `macro_rules! name { .. }`: token soup, leave alone.
    // The brace is a macro body only when the bang is *immediately* before it:
    // `m! { .. }`, or `macro_rules! name { .. }`. Scanning the whole segment
    // made `if a && matches!(..) {` look like a macro body because a macro
    // appeared somewhere earlier in the condition.
    let tail: Vec<&Token> = seg.iter().rev().filter(|t| !t.is_comment()).take(3).collect();
    let is_macro = match tail.as_slice() {
        // `m! {`
        [bang, name, ..] if bang.text == "!" && name.kind == Tk::Ident => true,
        // `macro_rules! name {` -- the keyword must be named, or `if !flag {`
        // matches this shape reversed.
        [name, bang, kw]
            if name.kind == Tk::Ident && bang.text == "!" && kw.text == "macro_rules" =>
        {
            true
        }
        _ => false,
    };
    if is_macro {
        return Brace::Verbatim;
    }

    // A block keyword anywhere in the segment, at segment depth zero.
    let mut d = 0i32;
    let mut found: Option<&str> = None;
    for t in seg {
        match t.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Ident if d == 0 => {
                if BLOCK_KEYWORDS.contains(&t.text.as_str()) && found.is_none() {
                    found = Some(t.text.as_str());
                }
            }
            _ => {}
        }
    }
    if let Some(kw) = found {
        // `if let E::G { body, .. } = &m {`: the first brace is the pattern's,
        // not the block's -- a `let` at segment depth zero with no `=` after
        // it means the pattern is still open. Verbatim, so it becomes the
        // `\` form like any struct literal (2026-09-19).
        if matches!(kw, "if" | "while" | "let") {
            let mut d = 0i32;
            let mut after_let = false;
            let mut pattern_open = false;
            for t in seg {
                match t.kind {
                    Tk::Open(_) => d += 1,
                    Tk::Close(_) => d -= 1,
                    Tk::Ident if d == 0 && t.is_kw("let") => {
                        after_let = true;
                        pattern_open = true;
                    }
                    Tk::Eq if d == 0 && after_let => pattern_open = false,
                    _ => {}
                }
            }
            if pattern_open {
                return Brace::Verbatim;
            }
        }
        return Brace::Block(match kw {
            "struct" | "enum" | "union" | "match" => Kind::Commas,
            "impl" | "trait" | "mod" => Kind::Items,
            _ => Kind::Stmts,
        });
    }
    // `extern "C" { .. }`: a foreign block, a block of items.
    if seg.iter().filter(|t| !t.is_comment()).any(|t| t.is_kw("extern"))
        && seg.iter().rev().find(|t| !t.is_comment()).map_or(false, |t| t.kind == Tk::Str)
    {
        return Brace::Block(Kind::Items);
    }

    // `pat => { .. }` is a match arm body: the fat arrow is already the opener.
    if seg.iter().rev().find(|t| !t.is_comment()).map(|t| t.kind == Tk::FatArrow).unwrap_or(false) {
        return Brace::Block(Kind::Stmts);
    }

    // Empty segment: a bare block in statement position -- or one carrying
    // only an attribute, `#[cfg(feature = "hydrate")] { .. }`.
    let only_attrs = {
        let mut d = 0i32;
        seg.iter().all(|t| {
            if t.is_comment() {
                return true;
            }
            match t.kind {
                Tk::Hash => true,
                Tk::Open('[') => {
                    d += 1;
                    true
                }
                Tk::Close(']') => {
                    d -= 1;
                    true
                }
                _ => d > 0,
            }
        })
    };
    if only_attrs {
        return Brace::Block(Kind::Stmts);
    }
    // A closure's body, `|x| { .. }` / `|x| -> T { .. }`: a block -- `|x|:`.
    if open > 0 && closure_before_brace(toks, open) {
        return Brace::Block(Kind::Stmts);
    }
    // A block as a value, `let x = { .. }`: a `do:` block. As an operand,
    // `a && { .. }`: a `(do:` block, isolated.
    if seg.iter().rev().find(|t| !t.is_comment()).map(|t| t.kind == Tk::Eq || is_binary_operator(t)).unwrap_or(false) {
        return Brace::Block(Kind::Stmts);
    }

    // Anything else -- struct literal, closure body, unrecognised construct --
    // keeps its braces. Always valid Harsh.
    Brace::Verbatim
}

struct Writer<'a> {
    out: String,
    /// Pieces of Rust in a DSL body that could not be written as Harsh.
    errors: Vec<String>,
    level: usize,
    at_line_start: bool,
    /// Commas that separate parameters of a `fn` declaration, each mapped to
    /// the `(` of its list. Harsh writes one group per parameter, so each of
    /// these becomes `) (`; a trailing one vanishes.
    param_commas: std::collections::HashMap<usize, usize>,
    /// The `(` of an empty parameter list, `fn f()`: Harsh writes `fn f$`.
    empty_params: std::collections::HashSet<usize>,
    /// The source, for the whitespace a macro body keeps.
    src: &'a str,
    /// How many Rust braces the writer is inside: no layout block can be
    /// opened there, so a `view!` keeps its brace form.
    brace_depth: usize,
    /// Inside an enum's body: a variant `V(A, B)` is written `V A B`, a
    /// record variant's name opens its fields with no mark.
    in_enum_body: bool,
}

/// Find every parameter-separating comma in every `fn` declaration.
fn param_commas(toks: &[Token]) -> std::collections::HashMap<usize, usize> {
    let mut out = std::collections::HashMap::new();
    for (i, t) in toks.iter().enumerate() {
        if !(t.kind == Tk::Ident && t.text == "fn") {
            continue;
        }
        // Reuse the shared finder on the slice from this `fn`; it yields the
        // groups of the first declaration it sees, so index from `i`.
        if let Some(groups) = crate::rules::fn_param_groups(&toks[i..]) {
            for (o, c) in groups {
                for k in crate::rules::param_list_commas(&toks[i..], o, c) {
                    out.insert(i + k, i + o);
                }
            }
        }
    }
    out
}

fn empty_params(toks: &[Token]) -> std::collections::HashSet<usize> {
    let mut out = std::collections::HashSet::new();
    for (i, t) in toks.iter().enumerate() {
        if !(t.kind == Tk::Ident && t.text == "fn") {
            continue;
        }
        if let Some(groups) = crate::rules::fn_param_groups(&toks[i..]) {
            // Rust has one list, so this is the only group there is.
            if let Some(&(o, c)) = groups.first() {
                if c == o + 1 {
                    out.insert(i + o);
                }
            }
        }
    }
    out
}

impl<'a> Writer<'a> {
    /// The indentation of the output line being written.
    fn current_line_indent(&self) -> usize {
        let line = self.out.rsplit('\n').next().unwrap_or("");
        line.len() - line.trim_start().len()
    }

    fn newline(&mut self) {
        if !self.at_line_start {
            while self.out.ends_with(' ') {
                self.out.pop();
            }
            self.out.push('\n');
            self.at_line_start = true;
        }
    }

    fn indent(&mut self) {
        if self.at_line_start {
            for _ in 0..self.level {
                self.out.push_str("    ");
            }
            self.at_line_start = false;
        }
    }

    fn word(&mut self, s: &str, space_before: bool) {
        self.indent();
        if space_before && !self.out.ends_with(' ') && !self.out.ends_with('\n') {
            self.out.push(' ');
        }
        self.out.push_str(s);
    }

    /// Emit tokens in `[from, to)` as the body of a block of kind `kind`.
    fn emit(&mut self, toks: &[Token], cl: &Classes, from: usize, to: usize, kind: Kind) {
        // Split the range into items at top-level `;`, `,` or nested-block `}`.
        let mut i = from;
        let mut item_start = from;
        let mut items: Vec<(usize, usize, bool)> = Vec::new(); // start, end, had_semi
        let mut depth = 0i32;
        while i < to {
            match toks[i].kind {
                Tk::Open(_) => depth += 1,
                Tk::Close(_) => depth -= 1,
                _ => {}
            }
            // A block's `}` ends the item -- unless the statement goes on
            // from it (`if .. { } else { }.into_iter()`, `match .. { }?`),
            // or an `else` follows.
            let continues = matches!(toks.get(i + 1).map(|t| &t.kind), Some(Tk::Dot) | Some(Tk::LArrow) | Some(Tk::Semi))
                || toks.get(i + 1).map_or(false, |t| t.text == "?" || t.is_kw("else") || t.is_kw("as") || is_binary_operator(t));
            let is_block_end = depth == 0
                && toks[i].kind == Tk::Close('}')
                && cl.partner[i] != usize::MAX
                && (matches!(cl.brace[cl.partner[i]], Brace::Block(_)) || is_macro_rules_open(toks, cl.partner[i]))
                && !continues;
            let is_sep = depth == 0
                && ((toks[i].kind == Tk::Semi)
                    || (kind == Kind::Commas && toks[i].kind == Tk::Comma));
            // A statement repetition `$( .. )*` ends its item at its op:
            // Harsh writes it alone on its line (rule 2).
            let rep_end = depth == 0
                && kind == Kind::Stmts
                && toks[i].kind == Tk::Close(')')
                && i + 1 < to
                && toks[i + 1].kind == Tk::Punct
                && matches!(toks[i + 1].text.as_str(), "*" | "+" | "?")
                && {
                    // back to the matching `(`: is it a `$(`?
                    let mut d = 0i32;
                    let mut o = None;
                    for k in (from..=i).rev() {
                        match toks[k].kind {
                            Tk::Close(_) => d += 1,
                            Tk::Open(_) => {
                                d -= 1;
                                if d == 0 {
                                    o = Some(k);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    o.map_or(false, |o| o > 0 && toks[o - 1].kind == Tk::Punct && toks[o - 1].text == "$")
                }
                && !(i + 2 < to && matches!(toks[i + 2].kind, Tk::Semi | Tk::Comma));
            if rep_end {
                items.push((item_start, i + 2, false));
                item_start = i + 2;
                i += 2;
                continue;
            }
            if is_sep {
                items.push((item_start, i, toks[i].kind == Tk::Semi));
                item_start = i + 1;
            } else if is_block_end {
                // A nested block ends the item unless a `;` or `,` follows.
                let nxt = i + 1;
                let follows_sep = nxt < to
                    && matches!(toks[nxt].kind, Tk::Semi | Tk::Comma);
                if !follows_sep {
                    items.push((item_start, i + 1, false));
                    item_start = i + 1;
                }
            }
            i += 1;
        }
        if item_start < to {
            items.push((item_start, to, false));
        }
        items.retain(|(a, b, _)| toks[*a..*b].iter().any(|t| !t.is_comment()) || a != b);

        // A comment-only item cannot be a block's tail expression, so it must
        // not displace the statement before it. The forward direction applies
        // the same rule when locating a block's tail.
        let last = items
            .iter()
            .rposition(|(a, b, _)| toks[*a..*b].iter().any(|t| !t.is_comment()))
            .unwrap_or(0);
        let mut prev_end_line: Option<usize> = None;
        for (n, (a, b, had_semi)) in items.iter().enumerate() {
            if toks[*a..*b].is_empty() {
                continue;
            }
            self.newline();
            // An author's blank line between two items is kept (as one).
            if let Some(pl) = prev_end_line {
                if toks[*a].line > pl + 1 && !self.out.ends_with("\n\n") && !self.out.is_empty() {
                    self.out.push('\n');
                }
            }
            prev_end_line = toks[..*b].last().map(|t| t.line);
            self.run(toks, cl, *a, *b, kind);
            // Semicolons are dropped except on the final statement of a block,
            // where dropping one would turn a discarded value into a tail
            // expression. Harsh re-inserts every intermediate `;` from layout.
            //
            // An item that ends with an indented block is excluded: the forward
            // direction re-derives that `;` from the `=` in the header, and
            // appending one here would land it on its own line.
            let ended_with_block = self.at_line_start;
            // Harsh writes no `;` (the user's rule, 2026-09-30): a block whose
            // last statement discards its value ends with a line `()` --
            // except after a `let` (Harsh terminates it) or a unit macro,
            // `println!` and the like, whose `;` the forward direction
            // restores.
            let lead = toks[*a..*b].iter().find(|t| !t.is_comment());
            let terminated = lead.map_or(false, |t| crate::rules::ALWAYS_SEMI.contains(&t.text.as_str()))
                || crate::rules::unit_macro_call(&toks[*a..*b]);
            if *had_semi && n == last && kind == Kind::Stmts && !ended_with_block && !terminated {
                self.newline();
                self.word("()", false);
            }
            self.newline();
        }
    }

    /// Is `toks[i]` the start of an atom, and where does it end?
    ///
    /// Mirrors the forward direction: an identifier (with optional `::` path,
    /// generics and macro bang), a literal, or a bracketed group.
    fn atom_end(toks: &[Token], i: usize, to: usize) -> Option<usize> {
        if i >= to {
            return None;
        }
        let mut k = i;
        match toks[i].kind {
            Tk::Open(_) => {
                let mut d = 0i32;
                while k < to {
                    match toks[k].kind {
                        Tk::Open(_) => d += 1,
                        Tk::Close(_) => {
                            d -= 1;
                            if d == 0 {
                                return Some(k + 1);
                            }
                        }
                        _ => {}
                    }
                    k += 1;
                }
                None
            }
            Tk::Ident => {
                if crate::juxt::NOT_CALLABLE.contains(&toks[i].text.as_str()) {
                    return None;
                }
                k += 1;
                while k + 1 < to && toks[k].kind == Tk::PathSep && toks[k + 1].kind == Tk::Ident {
                    k += 2;
                }
                if k < to && toks[k].kind == Tk::Lt {
                    let mut d = 0i32;
                    let mut j = k;
                    while j < to {
                        match toks[j].kind {
                            Tk::Lt => d += 1,
                            Tk::Gt => {
                                d -= 1;
                                if d == 0 {
                                    k = j + 1;
                                    break;
                                }
                            }
                            // `>>` closes two lists at once: `Vec<Vec<_>>`.
                            Tk::Punct if toks[j].text == ">>" => {
                                d -= 2;
                                if d <= 0 {
                                    k = j + 1;
                                    break;
                                }
                            }
                            Tk::Semi | Tk::Open('{') => break,
                            _ => {}
                        }
                        j += 1;
                    }
                }
                if k < to && toks[k].text == "!" {
                    k += 1;
                }
                Some(k)
            }
            Tk::Int | Tk::Float | Tk::Str | Tk::Char => Some(i + 1),
            _ => None,
        }
    }

    /// A Rust argument list `( a, b )` becomes juxtaposed atoms. Each argument
    /// that is not already an atom is isolated in parentheses, recursively.
    fn args_of(toks: &[Token], open: usize, close: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut d = 0i32;
        let mut start = open + 1;
        // A closure prototype's commas, `|a, b|`, are not argument commas.
        let mut in_proto = false;
        for i in (open + 1)..close {
            match toks[i].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                Tk::Punct if d == 0 && toks[i].text == "|" => in_proto = !in_proto,
                Tk::Comma if d == 0 && !in_proto => {
                    out.push((start, i));
                    start = i + 1;
                }
                _ => {}
            }
        }
        if start < close {
            out.push((start, close));
        }
        out
    }

    /// Emit one item, recursing into nested blocks.
    /// Statements inside a brace-delimited block each get their own expression
    /// region, rather than inheriting the enclosing item's.
    /// The tokens strictly between a macro's `{` at `open` and its `}` at
    /// `close`, with the source's own whitespace between them. Newlines keep
    /// their indentation relative to the `{`'s line, re-based on the current
    /// level. Substitutions: `::` → `.`, a member `.` → ` <- `, an empty call
    /// `()` → `$`.
    /// Tokens `a..b` with the macro-body substitutions and single spaces,
    /// for an argument inside a macro's braces.
    fn macro_body_span(&mut self, toks: &[Token], a: usize, b: usize) {
        let mut prev_hi: Option<u32> = None;
        let mut k = a;
        while k < b {
            let t = &toks[k];
            if let Some(ph) = prev_hi {
                if t.span.lo > ph && !matches!(t.kind, Tk::Comma | Tk::Close(_) | Tk::Semi) {
                    self.out.push(' ');
                }
            }
            let member = matches!(t.kind, Tk::Dot | Tk::LArrow) && k + 1 < b && toks[k + 1].kind == Tk::Ident;
            let empty_call = t.kind == Tk::Open('(') && k + 1 < b && toks[k + 1].kind == Tk::Close(')') && k > a && toks[k - 1].span.hi == t.span.lo;
            match t.kind {
                Tk::PathSep => self.out.push('.'),
                _ if member => {
                    while self.out.ends_with(' ') {
                        self.out.pop();
                    }
                    self.out.push_str(" <- ");
                }
                _ if empty_call => {
                    self.out.push('$');
                    k += 1;
                }
                _ => self.out.push_str(&t.text),
            }
            prev_hi = Some(toks[k].span.hi);
            k += 1;
        }
        self.at_line_start = false;
    }

    fn macro_body(&mut self, toks: &[Token], open: usize, close: usize) {
        let src = self.src;
        let base_line_start = src[..toks[open].span.lo as usize].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let base_indent = src[base_line_start..toks[open].span.lo as usize].chars().take_while(|c| *c == ' ').count();
        let mut k = open + 1;
        let mut prev_hi = toks[open].span.hi as usize;
        while k < close {
            let t = &toks[k];
            let gap = &src[prev_hi..t.span.lo as usize];
            if let Some(nl) = gap.rfind('\n') {
                self.out.push_str(gap[..=nl].trim_end_matches(' '));
                let ind = gap[nl + 1..].len().saturating_sub(base_indent);
                for _ in 0..self.level * 4 + ind {
                    self.out.push(' ');
                }
            } else {
                self.out.push_str(gap);
            }
            self.at_line_start = false;
            let empty_call = t.kind == Tk::Open('(')
                && k + 1 < close
                && toks[k + 1].kind == Tk::Close(')')
                && k > 0
                && toks[k - 1].span.hi == t.span.lo
                && matches!(toks[k - 1].kind, Tk::Ident | Tk::Close(')') | Tk::Gt);
            // `name(args)` inside a macro's braces is an application, as
            // everywhere: `m.insert($k, $v)` -> `m <- insert ($k) ($v)`.
            let bang = k + 1 < close && toks[k + 1].kind == Tk::Punct && toks[k + 1].text == "!" && toks[k + 1].span.lo == t.span.hi;
            let open_at = if bang { k + 2 } else { k + 1 };
            if t.kind == Tk::Ident
                && !crate::rules::is_keyword(&t.text)
                && open_at < close
                && toks[open_at].kind == Tk::Open('(')
                && toks[open_at].span.lo == toks[open_at - 1].span.hi
                && !(open_at + 1 < close && toks[open_at + 1].kind == Tk::Close(')'))
            {
                if let Some(ce) = Self::atom_end(toks, open_at, close) {
                    self.out.push_str(&t.text);
                    if bang {
                        self.out.push('!');
                    }
                    let args = Self::args_of(toks, open_at, ce - 1);
                    for (a, b) in &args {
                        self.out.push(' ');
                        // One token, a metavariable `$x`, or a repetition
                        // `$( .. )*` is an atom.
                        let is_meta = toks[*a].kind == Tk::Punct && toks[*a].text == "$";
                        let rep_end = if is_meta && *a + 1 < *b && toks[*a + 1].kind == Tk::Open('(') {
                            Self::atom_end(toks, *a + 1, *b).map(|e| if e < *b && toks[e].kind == Tk::Punct && matches!(toks[e].text.as_str(), "*" | "+" | "?") { e + 1 } else { e })
                        } else {
                            None
                        };
                        let atomic = (*b - *a == 1 && matches!(toks[*a].kind, Tk::Ident | Tk::Int | Tk::Str | Tk::Char))
                            || (is_meta && *b - *a == 2 && toks[*a + 1].kind == Tk::Ident)
                            || rep_end == Some(*b);
                        if !atomic {
                            self.out.push('(');
                        }
                        self.macro_body_span(toks, *a, *b);
                        if !atomic {
                            self.out.push(')');
                        }
                    }
                    prev_hi = toks[ce - 1].span.hi as usize;
                    k = ce;
                    continue;
                }
            }
            match t.kind {
                Tk::PathSep => self.out.push('.'),
                Tk::Dot | Tk::LArrow => {
                    let member = k + 1 < close && toks[k + 1].kind == Tk::Ident;
                    if member {
                        while self.out.ends_with(' ') {
                            self.out.pop();
                        }
                        self.out.push_str(" <- ");
                    } else {
                        self.out.push('.');
                    }
                }
                _ if empty_call => {
                    self.out.push('$');
                    k += 1;
                }
                _ => self.out.push_str(&t.text),
            }
            prev_hi = toks[k].span.hi as usize;
            k += 1;
        }
        let gap = &src[prev_hi..toks[close].span.lo as usize];
        if let Some(nl) = gap.rfind('\n') {
            self.out.push_str(gap[..=nl].trim_end_matches(' '));
            let ind = gap[nl + 1..].len().saturating_sub(base_indent);
            for _ in 0..self.level * 4 + ind {
                self.out.push(' ');
            }
        } else {
            self.out.push_str(gap);
        }
        self.at_line_start = false;
    }

    /// The arms of a `macro_rules!` body, one per line: the matcher in rule-4
    /// groups, the transcriber as written (rule 3, `::` -> `.`, member `.`
    /// -> `<-`, `()` -> `$`), the `;` between arms dropped.
    fn macro_rules_arms(&mut self, toks: &[Token], cl: &Classes, from: usize, to: usize) {
        let mut d = 0i32;
        let mut start = from;
        let mut arms: Vec<(usize, usize)> = Vec::new();
        for i in from..to {
            match toks[i].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                Tk::Semi if d == 0 => {
                    arms.push((start, i));
                    start = i + 1;
                }
                _ => {}
            }
        }
        if start < to && toks[start..to].iter().any(|t| !t.is_comment()) {
            arms.push((start, to));
        }
        let _ = cl;
        for (a, b) in arms {
            let sig: Vec<usize> = (a..b).filter(|&i| !toks[i].is_comment()).collect();
            let Some(&m0) = sig.first() else { continue };
            // Comments before the arm keep their own lines.
            for i in a..m0 {
                if toks[i].is_comment() {
                    self.newline();
                    self.word(&toks[i].text, false);
                    self.newline();
                }
            }
            let Some(m1) = Self::atom_end(toks, m0, b) else { continue };
            let arrow = sig.iter().copied().find(|&i| i >= m1 && toks[i].kind == Tk::FatArrow);
            self.newline();
            self.indent();
            self.matcher(toks, m0, m1 - 1);
            let Some(arrow) = arrow else {
                self.newline();
                continue;
            };
            self.word("=>", true);
            let rest: Vec<usize> = sig.iter().copied().filter(|&i| i > arrow).collect();
            if let Some(&o) = rest.first() {
                if let Tk::Open(ch) = toks[o].kind {
                    if let Some(c) = Self::atom_end(toks, o, b).map(|e| e - 1) {
                        // A transcriber over several lines is a `do:` block
                        // of Harsh; on one line it keeps its braces.
                        if ch == '{' && toks[c].line != toks[o].line {
                            self.word("do:", true);
                            self.newline();
                            self.level += 1;
                            self.emit(toks, cl, o + 1, c, Kind::Stmts);
                            self.level -= 1;
                        } else {
                            self.word(&ch.to_string(), true);
                            self.macro_body(toks, o, c);
                            self.out.push_str(&toks[c].text);
                            self.at_line_start = false;
                        }
                    }
                }
            }
            self.newline();
        }
    }

    /// Rule 4, converter side. In a parenthesised matcher, the items a
    /// top-level comma separates each become a group, and a repetition with
    /// a comma separator, `$( .. ),*`, becomes `$( ( .. ) )*` with its body
    /// mapped the same way. A matcher with neither is written as it is; a
    /// bracketed or braced matcher always is (rule 5).
    fn matcher(&mut self, toks: &[Token], open: usize, close: usize) {
        if toks[open].kind != Tk::Open('(') {
            self.out.push_str(&toks[open].text);
            self.macro_body(toks, open, close);
            self.out.push_str(&toks[close].text);
            self.at_line_start = false;
            return;
        }
        self.out.push('(');
        self.at_line_start = false;
        self.matcher_items(toks, open + 1, close);
        self.out.push(')');
    }

    fn matcher_items(&mut self, toks: &[Token], from: usize, to: usize) {
        // Items at top level, split at commas; repetitions are one item each.
        let mut items: Vec<(usize, usize)> = Vec::new();
        let mut d = 0i32;
        let mut start = from;
        let mut i = from;
        while i < to {
            match toks[i].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                Tk::Comma if d == 0 => {
                    // The `,` of a repetition's `),*` is its separator, not
                    // an item boundary.
                    let rep_sep = i > 0
                        && toks[i - 1].kind == Tk::Close(')')
                        && i + 1 < to
                        && matches!(toks[i + 1].text.as_str(), "*" | "+")
                        && Self::rep_open_of(toks, from, i - 1).is_some();
                    if !rep_sep {
                        items.push((start, i));
                        start = i + 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if start < to {
            items.push((start, to));
        }
        items.retain(|(a, b)| toks[*a..*b].iter().any(|t| !t.is_comment()));
        let rep_with_comma = |a: usize, b: usize| -> Option<(usize, usize, usize)> {
            // `$( body ),op` -- returns (group open, group close, op index)
            let sig: Vec<usize> = (a..b).filter(|&i| !toks[i].is_comment()).collect();
            if sig.len() < 4 || toks[sig[0]].text != "$" || toks[sig[1]].kind != Tk::Open('(') {
                return None;
            }
            let c = Self::atom_end(toks, sig[1], b)? - 1;
            let after: Vec<usize> = sig.iter().copied().filter(|&i| i > c).collect();
            match after.as_slice() {
                [sep, op] if toks[*sep].kind == Tk::Comma && matches!(toks[*op].text.as_str(), "*" | "+") => Some((sig[1], c, *op)),
                _ => None,
            }
        };
        let grouped = items.len() > 1 || items.iter().any(|&(a, b)| rep_with_comma(a, b).is_some());
        if !grouped {
            if let Some(&(a, b)) = items.first() {
                self.matcher_tokens(toks, a, b);
            }
            return;
        }
        for (n, &(a, b)) in items.iter().enumerate() {
            if n > 0 {
                self.out.push(' ');
            }
            if let Some((o, c, op)) = rep_with_comma(a, b) {
                self.out.push_str("$( (");
                self.matcher_items(toks, o + 1, c);
                self.out.push_str(") )");
                self.out.push_str(&toks[op].text);
            } else {
                let sig: Vec<usize> = (a..b).filter(|&i| !toks[i].is_comment()).collect();
                let bare_rep = sig.len() >= 2 && toks[sig[0]].text == "$" && toks[sig[1]].kind == Tk::Open('(');
                if bare_rep {
                    self.matcher_tokens(toks, a, b);
                } else {
                    self.out.push('(');
                    self.matcher_tokens(toks, a, b);
                    self.out.push(')');
                }
            }
        }
    }

    /// The `(` matching the `)` at `close`, if a `$` precedes it.
    fn rep_open_of(toks: &[Token], from: usize, close: usize) -> Option<usize> {
        let mut d = 0i32;
        let mut k = close;
        loop {
            match toks[k].kind {
                Tk::Close(_) => d += 1,
                Tk::Open(_) => {
                    d -= 1;
                    if d == 0 {
                        return (k > from && toks[k - 1].text == "$").then_some(k);
                    }
                }
                _ => {}
            }
            if k == from {
                return None;
            }
            k -= 1;
        }
    }

    /// Tokens as written with the rule-3 substitutions -- `::` -> `.`, a
    /// member `.` -> `<-`, an empty call `()` -> `$` -- and single spaces.
    fn matcher_tokens(&mut self, toks: &[Token], from: usize, to: usize) {
        let mut prev_hi: Option<u32> = None;
        let mut i = from;
        while i < to {
            let t = &toks[i];
            if t.is_comment() {
                i += 1;
                continue;
            }
            if let Some(ph) = prev_hi {
                if t.span.lo > ph {
                    self.out.push(' ');
                }
            }
            let empty_call = t.kind == Tk::Open('(')
                && i + 1 < to
                && toks[i + 1].kind == Tk::Close(')')
                && i > 0
                && toks[i - 1].span.hi == t.span.lo
                && matches!(toks[i - 1].kind, Tk::Ident | Tk::Close(')') | Tk::Gt);
            match t.kind {
                Tk::PathSep => self.out.push('.'),
                Tk::Dot | Tk::LArrow if i + 1 < to && toks[i + 1].kind == Tk::Ident => {
                    self.out.push_str(" <- ");
                    prev_hi = Some(toks[i + 1].span.lo);
                    i += 1;
                    continue;
                }
                _ if empty_call => {
                    self.out.push('$');
                    i += 1;
                }
                _ => self.out.push_str(&t.text),
            }
            prev_hi = Some(toks[i].span.hi);
            i += 1;
        }
        self.at_line_start = false;
    }

    /// Is `toks[open]` the `(` of a `fn` parameter list that holds a doc
    /// comment or an attribute at depth one? Returns the list's `)`.
    fn documented_list(&self, toks: &[Token], open: usize, to: usize) -> Option<usize> {
        // The list's `(` is one a param comma points at, or the first group
        // of a `fn` (found by walking back to the `fn`).
        let is_list = self.param_commas.values().any(|&o| o == open) || {
            let mut k = open;
            let mut seen_name = false;
            let mut gd = 0i32;
            loop {
                if k == 0 {
                    break false;
                }
                k -= 1;
                match toks[k].kind {
                    Tk::Gt => gd += 1,
                    Tk::Lt => gd -= 1,
                    Tk::Ident if gd == 0 && !seen_name => seen_name = true,
                    Tk::Ident if gd == 0 && seen_name => break toks[k].is_kw("fn"),
                    _ if gd == 0 => break false,
                    _ => {}
                }
            }
        };
        if !is_list {
            return None;
        }
        let mut d = 0i32;
        let mut has = false;
        for k in open..to {
            match toks[k].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => {
                    d -= 1;
                    if d == 0 {
                        return if has { Some(k) } else { None };
                    }
                }
                Tk::Hash if d == 1 => has = true,
                _ if d == 1 && toks[k].is_comment() => has = true,
                _ => {}
            }
        }
        None
    }

    /// The documented form of a parameter list: see `documented_list`.
    fn documented_params(&mut self, toks: &[Token], cl: &Classes, open: usize, close: usize) {
        // Split at the top-level commas.
        let mut params: Vec<(usize, usize)> = Vec::new();
        let mut d = 0i32;
        let mut start = open + 1;
        for k in open + 1..close {
            match toks[k].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                Tk::Comma if d == 0 => {
                    params.push((start, k));
                    start = k + 1;
                }
                _ => {}
            }
        }
        if start < close && toks[start..close].iter().any(|t| !t.is_comment()) {
            params.push((start, close));
        }
        self.level += 1;
        for (a, b) in params {
            let mut k = a;
            // Doc lines above the group.
            while k < b && toks[k].is_comment() {
                self.newline();
                self.word(&toks[k].text, false);
                k += 1;
            }
            self.newline();
            self.word("(", false);
            // Attributes inside it, then the parameter.
            while k < b && toks[k].kind == Tk::Hash {
                let lb = k + 1;
                let mut dd = 0i32;
                let mut rb = lb;
                while rb < b {
                    match toks[rb].kind {
                        Tk::Open(_) => dd += 1,
                        Tk::Close(_) => {
                            dd -= 1;
                            if dd == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    rb += 1;
                }
                self.out.push_str("#[");
                self.run_span(toks, cl, lb + 1, rb, true);
                while self.out.ends_with(' ') {
                    self.out.pop();
                }
                self.out.push_str("] ");
                k = rb + 1;
            }
            self.run_span(toks, cl, k, b, false);
            while self.out.ends_with(' ') {
                self.out.pop();
            }
            self.out.push(')');
        }
        // The return type, if any, on its own line.
        self.newline();
        self.indent();
        self.level -= 1;
    }

    /// The index just past a block expression starting at the keyword at
    /// `kw`: past the `}` of its block, and of every `else` block chained
    /// to it.
    fn block_expr_end(toks: &[Token], cl: &Classes, kw: usize, to: usize) -> Option<usize> {
        let mut i = kw;
        let mut d = 0i32;
        // The first `{` at depth zero after the keyword.
        while i < to {
            match toks[i].kind {
                Tk::Open('{') if d == 0 => break,
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                Tk::Semi if d == 0 => return None,
                _ => {}
            }
            i += 1;
        }
        if i >= to || !matches!(cl.brace[i], Brace::Block(_)) {
            return None;
        }
        let mut end = cl.partner[i] + 1;
        // `else` chains.
        while end < to && toks[end].is_kw("else") {
            let mut j = end + 1;
            let mut dd = 0i32;
            while j < to {
                match toks[j].kind {
                    Tk::Open('{') if dd == 0 => break,
                    Tk::Open(_) => dd += 1,
                    Tk::Close(_) => dd -= 1,
                    _ => {}
                }
                j += 1;
            }
            if j >= to || cl.partner[j] == usize::MAX {
                return None;
            }
            end = cl.partner[j] + 1;
        }
        Some(end)
    }

    /// Is the brace at `open` a struct literal's -- a path of names before
    /// it, in expression position, its body fields -- rather than a pattern
    /// (`let Name { .. } =`, `Name { .. } =>`, `if let`, `for`) or a block?
    /// A DSL body, `{` to `}`, with every piece of Rust code in it written as
    /// a hole of Harsh. The pieces are found by shape, knowing no DSL: a
    /// `{ … }` group, and a value after `=` that ends where Rust itself stops
    /// reading an expression -- before `>`, `/>`, `;` or a `,` outside a
    /// closure's parameters, or where two atoms meet with nothing between
    /// them. A literal stays as written, `type="button"`, `{"active"}`. Each
    /// hole is checked by transpiling it back; one that does not give the same
    /// Rust is an error, named with its line.
    fn dsl_body(&mut self, toks: &[Token], open: usize, close: usize) -> String {
        let src = self.src;
        let mut holes: Vec<(usize, usize)> = Vec::new();
        let is_lit = |t: &Token| matches!(t.kind, Tk::Str | Tk::Int | Tk::Float | Tk::Char) || (t.kind == Tk::Ident && (t.text == "true" || t.text == "false"));
        let atom_end = |t: &Token| matches!(t.kind, Tk::Ident | Tk::Str | Tk::Int | Tk::Float | Tk::Char | Tk::Close(_)) && !matches!(t.text.as_str(), "move" | "as" | "else" | "in" | "if" | "match" | "return" | "async");
        // (A `{` never ends an attribute's value: in Rust it continues the
        // expression, `if c {`, `Point {`. Only a `for`/`if` of the DSL ends
        // at its body's `{`. Found 2026-09-23: `class=if c { .. }` became a
        // hole `if c`, the rest left as Rust.)
        let atom_start = |t: &Token| matches!(t.kind, Tk::Ident | Tk::Str | Tk::Int | Tk::Float | Tk::Char) && !matches!(t.text.as_str(), "as" | "else");
        let sig = |k: usize| (k..close).find(|&j| !toks[j].is_comment());
        // A literal, or a bracket group of literals only, `["a", "b"]`: the
        // same text in both languages, and written as it is.
        let literal_range = |a: usize, b: usize| {
            let v: Vec<&Token> = toks[a..=b].iter().filter(|t| !t.is_comment()).collect();
            !v.is_empty()
                && v.iter().all(|t| is_lit(t) || matches!(t.kind, Tk::Comma | Tk::Open('[') | Tk::Close(']') | Tk::Open('(') | Tk::Close(')')))
                && (v.len() == 1 || v[0].kind == Tk::Open('[') || v[0].kind == Tk::Open('('))
        };
        // The value after `start`: up to where Rust stops reading an
        // expression -- `>`, `/>`, `;`, a `,` outside a closure's parameters,
        // the `}` that closes the element, a DSL body's `{` when `body_brace`,
        // or two atoms meeting with nothing between them.
        let value_end = |v: usize, body_brace: bool| -> usize {
            let (mut dd, mut bars, mut last, mut j) = (0i32, false, v, v);
            while j < close {
                let u = &toks[j];
                if u.is_comment() {
                    j += 1;
                    continue;
                }
                if dd == 0 && j > v {
                    let stop = u.kind == Tk::Gt
                        || (u.text == "/" && toks.get(j + 1).map_or(false, |n| n.kind == Tk::Gt))
                        || u.kind == Tk::Semi
                        || (u.kind == Tk::Comma && !bars)
                        || matches!(u.kind, Tk::Close(_))
                        || u.kind == Tk::FatArrow
                        || (body_brace && u.kind == Tk::Open('{'))
                        || (atom_start(u) && atom_end(&toks[last]));
                    if stop {
                        break;
                    }
                }
                match u.kind {
                    Tk::Open(_) => dd += 1,
                    Tk::Close(_) => dd -= 1,
                    _ => {}
                }
                if dd == 0 && u.text == "|" {
                    bars = !bars;
                }
                last = j;
                j += 1;
            }
            last
        };
        // A `[ … ]` holding `{ "key": … }` objects is the DSL's own array
        // (`json!`): scanned through, its Rust found at the leaves.
        let object_array = |k: usize| {
            toks[k].kind == Tk::Open('[') && {
                let c = Self::partner_of(toks, k, close);
                (k + 1..c).any(|j| toks[j].kind == Tk::Open('{') && sig(j + 1).map_or(false, |a| toks[a].kind == Tk::Str) && sig(j + 1).and_then(|a| sig(a + 1)).map_or(false, |b| toks[b].kind == Tk::Colon))
            }
        };
        // An `=` that ends at a `=>` is a `select!` arm's.
        let arm_eq_of = |k: usize| toks[k].kind == Tk::Eq && sig(k + 1).map_or(false, |v| toks[value_end(v, false) + 1..close].iter().find(|u| !u.is_comment()).map_or(false, |u| u.kind == Tk::FatArrow));
        // `d` counts brackets that are Rust's; a DSL's own body braces
        // (`div {`, `for … {`) are transparent, scanned as the body is.
        let mut d = 0i32;
        let mut kinds: Vec<bool> = Vec::new();
        let mut k = open + 1;
        while k < close {
            let t = &toks[k];
            if t.is_comment() {
                k += 1;
                continue;
            }
            let prev = (open + 1..k).rev().find(|&j| !toks[j].is_comment()).map(|j| &toks[j]);
            if d == 0 && object_array(k) {
                kinds.push(true);
                k += 1;
                continue;
            }
            if d == 0 && t.kind == Tk::Open('{') {
                // After a name or a `)` it opens the DSL's own nested body;
                // anywhere else it holds Rust code.
                let dsl = prev.map_or(false, |p| matches!(p.kind, Tk::Ident | Tk::Close(')')) && !matches!(p.text.as_str(), "move" | "else"))
                    // `{ "key": value, … }` is the DSL's own object (`json!`),
                    // never a block of Rust.
                    || (sig(k + 1).map_or(false, |a| toks[a].kind == Tk::Str) && sig(k + 1).and_then(|a| sig(a + 1)).map_or(false, |c| toks[c].kind == Tk::Colon));
                if dsl {
                    kinds.push(true);
                    k += 1;
                    continue;
                }
                let c = Self::partner_of(toks, k, close);
                let inner: Vec<&Token> = toks[k + 1..c].iter().filter(|t| !t.is_comment()).collect();
                if !inner.is_empty() && !literal_range(k + 1, c - 1) {
                    holes.push((inner[0].span.lo as usize, inner.last().unwrap().span.hi as usize));
                }
                k = c + 1;
                continue;
            }
            // `name=value` (markup) and `name: value` (a tree's attribute).
            // A tree's `:` is followed by a space, `class: "app"`; markup's
            // namespace is tight, `class:active=` / `on:click=`, and is part
            // of the attribute's name.
            let tree_colon = t.kind == Tk::Colon && toks.get(k + 1).map_or(false, |n| n.span.lo > t.span.hi);
            let attr = (t.kind == Tk::Eq || tree_colon) && (prev.map_or(false, |p| (p.kind == Tk::Ident && !crate::rules::is_keyword(&p.text)) || (tree_colon && p.kind == Tk::Str)) || arm_eq_of(k));
            // `pattern = future => handler,` (`select!`'s arms): the `=`
            // follows the arm's start, and a `=>` ends the value. The pattern
            // is a hole unless it is a bare name; so is the handler.
            if d == 0 && arm_eq_of(k) {
                let start = (open + 1..k).rev().take_while(|&j| !(d == 0 && (toks[j].kind == Tk::Comma || toks[j].kind == Tk::Close('}')))).last().unwrap_or(k);
                let pat: Vec<usize> = (start..k).filter(|&j| !toks[j].is_comment()).collect();
                if pat.len() > 1 {
                    holes.push((toks[pat[0]].span.lo as usize, toks[*pat.last().unwrap()].span.hi as usize));
                }
            }
            if d == 0 && t.kind == Tk::FatArrow {
                if let Some(v) = sig(k + 1) {
                    if toks[v].kind != Tk::Open('{') {
                        let last = value_end(v, false);
                        if !literal_range(v, last) {
                            holes.push((toks[v].span.lo as usize, toks[last].span.hi as usize));
                        }
                        k = last + 1;
                        continue;
                    }
                }
            }
            // `for pat in EXPR {` and `if EXPR {`: the DSL's control flow.
            let flow = t.kind == Tk::Ident && (t.text == "in" || (t.text == "if" && prev.map_or(true, |p| p.text != "else")));
            if d == 0 && (attr || flow) {
                if let Some(v) = sig(k + 1) {
                    if toks[v].kind != Tk::Open('{') && !object_array(v) {
                        let last = value_end(v, flow);
                        if !literal_range(v, last) {
                            holes.push((toks[v].span.lo as usize, toks[last].span.hi as usize));
                        }
                        k = last + 1;
                        continue;
                    }
                }
            }
            match t.kind {
                Tk::Open(_) => {
                    kinds.push(false);
                    d += 1;
                }
                Tk::Close(_) => {
                    if kinds.pop() == Some(false) {
                        d -= 1;
                    }
                }
                _ => {}
            }
            k += 1;
        }
        holes.sort();
        let (lo, hi) = (toks[open].span.lo as usize, toks[close].span.hi as usize);
        let mut out = String::new();
        let mut at = lo;
        for (a, b) in holes {
            out.push_str(&src[at..a]);
            let frag = &src[a..b];
            let line_start = src[..a].rfind('\n').map_or(0, |n| n + 1);
            let base = src[line_start..].len() - src[line_start..].trim_start().len();
            match hole_of(frag) {
                Ok(h) => {
                    let lines: Vec<&str> = h.lines().collect();
                    if lines.len() == 1 {
                        out.push_str(&format!("@: {} :@", lines[0]));
                    } else {
                        out.push_str(&format!("@: {}", lines[0]));
                        for l in &lines[1..] {
                            out.push('\n');
                            out.push_str(&" ".repeat(base));
                            out.push_str(l);
                        }
                        out.push('\n');
                        out.push_str(&" ".repeat(base));
                        out.push_str(":@");
                    }
                }
                Err(e) => {
                    let line = src[..a].matches('\n').count() + 1;
                    self.errors.push(format!("line {line}: this Rust in a macro's body could not be written as Harsh ({e}): {}", frag.trim()));
                    out.push_str(frag);
                }
            }
            at = b;
        }
        out.push_str(&src[at..hi]);
        out
    }

    /// Is the stream between `from` and `to` a comma-separated list of
    /// expressions -- the one stream Harsh writes by juxtaposition? Read by
    /// shape, the way Rust reads: a piece is not an expression when two atoms
    /// meet with nothing between them (`SELECT name`, `p class`), when it
    /// starts with markup's `<`, or when it is a brace group of `"key": value`
    /// pairs (`json!`). An empty stream is an empty list.
    fn is_expression_list(toks: &[Token], from: usize, to: usize) -> bool {
        let sig: Vec<&Token> = toks[from..to].iter().filter(|t| !t.is_comment()).collect();
        if sig.first().map_or(false, |t| t.kind == Tk::Lt) {
            return false;
        }
        let ends_atom = |t: &Token| matches!(t.kind, Tk::Ident | Tk::Str | Tk::Int | Tk::Float | Tk::Char | Tk::Close(_)) && !matches!(t.text.as_str(), "move" | "as" | "else" | "in" | "if" | "match" | "return" | "async" | "mut" | "ref" | "dyn" | "impl" | "const" | "unsafe" | "static" | "let");
        // `if` after a pattern is its guard, `matches!(t, Some(n) if n > 1)`:
        // part of the expression, like `as` and `else` (the self-host round
        // trip found it, 2026-09-23).
        let starts_atom = |t: &Token| matches!(t.kind, Tk::Ident | Tk::Str | Tk::Int | Tk::Float | Tk::Char) && !matches!(t.text.as_str(), "as" | "else" | "if");
        let mut d = 0i32;
        for w in 0..sig.len() {
            let t = sig[w];
            if d == 0 && w > 0 {
                if starts_atom(t) && ends_atom(sig[w - 1]) {
                    return false;
                }
                if t.kind == Tk::Comma && sig.get(w + 1).map_or(false, |n| n.kind == Tk::Lt) {
                    return false;
                }
            }
            // A brace group that is a piece of its own, holding `"key": v`.
            if t.kind == Tk::Open('{') && (w == 0 || matches!(sig[w - 1].kind, Tk::Comma | Tk::Open(_))) {
                if sig.get(w + 1).map_or(false, |a| a.kind == Tk::Str) && sig.get(w + 2).map_or(false, |c| c.kind == Tk::Colon) {
                    return false;
                }
            }
            match t.kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                _ => {}
            }
        }
        true
    }

    /// The matching closer of the bracket at `k`, or `end`.
    fn partner_of(toks: &[Token], k: usize, end: usize) -> usize {
        let mut d = 0i32;
        for (j, t) in toks.iter().enumerate().take(end + 1).skip(k) {
            match t.kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => {
                    d -= 1;
                    if d == 0 {
                        return j;
                    }
                }
                _ => {}
            }
        }
        end
    }

    fn is_struct_literal(toks: &[Token], open: usize, close: usize) -> bool {
        // The path before the brace.
        let mut k = open;
        let mut path_start = None;
        while k > 0 {
            let p = &toks[k - 1];
            if p.kind == Tk::Ident && !crate::rules::is_keyword(&p.text) && !BLOCK_KEYWORDS.contains(&p.text.as_str()) && p.text != "do" {
                path_start = Some(k - 1);
                k -= 1;
                if k > 0 && toks[k - 1].kind == Tk::PathSep {
                    k -= 1;
                    continue;
                }
                break;
            }
            break;
        }
        let Some(first) = path_start else { return false };
        // Uppercase first letter of the *last* segment: a type
        // (`geometry::Vec2`), not a variable before a block.
        if !toks[open - 1].text.chars().next().map_or(false, |c| c.is_uppercase()) {
            return false;
        }
        if first > 0 {
            let b = &toks[first - 1];
            if b.kind == Tk::Ident && (crate::rules::is_keyword(&b.text) || BLOCK_KEYWORDS.contains(&b.text.as_str()) || matches!(b.text.as_str(), "in" | "mut" | "ref")) {
                return false;
            }
            if b.kind == Tk::Punct && (b.text == "&" || b.text == "!") {
                return false;
            }
            // `|Point { x, y }| ..` is a pattern: the `|` *opens* a closure's
            // parameters. `|x| Point { x, y: 0 }` is a literal, the closure's
            // body: the `|` *closes* them, so a parameter stands before it.
            // An or-pattern, `A | B { .. } =>`, has a name there too, and is
            // told apart below by what follows it (2026-09-21).
            if b.kind == Tk::Punct && b.text == "|" {
                let closes = first >= 2 && matches!(toks[first - 2].kind, Tk::Ident | Tk::Close(_) | Tk::Gt);
                if !closes {
                    return false;
                }
            }
        }
        // A `let` before it, with no `=` between: a pattern, however deep
        // in tuples and `Some(..)` it sits -- `if let Some(Frame { .. }) =`.
        let mut d = 0i32;
        for k in (0..first).rev() {
            match toks[k].kind {
                // A previous statement's block ends here: the statement
                // starts after it.
                Tk::Close('}') if d == 0 => break,
                Tk::Close(_) => d += 1,
                // Out through the pattern's own parens and brackets, but a
                // brace is a block boundary: the statement starts after it.
                Tk::Open('{') if d == 0 => break,
                Tk::Open(_) => {
                    if d == 0 {
                        continue;
                    }
                    d -= 1;
                }
                Tk::Eq if d == 0 => break,
                Tk::Semi | Tk::FatArrow if d == 0 => break,
                // A `for`'s pattern ends at `in` (2026-10-03).
                Tk::Ident if d == 0 && toks[k].is_kw("in") => break,
                Tk::Ident if d == 0 && toks[k].is_kw("let") => return false,
                Tk::Ident if d == 0 && toks[k].is_kw("for") => return false,
                _ => {}
            }
        }
        // What follows, to the end of this expression: `=>`, `:` or a
        // depth-zero `=` make it a pattern. A closer at depth zero ends the
        // expression (`Err(LexError { .. })`), as do `;`, `,`, and a chain.
        let mut d = 0i32;
        for x in &toks[close + 1..] {
            match x.kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => {
                    if d == 0 {
                        break;
                    }
                    d -= 1;
                }
                Tk::FatArrow | Tk::Colon if d == 0 => return false,
                Tk::Eq if d == 0 => return false,
                Tk::Semi | Tk::Dot | Tk::LArrow | Tk::Comma if d == 0 => break,
                _ => {}
            }
        }
        // The body is fields, or empty.
        let inner: Vec<&Token> = toks[open + 1..close].iter().filter(|t| !t.is_comment()).collect();
        !inner.is_empty()
            && (matches!(inner[0].kind, Tk::Ident | Tk::DotDot)
                && (inner.len() == 1 || inner.iter().any(|t| matches!(t.kind, Tk::Colon | Tk::Comma | Tk::DotDot))))
    }

    /// A struct pattern's brace: a path of names before it, in pattern
    /// position (a `let`/`for` before with no `=` between, or `=>` after),
    /// its body field names, renames `x: px`, and `..`.
    fn is_struct_pattern(toks: &[Token], open: usize, close: usize) -> bool {
        // The path.
        let mut k = open;
        let mut first = None;
        while k > 0 {
            let p = &toks[k - 1];
            if p.kind == Tk::Ident && !crate::rules::is_keyword(&p.text) && !BLOCK_KEYWORDS.contains(&p.text.as_str()) {
                first = Some(k - 1);
                k -= 1;
                if k > 0 && toks[k - 1].kind == Tk::PathSep {
                    k -= 1;
                    continue;
                }
                break;
            }
            break;
        }
        let Some(first) = first else { return false };
        if !toks[open - 1].text.chars().next().map_or(false, |c| c.is_uppercase()) {
            return false;
        }
        // Pattern position.
        let mut d = 0i32;
        let mut binder = false;
        for j in (0..first).rev() {
            match toks[j].kind {
                Tk::Close('}') if d == 0 => break,
                Tk::Close(_) => d += 1,
                Tk::Open('{') if d == 0 => break,
                Tk::Open(_) => {
                    if d == 0 {
                        continue;
                    }
                    d -= 1;
                }
                Tk::Eq if d == 0 => break,
                Tk::Semi | Tk::FatArrow if d == 0 => break,
                // A `for`'s pattern ends at `in`: after it is an expression
                // (`for (P { a }, n) in [(P { a: 1 }, 2)]`, 2026-10-03).
                Tk::Ident if d == 0 && toks[j].is_kw("in") => break,
                Tk::Ident if d == 0 && (toks[j].is_kw("let") || toks[j].is_kw("for")) => {
                    binder = true;
                    break;
                }
                _ => {}
            }
        }
        if !binder {
            // The groups around the brace -- `Some(..)`, a tuple `(.., x)`,
            // a call `g(..)`: a comma inside one of them belongs to it; a
            // comma outside them all ends the arm, and a `=>` after that is
            // the next arm's. Without this, a literal in an arm's body,
            // `Some(s) => E { a: s }, _ => ..`, found the next arm's `=>` and
            // was read as a pattern (found 2026-10-03).
            let mut outer = 0i32;
            {
                let mut d = 0i32;
                for j in (0..open).rev() {
                    match toks[j].kind {
                        Tk::Close(_) => d += 1,
                        Tk::Open(c) => {
                            if d == 0 {
                                if c == '(' || c == '[' {
                                    outer += 1;
                                    continue;
                                }
                                break;
                            }
                            d -= 1;
                        }
                        Tk::FatArrow | Tk::Semi | Tk::Eq if d == 0 => break,
                        Tk::Comma if d == 0 && outer == 0 => break,
                        _ => {}
                    }
                }
            }
            let mut d = 0i32;
            let mut arm = false;
            for x in &toks[close + 1..] {
                match x.kind {
                    Tk::Open(_) => d += 1,
                    // Out through the parens a `Some(..)` puts around the
                    // pattern; a `}` at depth zero ends the arm's body.
                    Tk::Close('}') if d == 0 => break,
                    Tk::Close(_) => {
                        if d == 0 {
                            outer -= 1;
                            continue;
                        }
                        d -= 1;
                    }
                    Tk::Comma if d == 0 && outer <= 0 => break,
                    Tk::FatArrow if d == 0 => {
                        arm = true;
                        break;
                    }
                    Tk::Semi if d == 0 => break,
                    _ => {}
                }
            }
            if !arm {
                return false;
            }
        }
        // Body: names, `..`, `name: pat`, nested patterns -- but not braces.
        !toks[open + 1..close].iter().any(|t| t.kind == Tk::Open('{'))
    }

    /// Write a struct literal in Harsh's form. `Name` has already been
    /// written; this writes from the `{`.
    fn struct_literal(&mut self, toks: &[Token], cl: &Classes, open: usize, close: usize) {
        // Fields, split at top-level commas.
        let mut fields: Vec<(usize, usize)> = Vec::new();
        let mut d = 0i32;
        let mut start = open + 1;
        for k in open + 1..close {
            match toks[k].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                Tk::Comma if d == 0 => {
                    fields.push((start, k));
                    start = k + 1;
                }
                _ => {}
            }
        }
        if start < close && toks[start..close].iter().any(|t| !t.is_comment()) {
            fields.push((start, close));
        }
        let multi = toks[close].line != toks[open].line;
        while self.out.ends_with(' ') {
            self.out.pop();
        }
        // A multi-line literal that is a `let`'s value starts on the line
        // after the `=`: `let p =` / `Point:` / fields -- the preferred form.
        if multi {
            let line = self.out.rsplit('\n').next().unwrap_or("").to_string();
            if let Some(eq) = line.rfind(" = ") {
                let name = line[eq + 3..].to_string();
                if !name.contains(' ') {
                    let cut = self.out.len() - name.len() - 1;
                    self.out.truncate(cut);
                    let ind = self.current_line_indent();
                    self.out.push('\n');
                    for _ in 0..ind + 4 {
                        self.out.push(' ');
                    }
                    self.out.push_str(&name);
                }
            }
        }
        self.out.push('\\');
        self.at_line_start = false;
        if fields.is_empty() {
            return;
        }
        if multi {
            let saved = self.level;
            self.level = self.current_line_indent() / 4 + 1;
            for (a, b) in fields {
                self.newline();
                self.indent();
                self.literal_field(toks, cl, a, b);
            }
            self.level = saved;
            self.newline();
        } else {
            for (n, (a, b)) in fields.iter().enumerate() {
                self.out.push_str(if n == 0 { " " } else { ", " });
                self.at_line_start = false;
                self.literal_field(toks, cl, *a, *b);
            }
        }
    }

    /// One field of a literal: `name: value` -> `name = value`; `name`;
    /// `..base`. Comments among the fields are kept above their field.
    fn literal_field(&mut self, toks: &[Token], cl: &Classes, a: usize, b: usize) {
        let mut k = a;
        while k < b && toks[k].is_comment() {
            self.word(&toks[k].text, false);
            self.newline();
            self.indent();
            k += 1;
        }
        if k >= b {
            return;
        }
        if toks[k].kind == Tk::DotDot {
            self.out.push_str("..");
            self.at_line_start = false;
            self.run(toks, cl, k + 1, b, Kind::Stmts);
            while self.out.ends_with(' ') {
                self.out.pop();
            }
            return;
        }
        let colon = (k..b).find(|&j| toks[j].kind == Tk::Colon);
        match colon {
            Some(c) if c == k + 1 => {
                self.out.push_str(&toks[k].text);
                self.out.push_str(" = ");
                self.at_line_start = false;
                self.run(toks, cl, c + 1, b, Kind::Stmts);
                while self.out.ends_with(' ') {
                    self.out.pop();
                }
            }
            _ => {
                self.run(toks, cl, k, b, Kind::Stmts);
                while self.out.ends_with(' ') {
                    self.out.pop();
                }
            }
        }
    }

    fn run_braced(&mut self, toks: &[Token], cl: &Classes, from: usize, to: usize) {
        self.brace_depth += 1;
        self.run_braced_inner(toks, cl, from, to);
        self.brace_depth -= 1;
    }

    fn run_braced_inner(&mut self, toks: &[Token], cl: &Classes, from: usize, to: usize) {
        let mut d = 0i32;
        let mut start = from;
        for i in from..to {
            match toks[i].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                Tk::Semi if d == 0 => {
                    self.run(toks, cl, start, i, Kind::Stmts);
                    self.word(";", false);
                    start = i + 1;
                }
                _ => {}
            }
        }
        if start < to {
            self.run(toks, cl, start, to, Kind::Stmts);
        }
    }

    fn run(&mut self, toks: &[Token], cl: &Classes, from: usize, to: usize, kind: Kind) {
        match Self::expr_start(toks, from, to, kind) {
            None => self.run_span(toks, cl, from, to, false),
            Some(s) if s == from => self.run_span(toks, cl, from, to, true),
            Some(s) => {
                // Everything before the expression is a pattern or a binding
                // form: `Some(p)` there is a pattern, not a call.
                self.run_span(toks, cl, from, s, false);
                // The split resets `prev`, so the gap across it must be
                // re-emitted or `= 0` becomes `=0`.
                if s > from
                    && s < to
                    && toks[s].span.lo > toks[s - 1].span.hi
                    && !self.out.ends_with(' ')
                    && !self.out.ends_with('\n')
                {
                    self.out.push(' ');
                }
                self.run_span(toks, cl, s, to, true);
            }
        }
    }

    /// Where the expression part of an item begins, if any.
    ///
    /// Juxtaposition must not touch patterns, signatures or declarations --
    /// `Some(p)` is a pattern in `if let`, and `Block(Kind)` is an enum variant.
    fn expr_start(toks: &[Token], from: usize, to: usize, kind: Kind) -> Option<usize> {
        let first = toks[from..to].iter().find(|t| !t.is_comment())?;
        if first.kind == Tk::Hash {
            return None;
        }
        let top = |pred: &dyn Fn(&Token) -> bool| -> Option<usize> {
            let mut d = 0i32;
            for i in from..to {
                match toks[i].kind {
                    Tk::Open(_) => d += 1,
                    Tk::Close(_) => d -= 1,
                    _ if d == 0 && pred(&toks[i]) => return Some(i),
                    _ => {}
                }
            }
            None
        };
        // A match arm juxtaposes on both sides: the pattern `Rect(a, b)` and
        // the body are the same grammar.
        if top(&|t| t.kind == Tk::FatArrow).is_some() {
            return Some(from);
        }
        // A struct field is a declaration, not an expression; a tuple
        // variant `V(A, B)` is written `V A B` -- an application of types.
        if kind == Kind::Commas {
            let f = toks[from..to].iter().position(|t| !t.is_comment()).map(|k| from + k)?;
            if toks[f].kind == Tk::Ident && f + 1 < to && toks[f + 1].kind == Tk::Open('(') {
                return Some(f);
            }
            return None;
        }
        // A tuple struct's header, `struct Pair(i32, i32)`: the name applied
        // to its payload types, `struct Pair i32 i32`.
        {
            let mut k = from;
            while k < to && toks[k].kind == Tk::Ident && crate::rules::MODIFIERS.contains(&toks[k].text.as_str()) && !toks[k].is_kw("const") {
                k += 1;
                if k < to && toks[k].kind == Tk::Open('(') {
                    k = Self::atom_end(toks, k, to).unwrap_or(k + 1);
                }
            }
            if k + 2 < to && toks[k].is_kw("struct") && toks[k + 1].kind == Tk::Ident {
                // Past generics.
                let mut j = k + 2;
                if j < to && toks[j].kind == Tk::Lt {
                    let mut d = 0i32;
                    while j < to {
                        match toks[j].kind {
                            Tk::Lt => d += 1,
                            Tk::Gt => {
                                d -= 1;
                                if d == 0 {
                                    j += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                }
                if j < to && toks[j].kind == Tk::Open('(') {
                    return Some(k + 1);
                }
            }
        }
        // Past `pub`, `pub(crate)` and the other modifiers, to the keyword
        // that says what the item is: `pub const X = f(1)` has a value.
        let mut k = from;
        while k < to && toks[k].kind == Tk::Ident && crate::rules::MODIFIERS.contains(&toks[k].text.as_str()) && !toks[k].is_kw("const") {
            k += 1;
            if k < to && toks[k].kind == Tk::Open('(') {
                k = Self::atom_end(toks, k, to).unwrap_or(k + 1);
            }
        }
        let first = toks.get(k).unwrap_or(first);
        if first.kind == Tk::Ident {
            match first.text.as_str() {
                "fn" | "struct" | "enum" | "impl" | "trait" | "mod" | "use" | "type" | "where"
                | "pub" | "unsafe" | "extern" | "macro_rules" => return None,
                // A binding form: the pattern is an application too --
                // `let Some(x) = ..` is `let Some x = ..` -- so the whole
                // statement juxtaposes. `const` and `static` carry a type
                // annotation, not a pattern; their value is the expression.
                "let" => return Some(from),
                "const" | "static" => return top(&|t| t.kind == Tk::Eq).map(|i| i + 1),
                "if" | "while" => return Some(from),
                _ => {}
            }
        }
        Some(from)
    }

    fn run_span(&mut self, toks: &[Token], cl: &Classes, from: usize, to: usize, expr: bool) {
        let mut i = from;
        let mut prev: Option<&Token> = None;
        let mut attr_depth: Option<usize> = None;
        while i < to {
            let t = &toks[i];
            if let Some(d) = attr_depth.as_mut() {
                match toks[i].kind {
                    Tk::Open('[') => *d += 1,
                    Tk::Close(']') => *d -= 1,
                    _ => {}
                }
                if toks[i].kind == Tk::Close(']') && *d == 0 {
                    self.word("]", false);
                    self.newline();
                    attr_depth = None;
                    prev = None;
                    i += 1;
                    continue;
                }
            }
            // A metavariable `$k` is one atom; a repetition `$( .. )*` is
            // written `$(` + its body as Harsh + `)` + its op.
            if t.kind == Tk::Punct && t.text == "$" && i + 1 < to {
                if toks[i + 1].kind == Tk::Ident {
                    let space = needs_space(prev, t);
                    self.word("$", space);
                    self.out.push_str(&toks[i + 1].text);
                    prev = Some(&toks[i + 1]);
                    i += 2;
                    continue;
                }
                if toks[i + 1].kind == Tk::Open('(') {
                    if let Some(e) = Self::atom_end(toks, i + 1, to) {
                        let close = e - 1;
                        let space = needs_space(prev, t);
                        self.word("$(", space);
                        self.at_line_start = false;
                        // The body, its trailing `;` dropped: the layout
                        // supplies the separator (rule 2).
                        let mut b = close;
                        while b > i + 2 && (toks[b - 1].kind == Tk::Semi || toks[b - 1].is_comment()) {
                            b -= 1;
                        }
                        self.run(toks, cl, i + 2, b, Kind::Stmts);
                        while self.out.ends_with(' ') {
                            self.out.pop();
                        }
                        self.out.push(')');
                        let mut k = e;
                        if k < to && toks[k].kind == Tk::Punct && matches!(toks[k].text.as_str(), "*" | "+" | "?") {
                            self.out.push_str(&toks[k].text);
                            k += 1;
                        }
                        self.at_line_start = false;
                        prev = Some(&toks[k - 1]);
                        i = k;
                        continue;
                    }
                }
            }
            // A chain hanging off a block expression -- `if .. { } else
            // { }.into_iter()`, `match .. { }.unwrap_or(x)` -- has no home
            // in the layout as written; isolated in a group, with the `(`
            // on its own line and the chain after the `)`, it does.
            if expr && t.kind == Tk::Ident && matches!(t.text.as_str(), "if" | "match" | "unsafe" | "loop") {
                if let Some(end) = Self::block_expr_end(toks, cl, i, to) {
                    let chained = end < to && matches!(toks[end].kind, Tk::Dot | Tk::LArrow) && toks[end + 1..to].iter().any(|n| n.kind == Tk::Ident);
                    // The same for an operand with more of the expression
                    // after it, `a && match k { .. } && b`: beneath a
                    // block nothing can continue the line, so the `&& b` was
                    // written as a statement of its own (found 2026-09-21).
                    let operand = end < to && i > from && is_binary_operator(&toks[end]);
                    if chained || operand {
                        self.indent();
                        if prev.is_some() && !self.out.ends_with(' ') && !self.out.ends_with('\n') {
                            self.out.push(' ');
                        }
                        self.out.push('(');
                        self.newline();
                        self.level += 1;
                        self.run(toks, cl, i, end, Kind::Stmts);
                        self.level -= 1;
                        self.newline();
                        self.indent();
                        self.out.push(')');
                        self.at_line_start = false;
                        prev = Some(&toks[end - 1]);
                        i = end;
                        continue;
                    }
                }
            }
            // A `(` directly after an atom is a Rust call: rewrite it as
            // juxtaposed arguments. Macros and functions are identical here.
            // The parenthesised types are applications too (`Fn(i32, i32)`
            // -> `Fn i32 i32`, `FnOnce()` -> `FnOnce$`), and so is an
            // attribute's content (`derive(Debug, Clone)` -> `derive Debug
            // Clone`), in every region.
            let type_app = t.kind == Tk::Ident
                && (matches!(t.text.as_str(), "Fn" | "FnMut" | "FnOnce")
                    || (t.text == "fn"
                        && prev.map_or(false, |p| {
                            matches!(p.kind, Tk::Colon | Tk::Lt | Tk::Comma | Tk::Open('('))
                                || p.text == "->"
                                || p.text == "&"
                                || p.is_kw("dyn")
                                || p.is_kw("impl")
                                || p.is_kw("mut")
                        })));
            let in_attr = attr_depth.is_some();
            if (expr || type_app || in_attr) && matches!(t.kind, Tk::Ident) && !(t.text == "fn" && !type_app) {
                // A type's `fn` is a keyword to the atom rule; here it is a head.
                let head_end = if type_app { Some(i + 1) } else { Self::atom_end(toks, i, to) };
                // `struct Pair<T>(..)`: the head is the name with its
                // generics; the payload follows the `>`.
                let head_end = if i > 0 && toks[i - 1].is_kw("struct") && i + 1 < to && toks[i + 1].kind == Tk::Lt {
                    let mut j = i + 1;
                    let mut d = 0i32;
                    while j < to {
                        match toks[j].kind {
                            Tk::Lt => d += 1,
                            Tk::Gt => {
                                d -= 1;
                                if d == 0 {
                                    j += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                    Some(j)
                } else {
                    head_end
                };
                if let Some(ae) = head_end {
                    // A macro's arguments juxtapose exactly like a function's.
                    // `matches!(x, Some(n) if n > 1)` becomes
                    // `matches! x (Some n if n > 1)`; the pattern is one
                    // argument and is isolated like any other multi-token one.
                    let soup = false;
                    if soup {
                        // Emit the macro and its group exactly as written: its
                        // contents are patterns, not expressions.
                        if let Some(ce) = Self::atom_end(toks, ae, to) {
                            self.indent();
                            if prev.is_some()
                                && !self.out.ends_with(' ')
                                && !self.out.ends_with('\n')
                                && !self.out.ends_with('(')
                            {
                                self.out.push(' ');
                            }
                            self.run_span(toks, cl, i, ce, false);
                            i = ce;
                            prev = Some(&toks[ce - 1]);
                            continue;
                        }
                    }
                    // A macro's bracket call is a call like any other (the
                    // delimiter means nothing to the macro): `vec![1, 2]` ->
                    // `vec! 1 2`, `vec![]` -> `vec!$`. A stream that is not a
                    // list of expressions -- `vec![x; n]` -- is a token
                    // stream, `vec! { x; n }` (ruled 2026-09-22, A1 and D1).
                    // Its inside is written as Harsh while brace bodies are
                    // transpiled; when they become verbatim (ruling 16), it
                    // is copied as Rust.
                    let bang_bracket = ae < to
                        && ae > i
                        && toks[ae].kind == Tk::Open('[')
                        && toks[ae - 1].kind == Tk::Punct
                        && toks[ae - 1].text == "!";
                    if bang_bracket {
                        if let Some(ce) = Self::atom_end(toks, ae, to) {
                            let close = ce - 1;
                            let mut d = 0i32;
                            let repeat = toks[ae + 1..close].iter().any(|x| {
                                match x.kind {
                                    Tk::Open(_) => d += 1,
                                    Tk::Close(_) => d -= 1,
                                    _ => {}
                                }
                                d == 0 && x.kind == Tk::Semi
                            });
                            if repeat {
                                self.indent();
                                if prev.is_some() && !self.out.ends_with(' ') && !self.out.ends_with('\n') && !self.out.ends_with('(') {
                                    self.out.push(' ');
                                }
                                self.run_span(toks, cl, i, ae, false);
                                // Its inside is Rust, copied as written.
                                let inside = self.src[toks[ae].span.hi as usize..toks[close].span.lo as usize].trim();
                                self.out.push_str(&format!(" {{ {inside} }}"));
                                i = ce;
                                prev = Some(&toks[close]);
                                continue;
                            }
                        }
                    }
                    // A macro called with `(…)` or `[…]` whose stream is not a
                    // list of expressions is a DSL: braces delimit it in Harsh,
                    // with its Rust in holes, and only the delimiter changes
                    // (the user's rule, 2026-09-23: parens isolate Harsh code,
                    // braces delimit a whole stream). `sql!(SELECT name FROM t)`
                    // -> `sql! { SELECT name FROM t }`.
                    let bang_call = ae < to && ae > i && toks[ae - 1].kind == Tk::Punct && toks[ae - 1].text == "!" && matches!(toks[ae].kind, Tk::Open('(') | Tk::Open('['));
                    if bang_call {
                        if let Some(ce) = Self::atom_end(toks, ae, to) {
                            let close = ce - 1;
                            if !Self::is_expression_list(toks, ae + 1, close) {
                                self.indent();
                                if prev.is_some() && !self.out.ends_with(' ') && !self.out.ends_with('\n') && !self.out.ends_with('(') {
                                    self.out.push(' ');
                                }
                                self.run_span(toks, cl, i, ae, false);
                                let body = self.dsl_body(toks, ae, close);
                                // The stream's own delimiter becomes a brace pair.
                                let inner = &body[1..body.len() - 1];
                                self.out.push_str(&format!(" {{{inner}}}"));
                                self.at_line_start = false;
                                i = ce;
                                prev = Some(&toks[close]);
                                continue;
                            }
                        }
                    }
                    if ae < to && (toks[ae].kind == Tk::Open('(') || bang_bracket) {
                        if let Some(ce) = Self::atom_end(toks, ae, to) {
                            let close = ce - 1;
                            // Emit the callee, preserving the gap before it.
                            self.indent();
                            if prev.is_some()
                                && !self.out.ends_with(' ')
                                && !self.out.ends_with('\n')
                                && !self.out.ends_with('(')
                                && !self.out.ends_with("#[")
                            {
                                self.out.push(' ');
                            }
                            self.run_span(toks, cl, i, ae, false);
                            let args = Self::args_of(toks, ae, close);
                            // `f()` applies to nothing: `f$`, tight. `f(())`
                            // passes the unit value: `f ()`, an ordinary group.
                            if args.is_empty() {
                                while self.out.ends_with(' ') {
                                    self.out.pop();
                                }
                                self.out.push('$');
                            }
                            for (a, b) in &args {
                                self.out.push(' ');
                                let is_unit = *b - *a == 2
                                    && toks[*a].kind == Tk::Open('(')
                                    && toks[*a + 1].kind == Tk::Close(')');
                                if is_unit {
                                    self.out.push_str("()");
                                    continue;
                                }
                                // `f$` is one atom, so an argument that is an
                                // empty call needs no isolating parens.
                                let ae2 = Self::atom_end(toks, *a, *b);
                                // A brace group is never an atom in Harsh:
                                // `json! ({ .. })`, `f x ({ .. })`.
                                let brace = toks[*a].kind == Tk::Open('{');
                                // A tuple argument, `(a, b)`, is a construct:
                                // isolated as `((a, b))`, or Harsh reads two
                                // arguments.
                                let tuple = toks[*a].kind == Tk::Open('(') && {
                                    let mut d = 0i32;
                                    let mut comma = false;
                                    for x in &toks[*a + 1..*b - 1] {
                                        match x.kind {
                                            Tk::Open(_) => d += 1,
                                            Tk::Close(_) => d -= 1,
                                            Tk::Comma if d == 0 => comma = true,
                                            _ => {}
                                        }
                                    }
                                    comma && Self::atom_end(toks, *a, *b) == Some(*b)
                                };
                                // An array literal, `[1, 2]`, is isolated too:
                                // brackets always index in Harsh (the user's
                                // rule, 2026-09-11), so a bare `f [1, 2]` is
                                // an index of `f`. The converter wrote it bare
                                // until 2026-09-21, when the self-host round
                                // trip caught `starts_with(['=', '.'])`.
                                let array = toks[*a].kind == Tk::Open('[');
                                let brace = brace || tuple || array;
                                // A metavariable `$k` is one atom.
                                let meta = *b - *a == 2 && toks[*a].kind == Tk::Punct && toks[*a].text == "$" && toks[*a + 1].kind == Tk::Ident;
                                // In a type application (`Fn a b`) only a
                                // bare name or path is one atom; a generic
                                // `Option<T>` is isolated.
                                let simple_type = toks[*a..*b].iter().all(|x| x.kind == Tk::Ident || x.kind == Tk::PathSep);
                                let type_list = type_app || self.in_enum_body || (i > 0 && toks[i - 1].is_kw("struct"));
                                let atomic = meta || (!brace
                                    && (!type_list || simple_type)
                                    && (ae2 == Some(*b)
                                        || (ae2 == Some(*b - 2)
                                            && toks[*b - 2].kind == Tk::Open('(')
                                            && toks[*b - 1].kind == Tk::Close(')'))));
                                if atomic {
                                    self.run_span(toks, cl, *a, *b, true);
                                } else {
                                    self.out.push('(');
                                    // An argument that opens a block ends
                                    // on a new line; its `)` takes a line
                                    // of its own, at this level. A block
                                    // keyword on the callee's line (`Some
                                    // (if c:`) keeps that line's level, so
                                    // its `else` aligns with the header.
                                    let saved = self.level;
                                    // A brace-bodied closure is written with
                                    // its prototype on its own line, one unit
                                    // in; anything else opens its block on the
                                    // callee's line.
                                    let closure_block = {
                                        let mut k = *a;
                                        if k < *b && toks[k].is_kw("move") {
                                            k += 1;
                                        }
                                        if k < *b && toks[k].kind == Tk::Punct && toks[k].text.starts_with('|') {
                                            let bar_end = if toks[k].text == "||" { k + 1 } else { (k + 1..*b).find(|&j| toks[j].kind == Tk::Punct && toks[j].text == "|").map(|j| j + 1).unwrap_or(*b) };
                                            bar_end < *b && toks[bar_end].kind == Tk::Open('{')
                                        } else {
                                            false
                                        }
                                    };
                                    self.level = self.current_line_indent() / 4 + if closure_block { 1 } else { 0 };
                                    self.run_span(toks, cl, *a, *b, true);
                                    self.level = saved;
                                    while self.out.ends_with(' ') {
                                        self.out.pop();
                                    }
                                    if self.at_line_start {
                                        self.indent();
                                    }
                                    self.out.push(')');
                                }
                            }
                            i = ce;
                            prev = Some(&toks[ce - 1]);
                            continue;
                        }
                    }
                }
            }
            // A closure prototype `|a, b: T|` is written tight, its
            // parameters `, `-separated; `||` as itself. A `|` elsewhere is
            // the bit-or operator and is spaced.
            if t.kind == Tk::Punct && (t.text == "|" || t.text == "||") {
                let opens = prev.map_or(true, |p| {
                    matches!(p.kind, Tk::Open(_) | Tk::Comma | Tk::Eq | Tk::FatArrow | Tk::Semi)
                        || p.is_kw("move")
                        || p.is_kw("return")
                        || p.is_kw("mut")
                        || (p.kind == Tk::Punct && !p.text.ends_with('|'))
                });
                // To the closing bar, at depth zero; only then is this a
                // prototype the writer can lay out itself.
                let mut k = i + 1;
                let mut d = 0i32;
                while t.text == "|" && k < to {
                    match toks[k].kind {
                        Tk::Open(_) => d += 1,
                        Tk::Close(_) => d -= 1,
                        Tk::Punct if d == 0 && toks[k].text == "|" => break,
                        _ => {}
                    }
                    k += 1;
                }
                if opens && (t.text == "||" || k < to) {
                    let space = prev.map_or(false, |p| !matches!(p.kind, Tk::Open(_)));
                    self.word(&t.text, space);
                    if t.text == "|" {
                        {
                            let mut pp: Option<&Token> = None;
                            for j in i + 1..k {
                                let tj = &toks[j];
                                let sp = match tj.kind {
                                    Tk::Comma | Tk::Colon => false,
                                    _ => pp.map_or(false, |p| p.kind == Tk::Comma || p.kind == Tk::Colon || (needs_space(Some(p), tj) && p.kind != Tk::Punct)),
                                };
                                match tj.kind {
                                    Tk::PathSep => self.out.push('.'),
                                    _ => self.word(&tj.text, sp),
                                }
                                pp = Some(tj);
                            }
                            self.out.push('|');
                            prev = Some(&toks[k]);
                            i = k + 1;
                            continue;
                        }
                    }
                    prev = Some(t);
                    i += 1;
                    continue;
                }
            }
            match t.kind {
                Tk::Open('{') => match cl.brace[i] {
                    Brace::Block(kind) => {
                        let close = cl.partner[i];
                        self.indent();
                        while self.out.ends_with(' ') {
                            self.out.pop();
                        }
                        // A bare block, `{ .. }` with nothing before it on
                        // its line, is `do:`; after `=>` an arm's block needs
                        // no marker.
                        let line = self.out.rsplit('\n').next().unwrap_or("").to_string();
                        let mut operand_block = false;
                        if line.trim().is_empty() {
                            while self.out.ends_with(' ') {
                                self.out.pop();
                            }
                            self.at_line_start = true;
                            self.indent();
                            self.out.push_str("do");
                        } else if line.trim_end().ends_with('=') {
                            // A block as a value: `let x = do:`.
                            while self.out.ends_with(' ') {
                                self.out.pop();
                            }
                            self.out.push_str(" do");
                        } else if i > 0 && matches!(toks[i - 1].kind, Tk::Open('(') | Tk::Comma) {
                            // A block argument: `(do:` with the body beneath
                            // and `)` under the `(`.
                            while self.out.ends_with(' ') {
                                self.out.pop();
                            }
                            if self.out.ends_with('(') {
                                // The argument path wrote the `(` and will
                                // write the `)`.
                                self.out.push_str("do");
                            } else {
                                self.out.push_str(" (do");
                                operand_block = true;
                            }
                        } else if i > 0 && is_binary_operator(&toks[i - 1]) && !closure_before_brace(toks, i) {
                            // A block as an operand: `a && (do:` .. `)`.
                            while self.out.ends_with(' ') {
                                self.out.pop();
                            }
                            self.out.push_str(" (do");
                            operand_block = true;
                        }
                        // A declaration's body follows its name with no
                        // mark (`struct Point`, `enum Truth`, a record
                        // variant's bare name); every other block opens
                        // with `:`.
                        let decl = {
                            let words: Vec<&str> = line.split_whitespace().collect();
                            // `impl<'a> Emitter<'a>` is one word to
                            // `split_whitespace`, so a keyword is matched up
                            // to its generics.
                            let named = |k: &str| {
                                words
                                    .iter()
                                    .any(|w| *w == k || w.split('<').next() == Some(k))
                            };
                            (kind == Kind::Commas
                                && (named("struct") || named("enum") || named("union")
                                    || (self.in_enum_body && words.len() == 1)))
                                // Item bodies take no mark either, since
                                // 2026-09-17: `impl`, `trait`, `mod`, `extern`.
                                || (kind == Kind::Items
                                    && (named("impl") || named("trait") || named("mod") || named("extern")))
                        };
                        // An empty record -- a variant `Home {}`, a `struct
                        // Empty {}` -- stays as Rust writes it, on its line
                        // (Harsh reads `Home {}` so): a block would need a
                        // statement, and the unit `()` that fills an empty
                        // function body is no field (found 2026-09-26
                        // converting the website back: `Home { (), }`).
                        if decl && kind == Kind::Commas && close != usize::MAX && close == i + 1 {
                            while self.out.ends_with(' ') {
                                self.out.pop();
                            }
                            self.out.push_str(" {}");
                            self.at_line_start = false;
                            // An item ends its line; a variant's `,` does that itself.
                            if toks.get(close + 1).map_or(true, |t| t.text != ",") {
                                self.newline();
                            }
                            prev = Some(&toks[close]);
                            i = close + 1;
                            continue;
                        }
                        // A match's arms are a specification block in use:
                        // opened by `\`, like a literal's fields (2026-09-18).
                        let is_match = kind == Kind::Commas && {
                            let mut d = 0i32;
                            let mut seen = false;
                            // A bracket inside a literal is text: `Tk.Open '['`
                            // before the `match` left the depth at one, and the
                            // retired `match x:` was written (found 2026-09-21).
                            // The statement, not only its last line: a scrutinee
                            // ending in a paren block, `match x.and_then(|d| {
                            // .. })`, leaves `)` alone on the last line, and the
                            // `match` lines above it (found 2026-10-03).
                            let line = {
                                let lines: Vec<&str> = self.out.split('\n').collect();
                                let bal = |t: &str| {
                                    mask_literals(t).chars().fold(0i32, |d, c| match c {
                                        '(' | '[' => d + 1,
                                        ')' | ']' => d - 1,
                                        _ => d,
                                    })
                                };
                                let mut k = lines.len().saturating_sub(1);
                                let mut text = lines.get(k).copied().unwrap_or("").to_string();
                                while bal(&text) < 0 && k > 0 {
                                    k -= 1;
                                    text = format!("{} {}", lines[k], text);
                                }
                                mask_literals(&text)
                            };
                            for w in line.split_whitespace() {
                                for c in w.chars() {
                                    match c {
                                        '(' | '[' => d += 1,
                                        ')' | ']' => d -= 1,
                                        _ => {}
                                    }
                                }
                                if d == 0 && w == "match" {
                                    seen = true;
                                }
                            }
                            seen
                        };
                        if is_match {
                            self.out.push('\\');
                        } else if !self.out.ends_with("=>") && !decl {
                            self.out.push(':');
                        }
                        let was_enum = self.in_enum_body;
                        self.in_enum_body = kind == Kind::Commas && line.split_whitespace().any(|w| w == "enum");
                        let operand = operand_block;
                        self.newline();
                        self.level += 1;
                        self.emit(toks, cl, i + 1, close.min(to), kind);
                        // An empty body: the unit is its one statement,
                        // after any comment.
                        if close != usize::MAX && toks[i + 1..close.min(to)].iter().all(|t| t.is_comment()) {
                            self.newline();
                            self.indent();
                            self.out.push_str("()");
                            self.at_line_start = false;
                        }
                        self.level -= 1;
                        self.in_enum_body = was_enum;
                        self.newline();
                        if operand {
                            self.indent();
                            self.out.push(')');
                            self.at_line_start = false;
                            prev = Some(&toks[close]);
                            i = close + 1;
                            continue;
                        }
                        i = close + 1;
                        prev = None;
                        continue;
                    }
                    Brace::UseGroup => {
                        // The `::` before it was already emitted as `.`
                        self.word("(", false);
                    }
                    Brace::Verbatim => {
                        let close = cl.partner[i];
                        let macro_body = i > 0 && toks[i - 1].kind == Tk::Punct && toks[i - 1].text == "!";
                        let macro_rules = i >= 3
                            && toks[i - 1].kind == Tk::Ident
                            && toks[i - 2].text == "!"
                            && toks[i - 3].is_kw("macro_rules");
                        if macro_rules && close != usize::MAX && close < to && close > i + 1 {
                            // `macro_rules! name { arms }` -> `macro_rules! name:`
                            // with one arm per line, its matcher in rule-4 groups
                            // and its transcriber kept in Rust's braces (rule 3).
                            self.word(":", false);
                            self.newline();
                            self.level += 1;
                            self.macro_rules_arms(toks, cl, i + 1, close);
                            self.level -= 1;
                            self.newline();
                            i = close + 1;
                            prev = None;
                            continue;
                        }
                        // A Rust macro's brace body: the DSL is copied as
                        // written, and every piece of Rust code in it becomes a
                        // hole of Harsh, `@: … :@` (principle 2 and the user's
                        // algorithm, 2026-09-23: the converter writes the holes
                        // an author would).
                        if macro_body && !macro_rules && close != usize::MAX && close < to {
                            if !self.at_line_start && !self.out.ends_with(' ') && !self.out.ends_with('(') {
                                self.out.push(' ');
                            }
                            let body = self.dsl_body(toks, i, close);
                            // The body's lines follow the converter's layout: each is
                            // moved by as much as the line it starts on moved. Only
                            // whitespace at a line's start changes -- never inside a
                            // string that spans lines.
                            let orig_line = self.src[..toks[i].span.lo as usize].rsplit('\n').next().unwrap_or("");
                            let orig_ind = (orig_line.len() - orig_line.trim_start().len()) as isize;
                            let out_line = self.out.rsplit('\n').next().unwrap_or("");
                            let out_ind = (out_line.len() - out_line.trim_start().len()) as isize;
                            let multiline_str = toks[i..close].iter().any(|t| t.kind == Tk::Str && t.text.contains('\n'));
                            let body = if out_ind == orig_ind || multiline_str {
                                body
                            } else {
                                body.split('\n').enumerate().map(|(n, l)| {
                                    if n == 0 {
                                        return l.to_string();
                                    }
                                    let ind = (l.len() - l.trim_start().len()) as isize;
                                    let to = (ind + out_ind - orig_ind).max(0) as usize;
                                    if l.trim().is_empty() { String::new() } else { format!("{}{}", " ".repeat(to), l.trim_start()) }
                                }).collect::<Vec<_>>().join("\n")
                            };
                            self.out.push_str(&body);
                            self.at_line_start = false;
                            prev = Some(&toks[close]);
                            i = close + 1;
                            continue;
                        }
                        // (`hm!{ 1 => "a" }` was written `hm!\\ 1 => "a"` from
                        // 2026-09-20 to 2026-09-22; `m!\\` is retired, A2, and a
                        // brace call keeps its braces.)
                        if macro_body && close != usize::MAX && close < to {
                            // `view! { … }`, `quote! { … }`: the body has its own
                            // grammar and its own layout. Keep the author's
                            // whitespace, line for line; change only the tokens
                            // Harsh spells differently.
                            self.word("{", true);
                            self.macro_body(toks, i, close);
                            self.out.push('}');
                            i = close + 1;
                            prev = Some(&toks[close]);
                            continue;
                        }
                        // A struct literal: `Name { f: v, .. }` is built with
                        // `Name: f = v, ..` -- inline when the Rust was on one
                        // line, a block (one field per line) when it spanned
                        // lines. Patterns keep their braces.
                        // A struct pattern, `Name { a, b, .. }` in a `let`, a
                        // `for`, an arm: written `Name\ a, b, ..`.
                        if close != usize::MAX && close < to && Self::is_struct_pattern(toks, i, close) {
                            while self.out.ends_with(' ') {
                                self.out.pop();
                            }
                            // One element of a tuple pattern, `(P { a }, x)`:
                            // isolated, `((P\ a), x)` -- unparenthesised, its
                            // list would take the tuple's other elements as
                            // fields (as for a literal; found 2026-10-03).
                            let isolate = innermost_open_is_tuple(toks, i);
                            if isolate {
                                let mut first = i;
                                while first > 0 && matches!(toks[first - 1].kind, Tk::Ident | Tk::PathSep) {
                                    first -= 1;
                                }
                                let path: String = toks[first..i]
                                    .iter()
                                    .map(|t| if t.kind == Tk::PathSep { ".".to_string() } else { t.text.clone() })
                                    .collect();
                                if self.out.ends_with(&path) {
                                    let at = self.out.len() - path.len();
                                    self.out.insert(at, '(');
                                }
                            }
                            self.out.push_str("\\ ");
                            self.at_line_start = false;
                            let mut pp: Option<&Token> = None;
                            for k in i + 1..close {
                                let tk = &toks[k];
                                if tk.is_comment() {
                                    continue;
                                }
                                let sp = match tk.kind {
                                    Tk::Comma | Tk::Colon => false,
                                    _ => pp.map_or(false, |p| p.kind == Tk::Comma || p.kind == Tk::Colon || needs_space(Some(p), tk)),
                                };
                                match tk.kind {
                                    Tk::PathSep => self.out.push('.'),
                                    _ => self.word(&tk.text, sp),
                                }
                                pp = Some(tk);
                            }
                            while self.out.ends_with(' ') {
                                self.out.pop();
                            }
                            if isolate {
                                self.out.push(')');
                            }
                            self.out.push(' ');
                            i = close + 1;
                            prev = Some(&toks[close]);
                            continue;
                        }
                        if close != usize::MAX && close < to && !self.in_enum_body && Self::is_struct_literal(toks, i, close) {
                            // Inside `[ .. ]` a literal is isolated in parens
                            // (`vec! [(Point: x = 1)]`): an inline block reads
                            // to its `)`, and brackets are Rust's.
                            // A literal that is the last thing in its
                            // brackets needs none: its field list ends at the
                            // `]` anyway, and `vec![Arm { m: 1 }]` must come
                            // back as itself (2026-09-19).
                            // ... when it is written on one line. A literal
                            // spanning lines keeps its parens: its fields are
                            // laid out beneath, and the parens are what hold
                            // that layout apart from the bracket's.
                            let one_line = !toks[i + 1..close].iter().any(|t| t.line_start.is_some());
                            let last_in_brackets = one_line
                                && toks[close + 1..]
                                    .iter()
                                    .find(|t| !t.is_comment())
                                    .map_or(false, |t| t.kind == Tk::Close(']'));
                            // A macro's bracket list is juxtaposed now, and
                            // the call path isolates each argument itself: a
                            // second pair here would reach the Rust (A1,
                            // 2026-09-22). The `[x; n]` form still needs it.
                            // (The innermost opener of any kind: `enclosing_open`
                            // tracks braces only.)
                            let innermost = {
                                let mut stack: Vec<usize> = Vec::new();
                                for (k, t) in toks[..i].iter().enumerate() {
                                    match t.kind {
                                        Tk::Open(_) => stack.push(k),
                                        Tk::Close(_) => {
                                            stack.pop();
                                        }
                                        _ => {}
                                    }
                                }
                                stack.last().copied()
                            };
                            let macro_list = innermost.map_or(false, |o| {
                                o > 0
                                    && toks[o].kind == Tk::Open('[')
                                    && toks[o - 1].kind == Tk::Punct
                                    && toks[o - 1].text == "!"
                                    && {
                                        let mut d = 0i32;
                                        let mut semi = false;
                                        for t in &toks[o + 1..] {
                                            match t.kind {
                                                Tk::Open(_) => d += 1,
                                                Tk::Close(_) => d -= 1,
                                                Tk::Semi if d == 0 => semi = true,
                                                _ => {}
                                            }
                                            if d < 0 {
                                                break;
                                            }
                                        }
                                        !semi
                                    }
                            });
                            // A one-line literal inside another one-line literal:
                            // Harsh refuses it unparenthesised (the user's rule,
                            // 2026-09-29) -- where the inner one ends cannot be
                            // seen. Inside a multi-line literal the field's line
                            // ends it, and nothing is needed.
                            let nested_inline = toks[close].line == toks[i].line && {
                                let mut stack: Vec<(char, usize)> = Vec::new();
                                for (k, t) in toks[..i].iter().enumerate() {
                                    match t.kind {
                                        Tk::Open(c) => stack.push((c, k)),
                                        Tk::Close(_) => {
                                            stack.pop();
                                        }
                                        _ => {}
                                    }
                                }
                                stack.last().map_or(false, |&(c, o)| {
                                    let mut d = 0i32;
                                    let oc = (o..toks.len()).find(|&k| {
                                        match toks[k].kind {
                                            Tk::Open(_) => d += 1,
                                            Tk::Close(_) => d -= 1,
                                            _ => {}
                                        }
                                        d == 0
                                    });
                                    c == '{' && oc.map_or(false, |oc| toks[oc].line == toks[o].line && Self::is_struct_literal(toks, o, oc))
                                })
                            };
                            // A literal reached through, `P { r: 1 }.twice()`:
                            // isolated, `(P\ r = 1) <- twice$`, or its list
                            // would take the chain (found 2026-10-03).
                            let reached = toks.get(close + 1).map_or(false, |t| t.text == ".");
                            let in_brackets = (innermost_open_is_bracket(toks, i) && !last_in_brackets && !macro_list)
                                || innermost_open_is_tuple(toks, i)
                                || nested_inline
                                || reached;
                            if in_brackets {
                                // The name is already written: put `(` before it.
                                let line = self.out.rsplit('\n').next().unwrap_or("").to_string();
                                let name_len = line.trim_end().rsplit(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.')).next().map_or(0, |n| n.len());
                                let cut = self.out.trim_end().len() - name_len;
                                self.out.insert(cut, '(');
                            }
                            self.struct_literal(toks, cl, i, close);
                            if in_brackets {
                                self.out.push(')');
                            }
                            i = close + 1;
                            prev = Some(&toks[close]);
                            continue;
                        }
                        // A block expression over several lines in operand
                        // position -- `a && { .. }`, `x + { .. }` -- has no
                        // braces in Harsh: it is a `do:` block isolated in
                        // parens, `(do:` with the body beneath and `)`.
                        // Braces nested inside a brace *argument* -- the
                        // object literal of `json! ({ .. })` -- are data and
                        // keep their shape whatever their length.
                        // (A `macro_rules!` transcriber's brace, `=> { .. }`, is
                        // Harsh inside, like a macro body: not an argument.)
                        let inside_brace_arg = enclosing_open(toks, i).map_or(false, |o| {
                            matches!(cl.brace[o], Brace::Verbatim) && !(o > 0 && (toks[o - 1].kind == Tk::FatArrow || (toks[o - 1].kind == Tk::Punct && toks[o - 1].text == "!")))
                        }) || (i > 0 && matches!(toks[i - 1].kind, Tk::Open('(') | Tk::Comma));
                        if close != usize::MAX && close < to && close > i + 1 && toks[close].line != toks[i].line && !inside_brace_arg {
                            // A bare block in statement position -- at a
                            // line's start, after `;`, `{` or `}` -- is `do:`
                            // with no parens; an operand's is `(do:`.
                            let stmt_pos = prev.map_or(true, |p| matches!(p.kind, Tk::Semi | Tk::Open('{') | Tk::Close('}')));
                            if stmt_pos {
                                self.newline();
                                self.indent();
                                self.out.push_str("do:");
                                self.at_line_start = false;
                                self.newline();
                                self.level += 1;
                                self.emit(toks, cl, i + 1, close, Kind::Stmts);
                                self.level -= 1;
                                self.newline();
                                i = close + 1;
                                prev = None;
                                continue;
                            }
                            self.word("(do:", true);
                            let saved = self.level;
                            self.level = self.current_line_indent() / 4 + 1;
                            self.newline();
                            self.emit(toks, cl, i + 1, close, Kind::Stmts);
                            self.level = saved;
                            self.newline();
                            self.indent();
                            self.out.push(')');
                            self.at_line_start = false;
                            i = close + 1;
                            prev = Some(&toks[close]);
                            continue;
                        }
                        if close != usize::MAX && close < to && close > i + 1 {
                            self.word("{", true);
                            self.run_braced(toks, cl, i + 1, close);
                            self.word("}", true);
                            i = close + 1;
                            prev = Some(&toks[close]);
                            continue;
                        }
                        self.word("{", true)
                    }
                },
                Tk::Close('}') => match cl.partner[i] {
                    p if p != usize::MAX && cl.brace[p] == Brace::UseGroup => {
                        self.word(")", false)
                    }
                    p => {
                        self.word("}", true);
                        // An empty body kept in braces, `struct Empty {}`,
                        // ends its item: an item that follows on a later
                        // line starts a line of its own (found 2026-09-27:
                        // `struct Empty { } fn g$`).
                        let empty = p != usize::MAX && p + 1 == i;
                        let item_next = toks.get(i + 1).map_or(false, |n| {
                            n.line > t.line
                                && (n.kind == Tk::Hash
                                    || matches!(n.text.as_str(), "fn" | "struct" | "enum" | "union" | "impl" | "trait" | "mod" | "use" | "const" | "static" | "type" | "pub" | "extern" | "unsafe" | "async"))
                        });
                        if empty && item_next {
                            self.newline();
                        }
                    }
                },
                Tk::PathSep => {
                    // `::<T>` turbofish loses its `::`; Harsh writes `Vec<i32>.new()`.
                    let next_is_lt = toks.get(i + 1).map(|t| t.kind == Tk::Lt).unwrap_or(false);
                    if !next_is_lt {
                        self.word(".", false);
                    }
                }
                Tk::Dot | Tk::LArrow => self.word("<-", true),
                Tk::TupleIdx => self.word(&t.text, false),
                Tk::Hash => {
                    self.newline();
                    self.word("#", false);
                    attr_depth = Some(0);
                    // `#[name ..]`: the bracket and the name are tight.
                    if i + 2 < to && toks[i + 1].kind == Tk::Open('[') {
                        self.out.push('[');
                        self.at_line_start = false;
                        prev = Some(&toks[i + 1]);
                        i += 2;
                        if let Some(d) = attr_depth.as_mut() {
                            *d += 1;
                        }
                        continue;
                    }
                }
                Tk::Semi => self.word(";", false),
                // An empty call whose callee the call rule did not recognise
                // (a turbofish, say) still reaches here as `(` `)`: it is `$`.
                Tk::Open('(')
                    if self.empty_params.contains(&i)
                        || (i + 1 < to
                            && toks[i + 1].kind == Tk::Close(')')
                            && prev.map_or(false, |p| p.span.hi == t.span.lo
                                && match p.kind {
                                    Tk::Ident => !crate::rules::is_keyword(&p.text),
                                    Tk::Close(')') | Tk::Gt => true,
                                    Tk::Punct => p.text == "!" || p.text == ">>",
                                    _ => false,
                                })) => {
                    while self.out.ends_with(' ') {
                        self.out.pop();
                    }
                    self.out.push('$');
                    prev = Some(&toks[i + 1]);
                    i += 2;
                    continue;
                }
                // A parameter list with doc comments or attributes inside it
                // -- a Leptos component's props -- takes the documented form:
                // each parameter's doc lines above its own group, the
                // attributes inside it, one group per line, the return type
                // on the line after.
                Tk::Open('(') if self.param_commas.values().any(|&o| o == i) || self.documented_list(toks, i, to).is_some() => {
                    if let Some(close) = self.documented_list(toks, i, to) {
                        self.documented_params(toks, cl, i, close);
                        prev = Some(&toks[close]);
                        i = close + 1;
                        continue;
                    }
                    self.word("(", needs_space(prev, t));
                }
                Tk::Comma if self.param_commas.contains_key(&i) => {
                    let open = self.param_commas[&i];
                    let trailing = toks[i + 1..to]
                        .iter()
                        .find(|t| !t.is_comment())
                        .map(|t| t.kind == Tk::Close(')'))
                        .unwrap_or(true);
                    if !trailing {
                        self.word(") (", false);
                        // What follows is a fresh group: no space after its `(`.
                        prev = Some(&toks[open]);
                        i += 1;
                        continue;
                    }
                }
                Tk::Comma => self.word(",", false),
                Tk::LineComment => {
                    // A line comment runs to end of line: whatever follows it
                    // in the token stream must start a new line -- and, when
                    // the statement goes on (a comment between the links of
                    // a chain), that line is a continuation: one unit in.
                    // (An argument that begins with a comment is mid-statement
                    // too: what precedes it is the `(` or `,` of its list.)
                    // Whole attributes before it do not begin the statement:
                    // `#[inline]` then `/// Second.` then the item put the
                    // item one level too deep (found 2026-09-24).
                    let mid_statement = (!only_attributes(&toks[from..i])
                        || (from > 0 && matches!(toks[from - 1].kind, Tk::Comma | Tk::Open('(') | Tk::Open('[')) && innermost_open_char(toks, from).map_or(false, |c| c != '{')))
                        && toks[i + 1..to].iter().any(|n| !n.is_comment());
                    self.indent();
                    if !self.out.ends_with(' ') && !self.out.ends_with('\n') {
                        self.out.push(' ');
                    }
                    self.out.push_str(&t.text);
                    self.newline();
                    if mid_statement {
                        self.indent();
                        self.out.push_str("    ");
                    }
                    prev = None;
                    i += 1;
                    continue;
                }
                _ => {
                    let space = needs_space(prev, t);
                    self.word(&t.text, space);
                }
            }
            prev = Some(t);
            i += 1;
        }
    }
}

/// `toks[o]` is the `{` of `macro_rules! name {`.
fn is_macro_rules_open(toks: &[Token], o: usize) -> bool {
    o >= 3 && toks[o - 1].kind == Tk::Ident && toks[o - 2].text == "!" && toks[o - 3].is_kw("macro_rules")
}

/// Whether a space is written between `p` and `t`.
///
/// Purely cosmetic -- every variant below is valid Rust either way -- but the
/// output is meant to be read and edited, so it is worth getting close.
fn needs_space(prev: Option<&Token>, t: &Token) -> bool {
    let Some(p) = prev else { return false };

    // Nothing ever follows these directly.
    if matches!(p.kind, Tk::Open('(') | Tk::Open('[') | Tk::PathSep | Tk::Hash | Tk::Lt) {
        return false;
    }
    // `!` is a macro bang or a negation, both tight -- except after
    // `macro_rules!`, where a name follows.
    if p.text == "!" && t.kind == Tk::Ident {
        return true;
    }
    if p.text == "&" || p.text == "!" || p.text == "?" {
        return false;
    }
    // Nothing ever precedes these directly.
    if matches!(
        t.kind,
        Tk::Close(')') | Tk::Close(']') | Tk::Comma | Tk::Semi | Tk::Colon | Tk::TupleIdx
    ) {
        return false;
    }
    if t.text == "?" {
        return false;
    }
    // A negation after a keyword or an operator is spaced, `if !x`,
    // `&& !(a)`; a macro bang is glued to its name.
    if t.text == "!" {
        let after_kw = p.kind == Tk::Ident && crate::rules::is_keyword(&p.text);
        let after_op = p.kind == Tk::Punct && matches!(p.text.as_str(), "&&" | "||" | "==" | "!=");
        return after_kw || after_op || p.kind == Tk::Eq || p.kind == Tk::FatArrow;
    }
    // Generic brackets and ranges bind tight.
    if matches!(t.kind, Tk::Lt | Tk::Gt | Tk::DotDot) || p.kind == Tk::DotDot {
        return false;
    }
    if p.kind == Tk::Gt {
        // `>>`, `>,` bind tight; `> (a: T)`, `> where` and `> =` do not.
        return !matches!(t.kind, Tk::Gt | Tk::PathSep | Tk::Comma | Tk::Semi | Tk::Colon);
    }
    // `(a) > b`, `x) < y`: a comparison after a closer is spaced; only a
    // name opens a generic list.
    if matches!(t.kind, Tk::Gt | Tk::Lt) && matches!(p.kind, Tk::Close(_) | Tk::Int | Tk::Float | Tk::Str) {
        return true;
    }
    // `>>` closes nested generics and binds tight -- except before `=`, where
    // `Option<Vec<T>> = x` must not fuse into the `>>=` shift-assign token.
    if t.text == ">>" {
        return false;
    }
    if p.text == ">>" {
        return t.kind == Tk::Eq || t.text.starts_with('=');
    }
    // A name and the group that isolates its argument are separated by a
    // space in Harsh, `f (x)`; so are a pattern's `Some (x)`, a variant's
    // `Circle (f64)`, a signature's `fn f (a: T)`. Indexing, `xs[i]`, is
    // Rust's and stays tight, as does `)(`.
    if t.kind == Tk::Open('(') {
        return !matches!(p.kind, Tk::Close(')') | Tk::Close(']'));
    }
    if t.kind == Tk::Open('[') {
        let callable = matches!(p.kind, Tk::Ident | Tk::Close(')') | Tk::Close(']') | Tk::Gt);
        return !callable || crate::rules::is_keyword(&p.text);
    }
    true
}

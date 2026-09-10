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
    let toks = crate::lex::lex_rust(src).map_err(|e| e.msg)?;
    let classes = classify_all(&toks);
    let pc = param_commas(&toks);
    let ep = empty_params(&toks);
    let mut w = Writer { out: String::new(), level: 0, at_line_start: true, param_commas: pc, empty_params: ep, src, brace_depth: 0, in_enum_body: false };
    w.emit(&toks, &classes, 0, toks.len(), Kind::Items);
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
            classify_brace(toks, i)
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
            || (t.kind == Tk::Punct && matches!(t.text.as_str(), "&" | "*" | "+" | "'"));
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
    if open > 0
        && matches!(
            toks[open - 1].kind,
            Tk::Ident | Tk::Close(')') | Tk::Close(']') | Tk::Gt
        )
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

/// Walk back from a `{` to the start of its segment -- the run of tokens since
/// the last `;`, `{` or `}` at the same depth -- and decide what the brace is.
fn classify_brace(toks: &[Token], open: usize) -> Brace {
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
            // A `}` at depth zero closes the previous item: the segment ends.
            Tk::Close('}') if depth == 0 => break,
            Tk::Close(_) => depth += 1,
            Tk::Open(_) if depth == 0 => break,
            Tk::Open(_) => depth -= 1,
            Tk::Semi if depth == 0 => break,
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
                for k in crate::rules::top_level_commas(&toks[i..], o, c) {
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
            if *had_semi && n == last && kind == Kind::Stmts && !ended_with_block {
                self.out.push(';');
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

    /// The source whitespace before `toks[k]`, re-based: a line break keeps
    /// the line's indentation relative to `base_indent`, under this level.
    fn source_gap(&mut self, toks: &[Token], prev_hi: usize, k: usize, base_indent: usize) {
        let gap = &self.src[prev_hi..toks[k].span.lo as usize];
        if let Some(nl) = gap.rfind('\n') {
            let head = gap[..=nl].trim_end_matches(' ');
            let head = head.trim_start_matches(' ');
            while self.out.ends_with(' ') {
                self.out.pop();
            }
            self.out.push_str(head);
            let ind = gap[nl + 1..].len().saturating_sub(base_indent);
            for _ in 0..self.level * 4 + ind {
                self.out.push(' ');
            }
        } else {
            self.out.push_str(gap);
        }
        self.at_line_start = false;
    }

    /// Rule 1, converter side. The markup between `open` and `close` is
    /// copied with its own line structure; `::` becomes `.`. A `{ .. }` hole
    /// is converted as a Harsh expression. An attribute value written bare,
    /// `on:click=move |_| ..`, is wrapped in a hole and converted too -- its
    /// end is the depth-zero `>` or `/>` that closes the tag, or the next
    /// `name=` / `ns:name=` attribute; a literal value is left as it is.
    fn hsx_body(&mut self, toks: &[Token], cl: &Classes, open: usize, close: usize) {
        let src = self.src;
        let line_start = src[..toks[open].span.lo as usize].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let base_indent = src[line_start..toks[open].span.lo as usize].chars().take_while(|c| *c == ' ').count();
        // The markup's first line sits under `do:` at this level, so the
        // re-basing uses the first markup line's own indentation.
        let first = open + 1;
        let first_ls = src[..toks[first].span.lo as usize].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let first_indent = src[first_ls..toks[first].span.lo as usize].chars().take_while(|c| *c == ' ').count();
        let base_indent = if toks[first].span.lo as usize >= first_ls && first_ls > toks[open].span.lo as usize { first_indent } else { base_indent };
        self.indent();
        let mut k = open + 1;
        let mut prev_hi: Option<usize> = None;
        while k < close {
            let t = &toks[k];
            if let Some(ph) = prev_hi {
                self.source_gap(toks, ph, k, base_indent);
            }
            // A hole. Its contents are Harsh, and any block they open lays
            // out from the markup line holding the `{`: the writer's level
            // is set from that line's column while the hole is converted.
            if t.kind == Tk::Open('{') {
                let c = cl.partner[k];
                if c != usize::MAX && c < close {
                    self.out.push('{');
                    self.at_line_start = false;
                    let saved = self.level;
                    self.level = self.current_line_indent() / 4 + 1;
                    self.run(toks, cl, k + 1, c, Kind::Stmts);
                    self.level = saved;
                    while self.out.ends_with(' ') {
                        self.out.pop();
                    }
                    self.out.push('}');
                    prev_hi = Some(toks[c].span.hi as usize);
                    k = c + 1;
                    continue;
                }
            }
            // A bare attribute value: `=` after a name, not followed by a
            // literal or a hole.
            if t.kind == Tk::Eq && k > 0 && toks[k - 1].kind == Tk::Ident && k + 1 < close {
                let v = &toks[k + 1];
                let literal = matches!(v.kind, Tk::Str | Tk::Int | Tk::Float | Tk::Char) || v.kind == Tk::Open('{');
                if !literal {
                    let end = Self::attr_value_end(toks, k + 1, close);
                    if end > k + 1 {
                        // Isolated in braces -- the preferred spelling,
                        // one with a Dioxus tree's holes; RSX's own block
                        // form, kept on the way back to Rust. Parens are
                        // accepted too and are dropped.
                        self.out.push_str("={");
                        self.at_line_start = false;
                        let saved = self.level;
                        self.level = self.current_line_indent() / 4 + 1;
                        self.run(toks, cl, k + 1, end, Kind::Stmts);
                        self.level = saved;
                        while self.out.ends_with(' ') {
                            self.out.pop();
                        }
                        self.out.push('}');
                        prev_hi = Some(toks[end - 1].span.hi as usize);
                        k = end;
                        continue;
                    }
                }
            }
            match t.kind {
                Tk::PathSep => self.out.push('.'),
                _ => self.out.push_str(&t.text),
            }
            self.at_line_start = false;
            prev_hi = Some(t.span.hi as usize);
            k += 1;
        }
    }

    /// Does the body starting at `from` begin with an element, `name {` or
    /// `a::b::Name {`?
    fn tree_first(toks: &[Token], from: usize, to: usize) -> bool {
        let sig: Vec<usize> = (from..to).filter(|&i| !toks[i].is_comment()).collect();
        let mut i = 0;
        if sig.get(i).map_or(true, |&k| toks[k].kind != Tk::Ident || crate::rules::is_keyword(&toks[k].text) || BLOCK_KEYWORDS.contains(&toks[k].text.as_str())) {
            return false;
        }
        i += 1;
        while i + 1 < sig.len() && toks[sig[i]].kind == Tk::PathSep && toks[sig[i + 1]].kind == Tk::Ident {
            i += 2;
        }
        sig.get(i).map_or(false, |&k| toks[k].kind == Tk::Open('{'))
    }

    /// Rule 6, converter side: the items of a brace tree, one per line.
    /// `name { .. }` -> `name:` with the body beneath; `name: value,` ->
    /// `name = value` with a non-literal value in a hole; `for`/`if`/`else`
    /// headers copied with the token substitutions and their bodies as
    /// trees; a string, a `{ .. }` hole or a `..spread` copied, the hole's
    /// contents converted.
    fn tree_body(&mut self, toks: &[Token], cl: &Classes, from: usize, to: usize) {
        let mut i = from;
        while i < to {
            let t = &toks[i];
            if t.is_comment() {
                self.newline();
                self.word(&t.text, false);
                self.newline();
                i += 1;
                continue;
            }
            if t.kind == Tk::Comma {
                i += 1;
                continue;
            }
            self.newline();
            self.indent();
            // A `{ .. }` child hole.
            if t.kind == Tk::Open('{') {
                let c = cl.partner[i];
                self.out.push('{');
                self.at_line_start = false;
                self.run(toks, cl, i + 1, c, Kind::Stmts);
                while self.out.ends_with(' ') {
                    self.out.pop();
                }
                self.out.push('}');
                i = c + 1;
                continue;
            }
            // A string child.
            if t.kind == Tk::Str {
                self.out.push_str(&t.text);
                i += 1;
                continue;
            }
            // `..spread`
            if t.kind == Tk::DotDot {
                let e = Self::tree_item_end(toks, i, to);
                self.matcher_tokens(toks, i, e);
                i = e;
                continue;
            }
            // A header: everything up to the `{` of a block at depth zero, or
            // an attribute `name: value` up to its comma.
            let mut d = 0i32;
            let mut k = i;
            let mut brace: Option<usize> = None;
            let mut colon: Option<usize> = None;
            while k < to {
                match toks[k].kind {
                    Tk::Open('{') if d == 0 => {
                        brace = Some(k);
                        break;
                    }
                    Tk::Open(_) => d += 1,
                    Tk::Close(_) => d -= 1,
                    Tk::Colon if d == 0 && colon.is_none() && k == i + 1 => colon = Some(k),
                    Tk::Comma if d == 0 => break,
                    _ => {}
                }
                k += 1;
            }
            if let (Some(c), None) = (colon, brace.filter(|&b| b < Self::tree_item_end(toks, i, to))) {
                // `name: value,` -> `name = value`
                self.out.push_str(&toks[i].text);
                self.out.push_str(" = ");
                let e = Self::tree_item_end(toks, c + 1, to);
                let v = &toks[c + 1];
                let literal = matches!(v.kind, Tk::Str | Tk::Int | Tk::Float | Tk::Char) && e == c + 2;
                if literal {
                    self.out.push_str(&v.text);
                } else {
                    self.out.push('{');
                    self.run(toks, cl, c + 1, e, Kind::Stmts);
                    while self.out.ends_with(' ') {
                        self.out.pop();
                    }
                    self.out.push('}');
                }
                self.at_line_start = false;
                i = e;
                continue;
            }
            match brace {
                Some(b) => {
                    // `header { .. }`: the header with the substitutions
                    // (rule 3), then `:` and the body as a tree.
                    let c = cl.partner[b];
                    self.matcher_tokens(toks, i, b);
                    self.out.push(':');
                    self.at_line_start = false;
                    if toks[b + 1..c].iter().all(|t| t.is_comment()) {
                        // An empty element keeps Rust's braces: a block must
                        // have a body.
                        while self.out.ends_with(':') {
                            self.out.pop();
                        }
                        self.out.push_str(" {}");
                    } else {
                        self.newline();
                        self.level += 1;
                        self.tree_body(toks, cl, b + 1, c);
                        self.level -= 1;
                    }
                    i = c + 1;
                }
                None => {
                    // Something else at this level: copied to the comma.
                    let e = Self::tree_item_end(toks, i, to);
                    self.matcher_tokens(toks, i, e);
                    i = e;
                }
            }
        }
        self.newline();
    }

    /// The end of a tree item starting at `from`: the next comma at depth
    /// zero, or the end.
    fn tree_item_end(toks: &[Token], from: usize, to: usize) -> usize {
        let mut d = 0i32;
        for k in from..to {
            match toks[k].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                Tk::Comma if d == 0 => return k,
                _ => {}
            }
        }
        to
    }

    /// Where a bare attribute value that starts at `from` ends.
    fn attr_value_end(toks: &[Token], from: usize, to: usize) -> usize {
        let mut d = 0i32;
        let mut i = from;
        while i < to {
            let t = &toks[i];
            match t.kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                _ if d == 0 => {
                    // The tag closes.
                    if t.kind == Tk::Gt || (t.kind == Tk::Punct && t.text == "/" && toks.get(i + 1).map_or(false, |n| n.kind == Tk::Gt)) {
                        return i;
                    }
                    // The next attribute: `name=`, `ns:name=` or `aria-label=`.
                    if t.kind == Tk::Ident && i > from {
                        let mut j = i + 1;
                        while toks.get(j).map_or(false, |n| n.kind == Tk::Punct && n.text == "-")
                            && toks.get(j + 1).map_or(false, |n| n.kind == Tk::Ident)
                        {
                            j += 2;
                        }
                        let eq_next = toks.get(j).map_or(false, |n| n.kind == Tk::Eq)
                            || (toks.get(i + 1).map_or(false, |n| n.kind == Tk::Colon)
                                && toks.get(i + 2).map_or(false, |n| n.kind == Tk::Ident)
                                && toks.get(i + 3).map_or(false, |n| n.kind == Tk::Eq));
                        let after_op = toks[i - 1].kind == Tk::Punct || toks[i - 1].kind == Tk::Dot || toks[i - 1].kind == Tk::PathSep || toks[i - 1].kind == Tk::Eq;
                        if eq_next && !after_op {
                            return i;
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
        to
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
            if b.kind == Tk::Punct && (b.text == "|" || b.text == "&" || b.text == "!") {
                return false;
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
                Tk::Ident if d == 0 && (toks[j].is_kw("let") || toks[j].is_kw("for")) => {
                    binder = true;
                    break;
                }
                _ => {}
            }
        }
        if !binder {
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
                            continue;
                        }
                        d -= 1;
                    }
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
                    if end < to && matches!(toks[end].kind, Tk::Dot | Tk::LArrow) && toks[end + 1..to].iter().any(|n| n.kind == Tk::Ident) {
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
                    if ae < to && toks[ae].kind == Tk::Open('(') {
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
                                let brace = brace || tuple;
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
                        let decl = kind == Kind::Commas && {
                            let words: Vec<&str> = line.split_whitespace().collect();
                            words.iter().any(|w| matches!(*w, "struct" | "enum" | "union"))
                                || (self.in_enum_body && words.len() == 1)
                        };
                        if !self.out.ends_with("=>") && !decl {
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
                        let first_inside = toks[i + 1..close.min(to)].iter().find(|t| !t.is_comment());
                        let _ = first_inside;
                        let is_hsx = macro_body && close != usize::MAX && close < to && crate::juxt::is_markup(toks, i + 1, close);
                        // Inside Rust braces no layout block can open: the
                        // markup keeps its braces, its holes and attribute
                        // values still converted (rule 1 in brace form).
                        if is_hsx && self.brace_depth > 0 && close != usize::MAX && close < to {
                            self.word("{", true);
                            self.level += 1;
                            self.hsx_body(toks, cl, i, close);
                            self.level -= 1;
                            self.newline();
                            self.indent();
                            self.out.push('}');
                            self.at_line_start = false;
                            i = close + 1;
                            prev = Some(&toks[close]);
                            continue;
                        }
                        // `rsx! { div { .. } }`: a brace tree, its first item
                        // an element `name {` (rule 6).
                        let is_tree = macro_body && close != usize::MAX && close < to && Self::tree_first(toks, i + 1, close);
                        if is_tree {
                            self.word("do:", true);
                            self.newline();
                            self.level += 1;
                            self.tree_body(toks, cl, i + 1, close);
                            self.level -= 1;
                            self.newline();
                            i = close + 1;
                            prev = None;
                            continue;
                        }
                        if is_hsx && close != usize::MAX && close < to {
                            // `view! { <markup/> }` -> `view! do:` with the markup
                            // beneath (rule 1): markup kept line for line, every
                            // hole and every attribute value converted as Harsh.
                            self.word("do:", true);
                            self.newline();
                            self.level += 1;
                            self.hsx_body(toks, cl, i, close);
                            self.level -= 1;
                            self.newline();
                            i = close + 1;
                            prev = None;
                            continue;
                        }
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
                            self.out.push(' ');
                            i = close + 1;
                            prev = Some(&toks[close]);
                            continue;
                        }
                        if close != usize::MAX && close < to && !self.in_enum_body && Self::is_struct_literal(toks, i, close) {
                            // Inside `[ .. ]` a literal is isolated in parens
                            // (`vec! [(Point: x = 1)]`): an inline block reads
                            // to its `)`, and brackets are Rust's.
                            let in_brackets = innermost_open_is_bracket(toks, i)
                                || innermost_open_is_tuple(toks, i);
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
                    _ => self.word("}", true),
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
                    let mid_statement = (toks[from..i].iter().any(|n| !n.is_comment() && n.kind != Tk::Hash)
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

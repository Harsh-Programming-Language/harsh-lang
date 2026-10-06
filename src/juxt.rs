// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Juxtaposed application.
//!
//! `f a b` is a call with two arguments. Parentheses isolate a single argument
//! and never group several, so `(a, b)` is always a tuple.
//!
//! The whole difficulty is knowing where expressions are. Adjacent identifiers
//! mean application in expression position, but something else entirely in a
//! type (`&mut Formatter`), a pattern (`Some(d)`), or a declaration
//! (`let mut sum`). So rather than transform everywhere and carve out
//! exceptions, this pass computes the expression regions of a line and works
//! only inside them.

use crate::lex::{Span, Tk, Token};

#[derive(Debug)]
pub struct JuxtError {
    pub msg: String,
    pub span: Span,
}

/// What to do at a given token index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Jx {
    /// Write `(` before this token.
    Open,
    /// Write `, ` before this token.
    Comma,
    /// Write `)` after this token.
    Close,
    /// Drop this token: an isolating paren whose job the argument list now does.
    Drop,
    /// Close the argument list *after* the block that follows, because the last
    /// argument is a closure whose body is that block.
    CloseAfterBlock,
}

/// Keywords that cannot begin an application, because they introduce
/// something else.
pub const NOT_CALLABLE: [&str; 33] = [
    "let", "mut", "ref", "if", "else", "while", "for", "in", "loop", "match", "return", "break",
    "continue", "as", "where", "impl", "dyn", "move", "unsafe", "async", "await", "fn", "pub",
    "do", "const", "static", "type", "struct", "enum", "trait", "mod", "use", "extern",
];

/// A closure parameter list: `|a, b|`, or `||` for none.
fn closure_params_end(toks: &[Token], i: usize, end: usize) -> Option<usize> {
    // `async:` and `async move:` head a block the way a closure's parameters
    // do, so `spawn async:` with the body beneath is `spawn(async { … })`.
    // Only when the head ends the region -- an `async fn` is not this.
    if i < end && toks[i].is_kw("async") {
        let k = if i + 1 < end && toks[i + 1].is_kw("move") { i + 2 } else { i + 1 };
        return if k >= end { Some(k) } else { None };
    }
    // `move |x|` and `move ||`: the keyword is part of the closure's head.
    if i + 1 < end && toks[i].is_kw("move") {
        return closure_params_end(toks, i + 1, end);
    }
    if i >= end || toks[i].kind != Tk::Punct {
        return None;
    }
    if toks[i].text == "||" {
        return Some(i + 1);
    }
    if toks[i].text != "|" {
        return None;
    }
    let mut k = i + 1;
    let mut d = 0i32;
    while k < end {
        match toks[k].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Punct if d == 0 && toks[k].text == "|" => return Some(k + 1),
            Tk::Semi => return None,
            _ => {}
        }
        k += 1;
    }
    None
}

fn is_atom_start(toks: &[Token], i: usize, end: usize) -> bool {
    let t = &toks[i];
    match t.kind {
        Tk::Ident => !NOT_CALLABLE.contains(&t.text.as_str()),
        Tk::Int | Tk::Float | Tk::Str | Tk::Char => true,
        Tk::Open('(') => true,
        // `..n`: a range with no start is an atom when an atom follows.
        Tk::DotDot => i + 1 < end && toks[i + 1].kind != Tk::DotDot && is_atom_start(toks, i + 1, end),
        // `$x` -- a macro metavariable -- and `$( .. )*`, a repetition, are
        // atoms in a transcriber. A `$` that reaches this pass is always one
        // of those: the apply-to-nothing suffix became `()` before it.
        Tk::Punct if t.text == "$" && !t.synthetic => metavar_end(toks, i, end).is_some(),
        _ => false,
    }
}

/// Index just past a metavariable `$x` or a repetition `$( .. )*` starting at
/// `i`. The sigil is tight against what follows; `$( .. )` may take one
/// repetition operator, `*`, `+` or `?`.
fn metavar_end(toks: &[Token], i: usize, end: usize) -> Option<usize> {
    if i + 1 >= end || toks[i].text != "$" || toks[i].kind != Tk::Punct || toks[i + 1].span.lo != toks[i].span.hi {
        return None;
    }
    match toks[i + 1].kind {
        Tk::Ident => Some(i + 2),
        Tk::Open('(') => {
            let mut d = 0i32;
            let mut k = i + 1;
            while k < end {
                match toks[k].kind {
                    Tk::Open(_) => d += 1,
                    Tk::Close(_) => {
                        d -= 1;
                        if d == 0 {
                            let op = k + 1 < end
                                && toks[k + 1].kind == Tk::Punct
                                && matches!(toks[k + 1].text.as_str(), "*" | "+" | "?");
                            return Some(if op { k + 2 } else { k + 1 });
                        }
                    }
                    _ => {}
                }
                k += 1;
            }
            None
        }
        _ => None,
    }
}

/// Index just past a macro's brace body starting at `i`: `m! { .. }`, with
/// the bang immediately before the brace, or `macro_rules! name { .. }`.
/// (A brace body is Harsh like any other brace -- the earlier rule that
/// nothing inside juxtaposes was withdrawn -- so this is kept for callers
/// that need the body's extent.)
#[allow(dead_code)]
pub fn macro_brace_end(toks: &[Token], i: usize, end: usize) -> Option<usize> {
    if i >= end || toks[i].kind != Tk::Open('{') || i == 0 {
        return None;
    }
    let bang = |k: usize| toks[k].kind == Tk::Punct && toks[k].text == "!" && k > 0 && toks[k - 1].kind == Tk::Ident;
    let is_macro = bang(i - 1) || (i >= 3 && toks[i - 1].kind == Tk::Ident && bang(i - 2) && toks[i - 3].is_kw("macro_rules"));
    if !is_macro {
        return None;
    }
    let mut d = 0i32;
    let mut k = i;
    while k < end {
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

/// Index just past the atom starting at `i`, or None.
///
/// An atom is an identifier (optionally a path `a.b.c`, optionally with
/// generics, optionally a macro `name!`), a literal, or a parenthesised group.
/// Ranges are atoms (the user, 2026-10-05): a range whose ends are atoms is
/// itself one -- `0..3`, `a..=b`, `..n`, `start..` -- so `f 0..3` applies,
/// `f(0..3)`. Ends that are not atoms (`a + 1..n`, `-40..=0`) leave `..` an
/// operator, isolated as any argument: `f (a + 1..n)`.
fn atom_end(toks: &[Token], i: usize, end: usize) -> Option<usize> {
    // `..n` / `..=n`: a range with no start.
    if i < end && toks[i].kind == Tk::DotDot {
        let e = atom_end_base(toks, i + 1, end)?;
        return Some(e);
    }
    let base = atom_end_base(toks, i, end)?;
    if base < end && toks[base].kind == Tk::DotDot {
        if base + 1 >= end {
            return Some(base + 1); // `start..`, ending the arguments
        }
        if let Some(e) = atom_end_base(toks, base + 1, end) {
            return Some(e);
        }
    }
    Some(base)
}

fn atom_end_base(toks: &[Token], i: usize, end: usize) -> Option<usize> {
    // A closure is an argument atom only where it is unambiguous: when its
    // parameter list ends the region, as in a block header `map |n|:`. A
    // bitwise or cannot be followed by the block colon, so nothing else can
    // match. Anywhere else `|` and `||` stay operators -- `a || b` and
    // `"struct" | "enum"` must not be read as closures.
    if let Some(p) = closure_params_end(toks, i, end) {
        return if p >= end { Some(p) } else { None };
    }
    if i >= end || !is_atom_start(toks, i, end) {
        return None;
    }
    let mut k = i;
    match toks[k].kind {
        Tk::Punct => metavar_end(toks, i, end),
        Tk::Open('(') => {
            let mut d = 0i32;
            while k < end {
                match toks[k].kind {
                    Tk::Open(_) => d += 1,
                    Tk::Close(_) => {
                        d -= 1;
                        if d == 0 {
                            return Some(dollar_end(toks, tight_index_end(toks, k + 1, end), end));
                        }
                    }
                    _ => {}
                }
                k += 1;
            }
            None
        }
        Tk::Ident => {
            k += 1;
            // Path segments: `a.b.c`, and Harsh's turbofish `.<T>` as one more
            // step -- `parse_kv.<String> "x"` applies; until 2026-09-22 the
            // atom ended before the `.<` and the argument was left behind
            // (C1 of the edge sheet).
            loop {
                if k + 1 < end && toks[k].kind == Tk::Dot && toks[k + 1].kind == Tk::Ident {
                    k += 2;
                    continue;
                }
                if k + 1 < end && toks[k].kind == Tk::Dot && toks[k + 1].kind == Tk::Lt {
                    let mut d = 0i32;
                    let mut j = k + 1;
                    let mut closed = None;
                    while j < end {
                        match toks[j].kind {
                            Tk::Lt => d += 1,
                            Tk::Gt => d -= 1,
                            Tk::Punct if toks[j].text == ">>" => d -= 2,
                            Tk::Semi | Tk::Open('{') => break,
                            _ => {}
                        }
                        if d <= 0 {
                            if d == 0 {
                                closed = Some(j);
                            }
                            break;
                        }
                        j += 1;
                    }
                    if let Some(j) = closed {
                        k = j + 1;
                        continue;
                    }
                }
                break;
            }
            // No guess about a `<` (the user's rule, 2026-10-04): in an
            // expression only `.<` opens a generic list, extending the atom
            // in the loop above; a plain `<` is mapped as written, and rustc
            // judges. (A block here extended the atom over `Vec<i32>` when a
            // call or a path step followed its `>` -- the old turbofish.)
            // Macro bang. A macro is not a value, so nothing after its bang
            // indexes it: a bracket there, tight or spaced, is its first
            // argument, an array literal -- `vec! [1, 2]` is `vec!([1, 2])`
            // (ruled 2026-09-22, A1). The head ends at the bang.
            if k < end && toks[k].text == "!" {
                return Some(dollar_end(toks, k + 1, end));
            }
            // Tuple indexes: `t.0`, `t.0.1`. A path segment can never start
            // with a digit, so `.0` after a name is always an index and is
            // part of the atom -- `show t.0 t.1` is two arguments.
            while k < end && toks[k].kind == Tk::TupleIdx {
                k += 1;
            }
            Some(dollar_end(toks, tight_index_end(toks, k, end), end))
        }
        _ => Some(k + 1),
    }
}

/// The interiors of the `[..]` groups an atom carries are expression regions
/// of their own: `src[start..off <- min end]` applies `min` inside the index.
fn bracket_interiors(toks: &[Token], a: usize, b: usize, out: &mut Vec<(usize, Jx)>) {
    // Only the atom's own brackets: one inside a parenthesised group is
    // reached when that group is split, and must not be applied twice.
    let mut i = a;
    let mut depth = 0i32;
    while i < b {
        match toks[i].kind {
            Tk::Open('[') if depth == 0 => {
                if let Some(c) = matching_close_idx(toks, i, b) {
                    split_group(toks, i + 1, c, out);
                    i = c + 1;
                    continue;
                }
            }
            Tk::Open(_) => depth += 1,
            Tk::Close(_) => depth -= 1,
            _ => {}
        }
        i += 1;
    }
}

/// A `[..]` written tight against what precedes it is an index and part of
/// the atom: `f arr[1]` is `f(arr[1])`, as `t.0` is part of `t`. Written with
/// a space it is an index too (brackets never apply), but inside an
/// application that spelling is refused rather than read either way -- see
/// `check`. The user's rule, 2026-09-11.
fn tight_index_end(toks: &[Token], mut k: usize, end: usize) -> usize {
    loop {
        if k < end
            && k > 0
            && toks[k].kind == Tk::Open('[')
            && toks[k].span.lo == toks[k - 1].span.hi
        {
            if let Some(c) = matching_close_idx(toks, k, end) {
                k = c + 1;
                while k < end && toks[k].kind == Tk::TupleIdx {
                    k += 1;
                }
                continue;
            }
        }
        return k;
    }
}

/// The `<>` of `f<>`, written tight: `<` against what it marks, `>` against `<`.
pub fn is_each_mark(toks: &[Token], k: usize) -> bool {
    k >= 1
        && k + 1 < toks.len()
        && toks[k].kind == Tk::Lt
        && toks[k + 1].kind == Tk::Gt
        && toks[k].span.hi == toks[k + 1].span.lo
        && toks[k - 1].span.hi == toks[k].span.lo
        && match toks[k - 1].kind {
            Tk::Ident => !crate::rules::is_keyword(&toks[k - 1].text),
            Tk::Close(')') => true,
            _ => false,
        }
}

/// `f<>` -- apply to each. Until `rewrite_each` runs, after the pipes, the
/// mark belongs to the atom it follows, as `$` does: `a |> f64.sqrt<>` pipes
/// into `f64.sqrt<>`, one function.
fn each_end(toks: &[Token], k: usize, end: usize) -> usize {
    if k + 1 < end && is_each_mark(toks, k) { k + 2 } else { k }
}

/// `f$` -- apply to nothing. `rewrite_dollar` has already turned the `$` into
/// a synthetic `()`, and that pair belongs to the atom it follows: `f$` is one
/// atom, `f()`, wherever it stands, so `g f$ 5` is `g(f(), 5)` and `f$ |> g`
/// is `g(f())`. A *source* `()` is never absorbed: it is a group holding the
/// unit value, and `f ()` is `f(())`.
fn dollar_end(toks: &[Token], k: usize, end: usize) -> usize {
    let k = each_end(toks, k, end);
    if k + 1 < end
        && toks[k].dollar
        && toks[k].kind == Tk::Open('(')
        && toks[k + 1].dollar
        && toks[k + 1].kind == Tk::Close(')')
    {
        k + 2
    } else {
        k
    }
}

/// `a[i, j]` -- an index of several axes, as Julia writes it (ruled
/// 2026-09-21). Rust's `Index` takes one value, so the axes are a tuple:
/// `a[1, 2]` is `a[(1, 2)]`, `&a[0..2, ..]` is `&a[(0..2, ..)]`. One brick: *a
/// top-level comma in an index makes a tuple.* What the tuple selects is the
/// indexed type's business -- for `hrs_std`'s matrices an element, or a view.
///
/// A bracket indexes when it follows a value -- a name, `)`, `]`, `.0` --
/// spaced or not, which is the language's rule already. `[where T: A, U: B]`
/// follows a type and is not an index.
pub fn rewrite_comma_index(toks: &[Token]) -> Vec<Token> {
    if toks.iter().any(|t| t.is_kw("macro_rules")) || !toks.iter().any(|t| t.kind == Tk::Comma) {
        return toks.to_vec();
    }
    // Token index -> the paren to write before it.
    let mut before: std::collections::HashMap<usize, Tk> = std::collections::HashMap::new();
    for k in 1..toks.len() {
        let follows_value = match toks[k - 1].kind {
            Tk::Ident => !crate::rules::is_keyword(&toks[k - 1].text),
            Tk::Close(')') | Tk::Close(']') | Tk::TupleIdx => true,
            _ => false,
        };
        let indexes = toks[k].kind == Tk::Open('[')
            && follows_value
            && !toks.get(k + 1).map_or(false, |t| t.is_kw("where"));
        if !indexes {
            continue;
        }
        let (mut d, mut comma, mut close) = (0i32, false, None);
        for j in k..toks.len() {
            match toks[j].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => {
                    d -= 1;
                    if d == 0 {
                        close = Some(j);
                        break;
                    }
                }
                Tk::Comma if d == 1 => comma = true,
                _ => {}
            }
        }
        if let (true, Some(c)) = (comma, close) {
            before.insert(k + 1, Tk::Open('('));
            before.insert(c, Tk::Close(')'));
        }
    }
    if before.is_empty() {
        return toks.to_vec();
    }
    let mut v = Vec::with_capacity(toks.len() + before.len());
    for (k, t) in toks.iter().enumerate() {
        // `a[]`-with-a-comma cannot be: `k + 1 == c` needs an empty bracket.
        if let Some(kind) = before.get(&k) {
            let open = *kind == Tk::Open('(');
            let at = if open { toks[k - 1].span.hi } else { t.span.lo };
            v.push(Token {
                ctx: t.ctx,
                kind: kind.clone(),
                span: Span { lo: at, hi: at },
                text: if open { "(" } else { ")" }.to_string(),
                line_start: None,
                line: t.line,
                synthetic: true,
                dollar: false,
                tilde: false,
            });
        }
        v.push(t.clone());
    }
    v
}

/// A `use` declaration: `use a.b.*`, `pub use …`, `pub(crate) use …`. The one
/// place where `.*` is a glob and not an operator.
fn is_use_line(toks: &[Token]) -> bool {
    toks.iter().take(6).any(|t| t.is_kw("use"))
}

/// Julia's dotted operators: `a .* b`, `a .+ b`, `a .- b`, `a ./ b` -- the
/// operation applied element by element, shapes stretched as Julia stretches
/// them (ruled 2026-09-21).
///
/// A pure substitution, needing neither a type nor an expression parser:
/// `.*` becomes `* hrs_std.DOT *`. `a * DOT` is half an operation and `* b`
/// completes it; being two operators of one level, Rust's precedence and
/// associativity are already Julia's. `.` is otherwise the path separator,
/// which an operator can never follow -- except the glob of a `use`, so a
/// `use` declaration is left alone. Spacing does not matter: `a.*b` is the
/// same, as in Julia.
pub fn rewrite_dotted(toks: &[Token]) -> Vec<Token> {
    if is_use_line(toks) || toks.iter().any(|t| t.is_kw("macro_rules")) {
        return toks.to_vec();
    }
    let mut v: Vec<Token> = Vec::with_capacity(toks.len());
    for (i, t) in toks.iter().enumerate() {
        let op = toks.get(i + 1).filter(|n| {
            t.kind == Tk::Dot
                && n.kind == Tk::Punct
                && n.span.lo == t.span.hi
                && matches!(n.text.as_str(), "*" | "+" | "-" | "/")
        });
        let Some(op) = op else {
            v.push(t.clone());
            continue;
        };
        // The whole operator's span, so a diagnostic lands on `.*`.
        let span = Span { lo: t.span.lo, hi: op.span.hi };
        for (kind, text) in [
            (Tk::Punct, op.text.as_str()),
            (Tk::Ident, "hrs_std"),
            (Tk::Dot, "."),
            (Tk::Ident, "DOT"),
        ] {
            v.push(Token {
                ctx: t.ctx,
                kind,
                span,
                text: text.to_string(),
                line_start: None,
                line: t.line,
                synthetic: true,
                dollar: false,
                tilde: false,
            });
        }
        // The operator itself follows, untouched, and completes the pair.
    }
    v
}

/// `f<>` is *apply to each*: `f64.sqrt<> a`, `f64.powf<> a 2.0`,
/// `(|x| x * x)<> a`, `a |> f64.sqrt<>` -- Julia's `f.(a)` (the user's mark,
/// 2026-09-21). A suffix, tight like `$`, `!` and `~`, and like them it says
/// how the name is applied.
///
/// It becomes the head of a macro call, `hrs_std.each! f a b`, and the rest is
/// the ordinary application rule: the macro borrows every argument, because
/// applying a function to each element consumes nothing, and picks `each1` or
/// `each2` by their number. Runs after the pipes, which treat `f<>` as the one
/// function it is.
///
/// Rust's empty generic list (`f::<>(x)`, `P<>`) means nothing, so Harsh takes
/// the spelling and the converter drops Rust's -- the one place Harsh is not a
/// superset, recorded as such.
pub fn rewrite_each(toks: &[Token]) -> Result<Vec<Token>, JuxtError> {
    if toks.iter().any(|t| t.is_kw("macro_rules")) {
        return Ok(toks.to_vec());
    }
    let mut v: Vec<Token> = Vec::with_capacity(toks.len() + 4);
    let mut i = 0;
    while i < toks.len() {
        let t = &toks[i];
        let pair = t.kind == Tk::Lt
            && toks.get(i + 1).map_or(false, |n| n.kind == Tk::Gt && n.span.lo == t.span.hi);
        if !pair {
            v.push(t.clone());
            i += 1;
            continue;
        }
        if !is_each_mark(toks, i) {
            let name = i.checked_sub(1).map(|j| &toks[j]).filter(|p| p.kind == Tk::Ident).map(|p| p.text.clone()).unwrap_or_else(|| "f".into());
            return Err(JuxtError {
                msg: format!("`<>` applies a function to each element and is written tight against it: `{name}<> a`"),
                span: Span { lo: t.span.lo, hi: toks[i + 1].span.hi },
            });
        }
        // Where the marked function begins, in what has been written so far:
        // a parenthesised group, or a path `a.b.c`.
        let mut h = v.len() - 1;
        if v[h].kind == Tk::Close(')') {
            let mut d = 0i32;
            loop {
                match v[h].kind {
                    Tk::Close(_) => d += 1,
                    Tk::Open(_) => d -= 1,
                    _ => {}
                }
                if d == 0 || h == 0 {
                    break;
                }
                h -= 1;
            }
        } else {
            while h >= 2 && v[h - 1].kind == Tk::Dot && v[h - 2].kind == Tk::Ident {
                h -= 2;
            }
        }
        // Zero-width, and one short of the function: the inserted words are
        // tight among themselves (`each!`) but a space away from a marked
        // group, as `m! (..)` must be.
        let lo = v[h].span.lo.saturating_sub(1);
        let at = Span { lo, hi: lo };
        let head_starts_line = v[h].line_start.take();
        let mut head: Vec<Token> = Vec::with_capacity(4);
        for (kind, text) in [(Tk::Ident, "hrs_std"), (Tk::Dot, "."), (Tk::Ident, "each"), (Tk::Punct, "!")] {
            head.push(Token {
                ctx: t.ctx,
                kind,
                span: at,
                text: text.to_string(),
                line_start: None,
                line: t.line,
                synthetic: true,
                dollar: false,
                tilde: false,
            });
        }
        // If the function began its line, the inserted head begins it now.
        head[0].line_start = head_starts_line;
        v.splice(h..h, head);
        // The mark is gone from the tokens; its two characters now belong to
        // the function's last token, so they are not copied out as a gap and
        // a diagnostic on the function underlines `f<>`.
        if let Some(last) = v.last_mut() {
            last.span.hi = toks[i + 1].span.hi;
        }
        i += 2;
    }
    Ok(v)
}

/// `$` is *apply to nothing*: `main$`, `s <- len$`, `String.new$`, `m!$`,
/// `(add 10)$`, and `fn main$:` in a declaration. It is a suffix, tight like
/// the `!` of a macro, and it becomes a synthetic `()` before anything else
/// sees the line -- the pipes, the application rule and the parameter-group
/// rule then all handle it as the empty parenthesised group it stands for.
/// `()` written in source is therefore only ever the unit value.
///
/// A macro definition's transcriber uses `$` for metavariables; that line is
/// left untouched, as it is by the pipes.
pub fn rewrite_dollar(toks: &[Token]) -> Result<Vec<Token>, JuxtError> {
    if toks.iter().any(|t| t.is_kw("macro_rules")) {
        return Ok(toks.to_vec());
    }
    let mut v: Vec<Token> = Vec::with_capacity(toks.len() + 2);
    for (i, t) in toks.iter().enumerate() {
        // A `$` from a macro expansion is synthetic -- every spliced token is,
        // since its span points into the definition -- but it is still a call
        // marker and `f$` must still become `f()` (2026-09-18).
        if t.text != "$" || t.kind != Tk::Punct {
            v.push(t.clone());
            continue;
        }
        let prev = i.checked_sub(1).map(|j| &toks[j]);
        let after_name = prev.map_or(false, |p| {
            p.span.hi == t.span.lo
                && match p.kind {
                    Tk::Ident => !crate::rules::is_keyword(&p.text),
                    Tk::Close(')') | Tk::Gt | Tk::TupleIdx => true,
                    Tk::Punct => p.text == "!" || p.text == ">>",
                    _ => false,
                }
        });
        // `$a`, `$(` -- a metavariable or repetition in a macro transcriber
        // that reached here inside a brace body: a prefix, not a suffix.
        let is_prefix = toks.get(i + 1).map_or(false, |n| {
            n.span.lo == t.span.hi && matches!(n.kind, Tk::Ident | Tk::Open('('))
        });
        if !after_name && is_prefix {
            v.push(t.clone());
            continue;
        }
        if !after_name {
            let name = prev.filter(|p| p.kind == Tk::Ident).map(|p| p.text.clone()).unwrap_or_else(|| "f".into());
            return Err(JuxtError {
                msg: format!("`$` applies a name to nothing and is written tight against it: `{name}$`"),
                span: t.span,
            });
        }
        for (kind, text) in [(Tk::Open('('), "("), (Tk::Close(')'), ")")] {
            v.push(Token {
                // The `$`'s own context: a `$` a macro's transcriber wrote
                // belongs to that expansion, and the emitter must know it.
                ctx: t.ctx,
                kind,
                span: t.span,
                text: text.to_string(),
                line_start: None,
                line: t.line,
                synthetic: true,
                dollar: true,
                tilde: false,
            });
        }
    }
    Ok(v)
}

/// Compute the application fixups for the half-open expression region
/// `[from, to)`, recursing into parenthesised groups.
fn apply_region(toks: &[Token], from: usize, to: usize, out: &mut Vec<(usize, Jx)>) {
    let mut i = from;
    while i < to {
        rep_fixups(toks, i, to, out);
        let Some(head_end) = atom_end(toks, i, to) else {
            i += 1;
            continue;
        };
        // A repetition `$( .. )*` as a head: its body is Harsh -- statements
        // or expressions -- and is applied to on its own.
        if toks[i].kind == Tk::Punct && toks[i].text == "$" && i + 1 < head_end && toks[i + 1].kind == Tk::Open('(') {
            let close = (i + 2..head_end).rev().find(|&k| toks[k].kind == Tk::Close(')')).unwrap_or(head_end - 1);
            let mut seg = i + 2;
            let mut d = 0i32;
            for k in i + 2..close {
                match toks[k].kind {
                    Tk::Open(_) => d += 1,
                    Tk::Close(_) => d -= 1,
                    Tk::Semi if d == 0 => {
                        apply_region(toks, seg, k, out);
                        seg = k + 1;
                    }
                    _ => {}
                }
            }
            if seg < close {
                apply_region(toks, seg, close, out);
            }
            i = head_end;
            continue;
        }
        // Collect the run of argument atoms following the head.
        let mut args: Vec<(usize, usize)> = Vec::new();
        let mut k = head_end;
        // A `(` with no partner before the region ends is the isolating paren
        // of a block-bodied argument -- `fold 0 (|acc, x|:` -- whose `)`
        // arrives with the block's tail. It is the last argument.
        let mut open_last = false;
        bracket_interiors(toks, i, head_end, out);
        let bang_head = head_end > i && toks[head_end - 1].kind == Tk::Punct && toks[head_end - 1].text == "!";
        while k < to {
            // A bracket right after a macro's bang is its first argument, an
            // array literal. Its elements are applied with the other
            // arguments, below, as any argument's are.
            if k == head_end && bang_head && toks[k].kind == Tk::Open('[') {
                if let Some(c) = matching_close_idx(toks, k, to) {
                    args.push((k, c + 1));
                    k = c + 1;
                    continue;
                }
            }
            // After a constructor, a single argument needs no parentheses
            // (the user, 2026-10-05): `Some &x`, `Some &mut x`, `Some mut x`,
            // `Some ref x`. A constructor is never an operand of bit-and, and
            // `mut` / `ref` are never operators.
            if k == head_end && args.is_empty() && is_constructor_head(toks, i, head_end) {
                if let Some(e) = prefixed_single_arg(toks, k, to) {
                    args.push((k, e));
                    k = e;
                    break;
                }
            }
            match atom_end(toks, k, to) {
                Some(e) => {
                    rep_fixups(toks, k, to, out);
                    bracket_interiors(toks, k, e, out);
                    args.push((k, e));
                    k = e;
                }
                None => {
                    if toks[k].kind == Tk::Open('(') && unmatched_to_end(toks, k, to) {
                        args.push((k, to));
                        open_last = true;
                        k = to;
                    }
                    break;
                }
            }
        }
        // The head may itself be a group -- `(add 10) 7` applies the closure
        // that `add 10` returns.
        if toks[i].kind == Tk::Open('(') {
            // A call isolated so that a tuple index applies to its result,
            // `(f src).0`: Harsh's isolation, which Rust's call syntax does
            // not need -- `f(src).0` (2026-10-05). An operation keeps them:
            // `(a + b).0` needs them in Rust too.
            if let Some(c) = matching_close_idx(toks, i, head_end) {
                let indexed = toks.get(c + 1).map_or(false, |t| t.kind == Tk::TupleIdx);
                let call = c > i + 1 && toks[i + 1].kind == Tk::Ident && atom_end(toks, i + 1, c).map_or(false, |e| e < c && is_application(toks, e, c));
                if indexed && call {
                    out.push((i, Jx::Drop));
                    out.push((c, Jx::Drop));
                }
            }
            split_group(toks, i + 1, head_end - 1, out);
        }
        if args.is_empty() {
            i = head_end.max(i + 1);
            continue;
        }
        // A single parenthesised argument already delimits itself -- unless it
        // has top-level commas, in which case it is a tuple and still needs to
        // be wrapped: `f (a, b)` is `f((a, b))`, one argument.
        // The unit value `()` is a group too, and the one that must be wrapped
        // however many arguments there are: `f ()` is `f(())`.
        let is_unit = |a: usize, b: usize| toks[a].kind == Tk::Open('(') && b - a == 2;
        let single_group = args.len() == 1
            && toks[args[0].0].kind == Tk::Open('(')
            && !is_unit(args[0].0, args[0].1)
            && !has_top_comma(toks, args[0].0 + 1, args[0].1 - 1);
        if open_last {
            // `f a (|x|:` -> `f(a, |x| {` .. `})`: the list opens before the
            // first argument, the isolating `(` goes, and the tail's `)`
            // closes the list. The earlier arguments' isolating parens are
            // redundant here as in any list of two or more.
            out.push((args[0].0, Jx::Open));
            for w in args.windows(2) {
                out.push((w[1].0, Jx::Comma));
            }
            // An open group that is a tuple -- `push ((lb - 1, if c:` --
            // keeps its `(`; the isolating one goes.
            let last_open = args[args.len() - 1].0;
            if !has_top_comma(toks, last_open + 1, to) {
                out.push((last_open, Jx::Drop));
            } else {
                // The tuple's `)` in the tail is the tuple's own, so the
                // list's `)` is owed after the block: `f (1, (P\ x = 1))` is
                // `f((1, P { x: 1 }))` (ShopBill's S1, 2026-10-05).
                out.push((to - 1, Jx::CloseAfterBlock));
            }
            // The open group's inside is Harsh too: `Some (if toks <- get c:`.
            apply_region(toks, last_open + 1, to, out);
            let is_unit = |a: usize, b: usize| toks[a].kind == Tk::Open('(') && b - a == 2;
            for (a, b) in &args[..args.len() - 1] {
                if toks[*a].kind == Tk::Open('(') && !is_unit(*a, *b) && !has_top_comma(toks, a + 1, b - 1) {
                    out.push((*a, Jx::Drop));
                    out.push((*b - 1, Jx::Drop));
                }
            }
            for (a, b) in &args[..args.len() - 1] {
                if toks[*a].kind == Tk::Open('(') {
                    split_group(toks, a + 1, b - 1, out);
                }
            }
            i = to;
            continue;
        }
        if !single_group {
            out.push((args[0].0, Jx::Open));
            for w in args.windows(2) {
                out.push((w[1].0, Jx::Comma));
            }
            // A closure whose parameter list ends the region owns the block
            // that follows, so its argument list closes after the `}`.
            let (la, lb) = args[args.len() - 1];
            let owns_block = closure_params_end(toks, la, lb) == Some(lb) && lb == to;
            out.push((lb - 1, if owns_block { Jx::CloseAfterBlock } else { Jx::Close }));
        }
        // With two or more arguments the inserted list already separates them,
        // so an argument's isolating parens are redundant. A group with
        // top-level commas is a tuple and keeps them.
        if args.len() > 1 {
            for (a, b) in &args {
                if toks[*a].kind == Tk::Open('(') && !is_unit(*a, *b) && !has_top_comma(toks, a + 1, b - 1) {
                    out.push((*a, Jx::Drop));
                    out.push((*b - 1, Jx::Drop));
                }
            }
        }
        // Each argument is itself an expression region. For a closure, the
        // params are not part of it -- only the body.
        for (a, b) in &args {
            if toks[*a].kind == Tk::Open('(') {
                split_group(toks, a + 1, b - 1, out);
            } else if let Some(p) = closure_params_end(toks, *a, *b) {
                if p < *b {
                    apply_region(toks, p, *b, out);
                }
            } else {
                apply_region(toks, *a, *b, out);
            }
        }
        i = k;
    }
}

/// A repetition in an expression, `$( .. )*`. When its body is one
/// parenthesised fragment, `$( ($x) )*`, the fragment is one argument and the
/// repetition is a comma list: the parens go and a `,` precedes the operator,
/// `$($x),*`. A repetition of bare tokens -- `$($arg)*` forwarding `tt`s --
/// is Rust's and is copied through. The same brick as rule 4's matcher,
/// read from the transcriber's side. `?` admits no separator.
fn rep_fixups(toks: &[Token], i: usize, to: usize, out: &mut Vec<(usize, Jx)>) {
    let Some(e) = metavar_end(toks, i, to) else { return };
    if toks[i + 1].kind != Tk::Open('(') {
        return;
    }
    let has_op = toks[e - 1].kind == Tk::Punct;
    let close = if has_op { e - 2 } else { e - 1 };
    // Body is exactly one group?
    let a = i + 2;
    if a >= close || toks[a].kind != Tk::Open('(') {
        return;
    }
    let mut d = 0i32;
    let mut b = None;
    for k in a..close {
        match toks[k].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => {
                d -= 1;
                if d == 0 {
                    b = Some(k);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(b) = b else { return };
    if b + 1 != close {
        return;
    }
    out.push((a, Jx::Drop));
    out.push((b, Jx::Drop));
    if has_op && toks[e - 1].text != "?" {
        out.push((e - 1, Jx::Comma));
    }
}

/// The `(` at `i` has no partner in `[i, to)`.
fn unmatched_to_end(toks: &[Token], i: usize, to: usize) -> bool {
    let mut d = 0i32;
    for k in i..to {
        match toks[k].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => {
                d -= 1;
                if d == 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

/// A comma at depth zero -- outside any bracket, and outside a closure's
/// prototype `|a, b|`, whose commas separate parameters, not tuple elements.
fn has_top_comma(toks: &[Token], from: usize, to: usize) -> bool {
    let mut d = 0i32;
    let mut in_proto = false;
    for i in from..to {
        match toks[i].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Punct if d == 0 && toks[i].text == "|" => in_proto = !in_proto,
            // A `\` at depth zero: the commas after it are a struct's
            // field list, not a tuple's.
            Tk::Backslash if d == 0 => return false,
            Tk::Comma if d == 0 && !in_proto => return true,
            _ => {}
        }
    }
    false
}

/// Inside `( … )`, split at top-level commas and treat each part as its own
/// expression region. A group with commas is a tuple; its elements are still
/// expressions.
fn split_group(toks: &[Token], from: usize, to: usize, out: &mut Vec<(usize, Jx)>) {
    let mut d = 0i32;
    let mut start = from;
    for i in from..to {
        match toks[i].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Comma if d == 0 => {
                apply_region(toks, start, i, out);
                start = i + 1;
            }
            _ => {}
        }
    }
    apply_region(toks, start, to, out);
}

/// Expression regions of one logical line, given whether it sits in a
/// statement block.
pub fn regions(toks: &[Token], stmt_block: bool, is_header: bool) -> Vec<(usize, usize)> {
    let sig: Vec<usize> = (0..toks.len()).filter(|&i| !toks[i].is_comment()).collect();
    if sig.is_empty() {
        return Vec::new();
    }
    let end = *sig.last().unwrap() + 1;
    let first = toks[sig[0]].text.as_str();

    // An attribute is an application inside its brackets: `#[derive Debug
    // Clone]` is `derive` applied to two names; `#[cfg (feature = "x")]`
    // is `cfg` applied to one isolated argument.
    if toks[sig[0]].kind == Tk::Hash {
        let mut out = Vec::new();
        let mut i = sig[0];
        while i < end {
            if toks[i].kind == Tk::Open('[') {
                if let Some(c) = matching_close_idx(toks, i, end) {
                    out.push((i + 1, c));
                    i = c + 1;
                    continue;
                }
            }
            i += 1;
        }
        return out;
    }

    // "Top level" is the depth the line ends at: a block keyword opened
    // inside a group left open -- `Some (if c:` -- governs the header from
    // there (parens are transparent to layout).
    let end_depth: i32 = {
        let mut d = 0i32;
        for &i in &sig {
            match toks[i].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                _ => {}
            }
        }
        d.max(0)
    };
    let top_level = |pred: &dyn Fn(&Token) -> bool| -> Option<usize> {
        let mut d = 0i32;
        for &i in &sig {
            match toks[i].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                _ if d == end_depth && pred(&toks[i]) => return Some(i),
                _ => {}
            }
        }
        None
    };

    // `union U\\ a: u32, b: f32` inline: `union` is a contextual keyword, so
    // it is not refused as a callee the way `struct` is, and `union U` was
    // read as an application and emitted `union(U) {` (found 2026-09-20).
    // It is a declaration exactly when a name follows and then the `\\`, the
    // generics, or nothing; `union a b` stays a call to a function so named.
    {
        let mut k = 0;
        while k < sig.len() && crate::rules::MODIFIERS.contains(&toks[sig[k]].text.as_str()) && !toks[sig[k]].is_kw("const") {
            k += 1;
        }
        if k + 1 < sig.len() && toks[sig[k]].text == "union" && toks[sig[k + 1]].kind == Tk::Ident {
            let after = sig.get(k + 2).map(|&i| &toks[i].kind);
            if matches!(after, None | Some(Tk::Backslash) | Some(Tk::Lt)) {
                return Vec::new();
            }
        }
    }
    if is_header {
        // A `fn` header is a signature: only the parameter patterns juxtapose.
        // Every other header juxtaposes whole, because a match arm's pattern,
        // an `if let` pattern and a scrutinee are all ordinary applications.
        let has_fn = sig.iter().any(|&i| toks[i].is_kw("fn"));
        let kw = sig.iter().map(|&i| &toks[i]).find(|t| {
            t.kind == Tk::Ident && matches!(t.text.as_str(), "match" | "if" | "while" | "for")
        });
        // `macro_rules! name:` is a definition, not an application.
        if toks[sig[0]].is_kw("macro_rules") {
            return Vec::new();
        }
        if !has_fn {
            // The region ends at the block-opening colon. A `do` immediately
            // before it is the synonym marker, not part of the expression, so
            // `f || do:` presents `||` as the last atom just as `f ||:` does.
            let mut stop = *sig.last().unwrap();
            if sig.len() >= 2 && toks[sig[sig.len() - 2]].is_kw("do") {
                stop = sig[sig.len() - 2];
            }
            return if sig[0] < stop { vec![(sig[0], stop)] } else { Vec::new() };
        }
        let _ = kw;
        let kw: Option<&Token> = None;
        let Some(kw) = kw else {
            // `let PAT = EXPR else:` has no scrutinee keyword but is still an
            // ordinary statement up to the block-opening colon.
            if toks[sig[0]].is_kw("let") {
                return vec![(sig[0], *sig.last().unwrap())];
            }
            // A `fn` header: parameter *patterns* juxtapose, so
            // `Query params: Query<HelloParams>` becomes `Query(params): ..`.
            // Only the inside of each parameter group is exposed; the types and
            // the signature itself are not.
            if sig.iter().any(|&i| toks[i].is_kw("fn")) {
                let mut out = Vec::new();
                let mut d = 0i32;
                let mut start = None;
                for &i in &sig {
                    match toks[i].kind {
                        Tk::Open('(') => {
                            d += 1;
                            if d == 1 {
                                start = Some(i + 1);
                            }
                        }
                        Tk::Close(')') => {
                            d -= 1;
                            if d == 0 {
                                if let Some(s) = start.take() {
                                    // Each parameter's *pattern* -- up to its
                                    // type colon -- juxtaposes; the type does
                                    // not, so `impl Fn(i32, i32)` keeps its
                                    // argument list rather than becoming a
                                    // tuple.
                                    let mut seg = s;
                                    let mut dd = 0i32;
                                    let mut colon: Option<usize> = None;
                                    for j in s..i {
                                        match toks[j].kind {
                                            Tk::Open(_) => dd += 1,
                                            Tk::Close(_) => dd -= 1,
                                            Tk::Colon if dd == 0 && colon.is_none() => colon = Some(j),
                                            Tk::Comma if dd == 0 => {
                                                out.push((seg, colon.unwrap_or(j)));
                                                seg = j + 1;
                                                colon = None;
                                            }
                                            _ => {}
                                        }
                                    }
                                    if seg < i {
                                        out.push((seg, colon.unwrap_or(i)));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                return out;
            }
            return Vec::new();
        };
        let start = match kw.text.as_str() {
            "for" => top_level(&|t| t.is_kw("in")).map(|i| i + 1),
            // The pattern in `if let PAT = EXPR:` juxtaposes as well, so the
            // region runs from the keyword to the block colon.
            _ => top_level(&|t| t.text == kw.text).map(|i| i + 1),
        };
        let stop = *sig.last().unwrap(); // the block-opening `:`
        return start.filter(|&s| s < stop).map(|s| vec![(s, stop)]).unwrap_or_default();
    }

    // A macro call at item level -- `compile_error! "msg"`, a crate's
    // `define_x! a b` -- is applied as it is in a block: Harsh is a superset
    // of Rust, and an item may be a macro call. It was left unapplied,
    // `compile_error! "msg";`, invalid Rust (found 2026-09-24, when a derive
    // emitted one).
    let item_macro = !stmt_block
        && sig.len() >= 2
        && toks[sig[0]].kind == Tk::Ident
        && toks[sig[1]].text == "!"
        && toks[sig[0]].span.hi == toks[sig[1]].span.lo;
    if item_macro {
        return regions(toks, true, is_header);
    }

    if !stmt_block {
        // A tuple struct's payload, `struct Meters f64`, `struct Pair i32
        // i32`: an application after the name and its generics. And a
        // variant's, `Tuple i32 String`, on its own line in an enum body:
        // a bare name applied to types. A field line, `x: f64`, holds a
        // colon and is left alone.
        {
            let mut k = 0;
            while k < sig.len() && crate::rules::MODIFIERS.contains(&toks[sig[k]].text.as_str()) && !toks[sig[k]].is_kw("const") {
                k += 1;
                if k < sig.len() && toks[sig[k]].kind == Tk::Open('(') {
                    while k < sig.len() && toks[sig[k]].kind != Tk::Close(')') {
                        k += 1;
                    }
                    k += 1;
                }
            }
            if k < sig.len() && (toks[sig[k]].is_kw("struct") || toks[sig[k]].is_kw("union")) {
                // Past the name and its generics.
                let mut j = k + 2;
                if j < sig.len() && toks[sig[j]].kind == Tk::Lt {
                    let mut d = 0i32;
                    while j < sig.len() {
                        match toks[sig[j]].kind {
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
                // The name applies to the payload; the generics stand
                // between them and are skipped by the region's shape
                // (`MyBox<T> T`: head `MyBox<T>`, argument `T`).
                let start = sig[k + 1];
                let stop = if j < sig.len() && toks[sig[j]].kind == Tk::Open('[') { sig[j] } else { end };
                let has_generics = k + 2 < sig.len() && toks[sig[k + 2]].kind == Tk::Lt;
                if j < sig.len() && stop > start && !has_generics {
                    return vec![(start, stop)];
                }
                return Vec::new();
            }
            let no_colon = !sig.iter().any(|&i| matches!(toks[i].kind, Tk::Colon | Tk::Eq | Tk::Semi | Tk::Hash));
            let first_ok = toks[sig[0]].kind == Tk::Ident && !crate::rules::is_keyword(&toks[sig[0]].text) && !crate::rules::BLOCK_KEYWORDS.contains(&toks[sig[0]].text.as_str());
            if !is_header && no_colon && first_ok && sig.len() > 1 && toks[sig[1]].kind != Tk::Open('{') && toks[sig[1]].kind != Tk::Punct {
                return vec![(sig[0], end)];
            }
        }
        // A `const` or `static` item: its value after `=` is an expression.
        let mut k = 0;
        // (`const` is a modifier too, of `fn`; here it is the item itself.)
        while k < sig.len()
            && crate::rules::MODIFIERS.contains(&toks[sig[k]].text.as_str())
            && !toks[sig[k]].is_kw("const")
        {
            k += 1;
            if k < sig.len() && toks[sig[k]].kind == Tk::Open('(') {
                while k < sig.len() && toks[sig[k]].kind != Tk::Close(')') {
                    k += 1;
                }
                k += 1;
            }
        }
        if k < sig.len() && (toks[sig[k]].is_kw("const") || toks[sig[k]].is_kw("static")) {
            let mut d = 0i32;
            for &i in &sig[k..] {
                match toks[i].kind {
                    Tk::Open(_) => d += 1,
                    Tk::Close(_) => d -= 1,
                    Tk::Eq if d == 0 => return vec![(i + 1, end)],
                    _ => {}
                }
            }
        }
        // Not a statement -- an item, a field, a macro arm. A brace body on
        // the line is Harsh nonetheless: `name! { .. }`, or a macro arm's
        // transcriber `=> { .. }`. Its inside is an expression region.
        let mut out = Vec::new();
        let mut i = sig[0];
        while i < end {
            if toks[i].kind == Tk::Open('{') && i > 0 {
                let p = &toks[i - 1];
                let macro_body = (p.kind == Tk::Punct && p.text == "!") || p.kind == Tk::FatArrow;
                if macro_body {
                    if let Some(c) = matching_close_idx(toks, i, end) {
                        out.push((i + 1, c));
                        i = c + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        return out;
    }

    match first {
        // Both sides juxtapose: `let Some p = prev` binds a pattern, and the
        // pattern grammar is the same as application. `let` and `mut` are not
        // callable, so the binding keywords cannot start a run.
        "let" | "const" | "static" => vec![(sig[0], end)],
        "use" | "mod" | "type" | "impl" | "struct" | "enum" | "trait" | "fn" | "where"
        | "macro_rules" => Vec::new(),
        "return" | "break" => vec![(sig[0] + 1, end)],
        _ => vec![(sig[0], end)],
    }
}

/// A macro applied to a single parenthesised group with top-level commas is
/// almost always Rust call syntax written by mistake. A macro taking a tuple is
/// vanishingly rare, and `m! ((a, b))` still expresses it, so this is rejected.
pub fn check(toks: &[Token], stmt_block: bool, is_header: bool) -> Result<(), JuxtError> {
    // A name and the group that isolates its argument are separated by a
    // space. Written tight, `f(x)` reads as Rust's call, and Harsh has no
    // call syntax: `f (x)`, or `f x` when the argument is one token.
    for i in 0..toks.len().saturating_sub(1) {
        let (a, b) = (&toks[i], &toks[i + 1]);
        // A macro's bang is glued to its name; a bare `!` is negation, and
        // `!(a && b)` is Rust's, kept.
        let bang = a.kind == Tk::Punct && a.text == "!" && i > 0 && toks[i - 1].kind == Tk::Ident && toks[i - 1].span.hi == a.span.lo;
        let name = a.kind == Tk::Ident || bang;
        if name && b.kind == Tk::Open('(') && !b.synthetic && !b.dollar && a.span.hi == b.span.lo {
            let what = if a.text == "!" { "a macro" } else { "a name" };
            return Err(JuxtError {
                msg: format!(
                    "`{}(`: {what} and the group isolating its argument are separated by a space, `{} (..)`; a single-token argument needs no parens, `{} x`",
                    a.text, a.text, a.text
                ),
                span: b.span,
            });
        }
    }
    for (a, b) in regions(toks, stmt_block, is_header) {
        check_region(toks, a, b)?;
    }
    Ok(())
}

/// The macro-tuple check over one expression region.
fn check_region(toks: &[Token], a: usize, b: usize) -> Result<(), JuxtError> {
    let mut i = a;
    while i < b {
        // A group's inside is a region of its own: `f (vec! [1, 2])` is
        // checked as `vec! [1, 2]` is. Until 2026-09-22 the walk stepped over
        // a parenthesised head without looking in.
        if matches!(toks[i].kind, Tk::Open('(') | Tk::Open('[')) {
            if let Some(c) = matching_close_idx(toks, i, b) {
                check_region(toks, i + 1, c)?;
            }
        }
        let Some(head_end) = atom_end(toks, i, b) else {
            i += 1;
            continue;
        };
        // A `~` call's stream is its macro's (3.1.1): the `!` rules below
        // never judge it. Found 2026-09-22: `filled~ [0u8; 2]` and the
        // guide's own `pair~ (1, 2)` were refused on the raw path the editor
        // and the formatter's guard read.
        // A literal cannot be applied (ruled 2026-09-22, A3): `x = 1 b` was
        // `1(b)`, and `assert_eq! a + 1 b` was `assert_eq!(a) + 1(b)`, both
        // silently. A literal is a head -- not an argument -- when what
        // precedes it does not end an atom: an operator, `=`, a keyword, an
        // opener, or nothing.
        if matches!(toks[i].kind, Tk::Int | Tk::Float | Tk::Str | Tk::Char) && head_end < b && atom_end(toks, head_end, b).is_some() {
            let prev = (a..i).rev().find(|&k| !toks[k].is_comment()).map(|k| &toks[k]);
            let is_arg = prev.map_or(false, |p| match p.kind {
                Tk::Ident => !NOT_CALLABLE.contains(&p.text.as_str()),
                Tk::Int | Tk::Float | Tk::Str | Tk::Char | Tk::TupleIdx | Tk::Close(_) => true,
                Tk::Punct => p.text == "!" || p.text == "$" || (p.text == ">>" && closes_turbofish(toks, a, i)),
                // The `>` of a turbofish ends its atom: `f.<u8> 1 2` applies
                // (found 2026-10-04). A comparison's `>` does not.
                Tk::Gt => closes_turbofish(toks, a, i),
                _ => false,
            });
            if !is_arg {
                let next = &toks[head_end];
                return Err(JuxtError {
                    msg: format!(
                        "a literal cannot be applied: `{} {}`; if the literal ends an argument that has an operator in it, isolate that argument in parentheses, as in `(a + 1) b`",
                        toks[i].text, next.text
                    ),
                    span: next.span,
                });
            }
        }
        let is_macro = head_end > a && toks[head_end - 1].text == "!" && !toks[head_end - 1].tilde;
        // A macro applied to one array literal and nothing else is Rust's
        // bracket call written in Harsh, where it now means something else:
        // `vec! [1, 2, 3]` is a vector holding one array. Refused, both
        // spellings named; `vec! ([1, 2, 3])` says one array on purpose.
        // (Ruled 2026-09-22, A1.)
        if is_macro && head_end < b && toks[head_end].kind == Tk::Open('[') {
            if let Some(c) = matching_close_idx(toks, head_end, b) {
                if !(c + 1 < b && atom_end(toks, c + 1, b).is_some()) {
                    let name = toks[head_end - 2].text.clone();
                    let repeat = toks[head_end + 1..c].iter().any(|t| t.kind == Tk::Semi);
                    let want = if repeat {
                        format!("Rust's `{name}![x; n]` is `{name}! {{ x; n }}` in Harsh")
                    } else if c == head_end + 1 {
                        format!("Rust's `{name}![]` is `{name}!$` in Harsh")
                    } else {
                        format!("Rust's `{name}![a, b]` is `{name}! a b` in Harsh")
                    };
                    return Err(JuxtError {
                        msg: format!(
                            "`{name}! [..]` passes one array to the macro: {want}; to pass one array on purpose, isolate it, `{name}! ([..])`"
                        ),
                        span: toks[head_end].span,
                    });
                }
            }
        }
        // `f arr [1]`: the `[` is spaced, so it is an index -- but of what?
        // Of `arr`, the reader means; of `f(arr)`, the rule would say. Neither
        // reading is taken: the line is refused with both spellings named.
        // (A `[` right after a function's head is an index of the head with
        // no argument in between. After a macro's bang it is the first
        // argument, handled above.) A macro too, once it has an argument:
        // `assert_eq! v [1, 2]` is refused like `f arr [1]`.
        {
            let mut k = head_end;
            let mut nargs = 0;
            while k < b {
                match atom_end(toks, k, b) {
                    Some(e) => {
                        nargs += 1;
                        k = e;
                    }
                    None => break,
                }
            }
            if nargs > 0 && k < b && toks[k].kind == Tk::Open('[') && toks[k].span.lo > toks[k - 1].span.hi {
                let prev = &toks[k - 1];
                return Err(JuxtError {
                    msg: format!(
                        "`{} [` inside an application: write `{}[..]` for the element, or `({} [..])` to make it one argument",
                        prev.text, prev.text, prev.text
                    ),
                    span: toks[k].span,
                });
            }
        }
        if is_macro && head_end < b && toks[head_end].kind == Tk::Open('(') {
            if let Some(e) = atom_end(toks, head_end, b) {
                // A macro is applied like a function: `matches! x (Some n if n > 1)`.
                // A parenthesised list is a tuple, and one is never what a
                // macro was meant to receive, so it is rejected outright.
                if has_top_comma(toks, head_end + 1, e - 1) && e >= b {
                    return Err(JuxtError {
                        msg: "a macro takes juxtaposed arguments, not a parenthesised \
                              list; write `m! a b` (or `m! ((a, b))` for a tuple)"
                            .into(),
                        span: toks[head_end].span,
                    });
                }
            }
        }
        i = head_end.max(i + 1);
    }
    Ok(())
}

/// Fixups for one logical line.
pub fn fixups(toks: &[Token], stmt_block: bool, is_header: bool) -> Vec<(usize, Jx)> {
    let mut out = Vec::new();
    let regs = regions(toks, stmt_block, is_header);
    for &(a, b) in &regs {
        // A trailing `;` is not part of the expression.
        let mut b = b;
        while b > a && (toks[b - 1].kind == Tk::Semi || toks[b - 1].is_comment()) {
            b -= 1;
        }
        apply_region(toks, a, b, &mut out);
    }
    type_app_fixups(toks, &regs, &mut out);
    tuple_struct_fixups(toks, &mut out);
    if !is_header {
        grouping_fixups(toks, &mut out);
    }
    out
}

/// `struct MyBox<T> T`, `struct Pair<A, B> A B`: a tuple struct whose
/// payload follows its generics. The name-with-generics is not an
/// expression atom, so the payload is applied as a bare argument list.
fn tuple_struct_fixups(toks: &[Token], out: &mut Vec<(usize, Jx)>) {
    let sig: Vec<usize> = (0..toks.len()).filter(|&i| !toks[i].is_comment()).collect();
    let mut k = 0;
    while k < sig.len() && crate::rules::MODIFIERS.contains(&toks[sig[k]].text.as_str()) && !toks[sig[k]].is_kw("const") {
        k += 1;
        if k < sig.len() && toks[sig[k]].kind == Tk::Open('(') {
            while k < sig.len() && toks[sig[k]].kind != Tk::Close(')') {
                k += 1;
            }
            k += 1;
        }
    }
    if !(k + 2 < sig.len() && toks[sig[k]].is_kw("struct") && toks[sig[k + 2]].kind == Tk::Lt) {
        return;
    }
    // Past the generics.
    let mut j = k + 2;
    let mut d = 0i32;
    while j < sig.len() {
        match toks[sig[j]].kind {
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
    if j >= sig.len() || toks[sig[j]].kind == Tk::Open('[') || toks[sig[j]].kind == Tk::Semi {
        return;
    }
    let mut args: Vec<(usize, usize)> = Vec::new();
    let mut e = sig[j];
    let stop = sig.iter().find(|&&i| i >= e && toks[i].kind == Tk::Open('[')).copied().unwrap_or(toks.len());
    while e < stop {
        match atom_end(toks, e, stop) {
            Some(n) if toks[e].kind != Tk::Punct => {
                args.push((e, n));
                e = n;
            }
            _ => break,
        }
    }
    if !args.is_empty() {
        apply_args(toks, &args, out);
    }
}

/// The parenthesised types are applications too: `Fn i32 -> i32`,
/// `Fn (i32) (f64) -> i32`, `fn (x: i32) (y: f64) -> &str`, `FnOnce$`.
/// `Fn`, `FnMut` and `FnOnce` are only ever types; a `fn` is a type when
/// what precedes it says so (`:`, `->`, `<`, `&`, `dyn`, `impl`, `(`, `,`).
/// Types lie outside the expression regions, so they are found here; an
/// occurrence inside a region was already an application there.
fn type_app_fixups(toks: &[Token], regs: &[(usize, usize)], out: &mut Vec<(usize, Jx)>) {
    let in_region = |i: usize| regs.iter().any(|&(a, b)| i >= a && i < b);
    let mut i = 0;
    while i < toks.len() {
        let t = &toks[i];
        let is_trait = t.kind == Tk::Ident && matches!(t.text.as_str(), "Fn" | "FnMut" | "FnOnce");
        let is_fn_type = t.is_kw("fn") && i > 0 && {
            // Before `fn`, step over its qualifiers -- `unsafe`, `extern`,
            // an ABI string: `extern "C" fn (A) (B) -> C` is a function type
            // as `fn (A) (B) -> C` is (found 2026-09-28, a `qsort` callback).
            let mut k = i;
            loop {
                let Some(q) = (0..k).rev().find(|&j| !toks[j].is_comment()) else { break };
                if toks[q].is_kw("unsafe") || toks[q].is_kw("extern") || (toks[q].kind == Tk::Str && q > 0 && toks[q - 1].is_kw("extern")) {
                    k = q;
                } else {
                    break;
                }
            }
            let p = (0..k).rev().find(|&k| !toks[k].is_comment()).map(|k| &toks[k]);
            p.map_or(false, |p| {
                matches!(p.kind, Tk::Colon | Tk::Lt | Tk::Comma | Tk::Open('('))
                    || (p.kind == Tk::Punct && p.text == "->")
                    || (p.kind == Tk::Punct && (p.text == "&" || p.text == "&&"))
                    || p.is_kw("dyn")
                    || p.is_kw("impl")
                    || p.is_kw("mut")
                    || (p.kind == Tk::Eq && (0..i).rev().find(|&k| toks[k].is_kw("type")).is_some())
            })
        };
        // `FnOnce$` already carries its `()`; nothing to apply.
        let applied_to_nothing = toks.get(i + 1).map_or(false, |n| n.dollar);
        if (is_trait || is_fn_type) && !in_region(i) && !applied_to_nothing {
            // The argument run: atoms up to `->`, `+`, `>`, `,` or a closer.
            let mut args: Vec<(usize, usize)> = Vec::new();
            let mut e = i + 1;
            while e < toks.len() {
                match atom_end(toks, e, toks.len()) {
                    Some(n) if !(toks[e].kind == Tk::Punct) => {
                        args.push((e, n));
                        e = n;
                    }
                    _ => break,
                }
            }
            if !args.is_empty() {
                apply_args(toks, &args, out);
            }
            i = e.max(i + 1);
            continue;
        }
        i += 1;
    }
}

/// The fixups for a run of atoms that are the *rest* of an argument list
/// (the `, ` and `)` written by the caller): commas between the atoms,
/// each group's isolating parens dropped, each argument's inside applied.
pub fn args_only(toks: &[Token]) -> Vec<(usize, Jx)> {
    let mut out = Vec::new();
    let mut args: Vec<(usize, usize)> = Vec::new();
    let mut e = 0;
    while e < toks.len() {
        if toks[e].is_comment() {
            e += 1;
            continue;
        }
        match atom_end(toks, e, toks.len()) {
            Some(n) if toks[e].kind != Tk::Punct => {
                args.push((e, n));
                e = n;
            }
            _ => break,
        }
    }
    for w in args.windows(2) {
        out.push((w[1].0, Jx::Comma));
    }
    let is_unit = |a: usize, b: usize| toks[a].kind == Tk::Open('(') && b - a == 2;
    for (a, b) in &args {
        if toks[*a].kind == Tk::Open('(') && !is_unit(*a, *b) && !has_top_comma(toks, a + 1, b - 1) {
            out.push((*a, Jx::Drop));
            out.push((*b - 1, Jx::Drop));
            split_group(toks, a + 1, b - 1, &mut out);
        }
    }
    out
}

/// The argument list of a head that is not itself an expression atom (a
/// type's `Fn`, `fn`): the same list rule as `apply_region`'s.
fn apply_args(toks: &[Token], args: &[(usize, usize)], out: &mut Vec<(usize, Jx)>) {
    let is_unit = |a: usize, b: usize| toks[a].kind == Tk::Open('(') && b - a == 2;
    let single_group = args.len() == 1
        && toks[args[0].0].kind == Tk::Open('(')
        && !is_unit(args[0].0, args[0].1)
        && !has_top_comma(toks, args[0].0 + 1, args[0].1 - 1);
    if !single_group {
        out.push((args[0].0, Jx::Open));
        for w in args.windows(2) {
            out.push((w[1].0, Jx::Comma));
        }
        out.push((args[args.len() - 1].1 - 1, Jx::Close));
    }
    if args.len() > 1 {
        for (a, b) in args {
            if toks[*a].kind == Tk::Open('(') && !is_unit(*a, *b) && !has_top_comma(toks, a + 1, b - 1) {
                out.push((*a, Jx::Drop));
                out.push((*b - 1, Jx::Drop));
            }
        }
    }
}

/// Optional grouping is not emitted: parens around the whole of a statement
/// or the whole of a value after `=` or `=>` enclose nothing they could bind
/// tighter than. A tuple -- a comma inside -- is a construct, kept.
fn grouping_fixups(toks: &[Token], out: &mut Vec<(usize, Jx)>) {
    let sig: Vec<usize> = (0..toks.len()).filter(|&i| !toks[i].is_comment() && toks[i].kind != Tk::Semi).collect();
    if sig.is_empty() {
        return;
    }
    let mut start = 0usize;
    // After a top-level `=` or `=>`, the value.
    let mut d = 0i32;
    for (k, &i) in sig.iter().enumerate() {
        match toks[i].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Eq | Tk::FatArrow if d == 0 => start = k + 1,
            _ => {}
        }
    }
    let mut lo = start;
    let mut hi = sig.len();
    // Peeled from the outside in, so `((x))` loses both pairs.
    while lo + 1 < hi {
        let o = sig[lo];
        let c = sig[hi - 1];
        if toks[o].kind != Tk::Open('(') || toks[o].synthetic || toks[o].dollar || toks[c].kind != Tk::Close(')') {
            return;
        }
        if matching_close_idx(toks, o, c + 1) != Some(c) {
            return;
        }
        // A tuple, or the unit `()`: a construct, kept.
        if has_top_comma(toks, o + 1, c) || c == o + 1 {
            return;
        }
        out.push((o, Jx::Drop));
        out.push((c, Jx::Drop));
        lo += 1;
        hi -= 1;
    }
}

/// The index of the bracket closing the one at `i`, within `[i, end)`.
fn matching_close_idx(toks: &[Token], i: usize, end: usize) -> Option<usize> {
    let mut d = 0i32;
    for k in i..end {
        match toks[k].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => {
                d -= 1;
                if d == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

/// The pipes. `|>` fills a function's parameters from the left, `<|` from
/// the right; when the count reaches the arity the closure disappears and it
/// is a call, and when it falls short the rest is deferred as one flat
/// closure:
///
/// ```text
/// 3 |> sub            ->  move |__hrs1, __hrs2| sub 3 __hrs1 __hrs2    (sub has three parameters)
/// 3 5 |> sub          ->  move |__hrs1| sub 3 5 __hrs1
/// 3 5 10 |> sub       ->  sub 3 5 10
/// sub <| 3            ->  move |__hrs1, __hrs2| sub __hrs1 __hrs2 3
/// 1 |> sub <| 3       ->  move |__hrs1| sub 1 __hrs1 3
/// x |> f |> g         ->  g (f x)
/// f <| g <| x         ->  f (g x)
/// ```
///
/// Rules, each a brick:
/// - Left of `|>` is a sequence of atoms, all arguments; right of `<|` too.
///   To pipe the *result* of an application, isolate it: `(f a) |> g`.
/// - The function is one atom. `a |> f <| b` is one application: `a` from
///   the left, `b` from the right.
/// - `|>` is left-associative, `<|` right-associative; a chained result is
///   one value and is wrapped in synthetic parens.
/// - Arity comes from a `fn` declared in the project or from a partial bound
///   by `let`. Unknown arity: the atoms given are taken as all of them and it
///   is a plain call, so pipes into std and crate functions keep working;
///   rustc reports a wrong count. Too many for a known arity is an error here.
///
/// This runs on each logical line before juxtaposition, which then wraps the
/// emitted runs into calls; the source map and the separator rules never see a
/// pipe. The arity table is read here and nowhere else.
pub fn rewrite_pipes(
    toks: &[Token],
    arities: &std::collections::HashMap<String, usize>,
    partials: &mut std::collections::HashMap<String, usize>,
) -> Result<Vec<Token>, JuxtError> {
    // A macro definition's body is token soup, and `$` is its metavariable sigil.
    if toks.iter().any(|t| t.is_kw("macro_rules")) {
        return Ok(toks.to_vec());
    }
    // `let NAME = ...` binds whatever partial the line produces.
    let sig: Vec<usize> = (0..toks.len()).filter(|&i| !toks[i].is_comment()).collect();
    let bound: Option<String> = if sig.len() >= 4
        && toks[sig[0]].is_kw("let")
        && toks[sig[1]].kind == Tk::Ident
        && toks[sig[2]].kind == Tk::Eq
    {
        Some(toks[sig[1]].text.clone())
    } else {
        None
    };
    let mut cx = Pipes { arities, partials, bound, last_remaining: None, group_arity: None };
    let mut out = Vec::with_capacity(toks.len());
    cx.range(toks, 0, toks.len(), &mut out)?;
    if let (Some(n), Some(r)) = (&cx.bound, cx.last_remaining) {
        cx.partials.insert(n.clone(), r);
    }
    Ok(out)
}

struct Pipes<'a> {
    arities: &'a std::collections::HashMap<String, usize>,
    partials: &'a mut std::collections::HashMap<String, usize>,
    bound: Option<String>,
    /// Remaining arity of the last partial emitted at the top level of this
    /// line, so a `let` can bind it.
    last_remaining: Option<usize>,
    /// Arity of the callee just parsed by `callee_and_back` when it was a
    /// parenthesised partial.
    group_arity: Option<usize>,
}

/// One side of a pipe, rendered: a list of atoms, or one wrapped value.
enum Side {
    Atoms(Vec<Vec<Token>>),
    Value(Vec<Token>),
}

impl Side {
    fn into_atoms(self, at: Span, line: usize) -> Vec<Vec<Token>> {
        match self {
            Side::Atoms(a) => a,
            Side::Value(v) => {
                let mut w = vec![paren(Tk::Open('('), "(", at, line)];
                w.extend(v);
                w.push(paren(Tk::Close(')'), ")", at, line));
                vec![w]
            }
        }
    }
}

fn paren(kind: Tk, text: &str, at: Span, line: usize) -> Token {
    Token {
        ctx: 0,
        kind,
        span: Span { lo: at.lo, hi: at.lo },
        text: text.to_string(),
        line_start: None,
        line,
        synthetic: true,
        dollar: false,
        tilde: false,
    }
}

/// Index of the first / last pipe of each kind at bracket depth zero.
fn find_pipe(toks: &[Token], from: usize, to: usize, kind: Tk, last: bool) -> Option<usize> {
    let mut d = 0i32;
    let mut found = None;
    for i in from..to {
        match toks[i].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            ref k if d == 0 && *k == kind => {
                found = Some(i);
                if !last {
                    return found;
                }
            }
            _ => {}
        }
    }
    found
}

/// Where the expression containing `at` begins: after the nearest preceding
/// binding, separator or keyword. `let a = merge <| f` has `merge` as the
/// operand, not `let a = merge`.
fn expr_lo(toks: &[Token], from: usize, at: usize) -> usize {
    let mut d = 0i32;
    let mut lo = from;
    for i in from..at {
        match toks[i].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Eq | Tk::Comma | Tk::Semi | Tk::FatArrow | Tk::Colon if d == 0 => lo = i + 1,
            Tk::Ident if d == 0 && NOT_CALLABLE.contains(&toks[i].text.as_str()) => lo = i + 1,
            _ => {}
        }
    }
    lo
}

impl<'a> Pipes<'a> {
    /// Copy `from..to` to `out`, rewriting every pipe expression in it.
    fn range(&mut self, toks: &[Token], from: usize, to: usize, out: &mut Vec<Token>) -> Result<(), JuxtError> {
        let has_pipe = find_pipe(toks, from, to, Tk::PipeFwd, false).is_some()
            || find_pipe(toks, from, to, Tk::PipeBack, false).is_some();
        if !has_pipe {
            return self.groups(toks, from, to, out);
        }
        // The pipe expression starts after the last binding/separator before
        // the first pipe; what precedes it is copied through.
        let first = find_pipe(toks, from, to, Tk::PipeFwd, false)
            .into_iter()
            .chain(find_pipe(toks, from, to, Tk::PipeBack, false))
            .min()
            .unwrap();
        let lo = expr_lo(toks, from, first);
        for i in from..lo {
            out.push(toks[i].clone());
        }
        // A top-level pipe expression runs to the next separator at depth 0.
        let mut hi = to;
        let mut d = 0i32;
        for i in lo..to {
            match toks[i].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                Tk::Comma | Tk::Semi if d == 0 => {
                    hi = i;
                    break;
                }
                _ => {}
            }
        }
        let side = self.expr(toks, lo, hi, true)?;
        match side {
            Side::Value(v) => out.extend(v),
            Side::Atoms(a) => out.extend(a.into_iter().flatten()),
        }
        if hi < to {
            self.range(toks, hi, to, out)?;
        }
        Ok(())
    }

    /// Copy tokens, recursing into bracketed groups so pipes inside them are
    /// rewritten too.
    fn groups(&mut self, toks: &[Token], from: usize, to: usize, out: &mut Vec<Token>) -> Result<(), JuxtError> {
        let mut i = from;
        while i < to {
            if matches!(toks[i].kind, Tk::Open(_)) {
                let mut d = 0i32;
                let mut j = i;
                while j < to {
                    match toks[j].kind {
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
                if j < to {
                    out.push(toks[i].clone());
                    let saved = self.last_remaining.take();
                    self.range(toks, i + 1, j, out)?;
                    // A partial inside brackets is not what the line binds.
                    self.last_remaining = saved;
                    out.push(toks[j].clone());
                    i = j + 1;
                    continue;
                }
            }
            out.push(toks[i].clone());
            i += 1;
        }
        Ok(())
    }

    /// A pipe expression over `from..to`. `top` says whether a partial made
    /// here is the line's value, bindable by `let`.
    fn expr(&mut self, toks: &[Token], from: usize, to: usize, top: bool) -> Result<Side, JuxtError> {
        // `|>` is left-associative: split at the rightmost.
        if let Some(p) = find_pipe(toks, from, to, Tk::PipeFwd, true) {
            if p == from {
                return Err(JuxtError { msg: "`|>` needs a value on the left".into(), span: toks[p].span });
            }
            let left = self.expr(toks, from, p, false)?.into_atoms(toks[from].span, toks[p].line);
            // Right: one function atom, optionally `<|` and its arguments.
            let (callee, right) = self.callee_and_back(toks, p + 1, to, toks[p].span)?;
            return self.apply(toks, callee, left, right, toks[p].span, toks[p].line, top);
        }
        // `<|` alone: the function is everything before it, one atom.
        if let Some(p) = find_pipe(toks, from, to, Tk::PipeBack, false) {
            let (callee, right) = self.callee_and_back(toks, from, to, toks[p].span)?;
            return self.apply(toks, callee, Vec::new(), right, toks[p].span, toks[p].line, top);
        }
        Ok(Side::Atoms(self.atoms(toks, from, to)?))
    }

    /// `f` or `f <| args...` in `from..to`.
    fn callee_and_back(&mut self, toks: &[Token], from: usize, to: usize, at: Span) -> Result<(Vec<Token>, Vec<Vec<Token>>), JuxtError> {
        let mut i = from;
        while i < to && toks[i].is_comment() {
            i += 1;
        }
        let Some(e) = atom_end(toks, i, to).filter(|_| i < to) else {
            return Err(JuxtError { msg: "a pipe needs a function: one atom".into(), span: at });
        };
        let mut callee: Vec<Token> = Vec::new();
        // Pipes inside the callee's own brackets are rewritten too. A group
        // that is itself a partial, `(3 |> sub)`, has a known arity: what the
        // inner partial left open.
        let mut group_arity = None;
        if toks[i].kind == Tk::Open('(') && e >= i + 2 {
            let saved = self.last_remaining.take();
            callee.push(toks[i].clone());
            self.range(toks, i + 1, e - 1, &mut callee)?;
            callee.push(toks[e - 1].clone());
            group_arity = self.last_remaining.take();
            self.last_remaining = saved;
        } else {
            self.groups(toks, i, e, &mut callee)?;
        }
        self.group_arity = group_arity;
        let mut k = e;
        while k < to && toks[k].is_comment() {
            k += 1;
        }
        if k >= to {
            return Ok((callee, Vec::new()));
        }
        if toks[k].kind != Tk::PipeBack {
            return Err(JuxtError {
                msg: "after a pipe's function only `<|` may follow; to pipe into an application, isolate it: `x |> (f a)`".into(),
                span: toks[k].span,
            });
        }
        if k + 1 >= to {
            return Err(JuxtError { msg: "`<|` needs an argument on the right".into(), span: toks[k].span });
        }
        let right = self.expr(toks, k + 1, to, false)?.into_atoms(toks[k + 1].span, toks[k].line);
        Ok((callee, right))
    }

    /// Consecutive atoms over `from..to`; anything else is an error.
    fn atoms(&mut self, toks: &[Token], from: usize, to: usize) -> Result<Vec<Vec<Token>>, JuxtError> {
        let mut i = from;
        let mut out = Vec::new();
        while i < to {
            if toks[i].is_comment() {
                i += 1;
                continue;
            }
            let Some(e) = atom_end(toks, i, to) else {
                return Err(JuxtError {
                    msg: "a pipe's arguments are atoms; isolate an expression in parentheses".into(),
                    span: toks[i].span,
                });
            };
            let mut a = Vec::new();
            self.groups(toks, i, e, &mut a)?;
            out.push(a);
            i = e;
        }
        if out.is_empty() {
            return Err(JuxtError { msg: "a pipe needs at least one argument".into(), span: toks[from.min(toks.len() - 1)].span });
        }
        Ok(out)
    }

    /// Apply `callee` to `left ++ holes ++ right`, as a call when the count
    /// meets the arity, as a closure when it falls short.
    #[allow(clippy::too_many_arguments)]
    fn apply(
        &mut self,
        toks: &[Token],
        callee: Vec<Token>,
        left: Vec<Vec<Token>>,
        right: Vec<Vec<Token>>,
        at: Span,
        line: usize,
        top: bool,
    ) -> Result<Side, JuxtError> {
        let _ = toks;
        let name: String = callee.iter().filter(|t| !t.synthetic).map(|t| t.text.as_str()).collect::<Vec<_>>().join("");
        let last_seg = callee.iter().rev().find(|t| t.kind == Tk::Ident).map(|t| t.text.clone()).unwrap_or_default();
        let arity = self
            .group_arity
            .take()
            .or_else(|| self.partials.get(&name).copied())
            .or_else(|| self.arities.get(&name).copied())
            .or_else(|| self.arities.get(&last_seg).copied());
        let supplied = left.len() + right.len();
        let holes = match arity {
            Some(a) if supplied > a => {
                return Err(JuxtError {
                    msg: format!("`{name}` takes {a} parameter(s) and {supplied} were piped in"),
                    span: at,
                });
            }
            Some(a) => a - supplied,
            None => 0,
        };
        // The pieces leave source order, so the emitter must not copy the
        // whitespace that preceded each of them: the first token of every
        // piece is marked synthetic (its span still maps), and the tokens
        // inside a piece keep their own gaps.
        let mut callee = callee;
        let mut left = left;
        let mut right = right;
        if let Some(t) = callee.first_mut() {
            t.synthetic = true;
        }
        for a in left.iter_mut().chain(right.iter_mut()) {
            if let Some(t) = a.first_mut() {
                t.synthetic = true;
            }
        }
        let mut v: Vec<Token> = Vec::new();
        if holes > 0 {
            let params: Vec<String> = (1..=holes).map(|p| format!("__hrs{p}")).collect();
            v.push(Token {
                ctx: 0,
                kind: Tk::Punct,
                span: Span { lo: at.lo, hi: at.lo },
                text: format!("move |{}| ", params.join(", ")),
                line_start: None,
                line,
                synthetic: true,
                dollar: false,
                tilde: false,
            });
            v.extend(callee);
            v.extend(left.into_iter().flatten());
            let end_at = v.last().map(|t| t.span).unwrap_or(at);
            for p in params {
                v.push(Token {
                    ctx: 0,
                    kind: Tk::Ident,
                    span: Span { lo: end_at.hi, hi: end_at.hi },
                    text: p,
                    line_start: None,
                    line,
                    synthetic: true,
                    dollar: false,
                    tilde: false,
                });
            }
            v.extend(right.into_iter().flatten());
            if top {
                self.last_remaining = Some(holes);
            }
        } else {
            v.extend(callee);
            v.extend(left.into_iter().flatten());
            v.extend(right.into_iter().flatten());
            if top {
                self.last_remaining = None;
            }
        }
        Ok(Side::Value(v))
    }
}

/// The constructors the files being transpiled declare -- every `struct` and
/// every enum variant -- beside the prelude's `Some`, `Ok`, `Err`. Filled by
/// `collect_arities`, which reads each file's declarations before
/// juxtaposition; read by `is_constructor_head`. A name is a constructor
/// because it is declared one, never because of its case (the user,
/// 2026-10-05: Rust does not care how a name is spelt). The set only grows.
static CONSTRUCTORS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Record the constructors `toks` declare: `struct Name`, and each variant
/// of `enum Name` -- the first name on each line of its block.
fn collect_constructors(toks: &[Token]) {
    let mut found: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i + 1 < toks.len() {
        if toks[i].is_kw("struct") && toks[i + 1].kind == Tk::Ident {
            found.push(toks[i + 1].text.clone());
        }
        if toks[i].is_kw("enum") && toks[i + 1].kind == Tk::Ident {
            // The enum's line indentation; its variants are the first names
            // of the deeper lines that follow, until the block ends.
            let ind = (0..=i).rev().find_map(|k| toks[k].line_start).unwrap_or(0);
            let mut k = i + 2;
            while k < toks.len() {
                if let Some(li) = toks[k].line_start {
                    if li <= ind {
                        break;
                    }
                    if toks[k].kind == Tk::Ident && !toks[k].is_comment() {
                        found.push(toks[k].text.clone());
                    }
                }
                k += 1;
            }
            i = k;
            continue;
        }
        i += 1;
    }
    if let Ok(mut c) = CONSTRUCTORS.lock() {
        for f in found {
            if !c.contains(&f) {
                c.push(f);
            }
        }
    }
}

/// Parameter counts of every `fn` in a token stream, by name. Arity is the
/// number of parameters, whichever of Harsh's declaration forms was used.
pub fn collect_arities(toks: &[Token]) -> std::collections::HashMap<String, usize> {
    collect_constructors(toks);
    let mut m = std::collections::HashMap::new();
    let mut i = 0usize;
    while i < toks.len() {
        if toks[i].is_kw("fn") && i + 1 < toks.len() && toks[i + 1].kind == Tk::Ident {
            let name = toks[i + 1].text.clone();
            let mut j = i + 2;
            // Skip generics.
            if j < toks.len() && toks[j].text == "<" {
                let mut d = 0i32;
                while j < toks.len() {
                    if toks[j].text == "<" { d += 1; }
                    if toks[j].text == ">" { d -= 1; if d == 0 { j += 1; break; } }
                    if toks[j].text == ">>" { d -= 2; if d <= 0 { j += 1; break; } }
                    j += 1;
                }
            }
            // Parameters run to `->`, the block-opening `:`/`do`, or `where`.
            let mut n = 0usize;
            let mut k = j;
            let mut bare_seen = false;
            while k < toks.len() {
                let t = &toks[k];
                if t.text == "->" || t.is_kw("where") || t.is_kw("do") {
                    break;
                }
                if t.kind == Tk::Colon {
                    // A colon at depth 0 that is not inside a group: either a
                    // bare parameter's type annotation or the block opener.
                    if bare_seen { k += 1; continue; }
                    break;
                }
                if t.kind == Tk::Open('(') {
                    if let Some(e) = atom_end(toks, k, toks.len()) {
                        // Each group: one parameter, or several by commas;
                        // `self` counts; an empty group counts nothing.
                        let inner = &toks[k + 1..e - 1];
                        let sig: Vec<&Token> = inner.iter().filter(|t| !t.is_comment()).collect();
                        if !sig.is_empty() {
                            let mut d = 0i32;
                            let mut parts = 1usize;
                            for t in &sig {
                                match t.kind {
                                    Tk::Open(_) => d += 1,
                                    Tk::Close(_) => d -= 1,
                                    Tk::Comma if d == 0 => parts += 1,
                                    _ => {}
                                }
                            }
                            // A trailing comma adds no parameter.
                            if sig.last().map(|t| t.kind == Tk::Comma).unwrap_or(false) {
                                parts -= 1;
                            }
                            n += parts;
                        }
                        k = e;
                        continue;
                    }
                }
                if t.kind == Tk::Ident && !bare_seen && n == 0 {
                    // A bare parameter: `fn f name: T`.
                    bare_seen = true;
                    n = 1;
                }
                k += 1;
            }
            m.insert(name, n);
            i = k;
            continue;
        }
        i += 1;
    }
    m
}

/// Does the last `>` (or `>>`) before `at` close a generic list opened by
/// `.<` -- Harsh's turbofish? Walks back to its `<` and looks for the dot.
fn closes_turbofish(toks: &[Token], from: usize, at: usize) -> bool {
    let Some(k) = (from..at).rev().find(|&k| !toks[k].is_comment()) else { return false };
    let mut d = 0i32;
    let mut j = k + 1;
    while j > from {
        j -= 1;
        match toks[j].kind {
            Tk::Gt => d += 1,
            Tk::Punct if toks[j].text == ">>" => d += 2,
            Tk::Lt => {
                d -= 1;
                if d == 0 {
                    return j > from && toks[j - 1].kind == Tk::Dot;
                }
            }
            Tk::Semi | Tk::Open('{') | Tk::Close('}') => return false,
            _ => {}
        }
    }
    false
}

/// The head `toks[i..head_end]` is a constructor: its last name (`Some`,
/// `Point`, `Route.Home`'s `Home`) is one the files declare, or the
/// prelude's. Any other name -- a function, a constant, a constructor from
/// another crate -- keeps the isolation rule: `f (&x)`.
/// Is `name` a constructor -- declared by the files read so far, or the
/// prelude's? (The formatter's layout of `&` asks too.)
pub fn is_constructor_name(name: &str) -> bool {
    matches!(name, "Some" | "Ok" | "Err") || CONSTRUCTORS.lock().map_or(false, |c| c.iter().any(|n| n == name))
}

fn is_constructor_head(toks: &[Token], i: usize, head_end: usize) -> bool {
    let Some(last) = (i..head_end).rev().find(|&k| toks[k].kind == Tk::Ident) else { return false };
    let name = toks[last].text.as_str();
    matches!(name, "Some" | "Ok" | "Err") || CONSTRUCTORS.lock().map_or(false, |c| c.iter().any(|n| n == name))
}

/// `&x`, `& x`, `&mut x`, `& mut x`, `mut x`, `ref x`, `ref mut x` starting at
/// `k`, as the only argument: the prefix, one atom, then nothing that could be
/// a further argument. Returns the argument's end.
fn prefixed_single_arg(toks: &[Token], k: usize, to: usize) -> Option<usize> {
    let mut j = k;
    let amp = toks[j].kind == Tk::Punct && toks[j].text == "&";
    if amp {
        j += 1;
        if j < to && toks[j].is_kw("mut") {
            j += 1;
        }
    } else if toks[j].is_kw("mut") {
        j += 1;
    } else if toks[j].is_kw("ref") {
        j += 1;
        if j < to && toks[j].is_kw("mut") {
            j += 1;
        }
    } else {
        return None;
    }
    if j >= to {
        return None;
    }
    let e = atom_end(toks, j, to)?;
    if e < to && atom_end(toks, e, to).is_some() {
        return None; // more arguments: each is isolated, `Rgb (mut r) g b`
    }
    Some(e)
}

/// After a callee ending at `e`, do the tokens up to `c` form its arguments
/// alone -- atoms, one after another -- so that `toks[..c]` is one call?
fn is_application(toks: &[Token], e: usize, c: usize) -> bool {
    let mut k = e;
    while k < c {
        match atom_end(toks, k, c) {
            Some(n) if n > k => k = n,
            _ => return false,
        }
    }
    true
}


// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Rules shared by both directions of translation.

/// Keywords that introduce a block, indexed by the block kind they produce.
/// Read forwards this decides the separator inside a block; read backwards it
/// decides whether a `{` becomes indentation.
pub const BLOCK_KEYWORDS: [&str; 15] = [
    "fn", "struct", "enum", "union", "impl", "trait", "mod", "match", "if", "else", "while",
    "for", "loop", "unsafe", "async",
];

/// Modifiers skipped when looking for the keyword that classifies a header.
pub const MODIFIERS: [&str; 7] =
    ["pub", "async", "unsafe", "const", "extern", "default", "move"];

/// Leading keywords whose line always terminates with `;`, even as the last
/// line of a block, because they are statements rather than tail expressions.
/// `return` is deliberately absent: `{ return x }` is valid Rust, so forcing a
/// semicolon there would make the round trip inexact for no benefit.
/// Macros whose call is a statement -- they produce `()` or never return:
/// the converter writes no `()` after them as a block's last line, and a
/// round trip treats them with and without their final `;` as one
/// statement (2026-09-30).
pub const UNIT_MACROS: [&str; 14] = [
    "println", "print", "eprintln", "eprint", "assert", "assert_eq", "assert_ne", "debug_assert",
    "debug_assert_eq", "debug_assert_ne", "panic", "todo", "unimplemented", "unreachable",
];

/// Is `toks` a call of one of `UNIT_MACROS`: `println! ..`, `println!(..)`?
pub fn unit_macro_call(toks: &[crate::lex::Token]) -> bool {
    let sig: Vec<&crate::lex::Token> = toks.iter().filter(|t| !t.is_comment()).collect();
    sig.len() >= 2 && UNIT_MACROS.contains(&sig[0].text.as_str()) && sig[1].text == "!"
}

pub const ALWAYS_SEMI: [&str; 7] =
    ["let", "use", "const", "static", "type", "mod", "extern"];

/// Leading keywords that make a *block* a statement needing `;` after the brace.
pub const BLOCK_NEEDS_SEMI: [&str; 6] = ["let", "const", "static", "type", "use", "return"];

/// Rust keywords that are followed by a space before a parenthesis, as in
/// `if (a)` rather than `foo(a)`.
pub const SPACED_KEYWORDS: [&str; 10] = [
    "if", "while", "for", "match", "return", "in", "else", "let", "as", "where",
];

pub fn is_keyword(s: &str) -> bool {
    SPACED_KEYWORDS.contains(&s)
}

use crate::lex::{Tk, Token};

/// The parenthesised parameter groups of a `fn` *declaration* in `toks`:
/// `fn NAME [<generics>] (..) (..) ..`, with each group given as the indices
/// of its `(` and `)`. `None` when the line is not a `fn` declaration with a
/// parenthesised parameter list -- a bare single parameter, a function
/// pointer type (`fn(i32) -> i32` has no name), or no `fn` at all.
///
/// Shared by both directions: the forward direction rejects a comma inside a
/// group, the converter splits Rust's comma list into one group per parameter.
pub fn fn_param_groups(toks: &[Token]) -> Option<Vec<(usize, usize)>> {
    let mut d = 0i32;
    let mut fk = None;
    for (i, t) in toks.iter().enumerate() {
        match t.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Ident if d == 0 && t.text == "fn" => {
                fk = Some(i);
                break;
            }
            _ => {}
        }
    }
    let fk = fk?;
    let mut i = fk + 1;
    if toks.get(i).map(|t| t.kind != Tk::Ident).unwrap_or(true) {
        return None;
    }
    i += 1;
    if toks.get(i).map(|t| t.kind == Tk::Lt).unwrap_or(false) {
        let mut gd = 0i32;
        while i < toks.len() {
            match toks[i].kind {
                Tk::Lt => gd += 1,
                Tk::Gt => {
                    gd -= 1;
                    if gd == 0 {
                        i += 1;
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
    // A documented parameter: doc comments and attributes may stand between
    // the groups, each on its own line.
    while i < toks.len() && toks[i].is_comment() {
        i += 1;
    }
    if toks.get(i).map(|t| t.kind != Tk::Open('(')).unwrap_or(true) {
        return None;
    }
    // Consecutive top-level groups from here; the region ends at the first
    // token that is not a `(` or a comment.
    let mut groups = Vec::new();
    while i < toks.len() && (toks[i].kind == Tk::Open('(') || toks[i].is_comment()) {
        if toks[i].is_comment() {
            i += 1;
            continue;
        }
        let open = i;
        let mut gd = 0i32;
        let mut close = None;
        while i < toks.len() {
            match toks[i].kind {
                Tk::Open('(') => gd += 1,
                Tk::Close(')') => {
                    gd -= 1;
                    if gd == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        groups.push((open, close?));
        i += 1;
    }
    Some(groups)
}

/// Indices of the commas that separate one parameter from the next in a
/// `fn` parameter group -- **for parameter lists only**.
///
/// A comma is a separator only at depth zero of every node, and the nodes
/// compose from small bricks, each applied recursively (the user's framing,
/// 2026-09-21):
///
/// - the lexer's own: a string, a char, `->` and `>>` are single tokens, so
///   the `<` in `"<div "` never reaches here as a `<`;
/// - brackets: `()`, `[]`, `{}` hold their commas;
/// - generic lists: `<…>` holds its commas, `>>` closing two.
///
/// The third brick is safe **only here**. After `:` a parameter holds a type,
/// and in a type `<` can only open a list -- never compare -- so every `<`
/// counts, spaced or not, `Result <i32, String>` as much as `Result<…>`. In
/// an expression `<` may be less-than, and `f (a < b, c > d)` would be
/// misread by this rule; do not call it there. A `>` never takes the depth
/// below zero. Until 2026-09-21 the generic brick was missing, so
/// `(r: Result<i32, String>)` was refused and `hrs-from` split Rust's
/// `fn f(r: Result<i32, String>)` into `(r: Result<i32) (String>)`.
pub fn param_list_commas(toks: &[Token], open: usize, close: usize) -> Vec<usize> {
    let mut d = 0i32;
    let mut angle = 0i32;
    let mut out = Vec::new();
    for i in open + 1..close {
        match toks[i].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Lt => angle += 1,
            Tk::Gt => angle = (angle - 1).max(0),
            Tk::Punct if toks[i].text == ">>" => angle = (angle - 2).max(0),
            Tk::Comma if d == 0 && angle == 0 => out.push(i),
            _ => {}
        }
    }
    out
}

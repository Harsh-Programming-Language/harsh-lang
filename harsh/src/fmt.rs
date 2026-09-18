// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `hrs fmt` -- the formatter. Design in `docs/FMT.md`.
//!
//! The formatter changes no token and, in this step, no line break: it
//! assigns a column to every physical line and re-emits the source with that
//! indentation. So `transpile(fmt(src))` is token-identical to
//! `transpile(src)` by construction, and the tests check it anyway.
//!
//! The rules it applies are the guide's "Layout style", stated once here:
//!
//! - **Unit.** Four spaces; tabs become spaces.
//! - **Block bodies** indent one unit past the construct that opened them:
//!   the statement's column when the opener starts the line, the opener's own
//!   column when it is mid-line (`let f = |x|:` indents from `|x|`;
//!   `<- map (|x|:` from `|x|`, the token after the open paren).
//! - **Closers.** A `)` or `]` first on its line stands under its anchor --
//!   the callee before the bracket, or the bracket itself.
//! - **Bracket groups.** A line inside an open `(` or `[` sits one unit past
//!   the group's anchor.
//! - **Chains.** A continuation line beginning with `<-` aligns under the
//!   first arrow of its chain.
//! - **Continuations** otherwise sit one unit past the statement's first line.
//! - **Braces** are Rust's: lines inside `{ .. }` keep their shape, shifted
//!   with the line that opened them. So do the markup lines of an HSX body.
//! - Trailing whitespace goes; the file ends with one newline.
//!
//! The structure -- which line belongs to which block -- is read from the
//! source's own indentation through `layout::lines`, the never-failing part
//! of the layout pass. The formatter never guesses at structure; it only
//! moves lines to where the structure says they go.

use crate::layout::{self, Line};
use crate::lex::{self, Tk, Token};

pub const UNIT: usize = 4;

/// Format Harsh source. Returns the source unchanged on a lex error: a file
/// that does not lex has nothing to format, and the transpiler will report it.
pub fn format(src: &str) -> String {
    // Two passes: re-break (adds line breaks, never joins), then indent.
    let broken = rebreak(src);
    if std::env::var("HRS_FMT_DEBUG").is_ok() {
        eprintln!("--- rebroken ---\n{broken}--- end ---");
    }
    let Ok(toks) = lex::lex(&broken) else { return src.to_string() };
    let lines = layout::lines(toks);
    let plan = plan(&broken, &lines);
    let out = emit(&broken, &plan);
    // Rule 0, enforced: the formatted source must transpile to the same
    // Rust, token for token. A formatter that reads structure from
    // indentation can misread a shape it has not met; when it does, the
    // file is left as it was rather than changed in meaning.
    if std::env::var("HRS_FMT_DEBUG").is_ok() {
        eprintln!("--- planned ---\n{out}--- end ---");
    }
    match (rust_tokens(src), rust_tokens(&out)) {
        (Some(a), Some(b)) if a == b => out,
        (None, _) => out,
        _ => src.to_string(),
    }
}

/// The transpiled Rust's tokens, comments dropped; `None` when the source
/// does not transpile.
fn rust_tokens(src: &str) -> Option<Vec<String>> {
    let toks = lex::lex(src).ok()?;
    let nodes = layout::build(toks).ok()?;
    let mut em = crate::emit::Emitter::new(src);
    em.program(&nodes);
    let rust = lex::lex_rust(&em.out).ok()?;
    Some(rust.into_iter().filter(|t| !t.is_comment()).map(|t| t.text).collect())
}

/// Step 4 of `docs/FMT.md`: the line breaks the style asks for, added where
/// the source lacks them. Nothing is ever joined, and no token changes.
///
/// - **Break after `=`** when what follows on the same line is a group that
///   stays open past the line (`let row = vec! [` → `let row =` / `vec! [`)
///   or a chain that begins on the `=` line and continues for two or more
///   links (`let s = xs <- iter$` / `<- map …` / `<- collect$` → `let s =` /
///   `xs <- iter$` / …): `=` ends its line in chains. A receiver with a single
///   continuation link stays with its `=`, as the Book writes it.
/// - **Break after `=` before a block literal**: `let u = User\` with the
///   fields beneath becomes `let u =` / `User\` / the fields, so the fields
///   sit one unit past the literal's own column (the user's rule).
/// - **A paren block's closure prototype on its own line**: `<- map (|x|:`
///   with the body beneath and the `)` on its own line becomes `<- map (` /
///   `|x|:`. The compact form, whose `)` ends the body's last line, is kept.
/// - **Never mixed**: in a vertical chain, a line carrying two links is
///   split so every link after the first stands on its own line.
///
/// The Book keeps chains of three and more links horizontal when they fit,
/// so the formatter does not split or join chains by their length.
fn rebreak(src: &str) -> String {
    let Ok(toks) = lex::lex(src) else { return src.to_string() };
    let all = toks.clone();
    let lines = layout::lines(toks);
    // Byte offset of every token that must start a new line, with the
    // indentation to give it (the indent pass fixes the exact column).
    let mut breaks: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    // After a break moves part of a logical line, the line's later physical
    // lines shift by the same amount, so what is inside braces keeps its
    // shape relative to the line that opened it. Keyed by the offset of the
    // logical line's first token: (from offset, shift).
    let mut shifts: Vec<(u32, u32, isize)> = Vec::new();
    // Offsets of tokens whose preceding line break is removed: a short
    // vertical chain of one or two links joined onto its receiver's line.
    let mut joins: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for (li, l) in lines.iter().enumerate() {
        if l.dsl || l.toks.is_empty() {
            continue;
        }
        let t = &l.toks;
        let cont = l.indent + UNIT;
        // A moved part of a header moves its whole block: every following
        // line deeper than the header (body, `else`, the `)` tail).
        let last_hi = {
            let mut hi = t.last().map_or(0, |x| x.span.hi);
            if layout::opens(t).is_some() {
                for n in &lines[li + 1..] {
                    if n.toks.is_empty() {
                        continue;
                    }
                    if n.indent <= l.indent && !n.tail_of_block && !n.toks.first().map_or(false, |x| x.is_kw("else")) {
                        break;
                    }
                    hi = n.toks.last().map_or(hi, |x| x.span.hi).max(hi);
                }
            }
            hi
        };
        // Each break shifts what follows it up to the next break of the
        // same logical line (or the block's end): ranges do not overlap, so
        // several breaks on one line do not compound.
        let mut noted: Vec<(u32, isize)> = Vec::new();
        let mut note = |tok: &Token, ind: usize, breaks: &mut std::collections::HashMap<u32, usize>| {
            if breaks.contains_key(&tok.span.lo) {
                return;
            }
            breaks.insert(tok.span.lo, ind);
            let line_ind = (0..t.len()).rev().find(|&k| t[k].span.lo <= tok.span.lo && t[k].line_start.is_some()).and_then(|k| t[k].line_start).unwrap_or(l.indent);
            noted.push((tok.span.lo, ind as isize - line_ind as isize));
        };
        // A paren block's prototype: `( |x| :` line-final, `)` on its own line.
        if layout::opens(t).is_some() {
            if let Some(o) = innermost_open(t, t.len()) {
                if t[o].kind == Tk::Open('(') && o + 1 < t.len() && t[o + 1].line_start.is_none() {
                    let is_proto = (t[o + 1].kind == Tk::Punct && t[o + 1].text.starts_with('|')) || t[o + 1].is_kw("move");
                    if is_proto && closer_on_own_line(&all, &t[o]) {
                        note(&t[o + 1], cont, &mut breaks);
                    }
                }
            }
        }
        // Chains. A chain is a run of arrows each applied to the result of
        // the one before: `receiver <- lock$ <- unwrap$ <- recv$`. The arrows
        // of `self <- width > other <- width` belong to two operands and are
        // two chains of one link each. Three or more links: vertical, every
        // link after the first on its own line. One or two: one line, unless
        // that line's code would pass `CHAIN_WIDTH` columns.
        // The `)` tail of a paren block continues its header's chain: its
        // links count with the header's (`tail_links` below), and when the
        // whole is vertical every tail link goes on its own line -- the `)`
        // is no receiver.
        for run in chain_runs(t) {
            let first = run[0];
            let rest = &run[1..];
            // The receiver's line: the physical line holding the token
            // before the first arrow.
            let receiver_line = (0..first).rev().find(|&k| t[k].line_start.is_some()).unwrap_or(0);
            let end = chain_end(t, *run.last().unwrap());
            // A chain in operator context -- `a <- x > b <- y` -- is an
            // operand, not a statement's chain: its shape is the author's.
            if !chain_starts_expression(t, first) {
                continue;
            }
            // A paren block's header: the chain goes on after the `)` in
            // the tail line, so its links count here.
            let tail_links = if layout::opens(t).is_some() && innermost_open(t, t.len()).is_some() {
                lines[li + 1..].iter().find(|x| x.tail_of_block).map_or(0, |x| x.toks.iter().enumerate().filter(|(k, y)| y.kind == Tk::LArrow && depth_at(&x.toks, *k) == 0).count())
            } else if l.tail_of_block {
                // A tail line's chain continues its header's: the header's
                // links count here, so `) <- fallback .. <- with_state ..`
                // after `x <- leptos_routes (` is three links -- vertical.
                lines[..li]
                    .iter()
                    .rev()
                    .find(|x| x.indent <= l.indent && !x.tail_of_block && layout::opens(&x.toks).is_some())
                    .map_or(0, |x| x.toks.iter().enumerate().filter(|(k, y)| y.kind == Tk::LArrow && depth_at(&x.toks, *k) == 0).count())
            } else {
                0
            };
            // The receiver decides where the first link goes: it stays on
            // the receiver's line when the arrow sits within `LINK_ALIGN`
            // columns of that line's start (the links then align under it);
            // past that, the receiver stands alone and the first link goes
            // down with the rest, one unit in.
            let recv = receiver_start(t, first);
            let receiver_long = span_width(src, t, recv, first) > LINK_ALIGN;
            if run.len() + tail_links >= 3 {
                for &a in rest {
                    if t[a].line_start.is_none() {
                        breaks.insert(t[a].span.lo, cont);
                    }
                }
                if (receiver_long || l.tail_of_block) && t[first].line_start.is_none() {
                    breaks.insert(t[first].span.lo, cont);
                }
                continue;
            }
            // One or two links: one line when the code fits in
            // `CHAIN_WIDTH`, every link on its own line when it does not.
            // A chain that is the value of a `=` is measured on its own
            // line first: `let x = a <- b <- c` too long as one line breaks
            // after `=` and keeps the chain horizontal if it then fits.
            // One link: unchanged, whatever the length (rule 1 of the
            // user's algorithm) -- neither broken nor moved after its `=`.
            if run.len() + tail_links == 1 {
                continue;
            }
            let eq_before = recv > 0 && t[recv - 1].kind == Tk::Eq && t[recv].line_start.is_none();
            let whole = joined_width(src, t, receiver_line, end);
            let own = if eq_before { cont + span_width(src, t, recv, end) } else { whole };
            if own > CHAIN_WIDTH {
                for &a in rest {
                    if t[a].line_start.is_none() {
                        breaks.insert(t[a].span.lo, cont);
                    }
                }
                if receiver_long && t[first].line_start.is_none() {
                    breaks.insert(t[first].span.lo, cont);
                }
                if eq_before {
                    note(&t[recv], cont, &mut breaks);
                }
            } else if joinable(t, receiver_line, end) {
                if eq_before && whole > CHAIN_WIDTH {
                    note(&t[recv], cont, &mut breaks);
                }
                for &a in &run {
                    if t[a].line_start.is_some() {
                        joins.insert(t[a].span.lo);
                    }
                }
            }
        }
        // Rule 3, recursion: a chain inside a group -- `map (|x| x <- a$ <-
        // b$ <- c$)` -- is judged by the same count from its own line's
        // start. Every matched group at any depth is walked from the token
        // after its `(`; three or more links go vertical.
        {
            let mut i = 0;
            while i < t.len() {
                if t[i].kind == Tk::Open('(') {
                    if let Some(c) = matching_close_from(t, i) {
                        let inner = &t[i + 1..c];
                        for run in chain_runs(inner) {
                            let run: Vec<usize> = run.iter().map(|&k| k + i + 1).collect();
                            if run.len() >= 3 && chain_starts_expression(t, run[0]) {
                                let recv = receiver_start(t, run[0]);
                                let receiver_long = span_width(src, t, recv, run[0]) > LINK_ALIGN;
                                for (n, &a) in run.iter().enumerate() {
                                    if t[a].line_start.is_none() && (n > 0 || receiver_long) {
                                        breaks.insert(t[a].span.lo, cont + UNIT);
                                    }
                                }
                            }
                        }
                    }
                }
                i += 1;
            }
        }
        // Arguments beneath the callee (the user's rule, 2026-09-10): an
        // application whose line would pass `CHAIN_WIDTH`, or one of whose
        // arguments is a block (`(do:`, `(|x|:`), puts every argument on
        // its own line -- all or none, never a staircase. Applied to every
        // application in the line, outermost first; the columns are the
        // indent pass's (one unit past the callee).
        {
            let heads = argument_heads(t, layout::opens(t).is_some());
            // Group argument starts by head, in token order.
            let mut by_head: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
            for (&a, &h) in &heads {
                by_head.entry(h).or_default().push(a);
            }
            // Where an argument start moved to, so an application inside a
            // moved argument is measured from its new column, not its old.
            let mut moved: Vec<(usize, usize, usize)> = Vec::new(); // (arg start, arg end, new col)
            for (h, mut args) in by_head {
                args.sort_unstable();
                // One argument has nothing to list: it stays with its callee.
                if args.len() < 2 {
                    continue;
                }
                let already = args.iter().all(|&a| t[a].line_start.is_some() || breaks.contains_key(&t[a].span.lo));
                if already {
                    continue;
                }
                // The extent: from the head to the end of the last argument's atom.
                let last_start = *args.last().unwrap();
                let last_end = extent_end(t, last_start);
                // A block argument: an open paren whose contents open a block
                // (`(do:`, `(|x|:`) -- the `(` is unmatched within the line.
                // (A lone block argument -- `map (|x|:` -- is the paren
                // block's own shape, the prototype rule's; with a second
                // argument beside it, all go beneath.)
                let has_block = args.len() >= 2 && args.iter().any(|&a| t[a].kind == Tk::Open('(') && matching_close_from(t, a).is_none());
                let head_col = match moved.iter().rev().find(|&&(a, e, _)| a <= h && h < e) {
                    Some(&(a, _, new)) => new + orig_col_of(src, t, h).saturating_sub(orig_col_of(src, t, a)),
                    None => orig_col_of(src, t, h),
                };
                let width = head_col + span_width(src, t, h, last_end);
                if !(has_block || width > CHAIN_WIDTH) {
                    continue;
                }
                let arg_col = head_col + UNIT;
                for &a in &args {
                    if t[a].line_start.is_none() {
                        note(&t[a], arg_col, &mut breaks);
                    }
                    moved.push((a, extent_end(t, a), arg_col));
                }
            }
        }
        // Break after `=`.
        if let Some(eq) = top_level_eq(t) {
            if let Some(first) = (eq + 1..t.len()).find(|&i| !t[i].is_comment()) {
                if t[first].line_start.is_none() {
                    let line_end = (first..t.len()).find(|&i| t[i].line_start.is_some()).unwrap_or(t.len());
                    let open_past_line = (first..line_end).any(|i| matches!(t[i].kind, Tk::Open('(') | Tk::Open('[')) && matching_close_from(t, i).map_or(true, |c| c >= line_end));
                    // A chain that begins on the `=` line and goes on for
                    // two or more links: `=` ends its line. A receiver with
                    // one continuation link stays with its `=`.
                    let on_line = (first..line_end).any(|i| t[i].kind == Tk::LArrow && depth_at(t, i) == 0);
                    let links = (first..t.len())
                        .filter(|&i| t[i].kind == Tk::LArrow && depth_at(t, i) == 0 && (t[i].line_start.is_some() || breaks.contains_key(&t[i].span.lo)) && !joins.contains(&t[i].span.lo))
                        .count();
                    let vertical_chain = on_line && links >= 2;
                    // A value whose arguments were just listed beneath their
                    // callee is vertical too: `=` ends its line.
                    let vertical_args = (first..line_end).any(|i| breaks.contains_key(&t[i].span.lo));
                    // A block literal (the user's rule, 2026-09-10): `let u =
                    // User\` with the fields beneath puts `User\` on its own
                    // line, so the fields sit one unit past the literal's own
                    // column and it is plain what belongs to what.
                    // A match is a specification block in use, like a
                    // literal: `let r =` ends its line, `match n\` starts the
                    // next, and the arms sit one unit past it (the user's
                    // rule, 2026-09-18).
                    let block_literal = layout::opens(t).is_some()
                        && (first..line_end).rev().find(|&i| !t[i].is_comment()).map_or(false, |i| t[i].kind == Tk::Backslash);
                    if open_past_line || vertical_chain || vertical_args || block_literal {
                        note(&t[first], cont, &mut breaks);
                    }
                }
            }
        }
        noted.sort_unstable();
        let line_hi = t.last().map_or(0, |x| x.span.hi);
        for (n, &(lo, d)) in noted.iter().enumerate() {
            // Within the logical line: to the next break.
            let to = noted.get(n + 1).map(|&(next, _)| next).unwrap_or(line_hi);
            shifts.push((lo, to, d));
        }
        // The block beneath a moved header only needs to stay deeper than
        // the header's last line: shifted by what that takes, and no more.
        if let Some(&(_, _)) = noted.last() {
            if last_hi > line_hi {
                let last_ind = noted.last().map(|&(lo, _)| breaks.get(&lo).copied().unwrap_or(0)).unwrap_or(0);
                let body_ind = lines.get(li + 1).map(|n| n.indent).unwrap_or(0);
                if body_ind <= last_ind {
                    shifts.push((line_hi, last_hi, (last_ind + UNIT) as isize - body_ind as isize));
                }
            }
        }
    }
    if breaks.is_empty() && joins.is_empty() {
        return src.to_string();
    }
    let mut out = String::with_capacity(src.len() + 64);
    let mut prev_hi = 0usize;
    for t in &all {
        let lo = t.span.lo as usize;
        match breaks.get(&t.span.lo) {
            Some(&ind) => {
                // Drop the horizontal gap, keep a trailing comment's place.
                let gap = &src[prev_hi..lo];
                let trimmed = gap.trim_end_matches(' ');
                out.push_str(trimmed);
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                for _ in 0..ind {
                    out.push(' ');
                }
            }
            None if joins.contains(&t.span.lo) => out.push(' '),
            None => {
                let gap = &src[prev_hi..lo];
                // A later physical line of a logical line that was moved:
                // its indentation shifts by the same amount.
                let shift: isize = shifts.iter().filter(|&&(from, to, _)| t.span.lo > from && t.span.lo <= to).map(|&(_, _, d)| d).sum();
                if shift != 0 && gap.contains('\n') && t.line_start.is_some() {
                    let nl = gap.rfind('\n').unwrap();
                    out.push_str(&gap[..=nl]);
                    let ind = (t.line_start.unwrap_or(0) as isize + shift).max(0) as usize;
                    for _ in 0..ind {
                        out.push(' ');
                    }
                } else {
                    out.push_str(gap);
                }
            }
        }
        out.push_str(&src[lo..t.span.hi as usize]);
        prev_hi = t.span.hi as usize;
    }
    out.push_str(&src[prev_hi..]);
    out
}

/// Code width past which a chain of one or two links goes vertical.
pub const CHAIN_WIDTH: usize = 72;
/// A first link stays on the receiver's line when its arrow would sit
/// within this many columns of the line's start; the following links then
/// align under it. A longer receiver stands alone, its links one unit in.
pub const LINK_ALIGN: usize = 12;

/// Whether a token may sit between two arrows of one chain: a method or
/// field name, its arguments (atoms and groups), `$`, `?`, `!`, a turbofish.
/// An operator, `=`, `,`, `;`, `:`, `=>`, `as` ends the chain.
fn chain_token(t: &Token) -> bool {
    match t.kind {
        Tk::Ident => !is_kw_text(&t.text) || t.text == "self" || t.text == "Self",
        Tk::Int | Tk::Float | Tk::Str | Tk::Char | Tk::Open(_) | Tk::Close(_) | Tk::Dot | Tk::PathSep | Tk::LArrow => true,
        Tk::Punct => matches!(t.text.as_str(), "$" | "?" | "!" | "&" | "*"),
        _ => false,
    }
}

/// Index just past a turbofish `<..>` opening at `i`, when `toks[i]` is a
/// `<` tight against the name before it; `None` otherwise (a comparison).
fn turbofish_end(toks: &[Token], i: usize) -> Option<usize> {
    if toks[i].kind != Tk::Lt || i == 0 || toks[i - 1].kind != Tk::Ident || toks[i - 1].span.hi != toks[i].span.lo {
        return None;
    }
    let mut d = 0i32;
    for k in i..toks.len() {
        match toks[k].kind {
            Tk::Lt => d += 1,
            Tk::Gt => {
                d -= 1;
                if d == 0 {
                    return Some(k + 1);
                }
            }
            // `>>` closes two.
            Tk::Punct if toks[k].text == ">>" => {
                d -= 2;
                if d <= 0 {
                    return Some(k + 1);
                }
            }
            Tk::Ident | Tk::Comma | Tk::Dot | Tk::PathSep | Tk::Punct | Tk::Open(_) | Tk::Close(_) => {}
            _ => return None,
        }
    }
    None
}

/// Walk a chain from `i` (just after an arrow, or from its receiver) to the
/// token that ends it: returns (index just past the chain's last token,
/// the arrows met at depth zero).
fn chain_walk(toks: &[Token], from: usize) -> (usize, Vec<usize>) {
    let mut arrows = Vec::new();
    let mut d = 0i32;
    let mut i = from;
    while i < toks.len() {
        let t = &toks[i];
        if t.is_comment() {
            break;
        }
        if d == 0 {
            if let Some(e) = turbofish_end(toks, i) {
                i = e;
                continue;
            }
            if t.kind == Tk::LArrow {
                arrows.push(i);
            } else if !chain_token(t) {
                break;
            }
        }
        match t.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => {
                if d == 0 {
                    break;
                }
                d -= 1;
            }
            _ => {}
        }
        i += 1;
    }
    (i, arrows)
}

/// The runs of chained arrows at depth zero: each run is the arrow indices
/// of one chain, in order.
fn chain_runs(toks: &[Token]) -> Vec<Vec<usize>> {
    let mut runs: Vec<Vec<usize>> = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let (end, arrows) = chain_walk(toks, i);
        if !arrows.is_empty() {
            runs.push(arrows);
        }
        i = end.max(i + 1);
    }
    runs
}

/// Index just past the last token of the chain whose last arrow is `last`.
fn chain_end(toks: &[Token], last: usize) -> usize {
    chain_walk(toks, last + 1).0
}

/// Width of the code from the start of the physical line holding `from`
/// to the token before `end`, with every line break inside counted as one
/// space: what the line would measure joined, without its trailing comment.
fn joined_width(src: &str, toks: &[Token], from: usize, end: usize) -> usize {
    let mut w = toks[from].line_start.unwrap_or(0);
    let mut prev_hi: Option<usize> = None;
    for t in &toks[from..end.min(toks.len())] {
        if t.is_comment() {
            break;
        }
        if let Some(ph) = prev_hi {
            let gap = &src[ph..t.span.lo as usize];
            w += if gap.contains('\n') { 1 } else { gap.len() };
        }
        w += (t.span.hi - t.span.lo) as usize;
        prev_hi = Some(t.span.hi as usize);
    }
    w
}

/// May the physical lines from `from` to `end` be joined onto one? Not when
/// a comment sits among them, and not when a group inside them spans lines
/// (a paren block, a multi-line argument).
fn joinable(toks: &[Token], from: usize, end: usize) -> bool {
    let slice = &toks[from..end.min(toks.len())];
    if slice.iter().any(|t| t.is_comment()) {
        return false;
    }
    // A group left open (a paren block's header): the chain goes on in the
    // tail; the author's break stays.
    if depth_at(toks, end.min(toks.len())) > 0 {
        return false;
    }
    // A line break inside a group opened in the chain: keep the author's shape.
    let mut d = 0i32;
    for t in slice {
        if t.line_start.is_some() && d > 0 {
            return false;
        }
        match t.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            _ => {}
        }
    }
    true
}

/// Does the chain whose first arrow is `first` begin an expression -- its
/// receiver preceded by nothing, `=`, `=>`, a comma or a keyword -- rather
/// than sit inside one as an operand (`a <- x > b <- y`)?
fn chain_starts_expression(toks: &[Token], first: usize) -> bool {
    // Back over the receiver: chain tokens, groups skipped.
    let mut i = first;
    let mut d = 0i32;
    while i > 0 {
        let p = &toks[i - 1];
        match p.kind {
            Tk::Close(_) => d += 1,
            Tk::Open(_) => {
                if d == 0 {
                    break;
                }
                d -= 1;
            }
            _ if d == 0 && !chain_token(p) => break,
            _ => {}
        }
        i -= 1;
    }
    if i == 0 {
        return true;
    }
    let p = &toks[i - 1];
    matches!(p.kind, Tk::Eq | Tk::FatArrow | Tk::Comma | Tk::Colon | Tk::Semi | Tk::Open(_))
        || (p.kind == Tk::Ident && is_kw_text(&p.text))
        // A closure's body begins after its prototype's closing bar.
        || (p.kind == Tk::Punct && p.text.ends_with('|'))
}

/// The first token of the receiver of the chain whose first arrow is `first`.
fn receiver_start(toks: &[Token], first: usize) -> usize {
    let mut i = first;
    let mut d = 0i32;
    while i > 0 {
        let p = &toks[i - 1];
        match p.kind {
            Tk::Close(_) => d += 1,
            Tk::Open(_) => {
                if d == 0 {
                    break;
                }
                d -= 1;
            }
            _ if d == 0 && !chain_token(p) => break,
            _ => {}
        }
        i -= 1;
    }
    i
}

/// Width of the tokens `from..end` written on one line, with their gaps.
fn span_width(src: &str, toks: &[Token], from: usize, end: usize) -> usize {
    let mut w = 0usize;
    let mut prev_hi: Option<usize> = None;
    for t in &toks[from..end.min(toks.len())] {
        if t.is_comment() {
            break;
        }
        if let Some(ph) = prev_hi {
            let gap = &src[ph..t.span.lo as usize];
            w += if gap.contains('\n') { 1 } else { gap.len() };
        }
        w += (t.span.hi - t.span.lo) as usize;
        prev_hi = Some(t.span.hi as usize);
    }
    w
}

/// The first `=` at bracket depth zero.
fn top_level_eq(toks: &[Token]) -> Option<usize> {
    let mut d = 0i32;
    for (i, t) in toks.iter().enumerate() {
        match t.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Eq if d == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

fn depth_at(toks: &[Token], i: usize) -> i32 {
    let mut d = 0i32;
    for t in &toks[..i] {
        match t.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            _ => {}
        }
    }
    d
}

fn matching_close_from(toks: &[Token], i: usize) -> Option<usize> {
    let mut d = 0i32;
    for k in i..toks.len() {
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

/// Does the bracket `open` (a token of the whole file) close with a `)`
/// that starts its own line? Searched over the whole token stream, since a
/// paren block's `)` is on a later logical line.
fn closer_on_own_line(all: &[Token], open: &Token) -> bool {
    let Some(i) = all.iter().position(|t| t.span.lo == open.span.lo) else { return false };
    match matching_close_from(all, i) {
        Some(c) => all[c].line_start.is_some(),
        None => false,
    }
}

/// New indentation for each source line (zero-based), or `None` to leave the
/// line exactly as it is (inside a multi-line token, or blank).
struct Plan {
    indent: Vec<Option<usize>>,
}

/// Byte offset of the start of each source line.
fn line_starts(src: &str) -> Vec<usize> {
    let mut v = vec![0usize];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            v.push(i + 1);
        }
    }
    v
}

/// Column of a token in the original source, from its byte offset. The first
/// token of a physical line uses the lexer's tab-aware indent instead.
fn orig_col(t: &Token, starts: &[usize]) -> usize {
    if let Some(c) = t.line_start {
        return c;
    }
    let ls = starts.get(t.line.saturating_sub(1)).copied().unwrap_or(0);
    (t.span.lo as usize).saturating_sub(ls)
}

/// Index of the innermost `(` or `[` left open at the end of `toks[..end]`,
/// counted from the start of the slice; `None` when nothing is open or the
/// innermost open bracket is `{`.
fn innermost_open(toks: &[Token], end: usize) -> Option<usize> {
    let mut open: Vec<usize> = Vec::new();
    for (i, t) in toks[..end].iter().enumerate() {
        match t.kind {
            Tk::Open(_) => open.push(i),
            Tk::Close(_) => {
                open.pop();
            }
            _ => {}
        }
    }
    let i = *open.last()?;
    match toks[i].kind {
        Tk::Open('(') | Tk::Open('[') => Some(i),
        _ => None,
    }
}

/// The innermost open bracket of any kind at the end of `toks[..end]`.
fn innermost_open_any(toks: &[Token], end: usize) -> Option<usize> {
    let mut open: Vec<usize> = Vec::new();
    for (i, t) in toks[..end].iter().enumerate() {
        match t.kind {
            Tk::Open(_) => open.push(i),
            Tk::Close(_) => {
                open.pop();
            }
            _ => {}
        }
    }
    open.last().copied()
}

/// Index of the anchor token of the bracket at `open`: the start of the
/// callee path before it (`compute (`, `foo.bar (`, `vec![`), else the
/// bracket itself.
fn anchor_index(toks: &[Token], open: usize) -> usize {
    // `[where ..]` is Harsh's own construct, not an index: the bracket is
    // its anchor, whatever type precedes it.
    if toks.get(open + 1).map_or(false, |t| t.is_kw("where")) {
        return open;
    }
    // Walk back over the callee path, and over the complete groups of the
    // earlier arguments: in `compute (1 + 1) (`, the anchor is `compute`.
    let mut i = open;
    while i > 0 {
        let t = &toks[i - 1];
        let path_part = (t.kind == Tk::Ident && !is_kw_text(&t.text))
            || matches!(t.kind, Tk::Dot | Tk::PathSep)
            || (t.kind == Tk::Punct && (t.text == "!" || t.text == "$"));
        if path_part {
            i -= 1;
            continue;
        }
        if t.kind == Tk::Close(')') {
            if let Some(o) = matching_open(toks, i - 1) {
                if o > 0 && toks[o - 1].kind == Tk::Ident && !is_kw_text(&toks[o - 1].text) && toks[o].line_start.is_none() {
                    i = o;
                    continue;
                }
            }
        }
        break;
    }
    i
}

/// Index of the first `<-` at bracket depth zero in the logical line.
/// For every application in the line, each argument's first token mapped
/// to the callee's first token (a path's first segment). From the
/// juxtaposition fixups: `Open` marks the first argument, `Comma` the rest.
fn argument_heads(toks: &[Token], is_header: bool) -> std::collections::HashMap<usize, usize> {
    let mut out = std::collections::HashMap::new();
    let jx = crate::juxt::fixups(toks, true, is_header);
    // Group the fixups: each Open begins an argument list whose head is the
    // atom right before it; every Comma until the matching Close belongs to it.
    let mut head_of_open: Vec<(usize, usize)> = Vec::new(); // (open index, head)
    for (i, j) in &jx {
        if let crate::juxt::Jx::Open = j {
            let h = (0..*i).rev().find(|&k| !toks[k].is_comment());
            if let Some(mut h) = h {
                // A macro's bang, then back over a path: `a.b.c!`.
                if h > 0 && toks[h].kind == Tk::Punct && toks[h].text == "!" {
                    h -= 1;
                }
                while h >= 2 && matches!(toks[h - 1].kind, Tk::Dot | Tk::PathSep) && toks[h - 2].kind == Tk::Ident {
                    h -= 2;
                }
                head_of_open.push((*i, h));
                out.insert(*i, h);
            }
        }
    }
    for (i, j) in &jx {
        if let crate::juxt::Jx::Comma = j {
            // The nearest Open before it whose list is still open at `i`.
            let mut best: Option<usize> = None;
            for &(o, h) in head_of_open.iter().rev() {
                if o < *i {
                    // Is `i` inside the argument list that starts at `o`?
                    let mut depth = 0i32;
                    let mut inside = true;
                    for k in o..*i {
                        match toks[k].kind {
                            Tk::Open(_) => depth += 1,
                            Tk::Close(_) => {
                                depth -= 1;
                                if depth < 0 {
                                    inside = false;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    if inside && depth == 0 {
                        best = Some(h);
                        break;
                    }
                }
            }
            if let Some(h) = best {
                out.insert(*i, h);
            }
        }
    }
    out
}

/// The token index just past the atom that starts at `a`: a group to its
/// closer, else the token itself (and a following `$`).
fn extent_end(toks: &[Token], a: usize) -> usize {
    if let Tk::Open(_) = toks[a].kind {
        if let Some(c) = matching_close_from(toks, a) {
            return c + 1;
        }
        return toks.len();
    }
    let mut e = a + 1;
    while e < toks.len() && ((toks[e].kind == Tk::Punct && toks[e].text == "$") || matches!(toks[e].kind, Tk::Dot | Tk::PathSep) || (e > 0 && matches!(toks[e - 1].kind, Tk::Dot | Tk::PathSep) && toks[e].kind == Tk::Ident)) {
        e += 1;
    }
    e
}

/// A token's source column.
fn orig_col_of(src: &str, toks: &[Token], i: usize) -> usize {
    let lo = toks[i].span.lo as usize;
    let line_start = src[..lo].rfind('\n').map(|p| p + 1).unwrap_or(0);
    lo - line_start
}

/// The first arrow of the chain the arrow at `a` belongs to: back over the
/// chain's tokens (groups skipped) to the receiver's start, then forward
/// to the first arrow.
fn chain_first_arrow(toks: &[Token], a: usize) -> usize {
    let recv = receiver_start(toks, a);
    let mut d = 0i32;
    let mut k = recv;
    while k <= a {
        match toks[k].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::LArrow if d == 0 => return k,
            _ => {}
        }
        k += 1;
    }
    a
}

fn first_arrow(toks: &[Token]) -> Option<usize> {
    let mut d = 0usize;
    for (i, t) in toks.iter().enumerate() {
        match t.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d = d.saturating_sub(1),
            Tk::LArrow if d == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// A Rust keyword: never part of a callee path.
fn is_kw_text(s: &str) -> bool {
    crate::rules::is_keyword(s)
        || crate::rules::BLOCK_KEYWORDS.contains(&s)
        || crate::rules::ALWAYS_SEMI.contains(&s)
        || crate::rules::MODIFIERS.contains(&s)
        || matches!(s, "do" | "macro_rules" | "loop" | "break" | "continue" | "ref" | "mut" | "dyn" | "self" | "Self" | "super" | "crate")
}

fn is_markup_first(l: &Line) -> bool {
    l.toks.first().map_or(false, |t| t.kind == Tk::Lt)
}

/// One open block, as the walk sees it.
struct Frame {
    /// Indent of the header in the original source: structure is read from
    /// the source's own columns.
    header_orig: usize,
    /// Original and new column of the construct that opened the block (the
    /// `if` of `let x = if c:`): an `else` at that column belongs to it.
    opener_orig: usize,
    opener_new: usize,
    /// Where the body's lines go.
    body_new: usize,
    /// New column of the header's innermost open bracket's anchor, for the
    /// `)` tail of a paren block.
    anchor_new: Option<usize>,
    /// The column a chain link continuing the header's statement takes,
    /// once the paren block has closed.
    chain_new: Option<usize>,
    /// The header opened an HSX body: markup lines keep their shape.
    hsx: bool,
    /// Shift applied to markup lines under this header.
    hsx_shift: Option<isize>,
}

fn plan(src: &str, lines: &[Line]) -> Plan {
    let n_lines = src.lines().count() + 1;
    let mut indent: Vec<Option<usize>> = vec![None; n_lines + 1];
    let starts = line_starts(src);
    let mut stack: Vec<Frame> = Vec::new();
    // The frame the last tail line closed.
    let mut closed: Option<Frame>;

    for (li, l) in lines.iter().enumerate() {
        if l.toks.is_empty() {
            continue;
        }
        // Close blocks this line has stepped out of.
        closed = None;
        while let Some(f) = stack.last() {
            if l.indent <= f.header_orig {
                closed = stack.pop();
            } else {
                break;
            }
        }
        // An `else` at the column of the `if` that opened the block it
        // follows -- `let x = if c:` puts that `if` mid-line -- closes the
        // block and sits under the `if`.
        let is_else = l.toks.first().map_or(false, |t| t.is_kw("else"));
        let mut else_col: Option<usize> = None;
        // An `else` inherits the anchor of the `if` it continues: the `)`
        // that closes a group opened around them both finds it there.
        let mut inherited_anchor: Option<usize> = None;
        if is_else {
            if let Some(f) = closed.as_ref().filter(|f| f.opener_orig == l.indent) {
                else_col = Some(f.opener_new);
                inherited_anchor = f.anchor_new;
            } else if let Some(f) = stack.last() {
                if f.opener_orig == l.indent && f.opener_orig > f.header_orig {
                    else_col = Some(f.opener_new);
                    inherited_anchor = f.anchor_new;
                    stack.pop();
                }
            }
        }

        // Markup inside an HSX body: keep its shape, shifted with the header.
        if let Some(f) = stack.last_mut() {
            if f.hsx {
                let shift = *f.hsx_shift.get_or_insert(f.body_new as isize - l.indent as isize);
                for t in l.comments.iter().chain(l.toks.iter()) {
                    if let Some(c) = t.line_start {
                        set(&mut indent, t.line, (c as isize + shift).max(0) as usize);
                    }
                }
                continue;
            }
        }

        // The column of the logical line's first physical line.
        // A `)` tail closes the paren block whose header holds its `(`: the
        // nearest frame with an anchor, which may sit under an `if`/`else`
        // frame opened inside the group. Pop through it.
        if l.tail_of_block {
            if closed.as_ref().map_or(true, |f| f.anchor_new.is_none()) {
                if let Some(pos) = stack.iter().rposition(|f| f.anchor_new.is_some()) {
                    let f = stack.remove(pos);
                    stack.truncate(pos);
                    closed = Some(f);
                }
            }
        }
        let first_new = if let Some(c) = else_col {
            c
        } else if l.tail_of_block && l.toks.first().map_or(false, |t| t.line_start.is_none()) {
            // The `)` ended the body's last line (the compact form); the
            // tail's own first line is a chain link, at the chain column.
            closed.as_ref().and_then(|f| f.chain_new).unwrap_or_else(|| stack.last().map_or(0, |f| f.body_new))
        } else if l.tail_of_block {
            closed.as_ref().and_then(|f| f.anchor_new).unwrap_or_else(|| stack.last().map_or(0, |f| f.body_new))
        } else {
            stack.last().map_or(0, |f| f.body_new)
        };

        if std::env::var("HRS_FMT_DEBUG").is_ok() {
            eprintln!("line indent={} tail={} else={:?} first_new={} stack={:?}", l.indent, l.tail_of_block, else_col, first_new, stack.iter().map(|f| (f.header_orig, f.opener_orig, f.body_new)).collect::<Vec<_>>());
        }
        // Comment-only lines before it take its column.
        for c in &l.comments {
            if c.line_start.is_some() {
                set(&mut indent, c.line, first_new);
            }
        }

        // Physical lines of this logical line, in order.
        let phys: Vec<usize> = l.toks.iter().enumerate().filter(|(_, t)| t.line_start.is_some()).map(|(i, _)| i).collect();
        // New column of every token, filled in as its physical line is placed.
        let mut new_col: Vec<usize> = vec![0; l.toks.len()];
        let place = |from: usize, to: usize, new_indent: usize, indent: &mut Vec<Option<usize>>, new_col: &mut Vec<usize>| {
            let t0 = &l.toks[from];
            let orig_indent = t0.line_start.unwrap_or(0);
            set(indent, t0.line, new_indent);
            for k in from..to {
                let oc = orig_col(&l.toks[k], &starts);
                new_col[k] = (new_indent + oc).saturating_sub(orig_indent);
            }
        };
        let mut prev_new: Option<usize> = None;
        // Continuation lines keep their relative nesting: a line the author
        // indented past the previous continuation goes one unit past it.
        let mut cont_stack: Vec<(usize, usize)> = Vec::new(); // (orig indent, new col)
        // The column a chain link takes in this statement, for the tail.
        let link_col = |toks: &[Token], new_col: &[usize], start: usize, first_new: usize| -> usize {
            match first_arrow(toks) {
                Some(a) if a < start && (toks[a].line_start.is_some() || !receiver_is_long(toks, a)) => new_col[a],
                Some(a) if a < start => {
                    let ls = (0..=a).rev().find(|&k| toks[k].line_start.is_some()).unwrap_or(0);
                    new_col[ls] + UNIT
                }
                // This link is the chain's first arrow: one unit past the
                // receiver's line (the line of the token before it).
                _ if start > 0 => {
                    let ls = (0..start).rev().find(|&k| toks[k].line_start.is_some()).unwrap_or(0);
                    new_col[ls] + UNIT
                }
                _ => first_new + UNIT,
            }
        };
        let tail_link = if l.tail_of_block { closed.as_ref().and_then(|f| f.chain_new) } else { None };
        // Arguments listed beneath their callee: every argument start of
        // every application in the line, with its callee's first token.
        let arg_heads = argument_heads(&l.toks, layout::opens(&l.toks).is_some());
        for (p, &start) in phys.iter().enumerate() {
            let end = phys.get(p + 1).copied().unwrap_or(l.toks.len());
            let col = if p == 0 {
                first_new
            } else {
                let t = &l.toks[start];
                let orig_indent = t.line_start.unwrap_or(0);
                match innermost_open_any(&l.toks, start) {
                    // Inside a brace: Rust's. Keep the line's shape relative
                    // to the previous physical line.
                    Some(o) if l.toks[o].kind == Tk::Open('{') => {
                        let prev_start = phys[p - 1];
                        let prev_orig = l.toks[prev_start].line_start.unwrap_or(0);
                        (prev_new.unwrap_or(0) as isize + orig_indent as isize - prev_orig as isize).max(0) as usize
                    }
                    _ => {
                        if matches!(t.kind, Tk::Close(_)) {
                            // A closer first on its line: under its anchor.
                            match matching_open(&l.toks, start) {
                                Some(o) => new_col[anchor_index(&l.toks, o)],
                                None => first_new,
                            }
                        } else if let Some(&h) = arg_heads.get(&start) {
                            // An argument first on its line: one unit past
                            // its callee (the arguments-beneath-the-callee
                            // rule), inside a group or not.
                            new_col[h] + UNIT
                        } else if t.kind == Tk::LArrow && innermost_open(&l.toks, start).is_some() {
                            // A link first on its line inside a group: its own
                            // chain's rule -- under the chain's first arrow
                            // when the receiver is short, one unit past the
                            // receiver's line when long.
                            let fa = chain_first_arrow(&l.toks, start);
                            if fa < start && (l.toks[fa].line_start.is_some() || !receiver_is_long(&l.toks, fa)) {
                                new_col[fa]
                            } else {
                                let ls = (0..=fa).rev().find(|&k| l.toks[k].line_start.is_some()).unwrap_or(0);
                                new_col[ls] + UNIT
                            }
                        } else if let Some(o) = innermost_open(&l.toks, start) {
                            // Inside an open group: one unit past the anchor.
                            new_col[anchor_index(&l.toks, o)] + UNIT
                        } else if t.kind == Tk::LArrow {
                            // A chain link: under the first arrow when it
                            // starts its line or follows a single name; one
                            // unit past the receiver's line when the receiver
                            // is longer. After a paren block's `)`, the
                            // header's chain column.
                            match tail_link {
                                Some(c) => c,
                                None => link_col(&l.toks, &new_col, start, first_new),
                            }
                        } else {
                            // A plain continuation: one unit past the
                            // statement, or past the continuation it nests in.
                            while cont_stack.last().map_or(false, |&(o, _)| o >= orig_indent) {
                                cont_stack.pop();
                            }
                            let col = cont_stack.last().map_or(first_new + UNIT, |&(_, c)| c + UNIT);
                            cont_stack.push((orig_indent, col));
                            col
                        }
                    }
                }
            };
            place(start, end, col, &mut indent, &mut new_col);
            prev_new = Some(col);
        }

        // Does this line open a block?
        if layout::opens(&l.toks).is_some() {
            let last_phys = *phys.last().unwrap_or(&0);
            let opener_idx = opener_token(&l.toks).unwrap_or(last_phys);
            let opener_new = new_col[opener_idx];
            let anchor_new = innermost_open(&l.toks, l.toks.len()).map(|i| new_col[anchor_index(&l.toks, i)]).or(inherited_anchor);
            let hsx = macro_do_header(&l.toks) && lines.get(li + 1).map_or(false, |n| n.indent > l.indent && is_markup_first(n));
            let opener_orig = orig_col(&l.toks[opener_idx], &starts);
            let chain_new = if innermost_open(&l.toks, l.toks.len()).is_some() {
                Some(tail_link.unwrap_or_else(|| link_col(&l.toks, &new_col, l.toks.len(), first_new)))
            } else {
                None
            };
            stack.push(Frame {
                header_orig: l.indent,
                opener_orig,
                opener_new: new_col[opener_idx],
                body_new: first_new_body(&l.toks, opener_new, first_new),
                anchor_new,
                chain_new,
                hsx,
                hsx_shift: None,
            });
        }
    }
    Plan { indent }
}

/// The column a block's body indents from: one unit past this. It is the
/// start of the physical line holding the construct that opened the block --
/// `let guess = match n:` indents from `let`, `let sign =` / `if n > 0:`
/// from the `if` line, `fn f ..` / `[where ..]:` from the `fn` line, a
/// `=>` arm from itself. One exception, decided for the editor and kept: a
/// block opened inside a group on the same line as its `(` -- the compact
/// `<- map (|x|:` -- indents from the construct after the `(`.
fn opener_token(toks: &[Token]) -> Option<usize> {
    let sig: Vec<usize> = (0..toks.len()).filter(|&i| !toks[i].is_comment()).collect();
    let &colon = sig.last()?;
    let line_start_of = |i: usize| (0..=i).rev().find(|&k| toks[k].line_start.is_some()).unwrap_or(0);
    if toks[colon].kind != Tk::Colon {
        return Some(line_start_of(colon));
    }
    // Opened inside a group on this line?
    let mut d = 0i32;
    let mut i = colon;
    while i > 0 {
        i -= 1;
        match toks[i].kind {
            Tk::Close(_) => d += 1,
            Tk::Open(_) => {
                d -= 1;
                if d < 0 {
                    let c = i + 1;
                    return Some(if toks.get(c).map_or(true, |t| t.line_start.is_some()) { line_start_of(c.min(toks.len() - 1)) } else { c });
                }
            }
            _ => {}
        }
    }
    // A closure prototype before the colon: its line.
    if colon > 0 && toks[colon - 1].kind == Tk::Punct && toks[colon - 1].text.ends_with('|') {
        return Some(line_start_of(colon - 1));
    }
    // The first block keyword at depth zero, from the front: its line.
    let mut d = 0i32;
    for &k in &sig {
        match toks[k].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Ident if d == 0 => {
                let kw = toks[k].text.as_str();
                if kw == "do" || kw == "macro_rules" || crate::rules::BLOCK_KEYWORDS.contains(&kw) {
                    return Some(line_start_of(k));
                }
            }
            _ => {}
        }
    }
    Some(line_start_of(colon))
}

/// Is the receiver of the arrow at `a` -- what stands between the start of
/// its physical line (or the last `=`) and the arrow -- longer than
/// `LINK_ALIGN`? Then its links do not align under the arrow but hang one
/// unit in from the receiver's line.
fn receiver_is_long(toks: &[Token], a: usize) -> bool {
    // What alignment costs is the arrow's column from where its chain's
    // line begins: the physical line's start (or the `=` on it) for a chain
    // at depth zero -- `for (i, w) in words <- iter$` counts the `for` --
    // and the chain's own receiver for one inside a group.
    let from = if innermost_open(toks, a).is_some() {
        receiver_start(toks, a)
    } else {
        let ls = (0..=a).rev().find(|&k| toks[k].line_start.is_some()).unwrap_or(0);
        (ls..a).rev().find(|&k| toks[k].kind == Tk::Eq).map(|k| k + 1).unwrap_or(ls)
    };
    let lo = toks[from..a].iter().find(|t| !t.is_comment()).map(|t| t.span.lo);
    match lo {
        Some(lo) => (toks[a].span.lo - lo) as usize > LINK_ALIGN,
        None => true,
    }
}

/// Where a block's body goes: one unit past the opener.
fn first_new_body(_toks: &[Token], opener_new: usize, _line_new: usize) -> usize {
    opener_new + UNIT
}

fn macro_do_header(toks: &[Token]) -> bool {
    let sig: Vec<&Token> = toks.iter().filter(|t| !t.is_comment()).collect();
    let n = sig.len();
    n >= 4 && sig[n - 1].kind == Tk::Colon && sig[n - 2].is_kw("do") && sig[n - 3].kind == Tk::Punct && sig[n - 3].text == "!" && sig[n - 4].kind == Tk::Ident
}

/// The `(` / `[` / `{` closed by the closer at `i`.
fn matching_open(toks: &[Token], i: usize) -> Option<usize> {
    let mut d = 0i32;
    let mut k = i;
    loop {
        match toks[k].kind {
            Tk::Close(_) => d += 1,
            Tk::Open(_) => {
                d -= 1;
                if d == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
        if k == 0 {
            return None;
        }
        k -= 1;
    }
}

fn set(indent: &mut Vec<Option<usize>>, line_1based: usize, col: usize) {
    if line_1based == 0 {
        return;
    }
    let i = line_1based - 1;
    if i >= indent.len() {
        indent.resize(i + 1, None);
    }
    indent[i] = Some(col);
}

fn emit(src: &str, plan: &Plan) -> String {
    let mut out = String::with_capacity(src.len() + 16);
    for (i, line) in src.split('\n').enumerate() {
        let body = line.trim_end();
        match plan.indent.get(i).copied().flatten() {
            Some(col) if !body.trim().is_empty() => {
                for _ in 0..col {
                    out.push(' ');
                }
                out.push_str(body.trim_start());
            }
            // A line inside a multi-line token is not the formatter's; a
            // blank line loses its trailing whitespace.
            None if !line.trim().is_empty() => out.push_str(line),
            _ => out.push_str(body),
        }
        out.push('\n');
    }
    // One newline at the end of the file, no more.
    while out.ends_with("\n\n") {
        out.pop();
    }
    if out == "\n" {
        out.clear();
    }
    out
}

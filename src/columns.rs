// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Where may the next line start?
//!
//! The one question on-type formatting asks, answered from layout alone. The
//! server reads token kinds, spans and the two layout rules through
//! `layout::lines`; it never compares identifiers and never reads the arity
//! table. Everything it returns is a column. See `docs/LSP.md`.

use crate::layout::{self, Line};
use crate::lex::{self, Tk, Token};

/// The columns a fresh line may start at, for a cursor placed on `line`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Columns {
    /// Every legal starting column, ascending, without duplicates.
    pub legal: Vec<usize>,
    /// Where Enter lands.
    pub default: usize,
    /// The indentation unit detected from the buffer (4 when it has none).
    pub unit: usize,
    /// Where a `)` or `]` typed at the start of this line belongs: under the
    /// anchor of the innermost open bracket group. `None` when no `(` or `[`
    /// is open -- `{` is Rust's and gets no answer.
    pub closer: Option<usize>,
}

/// Compute the columns for a cursor on zero-based `line` of `src`. Only the
/// text above that line is read.
pub fn columns(src: &str, line: usize) -> Columns {
    let prefix = prefix_before(src, line);
    let toks = lex_tolerant(prefix);
    // The functions declared so far, with their arities: a line that applies
    // one to fewer arguments than it takes is waiting for the rest.
    let arities = crate::juxt::collect_arities(&toks);
    let lines = layout::lines(toks);
    let unit = detect_unit(&lines);
    let mut c = from_lines_with(&lines, unit, &arities);
    // The line just above is blank: its indentation is the clearest
    // statement of intent there is -- the editor put the cursor there, or
    // the user moved it with Tab, Shift-Tab or spaces -- so the new line
    // copies it. Blank lines mean nothing to the transpiler; to the editor
    // they are where overrides live. Tab still snaps to the grid from there.
    if let Some(blank) = blank_line_above(prefix) {
        c.default = blank;
        if !c.legal.contains(&blank) {
            c.legal.push(blank);
            c.legal.sort_unstable();
        }
    }
    c
}

/// Indentation of the last line of `prefix` if that line is whitespace only.
fn blank_line_above(prefix: &str) -> Option<usize> {
    let body = prefix.strip_suffix('\n')?;
    let last = body.rsplit('\n').next().unwrap_or(body);
    let last = last.trim_end_matches('\r');
    if last.chars().all(|c| c == ' ') {
        Some(last.len())
    } else {
        None
    }
}

/// The source up to and including the line before `line`.
fn prefix_before(src: &str, line: usize) -> &str {
    let mut end = 0usize;
    for (i, l) in src.split_inclusive('\n').enumerate() {
        if i >= line {
            break;
        }
        end += l.len();
    }
    &src[..end]
}

/// Lex, and on an unterminated literal or comment drop it and lex what came
/// before. A fragment that never closed cannot have opened or closed a block.
fn lex_tolerant(src: &str) -> Vec<Token> {
    let mut s = src;
    loop {
        match lex::lex(s) {
            Ok(t) => return t,
            Err(e) => {
                let lo = e.span.lo as usize;
                if lo == 0 || lo > s.len() {
                    return Vec::new();
                }
                s = &s[..lo];
            }
        }
    }
}

/// The smallest positive step between the indents of consecutive logical
/// lines; 4 when the buffer has none.
fn detect_unit(lines: &[Line]) -> usize {
    let mut best: Option<usize> = None;
    for w in lines.windows(2) {
        if w[1].indent > w[0].indent {
            let d = w[1].indent - w[0].indent;
            best = Some(best.map_or(d, |b| b.min(d)));
        }
    }
    best.unwrap_or(4)
}

/// Columns of every token in a logical line, from the spans and the
/// `line_start` of each physical line's first token.
fn token_cols(l: &Line) -> Vec<usize> {
    let mut origin = 0usize;
    l.toks
        .iter()
        .map(|t| {
            if let Some(c) = t.line_start {
                origin = t.span.lo as usize - c;
            }
            t.span.lo as usize - origin
        })
        .collect()
}

/// Index of the first token of the last physical line of a logical line --
/// the construct the style says a block indents from.
fn last_physical_start(l: &Line) -> Option<usize> {
    l.toks.iter().rposition(|t| t.line_start.is_some())
}

/// Columns of every `<-` at bracket depth zero from token `from` on. Depth
/// is counted from the start of the logical line, so an arrow inside a
/// paren opened on an earlier physical line is not a chain arrow.
/// When the line ends in a literal's `\`, the index of the first token of
/// the literal's name (a path: `geometry.Vec2\`); `None` otherwise.
fn literal_head(l: &Line) -> Option<usize> {
    let last = l.toks.iter().rposition(|t| !t.is_comment())?;
    if l.toks[last].kind != Tk::Backslash {
        return None;
    }
    let mut h = last;
    while h > 0 {
        let p = &l.toks[h - 1];
        let joined = matches!(p.kind, Tk::Ident | Tk::Dot | Tk::Lt | Tk::Gt | Tk::Comma) && p.span.hi == l.toks[h].span.lo;
        if !joined {
            break;
        }
        h -= 1;
    }
    Some(h)
}

/// Does the line end in an application of a declared function to fewer
/// arguments than its parameters? Then the column of that callee.
fn awaiting_arguments(l: &Line, arities: &std::collections::HashMap<String, usize>) -> Option<usize> {
    let sig: Vec<usize> = (0..l.toks.len()).filter(|&i| !l.toks[i].is_comment()).collect();
    if sig.is_empty() {
        return None;
    }
    // The expression: after `let x =` / `x =`, or the whole line.
    let mut d = 0i32;
    let mut start = 0usize;
    for (n, &i) in sig.iter().enumerate() {
        match l.toks[i].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Eq if d == 0 => start = n + 1,
            _ => {}
        }
    }
    let expr: &[usize] = &sig[start..];
    let Some(&head) = expr.first() else { return None };
    let ht = &l.toks[head];
    if ht.kind != Tk::Ident || crate::rules::is_keyword(&ht.text) {
        return None;
    }
    let callee_col = || {
        // The callee's column: its physical line's start plus its offset
        // from that line's first token.
        let ls = (0..=head).rev().find(|&k| l.toks[k].line_start.is_some()).unwrap_or(0);
        let base = l.toks[ls].line_start.unwrap_or(l.indent);
        base + (l.toks[head].span.lo - l.toks[ls].span.lo) as usize
    };
    // A line ending in a macro's `!` cannot be complete: `println!` alone is
    // not an expression -- a macro applied to nothing is `m!$` -- so what
    // follows is its first argument. Name-blind, and no arity: a macro takes
    // one argument or six, and the arity table is the pipes' alone.
    let last = *expr.last()?;
    if l.toks[last].kind == Tk::Punct
        && l.toks[last].text == "!"
        && last > 0
        && l.toks[last - 1].kind == Tk::Ident
    {
        return Some(callee_col());
    }
    // The callee: a path's last segment.
    let mut k = 0;
    while k + 2 < expr.len() && l.toks[expr[k + 1]].kind == Tk::Dot && l.toks[expr[k + 2]].kind == Tk::Ident {
        k += 2;
    }
    let name = &l.toks[expr[k]].text;
    let arity = *arities.get(name)?;
    if arity == 0 {
        return None;
    }
    // The arguments so far: atoms at depth zero after the callee; a `$`
    // means "applied to nothing", complete.
    let rest = &expr[k + 1..];
    if rest.first().map_or(false, |&i| l.toks[i].kind == Tk::Punct && l.toks[i].text == "$") {
        return None;
    }
    let mut args = 0usize;
    let mut d = 0i32;
    for &i in rest {
        match l.toks[i].kind {
            Tk::Open(_) => {
                if d == 0 {
                    args += 1;
                }
                d += 1;
            }
            Tk::Close(_) => d -= 1,
            Tk::LArrow | Tk::PipeBack | Tk::PipeFwd if d == 0 => return None,
            _ if d == 0 => args += 1,
            _ => {}
        }
    }
    if args < arity {
        Some(callee_col())
    } else {
        None
    }
}

fn arrow_cols(l: &Line, from: usize) -> Vec<usize> {
    let cols = token_cols(l);
    let mut depth = 0usize;
    let mut out = Vec::new();
    for (i, t) in l.toks.iter().enumerate() {
        match t.kind {
            Tk::Open(_) => depth += 1,
            Tk::Close(_) => depth = depth.saturating_sub(1),
            Tk::LArrow if depth == 0 && i >= from => out.push(cols[i]),
            _ => {}
        }
    }
    out
}

/// Arrow columns of the physical line that token `from` starts, and no
/// further: a chain continues under the *last* arrow of the line above, and
/// Shift-Tab walks back through the earlier ones.
fn line_arrow_cols(l: &Line, from: usize) -> Vec<usize> {
    let end = l.toks[from + 1..]
        .iter()
        .position(|t| t.line_start.is_some())
        .map_or(l.toks.len(), |p| from + 1 + p);
    let cols = token_cols(l);
    let mut depth = 0usize;
    let mut out = Vec::new();
    for (i, t) in l.toks.iter().enumerate().take(end) {
        match t.kind {
            Tk::Open(_) => depth += 1,
            Tk::Close(_) => depth = depth.saturating_sub(1),
            Tk::LArrow if depth == 0 && i >= from => out.push(cols[i]),
            _ => {}
        }
    }
    out
}

fn last_significant(l: &Line) -> Option<&Token> {
    l.toks.iter().rev().find(|t| !t.is_comment())
}

/// Bracket depth left open at the end of a logical line -- positive for a
/// block opened inside parens (`<- map (|x|:`).
fn open_depth(l: &Line) -> usize {
    let mut d = 0usize;
    for t in &l.toks {
        match t.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d = d.saturating_sub(1),
            _ => {}
        }
    }
    d
}

/// One open block, as the walk below sees it.
struct Frame {
    /// Indent of the header's logical line.
    header: usize,
    /// Indent of the first body line, once there is one.
    body: Option<usize>,
    /// Where a line continuing the header's statement goes once the block
    /// closes: the header's chain arrow, or one level in from its last
    /// physical line. Only set when the block was opened inside parens, since
    /// only then does the statement continue after the body.
    resume: Option<usize>,
    /// Anchor column of the header's innermost open bracket, for the closer.
    anchor: Option<usize>,
}

/// Index of the innermost `(` or `[` left unclosed at the end of a logical
/// line. `None` if nothing is open or the innermost open bracket is `{`:
/// braces are Rust's and the layout does not reach inside them.
fn innermost_open(l: &Line) -> Option<usize> {
    let mut open: Vec<usize> = Vec::new();
    for (i, t) in l.toks.iter().enumerate() {
        match t.kind {
            Tk::Open(_) => open.push(i),
            Tk::Close(_) => {
                open.pop();
            }
            _ => {}
        }
    }
    let i = *open.last()?;
    match l.toks[i].kind {
        Tk::Open('(') | Tk::Open('[') => Some(i),
        _ => None,
    }
}

/// Column of a bracket group's anchor: the start of the callee path before
/// the bracket (`compute (`, `foo.bar (`, `vec![`), or the bracket itself
/// when no name precedes it (`let t = (`). Contents indent one unit past the
/// anchor and the closer lands under it, so a group keeps one shape wherever
/// it sits.
fn anchor_col(l: &Line, open: usize) -> usize {
    let cols = token_cols(l);
    let mut i = open;
    while i > 0 {
        let t = &l.toks[i - 1];
        let path_part = matches!(t.kind, Tk::Ident | Tk::Dot | Tk::PathSep)
            || (t.kind == Tk::Punct && t.text == "!");
        if !path_part {
            break;
        }
        i -= 1;
    }
    cols[i]
}

fn from_lines_with(lines: &[Line], unit: usize, arities: &std::collections::HashMap<String, usize>) -> Columns {
    // The block stack: the indent of each open block's header. A line opens
    // a block by rule one; a later line at or below the header's indent
    // closes it. The body column of a block is the indent of its first body
    // line, or the header-relative default while it is still empty.
    let mut stack: Vec<Frame> = Vec::new();
    // The frame closed by the last line, when that line is the `)` tail of a
    // paren-block: the statement it belongs to continues from there.
    let mut closed: Option<Frame> = None;
    for l in lines {
        closed = None;
        while let Some(f) = stack.last() {
            if l.indent <= f.header {
                closed = stack.pop();
            } else {
                break;
            }
        }
        if let Some(Frame { body: body @ None, .. }) = stack.last_mut() {
            *body = Some(l.indent);
        }
        if layout::opens(&l.toks).is_some() {
            let resume = if open_depth(l) > 0 { arrow_cols(l, 0).last().copied() } else { None };
            let anchor = innermost_open(l).map(|i| anchor_col(l, i));
            stack.push(Frame { header: l.indent, body: None, resume, anchor });
        }
    }

    let mut legal: Vec<usize> = Vec::new();
    let default;

    match lines.last() {
        None => {
            default = 0;
        }
        Some(prev) => {
            let ends_eq = matches!(last_significant(prev).map(|t| &t.kind), Some(Tk::Eq));
            let cols = token_cols(prev);
            let last = last_physical_start(prev).unwrap_or(0);
            let multi = last > 0;
            if prev.tail_of_block {
                // The `)` that closed an isolated closure: the chain it sits
                // in continues under the header's arrow.
                default = closed
                    .as_ref()
                    .and_then(|f| f.resume)
                    .unwrap_or(prev.indent);
                legal.push(prev.indent + unit);
            } else if layout::opens(&prev.toks).is_some() {
                // Indent past the construct that opened the block: the first
                // token of the header's last physical line, or, when the
                // block opens inside a bracket group, the construct right
                // after the open bracket (`<- map (|x|:` indents from `|x|`).
                // A block literal: `let u = User\` -- the construct is the
                // literal's name, so the fields go one unit past `User`, not
                // past `let` (the user's rule).
                let literal_head = literal_head(prev);
                let opener = match innermost_open(prev) {
                    Some(i) if i + 1 < prev.toks.len() => cols[i + 1],
                    _ if literal_head.is_some() => cols[literal_head.unwrap()],
                    _ => cols[last],
                };
                default = opener + unit;
            } else if matches!(
                last_significant(prev).map(|t| &t.kind),
                Some(Tk::Open('(')) | Some(Tk::Open('['))
            ) {
                // A line-final `(` or `[`: the group's contents start one
                // unit past its anchor.
                let i = prev.toks.iter().rposition(|t| !t.is_comment()).unwrap_or(0);
                default = anchor_col(prev, i) + unit;
            } else if ends_eq {
                default = prev.indent + unit;
            } else if innermost_open(prev).is_some() && multi {
                // Inside an open bracket group: the next element is a peer
                // of the last one.
                default = cols[last];
                legal.push(prev.indent + unit);
            } else if multi && matches!(prev.toks[last].kind, Tk::Close(_)) {
                // The last physical line is a closer: the group is shut and
                // the statement complete. A chain continues under its arrow;
                // anything else starts a new statement.
                let arrows = arrow_cols(prev, 0);
                default = arrows.last().copied().unwrap_or(prev.indent);
                legal.extend(arrows);
                legal.push(prev.indent + unit);
            } else if multi {
                // A continuation in progress continues: under the last
                // arrow of the last physical line if it has one, else one
                // level in from that line (the long-receiver form). Every
                // arrow of that line is legal, so Shift-Tab walks back
                // through them: a chain may be horizontal, vertical, or a
                // line of several links followed by one link per line.
                // Without an arrow the last line is an argument, and the
                // next argument is its sibling: the same column (one more
                // unit in is a Tab away, never the default -- a staircase).
                // The one exception: a line that follows a line-final `=` is
                // the value's receiver, whose first link comes next, one
                // unit in (the long-receiver form).
                let arrows = line_arrow_cols(prev, last);
                let after_eq = prev.toks[..last].iter().rev().find(|t| !t.is_comment()).map_or(false, |t| t.kind == Tk::Eq);
                default = arrows.last().copied().unwrap_or(if after_eq { cols[last] + unit } else { cols[last] });
                legal.extend(arrows);
                legal.push(cols[last]);
                legal.push(cols[last] + unit);
                legal.push(prev.indent + unit);
            } else if let Some(callee_col) = awaiting_arguments(prev, arities) {
                // `my_func` (declared with three parameters) applied to fewer:
                // Enter lands where the next argument goes, one unit past
                // the callee.
                default = callee_col + unit;
                legal.push(callee_col + unit);
                legal.push(prev.indent + unit);
            } else {
                // A one-line statement is complete; the chain and
                // continuation columns are a Tab away.
                default = prev.indent;
                legal.push(prev.indent + unit);
                legal.extend(arrow_cols(prev, 0));
            }
        }
    }

    // Statement columns: the innermost open block's body, then each
    // enclosing block's, out to the margin. The innermost block is the one
    // the previous line is in or has just opened.
    for f in stack.iter().rev() {
        legal.push(f.body.unwrap_or(f.header + unit));
        if let Some(r) = f.resume {
            legal.push(r);
        }
    }
    legal.push(0);
    legal.push(default);
    legal.sort_unstable();
    legal.dedup();

    // A `)` or `]` typed first on the new line: under the anchor of the
    // innermost open bracket, whether it is open on the previous logical
    // line or on the header of an enclosing paren-block (closing every block
    // nested inside it on the way).
    let closer = lines
        .last()
        .and_then(|prev| innermost_open(prev).map(|i| anchor_col(prev, i)))
        .or_else(|| stack.iter().rev().find_map(|f| f.anchor));

    Columns { legal, default, unit, closer }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(src: &str, line: usize) -> Columns {
        columns(src, line)
    }

    #[test]
    fn empty_buffer() {
        assert_eq!(at("", 0), Columns { legal: vec![0], default: 0, unit: 4, closer: None });
    }

    #[test]
    fn enter_after_block_opener() {
        let s = "fn main$:\n";
        let c = at(s, 1);
        assert_eq!(c.default, 4);
        assert_eq!(c.legal, vec![0, 4]);
    }

    #[test]
    fn enter_after_arm_opener() {
        let s = "fn f n: i32 -> i32:\n    match n:\n        0 =>\n";
        let c = at(s, 3);
        assert_eq!(c.default, 12);
        assert_eq!(c.legal, vec![0, 4, 8, 12]);
    }

    #[test]
    fn enter_after_eq_lands_one_level_in() {
        let s = "fn main$:\n    let d =\n";
        assert_eq!(at(s, 2).default, 8);
    }

    /// A block literal opened mid-line indents its fields from the
    /// literal's name; on its own line, from that line.
    #[test]
    fn enter_after_a_block_literal_lands_past_its_name() {
        let s = "fn main$:\n    let u = User\\\n";
        assert_eq!(at(s, 2).default, 16, "one unit past `User`");
        let s = "fn main$:\n    let u = geometry.Vec2\\\n";
        assert_eq!(at(s, 2).default, 16, "one unit past the path's start");
        let s = "fn main$:\n    let u =\n        User\\\n";
        assert_eq!(at(s, 3).default, 12, "on its own line: one unit past it");
        let s = "fn main$:\n    let u = User\\\n                active = true\n";
        assert_eq!(at(s, 3).default, 16, "the next field is a sibling");
    }

    #[test]
    fn opener_on_continuation_indents_from_it() {
        // The guide's `let a =` / `|x: i32| -> i32:` shape: the body indents
        // from the closure, not from `let`.
        let s = "fn main$:\n    let a =\n        |x: i32| -> i32:\n";
        assert_eq!(at(s, 3).default, 12);
    }

    #[test]
    fn enter_in_vertical_chain_lands_under_arrow() {
        let s = "fn main$:\n    let d: Vec<i32> =\n        v <- iter$\n          <- map (|x| x * 10)\n";
        let c = at(s, 4);
        assert_eq!(c.default, 10);
        assert!(c.legal.contains(&4), "new statement in fn body: {:?}", c.legal);
        assert!(c.legal.contains(&0));
    }

    #[test]
    fn first_chain_line_offers_arrow_column() {
        // `let d =` / `v <- iter ()` is a continuation in progress: Enter
        // lands under the arrow; Tab reaches the statement column and the
        // continuation columns.
        let s = "fn main$:\n    let d: Vec<i32> =\n        v <- iter$\n";
        let c = at(s, 3);
        assert_eq!(c.default, 10);
        assert_eq!(c.legal, vec![0, 4, 8, 10, 12]);
    }

    #[test]
    fn long_receiver_hangs_one_level_in() {
        let s = "fn main$:\n    let listener =\n        TcpListener.bind \"127.0.0.1:8089\"\n";
        let c = at(s, 3);
        assert_eq!(c.default, 12);
        // No arrow yet, so no arrow column; the receiver's line plus a unit.
        assert_eq!(c.legal, vec![0, 4, 8, 12]);
    }

    #[test]
    fn one_line_statement_offers_chain_column_by_tab() {
        let s = "fn main$:\n    let n = v <- len$\n";
        let c = at(s, 2);
        assert_eq!(c.default, 4);
        assert_eq!(c.legal, vec![0, 4, 8, 14]);
    }

    #[test]
    fn arrow_inside_parens_is_not_a_chain_column() {
        let s = "fn main$:\n    show (v <- len$)\n";
        let c = at(s, 2);
        assert_eq!(c.legal, vec![0, 4, 8]);
    }

    #[test]
    fn statement_columns_walk_outward() {
        let s = "impl Foo:\n    fn a$:\n        if x:\n            y\n";
        let c = at(s, 4);
        assert_eq!(c.default, 12);
        assert_eq!(c.legal, vec![0, 4, 8, 12, 16]);
    }

    #[test]
    fn closed_blocks_leave_the_stack() {
        let s = "fn a$:\n    x\nfn b$:\n";
        let c = at(s, 3);
        assert_eq!(c.default, 4);
        assert_eq!(c.legal, vec![0, 4]);
    }

    #[test]
    fn only_text_above_the_cursor_is_read() {
        let s = "fn a$:\n\n    x\n";
        assert_eq!(at(s, 1).default, 4);
    }

    #[test]
    fn a_blank_line_above_keeps_its_column() {
        // Enter on the line the server placed: repeats the column.
        assert_eq!(at("fn a$:\n    x\n    \n", 3).default, 4);
        // Shift-Tab to column 0 at the end of a fn, then Enter: stays at 0.
        let c = at("fn a$:\n    x\n\n", 3);
        assert_eq!(c.default, 0);
        // An override off the grid is kept; the grid stays a Tab away.
        let c = at("fn a$:\n    x\n      \n", 3);
        assert_eq!(c.default, 6);
        assert!(c.legal.contains(&4) && c.legal.contains(&8));
        // The line with the cursor on it is not "above".
        assert_eq!(at("fn a$:\n    x\n      \n", 2).default, 4);
    }

    #[test]
    fn unit_is_detected() {
        let s = "fn a$:\n  x\n  if y:\n";
        let c = at(s, 3);
        assert_eq!(c.unit, 2);
        assert_eq!(c.default, 4);
    }

    #[test]
    fn unterminated_string_above_is_tolerated() {
        let s = "fn a$:\n    let s = \"oops\n    if y:\n";
        // The string swallows the rest of the prefix; what survives still
        // gives a sensible answer rather than a panic.
        let c = at(s, 3);
        assert!(c.legal.contains(&4));
    }

    #[test]
    fn comment_after_opener_still_opens() {
        let s = "fn a$: // body follows\n";
        assert_eq!(at(s, 1).default, 4);
    }
}

#[cfg(test)]
mod chain_tests {
    use super::columns;

    // The prescribed layout: a line-final `(`, the closure one unit past
    // the callee, its body one unit past the closure, `)` under the callee.
    const GUIDE: &str = "fn main$:\n    let d: Vec<i32> =\n        v <- iter$\n          <- map (\n                 |x|:\n                     x * 10\n             )\n          <- collect$\n";

    #[test]
    fn enter_after_line_final_paren_lands_past_callee() {
        assert_eq!(columns(GUIDE, 4).default, 17);
    }

    #[test]
    fn enter_after_closure_header_lands_past_closure() {
        let c = columns(GUIDE, 5);
        assert_eq!(c.default, 21);
        assert_eq!(c.closer, Some(13), "`)` under `map`");
    }

    #[test]
    fn closer_after_body_goes_under_callee() {
        assert_eq!(columns(GUIDE, 6).closer, Some(13));
        assert_eq!(columns(GUIDE, 2).closer, None, "nothing open");
    }

    #[test]
    fn enter_after_closing_paren_continues_chain() {
        assert_eq!(columns(GUIDE, 7).default, 10);
    }

    #[test]
    fn compact_form_closes_on_the_body_line() {
        // The accepted alternative: construct on the `(` line, one-line
        // body, closer ends it. Enter afterwards continues the chain.
        let s = "fn main$:\n    let d: Vec<i32> =\n        v <- iter$\n          <- map (|x|:\n                      x * 10)\n";
        assert_eq!(columns(s, 4).default, 22, "body one unit past `|x|`");
        let c = columns(s, 5);
        assert_eq!(c.default, 10, "{:?}", c);
    }

    #[test]
    fn call_with_arguments_on_their_own_lines() {
        let s = "fn main$:\n    let t = compute (\n                a\n";
        assert_eq!(columns(s, 2).default, 16, "first argument past `compute`");
        let c = columns(s, 3);
        assert_eq!(c.default, 16, "peer of `a`");
        assert_eq!(c.closer, Some(12), "`)` under `compute`");
    }

    #[test]
    fn bare_paren_and_bracket_anchor_on_themselves() {
        let s = "fn main$:\n    let t =\n        (\n            1\n";
        assert_eq!(columns(s, 3).default, 12);
        assert_eq!(columns(s, 4).closer, Some(8));
        let s = "fn main$:\n    let xs = [\n";
        let c = columns(s, 2);
        assert_eq!(c.default, 17);
        assert_eq!(c.closer, Some(13));
    }

    #[test]
    fn path_callee_and_macro_anchor_at_path_start() {
        let s = "fn main$:\n    let l = TcpListener.bind (\n";
        let c = columns(s, 2);
        assert_eq!(c.default, 16);
        assert_eq!(c.closer, Some(12));
        let s = "fn main$:\n    let v = vec![\n        1\n";
        assert_eq!(columns(s, 3).closer, Some(12));
    }

    #[test]
    fn nested_calls_close_under_their_own_callees() {
        let s = "fn main$:\n    foo (\n        bar (\n            x\n        )\n";
        assert_eq!(columns(s, 4).closer, Some(8), "inner under `bar`");
        assert_eq!(columns(s, 5).closer, Some(4), "outer under `foo`");
    }

    #[test]
    fn closer_closes_nested_blocks_too() {
        let s = "fn main$:\n    show (\n        |x|:\n            if x:\n                1\n";
        assert_eq!(columns(s, 5).closer, Some(4));
    }

    #[test]
    fn enter_after_a_closer_starts_a_new_statement() {
        let s = "fn main$:\n    let a = compute (\n                w\n            )\n    \n    \n";
        assert_eq!(columns(s, 4).default, 4);
        assert_eq!(columns(s, 6).default, 4, "blank lines at 4 keep 4");
        let s = "fn main$:\n    let a = compute (\n                w\n            )\n\n";
        assert_eq!(columns(s, 5).default, 0, "an empty line is column 0, and is kept");
        // A paren-block with no chain to resume: likewise.
        let s = "fn main$:\n    show (\n        |x|:\n            x\n    )\n";
        assert_eq!(columns(s, 5).default, 4);
    }

    #[test]
    fn enter_after_a_closer_in_a_chain_continues_it() {
        let s = "fn main$:\n    let d =\n        v <- iter$\n          <- map (\n                 |x| x * 10\n             )\n";
        assert_eq!(columns(s, 6).default, 10);
    }

    #[test]
    fn mixed_chain_lands_under_the_last_arrow() {
        // elem <- elem <- elem <- elem   arrows at 17, 23, 29
        let s = "fn main$:\n    let n = elem <- a$ <- b$ <- c$\n";
        let c = columns(s, 2);
        assert_eq!(c.default, 4, "a one-line statement is complete");
        assert!(c.legal.contains(&17) && c.legal.contains(&23) && c.legal.contains(&29), "{:?}", c.legal);
        // With a second line already under the last arrow, Enter repeats it
        // and Shift-Tab has the earlier arrows to walk back through.
        let s = "fn main$:\n    let n = elem <- a$ <- b$ <- c$\n                             <- d$ <- e$\n";
        let c = columns(s, 3);
        assert_eq!(c.default, 35, "last arrow of the line above");
        assert!(c.legal.contains(&29), "{:?}", c.legal);
        assert!(!c.legal.contains(&17), "arrows of earlier lines are not offered: {:?}", c.legal);
    }

    #[test]
    fn brace_gets_no_closer() {
        let s = "fn main$:\n    let p = Point {\n        x: 1,\n";
        assert_eq!(columns(s, 3).closer, None);
    }
}

#[cfg(test)]
mod argument_tests {
    use super::*;

    /// A declared function applied to fewer arguments than it takes: Enter
    /// lands one unit past the callee, where the next argument goes; met,
    /// the statement is complete.
    #[test]
    fn enter_after_a_declared_callee_lands_on_its_first_argument() {
        let decl = "fn my_func (a: i32) (b: i32) (c: i32) -> i32:\n    a + b + c\n\nfn f$:\n";
        let c = columns(&format!("{decl}    my_func"), 5);
        assert_eq!(c.default, 8);
        let c = columns(&format!("{decl}    my_func 1 2 3"), 5);
        assert_eq!(c.default, 4, "arity met: the statement is complete");
        let c = columns(&format!("{decl}    let x = my_func 1"), 5);
        assert_eq!(c.default, 16, "one unit past the callee, not the statement");
        let c = columns(&format!("{decl}    other_func"), 5);
        assert_eq!(c.default, 4, "an undeclared name is a complete statement");
    }

    /// A macro's `!` at the end of a line: `println!` alone is not an
    /// expression, so its arguments follow -- one unit past the callee, then
    /// each on its sibling's column. `m!$` is applied to nothing and is
    /// complete; a `!` inside brackets is already applied.
    #[test]
    fn enter_after_a_macro_bang_lands_on_its_first_argument() {
        let c = columns("fn f$:\n    println!", 2);
        assert_eq!(c.default, 8);
        let c = columns("fn f$:\n    println!\n        \"{} {}\"", 3);
        assert_eq!(c.default, 8, "the second argument is the first one's sibling");
        let c = columns("fn f$:\n    println!\n        \"{} {}\"\n        a", 4);
        assert_eq!(c.default, 8, "and the third");
        let c = columns("fn f$:\n    let v = vec!", 2);
        assert_eq!(c.default, 16, "one unit past the callee, not the statement");
        let c = columns("fn f$:\n    m!$", 2);
        assert_eq!(c.default, 4, "applied to nothing: complete");
        let c = columns("fn f$:\n    println! \"done\"", 2);
        assert_eq!(c.default, 4, "an applied macro is a complete statement");
        let c = columns("fn f$:\n    let x = !", 2);
        assert_eq!(c.default, 4, "a bare `!` is negation, not a macro");
    }

    /// Arguments listed beneath the callee are siblings: Enter after one
    /// lands at its column, never one unit further (the staircase).
    #[test]
    fn enter_after_an_argument_lands_on_its_sibling_column() {
        let src = "fn f$:\n    my_func\n        very_long_argument_1";
        let c = columns(src, 3);
        assert_eq!(c.default, 8);
        let src = "fn f$:\n    my_func\n        very_long_argument_1\n        very_long_argument_2";
        let c = columns(src, 4);
        assert_eq!(c.default, 8);
        assert!(c.legal.contains(&12), "one more unit in stays a Tab away: {:?}", c.legal);
    }
}

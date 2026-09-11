// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Layout pass.
//!
//! Turns the flat token stream into a tree of blocks. This is the only part of
//! hrs that understands anything structural, and it understands exactly one
//! thing: which lines are nested inside which.
//!
//! Two rules do all the work:
//!
//!   * A line whose last significant token is `:` or `=>` opens a block. The
//!     following, more-indented lines are its body.
//!   * A more-indented line in any *other* position is a continuation of the
//!     current logical line, not a new block. This is what makes multi-line
//!     method chains work without a leading-dot rule.

use crate::lex::{Span, Tk, Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// fn bodies, `do:`, if/else/while/for/loop/unsafe — separated by `;`
    Stmts,
    /// struct/enum/union bodies — separated by `,`
    Fields,
    /// match arms — separated by `,`
    Arms,
    /// impl/trait/mod bodies — no separator
    Items,
    /// A macro invocation's `do:` body (rule 2): statements take `;`, lines
    /// with a top-level `=>` are arms and take `,`
    Macro,
    /// A macro invocation's `do:` body whose first line begins with `<`
    /// (rule 1): markup copied through, `{ .. }` holes transpiled as Harsh
    Hsx,
    /// A `macro_rules!` body: one arm per line, each ending in `;`
    MacroRules,
    /// The statements of a repetition `$( .. )*` in a transcriber: every one
    /// takes `;`, the last included
    Rep,
    /// A struct literal, `Point:` in expression position: `field = expr`
    /// lines (emitted `field: expr,`), a bare `field` (shorthand), `..base`
    Lit,
    /// A macro invocation's `do:` body whose first line is `name:` (rule 6):
    /// a brace tree, each `:` block a pair of braces, attributes with `,`,
    /// holes Harsh
    Tree,
}

#[derive(Debug)]
pub enum Node {
    Line(Line),
    Block(Block),
    /// A run of markup inside an HSX block, emitted token by token with its
    /// source whitespace and nothing else changed.
    Markup(Vec<Token>),
    /// A delimited Harsh fragment inside a macro body: a `{ .. }` hole in
    /// markup, or a repetition `$( .. )*` that is the whole of its line. The
    /// body is laid out on its own, with the line holding the opener as its
    /// baseline.
    Group(Group),
    /// One line of a brace tree (rule 6): markup runs and holes, as an HSX
    /// body, but a line -- an attribute, a child, or an inline element.
    Tree(Vec<Node>),
}

#[derive(Debug)]
pub struct Group {
    /// `{`, or `$` `(`.
    pub open: Vec<Token>,
    pub body: Vec<Node>,
    /// `}`, or `)` and the repetition operator.
    pub close: Vec<Token>,
    /// `Stmts` for a hole, `Rep` for a repetition.
    pub kind: BlockKind,
    /// The body starts on the line after the opener (and so is written as
    /// indented lines) rather than on the opener's line.
    pub body_own_line: bool,
    /// An attribute value isolated in parens, `on:click=(|| body)`: the
    /// parens are optional grouping and are not emitted.
    pub bare: bool,
}

#[derive(Debug)]
pub struct Line {
    pub toks: Vec<Token>,
    pub comments: Vec<Token>,
    pub blank_before: usize,
    pub indent: usize,
    /// This line is the rest of the statement whose block just closed: it
    /// begins with the `)` that closed a paren opened before the block's `:`.
    pub tail_of_block: bool,
    /// This line is inside a DSL body -- markup (rule 1) or a brace tree
    /// (rule 6): the layout's continuation checks do not apply to it.
    pub dsl: bool,
}

#[derive(Debug)]
pub struct Block {
    /// Header line with the trailing `:` (or including `=>`) still attached.
    pub header: Line,
    /// True when the opener was `=>` rather than `:`.
    pub arrow_opener: bool,
    pub kind: BlockKind,
    pub body: Vec<Node>,
    /// Tokens after the body that belong to the same statement, when the
    /// block was opened inside a paren: `) <- filter (..) <- count ()`.
    /// The rest of the statement after a block opened inside a paren: begins
    /// with the `)` that closed the group. Nodes, because the tail may open
    /// another such block -- `(|p|: a) <- filter (|q|: b)`.
    pub tail: Vec<Node>,
}

#[derive(Debug)]
pub struct LayoutError {
    pub msg: String,
    pub span: Span,
}

struct PLine {
    indent: usize,
    toks: Vec<Token>,
    blank_before: usize,
}

/// Split the token stream into physical lines, tracking bracket depth so that
/// bracketed regions can suppress layout entirely.
fn physical_lines(toks: Vec<Token>) -> Vec<(PLine, usize)> {
    physical_lines_from(toks, 0)
}

/// As `physical_lines`, counting blank lines from `base_line`: a fragment
/// taken from the middle of a file starts with no blank lines before it.
fn physical_lines_from(toks: Vec<Token>, base_line: usize) -> Vec<(PLine, usize)> {
    let mut out: Vec<(PLine, usize)> = Vec::new();
    let mut cur: Option<PLine> = None;
    let mut depth_at_line_start = 0usize;
    let mut depth = 0usize;
    let mut last_line_no = base_line;

    for t in toks {
        if let Some(indent) = t.line_start {
            if let Some(p) = cur.take() {
                out.push((p, depth_at_line_start));
            }
            let blank = t.line.saturating_sub(last_line_no).saturating_sub(1);
            depth_at_line_start = depth;
            cur = Some(PLine { indent, toks: Vec::new(), blank_before: blank });
        }
        last_line_no = t.line;
        match t.kind {
            Tk::Open(_) => depth += 1,
            Tk::Close(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
        if let Some(p) = cur.as_mut() {
            p.toks.push(t);
        }
    }
    if let Some(p) = cur.take() {
        out.push((p, depth_at_line_start));
    }
    out
}

fn last_significant(toks: &[Token]) -> Option<&Token> {
    toks.iter().rev().find(|t| !t.is_comment())
}

fn opens_block(toks: &[Token]) -> Option<bool> {
    match last_significant(toks)?.kind {
        Tk::Colon => Some(false),
        Tk::FatArrow => Some(true),
        // `Point\` line-final: a literal's field list beneath.
        Tk::Backslash => Some(false),
        _ => {
            // `struct Point`, `enum Truth`, `union U`: the header takes one
            // name (and its generics, and a `[where ..]`), so what follows
            // deeper can only be the body. No mark needed.
            if decl_header(toks) {
                Some(false)
            } else {
                None
            }
        }
    }
}

/// A `struct` / `enum` / `union` header line -- after `pub`, `pub(crate)`
/// and the other modifiers -- that does not end in `;` or a mark of its own.
fn decl_header(toks: &[Token]) -> bool {
    let sig: Vec<&Token> = toks.iter().filter(|t| !t.is_comment()).collect();
    let mut i = 0;
    while i < sig.len() && sig[i].kind == Tk::Ident && crate::rules::MODIFIERS.contains(&sig[i].text.as_str()) {
        i += 1;
        if i < sig.len() && sig[i].kind == Tk::Open('(') {
            while i < sig.len() && sig[i].kind != Tk::Close(')') {
                i += 1;
            }
            i += 1;
        }
    }
    i < sig.len()
        && sig[i].kind == Tk::Ident
        && matches!(sig[i].text.as_str(), "struct" | "enum" | "union")
        && !matches!(sig.last().map(|t| &t.kind), Some(Tk::Semi) | Some(Tk::Colon) | Some(Tk::Backslash))
}

/// A record variant's header inside an enum body: a bare name on its line.
fn variant_header(toks: &[Token]) -> bool {
    let sig: Vec<&Token> = toks.iter().filter(|t| !t.is_comment()).collect();
    sig.len() == 1 && sig[0].kind == Tk::Ident && !crate::rules::is_keyword(&sig[0].text)
}

/// Merge continuation lines into logical lines.
///
/// Two things carry a statement across a physical line break: an unclosed
/// bracket, and a deeper-indented next line. Parens are transparent to layout,
/// so a line that ends in `:` while a paren is open still opens a block. The
/// body lines that follow are governed by the same rules with the paren's
/// depth as their baseline, and the line holding the matching `)` splits into
/// the body's last line and a *tail* that rejoins the header's statement.
fn logical_lines(pls: Vec<(PLine, usize)>) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    let mut cur: Option<Line> = None;
    let mut pending_comments: Vec<Token> = Vec::new();
    let mut pending_blank = 0usize;
    // Baseline bracket depth of each open paren-block, innermost last. A line
    // whose depth is above the baseline is inside the block's parens; the
    // token that takes it below closes the block.
    let mut baseline: Vec<usize> = Vec::new();
    let mut paren_headers: Vec<usize> = Vec::new();
    // Enum bodies whose variants may open a field list by a bare name:
    // (header indent, body indent once known).
    let mut enum_bodies: Vec<(usize, Option<usize>)> = Vec::new();
    // Absolute bracket depth at the start of the current logical line, so
    // a paren block opened inside another is measured from where it stands.
    let mut cur_depth_in = 0usize;

    let ends_open = |toks: &[Token]| -> bool {
        let sig: Vec<&Token> = toks.iter().filter(|t| !t.is_comment()).collect();
        matches!(sig.last().map(|t| &t.kind), Some(Tk::Colon) | Some(Tk::Backslash))
    };

    for (pl, depth_in) in pls {
        if !pl.toks.is_empty() && pl.toks.iter().all(|t| t.is_comment()) {
            if cur.is_none() {
                pending_blank += pl.blank_before;
                pending_comments.extend(pl.toks);
                continue;
            }
        }
        if pl.toks.is_empty() {
            continue;
        }
        let base = *baseline.last().unwrap_or(&0);

        // Does this line take the depth below the innermost baseline? If so
        // the token that does it is the `)` closing the paren-block.
        let mut split_at: Option<usize> = None;
        if base > 0 {
            let mut d = depth_in;
            for (i, t) in pl.toks.iter().enumerate() {
                match t.kind {
                    Tk::Open(_) => d += 1,
                    Tk::Close(_) => {
                        d = d.saturating_sub(1);
                        if d < base {
                            split_at = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut toks = pl.toks;
        let mut tail: Option<Vec<Token>> = None;
        let mut closed_header: Option<usize> = None;
        if let Some(k) = split_at {
            tail = Some(toks.split_off(k));
            baseline.pop();
            closed_header = paren_headers.pop();
        }

        // The body part, if any, follows the ordinary rules relative to the
        // baseline it sits above.
        if !toks.is_empty() {
            // Enum bodies: note them and where their variants stand.
            while let Some(&(h, _)) = enum_bodies.last() {
                if pl.indent <= h {
                    enum_bodies.pop();
                } else {
                    break;
                }
            }
            if let Some(last) = enum_bodies.last_mut() {
                if last.1.is_none() && pl.indent > last.0 {
                    last.1 = Some(pl.indent);
                }
            }
            let continues = if depth_in > base {
                true
            } else if let Some(c) = cur.as_ref() {
                // A tail takes deeper lines as its continuation (the chain
                // after a `)`) -- unless the tail itself opens a block, `):`,
                // whose body those lines are. A declaration's header, or a
                // record variant's bare name in an enum body, opens its
                // body by grammar: not a continuation either.
                let variant = variant_header(&c.toks) && enum_bodies.last().map_or(false, |&(_, b)| b == Some(c.indent));
                pl.indent > c.indent && opens_block(&c.toks).is_none() && !variant
            } else {
                false
            };
            if !continues && toks.iter().any(|t| !t.is_comment()) && decl_header(&toks) && toks.iter().any(|t| t.is_kw("enum")) {
                enum_bodies.push((pl.indent, None));
            }
            // (A line deeper than the baseline right after a paren-block
            // header was pushed has nothing to continue: it starts a line.)
            if continues && cur.is_some() {
                cur.as_mut().unwrap().toks.extend(toks);
            } else {
                if let Some(c) = cur.take() {
                    out.push(c);
                }
                cur_depth_in = depth_in;
                cur = Some(Line {
                    indent: pl.indent,
                    toks,
                    comments: std::mem::take(&mut pending_comments),
                    blank_before: pl.blank_before + std::mem::take(&mut pending_blank),
                    tail_of_block: false,
                    dsl: false,
                });
            }
            // Did this line open a block inside a paren? Then it is complete
            // as a header, and what follows is the block body.
            let c = cur.as_ref().unwrap();
            let depth_after = {
                let mut d = cur_depth_in;
                for t in &c.toks {
                    match t.kind {
                        Tk::Open(_) => d += 1,
                        Tk::Close(_) => d = d.saturating_sub(1),
                        _ => {}
                    }
                }
                d
            };
            let innermost_is_paren = {
                let mut stack: Vec<char> = Vec::new();
                for t in &c.toks {
                    match t.kind {
                        Tk::Open(ch) => stack.push(ch),
                        Tk::Close(_) => {
                            stack.pop();
                        }
                        _ => {}
                    }
                }
                stack.last() == Some(&'(')
            };
            if ends_open(&c.toks) && depth_after > base && innermost_is_paren {
                baseline.push(depth_after);
                paren_headers.push(out.len());
                out.push(cur.take().unwrap());
            }
        }

        // The tail rejoins the statement whose block just closed.
        if let Some(t) = tail {
            if let Some(c) = cur.take() {
                out.push(c);
            }
            // Its indent is the header's -- the line that opened the paren
            // block this `)` closed, not whatever nested block opened last
            // inside it -- so that deeper lines continue it.
            let header_indent = closed_header
                .and_then(|i| out.get(i))
                .map(|l| l.indent)
                .or_else(|| {
                    out.iter().rev().find(|l| opens_block(&l.toks).is_some()).map(|l| l.indent)
                })
                .unwrap_or(pl.indent);
            cur_depth_in = base;
            cur = Some(Line {
                indent: header_indent,
                toks: t,
                comments: Vec::new(),
                blank_before: 0,
                tail_of_block: true,
                dsl: false,
            });
            // The tail may itself open a paren block -- `) <- map (|line|
            // view! do:` -- and then it is a header too.
            let c = cur.as_ref().unwrap();
            let base = *baseline.last().unwrap_or(&0);
            let mut d = cur_depth_in;
            let mut stack: Vec<char> = Vec::new();
            for t in &c.toks {
                match t.kind {
                    Tk::Open(ch) => {
                        d += 1;
                        stack.push(ch);
                    }
                    Tk::Close(_) => {
                        d = d.saturating_sub(1);
                        stack.pop();
                    }
                    _ => {}
                }
            }
            if ends_open(&c.toks) && d > base && stack.last() == Some(&'(') {
                baseline.push(d);
                paren_headers.push(out.len());
                out.push(cur.take().unwrap());
            }
        }
    }
    if let Some(c) = cur.take() {
        out.push(c);
    }
    if !pending_comments.is_empty() {
        out.push(Line {
            indent: 0,
            toks: pending_comments,
            comments: Vec::new(),
            blank_before: pending_blank,
            tail_of_block: false,
            dsl: false,
        });
    }
    out
}

/// Classify a block from its header line.
///
/// Splitting at a top-level `=` first is what keeps `-> impl IntoResponse:`
/// from being misread as an impl block, while still letting
/// `let x = match y:` be recognised as a match.
fn classify(header: &[Token], outer: BlockKind) -> BlockKind {
    let sig: Vec<&Token> = header.iter().filter(|t| !t.is_comment()).collect();

    // `macro_rules! name:` holds arms; `name! do:` is a macro body written in
    // Harsh (HSX when its first line begins with `<`, decided by the caller).
    if sig.first().map_or(false, |t| t.is_kw("macro_rules")) {
        return BlockKind::MacroRules;
    }
    if macro_do_header(header) {
        return BlockKind::Macro;
    }
    // The transcriber of a `macro_rules!` arm is a macro body too.
    if outer == BlockKind::MacroRules {
        return BlockKind::Macro;
    }
    // Every block of a brace tree -- an element, a `for`, an `if` -- is one.
    if outer == BlockKind::Tree {
        return BlockKind::Tree;
    }

    // A header ending in `\`: a declaration's field list (`struct Point\`,
    // or a record variant inside an enum), else a literal's. A declaration
    // header without a mark is a field list too.
    if sig.last().map_or(false, |t| t.kind == Tk::Backslash) {
        let is_decl = sig.iter().any(|t| t.kind == Tk::Ident && matches!(t.text.as_str(), "struct" | "enum" | "union"));
        return if is_decl || outer == BlockKind::Fields { BlockKind::Fields } else { BlockKind::Lit };
    }
    if decl_header(header) {
        return BlockKind::Fields;
    }
    let mut depth = 0i32;
    let mut eq_at = None;
    for (i, t) in sig.iter().enumerate() {
        match t.kind {
            Tk::Open(_) => depth += 1,
            Tk::Close(_) => depth -= 1,
            Tk::Eq if depth == 0 => {
                eq_at = Some(i);
                break;
            }
            _ => {}
        }
    }
    let scan: &[&Token] = match eq_at {
        Some(i) => &sig[i + 1..],
        None => &sig[..],
    };

    // A `fn` anywhere makes a fn body (`-> impl IntoResponse:` is not an
    // impl). Otherwise the keyword nearest the colon, at the depth the
    // header ends at, governs: `x if guard && match p <- kind:` opens the
    // match's arms, not the guard's block.
    if scan.iter().any(|t| t.is_kw("fn")) {
        return BlockKind::Stmts;
    }
    let end_depth: i32 = {
        let mut d = 0i32;
        for t in scan {
            match t.kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                _ => {}
            }
        }
        d.max(0)
    };
    let mut d = 0i32;
    let mut found: Option<BlockKind> = None;
    for t in scan {
        match t.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Ident if d == end_depth => {
                let k = match t.text.as_str() {
                    "struct" | "enum" | "union" => Some(BlockKind::Fields),
                    "impl" | "trait" | "mod" | "extern" => Some(BlockKind::Items),
                    "match" => Some(BlockKind::Arms),
                    "if" | "else" | "while" | "for" | "loop" | "unsafe" | "do" => Some(BlockKind::Stmts),
                    _ => None,
                };
                if k.is_some() {
                    found = k;
                }
            }
            _ => {}
        }
    }
    if let Some(k) = found {
        return k;
    }
    // `Gpoint:` inside an enum body is a record variant, not a statement block.
    if outer == BlockKind::Fields {
        return BlockKind::Fields;
    }
    BlockKind::Stmts
}

/// The spellings this language does not have, each named with its
/// replacement: a declaration header ending in `:` (its body follows the
/// name, no mark), a record variant `Name:` (bare `Name`), and a literal
/// opened with `:` (`Point\`).
fn check_old_marks(ln: &Line, outer: BlockKind) -> Result<(), LayoutError> {
    let sig: Vec<&Token> = ln.toks.iter().filter(|t| !t.is_comment()).collect();
    let Some(last) = sig.last() else { return Ok(()) };
    if last.kind != Tk::Colon {
        return Ok(());
    }
    let mut k = 0;
    while k < sig.len() && sig[k].kind == Tk::Ident && crate::rules::MODIFIERS.contains(&sig[k].text.as_str()) && !sig[k].is_kw("const") {
        k += 1;
        if k < sig.len() && sig[k].kind == Tk::Open('(') {
            while k < sig.len() && sig[k].kind != Tk::Close(')') {
                k += 1;
            }
            k += 1;
        }
    }
    if k < sig.len() && sig[k].kind == Tk::Ident && matches!(sig[k].text.as_str(), "struct" | "enum" | "union") {
        let name = sig.get(k + 1).map(|t| t.text.clone()).unwrap_or_default();
        return Err(LayoutError {
            msg: format!(
                "a declaration's body follows its name with no mark: `{kw} {name}` with the fields beneath, or `{kw} {name}\\ a: T, b: U` inline",
                kw = sig[k].text
            ),
            span: last.span,
        });
    }
    if outer == BlockKind::Fields && sig.len() == 2 && sig[0].kind == Tk::Ident {
        return Err(LayoutError {
            msg: format!("a record variant's fields follow its name with no mark: `{}` with the fields beneath, or `({}\\ a: T)` inline", sig[0].text, sig[0].text),
            span: last.span,
        });
    }
    if literal_header(&sig, matches!(outer, BlockKind::Stmts | BlockKind::Macro | BlockKind::Rep | BlockKind::Lit | BlockKind::Arms)) {
        let name = sig[sig.len() - 2].text.clone();
        return Err(LayoutError {
            msg: format!("a struct is built with `{name}\\`: `{name}\\ x = 1, y = 2` inline, or `{name}\\` with one field per line"),
            span: last.span,
        });
    }
    Ok(())
}

/// The same for an inline `Point: x = 1` in a line: every colon that reads
/// as a literal's opener names `\`.
fn check_old_inline_literal(ln: &Line, outer: BlockKind) -> Result<(), LayoutError> {
    let value_position = matches!(outer, BlockKind::Stmts | BlockKind::Macro | BlockKind::Rep | BlockKind::Lit | BlockKind::Arms);
    let sig: Vec<&Token> = ln.toks.iter().filter(|t| !t.is_comment()).collect();
    let mut stack: Vec<char> = Vec::new();
    for (i, t) in sig.iter().enumerate() {
        match t.kind {
            Tk::Open(c) => stack.push(c),
            Tk::Close(_) => {
                stack.pop();
            }
            Tk::Colon if stack.last().map_or(true, |c| *c == '(') && i + 1 < sig.len() => {
                // Inside a `\` pattern's items, `x: a` renames a field.
                let after_backslash = sig[..i].iter().any(|t| t.kind == Tk::Backslash);
                if !after_backslash && literal_at(&sig, i, value_position) {
                    let name = sig[i - 1].text.clone();
                    return Err(LayoutError {
                        msg: format!("a struct is built with `{name}\\`: `{name}\\ x = 1, y = 2` inline, or `{name}\\` with one field per line"),
                        span: t.span,
                    });
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Does this header open a struct literal? Its last tokens before the `:`
/// are a path of non-keyword names, and what precedes the path is nothing,
/// `=`, `(`, `[`, `,`, `=>` or `return` -- expression position. `match x:`,
/// `if c:`, `impl Foo:` are governed by their keyword and never reach here.
fn literal_header(sig: &[&Token], line_start_allowed: bool) -> bool {
    literal_at(sig, sig.len() - 1, line_start_allowed)
}

/// `literal_header` for the colon at `colon` in `sig`, with the rest of the
/// line after it as evidence: after `(`, `[` or at a line's start the shape
/// `name:` is also an annotation's (`(a: i32`), so an inline literal there
/// must show a `=` or a `..` before the next `,` or closer. In the block
/// form (the colon ends the line) the body speaks for itself.
fn literal_at(sig: &[&Token], colon: usize, line_start_allowed: bool) -> bool {
    let n = colon + 1;
    if n < 2 || sig[colon].kind != Tk::Colon {
        return false;
    }
    // A block keyword anywhere before the colon governs it: `if let Some x
    // = y:` is an `if`, whatever stands last.
    if sig[..colon].iter().any(|t| t.kind == Tk::Ident && (crate::rules::BLOCK_KEYWORDS.contains(&t.text.as_str()) || t.text == "do")) {
        return false;
    }
    let mut i = n - 1;
    // The path, backwards: Ident (Dot Ident)*.
    if sig[i - 1].kind != Tk::Ident || crate::rules::is_keyword(&sig[i - 1].text) || crate::rules::BLOCK_KEYWORDS.contains(&sig[i - 1].text.as_str()) || sig[i - 1].text == "do" {
        return false;
    }
    i -= 1;
    while i >= 2 && sig[i - 1].kind == Tk::Dot && sig[i - 2].kind == Tk::Ident {
        i -= 2;
    }
    let evidence = || -> bool {
        // The colon ends the line: block form, fine.
        if colon + 1 >= sig.len() {
            return true;
        }
        // Any `=` or `..` among the fields, up to the group's closer:
        // `(PLine: indent, toks = Vec.new$)` shows its evidence second. A
        // shorthand-only body, `(Export: files, formatted)`, is a literal
        // when it holds commas and no second colon (`(a: i32, b: i32)` is a
        // parameter list).
        let mut d = 0i32;
        let mut commas = 0;
        let mut colons = 0;
        for t in &sig[colon + 1..] {
            match t.kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => {
                    if d == 0 {
                        break;
                    }
                    d -= 1;
                }
                Tk::Eq | Tk::DotDot if d == 0 => return true,
                Tk::Comma if d == 0 => commas += 1,
                Tk::Colon if d == 0 => colons += 1,
                _ => {}
            }
        }
        commas > 0 && colons == 0
    };
    // At a line's start where a value can stand, `Guess: value` can only be
    // a literal (a field declaration lives in a declaration block).
    if i == 0 {
        return line_start_allowed;
    }
    let p = sig[i - 1];
    match p.kind {
        Tk::Eq | Tk::FatArrow => true,
        Tk::Open('(') | Tk::Open('[') | Tk::Comma => evidence(),
        _ => p.is_kw("return"),
    }
}

/// A header that opens a macro body written in Harsh: `.. name! do:`.
fn macro_do_header(toks: &[Token]) -> bool {
    let sig: Vec<&Token> = toks.iter().filter(|t| !t.is_comment()).collect();
    let n = sig.len();
    n >= 4
        && sig[n - 1].kind == Tk::Colon
        && sig[n - 2].is_kw("do")
        && sig[n - 3].kind == Tk::Punct
        && sig[n - 3].text == "!"
        && sig[n - 4].kind == Tk::Ident
}

/// The separator an inline block of this kind writes between its items --
/// exactly the one the indented form would have inserted.
fn sep_of(kind: BlockKind) -> Tk {
    match kind {
        BlockKind::Stmts => Tk::Semi,
        _ => Tk::Comma,
    }
}

/// The block keyword governing a colon: the nearest one before it, scanning
/// backwards and stopping at an earlier colon. Scanning forwards would make
/// `if b: 1 else: 2` report `if` for the `else`'s colon.
fn opener_kw(toks: &[Token], from: usize, colon: usize, _outer: BlockKind) -> Option<&str> {
    // A closure's prototype, `|x|` / `move ||`, followed by `:`: the body
    // is a block, as after `fn f$`.
    if colon > from {
        let p = &toks[colon - 1];
        if p.kind == Tk::Punct && p.text.ends_with('|') {
            return Some("closure");
        }
        // With a return type: `|x| -> usize:`. Back over the type to a `->`
        // that follows a closing bar.
        let mut k = colon;
        while k > from {
            k -= 1;
            let t = &toks[k];
            if t.kind == Tk::Punct && t.text == "->" {
                if k > from && toks[k - 1].kind == Tk::Punct && toks[k - 1].text.ends_with('|') {
                    return Some("closure");
                }
                break;
            }
            if !(matches!(t.kind, Tk::Ident | Tk::PathSep | Tk::Dot | Tk::Lt | Tk::Gt | Tk::Open('[') | Tk::Close(']') | Tk::Open('(') | Tk::Close(')') | Tk::Comma | Tk::Lifetime) || (t.kind == Tk::Punct && matches!(t.text.as_str(), "&" | "'" | "*" | "+"))) {
                break;
            }
        }
    }
    // (A struct literal is opened with `\`, never with a colon: see
    // `check_old_marks` for the message a `Point:` gets.)
    let mut d = 0i32;
    let mut i = colon;
    while i > from {
        i -= 1;
        match toks[i].kind {
            Tk::Close(_) => d += 1,
            Tk::Open(_) => d -= 1,
            Tk::Colon if d == 0 => return None,
            Tk::Ident if d == 0 => {
                if crate::rules::BLOCK_KEYWORDS.contains(&toks[i].text.as_str())
                    || toks[i].text == "do"
                    || toks[i].text == "macro_rules"
                {
                    return Some(toks[i].text.as_str());
                }
            }
            _ => {}
        }
    }
    None
}

/// The first colon in `[from, to)` that opens an inline block: it is not the
/// item's last token, a block keyword or a closure prototype governs it, and
/// it sits at bracket depth zero or inside parens only. Parens isolate; they
/// neither open nor close a block, so a block may be written inside one and
/// ends where the group ends. `[ .. ]` and `{ .. }` are Rust's.
fn inline_colon(toks: &[Token], from: usize, to: usize, outer: BlockKind) -> Option<(usize, &str)> {
    let last = (from..to).rev().find(|&i| !toks[i].is_comment());
    let mut stack: Vec<char> = Vec::new();
    for i in from..to {
        match toks[i].kind {
            Tk::Open(ch) => stack.push(ch),
            Tk::Close(_) => {
                stack.pop();
            }
            // At depth zero, or inside a paren -- the innermost bracket must
            // be a paren, so `vec! [(Point: x = 1), (Point: x = 2)]` reads
            // each literal to its `)`.
            Tk::Colon if stack.last().map_or(true, |c| *c == '(') => {
                if Some(i) == last {
                    return None;
                }
                if let Some(kw) = opener_kw(toks, from, i, outer) {
                    return Some((i, kw));
                }
            }
            // `Point\ x = 1, y = 2` inline: a literal, or an inline
            // declaration `struct Point\ x: f64` -- unless it is a pattern
            // (`let Point\ x, y = p`, `Point\ x, .. =>`), which stays in
            // its line and is respelled by the emitter.
            Tk::Backslash if stack.last().map_or(true, |c| *c == '(') => {
                if Some(i) == last {
                    return None;
                }
                if !pattern_backslash(toks, from, to, i) {
                    return Some((i, "lit"));
                }
            }
            _ => {}
        }
    }
    None
}

/// Is the `\` at `i` a pattern's -- after `let` (or `if let`, `while
/// let`, `for`) before the `=` / `in`, or before a `=>` on its line?
pub fn pattern_backslash(toks: &[Token], from: usize, to: usize, i: usize) -> bool {
    let sig: Vec<usize> = (from..to).filter(|&k| !toks[k].is_comment()).collect();
    let pos = sig.iter().position(|&k| k == i).unwrap_or(0);
    // `let` / `for` before it, with no `=` / `in` at depth zero between.
    let mut d = 0i32;
    let mut binder = false;
    for &k in &sig[..pos] {
        match toks[k].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Eq if d == 0 => binder = false,
            Tk::Ident if toks[k].is_kw("let") || toks[k].is_kw("for") => binder = true,
            Tk::Ident if toks[k].is_kw("in") && d == 0 => binder = false,
            _ => {}
        }
    }
    if binder {
        return true;
    }
    // A `=>` after it at depth zero: a match arm's pattern.
    let mut d = 0i32;
    for &k in &sig[pos + 1..] {
        match toks[k].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => {
                if d == 0 {
                    // Out of the group the `\` sits in; keep scanning.
                    continue;
                }
                d -= 1;
            }
            Tk::FatArrow if d == 0 => return true,
            Tk::Eq if d == 0 => return false,
            _ => {}
        }
    }
    false
}

/// Expand a logical line's inline blocks into the tree an indented block would
/// have produced. Returns one node per top-level item.
fn expand_inline(toks: &[Token], from: usize, to: usize, kind: BlockKind, proto: &Line) -> Result<Vec<Node>, LayoutError> {
    let sep = sep_of(kind);
    let mut out = Vec::new();
    let mut start = from;
    let mut d = 0i32;
    let mut i = from;
    // Split into items at this block's own separator.
    let mut items: Vec<(usize, usize)> = Vec::new();
    while i < to {
        match toks[i].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            // A `;` that ends the whole inline block is the written one --
            // "discard this block's tail value" -- and stays with its
            // statement, exactly as it does on the last line of an indented
            // block: `do: f$;` is `{ f(); }`, not `{ f() }`.
            Tk::Semi if d == 0 && sep == Tk::Semi && toks[i + 1..to].iter().all(|t| t.is_comment()) => {
                items.push((start, i + 1));
                start = to;
            }
            ref k if d == 0 && *k == sep => {
                items.push((start, i));
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if start < to {
        items.push((start, to));
    }

    for (a, b) in items {
        if a >= b {
            continue;
        }
        out.extend(expand_item(toks, a, b, kind, proto)?);
    }
    Ok(out)
}

/// One item, which may itself open an inline block.
fn expand_item(toks: &[Token], from: usize, to: usize, outer: BlockKind, proto: &Line) -> Result<Vec<Node>, LayoutError> {
    let Some((colon, kw)) = inline_colon(toks, from, to, outer) else {
        return Ok(vec![Node::Line(sub_line(toks, from, to, proto))]);
    };
    let header = sub_line(toks, from, colon + 1, proto);
    let kind = classify(&header.toks, outer);

    // Where the body ends: at `else` for an `if`, at a foreign separator, or at
    // the end of the item.
    let own = sep_of(kind);
    let mut d = 0i32;
    let mut end = to;
    // Each nested inline `if` inside the body claims one `else`, so the first
    // unclaimed one is ours. This is what makes `if a: if b: 1 else: 2 else: 3`
    // bind each `else` to the nearest open `if`.
    let mut pending_if = 0usize;
    // A `)` taking the depth below zero closes the group the block was
    // opened in: the block owns no token past it.
    let mut group_close: Option<usize> = None;
    for i in (colon + 1)..to {
        match toks[i].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => {
                d -= 1;
                if d < 0 {
                    end = i;
                    group_close = Some(i);
                    break;
                }
            }
            // Any `if` in the body claims one `else`, whether it is written
            // inline or with braces.
            Tk::Ident if d == 0 && toks[i].text == "if" => pending_if += 1,
            Tk::Ident if d == 0 && toks[i].text == "else" && kw == "if" => {
                if pending_if > 0 {
                    pending_if -= 1;
                    continue;
                }
                end = i;
                break;
            }
            ref k if d == 0 && *k != own && matches!(k, Tk::Semi | Tk::Comma) => {
                end = i;
                break;
            }
            _ => {}
        }
    }

    // An inline block holds one expression -- one rule, no separator. A
    // `;` inside it (other than one ending the whole line, which discards
    // the tail value) means several statements were meant, and the layout
    // has no way to say where such a block ends: `A => do: f(); g(), B => 2`
    // puts `g()` outside the arm. Braces say it, or the indented form.
    // And a `{` right after the colon opens a second block inside the
    // first: `if c: { .. }` is `if c { { .. } }`, and with an `else` the
    // nesting goes wrong. Braces open the block on their own.
    if let Some(t) = toks[colon + 1..to].iter().find(|t| !t.is_comment()) {
        if t.kind == Tk::Open('{') {
            return Err(LayoutError {
                msg: "braces already open a block; write `if c { .. }`, not `if c: { .. }`".into(),
                span: t.span,
            });
        }
    }
    // The whole item is looked at, not just the body: a `;` after an arm's
    // inline block is the same mistake. Inside a group, the group's `)`
    // bounds the item.
    let stop = group_close.unwrap_or(to);
    let mut d = 0i32;
    for i in (colon + 1)..stop {
        match toks[i].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::Semi if d == 0 && i + 1 < stop && !toks[i + 1..stop].iter().all(|t| t.is_comment()) => {
                return Err(LayoutError {
                    msg: "an inline block holds one expression; for several statements, use braces on one line, `if c { f(); g() }`, or the indented form".into(),
                    span: toks[i].span,
                });
            }
            _ => {}
        }
    }

    let body = expand_inline(toks, colon + 1, end, kind, proto)?;
    // The rest of the statement after the group's `)` is the block's tail,
    // itself expanded: it may open another block, `(|p|: a) <- map (|q|: b)`.
    let tail = match group_close {
        Some(c) => expand_item(toks, c, to, outer, proto)?,
        None => Vec::new(),
    };
    let mut out = vec![Node::Block(Block { header, arrow_opener: false, kind, body, tail })];
    // Anything after the body -- typically `else: ..` -- is a sibling.
    if group_close.is_none() && end < to {
        out.extend(expand_item(toks, end, to, outer, proto)?);
    }
    Ok(out)
}

fn sub_line(toks: &[Token], from: usize, to: usize, proto: &Line) -> Line {
    Line {
        toks: toks[from..to].to_vec(),
        comments: Vec::new(),
        blank_before: 0,
        indent: proto.indent,
        tail_of_block: false,
        dsl: false,
    }
}

/// The logical lines of a token stream, before any validation: physical
/// lines merged by the two layout rules, nothing rejected. This is the entry
/// point for tools that read structure from a buffer that may be mid-edit --
/// the language server -- and must see the same lines the transpiler sees.
pub fn lines(toks: Vec<Token>) -> Vec<Line> {
    let mut lines = logical_lines(physical_lines(toks));
    // Markup and brace-tree bodies are flagged here too, so the formatter
    // and the editor leave them alone.
    mark_hsx(&mut lines);
    lines
}

/// Whether a logical line opens a block: `Some(false)` for a trailing `:`,
/// `Some(true)` for a trailing `=>`, `None` otherwise.
pub fn opens(toks: &[Token]) -> Option<bool> {
    opens_block(toks)
}

pub fn build(toks: Vec<Token>) -> Result<Vec<Node>, LayoutError> {
    let arities = crate::juxt::collect_arities(&toks);
    build_with(toks, &arities)
}

/// Build with arities known from beyond this file -- the driver collects them
/// across the project so `$` works on functions defined in other modules.
pub fn build_with(
    toks: Vec<Token>,
    arities: &std::collections::HashMap<String, usize>,
) -> Result<Vec<Node>, LayoutError> {
    let mut lines = logical_lines(physical_lines(toks));
    // The pipes become ordinary calls (or closures) before anything else
    // looks at the tokens, so the rest of the pipeline needs no knowledge of
    // them. Partials bound by `let` are remembered so pipes compose.
    let mut partials = std::collections::HashMap::new();
    for l in &mut lines {
        // `f$` first: it is a synthetic `()` by the time the pipes look.
        let toks = crate::juxt::rewrite_dollar(&l.toks)
            .map_err(|e| LayoutError { msg: e.msg, span: e.span })?;
        l.toks = crate::juxt::rewrite_pipes(&toks, arities, &mut partials)
            .map_err(|e| LayoutError { msg: e.msg, span: e.span })?;
    }
    let mut lines = lines;
    mark_hsx(&mut lines);
    let lines = lines;
    check_braces(&lines)?;
    check_group_lines(&lines)?;
    check_continuations(&lines)?;
    let mut idx = 0usize;
    let nodes = build_block(&lines, &mut idx, 0, BlockKind::Items, &mut Vec::new())?;
    check_semis(&nodes, BlockKind::Items)?;
    check_juxt(&nodes, BlockKind::Items)?;
    check_else(&nodes)?;
    check_bare_closure_chain(&nodes)?;
    Ok(nodes)
}

/// Words that only ever begin a statement or an item. A physical line that
/// starts with one of them but is indented past the statement above is a
/// continuation by rule two -- and `let x = 1 let y = 2` is nonsense -- so
/// it can only be a misaligned statement. `if`, `match`, `for` and the rest
/// are not here: `let d =` / `if c:` is a continuation on purpose.
const STATEMENT_KEYWORDS: [&str; 12] = [
    "let", "fn", "struct", "enum", "impl", "trait", "mod", "use", "pub", "const", "static", "type",
];

/// The body lines of a `name! do:` block whose first line begins with `<`
/// are markup, and those of one whose first line is `name:` are a brace
/// tree: their words mean nothing to the layout's statement
/// checks, so they are flagged before any check reads them.
fn mark_hsx(lines: &mut [Line]) {
    let mut i = 0;
    while i < lines.len() {
        // The body's first line, past any comment-only lines.
        let first_body = (i + 1..lines.len()).find(|&k| lines[k].toks.iter().any(|t| !t.is_comment()));
        let is_dsl_header = macro_do_header(&lines[i].toks)
            && first_body.map_or(false, |k| {
                let n = &lines[k];
                n.indent > lines[i].indent && (body_is_markup(lines, i, k) || tree_first_line(&n.toks))
            });
        if is_dsl_header {
            let col = lines[i].indent;
            let mut k = i + 1;
            while k < lines.len() && lines[k].indent > col {
                lines[k].dsl = true;
                k += 1;
            }
            i = k;
        } else {
            i += 1;
        }
    }
}

/// Is the body of the macro `do:` block at `header` markup? Judged over the
/// whole body, since it may open with a hole or a string before its first
/// tag: `{panel}` / `<button ..>`.
fn body_is_markup(lines: &[Line], header: usize, first_body: usize) -> bool {
    let col = lines[header].indent;
    let mut toks: Vec<Token> = Vec::new();
    let mut k = first_body;
    while k < lines.len() && lines[k].indent > col {
        toks.extend(lines[k].toks.iter().cloned());
        k += 1;
    }
    crate::juxt::is_markup(&toks, 0, toks.len())
}

/// `name:` -- a block opened by a non-keyword name, or a path of them -- is
/// the first line of a brace tree: a tree begins with an element. (`name =`
/// is not the test, since `select!`'s arms have that shape.)
fn tree_first_line(toks: &[Token]) -> bool {
    let sig: Vec<&Token> = toks.iter().filter(|t| !t.is_comment()).collect();
    let mut i = 0;
    if sig.get(i).map_or(true, |t| t.kind != Tk::Ident || crate::rules::is_keyword(&t.text) || crate::rules::BLOCK_KEYWORDS.contains(&t.text.as_str())) {
        return false;
    }
    i += 1;
    while i + 1 < sig.len() && sig[i].kind == Tk::Dot && sig[i + 1].kind == Tk::Ident {
        i += 2;
    }
    matches!(sig.get(i).map(|t| &t.kind), Some(Tk::Colon))
}

/// Braces hold a block on one line. A `{ .. }` that opens and closes on
/// different lines is an error naming `:` / `do:` -- two syntaxes for one
/// block across files is the drift the language rejects. Two exemptions,
/// where the braces are not a block: a hole in markup or a tree (those lines
/// are `dsl`-flagged and not read here; a hole's own fragment is), and a
/// brace group handed as an argument, `json! ({ .. })`. And a struct is
/// built with `Name: field = value`, so `Name { field: value }` is an error
/// even on one line; a pattern, `let Name { x, y } = p`, keeps its braces.
fn check_braces(lines: &[Line]) -> Result<(), LayoutError> {
    for l in lines {
        if l.dsl {
            continue;
        }
        let t = &l.toks;
        let markup = crate::juxt::markup_brace_spans(t);
        let in_markup = |i: usize| markup.iter().any(|&(a, b)| i > a && i < b);
        let mut i = 0;
        while i < t.len() {
            if t[i].kind != Tk::Open('{') || t[i].synthetic || in_markup(i) {
                i += 1;
                continue;
            }
            let Some(j) = matching_close(t, i) else { break };
            let prev = (0..i).rev().find(|&k| !t[k].is_comment()).map(|k| &t[k]);
            // A brace group as an argument: `f ({ .. })`, `[ { .. }, { .. } ]`.
            let argument = prev.map_or(false, |p| matches!(p.kind, Tk::Open('(') | Tk::Open('[') | Tk::Comma));
            // A struct literal, `Name { .. }`: a path before the brace, not
            // preceded by a keyword, and not a pattern (a `let`/`if let`/`for`
            // before it, or a `=>` or `:` after it).
            let literal = !argument && literal_brace(t, i, j);
            let spans_lines = t[j].line != t[i].line;
            if literal {
                let name: String = literal_path(t, i).iter().map(|k| t[*k].text.clone()).collect::<Vec<_>>().join("");
                return Err(LayoutError {
                    msg: format!("a struct is built with `{name}: field = value` (inline, comma-separated) or `{name}:` with one field per line; braces build nothing"),
                    span: t[i].span,
                });
            }
            if spans_lines && !argument {
                let form = match prev.map(|p| &p.kind) {
                    Some(Tk::Punct) if prev.map_or(false, |p| p.text == "!") => "a macro body over several lines is `name! do:` with the body beneath",
                    Some(Tk::FatArrow) => "an arm's block over several lines is `=> do:` with the body beneath",
                    _ => "a block over several lines is written with `:` or `do:` and the body beneath; braces hold a block on one line",
                };
                return Err(LayoutError { msg: form.into(), span: t[i].span });
            }
            i += 1;
        }
    }
    Ok(())
}

/// The token indices of the path right before the `{` at `open`, if any:
/// `Point`, `Message.Move`, `Self`.
fn literal_path(t: &[Token], open: usize) -> Vec<usize> {
    let mut path = Vec::new();
    let mut k = open;
    while k > 0 {
        let p = &t[k - 1];
        if p.is_comment() {
            k -= 1;
            continue;
        }
        if p.kind == Tk::Ident && !crate::rules::is_keyword(&p.text) && !crate::rules::BLOCK_KEYWORDS.contains(&p.text.as_str()) && p.text != "do" {
            path.push(k - 1);
            k -= 1;
            if k > 0 && t[k - 1].kind == Tk::Dot {
                k -= 1;
                continue;
            }
            break;
        }
        break;
    }
    path.reverse();
    path
}

/// Is the brace at `open` (closing at `close`) a struct literal's?
fn literal_brace(t: &[Token], open: usize, close: usize) -> bool {
    let path = literal_path(t, open);
    let Some(&first) = path.first() else { return false };
    // What precedes the path decides whether a value can stand here: `=`,
    // `(`, `[`, `,`, `=>`, `return`, or the line's start. After `<-` it is a
    // field (`p <- kind {` closes a `match`); after a keyword it is a
    // pattern (`let Point {`) or a block (`match x {`).
    let before = (0..first).rev().find(|&k| !t[k].is_comment()).map(|k| &t[k]);
    match before {
        None => {}
        Some(b) => {
            let ok = matches!(b.kind, Tk::Eq | Tk::Open('(') | Tk::Open('[') | Tk::Comma | Tk::FatArrow) || b.is_kw("return");
            if !ok {
                return false;
            }
        }
    }
    // A block keyword earlier on the line at depth zero -- `if let Some x =
    // parens {` -- makes the brace that keyword's body, not a literal.
    let mut d = 0i32;
    for k in (0..first).rev() {
        match t[k].kind {
            Tk::Close(_) => d += 1,
            Tk::Open(_) => {
                if d == 0 {
                    break;
                }
                d -= 1;
            }
            Tk::Ident if d == 0 && matches!(t[k].text.as_str(), "if" | "while" | "match" | "for" | "loop" | "unsafe") => return false,
            Tk::Semi if d == 0 => break,
            _ => {}
        }
    }
    // What follows the close: `=>` (a match pattern), `:` (a parameter
    // pattern), or -- at depth zero, past the brackets a tuple pattern
    // puts around it -- a `=` that makes the whole thing a binding pattern,
    // `let ((a, b), Point { x, y }) = …`.
    if let Some(a) = t.get(close + 1) {
        if matches!(a.kind, Tk::FatArrow | Tk::Colon) {
            return false;
        }
    }
    let mut d = 0i32;
    for x in &t[close + 1..] {
        match x.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => {
                if d == 0 {
                    // Still inside the pattern's brackets: keep looking.
                    continue;
                }
                d -= 1;
            }
            Tk::Eq if d == 0 => return false,
            Tk::FatArrow if d == 0 => return false,
            Tk::Semi | Tk::Colon | Tk::LArrow => break,
            _ => {}
        }
    }
    // The body is fields: `name: value`, `name`, `..base`. An empty pair,
    // `Empty {}` -- a struct declared with `{}` -- has nothing to respell.
    let inner = &t[open + 1..close];
    if inner.is_empty() {
        return false;
    }
    let sig: Vec<&Token> = inner.iter().filter(|x| !x.is_comment()).collect();
    matches!(sig.first().map(|x| &x.kind), Some(Tk::Ident) | Some(Tk::DotDot))
        && (sig.len() == 1 || sig.iter().any(|x| x.kind == Tk::Colon || x.kind == Tk::Comma || x.kind == Tk::DotDot))
}

/// Parens isolate; they do not suspend the layout. A physical line inside
/// an open `(` follows the column rules as everywhere: it indents past the
/// line that opened the group, or it is the `)` that closes it (the paren
/// block's tail, at the opener's column). `[ .. ]` and `{ .. }` are Rust's
/// and their insides are not read.
fn check_group_lines(lines: &[Line]) -> Result<(), LayoutError> {
    for l in lines {
        if l.dsl {
            continue;
        }
        // Each open bracket with the indent of the line it was opened on.
        let mut stack: Vec<(char, usize)> = Vec::new();
        let mut line_indent = l.indent;
        for (i, t) in l.toks.iter().enumerate() {
            // A synthetic token (the pipes', `$`'s) has a borrowed position.
            if let Some(ind) = t.line_start.filter(|_| !t.synthetic && !t.dollar) {
                if i > 0 {
                    if let Some(&(ch, opened_at)) = stack.last() {
                        if ch == '(' && !matches!(t.kind, Tk::Close(_)) && ind <= opened_at && !t.is_comment() {
                            return Err(LayoutError {
                                msg: format!(
                                    "column {ind} is not inside the group opened at column {opened_at}: a line inside `( .. )` indents past the line that opened it, or is the `)` that closes it"
                                ),
                                span: t.span,
                            });
                        }
                    }
                }
                line_indent = ind;
            }
            match t.kind {
                Tk::Open(ch) => stack.push((ch, line_indent)),
                Tk::Close(_) => {
                    stack.pop();
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn check_continuations(lines: &[Line]) -> Result<(), LayoutError> {
    for l in lines {
        if l.dsl {
            continue;
        }
        // Markup in brace form, `view! { .. }` spanning lines: its lines
        // are not statements either.
        let markup = crate::juxt::markup_brace_spans(&l.toks);
        let in_markup = |i: usize| markup.iter().any(|&(a, b)| i > a && i < b);
        let mut d = 0i32;
        for (i, t) in l.toks.iter().enumerate() {
            match t.kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                _ => {}
            }
            if i == 0 || d != 0 || t.line_start.is_none() || t.kind != Tk::Ident || in_markup(i) {
                continue;
            }
            if !STATEMENT_KEYWORDS.contains(&t.text.as_str()) {
                continue;
            }
            // `if let` / `while let` / `&& let` chains may break before `let`.
            let prev = l.toks[..i].iter().rev().find(|p| !p.is_comment());
            if t.text == "let"
                && prev.map_or(false, |p| {
                    (p.kind == Tk::Ident && (p.text == "if" || p.text == "while"))
                        || (p.kind == Tk::Punct && (p.text == "&&" || p.text == "||"))
                })
            {
                continue;
            }
            let head: String = l
                .toks
                .iter()
                .take_while(|p| p.line_start.is_none() || std::ptr::eq(*p, &l.toks[0]))
                .take(2)
                .map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join(" ");
            return Err(LayoutError {
                msg: format!(
                    "`{}` starts a statement, but this line is indented past the statement above; align it with `{}` at column {}, or indent it under a block opener",
                    t.text, head, l.indent
                ),
                span: t.span,
            });
        }
    }
    Ok(())
}

/// `;` means "discard this block's tail value". A struct field, an enum
/// variant, a match arm and a use-tree entry are not blocks and have no tail,
/// so a trailing `;` there is meaningless -- and would collide with the `,`
/// this position inserts.
/// An `else` must follow a block whose *header* opens an if-chain. Mixing the
/// inline and multi-line forms otherwise orphans it:
///
/// ```text
/// if n == 1: 10
/// else: if n == 2: 20     <- the inner if closes at end of line
/// else: 30                <- nothing left for this to attach to
/// ```
/// A block opened by a bare closure header -- `map |x|:` with no isolating
/// paren -- ends its statement. A following line that begins `<-` was meant to
/// continue the chain, but there is nothing left to attach it to. The fix is
/// the isolated form, `map (|x|: .. )`, whose `)` says where the closure ends.
fn check_bare_closure_chain(nodes: &[Node]) -> Result<(), LayoutError> {
    let ends_in_closure = |toks: &[Token]| -> bool {
        let sig: Vec<&Token> = toks.iter().filter(|t| !t.is_comment()).collect();
        let mut i = sig.len();
        if i > 0 && sig[i - 1].kind == Tk::Colon {
            i -= 1;
        }
        if i > 0 && sig[i - 1].is_kw("do") {
            i -= 1;
        }
        i > 0 && sig[i - 1].kind == Tk::Punct
            && (sig[i - 1].text == "|" || sig[i - 1].text == "||")
    };
    let mut prev_bare_closure = false;
    for n in nodes {
        match n {
            Node::Line(l) => {
                let first = l.toks.iter().find(|t| !t.is_comment());
                if prev_bare_closure && first.map(|t| t.kind == Tk::LArrow).unwrap_or(false) {
                    return Err(LayoutError {
                        msg: "a chain cannot continue after a bare closure block; \
                              isolate the closure in parens, `f (|x|: ..) <- ..`, \
                              so the `)` says where it ends"
                            .into(),
                        span: first.unwrap().span,
                    });
                }
                prev_bare_closure = false;
            }
            Node::Markup(_) | Node::Tree(_) => prev_bare_closure = false,
            Node::Group(g) => {
                check_bare_closure_chain(&g.body)?;
                prev_bare_closure = false;
            }
            Node::Block(b) => {
                check_bare_closure_chain(&b.body)?;
                // Bare means: closure header, and no tail -- an isolated one
                // has its `)` and chain in the tail already.
                prev_bare_closure = ends_in_closure(&b.header.toks) && b.tail.is_empty();
            }
        }
    }
    Ok(())
}

fn check_else(nodes: &[Node]) -> Result<(), LayoutError> {
    // The header opens an `if` when an `if` stands at its effective top
    // level: depth zero, or the depth of a group left open at the header's
    // end -- a block opened inside a paren group, `( if c:`.
    let opens_if = |toks: &[Token]| -> bool {
        let mut d = 0i32;
        let mut ifs: Vec<i32> = Vec::new();
        for t in toks {
            match t.kind {
                Tk::Open(_) => d += 1,
                Tk::Close(_) => d -= 1,
                Tk::Ident if t.text == "if" => ifs.push(d),
                _ => {}
            }
        }
        ifs.iter().any(|&k| k == 0 || k == d)
    };
    let mut prev_opens_if = false;
    for n in nodes {
        let toks = match n {
            Node::Line(l) => &l.toks,
            Node::Block(b) => &b.header.toks,
            Node::Markup(_) | Node::Tree(_) => {
                prev_opens_if = false;
                continue;
            }
            Node::Group(g) => {
                check_else(&g.body)?;
                prev_opens_if = false;
                continue;
            }
        };
        let first = toks.iter().find(|t| !t.is_comment());
        let is_else = first.map(|t| t.is_kw("else")).unwrap_or(false);
        if is_else && !prev_opens_if {
            return Err(LayoutError {
                msg: "this `else` has no `if` to attach to; the preceding block \
                      closed at the end of its line"
                    .into(),
                span: first.unwrap().span,
            });
        }
        if toks.iter().any(|t| !t.is_comment()) {
            prev_opens_if = opens_if(toks);
        }
        if let Node::Block(b) = n {
            check_else(&b.body)?;
        }
    }
    Ok(())
}

/// Reject Rust call syntax written into a macro invocation.
fn check_juxt(nodes: &[Node], kind: BlockKind) -> Result<(), LayoutError> {
    let stmt = matches!(kind, BlockKind::Stmts | BlockKind::Arms | BlockKind::Macro | BlockKind::Rep);
    for n in nodes {
        match n {
            Node::Line(l) => {
                if let Err(e) = crate::juxt::check(&l.toks, stmt, false) {
                    return Err(LayoutError { msg: e.msg, span: e.span });
                }
            }
            // A matcher is not an expression; the arm's body is checked.
            Node::Block(b) if kind == BlockKind::MacroRules => check_juxt(&b.body, b.kind)?,
            Node::Block(b) => {
                if let Err(e) = crate::juxt::check(&b.header.toks, stmt, true) {
                    return Err(LayoutError { msg: e.msg, span: e.span });
                }
                check_juxt(&b.body, b.kind)?;
            }
            Node::Markup(_) => {}
            Node::Group(g) => check_juxt(&g.body, g.kind)?,
            Node::Tree(parts) => check_juxt(parts, BlockKind::Hsx)?,
        }
    }
    Ok(())
}

/// One parameter per group. `fn f (a: T, b: U)` is Rust's spelling of what
/// Harsh writes `fn f (a: T) (b: U)`, and one construct has one spelling.
fn check_fn_params(toks: &[Token]) -> Result<(), LayoutError> {
    check_bare_param(toks)?;
    let Some(groups) = crate::rules::fn_param_groups(toks) else { return Ok(()) };
    let n = groups.len();
    for (o, c) in groups {
        // A group holds a parameter, so an empty one holds nothing -- and a
        // function with no parameters is `fn name$`. The synthetic `()` that
        // `$` becomes is the only empty group allowed here.
        if c == o + 1 && toks[o].dollar && n > 1 {
            return Err(LayoutError {
                msg: "`$` means no parameters; a function with parameters lists its groups only".into(),
                span: toks[o].span,
            });
        }
        if c == o + 1 && !toks[o].synthetic {
            let name = toks[..o].iter().rev().find(|t| t.kind == Tk::Ident && t.text != "fn").map(|t| t.text.clone()).unwrap_or_default();
            return Err(LayoutError {
                msg: format!("`()` is the unit value; a function with no parameters is `fn {name}$`"),
                span: Span { lo: toks[o].span.lo, hi: toks[c].span.hi },
            });
        }
        if let Some(&i) = crate::rules::top_level_commas(toks, o, c).first() {
            return Err(LayoutError {
                msg: "a parameter group holds one parameter; write `(a: T) (b: U)`, one group each".into(),
                span: toks[i].span,
            });
        }
    }
    Ok(())
}

/// A lone parameter may drop its parentheses; with more than one, every
/// parameter is a group. A bare parameter followed by a group -- `fn add &self
/// (k: i32)` -- used to emit `fn add(&self(k: i32))`, invalid Rust, silently.
fn check_bare_param(toks: &[Token]) -> Result<(), LayoutError> {
    let sig: Vec<usize> = (0..toks.len()).filter(|&i| !toks[i].is_comment()).collect();
    let Some(fpos) = sig.iter().position(|&i| toks[i].is_kw("fn")) else { return Ok(()) };
    let mut p = fpos + 1;
    if p >= sig.len() || toks[sig[p]].kind != Tk::Ident {
        return Ok(());
    }
    p += 1;
    // Skip generics.
    if p < sig.len() && toks[sig[p]].kind == Tk::Lt {
        let mut d = 0i32;
        while p < sig.len() {
            match toks[sig[p]].kind {
                Tk::Lt => d += 1,
                Tk::Gt => {
                    d -= 1;
                    if d == 0 {
                        p += 1;
                        break;
                    }
                }
                _ => {}
            }
            p += 1;
        }
    }
    // The paren form is checked by `fn_param_groups`; here only the bare one.
    if p >= sig.len() || toks[sig[p]].kind == Tk::Open('(') {
        return Ok(());
    }
    let start = sig[p];
    // The bare region ends at a top-level `->`, or at the block colon / `;`.
    let mut d = 0i32;
    let mut q = p;
    while q < sig.len() {
        let t = &toks[sig[q]];
        match t.kind {
            // `[where ..]` ends the parameter list; it is not a group of it.
            Tk::Open('[') if d == 0 && toks.get(sig[q] + 1).map_or(false, |n| n.is_kw("where")) => break,
            Tk::Open(_) => {
                // A top-level group inside a bare parameter is fine only when
                // it is part of the type or pattern: after `:`, after `fn`,
                // or opening a tuple pattern. After a name or `self` it is a
                // second parameter.
                if d == 0 && q > p {
                    let prev = &toks[sig[q - 1]];
                    let param_like = (prev.kind == Tk::Ident && !crate::rules::is_keyword(&prev.text) && prev.text != "fn")
                        || prev.is_kw("self");
                    if param_like && t.kind == Tk::Open('(') {
                        return Err(LayoutError {
                            msg: format!(
                                "one parameter may be bare; with more, every one is a group: `({}) {}`",
                                toks[start..sig[q]].iter().map(|t| t.text.as_str()).collect::<Vec<_>>().join(" ").replace("& ", "&").replace("&mut ", "&mut "),
                                toks[sig[q]].text
                            ),
                            span: t.span,
                        });
                    }
                }
                d += 1;
            }
            Tk::Close(_) => d -= 1,
            Tk::Punct if d == 0 && t.text == "->" => break,
            Tk::Colon if d == 0 && q + 1 == sig.len() => break,
            Tk::Semi if d == 0 => break,
            _ => {}
        }
        q += 1;
    }
    Ok(())
}

fn check_semis(nodes: &[Node], kind: BlockKind) -> Result<(), LayoutError> {
    let named = match kind {
        BlockKind::Fields => "a field or variant",
        BlockKind::Arms => "a match arm",
        BlockKind::MacroRules => "a macro arm",
        BlockKind::Lit => "a field",
        _ => "",
    };
    // Which node is the block's last statement: a `;` is legal only there.
    let last = nodes.iter().rposition(|n| {
        let toks = match n {
            Node::Line(l) => &l.toks,
            Node::Block(b) => &b.header.toks,
            Node::Markup(_) | Node::Tree(_) => return true,
            Node::Group(_) => return true,
        };
        toks.iter().any(|t| !t.is_comment() && t.kind != Tk::Hash)
    });
    for (i, n) in nodes.iter().enumerate() {
        match n {
            Node::Markup(_) => {}
            Node::Tree(parts) => check_semis(parts, BlockKind::Hsx)?,
            Node::Group(g) => check_semis(&g.body, g.kind)?,
            Node::Line(l) => {
                let trailing = l.toks.iter().rev().find(|t| !t.is_comment());
                // A `;` means "emit one after the block's last statement". On
                // any earlier statement the newline already ends it, so the
                // `;` is redundant at best and a Rust habit at worst.
                if named.is_empty() && Some(i) != last {
                    if let Some(t) = trailing {
                        if t.kind == Tk::Semi {
                            return Err(LayoutError {
                                msg: "`;` is only written after a block's last statement; \
                                      a newline ends every other one"
                                    .into(),
                                span: t.span,
                            });
                        }
                    }
                }
                if !named.is_empty() {
                    if let Some(t) = trailing {
                        if t.kind == Tk::Semi {
                            return Err(LayoutError {
                                msg: format!("`;` is not valid at the end of {}", named),
                                span: t.span,
                            });
                        }
                        // The block inserts the commas Rust needs; a written
                        // one is a second spelling of the same thing, and
                        // would come out doubled.
                        if t.kind == Tk::Comma {
                            return Err(LayoutError {
                                msg: format!(
                                    "`,` is not written at the end of {}; the newline separates entries",
                                    named
                                ),
                                span: t.span,
                            });
                        }
                    }
                }
            }
            Node::Block(b) => check_semis(&b.body, b.kind)?,
        }
    }
    Ok(())
}

fn build_block(lines: &[Line], idx: &mut usize, min_indent: usize, outer: BlockKind, open: &mut Vec<usize>) -> Result<Vec<Node>, LayoutError> {
    let mut out = Vec::new();
    open.push(min_indent);
    // The column of the nested block that closed most recently, for the
    // message: a misaligned line usually meant to be in that one.
    let mut closed: Option<usize> = None;
    // Where the physical lines of that block's header start: an `else` may
    // align with the `if` it answers even when the `if` is a continuation
    // line (`let d =` / `if c:` / ... / `else:` under the `if`).
    let mut closed_header_cols: Vec<usize> = Vec::new();
    while *idx < lines.len() {
        let ln = &lines[*idx];
        if ln.indent < min_indent {
            break;
        }
        // A line deeper than this block's column but not a continuation
        // (those were merged already) has dedented from a nested block to a
        // column no block uses. It can only be a typo, and reading it as a
        // statement of this block would move it between scopes silently.
        let is_else = ln.toks.iter().find(|t| !t.is_comment()).map_or(false, |t| t.kind == Tk::Ident && t.text == "else");
        if ln.indent > min_indent && !ln.tail_of_block && !(is_else && closed_header_cols.contains(&ln.indent)) {
            let cols: Vec<String> = open.iter().map(|c| c.to_string()).collect();
            let sp = ln.toks.first().map(|t| t.span).unwrap_or(Span::new(0, 0));
            open.pop();
            let above = closed.map(|c| format!(" the block above is at column {},", c)).unwrap_or_default();
            return Err(LayoutError {
                msg: format!(
                    "column {} matches no open block;{} the enclosing block{} at column{} {}",
                    ln.indent,
                    above,
                    if cols.len() == 1 { " is" } else { "s are" },
                    if cols.len() == 1 { "" } else { "s" },
                    cols.join(", ")
                ),
                span: sp,
            });
        }
        *idx += 1;

        // The rest of a statement whose paren-block just closed: it belongs
        // to the block node emitted immediately before, not to a new line.
        if ln.tail_of_block {
            // A tail that ends in `:` -- `if xs <- any (|x|:` / body / `):`
            // -- opens the header's own block: the `if` body follows it.
            let tail_opens = opens_block(&ln.toks);
            if let Some(Node::Block(_)) = out.last() {
                let tail = match tail_opens {
                    Some(arrow) => {
                        let kind = classify(&ln.toks, outer);
                        let body_indent = lines.get(*idx).map(|l| l.indent).unwrap_or(0);
                        if *idx >= lines.len() || body_indent <= ln.indent {
                            let sp = last_significant(&ln.toks).map(|t| t.span).unwrap_or(Span::new(0, 0));
                            return Err(LayoutError { msg: "expected an indented block after this".into(), span: sp });
                        }
                        // The tail may open a markup or tree block as a
                        // header does: `) <- map (|line| view! do:`.
                        let kind = dsl_kind(lines, *idx, kind);
                        let block = if kind == BlockKind::Hsx {
                            hsx_block(lines, idx, ln, arrow)?
                        } else {
                            let body = build_block(lines, idx, body_indent, kind, open)?;
                            Block { header: clone_line(ln), arrow_opener: arrow, kind, body, tail: Vec::new() }
                        };
                        closed = Some(body_indent);
                        closed_header_cols = ln.toks.iter().filter_map(|t| t.line_start).collect();
                        vec![Node::Block(block)]
                    }
                    None => expand_item(&ln.toks, 0, ln.toks.len(), outer, ln)?,
                };
                if let Some(Node::Block(b)) = out.last_mut() {
                    // The tail belongs to the innermost block still open at
                    // the end: a tail that opened its own block (`) <- map
                    // (|line| view! do:`) takes the next tail, not the
                    // block that tail closed first.
                    let target = innermost_tail_block(b);
                    target.tail = tail;
                }
                continue;
            }
        }

        // A declaration header opens its body by grammar -- when a deeper
        // line follows. Without one it is a unit or tuple struct, a line.
        // A record variant in an enum body opens the same way.
        let next_deeper = lines.get(*idx).map_or(false, |n| n.indent > ln.indent);
        let opens = match opens_block(&ln.toks) {
            Some(a) => {
                if decl_header(&ln.toks) && !next_deeper {
                    None
                } else {
                    Some(a)
                }
            }
            None if outer == BlockKind::Fields && variant_header(&ln.toks) && next_deeper => Some(false),
            None => None,
        };
        match opens {
            None => {
                check_old_inline_literal(ln, outer)?;
                if inline_colon(&ln.toks, 0, ln.toks.len(), outer).is_some() {
                    let mut nodes = expand_item(&ln.toks, 0, ln.toks.len(), outer, ln)?;
                    if let Some(Node::Block(b)) = nodes.first_mut() {
                        b.header.comments = ln.comments.clone();
                        b.header.blank_before = ln.blank_before;
                    }
                    out.extend(nodes);
                } else {
                    // `$( .. )*` alone on its line is a repetition of
                    // whatever the block holds -- statements, arms, fields.
                    if let Some(g) = repetition(ln, outer)? {
                        out.push(Node::Group(g));
                        continue;
                    }
                    if outer == BlockKind::MacroRules {
                        check_macro_arm(&ln.toks)?;
                    }
                    if outer == BlockKind::Tree {
                        let mut all: Vec<Token> = ln.comments.clone();
                        all.extend(ln.toks.iter().cloned());
                        out.push(Node::Tree(hsx_parts(&all)?));
                        continue;
                    }
                    check_fn_params(&ln.toks)?;
                    out.push(Node::Line(clone_line(ln)))
                }
            }
            Some(arrow) => {
                // `use a.b:` with entries below never produced valid Rust and
                // would be a second spelling of the paren tree. One spelling.
                if ln.toks.iter().find(|t| !t.is_comment()).map_or(false, |t| t.kind == Tk::Ident && t.text == "use") {
                    return Err(LayoutError {
                        msg: "`use` does not open a block; group imports in parentheses, `use a.b.(C, D)`".into(),
                        span: last_significant(&ln.toks).map(|t| t.span).unwrap_or(Span::new(0, 0)),
                    });
                }
                check_fn_params(&ln.toks)?;
                check_old_marks(ln, outer)?;
                let kind = classify(&ln.toks, outer);
                let body_indent = lines.get(*idx).map(|l| l.indent).unwrap_or(0);
                if *idx >= lines.len() || body_indent <= ln.indent {
                    let sp = last_significant(&ln.toks).map(|t| t.span).unwrap_or(Span::new(0, 0));
                    return Err(LayoutError {
                        msg: "expected an indented block after this".into(),
                        span: sp,
                    });
                }
                let kind = dsl_kind(lines, *idx, kind);
                if kind == BlockKind::Hsx {
                    let b = hsx_block(lines, idx, ln, arrow)?;
                    closed = Some(body_indent);
                    out.push(Node::Block(b));
                    continue;
                }
                let body = build_block(lines, idx, body_indent, kind, open)?;
                closed = Some(body_indent);
                closed_header_cols = ln.toks.iter().filter_map(|t| t.line_start).collect();
                out.push(Node::Block(Block {
                    header: clone_line(ln),
                    arrow_opener: arrow,
                    kind,
                    body,
                    tail: Vec::new(),
                }));
            }
        }
    }
    open.pop();
    Ok(out)
}

/// The block a new tail attaches to: the last block, or the block its
/// own tail ends with, and so on down.
fn innermost_tail_block(b: &mut Block) -> &mut Block {
    // Walk the chain of tail blocks iteratively: the borrow checker will
    // not let a recursive `&mut` return borrow `b` twice.
    let mut cur: &mut Block = b;
    loop {
        let has_block = matches!(cur.tail.last(), Some(Node::Block(_)));
        if !has_block {
            return cur;
        }
        match cur.tail.last_mut() {
            Some(Node::Block(inner)) => cur = inner,
            _ => unreachable!(),
        }
    }
}

/// A macro `do:` body's kind, from the shape of its lines: markup, a brace
/// tree, or the Harsh block `kind` says.
fn dsl_kind(lines: &[Line], idx: usize, kind: BlockKind) -> BlockKind {
    if kind != BlockKind::Macro || idx >= lines.len() || !lines[idx].dsl {
        return kind;
    }
    let first_body = (idx..lines.len()).find(|&k| lines[k].toks.iter().any(|t| !t.is_comment())).unwrap_or(idx);
    if tree_first_line(&lines[first_body].toks) {
        return BlockKind::Tree;
    }
    if body_is_markup(lines, idx - 1, first_body) {
        return BlockKind::Hsx;
    }
    kind
}

/// HSX: the body lines are markup, flagged by `mark_hsx`. They are taken
/// flat, whatever their columns, and split into markup runs and holes.
fn hsx_block(lines: &[Line], idx: &mut usize, header: &Line, arrow: bool) -> Result<Block, LayoutError> {
    let mut toks: Vec<Token> = Vec::new();
    while *idx < lines.len() && lines[*idx].dsl {
        toks.extend(lines[*idx].comments.iter().cloned());
        toks.extend(lines[*idx].toks.iter().cloned());
        *idx += 1;
    }
    toks.sort_by_key(|t| t.span.lo);
    let body = hsx_parts(&toks)?;
    Ok(Block { header: clone_line(header), arrow_opener: arrow, kind: BlockKind::Hsx, body, tail: Vec::new() })
}

/// Lay out a delimited fragment -- the tokens strictly inside a hole or a
/// repetition -- on its own. The first token takes the column of the line
/// holding the opener as its own, so the body indents past that line and the
/// closer returns to it: the paren-block shape, applied to `{` and `$(`.
fn fragment(toks: &[Token], opener_line_indent: usize, kind: BlockKind) -> Result<Vec<Node>, LayoutError> {
    let mut v = toks.to_vec();
    if v.is_empty() {
        return Ok(Vec::new());
    }
    if v[0].line_start.is_none() {
        v[0].line_start = Some(opener_line_indent);
    }
    let base_line = v[0].line.saturating_sub(1);
    let mut lines = logical_lines(physical_lines_from(v, base_line));
    mark_hsx(&mut lines);
    check_braces(&lines)?;
    check_group_lines(&lines)?;
    check_continuations(&lines)?;
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    let min = lines[0].indent;
    let mut idx = 0usize;
    let nodes = build_block(&lines, &mut idx, min, kind, &mut Vec::new())?;
    if idx < lines.len() {
        let sp = lines[idx].toks.first().map(|t| t.span).unwrap_or(Span::new(0, 0));
        return Err(LayoutError {
            msg: format!(
                "column {} is left of the fragment's first line at column {}; its lines indent past the line that opens it",
                lines[idx].indent, min
            ),
            span: sp,
        });
    }
    Ok(nodes)
}

/// The column of the physical line holding `toks[i]`.
fn line_indent_of(toks: &[Token], i: usize) -> usize {
    (0..=i).rev().find_map(|k| toks[k].line_start).unwrap_or(0)
}

/// The index of the bracket closing the one opened at `i`.
fn matching_close(toks: &[Token], i: usize) -> Option<usize> {
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

/// Split an HSX body into markup runs and `{ .. }` holes (rule 1). The
/// contents of a hole are Harsh and are laid out as a fragment.
fn hsx_parts(toks: &[Token]) -> Result<Vec<Node>, LayoutError> {
    let mut out = Vec::new();
    let mut run: Vec<Token> = Vec::new();
    let mut i = 0usize;
    while i < toks.len() {
        // A `{ .. }` hole, or an attribute value isolated in parens after
        // `=`: `on:click=(|| body)`.
        let paren_value = toks[i].kind == Tk::Open('(') && i > 0 && toks[i - 1].kind == Tk::Eq;
        if toks[i].kind == Tk::Open('{') || paren_value {
            let Some(c) = matching_close(toks, i) else {
                return Err(LayoutError { msg: "unclosed bracket in markup".into(), span: toks[i].span });
            };
            if !run.is_empty() {
                out.push(Node::Markup(std::mem::take(&mut run)));
            }
            let body = fragment(&toks[i + 1..c], line_indent_of(toks, i), BlockKind::Stmts)?;
            out.push(Node::Group(Group {
                open: vec![toks[i].clone()],
                body,
                close: vec![toks[c].clone()],
                kind: BlockKind::Stmts,
                body_own_line: toks[i + 1].line_start.is_some(),
                bare: paren_value,
            }));
            i = c + 1;
            continue;
        }
        run.push(toks[i].clone());
        i += 1;
    }
    if !run.is_empty() {
        out.push(Node::Markup(run));
    }
    Ok(out)
}

/// A repetition that is the whole of its line, `$( .. )*`, in a macro body
/// (rule 2): it holds what its block holds -- statements, each of which
/// takes `;`; arms or fields, each with its `,`. A multi-line one has `$(`
/// line-final, the body beneath, `)*` as its own line.
fn repetition(ln: &Line, outer: BlockKind) -> Result<Option<Group>, LayoutError> {
    let toks = &ln.toks;
    let sig: Vec<usize> = (0..toks.len()).filter(|&i| !toks[i].is_comment()).collect();
    if sig.len() < 3
        || toks[sig[0]].kind != Tk::Punct
        || toks[sig[0]].text != "$"
        || toks[sig[0]].synthetic
        || toks[sig[1]].kind != Tk::Open('(')
        || toks[sig[1]].span.lo != toks[sig[0]].span.hi
    {
        return Ok(None);
    }
    let Some(c) = matching_close(toks, sig[1]) else { return Ok(None) };
    let after: Vec<usize> = sig.iter().copied().filter(|&i| i > c).collect();
    let is_op = |i: usize| toks[i].kind == Tk::Punct && matches!(toks[i].text.as_str(), "*" | "+" | "?");
    match after.as_slice() {
        [] => {}
        [op] if is_op(*op) => {}
        [sep, op] if is_op(*op) => {
            return Err(LayoutError {
                msg: format!(
                    "a repetition takes no separator in Harsh; its statements take `;` and its expressions `,` from the layout, so write `)`{}",
                    toks[*op].text
                ),
                span: toks[*sep].span,
            });
        }
        // Something follows the repetition: it is part of an expression, not
        // a statement of its own.
        _ => return Ok(None),
    }
    if c == sig[1] + 1 {
        return Err(LayoutError { msg: "an empty repetition".into(), span: toks[sig[1]].span });
    }
    let kind = match outer {
        BlockKind::Stmts | BlockKind::Macro | BlockKind::Rep => BlockKind::Rep,
        k => k,
    };
    let body_own_line = toks[sig[1] + 1].line_start.is_some();
    let body = fragment(&toks[sig[1] + 1..c], ln.indent, kind)?;
    Ok(Some(Group {
        open: vec![toks[sig[0]].clone(), toks[sig[1]].clone()],
        body,
        close: toks[c..].to_vec(),
        kind,
        body_own_line,
        bare: false,
    }))
}

/// A one-line arm of a `macro_rules!` body must be `( matcher ) => group`,
/// the transcriber a bracketed group of Rust's. The Harsh spelling of a
/// transcriber is the block, `=> do:`.
fn check_macro_arm(toks: &[Token]) -> Result<(), LayoutError> {
    let sig: Vec<usize> = (0..toks.len()).filter(|&i| !toks[i].is_comment()).collect();
    let Some(first) = sig.first() else { return Ok(()) };
    if toks[*first].kind == Tk::Hash {
        return Ok(());
    }
    let mut d = 0i32;
    let mut arrow = None;
    for &i in &sig {
        match toks[i].kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => d -= 1,
            Tk::FatArrow if d == 0 => {
                arrow = Some(i);
                break;
            }
            _ => {}
        }
    }
    let Some(arrow) = arrow else {
        return Err(LayoutError {
            msg: "expected a macro arm, `( matcher ) => do:`".into(),
            span: toks[*first].span,
        });
    };
    let mut rest: Vec<usize> = sig.iter().copied().filter(|&i| i > arrow).collect();
    // A written `;` or `,` is `check_semis`'s to reject, by name.
    if rest.last().map_or(false, |&i| matches!(toks[i].kind, Tk::Semi | Tk::Comma)) {
        rest.pop();
    }
    let group = match rest.first() {
        Some(&o) if matches!(toks[o].kind, Tk::Open(_)) => matching_close(toks, o).map(|c| (o, c)),
        _ => None,
    };
    let ok = matches!(group, Some((_, c)) if rest.last() == Some(&c));
    if !ok {
        let at = rest.first().copied().unwrap_or(arrow);
        return Err(LayoutError {
            msg: "a macro arm's transcriber is a block: `=> do:` with the body beneath, `=> do: expr` on one line, or Rust's `=> { .. }`".into(),
            span: toks[at].span,
        });
    }
    Ok(())
}

fn clone_line(l: &Line) -> Line {
    Line {
        toks: l.toks.clone(),
        comments: l.comments.clone(),
        blank_before: l.blank_before,
        indent: l.indent,
        tail_of_block: l.tail_of_block,
        dsl: l.dsl,
    }
}

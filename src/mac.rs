//! `macro_rules~` — Harsh's own declarative macros: **the definition**.
//!
//! Harsh has two macro systems (`docs/dev/MACRO-DESIGN.md`). `macro_rules!`
//! is Rust's, copied verbatim (`rawzone`). `macro_rules~` is Harsh's: its
//! matchers and transcribers are Harsh, and it unfolds **in Harsh**, before
//! anything is transpiled, so what a macro produces is ordinary Harsh that
//! meets the ordinary rules.
//!
//! This module reads a definition into a tree and says what is wrong with it,
//! on the author's own lines. It does not expand anything yet.
//!
//! # The shape of a definition
//!
//! ```text
//! macro_rules~ push_all
//!     (($v:ident) $( ($x:expr) )*) => do:
//!         $( $v <- push $x )*
//!     (($v:ident)) => do:
//!         ()
//! ```
//!
//! `macro_rules~ name` takes no mark: nothing but a name can follow, so the
//! header ends itself, as `struct Point` and `impl Foo` do. The arms are a
//! *specification* block -- Rust separates them with `;`, Harsh separates
//! them by layout, and an arm ends where the next returns to the arm column.
//!
//! # The fragments
//!
//! Fifteen, in Harsh's terms rather than Rust's. Where Rust's meaning makes
//! no sense here it is replaced, not carried: `path` is dotted, `pat` has no
//! `pat_param` (Rust edition history), `expr` may span a continued line, and
//! `stmt`, `line`, `block` and `entry` are shapes the layout gives us.
//!
//! | kind | captures |
//! |---|---|
//! | `ident` | an identifier or keyword |
//! | `lifetime` | `'a` |
//! | `literal` | a literal, with an optional `-` |
//! | `tt` | one token, or one bracketed group (indentation is not a token) |
//! | `expr` | a Harsh expression, across a continued line |
//! | `stmt` | a logical line **and whatever is indented under it** |
//! | `line` | the logical line alone, without its block |
//! | `block` | an opener and its indented body, or a one-line brace form |
//! | `pat` | a Harsh pattern: `Some n`, `Point\ x, y`, `_` |
//! | `ty` | a type as Harsh writes it |
//! | `path` | a dotted path: `std.collections.HashMap` |
//! | `item` | `fn`, `struct`, `impl`, `mod`, `use`, … with its body |
//! | `entry` | one item of a `\` list |
//! | `meta` | an attribute's content, `derive Debug` |
//! | `vis` | `pub`, `pub(crate)`, or nothing |
//!
//! # Where a fragment ends
//!
//! Rust's follow-set table exists to keep a token-stream parser unambiguous.
//! Harsh knows more from the page, so the rule is local: a fragment ends at
//! the end of its logical line, at a dedent, at the close of its enclosing
//! group, or at the next literal token the matcher names. `(($a:expr)
//! ($b:expr))` is therefore fine, where Rust would refuse `($a:expr $b:expr)`.

use crate::lex::{Span, Tk, Token};

/// What a metavariable captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ident,
    Lifetime,
    Literal,
    Tt,
    Expr,
    Stmt,
    Line,
    Block,
    Pat,
    Ty,
    Path,
    Item,
    Entry,
    Meta,
    Vis,
}

impl Kind {
    pub fn from_name(s: &str) -> Option<Kind> {
        Some(match s {
            "ident" => Kind::Ident,
            "lifetime" => Kind::Lifetime,
            "literal" => Kind::Literal,
            "tt" => Kind::Tt,
            "expr" => Kind::Expr,
            "stmt" => Kind::Stmt,
            "line" => Kind::Line,
            "block" => Kind::Block,
            "pat" => Kind::Pat,
            "ty" => Kind::Ty,
            "path" => Kind::Path,
            "item" => Kind::Item,
            "entry" => Kind::Entry,
            "meta" => Kind::Meta,
            "vis" => Kind::Vis,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Ident => "ident",
            Kind::Lifetime => "lifetime",
            Kind::Literal => "literal",
            Kind::Tt => "tt",
            Kind::Expr => "expr",
            Kind::Stmt => "stmt",
            Kind::Line => "line",
            Kind::Block => "block",
            Kind::Pat => "pat",
            Kind::Ty => "ty",
            Kind::Path => "path",
            Kind::Item => "item",
            Kind::Entry => "entry",
            Kind::Meta => "meta",
            Kind::Vis => "vis",
        }
    }
}

/// How many times a repetition runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rep {
    /// `*` — zero or more
    Star,
    /// `+` — one or more
    Plus,
    /// `?` — zero or one; takes no separator
    Opt,
}

/// One element of a matcher.
#[derive(Debug, Clone)]
pub enum Elem {
    /// A token written verbatim, which must appear as written.
    Lit(Token),
    /// `$name:kind`
    Var { name: String, kind: Kind, span: Span },
    /// A bracketed group, and what is inside it.
    Group { open: char, body: Vec<Elem>, span: Span },
    /// `$( body )sep rep`
    Repeat {
        body: Vec<Elem>,
        sep: Option<Token>,
        rep: Rep,
        span: Span,
    },
}

/// One arm: a matcher, and the transcriber it produces.
#[derive(Debug, Clone)]
pub struct Arm {
    pub matcher: Vec<Elem>,
    /// The transcriber's tokens and their layout, kept as the author wrote
    /// them: a Harsh macro unfolds in Harsh, so the transcriber is Harsh and
    /// its shape is what the expansion's shape will be.
    pub body: Vec<Token>,
    pub span: Span,
}

/// A whole `macro_rules~` definition.
#[derive(Debug, Clone)]
pub struct Def {
    pub name: String,
    pub arms: Vec<Arm>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Error {
    pub msg: String,
    pub span: Span,
}

fn err<T>(span: Span, msg: impl Into<String>) -> Result<T, Error> {
    Err(Error { msg: msg.into(), span })
}

/// An arm's matcher, written `( … )`: its own parentheses are stripped
/// *before* parsing, so `(($e:expr))` is the arm's parentheses around a
/// metavariable, and `($e:expr)` alone is a bare metavariable, refused
/// (the user, 2026-09-25). Anything else is parsed whole.
pub fn parse_arm_matcher(toks: &[Token]) -> Result<Vec<Elem>, Error> {
    if toks.first().map_or(false, |t| t.kind == Tk::Open('(')) && matching_close(toks, 0) == Some(toks.len() - 1) {
        return parse_matcher(&toks[1..toks.len() - 1]);
    }
    parse_matcher(toks)
}

/// Parse a matcher's elements from the tokens between its delimiters.
pub fn parse_matcher(toks: &[Token]) -> Result<Vec<Elem>, Error> {
    let mut i = 0;
    parse_elems(toks, &mut i, None)
}

fn parse_elems(toks: &[Token], i: &mut usize, close: Option<char>) -> Result<Vec<Elem>, Error> {
    let mut out = Vec::new();
    while *i < toks.len() {
        let t = &toks[*i];
        match t.kind {
            Tk::Close(c) => {
                if close == Some(c) {
                    return Ok(out);
                }
                return err(t.span, format!("`{c}` closes nothing here"));
            }
            // `($name:kind)`: a metavariable, its parentheses part of the
            // construct as `$( … )*`'s are the repetition's -- Harsh's
            // parameter shape (the user, 2026-09-25). Any other group is
            // literal, matched as written.
            Tk::Open('(')
                if toks.get(*i + 1).map_or(false, |d| d.text == "$")
                    && toks.get(*i + 2).map_or(false, |n| n.kind == Tk::Ident)
                    && toks.get(*i + 3).map_or(false, |c| c.kind == Tk::Colon)
                    && toks.get(*i + 5).map_or(false, |c| c.kind == Tk::Close(')')) =>
            {
                let dollar = toks[*i + 1].span;
                *i += 2;
                let var = parse_var(toks, i, dollar)?;
                *i += 1; // the `)`
                out.push(var);
            }
            Tk::Open(c) => {
                let span = t.span;
                *i += 1;
                let body = parse_elems(toks, i, Some(matching(c)))?;
                if *i >= toks.len() {
                    return err(span, format!("`{c}` is never closed"));
                }
                *i += 1; // the closer
                out.push(Elem::Group { open: c, body, span });
            }
            _ if t.text == "$" => {
                let span = t.span;
                *i += 1;
                let Some(next) = toks.get(*i) else {
                    return err(span, "`$` at the end of a matcher: write `$name:kind` or `$( … )*`");
                };
                if next.kind == Tk::Open('(') {
                    *i += 1;
                    let body = parse_elems(toks, i, Some(')'))?;
                    if *i >= toks.len() {
                        return err(span, "a repetition `$( … )` is never closed");
                    }
                    *i += 1; // the `)`
                    let (sep, rep) = parse_rep_tail(toks, i, span)?;
                    out.push(Elem::Repeat { body, sep, rep, span });
                } else if next.kind == Tk::Ident {
                    // A bare `$name:kind` is refused: a metavariable is
                    // written in its parentheses, `($name:kind)`, which keeps
                    // every one the same shape (the user, 2026-09-25).
                    let name = &next.text;
                    return match (toks.get(*i + 1), toks.get(*i + 2)) {
                        (Some(c), Some(k)) if c.kind == Tk::Colon => {
                            err(span, format!("a metavariable is written in its parentheses: `(${name}:{})`", k.text))
                        }
                        _ => err(span, format!("`${name}` needs a kind, in its parentheses: `(${name}:expr)`, `(${name}:ident)`, …")),
                    };
                } else {
                    out.push(parse_var(toks, i, span)?);
                }
            }
            _ => {
                out.push(Elem::Lit(t.clone()));
                *i += 1;
            }
        }
    }
    if let Some(c) = close {
        return err(
            toks.last().map(|t| t.span).unwrap_or(Span::new(0, 0)),
            format!("a group is never closed; `{c}` is missing"),
        );
    }
    Ok(out)
}

/// `$name:kind` — the `$` is already consumed.
fn parse_var(toks: &[Token], i: &mut usize, dollar: Span) -> Result<Elem, Error> {
    let name_tok = &toks[*i];
    if name_tok.kind != Tk::Ident {
        return err(name_tok.span, "a capture is `$name:kind`, as in `$x:expr`");
    }
    let name = name_tok.text.clone();
    *i += 1;
    let Some(colon) = toks.get(*i) else {
        return err(name_tok.span, format!("`${name}` needs a kind: `${name}:expr`, `${name}:ident`, …"));
    };
    if colon.kind != Tk::Colon {
        return err(colon.span, format!("`${name}` needs a kind: `${name}:expr`, `${name}:ident`, …"));
    }
    *i += 1;
    let Some(kind_tok) = toks.get(*i) else {
        return err(colon.span, "a capture's kind is missing");
    };
    let Some(kind) = Kind::from_name(&kind_tok.text) else {
        return err(
            kind_tok.span,
            format!(
                "`{}` is not a capture kind; Harsh has ident, lifetime, literal, tt, expr, stmt, line, block, pat, ty, path, item, entry, meta, vis",
                kind_tok.text
            ),
        );
    };
    *i += 1;
    Ok(Elem::Var { name, kind, span: dollar })
}

/// What follows a repetition's `)`: an optional separator, then `*`, `+`, `?`.
fn parse_rep_tail(toks: &[Token], i: &mut usize, span: Span) -> Result<(Option<Token>, Rep), Error> {
    // `)` then: `*`, `+` or `?`, or a separator then one of those. The
    // grammar is Rust's exactly: a repetition with no marker is refused
    // (ruling 6, 2026-09-19). A separator makes sense only before `*` or `+`.
    let marker = |t: &Token| match t.text.as_str() {
        "*" => Some(Rep::Star),
        "+" => Some(Rep::Plus),
        "?" => Some(Rep::Opt),
        _ => None,
    };
    if let Some(t) = toks.get(*i) {
        if let Some(r) = marker(t) {
            *i += 1;
            return Ok((None, r));
        }
        // a separator, then a marker
        if let Some(n) = toks.get(*i + 1) {
            if let Some(r) = marker(n) {
                let sep = t.clone();
                if r == Rep::Opt {
                    return err(sep.span, "`?` repeats at most once, so it takes no separator");
                }
                *i += 2;
                return Ok((Some(sep), r));
            }
        }
    }
    err(
        span,
        "a repetition `$( … )` takes `*`, `+` or `?`; a group that must appear \
         exactly once is written without the `$( … )`",
    )
}

fn matching(open: char) -> char {
    match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        c => c,
    }
}

/// Every capture a matcher binds, with the depth of repetition it sits at.
/// A transcriber may only use a name the matcher bound, at the same depth.
pub fn bindings(elems: &[Elem]) -> Vec<(String, Kind, usize)> {
    fn walk(elems: &[Elem], depth: usize, out: &mut Vec<(String, Kind, usize)>) {
        for e in elems {
            match e {
                Elem::Var { name, kind, .. } => out.push((name.clone(), *kind, depth)),
                Elem::Group { body, .. } => walk(body, depth, out),
                Elem::Repeat { body, .. } => walk(body, depth + 1, out),
                Elem::Lit(_) => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(elems, 0, &mut out);
    out
}

/// A name may not be bound twice: the second binding would silently win.
pub fn check_unique(elems: &[Elem]) -> Result<(), Error> {
    let mut seen: Vec<String> = Vec::new();
    fn walk(elems: &[Elem], seen: &mut Vec<String>) -> Result<(), Error> {
        for e in elems {
            match e {
                Elem::Var { name, span, .. } => {
                    if seen.contains(name) {
                        return err(*span, format!("`${name}` is captured twice in one matcher"));
                    }
                    seen.push(name.clone());
                }
                Elem::Group { body, .. } | Elem::Repeat { body, .. } => walk(body, seen)?,
                Elem::Lit(_) => {}
            }
        }
        Ok(())
    }
    walk(elems, &mut seen)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher(src: &str) -> Result<Vec<Elem>, Error> {
        let toks = crate::lex::lex(src).expect("lex");
        let sig: Vec<Token> = toks.into_iter().filter(|t| !t.is_comment()).collect();
        parse_arm_matcher(&sig)
    }

    #[test]
    fn a_capture_and_its_kind() {
        let m = matcher("(($x:expr))").expect("parse");
        let b = bindings(&m);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].0, "x");
        assert_eq!(b[0].1, Kind::Expr);
        assert_eq!(b[0].2, 0, "not inside a repetition");
    }

    #[test]
    fn every_harsh_kind_is_known() {
        for k in [
            "ident", "lifetime", "literal", "tt", "expr", "stmt", "line", "block", "pat", "ty",
            "path", "item", "entry", "meta", "vis",
        ] {
            let m = matcher(&format!("(($x:{k}))")).unwrap_or_else(|e| panic!("{k}: {}", e.msg));
            assert_eq!(bindings(&m)[0].1.name(), k);
        }
    }

    #[test]
    fn rusts_edition_kinds_are_refused_by_name() {
        for k in ["pat_param", "expr_2021"] {
            let e = matcher(&format!("(($x:{k}))")).err().expect("refused");
            assert!(e.msg.contains("is not a capture kind"), "{}", e.msg);
        }
    }

    #[test]
    fn a_repetition_carries_its_separator_and_operator() {
        let m = matcher("($( ($x:expr) ),*)").expect("parse");
        let Elem::Repeat { sep, rep, .. } = &m[0] else { panic!("{m:?}") };
        assert_eq!(sep.as_ref().map(|t| t.text.as_str()), Some(","));
        assert_eq!(*rep, Rep::Star);
        // the capture is one level deep
        assert_eq!(bindings(&m)[0].2, 1);
    }

    #[test]
    fn no_separator_is_the_juxtaposed_form() {
        let m = matcher("($( ($x:expr) )*)").expect("parse");
        let Elem::Repeat { sep, rep, .. } = &m[0] else { panic!() };
        assert!(sep.is_none());
        assert_eq!(*rep, Rep::Star);
    }

    #[test]
    fn nested_repetitions_count_their_depth() {
        let m = matcher("($( ($( ($x:expr) )*) )*)").expect("parse");
        assert_eq!(bindings(&m)[0].2, 2);
    }

    #[test]
    fn an_optional_repetition_takes_no_separator() {
        assert!(matcher("($( ($x:expr) )?)").is_ok());
        let e = matcher("($( ($x:expr) ),?)").err().expect("refused");
        assert!(e.msg.contains("takes no separator"), "{}", e.msg);
    }

    #[test]
    fn a_repetition_without_a_marker_is_refused() {
        // Rust's grammar exactly (ruling 6): `$( … )` takes `*`, `+` or `?`.
        // The caret lands on the `$` that opens the repetition.
        for src in ["($( ($x:expr) ))", "($( ($x:expr) ) ($y:expr))", "(($a:expr) $(,))"] {
            let e = matcher(src).err().expect("refused");
            assert!(e.msg.contains("takes `*`, `+` or `?`"), "{src}: {}", e.msg);
            assert_eq!(e.span.lo as usize, src.find("$(").unwrap(), "{src}");
        }
        // The trailing-comma idiom is untouched.
        assert!(matcher("($( ($x:expr) ),* $(,)?)").is_ok());
    }

    /// A metavariable is written `($name:kind)`, its parentheses part of the
    /// construct (the user, 2026-09-25); the bare form is refused, naming it,
    /// in a repetition too.
    #[test]
    fn a_metavariable_is_written_in_its_parentheses() {
        let m = matcher("(($k:expr) => ($v:expr))").expect("parse");
        assert_eq!(bindings(&m).len(), 2);
        // The arm's own parentheses around a bare metavariable, a bare one in
        // a repetition, and one beside a literal: each refused, naming its form.
        for (src, form) in [("($x:expr)", "($x:expr)"), ("(($a:ident) $( $x:expr )*)", "($x:expr)"), ("(add $b:tt)", "($b:tt)")] {
            let e = matcher(src).err().expect("refused");
            assert!(e.msg.contains("a metavariable is written in its parentheses") && e.msg.contains(form), "{src}: {}", e.msg);
        }
    }

    #[test]
    fn two_fragments_in_their_own_groups_are_fine() {
        // Rust refuses `($a:expr $b:expr)` -- its follow set says nothing may
        // follow `expr` but `=>`, `,`, `;`. In Harsh each group says where it
        // ends, so this is ordinary.
        let m = matcher("(($a:expr) ($b:expr))").expect("parse");
        assert_eq!(bindings(&m).len(), 2);
    }

    #[test]
    fn literal_tokens_are_kept_as_written() {
        let m = matcher("(add ($a:expr) to ($b:expr))").expect("parse");
        let body = &m;
        assert!(matches!(&body[0], Elem::Lit(t) if t.text == "add"));
        assert!(matches!(&body[2], Elem::Lit(t) if t.text == "to"));
    }

    #[test]
    fn a_name_is_not_captured_twice() {
        let m = matcher("(($x:expr) ($x:ident))").expect("parse");
        let e = check_unique(&m).err().expect("refused");
        assert!(e.msg.contains("captured twice"), "{}", e.msg);
    }

    #[test]
    fn a_capture_needs_a_kind() {
        let e = matcher("($x)").err().expect("refused");
        assert!(e.msg.contains("needs a kind"), "{}", e.msg);
    }
}

// ---------------------------------------------------------------------------
// Stage 2: matching a call against an arm
// ---------------------------------------------------------------------------

/// What one capture bound, kept as the caller's tokens with the caller's
/// syntax context — substitution must not touch either.
#[derive(Debug, Clone)]
pub enum Binding {
    /// A capture at this depth: the tokens it matched.
    One(Vec<Token>),
    /// A capture inside a repetition: one entry per repeat.
    Many(Vec<Binding>),
}

impl Binding {
    pub fn tokens(&self) -> &[Token] {
        match self {
            Binding::One(t) => t,
            Binding::Many(_) => &[],
        }
    }
}

/// The captures an arm bound, by name.
pub type Bindings = std::collections::HashMap<String, Binding>;

/// Try each arm in order; the first whose matcher consumes the whole call
/// wins. Rust's rule, and for Rust's reason: an arm is a pattern, and a
/// pattern that fits is the author's answer.
pub fn match_call<'a>(def: &'a Def, call: &[Token]) -> Result<(&'a Arm, Bindings), Error> {
    let mut why: Vec<String> = Vec::new();
    // The stream is what the caller wrote, parens included: nothing after
    // the `~` is Harsh's (2026-09-19). A paren group is one token, so
    // `m~ (1, 2)` passes a tuple, and a matcher that wants the tuple's parts
    // names the group: `(($a:expr, $b:expr))`.
    for (n, arm) in def.arms.iter().enumerate() {
        let mut b = Bindings::new();
        let mut i = 0;
        match match_elems_in(&arm.matcher, call, &mut i, &mut b, true) {
            Ok(()) if i == call.len() => return Ok((arm, b)),
            Ok(()) => why.push(format!("arm {}: matched, but {} token(s) were left over", n + 1, call.len() - i)),
            Err(e) => why.push(format!("arm {}: {}", n + 1, e.msg)),
        }
    }
    let span = call.first().map(|t| t.span).unwrap_or(Span::new(0, 0));
    err(
        span,
        format!("no arm of `{}~` matches this call\n  {}", def.name, why.join("\n  ")),
    )
}

/// `match_elems_in` with the enclosing repetition's separator, which bounds
/// the body's last fragment on every round but the last.
fn match_elems_sep(
    elems: &[Elem],
    call: &[Token],
    i: &mut usize,
    b: &mut Bindings,
    whole: bool,
    follow: &[String],
) -> Result<(), Error> {
    match_elems_impl(elems, call, i, b, whole, follow)
}

/// `whole` says the slice is one argument's contents -- the inside of a
/// group -- so a fragment may span all of it. At a call's top level the
/// arguments are juxtaposed, so a fragment takes one atom instead: `sum~ 1 2`
/// passes two arguments, exactly as `f 1 2` does.
fn match_elems_in(
    elems: &[Elem],
    call: &[Token],
    i: &mut usize,
    b: &mut Bindings,
    whole: bool,
) -> Result<(), Error> {
    match_elems_impl(elems, call, i, b, whole, &[])
}

/// The literal tokens that can begin `elems` -- looking into a repetition,
/// and past one that may match nothing (`*`, `?`) to what follows it. When
/// `elems` can match nothing at all, what may follow it (`outer`) counts too.
/// A fragment first in `elems` contributes nothing: it can begin with any
/// token, so it names no literal to stop at.
fn first_lits(elems: &[Elem], outer: &[String]) -> Vec<String> {
    let Some(first) = elems.first() else { return outer.to_vec() };
    match first {
        Elem::Lit(t) => vec![t.text.clone()],
        Elem::Repeat { body, rep, .. } => {
            let mut v = first_lits(body, &[]);
            if *rep != Rep::Plus {
                for s in first_lits(&elems[1..], outer) {
                    if !v.contains(&s) {
                        v.push(s);
                    }
                }
            }
            v
        }
        _ => Vec::new(),
    }
}

fn match_elems_impl(
    elems: &[Elem],
    call: &[Token],
    i: &mut usize,
    b: &mut Bindings,
    whole: bool,
    follow: &[String],
) -> Result<(), Error> {
    for (n, e) in elems.iter().enumerate() {
        match e {
            Elem::Lit(want) => {
                let Some(got) = call.get(*i) else {
                    return err(want.span, format!("expected `{}`, and the call ended", want.text));
                };
                if got.text != want.text {
                    return err(got.span, format!("expected `{}`, found `{}`", want.text, got.text));
                }
                *i += 1;
            }
            Elem::Group { open, body, span } => {
                let Some(got) = call.get(*i) else {
                    return err(*span, format!("expected `{open}`, and the call ended"));
                };
                let Tk::Open(c) = got.kind else {
                    // A group in a matcher is a group the caller must write:
                    // the stream is tokens, and a paren group is one of them.
                    return err(got.span, format!("expected `{open}`, found `{}`", got.text));
                };
                if c != *open {
                    return err(got.span, format!("expected `{open}`, found `{c}`"));
                }
                let close = match matching_close(call, *i) {
                    Some(c) => c,
                    None => return err(got.span, format!("`{c}` is never closed in this call")),
                };
                let inner = &call[*i + 1..close];
                let mut j = 0;
                match_elems_impl(body, inner, &mut j, b, true, &[])?;
                if j != inner.len() {
                    return err(inner[j].span, format!("`{}` was not expected here", inner[j].text));
                }
                *i = close + 1;
            }
            Elem::Var { name, kind, .. } => {
                // The extent rule (2026-09-19): a fragment runs to the next
                // thing the matcher names -- a literal token, or the
                // repetition's separator -- and when nothing follows but
                // another fragment, it takes one atom, as an application's
                // argument does. `$a:expr , $b:expr` spans to the comma;
                // `$a:expr $b:expr` is juxtaposition. Both are Harsh's.
                //
                // "What the matcher names next" is a follow set, not one
                // token (2026-09-20): a repetition that comes next names the
                // literal its body begins with, and one that may match
                // nothing lets what follows it count too. So in
                // `$it:expr $(if $c:expr)*` the fragment stops at `if` --
                // before this it took one atom and `0..4` was cut to `0`.
                let stops = first_lits(&elems[n + 1..], follow);
                let spans = !stops.is_empty() || (elems.get(n + 1).is_none() && whole);
                let taken = take_fragment_until(*kind, call, *i, spans, &stops)?;
                b.insert(name.clone(), Binding::One(call[*i..taken].to_vec()));
                *i = taken;
            }
            Elem::Repeat { body, sep, rep, span } => {
                let rest = &elems[n + 1..];
                let after = first_lits(rest, follow);
                match_repeat(body, sep.as_ref(), *rep, rest, &after, call, i, b, *span, whole)?;
            }
        }
    }
    Ok(())
}

/// A repetition: match its body as many times as the call allows, stopping
/// when the body no longer matches or when what follows the repetition does.
#[allow(clippy::too_many_arguments)]
fn match_repeat(
    body: &[Elem],
    sep: Option<&Token>,
    rep: Rep,
    rest: &[Elem],
    after: &[String],
    call: &[Token],
    i: &mut usize,
    b: &mut Bindings,
    span: Span,
    whole: bool,
) -> Result<(), Error> {
    let names: Vec<(String, Kind)> = bindings(body).into_iter().map(|(n, k, _)| (n, k)).collect();
    let mut rounds: Vec<Bindings> = Vec::new();

    loop {
        if rep == Rep::Opt && !rounds.is_empty() {
            break;
        }
        // A round is tried on a copy: a half-matched round must leave nothing
        // behind, and `i` must not move.
        let mut probe = *i;
        // A repetition without a separator ends where what follows it begins:
        // the `;` after a matrix row is the outer repetition's, never one more
        // element of the row. Without this a one-atom fragment took the `;`
        // itself (an `expr` accepts any single token as an atom -- the known
        // weakness recorded for the user's macro testing).
        if sep.is_none() && call.get(probe).map_or(false, |t| after.iter().any(|a| *a == t.text)) {
            break;
        }
        if !rounds.is_empty() {
            if let Some(s) = sep {
                match call.get(probe) {
                    Some(t) if t.text == s.text => probe += 1,
                    _ => break,
                }
            }
        }
        let mut round = Bindings::new();
        let mut j = probe;
        // Inside the body, "what follows" is the next round -- its separator,
        // or the literal its body begins with -- or whatever follows the
        // repetition. A trailing fragment stops at any of them; with none,
        // it takes one atom.
        let mut body_follow: Vec<String> = Vec::new();
        // With no separator, a body that *begins with a fragment* is followed
        // by another fragment -- the next round's -- and a fragment followed
        // directly by a fragment takes one atom: juxtaposition, the 0.1.11
        // rule. It must win over the follow set, or `$( $x:expr )*` spans to
        // whatever comes after the repetition: `m~ [1.0 2.0; 3.0 4.0]` read
        // the row `1.0 2.0` as one element and emitted `1.0(2.0)`
        // (2026-09-21, the first matrix literal). A body that begins with a
        // literal (`$(if $c:expr)*`) still stops at it, and a separator
        // (`$( $x:expr ),*`) still bounds each round.
        let juxtaposed = sep.is_none() && matches!(body.first(), Some(Elem::Var { .. }));
        if let Some(s) = sep {
            body_follow.push(s.text.clone());
        } else if !juxtaposed {
            body_follow.extend(first_lits(body, &[]));
        }
        if !juxtaposed {
            for s in after {
                if !body_follow.contains(s) {
                    body_follow.push(s.clone());
                }
            }
        }
        if match_elems_sep(body, call, &mut j, &mut round, false, &body_follow).is_err() {
            break;
        }
        if j == probe && rounds.is_empty() {
            // A body that consumes nothing would repeat forever.
            return err(span, "this repetition's body matches nothing; it would repeat for ever");
        }
        // If what follows the repetition can match from here, prefer stopping:
        // the repetition is greedy but must not eat the rest of the matcher.
        if !rest.is_empty() {
            let mut after = j;
            let mut probe_b = round.clone();
            if match_elems_in(rest, call, &mut after, &mut probe_b, whole).is_ok() && after == call.len() {
                rounds.push(round);
                *i = j;
                break;
            }
        }
        rounds.push(round);
        *i = j;
        if j >= call.len() {
            break;
        }
    }

    if rep == Rep::Plus && rounds.is_empty() {
        return err(span, "this repetition needs at least one round (`+`)");
    }
    for (name, _) in names {
        let per_round: Vec<Binding> = rounds
            .iter()
            .map(|r| r.get(&name).cloned().unwrap_or(Binding::One(Vec::new())))
            .collect();
        b.insert(name, Binding::Many(per_round));
    }
    Ok(())
}

/// One juxtaposed argument: a token, or a whole bracketed group.
fn atom_end(call: &[Token], at: usize) -> usize {
    match call.get(at).map(|t| &t.kind) {
        Some(Tk::Open(_)) => matching_close(call, at).map(|c| c + 1).unwrap_or(call.len()),
        // `-5` is one literal, as `literal` promises: a `-` written tight
        // against a number is part of it.
        Some(Tk::Punct)
            if call[at].text == "-"
                && call.get(at + 1).map_or(false, |n| {
                    matches!(n.kind, Tk::Int | Tk::Float) && n.span.lo == call[at].span.hi
                }) =>
        {
            at + 2
        }
        // No expression begins with a separator or a closer, so none of them
        // is an atom: a one-atom fragment stops at it instead of taking it.
        // Until 2026-09-21 any token counted, so `$( $x:expr )*` took the
        // commas of `[1, 4, 4]` as entries -- the weakness recorded for the
        // user's macro testing, narrowed here to what cannot be ambiguous.
        Some(Tk::Comma | Tk::Semi | Tk::FatArrow | Tk::Close(_)) => at,
        Some(_) => at + 1,
        None => at,
    }
}

/// The index just past the `)` that closes the group opening at `at`.
fn matching_close(toks: &[Token], at: usize) -> Option<usize> {
    let mut d = 0i32;
    for (k, t) in toks.iter().enumerate().skip(at) {
        match t.kind {
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

/// How far a fragment of this kind reaches from `at`.
///
/// Harsh's rule, not Rust's follow-set table: a fragment ends at the end of
/// its logical line, at a dedent, at the close of its enclosing group, or at
/// the next literal token the matcher names. Since a call's tokens arrive
/// already grouped, "the close of the group" is the common case here.
fn take_fragment_until(
    kind: Kind,
    call: &[Token],
    at: usize,
    whole: bool,
    stops: &[String],
) -> Result<usize, Error> {
    let Some(t) = call.get(at) else {
        return err(Span::new(0, 0), "the call ended where a capture was expected");
    };
    let one = |want: &str, ok: bool| -> Result<usize, Error> {
        if ok {
            Ok(at + 1)
        } else {
            err(t.span, format!("expected {want}, found `{}`", t.text))
        }
    };
    match kind {
        Kind::Ident => one("an identifier", t.kind == Tk::Ident),
        Kind::Lifetime => one("a lifetime", t.kind == Tk::Lifetime),
        Kind::Literal => {
            let neg = t.text == "-";
            let k = if neg { at + 1 } else { at };
            let is_lit = call
                .get(k)
                .map_or(false, |x| matches!(x.kind, Tk::Int | Tk::Float | Tk::Str | Tk::Char));
            if is_lit {
                Ok(k + 1)
            } else {
                err(t.span, format!("expected a literal, found `{}`", t.text))
            }
        }
        // An atom: one token, or one bracketed group.
        Kind::Tt => match t.kind {
            Tk::Open(_) => match matching_close(call, at) {
                Some(c) => Ok(c + 1),
                None => err(t.span, "a group is never closed in this call"),
            },
            _ => Ok(at + 1),
        },
        // A path is dotted, and may carry `.<T>` generics.
        Kind::Path => {
            let mut k = at;
            if call.get(k).map_or(false, |x| x.kind == Tk::Ident) {
                k += 1;
                while call.get(k).map_or(false, |x| x.kind == Tk::Dot)
                    && call.get(k + 1).map_or(false, |x| x.kind == Tk::Ident)
                {
                    k += 2;
                }
                Ok(k)
            } else {
                err(t.span, format!("expected a path, found `{}`", t.text))
            }
        }
        // Everything else spans the argument it sits in: all of it when the
        // slice is one argument's contents, one atom when the arguments are
        // juxtaposed at a call's top level.
        _ if !whole => {
            let k = atom_end(call, at);
            if k == at {
                return err(t.span, format!("expected {}, found `{}`", kind.name(), t.text));
            }
            Ok(k)
        }
        _ => {
            let mut k = at;
            let mut d = 0i32;
            // An expression, a type or a pattern never contains a `,` or a
            // `;` outside its own brackets -- in Rust as in Harsh -- so a
            // spanning one stops there. Until 2026-09-21 it ran on: a
            // comma-separated arm took `v~ [1; 2; 3]` as the single
            // expression `1; 2; 3`. The same principle as the atom rule: a
            // separator is never part of what it separates.
            let stops_at_separators = matches!(kind, Kind::Expr | Kind::Ty | Kind::Pat | Kind::Path);
            while k < call.len() {
                if d == 0 && stops.iter().any(|s| call[k].text == *s) {
                    break;
                }
                if d == 0 && stops_at_separators && matches!(call[k].kind, Tk::Comma | Tk::Semi) {
                    break;
                }
                match call[k].kind {
                    Tk::Open(_) => d += 1,
                    Tk::Close(_) => {
                        if d == 0 {
                            break;
                        }
                        d -= 1;
                    }
                    _ => {}
                }
                k += 1;
            }
            if k == at {
                return err(t.span, format!("expected {}, found `{}`", kind.name(), t.text));
            }
            Ok(k)
        }
    }
}

#[cfg(test)]
mod match_tests {
    use super::*;

    fn def(src: &str) -> Def {
        // A definition written as its arms: `(matcher) => do:` bodies are not
        // needed for matching, so the test builds arms directly.
        let mut arms = Vec::new();
        for line in src.lines().filter(|l| !l.trim().is_empty()) {
            let toks: Vec<Token> = crate::lex::lex(line)
                .expect("lex")
                .into_iter()
                .filter(|t| !t.is_comment())
                .collect();
            // A matcher's outer parentheses are the arm's own, stripped
            // before parsing, as a definition's are.
            let m = parse_arm_matcher(&toks).expect("matcher");
            arms.push(Arm {
                matcher: m,
                body: Vec::new(),
                span: Span::new(0, 0),
            });
        }
        Def { name: "m".into(), arms, span: Span::new(0, 0) }
    }

    fn call(src: &str) -> Vec<Token> {
        crate::lex::lex(src).expect("lex").into_iter().filter(|t| !t.is_comment()).collect()
    }

    fn text(b: &Binding) -> String {
        b.tokens().iter().map(|t| t.text.as_str()).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn a_literal_token_must_appear() {
        let d = def("(add ($a:expr))");
        let (_, b) = match_call(&d, &call("add (1 + 2)")).expect("match");
        // `($a:expr)` is the metavariable; `(1 + 2)` is an expression, parens
        // and all (the user, 2026-09-25)
        assert_eq!(text(&b["a"]), "( 1 + 2 )");
        let e = match_call(&d, &call("sub 1")).err().expect("refused");
        assert!(e.msg.contains("expected `add`"), "{}", e.msg);
    }

    #[test]
    fn groups_bound_a_capture() {
        let d = def("(($a:expr) ($b:expr))");
        let (_, b) = match_call(&d, &call("(1 + 2) x")).expect("match");
        assert_eq!(text(&b["a"]), "( 1 + 2 )");
        assert_eq!(text(&b["b"]), "x");
    }

    #[test]
    fn a_repetition_binds_one_entry_per_round() {
        let d = def("($( ($x:expr) )*)");
        let (_, b) = match_call(&d, &call("1 2 3")).expect("match");
        let Binding::Many(rounds) = &b["x"] else { panic!("{:?}", b["x"]) };
        assert_eq!(rounds.len(), 3);
        assert_eq!(text(&rounds[0]), "1");
        assert_eq!(text(&rounds[2]), "3");
    }

    #[test]
    fn a_repetition_with_a_separator() {
        let d = def("($( ($x:expr) ),*)");
        let (_, b) = match_call(&d, &call("1 , 2")).expect("match");
        let Binding::Many(rounds) = &b["x"] else { panic!() };
        assert_eq!(rounds.len(), 2);
    }

    #[test]
    fn plus_needs_a_round_and_star_does_not() {
        assert!(match_call(&def("($( ($x:expr) )*)"), &call("")).is_ok());
        let e = match_call(&def("($( ($x:expr) )+)"), &call("")).err().expect("refused");
        assert!(e.msg.contains("at least one round"), "{}", e.msg);
    }

    #[test]
    fn a_repetition_stops_where_the_rest_of_the_matcher_begins() {
        let d = def("($( ($x:expr) )* stop ($last:ident))");
        let (_, b) = match_call(&d, &call("1 2 stop end")).expect("match");
        let Binding::Many(rounds) = &b["x"] else { panic!() };
        assert_eq!(rounds.len(), 2, "the literal `stop` ends the repetition");
        assert_eq!(text(&b["last"]), "end");
    }

    #[test]
    fn arms_are_tried_in_order() {
        let d = def("(($a:ident))\n(($a:expr))");
        let (arm, _) = match_call(&d, &call("x")).expect("match");
        assert!(std::ptr::eq(arm, &d.arms[0]), "the first arm that fits wins");
        let (arm, _) = match_call(&d, &call("1 + 2")).expect("match");
        assert!(std::ptr::eq(arm, &d.arms[1]));
        // `(x)` is no `:ident` -- one identifier token, as in Rust -- but it is
        // an expression (the user, 2026-09-25)
        let (arm, _) = match_call(&d, &call("(x)")).expect("match");
        assert!(std::ptr::eq(arm, &d.arms[1]));
    }

    #[test]
    fn no_arm_matching_says_why_each_failed() {
        let d = def("(add ($a:expr))\n(sub ($a:expr))");
        let e = match_call(&d, &call("mul 1")).err().expect("refused");
        assert!(e.msg.contains("no arm of `m~` matches"), "{}", e.msg);
        assert!(e.msg.contains("arm 1:") && e.msg.contains("arm 2:"), "{}", e.msg);
    }

    #[test]
    fn a_capture_keeps_the_callers_tokens_and_context() {
        let d = def("(($a:expr))");
        let (_, b) = match_call(&d, &call("a / 10")).expect("match");
        // the caller's tokens, with the caller's context (root, here)
        assert!(b["a"].tokens().iter().all(|t| t.ctx == 0));
        assert_eq!(text(&b["a"]), "a / 10");
    }

    #[test]
    fn a_kind_refuses_what_it_is_not() {
        let d = def("(($a:ident))");
        let e = match_call(&d, &call("1")).err().expect("refused");
        assert!(e.msg.contains("expected an identifier"), "{}", e.msg);
    }

    #[test]
    fn tt_takes_a_token_or_a_whole_group() {
        let d = def("(($a:tt) ($b:tt))");
        let (_, b) = match_call(&d, &call("x (1 + 2)")).expect("match");
        assert_eq!(text(&b["a"]), "x");
        assert_eq!(text(&b["b"]), "( 1 + 2 )");
    }

    #[test]
    fn a_dotted_path_is_one_capture() {
        let d = def("(($p:path))");
        let (_, b) = match_call(&d, &call("std.collections.HashMap")).expect("match");
        assert_eq!(text(&b["p"]), "std . collections . HashMap");
    }
}

// ---------------------------------------------------------------------------
// Stage 3: substitution — the transcriber's layout is the expansion's shape
// ---------------------------------------------------------------------------

/// A line of the expansion: its indentation, relative to the transcriber's own
/// first line, and its tokens.
#[derive(Debug, Clone)]
pub struct OutLine {
    pub indent: usize,
    pub toks: Vec<Token>,
}

/// Substitute an arm's bindings into its transcriber.
///
/// The transcriber is Harsh and the result is Harsh: nothing here knows about
/// Rust, and no punctuation is inserted. What the author wrote is what comes
/// out, with `$name` replaced by what it captured and `$( … )` repeated.
///
/// **Layout is the shape.** A repetition whose body occupies its own line
/// yields one line per round, at that line's indentation; a repetition written
/// inside a line yields its rounds inside that line, with its separator
/// between them, exactly as written.
///
/// `ctx` is the fresh syntax context for this expansion: it is stamped on the
/// tokens the *transcriber* contributed. Captured tokens keep the context they
/// arrived with, which is the caller's — that is what makes the macro's `a`
/// and the caller's `a` two different names.
pub fn substitute(body: &[Token], b: &Bindings, ctx: u32) -> Result<Vec<OutLine>, Error> {
    let lines = split_lines(body);
    let base = lines.first().map(|l| l.indent).unwrap_or(0);
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        // A repetition may span lines: `$(` alone on a line, the body
        // beneath, and `)*` closing it. The body's lines repeat together.
        if let Some((body_lines, sep, rep_end)) = spanning_repetition(&lines, i)? {
            let inner: Vec<Token> = body_lines.iter().flat_map(|l| l.toks.clone()).collect();
            let rounds = rounds_for(&inner, b, &[])?;
            // The body's lines sit one step in from the `$(`; in the
            // expansion they stand where the `$(` stood.
            let open_indent = lines[i].indent;
            let body_base = body_lines.iter().map(|l| l.indent).min().unwrap_or(open_indent);
            for r in 0..rounds {
                for l in &body_lines {
                    let mut toks = Vec::new();
                    emit_tokens(&l.toks, b, ctx, &[r], &mut toks)?;
                    if toks.is_empty() {
                        continue;
                    }
                    let indent = open_indent + (l.indent - body_base);
                    out.push(OutLine { indent: indent.saturating_sub(base), toks });
                }
                if let (Some(sp), true) = (&sep, r + 1 < rounds) {
                    if let Some(last) = out.last_mut() {
                        last.toks.push(stamp(sp.clone(), ctx));
                    }
                }
            }
            i = rep_end;
            continue;
        }
        expand_line(&lines[i], b, ctx, base, &mut out)?;
        i += 1;
    }
    Ok(out)
}

/// A repetition written across lines: `$(` opening a line, its body beneath,
/// and a line beginning `)` with the repeat operator. Returns the body's
/// lines, the separator if one was written, and the line after the closer.
#[allow(clippy::type_complexity)]
fn spanning_repetition(
    lines: &[OutLine],
    at: usize,
) -> Result<Option<(Vec<OutLine>, Option<Token>, usize)>, Error> {
    let sig: Vec<&Token> = lines[at].toks.iter().filter(|t| !t.is_comment()).collect();
    if sig.len() != 2 || sig[0].text != "$" || sig[1].kind != Tk::Open('(') {
        return Ok(None);
    }
    for k in at + 1..lines.len() {
        let close: Vec<&Token> = lines[k].toks.iter().filter(|t| !t.is_comment()).collect();
        if close.first().map_or(false, |t| t.kind == Tk::Close(')')) {
            let tail = &close[1..];
            let (sep, ok) = match tail.len() {
                1 => (None, matches!(tail[0].text.as_str(), "*" | "+" | "?")),
                2 => (Some((*tail[0]).clone()), matches!(tail[1].text.as_str(), "*" | "+" | "?")),
                _ => (None, false),
            };
            if !ok {
                // The closer is there; what it lacks is its marker. Saying
                // "never closed" here would send the author to the wrong line.
                return err(
                    close[0].span,
                    "a repetition `$( … )` takes `*`, `+` or `?` after its `)`; lines that must \
                     appear exactly once are written without the `$( … )`",
                );
            }
            if sep.is_some() && tail[1].text == "?" {
                return err(tail[0].span, "`?` repeats at most once, so it takes no separator");
            }
            return Ok(Some((lines[at + 1..k].to_vec(), sep, k + 1)));
        }
    }
    Ok(None)
}

/// The transcriber's tokens, cut into lines with their indentation.
fn split_lines(body: &[Token]) -> Vec<OutLine> {
    let mut out: Vec<OutLine> = Vec::new();
    for t in body {
        match t.line_start {
            Some(col) if !out.is_empty() || !t.is_comment() => {
                out.push(OutLine { indent: col, toks: vec![t.clone()] });
            }
            _ => {
                if let Some(last) = out.last_mut() {
                    last.toks.push(t.clone());
                } else {
                    out.push(OutLine { indent: 0, toks: vec![t.clone()] });
                }
            }
        }
    }
    out
}

/// One transcriber line becomes one or more expansion lines.
fn expand_line(
    line: &OutLine,
    b: &Bindings,
    ctx: u32,
    base: usize,
    out: &mut Vec<OutLine>,
) -> Result<(), Error> {
    // A line that is *only* a repetition repeats the line: one round per line,
    // at this line's indentation. A repetition inside a line repeats inside it.
    if let Some((body, sep, rounds)) = whole_line_repetition(line, b, &[])? {
        for r in 0..rounds {
            let mut toks = Vec::new();
            emit_tokens(&body, b, ctx, &[r], &mut toks)?;
            if toks.is_empty() {
                continue;
            }
            if let Some(s) = &sep {
                if r + 1 < rounds {
                    toks.push(stamp(s.clone(), ctx));
                }
            }
            out.push(OutLine { indent: line.indent.saturating_sub(base), toks });
        }
        return Ok(());
    }
    let mut toks = Vec::new();
    emit_tokens(&line.toks, b, ctx, &[], &mut toks)?;
    out.push(OutLine { indent: line.indent.saturating_sub(base), toks });
    Ok(())
}

/// Is this line exactly `$( … )sep rep`? Then it is a repeated *line*.
#[allow(clippy::type_complexity)]
fn whole_line_repetition(
    line: &OutLine,
    b: &Bindings,
    path: &[usize],
) -> Result<Option<(Vec<Token>, Option<Token>, usize)>, Error> {
    let sig: Vec<&Token> = line.toks.iter().filter(|t| !t.is_comment()).collect();
    if sig.len() < 4 || sig[0].text != "$" || sig[1].kind != Tk::Open('(') {
        return Ok(None);
    }
    let owned: Vec<Token> = sig.iter().map(|t| (*t).clone()).collect();
    let close = match matching_close(&owned, 1) {
        Some(c) => c,
        None => return Ok(None),
    };
    // What follows the `)` is an optional separator then `*`, `+` or `?`.
    let tail = &owned[close + 1..];
    let (sep, rep_at) = match tail.len() {
        1 => (None, 0),
        2 => (Some(tail[0].clone()), 1),
        _ => return Ok(None),
    };
    if !matches!(tail.get(rep_at).map(|t| t.text.as_str()), Some("*") | Some("+") | Some("?")) {
        return Ok(None);
    }
    let inner: Vec<Token> = owned[2..close].to_vec();
    let rounds = rounds_for(&inner, b, path)?;
    Ok(Some((inner, sep, rounds)))
}

/// How many rounds a repetition runs: the length of the bindings it uses.
/// Follow a path of round indices into a nested binding.
fn at_path<'a>(b: &'a Binding, path: &[usize]) -> Option<&'a Binding> {
    let mut cur = b;
    for &r in path {
        match cur {
            Binding::Many(rs) => cur = rs.get(r)?,
            Binding::One(_) => return Some(cur),
        }
    }
    Some(cur)
}

fn rounds_for(body: &[Token], b: &Bindings, path: &[usize]) -> Result<usize, Error> {
    let mut found: Option<(String, usize)> = None;
    let mut i = 0;
    while i < body.len() {
        if body[i].text == "$" {
            if let Some(name) = body.get(i + 1) {
                let here = b.get(&name.text).and_then(|x| at_path(x, path));
                if let Some(Binding::Many(rs)) = here {
                    match &found {
                        None => found = Some((name.text.clone(), rs.len())),
                        Some((other, n)) if *n != rs.len() => {
                            return err(
                                name.span,
                                format!(
                                    "`${}` repeats {} time(s) and `${other}` {n} -- a repetition's captures must agree",
                                    name.text,
                                    rs.len()
                                ),
                            )
                        }
                        _ => {}
                    }
                }
            }
        }
        i += 1;
    }
    match found {
        Some((_, n)) => Ok(n),
        None => err(
            body.first().map(|t| t.span).unwrap_or(Span::new(0, 0)),
            "this repetition uses no capture, so there is nothing to repeat over",
        ),
    }
}

/// Write a run of transcriber tokens, substituting captures.
fn emit_tokens(
    toks: &[Token],
    b: &Bindings,
    ctx: u32,
    path: &[usize],
    out: &mut Vec<Token>,
) -> Result<(), Error> {
    let mut i = 0;
    while i < toks.len() {
        let t = &toks[i];
        // `$` introduces a capture or a repetition; anything else -- `f$`,
        // Harsh's own call marker -- is an ordinary token. A `$name` the
        // matcher never bound is an error, not a call marker: the two are
        // told apart by whether a name follows tight against the `$`.
        let metavar = t.text == "$"
            && match toks.get(i + 1) {
                Some(n) if n.kind == Tk::Open('(') => true,
                Some(n) => n.kind == Tk::Ident && n.span.lo == t.span.hi,
                None => false,
            };
        if metavar {
            // `$( … )sep rep` inside a line: the rounds go here, in place.
            if toks.get(i + 1).map_or(false, |x| x.kind == Tk::Open('(')) {
                let close = match matching_close(toks, i + 1) {
                    Some(c) => c,
                    None => return err(t.span, "a repetition `$( … )` is never closed"),
                };
                let inner = &toks[i + 2..close];
                let tail = &toks[close + 1..];
                let is_marker = |x: Option<&Token>| {
                    matches!(x.map(|x| x.text.as_str()), Some("*") | Some("+") | Some("?"))
                };
                // A separator is only a separator if a marker follows it;
                // otherwise these are the author's own tokens and must not
                // be eaten. No marker at all is refused, as in the matcher
                // and as in Rust (ruling 6, 2026-09-19).
                let (sep, skip) = if is_marker(tail.first()) {
                    (None, 1)
                } else if is_marker(tail.get(1)) {
                    if tail[1].text == "?" {
                        return err(tail[0].span, "`?` repeats at most once, so it takes no separator");
                    }
                    (tail.first().cloned(), 2)
                } else {
                    return err(
                        t.span,
                        "a repetition `$( … )` takes `*`, `+` or `?`; tokens that must appear \
                         exactly once are written without the `$( … )`",
                    );
                };
                let n = rounds_for(inner, b, path)?;
                for r in 0..n {
                    let mut deeper = path.to_vec();
                    deeper.push(r);
                    emit_tokens(inner, b, ctx, &deeper, out)?;
                    if let (Some(s), true) = (&sep, r + 1 < n) {
                        out.push(stamp(s.clone(), ctx));
                    }
                }
                i = close + 1 + skip;
                continue;
            }
            // `$name`: what it captured, with the caller's context intact.
            if let Some(name) = toks.get(i + 1) {
                let Some(binding) = b.get(&name.text) else {
                    return err(
                        name.span,
                        format!("`${}` was not captured by this arm's matcher", name.text),
                    );
                };
                let picked = match at_path(binding, path) {
                    Some(Binding::One(v)) => v.clone(),
                    Some(Binding::Many(_)) | None => {
                        return err(
                            name.span,
                            format!(
                                "`${}` was captured inside a repetition, so it must be used inside one",
                                name.text
                            ),
                        )
                    }
                };
                out.extend(picked);
                i += 2;
                continue;
            }
        }
        out.push(stamp(t.clone(), ctx));
        i += 1;
    }
    Ok(())
}

/// A token the transcriber contributed: it carries this expansion's context.
fn stamp(mut t: Token, ctx: u32) -> Token {
    t.ctx = ctx;
    t.line_start = None;
    t
}

/// The expansion as Harsh source, at a column: what `hrs expand` prints and
/// what is spliced back into the caller's file.
pub fn render(lines: &[OutLine], at: usize) -> String {
    let mut s = String::new();
    for (n, l) in lines.iter().enumerate() {
        if n > 0 {
            s.push('\n');
        }
        s.push_str(&" ".repeat(at + l.indent));
        let mut prev: Option<&Token> = None;
        for t in &l.toks {
            let tight = match (prev, &t.kind) {
                (Some(p), _) if p.span.hi == t.span.lo && p.ctx == t.ctx => true,
                (_, Tk::Comma) | (_, Tk::Semi) | (_, Tk::Colon) => true,
                (Some(p), _) => matches!(p.kind, Tk::Open(_)) || matches!(t.kind, Tk::Close(_)),
                (None, _) => true,
            };
            if !tight {
                s.push(' ');
            }
            s.push_str(&t.text);
            prev = Some(t);
        }
    }
    s
}

#[cfg(test)]
mod subst_tests {
    use super::*;

    fn toks(src: &str) -> Vec<Token> {
        crate::lex::lex(src).expect("lex").into_iter().filter(|t| !t.is_comment()).collect()
    }

    fn matcher_of(src: &str) -> Vec<Elem> {
        let m = parse_arm_matcher(&toks(src)).expect("matcher");
        m
    }

    /// Match a call, substitute into a transcriber, render the Harsh.
    fn expand(matcher: &str, body: &str, call: &str) -> String {
        let arm = Arm { matcher: matcher_of(matcher), body: toks(body), span: Span::new(0, 0) };
        let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
        let (arm, b) = match_call(&def, &toks(call)).expect("match");
        let lines = substitute(&arm.body, &b, 1).expect("substitute");
        render(&lines, 0)
    }

    #[test]
    fn a_capture_is_substituted_as_written() {
        let out = expand("(($a:expr))", "let x = $a", "(1 + 2)");
        // the expression as the caller wrote it, its parentheses with it
        assert_eq!(out, "let x = (1 + 2)");
    }

    #[test]
    fn a_repetition_on_its_own_line_yields_one_line_per_round() {
        let out = expand(
            "(($v:ident) $( ($x:expr) )*)",
            "$( $v <- push $x )*",
            "v 1 2 3",
        );
        assert_eq!(out, "v <- push 1\nv <- push 2\nv <- push 3");
    }

    #[test]
    fn a_repetition_inside_a_line_yields_entries_in_place() {
        let out = expand("($( ($x:expr) )*)", "let xs = [$( $x ),*]", "1 2 3");
        assert_eq!(out, "let xs = [1, 2, 3]");
    }

    #[test]
    fn the_transcribers_indentation_is_kept() {
        let out = expand("(($a:expr))", "do:\n    let x = $a\n    x + x", "7");
        assert_eq!(out, "do:\n    let x = 7\n    x + x");
    }

    #[test]
    fn the_expansion_lands_at_the_callers_column() {
        let arm = Arm { matcher: matcher_of("(($a:expr))"), body: toks("do:\n    let x = $a"), span: Span::new(0, 0) };
        let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
        let (arm, b) = match_call(&def, &toks("7")).expect("match");
        let lines = substitute(&arm.body, &b, 1).expect("substitute");
        assert_eq!(render(&lines, 8), "        do:\n            let x = 7");
    }

    #[test]
    fn a_transcribers_tokens_carry_this_expansions_context() {
        let arm = Arm { matcher: matcher_of("(($a:expr))"), body: toks("let tmp = $a"), span: Span::new(0, 0) };
        let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
        let (arm, b) = match_call(&def, &toks("(a / 10)")).expect("match");
        let lines = substitute(&arm.body, &b, 7).expect("substitute");
        let by_text = |t: &str| lines[0].toks.iter().find(|x| x.text == t).unwrap().ctx;
        assert_eq!(by_text("tmp"), 7, "the transcriber's own token is marked");
        assert_eq!(by_text("a"), 0, "the caller's token keeps its context");
    }

    #[test]
    fn using_a_capture_the_matcher_never_bound_is_refused() {
        let arm = Arm { matcher: matcher_of("(($a:expr))"), body: toks("$b + 1"), span: Span::new(0, 0) };
        let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
        let (arm, b) = match_call(&def, &toks("1")).expect("match");
        let e = substitute(&arm.body, &b, 1).err().expect("refused");
        assert!(e.msg.contains("was not captured"), "{}", e.msg);
    }

    #[test]
    fn a_repeated_capture_used_outside_a_repetition_is_refused() {
        let arm = Arm { matcher: matcher_of("($( ($x:expr) )*)"), body: toks("let a = $x"), span: Span::new(0, 0) };
        let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
        let (arm, b) = match_call(&def, &toks("1 2")).expect("match");
        let e = substitute(&arm.body, &b, 1).err().expect("refused");
        assert!(e.msg.contains("used inside one"), "{}", e.msg);
    }

    #[test]
    fn a_transcribers_repetition_without_a_marker_is_refused() {
        // It used to run every round silently; and with tokens after it,
        // `$( $a ) x y` took `x` for a separator and dropped `y`.
        for body in ["0 $( + $x )", "[$( $x ) a b]", "[$( $x ) a]"] {
            let arm = Arm { matcher: matcher_of("($( ($x:expr) )*)"), body: toks(body), span: Span::new(0, 0) };
            let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
            let (arm, b) = match_call(&def, &toks("1 2")).expect("match");
            let e = substitute(&arm.body, &b, 1).err().expect("refused");
            assert!(e.msg.contains("takes `*`, `+` or `?`"), "{body}: {}", e.msg);
            assert_eq!(e.span.lo as usize, body.find("$(").unwrap(), "{body}");
        }
        // With its marker the author's tokens come through untouched.
        let out = expand("($( ($x:expr) )*)", "[$( $x ),* , a, b]", "1 2");
        assert_eq!(out, "[1, 2, a, b]");
        // Across lines the closer is there and the marker is not: the error
        // says so, on the `)`, rather than "never closed".
        let body = "$(\n    v <- push $x\n)";
        let arm = Arm { matcher: matcher_of("($( ($x:expr) )*)"), body: crate::lex::lex(body).expect("lex"), span: Span::new(0, 0) };
        let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
        let (arm, b) = match_call(&def, &toks("1 2")).expect("match");
        let e = substitute(&arm.body, &b, 1).err().expect("refused");
        assert!(e.msg.contains("after its `)`"), "{}", e.msg);
        assert_eq!(e.span.lo as usize, body.rfind(')').unwrap());
        // `?` takes no separator here either.
        let arm = Arm { matcher: matcher_of("($( ($x:expr) )?)"), body: toks("[$( $x ),?]"), span: Span::new(0, 0) };
        let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
        let (arm, b) = match_call(&def, &toks("1")).expect("match");
        let e = substitute(&arm.body, &b, 1).err().expect("refused");
        assert!(e.msg.contains("takes no separator"), "{}", e.msg);
    }

    #[test]
    fn two_captures_in_one_repetition_must_agree_in_length() {
        let out = expand(
            "($( ($n:ident) ($t:ty) )*)",
            "Config\\\n    $( $n: $t )*",
            "host String port u16",
        );
        assert_eq!(out, "Config\\\n    host: String\n    port: u16");
    }
}

// ---------------------------------------------------------------------------
// Stage 4: hygiene — encoding a context clash as a spelling
// ---------------------------------------------------------------------------

/// Make the expansion safe to hand on as *source*.
///
/// Contexts already say which identifiers are the same name: two agree only
/// when their text and their `ctx` agree (`Token::ctx`). rustc can stop there,
/// because it is the resolver. Harsh hands its result to its own layout pass
/// and then to rustc as generated Rust, and neither can see a context — so
/// where two identifiers share a spelling, differ in context, and would stand
/// in one scope, one of them is respelled here. The contexts decide *that* it
/// must happen and *which* one moves; nothing is guessed.
///
/// A macro's local keeps its own spelling whenever it clashes with nothing, so
/// an expansion reads as its author wrote it. `taken` is every name visible at
/// the call site (the caller's own bindings and anything else in scope).
pub fn respell(lines: &mut [OutLine], ctx: u32, taken: &[String]) -> Vec<(String, String)> {
    // The names this expansion introduced: identifiers bound by the
    // transcriber itself. A capture's tokens carry the caller's context and
    // are never touched.
    let mut introduced: Vec<String> = Vec::new();
    let mut add = |t: &Token| {
        if t.kind == Tk::Ident && t.ctx == ctx && !crate::rules::is_keyword(&t.text) && !introduced.contains(&t.text) {
            introduced.push(t.text.clone());
        }
    };
    for l in lines.iter() {
        let sig: Vec<&Token> = l.toks.iter().filter(|t| !t.is_comment()).collect();
        let mut i = 0;
        while i < sig.len() {
            let t = sig[i];
            // `let [mut] x`, `const X`, `static X`
            if t.is_kw("let") || t.is_kw("const") || t.is_kw("static") {
                let mut k = i + 1;
                if sig.get(k).map_or(false, |x| x.is_kw("mut")) {
                    k += 1;
                }
                // `let x`, or `let Some x` / `let (a, b)` / `let Point\ x, y`:
                // every identifier of the pattern, up to the `=`.
                let end = sig[k..].iter().position(|x| x.kind == Tk::Eq).map(|p| k + p).unwrap_or(sig.len());
                for x in &sig[k..end] {
                    add(x);
                }
                i = end;
                continue;
            }
            // `for x in …` / `for (a, b) in …`
            if t.is_kw("for") {
                let end = sig[i + 1..].iter().position(|x| x.is_kw("in")).map(|p| i + 1 + p).unwrap_or(sig.len());
                for x in &sig[i + 1..end] {
                    add(x);
                }
                i = end;
                continue;
            }
            // `|x, y|` closure parameters: identifiers between the bars,
            // types after a `:` excluded.
            if t.text == "|" {
                let mut k = i + 1;
                let mut in_type = false;
                while k < sig.len() && sig[k].text != "|" {
                    match sig[k].kind {
                        Tk::Colon => in_type = true,
                        Tk::Comma => in_type = false,
                        _ if !in_type => add(sig[k]),
                        _ => {}
                    }
                    k += 1;
                }
                i = k + 1;
                continue;
            }
            // a `match` arm: `pattern =>` -- identifiers of the pattern that
            // are not paths or constructors (a lower-case first letter).
            if t.kind == Tk::FatArrow {
                let start = sig[..i].iter().rposition(|x| x.line_start.is_some()).unwrap_or(0);
                for x in &sig[start..i] {
                    if x.text.chars().next().map_or(false, |c| c.is_lowercase() || c == '_') {
                        add(x);
                    }
                }
            }
            // `fn name (a: T) (b: U)`: the names of the parameter groups
            if t.is_kw("fn") {
                let mut k = i + 2;
                while k < sig.len() && sig[k].kind != Tk::Colon && !matches!(sig[k].kind, Tk::Punct if sig[k].text == "$") {
                    if sig[k].kind == Tk::Open('(') {
                        if let Some(x) = sig.get(k + 1) {
                            if sig.get(k + 2).map_or(false, |c| c.kind == Tk::Colon) {
                                add(x);
                            }
                        }
                    }
                    k += 1;
                }
            }
            i += 1;
        }
    }

    let mut renames: Vec<(String, String)> = Vec::new();
    for name in introduced {
        if !taken.contains(&name) {
            continue; // nothing to clash with: the author's spelling stands
        }
        let mut n = 1;
        let mut candidate = format!("{name}__{n}");
        while taken.contains(&candidate) || renames.iter().any(|(_, to)| *to == candidate) {
            n += 1;
            candidate = format!("{name}__{n}");
        }
        renames.push((name, candidate));
    }

    for (from, to) in &renames {
        for l in lines.iter_mut() {
            for t in l.toks.iter_mut() {
                if t.kind == Tk::Ident && t.ctx == ctx && t.text == *from {
                    t.text = to.clone();
                }
            }
        }
    }
    renames
}

#[cfg(test)]
mod hygiene_tests {
    use super::*;

    fn toks(src: &str) -> Vec<Token> {
        crate::lex::lex(src).expect("lex").into_iter().filter(|t| !t.is_comment()).collect()
    }

    fn matcher_of(src: &str) -> Vec<Elem> {
        let m = parse_arm_matcher(&toks(src)).expect("matcher");
        m
    }

    /// Expand, then make it safe as source, given what the caller has in scope.
    fn expand_hygienic(matcher: &str, body: &str, call: &str, taken: &[&str]) -> String {
        let arm = Arm { matcher: matcher_of(matcher), body: toks(body), span: Span::new(0, 0) };
        let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
        let (arm, b) = match_call(&def, &toks(call)).expect("match");
        let mut lines = substitute(&arm.body, &b, 9).expect("substitute");
        let names: Vec<String> = taken.iter().map(|s| (*s).to_string()).collect();
        respell(&mut lines, 9, &names);
        render(&lines, 0)
    }

    #[test]
    fn a_macros_local_keeps_its_name_when_nothing_clashes() {
        let out = expand_hygienic("(($x:expr))", "let tmp = $x\ntmp + tmp", "1", &["other"]);
        assert_eq!(out, "let tmp = 1\ntmp + tmp");
    }

    #[test]
    fn a_clash_moves_the_macros_local_not_the_callers() {
        // The caller has `tmp`, and passes an expression that reads it. The
        // macro's own `tmp` is the one that moves; the caller's stays.
        let out = expand_hygienic("(($x:expr))", "let tmp = $x\ntmp + tmp", "(tmp + 1)", &["tmp"]);
        assert_eq!(out, "let tmp__1 = (tmp + 1)\ntmp__1 + tmp__1");
    }

    #[test]
    fn the_new_spelling_avoids_everything_in_scope() {
        let out = expand_hygienic("(($x:expr))", "let tmp = $x\ntmp", "1", &["tmp", "tmp__1"]);
        assert_eq!(out, "let tmp__2 = 1\ntmp__2");
    }

    #[test]
    fn a_name_the_caller_passed_in_is_never_respelled() {
        // `$n:ident` is the caller's: `declare~ count 0` must bind the
        // caller's `count`, clash or no clash.
        let out = expand_hygienic(
            "(($n:ident) ($v:expr))",
            "let $n = $v",
            "count 0",
            &["count"],
        );
        assert_eq!(out, "let count = 0");
    }

    #[test]
    fn several_locals_each_get_their_own_spelling() {
        let out = expand_hygienic(
            "(($x:expr))",
            "let a = $x\nlet b = a\nb + a",
            "1",
            &["a", "b"],
        );
        assert_eq!(out, "let a__1 = 1\nlet b__1 = a__1\nb__1 + a__1");
    }

    #[test]
    fn a_mutable_local_is_found_too() {
        let out = expand_hygienic("(($x:expr))", "let mut v = $x\nv", "1", &["v"]);
        assert_eq!(out, "let mut v__1 = 1\nv__1");
    }

    #[test]
    fn the_renames_are_reported_for_hrs_expand() {
        let arm = Arm { matcher: matcher_of("(($x:expr))"), body: toks("let tmp = $x"), span: Span::new(0, 0) };
        let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
        let (arm, b) = match_call(&def, &toks("1")).expect("match");
        let mut lines = substitute(&arm.body, &b, 3).expect("substitute");
        let done = respell(&mut lines, 3, &["tmp".to_string()]);
        assert_eq!(done, vec![("tmp".to_string(), "tmp__1".to_string())]);
    }
}

// ---------------------------------------------------------------------------
// Stage 5: expansion in the pipeline
// ---------------------------------------------------------------------------

/// Harsh's prelude: macros every file may call without a `use`
/// (the user's ruling, 2026-09-20).
///
/// Each is reachable two ways. `hrs_std.g~` always names the prelude's own
/// macro; bare `g~` does too, **unless the file defines its own `g`**, which
/// then shadows it -- Rust's rule for its prelude, copied rather than
/// invented. The prelude's macros recurse and delegate through the qualified
/// path, so a user's `g` can never be picked up by accident from inside one.
///
/// `m~ [1 2; 3 4]` and `v~ [1, 2, 3]` are Julia's matrix and vector
/// literals -- spaces between entries, `;` between rows; commas make a
/// vector, as in Julia. Unlike the others they expand to a *type*,
/// `hrs_std.Matrix` / `hrs_std.Vector`, so a project that calls them needs
/// the `hrs_std` crate; `hrs` says so, with the line to add, when it is
/// missing (`driver::check_hrs_std`).
///
/// `g~` is a comprehension: `g~ e for p in it if c for q in it2 if d …`.
/// Each `for` becomes one closure; the `if`s that follow a `for` belong to
/// it and fold into that closure's one condition, `true && (c) && (d)`,
/// under `bool::then`; every level but the innermost is flattened. It is
/// lazy -- a plain iterator -- and `for` means what `for` means in the
/// language: `IntoIterator`, so it consumes, and a borrow is `xs <- iter$`.
pub const PRELUDE: &str = r#"
macro_rules~ g
    (($e:expr) for ($p:pat) in ($it:expr) $(if ($c:expr))* for $(($rest:tt))*) => do:
        ($it) <- into_iter$ <- flat_map (move |$p| ((true $( && ($c) )*) <- then (|| (hrs_std.g~ $e for $($rest)*)))) <- flatten$

    (($e:expr) for ($p:pat) in ($it:expr) $(if ($c:expr))*) => do:
        ($it) <- into_iter$ <- flat_map (move |$p| ((true $( && ($c) )*) <- then (|| $e)))

macro_rules~ list
    ($(($t:tt))*) => do: (hrs_std.g~ $($t)*) <- collect<Vec<_>>$

macro_rules~ set
    ($(($t:tt))*) => do: (hrs_std.g~ $($t)*) <- collect<std.collections.HashSet<_>>$

macro_rules~ dict
    (($k:expr) => ($v:expr) for $(($rest:tt))*) => do: (hrs_std.g~ ($k, $v) for $($rest)*) <- collect<std.collections.HashMap<_, _>>$

macro_rules~ m
    ([ [ $(($a:tt))* ] $( , [ $(($b:tt))* ] )+ ]) => do: compile_error! "`m~ [[…], […]]`: in Julia, commas never concatenate, so this is a vector of two matrices, not a matrix. Stack the rows with `;` or a line break: `m~ [[1 2]; [3 4]]`."
    ([ $( $( ($x:literal) )+ );+ ]) => do: hrs_std.Matrix.from_rows (vec! $( (vec! $( ($x) )*) )*)
    ([ $( ($x:literal) ),+ ]) => do: hrs_std.Matrix.from_rows (vec! $( (vec! $x) )*)
    ([ $( $( ($x:expr) )+ );+ ]) => do: hrs_std.vcat (vec! $( (hrs_std.hcat (vec! $( (hrs_std.mblock~ $x) )*)) )*)
    ([ $( ($x:expr) ),+ ]) => do: hrs_std.vcat (vec! $( (hrs_std.entry ($x)) )*)

macro_rules~ mblock
    ([ $(($t:tt))* ]) => do: hrs_std.m~ [$($t)*]
    (( $(($t:tt))* )) => do: hrs_std.block ($($t)*)
    (($e:expr)) => do: hrs_std.block ($e)

macro_rules~ v
    ([ ($x:literal) $( ($y:literal) )+ ]) => do: compile_error! "`v~ [1 2 3]`: spaces make a row, which in Julia is a 1×n matrix, not a Vector. A Vector is a column: `v~ [1, 2, 3]`, or one entry per line."
    ([ $( ($x:expr) ),+ ]) => do: hrs_std.Vector.from_vec (vec! $( ($x) )*)
    ([ $( ($x:expr) );+ ]) => do: hrs_std.Vector.from_vec (vec! $( ($x) )*)
"#;

/// The prelude path, `hrs_std`, as written before a prelude macro's name.
pub const PRELUDE_PATH: &str = "hrs_std";

/// Rust's ceiling, replicated: expansion runs in passes, and a crate that is
/// still expanding after this many is aborted rather than left to recurse for
/// ever. Raised per file with `#![recursion_limit = "…"]`, as in Rust.
pub const RECURSION_LIMIT: usize = 128;

/// Read every `macro_rules~` definition in a token stream.
///
/// A definition is `macro_rules~ name` followed by its arms, which are the
/// lines indented beneath it; an arm is `( matcher ) => do:` with its
/// transcriber beneath. The definition's own tokens are removed from the
/// stream by [`expand_all`], since a Harsh macro leaves no trace in the
/// output -- it is unfolded here, not handed to Rust.
pub fn collect_defs(toks: &[Token]) -> Result<(Vec<Def>, Vec<std::ops::Range<usize>>), Error> {
    let mut defs = Vec::new();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let is_def = toks[i].is_kw("macro_rules")
            && toks.get(i + 1).map_or(false, |t| t.text == "~")
            && toks.get(i + 2).map_or(false, |t| t.kind == Tk::Ident);
        if !is_def {
            i += 1;
            continue;
        }
        let name = toks[i + 2].text.clone();
        // The header takes no mark: `macro_rules~ name` with the arms beneath,
        // as `struct Point` and `impl Foo` are written (2026-09-17).
        if toks.get(i + 3).map_or(false, |t| t.kind == Tk::Colon) {
            return err(
                toks[i + 3].span,
                format!("a macro's arms follow its header with no mark: `macro_rules~ {name}` with the arms beneath it"),
            );
        }
        let header_col = line_col(toks, i);
        // The definition runs to the first later line at or left of its own
        // column: the ordinary dedent rule.
        let mut end = i + 3;
        while end < toks.len() {
            match toks[end].line_start {
                Some(c) if c <= header_col => break,
                _ => end += 1,
            }
        }
        let arms = read_arms(&toks[i + 3..end])?;
        defs.push(Def { name, arms, span: toks[i].span });
        spans.push(i..end);
        i = end;
    }
    Ok((defs, spans))
}

fn line_col(toks: &[Token], at: usize) -> usize {
    for t in toks[..=at].iter().rev() {
        if let Some(c) = t.line_start {
            return c;
        }
    }
    0
}

/// The arms of a definition: `( matcher ) => do:` and the body beneath it.
fn read_arms(toks: &[Token]) -> Result<Vec<Arm>, Error> {
    let mut arms = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        if toks[i].is_comment() {
            i += 1;
            continue;
        }
        // A matcher's own delimiter is parens and nothing else; brackets and
        // braces are a DSL's tokens and go inside them (ruling 6, 2026-09-19).
        if toks[i].kind != Tk::Open('(') {
            return err(toks[i].span, "an arm begins with its matcher in parens, `( … ) => do:`; a DSL's own brackets go inside them, `([ … ]) => do:`");
        }
        let arm_col = line_col(toks, i);
        let close = match matching_close(toks, i) {
            Some(c) => c,
            None => return err(toks[i].span, "this matcher is never closed"),
        };
        // The matcher's own delimiter is its outer parens, stripped before
        // parsing. Brackets and braces are never the matcher's: they are a
        // DSL's own group, written inside the parens -- `([ ($e:expr) ; ($n:expr) ])`
        // matches the stream `[0u8; 2]` (2026-09-19).
        let m = parse_arm_matcher(&toks[i..=close])?;
        check_unique(&m)?;
        // `=> do:` then the transcriber, indented beneath.
        let mut k = close + 1;
        while toks.get(k).map_or(false, |t| t.is_comment()) {
            k += 1;
        }
        if toks.get(k).map_or(true, |t| t.kind != Tk::FatArrow) {
            let sp = toks.get(k).map(|t| t.span).unwrap_or(toks[close].span);
            return err(sp, "an arm is `( matcher ) => do:` with its body beneath");
        }
        k += 1;
        // The transcriber is a block: `=> do:`, inline or with its body
        // beneath. A bare expression after the arrow is refused, as it is in
        // Rust, where a transcriber is always delimited.
        if !toks.get(k).map_or(false, |t| t.is_kw("do")) {
            let sp = toks.get(k).map(|t| t.span).unwrap_or(toks[close].span);
            return err(sp, "a transcriber is a block: write `=> do:` with the body beneath, or `=> do: expr` on one line");
        }
        k += 1;
        if toks.get(k).map_or(false, |t| t.kind == Tk::Colon) {
            k += 1;
        }
        let mut end = k;
        while end < toks.len() {
            match toks[end].line_start {
                Some(c) if c <= arm_col => break,
                _ => end += 1,
            }
        }
        arms.push(Arm { matcher: m, body: toks[k..end].to_vec(), span: toks[i].span });
        i = end;
    }
    if arms.is_empty() {
        return err(Span::new(0, 0), "a macro needs at least one arm");
    }
    Ok(arms)
}

/// Every `name~ …` call of a known macro, as a range over the stream.
/// Returns the definition, where the call begins (its path, if qualified),
/// where it ends, and where its stream begins.
/// What a `name~` call resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    /// A `macro_rules~` definition: the file's own, or the prelude's.
    Def(usize),
    /// A Harsh proc macro registered for this project (ruling 17).
    Proc(String),
    /// `quote~`, where the file imports `hrs_quote.quote` (`SYN-QUOTE-PLAN`,
    /// step 3; call Q1).
    Quote,
}

/// The next `~` call from `from`: what it resolves to, where its head
/// begins, where its stream ends, and where the stream begins.
///
/// **Who wins a name**, most local first, as in Rust: a file's own
/// `macro_rules~`, then the project's proc macros, then the prelude's -- the
/// project's choice shadows Harsh's default, as a file's own definition
/// does. `hrs_std.NAME~` always reaches the prelude.
fn find_call(
    toks: &[Token],
    defs: &[Def],
    own: usize,
    procs: &dyn ProcMacros,
    quote: bool,
    from: usize,
) -> Option<(Target, usize, usize, usize)> {
    let mut i = from;
    while i + 1 < toks.len() {
        let marked = toks[i].kind == Tk::Ident
            && toks[i + 1].text == "~"
            && toks[i].span.hi == toks[i + 1].span.lo;
        if marked {
            // `hrs_std.g~`: the prelude's own macro, whatever the file defines.
            let qualified = i >= 2
                && toks[i - 1].kind == Tk::Dot
                && toks[i - 2].kind == Tk::Ident
                && toks[i - 2].text == PRELUDE_PATH;
            let key = if qualified { format!("{PRELUDE_PATH}.{}", toks[i].text) } else { toks[i].text.clone() };
            let def = defs.iter().position(|d| d.name == key);
            let target = match def {
                Some(d) if qualified || d < own => Some(Target::Def(d)),
                _ if !qualified && quote && key == "quote" => Some(Target::Quote),
                _ if !qualified && procs.has(&key) => Some(Target::Proc(key)),
                Some(d) => Some(Target::Def(d)),
                None => None,
            };
            if let Some(target) = target {
                // A qualified call's range begins at the path.
                let head = if qualified { i - 2 } else { i };
                let col = line_col(toks, i);
                let mut end = i + 2;
                let mut depth = 0i32;
                while end < toks.len() {
                    let t = &toks[end];
                    if depth == 0 && t.line_start.map_or(false, |c| c <= col) {
                        break;
                    }
                    match t.kind {
                        Tk::Open(_) => depth += 1,
                        Tk::Close(_) => {
                            if depth == 0 {
                                break;
                            }
                            depth -= 1;
                        }
                        _ => {}
                    }
                    end += 1;
                }
                return Some((target, head, end, i + 2));
            }
        }
        i += 1;
    }
    None
}

/// Expand every Harsh macro in a file, in passes, until none is left.
///
/// The definitions are removed and each call is replaced by its expansion, so
/// what leaves this function is ordinary Harsh: the transpiler then does to it
/// exactly what it does to hand-written code, and the emitted Rust holds no
/// macro at all.
/// One expansion that happened in a file: which macro, and where it was
/// called. Tokens the expansion produced carry `ctx`, so a diagnostic inside
/// the expanded code can be pointed back at the call as well as at the
/// definition (the source map through expansion, 2026-09-19).
#[derive(Debug, Clone)]
pub struct Expansion {
    pub ctx: u32,
    pub name: String,
    pub call_lo: u32,
    pub call_hi: u32,
    /// For a proc macro's expansion, where its tokens' spans begin (see
    /// `PROC_BASE`); 0 for a `macro_rules~` expansion.
    pub proc_lo: u32,
    /// Source a derive removed from the file -- its `#[derive~ …]` and the
    /// helper attributes it consumed -- which the emitter must not copy back
    /// through a gap (`erase`).
    pub erased: Vec<Span>,
    /// For a proc macro's expansion, the tokens it copied from its input:
    /// their place in the region, and the programmer's token they came from.
    /// A diagnostic on one lands on that token, as in Rust, where a copied
    /// token keeps its span (`at_call`).
    pub origins: Vec<(Span, Span)>,
}

/// `text` with every span a derive removed replaced by spaces (newlines
/// kept), for the emitter, which copies the source between tokens: an inline
/// `#[show skip] x: i32` would otherwise come back through the gap before
/// `x`. Same length, so every offset holds.
pub fn erase(text: &str, trace: &[Expansion]) -> String {
    let mut out = text.to_string();
    for s in trace.iter().flat_map(|x| x.erased.iter()) {
        let (lo, hi) = (s.lo as usize, s.hi as usize);
        if let Some(piece) = text.get(lo..hi) {
            let spaces: String = piece.chars().map(|c| if c == '\n' { '\n' } else { ' ' }).collect();
            out.replace_range(lo..hi, &spaces);
        }
    }
    out
}

/// The project's Harsh proc macros, as the expander sees them: names it may
/// call, and a way to run one (ruling 17). The driver's runner implements it;
/// tests use a fake; everything else passes `NoProcMacros`.
pub trait ProcMacros {
    /// Whether `name` is a function-like proc macro, called `name~ stream`.
    fn has(&self, name: &str) -> bool;
    /// For a derive macro, called `#[derive~ Name]`, the helper attributes
    /// it declares; `None` when `name` is not a derive.
    fn derive_helpers(&self, _name: &str) -> Option<Vec<String>> {
        None
    }
    /// Whether `name` is an attribute-like macro, called `#[name~ …]`.
    fn is_attribute(&self, _name: &str) -> bool {
        false
    }
    /// Run the macro on the stream's text; `Ok` is its expansion, Harsh
    /// text, `Err` the macro's own message.
    fn expand(&self, name: &str, input: &str) -> Result<String, String>;
    /// Whether there are none at all, so a file with no `~` definitions and
    /// no proc macros is left as it was without a scan.
    fn none(&self) -> bool {
        false
    }
}

/// No proc macros: single-file transpiles, holes, the formatter's paths.
pub struct NoProcMacros;

impl ProcMacros for NoProcMacros {
    fn has(&self, _: &str) -> bool {
        false
    }
    fn expand(&self, name: &str, _: &str) -> Result<String, String> {
        Err(format!("no proc macro `{name}`"))
    }
    fn none(&self) -> bool {
        true
    }
}

/// **Where a proc macro's tokens live.** They are lexed from the macro's own
/// output, so their spans index that text, not the file. Kept as they are,
/// they would index the file wherever they fell, and every place that reads
/// the source between two tokens would copy the user's text into the Rust.
/// So each expansion's spans are shifted into a region of their own, far past
/// any file: relative positions stay (`f$`, `m!`, `a[1]` are still tight),
/// every read of the source there finds nothing, and `at_call` sends a
/// position in a region back to its call -- where a diagnostic inside a proc
/// expansion points in v1.
pub const PROC_BASE: u32 = 1 << 30;
/// The size of one expansion's region: the most text one call may produce.
pub const PROC_STRIDE: u32 = 1 << 20;

/// A span inside a proc macro's expansion, moved to the programmer's code:
/// to the token it was copied from, when the macro copied it (Rust keeps a
/// copied token's span), and to the call otherwise. Any other span is
/// unchanged.
pub fn at_call(span: Span, trace: &[Expansion]) -> Span {
    if span.lo < PROC_BASE {
        return span;
    }
    let Some(x) = trace.iter().find(|x| x.proc_lo != 0 && x.proc_lo <= span.lo && span.lo < x.proc_lo + PROC_STRIDE) else {
        return span;
    };
    x.origins
        .iter()
        .find(|(at, _)| at.lo <= span.lo && span.lo < at.hi.max(at.lo + 1))
        .map(|(_, from)| *from)
        .unwrap_or(Span { lo: x.call_lo, hi: x.call_hi })
}

pub fn expand_all(toks: Vec<Token>, taken: &[String]) -> Result<Vec<Token>, Error> {
    expand_all_traced(toks, taken).map(|(t, _)| t)
}

/// The prelude's macros, added to a file's own definitions when the file
/// calls one of them: each under `hrs_std.NAME`, and under bare `NAME` unless
/// the file defines a macro of that name, which shadows it. A file that calls
/// none is left exactly as it was.
fn add_prelude(toks: &[Token], defs: &mut Vec<Def>) -> Result<(), Error> {
    let lexed = crate::lex::lex(PRELUDE).map_err(|e| Error { msg: format!("the prelude does not lex: {e:?}"), span: Span::new(0, 0) })?;
    let (prelude, _) = collect_defs(&lexed)?;
    let called = |name: &str| {
        toks.windows(2).any(|w| w[0].kind == Tk::Ident && w[0].text == name && w[1].text == "~" && w[0].span.hi == w[1].span.lo)
    };
    if !prelude.iter().any(|d| called(&d.name)) {
        return Ok(());
    }
    for d in prelude {
        if !defs.iter().any(|o| o.name == d.name) {
            defs.push(Def { name: d.name.clone(), arms: d.arms.clone(), span: Span::new(0, 0) });
        }
        defs.push(Def { name: format!("{PRELUDE_PATH}.{}", d.name), arms: d.arms, span: Span::new(0, 0) });
    }
    Ok(())
}

/// Inside the first bracket group of a call, a token that begins a new line
/// is preceded by a `;`, unless one is already there or the line holds only
/// the closing bracket.
fn rows_by_line(call: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(call.len() + 4);
    let mut depth = 0i32;
    for t in call {
        let starts_line = t.line_start.is_some();
        let closes = matches!(t.kind, Tk::Close(_));
        if depth == 1 && starts_line && !closes {
            // A line ending in `,` continues its row: more entries follow.
            let after_row = out.iter().rev().find(|x| !x.is_comment()).map_or(false, |p| {
                !matches!(p.kind, Tk::Semi | Tk::Comma | Tk::Open(_))
            });
            if after_row {
                let mut semi = t.clone();
                semi.kind = Tk::Semi;
                semi.text = ";".into();
                semi.synthetic = true;
                semi.line_start = None;
                out.push(semi);
            }
        }
        match t.kind {
            Tk::Open(_) => depth += 1,
            Tk::Close(_) => depth -= 1,
            _ => {}
        }
        out.push(t);
    }
    out
}

pub fn expand_all_traced(toks: Vec<Token>, taken: &[String]) -> Result<(Vec<Token>, Vec<Expansion>), Error> {
    expand_all_with(toks, taken, &NoProcMacros)
}

/// `expand_all_traced` with the project's proc macros callable as well.
pub fn expand_all_with(
    toks: Vec<Token>,
    _taken: &[String],
    procs: &dyn ProcMacros,
) -> Result<(Vec<Token>, Vec<Expansion>), Error> {
    let mut trace: Vec<Expansion> = Vec::new();
    let (mut defs, def_spans) = collect_defs(&toks)?;
    let own = defs.len();
    add_prelude(&toks, &mut defs)?;
    let quote = quote_imported(&toks);
    if defs.is_empty() && procs.none() && !quote && find_derive(&toks).is_none() && find_attribute(&toks).is_none() {
        return Ok((toks, trace));
    }
    // Drop the definitions themselves, back to front so the indices hold.
    let mut out = toks;
    for r in def_spans.iter().rev() {
        out.drain(r.clone());
    }
    // What an expansion must not capture is what the *caller* has in scope --
    // computed after the definitions are gone, or a macro's own local would
    // look like a clash with itself.
    let taken: Vec<String> = crate::layout::names_in_scope(&out);
    let taken = &taken[..];

    // A call whose mark does not match its definition: `adding!` against a
    // `macro_rules~ adding` is not a Rust macro that happens to be missing --
    // it is the wrong mark, and saying so here beats letting rustc report a
    // macro it cannot find (the user's request, 2026-09-17).
    // Only the file's own macros: a prelude macro's name used with `!` is
    // simply some Rust macro, and refusing it would make the prelude a trap.
    check_marks(&out, &defs[..own], procs)?;

    let limit = recursion_limit(&out);
    let mut ctx = 0u32;
    let mut regions = 0u32;
    for pass in 0..limit {
        // An attribute macro runs first, as in Rust -- a derive on the same
        // item sees the item only once the attribute macros have run.
        if let Some(at) = find_attribute(&out) {
            out = expand_attribute(out, at, procs, &mut trace, &mut ctx, &mut regions)?;
            continue;
        }
        // A derive runs on its item as written, before the calls inside it
        // expand -- Rust's order.
        if let Some(at) = find_derive(&out) {
            out = expand_derive(out, at, procs, &mut trace, &mut ctx, &mut regions)?;
            continue;
        }
        let Some((target, at, end, args)) = find_call(&out, &defs, own, procs, quote, 0) else {
            return Ok((out, trace));
        };
        let _ = pass;
        ctx += 1;
        let name = match &target {
            Target::Def(d) => defs[*d].name.clone(),
            Target::Proc(n) => n.clone(),
            Target::Quote => "quote".to_string(),
        };
        let proc_lo = match target {
            Target::Proc(_) | Target::Quote => {
                regions += 1;
                PROC_BASE.checked_add(regions * PROC_STRIDE).ok_or_else(|| Error {
                    span: out[at].span,
                    msg: "too many proc macro expansions in one file".into(),
                })?
            }
            Target::Def(_) => 0,
        };
        // A call inside a proc expansion has no place of its own in the
        // file: it is recorded at the call that produced it, so every
        // recorded call is somewhere the programmer can look.
        let call = at_call(Span { lo: out[at].span.lo, hi: out[args - 1].span.hi }, &trace);
        trace.push(Expansion { ctx, name: name.clone(), call_lo: call.lo, call_hi: call.hi, proc_lo, erased: Vec::new(), origins: Vec::new() });
        let (mut call, block_form) = call_stream(&out[args..end]);
        // A call with no tokens gives the matcher nothing to take a span
        // from; the error belongs on the call itself, not on line 1.
        let call_site = at_call(Span::new(out[at].span.lo as usize, out[at + 1].span.hi as usize), &trace);
        let col = line_col(&out, at);
        let lines = match target {
            Target::Def(d) => {
                let def = &defs[d];
                // The matrix literal's own grammar, Julia's: inside `m~ [ … ]` a line
                // break separates rows as `;` does, so a matrix can be laid out as it
                // reads (the user's request, 2026-09-21):
                //
                //     m~ [2 3 5
                //         5 7 8]
                //
                // Only the prelude's `m` -- `d >= own` marks the prelude's definitions
                // exactly -- never a file's own macro, and never any other bracket
                // list, where a line break carries no meaning.
                if d >= own && matches!(def.name.as_str(), "m" | "hrs_std.m" | "v" | "hrs_std.v") {
                    call = rows_by_line(call);
                }
                // Without an opener the deeper lines *continue the call's line*: they
                // are one logical line, so their line marks go. With `do:` they are a
                // block and keep them.
                if !block_form {
                    for t in call.iter_mut() {
                        t.line_start = None;
                    }
                }
                let (arm, b) = match_call(def, &call).map_err(|mut e| {
                    if e.span.lo == 0 && e.span.hi == 0 {
                        e.span = call_site;
                    }
                    e
                })?;
                let mut lines = substitute(&arm.body, &b, ctx)?;
                respell(&mut lines, ctx, taken);
                lines
            }
            Target::Quote => quote_expansion(&call, col, ctx, proc_lo)?,
            Target::Proc(_) => {
                let (lines, origins) = proc_expansion(&name, &call, col, call_site, procs, ctx, proc_lo, &trace)?;
                if let Some(x) = trace.last_mut() {
                    x.origins = origins;
                }
                lines
            }
        };
        let mut spliced: Vec<Token> = Vec::new();
        for (n, l) in lines.iter().enumerate() {
            for (k, t) in l.toks.iter().enumerate() {
                let mut t = t.clone();
                t.line_start = if k == 0 && n > 0 { Some(col + l.indent) } else { None };
                // The emitter copies source whitespace between spans; an
                // expanded token's span points into the definition, which is
                // no longer in the file, so it is written as synthetic.
                t.synthetic = true;
                spliced.push(t);
            }
        }
        // The first line of the expansion stands where the call stood, so it
        // keeps the call's own line_start.
        if let (Some(first), Some(head)) = (spliced.first_mut(), out.get(at)) {
            first.line_start = head.line_start;
        }
        out.splice(at..end, spliced);
    }
    err(
        out.first().map(|t| t.span).unwrap_or(Span::new(0, 0)),
        format!(
            "macro expansion went {limit} passes deep and is still going; raise it with `#![recursion_limit = \"…\"]` if it is not a loop"
        ),
    )
}

/// The `name~` calls still in `toks`, in order, each named once: after
/// expansion, the calls nothing defined -- neither the file, the prelude nor
/// the project's proc macros. `hrs expand` names them rather than print
/// them back as if they had been expanded.
pub fn unexpanded(toks: &[Token]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for w in toks.windows(2) {
        let marked = w[0].kind == Tk::Ident && w[1].text == "~" && w[0].span.hi == w[1].span.lo;
        if marked && !names.contains(&w[0].text) {
            names.push(w[0].text.clone());
        }
    }
    names
}

/// `#[name~ …]` at `i`, where `name` is not `derive`: the index of its `]`.
fn attribute_call_at(toks: &[Token], i: usize) -> Option<usize> {
    let call = toks[i].kind == Tk::Hash
        && toks.get(i + 1)?.kind == Tk::Open('[')
        && toks.get(i + 2)?.kind == Tk::Ident
        && toks[i + 2].text != "derive"
        && toks.get(i + 3)?.text == "~"
        && toks[i + 2].span.hi == toks[i + 3].span.lo;
    if call {
        matching_close(toks, i + 1)
    } else {
        None
    }
}

/// The first `#[name~ …]` in the file.
fn find_attribute(toks: &[Token]) -> Option<usize> {
    (0..toks.len()).find(|&i| attribute_call_at(toks, i).is_some())
}

/// The item an attribute at `at..=close` stands on, as Rust hands it to an
/// attribute macro: every outer attribute and doc comment of the item --
/// those above the macro's attribute and those below it -- its header, and
/// every line deeper (the `~` extent rule). Returns `start`, `head`, `end`.
fn attributed_item(out: &[Token], at: usize, close: usize, attr_span: Span) -> Result<(usize, usize, usize), Error> {
    let mut start = at;
    loop {
        if start > 0 && is_doc(&out[start - 1]) {
            start -= 1;
            continue;
        }
        if start > 0 && out[start - 1].kind == Tk::Close(']') {
            let open = (0..start - 1).rev().find(|&o| matching_close(out, o) == Some(start - 1));
            if let Some(o) = open.filter(|&o| o > 0 && out[o].kind == Tk::Open('[') && out[o - 1].kind == Tk::Hash) {
                start = o - 1;
                continue;
            }
        }
        break;
    }
    let mut head = close + 1;
    loop {
        let Some(t) = out.get(head) else {
            return err(attr_span, "an attribute macro stands on an item, and none follows it");
        };
        if t.is_comment() {
            head += 1;
            continue;
        }
        match attr_at(out, head) {
            Some(c) => head = c + 1,
            None => break,
        }
    }
    let col = line_col(out, head);
    let mut end = head + 1;
    let mut depth = 0i32;
    while end < out.len() {
        let t = &out[end];
        if depth == 0 && t.line_start.map_or(false, |c| c <= col) {
            break;
        }
        match t.kind {
            Tk::Open(_) => depth += 1,
            Tk::Close(_) if depth == 0 => break,
            Tk::Close(_) => depth -= 1,
            _ => {}
        }
        end += 1;
    }
    Ok((start, head, end))
}

/// Run the attribute-like macro of the `#[name~ …]` at `at`, as Rust does
/// (checked with rustc, 2026-09-24): it receives two streams -- the
/// attribute's arguments as written, and the item with all its other
/// attributes -- and what it returns *replaces* the item.
fn expand_attribute(
    mut out: Vec<Token>,
    at: usize,
    procs: &dyn ProcMacros,
    trace: &mut Vec<Expansion>,
    ctx: &mut u32,
    regions: &mut u32,
) -> Result<Vec<Token>, Error> {
    let close = attribute_call_at(&out, at).expect("find_attribute found it");
    let name = out[at + 2].text.clone();
    let attr_span = at_call(Span { lo: out[at].span.lo, hi: out[close].span.hi }, trace);
    if !procs.is_attribute(&name) {
        let msg = if procs.has(&name) {
            format!("`{name}` is a function-like proc macro, called `{name}~ stream`, not an attribute")
        } else if procs.derive_helpers(&name).is_some() {
            format!("`{name}` is a derive, called `#[derive~ {name}]`")
        } else {
            format!("no attribute macro `{name}` among this project's proc macros (`proc-macros` under [package.metadata.harsh])")
        };
        return err(attr_span, &msg);
    }
    let (start, head, end) = attributed_item(&out, at, close, attr_span)?;
    let col = line_col(&out, head);
    // The two streams: the arguments as written, and the item without this
    // attribute (plain comments dropped, doc comments kept).
    let args: Vec<Token> = out[at + 4..close].to_vec();
    let item: Vec<Token> = without(&out[start..end], start, &[(at, close)])
        .into_iter()
        .filter(|t| !t.is_comment() || is_doc(t))
        .collect();
    let (attr_text, attr_map) = if args.is_empty() { (String::new(), Vec::new()) } else { stream_text(&args, line_col(&out, at + 4), trace) };
    let (item_text, item_map) = stream_text(&item, col, trace);
    let sep = "\n\u{1e}\n";
    let from = (attr_text.len() + sep.len()) as u32;
    let mut map = attr_map;
    map.extend(item_map.into_iter().map(|(s, f)| (Span { lo: s.lo + from, hi: s.hi + from }, f)));
    let input = format!("{attr_text}{sep}{item_text}");
    *ctx += 1;
    *regions += 1;
    let proc_lo = PROC_BASE.checked_add(*regions * PROC_STRIDE).ok_or_else(|| Error {
        span: attr_span,
        msg: "too many proc macro expansions in one file".into(),
    })?;
    let call = format!("#[{name}~]");
    trace.push(Expansion { ctx: *ctx, name: call.clone(), call_lo: attr_span.lo, call_hi: attr_span.hi, proc_lo, erased: Vec::new(), origins: Vec::new() });
    let (lines, origins) = run_proc(&name, &format!("the attribute `{call}`"), &input, &map, attr_span, procs, *ctx, proc_lo)?;
    let mut made: Vec<Token> = Vec::new();
    for l in &lines {
        for (j, t) in l.toks.iter().enumerate() {
            let mut t = t.clone();
            t.line_start = if j == 0 { Some(col + l.indent) } else { None };
            t.synthetic = true;
            made.push(t);
        }
    }
    // The replaced item is erased from the text the emitter reads, as a
    // derive's attribute is.
    let erased = Span { lo: out[start].span.lo, hi: out[end - 1].span.hi };
    if let Some(x) = trace.last_mut() {
        x.origins = origins;
        if erased.lo < PROC_BASE && erased.hi < PROC_BASE {
            x.erased = vec![erased];
        }
    }
    out.splice(start..end, made);
    Ok(out)
}

/// `#[derive~ …]` at `i`: the index of its `]`.
fn derive_attr_at(toks: &[Token], i: usize) -> Option<usize> {
    let tilde = toks[i].kind == Tk::Hash
        && toks.get(i + 1)?.kind == Tk::Open('[')
        && toks.get(i + 2)?.kind == Tk::Ident
        && toks[i + 2].text == "derive"
        && toks.get(i + 3)?.text == "~"
        && toks[i + 2].span.hi == toks[i + 3].span.lo;
    if tilde {
        matching_close(toks, i + 1)
    } else {
        None
    }
}

/// Any outer attribute `#[…]` at `i`: the index of its `]`.
fn attr_at(toks: &[Token], i: usize) -> Option<usize> {
    (toks[i].kind == Tk::Hash && toks.get(i + 1)?.kind == Tk::Open('[')).then(|| matching_close(toks, i + 1))?
}

/// The first `#[derive~ …]` in the file.
fn find_derive(toks: &[Token]) -> Option<usize> {
    (0..toks.len()).find(|&i| derive_attr_at(toks, i).is_some())
}

fn is_doc(t: &Token) -> bool {
    t.is_comment() && ["///", "/**"].iter().any(|d| t.text.starts_with(d))
}

/// `toks` without the inclusive index ranges `cut`. A token that follows a
/// cut which began a line, on that same line, begins the line in its place
/// (`#[derive~ A] struct P` written inline leaves `struct` where `#` was).
fn without(toks: &[Token], from: usize, cut: &[(usize, usize)]) -> Vec<Token> {
    let mut kept = Vec::new();
    let mut pending: Option<usize> = None;
    for (k, t) in toks.iter().enumerate() {
        let at = from + k;
        if cut.iter().any(|&(a, b)| a <= at && at <= b) {
            if cut.iter().any(|&(a, _)| a == at) && pending.is_none() {
                pending = t.line_start;
            }
            continue;
        }
        let mut t = t.clone();
        if t.line_start.is_some() {
            pending = None;
        } else if let Some(c) = pending.take() {
            t.line_start = Some(c);
        }
        kept.push(t);
    }
    kept
}

/// Run the derives on the item beneath the `#[derive~ …]` at `at`, as Rust
/// does (the user's rule, 2026-09-24: for all Harsh proc macros, mirror
/// Rust's; checked with rustc the same day):
///
/// - **the item** is its outer attributes and doc comments, its header, and
///   every line deeper -- the `~` extent rule; it must be a `struct`, an
///   `enum` or a `union`;
/// - **the input** is that item as written, with every derive attribute
///   removed, Rust's and Harsh's (plain comments dropped, as for any call);
/// - **each derive** named by the item's `#[derive~ …]` attributes runs in
///   order, and what it returns is **added after the item**, which it cannot
///   change;
/// - then the **helper attributes** those derives declare are removed from
///   the item, as they are the derives' to read and rustc does not know them.
fn expand_derive(
    mut out: Vec<Token>,
    at: usize,
    procs: &dyn ProcMacros,
    trace: &mut Vec<Expansion>,
    ctx: &mut u32,
    regions: &mut u32,
) -> Result<Vec<Token>, Error> {
    let close = derive_attr_at(&out, at).expect("find_derive found it");
    let attr_span = at_call(Span { lo: out[at].span.lo, hi: out[close].span.hi }, trace);
    // Outer attributes and doc comments above belong to the item.
    let mut start = at;
    loop {
        if start > 0 && is_doc(&out[start - 1]) {
            start -= 1;
            continue;
        }
        if start > 0 && out[start - 1].kind == Tk::Close(']') {
            let open = (0..start - 1).rev().find(|&o| matching_close(&out, o) == Some(start - 1));
            if let Some(o) = open.filter(|&o| o > 0 && out[o].kind == Tk::Open('[') && out[o - 1].kind == Tk::Hash) {
                start = o - 1;
                continue;
            }
        }
        break;
    }
    // The item's header: past every attribute and comment.
    // (Not one `match` with a guard arm: `hrs-from` writes a `match` that
    // follows a guard arm wrongly -- a known gap, recorded 2026-09-24.)
    let mut head = close + 1;
    loop {
        let Some(t) = out.get(head) else {
            return err(attr_span, "`#[derive~ …]` has no item beneath it");
        };
        if t.is_comment() {
            head += 1;
            continue;
        }
        match attr_at(&out, head) {
            Some(c) => head = c + 1,
            None => break,
        }
    }
    let mut kw = head;
    if out[kw].is_kw("pub") {
        kw += 1;
        if out.get(kw).map_or(false, |t| t.kind == Tk::Open('(')) {
            kw = matching_close(&out, kw).map_or(kw, |c| c + 1);
        }
    }
    if !out.get(kw).map_or(false, |t| ["struct", "enum", "union"].iter().any(|k| t.is_kw(k) || t.text == *k)) {
        return err(attr_span, "a derive applies to a `struct`, an `enum` or a `union`, as in Rust");
    }
    let col = line_col(&out, head);
    // The item ends at the first line not deeper than its header.
    let mut end = head + 1;
    let mut depth = 0i32;
    while end < out.len() {
        let t = &out[end];
        if depth == 0 && t.line_start.map_or(false, |c| c <= col) {
            break;
        }
        match t.kind {
            Tk::Open(_) => depth += 1,
            Tk::Close(_) if depth == 0 => break,
            Tk::Close(_) => depth -= 1,
            _ => {}
        }
        end += 1;
    }
    // The item's derive attributes: Harsh's are run and removed; Rust's stay
    // in the item, and neither kind reaches a derive's input.
    let mut tilde: Vec<(usize, usize)> = Vec::new();
    let mut rust: Vec<(usize, usize)> = Vec::new();
    let mut names: Vec<(String, Vec<String>)> = Vec::new();
    let mut k = start;
    while k < head {
        if let Some(c) = derive_attr_at(&out, k) {
            for t in &out[k + 4..c] {
                if t.kind != Tk::Ident {
                    return err(t.span, "derives are named side by side, `#[derive~ Describe Show]`");
                }
                let helpers = procs.derive_helpers(&t.text).ok_or_else(|| Error {
                    span: attr_span,
                    msg: if procs.has(&t.text) {
                        format!("`{0}` is a function-like proc macro, called `{0}~ stream`; a derive is registered with `#[proc_macro_derive~ Name]`", t.text)
                    } else {
                        format!("no derive macro `{}` among this project's proc macros (`proc-macros` under [package.metadata.harsh])", t.text)
                    },
                })?;
                names.push((t.text.clone(), helpers));
            }
            tilde.push((k, c));
            k = c + 1;
        } else if let Some(c) = attr_at(&out, k) {
            if out.get(k + 2).map_or(false, |t| t.text == "derive") {
                rust.push((k, c));
            }
            k = c + 1;
        } else {
            k += 1;
        }
    }
    // The input, and the derives run on it.
    let mut cut_input = tilde.clone();
    cut_input.extend(rust.iter().copied());
    let input: Vec<Token> = without(&out[start..end], start, &cut_input)
        .into_iter()
        .filter(|t| !t.is_comment() || is_doc(t))
        .collect();
    let (text, map) = stream_text(&input, col, trace);
    let mut added: Vec<Token> = Vec::new();
    let mut helpers: Vec<String> = Vec::new();
    for (name, hs) in &names {
        *ctx += 1;
        *regions += 1;
        let proc_lo = PROC_BASE.checked_add(*regions * PROC_STRIDE).ok_or_else(|| Error {
            span: attr_span,
            msg: "too many proc macro expansions in one file".into(),
        })?;
        // Recorded as the call is written, which is how the remapper names it.
        let call = format!("#[derive~ {name}]");
        trace.push(Expansion { ctx: *ctx, name: call, call_lo: attr_span.lo, call_hi: attr_span.hi, proc_lo, erased: Vec::new(), origins: Vec::new() });
        let (lines, origins) = run_proc(name, &format!("the derive `{name}`"), &text, &map, attr_span, procs, *ctx, proc_lo)?;
        if let Some(x) = trace.last_mut() {
            x.origins = origins;
        }
        for l in &lines {
            for (j, t) in l.toks.iter().enumerate() {
                let mut t = t.clone();
                t.line_start = if j == 0 { Some(col + l.indent) } else { None };
                t.synthetic = true;
                added.push(t);
            }
        }
        helpers.extend(hs.iter().cloned());
    }
    // The helpers the derives declared, anywhere in the item.
    let mut cut = tilde.clone();
    let mut k = start;
    while k < end {
        match attr_at(&out, k) {
            Some(c) => {
                if out.get(k + 2).map_or(false, |t| helpers.contains(&t.text)) {
                    cut.push((k, c));
                }
                k = c + 1;
            }
            None => k += 1,
        }
    }
    let erased: Vec<Span> = cut
        .iter()
        .map(|&(a, b)| Span { lo: out[a].span.lo, hi: out[b].span.hi })
        .filter(|s| s.lo < PROC_BASE)
        .collect();
    if let Some(x) = trace.last_mut() {
        x.erased = erased;
    }
    let mut item = without(&out[start..end], start, &cut);
    item.extend(added);
    out.splice(start..end, item);
    Ok(out)
}

/// Whether the file imports `hrs_quote.quote` -- a `use` line naming both --
/// which is what makes `quote~` available, as `use quote::quote` does for
/// Rust's `quote!`.
fn quote_imported(toks: &[Token]) -> bool {
    toks.iter().enumerate().any(|(i, t)| {
        if !t.is_kw("use") {
            return false;
        }
        let line: Vec<&Token> = toks[i + 1..].iter().take_while(|u| u.line_start.is_none()).collect();
        line.iter().any(|u| u.text == "hrs_quote") && line.iter().any(|u| u.text == "quote")
    })
}

/// `quote~ do:` and its template, into the call of `hrs_quote`'s `quote!`
/// that builds the stream at run time (`SYN-QUOTE-PLAN`, steps 3 and 4). The
/// template is rendered with its layout; each `#name` becomes a value piece
/// and each `#( … )*` a repetition piece, repeating as lines when it begins
/// one; the text between becomes text pieces. Columns are measured by
/// `hrs_quote` at run time, when the values' widths are known.
fn quote_expansion(call: &[Token], col: usize, ctx: u32, base: u32) -> Result<Vec<OutLine>, Error> {
    struct Rep {
        vars: Vec<String>,
        inner: String,
        sep: String,
        lines: bool,
    }
    let tight_hash = |toks: &[Token], k: usize| -> Option<usize> {
        let t = toks.get(k)?;
        let n = toks.get(k + 1)?;
        (t.kind == Tk::Hash && t.span.hi == n.span.lo).then_some(k + 1)
    };
    let placeholder = |from: &Token, to: &Token, text: String| -> Token {
        let mut p = from.clone();
        p.kind = Tk::Ident;
        p.text = text;
        p.span = Span { lo: from.span.lo, hi: to.span.hi };
        p
    };
    let mut toks: Vec<Token> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut reps: Vec<Rep> = Vec::new();
    let mut k = 0;
    while k < call.len() {
        let t = &call[k];
        match tight_hash(call, k).map(|n| (n, &call[n])) {
            Some((_, n)) if n.kind == Tk::Ident => {
                toks.push(placeholder(t, n, format!("__hrs_quote_{}__", names.len())));
                names.push(n.text.clone());
                k += 2;
            }
            Some((open, n)) if n.kind == Tk::Open('(') => {
                let close = matching_close(call, open).ok_or_else(|| Error { span: t.span, msg: "`#(` is never closed".into() })?;
                // `)*`, or `)` + one separator + `*`, as in `quote`.
                let (sep, end) = match (call.get(close + 1), call.get(close + 2)) {
                    (Some(s), _) if s.text == "*" => (String::new(), close + 1),
                    (Some(s), Some(star)) if star.text == "*" && !matches!(s.kind, Tk::Ident | Tk::Open(_) | Tk::Close(_)) => (s.text.clone(), close + 2),
                    _ => return err(t.span, "a repetition ends `)*`, or `)` then one separator then `*` -- `#( #x ),*` -- as in `quote`"),
                };
                let mut inner: Vec<Token> = Vec::new();
                let mut vars: Vec<String> = Vec::new();
                let mut j = open + 1;
                while j < close {
                    match tight_hash(call, j).map(|n| &call[n]) {
                        Some(n) if n.kind == Tk::Ident => {
                            let at = vars.iter().position(|v| *v == n.text).unwrap_or_else(|| {
                                vars.push(n.text.clone());
                                vars.len() - 1
                            });
                            inner.push(placeholder(&call[j], n, format!("__hrs_qv_{at}__")));
                            j += 2;
                        }
                        Some(n) if n.kind == Tk::Open('(') => {
                            return err(call[j].span, "a repetition inside a repetition is not built yet");
                        }
                        _ => {
                            inner.push(call[j].clone());
                            j += 1;
                        }
                    }
                }
                if vars.is_empty() {
                    return err(t.span, "a repetition needs a `#name` to iterate, as in `quote`");
                }
                if let Some(first) = inner.first_mut() {
                    first.line_start = None;
                }
                reps.push(Rep { vars, inner: stream_render(&inner, 0), sep, lines: t.line_start.is_some() });
                toks.push(placeholder(t, &call[end], format!("__hrs_rep_{}__", reps.len() - 1)));
                k = end + 1;
            }
            _ => {
                toks.push(t.clone());
                k += 1;
            }
        }
    }
    let text = stream_render(&toks, col);
    let text_piece = |t: &str| format!(" (hrs_quote.Piece.Text {t:?})");
    let mut code = String::from("quote! (vec!");
    let mut pieces = 0;
    let mut rest = text.as_str();
    loop {
        let next = [("__hrs_quote_", false), ("__hrs_rep_", true)]
            .iter()
            .filter_map(|(m, is_rep)| rest.find(m).map(|at| (at, *m, *is_rep)))
            .min_by_key(|x| x.0);
        let Some((at, marker, is_rep)) = next else { break };
        let before = &rest[..at];
        let after = &rest[at + marker.len()..];
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        let n: usize = digits.parse().unwrap_or(0);
        if !before.is_empty() {
            code.push_str(&text_piece(before));
            pieces += 1;
        }
        if is_rep {
            let r = &reps[n];
            let cols: Vec<String> = r.vars.iter().map(|v| format!(" (hrs_quote.each (&{v}))")).collect();
            let mut subs = String::new();
            let mut inner = r.inner.as_str();
            while let Some(p) = inner.find("__hrs_qv_") {
                let (b, a) = (&inner[..p], &inner[p + "__hrs_qv_".len()..]);
                let d: String = a.chars().take_while(|c| c.is_ascii_digit()).collect();
                if !b.is_empty() {
                    subs.push_str(&format!(" (hrs_quote.Sub.Text {b:?})"));
                }
                subs.push_str(&format!(" (hrs_quote.Sub.Var {d})"));
                inner = &a[d.len() + "__".len()..];
            }
            if !inner.is_empty() {
                subs.push_str(&format!(" (hrs_quote.Sub.Text {inner:?})"));
            }
            let lines = r.lines;
            code.push_str(&format!(
                " (hrs_quote.Piece.Repeat (hrs_quote.rows (vec!{})) (vec!{subs}) {:?} {lines})",
                cols.concat(),
                r.sep
            ));
        } else {
            code.push_str(&format!(" (hrs_quote.Piece.Value (hrs_quote.to_tokens (&{})))", names[n]));
        }
        pieces += 1;
        rest = &after[digits.len() + "__".len()..];
    }
    if !rest.is_empty() {
        code.push_str(&text_piece(rest));
        pieces += 1;
    }
    code.push(')');
    if pieces == 0 {
        code = "quote! (Vec.new$)".to_string();
    }
    let at = call.first().map_or(Span::new(0, 0), |t| t.span);
    let mut out = crate::lex::lex(&code).map_err(|e| Error { span: at, msg: format!("quote~: {}", e.msg) })?;
    for t in out.iter_mut() {
        t.span = Span { lo: t.span.lo + base, hi: t.span.hi + base };
        t.ctx = ctx;
    }
    Ok(split_lines(&out))
}

/// A call's stream as its macro receives it, and whether it was handed as a
/// block (`m~ do:` with the body beneath). Shared by both kinds of macro, so
/// a call reads the same whichever implements it.
fn call_stream(raw: &[Token]) -> (Vec<Token>, bool) {
    let mut call: Vec<Token> = raw.to_vec();
    // A macro never sees a comment: Rust's lexer drops them before
    // `macro_rules!` runs. Here they reached the matcher, a spanning
    // fragment took `1..20 // only a is available` as one expression,
    // and the expansion -- one line -- was commented out from there on
    // (2026-09-21, found writing the Book's annotated comprehension).
    // Doc comments are the exception, in Rust as here: they are
    // attributes, and a macro may be given documented items.
    call.retain(|t| {
        !t.is_comment()
            || ["///", "//!", "/**", "/*!"].iter().any(|d| t.text.starts_with(d))
    });
    // When the arguments were handed as a block the opener belongs to the
    // call, not to the arguments, so it is dropped: what the macro sees is
    // the block's own tokens.
    let mut block_form = false;
    loop {
        let head_len = call
            .iter()
            .position(|t| t.line_start.is_some())
            .unwrap_or(call.len());
        let last = match call[..head_len].iter().rposition(|t| !t.is_comment()) {
            Some(k) => k,
            None => break,
        };
        let opener = call[last].kind == Tk::Colon || call[last].is_kw("do");
        if !opener {
            break;
        }
        call.remove(last);
        block_form = true;
    }
    (call, block_form)
}

/// A proc macro's call, run: the stream rendered as text laid out as it was
/// written -- each line's indentation relative to the stream's least-indented
/// line, so a one-line stream is one line and a `do:` body starts at column
/// 0 -- handed to the macro, and what comes back read as Harsh (ruling 17).
///
/// **No hygiene** (Claude's decision 1, the user's to overrule): Rust's
/// `"…".parse::<TokenStream>()` gives every token `Span::call_site()`, so a
/// text-built Rust proc macro is unhygienic, and Harsh mirrors Rust. The
/// tokens still carry `ctx`, so the trace and diagnostics know them.
fn proc_expansion(
    name: &str,
    call: &[Token],
    col: usize,
    call_site: Span,
    procs: &dyn ProcMacros,
    ctx: u32,
    base: u32,
    trace: &[Expansion],
) -> Result<(Vec<OutLine>, Vec<(Span, Span)>), Error> {
    let (input, map) = stream_text(call, col, trace);
    run_proc(name, &format!("`{name}~`"), &input, &map, call_site, procs, ctx, base)
}

/// Tokens as a proc macro receives them: text, each line indented relative
/// to the least-indented line (decision C2) -- and the input map: for each
/// token's bytes in that text, the programmer's token it is. Built by lexing
/// the text and pairing its tokens with `call`'s, one for one; when the
/// counts differ (they should not) the map is empty and every diagnostic
/// goes to the call, as before.
fn stream_text(call: &[Token], col: usize, trace: &[Expansion]) -> (String, Vec<(Span, Span)>) {
    let text = stream_render(call, col);
    let map = match crate::lex::lex(&text) {
        Ok(back) if back.len() == call.len() => back.iter().zip(call).map(|(b, t)| (b.span, at_call(t.span, trace))).collect(),
        _ => Vec::new(),
    };
    (text, map)
}

fn stream_render(call: &[Token], col: usize) -> String {
    let mut lines = split_lines(call);
    for l in lines.iter_mut() {
        l.indent = l.indent.saturating_sub(col);
    }
    let least = lines.iter().map(|l| l.indent).min().unwrap_or(0);
    for l in lines.iter_mut() {
        l.indent -= least;
    }
    render(&lines, 0)
}

/// Run one proc macro on `input` and read what it returns as Harsh, its
/// tokens moved into their region past the file and marked with `ctx`.
/// `label` names the macro as its call does, for errors: `` `name~` `` for a
/// function-like macro, "the derive `Name`" for a derive.
fn run_proc(
    name: &str,
    label: &str,
    input: &str,
    input_map: &[(Span, Span)],
    call_site: Span,
    procs: &dyn ProcMacros,
    ctx: u32,
    base: u32,
) -> Result<(Vec<OutLine>, Vec<(Span, Span)>), Error> {
    let answer = procs
        .expand(name, input)
        .map_err(|m| Error { span: call_site, msg: format!("{label} failed: {}", m.trim_end()) })?;
    // The runner's answer: the text, then (after a separator) where the
    // pieces it copied from the input came from -- `hrs_proc_macro::SPANS`.
    let (output, table) = match answer.split_once("\n\u{1e}\n") {
        Some((text, spans)) => (text.to_string(), spans_table(spans)),
        None => (answer, Vec::new()),
    };
    if output.len() >= PROC_STRIDE as usize {
        return Err(Error {
            span: call_site,
            msg: format!("{label} expanded to {} bytes; one call may produce at most {PROC_STRIDE}", output.len()),
        });
    }
    let mut toks = crate::lex::lex(&output).map_err(|e| Error {
        span: call_site,
        msg: format!("{label} expanded to Harsh that does not read: {}", e.msg),
    })?;
    // Each token's origin: the piece of the answer it lies in, that piece's
    // place in the input, and the programmer's token at that place.
    let mut origins = Vec::new();
    for t in toks.iter() {
        let from = table
            .iter()
            .find(|(out_lo, out_hi, _, _)| *out_lo <= t.span.lo && t.span.lo < *out_hi)
            .and_then(|(_, _, in_lo, _)| input_map.iter().find(|(at, _)| at.lo <= *in_lo && *in_lo < at.hi.max(at.lo + 1)))
            .map(|(_, file)| *file);
        if let Some(file) = from {
            origins.push((Span { lo: t.span.lo + base, hi: t.span.hi + base }, file));
        }
    }
    for t in toks.iter_mut() {
        t.span = Span { lo: t.span.lo + base, hi: t.span.hi + base };
        t.ctx = ctx;
    }
    Ok((split_lines(&toks), origins))
}

/// `text_lo text_hi input_lo input_hi` lines; anything else is skipped.
fn spans_table(text: &str) -> Vec<(u32, u32, u32, u32)> {
    text.lines()
        .filter_map(|l| {
            let n: Vec<u32> = l.split_whitespace().filter_map(|w| w.parse().ok()).collect();
            (n.len() == 4).then(|| (n[0], n[1], n[2], n[3]))
        })
        .collect()
}

#[cfg(test)]
mod pipeline_tests {
    use super::*;

    fn toks(src: &str) -> Vec<Token> {
        crate::lex::lex(src).expect("lex")
    }

    fn expand(src: &str) -> String {
        let out = expand_all(toks(src), &[]).expect("expand");
        let lines = split_lines(&out);
        render(&lines, 0)
    }

    #[test]
    fn a_definition_is_read_from_a_file() {
        let (defs, _) = collect_defs(&toks(
            "macro_rules~ pair\n    (($a:expr) ($b:expr)) => do:\n        ($a, $b)\n",
        ))
        .expect("collect");
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "pair");
        assert_eq!(defs[0].arms.len(), 1);
        assert_eq!(bindings(&defs[0].arms[0].matcher).len(), 2);
    }

    #[test]
    fn several_arms_are_read() {
        let (defs, _) = collect_defs(&toks(
            "macro_rules~ m\n    (($a:expr)) => do:\n        $a\n    (($a:expr) ($b:expr)) => do:\n        $a + $b\n",
        ))
        .expect("collect");
        assert_eq!(defs[0].arms.len(), 2);
    }

    #[test]
    fn the_definition_leaves_no_trace_and_the_call_expands() {
        let out = expand("macro_rules~ twice\n    (($x:expr)) => do:\n        $x * 2\n\nlet n = twice~ 21\n");
        assert!(!out.contains("macro_rules"), "the definition is gone:\n{out}");
        assert!(out.contains("let n = 21 * 2"), "{out}");
    }

    #[test]
    fn a_repetition_expands_to_lines_at_the_calls_column() {
        let out = expand(
            "macro_rules~ push_all\n    (($v:ident) $( ($x:expr) )*) => do:\n        $( $v <- push $x )*\n\nfn main$:\n    push_all~ v 1 2\n",
        );
        assert!(out.contains("v <- push 1"), "{out}");
        assert!(out.contains("v <- push 2"), "{out}");
    }

    #[test]
    fn a_call_with_no_matching_arm_says_so() {
        let e = expand_all(
            toks("macro_rules~ one\n    (($a:ident)) => do:\n        $a\n\nlet x = one~ 1\n"),
            &[],
        )
        .err()
        .expect("refused");
        assert!(e.msg.contains("no arm of `one~` matches"), "{}", e.msg);
    }

    #[test]
    fn an_empty_call_that_matches_nothing_is_reported_at_the_call() {
        let src = "macro_rules~ one\n    (($a:ident)) => do:\n        $a\n\nlet x = one~\n";
        let e = expand_all(toks(src), &[]).err().expect("refused");
        assert!(e.msg.contains("no arm of `one~` matches"), "{}", e.msg);
        assert_eq!(e.span.lo as usize, src.find("one~").unwrap(), "not at the top of the file");
    }

    #[test]
    fn a_macro_that_calls_itself_for_ever_is_stopped() {
        let e = expand_all(
            toks("macro_rules~ loopy\n    (($a:expr)) => do:\n        loopy~ $a\n\nlet x = loopy~ 1\n"),
            &[],
        )
        .err()
        .expect("refused");
        assert!(e.msg.contains("passes deep"), "{}", e.msg);
        assert!(e.msg.contains("recursion_limit"), "{}", e.msg);
    }

    #[test]
    fn a_file_with_no_harsh_macro_is_untouched() {
        let src = "fn main$:\n    println! \"hi\"\n";
        let before = toks(src);
        let after = expand_all(before.clone(), &[]).expect("expand");
        assert_eq!(after.len(), before.len());
    }
}

/// Refuse a call marked `!` when the name is a Harsh macro.
///
/// The other direction -- `name~` where no Harsh macro of that name exists --
/// is left to the ordinary path: the mark is normalised to `!` and rustc
/// names the macro it cannot find, which is the right answer when the macro
/// is a Rust one whose crate was not imported.
fn check_marks(toks: &[Token], defs: &[Def], procs: &dyn ProcMacros) -> Result<(), Error> {
    for i in 0..toks.len().saturating_sub(1) {
        let bang = toks[i].kind == Tk::Ident
            && toks[i + 1].text == "!"
            && toks[i].span.hi == toks[i + 1].span.lo;
        if !bang {
            continue;
        }
        // `macro_rules!` is Rust's own zone opener, not a call.
        if toks[i].is_kw("macro_rules") {
            continue;
        }
        if let Some(d) = defs.iter().find(|d| d.name == toks[i].text) {
            return err(
                toks[i].span,
                format!(
                    "`{name}` is a Harsh macro, so it is called `{name}~`; `{name}!` is Rust's mark",
                    name = d.name
                ),
            );
        }
        if procs.has(&toks[i].text) {
            return err(
                toks[i].span,
                format!(
                    "`{name}` is a Harsh proc macro, so it is called `{name}~`; `{name}!` is Rust's mark",
                    name = toks[i].text
                ),
            );
        }
    }
    Ok(())
}

/// `#![recursion_limit = "N"]` at the top of the file, else Rust's default.
fn recursion_limit(toks: &[Token]) -> usize {
    for i in 0..toks.len().saturating_sub(5) {
        let attr = toks[i].text == "#"
            && toks[i + 1].text == "!"
            && toks[i + 2].kind == Tk::Open('[')
            && toks[i + 3].is_kw("recursion_limit")
            && toks[i + 4].kind == Tk::Eq
            && toks[i + 5].kind == Tk::Str;
        if attr {
            let n = toks[i + 5].text.trim_matches('"');
            if let Ok(n) = n.parse::<usize>() {
                return n;
            }
        }
    }
    RECURSION_LIMIT
}

/// A whole token stream as Harsh source, line by line: what `hrs expand`
/// prints. Each line keeps the column its first token records.
pub fn render_file(toks: &[Token]) -> String {
    let lines = split_lines(toks);
    let mut s = String::new();
    let mut first = true;
    for l in &lines {
        if !first {
            s.push('\n');
        }
        first = false;
        let one = vec![OutLine { indent: 0, toks: l.toks.clone() }];
        s.push_str(&render(&one, l.indent));
    }
    s
}

/// Harsh proc macros in the expander (ruling 17, step 3 of the path), with a
/// fake registry: the call, the stream as the macro receives it, the splice,
/// the errors, who wins a name. The real runner is the driver's business.
#[cfg(test)]
mod proc_tests {
    use super::*;
    use std::cell::RefCell;

    type Mac = fn(&str) -> Result<String, String>;

    struct Fake {
        macros: Vec<(&'static str, Mac)>,
        derives: Vec<(&'static str, &'static [&'static str], Mac)>,
        attributes: Vec<(&'static str, Mac)>,
        seen: RefCell<Vec<String>>,
    }

    impl ProcMacros for Fake {
        fn has(&self, name: &str) -> bool {
            self.macros.iter().any(|(n, _)| *n == name)
        }
        fn derive_helpers(&self, name: &str) -> Option<Vec<String>> {
            self.derives.iter().find(|(n, _, _)| *n == name).map(|(_, h, _)| h.iter().map(|s| s.to_string()).collect())
        }
        fn is_attribute(&self, name: &str) -> bool {
            self.attributes.iter().any(|(n, _)| *n == name)
        }
        fn expand(&self, name: &str, input: &str) -> Result<String, String> {
            self.seen.borrow_mut().push(input.to_string());
            let f = self.macros.iter().find(|(n, _)| *n == name).map(|m| m.1);
            let f = f.or_else(|| self.derives.iter().find(|(n, _, _)| *n == name).map(|d| d.2));
            let f = f.or_else(|| self.attributes.iter().find(|(n, _)| *n == name).map(|a| a.1)).unwrap();
            f(input)
        }
    }

    fn hello(i: &str) -> Result<String, String> {
        Ok(format!("\"Hello, {i}!\""))
    }
    fn unit(_: &str) -> Result<String, String> {
        Ok("()".into())
    }
    fn two_lets(_: &str) -> Result<String, String> {
        Ok("let a = 1\nlet b = a + 1".into())
    }
    fn nested(_: &str) -> Result<String, String> {
        Ok("hello_macro~ inner".into())
    }
    /// A derive that reads the item's name from the text, as a v1 derive must.
    fn describe(i: &str) -> Result<String, String> {
        let words: Vec<&str> = i.split_whitespace().collect();
        let at = words.iter().position(|w| ["struct", "enum", "union"].contains(w)).ok_or("no item")?;
        let name = words[at + 1];
        Ok(format!("impl {name}\n    pub fn describe$ -> &'static str:\n        \"{name}\""))
    }
    /// An attribute macro: logs the route, then the item unchanged.
    fn route(i: &str) -> Result<String, String> {
        let (args, item) = i.split_once("\n\u{1e}\n").ok_or("two streams")?;
        Ok(format!("const ROUTE: &str = {:?}\n{item}", args))
    }
    fn named(_: &str) -> Result<String, String> {
        Ok("const NAMED: bool = true".into())
    }

    fn fake() -> Fake {
        Fake {
            macros: vec![("hello_macro", hello), ("rec", unit), ("two", two_lets), ("g", unit), ("nest", nested)],
            derives: vec![("Describe", &["describe"], describe), ("Named", &[], named)],
            attributes: vec![("route", route)],
            seen: RefCell::new(Vec::new()),
        }
    }

    fn expand_with(src: &str, procs: &Fake) -> Result<(Vec<Token>, Vec<Expansion>), Error> {
        let toks = crate::lex::lex(src).expect("lex");
        let taken = crate::layout::names_in_scope(&toks);
        expand_all_with(toks, &taken, procs)
    }

    fn rust(src: &str, procs: &Fake) -> String {
        let (toks, trace) = expand_with(src, procs).expect("expand");
        let tree = crate::layout::build_with(toks, &Default::default()).expect("layout");
        let text = erase(src, &trace);
        let mut em = crate::emit::Emitter::new(&text);
        em.program(&tree);
        em.out
    }

    #[test]
    fn a_proc_call_is_replaced_by_what_the_macro_returns() {
        let src = "fn main$:\n    println! \"{}\" (hello_macro~ world)\n";
        assert_eq!(rust(src, &fake()), "fn main() {\n    println!(\"{}\", \"Hello, world!\")\n}\n");
    }

    #[test]
    fn the_stream_arrives_laid_out_as_written() {
        let f = fake();
        let src = "fn main$:\n    let x = rec~ a, b\n        c <- d$\n            e\n    rec~ do:\n        p\n            q\n    rec~ (1, 2) // a comment\n";
        expand_with(src, &f).unwrap();
        assert_eq!(*f.seen.borrow(), ["a, b\n    c <- d$\n        e", "p\n    q", "(1, 2)"]);
    }

    /// A proc macro is unhygienic, as a text-built Rust one is: what it
    /// introduces the caller sees (decision 1). A multi-line expansion lands
    /// at the call's column.
    #[test]
    fn a_multi_line_expansion_lands_at_the_call_and_is_not_hygienic() {
        let src = "fn main$:\n    two~\n    println! \"{}\" b\n";
        assert_eq!(rust(src, &fake()), "fn main() {\n    let a = 1;\n    let b = a + 1;\n    println!(\"{}\", b)\n}\n");
    }

    #[test]
    fn an_expansion_lives_past_the_file_and_is_traced_to_its_call() {
        let src = "fn main$:\n    let s = hello_macro~ world\n";
        let (toks, trace) = expand_with(src, &fake()).unwrap();
        let x = &trace[0];
        // A call is recorded as its head, `name~`, as a `macro_rules~` call is.
        assert_eq!((x.name.as_str(), &src[x.call_lo as usize..x.call_hi as usize]), ("hello_macro", "hello_macro~"));
        let made: Vec<&Token> = toks.iter().filter(|t| t.ctx == x.ctx).collect();
        assert_eq!(made.len(), 1);
        assert!(made[0].span.lo >= PROC_BASE && made[0].synthetic);
        assert_eq!(at_call(made[0].span, &trace), Span { lo: x.call_lo, hi: x.call_hi });
    }

    /// A proc call inside a proc expansion is recorded at the outer call, so
    /// every recorded call is a place in the file.
    #[test]
    fn a_call_inside_an_expansion_is_recorded_at_the_outer_call() {
        let src = "fn main$:\n    let s = nest~ x\n";
        let (_, trace) = expand_with(src, &fake()).unwrap();
        assert_eq!(trace.len(), 2);
        assert_eq!((trace[1].call_lo, trace[1].call_hi), (trace[0].call_lo, trace[0].call_hi));
    }

    #[test]
    fn a_failing_macro_is_reported_at_its_call() {
        fn boom(_: &str) -> Result<String, String> {
            Err("boom\n".into())
        }
        fn unread(_: &str) -> Result<String, String> {
            Ok("\"never closed".into())
        }
        let f = Fake { macros: vec![("boom", boom), ("unread", unread)], derives: vec![], attributes: vec![], seen: RefCell::new(Vec::new()) };
        let src = "fn main$:\n    let s = boom~ 1\n";
        let e = expand_with(src, &f).unwrap_err();
        assert_eq!(e.msg, "`boom~` failed: boom");
        assert_eq!(&src[e.span.lo as usize..e.span.hi as usize], "boom~");
        let e = expand_with("fn main$:\n    unread~\n", &f).unwrap_err();
        assert!(e.msg.starts_with("`unread~` expanded to Harsh that does not read"), "{}", e.msg);
    }

    /// Harsh that lexes but does not lay out is refused by the layout pass,
    /// on a token of the expansion; `at_call` puts that at the call. (Not
    /// every malformed expansion is layout's to refuse: `let x =` lays out,
    /// and rustc's error reaches the call through the map -- the end-to-end
    /// test pins that.)
    #[test]
    fn a_malformed_expansion_is_an_error_at_the_call() {
        fn dangling(_: &str) -> Result<String, String> {
            // A retired spelling: it lexes, and the layout pass refuses it on
            // the expansion's own token. (An unclosed `(` is refused too, but
            // on the caller's next line, the first token that proves it; and
            // `::` is refused by the lexer, "does not read".)
            Ok("match 1: 1 => 1, _ => 0".into())
        }
        let f = Fake { macros: vec![("dangling", dangling)], derives: vec![], attributes: vec![], seen: RefCell::new(Vec::new()) };
        let src = "fn main$:\n    dangling~ 1\n    let y = 2\n";
        let (toks, trace) = expand_with(src, &f).unwrap();
        let e = crate::layout::build_with(toks, &Default::default()).expect_err("layout");
        let at = at_call(e.span, &trace);
        assert_eq!(&src[at.lo as usize..at.hi as usize], "dangling~", "{}", e.msg);
    }

    /// Who wins a name: the file's own `macro_rules~`, then the project's
    /// proc macros, then the prelude; `hrs_std.g~` is always the prelude's.
    #[test]
    fn a_files_own_macro_shadows_a_proc_macro_which_shadows_the_prelude() {
        let f = fake();
        let own = "macro_rules~ hello_macro\n    (($x:ident)) => do: \"own\"\n\nfn main$:\n    let s = hello_macro~ w\n";
        assert!(rust(own, &f).contains("let s = \"own\""));
        assert!(f.seen.borrow().is_empty());
        assert!(rust("fn main$:\n    let s = g~ x for x in 0..3\n", &f).contains("let s = ()"));
        let prelude = rust("fn main$:\n    let s: Vec<i32> = hrs_std.g~ x for x in 0..3 <- collect$\n", &f);
        assert!(prelude.contains("flat_map") || prelude.contains("filter_map") || prelude.contains("map"), "{prelude}");
    }

    /// A derive receives its item as Rust gives it -- outer attributes and
    /// doc comments kept, every derive attribute removed, its helpers present
    /// -- and its output is added after the item, which it does not change;
    /// then its helpers leave the item and Rust's own derive stays.
    #[test]
    fn a_derive_sees_its_item_as_rust_gives_it_and_adds_beside_it() {
        let f = fake();
        let src = "/// A point.\n#[derive Debug]\n#[derive~ Describe]\n#[allow dead_code]\npub struct P\n    #[describe skip] x: i32\n    y: i32\n\nfn main$:\n    println! \"{}\" (P.describe$)\n";
        let out = rust(src, &f);
        assert_eq!(*f.seen.borrow(), ["/// A point.\n#[allow dead_code]\npub struct P\n    #[describe skip] x: i32\n    y: i32"]);
        assert!(out.contains("#[derive(Debug)]\n"), "{out}");
        assert!(out.contains("pub struct P {\n    x: i32,\n    y: i32,\n}\nimpl P {\n    pub fn describe() -> &'static str {\n        \"P\"\n    }\n}"), "{out}");
        assert!(!out.contains("describe skip") && !out.contains("derive~") && !out.contains("derive!"), "{out}");
    }

    /// Written inline, the derive and a helper leave no text behind. The
    /// emitter copies source through some gaps: a helper on a tuple
    /// variant's field came back as `A(#[describe skip] i32)` -- Harsh's
    /// spelling, raw -- until the removed spans were erased from the text it
    /// reads (a mutation disabling `erase` makes this test fail; the first
    /// version of it, with the derive at the top of the file and no gap
    /// before it, did not). The erased width stays as spaces, `A(      i32)`:
    /// valid Rust, recorded as cosmetic.
    #[test]
    fn an_inline_derive_and_helper_leave_nothing_behind() {
        let src = "const K: i32 = 1\n#[derive~ Describe] enum E\n    #[describe skip] A (#[describe skip] i32)\n    B\n";
        let out = rust(src, &fake());
        assert!(!out.contains("describe skip") && !out.contains("derive"), "{out}");
        assert!(out.contains("enum E {\n    A("), "{out}");
        assert!(out.contains("i32),\n    B,\n}\nimpl E {"), "{out}");
    }

    /// Several derives run in the order written, one attribute or several,
    /// each on the same input, each output added after the item in turn.
    #[test]
    fn derives_run_in_the_order_written() {
        let f = fake();
        let out = rust("#[derive~ Named Describe]\nstruct P\n    x: i32\n", &f);
        let (n, d) = (out.find("NAMED").unwrap(), out.find("impl P").unwrap());
        assert!(n < d, "{out}");
        assert_eq!(f.seen.borrow().len(), 2);
        assert_eq!(f.seen.borrow()[0], f.seen.borrow()[1]);
        let f = fake();
        let out = rust("#[derive~ Describe]\n#[derive~ Named]\nstruct P\n    x: i32\n", &f);
        assert!(out.find("impl P").unwrap() < out.find("NAMED").unwrap(), "{out}");
        assert_eq!(f.seen.borrow()[0], "struct P\n    x: i32");
    }

    #[test]
    fn a_derive_call_that_cannot_be_answered_says_why() {
        let e = |src: &str| expand_with(src, &fake()).unwrap_err().msg;
        assert!(e("#[derive~ Nope]\nstruct P\n    x: i32\n").contains("no derive macro `Nope`"));
        assert!(e("#[derive~ hello_macro]\nstruct P\n    x: i32\n").contains("is a function-like proc macro, called `hello_macro~ stream`"));
        assert!(e("#[derive~ Describe]\nfn f$:\n    ()\n").contains("applies to a `struct`, an `enum` or a `union`"));
        assert!(e("#[derive~ Describe, Named]\nstruct P\n    x: i32\n").contains("side by side"));
        assert!(e("#[derive~ Describe]\n").contains("no item beneath it"));
        let none = expand_all_with(crate::lex::lex("#[derive~ Describe]\nstruct P\n    x: i32\n").unwrap(), &[], &NoProcMacros);
        assert!(none.unwrap_err().msg.contains("no derive macro `Describe`"));
    }

    /// A helper no derive on the item declared is left for rustc, which
    /// refuses it -- as Rust does.
    #[test]
    fn a_helper_without_its_derive_is_left_for_rustc() {
        let out = rust("#[derive~ Named]\nstruct P\n    #[describe skip] x: i32\n", &fake());
        assert!(out.contains("#[describe(skip)] x: i32"), "{out}");
    }

    /// The derive's item ends where the next item begins, even one indented
    /// the same inside a module; and a derive is recorded at its attribute.
    #[test]
    fn a_derive_takes_its_item_and_nothing_after_it() {
        let f = fake();
        let src = "mod m\n    #[derive~ Describe]\n    pub struct P\n        x: i32\n    pub struct Q\n        y: i32\n";
        let (_, trace) = expand_with(src, &f).unwrap();
        assert_eq!(f.seen.borrow()[0], "pub struct P\n    x: i32");
        let x = &trace[0];
        assert_eq!(&src[x.call_lo as usize..x.call_hi as usize], "#[derive~ Describe]");
        assert_eq!(x.erased.len(), 1);
    }

    /// Step 2 of `syn`/`quote`: a token the macro copied from its input
    /// carries its origin, and a diagnostic on it lands on the programmer's
    /// token -- Rust keeps a copied token's span. A token the macro made
    /// lands on the call.
    #[test]
    fn a_copied_token_is_traced_to_the_programmers_token() {
        fn wrap(i: &str) -> Result<String, String> {
            // `(` + the input + `)`, with the span lines the runtime writes
            // for every input token (shifted by the `(`).
            let toks = crate::lex::lex(i).unwrap();
            let mut out = format!("({i})\n\u{1e}\n");
            for t in toks {
                out.push_str(&format!("{} {} {} {}\n", t.span.lo + 1, t.span.hi + 1, t.span.lo, t.span.hi));
            }
            Ok(out)
        }
        let f = Fake { macros: vec![("wrap", wrap)], derives: vec![], attributes: vec![], seen: RefCell::new(Vec::new()) };
        let src = "fn main$:\n    let s = wrap~ a + \"b\"\n";
        let (toks, trace) = expand_with(src, &f).unwrap();
        let made: Vec<&Token> = toks.iter().filter(|t| t.ctx == trace[0].ctx).collect();
        let texts: Vec<&str> = made.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["(", "a", "+", "\"b\"", ")"]);
        let back = |t: &Token| {
            let s = at_call(t.span, &trace);
            src[s.lo as usize..s.hi as usize].to_string()
        };
        assert_eq!(back(made[3]), "\"b\"");
        assert_eq!(back(made[1]), "a");
        assert_eq!(back(made[0]), "wrap~");
    }

    /// `quote~` (step 3 of `syn`/`quote`): available where the file imports
    /// `hrs_quote.quote`; the template becomes text pieces and `#name`
    /// values, each value with the column its `#name` stands at.
    #[test]
    fn quote_becomes_pieces() {
        let src = "use hrs_quote.quote\n\nfn f (name: T) -> S:\n    quote~ do:\n        impl H for #name\n            fn h$:\n                g #name\n";
        let (toks, _) = expand_with(src, &fake()).unwrap();
        let text = render_file(&toks);
        assert!(text.contains("quote! (vec! (hrs_quote.Piece.Text \"impl H for \") (hrs_quote.Piece.Value (hrs_quote.to_tokens (&name))) (hrs_quote.Piece.Text \"\\n    fn h$:\\n        g \") (hrs_quote.Piece.Value (hrs_quote.to_tokens (&name))))"), "{text}");
    }

    #[test]
    fn quote_needs_its_import() {
        let without = "fn f$:\n    quote~ do:\n        x\n";
        let (toks, _) = expand_with(without, &fake()).unwrap();
        assert!(render_file(&toks).contains("quote~ do:"));
    }

    /// Step 4: `#( … )*` -- inline with its separator, or as lines when it
    /// begins one; every `#name` inside iterated, side by side.
    #[test]
    fn quote_repeats_inline_or_as_lines() {
        let src = "use hrs_quote.quote\n\nfn f$:\n    quote~ do:\n        g (#(#a: #b),*)\n        struct S\n            #(#a: i32)*\n";
        let (toks, _) = expand_with(src, &fake()).unwrap();
        let text = render_file(&toks);
        assert!(text.contains("(hrs_quote.Piece.Repeat (hrs_quote.rows (vec! (hrs_quote.each (&a)) (hrs_quote.each (&b)))) (vec! (hrs_quote.Sub.Var 0) (hrs_quote.Sub.Text \": \") (hrs_quote.Sub.Var 1)) \",\" false)"), "{text}");
        assert!(text.contains("(hrs_quote.Piece.Repeat (hrs_quote.rows (vec! (hrs_quote.each (&a)))) (vec! (hrs_quote.Sub.Var 0) (hrs_quote.Sub.Text \": i32\")) \"\" true)"), "{text}");
    }

    #[test]
    fn quote_refuses_what_quote_refuses() {
        let e = |body: &str| expand_with(&format!("use hrs_quote.quote\n\nfn f$:\n    quote~ do:\n        {body}\n"), &fake()).unwrap_err().msg;
        assert!(e("#(x)*").contains("needs a `#name` to iterate"));
        assert!(e("#(#x) y").contains("a repetition ends `)*`"));
        assert!(e("#(#(#x)*)*").contains("inside a repetition is not built yet"));
    }

    /// An attribute macro receives the arguments as written, and the item
    /// with all its other attributes -- above and below its own -- as rustc
    /// hands them; what it returns replaces the item.
    #[test]
    fn an_attribute_macro_receives_its_item_whole_and_replaces_it() {
        let f = fake();
        let src = "/// Doc.\n#[allow dead_code]\n#[route~ GET \"/\"]\n#[inline]\nfn index$ -> i32:\n    1\n";
        let out = rust(src, &f);
        assert_eq!(*f.seen.borrow(), ["GET \"/\"\n\u{1e}\n/// Doc.\n#[allow dead_code]\n#[inline]\nfn index$ -> i32:\n    1"]);
        assert!(out.starts_with("const ROUTE: &str = \"GET \\\"/\\\"\";\n/// Doc.\n#[allow(dead_code)]\n#[inline]\nfn index() -> i32 {"), "{out}");
        assert!(!out.contains("route"), "{out}");
    }

    #[test]
    fn an_attribute_call_that_cannot_be_answered_says_why() {
        let e = |src: &str| expand_with(src, &fake()).unwrap_err().msg;
        assert!(e("#[nope~ x]\nfn f$:\n    ()\n").contains("no attribute macro `nope`"));
        assert!(e("#[hello_macro~]\nfn f$:\n    ()\n").contains("function-like proc macro"));
        assert!(e("#[Describe~]\nstruct P\n").contains("is a derive, called `#[derive~ Describe]`"));
        assert!(e("#[route~]\n").contains("none follows it"));
    }

    #[test]
    fn what_nothing_defined_is_named_once() {
        let src = "fn main$:\n    let s = hello_macro~ w\n    let t = nope~ 1\n    let u = nope~ 2\n    let v = a ~ b\n";
        let (toks, _) = expand_with(src, &Fake { macros: vec![], derives: vec![], attributes: vec![], seen: RefCell::new(Vec::new()) }).unwrap();
        assert_eq!(unexpanded(&toks), ["hello_macro", "nope"]);
        let (toks, _) = expand_with(src.replace("nope", "hello_macro").as_str(), &fake()).unwrap();
        assert!(unexpanded(&toks).is_empty());
    }

    #[test]
    fn a_proc_macro_called_with_a_bang_is_the_wrong_mark() {
        let e = expand_with("fn main$:\n    let s = hello_macro! w\n", &fake()).unwrap_err();
        assert!(e.msg.contains("called `hello_macro~`"), "{}", e.msg);
    }
}

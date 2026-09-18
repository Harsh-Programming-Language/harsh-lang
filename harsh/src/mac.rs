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
    let mut sep = None;
    let rep = loop {
        let Some(t) = toks.get(*i) else {
            return err(span, "a repetition ends in `*`, `+` or `?`");
        };
        match t.text.as_str() {
            "*" => break Rep::Star,
            "+" => break Rep::Plus,
            "?" => break Rep::Opt,
            _ if sep.is_none() => {
                // One separator token, and only one.
                sep = Some(t.clone());
                *i += 1;
            }
            _ => {
                return err(
                    t.span,
                    "a repetition takes one separator at most, then `*`, `+` or `?`",
                )
            }
        }
    };
    *i += 1;
    if rep == Rep::Opt {
        if let Some(s) = sep {
            return err(s.span, "`?` repeats at most once, so it takes no separator");
        }
    }
    Ok((sep, rep))
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
        parse_matcher(&sig)
    }

    #[test]
    fn a_capture_and_its_kind() {
        let m = matcher("($x:expr)").expect("parse");
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
            let m = matcher(&format!("($x:{k})")).unwrap_or_else(|e| panic!("{k}: {}", e.msg));
            assert_eq!(bindings(&m)[0].1.name(), k);
        }
    }

    #[test]
    fn rusts_edition_kinds_are_refused_by_name() {
        for k in ["pat_param", "expr_2021"] {
            let e = matcher(&format!("($x:{k})")).err().expect("refused");
            assert!(e.msg.contains("is not a capture kind"), "{}", e.msg);
        }
    }

    #[test]
    fn a_repetition_carries_its_separator_and_operator() {
        let m = matcher("($( ($x:expr) ),*)").expect("parse");
        let Elem::Group { body, .. } = &m[0] else { panic!("{m:?}") };
        let Elem::Repeat { sep, rep, .. } = &body[0] else { panic!("{body:?}") };
        assert_eq!(sep.as_ref().map(|t| t.text.as_str()), Some(","));
        assert_eq!(*rep, Rep::Star);
        // the capture is one level deep
        assert_eq!(bindings(&m)[0].2, 1);
    }

    #[test]
    fn no_separator_is_the_juxtaposed_form() {
        let m = matcher("($( ($x:expr) )*)").expect("parse");
        let Elem::Group { body, .. } = &m[0] else { panic!() };
        let Elem::Repeat { sep, rep, .. } = &body[0] else { panic!() };
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
        let Elem::Group { body, .. } = &m[0] else { panic!() };
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
    for (n, arm) in def.arms.iter().enumerate() {
        let mut b = Bindings::new();
        let mut i = 0;
        match match_elems(&arm.matcher, call, &mut i, &mut b) {
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

/// Match a sequence of matcher elements against the call's tokens from `i`.
fn match_elems(elems: &[Elem], call: &[Token], i: &mut usize, b: &mut Bindings) -> Result<(), Error> {
    match_elems_in(elems, call, i, b, false)
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
                    // A group in a matcher is one *argument*, and a Harsh call
                    // juxtaposes its arguments rather than parenthesising them:
                    // `twice~ 21` passes what `($x:expr)` captures, exactly as
                    // `f 21` passes one argument. So an unparenthesised atom
                    // matches a group whose body is one capture.
                    let stop = atom_end(call, *i);
                    let inner = &call[*i..stop];
                    let mut j = 0;
                    match_elems_in(body, inner, &mut j, b, true)?;
                    if j != inner.len() {
                        return err(inner[j].span, format!("`{}` was not expected here", inner[j].text));
                    }
                    *i = stop;
                    continue;
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
                match_elems_in(body, inner, &mut j, b, true)?;
                if j != inner.len() {
                    return err(inner[j].span, format!("`{}` was not expected here", inner[j].text));
                }
                *i = close + 1;
            }
            Elem::Var { name, kind, .. } => {
                let taken = take_fragment(*kind, call, *i, whole)?;
                b.insert(name.clone(), Binding::One(call[*i..taken].to_vec()));
                *i = taken;
            }
            Elem::Repeat { body, sep, rep, span } => {
                let rest = &elems[n + 1..];
                match_repeat(body, sep.as_ref(), *rep, rest, call, i, b, *span, whole)?;
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
        if match_elems_in(body, call, &mut j, &mut round, whole).is_err() {
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
fn take_fragment(kind: Kind, call: &[Token], at: usize, whole: bool) -> Result<usize, Error> {
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
            while k < call.len() {
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
            // A matcher's outer delimiters are the matcher's own, not part of
            // the pattern -- as in Rust, where `($a:expr)` matches `1 + 2`.
            let mut m = parse_matcher(&toks).expect("matcher");
            if m.len() == 1 {
                let inner = match &m[0] {
                    Elem::Group { body, .. } => Some(body.clone()),
                    _ => None,
                };
                if let Some(b) = inner {
                    m = b;
                }
            }
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
        // the group is the matcher's, so the capture is what was inside it
        assert_eq!(text(&b["a"]), "1 + 2");
        let e = match_call(&d, &call("sub (1)")).err().expect("refused");
        assert!(e.msg.contains("expected `add`"), "{}", e.msg);
    }

    #[test]
    fn groups_bound_a_capture() {
        let d = def("(($a:expr) ($b:expr))");
        let (_, b) = match_call(&d, &call("(1 + 2) (x)")).expect("match");
        assert_eq!(text(&b["a"]), "1 + 2");
        assert_eq!(text(&b["b"]), "x");
    }

    #[test]
    fn a_repetition_binds_one_entry_per_round() {
        let d = def("($( ($x:expr) )*)");
        let (_, b) = match_call(&d, &call("(1) (2) (3)")).expect("match");
        let Binding::Many(rounds) = &b["x"] else { panic!("{:?}", b["x"]) };
        assert_eq!(rounds.len(), 3);
        assert_eq!(text(&rounds[0]), "1");
        assert_eq!(text(&rounds[2]), "3");
    }

    #[test]
    fn a_repetition_with_a_separator() {
        let d = def("($( ($x:expr) ),*)");
        let (_, b) = match_call(&d, &call("(1) , (2)")).expect("match");
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
        let d = def("($( ($x:expr) )* ($last:ident))");
        let (_, b) = match_call(&d, &call("(1) (2) (end)")).expect("match");
        let Binding::Many(rounds) = &b["x"] else { panic!() };
        assert_eq!(rounds.len(), 2, "the last group belongs to `$last`");
        assert_eq!(text(&b["last"]), "end");
    }

    #[test]
    fn arms_are_tried_in_order() {
        let d = def("(($a:ident))\n(($a:expr))");
        let (arm, _) = match_call(&d, &call("(x)")).expect("match");
        assert!(std::ptr::eq(arm, &d.arms[0]), "the first arm that fits wins");
        let (arm, _) = match_call(&d, &call("(1 + 2)")).expect("match");
        assert!(std::ptr::eq(arm, &d.arms[1]));
    }

    #[test]
    fn no_arm_matching_says_why_each_failed() {
        let d = def("(add ($a:expr))\n(sub ($a:expr))");
        let e = match_call(&d, &call("mul (1)")).err().expect("refused");
        assert!(e.msg.contains("no arm of `m~` matches"), "{}", e.msg);
        assert!(e.msg.contains("arm 1:") && e.msg.contains("arm 2:"), "{}", e.msg);
    }

    #[test]
    fn a_capture_keeps_the_callers_tokens_and_context() {
        let d = def("(($a:expr))");
        let (_, b) = match_call(&d, &call("(a / 10)")).expect("match");
        // the caller's tokens, with the caller's context (root, here)
        assert!(b["a"].tokens().iter().all(|t| t.ctx == 0));
        assert_eq!(text(&b["a"]), "a / 10");
    }

    #[test]
    fn a_kind_refuses_what_it_is_not() {
        let d = def("(($a:ident))");
        let e = match_call(&d, &call("(1)")).err().expect("refused");
        assert!(e.msg.contains("expected an identifier"), "{}", e.msg);
    }

    #[test]
    fn tt_takes_a_token_or_a_whole_group() {
        let d = def("(($a:tt) ($b:tt))");
        let (_, b) = match_call(&d, &call("(x) ((1 + 2))")).expect("match");
        assert_eq!(text(&b["a"]), "x");
        assert_eq!(text(&b["b"]), "( 1 + 2 )");
    }

    #[test]
    fn a_dotted_path_is_one_capture() {
        let d = def("(($p:path))");
        let (_, b) = match_call(&d, &call("(std.collections.HashMap)")).expect("match");
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
    for l in &lines {
        expand_line(l, b, ctx, base, &mut out)?;
    }
    Ok(out)
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
    if let Some((body, sep, rounds)) = whole_line_repetition(line, b)? {
        for r in 0..rounds {
            let mut toks = Vec::new();
            emit_tokens(&body, b, ctx, Some(r), &mut toks)?;
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
    emit_tokens(&line.toks, b, ctx, None, &mut toks)?;
    out.push(OutLine { indent: line.indent.saturating_sub(base), toks });
    Ok(())
}

/// Is this line exactly `$( … )sep rep`? Then it is a repeated *line*.
#[allow(clippy::type_complexity)]
fn whole_line_repetition(
    line: &OutLine,
    b: &Bindings,
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
    let rounds = rounds_for(&inner, b)?;
    Ok(Some((inner, sep, rounds)))
}

/// How many rounds a repetition runs: the length of the bindings it uses.
fn rounds_for(body: &[Token], b: &Bindings) -> Result<usize, Error> {
    let mut found: Option<(String, usize)> = None;
    let mut i = 0;
    while i < body.len() {
        if body[i].text == "$" {
            if let Some(name) = body.get(i + 1) {
                if let Some(Binding::Many(rs)) = b.get(&name.text) {
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
    round: Option<usize>,
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
                let (sep, skip) = match tail.first().map(|x| x.text.as_str()) {
                    Some("*") | Some("+") | Some("?") => (None, 1),
                    Some(_) => (tail.first().cloned(), 2),
                    None => (None, 0),
                };
                let n = rounds_for(inner, b)?;
                for r in 0..n {
                    emit_tokens(inner, b, ctx, Some(r), out)?;
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
                let picked = match (binding, round) {
                    (Binding::One(v), _) => v.clone(),
                    (Binding::Many(rs), Some(r)) => rs
                        .get(r)
                        .map(|x| x.tokens().to_vec())
                        .unwrap_or_default(),
                    (Binding::Many(_), None) => {
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
        let mut m = parse_matcher(&toks(src)).expect("matcher");
        if m.len() == 1 {
            let inner = match &m[0] {
                Elem::Group { body, .. } => Some(body.clone()),
                _ => None,
            };
            if let Some(x) = inner {
                m = x;
            }
        }
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
        assert_eq!(out, "let x = 1 + 2");
    }

    #[test]
    fn a_repetition_on_its_own_line_yields_one_line_per_round() {
        let out = expand(
            "(($v:ident) $( ($x:expr) )*)",
            "$( $v <- push $x )*",
            "(v) (1) (2) (3)",
        );
        assert_eq!(out, "v <- push 1\nv <- push 2\nv <- push 3");
    }

    #[test]
    fn a_repetition_inside_a_line_yields_entries_in_place() {
        let out = expand("($( ($x:expr) )*)", "let xs = [$( $x ),*]", "(1) (2) (3)");
        assert_eq!(out, "let xs = [1, 2, 3]");
    }

    #[test]
    fn the_transcribers_indentation_is_kept() {
        let out = expand("(($a:expr))", "do:\n    let x = $a\n    x + x", "(7)");
        assert_eq!(out, "do:\n    let x = 7\n    x + x");
    }

    #[test]
    fn the_expansion_lands_at_the_callers_column() {
        let arm = Arm { matcher: matcher_of("(($a:expr))"), body: toks("do:\n    let x = $a"), span: Span::new(0, 0) };
        let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
        let (arm, b) = match_call(&def, &toks("(7)")).expect("match");
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
        let (arm, b) = match_call(&def, &toks("(1)")).expect("match");
        let e = substitute(&arm.body, &b, 1).err().expect("refused");
        assert!(e.msg.contains("was not captured"), "{}", e.msg);
    }

    #[test]
    fn a_repeated_capture_used_outside_a_repetition_is_refused() {
        let arm = Arm { matcher: matcher_of("($( ($x:expr) )*)"), body: toks("let a = $x"), span: Span::new(0, 0) };
        let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
        let (arm, b) = match_call(&def, &toks("(1) (2)")).expect("match");
        let e = substitute(&arm.body, &b, 1).err().expect("refused");
        assert!(e.msg.contains("used inside one"), "{}", e.msg);
    }

    #[test]
    fn two_captures_in_one_repetition_must_agree_in_length() {
        let out = expand(
            "($( ($n:ident) ($t:ty) )*)",
            "Config\\\n    $( $n: $t )*",
            "(host) (String) (port) (u16)",
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
    for l in lines.iter() {
        let sig: Vec<&Token> = l.toks.iter().filter(|t| !t.is_comment()).collect();
        for (i, t) in sig.iter().enumerate() {
            let binds = t.is_kw("let") || t.is_kw("const") || t.is_kw("static");
            if !binds {
                continue;
            }
            // `let mut x` / `let x`, and only when the name is the macro's own.
            let mut k = i + 1;
            if sig.get(k).map_or(false, |x| x.is_kw("mut")) {
                k += 1;
            }
            if let Some(name) = sig.get(k) {
                if name.kind == Tk::Ident && name.ctx == ctx && !introduced.contains(&name.text) {
                    introduced.push(name.text.clone());
                }
            }
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
        let mut m = parse_matcher(&toks(src)).expect("matcher");
        if m.len() == 1 {
            let inner = match &m[0] {
                Elem::Group { body, .. } => Some(body.clone()),
                _ => None,
            };
            if let Some(x) = inner {
                m = x;
            }
        }
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
        let out = expand_hygienic("(($x:expr))", "let tmp = $x\ntmp + tmp", "(1)", &["other"]);
        assert_eq!(out, "let tmp = 1\ntmp + tmp");
    }

    #[test]
    fn a_clash_moves_the_macros_local_not_the_callers() {
        // The caller has `tmp`, and passes an expression that reads it. The
        // macro's own `tmp` is the one that moves; the caller's stays.
        let out = expand_hygienic("(($x:expr))", "let tmp = $x\ntmp + tmp", "(tmp + 1)", &["tmp"]);
        assert_eq!(out, "let tmp__1 = tmp + 1\ntmp__1 + tmp__1");
    }

    #[test]
    fn the_new_spelling_avoids_everything_in_scope() {
        let out = expand_hygienic("(($x:expr))", "let tmp = $x\ntmp", "(1)", &["tmp", "tmp__1"]);
        assert_eq!(out, "let tmp__2 = 1\ntmp__2");
    }

    #[test]
    fn a_name_the_caller_passed_in_is_never_respelled() {
        // `$n:ident` is the caller's: `declare~ count 0` must bind the
        // caller's `count`, clash or no clash.
        let out = expand_hygienic(
            "(($n:ident) ($v:expr))",
            "let $n = $v",
            "(count) (0)",
            &["count"],
        );
        assert_eq!(out, "let count = 0");
    }

    #[test]
    fn several_locals_each_get_their_own_spelling() {
        let out = expand_hygienic(
            "(($x:expr))",
            "let a = $x\nlet b = a\nb + a",
            "(1)",
            &["a", "b"],
        );
        assert_eq!(out, "let a__1 = 1\nlet b__1 = a__1\nb__1 + a__1");
    }

    #[test]
    fn a_mutable_local_is_found_too() {
        let out = expand_hygienic("(($x:expr))", "let mut v = $x\nv", "(1)", &["v"]);
        assert_eq!(out, "let mut v__1 = 1\nv__1");
    }

    #[test]
    fn the_renames_are_reported_for_hrs_expand() {
        let arm = Arm { matcher: matcher_of("(($x:expr))"), body: toks("let tmp = $x"), span: Span::new(0, 0) };
        let def = Def { name: "m".into(), arms: vec![arm], span: Span::new(0, 0) };
        let (arm, b) = match_call(&def, &toks("(1)")).expect("match");
        let mut lines = substitute(&arm.body, &b, 3).expect("substitute");
        let done = respell(&mut lines, 3, &["tmp".to_string()]);
        assert_eq!(done, vec![("tmp".to_string(), "tmp__1".to_string())]);
    }
}

// ---------------------------------------------------------------------------
// Stage 5: expansion in the pipeline
// ---------------------------------------------------------------------------

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
        if toks[i].kind != Tk::Open('(') {
            return err(toks[i].span, "an arm begins with its matcher, `( … ) => do:`");
        }
        let arm_col = line_col(toks, i);
        let close = match matching_close(toks, i) {
            Some(c) => c,
            None => return err(toks[i].span, "this matcher is never closed"),
        };
        let mut m = parse_matcher(&toks[i..=close])?;
        if m.len() == 1 {
            let inner = match &m[0] {
                Elem::Group { body, .. } => Some(body.clone()),
                _ => None,
            };
            if let Some(b) = inner {
                m = b;
            }
        }
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
fn find_call(toks: &[Token], defs: &[Def], from: usize) -> Option<(usize, usize, usize)> {
    let mut i = from;
    while i + 1 < toks.len() {
        let marked = toks[i].kind == Tk::Ident
            && toks[i + 1].text == "~"
            && toks[i].span.hi == toks[i + 1].span.lo;
        if marked {
            if let Some(d) = defs.iter().position(|d| d.name == toks[i].text) {
                // The call's arguments run to the end of the logical line, or
                // to the close of the group the call sits in -- `(sum~ 1 2)`
                // ends at its own `)`, which belongs to the caller.
                let mut end = i + 2;
                let mut depth = 0i32;
                while end < toks.len() && toks[end].line_start.is_none() {
                    match toks[end].kind {
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
                return Some((d, i, end));
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
pub fn expand_all(toks: Vec<Token>, _taken: &[String]) -> Result<Vec<Token>, Error> {
    let (defs, def_spans) = collect_defs(&toks)?;
    if defs.is_empty() {
        return Ok(toks);
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
    check_marks(&out, &defs)?;

    let mut ctx = 0u32;
    for pass in 0..RECURSION_LIMIT {
        let Some((d, at, end)) = find_call(&out, &defs, 0) else {
            return Ok(out);
        };
        let _ = pass;
        ctx += 1;
        let def = &defs[d];
        let call: Vec<Token> = out[at + 2..end].to_vec();
        let (arm, b) = match_call(def, &call)?;
        let mut lines = substitute(&arm.body, &b, ctx)?;
        respell(&mut lines, ctx, taken);
        let col = line_col(&out, at);
        let mut spliced: Vec<Token> = Vec::new();
        for (n, l) in lines.iter().enumerate() {
            for (k, t) in l.toks.iter().enumerate() {
                let mut t = t.clone();
                t.line_start = if k == 0 && n > 0 { Some(col + l.indent) } else { None };
                // The emitter copies source whitespace between spans; an
                // expanded token's span points into the definition, which is
                // no longer in the file, so it is written as synthetic.
                //
                // Except `$`, Harsh's own call marker: `juxt` turns `f$` into
                // `f()` only for a `$` the author wrote, and an expansion's
                // `$` was written by the author -- of the macro.
                t.synthetic = !(t.kind == Tk::Punct && t.text == "$");
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
            "macro expansion went {RECURSION_LIMIT} passes deep and is still going; raise it with `#![recursion_limit = \"…\"]` if it is not a loop"
        ),
    )
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
fn check_marks(toks: &[Token], defs: &[Def]) -> Result<(), Error> {
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
    }
    Ok(())
}

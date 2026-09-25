//! Tokens, as `proc_macro` and `proc-macro2` give them to a Rust macro --
//! `TokenTree` with `Group`, `Ident`, `Punct` and `Literal`, `Delimiter`,
//! `Spacing`, `Span` -- plus the one thing Harsh needs that Rust does not:
//! **where each token stands** (`Place`). Rust's tokens ignore whitespace;
//! Harsh's indentation is grammar, and `f$`, `m!`, `a[1]` are tight on
//! purpose. So every tree knows whether it begins a line and at what
//! indentation, or touches the token before it, or is spaced from it (the
//! user's ruling, 2026-09-24: layout lives in the tokens).
//!
//! Streams are lexed by the transpiler's own lexer (`harsh_lang::lex`),
//! never by a second parser of Harsh.

use std::fmt;
use std::str::FromStr;

use harsh_lang::lex::{self, Tk, Token};

/// Where a piece of a stream came from: byte offsets into the stream's text
/// as the macro received it. `hrs` knows where each of those bytes came from
/// in the programmer's file, so a span can always be taken back there. A
/// token a macro makes itself has `Span::call_site()`, as in Rust.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Span {
    lo: u32,
    hi: u32,
}

impl Span {
    /// The call itself: what Rust gives a token built rather than received.
    pub fn call_site() -> Span {
        Span::default()
    }
    /// A span over bytes `lo..hi` of the stream's text.
    #[doc(hidden)]
    pub fn new_at(lo: u32, hi: u32) -> Span {
        Span { lo, hi }
    }
    pub fn lo(&self) -> u32 {
        self.lo
    }
    pub fn hi(&self) -> u32 {
        self.hi
    }
    /// A span covering both, as `Span::join` does in Rust.
    pub fn join(&self, other: Span) -> Span {
        if *self == Span::call_site() {
            return other;
        }
        if other == Span::call_site() {
            return *self;
        }
        Span { lo: self.lo.min(other.lo), hi: self.hi.max(other.hi) }
    }
}

/// Where a token stands: Harsh's addition to Rust's token model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// It begins a line, indented this far past the stream's least-indented
    /// line.
    Line(usize),
    /// It touches the token before it: `f$`, `m!`, `a[1]`, `x.0`.
    Tight,
    /// A space before it.
    Spaced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delimiter {
    Parenthesis,
    Brace,
    Bracket,
    None,
}

impl Delimiter {
    fn chars(self) -> (&'static str, &'static str) {
        match self {
            Delimiter::Parenthesis => ("(", ")"),
            Delimiter::Brace => ("{", "}"),
            Delimiter::Bracket => ("[", "]"),
            Delimiter::None => ("", ""),
        }
    }
}

/// As in Rust: `Joint` when the next token is a punctuation character
/// touching this one (`<` of `<-`), `Alone` otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spacing {
    Alone,
    Joint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ident {
    name: String,
    span: Span,
    place: Place,
}

impl Ident {
    pub fn new(name: &str, span: Span) -> Ident {
        Ident { name: name.to_string(), span, place: Place::Spaced }
    }
    pub fn span(&self) -> Span {
        self.span
    }
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl PartialEq<str> for Ident {
    fn eq(&self, other: &str) -> bool {
        self.name == other
    }
}

impl PartialEq<&str> for Ident {
    fn eq(&self, other: &&str) -> bool {
        self.name == *other
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Punct {
    ch: char,
    spacing: Spacing,
    span: Span,
    place: Place,
}

impl Punct {
    pub fn new(ch: char, spacing: Spacing) -> Punct {
        Punct { ch, spacing, span: Span::call_site(), place: Place::Spaced }
    }
    pub fn as_char(&self) -> char {
        self.ch
    }
    pub fn spacing(&self) -> Spacing {
        self.spacing
    }
    pub fn span(&self) -> Span {
        self.span
    }
}

/// A literal, kept as written: `"text"`, `42`, `1.5`, `'c'`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Literal {
    repr: String,
    span: Span,
    place: Place,
}

impl Literal {
    fn made(repr: String) -> Literal {
        Literal { repr, span: Span::call_site(), place: Place::Spaced }
    }
    /// A string literal, escaped as Rust's `Literal::string` does.
    pub fn string(s: &str) -> Literal {
        Literal::made(format!("{s:?}"))
    }
    pub fn character(c: char) -> Literal {
        Literal::made(format!("{c:?}"))
    }
    pub fn i64_unsuffixed(n: i64) -> Literal {
        Literal::made(n.to_string())
    }
    pub fn u64_unsuffixed(n: u64) -> Literal {
        Literal::made(n.to_string())
    }
    pub fn usize_unsuffixed(n: usize) -> Literal {
        Literal::made(n.to_string())
    }
    pub fn f64_unsuffixed(n: f64) -> Literal {
        let s = n.to_string();
        Literal::made(if s.contains('.') || s.contains('e') { s } else { format!("{s}.0") })
    }
    pub fn span(&self) -> Span {
        self.span
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.repr)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    delimiter: Delimiter,
    stream: TokenStream,
    span: Span,
    place: Place,
    /// Where the closing delimiter stands: on its own line, or not.
    close: Place,
}

impl Group {
    pub fn new(delimiter: Delimiter, stream: TokenStream) -> Group {
        Group { delimiter, stream, span: Span::call_site(), place: Place::Spaced, close: Place::Tight }
    }
    pub fn delimiter(&self) -> Delimiter {
        self.delimiter
    }
    pub fn stream(&self) -> TokenStream {
        self.stream.clone()
    }
    pub fn span(&self) -> Span {
        self.span
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenTree {
    Group(Group),
    Ident(Ident),
    Punct(Punct),
    Literal(Literal),
}

impl TokenTree {
    pub fn span(&self) -> Span {
        match self {
            TokenTree::Group(g) => g.span,
            TokenTree::Ident(i) => i.span,
            TokenTree::Punct(p) => p.span,
            TokenTree::Literal(l) => l.span,
        }
    }
    /// Where this tree stands: Harsh's addition (see `Place`).
    pub fn place(&self) -> Place {
        match self {
            TokenTree::Group(g) => g.place,
            TokenTree::Ident(i) => i.place,
            TokenTree::Punct(p) => p.place,
            TokenTree::Literal(l) => l.place,
        }
    }
    /// The tree alone, rendered.
    pub fn to_string_tree(&self) -> String {
        TokenStream::from(self.clone()).to_string()
    }
    /// As in Rust: give a token a span -- an input token's, so that an error
    /// built from it (`compile_error!`) points at the programmer's code.
    pub fn set_span(&mut self, span: Span) {
        match self {
            TokenTree::Group(g) => g.span = span,
            TokenTree::Ident(i) => i.span = span,
            TokenTree::Punct(p) => p.span = span,
            TokenTree::Literal(l) => l.span = span,
        }
    }
    pub fn set_place(&mut self, place: Place) {
        match self {
            TokenTree::Group(g) => g.place = place,
            TokenTree::Ident(i) => i.place = place,
            TokenTree::Punct(p) => p.place = place,
            TokenTree::Literal(l) => l.place = place,
        }
    }
}

impl From<Ident> for TokenTree {
    fn from(i: Ident) -> Self {
        TokenTree::Ident(i)
    }
}
impl From<Punct> for TokenTree {
    fn from(p: Punct) -> Self {
        TokenTree::Punct(p)
    }
}
impl From<Literal> for TokenTree {
    fn from(l: Literal) -> Self {
        TokenTree::Literal(l)
    }
}
impl From<Group> for TokenTree {
    fn from(g: Group) -> Self {
        TokenTree::Group(g)
    }
}

/// A macro's input or output: a sequence of token trees, with their layout.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TokenStream {
    trees: Vec<TokenTree>,
}

impl TokenStream {
    pub fn new() -> Self {
        TokenStream::default()
    }
    /// Whether the stream holds no tokens.
    pub fn is_empty(&self) -> bool {
        self.trees.is_empty()
    }
    pub fn iter(&self) -> std::slice::Iter<'_, TokenTree> {
        self.trees.iter()
    }
    pub fn trees(&self) -> &[TokenTree] {
        &self.trees
    }
}

impl IntoIterator for TokenStream {
    type Item = TokenTree;
    type IntoIter = std::vec::IntoIter<TokenTree>;
    fn into_iter(self) -> Self::IntoIter {
        self.trees.into_iter()
    }
}

impl FromIterator<TokenTree> for TokenStream {
    fn from_iter<I: IntoIterator<Item = TokenTree>>(iter: I) -> Self {
        TokenStream { trees: iter.into_iter().collect() }
    }
}

impl Extend<TokenTree> for TokenStream {
    fn extend<I: IntoIterator<Item = TokenTree>>(&mut self, iter: I) {
        self.trees.extend(iter);
    }
}

impl Extend<TokenStream> for TokenStream {
    fn extend<I: IntoIterator<Item = TokenStream>>(&mut self, iter: I) {
        for s in iter {
            self.trees.extend(s.trees);
        }
    }
}

impl From<TokenTree> for TokenStream {
    fn from(t: TokenTree) -> Self {
        TokenStream { trees: vec![t] }
    }
}

/// Text that does not lex as Harsh: an unterminated string, a stray quote.
#[derive(Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// The message itself, so that `parse$ <- unwrap$` on text that does not
/// lex panics with something a person can read.
impl fmt::Debug for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (bytes {}..{} of the text)", self.message, self.span.lo, self.span.hi)
    }
}

impl std::error::Error for LexError {}

impl FromStr for TokenStream {
    type Err = LexError;
    /// Lex Harsh text with the transpiler's own lexer. As in Rust, tokens
    /// made this way have `Span::call_site()`: they come from the macro, not
    /// from the programmer's code. Whether the result *lays out* as Harsh is
    /// decided by `hrs` when it is spliced, where an error can name the call.
    fn from_str(text: &str) -> Result<Self, LexError> {
        let mut s = TokenStream::lex_input(text)?;
        s.forget_spans();
        Ok(s)
    }
}

impl TokenStream {
    /// The stream as the runner receives it: each token's span is its place
    /// in `text`, which `hrs` can take back to the programmer's file. Only
    /// the runner calls this; a macro's own `parse` gives `call_site` spans.
    #[doc(hidden)]
    pub fn lex_input(text: &str) -> Result<TokenStream, LexError> {
        let toks = lex::lex(text).map_err(|e| LexError { message: e.msg, span: Span { lo: e.span.lo, hi: e.span.hi } })?;
        let least = toks.iter().filter_map(|t| t.line_start).min().unwrap_or(0);
        let mut at = 0;
        let (trees, _) = build(&toks, &mut at, least, None);
        Ok(TokenStream { trees })
    }

    /// Every span in the stream, groups included, replaced by what `f`
    /// makes of it -- how `hrs_quote` gives an interpolated token back the
    /// span it had.
    pub fn map_spans(&mut self, f: &mut dyn FnMut(Span) -> Span) {
        for t in self.trees.iter_mut() {
            let s = f(t.span());
            t.set_span(s);
            if let TokenTree::Group(g) = t {
                g.stream.map_spans(f);
            }
        }
    }

    fn forget_spans(&mut self) {
        for t in self.trees.iter_mut() {
            match t {
                TokenTree::Group(g) => {
                    g.span = Span::call_site();
                    g.stream.forget_spans();
                }
                TokenTree::Ident(i) => i.span = Span::call_site(),
                TokenTree::Punct(p) => p.span = Span::call_site(),
                TokenTree::Literal(l) => l.span = Span::call_site(),
            }
        }
    }
}

fn place_of(toks: &[Token], k: usize, least: usize) -> Place {
    if let Some(c) = toks[k].line_start {
        return Place::Line(c.saturating_sub(least));
    }
    match k.checked_sub(1).map(|p| &toks[p]) {
        Some(p) if p.span.hi == toks[k].span.lo => Place::Tight,
        _ => Place::Spaced,
    }
}

fn span_of(t: &Token) -> Span {
    Span { lo: t.span.lo, hi: t.span.hi }
}

/// Trees from `toks[*at..]` up to the close matching `until`; returns them
/// and, when a close was found, where it stands.
fn build(toks: &[Token], at: &mut usize, least: usize, until: Option<char>) -> (Vec<TokenTree>, Option<Place>) {
    let mut out: Vec<TokenTree> = Vec::new();
    while *at < toks.len() {
        let k = *at;
        let t = &toks[k];
        let place = place_of(toks, k, least);
        *at += 1;
        match t.kind {
            Tk::Close(c) if Some(c) == until => return (out, Some(place)),
            Tk::Open(c) => {
                let delimiter = match c {
                    '(' => Delimiter::Parenthesis,
                    '[' => Delimiter::Bracket,
                    _ => Delimiter::Brace,
                };
                let close_char = match c {
                    '(' => ')',
                    '[' => ']',
                    _ => '}',
                };
                let (inner, close) = build(toks, at, least, Some(close_char));
                let hi = toks.get(*at - 1).map_or(t.span.hi, |c| c.span.hi);
                out.push(TokenTree::Group(Group {
                    delimiter,
                    stream: TokenStream { trees: inner },
                    span: Span { lo: t.span.lo, hi },
                    place,
                    close: close.unwrap_or(Place::Tight),
                }));
            }
            Tk::Ident => out.push(TokenTree::Ident(Ident { name: t.text.clone(), span: span_of(t), place })),
            Tk::Int | Tk::Float | Tk::Str | Tk::Char => {
                out.push(TokenTree::Literal(Literal { repr: t.text.clone(), span: span_of(t), place }))
            }
            // `'a`: a joint `'` and an identifier, as Rust gives it.
            Tk::Lifetime => {
                let s = span_of(t);
                out.push(TokenTree::Punct(Punct { ch: '\'', spacing: Spacing::Joint, span: s, place }));
                out.push(TokenTree::Ident(Ident { name: t.text.trim_start_matches('\'').to_string(), span: s, place: Place::Tight }));
            }
            // `.0`: a `.` and the index, as Rust gives `x.0`.
            Tk::TupleIdx => {
                let s = span_of(t);
                out.push(TokenTree::Punct(Punct { ch: '.', spacing: Spacing::Alone, span: s, place }));
                out.push(TokenTree::Literal(Literal { repr: t.text.trim_start_matches('.').to_string(), span: s, place: Place::Tight }));
            }
            // A doc comment is the attribute Rust makes of it,
            // `#[doc = " text"]`, which Harsh accepts; it is written back as
            // the comment (`render`).
            Tk::LineComment | Tk::BlockComment => {
                if let Some(doc) = t.text.strip_prefix("///") {
                    let s = span_of(t);
                    let body = TokenStream {
                        trees: vec![
                            TokenTree::Ident(Ident { name: "doc".into(), span: s, place: Place::Tight }),
                            TokenTree::Punct(Punct { ch: '=', spacing: Spacing::Alone, span: s, place: Place::Spaced }),
                            TokenTree::Literal(Literal { repr: format!("{doc:?}"), span: s, place: Place::Spaced }),
                        ],
                    };
                    out.push(TokenTree::Punct(Punct { ch: '#', spacing: Spacing::Alone, span: s, place }));
                    out.push(TokenTree::Group(Group {
                        delimiter: Delimiter::Bracket,
                        stream: body,
                        span: s,
                        place: Place::Tight,
                        close: Place::Tight,
                    }));
                }
            }
            // Every other token is punctuation: one `Punct` per character,
            // joint within the token, as Rust splits `<-` or `=>`.
            _ => {
                let chars: Vec<char> = t.text.chars().collect();
                for (j, ch) in chars.iter().enumerate() {
                    out.push(TokenTree::Punct(Punct {
                        ch: *ch,
                        spacing: if j + 1 < chars.len() { Spacing::Joint } else { Spacing::Alone },
                        span: span_of(t),
                        place: if j == 0 { place } else { Place::Tight },
                    }));
                }
            }
        }
    }
    (out, None)
}

impl fmt::Display for TokenStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        render(&self.trees, &mut out, &mut Vec::new());
        f.write_str(&out)
    }
}

impl TokenStream {
    /// The text, and for each piece of it that came from the input -- a
    /// token whose span is not `call_site` -- its byte range in the text and
    /// its span. This is what lets `hrs` put a copied token back on the
    /// programmer's code, as Rust keeps a copied token's span.
    pub fn to_string_with_spans(&self) -> (String, Vec<(u32, u32, Span)>) {
        let mut out = String::new();
        let mut spans = Vec::new();
        render(&self.trees, &mut out, &mut spans);
        (out, spans)
    }
}

fn place_into(place: Place, out: &mut String) {
    match place {
        // A stream starts where it is put: its first token's own place --
        // the indentation it had in the input -- is not written (a field
        // name rendered alone came out as "    x"; found 2026-09-24).
        Place::Line(n) => {
            if !out.is_empty() {
                out.push('\n');
                out.push_str(&" ".repeat(n));
            }
        }
        Place::Tight => {}
        Place::Spaced => {
            if !out.is_empty() && !out.ends_with(' ') && !out.ends_with('\n') {
                out.push(' ');
            }
        }
    }
}

/// `#[doc = "text"]`, the shape a doc comment was lexed into.
fn doc_text(trees: &[TokenTree], k: usize) -> Option<String> {
    let (TokenTree::Punct(p), Some(TokenTree::Group(g))) = (&trees[k], trees.get(k + 1)) else { return None };
    if p.ch != '#' || g.delimiter != Delimiter::Bracket {
        return None;
    }
    match g.stream.trees.as_slice() {
        [TokenTree::Ident(d), TokenTree::Punct(e), TokenTree::Literal(l)] if d.name == "doc" && e.ch == '=' => {
            let inner = l.repr.strip_prefix('"')?.strip_suffix('"')?;
            // Undo the escaping `{:?}` did.
            let text: String = inner.replace("\\\"", "\"").replace("\\\\", "\\");
            Some(text)
        }
        _ => None,
    }
}

fn render(trees: &[TokenTree], out: &mut String, spans: &mut Vec<(u32, u32, Span)>) {
    let mut k = 0;
    while k < trees.len() {
        if let Some(text) = doc_text(trees, k) {
            place_into(trees[k].place(), out);
            out.push_str("///");
            out.push_str(&text);
            k += 2;
            continue;
        }
        let t = &trees[k];
        place_into(t.place(), out);
        let lo = out.len() as u32;
        let note = |out: &String, spans: &mut Vec<(u32, u32, Span)>, lo: u32, span: Span| {
            if span != Span::call_site() {
                spans.push((lo, out.len() as u32, span));
            }
        };
        match t {
            TokenTree::Ident(i) => {
                out.push_str(&i.name);
                note(out, spans, lo, i.span);
            }
            TokenTree::Literal(l) => {
                out.push_str(&l.repr);
                note(out, spans, lo, l.span);
            }
            TokenTree::Punct(p) => {
                out.push(p.ch);
                note(out, spans, lo, p.span);
            }
            TokenTree::Group(g) => {
                let (open, close) = g.delimiter.chars();
                out.push_str(open);
                note(out, spans, lo, g.span);
                render(&g.stream.trees, out, spans);
                place_into(g.close, out);
                let at = out.len() as u32;
                out.push_str(close);
                note(out, spans, at, g.span);
            }
        }
        k += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(text: &str) -> String {
        text.parse::<TokenStream>().unwrap().to_string()
    }

    /// Text in the form `hrs` hands a macro (C2) comes back byte for byte:
    /// tight marks, spacing, indentation, groups over lines, doc comments.
    #[test]
    fn a_stream_renders_back_as_it_was_written() {
        for text in [
            "world",
            "Harsh readers",
            "a, b\n    c <- d$\n        e",
            "println! \"{}\" (hello_macro~ world)",
            "let s: Vec<i32> = hrs_std.g~ x for x in 0..3 <- collect$",
            "a[1, ..] .* b |> relu<> |> f64.sqrt<>",
            "/// A point.\n#[allow dead_code]\npub struct P\n    #[describe skip] x: i32\n    y: i32",
            "impl P\n    pub fn describe$ -> &'static str:\n        \"P { x, y }\"",
            "match x\\\n    Some v => v.0\n    None => 0",
            "f (1 + 2) [3, 4] {5}",
            "vec! (\n    1\n    2\n)",
        ] {
            assert_eq!(round(text), text);
        }
    }

    #[test]
    fn trees_are_rusts_shapes() {
        let s: TokenStream = "f$ <- g (a, 'x') 'b".parse().unwrap();
        let kinds: Vec<String> = s
            .iter()
            .map(|t| match t {
                TokenTree::Ident(i) => format!("I:{i}"),
                TokenTree::Punct(p) => format!("P:{}{}", p.as_char(), if p.spacing() == Spacing::Joint { "+" } else { "" }),
                TokenTree::Literal(l) => format!("L:{l}"),
                TokenTree::Group(g) => format!("G:{:?}:{}", g.delimiter(), g.stream().trees().len()),
            })
            .collect();
        assert_eq!(kinds, ["I:f", "P:$", "P:<+", "P:-", "I:g", "G:Parenthesis:3", "P:'+", "I:b"]);
        assert_eq!(s.trees()[1].place(), Place::Tight);
        assert_eq!(s.trees()[2].place(), Place::Spaced);
    }

    /// Spans are offsets into the stream's text.
    #[test]
    fn spans_point_into_the_text() {
        let text = "let x = y\n    z";
        let s = TokenStream::lex_input(text).unwrap();
        let spans: Vec<&str> = s.iter().map(|t| &text[t.span().lo() as usize..t.span().hi() as usize]).collect();
        assert_eq!(spans, ["let", "x", "=", "y", "z"]);
        assert_eq!(s.trees()[4].place(), Place::Line(4));
    }

    /// A token copied from the input keeps its span through rendering; a
    /// token the macro made has none to report.
    #[test]
    fn rendering_reports_where_copied_tokens_came_from() {
        let input = TokenStream::lex_input("struct Point").unwrap();
        let name = input.trees()[1].clone();
        let out: TokenStream = [TokenTree::from(Ident::new("impl", Span::call_site())), name].into_iter().collect();
        let (text, spans) = out.to_string_with_spans();
        assert_eq!(text, "impl Point");
        assert_eq!(spans, [(5, 10, Span { lo: 7, hi: 12 })]);
    }

    /// A token rendered on its own is its text, whatever its place was.
    #[test]
    fn a_token_alone_is_its_text() {
        let s = TokenStream::lex_input("struct P\n    x: i32").unwrap();
        let x = s.iter().find(|t| t.to_string_tree().trim() == "x").unwrap();
        assert_eq!(x.place(), Place::Line(4));
        assert_eq!(x.to_string_tree(), "x");
    }

    /// As in Rust: what a macro parses itself comes from the macro, and
    /// has no span in the programmer's code to report.
    #[test]
    fn a_macros_own_parse_has_call_site_spans() {
        let s: TokenStream = "impl P\n    x".parse().unwrap();
        assert!(s.iter().all(|t| t.span() == Span::call_site()));
        assert!(s.to_string_with_spans().1.is_empty());
    }

    #[test]
    fn a_doc_comment_is_rusts_doc_attribute() {
        let s: TokenStream = "/// Hello.\nstruct P".parse().unwrap();
        match (&s.trees()[0], &s.trees()[1]) {
            (TokenTree::Punct(p), TokenTree::Group(g)) => {
                assert_eq!(p.as_char(), '#');
                assert_eq!(g.stream().to_string(), "doc = \" Hello.\"");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn built_tokens_render_spaced_and_text_that_does_not_lex_is_an_error() {
        let s: TokenStream = [
            TokenTree::from(Ident::new("impl", Span::call_site())),
            TokenTree::from(Ident::new("P", Span::call_site())),
            TokenTree::from(Literal::string("a \"q\"")),
        ]
        .into_iter()
        .collect();
        assert_eq!(s.to_string(), "impl P \"a \\\"q\\\"\"");
        assert!("\"never closed".parse::<TokenStream>().is_err());
    }
}

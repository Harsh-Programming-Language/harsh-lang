//! Harsh's `syn` (the user's ruling, 2026-09-24; `docs/dev/SYN-QUOTE-PLAN.md`,
//! step 5): a Harsh item -- a `struct`, an `enum`, a `union` -- parsed into
//! the shapes Rust's `syn` gives a derive, `DeriveInput` first.
//!
//! The tokens are the transpiler's (`hrs_proc_macro`, lexed by
//! `harsh_lang::lex`), and they carry Harsh's layout: which token begins a
//! line, at what indentation, and which touches the one before it. The item
//! grammar is read from that -- a named field per line beneath the header,
//! fields inline after `\`, a tuple struct's types juxtaposed, an enum's
//! variants one per line -- which is what makes this a parser of Harsh, and
//! the same grammar the transpiler reads.

use hrs_proc_macro::{Delimiter, Ident, Literal, Place, Punct, Spacing, Span, TokenStream, TokenTree};
use hrs_quote::ToTokens;

// ---------------------------------------------------------------------------
// Errors, as `syn::Error`
// ---------------------------------------------------------------------------

/// A parse error at a token, as `syn::Error`: `to_compile_error` makes the
/// `compile_error!` that points rustc -- and so the programmer -- at it.
#[derive(Clone, Debug)]
pub struct Error {
    span: Span,
    message: String,
}

impl Error {
    pub fn new(span: Span, message: impl Into<String>) -> Error {
        Error { span, message: message.into() }
    }
    pub fn span(&self) -> Span {
        self.span
    }
    /// `compile_error! "message"`, its tokens given the error's span.
    pub fn to_compile_error(&self) -> TokenStream {
        let mut name = TokenTree::from(Ident::new("compile_error", self.span));
        name.set_place(Place::Spaced);
        let mut bang = TokenTree::from(Punct::new('!', Spacing::Alone));
        bang.set_place(Place::Tight);
        bang.set_span(self.span);
        let mut text = TokenTree::from(Literal::string(&self.message));
        text.set_span(self.span);
        [name, bang, text].into_iter().collect()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// What can be parsed from a stream, as `syn::parse::Parse`.
pub trait Parse: Sized {
    fn parse(input: TokenStream) -> Result<Self>;
}

/// `syn::parse`.
pub fn parse<T: Parse>(input: TokenStream) -> Result<T> {
    T::parse(input)
}

/// `syn`'s `parse_macro_input!`: parse, or return the error as the macro's
/// output. A Rust macro from a library, so called as one -- in braces, its
/// stream not being a list of values: `parse_macro_input! { input as
/// DeriveInput }`.
#[macro_export]
macro_rules! parse_macro_input {
    ($input:ident as $ty:ty) => {
        match $crate::parse::<$ty>($input) {
            Ok(parsed) => parsed,
            Err(error) => return error.to_compile_error(),
        }
    };
}

// ---------------------------------------------------------------------------
// The shapes, as `syn`'s
// ---------------------------------------------------------------------------

/// `#[…]` -- a doc comment included, which arrives as `#[doc = "…"]`.
#[derive(Clone, Debug)]
pub struct Attribute {
    /// What stands between the brackets, `serde skip`, `doc = " text"`.
    pub meta: TokenStream,
    pub span: Span,
}

impl Attribute {
    /// The attribute's name, its first identifier: `serde` for `#[serde skip]`.
    pub fn path(&self) -> Option<Ident> {
        match self.meta.trees().first() {
            Some(TokenTree::Ident(i)) => Some(i.clone()),
            _ => None,
        }
    }
    /// Whether this is `#[name …]`.
    pub fn is(&self, name: &str) -> bool {
        self.path().map_or(false, |p| p == name)
    }
}

#[derive(Clone, Debug)]
pub enum Visibility {
    /// `pub`, or `pub (crate)` and the like, as written.
    Public(TokenStream),
    Inherited,
}

/// The generic parameters, `<T: Clone, 'a>`, and a `where` clause.
#[derive(Clone, Debug, Default)]
pub struct Generics {
    /// Between the angle brackets, as written.
    pub params: TokenStream,
    /// After `where`, as written.
    pub where_clause: Option<TokenStream>,
}

/// `impl<T: Clone>` -- the parameters with their bounds.
pub struct ImplGenerics<'a>(&'a Generics);
/// `Name<T>` -- the parameters' names only.
pub struct TypeGenerics<'a>(&'a Generics);
/// `where T: Clone`, or nothing.
pub struct WhereClause<'a>(&'a Generics);

impl Generics {
    /// As `syn`'s: what goes after `impl`, after the type's name, and at the
    /// end of the header.
    pub fn split_for_impl(&self) -> (ImplGenerics<'_>, TypeGenerics<'_>, WhereClause<'_>) {
        (ImplGenerics(self), TypeGenerics(self), WhereClause(self))
    }
}

fn angled(inner: TokenStream) -> TokenStream {
    if inner.is_empty() {
        return inner;
    }
    let mut out: Vec<TokenTree> = Vec::new();
    let mut open = TokenTree::from(Punct::new('<', Spacing::Alone));
    open.set_place(Place::Tight);
    out.push(open);
    let mut first = true;
    for mut t in inner {
        if first {
            t.set_place(Place::Tight);
            first = false;
        }
        out.push(t);
    }
    let mut close = TokenTree::from(Punct::new('>', Spacing::Alone));
    close.set_place(Place::Tight);
    out.push(close);
    out.into_iter().collect()
}

impl ToTokens for ImplGenerics<'_> {
    fn to_token_stream(&self) -> TokenStream {
        angled(self.0.params.clone())
    }
}

impl ToTokens for TypeGenerics<'_> {
    /// Each parameter's name: `'a: 'b` → `'a`, `T: Clone` → `T`,
    /// `const N: usize` → `N`.
    fn to_token_stream(&self) -> TokenStream {
        let mut names: Vec<TokenTree> = Vec::new();
        for param in split_commas(&self.0.params) {
            let mut piece: Vec<TokenTree> = Vec::new();
            let mut it = param.into_iter().peekable();
            if matches!(it.peek(), Some(TokenTree::Ident(i)) if *i == "const") {
                it.next();
            }
            for t in it {
                if matches!(&t, TokenTree::Punct(p) if p.as_char() == ':' || p.as_char() == '=') {
                    break;
                }
                piece.push(t);
            }
            if !names.is_empty() {
                names.push(TokenTree::from(Punct::new(',', Spacing::Alone)));
                names.last_mut().unwrap().set_place(Place::Tight);
            }
            names.extend(piece);
        }
        angled(names.into_iter().collect())
    }
}

impl ToTokens for WhereClause<'_> {
    fn to_token_stream(&self) -> TokenStream {
        match &self.0.where_clause {
            Some(w) => {
                let mut out = vec![TokenTree::from(Ident::new("where", Span::call_site()))];
                out.extend(w.clone());
                out.into_iter().collect()
            }
            None => TokenStream::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Field {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    /// The field's name; `None` for a tuple struct's.
    pub ident: Option<Ident>,
    pub ty: TokenStream,
}

#[derive(Clone, Debug)]
pub enum Fields {
    /// `x: i32` per line, or `\ x: i32, y: i32`.
    Named(Vec<Field>),
    /// A tuple struct's or variant's types, juxtaposed: `Pair i32 String`.
    Unnamed(Vec<Field>),
    Unit,
}

impl Fields {
    pub fn iter(&self) -> std::slice::Iter<'_, Field> {
        match self {
            Fields::Named(f) | Fields::Unnamed(f) => f.iter(),
            Fields::Unit => [].iter(),
        }
    }
    pub fn len(&self) -> usize {
        self.iter().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    pub fields: Fields,
    /// `= 7`: the expression after `=`.
    pub discriminant: Option<TokenStream>,
}

#[derive(Clone, Debug)]
pub struct DataStruct {
    pub fields: Fields,
}
#[derive(Clone, Debug)]
pub struct DataEnum {
    pub variants: Vec<Variant>,
}
#[derive(Clone, Debug)]
pub struct DataUnion {
    pub fields: Fields,
}

#[derive(Clone, Debug)]
pub enum Data {
    Struct(DataStruct),
    Enum(DataEnum),
    Union(DataUnion),
}

impl ToTokens for Visibility {
    /// `pub`, as written, or nothing.
    fn to_token_stream(&self) -> TokenStream {
        match self {
            Visibility::Public(t) => t.clone(),
            Visibility::Inherited => TokenStream::new(),
        }
    }
}

/// A function, as `syn::ItemFn` -- what an attribute-like macro on a
/// function most often parses.
#[derive(Clone, Debug)]
pub struct ItemFn {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    /// The header from `fn` (or its qualifiers) to the end of its line,
    /// without the `:` that opens the body: `fn index$ -> i32`.
    pub sig: TokenStream,
    pub ident: Ident,
    /// The body's lines, each indented relative to the body's own column,
    /// so `#body` in a template lands where it is written.
    pub block: TokenStream,
}

impl Parse for ItemFn {
    fn parse(input: TokenStream) -> Result<ItemFn> {
        let trees: Vec<TokenTree> = input.into_iter().collect();
        let (attrs, rest) = attributes(&trees);
        let (vis, rest) = visibility(rest);
        // The body begins at the first line after the header's; the header's
        // own first token may begin a line too (`fn` with no `pub` before it).
        let split = rest.iter().skip(1).position(|t| matches!(t.place(), Place::Line(_))).map_or(rest.len(), |p| p + 1);
        let (header, body) = rest.split_at(split);
        let at_fn = header.iter().position(|t| ident(t, "fn"));
        let Some(k) = at_fn else {
            return Err(Error::new(header.first().map_or(Span::call_site(), |t| t.span()), "expected a function"));
        };
        let Some(TokenTree::Ident(name)) = header.get(k + 1) else {
            return Err(Error::new(header[k].span(), "`fn` is followed by its name"));
        };
        let sig = match header.last() {
            Some(t) if punct(t, ':') => &header[..header.len() - 1],
            _ => header,
        };
        let base = body.iter().filter_map(|t| match t.place() {
            Place::Line(n) => Some(n),
            _ => None,
        }).min().unwrap_or(0);
        let block: TokenStream = body
            .iter()
            .cloned()
            .map(|mut t| {
                if let Place::Line(n) = t.place() {
                    t.set_place(Place::Line(n - base));
                }
                t
            })
            .collect();
        Ok(ItemFn { attrs, vis, sig: stream(sig), ident: name.clone(), block })
    }
}

/// What a derive receives, as `syn::DeriveInput`.
#[derive(Clone, Debug)]
pub struct DeriveInput {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    pub generics: Generics,
    pub data: Data,
}

// ---------------------------------------------------------------------------
// Reading Harsh's item grammar
// ---------------------------------------------------------------------------

fn punct(t: &TokenTree, c: char) -> bool {
    matches!(t, TokenTree::Punct(p) if p.as_char() == c)
}

fn ident(t: &TokenTree, s: &str) -> bool {
    matches!(t, TokenTree::Ident(i) if *i == s)
}

fn stream(trees: &[TokenTree]) -> TokenStream {
    trees.iter().cloned().collect()
}

/// A stream split at its top-level commas (angle brackets count as a level).
fn split_commas(s: &TokenStream) -> Vec<Vec<TokenTree>> {
    let mut parts: Vec<Vec<TokenTree>> = vec![Vec::new()];
    let mut depth = 0i32;
    let trees = s.trees();
    for (k, t) in trees.iter().enumerate() {
        let arrow = k > 0 && punct(&trees[k - 1], '-');
        match t {
            TokenTree::Punct(p) if p.as_char() == '<' => depth += 1,
            TokenTree::Punct(p) if p.as_char() == '>' && !arrow => depth -= 1,
            TokenTree::Punct(p) if p.as_char() == ',' && depth == 0 => {
                parts.push(Vec::new());
                continue;
            }
            _ => {}
        }
        parts.last_mut().unwrap().push(t.clone());
    }
    parts.retain(|p| !p.is_empty());
    parts
}

/// The lines of an item's body: each is its first token's indentation and
/// its trees, a line indented deeper than the body's own continuing the one
/// before it (a variant's fields beneath it).
struct Line {
    trees: Vec<TokenTree>,
    below: Vec<Line>,
}

fn lines(trees: &[TokenTree]) -> Vec<Line> {
    let mut flat: Vec<(usize, Vec<TokenTree>)> = Vec::new();
    for t in trees {
        match t.place() {
            Place::Line(n) => flat.push((n, vec![t.clone()])),
            _ => match flat.last_mut() {
                Some((_, l)) => l.push(t.clone()),
                None => flat.push((0, vec![t.clone()])),
            },
        }
    }
    nest(&flat)
}

fn nest(flat: &[(usize, Vec<TokenTree>)]) -> Vec<Line> {
    let Some(base) = flat.iter().map(|(n, _)| *n).min() else { return Vec::new() };
    let mut out: Vec<Line> = Vec::new();
    let mut k = 0;
    while k < flat.len() {
        let (n, ref trees) = flat[k];
        let mut j = k + 1;
        while j < flat.len() && flat[j].0 > base {
            j += 1;
        }
        let below = if n == base { nest(&flat[k + 1..j]) } else { Vec::new() };
        out.push(Line { trees: trees.clone(), below });
        k = if n == base { j } else { k + 1 };
    }
    out
}

/// Leading `#[…]` attributes of a slice: the attributes and what follows.
fn attributes(trees: &[TokenTree]) -> (Vec<Attribute>, &[TokenTree]) {
    let mut attrs = Vec::new();
    let mut k = 0;
    while k + 1 < trees.len() && punct(&trees[k], '#') {
        match &trees[k + 1] {
            TokenTree::Group(g) if g.delimiter() == Delimiter::Bracket => {
                attrs.push(Attribute { meta: g.stream(), span: trees[k].span().join(g.span()) });
                k += 2;
            }
            _ => break,
        }
    }
    (attrs, &trees[k..])
}

fn visibility(trees: &[TokenTree]) -> (Visibility, &[TokenTree]) {
    if trees.first().map_or(false, |t| ident(t, "pub")) {
        let mut n = 1;
        if let Some(TokenTree::Group(g)) = trees.get(1) {
            if g.delimiter() == Delimiter::Parenthesis {
                n = 2;
            }
        }
        return (Visibility::Public(stream(&trees[..n])), &trees[n..]);
    }
    (Visibility::Inherited, trees)
}

/// Juxtaposed types, as a tuple struct's: split where a token is spaced from
/// the one before at the top level; an isolated group, `(&'a str)`, is one
/// type and loses its isolation parens (Harsh's application rule).
fn juxtaposed(trees: &[TokenTree]) -> Vec<TokenStream> {
    let mut out: Vec<Vec<TokenTree>> = Vec::new();
    let mut depth = 0i32;
    for (k, t) in trees.iter().enumerate() {
        let starts = depth == 0 && (k == 0 || t.place() != Place::Tight);
        if starts {
            out.push(Vec::new());
        }
        let arrow = k > 0 && punct(&trees[k - 1], '-');
        if punct(t, '<') {
            depth += 1;
        } else if punct(t, '>') && !arrow {
            depth -= 1;
        }
        out.last_mut().unwrap().push(t.clone());
    }
    out.into_iter()
        .map(|ty| match ty.as_slice() {
            [TokenTree::Group(g)] if g.delimiter() == Delimiter::Parenthesis && split_commas(&g.stream()).len() == 1 => {
                let mut inner = g.stream();
                if let Some(first) = inner.trees().first().cloned() {
                    let mut v: Vec<TokenTree> = inner.into_iter().collect();
                    v[0] = first;
                    v[0].set_place(Place::Spaced);
                    inner = v.into_iter().collect();
                }
                inner
            }
            _ => stream(&ty),
        })
        .collect()
}

/// A named field, `[attrs] [vis] name: type`.
fn named_field(attrs: Vec<Attribute>, trees: &[TokenTree]) -> Result<Field> {
    let (more, rest) = attributes(trees);
    let (vis, rest) = visibility(rest);
    let mut all = attrs;
    all.extend(more);
    match rest {
        [TokenTree::Ident(name), colon, ty @ ..] if punct(colon, ':') && !ty.is_empty() => {
            Ok(Field { attrs: all, vis, ident: Some(name.clone()), ty: stream(ty) })
        }
        _ => Err(Error::new(rest.first().map_or(Span::call_site(), |t| t.span()), "a named field is written `name: Type`")),
    }
}

/// Fields beneath a header, one per line, attribute lines attached to the
/// field after them.
fn block_fields(body: &[Line]) -> Result<Vec<Field>> {
    let mut fields = Vec::new();
    let mut pending: Vec<Attribute> = Vec::new();
    for line in body {
        let (attrs, rest) = attributes(&line.trees);
        if rest.is_empty() {
            pending.extend(attrs);
            continue;
        }
        let mut all = std::mem::take(&mut pending);
        all.extend(attrs);
        fields.push(named_field(all, rest)?);
    }
    Ok(fields)
}

/// Fields after `\`, on one line: `a: i32, b: i32`.
fn inline_fields(trees: &[TokenTree]) -> Result<Vec<Field>> {
    split_commas(&stream(trees)).iter().map(|f| named_field(Vec::new(), f)).collect()
}

/// A body given on the header's line (after the name and generics) and the
/// lines beneath it.
fn fields_of(on_line: &[TokenTree], body: &[Line]) -> Result<Fields> {
    if let Some((first, rest)) = on_line.split_first() {
        if punct(first, '\\') {
            return Ok(Fields::Named(inline_fields(rest)?));
        }
        let types = juxtaposed(on_line);
        return Ok(Fields::Unnamed(
            types.into_iter().map(|ty| Field { attrs: Vec::new(), vis: Visibility::Inherited, ident: None, ty }).collect(),
        ));
    }
    if body.is_empty() {
        return Ok(Fields::Unit);
    }
    Ok(Fields::Named(block_fields(body)?))
}

fn variants(body: &[Line]) -> Result<Vec<Variant>> {
    let mut out = Vec::new();
    let mut pending: Vec<Attribute> = Vec::new();
    for line in body {
        let (attrs, rest) = attributes(&line.trees);
        if rest.is_empty() {
            pending.extend(attrs);
            continue;
        }
        let mut all = std::mem::take(&mut pending);
        all.extend(attrs);
        let Some(TokenTree::Ident(name)) = rest.first() else {
            return Err(Error::new(rest[0].span(), "a variant begins with its name"));
        };
        let after = &rest[1..];
        let (fields, discriminant) = match after.first() {
            Some(t) if punct(t, '=') => (Fields::Unit, Some(stream(&after[1..]))),
            _ => (fields_of(after, &line.below)?, None),
        };
        out.push(Variant { attrs: all, ident: name.clone(), fields, discriminant });
    }
    Ok(out)
}

impl Parse for DeriveInput {
    fn parse(input: TokenStream) -> Result<DeriveInput> {
        let trees: Vec<TokenTree> = input.into_iter().collect();
        let (attrs, rest) = attributes(&trees);
        let (vis, rest) = visibility(rest);
        let at = |k: usize| rest.get(k).map_or(Span::call_site(), |t| t.span());
        let kind = match rest.first() {
            Some(TokenTree::Ident(i)) if *i == "struct" || *i == "enum" || *i == "union" => i.to_string(),
            _ => return Err(Error::new(at(0), "a derive applies to a `struct`, an `enum` or a `union`")),
        };
        let Some(TokenTree::Ident(name)) = rest.get(1) else {
            return Err(Error::new(at(1), format!("`{kind}` is followed by its name")));
        };
        // Generic parameters, touching the name: `Wrapper<T>`.
        let mut k = 2;
        let mut generics = Generics::default();
        if rest.get(k).map_or(false, |t| punct(t, '<') && t.place() == Place::Tight) {
            let mut depth = 0;
            let start = k + 1;
            while k < rest.len() {
                let arrow = k > 0 && punct(&rest[k - 1], '-');
                if punct(&rest[k], '<') {
                    depth += 1;
                } else if punct(&rest[k], '>') && !arrow {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                k += 1;
            }
            generics.params = stream(&rest[start..k]);
            k += 1;
        }
        let tail = &rest[k.min(rest.len())..];
        // What stands on the header's line, and the lines beneath it.
        let split = tail.iter().position(|t| matches!(t.place(), Place::Line(_))).unwrap_or(tail.len());
        let (mut on_line, beneath) = tail.split_at(split);
        let body = lines(beneath);
        // `[where T: Clone]` on the header line, where Harsh writes a
        // declaration's where clause: `struct W<T> [where T: Clone]`.
        if let Some(TokenTree::Group(g)) = on_line.first() {
            let inner = g.stream();
            if g.delimiter() == Delimiter::Bracket && inner.trees().first().map_or(false, |t| ident(t, "where")) {
                generics.where_clause = Some(inner.into_iter().skip(1).collect());
                on_line = &on_line[1..];
            }
        }
        let data = match kind.as_str() {
            "struct" => Data::Struct(DataStruct { fields: fields_of(on_line, &body)? }),
            "union" => Data::Union(DataUnion { fields: fields_of(on_line, &body)? }),
            _ => {
                if !on_line.is_empty() {
                    return Err(Error::new(on_line[0].span(), "an enum's variants are written beneath it, one per line"));
                }
                Data::Enum(DataEnum { variants: variants(&body)? })
            }
        };
        Ok(DeriveInput { attrs, vis, ident: name.clone(), generics, data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(text: &str) -> DeriveInput {
        parse::<DeriveInput>(TokenStream::lex_input(text).unwrap()).unwrap()
    }

    fn names(f: &Fields) -> Vec<String> {
        f.iter().map(|f| f.ident.as_ref().map_or("_".to_string(), |i| i.to_string())).collect()
    }

    fn types(f: &Fields) -> Vec<String> {
        f.iter().map(|f| f.ty.to_string()).collect()
    }

    #[test]
    fn a_named_struct_with_attributes_visibility_and_generics() {
        let d = input("/// A point.\n#[allow dead_code]\npub struct Named<T: Clone>\n    pub x: i32\n    #[serde skip]\n    y: Vec<T>");
        assert_eq!(d.ident.to_string(), "Named");
        assert!(matches!(d.vis, Visibility::Public(_)));
        assert_eq!(d.attrs.len(), 2);
        assert!(d.attrs[0].is("doc") && d.attrs[1].is("allow"));
        let Data::Struct(s) = &d.data else { panic!() };
        assert_eq!(names(&s.fields), ["x", "y"]);
        assert_eq!(types(&s.fields), ["i32", "Vec<T>"]);
        let y = s.fields.iter().nth(1).unwrap();
        assert!(y.attrs[0].is("serde"));
        let (i, t, w) = d.generics.split_for_impl();
        assert_eq!((i.to_token_stream().to_string(), t.to_token_stream().to_string(), w.to_token_stream().to_string()), ("<T: Clone>".into(), "<T>".into(), "".into()));
    }

    #[test]
    fn inline_tuple_and_unit_structs() {
        let Data::Struct(s) = input("struct Inline\\ a: i32, b: i32").data else { panic!() };
        assert_eq!(names(&s.fields), ["a", "b"]);
        let Data::Struct(s) = input("struct Pair i32 (&'static str) Vec<u8>").data else { panic!() };
        assert!(matches!(s.fields, Fields::Unnamed(_)));
        assert_eq!(types(&s.fields), ["i32", "&'static str", "Vec<u8>"]);
        let Data::Struct(s) = input("struct Pancakes").data else { panic!() };
        assert!(matches!(s.fields, Fields::Unit));
    }

    #[test]
    fn an_enum_of_every_kind_of_variant() {
        let d = input("enum Shape\n    Unit\n    Tuple i32 String\n    Block\n        x: f64\n        y: f64\n    Inline\\ w: f64, h: f64\n    #[default]\n    Valued = 7");
        let Data::Enum(e) = d.data else { panic!() };
        let v: Vec<String> = e.variants.iter().map(|v| v.ident.to_string()).collect();
        assert_eq!(v, ["Unit", "Tuple", "Block", "Inline", "Valued"]);
        assert!(matches!(e.variants[0].fields, Fields::Unit));
        assert_eq!(types(&e.variants[1].fields), ["i32", "String"]);
        assert_eq!(names(&e.variants[2].fields), ["x", "y"]);
        assert_eq!(names(&e.variants[3].fields), ["w", "h"]);
        assert_eq!(e.variants[4].discriminant.as_ref().unwrap().to_string(), "7");
        assert!(e.variants[4].attrs[0].is("default"));
    }

    #[test]
    fn generics_split_as_syn_splits_them() {
        let d = input("struct W<'a, T: Clone + 'a, const N: usize> [where T: Default]\n    t: &'a [T; N]");
        let (i, t, w) = d.generics.split_for_impl();
        assert_eq!(i.to_token_stream().to_string(), "<'a, T: Clone + 'a, const N: usize>");
        assert_eq!(t.to_token_stream().to_string(), "<'a, T, N>");
        assert_eq!(w.to_token_stream().to_string(), "where T: Default");
        let Data::Struct(s) = &d.data else { panic!() };
        assert_eq!(names(&s.fields), ["t"]);
    }

    #[test]
    fn a_function_as_syn_gives_it() {
        let text = "/// Doc.\npub fn handle (x: i32) -> i32:\n    let y = x + 1\n    if y > 2:\n        y\n    else:\n        0";
        let f = parse::<ItemFn>(TokenStream::lex_input(text).unwrap()).unwrap();
        assert_eq!(f.ident.to_string(), "handle");
        assert!(f.attrs[0].is("doc") && matches!(f.vis, Visibility::Public(_)));
        assert_eq!(f.sig.to_string(), "fn handle (x: i32) -> i32");
        assert_eq!(f.block.to_string(), "let y = x + 1\nif y > 2:\n    y\nelse:\n    0");
        // No `pub`: the header's `fn` begins the stream's first line.
        let g = parse::<ItemFn>(TokenStream::lex_input("fn index$:\n    1").unwrap()).unwrap();
        assert_eq!((g.sig.to_string(), g.block.to_string()), ("fn index$".to_string(), "1".to_string()));
        assert!(matches!(g.vis, Visibility::Inherited));
    }

    /// An error points at the token, as `syn::Error` does.
    #[test]
    fn an_error_names_the_token_it_is_about() {
        let e = parse::<DeriveInput>(TokenStream::lex_input("fn f$:\n    ()").unwrap()).unwrap_err();
        assert!(e.to_string().contains("a derive applies"));
        assert_eq!(e.span(), Span::call_site().join(TokenStream::lex_input("fn").unwrap().trees()[0].span()));
        let out = e.to_compile_error().to_string();
        assert_eq!(out, "compile_error! \"a derive applies to a `struct`, an `enum` or a `union`\"");
    }
}

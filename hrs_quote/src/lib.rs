//! Harsh's `quote` (the user's ruling, 2026-09-24: a Harsh counterpart to
//! `syn`/`quote`, `docs/dev/SYN-QUOTE-PLAN.md`, step 3).
//!
//! A macro crate writes `quote~ do:` with a template beneath it; `hrs`
//! expands that into a call of this crate's `quote!`, handing it the
//! template in pieces -- text, and the values to interpolate with the column
//! each stands at. `render` assembles them, lexes the result with the
//! transpiler's lexer, and gives every interpolated token back the span it
//! had, so an error on it still points at the programmer's code (step 2).

use hrs_proc_macro::{Group, Ident, Literal, Punct, Span, TokenStream, TokenTree};

/// What `#name` may interpolate, as `quote::ToTokens`.
pub trait ToTokens {
    fn to_token_stream(&self) -> TokenStream;
}

impl<T: ToTokens + ?Sized> ToTokens for &T {
    fn to_token_stream(&self) -> TokenStream {
        (**self).to_token_stream()
    }
}
impl ToTokens for TokenStream {
    fn to_token_stream(&self) -> TokenStream {
        self.clone()
    }
}
impl ToTokens for TokenTree {
    fn to_token_stream(&self) -> TokenStream {
        TokenStream::from(self.clone())
    }
}
impl ToTokens for Ident {
    fn to_token_stream(&self) -> TokenStream {
        TokenTree::from(self.clone()).into()
    }
}
impl ToTokens for Literal {
    fn to_token_stream(&self) -> TokenStream {
        TokenTree::from(self.clone()).into()
    }
}
impl ToTokens for Punct {
    fn to_token_stream(&self) -> TokenStream {
        TokenTree::from(self.clone()).into()
    }
}
impl ToTokens for Group {
    fn to_token_stream(&self) -> TokenStream {
        TokenTree::from(self.clone()).into()
    }
}
/// A string interpolates as a string literal, as in `quote`.
impl ToTokens for str {
    fn to_token_stream(&self) -> TokenStream {
        Literal::string(self).to_token_stream()
    }
}
impl ToTokens for String {
    fn to_token_stream(&self) -> TokenStream {
        self.as_str().to_token_stream()
    }
}
impl ToTokens for char {
    fn to_token_stream(&self) -> TokenStream {
        Literal::character(*self).to_token_stream()
    }
}
impl ToTokens for bool {
    fn to_token_stream(&self) -> TokenStream {
        Ident::new(if *self { "true" } else { "false" }, Span::call_site()).to_token_stream()
    }
}
macro_rules! numbers {
    ($($t:ty => $how:ident as $as:ty),*) => {$(
        impl ToTokens for $t {
            fn to_token_stream(&self) -> TokenStream {
                Literal::$how(*self as $as).to_token_stream()
            }
        }
    )*};
}
numbers!(i8 => i64_unsuffixed as i64, i16 => i64_unsuffixed as i64, i32 => i64_unsuffixed as i64, i64 => i64_unsuffixed as i64,
         u8 => u64_unsuffixed as u64, u16 => u64_unsuffixed as u64, u32 => u64_unsuffixed as u64, u64 => u64_unsuffixed as u64,
         usize => usize_unsuffixed as usize, f32 => f64_unsuffixed as f64, f64 => f64_unsuffixed as f64);
/// `None` interpolates nothing, as in `quote`.
impl<T: ToTokens> ToTokens for Option<T> {
    fn to_token_stream(&self) -> TokenStream {
        self.as_ref().map(|t| t.to_token_stream()).unwrap_or_default()
    }
}

/// The value of `#name`, for the code `hrs` generates.
pub fn to_tokens<T: ToTokens + ?Sized>(value: &T) -> TokenStream {
    value.to_token_stream()
}

/// A piece of a template: its own text, a value, or a repetition,
/// `#( … )*`. Columns are not carried: where a value stands is only known
/// when the text before it has been assembled, values included, so
/// `render` measures it then (found 2026-09-24: columns computed at
/// expansion counted a variable's *name* for its value's width).
pub enum Piece {
    Text(&'static str),
    Value(TokenStream),
    /// Rows of values (one per iteration, the repetition's variables side
    /// by side), the repeated template, the separator, and whether the
    /// repetition began a line -- then each iteration is a line of its own,
    /// at that column; otherwise they run inline.
    Repeat(Vec<Vec<TokenStream>>, Vec<Sub>, &'static str, bool),
}

/// A piece of a repeated template: its text, or the row's value at an index.
pub enum Sub {
    Text(&'static str),
    Var(usize),
}

/// A repetition's variable, as streams: `#( #fields )*` iterates `fields`.
pub fn each<I>(values: I) -> Vec<TokenStream>
where
    I: IntoIterator,
    I::Item: ToTokens,
{
    values.into_iter().map(|v| v.to_token_stream()).collect()
}

/// The rows of a repetition: its variables side by side, as `zip` pairs them
/// -- the shortest decides how many (call Q2).
pub fn rows(columns: Vec<Vec<TokenStream>>) -> Vec<Vec<TokenStream>> {
    let n = columns.iter().map(|c| c.len()).min().unwrap_or(0);
    (0..n).map(|i| columns.iter().map(|c| c[i].clone()).collect()).collect()
}

/// The column the next character of `text` will stand at.
fn column(text: &str) -> usize {
    text.rsplit('\n').next().unwrap_or("").chars().count()
}

/// A value put in its place: later lines indented to the column where it
/// begins, its tokens' spans recorded against where they now stand.
fn place(text: &mut String, spans: &mut Vec<(u32, u32, Span)>, stream: &TokenStream) {
    let col = column(text);
    let (value, value_spans) = stream.to_string_with_spans();
    let base = text.len() as u32;
    let moved = |o: u32| o + (col as u32) * value[..o as usize].matches('\n').count() as u32;
    for (lo, hi, span) in value_spans {
        spans.push((base + moved(lo), base + moved(hi), span));
    }
    text.push_str(&value.replace('\n', &format!("\n{}", " ".repeat(col))));
}

/// `quote~`, expanded by `hrs` into a call of this macro with the template's
/// pieces. Importing it is what makes `quote~` available, as `use
/// quote::quote` does for `quote!`.
#[macro_export]
macro_rules! quote {
    ($pieces:expr) => {
        $crate::render($pieces)
    };
}

/// Assemble the template: text as written; a value's text in its place, its
/// later lines indented to the column where it stands. Then lex, and give
/// each interpolated token back its span.
pub fn render(pieces: Vec<Piece>) -> TokenStream {
    let mut text = String::new();
    let mut spans: Vec<(u32, u32, Span)> = Vec::new();
    // An empty value at the start of a line leaves nothing, the space after
    // it included, as `quote` leaves nothing: `#vis #sig:` for a function
    // that is not `pub` must not begin its line with a space, which Harsh
    // would read as indentation.
    let mut eat_space = false;
    for piece in pieces {
        let at_line_start = text.rsplit('\n').next().unwrap_or("").trim().is_empty();
        match piece {
            Piece::Text(t) => {
                let t = if eat_space { t.strip_prefix(' ').unwrap_or(t) } else { t };
                text.push_str(t);
            }
            Piece::Value(ref stream) if stream.is_empty() && at_line_start => {
                eat_space = true;
                continue;
            }
            Piece::Value(ref stream) => place(&mut text, &mut spans, stream),
            Piece::Repeat(rows, subs, sep, lines) => {
                let start = column(&text);
                for (r, row) in rows.iter().enumerate() {
                    if r > 0 {
                        text.push_str(sep);
                        if lines {
                            text.push('\n');
                            text.push_str(&" ".repeat(start));
                        } else {
                            text.push(' ');
                        }
                    }
                    for sub in &subs {
                        match sub {
                            Sub::Text(t) => text.push_str(t),
                            Sub::Var(i) => place(&mut text, &mut spans, &row[*i]),
                        }
                    }
                }
            }
        }
        eat_space = false;
    }
    let mut out = TokenStream::lex_input(&text).unwrap_or_else(|e| panic!("quote~ built text that does not lex as Harsh: {e}\n{text}"));
    out.map_spans(&mut |s| {
        spans.iter().find(|(lo, hi, _)| *lo <= s.lo() && s.lo() < *hi).map(|x| x.2).unwrap_or(Span::call_site())
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_with_values_renders_as_laid_out() {
        let name: TokenStream = "Pancakes".parse().unwrap();
        let out = render(vec![
            Piece::Text("impl HelloMacro for "),
            Piece::Value(to_tokens(&name)),
            Piece::Text("\n    fn hello_macro$:\n        println! \"{}\" (stringify! "),
            Piece::Value(to_tokens(&name)),
            Piece::Text(")"),
        ]);
        assert_eq!(out.to_string(), "impl HelloMacro for Pancakes\n    fn hello_macro$:\n        println! \"{}\" (stringify! Pancakes)");
    }

    /// A multi-line value is indented to where it stands.
    #[test]
    fn a_multi_line_value_takes_its_column() {
        let body: TokenStream = "let a = 1\nlet b = 2".parse().unwrap();
        let out = render(vec![Piece::Text("fn f$:\n    "), Piece::Value(body)]);
        assert_eq!(out.to_string(), "fn f$:\n    let a = 1\n    let b = 2");
    }

    /// An input token interpolated keeps its span; the template's own
    /// tokens have none.
    #[test]
    fn an_interpolated_input_token_keeps_its_span() {
        let input = TokenStream::lex_input("struct Pancakes").unwrap();
        let name = input.trees()[1].clone();
        let out = render(vec![Piece::Text("impl T for "), Piece::Value(to_tokens(&name))]);
        let spans: Vec<(String, Span)> = out.iter().map(|t| (t.to_string_tree(), t.span())).collect();
        assert_eq!(spans[3].1, name.span());
        assert_eq!(spans[0].1, Span::call_site());
    }

    fn words(ws: &[&str]) -> Vec<TokenStream> {
        ws.iter().map(|w| w.parse().unwrap()).collect()
    }

    /// `#( #name: #ty ),*` inline: the variables side by side, the
    /// separator between iterations.
    #[test]
    fn a_repetition_runs_inline_with_its_separator() {
        let out = render(vec![
            Piece::Text("f ("),
            Piece::Repeat(rows(vec![words(&["a", "b"]), words(&["i32", "u8"])]), vec![Sub::Var(0), Sub::Text(": "), Sub::Var(1)], ",", false),
            Piece::Text(")"),
        ]);
        assert_eq!(out.to_string(), "f (a: i32, b: u8)");
    }

    /// A repetition that begins a line repeats as lines, at its column.
    #[test]
    fn a_repetition_at_a_line_start_repeats_as_lines() {
        let out = render(vec![
            Piece::Text("struct P\n    "),
            Piece::Repeat(rows(vec![words(&["x", "y"])]), vec![Sub::Var(0), Sub::Text(": f64")], "", true),
        ]);
        assert_eq!(out.to_string(), "struct P\n    x: f64\n    y: f64");
    }

    /// The case that exposed computing columns at expansion: a multi-line
    /// value after another value on the same line takes the column where it
    /// actually begins, which depends on the first value's width.
    #[test]
    fn a_value_takes_the_column_it_actually_begins_at() {
        let long: TokenStream = "a_rather_long_name".parse().unwrap();
        let body: TokenStream = "x\ny".parse().unwrap();
        let out = render(vec![Piece::Value(long), Piece::Text(" = "), Piece::Value(body)]);
        assert_eq!(out.to_string(), "a_rather_long_name = x\n                     y");
    }

    #[test]
    fn an_empty_value_at_a_line_start_leaves_nothing() {
        let out = render(vec![Piece::Value(TokenStream::new()), Piece::Text(" fn f$:\n    x")]);
        assert_eq!(out.to_string(), "fn f$:\n    x");
    }

    #[test]
    fn rows_stop_at_the_shortest() {
        assert_eq!(rows(vec![words(&["a", "b", "c"]), words(&["1", "2"])]).len(), 2);
        assert!(rows(vec![]).is_empty());
    }

    #[test]
    fn values_of_rust_types_interpolate_as_literals() {
        let out = render(vec![Piece::Value(to_tokens("a\"b")), Piece::Text(" "), Piece::Value(to_tokens(&42)), Piece::Text(" "), Piece::Value(to_tokens(&None::<i32>))]);
        assert_eq!(out.to_string(), "\"a\\\"b\" 42");
    }
}

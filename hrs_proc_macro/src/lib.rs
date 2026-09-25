//! The runtime of Harsh's own procedural macros.
//!
//! A Harsh proc macro is a `pub fn` marked `#[proc_macro~]`, taking and
//! returning a [`TokenStream`]. `hrs` generates a small runner binary that
//! depends on the macro crates and calls [`serve`]; for each macro call in
//! the source being transpiled, `hrs` runs it once.
//!
//! Since 0.2.0 a `TokenStream` is token trees, as Rust's is -- lexed by the
//! transpiler's own lexer, each tree knowing where it stands (`Place`) --
//! and still converts to and from text with `to_string` and `parse`, so a
//! macro written against 0.1.0 compiles unchanged.
//!
//! **The protocol** (one request per process): stdin holds the macro's name,
//! a newline, then the stream's text to the end; stdout receives the
//! expansion, Harsh text -- followed, when any of its tokens came from the
//! input, by `SPANS` and one line per such piece, `text_lo text_hi input_lo
//! input_hi` -- and the exit code is 0. An error is exit 1 with the
//! message on stderr. A macro that panics exits as Rust does (101), its
//! panic message on stderr, and `hrs` reports it at the call.

use std::io::{Read, Write};

mod tokens;
pub use tokens::{Delimiter, Group, Ident, LexError, Literal, Place, Punct, Spacing, Span, TokenStream, TokenTree};


/// What separates an expansion's text from its spans in the runner's answer:
/// a newline, the ASCII record separator, a newline. It cannot occur in
/// Harsh text, which the lexer would refuse.
pub const SPANS: &str = "\n\u{1e}\n";

/// A function-like or derive macro: one stream in, one out.
pub type ProcMacro = fn(TokenStream) -> TokenStream;

/// An attribute-like macro: the attribute's arguments and the item, as
/// Rust's `proc_macro_attribute` takes them.
pub type ProcMacroAttribute = fn(TokenStream, TokenStream) -> TokenStream;

/// A macro as the runner registers it.
#[derive(Clone, Copy)]
pub enum Registered {
    Function(ProcMacro),
    Attribute(ProcMacroAttribute),
}

/// What separates an attribute macro's two streams in a request: the
/// attribute's arguments, then the item. The same bytes as `SPANS`, which
/// cannot occur in Harsh text.
pub const ITEM: &str = "\n\u{1e}\n";

/// One request, answered: `input` is the whole of stdin (name, newline,
/// stream). The core of [`serve`], separate so it can be tested.
pub fn serve_from(input: &str, macros: &[(&str, Registered)]) -> Result<String, String> {
    let (name, stream) = match input.split_once('\n') {
        Some((n, s)) => (n, s),
        None => (input, ""),
    };
    let name = name.trim_end_matches('\r');
    let Some((_, f)) = macros.iter().find(|(n, _)| *n == name) else {
        return Err(format!("no proc macro `{name}` in this crate"));
    };
    let lex = |text: &str, from: u32| -> Result<TokenStream, String> {
        let mut s = TokenStream::lex_input(text).map_err(|e| format!("the stream does not lex as Harsh: {e}"))?;
        if from > 0 {
            // An item's spans count from the start of the whole request's
            // stream, after the arguments and the separator.
            s.map_spans(&mut |sp| if sp == Span::call_site() { sp } else { shift(sp, from) });
        }
        Ok(s)
    };
    let output = match f {
        Registered::Function(f) => f(lex(stream, 0)?),
        Registered::Attribute(f) => {
            let (attr, item) = stream.split_once(ITEM).unwrap_or((stream, ""));
            let at = (attr.len() + ITEM.len()) as u32;
            f(lex(attr, 0)?, lex(item, at)?)
        }
    };
    let (text, spans) = output.to_string_with_spans();
    if spans.is_empty() {
        return Ok(text);
    }
    // After the text, a record separator on a line of its own, then one line
    // per piece that came from the input: its range in the text, and its
    // span in the input (see the protocol above).
    let mut out = text;
    out.push_str(SPANS);
    for (lo, hi, s) in spans {
        out.push_str(&format!("{lo} {hi} {} {}\n", s.lo(), s.hi()));
    }
    Ok(out)
}

fn shift(s: Span, by: u32) -> Span {
    Span::new_at(s.lo() + by, s.hi() + by)
}

/// The runner's `main`: read one request on stdin, answer on stdout, exit.
pub fn serve(macros: &[(&str, Registered)]) -> ! {
    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("the proc-macro runner could not read its input: {e}");
        std::process::exit(1);
    }
    match serve_from(&input, macros) {
        Ok(out) => {
            let mut stdout = std::io::stdout();
            let _ = stdout.write_all(out.as_bytes());
            let _ = stdout.flush();
            std::process::exit(0)
        }
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(input: TokenStream) -> TokenStream {
        format!("\"Hello, {}!\"", input).parse().unwrap()
    }

    fn echo(input: TokenStream) -> TokenStream {
        input
    }

    fn wrap(attr: TokenStream, item: TokenStream) -> TokenStream {
        format!("#[wrapped {attr}]\n{item}").parse().unwrap()
    }

    const MACROS: &[(&str, Registered)] =
        &[("hello", Registered::Function(hello)), ("echo", Registered::Function(echo)), ("wrap", Registered::Attribute(wrap))];

    #[test]
    fn dispatches_by_name() {
        assert_eq!(serve_from("hello\nworld", MACROS).unwrap(), "\"Hello, world!\"");
    }

    /// The text part of an answer (the spans follow `SPANS`).
    fn text(answer: String) -> String {
        answer.split(SPANS).next().unwrap().to_string()
    }

    #[test]
    fn a_multi_line_stream_arrives_as_written() {
        let s = "a b\n    c d\ne";
        assert_eq!(text(serve_from(&format!("echo\n{s}"), MACROS).unwrap()), s);
    }

    /// Returned as received, every token says where it came from: the
    /// answer carries one line per token, text range then input range.
    /// An attribute macro receives two streams, the arguments and the item.
    #[test]
    fn an_attribute_macro_receives_its_arguments_and_the_item() {
        let out = serve_from(&format!("wrap\nGET \"/\"{ITEM}fn index$:\n    1"), MACROS).unwrap();
        assert_eq!(text(out), "#[wrapped GET \"/\"]\nfn index$:\n    1");
    }

    #[test]
    fn an_answer_carries_the_spans_of_what_came_from_the_input() {
        let answer = serve_from("echo\nab cd", MACROS).unwrap();
        assert_eq!(answer, format!("ab cd{SPANS}0 2 0 2\n3 5 3 5\n"));
    }

    #[test]
    fn an_empty_stream_is_empty() {
        assert_eq!(text(serve_from("echo\n", MACROS).unwrap()), "");
        assert_eq!(text(serve_from("echo", MACROS).unwrap()), "");
        assert!("  \n".parse::<TokenStream>().unwrap().is_empty());
    }

    #[test]
    fn an_unknown_name_is_an_error() {
        let e = serve_from("nope\nx", MACROS).unwrap_err();
        assert_eq!(e, "no proc macro `nope` in this crate");
    }

    #[test]
    fn text_round_trips() {
        let t: TokenStream = "f x (y + 1)".parse().unwrap();
        assert_eq!(t.to_string(), "f x (y + 1)");
    }
}

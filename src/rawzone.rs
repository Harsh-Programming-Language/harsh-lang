//! `macro_rules!` — a zone of Rust inside a Harsh file, copied verbatim.
//!
//! Harsh has two declarative macro systems (2026-09-17, `docs/dev/MACRO-DESIGN.md`).
//! `macro_rules~` is Harsh's: Harsh layout, Harsh matchers, Harsh transcribers.
//! `macro_rules!` is **Rust's, and stays Rust** — the author is writing Rust
//! and knows it, so the transpiler reads none of it and rewrites none of it:
//! the definition goes out byte for byte as it came in, and a Rust
//! `macro_rules!` comes back the same way.
//!
//! `macro_rules! name` marks the start of the zone; the `{` that follows opens
//! it and its matching `}` ends it. The braces are the reader's — the
//! transpiler needs only the name to know a zone has begun. No Harsh opener
//! (`:`, `do:`, `raw:`) is used, deliberately: an opener would make the body's
//! indentation meaningful, and the body is Rust, laid out however its author
//! likes.
//!
//! **Finding the end.** A `}` is only a brace when it is one: inside a string,
//! a char literal, a raw string or a comment it is text. The lexer already
//! reads each of those as a single token, so the scan counts `Tk::Open('{')`
//! and `Tk::Close('}')` tokens and never characters — `println!("}")` inside a
//! rule closes nothing.
//!
//! The zone's *calls* are not affected. `my_vec! 1 2 3` in Harsh is a Harsh
//! call and becomes `my_vec!(1, 2, 3)`; only the definition is foreign.

use crate::lex::{Tk, Token};

/// One zone: the byte range of the whole definition in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zone {
    pub lo: usize,
    pub hi: usize,
}

/// Every `macro_rules! name { … }` in a token stream, in source order.
///
/// A `macro_rules` keyword followed by `~` is Harsh's and is not a zone; the
/// mark is still `~` here because zones are found before
/// `layout::normalise_macro_mark` runs.
pub fn zones(toks: &[Token]) -> Vec<Zone> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        if !toks[i].is_kw("macro_rules") {
            i += 1;
            continue;
        }
        // `macro_rules` `!` `name` `{` … `}` -- anything else is not a zone.
        let bang = toks.get(i + 1).filter(|t| t.text == "!");
        let name = toks.get(i + 2).filter(|t| t.kind == Tk::Ident);
        let open = toks
            .get(i + 3)
            .filter(|t| t.kind == Tk::Open('{'));
        let (Some(_), Some(_), Some(open)) = (bang, name, open) else {
            i += 1;
            continue;
        };
        let mut depth = 0i32;
        let mut j = i + 3;
        let close = loop {
            match toks.get(j) {
                None => break None,
                Some(t) => {
                    match t.kind {
                        Tk::Open('{') => depth += 1,
                        Tk::Close('}') => {
                            depth -= 1;
                            if depth == 0 {
                                break Some(t);
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
            }
        };
        match close {
            Some(close) => {
                out.push(Zone {
                    lo: toks[i].span.lo as usize,
                    hi: close.span.hi as usize,
                });
                i = j + 1;
            }
            // Unclosed: leave it to the ordinary error path, which reports the
            // brace with a span the author can find.
            None => {
                let _ = open;
                i += 1;
            }
        }
    }
    out
}

/// Replace each zone's text with a marker comment padded to the zone's exact
/// shape, so every byte offset, line number and column in the rest of the file
/// stays where it was. The marker is a line comment, which travels through the
/// transpiler untouched and marks where the zone belongs in the output.
pub fn blank_out(src: &str, zones: &[Zone]) -> String {
    let mut out = String::with_capacity(src.len());
    let mut at = 0usize;
    for (n, z) in zones.iter().enumerate() {
        out.push_str(&src[at..z.lo]);
        let marker = format!("//{}{}", MARKER, n);
        let text = &src[z.lo..z.hi];
        let first_line_len = text.find('\n').unwrap_or(text.len());
        if first_line_len >= marker.len() {
            out.push_str(&marker);
            out.extend(std::iter::repeat(' ').take(first_line_len - marker.len()));
        } else {
            // A zone whose first line is shorter than the marker cannot be
            // padded to shape; blank it and let `restore` place it by order.
            out.extend(std::iter::repeat(' ').take(first_line_len));
        }
        for ch in text[first_line_len..].chars() {
            out.push(if ch == '\n' { '\n' } else { ' ' });
        }
        at = z.hi;
    }
    out.push_str(&src[at..]);
    out
}

/// The marker's body. Chosen to be impossible in ordinary source.
pub const MARKER: &str = "__hrs_raw_zone_";

/// Put each zone's text back into generated Rust, where its marker stands.
pub fn restore(generated: &mut String, src: &str, zones: &[Zone]) {
    if zones.is_empty() {
        return;
    }
    for (n, z) in zones.iter().enumerate() {
        let marker = format!("//{}{}", MARKER, n);
        let text = &src[z.lo..z.hi];
        if let Some(at) = generated.find(&marker) {
            // The zone carries its own indentation, so any spaces the marker
            // was written with are dropped with it.
            let line_start = generated[..at].rfind('\n').map(|n| n + 1).unwrap_or(0);
            let at = if generated[line_start..at].chars().all(|c| c == ' ') {
                line_start
            } else {
                at
            };
            // The marker stands alone on its line, padded with the spaces that
            // kept the columns in place; the zone replaces the whole line.
            let line_end = generated[at..]
                .find('\n')
                .map(|n| at + n)
                .unwrap_or(generated.len());
            let mark_end = generated[at..line_end].find(&marker).map(|n| at + n + marker.len()).unwrap_or(at + marker.len());
            let blank_after = generated[mark_end..line_end]
                .chars()
                .all(|c| c == ' ');
            let end = if blank_after { line_end } else { mark_end };
            generated.replace_range(at..end, text);
        } else {
            // The marker did not survive (an empty file, or a zone the
            // emitter dropped): put the definition at the top, where a macro
            // must in any case be defined before its first use.
            let mut item = String::from(text);
            item.push_str("\n\n");
            generated.insert_str(0, &item);
        }
    }
    // A zone at the top of the file leaves the blank lines its body occupied.
    while generated.starts_with('\n') {
        generated.remove(0);
    }
}

/// Lex, find the zones, and hand back the text the transpiler should work on.
/// When there are none — the usual case — the source is returned untouched and
/// nothing else in the pipeline knows this pass exists.
pub fn prepare(src: &str) -> (String, Vec<Zone>) {
    // The scan is done in Rust mode: a zone *is* Rust, and Harsh's lexer
    // refuses Rust spellings (`::` among them) before this pass could see
    // them. Rust mode reads a string, a char, a raw string and a comment as
    // single tokens too, which is all the scan needs — a `}` inside one of
    // them is text, not a brace.
    let Ok(toks) = crate::lex::lex_rust(src) else {
        return (src.to_string(), Vec::new());
    };
    let zs = zones(&toks);
    if zs.is_empty() {
        return (src.to_string(), zs);
    }
    (blank_out(src, &zs), zs)
}

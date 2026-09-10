// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Doc examples are Harsh.
//!
//! A fenced block inside a `///` or `//!` comment is not prose: rustdoc lifts
//! it out, compiles it and runs it, so it is code, and code in a `.hrs` file is
//! Harsh. This module transpiles it on the way out and converts it on the way
//! in, so that `hrs` writes the Rust rustdoc expects and `hrs-from` writes the
//! Harsh an author reads.
//!
//! Which fences: exactly the ones rustdoc treats as Rust. An empty info string,
//! `rust`, and the doctest attributes (`ignore`, `no_run`, `should_panic`,
//! `compile_fail`, `edition2021`, ...) are code; any other tag names another
//! language (```` ```text ````, ```` ```toml ````) and is left alone. The
//! decision is rustdoc's, not a new rule of Harsh's.
//!
//! A body is transpiled inside a `fn main$:` wrapper, which is what rustdoc
//! does with it too: without a surrounding item the layout has no expression
//! region and would copy `f x` through unapplied.

use crate::lex::Span;

/// The doctest attributes rustdoc accepts on a fence that still holds Rust.
/// Anything else in the info string names another language.
fn is_rust_fence(info: &str) -> bool {
    info.split(|c: char| c == ',' || c.is_whitespace())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .all(|w| {
            matches!(w, "rust" | "ignore" | "no_run" | "should_panic" | "compile_fail"
                        | "allow_fail" | "test_harness" | "standalone_crate")
                || w.starts_with("edition")
                || w.starts_with("ignore-")
        })
}

/// One fenced example: the span of its body in the text it was found in
/// (whole lines, prefixes included, the fence lines excluded), and the body's
/// lines split into the comment prefix and the code after it.
pub struct Example {
    pub lo: u32,
    pub hi: u32,
    /// `(prefix, code)` per line: `"    /// "` and `"let x = 1"`.
    lines: Vec<(String, String)>,
}

/// Every Rust-fenced doc example in `text`, in order.
pub fn examples(text: &str) -> Vec<Example> {
    let mut out = Vec::new();
    let mut off = 0usize;
    let mut open: Option<(usize, Vec<(String, String)>)> = None; // body start, lines
    for line in text.split_inclusive('\n') {
        let next = off + line.len();
        let body = line.trim_end_matches('\n');
        let trimmed = body.trim_start();
        let marker = if trimmed.starts_with("///") {
            Some("///")
        } else if trimmed.starts_with("//!") {
            Some("//!")
        } else {
            None
        };
        match marker {
            None => open = None, // a doc run ends at the first line that is not one
            Some(m) => {
                let indent = &body[..body.len() - trimmed.len()];
                let after = &trimmed[m.len()..];
                let space = if after.starts_with(' ') { " " } else { "" };
                let code = &after[space.len()..];
                let prefix = format!("{}{}{}", indent, m, space);
                if let Some(rest) = code.trim_start().strip_prefix("```") {
                    match open.take() {
                        // The closing fence: the example ends at the line above.
                        Some((lo, lines)) if rest.trim().is_empty() => {
                            if !lines.is_empty() {
                                out.push(Example { lo: lo as u32, hi: off as u32, lines });
                            }
                        }
                        // A fence that opens (a nested one cannot: rustdoc has none).
                        _ if is_rust_fence(rest) => open = Some((next, Vec::new())),
                        _ => open = None,
                    }
                } else if let Some((_, lines)) = open.as_mut() {
                    lines.push((prefix, code.to_string()));
                }
            }
        }
        off = next;
    }
    out
}

/// rustdoc hides a line beginning `# ` from the rendered page and still
/// compiles it. The marker is stripped before the code is read and put back
/// after, so it is neither Harsh's `#` nor a lost line.
fn hidden(code: &str) -> Option<(&str, &str)> {
    let t = code.trim_start();
    let ind = &code[..code.len() - t.len()];
    if t == "#" {
        Some((ind, ""))
    } else {
        t.strip_prefix("# ").map(|rest| (ind, rest))
    }
}

impl Example {
    /// The code of the example, with the comment prefixes and the hidden-line
    /// markers taken off, and `(prefix, hidden marker)` kept per line.
    fn strip(&self) -> (String, Vec<(String, String)>) {
        let mut code = String::new();
        let mut back = Vec::new();
        for (prefix, line) in &self.lines {
            match hidden(line) {
                Some((ind, rest)) => {
                    code.push_str(ind);
                    code.push_str(rest);
                    back.push((prefix.clone(), format!("{}# ", ind)));
                }
                None => {
                    code.push_str(line);
                    back.push((prefix.clone(), String::new()));
                }
            }
            code.push('\n');
        }
        (code, back)
    }

    /// Put `body`'s lines back behind the prefixes. A line the transpiler
    /// added takes the prefix of the line before it.
    fn dress(&self, body: &str, back: &[(String, String)]) -> String {
        let last = back.last().cloned().unwrap_or_default();
        let mut out = Vec::new();
        for (i, line) in body.lines().enumerate() {
            let (prefix, hide) = back.get(i).unwrap_or(&last);
            let prefix = prefix.trim_end();
            if line.trim().is_empty() && hide.is_empty() {
                out.push(prefix.to_string());
            } else {
                out.push(format!("{} {}{}", prefix, hide, line.trim_end()));
            }
        }
        out.join("\n")
    }
}

/// Transpile one example's Harsh to Rust. The error's span is in `text`'s
/// coordinates: the reader is pointed at the line he wrote.
fn to_rust(ex: &Example) -> Result<String, (Span, String)> {
    let (code, back) = ex.strip();
    // Inside `fn main$:`, as rustdoc will also wrap it; four spaces in.
    let mut wrapped = String::from("fn main$:\n");
    for line in code.lines() {
        if line.trim().is_empty() {
            wrapped.push('\n');
        } else {
            wrapped.push_str("    ");
            wrapped.push_str(line);
            wrapped.push('\n');
        }
    }
    // A trailing `()` so that the reader's last line is a statement like the
    // others: without it the block's tail expression loses its `;`, and
    // rustdoc's own wrapper would then read it as what `main` returns.
    wrapped.push_str("    ()\n");
    let err = |span: Span, msg: String| {
        // A span inside the wrapper, back to a span inside `text`: find the
        // line it fell on, then that line's own offset.
        let head = "fn main$:\n".len();
        let off = (span.lo as usize).max(head) - head;
        let mut line = 0usize;
        let mut seen = 0usize;
        for (k, l) in wrapped[head..].split_inclusive('\n').enumerate() {
            if seen + l.len() > off {
                line = k;
                break;
            }
            seen += l.len();
        }
        let mut at = ex.lo as usize;
        for (k, (prefix, body)) in ex.lines.iter().enumerate() {
            if k == line {
                at += prefix.len() + body.len() - body.trim_start().len();
                break;
            }
            at += prefix.len() + body.len() + 1;
        }
        (Span { lo: at as u32, hi: at as u32 + 1 }, msg)
    };
    let toks = crate::lex::lex(&wrapped).map_err(|e| err(e.span, e.msg))?;
    let arities = crate::juxt::collect_arities(&toks);
    let tree = crate::layout::build_with(toks, &arities).map_err(|e| err(e.span, e.msg))?;
    let mut em = crate::emit::Emitter::new(&wrapped);
    em.program(&tree);
    // Unwrap: drop `fn main() {` and the closing `}`, dedent by four.
    let rust = em.out;
    let inner: Vec<&str> = rust.lines().collect();
    let body: String = inner[1..inner.len().saturating_sub(2)]
        .iter()
        .map(|l| l.strip_prefix("    ").unwrap_or(l))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(ex.dress(&body, &back))
}

/// Rewrite every Rust-fenced doc example in the generated Rust, shifting the
/// source map by what each rewrite changes. `src` is the Harsh the examples
/// were read from; the comments reach `out` verbatim, so each body is found
/// there by its own text.
pub fn to_rust_in(
    out: &mut String,
    map: &mut Vec<crate::emit::MapEntry>,
    src: &str,
) -> Result<(), (Span, String)> {
    let mut cursor = 0usize;
    for ex in examples(src) {
        let old = &src[ex.lo as usize..ex.hi as usize];
        let old = old.strip_suffix('\n').unwrap_or(old);
        let new = to_rust(&ex)?;
        if new == old {
            continue;
        }
        let Some(rel) = out[cursor..].find(old) else { continue };
        let at = cursor + rel;
        let end = at + old.len();
        out.replace_range(at..end, &new);
        let delta = new.len() as i64 - old.len() as i64;
        for e in map.iter_mut() {
            if e.gen_lo as i64 >= end as i64 {
                e.gen_lo = (e.gen_lo as i64 + delta) as u32;
                e.gen_hi = (e.gen_hi as i64 + delta) as u32;
            }
        }
        cursor = at + new.len();
    }
    Ok(())
}

/// The other direction: the Rust of a doc example becomes Harsh, so that a
/// converted crate's documentation is in the language the file is written in.
/// A body that does not convert is left as it stands -- the converter's rule
/// everywhere: translate what it understands, copy the rest.
pub fn to_harsh_in(text: &mut String) {
    // The spans are read from a snapshot; the text is rewritten as we go, so
    // each example is found in it by its own body rather than by an offset
    // an earlier rewrite has moved.
    let snap = text.clone();
    let mut cursor = 0usize;
    for ex in examples(&snap) {
        let (code, back) = ex.strip();
        let mut wrapped = String::from("fn main() {\n");
        for line in code.lines() {
            wrapped.push_str(line);
            wrapped.push('\n');
        }
        wrapped.push_str("}\n");
        let Ok(harsh) = crate::unbrace::convert(&wrapped) else { continue };
        let lines: Vec<&str> = harsh.trim_end().lines().collect();
        if lines.len() < 2 {
            continue;
        }
        let body: String = lines[1..]
            .iter()
            .map(|l| l.strip_prefix("    ").unwrap_or(l))
            .collect::<Vec<_>>()
            .join("\n");
        // The wrapper's last statement carried a `;`, which in Harsh means
        // *discard this value* at the end of a block. An example is a list of
        // statements, not a block with a tail, so the mark goes: `to_rust`
        // adds a statement after the last line for the same reason.
        let body = match body.trim_end().strip_suffix(';') {
            Some(cut) => cut.to_string(),
            None => body,
        };
        let dressed = ex.dress(&body, &back);
        let old = {
            let o = &snap[ex.lo as usize..ex.hi as usize];
            o.strip_suffix('\n').unwrap_or(o)
        };
        if dressed == old {
            continue;
        }
        let Some(rel) = text[cursor..].find(old) else { continue };
        let at = cursor + rel;
        text.replace_range(at..at + old.len(), &dressed);
        cursor = at + dressed.len();
    }
}

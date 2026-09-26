//! A Rust macro's brace stream — `name! { … }` — is Rust, even in a Harsh
//! file, and is copied verbatim; only its holes, `@: … :@`, are Harsh
//! (ruling 16, `docs/dev/MACRO-DESIGN.md`, and the user's principles of
//! 2026-09-23: a `!` macro's stream is written in Rust, and no Harsh syntax or
//! layout applies inside it except between `@:` and `:@`).
//!
//! The mechanism is `rawzone`'s: each body is set aside before the file is
//! lexed, replaced by a one-line placeholder `{/*Zn*/}` so the Harsh around it
//! reads as ordinary Harsh, and put back into the generated Rust with each hole
//! transpiled in place. Harsh knows no DSL: the body's own syntax — `view!`'s
//! markup, `rsx!`'s tree, `sql!`'s query — is never read, only scanned for the
//! paired marks.
//!
//! **Holes always close** (ruled 2026-09-23): `{@: value$ :@}`,
//! `on:click= @: move |_| f$ :@`. A hole's code is Harsh; its baseline is the
//! indentation of the line the hole opens on, so a hole may be one line or a
//! block beneath `@:` (B2). A hole may hold a macro call with a brace body of
//! its own, with holes of its own. `@@:` is a literal `@:` in the body's text.
//!
//! Not yet covered: a brace body produced by a `~` macro's expansion (the
//! body reaches the transpiler as tokens, after this pass has run), and
//! bodies inside a `macro_rules~` definition, which are left to its expansion.

use crate::lex::{Tk, Token};

/// One body: byte offsets of its `{` and its matching `}` in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zone {
    pub open: usize,
    pub close: usize,
    /// Bytes the placeholder added beyond the body's first line, when that
    /// line was shorter than it: every source offset after `open` in the
    /// blanked text is this much too large, and the map is corrected by it.
    pub extra: usize,
}

fn placeholder(n: usize) -> String {
    format!("{{/*Z{n}*/}}")
}

/// Line ranges (byte offsets) of `macro_rules~` definitions: the header line
/// and every following line indented deeper. A brace body there is part of a
/// transcriber and belongs to the expansion, not to this pass.
fn harsh_macro_defs(src: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let lines: Vec<(usize, &str)> = {
        let mut v = Vec::new();
        let mut at = 0;
        for l in src.split_inclusive('\n') {
            v.push((at, l));
            at += l.len();
        }
        v
    };
    let indent = |l: &str| l.len() - l.trim_start().len();
    let mut i = 0;
    while i < lines.len() {
        let (at, l) = lines[i];
        if l.trim_start().starts_with("macro_rules~") {
            let base = indent(l);
            let mut j = i + 1;
            while j < lines.len() && (lines[j].1.trim().is_empty() || indent(lines[j].1) > base) {
                j += 1;
            }
            let end = if j < lines.len() { lines[j].0 } else { src.len() };
            out.push((at, end));
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Every outermost `name! { … }` body, in source order. Tokens are read in
/// Rust mode, so a `}` inside a string, a char or a comment is text.
pub fn zones(src: &str) -> Vec<Zone> {
    let Ok(toks) = crate::lex::lex_rust(src) else { return Vec::new() };
    let defs = harsh_macro_defs(src);
    let in_def = |at: usize| defs.iter().any(|&(a, b)| at >= a && at < b);
    let mut out: Vec<Zone> = Vec::new();
    let mut i = 0;
    while i + 2 < toks.len() {
        let bang = toks[i].kind == Tk::Ident
            && !toks[i].is_kw("macro_rules")
            && toks[i + 1].kind == Tk::Punct
            && toks[i + 1].text == "!"
            && toks[i].span.hi == toks[i + 1].span.lo;
        let open = (i + 2..toks.len()).find(|&k| !toks[k].is_comment());
        let Some(o) = open.filter(|&o| bang && toks[o].kind == Tk::Open('{')) else {
            i += 1;
            continue;
        };
        if in_def(toks[o].span.lo as usize) {
            i += 1;
            continue;
        }
        match close_of(&toks, o) {
            Some(c) => {
                out.push(Zone { open: toks[o].span.lo as usize, close: toks[c].span.lo as usize, extra: 0 });
                i = c + 1;
            }
            None => i += 1,
        }
    }
    out
}

fn close_of(toks: &[Token], o: usize) -> Option<usize> {
    let mut d = 0i32;
    for (k, t) in toks.iter().enumerate().skip(o) {
        match t.kind {
            Tk::Open(_) => d += 1,
            Tk::Close(_) => {
                d -= 1;
                if d == 0 {
                    return (t.kind == Tk::Close('}')).then_some(k);
                }
            }
            _ => {}
        }
    }
    None
}

/// Replace each body, braces included, by its placeholder, padded with spaces
/// and newlines to the body's shape so every line after it keeps its number.
/// A body shorter than its placeholder lengthens its own line; the columns
/// after it on that line shift, nothing else does.
pub fn blank_out(src: &str, zones: &[Zone]) -> String {
    blank_out_mut(src, &mut zones.to_vec())
}

/// `blank_out`, recording in each zone the bytes its placeholder added.
pub fn blank_out_mut(src: &str, zones: &mut [Zone]) -> String {
    let mut out = String::with_capacity(src.len());
    let mut at = 0;
    for (n, z) in zones.iter_mut().enumerate() {
        out.push_str(&src[at..z.open]);
        let body = &src[z.open..=z.close];
        let ph = placeholder(n);
        out.push_str(&ph);
        let first = body.find('\n').unwrap_or(body.len());
        z.extra = ph.len().saturating_sub(first);
        out.extend(std::iter::repeat(' ').take(first.saturating_sub(ph.len())));
        for ch in body[first..].chars() {
            out.push(if ch == '\n' { '\n' } else { ' ' });
        }
        at = z.close + 1;
    }
    out.push_str(&src[at..]);
    out
}

/// Lex-free preparation: the text the transpiler reads, and the zones to put
/// back. When there are none the source is returned untouched.
pub fn prepare(src: &str) -> (String, Vec<Zone>) {
    let mut zs = zones(src);
    if zs.is_empty() {
        return (src.to_string(), zs);
    }
    let work = blank_out_mut(src, &mut zs);
    (work, zs)
}

/// Put each body back into generated Rust where its placeholder stands, with
/// its holes transpiled. `hole` turns one hole's Harsh into Rust. The source
/// map is kept right: entries after the body move by what the body added,
/// the placeholder's own entries go, and the body gets one entry per piece --
/// a DSL segment maps to itself in the source, a hole's Rust to the hole's
/// Harsh, `@:` to `:@` -- so a diagnostic inside a body points at the `.hrs`
/// line and the hole it is in.
pub fn restore(
    generated: &mut String,
    src: &str,
    zones: &[Zone],
    hole: &dyn Fn(&str) -> Result<String, String>,
    map: &mut Vec<crate::emit::MapEntry>,
) -> Result<(), String> {
    // The emitter's source offsets are the blanked text's: every placeholder
    // that outgrew its body's first line pushed what follows it along.
    if zones.iter().any(|z| z.extra > 0) {
        let mut cum = 0usize;
        let starts: Vec<(usize, usize)> = zones.iter().map(|z| { let s = (z.open + cum, z.extra); cum += z.extra; s }).collect();
        for e in map.iter_mut() {
            let back: usize = starts.iter().filter(|&&(o, _)| (e.src_lo as usize) > o).map(|&(_, x)| x).sum();
            e.src_lo -= back as u32;
            let back_hi: usize = starts.iter().filter(|&&(o, _)| (e.src_hi as usize) > o).map(|&(_, x)| x).sum();
            e.src_hi -= back_hi as u32;
        }
    }
    for (n, z) in zones.iter().enumerate() {
        let ph = placeholder(n);
        let Some(at) = generated.find(&ph) else {
            return Err(format!("a macro's brace body (zone {n}) was lost in transpiling"));
        };
        let pieces = fill_holes(src, z.open + 1, z.close, hole)?;
        let mut text = String::from("{");
        let mut entries: Vec<crate::emit::MapEntry> = vec![crate::emit::MapEntry { gen_lo: at as u32, gen_hi: at as u32 + 1, src_lo: z.open as u32, src_hi: z.open as u32 + 1, ctx: 0 }];
        for p in &pieces {
            let g = at + text.len();
            entries.push(crate::emit::MapEntry { gen_lo: g as u32, gen_hi: (g + p.text.len()) as u32, src_lo: p.src_lo as u32, src_hi: p.src_hi as u32, ctx: 0 });
            text.push_str(&p.text);
        }
        let g = at + text.len();
        entries.push(crate::emit::MapEntry { gen_lo: g as u32, gen_hi: g as u32 + 1, src_lo: z.close as u32, src_hi: z.close as u32 + 1, ctx: 0 });
        text.push('}');
        let old_end = at + ph.len();
        let delta = text.len() as i64 - ph.len() as i64;
        map.retain(|e| (e.gen_lo as usize) < at || (e.gen_lo as usize) >= old_end);
        shift(map, old_end, delta);
        map.extend(entries);
        generated.replace_range(at..old_end, &text);
        // The lines the body occupied were blanked, and came out as empty
        // lines after the call: as many as the body spans are dropped.
        let spans = src[z.open..=z.close].matches('\n').count();
        let after = at + text.len();
        if let Some(nl) = generated[after..].find('\n').map(|k| after + k + 1) {
            let mut end = nl;
            for _ in 0..spans {
                match generated[end..].find('\n') {
                    Some(k) if generated[end..end + k].trim().is_empty() => end += k + 1,
                    _ => break,
                }
            }
            if end > nl {
                generated.replace_range(nl..end, "");
                // The blank lines' own entries go with them.
                map.retain(|e| (e.gen_lo as usize) < nl || (e.gen_lo as usize) >= end);
                shift(map, end, nl as i64 - end as i64);
            }
        }
        map.sort_by_key(|e| e.gen_lo);
    }
    Ok(())
}

/// Move every entry at or after `from` in the generated text by `delta`.
fn shift(map: &mut [crate::emit::MapEntry], from: usize, delta: i64) {
    for e in map.iter_mut() {
        if e.gen_lo as usize >= from {
            e.gen_lo = (e.gen_lo as i64 + delta) as u32;
            e.gen_hi = (e.gen_hi as i64 + delta) as u32;
        }
    }
}

/// One piece of a restored body: generated text and the source range it
/// stands for.
#[derive(Debug)]
struct Piece {
    text: String,
    src_lo: usize,
    src_hi: usize,
}

/// The body's text between `lo` and `hi` with every hole replaced by its
/// Rust, and `@@:` by a literal `@:`.
fn fill_holes(src: &str, lo: usize, hi: usize, hole: &dyn Fn(&str) -> Result<String, String>) -> Result<Vec<Piece>, String> {
    let text = &src[lo..hi];
    let b = text.as_bytes();
    let mut pieces: Vec<Piece> = Vec::new();
    let mut out = String::with_capacity(text.len());
    let mut seg_lo = lo;
    let mut i = 0;
    while i < b.len() {
        if text[i..].starts_with("@@:") {
            out.push_str("@:");
            i += 3;
            continue;
        }
        if text[i..].starts_with("@:") {
            // The matching `:@`, counting holes nested inside this one.
            let mut d = 0i32;
            let mut j = i;
            let mut end = None;
            while j + 1 < b.len() {
                // Byte comparisons: this scan steps one byte at a time, and a
                // slice `text[j..]` inside a multi-byte character panics
                // (found 2026-09-26: `{@: "…" :@}`). The marks are ASCII, so
                // bytes find exactly what characters would.
                if b[j..].starts_with(b"@@:") {
                    j += 3;
                    continue;
                }
                if b[j..].starts_with(b"@:") {
                    d += 1;
                    j += 2;
                    continue;
                }
                if b[j..].starts_with(b":@") {
                    d -= 1;
                    if d == 0 {
                        end = Some(j);
                        break;
                    }
                    j += 2;
                    continue;
                }
                j += 1;
            }
            let Some(e) = end else {
                let line = src[..lo + i].matches('\n').count() + 1;
                return Err(format!("line {line}: `@:` opens a hole that is never closed; a hole always ends in `:@`"));
            };
            // The hole's baseline: the indentation of the line it opens on.
            let line_start = src[..lo + i].rfind('\n').map_or(0, |k| k + 1);
            let base = src[line_start..].len() - src[line_start..].trim_start().len();
            let code = dedent(&text[i + 2..e], base);
            let rust = hole(&code).map_err(|m| {
                let line = src[..lo + i].matches('\n').count() + 1;
                format!("in the hole opened on line {line}: {m}")
            })?;
            // Statements as a value are a block -- unless the hole already
            // sits directly inside braces, `{@: …; x :@}`, which make it one.
            let before = text[..i].trim_end();
            let after = text[e + 2..].trim_start();
            let enclosed = before.ends_with('{') && after.starts_with('}');
            let rust = if has_statements(&rust) && !enclosed { format!("{{ {rust} }}") } else { rust };
            // A hole over several lines keeps its place: its following lines
            // are indented by the hole's baseline, as its Harsh was.
            let pad = " ".repeat(base);
            let rust = rust.replace('\n', &format!("\n{pad}"));
            // The DSL segment before the hole, then the hole itself.
            if !out.is_empty() {
                pieces.push(Piece { text: std::mem::take(&mut out), src_lo: seg_lo, src_hi: lo + i });
            }
            let code_lo = lo + i + 2 + text[i + 2..e].len() - text[i + 2..e].trim_start().len();
            let code_hi = lo + e - (text[i + 2..e].len() - text[i + 2..e].trim_end().len());
            pieces.push(Piece { text: rust, src_lo: code_lo, src_hi: code_hi.max(code_lo) });
            seg_lo = lo + e + 2;
            i = e + 2;
            continue;
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    if !out.is_empty() {
        pieces.push(Piece { text: out, src_lo: seg_lo, src_hi: hi });
    }
    Ok(pieces)
}

/// A hole's code as a Harsh fragment: the text on the `@:` line at indent 0,
/// every following line moved left by the hole's baseline.
/// B7 (the user, 2026-09-25, option (a)): `hrs fmt` writes one space inside
/// a hole's marks, `@: x :@` -- after `@:` unless the code starts on the next
/// line, before `:@` unless the mark starts its own line. `@@:`, a literal
/// `@:`, is not a mark. Outside the marks the text is the DSL's and is left
/// as written. One line at a time: a mark's spacing is its own line's.
pub fn normalize_hole_line(line: &str) -> String {
    let b = line.as_bytes();
    let mut out = String::with_capacity(line.len() + 4);
    let mut i = 0;
    while i < b.len() {
        if line[i..].starts_with("@@:") {
            out.push_str("@@:");
            i += 3;
            continue;
        }
        if line[i..].starts_with("@:") {
            out.push_str("@:");
            i += 2;
            let mut j = i;
            while j < b.len() && (b[j] == b' ' || b[j] == b'\t') {
                j += 1;
            }
            // Code follows on this line: exactly one space before it.
            if j < b.len() {
                out.push(' ');
            }
            i = j;
            continue;
        }
        if line[i..].starts_with(":@") {
            // The mark ends a hole: one space after the code, unless the mark
            // starts its line.
            let trimmed_len = out.trim_end_matches([' ', '\t']).len();
            let own_line = out[..trimmed_len].trim().is_empty();
            if !own_line {
                out.truncate(trimmed_len);
                out.push(' ');
            }
            out.push_str(":@");
            i += 2;
            continue;
        }
        let ch = line[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn dedent(code: &str, base: usize) -> String {
    let mut lines = code.split('\n');
    let mut out: Vec<String> = Vec::new();
    if let Some(first) = lines.next() {
        if !first.trim().is_empty() {
            out.push(first.trim().to_string());
        }
    }
    for l in lines {
        if l.trim().is_empty() {
            continue;
        }
        let ind = l.len() - l.trim_start().len();
        let keep = ind.saturating_sub(base);
        out.push(format!("{}{}", " ".repeat(keep), l.trim_start()));
    }
    // A block beneath `@:` starts at its own indent: move it all to 0.
    let min = out.iter().map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    out.iter().map(|l| l[min.min(l.len() - l.trim_start().len())..].to_string()).collect::<Vec<_>>().join("\n")
}

/// Transpile one hole: its code becomes the body of a function, and the
/// function's body is the hole's Rust. A body with more than one statement is
/// a block.
pub fn transpile_hole(code: &str, whole: &dyn Fn(&str) -> Result<String, String>) -> Result<String, String> {
    let mut src = String::from("fn __hrs_hole$:\n");
    for l in code.lines() {
        src.push_str("    ");
        src.push_str(l);
        src.push('\n');
    }
    let rust = whole(&src)?;
    let open = rust.find('{').ok_or("a hole produced no code")?;
    let close = rust.rfind('}').ok_or("a hole produced no code")?;
    let body = rust[open + 1..close].trim();
    let lines: Vec<&str> = body.lines().collect();
    let min = lines.iter().filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    let body: String = lines.iter().map(|l| if l.len() >= min { &l[min..] } else { l.trim() }).collect::<Vec<_>>().join("\n");
    let mut d = 0i32;
    let mut stmts = false;
    for ch in body.chars() {
        match ch {
            '(' | '[' | '{' => d += 1,
            ')' | ']' | '}' => d -= 1,
            ';' if d == 0 => stmts = true,
            _ => {}
        }
    }
    // A hole written on one line stays on one line: joining is safe, since a
    // one-line hole holds no line comment and no multi-line string.
    let body = if code.contains('\n') { body } else { body.split_whitespace().collect::<Vec<_>>().join(" ") };
    let _ = stmts;
    Ok(body)
}

/// Does this Rust hold statements at its top level -- a `;` outside any
/// bracket? Then, as a value, it must be a block.
pub fn has_statements(rust: &str) -> bool {
    let mut d = 0i32;
    for ch in rust.chars() {
        match ch {
            '(' | '[' | '{' => d += 1,
            ')' | ']' | '}' => d -= 1,
            ';' if d == 0 => return true,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_holes_marks_get_one_space_inside_and_nothing_else_changes() {
        use super::normalize_hole_line as n;
        assert_eq!(n("<p>{@:count + 1:@}</p>"), "<p>{@: count + 1 :@}</p>");
        assert_eq!(n("<p>{@:    count :@}</p>"), "<p>{@: count :@}</p>");
        assert_eq!(n("on:click=@:f 1:@>"), "on:click=@: f 1 :@>");
        // Outside the marks is the DSL's own text.
        assert_eq!(n("WHERE id = @:id:@ AND x"), "WHERE id = @: id :@ AND x");
        // A multi-line hole: `@:` at the line's end, `:@` starting its own.
        assert_eq!(n("<ul>{@:"), "<ul>{@:");
        assert_eq!(n("        :@}</ul>"), "        :@}</ul>");
        // `@@:` is a literal `@:`, never a mark.
        assert_eq!(n("mail @@:x"), "mail @@:x");
        assert_eq!(n("{@: x :@}"), "{@: x :@}");
    }

    use super::*;

    #[test]
    fn a_body_is_found_whole_and_nested_bodies_with_it() {
        let src = "fn f$:\n    view! {\n        <p>{@: x :@}</p>\n        <q>{@: view! { <b/> } :@}</q>\n    }\n    let n = 1\n";
        let z = zones(src);
        assert_eq!(z.len(), 1);
        assert_eq!(&src[z[0].open..z[0].open + 1], "{");
        assert_eq!(&src[z[0].close..z[0].close + 1], "}");
        let b = blank_out(src, &z);
        assert_eq!(b.lines().count(), src.lines().count());
        assert!(b.contains("view! {/*Z0*/}"));
        assert!(b.contains("    let n = 1"));
    }

    #[test]
    fn a_harsh_macro_definition_and_a_tilde_call_are_not_zones() {
        let src = "macro_rules~ filled\n    ([ $e:expr ; $n:expr ]) => do:\n        vec! { $e; $n }\n\nfn f$:\n    let a = m~ { x }\n    let b = macro_rules! { }\n";
        assert!(zones(src).is_empty(), "{:?}", zones(src));
    }

    #[test]
    fn holes_always_close_and_nest() {
        let id = |s: &str| Ok::<String, String>(format!("<{s}>"));
        let src = "{a @: x :@ b @: y @: z :@ :@ c @@: d}";
        let joined: String = fill_holes(src, 1, src.len() - 1, &id).unwrap().iter().map(|p| p.text.as_str()).collect();
        assert_eq!(joined, "a <x> b <y @: z :@> c @: d");
        let bad = "{a @: x }";
        assert!(fill_holes(bad, 1, bad.len() - 1, &id).unwrap_err().contains("never closed"));
    }

    #[test]
    fn a_block_hole_keeps_its_relative_layout() {
        let code = "\n    if a:\n        1\n    else:\n        2\n    ";
        assert_eq!(dedent(code, 4), "if a:\n    1\nelse:\n    2");
        assert_eq!(dedent(" move |_| f$ ", 8), "move |_| f$");
    }
}

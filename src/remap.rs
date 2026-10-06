// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Remaps rustc and clippy diagnostics from generated Rust back to Harsh source.
//!
//! Usage:
//!     cargo build --message-format=json 2>&1 | hrs-remap --map target/src.map.json
//!
//! rustc's `rendered` field is discarded and the diagnostic is re-rendered from
//! the structured spans, because `rendered` has the generated file's paths and
//! line numbers baked into it.

use std::io::{self, Write};

pub struct Entry {
    gen_lo: u32,
    gen_hi: u32,
    src_lo: u32,
    src_hi: u32,
    /// The macro expansion the generated text came from, 0 for none.
    ctx: u32,
}

/// A Harsh macro expansion recorded by the transpiler: a diagnostic inside
/// the expanded code is pointed at the definition (through the entries) and
/// at the call (through this).
pub struct Expansion {
    ctx: u32,
    name: String,
    call_lo: u32,
}

pub struct SourceMap {
    source_path: String,
    generated_path: String,
    entries: Vec<Entry>,
    expansions: Vec<Expansion>,
    src: String,
    line_starts: Vec<usize>,
}

impl SourceMap {
    pub fn load(path: &str) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))?;
        let v: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| format!("{}: {}", path, e))?;
        let source_path = v["source"].as_str().unwrap_or("").to_string();
        let generated_path = v["generated"].as_str().unwrap_or("").to_string();
        let mut entries = Vec::new();
        if let Some(arr) = v["entries"].as_array() {
            for e in arr {
                let a = e.as_array().ok_or("bad entry")?;
                entries.push(Entry {
                    gen_lo: a[0].as_u64().unwrap_or(0) as u32,
                    gen_hi: a[1].as_u64().unwrap_or(0) as u32,
                    src_lo: a[2].as_u64().unwrap_or(0) as u32,
                    src_hi: a[3].as_u64().unwrap_or(0) as u32,
                    ctx: a.get(4).and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                });
            }
        }
        entries.sort_by_key(|e| e.gen_lo);
        let mut expansions = Vec::new();
        if let Some(arr) = v["expansions"].as_array() {
            for e in arr {
                if let Some(a) = e.as_array() {
                    expansions.push(Expansion {
                        ctx: a[0].as_u64().unwrap_or(0) as u32,
                        name: a[1].as_str().unwrap_or("").to_string(),
                        call_lo: a[2].as_u64().unwrap_or(0) as u32,
                    });
                }
            }
        }
        let src = std::fs::read_to_string(&source_path)
            .map_err(|e| format!("{}: {}", source_path, e))?;
        let mut line_starts = vec![0usize];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Ok(SourceMap { source_path, generated_path, entries, expansions, src, line_starts })
    }

    /// The generated file this map describes.
    pub fn generated(&self) -> &str {
        &self.generated_path
    }

    /// The Harsh file this map describes.
    pub fn source(&self) -> &str {
        &self.source_path
    }

    /// A 1-based `line:column` in the generated file -- as a panic reports it
    /// -- back to the 1-based line and column in the Harsh source. Columns
    /// count characters, as `std::panic::Location` does.
    pub fn locate(&self, line: usize, col: usize) -> Option<(usize, usize)> {
        let gen = std::fs::read_to_string(&self.generated_path).ok()?;
        let start = if line <= 1 {
            0
        } else {
            gen.match_indices('\n').nth(line - 2).map(|(i, _)| i + 1)?
        };
        let text = &gen[start..];
        let off = start + text.char_indices().nth(col.saturating_sub(1)).map_or(text.len(), |(i, _)| i);
        let src_off = self.remap(off as u32)? as usize;
        let l = self.line_starts.partition_point(|&s| s <= src_off);
        let line_start = self.line_starts[l - 1];
        let c = self.src.get(line_start..src_off).map_or(1, |s| s.chars().count() + 1);
        Some((l, c))
    }

    /// The other direction: a 1-based `line:column` in the Harsh source to the
    /// 1-based `line:column` of the generated Rust it became -- for asking
    /// rust-analyzer about a place in the `.hrs` (the language server's hover
    /// and go-to-definition). The narrowest entry whose source range holds the
    /// position gives the Rust range, and the offset inside the token is
    /// carried over. None where the position produced no Rust (a comment, a
    /// blank). Columns count characters.
    pub fn find(&self, line: usize, col: usize) -> Option<(usize, usize)> {
        let start = *self.line_starts.get(line.checked_sub(1)?)?;
        let text = &self.src[start..];
        let off = (start + text.char_indices().nth(col.saturating_sub(1)).map_or(text.len(), |(i, _)| i)) as u32;
        // Code written as it stands first; else code that came out of a
        // macro expansion -- a name passed to a macro, `apply~ adding (4)`,
        // is written by you and becomes Rust inside the expansion (found
        // 2026-09-27: hover on such a name answered nothing).
        let e = self
            .entries
            .iter()
            .filter(|e| e.src_lo <= off && off < e.src_hi)
            .min_by_key(|e| (e.ctx != 0, e.src_hi - e.src_lo))?;
        let gen_off = (e.gen_lo + (off - e.src_lo).min(e.gen_hi.saturating_sub(e.gen_lo + 1))) as usize;
        let gen = std::fs::read_to_string(&self.generated_path).ok()?;
        let gen_off = gen_off.min(gen.len());
        let l = gen[..gen_off].matches('\n').count() + 1;
        let line_start = gen[..gen_off].rfind('\n').map_or(0, |i| i + 1);
        Some((l, gen[line_start..gen_off].chars().count() + 1))
    }

    /// Narrowest entry containing `off`, else the next entry that starts after
    /// it. Inserted braces and separators have no entry of their own, so a
    /// diagnostic pointing at one lands on the following real token.
    fn remap(&self, off: u32) -> Option<u32> {
        let i = self.entries.partition_point(|e| e.gen_lo <= off);
        if i > 0 {
            let e = &self.entries[i - 1];
            if off < e.gen_hi {
                let delta = off - e.gen_lo;
                let width = e.src_hi - e.src_lo;
                return Some(e.src_lo + delta.min(width));
            }
            // Inserted punctuation right after a token -- the `;` the
            // transpiler wrote after `1 +` -- has no entry. A diagnostic
            // there is about what precedes it, not what follows: map to the
            // end of the preceding token unless a real token begins here.
            let next_starts_here = self.entries.get(i).map_or(false, |n| n.gen_lo == off);
            if off == e.gen_hi && !next_starts_here {
                return Some(e.src_hi);
            }
        }
        self.entries.get(i).map(|e| e.src_lo)
    }

    /// Where an *insertion* at `off` belongs: a zero-width span sitting at the
    /// end of a token (`.to_string()` after a literal) maps to the end of that
    /// token, not to whatever follows -- which may be inserted punctuation with
    /// no entry at all.
    fn remap_insert(&self, off: u32) -> Option<u32> {
        let i = self.entries.partition_point(|e| e.gen_lo <= off);
        if i > 0 {
            let e = &self.entries[i - 1];
            if off <= e.gen_hi {
                let delta = off - e.gen_lo;
                let width = e.src_hi - e.src_lo;
                return Some(e.src_lo + delta.min(width));
            }
        }
        self.entries.get(i).map(|e| e.src_lo)
    }

    /// The expansion the generated text at `off` came from, if any.
    fn expansion_at(&self, off: u32) -> Option<&Expansion> {
        let i = self.entries.partition_point(|e| e.gen_lo <= off);
        let e = if i > 0 { &self.entries[i - 1] } else { return None };
        let next_starts_here = self.entries.get(i).map_or(false, |n| n.gen_lo == off);
        let inside = off < e.gen_hi || (off == e.gen_hi && !next_starts_here);
        if e.ctx == 0 || !inside {
            return None;
        }
        self.expansions.iter().find(|x| x.ctx == e.ctx)
    }

    fn line_col(&self, off: u32) -> (usize, usize) {
        let off = off as usize;
        let li = self.line_starts.partition_point(|&s| s <= off).saturating_sub(1);
        let col = self.src[self.line_starts[li]..off.min(self.src.len())].chars().count() + 1;
        (li + 1, col)
    }

    fn line_text(&self, line: usize) -> &str {
        let start = self.line_starts[line - 1];
        let end = self.src[start..].find('\n').map(|k| start + k).unwrap_or(self.src.len());
        &self.src[start..end]
    }

    fn matches(&self, file_name: &str) -> bool {
        let norm = |s: &str| {
            std::fs::canonicalize(s)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| s.to_string())
        };
        norm(file_name) == norm(&self.generated_path)
            || file_name.ends_with(&self.generated_path)
            || self.generated_path.ends_with(file_name)
    }
}

pub struct Loc {
    line: usize,
    col_start: usize,
    col_end: usize,
    label: Option<String>,
    is_primary: bool,
    /// The offset in the generated file this location came from.
    gen_off: u32,
}

pub fn collect(map: &SourceMap, msg: &serde_json::Value, out: &mut Vec<Loc>) {
    if let Some(spans) = msg["spans"].as_array() {
        for s in spans {
            let file = s["file_name"].as_str().unwrap_or("");
            if !map.matches(file) {
                continue;
            }
            let bs = s["byte_start"].as_u64().unwrap_or(0) as u32;
            let be = s["byte_end"].as_u64().unwrap_or(0) as u32;
            let (lo, hi) = if be <= bs {
                let at = map.remap_insert(bs);
                (at, at)
            } else {
                (map.remap(bs), map.remap(be - 1))
            };
            let (Some(sl), Some(sh)) = (lo, hi) else {
                continue;
            };
            let (line, col_start) = map.line_col(sl);
            let (eline, mut col_end) = map.line_col(sh);
            if eline != line {
                col_end = map.line_text(line).chars().count() + 1;
            } else {
                col_end += 1;
            }
            out.push(Loc {
                gen_off: bs,
                line,
                col_start,
                col_end: col_end.max(col_start + 1),
                label: s["label"].as_str().map(|s| s.to_string()),
                is_primary: s["is_primary"].as_bool().unwrap_or(false),
            });
        }
    }
}

pub fn render(map: &SourceMap, msg: &serde_json::Value, w: &mut impl Write) -> io::Result<bool> {
    let level = msg["level"].as_str().unwrap_or("error");
    let text = msg["message"].as_str().unwrap_or("");
    let code = msg["code"]["code"].as_str();

    let mut locs = Vec::new();
    collect(map, msg, &mut locs);
    if locs.is_empty() {
        return Ok(false);
    }
    locs.sort_by_key(|l| (l.line, l.col_start));

    match code {
        Some(c) => writeln!(w, "{}[{}]: {}", level, c, text)?,
        None => writeln!(w, "{}: {}", level, text)?,
    }
    let primary = locs.iter().find(|l| l.is_primary).unwrap_or(&locs[0]);
    let primary_gen = Some(primary.gen_off);
    writeln!(w, "  --> {}:{}:{}", map.source_path, primary.line, primary.col_start)?;

    let gutter = locs.iter().map(|l| l.line.to_string().len()).max().unwrap_or(1).max(2);
    writeln!(w, "{:>w$} |", "", w = gutter)?;
    let mut last_line = 0usize;
    for l in &locs {
        if l.line != last_line {
            writeln!(w, "{:>w$} | {}", l.line, map.line_text(l.line), w = gutter)?;
            last_line = l.line;
        }
        let caret = if l.is_primary { '^' } else { '-' };
        let pad = " ".repeat(l.col_start.saturating_sub(1));
        let marks = caret.to_string().repeat((l.col_end - l.col_start).max(1));
        match &l.label {
            Some(lab) if !lab.is_empty() => {
                writeln!(w, "{:>w$} | {}{} {}", "", pad, marks, lab, w = gutter)?
            }
            _ => writeln!(w, "{:>w$} | {}{}", "", pad, marks, w = gutter)?,
        }
    }

    // Inside a Harsh macro's expansion: the snippet above is the definition's
    // line; say which call produced it, so both ends are on the page.
    if let Some(x) = primary_gen.and_then(|off| map.expansion_at(off)) {
        let (cl, cc) = map.line_col(x.call_lo);
        // A derive is recorded as its call is written, `#[derive~ Name]`; a
        // `~` call by its name.
        let call = if x.name.starts_with("#[") { x.name.clone() } else { format!("{}~", x.name) };
        writeln!(w, "{:>w$} = note: in the expansion of `{}` called at {}:{}:{}", "", call, map.source_path, cl, cc, w = gutter)?;
        writeln!(w, "{:>w$} | {}", cl, map.line_text(cl), w = gutter)?;
    }

    // Sub-diagnostics: notes and helps that point into generated code.
    if let Some(children) = msg["children"].as_array() {
        for c in children {
            let lvl = c["level"].as_str().unwrap_or("note");
            let m = c["message"].as_str().unwrap_or("");
            let mut clocs = Vec::new();
            collect(map, c, &mut clocs);
            if let Some(cl) = clocs.first() {
                writeln!(w, "{:>w$} = {}: {} (hrs {}:{})", "", lvl, m, cl.line, cl.col_start, w = gutter)?;
            } else {
                writeln!(w, "{:>w$} = {}: {}", "", lvl, m, w = gutter)?;
            }
        }
    }
    writeln!(w)?;
    Ok(true)
}


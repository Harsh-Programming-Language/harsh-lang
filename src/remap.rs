// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Remaps rustc and clippy diagnostics from generated Rust back to Harsh source.
//!
//! Usage:
//!     cargo build --message-format=json 2>&1 | hrs-remap --map target/hrs.map.json
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
}

pub struct SourceMap {
    source_path: String,
    generated_path: String,
    entries: Vec<Entry>,
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
                });
            }
        }
        entries.sort_by_key(|e| e.gen_lo);
        let src = std::fs::read_to_string(&source_path)
            .map_err(|e| format!("{}: {}", source_path, e))?;
        let mut line_starts = vec![0usize];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Ok(SourceMap { source_path, generated_path, entries, src, line_starts })
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


// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Read a cargo `--message-format=json` stream and re-render each diagnostic
//! against the Harsh source it came from.

use std::io::{self, BufRead, Write};

fn main() {
    if std::env::args().skip(1).any(|a| a == "--version" || a == "-V") {
        println!("hrs-remap {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let maps: Vec<String> = std::env::args()
        .skip(1)
        .scan(false, |want, a| {
            let take = *want;
            *want = a == "--map";
            Some(if take { Some(a) } else { None })
        })
        .flatten()
        .collect();
    if maps.is_empty() {
        eprintln!("usage: cargo build --message-format=json | hrs-remap --map <file.map.json> ...");
        std::process::exit(2);
    }
    let mut loaded = Vec::new();
    for m in &maps {
        match harsh_lang::remap::SourceMap::load(m) {
            Ok(s) => loaded.push(s),
            Err(e) => {
                eprintln!("hrs-remap: {}", e);
                std::process::exit(1);
            }
        }
    }
    let stdout = io::stdout();
    let mut w = stdout.lock();
    let (mut errors, mut warnings) = (0usize, 0usize);
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
        if v["reason"].as_str() != Some("compiler-message") {
            continue;
        }
        let msg = &v["message"];
        let level = msg["level"].as_str().unwrap_or("");
        if level == "failure-note" {
            continue;
        }
        for m in &loaded {
            if harsh_lang::remap::render(m, msg, &mut w).unwrap_or(false) {
                match level {
                    "error" => errors += 1,
                    "warning" => warnings += 1,
                    _ => {}
                }
                break;
            }
        }
    }
    if errors > 0 || warnings > 0 {
        let _ = writeln!(w, "hrs: {} error(s), {} warning(s)", errors, warnings);
    }
    if errors > 0 {
        std::process::exit(1);
    }
}

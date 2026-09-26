// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Convert Rust source to Harsh.

use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().skip(1).any(|a| a == "--version" || a == "-V") {
        println!("hrs-from {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input = None;
    let mut output = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                output = args.get(i).cloned();
            }
            "--raw" => {}
            s if input.is_none() => input = Some(s.to_string()),
            _ => {
                eprintln!("usage: hrs-from <input.rs> [-o <output.hrs>]");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    let Some(input) = input else {
        eprintln!("usage: hrs-from <input.rs> [-o <output.hrs>]");
        return ExitCode::from(2);
    };
    let src = match fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("hrs-from: cannot read {}: {}", input, e);
            return ExitCode::FAILURE;
        }
    };
    // The converter's output goes through the formatter, so what it writes
    // is in the recommended layout.
    // `--raw`: the converter's own output, before the formatter.
    let raw = std::env::args().any(|a| a == "--raw");
    match harsh_lang::unbrace::convert(&src)
        .map(|o| if raw { o } else { format_if_sound(o) })
        .map(|mut o| {
            // A doc example is code, so it is converted like the rest.
            harsh_lang::docex::to_harsh_in(&mut o);
            o
        }) {
        Ok(out) => match output {
            Some(p) => {
                if let Err(e) = fs::write(&p, out) {
                    eprintln!("hrs-from: cannot write {}: {}", p, e);
                    return ExitCode::FAILURE;
                }
            }
            None => print!("{}", out),
        },
        Err(e) => {
            eprintln!("hrs-from: {}", e);
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// The conversion, formatted -- when it transpiles. When it does not (a gap in
/// the converter), it is returned as the converter wrote it: the formatter,
/// guarded only against changing a *working* file, would otherwise reflow the
/// broken one, and the gap would look like a second problem (found
/// 2026-09-25: the "width-reflowed layout" seen twice was this).
fn format_if_sound(o: String) -> String {
    if harsh_lang::driver::transpile_str(&o).is_ok() {
        harsh_lang::fmt::format(&o)
    } else {
        o
    }
}

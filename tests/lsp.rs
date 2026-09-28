// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Hover and go-to-definition through rust-analyzer (LSP-RA-DESIGN): the
//! source map both ways, and `hrs-lsp` asking a rust-analyzer about the Rust
//! a Harsh file became -- a fake one, here, so the suite needs none installed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A small Harsh project, transpiled: `helper` defined on line 1, called on 5.
fn project(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("harsh-lsp-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"lsp\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"lsp\"\npath = \"target/hrs/main.rs\"\n").unwrap();
    fs::write(root.join("src/main.hrs"), "fn helper x: i32 -> i32:\n    x + 1\n\nfn main$:\n    let y = helper 41\n    println! \"{y}\"\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_hrs")).args(["cargo", "metadata", "--no-deps", "--format-version", "1"]).current_dir(&root).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    root
}

/// `find` takes a `.hrs` position to the `.rs` one it became, and `locate`
/// brings it back: the two directions agree on every token asked.
#[test]
fn the_source_map_goes_both_ways() {
    let root = project("find");
    let m = harsh_lang::remap::SourceMap::load(root.join("target/hrs/main.map.json").to_str().unwrap()).unwrap();
    let rust = fs::read_to_string(root.join("target/hrs/main.rs")).unwrap();
    // `helper` at its call: Harsh 5:13, Rust `let y = helper(41);` 6:13.
    assert_eq!(m.find(5, 13), Some((6, 13)), "{rust}");
    assert_eq!(m.locate(6, 13), Some((5, 13)));
    // `helper` at its definition: 1:4 both sides.
    assert_eq!(m.find(1, 4), Some((1, 4)));
    // Inside a token, the offset is carried: `helper`'s third letter.
    assert_eq!(m.find(5, 15), Some((6, 15)));
    let _ = fs::remove_dir_all(&root);
}

/// An editor's side of the conversation with `hrs-lsp`.
struct Client {
    child: std::process::Child,
    out: std::io::BufReader<std::process::ChildStdout>,
    next: i64,
    /// Notifications that arrived while waiting for an answer.
    seen: Vec<serde_json::Value>,
}

impl Client {
    fn start(ra: &str) -> Client {
        Client::spawn(ra, None)
    }
    fn start_with_log(ra: &str, log: &Path) -> Client {
        Client::spawn(ra, Some(log))
    }
    fn spawn(ra: &str, log: Option<&Path>) -> Client {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_hrs-lsp"));
        if let Some(l) = log {
            cmd.env("HRS_LSP_LOG", l);
        }
        let mut child = cmd
            .env("HRS_RA", ra)
            .env("HRS", env!("CARGO_BIN_EXE_hrs"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let out = std::io::BufReader::new(child.stdout.take().unwrap());
        Client { child, out, next: 0, seen: Vec::new() }
    }
    fn send(&mut self, v: serde_json::Value) {
        use std::io::Write;
        let body = v.to_string();
        let stdin = self.child.stdin.as_mut().unwrap();
        write!(stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        stdin.flush().unwrap();
    }
    fn read(&mut self) -> serde_json::Value {
        use std::io::{BufRead, Read};
        let mut len = 0;
        loop {
            let mut line = String::new();
            self.out.read_line(&mut line).unwrap();
            let t = line.trim_end();
            if t.is_empty() {
                break;
            }
            if let Some(v) = t.strip_prefix("Content-Length:") {
                len = v.trim().parse().unwrap();
            }
        }
        let mut buf = vec![0; len];
        self.out.read_exact(&mut buf).unwrap();
        serde_json::from_slice(&buf).unwrap()
    }
    fn request(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        self.next += 1;
        let id = self.next;
        self.send(serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let v = self.read();
            if v["id"] == serde_json::json!(id) {
                return v["result"].clone();
            }
            self.seen.push(v);
        }
    }
    fn notify(&mut self, method: &str, params: serde_json::Value) {
        self.send(serde_json::json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }
    fn open(&mut self, root: &Path, text: &str) -> String {
        let uri = format!("file://{}", root.join("src/main.hrs").display());
        self.request("initialize", serde_json::json!({"processId": null, "rootUri": format!("file://{}", root.display()), "capabilities": {}}));
        self.notify("initialized", serde_json::json!({}));
        self.notify("textDocument/didOpen", serde_json::json!({"textDocument": {"uri": uri, "languageId": "harsh", "version": 1, "text": text}}));
        uri
    }
    fn at(uri: &str, line: u32, character: u32) -> serde_json::Value {
        serde_json::json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}})
    }
    fn stop(mut self) {
        self.request("shutdown", serde_json::Value::Null);
        self.notify("exit", serde_json::Value::Null);
        let _ = self.child.wait();
    }
}

fn fake_ra() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-rust-analyzer.py").display().to_string()
}

/// Hover and go-to-definition, asked in a `.hrs`, reach rust-analyzer at the
/// place in the `.rs` the Harsh became, and come back at the `.hrs` place: the
/// design's path end to end, with a fake rust-analyzer standing in.
#[test]
fn hover_and_definition_go_through_rust_analyzer_and_back() {
    let root = project("ra");
    let text = fs::read_to_string(root.join("src/main.hrs")).unwrap();
    let mut c = Client::start(&fake_ra());
    let uri = c.open(&root, &text);
    // `helper` at its call: Harsh 0-based 4:12, Rust 5:12 (`let y = helper(41);`).
    let h = c.request("textDocument/hover", Client::at(&uri, 4, 12));
    assert_eq!(h["contents"]["value"], "fake hover at 5:12 in main.rs", "{h}");
    assert_eq!(h["range"]["start"], serde_json::json!({"line": 4, "character": 12}), "{h}");
    // Its definition: `fn helper` on the first line of the `.hrs`, not the `.rs`.
    let d = c.request("textDocument/definition", Client::at(&uri, 4, 12));
    assert!(d["uri"].as_str().unwrap().ends_with("src/main.hrs"), "{d}");
    assert_eq!(d["range"]["start"], serde_json::json!({"line": 0, "character": 3}), "{d}");
    // Typing, unsaved: the changed line is not answered -- nothing, rather than wrong.
    let edited = text.replace("helper 41", "helper 40 + helper 1");
    c.notify("textDocument/didChange", serde_json::json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": edited}]}));
    assert!(c.request("textDocument/hover", Client::at(&uri, 4, 12)).is_null());
    // An unchanged line still is.
    let d = c.request("textDocument/definition", Client::at(&uri, 0, 4));
    assert!(!d.is_null(), "an unchanged line is answered from the last good translation");
    c.stop();
    let _ = fs::remove_dir_all(&root);
}

/// No rust-analyzer: said once, plainly, and hover answers nothing -- while
/// formatting, which needs none, carries on.
#[test]
fn a_missing_rust_analyzer_is_said_once() {
    let root = project("missing");
    let text = fs::read_to_string(root.join("src/main.hrs")).unwrap();
    let mut c = Client::start("/nonexistent/rust-analyzer");
    let uri = c.open(&root, &text);
    assert!(c.request("textDocument/hover", Client::at(&uri, 4, 12)).is_null());
    assert!(c.request("textDocument/hover", Client::at(&uri, 4, 12)).is_null());
    let told: Vec<_> = c.seen.iter().filter(|v| v["method"] == "window/showMessage").collect();
    assert_eq!(told.len(), 1, "{told:?}");
    assert!(told[0]["params"]["message"].as_str().unwrap().contains("rustup component add rust-analyzer"));
    let f = c.request("textDocument/formatting", serde_json::json!({"textDocument": {"uri": uri}, "options": {"tabSize": 4, "insertSpaces": true}}));
    assert!(f.is_array() || f.is_null(), "formatting still answers: {f}");
    c.stop();
    let _ = fs::remove_dir_all(&root);
}

/// A name passed through a macro -- `apply~ adding (4) (5)` -- is written by
/// you and becomes Rust inside the expansion: it maps (found 2026-09-27 on the
/// user's Mac, where hover on it answered nothing).
#[test]
fn a_name_passed_to_a_macro_maps_to_its_rust() {
    let root = project("macro");
    fs::write(root.join("src/main.hrs"), "macro_rules~ apply\n    (($v:ident) $( ($x:expr) )*) => do:\n        $v $( $x )*\n\nfn adding (x:i32) (y:i32) -> i32 do:\n    x + y\n\nfn main$:\n    let x = apply~ adding (4) (5)\n    println! \"{x}\"\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_hrs")).args(["cargo", "metadata", "--no-deps", "--format-version", "1"]).current_dir(&root).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let m = harsh_lang::remap::SourceMap::load(root.join("target/hrs/main.map.json").to_str().unwrap()).unwrap();
    let rust = fs::read_to_string(root.join("target/hrs/main.rs")).unwrap();
    let (line, text) = rust.lines().enumerate().find(|(_, l)| l.contains("adding(4")).unwrap();
    let col = text.find("adding").unwrap() + 1;
    // `adding` at the call, Harsh 9:20.
    assert_eq!(m.find(9, 20), Some((line + 1, col)), "{rust}");
    let _ = fs::remove_dir_all(&root);
}

/// Completion works on the text as it is being typed: `y <- ` completes to
/// the names rust-analyzer offers at `y.`, given as plain names.
#[test]
fn completion_asks_at_the_text_being_typed() {
    let root = project("complete");
    let text = fs::read_to_string(root.join("src/main.hrs")).unwrap();
    let mut c = Client::start(&fake_ra());
    let uri = c.open(&root, &text);
    let typing = text.replace("    println! \"{y}\"\n", "    let z = y <- \n    println! \"{y}\"\n");
    c.notify("textDocument/didChange", serde_json::json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": typing}]}));
    // The cursor after `y <- `: line 5 (0-based), column 17.
    let r = c.request("textDocument/completion", Client::at(&uri, 5, 17));
    let items = r["items"].as_array().unwrap_or_else(|| panic!("{r}"));
    let labels: Vec<&str> = items.iter().map(|i| i["label"].as_str().unwrap()).collect();
    // The fake answers with where it was asked: the marker in `y.hrsCompletionMark`.
    let side = fs::read_to_string(root.join("target/hrs/.complete/cell.rs")).unwrap();
    let (line, l) = side.lines().enumerate().find(|(_, l)| l.contains("y.hrsCompletionMark")).unwrap();
    let col = l.find("hrsCompletionMark").unwrap();
    assert!(labels.contains(&format!("at {line}:{col}").as_str()), "{labels:?}\n{side}");
    // `len()` comes as the plain name `len`; the marker itself is never offered.
    let len = items.iter().find(|i| i["label"] == "len()").unwrap();
    assert_eq!(len["insertText"], "len");
    assert!(!labels.contains(&"hrsCompletionMark"));
    c.stop();
    let _ = fs::remove_dir_all(&root);
}

/// The log, when asked for: each question and answer, so a silent answer on
/// someone's machine explains itself.
#[test]
fn the_log_records_what_was_asked_and_answered() {
    let root = project("log");
    let log = root.join("lsp.log");
    let text = fs::read_to_string(root.join("src/main.hrs")).unwrap();
    let mut child_env = Client::start_with_log(&fake_ra(), &log);
    let uri = child_env.open(&root, &text);
    child_env.request("textDocument/hover", Client::at(&uri, 4, 12));
    child_env.stop();
    let written = fs::read_to_string(&log).unwrap();
    assert!(written.contains("rust-analyzer started") && written.contains("-> textDocument/hover at .hrs 5:13 = .rs 6:13") && written.contains("<- textDocument/hover"), "{written}");
    let _ = fs::remove_dir_all(&root);
}


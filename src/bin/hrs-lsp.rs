// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `hrs-lsp`: the Harsh language server.
//!
//! One feature, layout-aware on-type formatting, and one custom request,
//! `harsh/columns`, behind it. The server keeps the text of each open file
//! and answers every question through `harsh_lang::columns`; it has no model of
//! names, types or projects. Design in `docs/LSP.md`.

use std::collections::HashMap;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use lsp_server::{Connection, Message, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument, Notification,
};
use lsp_types::request::{Completion, Formatting, GotoDefinition, HoverRequest, OnTypeFormatting, Request as _};
use lsp_types::{
    DocumentFormattingParams, DocumentOnTypeFormattingOptions, DocumentOnTypeFormattingParams,
    InitializeParams, OneOf, Position, Range, ServerCapabilities, TextDocumentPositionParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Url,
};
use serde::{Deserialize, Serialize};

/// `harsh/columns`: the legal starting columns for the line the cursor is on.
/// The client uses it to bind Tab and Shift-Tab; see `docs/LSP.md`.
const COLUMNS_REQUEST: &str = "harsh/columns";

#[derive(Serialize, Deserialize)]
struct ColumnsResult {
    legal: Vec<usize>,
    default: usize,
    unit: usize,
    /// The line's present indentation.
    current: usize,
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    if std::env::args().skip(1).any(|a| a == "--version" || a == "-V") {
        println!("hrs-lsp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let (connection, io_threads) = Connection::stdio();

    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(lsp_types::TextDocumentSyncOptions {
            open_close: Some(true),
            change: Some(TextDocumentSyncKind::FULL),
            save: Some(lsp_types::TextDocumentSyncSaveOptions::Supported(true)),
            ..Default::default()
        })),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        completion_provider: Some(lsp_types::CompletionOptions {
            trigger_characters: Some(vec![".".into()]),
            ..Default::default()
        }),
        document_on_type_formatting_provider: Some(DocumentOnTypeFormattingOptions {
            first_trigger_character: "\n".into(),
            more_trigger_character: Some(vec![")".into(), "]".into()]),
        }),
        // `textDocument/formatting` -- format on save -- is `hrs fmt` on
        // the buffer: one edit replacing the whole document.
        document_formatting_provider: Some(OneOf::Left(true)),
        ..Default::default()
    };
    let init = connection.initialize(serde_json::to_value(capabilities)?)?;
    let _params: InitializeParams = serde_json::from_value(init)?;

    let mut docs: HashMap<Url, String> = HashMap::new();
    let mut bridge = Bridge::default();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
                if req.method == HoverRequest::METHOD || req.method == GotoDefinition::METHOD || req.method == Completion::METHOD {
                    let result = match serde_json::from_value::<TextDocumentPositionParams>(req.params.clone()) {
                        Ok(p) if req.method == Completion::METHOD => bridge.complete(&docs, &connection, &p),
                        Ok(p) => bridge.ask(&docs, &connection, &req.method, &p),
                        Err(_) => Value::Null,
                    };
                    connection.sender.send(Message::Response(Response::new_ok(req.id, result)))?;
                    continue;
                }
                let resp = handle_request(&docs, req);
                connection.sender.send(Message::Response(resp))?;
            }
            Message::Notification(n) => match n.method.as_str() {
                DidOpenTextDocument::METHOD => {
                    let p: lsp_types::DidOpenTextDocumentParams =
                        serde_json::from_value(n.params)?;
                    docs.insert(p.text_document.uri, p.text_document.text);
                }
                DidChangeTextDocument::METHOD => {
                    let p: lsp_types::DidChangeTextDocumentParams =
                        serde_json::from_value(n.params)?;
                    // Full sync: the last change carries the whole text.
                    if let Some(c) = p.content_changes.into_iter().last() {
                        docs.insert(p.text_document.uri, c.text);
                    }
                }
                DidSaveTextDocument::METHOD => {
                    let p: lsp_types::DidSaveTextDocumentParams =
                        serde_json::from_value(n.params)?;
                    bridge.saved(&p.text_document.uri);
                }
                DidCloseTextDocument::METHOD => {
                    let p: lsp_types::DidCloseTextDocumentParams =
                        serde_json::from_value(n.params)?;
                    docs.remove(&p.text_document.uri);
                }
                _ => {}
            },
            Message::Response(_) => {}
        }
    }

    // The writer thread ends when the last sender is dropped; drop ours
    // before joining or the join waits forever.
    drop(connection);
    io_threads.join()?;
    Ok(())
}

fn handle_request(docs: &HashMap<Url, String>, req: Request) -> Response {
    match req.method.as_str() {
        OnTypeFormatting::METHOD => {
            let p: DocumentOnTypeFormattingParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return invalid(req.id, e),
            };
            let edits = on_type(docs, &p);
            Response::new_ok(req.id, edits)
        }
        Formatting::METHOD => {
            let p: DocumentFormattingParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return invalid(req.id, e),
            };
            Response::new_ok(req.id, format_document(docs, &p))
        }
        COLUMNS_REQUEST => {
            let p: TextDocumentPositionParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return invalid(req.id, e),
            };
            Response::new_ok(req.id, columns_at(docs, &p))
        }
        _ => Response::new_err(
            req.id,
            lsp_server::ErrorCode::MethodNotFound as i32,
            format!("unknown method {}", req.method),
        ),
    }
}

fn invalid(id: RequestId, e: serde_json::Error) -> Response {
    Response::new_err(id, lsp_server::ErrorCode::InvalidParams as i32, e.to_string())
}

/// The whole buffer through `hrs fmt`. No edits when nothing changes.
fn format_document(docs: &HashMap<Url, String>, p: &DocumentFormattingParams) -> Option<Vec<TextEdit>> {
    let src = docs.get(&p.text_document.uri)?;
    let out = harsh_lang::fmt::format(src);
    if out == *src {
        return Some(Vec::new());
    }
    let lines = src.split('\n').count() as u32;
    let last_len = src.split('\n').last().map_or(0, |l| l.chars().count()) as u32;
    Some(vec![TextEdit {
        range: Range::new(Position::new(0, 0), Position::new(lines.saturating_sub(1), last_len)),
        new_text: out,
    }])
}

/// Leading-space count of a document line.
fn indent_of(src: &str, line: u32) -> usize {
    let text = src.split('\n').nth(line as usize).unwrap_or("");
    text.chars().take_while(|c| *c == ' ').count()
}

fn columns_at(
    docs: &HashMap<Url, String>,
    p: &TextDocumentPositionParams,
) -> Option<ColumnsResult> {
    let src = docs.get(&p.text_document.uri)?;
    let line = p.position.line;
    let c = harsh_lang::columns::columns(src, line as usize);
    let current = indent_of(src, line);
    Some(ColumnsResult { legal: c.legal, default: c.default, unit: c.unit, current })
}

/// A trigger character has just been typed and the document already holds
/// it. Enter: replace the new line's indentation (whatever the editor's own
/// rules put there) with the layout's default column. `)` or `]` typed as
/// the first character of a line: move the line under the anchor of the
/// group it closes. Either way one edit to the leading whitespace only, so
/// the keystroke and the re-indent are a single undo step, and text the
/// editor carried onto the line stays where it is.
fn on_type(
    docs: &HashMap<Url, String>,
    p: &DocumentOnTypeFormattingParams,
) -> Option<Vec<TextEdit>> {
    let src = docs.get(&p.text_document_position.text_document.uri)?;
    let pos = p.text_document_position.position;
    let ws = indent_of(src, pos.line);
    let c = harsh_lang::columns::columns(src, pos.line as usize);
    let target = match p.ch.as_str() {
        "\n" => c.default,
        ")" | "]" => {
            // Only a closer that begins its line is ours; one ending an
            // expression (`x * 10)`) is the compact form and stays.
            if pos.character as usize != ws + 1 {
                return Some(Vec::new());
            }
            c.closer?
        }
        _ => return None,
    };
    let mut edits = Vec::new();
    if ws != target {
        edits.push(reindent(pos.line, ws, target));
    }
    // Enter between an auto-closed pair: the editor has put the closer on
    // the line below, never typed, so the closer trigger never fires. When
    // that line is nothing but a `)` or `]`, place it under its anchor too.
    if p.ch == "\n" {
        let next = pos.line + 1;
        let text = src.split('\n').nth(next as usize).unwrap_or("").trim_end_matches('\r');
        let rest = text.trim_start_matches(' ');
        if rest == ")" || rest == "]" {
            let nws = text.len() - rest.len();
            if let Some(cl) = harsh_lang::columns::columns(src, next as usize).closer {
                if cl != nws {
                    edits.push(reindent(next, nws, cl));
                }
            }
        }
    }
    Some(edits)
}

fn reindent(line: u32, ws: usize, target: usize) -> TextEdit {
    TextEdit {
        range: Range::new(Position::new(line, 0), Position::new(line, ws as u32)),
        new_text: " ".repeat(target),
    }
}

// ------------------------------------------------ rust-analyzer behind it
//
// Hover and go-to-definition (LSP-RA-DESIGN, 2026-09-25; built 2026-09-27
// with the user's three rulings: the distribution shown to rust-analyzer by
// `hrs`, answers from the last good translation for unchanged lines, hover and
// definition first). `hrs-lsp` learns no names: it asks rust-analyzer about
// the Rust a `.hrs` became and maps positions both ways through the source
// maps -- `.hrs` to `.rs` with `SourceMap::find`, back with `locate`.

/// One rust-analyzer, a child process spoken to over its stdin and stdout.
struct Ra {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    rx: Receiver<Value>,
    next: i64,
    /// Set when rust-analyzer says it has finished loading the project
    /// (`experimental/serverStatus`, quiescent): until then it answers
    /// questions with nothing, so they wait for it -- a minute at most.
    ready: Arc<AtomicBool>,
    started: Instant,
    version: i64,
    /// The generated files handed to rust-analyzer, with the text given.
    /// rust-analyzer loads nothing under a project's `target/` from disk --
    /// the generated Rust included -- and answered every question about it
    /// with nothing (found 2026-09-27 on the user's Mac: `HRS_LSP_LOG` showed
    /// each hover mapped right and answered `null`; the same file opened in
    /// VS Code was answered). So `hrs-lsp` hands it the text, as an editor
    /// hands it an open file.
    given: HashMap<Url, String>,
}

/// A line in the log, when `HRS_LSP_LOG` names a file: what `hrs-lsp` asked
/// rust-analyzer and what came back, so a silent answer explains itself.
fn log(msg: &str) {
    if let Ok(path) = std::env::var("HRS_LSP_LOG") {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "{msg}");
        }
    }
}

/// The rust-analyzers to try, in order: `HRS_RA` alone when set; else the
/// one on the PATH (rustup's), then the one inside an editor's rust-analyzer
/// extension -- VS Code's, Insiders', Cursor's, VSCodium's -- the newest.
fn candidates() -> Vec<PathBuf> {
    if let Ok(p) = std::env::var("HRS_RA") {
        return vec![PathBuf::from(p)];
    }
    let mut v = vec![PathBuf::from("rust-analyzer")];
    if let Ok(home) = std::env::var("HOME") {
        for dir in [".vscode/extensions", ".vscode-insiders/extensions", ".cursor/extensions", ".vscode-oss/extensions"] {
            let Ok(entries) = std::fs::read_dir(Path::new(&home).join(dir)) else { continue };
            let mut found: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.file_name().map_or(false, |n| n.to_string_lossy().starts_with("rust-lang.rust-analyzer-")))
                .map(|p| p.join("server").join("rust-analyzer"))
                .filter(|p| p.is_file())
                .collect();
            found.sort();
            if let Some(newest) = found.pop() {
                v.push(newest);
            }
        }
    }
    v
}

fn frame(v: &Value) -> Vec<u8> {
    let body = v.to_string();
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body).into_bytes()
}

fn read_frame(r: &mut impl BufRead) -> Option<Value> {
    let mut len = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let t = line.trim_end();
        if t.is_empty() {
            break;
        }
        if let Some(v) = t.strip_prefix("Content-Length:") {
            len = v.trim().parse::<usize>().ok();
        }
    }
    let mut buf = vec![0; len?];
    r.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

impl Ra {
    /// rust-analyzer on a project: `HRS_RA` names it (the tests' fake), else
    /// `rust-analyzer` on the PATH. The distribution's patches (the crates
    /// shipped inside `hrs`) are handed to its cargo as `--config` arguments,
    /// so a project naming `hrs_std = "0.1"` resolves for it as for `hrs`.
    fn start(root: &Path) -> Result<Ra, String> {
        let mut last = String::from("no rust-analyzer found");
        for exe in candidates() {
            match Ra::start_with(root, &exe) {
                Ok(ra) => {
                    log(&format!("rust-analyzer started: {}", exe.display()));
                    return Ok(ra);
                }
                Err(e) => {
                    log(&format!("rust-analyzer {} did not start: {e}", exe.display()));
                    last = e;
                }
            }
        }
        Err(last)
    }

    fn start_with(root: &Path, exe: &Path) -> Result<Ra, String> {
        let exe = exe.display().to_string();
        let mut child = Command::new(&exe)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("{exe}: {e}"))?;
        let stdin = Arc::new(Mutex::new(child.stdin.take().ok_or("no stdin")?));
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let (tx, rx) = channel();
        let writer = stdin.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let ready_in = ready.clone();
        std::thread::spawn(move || {
            let mut r = BufReader::new(stdout);
            while let Some(v) = read_frame(&mut r) {
                if v["method"] == "experimental/serverStatus" && v["params"]["quiescent"] == json!(true) {
                    ready_in.store(true, Ordering::SeqCst);
                }
                // A request from rust-analyzer: answered with nothing, as an
                // editor that asked for no such features would.
                if v.get("method").is_some() {
                    if let Some(id) = v.get("id") {
                        if let Ok(mut w) = writer.lock() {
                            let _ = w.write_all(&frame(&json!({"jsonrpc": "2.0", "id": id, "result": null})));
                        }
                    }
                    continue;
                }
                if v.get("id").is_some() && tx.send(v).is_err() {
                    break;
                }
            }
        });
        let mut ra = Ra { child, stdin, rx, next: 0, ready, started: Instant::now(), version: 0, given: HashMap::new() };
        let root_uri = Url::from_directory_path(root).map_err(|_| "the project's root is not a path".to_string())?;
        // Harsh's standard distribution, only for a project that names it.
        let patches = harsh_lang::driver::distribution(root, &[root.join("Cargo.toml")]).unwrap_or_default();
        let options = if patches.is_empty() { json!({}) } else { json!({"cargo": {"extraArgs": patches}}) };
        log(&format!("initialize {} with {options}", root.display()));
        ra.request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {"experimental": {"serverStatusNotification": true}},
                "initializationOptions": options,
            }),
            120,
        )?;
        ra.notify("initialized", json!({}))?;
        Ok(ra)
    }

    fn send(&self, v: Value) -> Result<(), String> {
        let mut w = self.stdin.lock().map_err(|_| "rust-analyzer's input is poisoned".to_string())?;
        w.write_all(&frame(&v)).and_then(|_| w.flush()).map_err(|e| e.to_string())
    }

    fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}))
    }

    /// Hand rust-analyzer the text of every generated Rust file of the
    /// project (`target/hrs/**/*.rs`, the distribution's crates included, the
    /// completion side-file not), opening the new and updating the changed.
    fn give(&mut self, root: &Path) {
        let mut files = Vec::new();
        let mut stack = vec![root.join("target/hrs")];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    if p.file_name().map_or(true, |n| n != ".complete" && n != "target") {
                        stack.push(p);
                    }
                } else if p.extension().map_or(false, |x| x == "rs") {
                    files.push(p);
                }
            }
        }
        let (mut opened, mut changed) = (0, 0);
        for f in files {
            let (Ok(uri), Ok(text)) = (Url::from_file_path(&f), std::fs::read_to_string(&f)) else { continue };
            self.version += 1;
            let v = self.version;
            match self.given.get(&uri) {
                None => {
                    let _ = self.notify("textDocument/didOpen", json!({"textDocument": {"uri": uri, "languageId": "rust", "version": v, "text": text}}));
                    opened += 1;
                }
                Some(old) if *old != text => {
                    let _ = self.notify("textDocument/didChange", json!({"textDocument": {"uri": uri, "version": v}, "contentChanges": [{"text": text}]}));
                    changed += 1;
                }
                Some(_) => continue,
            }
            self.given.insert(uri, text);
        }
        log(&format!("gave rust-analyzer the generated Rust: {opened} opened, {changed} updated"));
    }

    /// One file's text, as rust-analyzer should see it now (completion's
    /// marked text, then the file's own again).
    fn set_text(&mut self, uri: &Url, text: &str) -> Result<(), String> {
        self.version += 1;
        let v = self.version;
        if self.given.contains_key(uri) {
            self.notify("textDocument/didChange", json!({"textDocument": {"uri": uri, "version": v}, "contentChanges": [{"text": text}]}))
        } else {
            self.given.insert(uri.clone(), text.to_string());
            self.notify("textDocument/didOpen", json!({"textDocument": {"uri": uri, "languageId": "rust", "version": v, "text": text}}))
        }
    }

    /// Wait for rust-analyzer to finish loading the project, a minute at most.
    fn wait_ready(&self) {
        while !self.ready.load(Ordering::SeqCst) && self.started.elapsed() < Duration::from_secs(60) {
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    fn request(&mut self, method: &str, params: Value, secs: u64) -> Result<Value, String> {
        self.next += 1;
        let id = self.next;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        loop {
            let v = self.rx.recv_timeout(Duration::from_secs(secs)).map_err(|_| format!("rust-analyzer did not answer {method}"))?;
            if v["id"] == json!(id) {
                let result = v.get("result").cloned().unwrap_or(Value::Null);
                let shown = result.to_string();
                log(&format!("<- {method}: {}", if shown.len() > 300 { &shown[..300] } else { &shown }));
                return Ok(result);
            }
        }
    }
}

impl Drop for Ra {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// The rust-analyzers, one per project, and each file's last good translation.
#[derive(Default)]
struct Bridge {
    ras: HashMap<PathBuf, Ra>,
    /// Each `.hrs` as it was when it last transpiled: a line unchanged since
    /// is answered from that translation; a changed one is not answered.
    good: HashMap<Url, String>,
    told_missing: bool,
}

/// The project a file belongs to: the nearest folder above it with a `Cargo.toml`.
fn root_of(file: &Path) -> Option<PathBuf> {
    file.ancestors().skip(1).find(|d| d.join("Cargo.toml").is_file()).map(Path::to_path_buf)
}

/// Where `hrs` writes a source's Rust and its map: `src/a/b.hrs` ->
/// `target/hrs/a/b.rs` and `target/hrs/a/b.map.json`.
fn generated(root: &Path, file: &Path) -> Option<(PathBuf, PathBuf)> {
    let rel = file.strip_prefix(root.join("src")).ok()?;
    let gen = root.join("target/hrs").join(rel);
    Some((gen.with_extension("rs"), gen.with_extension("map.json")))
}

/// The `hrs` beside this `hrs-lsp` (they are installed together), else the PATH's.
fn hrs_exe() -> PathBuf {
    if let Ok(p) = std::env::var("HRS") {
        return PathBuf::from(p);
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("hrs")))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("hrs"))
}

/// A range in a generated `.rs`, back to its `.hrs` (0-based, as LSP counts).
fn range_back(m: &harsh_lang::remap::SourceMap, r: &Value) -> Option<Value> {
    let at = |p: &Value| -> Option<Value> {
        let (l, c) = m.locate(p["line"].as_u64()? as usize + 1, p["character"].as_u64()? as usize + 1)?;
        Some(json!({"line": l - 1, "character": c - 1}))
    };
    Some(json!({"start": at(&r["start"])?, "end": at(&r["end"])?}))
}

impl Bridge {
    /// A save: the project is transpiled again, as `hrs build` would, and a
    /// good translation becomes the one answers come from.
    fn saved(&mut self, uri: &Url) {
        let Some(file) = uri.to_file_path().ok() else { return };
        let Some(root) = root_of(&file) else { return };
        let ok = Command::new(hrs_exe())
            .args(["cargo", "metadata", "--no-deps", "--format-version", "1"])
            .current_dir(&root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_or(false, |s| s.success());
        if !ok {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(&file) {
            self.good.insert(uri.clone(), text);
        }
        if let Some(ra) = self.ras.get_mut(&root) {
            ra.give(&root);
        }
    }

    /// Hover or go-to-definition at a place in a `.hrs`: asked of
    /// rust-analyzer at the place in the `.rs` it became, the answer mapped
    /// back. Nothing, rather than something wrong, when the line changed
    /// since the last good translation or no Rust came from the place.
    fn ask(&mut self, docs: &HashMap<Url, String>, conn: &Connection, method: &str, p: &TextDocumentPositionParams) -> Value {
        self.try_ask(docs, conn, method, p).unwrap_or(Value::Null)
    }

    fn try_ask(&mut self, docs: &HashMap<Url, String>, conn: &Connection, method: &str, p: &TextDocumentPositionParams) -> Option<Value> {
        let uri = &p.text_document.uri;
        let file = uri.to_file_path().ok()?;
        let root = root_of(&file)?;
        let (rs, map) = generated(&root, &file)?;
        let line = p.position.line as usize;
        let now = docs.get(uri).cloned().or_else(|| std::fs::read_to_string(&file).ok())?;
        let then = match self.good.get(uri) {
            Some(t) => t.clone(),
            None => std::fs::read_to_string(&file).ok()?,
        };
        if now.lines().nth(line) != then.lines().nth(line) {
            return None;
        }
        let m = harsh_lang::remap::SourceMap::load(map.to_str()?).ok()?;
        let Some((rl, rc)) = m.find(line + 1, p.position.character as usize + 1) else {
            log(&format!("{method} at {}:{}: no Rust came from there", line + 1, p.position.character + 1));
            return None;
        };
        log(&format!("-> {method} at .hrs {}:{} = .rs {rl}:{rc}", line + 1, p.position.character + 1));
        self.ensure(&root, conn)?;
        let params = json!({
            "textDocument": {"uri": Url::from_file_path(&rs).ok()?},
            "position": {"line": rl - 1, "character": rc - 1},
        });
        let ra = self.ras.get_mut(&root)?;
        ra.wait_ready();
        let answer = match ra.request(method, params, 30) {
            Ok(v) => v,
            Err(e) => {
                log(&format!("{method}: {e}; rust-analyzer restarted on the next request"));
                self.ras.remove(&root);
                return None;
            }
        };
        self.shape(method, &root, &m, answer)
    }

    /// rust-analyzer for a project, started on first need; a missing one said once.
    fn ensure(&mut self, root: &Path, conn: &Connection) -> Option<()> {
        if !self.ras.contains_key(root) {
            match Ra::start(root) {
                Ok(mut ra) => {
                    ra.give(root);
                    self.ras.insert(root.to_path_buf(), ra);
                }
                Err(e) => {
                    if !self.told_missing {
                        self.told_missing = true;
                        let msg = format!("hover, go-to-definition and completion need rust-analyzer ({e}): `rustup component add rust-analyzer`, or the rust-analyzer extension in VS Code");
                        let _ = conn.sender.send(Message::Notification(lsp_server::Notification::new(
                            "window/showMessage".into(),
                            json!({"type": 2, "message": msg}),
                        )));
                    }
                    return None;
                }
            }
        }
        Some(())
    }

    /// An answer's positions, back from the `.rs` to the `.hrs`.
    fn shape(&self, method: &str, root: &Path, m: &harsh_lang::remap::SourceMap, answer: Value) -> Option<Value> {
        if method == HoverRequest::METHOD {
            let mut h = answer;
            if h.is_null() {
                return None;
            }
            let back = h.get("range").and_then(|r| range_back(m, r));
            match back {
                Some(r) => h["range"] = r,
                None => {
                    if let Some(o) = h.as_object_mut() {
                        o.remove("range");
                    }
                }
            }
            Some(h)
        } else {
            Some(self.locations_back(root, answer))
        }
    }

    /// Definition answers: a place in the project's generated Rust becomes
    /// its place in the `.hrs`; any other place -- the standard library, a
    /// Rust dependency -- is a Rust file, returned as it is.
    fn locations_back(&self, root: &Path, v: Value) -> Value {
        let one = |loc: Value| -> Value {
            let (uri_key, range_key) = if loc.get("targetUri").is_some() { ("targetUri", "targetRange") } else { ("uri", "range") };
            let Some(path) = loc[uri_key].as_str().and_then(|u| Url::parse(u).ok()).and_then(|u| u.to_file_path().ok()) else { return loc };
            if !path.starts_with(root.join("target/hrs")) || path.extension().map_or(true, |e| e != "rs") {
                return loc;
            }
            let Ok(m) = harsh_lang::remap::SourceMap::load(path.with_extension("map.json").to_str().unwrap_or("")) else { return loc };
            let Some(range) = range_back(&m, &loc[range_key]) else { return loc };
            let Ok(src) = Url::from_file_path(m.source()) else { return loc };
            if uri_key == "uri" {
                json!({"uri": src, "range": range})
            } else {
                let sel = range_back(&m, &loc["targetSelectionRange"]).unwrap_or_else(|| range.clone());
                json!({"targetUri": src, "targetRange": range, "targetSelectionRange": sel})
            }
        };
        match v {
            Value::Array(items) => Value::Array(items.into_iter().map(one).collect()),
            Value::Null => Value::Null,
            loc => one(loc),
        }
    }

    /// Completion, at the text as it is now -- being typed, so it cannot wait
    /// for a save. A marker is put at the cursor, making `x <- ` a complete
    /// `x <- MARK` (rust-analyzer does the same inside); the text is
    /// transpiled on the side; rust-analyzer is shown that Rust in memory,
    /// asked at the marker, and set back to the file on disk. Its answers are
    /// Rust's names, given as plain names: Rust's snippets (`len()`) are not
    /// Harsh's calls (`len$`).
    fn complete(&mut self, docs: &HashMap<Url, String>, conn: &Connection, p: &TextDocumentPositionParams) -> Value {
        self.try_complete(docs, conn, p).unwrap_or(Value::Null)
    }

    fn try_complete(&mut self, docs: &HashMap<Url, String>, conn: &Connection, p: &TextDocumentPositionParams) -> Option<Value> {
        const MARK: &str = "hrsCompletionMark";
        let uri = &p.text_document.uri;
        let file = uri.to_file_path().ok()?;
        let root = root_of(&file)?;
        let (rs, _) = generated(&root, &file)?;
        let now = docs.get(uri).cloned().or_else(|| std::fs::read_to_string(&file).ok())?;
        let (line, col) = (p.position.line as usize, p.position.character as usize);
        let start = now.split_inclusive('\n').take(line).map(str::len).sum::<usize>();
        let at = start + now[start..].chars().take(col).map(char::len_utf8).sum::<usize>();
        let marked = format!("{}{MARK}{}", &now[..at], &now[at..]);
        let side = root.join("target/hrs/.complete");
        std::fs::create_dir_all(&side).ok()?;
        let (hrs, rs_side, map) = (side.join("cell.hrs"), side.join("cell.rs"), side.join("cell.map.json"));
        std::fs::write(&hrs, &marked).ok()?;
        let out = Command::new(hrs_exe()).arg(&hrs).arg("-o").arg(&rs_side).arg("--map").arg(&map).output().ok()?;
        if !out.status.success() {
            log(&format!("completion at {}:{}: the text does not transpile: {}", line + 1, col + 1, String::from_utf8_lossy(&out.stderr).lines().next().unwrap_or("")));
            return None;
        }
        let m = harsh_lang::remap::SourceMap::load(map.to_str()?).ok()?;
        let (rl, rc) = m.find(line + 1, col + 1)?;
        let rust = std::fs::read_to_string(&rs_side).ok()?;
        log(&format!("-> completion at .hrs {}:{} = .rs {rl}:{rc}", line + 1, col + 1));
        self.ensure(&root, conn)?;
        let rs_uri = Url::from_file_path(&rs).ok()?;
        let on_disk = std::fs::read_to_string(&rs).unwrap_or_default();
        let ra = self.ras.get_mut(&root)?;
        ra.wait_ready();
        ra.set_text(&rs_uri, &rust).ok()?;
        let answer = ra.request("textDocument/completion", json!({"textDocument": {"uri": rs_uri}, "position": {"line": rl - 1, "character": rc - 1}}), 30);
        let _ = ra.set_text(&rs_uri, &on_disk);
        let answer = match answer {
            Ok(v) => v,
            Err(e) => {
                log(&format!("completion: {e}; rust-analyzer restarted on the next request"));
                self.ras.remove(&root);
                return None;
            }
        };
        let items = match answer {
            Value::Array(a) => a,
            v => v.get("items").and_then(|i| i.as_array()).cloned().unwrap_or_default(),
        };
        let items: Vec<Value> = items
            .into_iter()
            .filter_map(|it| {
                let label = it["label"].as_str()?.to_string();
                let name = label.split(['(', ' ']).next().unwrap_or(&label).trim_end_matches('!').to_string();
                if name.is_empty() || name.contains(MARK) {
                    return None;
                }
                let mut o = serde_json::Map::new();
                o.insert("label".into(), json!(label));
                o.insert("insertText".into(), json!(name));
                o.insert("filterText".into(), json!(name));
                for k in ["kind", "detail", "documentation", "sortText", "deprecated"] {
                    if let Some(v) = it.get(k) {
                        o.insert(k.into(), v.clone());
                    }
                }
                Some(Value::Object(o))
            })
            .collect();
        Some(json!({"isIncomplete": true, "items": items}))
    }
}


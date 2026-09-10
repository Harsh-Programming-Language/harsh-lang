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

use lsp_server::{Connection, Message, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
};
use lsp_types::request::{Formatting, OnTypeFormatting, Request as _};
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
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
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

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
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

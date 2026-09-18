//! Drives `hrs-lsp` over stdio the way an editor does: initialize, open a
//! document, press Enter (on-type formatting with `\n`), ask for the legal
//! columns, shut down. Checks the wire shape, not just the library.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl Client {
    fn spawn() -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hrs-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn hrs-lsp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Client { child, stdin, stdout, next_id: 1 }
    }

    fn send(&mut self, v: Value) {
        let s = v.to_string();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", s.len(), s).unwrap();
        self.stdin.flush().unwrap();
    }

    fn recv(&mut self) -> Value {
        let mut len = 0usize;
        loop {
            let mut line = String::new();
            let n = self.stdout.read_line(&mut line).unwrap();
            assert!(n > 0, "server closed its stdout");
            if line == "\r\n" {
                break;
            }
            if let Some(n) = line.strip_prefix("Content-Length: ") {
                len = n.trim().parse().unwrap();
            }
        }
        let mut buf = vec![0u8; len];
        self.stdout.read_exact(&mut buf).unwrap();
        serde_json::from_slice(&buf).unwrap()
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let v = self.recv();
            if v["id"] == json!(id) {
                return v;
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }
}

const URI: &str = "file:///tmp/session.hrs";

fn open(c: &mut Client, text: &str) {
    c.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": URI, "languageId": "harsh", "version": 1, "text": text}}),
    );
}

fn enter(c: &mut Client, line: u32, character: u32) -> Value {
    c.request(
        "textDocument/onTypeFormatting",
        json!({
            "textDocument": {"uri": URI},
            "position": {"line": line, "character": character},
            "ch": "\n",
            "options": {"tabSize": 4, "insertSpaces": true}
        }),
    )
}

#[test]
fn session() {
    let mut c = Client::spawn();

    let init = c.request("initialize", json!({"capabilities": {}}));
    let caps = &init["result"]["capabilities"];
    assert_eq!(caps["documentOnTypeFormattingProvider"]["firstTriggerCharacter"], "\n");
    assert_eq!(caps["documentOnTypeFormattingProvider"]["moreTriggerCharacter"], json!([")", "]"]));
    assert_eq!(caps["textDocumentSync"], 1, "full sync");
    assert_eq!(caps["documentFormattingProvider"], true, "format on save");
    c.notify("initialized", json!({}));

    // `textDocument/formatting` is `hrs fmt` on the buffer: a misindented
    // body comes back as one whole-document edit; a formatted one, as none.
    let doc = "fn main$:\n  let x = 1\n  x\n";
    open(&mut c, doc);
    let r = c.request(
        "textDocument/formatting",
        json!({"textDocument": {"uri": URI}, "options": {"tabSize": 4, "insertSpaces": true}}),
    );
    let edits = r["result"].as_array().expect("edits").clone();
    assert_eq!(edits.len(), 1, "{r}");
    assert_eq!(edits[0]["newText"], "fn main$:\n    let x = 1\n    x\n");
    assert_eq!(edits[0]["range"]["start"], json!({"line": 0, "character": 0}));
    open(&mut c, "fn main$:\n    let x = 1\n    x\n");
    let r = c.request(
        "textDocument/formatting",
        json!({"textDocument": {"uri": URI}, "options": {"tabSize": 4, "insertSpaces": true}}),
    );
    assert_eq!(r["result"], json!([]));

    // Enter after `<- map (...)`: the editor has inserted the newline and
    // copied the previous line's indent (10 spaces, as VSCode would). The
    // server must leave it alone -- an empty edit list.
    let doc = "fn main$:\n    let d: Vec<i32> =\n        v <- iter$\n          <- map (|x| x * 10)\n          \n";
    open(&mut c, doc);
    let r = enter(&mut c, 4, 10);
    assert_eq!(r["result"], json!([]), "already at the arrow column: {r}");

    // Same document, but the editor put the new line at the previous line's
    // statement indent (8). The server moves it under the arrow.
    let doc = "fn main$:\n    let d: Vec<i32> =\n        v <- iter$\n          <- map (|x| x * 10)\n        \n";
    c.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 2}, "contentChanges": [{"text": doc}]}),
    );
    let r = enter(&mut c, 4, 8);
    let edit = &r["result"][0];
    assert_eq!(edit["newText"], "          ", "{r}");
    assert_eq!(edit["range"]["start"], json!({"line": 4, "character": 0}));
    assert_eq!(edit["range"]["end"], json!({"line": 4, "character": 8}));

    // Enter after `let d =` lands one level in.
    let doc = "fn main$:\n    let d =\n\n";
    c.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 3}, "contentChanges": [{"text": doc}]}),
    );
    let r = enter(&mut c, 2, 0);
    assert_eq!(r["result"][0]["newText"], "        ", "{r}");

    // Enter after `println!` -- a macro with its arguments still to come --
    // lands on the first argument's column; after that argument, on its
    // sibling's, the same column.
    let doc = "fn main$:\n    println!\n\n";
    c.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 4}, "contentChanges": [{"text": doc}]}),
    );
    let r = enter(&mut c, 2, 0);
    assert_eq!(r["result"][0]["newText"], "        ", "after the macro: {r}");
    let doc = "fn main$:\n    println!\n        \"{} {}\"\n\n";
    c.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 5}, "contentChanges": [{"text": doc}]}),
    );
    let r = enter(&mut c, 3, 0);
    assert_eq!(r["result"][0]["newText"], "        ", "after the first argument: {r}");

    // The custom request: legal columns for Tab.
    let doc = "fn main$:\n    let n = v <- len$\n    \n";
    c.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 4}, "contentChanges": [{"text": doc}]}),
    );
    let r = c.request(
        "harsh/columns",
        json!({"textDocument": {"uri": URI}, "position": {"line": 2, "character": 4}}),
    );
    assert_eq!(r["result"]["legal"], json!([0, 4, 8, 14]), "{r}");
    assert_eq!(r["result"]["default"], 4);
    assert_eq!(r["result"]["current"], 4);
    assert_eq!(r["result"]["unit"], 4);

    // `)` typed at the start of a line lands under the callee.
    let doc = "fn main$:\n    let t = compute (\n                a\n                )\n";
    c.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 5}, "contentChanges": [{"text": doc}]}),
    );
    let r = c.request(
        "textDocument/onTypeFormatting",
        json!({
            "textDocument": {"uri": URI},
            "position": {"line": 3, "character": 17},
            "ch": ")",
            "options": {"tabSize": 4, "insertSpaces": true}
        }),
    );
    assert_eq!(r["result"][0]["newText"], "            ", "{r}");
    assert_eq!(r["result"][0]["range"]["end"], json!({"line": 3, "character": 16}));

    // Enter between an auto-closed pair: the editor has already placed the
    // `)` on the line below at the statement's indent; both lines move.
    let doc = "fn main$:\n    let t = compute (\n    \n    )\n";
    c.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 7}, "contentChanges": [{"text": doc}]}),
    );
    let r = enter(&mut c, 2, 4);
    assert_eq!(r["result"][0]["newText"], "                ", "{r}");
    assert_eq!(r["result"][1]["newText"], "            ", "{r}");
    assert_eq!(r["result"][1]["range"]["start"], json!({"line": 3, "character": 0}));
    assert_eq!(r["result"][1]["range"]["end"], json!({"line": 3, "character": 4}));

    // The compact form: a `)` ending an expression is left alone.
    let doc = "fn main$:\n    let d =\n        v <- map (|x|:\n                      x * 10)\n";
    c.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 6}, "contentChanges": [{"text": doc}]}),
    );
    let r = c.request(
        "textDocument/onTypeFormatting",
        json!({
            "textDocument": {"uri": URI},
            "position": {"line": 3, "character": 29},
            "ch": ")",
            "options": {"tabSize": 4, "insertSpaces": true}
        }),
    );
    assert_eq!(r["result"], json!([]), "{r}");

    // A trigger the server did not ask for is answered with null.
    let r = c.request(
        "textDocument/onTypeFormatting",
        json!({
            "textDocument": {"uri": URI},
            "position": {"line": 2, "character": 4},
            "ch": "}",
            "options": {"tabSize": 4, "insertSpaces": true}
        }),
    );
    assert_eq!(r["result"], Value::Null);

    let r = c.request("shutdown", Value::Null);
    assert_eq!(r["result"], Value::Null);
    c.notify("exit", Value::Null);
    let status = c.child.wait().unwrap();
    assert!(status.success(), "clean exit: {status}");
}

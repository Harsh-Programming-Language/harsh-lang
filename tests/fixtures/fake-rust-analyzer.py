#!/usr/bin/env python3
"""A fake rust-analyzer for the suite (tests/lsp.rs): just enough LSP to
answer hover and definition about the generated Rust, so the translation of
positions both ways is tested without rust-analyzer installed.

Hover answers with the very `.rs` position it was asked about, so the test
sees what `hrs-lsp` sent; definition points at `fn helper` in the same file.

Like the real one, it loads nothing under a project's `target/` from disk: it
knows such a file only once it is handed its text (`didOpen`), and answers
null about it otherwise -- the cause of empty hovers found on the user's Mac,
2026-09-27."""
import json, sys, urllib.parse

def read():
    length = None
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        line = line.decode().strip()
        if not line:
            break
        if line.lower().startswith("content-length:"):
            length = int(line.split(":")[1])
    return json.loads(sys.stdin.buffer.read(length))

def send(v):
    body = json.dumps(v).encode()
    sys.stdout.buffer.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
    sys.stdout.buffer.flush()

opened = {}

while True:
    m = read()
    if m is None or m.get("method") == "exit":
        break
    if m.get("method") == "initialized":
        # As rust-analyzer does once the project is loaded.
        send({"jsonrpc": "2.0", "method": "experimental/serverStatus", "params": {"health": "ok", "quiescent": True}})
    if m.get("method") == "textDocument/didOpen":
        td = m["params"]["textDocument"]
        opened[td["uri"]] = td["text"]
    elif m.get("method") == "textDocument/didChange":
        opened[m["params"]["textDocument"]["uri"]] = m["params"]["contentChanges"][-1]["text"]
    elif m.get("method") == "textDocument/didClose":
        opened.pop(m["params"]["textDocument"]["uri"], None)
    if "id" not in m:
        continue
    method, p = m["method"], m.get("params") or {}
    result = None
    if method == "initialize":
        result = {"capabilities": {"hoverProvider": True, "definitionProvider": True}}
    elif method in ("textDocument/hover", "textDocument/definition") and "/target/" in p["textDocument"]["uri"] and p["textDocument"]["uri"] not in opened:
        result = None
    elif method == "textDocument/hover":
        pos, uri = p["position"], p["textDocument"]["uri"]
        name = uri.rsplit("/", 1)[1]
        result = {"contents": {"kind": "plaintext", "value": f"fake hover at {pos['line']}:{pos['character']} in {name}"},
                  "range": {"start": pos, "end": {"line": pos["line"], "character": pos["character"] + 6}}}
    elif method == "textDocument/definition":
        uri = p["textDocument"]["uri"]
        for i, line in enumerate(opened[uri].split("\n")):
            if line.startswith("fn helper"):
                result = {"uri": uri, "range": {"start": {"line": i, "character": 3}, "end": {"line": i, "character": 9}}}
    elif method == "textDocument/completion":
        pos = p["position"]
        result = [{"label": f"at {pos['line']}:{pos['character']}"}, {"label": "len()", "kind": 2}, {"label": "hrsCompletionMark"}]
    send({"jsonrpc": "2.0", "id": m["id"], "result": result})

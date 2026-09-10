// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// Launches `hrs-lsp` and binds Tab / Shift-Tab to its `harsh/columns`
// request. Enter needs no code here: the server advertises on-type
// formatting on `\n` and VSCode calls it, given `editor.formatOnType`,
// which package.json turns on for Harsh.

const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const command = vscode.workspace.getConfiguration("harsh").get("serverPath") || "hrs-lsp";
  const serverOptions = {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio },
  };
  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "harsh" }],
  };
  client = new LanguageClient("harsh", "Harsh", serverOptions, clientOptions);
  client.start().catch((e) => {
    vscode.window.showWarningMessage(
      `Harsh: could not start ${command} (${e.message}). Install it with \`cargo install --path .\` in the hrust checkout, or set harsh.serverPath.`
    );
  });

  context.subscriptions.push(
    vscode.commands.registerCommand("harsh.tab", () => cycle(+1)),
    vscode.commands.registerCommand("harsh.shiftTab", () => cycle(-1)),
    vscode.commands.registerCommand("harsh.enter", () => enter())
  );
}

// Move the line's indentation to the next (or previous) legal column. When
// the cursor is not in the leading whitespace, or the server is not there,
// fall back to VSCode's own Tab / Outdent.
async function cycle(dir) {
  const editor = vscode.window.activeTextEditor;
  const fallback = () => vscode.commands.executeCommand(dir > 0 ? "tab" : "outdent");
  if (!editor || editor.selections.length !== 1 || !editor.selection.isEmpty) return fallback();
  const pos = editor.selection.active;
  const line = editor.document.lineAt(pos.line);
  const ws = line.firstNonWhitespaceCharacterIndex;
  if (pos.character > ws) return fallback();
  if (!client || !client.isRunning()) return fallback();

  let r;
  try {
    r = await client.sendRequest("harsh/columns", {
      textDocument: { uri: editor.document.uri.toString() },
      position: { line: pos.line, character: pos.character },
    });
  } catch {
    return fallback();
  }
  if (!r || !r.legal || r.legal.length === 0) return fallback();

  // Tab: the next legal column to the right; past the last one, a unit
  // further, without limit -- a more-indented line always continues, so
  // there is no right boundary.
  // Shift-Tab: the previous legal column when one lies within a unit --
  // the earlier arrows of a chain line -- otherwise a unit back; to 0.
  const legal = r.legal;
  const unit = r.unit || 4;
  let target;
  if (dir > 0) {
    target = legal.find((c) => c > ws);
    if (target === undefined) target = ws + unit;
  } else {
    const below = legal.filter((c) => c < ws);
    const onGrid = below.length ? below[below.length - 1] : -1;
    target = onGrid >= ws - unit ? onGrid : Math.max(ws - unit, 0);
  }
  if (target === ws) return;

  const range = new vscode.Range(pos.line, 0, pos.line, ws);
  await editor.edit((b) => b.replace(range, " ".repeat(target)), {
    undoStopBefore: false,
    undoStopAfter: false,
  });
  const p = new vscode.Position(pos.line, target);
  editor.selection = new vscode.Selection(p, p);
}

// Enter, asked of the server before the newline exists, so the cursor lands
// once. VSCode's own Enter runs first and places the cursor by its indent
// rules; the server's on-type edit then corrects it, and where the two
// disagree (after a closing bracket, say) the correction is visible. Asking
// first removes the first landing. Falls back to VSCode's Enter whenever
// the cursor is not at the end of its line -- the auto-closed `(|)` flow
// needs VSCode's bracket splitting, and the on-type edits place that one.
async function enter() {
  const editor = vscode.window.activeTextEditor;
  const fallback = () => vscode.commands.executeCommand("type", { text: "\n" });
  if (!editor || editor.selections.length !== 1 || !editor.selection.isEmpty) return fallback();
  const pos = editor.selection.active;
  const line = editor.document.lineAt(pos.line);
  if (pos.character < line.text.length) return fallback();
  if (!client || !client.isRunning()) return fallback();

  let r;
  try {
    // The line the newline will create is pos.line + 1; the server reads
    // only the lines above it, which is the document as it stands.
    r = await client.sendRequest("harsh/columns", {
      textDocument: { uri: editor.document.uri.toString() },
      position: { line: pos.line + 1, character: 0 },
    });
  } catch {
    return fallback();
  }
  if (!r || typeof r.default !== "number") return fallback();

  await editor.edit((b) => b.insert(pos, "\n" + " ".repeat(r.default)), {
    undoStopBefore: true,
    undoStopAfter: false,
  });
  const p = new vscode.Position(pos.line + 1, r.default);
  editor.selection = new vscode.Selection(p, p);
  editor.revealRange(new vscode.Range(p, p));
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };

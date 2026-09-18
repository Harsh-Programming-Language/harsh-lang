// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// Launches `hrs-lsp` and binds Tab / Shift-Tab to its `harsh/columns`
// request. Enter needs no code here: the server advertises on-type
// formatting on `\n` and VSCode calls it, given `editor.formatOnType`,
// which package.json turns on for Harsh.

const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");
const fs = require("fs");
const os = require("os");
const path = require("path");

let client;

// Where `hrs-lsp` is, in order of authority.
//
// An editor launched from the Dock or from Finder does not inherit a login
// shell's PATH, so `~/.cargo/bin` is often missing from it and spawning
// `hrs-lsp` fails even though the terminal finds it. The extension therefore
// looks for itself, and says in its output channel what it tried, so a
// failure is diagnosable rather than mysterious (2026-09-17).
function findServer(log) {
  const set = vscode.workspace.getConfiguration("harsh").get("serverPath");
  if (set) {
    log(`harsh.serverPath is set: ${set}`);
    return set;
  }
  const exe = process.platform === "win32" ? "hrs-lsp.exe" : "hrs-lsp";
  log(`looking for ${exe}; platform ${process.platform}, home ${os.homedir()}`);

  const dirs = [];
  if (process.env.CARGO_HOME) dirs.push(path.join(process.env.CARGO_HOME, "bin"));
  dirs.push(path.join(os.homedir(), ".cargo", "bin"));
  for (const d of (process.env.PATH || "").split(path.delimiter)) {
    if (d) dirs.push(d);
  }
  dirs.push("/usr/local/bin", "/opt/homebrew/bin", path.join(os.homedir(), ".local", "bin"));
  for (const d of dirs) {
    const p = path.join(d, exe);
    try {
      fs.accessSync(p, fs.constants.X_OK);
      log(`found: ${p}`);
      return p;
    } catch (e) {
      log(`  not at ${p} (${e.code || e.message})`);
    }
  }

  // Last resort, and the one that fixes the Dock/Finder case on macOS and
  // Linux: ask a login shell, which has the user's own PATH.
  try {
    const shell = process.env.SHELL || "/bin/sh";
    const found = require("child_process")
      .execFileSync(shell, ["-lc", `command -v ${exe}`], { encoding: "utf8", timeout: 5000 })
      .trim()
      .split("\n")[0];
    if (found) {
      log(`a login shell (${shell}) found it: ${found}`);
      return found;
    }
  } catch (e) {
    log(`a login shell could not find it (${e.message})`);
  }

  log(`not found; trying ${exe} on the extension host's PATH`);
  return exe;
}

function activate(context) {
  // Not "Harsh": `LanguageClient` makes a channel of that name for the
  // server's own traffic, and two channels with one name is a way to lose a
  // log (2026-09-17).
  const out = vscode.window.createOutputChannel("Harsh: server search");
  context.subscriptions.push(out);
  const log = (m) => {
    out.appendLine(m);
    console.log(`[harsh] ${m}`);
  };
  out.show(true);
  const command = findServer(log);
  const serverOptions = {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio },
  };
  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "harsh" }],
  };
  client = new LanguageClient("harsh", "Harsh", serverOptions, clientOptions);
  client.start().catch(async (e) => {
    const install = "How to install";
    const setPath = "Set harsh.serverPath";
    const pick = await vscode.window.showWarningMessage(
      `Harsh: the language server could not start (${command}: ${e && e.message}). ` +
        `The "Harsh: server search" output channel lists the paths tried. ` +
        `Highlighting still works; Enter, Tab and format on save need \`hrs-lsp\`.`,
      install,
      setPath
    );
    if (pick === install) {
      vscode.window.showInformationMessage(
        "Install the Harsh toolchain with `cargo install harsh-lang`, then reload the window. " +
          "If it is installed but not found, your editor's PATH may not include ~/.cargo/bin: set harsh.serverPath to the full path."
      );
    } else if (pick === setPath) {
      vscode.commands.executeCommand("workbench.action.openSettings", "harsh.serverPath");
    }
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

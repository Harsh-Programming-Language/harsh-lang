// The website's editor, driven by simulated keys: its script (from
// site/src/components/editor.hrs, KEYS) against a stand-in textarea, its
// questions answered by harsh_lang::columns natively -- the code the page's
// wasm calls. Run by site/check-editor.py; COLS and KEYS name the helper
// and the extracted script.
// The editor's script, driven by simulated keys against a stand-in textarea;
// its questions are answered by the native `cols`, which calls the same
// harsh_lang::columns the page's wasm calls.
const { execFileSync } = require("child_process");
const fs = require("fs");
const listeners = {};
const area = {
  value: "", selectionStart: 0, selectionEnd: 0,
  addEventListener: (k, f) => (listeners[k] = f),
  focus() {}, dispatchEvent() {},
  setSelectionRange(a, b) { this.selectionStart = a; this.selectionEnd = b; },
  setRangeText(t, a, b) { this.value = this.value.slice(0, a) + t + this.value.slice(b); },
  previousElementSibling: {},
};
global.document = { getElementById: () => area, execCommand: () => false };
let pending = null;
global.dioxus = {
  send: ([text, line]) => { pending = execFileSync(process.env.COLS, [String(line)], { input: text }).toString(); },
  recv: async () => JSON.parse(pending),
};
const script = fs.readFileSync(process.env.KEYS, "utf8");
eval(script)();
async function key(k, shift = false) {
  const ev = { key: k, shiftKey: shift, prevented: false, preventDefault() { this.prevented = true; } };
  await listeners.keydown(ev);
  return ev.prevented;
}
function type(text) { // plain typing: the browser's own behaviour
  const s = area.selectionStart;
  area.value = area.value.slice(0, s) + text + area.value.slice(area.selectionEnd);
  area.setSelectionRange(s + text.length, s + text.length);
}
const show = () => JSON.stringify(area.value.slice(0, area.selectionStart) + "▮" + area.value.slice(area.selectionStart));
let failures = 0;
function expect(name, want) {
  const got = area.value.slice(0, area.selectionStart) + "▮" + area.value.slice(area.selectionStart);
  const ok = got === want;
  if (!ok) failures++;
  console.log((ok ? "ok   " : "FAIL ") + name + (ok ? "" : "\n     want " + JSON.stringify(want) + "\n     got  " + JSON.stringify(got)));
}
(async () => {
  type("fn main$:"); await key("Enter");
  expect("Enter after a block opener goes in a level", "fn main$:\n    ▮");
  type("let total ="); await key("Enter");
  expect("Enter after `=` goes in a level", "fn main$:\n    let total =\n        ▮");
  type("1 + 2"); await key("Enter");
  expect("Enter after a continued value: one unit past it (LSP.md, the continuation rule)", "fn main$:\n    let total =\n        1 + 2\n            ▮");
  let p = await key("Tab");
  expect("Tab past the last legal column: one unit further", "fn main$:\n    let total =\n        1 + 2\n                ▮");
  console.log((p ? "ok   " : "FAIL ") + "Tab is kept in the editor (default prevented)"); if (!p) failures++;
  await key("Tab", true);
  expect("Shift-Tab: back to the legal column within a unit", "fn main$:\n    let total =\n        1 + 2\n            ▮");
  await key("Tab", true); await key("Tab", true);
  expect("Shift-Tab twice more: the statement column", "fn main$:\n    let total =\n        1 + 2\n    ▮");
  type("println! \"{total}\"");
  const before = area.value;
  area.setSelectionRange(area.value.indexOf("println"), area.value.indexOf("println"));
  area.setSelectionRange(area.value.length - 3, area.value.length - 3);
  await key("Tab");
  expect("Tab past the indentation is an ordinary tab stop", before.slice(0, -3) + " ▮" + before.slice(-3));
  // Escape, then Tab: the Tab is not taken, so focus can leave.
  await key("Escape");
  p = await key("Tab");
  console.log((!p ? "ok   " : "FAIL ") + "after Escape, Tab is left to the browser (focus can leave)"); if (p) failures++;
  // A closing bracket at the start of a line goes under its opener.
  area.value = "fn main$:\n    let v = vec! (\n        1, 2\n        "; area.setSelectionRange(area.value.length, area.value.length);
  await key(")");
  const lines = area.value.split("\n"); console.log("     `)` landed at column " + lines[lines.length - 1].indexOf(")") + " of " + JSON.stringify(lines[lines.length - 1]));
  // Enter in the middle of a line keeps the line's indentation.
  area.value = "fn main$:\n    let a = 1"; area.setSelectionRange(area.value.length - 1, area.value.length - 1);
  await key("Enter");
  expect("Enter mid-line keeps the indentation", "fn main$:\n    let a = \n    ▮1");
  // --- Pairs --- (▮ marks the caret; a selection is [▮…◆])
  const at = (text) => {
    const i = text.indexOf("▮"), j = text.indexOf("◆");
    area.value = text.replace("▮", "").replace("◆", "");
    area.setSelectionRange(i, j < 0 ? i : j - 1);
  };
  const left = (name, prevented) => { const ok = !prevented; if (!ok) failures++; console.log((ok ? "ok   " : "FAIL ") + name); };
  at("fn main$:\n    let v = vec! ▮"); await key("(");
  expect("`(` closes itself", "fn main$:\n    let v = vec! (▮)");
  await key(")");
  expect("typing the `)` already next steps over it", "fn main$:\n    let v = vec! ()▮");
  at("fn main$:\n    let v = vec! (▮)"); await key("Backspace");
  expect("Backspace in an empty pair takes both", "fn main$:\n    let v = vec! ▮");
  at("fn main$:\n    println! ▮"); await key('"');
  expect("a quote closes itself", "fn main$:\n    println! \"▮\"");
  type("hi"); await key('"');
  expect("the closing quote is stepped over", "fn main$:\n    println! \"hi\"▮");
  at("fn main$:\n    let s = \"it▮"); left("inside a string, a quote is left to the browser, not doubled", await key('"'));
  at("fn main$:\n    let r = ▮x"); left("before a word, an opener is left to the browser, not closed", await key("["));
  at("fn main$:\n    let s = ▮name◆"); await key('"');
  const wrapped = area.value.endsWith('"name"') && area.value.slice(area.selectionStart, area.selectionEnd) === "name";
  console.log((wrapped ? "ok   " : "FAIL ") + "a quote wraps a selection, which stays selected"); if (!wrapped) failures++;
  at("fn main$:\n    /▮"); await key("*");
  expect("`/*` closes with ` */`", "fn main$:\n    /*▮ */");
  at("fn main$:\n    let v = vec! (▮)"); await key("Enter");
  const got = area.value;
  console.log("     after Enter in `vec! (▮)`:\n" + (got.slice(0, area.selectionStart) + "▮" + got.slice(area.selectionStart)).split("\n").map((l) => "       |" + l).join("\n"));
  const ls = got.split("\n");
  const ok3 = ls.length === 4 && ls[3].trim() === ")" && ls[3].indexOf(")") === ls[1].indexOf("vec!") && ls[2].trim() === "";
  console.log((ok3 ? "ok   " : "FAIL ") + "Enter in a pair: three lines, the `)` under `vec!`"); if (!ok3) failures++;
  // The cases the block above does not cover. A key the script does not
  // take is typed as the browser would.
  async function press(ch) { if (!(await key(ch))) type(ch); }
  at("let c = ▮"); await press("'");
  expect("`'` never closes (lifetimes)", "let c = '▮");
  at("let s = r▮"); await press('"');
  expect('`"` right after a word does not close', 'let s = r"▮');
  at("let v = vec! ▮x"); await press("(");
  expect("`(` before a word does not close", "let v = vec! (▮x");
  at("f ▮x◆"); await press("(");
  const w1 = area.value === "f (x)" && area.selectionStart === 3 && area.selectionEnd === 4;
  console.log((w1 ? "ok   " : "FAIL ") + "`(` wraps a selection, which stays selected"); if (!w1) failures++;
  at("g ▮a◆"); await press("'");
  const w2 = area.value === "g 'a'" && area.selectionStart === 3 && area.selectionEnd === 4;
  console.log((w2 ? "ok   " : "FAIL ") + "`'` wraps a selection, though it never closes"); if (!w2) failures++;
  console.log(failures ? failures + " failed" : "all passed");
  process.exit(failures ? 1 : 0);
})();

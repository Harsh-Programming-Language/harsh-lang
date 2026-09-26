#!/usr/bin/env python3
"""The website's editor, tested with simulated keys.

Builds a small helper that answers as the page's wasm does
(`harsh_lang::columns`, native), takes the editor's script out of
`site/src/components/editor.hrs`, and runs `site/check-editor.js` under node:
Enter, Tab, Shift-Tab, closing brackets, and the pairs. Run from the
repository root. Skipped, with a note, where node is not installed.
"""
import os, re, shutil, subprocess, sys, tempfile

ROOT = os.getcwd()
if not shutil.which("node"):
    print("  (node not installed here; editor not checked)")
    sys.exit(0)
work = os.path.join(tempfile.gettempdir(), "hrs-site-editor")
os.makedirs(os.path.join(work, "src"), exist_ok=True)
open(os.path.join(work, "Cargo.toml"), "w").write(
    '[package]\nname = "cols"\nversion = "0.1.0"\nedition = "2021"\n[dependencies]\n'
    f'harsh-lang = {{ path = "{ROOT}", default-features = false }}\n')
open(os.path.join(work, "src/main.rs"), "w").write(r'''
use std::io::Read;
fn main() {
    let line: usize = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).unwrap();
    let c = harsh_lang::columns::columns(&text, line);
    let legal: Vec<String> = c.legal.iter().map(|n| n.to_string()).collect();
    print!("[{},[{}],{},{}]", c.default, legal.join(","), c.unit, c.closer.map(|n| n as i64).unwrap_or(-1));
}
''')
subprocess.run(["cargo", "build", "--quiet", "--release"], cwd=work, check=True)
src = open(os.path.join(ROOT, "site/src/components/editor.hrs")).read()
js = re.search(r'const KEYS: &str = r#"(.*?)"#', src, re.S).group(1).replace("__EDITOR_ID__", '"editor"')
keys = os.path.join(work, "keys.js")
open(keys, "w").write("(async function(){\n" + js + "\n})\n")
env = dict(os.environ, COLS=os.path.join(work, "target/release/cols"), KEYS=keys)
r = subprocess.run(["node", os.path.join(ROOT, "site/check-editor.js")], env=env, capture_output=True, text=True)
lines = [l for l in r.stdout.splitlines() if l.startswith(("ok", "FAIL"))]
fails = [l for l in lines if l.startswith("FAIL")]
if r.returncode != 0 or fails or not lines:
    print(r.stdout + r.stderr)
    sys.exit(1)
print(f"  {len(lines)} editor behaviours, keys simulated: all as intended")

#!/usr/bin/env python3
"""The site's highlighter against the books'.

The site highlights code with `src/highlight.hrs`, a port of the highlighter
in `docs/build.py` (the user's choice, 2026-09-26: the books' highlights,
everywhere). This builds the port -- as `hrs` transpiled it, in
`site/target/hrs/highlight.rs` -- into a small program, runs it on every code
block of the Book, By Example and the guide, and compares its HTML with
`build.py`'s own function, byte for byte. Run from the repository root,
after the site is transpiled. Needs cargo and the `regex` crate.
"""
import glob, html, os, re, shutil, subprocess, sys, tempfile, textwrap

ROOT = os.getcwd()
src = open(os.path.join(ROOT, "docs/build.py")).read()
a, b = src.index("  KW = set("), src.index("  blocks = []")
ns = {"re": re, "html": html}
exec(textwrap.dedent(src[a:b]), ns)  # build.py's own hl, not a copy
hl = ns["hl"]

# The public tree has book/chapters and by-example/pages, the development
# tree the sources as well; whichever exist are read.
files = sorted(glob.glob("book/sources/*.md") + glob.glob("book/chapters/*.md")
               + glob.glob("by-example/sources/*.md")
               + glob.glob("by-example/pages/*.md")
               + ["docs/LANGUAGE.md", "docs/MACROS.md", "docs/TUTORIAL.md"])
fence = re.compile(r"```([^\n]*)\n(.*?)```", re.S)
blocks = [m.group(2) for f in files for m in fence.finditer(open(f, encoding="utf-8").read())
          if m.group(1).strip() not in ("text", "toml", "sh") and "\0" not in m.group(2)]

work = os.path.join(tempfile.gettempdir(), "hrs-site-highlight")
os.makedirs(os.path.join(work, "src"), exist_ok=True)
shutil.copy("site/target/hrs/highlight.rs", os.path.join(work, "src/highlight.rs"))
open(os.path.join(work, "Cargo.toml"), "w").write(
    '[package]\nname = "hl"\nversion = "0.1.0"\nedition = "2021"\n[dependencies]\nregex = "1"\n')
open(os.path.join(work, "src/main.rs"), "w").write(r'''
#[path = "highlight.rs"] mod highlight;
use std::io::Read;
fn esc(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;") }
fn main() {
    let mut all = String::new();
    std::io::stdin().read_to_string(&mut all).unwrap();
    let outs: Vec<String> = all.split('\0').map(|block| {
        highlight::tokens(block).iter().map(|t| if t.class.is_empty() { esc(&t.text) }
            else { format!("<span class=\"{}\">{}</span>", t.class, esc(&t.text)) }).collect()
    }).collect();
    print!("{}", outs.join("\0"));
}
''')
subprocess.run(["cargo", "build", "--quiet", "--release"], cwd=work, check=True)
got = subprocess.run([os.path.join(work, "target/release/hl")], input="\0".join(blocks),
                     capture_output=True, text=True, check=True).stdout.split("\0")
bad = [i for i, (w, g) in enumerate(zip([hl(b) for b in blocks], got)) if w != g]
if bad or len(got) != len(blocks):
    print(f"  {len(bad)} of {len(blocks)} code blocks highlight differently from build.py")
    sys.exit(1)
print(f"  {len(blocks)} code blocks of {len(files)} files: highlighted as build.py does, byte for byte")

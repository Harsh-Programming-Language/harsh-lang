#!/usr/bin/env python3
"""Regenerate the rendered forms: docs/language.{ipynb,html} from docs/LANGUAGE.md, book/harsh-book.{ipynb,html} from book/HARSH-BOOK.md, and by-example/harsh-by-example.{ipynb,html} from by-example/HARSH-BY-EXAMPLE.md. Run from anywhere.

The .md is the source; the two rendered forms are derived and must never be
edited by hand. The page chrome (the <head> with the CSS, the trailing
<script>) lives in docs/page-head.html and docs/page-tail.html.
"""
import json, re, html, sys, os
import markdown
DOCS = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(DOCS)

def unique(slugs):
  """Each anchor once: a repeated heading's gets `-1`, `-2`, … in order, as
  mdBook and GitHub number them. Two parts of the macros chapter share their
  subsection names, and every link under the second went to the first
  (found 2026-09-24)."""
  seen, out = {}, []
  for s in slugs:
      n = seen.get(s, 0)
      out.append(s if n == 0 else f"{s}-{n}")
      seen[s] = n + 1
  return out

def headings(lines):
  """The navigation's entries: (depth, text) for every heading from `#` to
  `####`, outside code blocks, after the document's own title (its first
  `#` line). Depth 0 is the highest level present -- a book's chapters, a
  guide's sections -- so every document's navigation starts at its top
  (found 2026-09-24: the Book's chapters, `#`, were missing from it)."""
  every = all_headings(lines)
  found = [(lvl, text) for lvl, text in every if lvl <= 4]
  top = min((lvl for lvl, _ in found), default=1)
  return [(lvl - top, text) for lvl, text in found]

def all_headings(lines):
  """Every heading, `#` to `######`, outside code blocks, after the
  document's title -- in order, as the body renders them."""
  found, fence, titled = [], False, False
  for ln in lines:
      if ln.startswith("```"):
          fence = not fence
          continue
      m = None if fence else re.match(r"^(#{1,6}) (.*)$", ln)
      if not m:
          continue
      if len(m.group(1)) == 1 and not titled:
          titled = True
          continue
      found.append((len(m.group(1)), m.group(2).strip()))
  return found

def render(SRC, IPYNB, HTML, TITLE, TAG):
  md = open(SRC, encoding="utf-8").read()

  # notebook
  def slug(s):
      s = re.sub(r"[`*_]", "", s).lower()
      s = re.sub(r"[^a-z0-9]+", "-", s).strip("-")
      return s

  lines = md.split("\n")
  # one cell per heading; the first cell is the H1 and its intro
  cells, cur = [], []
  for ln in lines:
      if re.match(r"^#{1,3} ", ln) and cur:
          cells.append("\n".join(cur).strip("\n"))
          cur = []
      cur.append(ln)
  if cur:
      cells.append("\n".join(cur).strip("\n"))

  heads = headings(lines)
  toc = ["## Contents", ""]
  for depth, text in heads:
      toc.append(f"{'    ' * depth}- {text}")
  cells.insert(1, "\n".join(toc))

  nb = {
      "cells": [{"cell_type": "markdown", "metadata": {}, "source": c.splitlines(keepends=True)} for c in cells],
      "metadata": {"kernelspec": {"display_name": "Python 3", "language": "python", "name": "python3"},
                   "language_info": {"name": "python", "version": "3.11"}},
      "nbformat": 4, "nbformat_minor": 5,
  }
  json.dump(nb, open(IPYNB, "w", encoding="utf-8"), ensure_ascii=False, indent=1)

  # -------------------------------------------------------------------- html
  KW = set("""as async await break const continue crate do dyn else enum extern false fn for if impl in
  let loop match mod move mut pub ref return self Self static struct super trait true type unsafe use
  where while union""".split())
  OPS = ["<-", "->", "=>", "<|", "|>", "::", "..=", "..", "&&", "||", "==", "!=", "<=", ">=", "+=", "-=", "*=", "/="]

  TOK = re.compile(
      r"(?P<com>//[^\n]*)|(?P<str>\"(?:\\.|[^\"\\])*\"|'(?:\\.|[^'\\])'|b\"(?:\\.|[^\"\\])*\")"
      r"|(?P<attr>#!?\[[^\]\n]*\])|(?P<num>\b\d[\w.]*\b)|(?P<mac>\b[A-Za-z_]\w*!)"
      r"|(?P<life>'[A-Za-z_]\w*)|(?P<id>\b[A-Za-z_]\w*\b)|(?P<defer>\$)"
      r"|(?P<op>" + "|".join(re.escape(o) for o in sorted(OPS, key=len, reverse=True)) + r")"
      r"|(?P<p>[()\[\]{}<>|&,;:.=+\-*/!?@#%^~])|(?P<ws>\s+)|(?P<other>.)")

  def hl(code):
      out = []
      for m in TOK.finditer(code):
          k = m.lastgroup; t = html.escape(m.group(), quote=False)
          if k == "id":
              if m.group() in KW: k = "k"
              elif m.group()[0].isupper(): k = "t"
              else: k = None
          if k in ("ws", "other", "life", None):
              out.append(t)
          else:
              out.append(f'<span class="{k}">{t}</span>')
      return "".join(out)

  blocks = []
  def fence(m):
      lang, body = m.group(1).strip(), m.group(2)
      # `rust harsh` and `rust fragment` are Harsh: the first word is for
      # GitLab and GitHub, which colour it as Rust; here Harsh's own
      # highlighter runs. Plain `rust` is Rust.
      if lang == "rust":
          h = f'<pre class="code rust"><code>{hl(body)}</code></pre>'
      elif lang in ("text", "toml", "sh"):
          h = f'<pre class="code"><code>{html.escape(body, quote=False)}</code></pre>'
      elif re.search(r"[│└─┌┐┘├┤]", body):
          h = f'<pre class="code diagram"><code>{html.escape(body, quote=False)}</code></pre>'
      else:
          h = f'<pre class="code harsh"><code>{hl(body)}</code></pre>'
      blocks.append(h)
      return f"\n\nFENCEBLOCK{len(blocks)-1}\n\n"

  # A fence opens and closes only at the start of a line: a ``` inside a
  # sentence is a code span, and once swallowed one as a fence up to the
  # next block, which took the rest of the Book with it.
  body_md = re.sub(r"^```([^\n]*)\n(.*?)\n```[ \t]*$", fence, md, flags=re.S | re.M)
  # drop the document H1; the HTML header carries the title
  body_md = re.sub(r"^# [^\n]*\n", "", body_md, count=1)
  body = markdown.markdown(body_md, extensions=["tables"])
  body = re.sub(r"<p>FENCEBLOCK(\d+)</p>", lambda m: blocks[int(m.group(1))], body)
  # One H1 for the whole document -- the page's title -- and every other
  # heading shifted down from it (the user's styling rule). A document with
  # chapters, the Book, has `#` headings of its own; they become h2.
  if re.search(r"<h1>", body):
      body = re.sub(r"<(/?)h([1-5])>", lambda m: f"<{m.group(1)}h{int(m.group(2)) + 1}>", body)
  # ids on every heading, for the navigation
  # Anchors for every heading, in document order, each unique; the body and
  # the navigation take them from the same list.
  every = all_headings(lines)
  anchors = unique([slug(text) for _, text in every])
  body_ids = iter(anchors)
  def add_id(m):
      lvl, text = m.group(1), m.group(2)
      return f'<h{lvl} id="{next(body_ids, slug(html.unescape(re.sub("<[^>]+>", "", text))))}">{text}</h{lvl}>'
  body = re.sub(r"<h([2-6])>(.*?)</h\1>", add_id, body)

  top = min((lvl for lvl, _ in every if lvl <= 4), default=1)
  nav = ['<nav class="toc"><p class="toc-h">Contents</p><ul>']
  for (lvl, text), anchor in zip(every, anchors):
      if lvl <= 4:
          nav.append(f'<li class="d{lvl - top}"><a href="#{anchor}">{html.escape(text)}</a></li>')
  nav.append("</ul></nav>")

  # The page chrome -- the CSS in the head and the script at the end -- is
  # its own source, docs/page-head.html and docs/page-tail.html, so a tree
  # with no rendered page yet can still render one.
  head = open(os.path.join(DOCS, "page-head.html"), encoding="utf-8").read()
  # The chrome is the guide's; the name on it is this document's. Without
  # this every page called itself the language guide, the book included.
  head = re.sub(r"<title>.*?</title>", f"<title>{html.escape(TITLE)}</title>", head, count=1, flags=re.S)
  tail = open(os.path.join(DOCS, "page-tail.html"), encoding="utf-8").read()
  header = '''
  <button class="toggle" id="nav-toggle" title="Show or hide the contents">☰</button>
  <div class="wrap">
    <a class="home" href="https://harsh-lang.com/learn">&larr; Learn &middot; harsh-lang.com</a>
    <header>
      <p class="logo"><span class="lg-b">#[</span><span class="lg-n">Ha</span><span class="lg-a">&lt;</span><span class="lg-r">rs</span><span class="lg-a">&gt;</span><span class="lg-a">.</span><span class="lg-n lg-i">h</span><span class="lg-b">]</span></p>
      <h1 class="title">Harsh</h1>
      <p class="tag">''' + TAG + '''</p>
    </header>
    <div class="cols">
      ''' + "\n".join(nav) + '''
      <main>
  '''
  footer = "\n    </main>\n  </div>\n</div>\n"
  open(HTML, "w", encoding="utf-8").write(head + header + body + footer + tail)
  print(f"{os.path.relpath(IPYNB, ROOT)}: {len(cells)} cells; {os.path.relpath(HTML, ROOT)}: {len(blocks)} code blocks")

if __name__ == "__main__":
  # The guide's code blocks are verified before anything is rendered: every
  # `rust harsh` block transpiles, and a Rust column is what the transpiler
  # writes (docs/check-guide.py). A guide that lies does not render.
  import subprocess
  r = subprocess.run([sys.executable, os.path.join(DOCS, "check-guide.py")], capture_output=True, text=True)
  if r.returncode != 0:
    sys.exit(r.stdout + r.stderr)
  print(r.stdout.strip())
  # The guide first: its page supplies the CSS and script the book reuses.
  d = lambda f: os.path.join(DOCS, f)
  b = lambda f: os.path.join(ROOT, "book", f)
  # The guide first: its page supplies the chrome every other page reuses.
  render(d("LANGUAGE.md"), d("language.ipynb"), d("language.html"), "The Harsh Language Guide",
         "<em>Rust without the braces.</em> &nbsp;Language guide.")
  # The Book's *Harsh at a glance*, as a page of its own (2026-10-03): rendered
  # from the chapter the Book's build writes, so the two never disagree;
  # published at /glance/.
  render(b("chapters/25_at_a_glance.md"), b("glance.ipynb"), b("glance.html"), "Harsh at a glance",
         "<em>Rust without the braces.</em> &nbsp;Every rule of Harsh's own, from the Book.")
  # The cheat sheet (2026-10-03): the same chrome, published at /cheatsheet/.
  render(d("CHEATSHEET.md"), d("cheatsheet.ipynb"), d("cheatsheet.html"), "The Fast Track to Harsh",
         "<em>Rust without the braces.</em> &nbsp;The cheat sheet.")
  # Every other document in docs/ and docs/dev/, same three forms, same theme.
  others = {
    "START-HERE.md": ("Harsh — start here", "The map of the project."),
    "SPEC.md": ("Harsh — specification", "Implementation notes."),
    "LSP.md": ("Harsh — language server", "Design of hrs-lsp."),
    "FMT.md": ("Harsh — formatter", "Design of hrs fmt."),
    "MACROS.md": ("Harsh — macros", "Design of Harsh's macro rules: HSX, expression bodies, matchers."),
    "TUTORIAL.md": ("Harsh — lessons", "What building Harsh taught."),
    "ROADMAP.md": ("Harsh — roadmap", "What is done, what is next, what is settled."),
    "GOVERNANCE.md": ("Harsh — governance", "Who decides, and what will not change."),
    "TRADEMARK.md": ("Harsh — trademark", "The name, the logo, the binaries."),
    "BRAND.md": ("Harsh — brand", "The mark and how to use it."),
    "dev/HANDOVER.md": ("Harsh — handover", "Development bundle only."),
    "dev/RESUME.md": ("Harsh — resume prompt", "Development bundle only."),
    "dev/PUBLISHING.md": ("Harsh — publishing", "Development bundle only."),
  }
  for f, (title, tag) in others.items():
    src = d(f)
    if not os.path.exists(src):
      continue
    stem = os.path.splitext(os.path.basename(f))[0].lower()
    out = os.path.join(os.path.dirname(src), stem)
    render(src, out + ".ipynb", out + ".html", title, f"<em>{tag}</em>")
  if os.path.exists(b("HARSH-BOOK.md")):
    render(b("HARSH-BOOK.md"), b("harsh-book.ipynb"), b("harsh-book.html"), "The Harsh Programming Language",
           "The Harsh Programming Language")
  e = lambda f: os.path.join(ROOT, "by-example", f)
  if os.path.exists(e("HARSH-BY-EXAMPLE.md")):
    render(e("HARSH-BY-EXAMPLE.md"), e("harsh-by-example.ipynb"), e("harsh-by-example.html"),
           "Harsh by Example", "Harsh by Example")
  dp = lambda f: os.path.join(ROOT, "patterns", f)
  if os.path.exists(dp("HARSH-DESIGN-PATTERNS.md")):
    render(dp("HARSH-DESIGN-PATTERNS.md"), dp("harsh-design-patterns.ipynb"), dp("harsh-design-patterns.html"),
           "Harsh Design Patterns", "Harsh Design Patterns")
  dn = lambda f: os.path.join(ROOT, "nomicon", f)
  if os.path.exists(dn("THE-HARSHONOMICON.md")):
    render(dn("THE-HARSHONOMICON.md"), dn("the-harshonomicon.ipynb"), dn("the-harshonomicon.html"),
           "The Harshonomicon", "The Harshonomicon")

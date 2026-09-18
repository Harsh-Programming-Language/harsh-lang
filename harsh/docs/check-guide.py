#!/usr/bin/env python3
"""The Harsh Language Guide's harness: docs/LANGUAGE.md's code blocks, verified.

The guide shows Harsh beside Rust, construct by construct, and until now
nothing checked that the Harsh it shows is Harsh the transpiler accepts,
or that the Rust beside it is what the transpiler writes. This does both:

- an untagged fence is Harsh and must transpile;
- a fence whose lines carry a second column -- Harsh on the left, Rust on
  the right, separated by four spaces or more -- or written `harsh  →  rust`
  on one line, is split: the left must transpile and the transpiler's Rust
  must match the right token for token (commas, whitespace and `;` aside).
  The convention (decided 2026-09-10): a second column is always Rust; a
  note is a third column, after another run of four spaces, and is dropped;
  a note beside Harsh that has no Rust column is a `//` comment;
- a block that does not begin with an item is statements, and is transpiled
  inside `fn main$:`; a method shown on its own, indented, inside `impl`;
- ```fragment marks a block that is deliberately not a whole file (a
  catalogue of headers, a rejected form beside an accepted one) and is not
  checked; ```text, ```toml, ```sh, ```rust are not Harsh and are not checked.

Run from anywhere; `--all` reports every failure instead of the first.
docs/build.py runs it before rendering.
"""
import os, re, subprocess, sys, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GUIDE = os.path.join(ROOT, "docs", "LANGUAGE.md")
HRS = os.path.join(ROOT, "target", "release", "hrs")

ITEM = re.compile(r"^(pub\b|fn\b|struct\b|enum\b|impl\b|trait\b|mod\b|use\b|const\b|static\b|type\b|extern\b|macro_rules!|#\[|#!\[|//)")

def is_items(harsh):
    """Does the block begin with an item? Otherwise it is statements, and is
    transpiled inside `fn main$:` -- at file level there is no expression
    region and `f x` would be copied through unapplied."""
    first = next((l for l in harsh.split("\n") if l.strip()), "")
    return bool(ITEM.match(first.lstrip()))

def transpile(harsh):
    """The transpiler's Rust for a block, or (None, error)."""
    first = next((l for l in harsh.split("\n") if l.strip()), "")
    # A method shown on its own, indented as it sits in its `impl`: wrap it
    # in one, so the block reads as it does in the tutorial's source.
    impl = first.startswith("    ") and ITEM.match(first.lstrip())
    if impl:
        harsh = "impl Fragment\n" + harsh
    wrapped = not impl and not is_items(harsh)
    if wrapped:
        # A trailing `()` keeps the last line a statement rather than the
        # block's tail, as the doc-example pass does.
        harsh = "fn main$:\n" + "\n".join(("    " + l) if l.strip() else "" for l in harsh.split("\n")) + "\n    ()\n"
    with tempfile.NamedTemporaryFile("w", suffix=".hrs", delete=False, dir="/tmp") as f:
        f.write(harsh)
        path = f.name
    r = subprocess.run([HRS, path, "-o", path[:-4] + ".rs"], capture_output=True, text=True)
    out = open(path[:-4] + ".rs").read() if r.returncode == 0 else None
    for p in (path, path[:-4] + ".rs", path[:-4] + ".rs.map.json"):
        try:
            os.unlink(p)
        except OSError:
            pass
    if out is not None and wrapped:
        lines = out.split("\n")
        out = "\n".join(l[4:] if l.startswith("    ") else l for l in lines[1:-3])
    elif out is not None and impl:
        out = "\n".join(out.split("\n")[1:-2])
    return out, r.stderr.strip()

def tokens(rust):
    """Rust tokens, comments, commas and whitespace aside -- the comparison
    tests/roundtrip.rs makes, done here with a small lexer of its own. The
    guide shows statements and expressions alike without a trailing `;`
    while the wrapper makes each a statement, so `;` is not compared either;
    the semicolon rules are pinned by tests/roundtrip.rs."""
    rust = re.sub(r"//[^\n]*", "", rust)
    rust = re.sub(r"/\*.*?\*/", "", rust, flags=re.S)
    toks = re.findall(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])\'|\w+|::|->|=>|\.\.=|\.\.|&&|\|\||==|!=|<=|>=|[^\s,]', rust)
    return [t for t in toks if t != ";"]

def split_columns(body):
    """(left, right) for a two-column block, or None. Two notations are in
    use: Harsh on the left and Rust on the right, separated by four spaces
    or more; and `harsh  →  rust` on one line. Either may carry a third
    column of annotation after another run of four spaces, which is prose
    and is dropped -- Rust on one line never holds four spaces. Each line
    splits at its own gap, since hand alignment drifts by a column or two;
    a line whose text begins at the right column or beyond (a lone `}`) is
    right-only, Harsh never indenting that far."""
    lines = body.split("\n")
    if any("→" in l for l in lines):
        left, right = [], []
        for l in lines:
            if "→" in l:
                a, b = l.split("→", 1)
                left.append(a.rstrip())
                right.append(re.split(r" {4,}", b.strip(), 1)[0])
            else:
                left.append(re.split(r" {4,}", l.rstrip(), 1)[0] if l.strip() else "")
                right.append("")
        return "\n".join(left), "\n".join(right)
    # A trailing `//` comment after a gap is part of the Harsh line, not a
    # column: the convention's spelling for a note where there is no Rust.
    gap = re.compile(r"\S( {4,})(?!//)\S")
    starts = [m.end(1) for l in lines for m in [gap.search(l)] if m]
    if not starts:
        return None
    c_min = min(starts)
    left, right = [], []
    for l in lines:
        lead = len(l) - len(l.lstrip(" "))
        if l.strip() and lead >= c_min - 2:
            left.append("")
            right.append(l.strip())
            continue
        m = gap.search(l)
        if m:
            left.append(l[: m.start(1)].rstrip())
            right.append(re.split(r" {4,}", l[m.end(1):].rstrip(), 1)[0])
        else:
            left.append(l.rstrip())
            right.append("")
    return "\n".join(left), "\n".join(right)

def right_is_code(right):
    """Is the right column Rust rather than annotation? Every non-blank,
    non-comment line must carry code punctuation; a note like `shorthand`
    or `two parameters, comma list` carries none."""
    for l in right.split("\n"):
        s = l.strip()
        if not s or s.startswith("//"):
            continue
        if not re.search(r"[;{}()\[\]<>=.:!&|*+/$#\"]", s):
            return False
    return True

def main():
    if not os.path.exists(HRS):
        sys.exit("build hrs first: cargo build --release")
    keep_going = "--all" in sys.argv
    fails = []
    def fail(msg):
        if keep_going:
            fails.append(msg)
        else:
            sys.exit(msg)
    md = open(GUIDE).read()
    n_ok = n_cmp = n_frag = 0
    for m in re.finditer(r"^```([^\n]*)\n(.*?)^```", md, flags=re.M | re.S):
        tag, body = m.group(1).strip(), m.group(2).rstrip("\n")
        line = md[: m.start()].count("\n") + 1
        where = f"docs/LANGUAGE.md:{line} ({body.splitlines()[0][:50]!r})"
        if tag in ("text", "toml", "sh", "rust"):
            continue
        if re.search(r"[│└─┌┐┘├┤]", body):
            continue
        if tag == "fragment":
            n_frag += 1
            continue
        if tag:
            fail(f"{where}: unknown fence tag {tag!r}")
            continue
        cols = split_columns(body)
        harsh = cols[0] if cols else body
        rust, err = transpile(harsh)
        if rust is None:
            fail(f"{where}: does not transpile:\n{err}")
            continue
        n_ok += 1
        if cols:
            want, got = tokens(cols[1]), tokens(rust)
            if want != got:
                k = next((i for i, (a, b) in enumerate(zip(want, got)) if a != b), min(len(want), len(got)))
                fail(f"{where}: the Rust column is not what the transpiler writes\n"
                     f"  shown:   {' '.join(want[max(0, k - 4):k + 6])}\n"
                     f"  written: {' '.join(got[max(0, k - 4):k + 6])}")
                continue
            n_cmp += 1
    if fails:
        sys.exit("\n\n".join(fails) + f"\n\n{len(fails)} failure(s)")
    print(f"docs/LANGUAGE.md: {n_ok} block(s) transpile, {n_cmp} of them matched against their Rust column; {n_frag} fragment(s) not checked")

if __name__ == "__main__":
    main()

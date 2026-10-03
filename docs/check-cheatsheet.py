#!/usr/bin/env python3
"""Every example in docs/CHEATSHEET.md transpiles into Rust that parses (the
user's cheat sheet, 2026-10-03): each code span in the Harsh column of a `| Task | Harsh |`
table -- as statements inside a function, or else as items -- and each
```rust harsh block as it stands. Run by check.sh, beside check-guide.py."""
import os, re, subprocess, sys, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HRS = os.environ.get("HRS") or os.path.join(ROOT, "target", "release", "hrs")
SHEET = os.path.join(ROOT, "docs", "CHEATSHEET.md")


def transpile(src):
    with tempfile.TemporaryDirectory() as d:
        path = os.path.join(d, "t.hrs")
        open(path, "w").write(src)
        out = os.path.join(d, "t.rs")
        r = subprocess.run([HRS, path, "-o", out], capture_output=True, text=True)
        if r.returncode != 0:
            return False, r.stderr.strip()
        # Transpiling is not enough: the Rust must parse. rustfmt parses
        # without resolving names, so an example missing its context passes
        # and a line the transpiler mangled does not.
        f = subprocess.run(["rustfmt", "--edition", "2021", "--emit", "stdout", out], capture_output=True, text=True)
        if f.returncode != 0:
            return False, "the Rust does not parse: " + open(out).read().replace("\n", " ")[:160]
        return True, ""


def check_span(code):
    if code.startswith("#[") and code.endswith("]"):
        return transpile(code + "\nstruct Shown\n")
    ok, err = transpile("fn main$:\n" + "".join("    " + l + "\n" for l in code.split("\n")))
    if ok:
        return ok, err
    ok2, err2 = transpile(code + "\n")
    return (True, "") if ok2 else (False, err)


def main():
    text = open(SHEET, encoding="utf8").read()
    failures, spans, blocks = [], 0, 0
    in_table = False
    for n, line in enumerate(text.split("\n"), 1):
        if line.startswith("| Task | Harsh |"):
            in_table = True
            continue
        if in_table and not line.startswith("|"):
            in_table = False
        if in_table and not line.startswith("| ---"):
            cells = re.split(r"(?<!\\)\|", line)
            if len(cells) < 3:
                continue
            for code in re.findall(r"`([^`]+)`", cells[2]):
                code = code.replace("\\|", "|")
                if code.startswith("hrs ") or code == "()":
                    continue
                spans += 1
                ok, err = check_span(code)
                if not ok:
                    failures.append(f"CHEATSHEET.md:{n}: `{code}`\n    {err.splitlines()[0] if err else '?'}")
    for m in re.finditer(r"^```rust harsh\n(.*?)^```", text, re.M | re.S):
        blocks += 1
        ok, err = transpile(m.group(1))
        if not ok:
            line = text[: m.start()].count("\n") + 1
            failures.append(f"CHEATSHEET.md:{line}: block\n    {err.splitlines()[0] if err else '?'}")
    for f in failures:
        print(f)
    print(f"docs/CHEATSHEET.md: {spans} example(s) and {blocks} block(s), {len(failures)} failing")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()

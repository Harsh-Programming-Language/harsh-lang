#!/usr/bin/env python3
"""The harness of The Harsh Programming Language (the Book).

Chapters are book/NN_name.md. A line of the form

    @@ snippet_name            three-way: Harsh, generated Rust, output
    @@ snippet_name  !error    two-way: Harsh, then the error rustc reports (remapped),
                               or Harsh's own error when the transpiler refuses the program
    @@ snippet_name  !panic    three-way, but the run must panic: what it printed, then the panic
    @@ snippet_name  !test     build with `rustc --test` and run the tests; show the report
    @@ snippet_name  !test!fail   the same, and at least one test must fail
    @@ snippet_name  !doc      a cargo snippet run with `hrs test`, showing the
                               doc-test report: the examples in its `///`
                               comments are compiled and run like any test
    A directory with a Cargo.toml is run with `hrs run` (the driver), so it may
    have dependencies; all such snippets share one cargo target directory.
    @@ snippet_name  arg1 arg2 VAR=x   arguments (and environment) for the run; a
                                       directory snippet shows them as a cargo command
                                       line, and !panic there means "must exit nonzero"
    @@ snippet_name  <"text"   feed text on stdin when running

is replaced by the rendering of book/src/NN_name/snippet_name.hrs -- or, when
book/src/NN_name/snippet_name/ is a directory, of every .hrs file in it as a
project (main.hrs plus modules), each shown under its src/ path -- produced
by transpiling with hrs, compiling with rustc, and running. Nothing in the
book is typed by hand except the prose and the Harsh; a snippet that does
not build, or whose output changes, fails the build.

There is one edition, and it contains no Rust code. The generated Rust of
every snippet is still transpiled, compiled and run -- that is what makes
the book true -- but its block is stripped from the page, with the `.rs`
path label that introduces it, so a Rust spelling cannot reach a reader who
has not met one. A paragraph (a block between blank lines) beginning

    @harsh ...                 a callout: this is Harsh's own, not Rust

is rendered as one; everything unmarked is prose. The marker is
paragraph-level by design: a sentence that is a callout becomes its own
paragraph.

    @rust  ...                 the retired Rust-column paragraph

is an error. All 37 have been folded in -- each read, the concept it explained
kept and restated in Harsh terms, the Rust spelling gone -- and the marker is
rejected so none comes back.
`@guide`, which inlined a section of docs/LANGUAGE.md, is retired with the
Rust column: the guide relates Harsh spellings to Rust ones for a reader who
knows Rust, which is the one thing this book must not do. Output: HARSH-BOOK.md.
"""
import re, subprocess, sys, os, glob, shlex
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HRS = os.path.join(ROOT, "target", "release", "hrs")
HRS_REMAP = os.path.join(ROOT, "target", "release", "hrs-remap")
BOOK = os.path.dirname(os.path.abspath(__file__))
TMP = "/tmp/harsh-book"
SHOWN_DATA = set()
os.makedirs(TMP, exist_ok=True)

def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)

def strip_ansi(s):
    return re.sub(r"\x1b\[[0-9;]*m", "", s)

def render_project(chapter, name, stdin, args=(), expect_fail=False):
    """A directory snippet: book/src/NN_name/name/ holds main.hrs and its
    modules (garden.hrs, garden/vegetables.hrs, ...), laid out as src/ of a
    project. Every file is transpiled beside its neighbours so rustc resolves
    `mod garden;` the way cargo would; each file is shown under its path."""
    root = os.path.join(BOOK, "src", chapter, name)
    files = sorted(os.path.relpath(os.path.join(d, f), root)
                   for d, _, fs in os.walk(root) for f in fs if f.endswith(".hrs"))
    files.sort(key=lambda f: (f != "main.hrs", f != "lib.hrs", f))
    tmp = os.path.join(TMP, f"{chapter}_{name}")
    os.makedirs(tmp, exist_ok=True)
    # Any other file (a poem.txt to search) goes beside the binary, which
    # runs with the snippet directory as its working directory.
    data = [os.path.relpath(os.path.join(d, f), root)
            for d, _, fs in os.walk(root) for f in fs if not f.endswith(".hrs")]
    for f in data:
        os.makedirs(os.path.dirname(os.path.join(tmp, f)) or tmp, exist_ok=True)
        open(os.path.join(tmp, f), "w").write(open(os.path.join(root, f)).read())
    harsh, rust = [], []
    for f in sorted(data):
        # A data file is shown the first time it appears, not with every stage.
        content = open(os.path.join(root, f)).read().rstrip()
        if (f, content) in SHOWN_DATA:
            continue
        SHOWN_DATA.add((f, content))
        harsh.append(f"`{f}`\n\n```text\n{content}\n```")
    for f in files:
        src = os.path.join(root, f)
        rs = os.path.join(tmp, f[:-4] + ".rs")
        os.makedirs(os.path.dirname(rs), exist_ok=True)
        t = run([HRS, src, "-o", rs, "--map", rs + ".map.json"])
        if t.returncode != 0:
            sys.exit(f"{src}: hrs failed:\n{t.stderr}")
        harsh.append(f"`src/{f}`\n\n```\n{open(src).read().rstrip()}\n```")
        rust.append(f"`src/{f[:-4]}.rs`\n\n```rust\n{open(rs).read().rstrip()}\n```")
    exe = os.path.join(tmp, "main")
    extern = []
    if "lib.hrs" in files:
        # src/lib.hrs is a library crate named after the snippet; the binary
        # links it, as cargo would for a package with both.
        rlib = os.path.join(tmp, f"lib{name}.rlib")
        c = run(["rustc", "--edition", "2021", "-A", "warnings", "--crate-type", "lib",
                 "--crate-name", name, os.path.join(tmp, "lib.rs"), "-o", rlib])
        if c.returncode != 0:
            sys.exit(f"{root}/lib.hrs: rustc failed:\n{strip_ansi(c.stderr)}")
        extern = ["--extern", f"{name}={rlib}"]
    c = run(["rustc", "--edition", "2021", "-A", "warnings"] + extern + [os.path.join(tmp, "main.rs"), "-o", exe])
    if c.returncode != 0:
        sys.exit(f"{root}: rustc failed:\n{strip_ansi(c.stderr)}")
    env = dict(os.environ, RUST_BACKTRACE="0")
    argv = []
    for a in args:
        if "=" in a and a.split("=")[0].isupper():
            k, v = a.split("=", 1)
            env[k] = v
        else:
            argv.append(a)
    r = run([exe] + argv, input=stdin, env=env, cwd=tmp)
    if expect_fail and r.returncode == 0:
        sys.exit(f"{root}: expected the program to fail")
    if not expect_fail and r.returncode != 0:
        sys.exit(f"{root}: program failed:\n{r.stderr}")
    out = harsh + rust
    cmd = "$ cargo run" + (" -- " + " ".join(a for a in args if not ("=" in a and a.split("=")[0].isupper())) if argv else "")
    cmd = " ".join(a for a in args if "=" in a and a.split("=")[0].isupper()) + (" " if any("=" in a for a in args) else "") + cmd
    o = r.stdout.rstrip("\n")
    e = strip_ansi(r.stderr).replace(os.path.join(tmp, "main.rs"), "src/main.hrs").rstrip("\n")
    e = re.sub(r"src/main\.hrs:(\d+):\d+", r"src/main.hrs:\1", e)
    e = e.replace("note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace", "").rstrip()
    body = "\n".join(x for x in [o, e] if x)
    out.append(f"```text\n{cmd}\n{body}\n```")
    return "\n\n".join(out)

def render_test(chapter, name, expect_fail, args):
    """`rustc --test`: the file is compiled as a test crate and its tests are
    run; the report is shown as the run's output, with the paths mapped to
    the .hrs name and the timings removed so the book is stable."""
    src = os.path.join(BOOK, "src", chapter, name + ".hrs")
    harsh = open(src).read().rstrip("\n")
    rs = os.path.join(TMP, f"{chapter}_{name}.rs")
    exe = os.path.join(TMP, f"{chapter}_{name}_test")
    t = run([HRS, src, "-o", rs])
    if t.returncode != 0:
        sys.exit(f"{src}: hrs failed:\n{t.stderr}")
    rust = open(rs).read().rstrip("\n")
    c = run(["rustc", "--edition", "2021", "-A", "warnings", "--test", rs, "-o", exe])
    if c.returncode != 0:
        sys.exit(f"{src}: rustc --test failed:\n{strip_ansi(c.stderr)}")
    r = run([exe] + args + ["--test-threads=1"], env=dict(os.environ, RUST_BACKTRACE="0"))
    if expect_fail and r.returncode == 0:
        sys.exit(f"{src}: expected a failing test, but all passed")
    if not expect_fail and r.returncode != 0:
        sys.exit(f"{src}: tests failed:\n{r.stdout}")
    report = strip_ansi(r.stdout + r.stderr).strip()
    report = re.sub(re.escape(rs) + r":(\d+):\d+", name + r".hrs:\1", report)
    report = re.sub(r"; finished in [0-9.]+s", "", report)
    report = report.replace("note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace", "").rstrip()
    report = "\n".join(l.rstrip() for l in report.splitlines() if l.strip())
    return "\n\n".join([f"```\n{harsh}\n```", f"```rust\n{rust}\n```", f"```text\n$ cargo test{(' ' + ' '.join(args)) if args else ''}\n{report}\n```"])

def render_cargo(chapter, name, stdin, args, expect_fail, doc=False):
    """A snippet directory with a Cargo.toml is a real project: it is copied
    beside the others and run with `hrs run` -- the driver, transpiling
    src/**.hrs and handing cargo the result -- so it may have dependencies.
    Shown: Cargo.toml's [dependencies], every .hrs under src/, the generated
    Rust, and the run's output. All such snippets share one target directory."""
    import shutil
    root = os.path.join(BOOK, "src", chapter, name)
    tmp = os.path.join(TMP, f"{chapter}_{name}")
    shutil.rmtree(tmp, ignore_errors=True)
    shutil.copytree(root, tmp)
    files = sorted(os.path.relpath(os.path.join(d, f), os.path.join(root, "src"))
                   for d, _, fs in os.walk(os.path.join(root, "src")) for f in fs if f.endswith(".hrs"))
    files.sort(key=lambda f: (f != "main.hrs", f != "lib.hrs", f))
    out = []
    toml = open(os.path.join(root, "Cargo.toml")).read()
    deps = toml[toml.index("[dependencies]"):].rstrip() if "[dependencies]" in toml else ""
    if deps.strip() != "[dependencies]":
        out.append(f"`Cargo.toml`\n\n```text\n{deps}\n```")
    for f in files:
        out.append(f"`src/{f}`\n\n```\n{open(os.path.join(root, 'src', f)).read().rstrip()}\n```")
    env = dict(os.environ, RUST_BACKTRACE="0", CARGO_TARGET_DIR=os.path.join(TMP, "cargo-target"), CARGO_TERM_COLOR="never")
    argv = [a for a in args if not ("=" in a and a.split("=")[0].isupper())]
    for a in args:
        if "=" in a and a.split("=")[0].isupper():
            k, v = a.split("=", 1)
            env[k] = v
    # `!doc`: the crate's doc examples are its tests, so the driver runs them
    # (`cargo test` runs every fenced block in a `///` comment) and the report
    # is what the page shows.
    r = run([HRS, "test"] if doc else
            [HRS, "run"] + (["--"] + argv if argv else []), input=stdin, env=env, cwd=tmp)
    if expect_fail and r.returncode == 0:
        sys.exit(f"{root}: expected the program to fail")
    if not expect_fail and r.returncode != 0:
        sys.exit(f"{root}: hrs {'test' if doc else 'run'} failed:\n{r.stdout}\n{r.stderr}")
    for f in files:
        rs = os.path.join(tmp, "target", "hrs", f[:-4] + ".rs")
        out.append(f"`src/{f[:-4]}.rs`\n\n```rust\n{open(rs).read().rstrip()}\n```")
    cmd = "$ hrs test" if doc else "$ hrs run" + (" -- " + " ".join(argv) if argv else "")
    o = r.stdout.rstrip("\n")
    if doc:
        # `cargo test` runs the unit tests first and the doc tests last, and
        # puts its own "Doc-tests <crate>" heading on stderr with the rest of
        # its chatter. The last report on stdout is the doc tests'; the
        # heading is put back so the page reads as the terminal does.
        at = o.rfind("\nrunning ")
        if at < 0 or " 0 measured" not in o[at:]:
            sys.exit(f"{root}: no doc tests ran:\n{r.stdout}\n{r.stderr}")
        crate = re.search(r'name *= *"([^"]+)"', toml).group(1)
        o = f"   Doc-tests {crate}\n\n" + o[at:].strip("\n")
    # cargo's own chatter (Compiling/Finished/Running) is on stderr and is dropped;
    # a program's stderr and a panic are kept.
    e = "\n".join(l for l in strip_ansi(r.stderr).splitlines()
                  if l.strip() and not re.match(r"\s*(hrs: |Compiling|Finished|Running|Doc-tests|Updating|Downloaded|Downloading|Locking|Adding|transpiled|\d+ file\(s\))", l))
    e = e.replace("note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace", "").rstrip()
    body = "\n".join(x for x in [o, e] if x)
    out.append(f"```text\n{cmd}\n{body}\n```")
    return "\n\n".join(out)

def render(chapter, name, expect_error, stdin, expect_panic=False, expect_test=None, args=(), doc=False):
    if expect_test is not None:
        return render_test(chapter, name, expect_test, list(args))
    if os.path.isfile(os.path.join(BOOK, "src", chapter, name, "Cargo.toml")):
        return render_cargo(chapter, name, stdin, args, expect_panic, doc)
    if os.path.isdir(os.path.join(BOOK, "src", chapter, name)):
        if expect_error:
            sys.exit(f"{chapter}/{name}: a directory snippet cannot be !error (one map per file is not remapped)")
        return render_project(chapter, name, stdin, args, expect_panic)
    src = os.path.join(BOOK, "src", chapter, name + ".hrs")
    harsh = open(src).read().rstrip("\n")
    rs = os.path.join(TMP, f"{chapter}_{name}.rs")
    exe = os.path.join(TMP, f"{chapter}_{name}")
    mp = os.path.join(TMP, f"{chapter}_{name}.map.json")
    t = run([HRS, src, "-o", rs, "--map", mp])
    if t.returncode != 0:
        if expect_error:
            # The error is Harsh's own -- the layout or the pipes refusing the
            # program before rustc sees it -- shown as `hrs` prints it.
            block = strip_ansi(t.stderr).strip().replace(src, name + ".hrs")
            return "\n\n".join([f"```\n{harsh}\n```", f"```text\n{block}\n```"])
        sys.exit(f"{src}: hrs failed:\n{t.stderr}")
    rust = open(rs).read().rstrip("\n")
    c = run(["rustc", "--edition", "2021", "-A", "warnings", rs, "-o", exe])
    out = [f"```\n{harsh}\n```"]
    if expect_error:
        if c.returncode == 0:
            sys.exit(f"{src}: expected a compile error, but it compiled")
        # Re-run for JSON diagnostics and remap them onto the Harsh source,
        # exactly as `hrs build` does, so the reader sees the .hrs lines.
        j = run(["rustc", "--edition", "2021", "-A", "warnings", "--error-format=json", rs, "-o", exe])
        stream = "".join(
            '{"reason":"compiler-message","message":' + line + "}\n"
            for line in j.stderr.splitlines() if line.startswith("{")
        )
        r = run([HRS_REMAP, "--map", mp], input=stream)
        err = strip_ansi(r.stdout)
        block = err.split("\nerror: aborting")[0].strip()
        block = "\n".join(l for l in block.splitlines() if not l.startswith("hrs: "))
        block = block.replace(src, name + ".hrs").replace(rs, name + ".hrs").rstrip()
        out.append(f"```text\n{block}\n```")
        return "\n\n".join(out)
    if c.returncode != 0:
        sys.exit(f"{src}: rustc failed:\n{strip_ansi(c.stderr)}")
    env = dict(os.environ, RUST_BACKTRACE="0")
    r = run([exe], input=stdin, env=env)
    out.append(f"```rust\n{rust}\n```")
    o = r.stdout.rstrip("\n")
    if expect_panic:
        if r.returncode == 0:
            sys.exit(f"{src}: expected a panic, but it ran to completion")
        # The generated Rust keeps the .hrs line structure, so the line number
        # is the Harsh line; the column is the Rust's and is dropped.
        msg = strip_ansi(r.stderr).strip()
        msg = re.sub(re.escape(rs) + r":(\d+):\d+", name + r".hrs:\1", msg)
        msg = msg.replace("note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace", "").rstrip()
        o = (o + "\n" if o else "") + msg
        out.append(f"```text\n{o}\n```")
        return "\n\n".join(out)
    if r.returncode != 0:
        sys.exit(f"{src}: program failed:\n{r.stderr}")
    if o:
        out.append(f"```text\n{o}\n```")
    return "\n\n".join(out)

# A generated-Rust block, with the `src/x.rs` label that introduces it when
# the snippet is a project. The label is part of the block: stripping the
# fence and leaving the path was how the Harsh-only edition came to carry 28
# headings with nothing under them.
RUST_BLOCK = re.compile(r"(?:^`[^`\n]*\.rs`\n\n)?```rust\n.*?\n```\n\n", flags=re.S | re.M)

# Spellings the transpiler rejects, which the prose must not teach. The
# harness verifies every snippet and nothing it says *about* a snippet; this
# is how five chapters kept teaching `struct User:` after the snippets had
# moved on. Each pattern is matched inside backticks in the prose only.
RETIRED = [
    (r"`(?:pub )?(?:struct|enum) \w+(?:<[^>`]*>)?:`", "a declaration takes no mark: `struct Point`"),
    (r"`\w+ \{ ?\w+: [^`]*\}`", "a brace literal: the form is `Point\\ x = 1`"),
    (r"`#\[\w+\(", "an attribute applies by juxtaposition: `#[cfg test]`"),
    (r"`\w+::\w+", "the path separator is `.`"),
    (r"`(?:\w+\.)*\w+!\(", "a macro applies by juxtaposition: `m! x`"),
    (r"`\w+\.\w+\(\)`", "a method is `x <- f$`"),
    (r"`(?:Ok|Err|Some|Circle|Write) \((?:[A-Z]\w*)\)`", "a tuple payload is an application: `Write String`"),
]

def check_prose(text, chapter):
    """Reject retired spellings in the chapter's prose (not its snippets --
    those are verified by being built)."""
    fenced = False
    for n, line in enumerate(text.splitlines(), 1):
        if line.startswith("```"):
            fenced = not fenced
            continue
        if fenced or line.startswith("@@"):
            continue
        if "```" in line:
            sys.exit(f"{chapter}.md:{n}: three backticks inside a sentence -- the renderer reads a fence there and swallows the page to the next block; write `text`, not ```` ```text ````")
        for pat, why in RETIRED:
            m = re.search(pat, line)
            if m:
                sys.exit(f"{chapter}.md:{n}: retired spelling {m.group(0)} -- {why}")

def markers(text, chapter):
    """Render the @harsh callouts; reject the retired @rust and @guide."""
    out = []
    for para in text.split("\n\n"):
        if re.match(r"@guide\b", para):
            sys.exit(f"{chapter}: @guide is retired with the Rust column — the book "
                     f"restates the guide's material in its own voice (book/PLAN.md)")
        m = re.match(r"@(rust|harsh)\b[ \t]*(.*)", para, flags=re.S)
        if m:
            if m.group(1) == "rust":
                sys.exit(f"{chapter}: @rust is retired with the Rust column — the "
                         f"concept belongs in the prose, the Rust spelling in the guide")
            body = m.group(2).strip("\n")
            para = "> **Harsh —** " + body.replace("\n", "\n> ")
        out.append(para)
    return "\n\n".join(out)

def build():
    if not os.path.exists(HRS):
        sys.exit("build hrs first: cargo build --release")
    chapters = sorted(glob.glob(os.path.join(BOOK, "[0-9][0-9]_*.md")))
    parts = []
    rendered = {}
    n = 0
    for ch in chapters:
        chapter = os.path.basename(ch)[:-3]
        source = open(ch).read()
        def sub(m):
            nonlocal n
            name, rest = m.group(1), m.group(2) or ""
            key = (chapter, name, rest)
            if key not in rendered:
                n += 1
                expect_error = "!error" in rest
                expect_panic = "!panic" in rest
                expect_test = ("!test!fail" in rest) if "!test" in rest else None
                args = tuple(a for a in rest.split() if not a.startswith("!") and not a.startswith("<"))
                stdin = ""
                sm = re.search(r'<"((?:[^"\\]|\\.)*)"', rest)
                if sm:
                    stdin = sm.group(1).encode().decode("unicode_escape")
                rendered[key] = render(chapter, name, expect_error, stdin, expect_panic, expect_test, args, "!doc" in rest)
            return rendered[key]
        check_prose(source, chapter)
        text = markers(source, chapter)
        text = re.sub(r"^@@ +(\S+)(.*)$", sub, text, flags=re.M)
        parts.append(text.rstrip("\n"))
    # Every snippet was transpiled, compiled and run to get here; the page
    # keeps the Harsh and the output and drops the Rust. The rust fences are
    # the harness's own -- a chapter never writes one by hand -- so this is a
    # mechanical strip, and it is what guarantees the rule.
    full = RUST_BLOCK.sub("", "\n\n".join(parts) + "\n")
    if "```rust" in full:
        sys.exit("a rust fence survived the strip")
    open(os.path.join(BOOK, "HARSH-BOOK.md"), "w").write(full)
    print(f"HARSH-BOOK.md: {len(chapters)} chapter(s), {n} snippet(s), all built and run")
    # The three forms are always produced together: .md, .ipynb, .html.
    r = subprocess.run([sys.executable, os.path.join(ROOT, "docs/build.py")], cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit(f"docs/build.py failed:\n{r.stderr}")
    print(r.stdout.strip())

if __name__ == "__main__":
    build()

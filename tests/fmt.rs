//! `hrs fmt` over the corpus: every `.hrs` in `examples/`, `examples/guide/`
//! and `book/src/`, all hand-written in the recommended layout.
//!
//! Three checks. Rule 0: formatting changes no token, so the formatted
//! source transpiles to the same Rust. Idempotence: formatting twice is
//! formatting once. The no-op: a file already in the recommended layout is
//! left as it is -- and when it is not, the diff is either a formatter bug
//! or a guide sentence to sharpen, and this test prints it.

use std::fs;
use std::path::{Path, PathBuf};

fn corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in [root.join("examples"), root.join("examples").join("guide")] {
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.flatten() {
                if e.path().extension().map_or(false, |x| x == "hrs") {
                    files.push(e.path());
                }
            }
        }
    }
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().map_or(false, |x| x == "hrs") {
                    out.push(p);
                }
            }
        }
    }
    walk(&root.join("book").join("src"), &mut files);
    files.sort();
    files
}

fn transpile(src: &str) -> Option<String> {
    let toks = harsh_lang::lex::lex(src).ok()?;
    let nodes = harsh_lang::layout::build(toks).ok()?;
    let mut em = harsh_lang::emit::Emitter::new(src);
    em.program(&nodes);
    Some(em.out)
}

fn tokens(s: &str) -> Vec<String> {
    harsh_lang::lex::lex_rust(s).map(|v| v.into_iter().filter(|t| !t.is_comment()).map(|t| t.text).collect()).unwrap_or_default()
}

fn first_diff(a: &str, b: &str) -> String {
    for (i, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}:\n  was: {x:?}\n  fmt: {y:?}", i + 1);
        }
    }
    format!("lengths differ: {} vs {} lines", a.lines().count(), b.lines().count())
}

#[test]
fn fmt_changes_no_token() {
    let files = corpus();
    // The development tree has the book's snippets too (266 files); a public
    // clone has the examples and the transpiler's own Harsh (43). Either is a
    // corpus; an empty one would mean the walk broke.
    assert!(files.len() > 40, "{} files", files.len());
    for f in &files {
        let src = fs::read_to_string(f).unwrap();
        let out = harsh_lang::fmt::format(&src);
        let (a, b) = (transpile(&src), transpile(&out));
        if a.is_none() {
            // A Book snippet that Harsh itself refuses, on purpose (`!error`
            // at the transpiler, not at rustc): nothing to compare, and the
            // formatter must have left it alone.
            assert_eq!(src, out, "{}: a file that does not transpile is not reformatted", f.display());
            continue;
        }
        assert!(b.is_some(), "{}: formatted source no longer transpiles", f.display());
        assert_eq!(tokens(&a.unwrap()), tokens(&b.unwrap()), "{}: rule 0 broken\n{}", f.display(), first_diff(&src, &out));
    }
}

#[test]
fn fmt_is_idempotent() {
    for f in corpus() {
        let src = fs::read_to_string(&f).unwrap();
        let once = harsh_lang::fmt::format(&src);
        let twice = harsh_lang::fmt::format(&once);
        assert_eq!(once, twice, "{}: not idempotent\n{}", f.display(), first_diff(&once, &twice));
    }
}

/// The corpus is hand-written in the recommended layout, so `fmt` over it is
/// a no-op. Files that would change are listed with their first differing
/// line; the count is the number to bring to zero.
#[test]
/// Every corpus file must transpile — except the ones the Book tags
/// `!error`, which exist to fail.
///
/// A file that does not transpile is invisible to several checks: `hrs fmt`
/// leaves it alone, so the no-op test passes over it, and the round trip
/// skips it. On 2026-09-18 a macro corpus file was refused by the new arm
/// reader and quietly stopped exercising six shapes; this is the guard.
#[test]
fn every_corpus_file_transpiles() {
    const DELIBERATELY_BROKEN: [&str; 1] = ["pipe_too_many.hrs"];
    let mut bad = Vec::new();
    for f in corpus() {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        if DELIBERATELY_BROKEN.contains(&name.as_str()) {
            continue;
        }
        let src = fs::read_to_string(&f).unwrap();
        let toks = match harsh_lang::lex::lex(&src) {
            Ok(t) => t,
            Err(e) => {
                bad.push(format!("{}: {}", f.display(), e.msg));
                continue;
            }
        };
        let taken = harsh_lang::layout::names_in_scope(&toks);
        let toks = match harsh_lang::mac::expand_all(toks, &taken) {
            Ok(t) => t,
            Err(e) => {
                bad.push(format!("{}: {}", f.display(), e.msg));
                continue;
            }
        };
        if let Err(e) = harsh_lang::layout::build(toks) {
            bad.push(format!("{}: {}", f.display(), e.msg));
        }
    }
    assert!(bad.is_empty(), "{} corpus file(s) do not transpile:\n{}", bad.len(), bad.join("\n"));
}

/// Every corpus example must not only transpile but **compile**. `check.sh`
/// transpiles the corpus; three guide examples transpiled to invalid Rust for
/// days before anyone ran `rustc` on them (2026-09-18). Examples that need a
/// dependency are listed and skipped.
#[test]
fn every_example_compiles() {
    const NEEDS_DEPENDENCIES: [&str; 1] = ["hello.hrs"];
    const DELIBERATELY_BROKEN: [&str; 1] = ["pipe_too_many.hrs"];
    let tmp = std::env::temp_dir().join(format!("harsh-compile-{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap();
    let hrs = env!("CARGO_BIN_EXE_hrs");
    let mut bad = Vec::new();
    for f in corpus() {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        if NEEDS_DEPENDENCIES.contains(&name.as_str()) || DELIBERATELY_BROKEN.contains(&name.as_str()) {
            continue;
        }
        let stem = name.trim_end_matches(".hrs").replace('.', "_");
        let rs = tmp.join(format!("{stem}.rs"));
        let t = std::process::Command::new(hrs).arg(&f).arg("-o").arg(&rs).output().unwrap();
        if !t.status.success() {
            bad.push(format!("{}: does not transpile", f.display()));
            continue;
        }
        let c = std::process::Command::new("rustc")
            .args(["--edition", "2021", "-A", "warnings", "--crate-type", "bin", "-o"])
            .arg(tmp.join(format!("{stem}.bin")))
            .arg(&rs)
            .output()
            .unwrap();
        if !c.status.success() {
            let err = String::from_utf8_lossy(&c.stderr);
            let first = err.lines().find(|l| l.starts_with("error")).unwrap_or("").to_string();
            bad.push(format!("{}: {first}", f.display()));
        }
    }
    let _ = fs::remove_dir_all(&tmp);
    assert!(bad.is_empty(), "{} example(s) do not compile:\n{}", bad.len(), bad.join("\n"));
}

#[test]
fn fmt_is_a_no_op_over_the_corpus() {
    let mut changed = Vec::new();
    for f in corpus() {
        let src = fs::read_to_string(&f).unwrap();
        let out = harsh_lang::fmt::format(&src);
        if out != src {
            changed.push(format!("{}\n  {}", f.display(), first_diff(&src, &out).replace('\n', "\n  ")));
        }
    }
    assert!(changed.is_empty(), "{} of {} files would change:\n{}", changed.len(), corpus().len(), changed.join("\n"));
}


/// The rules, one input each: what `hrs fmt` does to a file that is not in
/// the recommended layout. Nothing joins; tokens never change.
#[test]
fn fmt_shapes() {
    let cases: &[(&str, &str)] = &[
        // Block bodies indent one unit past the line holding the construct.
        ("fn main$:\n  let x = 1\n  x\n", "fn main$:\n    let x = 1\n    x\n"),
        ("fn main$:\n    let g = match n\\\n            1 => 2\n            _ => 3\n", "fn main$:\n    let g =\n        match n\\\n            1 => 2\n            _ => 3\n"),
        // `let x =` / `if c:` on the next line: the body from the `if` line;
        // the `else` under the `if`.
        (
            "fn main$:\n    let s =\n        if n > 0:\n                1\n        else:\n                2\n",
            "fn main$:\n    let s =\n        if n > 0:\n            1\n        else:\n            2\n",
        ),
        // Bracket contents one unit past the anchor, closer under it; the
        // `=` breaks before a multi-line group.
        (
            "fn main$:\n    let row = vec! [\n        1,\n        2,\n    ]\n",
            "fn main$:\n    let row =\n        vec! [\n            1,\n            2,\n        ]\n",
        ),
        // A paren block: `(` line-final, prototype and body beneath, `)`
        // under the callee; the chain resumes under its arrow.
        (
            "fn main$:\n    let d: Vec<i32> =\n        v <- iter$\n            <- map (\n                |x|:\n                    x * 10\n            )\n            <- collect$\n",
            "fn main$:\n    let d: Vec<i32> =\n        v <- iter$\n          <- map (\n                 |x|:\n                     x * 10\n             )\n          <- collect$\n",
        ),
        // The compact prototype with the `)` on its own line is opened up.
        (
            "fn main$:\n    v <- iter$\n      <- map (|x|:\n            x * 10\n        )\n      <- count$\n",
            "fn main$:\n    v <- iter$\n      <- map (\n             |x|:\n                 x * 10\n         )\n      <- count$\n",
        ),
        // The compact form with the closer ending the body is kept.
        ("fn main$:\n    v <- iter$\n      <- map (|x|:\n                  x * 10)\n      <- count$\n", "fn main$:\n    v <- iter$\n      <- map (|x|:\n                  x * 10)\n      <- count$\n"),
        // A long receiver: links one unit past its line; the `=` breaks
        // before a chain of two or more links.
        (
            "fn main$:\n    let s = [1, 2] <- iter$\n                   <- map (|x| x)\n                   <- sum$\n", "fn main$:\n    let s =\n        [1, 2] <- iter$\n               <- map (|x| x)\n               <- sum$\n"),
        // One or two links fit on one line: joined.
        // One link: unchanged, written vertically or not (rule 1); its column
        // follows the receiver rule (`File.open "x"` is 13 wide: hangs).
        ("fn main$:\n    let f = File.open \"x\"\n            <- expect \"y\"\n", "fn main$:\n    let f = File.open \"x\"\n        <- expect \"y\"\n"),
        // ... unless the line would pass 72 columns: then every link alone.
        (
            "fn main$:\n    let contents = fs.read_to_string (config <- file_path) <- expect \"Should have been able to read the file\"\n",
            "fn main$:\n    let contents = fs.read_to_string (config <- file_path) <- expect \"Should have been able to read the file\"\n",
        ),
        (
            "fn main$:\n    thread.spawn (move || println! \"From thread: {list:?}\")\n        <- join$\n        <- unwrap$\n",
            "fn main$:\n    thread.spawn (move || println! \"From thread: {list:?}\")\n        <- join$\n        <- unwrap$\n",
        ),
        // A `=` value too long as one line but fitting alone: break after
        // `=`, chain horizontal.
        (
            "fn main$:\n    let statuses: Vec<Status> = (0u32..3) <- map Status.Value <- collect<Vec<_>>$\n",
            "fn main$:\n    let statuses: Vec<Status> =\n        (0u32..3) <- map Status.Value <- collect<Vec<_>>$\n",
        ),
        // Three or more links: vertical, whatever their length, the
        // receiver on the line after `=`.
        ("fn main$:\n    v <- iter$ <- map (|x| x)\n      <- count$\n", "fn main$:\n    v <- iter$\n      <- map (|x| x)\n      <- count$\n"),
        (
            "fn main$:\n    let message = receiver <- lock$ <- unwrap$ <- recv$\n",
            "fn main$:\n    let message =\n        receiver <- lock$\n                 <- unwrap$\n                 <- recv$\n",
        ),
        // Arrows on separate operands are not a chain.
        (
            "fn main$:\n    self <- width > other <- width && self <- height > other <- height\n",
            "fn main$:\n    self <- width > other <- width && self <- height > other <- height\n",
        ),
        // `where` bounds keep their nesting; braces keep their shape.
        (
            "fn three<T> (a: T) -> String\n  where\n      T: Clone:\n  format! \"{a:?}\"\n",
            "fn three<T> (a: T) -> String\n    where\n        T: Clone:\n    format! \"{a:?}\"\n",
        ),
        // A struct literal is a block: its fields one unit past the name.
        ("fn main$:\n    let p =\n        Point\\\n                x = 1\n                y = 2\n", "fn main$:\n    let p =\n        Point\\\n            x = 1\n            y = 2\n"),
        // Braces keep their shape only where they still stand: a one-line block.
        ("fn main$:\n    let h = { let u = 3; u * u }\n", "fn main$:\n    let h = { let u = 3; u * u }\n"),
        // Trailing whitespace goes; the file ends with one newline.
        ("fn main$:   \n    1  \n\n\n", "fn main$:\n    1\n"),
    ];
    for (src, want) in cases {
        let got = harsh_lang::fmt::format(src);
        assert_eq!(&got, want, "\n  input:\n{src}\n  got:\n{got}");
        assert_eq!(harsh_lang::fmt::format(&got), got, "idempotent:\n{got}");
        let a = transpile(src).unwrap_or_else(|| panic!("input does not transpile:\n{src}"));
        let b = transpile(&got).unwrap_or_else(|| panic!("output does not transpile:\n{got}"));
        assert_eq!(tokens(&a), tokens(&b), "rule 0:\n{src}");
    }
}


/// The receiver decides a vertical chain's shape (the user's algorithm,
/// 2026-09-09): links two-to-last on their own lines; the first link stays
/// on the receiver's line when its arrow sits within `LINK_ALIGN` columns
/// of the line's start (the links align under it), else the receiver
/// stands alone and every link hangs one unit in. One link never breaks;
/// two fit on a line within `CHAIN_WIDTH`.
#[test]
fn the_receiver_decides() {
    let src = "fn f$:\n    let a = very_long_receiver_name <- very_long_method_name\n    let b = receiver <- method <- method\n    let c = very_long_receiver_name <- very_long_method_name <- very_long_method_name <- more\n    let d = receiver <- very_long_method_name <- very_long_method_name <- very_long_method_name\n    receiver <- very_long_method_name <- very_long_method_name <- very_long_method_name <- more\n    very_long_receiver_name <- very_long_method_name <- very_long_method_name <- more\n";
    let want = "fn f$:\n    let a = very_long_receiver_name <- very_long_method_name\n    let b = receiver <- method <- method\n    let c =\n        very_long_receiver_name\n            <- very_long_method_name\n            <- very_long_method_name\n            <- more\n    let d =\n        receiver <- very_long_method_name\n                 <- very_long_method_name\n                 <- very_long_method_name\n    receiver <- very_long_method_name\n             <- very_long_method_name\n             <- very_long_method_name\n             <- more\n    very_long_receiver_name\n        <- very_long_method_name\n        <- very_long_method_name\n        <- more\n";
    assert_eq!(harsh_lang::fmt::format(src), want);
    assert_eq!(harsh_lang::fmt::format(want), want, "idempotent");
}

/// Arguments beneath the callee (the user's rule, 2026-09-10): an
/// application whose line passes the width, or that has a block argument
/// beside another, lists every argument on its own line one unit past the
/// callee -- recursively, an application inside an argument measured from
/// where it now stands. One argument stays with its callee; a lone block
/// argument is the paren block's own shape.
#[test]
fn arguments_beneath_the_callee() {
    let src = "fn f$:\n    let app =\n        Router.new$ <- leptos_routes (&leptos_options) routes (do:\n                            let leptos_options = leptos_options <- clone$\n                            move || shell (leptos_options <- clone$)\n                        )\n                    <- fallback (leptos_axum.file_and_error_handler shell)\n                    <- with_state leptos_options\n    println! \"The area of the rectangle is {} square pixels.\" (area width1 height1)\n    app\n";
    let want = "fn f$:\n    let app =\n        Router.new$ <- leptos_routes\n                           (&leptos_options)\n                           routes\n                           (do:\n                                let leptos_options = leptos_options <- clone$\n                                move || shell (leptos_options <- clone$)\n                           )\n                    <- fallback (leptos_axum.file_and_error_handler shell)\n                    <- with_state leptos_options\n    println!\n        \"The area of the rectangle is {} square pixels.\"\n        (area width1 height1)\n    app\n";
    assert_eq!(harsh_lang::fmt::format(src), want);
    assert_eq!(harsh_lang::fmt::format(want), want, "idempotent");
}

/// Rule 3, recursion: a chain inside an argument is judged by the same
/// count from its own receiver; a paren block's tail continues its header's
/// chain, so `)` / `<- fallback` / `<- with_state` go vertical under the
/// header's arrow when the whole is three links.
#[test]
fn chains_recurse_and_tails_continue() {
    let src = "fn f$:\n    let e = xs <- iter$ <- map (|x| x <- foo$ <- bar$ <- baz$ <- qux$) <- collect$\n    let app =\n        Router.new$ <- leptos_routes\n                           (&leptos_options)\n                           routes\n                           (do:\n                                let leptos_options = leptos_options <- clone$\n                                move || shell (leptos_options <- clone$)\n                           ) <- fallback (leptos_axum.file_and_error_handler shell) <- with_state leptos_options\n    app\n";
    let want = "fn f$:\n    let e =\n        xs <- iter$\n           <- map (|x| x <- foo$\n                         <- bar$\n                         <- baz$\n                         <- qux$)\n           <- collect$\n    let app =\n        Router.new$ <- leptos_routes\n                           (&leptos_options)\n                           routes\n                           (do:\n                                let leptos_options = leptos_options <- clone$\n                                move || shell (leptos_options <- clone$)\n                           )\n                    <- fallback (leptos_axum.file_and_error_handler shell)\n                    <- with_state leptos_options\n    app\n";
    assert_eq!(harsh_lang::fmt::format(src), want);
    assert_eq!(harsh_lang::fmt::format(want), want, "idempotent");
}

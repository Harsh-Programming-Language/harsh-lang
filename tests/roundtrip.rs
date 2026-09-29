// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Regression tests that run on every `cargo test`.
//!
//! The valuable one is `round_trip`: Rust -> Harsh -> Rust over a corpus. It
//! catches the class of bug that hand-written examples miss, because it
//! exercises real code rather than code written to demonstrate a feature.

use std::fs;
use std::path::Path;

fn transpile(harsh: &str) -> String {
    // The same pipeline the driver runs: a Rust `macro_rules!` is blanked out
    // before anything reads it and put back verbatim after.
    // The driver's path, in memory: Rust zones and a Rust macro's brace
    // bodies set aside, Harsh's own macros expanded, the rest read as a
    // program, and the bodies put back with their holes transpiled.
    harsh_lang::driver::transpile_str(harsh).expect("transpile")
}

fn convert(rust: &str) -> String {
    harsh_lang::unbrace::convert(rust).expect("convert")
}

/// Token stream with whitespace, comments and optional trailing commas removed.
fn norm_tokens(s: &str) -> Vec<String> {
    let mut out: Vec<String> = harsh_lang::lex::lex_rust(s)
        .expect("lex_rust")
        .into_iter()
        .filter(|t| !t.is_comment())
        .map(|t| t.text)
        .collect();
    // A macro call's delimiter means nothing to the macro: Rust hands it the
    // stream inside, so `vec![a]`, `vec!(a)` and `vec!{a}` are one call (the
    // user's delimiter principle, 2026-09-22). Harsh writes `vec! a` and emits
    // parens, so the delimiters of a group opened right after `name!` compare
    // as parens.
    let mut stack: Vec<bool> = Vec::new();
    for i in 0..out.len() {
        let opener = matches!(out[i].as_str(), "(" | "[" | "{");
        if opener {
            let bang = i >= 2 && out[i - 1] == "!" && out[i - 2].chars().next().map_or(false, |c| c.is_alphanumeric() || c == '_');
            stack.push(bang);
            if bang {
                out[i] = "(".into();
            }
        } else if matches!(out[i].as_str(), ")" | "]" | "}") {
            if stack.pop() == Some(true) {
                out[i] = ")".into();
            }
        }
    }
    // Rust accepts a trailing comma before any closer, and after a block-bodied
    // match arm. Both are inserted by the layout rules and carry no meaning.
    let mut i = 0;
    while i < out.len() {
        let before_closer = matches!(
            out.get(i + 1).map(String::as_str),
            Some("}") | Some(")") | Some("]")
        );
        let after_block = i > 0 && out[i - 1] == "}";
        if out[i] == "," && (before_closer || after_block) {
            out.remove(i);
        } else {
            i += 1;
        }
    }
    out
}

fn same_tokens(a: &str, b: &str) -> bool {
    norm_tokens(a) == norm_tokens(b)
}

fn first_diff(a: &str, b: &str) -> String {
    let (x, y) = (norm_tokens(a), norm_tokens(b));
    for i in 0..x.len().min(y.len()) {
        if x[i] != y[i] {
            let lo = i.saturating_sub(8);
            return format!(
                "      at {i}\n      orig: {}\n      back: {}",
                x[lo..(i + 8).min(x.len())].join(" "),
                y[lo..(i + 8).min(y.len())].join(" ")
            );
        }
    }
    format!("      lengths {} vs {}", x.len(), y.len())
}

fn corpus() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for dir in ["src", "src/bin"] {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        let Ok(rd) = fs::read_dir(&p) else { continue };
        for e in rd.flatten() {
            let path = e.path();
            if path.extension().map(|x| x == "rs").unwrap_or(false) {
                let name = path.display().to_string();
                if let Ok(s) = fs::read_to_string(&path) {
                    out.push((name, s));
                }
            }
        }
    }
    out
}

#[test]
fn round_trip_own_source() {
    let mut failures = Vec::new();
    for (name, rust) in corpus() {
        let harsh = convert(&rust);
        let back = transpile(&harsh);
        if !same_tokens(&rust, &back) {
            failures.push(format!("{}\n{}", name, first_diff(&rust, &back)));
        }
    }
    assert!(failures.is_empty(), "round trip changed: {:?}", failures);
}

#[test]
fn examples_transpile() {
    // `examples/` and `examples/guide/` -- the guide's snippets included,
    // which this walk once missed.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut n = 0;
    for dir in [root.clone(), root.join("guide")] {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let path = e.path();
            if path.extension().map(|x| x == "hrs").unwrap_or(false) {
                let src = fs::read_to_string(&path).unwrap();
                // The driver's whole path: zones and brace bodies set aside,
                // Harsh's own macros expanded (an expansion that produced a
                // brace body read again), the layout, the emitter. Until
                // 2026-09-23 this walk only lexed and laid out, and a guide
                // example whose `~` call did not expand went unnoticed.
                harsh_lang::driver::transpile_str(&src)
                    .unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
                n += 1;
            }
        }
    }
    assert!(n > 40, "only {n} example files found");
}

#[test]
fn rejects_rust_spellings() {
    assert!(harsh_lang::lex::lex("fn main$:\n    let x = a::b\n").is_err(), "`::` must be rejected");
}

/// Each case is (harsh, expected substring of the emitted Rust).
fn check(cases: &[(&str, &str)]) {
    for (harsh, want) in cases {
        let got = transpile(harsh);
        assert!(
            got.contains(want),
            "\n  input:    {harsh:?}\n  expected: {want:?}\n  got:      {got:?}"
        );
    }
}

#[test]
fn paths_and_members() {
    check(&[
        ("fn main$:\n    let a = std.fmt.Debug\n", "std::fmt::Debug"),
        ("fn main$:\n    let a = s <- len$\n", "s.len()"),
        ("fn main$:\n    let a = s<-len$\n", "s.len()"),
        ("fn main$:\n    let a = t.0\n", "t.0"),
        ("fn main$:\n    let a = 1.5\n", "1.5"),
        ("fn main$:\n    for i in 0..3:\n        f$\n", "0..3"),
    ]);
}

#[test]
fn generics_turbofish() {
    check(&[
        ("fn main$:\n    let v = Vec<i32>.new$\n", "Vec::<i32>::new()"),
        ("fn f<T> (x: T) -> T:\n    x\n", "fn f<T>(x: T) -> T"),
        ("fn main$:\n    let v: Vec<i32> = q$\n", "let v: Vec<i32>"),
    ]);
}

#[test]
fn curried_parameters() {
    check(&[
        ("fn g name: &str -> String:\n    x\n", "fn g(name: &str) -> String"),
        ("fn g (a: i32) (b: i32) -> i32:\n    a\n", "fn g(a: i32, b: i32) -> i32"),
        ("fn g (u: ()) (b: i32) -> i32:\n    b\n", "fn g(u: (), b: i32) -> i32"),
        ("fn g$ -> i32:\n    1\n", "fn g() -> i32"),
        // A one-token bare parameter: `self` by value.
        ("impl A\n    fn g self -> i32:\n        1\n", "fn g(self) -> i32"),
        // A bare parameter ends at a `[where …]` clause as it does at `->`.
        ("fn g<T> x: &T\n    [where T: Clone]:\n    1\n", "fn g<T>(x: &T) where T: Clone {"),
    ]);
}

#[test]
fn block_separators() {
    check(&[
        ("struct P\n    x: i32\n    y: i32\n", "x: i32,"),
        ("enum E\n    A\n    B\n", "A,"),
        ("trait T\n    fn a (&self) -> i32\n", "fn a(&self) -> i32;"),
        ("fn main$:\n    let x = do:\n        1\n", "let x = {"),
        ("fn main$:\n    match x\\\n        A => 1\n        B => 2\n", "A => 1,"),
    ]);
}

#[test]
fn semicolons() {
    check(&[
        ("fn f$:\n    let a = 1\n    a\n", "let a = 1;"),
        ("pub const X: i32 = 1\nfn f$:\n    ()\n", "pub const X: i32 = 1;"),
        ("fn f$:\n    let x = if c:\n        1\n    else:\n        2\n    x\n", "};"),
        ("fn f$:\n    if let Some p = q:\n        g$\n    h$\n", "}\n    h()"),
    ]);
}

/// A tight `[..]` is part of its atom, so `f arr[1]` is `f(arr[1])`; a spaced
/// `[` inside an application is refused, since it could only mean an index
/// of the wrong thing. Brackets never apply. The user's rule, 2026-09-11.
#[test]
fn tight_index_is_an_atom_and_spaced_index_in_an_application_is_refused() {
    check(&[
        ("fn f$:\n    g arr[1]\n", "g(arr[1])"),
        ("fn f$:\n    g arr[1] x\n", "g(arr[1], x)"),
        ("fn f$:\n    g (arr [1])\n", "g(arr [1])"),
        ("fn f$:\n    g t.0[1].2\n", "g(t.0[1].2)"),
        ("fn f$:\n    let a = arr [1]\n", "let a = arr [1];"),
        ("fn f$:\n    let a = v[g l 2]\n", "let a = v[g(l, 2)];"),
        ("fn f$:\n    let b = (v[g (l) 1])\n", "let b = v[g(l, 1)];"),
        ("fn f$:\n    let c = vec! (g (l + 1) 2)\n", "let c = vec!(g(l + 1, 2));"),
        ("fn f$:\n    assert_eq! v ([1, 2])\n", "assert_eq!(v, [1, 2])"),
    ]);
    // A macro with an argument before a spaced `[` is refused too. A `[`
    // right after the bang is the first argument, an array (A1, 2026-09-22).
    let toks = harsh_lang::lex::lex("fn f$:\n    assert_eq! v [1, 2]\n").expect("lex");
    assert!(harsh_lang::layout::build(toks).is_err());
    let toks = harsh_lang::lex::lex("fn f$:\n    g arr [1]\n").expect("lex");
    let err = harsh_lang::layout::build(toks).err().expect("refused").msg;
    assert!(err.contains("`arr [` inside an application"), "{err}");
}

/// The written `;` reaches the Rust from an inline block too: `do: f$;` is
/// `{ f(); }`, as its indented form is. Found by Harshlings' runner, whose
/// arm `Some ex => do: run_one (&ex);` came out returning a `bool`.
#[test]
fn inline_block_keeps_its_written_semicolon() {
    check(&[
        ("fn f$:\n    let x = 1\n    if x == 1: g$;\n", "if x == 1 {\n        g();\n    }"),
        ("fn f$:\n    let y = do: g$;\n", "let y = {\n        g();\n    };"),
        ("fn f$:\n    match 1\\\n        1 => do: g$;\n        _ => ()\n", "1 => {\n            g();\n        },"),
    ]);
    // And the converter writes it back, so the trip is stable.
    let rust = "fn f() {\n    if true {\n        g();\n    }\n}\n";
    let harsh = convert(rust);
    assert!(harsh.contains("g$;"), "{harsh}");
    assert!(transpile(&harsh).contains("g();"), "{}", transpile(&harsh));
}

/// `;` marks a discarded tail value. It is legal only in statement blocks, and
/// must never double up with an inserted separator.
#[test]
fn semicolon_discipline() {
    // Tail expression kept, or discarded when written explicitly.
    check(&[
        ("fn f$ -> i32:\n    let a = 1\n    a\n", "    a\n}"),
        ("fn f$:\n    let a = 1\n    a;\n", "    a;\n}"),
        ("fn f$:\n    g$\n    h$\n", "g();"),
    ]);

    // No output anywhere may contain a doubled or mixed separator. A `;` is
    // legal only on a block's last statement, so these place it only there.
    let sources = [
        "fn a$:\n    let x = 1\n    let y = 2;\n",
        "fn b$:\n    f$\n    g$;\n",
        "fn c$ -> i32:\n    let v = do:\n        1;\n    2\n",
        "fn d$:\n    let z = vec! { 0; 4 }\n    let w = vec! { 0; 4 }\n    q z w\n",
        "fn e$:\n    if c:\n        f$;\n    g$;\n",
        "use std.fmt\nfn f$:\n    ()\n",
        "struct P\n    x: i32\n    y: i32\n",
        "fn g$ -> i32:\n    match x\\\n        A => 1\n        B => 2\n",
    ];
    for src in sources {
        let got = transpile(src);
        for bad in [";;", ";,", ",;", ",,"] {
            assert!(!got.contains(bad), "{bad:?} in output of {src:?}:\n{got}");
        }
    }
}

/// A macro is applied exactly like a function, with no exemption for pattern
/// or DSL contents: `matches! x (Some n if n > 1)` is the spelling, and the
/// parenthesised-list form is a tuple, rejected like any other.
#[test]
fn macros_juxtapose_like_functions() {
    check(&[
        ("fn m$:\n    let a = matches! x (Some n if n > 1)\n", "matches!(x, Some(n) if n > 1)"),
        ("fn m$:\n    let b = matches! a (1 | 2 | 5)\n", "matches!(a, 1 | 2 | 5)"),
        ("fn m$:\n    let c = matches! r (Ok 7)\n", "matches!(r, Ok(7))"),
        ("fn m$:\n    assert_eq! a 5\n", "assert_eq!(a, 5)"),
    ]);
    let toks = harsh_lang::lex::lex("fn m$:\n    let a = matches! (x, Some(n) if n > 1)\n").expect("lex");
    assert!(harsh_lang::layout::build(toks).is_err(), "the tuple spelling must be rejected");
}

/// `;` has no meaning where a `,` separator is inserted, and on any statement
/// but a block's last the newline already ends it. Both are rejected rather
/// than silently swallowed.
#[test]
fn rejects_semicolon_in_comma_blocks() {
    for src in [
        // Mid-block `;` on a statement that is not last.
        "fn a$:\n    let x = 1;\n    let y = 2\n",
        "fn b$:\n    f$;\n    g$\n",
        "fn g$ -> i32:\n    match x\\\n        A => 1;\n        B => 2\n",
        "struct P\n    x: i32;\n    y: i32\n",
        "enum E\n    A;\n    B\n",
    ] {
        let toks = harsh_lang::lex::lex(src).expect("lex");
        assert!(harsh_lang::layout::build(toks).is_err(), "should reject: {src:?}");
    }
}

/// A comma block inserts its own commas, so a written one is a second
/// spelling and is rejected rather than emitted doubled.
#[test]
fn rejects_comma_in_comma_blocks() {
    for src in [
        "fn g$ -> i32:\n    match x\\\n        A => 1,\n        B => 2\n",
        "fn g$ -> i32:\n    match x\\\n        A => 1\n        B => 2,\n",
        "struct P\n    x: i32,\n    y: i32\n",
        "enum E\n    A,\n    B\n",
    ] {
        let toks = harsh_lang::lex::lex(src).expect("lex");
        assert!(harsh_lang::layout::build(toks).is_err(), "should reject: {src:?}");
    }
}

/// One parameter per group. Rust's comma list inside a `fn` declaration's
/// parentheses is rejected at the comma, in a declaration with a body and in
/// a trait signature alike; tuple patterns and `fn` pointer types keep their
/// commas, since those are not parameter separators.
#[test]
fn rejects_comma_parameter_lists() {
    for src in [
        "fn f (a: i32, b: i32) -> i32:\n    a\n",
        "trait T\n    fn f (a: i32, b: i32) -> i32;\n",
        "impl S\n    fn m (&self, x: i32) -> i32:\n        x\n",
    ] {
        let toks = harsh_lang::lex::lex(src).expect("lex");
        assert!(harsh_lang::layout::build(toks).is_err(), "should reject: {src:?}");
    }
    check(&[
        ("fn t ((a, b): (i32, i32)) -> i32:\n    a + b\n", "fn t((a, b): (i32, i32)) -> i32"),
        ("fn p (g: fn i32 i32 -> i32) (v: i32) -> i32:\n    g v v\n", "fn p(g: fn(i32, i32) -> i32, v: i32) -> i32"),
        ("impl S\n    fn m (&self) (x: i32) -> i32:\n        x\n", "fn m(&self, x: i32) -> i32"),
    ]);
}

/// The converter writes one group per parameter, drops a trailing comma, and
/// leaves tuple patterns and `fn` pointer types alone.
#[test]
fn converter_splits_parameter_groups() {
    let got = harsh_lang::unbrace::convert(
        "fn add(a: i32, b: i32,) -> i32 { a + b }\nimpl S { fn m(&self, (p, q): (i32, i32), g: fn(i32, i32) -> i32) -> i32 { p } }\n",
    )
    .expect("convert");
    assert!(got.contains("fn add (a: i32) (b: i32) -> i32:"), "{got}");
    assert!(got.contains("fn m (&self) ((p, q): (i32, i32)) (g: fn i32 i32 -> i32) -> i32:"), "{got}");
}

/// A trailing comment stays trailing: the inserted separator goes before it.
#[test]
fn trailing_comment_keeps_separator() {
    check(&[
        ("fn m$:\n    let a = 42  // note\n    let b = a\n", "let a = 42; // note"),
        ("struct P\n    x: i32  // the x\n    y: i32\n", "x: i32, // the x"),
        // A block header's trailing comment goes after the brace, not before it.
        ("enum M\n    Quit\n    Move  // named fields\n        x: i32\n", "Move { // named fields"),
        ("fn m$:  // note\n    1\n", "fn m() { // note"),
    ]);
}

/// Grouped imports have one spelling, the paren tree. A colon block after
/// `use` is rejected rather than emitted as the invalid `use a::b {`.
#[test]
fn rejects_use_block() {
    let toks = harsh_lang::lex::lex("use std.io:\n    Read\n    Write\n").expect("lex");
    assert!(harsh_lang::layout::build(toks).is_err());
    check(&[("use std.io.(\n    Read,\n    Write\n)\n", "use std::io::{\n    Read,\n    Write\n};")]);
}

/// `[where ..]` -- the brackets suppress layout so the bound's `:` cannot be
/// mistaken for a block opener. They are stripped on emission.
#[test]
fn bracketed_where_clause() {
    check(&[
        (
            "fn f<T> (x: T) -> T\n    [where T: Clone + Send]:\n    x\n",
            "fn f<T>(x: T) -> T where T: Clone + Send {",
        ),
        (
            "fn g<T> (a: T) (b: T) -> T\n    [where T: Clone]:\n    a\n",
            "fn g<T>(a: T, b: T) -> T where T: Clone {",
        ),
        // Single-line form still works and is untouched.
        ("fn h<T> (x: T) -> T where T: Clone:\n    x\n", "-> T where T: Clone {"),
    ]);
}

/// Rust forbids a struct literal in a scrutinee or condition; Harsh does not,
/// so the expression is parenthesised on the way out. (A literal is built
/// with `P\\ x = 1`, a block; inside a header it is bound first -- a block
/// cannot open inside another block's header line.)
#[test]
fn parenthesised_scrutinee() {
    check(&[
        (
            "fn f$ -> i32:\n    let p = P\\ x = 1\n    match p\\\n        P\\ x => x\n",
            "match p {",
        ),
        (
            "fn f (p: P) -> i32:\n    let q = P\\ x = 1\n    if p == q:\n        1\n    else:\n        0\n",
            "if p == q {",
        ),
        // No brace in the scrutinee: no parens added.
        ("fn f (a: i32) -> i32:\n    match a\\\n        1 => 2\n", "match a {"),
        ("fn f (a: i32) -> i32:\n    if a > 1:\n        2\n    else:\n        0\n", "if a > 1 {"),
    ]);
}

/// Juxtaposed application: `f a b` is a call, `(a, b)` is always a tuple.
#[test]
fn juxtaposition() {
    check(&[
        ("fn m$:\n    h x\n", "h(x)"),
        ("fn m$:\n    let p = Point.new 3 4\n", "Point::new(3, 4)"),
        ("fn m$:\n    println! \"{}\" (greet \"Ben\")\n", "println!(\"{}\", greet(\"Ben\"))"),
        ("fn m$:\n    format! \"Hello, {n}\"\n", "format!(\"Hello, {n}\")"),
        ("fn m$:\n    let v = m <- entry k <- or_insert 0\n", "m.entry(k).or_insert(0)"),
        // A parenthesised group with commas is a tuple, never an argument list.
        ("fn m$:\n    let t = f (a, b)\n", "f((a, b))"),
        // Empty parens are a zero-argument call, not a unit argument.
        ("fn m$:\n    let u = g$\n", "g()"),
        // A tuple index is part of its atom: two arguments, not one and a stray.
        ("fn m$:\n    show t.0 t.1\n", "show(t.0, t.1)"),
        ("fn m$:\n    let a = f t.0.1 x\n", "f(t.0.1, x)"),
        // Declarations expose only their initialiser: `let mut sum` is not a call.
        ("fn m$:\n    let mut sum = 0\n", "let mut sum = 0;"),
    ]);
}

/// Rust call syntax written into a macro is rejected rather than silently
/// becoming a tuple argument.
#[test]
fn rejects_rust_call_syntax_in_macros() {
    for src in [
        "fn m$:\n    println!(\"{}\", greet(\"Ben\"))\n",
        "fn m$:\n    format!(\"{} {}\", a, b)\n",
    ] {
        let toks = harsh_lang::lex::lex(src).expect("lex");
        assert!(harsh_lang::layout::build(toks).is_err(), "should reject: {src:?}");
    }
    // A genuine tuple argument to a macro is still expressible.
    let toks = harsh_lang::lex::lex("fn m$:\n    dbg! ((a, b))\n").expect("lex");
    assert!(harsh_lang::layout::build(toks).is_ok());
}

/// A colon that is not the last token opens an inline block, closed by the end
/// of the logical line, by `else`, or by a separator that is not its own. Items
/// inside carry the separator the indented form would have inserted.
#[test]
fn inline_blocks() {
    check(&[
        ("fn a (t: bool) -> i32:\n    if t: 1 else: 2\n", "if t {"),
        ("fn b (x: E) -> i32:\n    match x\\ A => 1, B => 2\n", "A => 1,"),
        ("fn c$ -> i32:\n    { let a = 1; a * 2 }\n", "let a = 1;"),
        ("fn d$:\n    for i in 0..3: f i\n", "for i in 0..3 {"),
        ("struct P\\ x: i32, y: i32\n", "x: i32,"),
    ]);
}

/// Each `else` binds to the nearest open `if`; braces select the other reading.
#[test]
fn inline_else_binding() {
    let nearest = transpile("fn e (a: bool) (b: bool) -> i32:\n    if a: if b: 1 else: 2 else: 3\n");
    // else 2 sits inside the outer block, else 3 outside it.
    assert!(nearest.contains("        else {\n            2"), "{nearest}");
    assert!(nearest.contains("    else {\n        3"), "{nearest}");

    let outer = transpile(
        "fn e (a: bool) (b: bool) -> i32:\n    if a: if b { 1 } else { 0 } else: 2\n",
    );
    // The braced inner `if` is complete, so the trailing else binds outward.
    assert!(outer.contains("if b { 1 } else { 0 }"), "{outer}");
    assert!(outer.contains("    else {\n        2"), "{outer}");
}

/// The inline and indented forms of the same program emit identical Rust.
#[test]
fn inline_matches_indented() {
    let pairs = [
        (
            "fn a (t: bool) -> i32:\n    if t: 1 else: 2\n",
            "fn a (t: bool) -> i32:\n    if t:\n        1\n    else:\n        2\n",
        ),
        (
            "fn b (x: E) -> i32:\n    match x\\ A => 1, B => 2\n",
            "fn b (x: E) -> i32:\n    match x\\\n        A => 1\n        B => 2\n",
        ),
        (
            "struct P\\ x: i32, y: i32\n",
            "struct P\n    x: i32\n    y: i32\n",
        ),
    ];
    for (inline, indented) in pairs {
        assert_eq!(transpile(inline), transpile(indented), "inline: {inline:?}");
    }
}

/// A closure is an ordinary argument atom. When it ends a block header, the
/// block is its body and the argument list closes after the `}`.
#[test]
fn closures() {
    check(&[
        // Indented body, bound to a name.
        ("fn m$:\n    let f = |x: i32| -> i32:\n        x * 2\n", "let f = |x: i32| -> i32 {"),
        ("fn m$:\n    let g = |x: i32| do:\n        x * 4\n", "let g = |x: i32| {"),
        ("fn m$:\n    let z = ||:\n        7\n", "let z = || {"),
        // Trailing closure argument: the paren closes after the block.
        ("fn m$:\n    v <- map |n|:\n        n * 2\n", "v.map(|n| {"),
        // Inline closure body is not read as arguments to the closure.
        ("fn m$:\n    let a = v <- map (|n| n * 2)\n", "v.map(|n| n * 2)"),
    ]);
    // The closing paren lands after the brace, not at the end of the header.
    let got = transpile("fn m$:\n    v <- map |n|:\n        n * 2\n");
    assert!(got.contains("})"), "{got}");
}

/// The pipes fill a function's parameters -- `|>` from the left, `<|` from
/// the right -- and defer the rest as one flat closure; with the arity met
/// they are a plain call. Each side is a sequence of atoms.
#[test]
fn pipes() {
    let sub = "fn sub (a: i32) (b: i32) (c: i32) -> i32:\n    a\n";
    check(&[
        (&format!("{sub}fn m$:\n    let a = 3 |> sub\n"), "let a = move |__hrs1, __hrs2| sub(3, __hrs1, __hrs2);"),
        (&format!("{sub}fn m$:\n    let b = 3 5 |> sub\n"), "let b = move |__hrs1| sub(3, 5, __hrs1);"),
        (&format!("{sub}fn m$:\n    let c = 3 5 10 |> sub\n"), "let c = sub(3, 5, 10);"),
        (&format!("{sub}fn m$:\n    let d = sub <| 3\n"), "let d = move |__hrs1, __hrs2| sub(__hrs1, __hrs2, 3);"),
        (&format!("{sub}fn m$:\n    let e = sub <| 3 5\n"), "let e = move |__hrs1| sub(__hrs1, 3, 5);"),
        (&format!("{sub}fn m$:\n    let f = sub <| 3 5 10\n"), "let f = sub(3, 5, 10);"),
        // The middle hole: left and right on one function.
        (&format!("{sub}fn m$:\n    let g = 1 |> sub <| 3\n"), "let g = move |__hrs1| sub(1, __hrs1, 3);"),
        // Unknown arity: the atoms given are all of them, a plain call.
        // `f$` is one atom, so an empty call pipes without isolation.
        ("fn m$:\n    let a = merge <| routes_hello$\n", "merge(routes_hello())"),
        ("fn m$:\n    let a = routes_hello$ |> merge\n", "merge(routes_hello())"),
        // `()` is the unit value and pipes like any atom.
        ("fn m$:\n    let a = () |> send\n", "send(())"),
        ("fn m$:\n    let b = f <| g <| x\n", "f(g(x))"),
        ("fn m$:\n    let c = x |> f\n", "f(x)"),
        ("fn m$:\n    let d = x |> f |> g\n", "g(f(x))"),
        ("fn m$:\n    let e = h (f <| x)\n", "h(f(x))"),
        // A result is piped by isolating the application.
        ("fn m$:\n    let e = (f a) |> g\n", "g(f(a))"),
        // A partial bound by `let` composes; so does an anonymous one.
        (&format!("{sub}fn m$:\n    let s3 = 3 |> sub\n    let r = 5 |> s3\n    let t = 5 10 |> s3\n"), "let t = s3(5, 10);"),
        (&format!("{sub}fn m$:\n    let s3 = 3 |> sub\n    let r = 5 |> s3\n"), "let r = move |__hrs1| s3(5, __hrs1);"),
        (&format!("{sub}fn m$:\n    let t = 5 |> (3 |> sub)\n"), "let t = move |__hrs1| (move |__hrs1, __hrs2| sub(3, __hrs1, __hrs2))(5, __hrs1);"),
    ]);
}

/// Misuse is a Harsh error, not a rustc one.
#[test]
fn rejects_bad_pipes() {
    let sub = "fn sub (a: i32) (b: i32) (c: i32) -> i32:\n    a\n";
    for (src, needle) in [
        (format!("{sub}fn m$:\n    let a = 1 2 3 4 |> sub\n"), "takes 3 parameter(s) and 4"),
        (format!("{sub}fn m$:\n    let a = sub $\n"), "written tight"),
        ("fn m ():\n    1\n".to_string(), "unit value"),
        ("fn m$:\n    let a = x |> f y\n".to_string(), "isolate it"),
        ("fn m$:\n    let a = f x <| y\n".to_string(), "isolate it"),
        ("fn m$:\n    let a = 1 + 2 |> f\n".to_string(), "atoms"),
        ("fn m$:\n    let a = f <|\n".to_string(), "needs an argument"),
    ] {
        let toks = harsh_lang::lex::lex(&src).expect("lex");
        let err = harsh_lang::layout::build(toks).err().expect("must be rejected");
        assert!(err.msg.contains(needle), "{}", err.msg);
    }
    // `$` as a macro metavariable is substituted, not emitted: a Harsh macro
    // unfolds here, so nothing of the definition reaches the output.
    let got = transpile("macro_rules~ pair\n    (($a:expr)) => do: $a\n\nfn m$:\n    let x = pair~ 7\n");
    assert!(!got.contains("$a"), "{got}");
    assert!(got.contains("let x = 7;"), "{got}");
}

/// Application binds tighter than `<-`; the arrow applies to the argument only
/// when the argument is isolated.
#[test]
fn application_binds_tighter_than_arrow() {
    check(&[
        ("fn m$:\n    let a = show \"abc\" <- to_string$\n", "show(\"abc\").to_string()"),
        ("fn m$:\n    let b = show (\"abc\" <- trim$)\n", "show(\"abc\".trim())"),
    ]);
}

/// A function returning a closure is applied with the same rules as anything
/// else; a pipe is not needed for hand-written currying.
#[test]
fn closure_returning_functions() {
    check(&[
        ("fn m$:\n    let inc = add 1\n", "add(1)"),
        ("fn m$:\n    let s = (add 10) 7\n", "(add(10))(7)"),
        // Both groupings are meaningful and apply the arguments in opposite
        // orders. `(3 |> sub) 100` is `sub(3)(100)`; `3 |> (sub 100)` is
        // `sub(100)(3)`.
        ("fn m$:\n    let t = 3 |> (add 100)\n", "(add(100))(3)"),
        ("fn m$:\n    let u = (3 |> add) 100\n", "(add(3))(100)"),
    ]);
}



/// The call forms for a Rust macro (2026-09-23): a list is applied, isolated
/// or not; any other stream is written in braces, as Rust, one line or many;
/// a block passed as an argument is isolated like any argument.
#[test]
fn macro_call_forms() {
    let defs = "macro_rules! m {\n    ( $( $x:expr ),* ) => { 0 };\n}\nmacro_rules! s {\n    ( $( $x:stmt );* ) => { 0 };\n}\n";
    let got = transpile(&format!(
        "{defs}\nfn main$:\n    let a = m! 1 2 3\n    let b = m! (1) (2)\n    let c = m! {{ 1, 2 }}\n    let d = s! {{\n        let q = 1;\n        q\n    }}\n    let e = s! {{ let q = 1; q }}\n    let f = m! (do:\n        let r = 2\n        r\n    ) 3\n"
    ));
    let flat: String = got.split_whitespace().collect();
    for want in ["m!(1, 2, 3)", "m!(1, 2)", "m! { 1, 2 }", "s! {\n        let q = 1;\n        q\n    }", "s! { let q = 1; q }"] {
        assert!(got.contains(want), "missing `{want}` in:\n{got}");
    }
    assert!(flat.contains("letf=m!({letr=2;r},3);"), "{got}");
}

/// A Rust `macro_rules!` is a zone of Rust inside a Harsh file: the
/// transpiler reads none of it and rewrites none of it, and the converter
/// copies it back the same way. Its *calls* stay Harsh — `my_vec! 1 2 3`
/// becomes `my_vec!(1, 2, 3)` — because only the definition is foreign.
#[test]
fn a_rust_macro_rules_is_copied_verbatim() {
    let body = "macro_rules! my_vec {\n    ( $( $x:expr ),* ) => {\n        {\n            let mut v = Vec::new();\n            $( v.push($x); )*\n            v\n        }\n    };\n}\n";
    let harsh = format!("{body}\nfn main$:\n    let v = my_vec! 1 2 3\n    println! \"{{v:?}}\"\n");
    let rust = transpile(&harsh);
    assert!(rust.contains(body.trim_end()), "the zone is not byte-identical:\n{rust}");
    assert!(rust.contains("my_vec!(1, 2, 3)"), "the call is not Harsh's:\n{rust}");
    // A `}` inside a string is text, not a brace: the zone ends at the real one.
    let tricky = "macro_rules! m {\n    () => {\n        println!(\"}\");\n    };\n}\n";
    let out = transpile(&format!("{tricky}\nfn main$:\n    m$\n"));
    assert!(out.contains(tricky.trim_end()), "{out}");
    assert!(out.contains("fn main()"), "the file after the zone was lost:\n{out}");
    // And back: Rust -> Harsh -> Rust through a zone is byte-exact.
    let back = convert(&rust);
    assert!(back.contains("macro_rules! my_vec {"), "{back}");
    assert_eq!(transpile(&back), rust);
}

/// A Harsh macro unfolds **into Harsh**, before anything is transpiled: the
/// definition leaves no trace, the call becomes what the transcriber says,
/// and the emitted Rust holds no macro of ours at all.
#[test]
fn a_harsh_macro_expands_into_harsh() {
    let got = transpile(
        "macro_rules~ twice\n    (($e:expr)) => do:\n        $e * 2\n\nfn main$:\n    let n = twice~ 21\n",
    );
    assert!(!got.contains("macro_rules"), "the definition leaked:\n{got}");
    assert!(got.contains("let n = 21 * 2;"), "{got}");

    // A repetition becomes one statement per round, and `<-` and `$` are
    // translated as in any Harsh: the expansion is ordinary code.
    let got = transpile(
        "macro_rules~ push_all\n    (($v:ident) $( ($x:expr) )*) => do:\n        $( $v <- push $x )*\n\nfn main$:\n    let mut v = Vec.new$\n    push_all~ v 1 2\n",
    );
    // the last push is the function's tail expression, so it carries no `;`
    assert!(got.contains("v.push(1);"), "{got}");
    assert!(got.contains("v.push(2)"), "{got}");

    // Hygiene: the macro's own local moves when the caller has that name.
    let got = transpile(
        "macro_rules~ dbl\n    (($e:expr)) => do:\n        do:\n            let tmp = $e\n            tmp + tmp\n\nfn main$:\n    let tmp = 5\n    let n = dbl~ tmp + 1\n",
    );
    assert!(got.contains("let tmp = 5;"), "the caller's name moved:\n{got}");
    assert!(got.contains("let tmp__1 = tmp + 1;"), "{got}");

    // A call may hand its arguments as a block — `m~ do:` with the body
    // beneath, or `m~\` with its entries — as a `!` call does. The opener is
    // the call's; what the matcher sees is the block's own tokens.
    let got = transpile(
        "macro_rules~ first_of\n    ( $(($t:tt))* ) => do:\n        match ($($t)*)\\ (a, _) => a\n\nfn main$:\n    let f = first_of~ do:\n        (4, 0)\n",
    );
    assert!(got.contains("match((4, 0))"), "{got}");
    // A bare call is a stream to the end of its line; juxtaposed atoms are
    // the arguments, as for any application.
    let got = transpile(
        "macro_rules~ sum\n    ($( ($x:expr) )*) => do:\n        0 $( + $x )*\n\nfn main$:\n    let n = sum~ 1 2\n",
    );
    assert!(got.contains("let n = 0 + 1 + 2;"), "{got}");

    // Hygiene covers every binding a transcriber can write, not `let` alone:
    // a closure parameter, a `for` binding, a `match` arm binding. Each of
    // these would otherwise capture the caller's expression, silently.
    for (src, want) in [
        ("macro_rules~ m\n    (($e:expr)) => do:\n        (|x| $e) 5\n\nfn f$:\n    let x = 10\n    let n = m~ x + 1\n", "(| x__1 | x + 1)(5)"),
        ("macro_rules~ m\n    (($e:expr)) => do:\n        for i in 0..3:\n            acc += $e\n\nfn f$:\n    let i = 100\n    m~ i\n", "for i__1 in 0 .. 3"),
        ("macro_rules~ m\n    (($e:expr)) => do:\n        match Some 1\\\n            Some v => $e\n            None => 0\n\nfn f$:\n    let v = 50\n    let n = m~ v * 2\n", "Some(v__1) => v * 2"),
    ] {
        let got = transpile(src);
        assert!(got.contains(want), "expected {want:?} in:\n{got}");
    }

    // The call is a stream: every token to the end of the line, or to the
    // close of the group the call sits in, reaches the matcher as written --
    // commas are the DSL's, not Harsh's. `$(,)?` takes an optional trailing
    // comma; a `(…)` after the mark is a group, one token, so a tuple is
    // written once and a matcher that wants it inside writes the parens.
    // Arms are tried in order, so the arm that takes the tuple apart comes
    // first: to the second, a paren group is simply one `expr`.
    let src = "macro_rules~ list\n    (($( ($x:expr) ),* $(,)?)) => do:\n        [$( $x ),*]\n    ($( ($x:expr) ),* $(,)?) => do:\n        [$( $x ),*]\n\nfn main$:\n    let a = list~ 1, 2\n    let b = list~ 1, 2,\n    let c = list~ (1, 2,)\n";
    let got = transpile(src);
    assert_eq!(got.matches("[1, 2]").count(), 3, "{got}");
    // A call inside a tuple is isolated, as any application is: the stream
    // would otherwise run to the tuple's close.
    let got = transpile("macro_rules~ twice\n    (($e:expr)) => do:\n        $e * 2\n\nfn main$:\n    let t = ((twice~ 4), 0)\n");
    assert!(got.contains("((4 * 2), 0)"), "{got}");

    // A Rust `macro_rules!` in the same file is untouched by any of this.
    let got = transpile(
        "macro_rules! keep {\n    () => { 1 };\n}\n\nfn main$:\n    let n = keep$\n",
    );
    assert!(got.contains("macro_rules! keep {"), "{got}");
}

/// Harsh's own declarative macros are marked `~`, at the definition and at
/// the call. Since expansion landed, the mark is the fork: a `~` macro is
/// unfolded here, into Harsh, and leaves no trace; a `!` one is Rust's.
#[test]
fn tilde_is_the_harsh_macro_mark() {
    let got = transpile("macro_rules~ pair\n    (($a:expr)) => do: $a\n\nfn main$:\n    println! \"{}\" (pair~ 7)\n");
    assert!(!got.contains("macro_rules"), "a Harsh macro leaves no trace:\n{got}");
    assert!(got.contains("println!(\"{}\", 7)"), "{got}");
    // A spaced `~` is not a mark: nothing in Harsh writes one, so it is left
    // alone rather than quietly becoming a macro call.
    let plain = transpile("fn main$:\n    let a = 1\n    let b = a ~ 2\n");
    assert!(plain.contains("a ~ 2"), "{plain}");
}

/// `macro_rules! name:` is a definition; the name is not an argument.
#[test]
fn macro_rules_definition() {
    // Since expansion landed, a `~` definition emits nothing at all and its
    // calls become what the transcriber says.
    let got = transpile("macro_rules~ pair\n    (($a:expr)) => do: $a\n\nfn main$:\n    let x = pair~ 7\n");
    assert!(!got.contains("macro_rules"), "{got}");
    assert!(got.contains("let x = 7;"), "{got}");
    // An arm's transcriber must be a block: `=> do:`, not a bare expression.
    assert!(
        layout_err("macro_rules~ pair\n    (($a:expr)) => $a\n\nfn main$:\n    let x = pair~ 7\n")
            .contains("transcriber is a block")
    );
}

/// Mixing the inline and multi-line `else` forms orphans the second `else`,
/// because the inline block closes at the end of its line.
#[test]
fn rejects_orphaned_else() {
    let src = "fn b (n: i32) -> i32:\n    if n == 1: 10\n    else: if n == 2: 20\n    else: 30\n";
    let toks = harsh_lang::lex::lex(src).expect("lex");
    assert!(harsh_lang::layout::build(toks).is_err());

    // The same chain on one line, and the fully indented form, both work.
    for ok in [
        "fn b (n: i32) -> i32:\n    if n == 1: 10 else: if n == 2: 20 else: 30\n",
        "fn b (n: i32) -> i32:\n    if n == 1:\n        10\n    else if n == 2:\n        20\n    else:\n        30\n",
    ] {
        let toks = harsh_lang::lex::lex(ok).expect("lex");
        assert!(harsh_lang::layout::build(toks).is_ok(), "{ok:?}");
    }
}

/// A statement containing an indented closure is a statement, not a block: it
/// ends in `;` when it is not the last in its block. The closure body is
/// governed at its own level by the same rule. Both worked examples were
/// agreed with the user as the specification.
#[test]
fn closure_statement_semicolons() {
    let src = "fn main$:\n    a$\n    f || do:\n        b$\n        c$\n    d$\n    e$\n";
    let got = transpile(src);
    assert!(got.contains("    a();\n    f(|| {\n        b();\n        c()\n    });\n    d();\n    e()\n"), "{got}");

    // An explicit `;` on a last statement is kept, at both levels.
    let src = "fn main$:\n    a$\n    f || do:\n        b$\n        c$;\n    d$\n    e$;\n";
    let got = transpile(src);
    assert!(got.contains("        c();\n    });\n    d();\n    e();\n"), "{got}");

    // `||:` and `|| do:` are the same opener.
    assert_eq!(
        transpile("fn main$:\n    f ||:\n        b$\n    d$\n"),
        transpile("fn main$:\n    f || do:\n        b$\n    d$\n"),
    );

    // A closure statement that IS last takes no semicolon, like any statement.
    let got = transpile("fn main$:\n    f || do:\n        b$\n");
    assert!(got.contains("    })\n}"), "{got}");
}

/// Parens are transparent to layout: a `:` ending a line inside an open paren
/// opens a block, and the matching `)` closes it wherever it sits. The rest of
/// the statement rejoins after the block.
#[test]
fn paren_blocks() {
    // `)` on the body's last line; the chain resumes on continuation lines.
    let got = transpile(
        "fn f (v: Vec<i32>) -> usize:\n    v <- iter$ <- map (|x|:\n        x * 2)\n      <- filter (|x| x > &2)\n      <- count$\n",
    );
    assert!(got.contains("map(|x| {\n        x * 2\n    })\n"), "{got}");
    assert!(got.contains(".filter(|x| x > &2)\n"), "{got}");
    assert!(!got.contains("});\n"), "no semicolon may split the chain:\n{got}");

    // `)` on its own line, and the statement bound with `let`.
    let got = transpile(
        "fn main$:\n    let d: Vec<i32> = v <- iter$ <- map (|x|:\n            x * 10\n        ) <- collect$\n    g$\n",
    );
    assert!(got.contains("    }).collect();\n    g()"), "{got}");

    // A multi-statement body inside the paren, governed at its own level.
    let got = transpile(
        "fn main$:\n    f (|x|:\n        let y = x\n        y * 2)\n    g$\n",
    );
    assert!(got.contains("        let y = x;\n        y * 2\n    });\n    g()"), "{got}");

    // A struct literal across lines is a block, `P:` with its fields
    // beneath; the statement's `;` follows its `}`.
    let got = transpile("fn main$:\n    let p =\n        P\\\n            x = 1\n            y = 2\n    g$\n");
    assert!(got.contains("P {\n") && got.contains("y: 2,\n") && got.contains("};\n    g()"), "{got}");
    // Braces over several lines are an error naming `:` / `do:`.
    assert!(layout_err("fn main$:\n    unsafe {\n        x$\n    }\n").contains("written with `:` or `do:`"));
    assert!(layout_err("fn main$:\n    let v = m! {\n        a\n    }\n").contains("`name! do:`"));
    // ... except a brace group handed as an argument.
    let got = transpile("fn main$:\n    let b = json! ({\n        \"a\": 1\n    })\n    b\n");
    assert!(got.contains("json!({"), "{got}");
}

/// A chain after a bare closure block is rejected with the fix named, rather
/// than emitting Rust that fails to parse.
#[test]
fn rejects_chain_after_bare_closure() {
    let bare = "fn f (v: Vec<i32>) -> usize:\n    v <- iter$ <- map |x|:\n        x * 2\n    <- count$\n";
    let toks = harsh_lang::lex::lex(bare).expect("lex");
    let err = harsh_lang::layout::build(toks).err().expect("must be rejected");
    assert!(err.msg.contains("isolate the closure"), "{}", err.msg);

    // The isolated form is the fix, and a bare closure with nothing after it
    // is still fine.
    for ok in [
        "fn f (v: Vec<i32>) -> usize:\n    v <- iter$ <- map (|x|:\n        x * 2)\n      <- count$\n",
        "fn main$:\n    v <- sort_by |a, b|:\n        a <- cmp b\n    g$\n",
    ] {
        let toks = harsh_lang::lex::lex(ok).expect("lex");
        assert!(harsh_lang::layout::build(toks).is_ok(), "{ok:?}");
    }
}

/// Generated Rust follows Rust's own spacing around call parens, whatever the
/// `.hrs` writes: tight after a callee, a closer, a turbofish or a macro
/// bang; spaced after a keyword. The `.hrs` keeps its idiom.
#[test]
fn call_parens_follow_rust_spacing() {
    check(&[
        ("fn m$:\n    let a = v <- iter$ <- map (|x| x)\n", "v.iter().map(|x| x)"),
        ("fn m$:\n    let a = Some 1\n", "Some(1)"),
        ("fn m$:\n    let a = format! \"{}\" 1\n", "format!(\"{}\", 1)"),
        ("fn m$:\n    let a = f (1) (2)\n", "f(1, 2)"),
        ("fn m$:\n    let a = v <- get (0) <- unwrap$\n", "v.get(0).unwrap()"),
        ("fn m$:\n    if (a):\n        b\n", "if (a) {"),
        ("fn m$:\n    return (a)\n", "return (a)"),
        ("fn g name: &str -> String:\n    x\n", "fn g(name: &str) -> String"),
        ("fn g (u: ()) (b: i32) -> i32:\n    b\n", "fn g(u: (), b: i32) -> i32"),
    ]);
}

fn layout_err(src: &str) -> String {
    let toks = harsh_lang::lex::lex(src).expect("lex");
    // Harsh's own macros are read and expanded before the file is a program,
    // as the driver does it; a macro's own errors arrive from there.
    let taken = harsh_lang::layout::names_in_scope(&toks);
    let toks = match harsh_lang::mac::expand_all(toks, &taken) {
        Ok(t) => t,
        Err(e) => return e.msg,
    };
    match harsh_lang::layout::build(toks) {
        Err(e) => e.msg,
        Ok(_) => panic!("accepted:\n{src}"),
    }
}

/// Indentation discipline: no unit is imposed, but every line must be
/// perfectly aligned -- a dedent lands on an open block's column, a line
/// deeper than its statement is a continuation and so cannot begin with a
/// statement keyword, and an inline block holds one expression.
#[test]
fn rejects_misaligned_lines() {
    // A dedent to a column no block uses: previously this silently moved
    // `let y` out of `main`.
    let m = layout_err("fn main$:\n    let x = 1\n   let y = 2\n");
    assert!(m.contains("column 3 matches no open block") && m.contains("block above is at column 4"), "{m}");
    let m = layout_err("fn main$:\n    if x:\n        a$\n      b$\n");
    assert!(m.contains("column 6 matches no open block"), "{m}");
    // A statement keyword on a continuation line: previously `let x = 1
    // let y = 2` became one statement.
    let m = layout_err("fn main$:\n    let x = 1\n     let y = 2\n");
    assert!(m.contains("`let` starts a statement") && m.contains("`let x` at column 4"), "{m}");
    let m = layout_err("fn main$:\n    let x = 1\n      fn g$:\n          1\n");
    assert!(m.contains("`fn` starts a statement"), "{m}");
    // A `;` inside an inline block, in an arm and in an `if`.
    let m = layout_err("fn f x: i32 -> i32:\n    match x\\ 1 => do: g$; h$, _ => 0\n");
    assert!(m.contains("inline block holds one expression"), "{m}");
    let m = layout_err("fn f c: bool -> i32:\n    if c: g$; h$ else: 0\n");
    assert!(m.contains("inline block holds one expression"), "{m}");
    // `:` before `{` opens a block inside a block.
    let m = layout_err("fn f$ -> i32:\n    do: { let a = 1; a * 2 }\n");
    assert!(m.contains("braces already open a block"), "{m}");
    let m = layout_err("fn f c: bool -> i32:\n    if c do: { f$ } else do: { g$ }\n");
    assert!(m.contains("braces already open a block"), "{m}");
}

/// The forms the discipline must keep accepting.
#[test]
fn accepts_aligned_continuations() {
    check(&[
        // `else` under an `if` that is itself a continuation line.
        ("fn m$:\n    let d =\n        if c:\n            1\n        else:\n            2\n", "    else {\n        2\n    };"),
        // `if` and `<-` continue an expression; `let` chains may break before `let`.
        ("fn m$:\n    let n = v <- iter$\n              <- count$\n", ".count();"),
        ("fn m$:\n    if a\n        && let Some x = y:\n        1\n", "&& let Some(x) = y {"),
        // A `;` ending the line discards the tail; braces hold several statements.
        ("fn m$:\n    if c: g$;\n    h$\n", "h()"),
        ("fn f x: i32 -> i32:\n    match x\\ 1 => { g$; h$ }, _ => 0\n", "1 => { g(); h() },"),
        ("fn f c: bool -> i32:\n    if c { f$; g$ } else { h$ }\n", "if c { f(); g() } else { h() }"),
        ("fn f$ -> i32:\n    { let a = 1; a * 2 }\n", "{ let a = 1; a * 2 }"),
        // Any deeper column continues: no unit is imposed.
        ("fn m$:\n    let n = elem <- a$\n                 <- b$\n", ".b();"),
    ]);
}

/// A chain continues after an isolated closure whose body has a nested block:
/// the `)` tail rejoins the closure's header, not the last block opened inside.
#[test]
fn paren_block_tail_after_nested_block() {
    check(&[
        ("fn m$:\n    let c: Vec<i32> =\n        v <- iter$\n          <- map (\n                 |p|:\n                     if p > 1:\n                         p\n                     else:\n                         0\n             )\n          <- collect$\n    c\n",
         ".collect()"),
    ]);
}

/// A `move` closure is a trailing block-bodied argument like any closure.
#[test]
fn move_closure_as_trailing_argument() {
    check(&[
        ("fn m$:\n    let h = spawn move ||:\n        v\n    h\n", "spawn(move || {\n        v\n    })"),
        ("fn m$:\n    let h = spawn move |x|:\n        x\n    h\n", "spawn(move |x| {\n        x\n    })"),
        ("fn m$:\n    let h = spawn (move || v)\n", "spawn(move || v)"),
        // `async:` / `async move:` head a block argument the same way.
        ("fn m$:\n    let h = spawn async:\n        v\n    h\n", "spawn(async {\n        v\n    })"),
        ("fn m$:\n    let h = spawn async move:\n        v\n    h\n", "spawn(async move {\n        v\n    })"),
        ("fn m$:\n    let f = async:\n        v\n    f\n", "let f = async {\n        v\n    };"),
    ]);
}

/// `$` applies a name to nothing; `()` is only ever the unit value.
#[test]
fn dollar_applies_to_nothing() {
    check(&[
        ("fn main$:\n    let a = 1\n", "fn main() {"),
        ("fn one$ -> i32:\n    1\n", "fn one() -> i32 {"),
        ("fn m$:\n    let v = Vec<i32>.new$\n", "Vec::<i32>::new()"),
        ("fn m$:\n    let n = s <- trim$ <- len$\n", "s.trim().len()"),
        ("fn m$:\n    let n = m!$\n", "m!()"),
        ("fn m$:\n    let n = (add 10)$\n", "(add(10))()"),
        // One atom: an argument, a head, a pipe side.
        ("fn m$:\n    let n = g f$ 5\n", "g(f(), 5)"),
        ("fn m$:\n    let n = f$ 5\n", "f()(5)"),
        // The unit value is an ordinary argument, wrapped like any group.
        ("fn m$:\n    let n = f ()\n", "f(())"),
        ("fn m$:\n    let n = f () 5\n", "f((), 5)"),
        ("fn m$:\n    let r: Result<(), E> = Ok ()\n", "Ok(())"),
        // Type and value positions of `()` are untouched.
        ("fn m$ -> ():\n    ()\n", "fn m() -> () {\n    ()\n}"),
    ]);
    // The converter writes `$` for an empty call and `()` for a unit argument.
    let back = harsh_lang::unbrace::convert("fn f() -> i32 { g(()) + h::<i32>() }\n").expect("convert");
    assert!(back.contains("fn f$ -> i32:"), "{back}");
    assert!(back.contains("g () + h<i32>$") || back.contains("g () + h.<i32>$") || back.contains("h<i32>$"), "{back}");
}

/// A lone parameter may be bare; with more, every one is a group.
#[test]
fn rejects_bare_parameter_followed_by_group() {
    for (src, ok) in [
        ("impl C\n    fn top &self -> i32:\n        1\n", true),
        ("impl C\n    fn top (&self) -> i32:\n        1\n", true),
        ("impl C\n    fn add (&self) (k: i32) -> i32:\n        k\n", true),
        ("fn f g: fn i32:\n    g 1\n", true),
        ("impl C\n    fn add &self (k: i32) -> i32:\n        k\n", false),
        ("impl C\n    fn add &mut self (k: i32):\n        k\n", false),
        ("fn add a: i32 (b: i32) -> i32:\n    a\n", false),
    ] {
        let toks = harsh_lang::lex::lex(src).expect("lex");
        let r = harsh_lang::layout::build(toks);
        if ok {
            assert!(r.is_ok(), "{src}: {:?}", r.err());
        } else {
            let err = r.err().expect("must be rejected");
            assert!(err.msg.contains("every one is a group"), "{}", err.msg);
        }
    }
    let err = harsh_lang::layout::build(harsh_lang::lex::lex("impl C\n    fn add &self (k: i32):\n        k\n").unwrap()).err().unwrap();
    assert!(err.msg.contains("`(&self) (`"), "{}", err.msg);
}

/// A Rust macro's brace body is Rust, even in Harsh, and copied verbatim; only
/// its holes, `@: … :@`, are Harsh (the user's principle 2 and ruling 16,
/// "always closing", 2026-09-23). It replaced the rule that a brace body was
/// A character of several bytes inside a hole -- or on either side of its
/// marks -- is carried through, not a panic (found 2026-09-26 writing the
/// website: the hole scan stepped one byte at a time and sliced inside `…`).
/// The website's Converter and Playground run this code in the browser,
/// where a panic stops the page.
#[test]
fn a_hole_carries_characters_of_several_bytes() {
    check(&[
        ("fn m$:\n    view! { <p>{@: \"…\" :@}</p> }\n", "view! { <p>{\"…\"}</p> }"),
        ("fn m$:\n    view! { <p>{@: \"café\" <- len$ :@}</p> }\n", "view! { <p>{\"café\".len()}</p> }"),
        ("fn m$:\n    view! { <p>\"é\"{@: n :@}\"😀\"</p> }\n", "view! { <p>\"é\"{n}\"😀\"</p> }"),
        ("fn m$:\n    view! { <p>{@: f \"日本\" :@}</p> }\n", "view! { <p>{f(\"日本\")}</p> }"),
    ]);
}

/// Harsh on one line.
#[test]
fn a_macro_brace_body_is_rust_and_its_holes_are_harsh() {
    check(&[
        ("fn m$:\n    let z = m! { f a b }\n", "m! { f a b }"),
        ("fn m$:\n    let z = m! { @: f a b :@ }\n", "m! { f(a, b) }"),
        ("fn m$:\n    let y = quote! { fn #name() -> u32 { #body } }\n", "quote! { fn #name() -> u32 { #body } }"),
        ("fn m$:\n    let s = tokio.select! { n = @: slow \"slow\" :@ => n, }\n", "tokio::select! { n = slow(\"slow\") => n, }"),
        ("fn m$:\n    let b = m! { matches!(a, b) }\n", "m! { matches!(a, b) }"),
        ("fn m$:\n    let k = key! { server.port }\n", "key! { server.port }"),
        ("fn m$:\n    let k = key! { @: server.port :@ }\n", "key! { server::port }"),
        ("fn m$:\n    let t = m! { \"contact @@: us\" }\n", "m! { \"contact @: us\" }"),
    ]);
    // `!(a && b)` is negation, Rust's, kept.
    check(&[("fn m$:\n    let b = !(a && c)\n", "let b = !(a && c);")]);
    // The user's sketch: markup verbatim, two holes transpiled, one a block.
    let out = transpile("fn g (name: String) (age: u32):\n    view! {\n        <p>{@: if age >= 18: \"adult\" else: \"minor\" :@}</p>\n        <button on:click= @: move |_| println! \"{name}\" :@>\"Click\"</button>\n        <ul>\n            {@:\n            (1..=3) <- map (|n| view! { <li>{@: n * 2 :@}</li> })\n            :@}\n        </ul>\n    }\n");
    for want in [
        "<p>{if age >= 18 { \"adult\" } else { \"minor\" }}</p>",
        "<button on:click= move |_| println!(\"{name}\")>\"Click\"</button>",
        "(1..=3).map(|n|view!{<li>{n*2}</li>})",
    ] {
        let flat: String = out.split_whitespace().collect();
        let w: String = want.split_whitespace().collect();
        assert!(flat.contains(&w), "missing `{want}` in:\n{out}");
    }
    // A hole always closes.
    let err = harsh_lang::driver::transpile_str("fn m$:\n    let z = m! { @: f a b }\n").unwrap_err();
    assert!(err.contains("never closed"), "{err}");
}

/// A `~` macro's transcriber is Harsh and so is its expansion; a Rust macro
/// taking a list is applied. `m! do:` -- a Rust macro's body as a Harsh block
/// -- is retired (2026-09-23): the macro's stream is written in braces.
#[test]
fn harsh_macros_expand_and_rust_macros_apply() {
    check(&[
        (
            "macro_rules~ my_vec\n    ( $( ($x:expr) )* ) => do:\n        do:\n            let mut tmp = Vec.new$\n            $(tmp <- push $x)*\n            tmp\n\nfn m$:\n    let v = my_vec~ 1 2\n",
            "let v = {\n        let mut tmp = Vec::new();\n        tmp.push(1);\n        tmp.push(2);\n        tmp\n    };",
        ),
        ("fn m$:\n    let v = my_vec! 1 2 3\n    let m = hashmap! (\"a\" => 1) (\"b\" => 2)\n", "my_vec!(1, 2, 3);\n    let m = hashmap!(\"a\" => 1, \"b\" => 2)"),
        ("fn m$:\n    let w = tokio.select! {\n        n = @: slow \"slow\" :@ => n,\n        n = @: fast \"fast\" :@ => n,\n    }\n    w\n", "let w = tokio::select! {\n        n = slow(\"slow\") => n,\n        n = fast(\"fast\") => n,\n    };"),
    ]);
    let e = layout_err("fn m$:\n    let w = tokio.select! do:\n        n = slow 1 => n\n");
    assert!(e.contains("is retired") && e.contains("(do: …)") && e.contains("{ … }"), "{e}");
}

/// Rules 4 and 5, and the same brick in a transcriber: a parenthesised
/// fragment is one parameter (or argument), a sequence is a comma list, a
/// repetition of them is a comma-separated repetition. Bare tokens, `tt`
/// forwarding included, are Rust's. Bracketed and braced matchers are Rust's.
#[test]
fn macro_matchers_are_parameter_groups() {
    // One group per argument, and a call juxtaposes them: `m~ 1 2`.
    let got = transpile("macro_rules~ m\n    (($a:expr) ($b:expr)) => do: $a + $b\n\nfn main$:\n    let n = m~ 1 2\n");
    assert!(got.contains("let n = 1 + 2;"), "{got}");
    // A repetition of groups is a list of arguments.
    let got = transpile("macro_rules~ m\n    ($( ($x:expr) )*) => do:\n        [$( $x ),*]\n\nfn main$:\n    let v = m~ 1 2 3\n");
    assert!(got.contains("[1, 2, 3]"), "{got}");
    // A literal token in the matcher must appear in the call.
    let got = transpile("macro_rules~ m\n    (add ($a:expr)) => do: $a\n\nfn main$:\n    let n = m~ add 5\n");
    assert!(got.contains("let n = 5;"), "{got}");
    // An empty matcher takes no argument.
    let got = transpile("macro_rules~ m\n    () => do: 0\n\nfn main$:\n    let n = m~\n");
    assert!(got.contains("let n = 0;"), "{got}");
}


/// The macro rules' errors.
#[test]
fn rejects_bad_macro_spellings() {
    for (src, needle) in [
        // the header takes no mark, and an arm needs its matcher first
        ("macro_rules~ kept:\n    (($e:expr)) => do: $e\n", "arms follow its header"),
        ("macro_rules~ m\n    $a\n", "an arm begins with its matcher"),
        // the transcriber is a block
        ("macro_rules~ twice\n    (($e:expr)) => $e * 2\n", "transcriber is a block"),
        // a capture the matcher never bound, and one used at the wrong depth
        ("macro_rules~ m\n    (($a:expr)) => do: $b\n\nfn f$:\n    let x = m~ 1\n", "not captured"),
        ("macro_rules~ m\n    ($( ($x:expr) )*) => do: $x\n\nfn f$:\n    let x = m~ 1 2\n", "used inside one"),
        // and a call no arm fits
        ("macro_rules~ one\n    (($a:ident)) => do: $a\n\nfn f$:\n    let x = one~ 1\n", "no arm of `one~` matches"),
        // the wrong mark on a call, named rather than left to rustc
        ("macro_rules~ adding\n    (($a:expr)) => do: $a\n\nfn f$:\n    let x = adding! 3\n", "is called `adding~`"),
    ] {
        assert!(layout_err(src).contains(needle), "{src:?}: {}", layout_err(src));
    }
}

/// Two converter shapes the self-host caught on 2026-09-18, fixed the 19th: a
/// brace group after a name that is a *pattern* (`if let E::G { .. } = &m`)
/// or the sole element of a bracket group (`vec![Arm { m: 1 }]`).
#[test]
fn converter_brace_after_a_name() {
    let got = convert("enum E { G { body: Vec<i32> } }\nfn main() {\n    let m = E::G { body: vec![1] };\n    if let E::G { body, .. } = &m {\n        let _ = body.clone();\n    }\n}\n");
    assert!(got.contains("if let E.G\\ body, .. = &m:"), "{got}");
    assert!(!got.contains("(do:"), "{got}");
    let got = convert("struct Arm { m: i32 }\nfn main() {\n    let arms = vec![Arm { m: 1 }];\n    let _ = arms;\n}\n");
    assert!(got.contains("vec! (Arm\\ m = 1)"), "{got}");
    // two literals in a macro's list: each is an argument, isolated once
    // (A1, 2026-09-22; they were a bracket group before)
    let got = convert("struct A { m: i32 }\nfn main() {\n    let v = vec![A { m: 1 }, A { m: 2 }];\n    let _ = v;\n}\n");
    assert!(got.contains("vec! (A\\ m = 1) (A\\ m = 2)"), "{got}");
}

/// The converter side of the macro rules: `macro_rules!` bodies become the
/// `:` form with rule-4 matchers; `view! { .. }` keeps its markup, and its
/// Rust code becomes holes; both round-trip back to the Rust they came from.
#[test]
fn converter_macros() {
    let rust = "macro_rules! hashmap {\n    ( $( $k:expr => $v:expr ),* ) => {\n        {\n            let mut m = HashMap::new();\n            $( m.insert($k, $v); )*\n            m\n        }\n    };\n}\n\nmacro_rules! add {\n    ($a:expr, $b:expr) => { $a + $b };\n    ($h:expr, $($t:tt)*) => { $h + add!($($t)*) };\n}\n\nmacro_rules! filled {\n    [ $elem:expr ; $n:expr ] => { vec![$elem; $n] };\n}\n\nfn main() {\n    let m = hashmap!(\"a\" => 1, \"b\" => 2);\n    let s = add!(1, 2, 3);\n    let z = filled![0u8; 4];\n}\n";
    let harsh = convert(rust);
    // Since 2026-09-17 a Rust `macro_rules!` is a zone of Rust: the converter
    // copies the definition through byte for byte (Harsh's own macros are
    // `macro_rules~`, which this converter never writes). The *calls* are
    // converted as any other macro call is.
    for want in [
        "macro_rules! hashmap {\n    ( $( $k:expr => $v:expr ),* ) => {\n",
        "macro_rules! add {\n    ($a:expr, $b:expr) => { $a + $b };\n",
        "macro_rules! filled {\n    [ $elem:expr ; $n:expr ] => { vec![$elem; $n] };\n}",
        "hashmap! (\"a\" => 1) (\"b\" => 2)",
        "add! 1 2 3",
    ] {
        assert!(harsh.contains(want), "\n  expected: {want:?}\n  got:\n{harsh}");
    }
    let back = transpile(&harsh);
    assert!(same_tokens(rust, &back), "{}", first_diff(rust, &back));

    let rust = "fn main() {\n    view! {\n        <div class=\"app\">\n            <p>{count}</p>\n            <button on:click=move |_| set_count.update(|n| *n += 1)>\"+\"</button>\n            <Greeting name={\"World\".to_string()} />\n            <input type=\"text\" prop:value=name.get() on:input=move |ev| set_name(event_target_value(&ev)) />\n            <leptos_router::A href=\"/\">\"home\"</leptos_router::A>\n        </div>\n    }\n}\n";
    let harsh = convert(rust);
    // The markup is `view!`'s and stays as written; its Rust is in holes.
    for want in [
        "<p>{@: count :@}</p>",
        "<button on:click=@: move |_| set_count <- update (|n| * n += 1) :@>\"+\"</button>",
        "<Greeting name={@: \"World\" <- to_string$ :@} />",
        "<input type=\"text\" prop:value=@: name <- get$ :@ on:input=@: move |ev| set_name (event_target_value (&ev)) :@ />",
    ] {
        assert!(harsh.contains(want), "\n  expected: {want:?}\n  got:\n{harsh}");
    }
    let back = transpile(&harsh);
    assert!(same_tokens(rust, &back), "{}", first_diff(rust, &back));
}

/// A bare parameter followed by a `[where ..]` clause: the bracket ends the
/// parameter, it is not a group of it. (The arm that said so was
/// unreachable behind `Tk::Open(_)` until a build warning pointed at it.)
#[test]
fn bare_parameter_before_where_clause() {
    check(&[
        ("fn show<T> x: T [where T: std.fmt.Debug]:\n    println! \"{x:?}\"\n", "fn show<T>(x: T) where T: std::fmt::Debug {"),
        // Optional grouping peels from the outside in.
        ("fn m$:\n    let s = ((1 + 2))\n", "let s = 1 + 2;"),
    ]);
}


/// `rsx!`'s tree through the converter and back: the tree as written, its
/// Rust in holes (2026-09-23; rule 6's Harsh tree is retired). And a lexer gap
/// the old test exposed: raw identifiers.
#[test]
fn converter_brace_tree() {
    let rust = "fn main() {\n    rsx! {\n        div {\n            class: \"app\",\n            onclick: move |_| count.set(count() + 1),\n            \"Hello {count}\"\n            for item in items.iter() {\n                li { \"{item}\" }\n            }\n            Button {\n                onclick: move |_| reset(),\n                \"Reset\"\n            }\n            {count}\n            input { r#type: \"text\", ..attrs }\n        }\n    }\n}\n";
    let harsh = convert(rust);
    // The tree is `rsx!`'s and stays as written; every piece of Rust in it
    // becomes a hole: an attribute's value, a `for`'s expression, a child.
    for want in [
        "            class: \"app\",\n",
        "            onclick: @: move |_| count <- set (count$ + 1) :@,\n",
        "            for item in @: items <- iter$ :@ {\n                li { \"{item}\" }\n",
        "                onclick: @: move |_| reset$ :@,\n",
        "            {@: count :@}\n",
        "            input { r#type: \"text\", ..attrs }\n",
    ] {
        assert!(harsh.contains(want), "\n  expected: {want:?}\n  got:\n{harsh}");
    }
    let back = transpile(&harsh);
    assert!(same_tokens(rust, &back), "{}", first_diff(rust, &back));
    check(&[("fn main$:\n    let r#type = 1\n    println! \"{}\" r#type\n", "println!(\"{}\", r#type)")]);
}

/// Parens isolate, or raise precedence; they neither open nor close a block.
/// A block may be written inside a group, `:` or `do:` after a closure's
/// prototype included, and ends where the group ends -- the `)` is a boundary
/// the block cannot pass, not a rule about parens. The rest of the statement
/// after the `)` is the block's tail and may open another such block.
#[test]
fn blocks_inside_groups() {
    check(&[
        (
            "fn m$:\n    let a: Vec<i32> = xs <- iter$ <- map (|p|: if *p > 1: *p else: 0) <- collect$\n",
            "let a: Vec<i32> = xs.iter().map(|p| {\n        if *p > 1 {\n            *p\n        }\n        else {\n            0\n        }\n    }).collect();",
        ),
        // Both colons: the first is the parameter's annotation.
        ("fn m$:\n    let b: i32 = xs <- iter$ <- map (|p: &i32|: *p + 1) <- sum$\n", "map(|p: &i32| {\n        *p + 1\n    }).sum();"),
        // Two blocks in one chain: the first block's tail opens the second.
        (
            "fn m$:\n    let d: Vec<i32> = xs <- iter$ <- map (|p|: *p * 2) <- filter (|q|: *q > 2) <- collect$\n",
            "map(|p| {\n        *p * 2\n    }).filter(|q| {\n        *q > 2\n    }).collect();",
        ),
        // A block-bodied group after an earlier argument, inline and as a
        // paren block: the unmatched `(` is the last argument.
        ("fn m$:\n    let e = xs <- iter$ <- fold 0 (|acc, x|: acc + x)\n", "fold(0, |acc, x| {\n        acc + x\n    });"),
        ("fn m$:\n    let e = xs <- iter$ <- fold 0 (|acc, x|:\n        acc + x\n    )\n", "fold(0, |acc, x| {\n        acc + x\n    });"),
        // `|x|:` and `|x| do:` are both a closure prototype followed by a
        // block, and mean the same thing.
        ("fn m$:\n    let f = |x|: x + 1\n", "let f = |x| {\n        x + 1\n    };"),
        ("fn m$:\n    let g = |x| do: x * 2\n", "let g = |x| {\n        x * 2\n    };"),
        // Annotations and tuples are untouched.
        ("fn m$:\n    let t: (i32, i32) = (1, 2)\n", "let t: (i32, i32) = (1, 2);"),
        ("fn f (a: (i32, i32)) (b: u8) -> u8:\n    b\n", "fn f(a: (i32, i32), b: u8) -> u8 {"),
    ]);
    // A `:` inside `[ .. ]` or `{ .. }` is Rust's, not a block opener.
    let got = transpile("fn m$:\n    let p = Point\\ x = 1, y = 2\n");
    assert!(got.contains("Point {\n        x: 1,\n        y: 2,\n    };"), "{got}");
    // Braces build nothing: a brace literal is an error, even on one line.
    assert!(layout_err("fn m$:\n    let p = Point { x: 1, y: 2 }\n").contains("a struct is built with"));
    // A pattern is written `Point\\ x, y` and comes out braced; the brace
    // form of a pattern is still accepted.
    let got = transpile("fn m$:\n    let Point\\ x, y = p\n    let ((a, b), Point { x: px, .. }) = q\n");
    assert!(got.contains("let Point { x, y } = p;"), "{got}");
    assert!(got.contains("Point { x: px, .. }) = q;"), "{got}");
    // A line inside an open group indents past the line that opened it, or
    // is the `)` that closes it.
    let e = layout_err("fn m$:\n    let e = foo (a,\n    b)\n");
    assert!(e.contains("indents past the line that opened it"), "{e}");
    let e = layout_err("fn m$:\n    let e = foo (a,\n  b)\n");
    assert!(e.contains("indents past the line that opened it"), "{e}");
    // Deeper is a continuation; the `)` at the opener's column is the tail.
    let got = transpile("fn m$:\n    let e = foo (a,\n        b)\n    let t = (\n        1,\n        2,\n    )\n");
    assert!(got.contains("foo((a,\n        b));") || got.contains("foo((a,\n            b));"), "{got}");
    assert!(got.contains("let t = (\n"), "{got}");
    // An inline block still holds one expression, inside a group too.
    let e = layout_err("fn m$:\n    let a = xs <- map (|p|: f p; g p)\n");
    assert!(e.contains("one expression"), "{e}");
}

/// What a real Leptos + Axum crate taught, each pinned: the shapes a corpus
/// written for the purpose never had.
#[test]
fn leptos_shapes() {
    check(&[
        // Documented parameters: doc lines above each group, attributes
        // inside it, the `(` opening before the first parameter's doc.
        (
            "#[component]\npub fn Card\n    /// The service.\n    (service: Service)\n    /// Stagger, in seconds.\n    (#[prop (default = 0.0)] delay: f32)\n    -> impl IntoView:\n    delay\n",
            "pub fn Card(/// The service.\n    service: Service,\n    /// Stagger, in seconds.\n    #[prop(default = 0.0)] delay: f32)\n    -> impl IntoView {",
        ),
        // A Rust macro's brace body is Rust: copied verbatim, one line or
        // many, a hole first or a comment first (principle 2, 2026-09-23;
        // these were `view! do:` markup blocks until then).
        ("fn m$:\n    let v = view! { <p class=\"x\">{count}</p> }\n    v\n", "view! { <p class=\"x\">{count}</p> }"),
        ("fn m$:\n    view! {\n        {panel}\n        <button type=\"button\" class=\"x\">\"+\"</button>\n    }\n", "view! {\n        {panel}\n        <button type=\"button\" class=\"x\">\"+\"</button>\n    }"),
        ("fn m$:\n    view! {\n        // the sheet\n        <Stylesheet id=\"leptos\" href=\"/x\"/>\n    }\n", "// the sheet\n        <Stylesheet id=\"leptos\" href=\"/x\"/>"),
        // A body inside a group keeps what follows it.
        ("fn m$:\n    let v = (0..3) <- map (|_| view! {\n            <i class=\"x\"></i>\n    }) <- collect_view$\n    v\n", ").collect_view();"),
        // A paren block inside a paren block, a body inside that.
        (
            "fn m$:\n    let dots = (len > 1) <- then (||:\n            (0..len) <- map (|i|:\n                    view! {\n                        <button\n                            type=\"button\"\n                        ></button>\n                    }\n            ) <- collect_view$\n    )\n    dots\n",
            "(0..len).map(|i| {\n            view! {\n",
        ),
        // A `):` tail opens the header's own block.
        ("fn m$:\n    if xs <- iter$ <- any (|x|:\n            *x > 1\n    ):\n        return 1\n    2\n", "if xs.iter().any(|x| {\n        *x > 1\n    }) {\n        return 1\n    }"),
        // A chain hanging off a block expression, isolated in a group.
        ("fn m$:\n    let b =\n        (\n            if c:\n                vec! 1\n            else:\n                y <- clone$\n        ) <- into_iter$ <- count$\n    b\n", "}).into_iter().count();"),
        // A brace group is never an atom: `json! ({ .. })` is isolated.
        ("fn m$:\n    let body = json! ({\"a\": 1})\n    body\n", "json!({\"a\": 1})"),
    ]);
    // A body is never read as Harsh, so its lines are not statements.
    let toks = harsh_lang::lex::lex("fn m$:\n    let v = (0..3) <- map (|_| { view! { <input type=\"text\" /> } })\n").expect("lex");
    assert!(harsh_lang::layout::build(toks).is_ok());
}

/// The converter on the same shapes.
#[test]
fn converter_leptos_shapes() {
    let rust = "#[component]\npub fn Card(\n    /// The service.\n    service: Service,\n    /// Stagger.\n    #[prop(default = 0.0)]\n    delay: f32,\n) -> impl IntoView {\n    let v = items.into_iter().map(|item| {\n        let y = item;\n        view! { <li aria-label=\"x\">{y}</li> }\n    }).collect_view();\n    let r = (extra > 0).then(|| { view! { <li>{extra}</li> } });\n    let b = if c { vec![1] } else { y.clone() }.into_iter().count();\n    {\n        let z = 1;\n        f(z);\n    }\n    leptos_routes(&opts, routes, { let o = o.clone(); move || shell(o.clone()) });\n    v\n}\n";
    let harsh = convert(rust);
    for want in [
        "pub fn Card\n    /// The service.\n    (service: Service)\n    /// Stagger.\n    (#[prop (default = 0.0)] delay: f32)\n    -> impl IntoView:\n",
        "<- map (|item|:\n            let y = item\n            view! { <li aria-label=\"x\">{@: y :@}</li> }\n    ) <- collect_view$\n",
        "<- then (||:\n            view! { <li>{@: extra :@}</li> }\n    )\n",
        "    let b = (\n        if c:\n            vec! 1\n        else:\n            y <- clone$\n    ) <- into_iter$ <- count$\n",
        "    do:\n        let z = 1\n        f z;\n",
        "    leptos_routes (&opts) routes (do:\n        let o = o <- clone$\n        move || shell (o <- clone$)\n    )\n",
    ] {
        assert!(harsh.contains(want), "\n  expected: {want:?}\n  got:\n{harsh}");
    }
    let back = transpile(&harsh);
    // Identical but for the parens the chain off a block expression needed.
    let strip = |s: &str| {
        let one: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
        one.replace("( if c", "if c").replace("(if c", "if c").replace("} ).into_iter", "}.into_iter").replace("}).into_iter", "}.into_iter")
    };
    assert!(same_tokens(&strip(rust), &strip(&back)), "{}", first_diff(&strip(rust), &strip(&back)));
}

/// From the crate's first real build: a paren block's tail holding a brace
/// body, and include paths re-based for the generated tree.
#[test]
fn leptos_build_findings() {
    check(&[(
        "fn m$:\n    let brand_name =\n        (\n            if c:\n                vec! a\n            else:\n                b\n        ) <- into_iter$ <- map (|line| view! {\n                <span class=\"x\">{line}</span>\n        }) <- collect_view$\n    brand_name\n",
        ".into_iter().map(|line| view! {\n                <span class=\"x\">{line}</span>\n        }",
    )]);
    // Include paths: relative to the source, re-based to where the Rust is
    // written (`src/content.hrs` -> `target/hrs/content.rs` adds one `../`).
    let dir = std::env::temp_dir().join(format!("hrs-inc-{}", std::process::id()));
    let src_dir = dir.join("src");
    let gen_dir = dir.join("target").join("hrs");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&gen_dir).unwrap();
    let hrs = src_dir.join("content.hrs");
    std::fs::write(&hrs, "fn main$:\n    let s = include_str! \"../content/site.toml\"\n    let t = include_bytes! \"data.bin\"\n    s\n").unwrap();
    let rs = gen_dir.join("content.rs");
    let map = gen_dir.join("content.map.json");
    harsh_lang::driver::transpile_one(&std::fs::read_to_string(&hrs).unwrap(), &hrs, &rs, &map).expect("transpile");
    let out = std::fs::read_to_string(&rs).unwrap();
    assert!(out.contains("include_str!(\"../../content/site.toml\")"), "{out}");
    assert!(out.contains("include_bytes!(\"../../src/data.bin\")"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
    // A single file written beside its source keeps its paths.
    check(&[("fn m$:\n    let s = include_str! \"../content/site.toml\"\n    s\n", "include_str!(\"../content/site.toml\")")]);
}

/// Every spelling the language has retired is refused, in **both** forms --
/// across lines and on one line -- and each refusal names its replacement.
///
/// This is the guard the migrations lacked. Each of them refused the header
/// that *ends* in its old mark and left the inline form standing, so for days
/// `match x: a => 1` and `match x\ a => 1` emitted the same Rust, the Book
/// taught both on one page, and four fixtures in this very file pinned the
/// retired one (found 2026-09-20). A corpus that compiles cannot catch this:
/// a retired spelling the transpiler still accepts builds, runs and
/// round-trips. When a spelling is retired, its two rows go here first.
#[test]
fn retired_spellings_are_refused_in_both_forms() {
    // (what it was, the source, a phrase the refusal must contain)
    let table: &[(&str, &str, &str)] = &[
        ("match x:  across lines", "fn f (x: i32) -> i32:\n    match x:\n        1 => 1\n        _ => 0\n", "opened by `\\`"),
        ("match x:  on one line", "fn f (x: i32) -> i32:\n    match x: 1 => 1, _ => 0\n", "opened by `\\`"),
        ("match x do:  across lines", "fn f (x: i32) -> i32:\n    match x do:\n        1 => 1\n        _ => 0\n", "opened by `\\`"),
        ("match x do:  on one line", "fn f (x: i32) -> i32:\n    match x do: 1 => 1, _ => 0\n", "opened by `\\`"),
        ("match x:  on one line, in parens", "fn f (x: i32) -> i32:\n    g (match x: 1 => 1, _ => 0)\n", "opened by `\\`"),
        ("struct P:  across lines", "struct P:\n    x: i32\n", "no mark: `struct P`"),
        ("struct P:  on one line", "struct P: x: i32, y: i32\n", "`struct P\\ a: T, b: U` inline"),
        ("enum E:  across lines", "enum E:\n    A\n    B\n", "no mark: `enum E`"),
        ("enum E:  on one line", "enum E: A, B\n", "`enum E\\ a: T, b: U` inline"),
        ("union U:  across lines", "union U:\n    a: u32\n", "no mark: `union U`"),
        ("union U:  on one line", "union U: a: u32, b: f32\n", "`union U\\ a: T, b: U` inline"),
        ("struct P do", "struct P do\n    x: i32\n", "takes no opener"),
        ("impl P:", "struct P\n    x: i32\nimpl P:\n    fn get (&self) -> i32:\n        self <- x\n", "impl P"),
        ("trait T:", "trait T:\n    fn f (&self)\n", "trait T"),
        ("mod m:", "mod m:\n    fn f$:\n        g$\n", "mod m"),
        ("macro_rules~ name:", "macro_rules~ tw:\n    (($a:expr)) => do: $a\n", "no mark"),
        ("fn main ():", "fn main ():\n    g$\n", "fn main$"),
        // A1, 2026-09-22: a bracket after `!` is an array argument, so Rust's
        // bracket call written in Harsh would mean something else.
        ("vec! [1, 2]  spaced", "fn f$:\n    let v = vec! [1, 2]\n", "is `vec! a b` in Harsh"),
        ("vec![1, 2]  tight", "fn f$:\n    let v = vec![1, 2]\n", "is `vec! a b` in Harsh"),
        ("vec! [0; 4]", "fn f$:\n    let v = vec! [0; 4]\n", "is `vec! { x; n }` in Harsh"),
        ("vec![]", "fn f$:\n    let v: Vec<i32> = vec![]\n", "is `vec!$` in Harsh"),
        ("vec! [..] inside parens", "fn f$:\n    g (vec! [1, 2])\n", "is `vec! a b` in Harsh"),
        // A2, 2026-09-22: `m!\` retired; the stream goes in braces.
        ("m!\\  across lines", "fn f$:\n    lazy_static!\\\n        static ref X: u8 = 1\n", "`lazy_static! { … }`"),
        ("m!\\  on one line", "fn f$:\n    let m = hm!\\ 1 => 2\n", "`hm! { … }`"),
        // B1, 2026-09-22: `@:` / `:@` belong inside a macro's body.
        ("@: outside a body", "fn f$:\n    let d = f x@: y :@\n", "`@:` marks a hole"),
        (":@ outside a body", "fn f$:\n    let d = f y :@\n", "`:@` marks a hole"),
    ];
    for (what, src, phrase) in table {
        let msg = layout_err(src);
        assert!(msg.contains(phrase), "{what}: refused, but the message does not name the replacement ({phrase:?}):\n  {msg}");
    }
    // And the spellings that replaced them are accepted.
    for src in [
        "fn f (x: i32) -> i32:\n    match x\\ 1 => 1, _ => 0\n",
        "fn f (x: i32) -> i32:\n    match x\\ 0 => 0, 1 | 2 => 1, _ => 2\n",
        "fn f (x: i32) -> i32:\n    g (match x\\ 1 => 1, _ => 0)\n",
        "struct P\\ x: i32, y: i32\n",
        "struct W<T: Clone>\\ v: T\n",
        "enum E\\ A, B\n",
    ] {
        transpile(src);
    }
    // `union` is a contextual keyword: `union U\` inline was read as the
    // application `union(U)`; a function that happens to be named `union`
    // must still be callable.
    assert!(transpile("union U\\ a: u32, b: f32\n").starts_with("union U {"));
    assert!(transpile("fn main$:\n    let n = union 1 2\n").contains("union(1, 2)"));
}

/// Where a `~` call's token stream ends (rulings of 2026-09-19 and 09-20):
/// the rest of its line **and every following line indented deeper than the
/// line the call sits on**; an unmatched closing bracket ends it sooner.
/// The column decides -- the dedent rule that closes any block -- and no
/// token's kind is consulted. Until 0.1.14 the stream stopped at the physical
/// newline, so a continued call emitted `[1, 2]3(4)` with no Harsh error and
/// the isolated multi-line form failed with "1 token(s) were left over".
#[test]
fn a_harsh_macros_stream_is_bounded_by_indentation_and_by_its_group() {
    const LST: &str = "macro_rules~ lst\n    ($( ($x:expr) )*) => do: [$( $x ),*]\n\n";
    let t = |body: &str| transpile(&format!("{LST}fn main$:\n{body}"));

    // Lines deeper than the call's line belong to the stream.
    assert!(t("    let v =\n        lst~ 1 2\n            3 4\n").contains("[1, 2, 3, 4]"));
    assert!(t("    let v = lst~ 1 2\n        3 4\n").contains("[1, 2, 3, 4]"));
    // A line at the call line's own column does not: the stream is `1 2`.
    assert!(t("    let v =\n        lst~ 1 2\n        g$\n").contains("[1, 2]"));
    // The next statement, of course, ends it.
    assert!(t("    let v = lst~ 1 2\n    let w = lst~ 3\n").contains("[1, 2];"));

    // Isolation: the `)` of a group the call was written *inside* ends the
    // stream, on one line and across lines...
    assert!(t("    let t = ((lst~ 4), 0)\n").contains("(([4]), 0)"), "the `, 0` is the tuple's");
    assert!(t("    let v =\n        (lst~ 1 2\n            3 4)\n").contains("[1, 2, 3, 4]"));
    // ...and sooner than the indentation would: `<- len$` sits on a deeper
    // line, but the `)` came first, so it chains on the *result*.
    assert!(t("    let n =\n        (lst~ 1 2\n            3 4) <- len$\n").contains("([1, 2, 3, 4]).len()"));
    assert!(t("    let n = (lst~ 1 2) <- len$\n").contains("([1, 2]).len()"));
    // A bracket opened inside the stream is matched inside it, across lines.
    assert!(t("    let v = lst~ (1,\n        2) 3\n").contains("[(1, 2), 3]"));

    // A `\` after the mark is a token of the stream, never an opener: a
    // matcher that names it receives it (the user's ruling, 2026-09-20).
    let named = "macro_rules~ tw\n    (\\ ($a:expr)) => do: $a * 2\n\nfn main$:\n    let x = tw~\\ 4\n";
    assert!(transpile(named).contains("let x = 4 * 2"));
}

/// The block form: `m~ do:` hands the lines beneath as the stream, the opener
/// dropped; isolated, the group's `)` ends it (it used to be swallowed).
#[test]
fn a_harsh_macros_block_form_ends_at_its_dedent_or_its_group() {
    const TW: &str = "macro_rules~ twice\n    (($b:expr)) => do:\n        $b\n        $b\n\n";
    let bare = transpile(&format!("{TW}fn main$:\n    twice~ do:\n        g$\n    h$\n"));
    assert_eq!(bare.matches("g()").count(), 2, "{bare}");
    assert_eq!(bare.matches("h()").count(), 1, "the next statement is not the stream's:\n{bare}");
    let isolated = transpile(&format!("{TW}fn main$:\n    (twice~ do:\n        g$)\n    h$\n"));
    assert_eq!(isolated.matches("g()").count(), 2, "{isolated}");
    assert_eq!(isolated.matches("h()").count(), 1, "{isolated}");
}

/// A `\` opens a specification block -- Rust's `{ a, b }` groupings -- and
/// always belongs to the construct written before it; alone it means nothing
/// (the user's ruling, 2026-09-20). `let x = \ 4 * 2` used to emit
/// `{ 4 * 2, }`. Because a Harsh macro's expansion is literal, this is also
/// what makes a malformed expansion an error instead of invalid Rust.
#[test]
fn a_backslash_follows_the_construct_it_specifies() {
    for src in [
        "fn main$:\n    let x = \\ 4 * 2\n",
        "fn main$:\n    f (\\ a = 1)\n",
        "fn main$:\n    let x = 1 + \\ a = 1\n",
        // `$a` captures `\ 4`; the expansion `\ 4 * 2` is refused, at the call.
        "macro_rules~ tw\n    (($a:expr)) => do: $a * 2\n\nfn main$:\n    let x = tw~\\ 4\n",
        // `\` is a token of the stream, so nothing beneath is a block's entries.
        "macro_rules~ lst\n    ($( ($x:expr) )*) => do: [$( $x ),*]\n\nfn main$:\n    let v = lst~\\\n        1\n        2\n",
    ] {
        let msg = layout_err(src);
        assert!(msg.contains("follows the construct it specifies"), "{src}\n  {msg}");
    }
    // Every construct that owns one still does.
    for src in [
        "fn main$:\n    let p = P\\ x = 1, y = 2\n",
        "fn main$:\n    let p =\n        a.b.P\\\n            x = 1\n",
        "struct P\n    x: i32\nimpl P\n    fn new$ -> Self:\n        Self\\ x = 1\n",
        "fn main$:\n    let w = W<i32>\\ v = 1\n",
        "fn main$:\n    let P\\ x, y = p\n",
        "fn main$:\n    let a = match f (x)\\ 0 => 0, _ => 1\n",
        "struct P\\ x: i32, y: i32\n",
        // A header may end in a bracketed where clause (found by the guide's
        // own build, which the first version of this check refused).
        "struct Wrapper<T> [where T: Display]\\ field: T, other: u32\n",
    ] {
        transpile(src);
    }
}

/// A Rust macro's brace call is written in braces, as Rust writes it, one line
/// or many (`m!\\` was the spelling until 2026-09-22, A2). The body is Rust.
#[test]
fn a_rust_macros_brace_call_is_written_in_braces() {
    let one = transpile("fn main$:\n    let v = hm! { 1 => \"a\", 2 => \"b\" }\n");
    assert!(one.contains("hm! { 1 => \"a\", 2 => \"b\" }"), "{one}");
    let many = transpile("fn main$:\n    let v = hm! {\n        1 => \"a\",\n        2 => \"b\"\n    }\n    let w = 1\n");
    assert!(many.contains("hm! {\n        1 => \"a\",\n        2 => \"b\"\n    };\n    let w = 1;"), "{many}");
    // The paren call stays juxtaposed.
    assert!(transpile("fn main$:\n    let v = lst! 1 2 3\n").contains("lst!(1, 2, 3)"));
}

/// `hrs-from` copies a Rust macro's brace body byte for byte (principle 2,
/// 2026-09-23), and `hrs` copies it back: a DSL's own tokens are never read.
#[test]
fn converter_copies_a_brace_call_as_rust() {
    for rust in [
        "fn main() {\n    let v = hm!{ 1 => \"a\", id(2) => \"b\" };\n}\n",
        "fn main() {\n    let v = hm! {\n        1 => \"a\",\n        id(2) => \"b\"\n    };\n}\n",
        "fn main() {\n    let v = hm!{ 1 => \"a\", 2 => \"b\", };\n}\n",
        "fn main() {\n    let v = sel!{ a => { f() }, b => { g() } };\n}\n",
        "fn main() {\n    tl!{ static A: i32 = 1; static B: i32 = 2 };\n}\n",
        "fn main() {\n    let v = calc!{ eval 1 + 2 };\n}\n",
    ] {
        let harsh = convert(rust);
        let back = transpile(&harsh);
        assert_eq!(norm_tokens(rust), norm_tokens(&back), "through:\n{harsh}\nback:\n{back}");
    }
    assert!(convert("fn main() {\n    let v = hm!{ 1 => \"a\", id(2) => \"b\" };\n}\n").contains("id(2)"));
}

/// A fragment stops at whatever the matcher names **next** -- a follow set,
/// not a single token (2026-09-20). A repetition that comes next names the
/// literal its body begins with, and one that may match nothing lets what
/// follows it count too; inside a repetition, the next round's leading
/// literal is a stop as well. Before this, a fragment followed by a
/// repetition took one atom: `$it:expr $(if $c:expr)*` cut `0..4` to `0`
/// and no arm matched, and a `$c` inside the repetition took `x` from
/// `x > 1`. It is what a comprehension's matcher needs.
#[test]
fn a_fragment_stops_at_what_a_following_repetition_begins_with() {
    const G: &str = "macro_rules~ g\n    (($e:expr) for ($p:pat) in ($it:expr) $(if ($c:expr))*) => do:\n        ($it) <- into_iter$ <- flat_map (move |$p| ((true $( && ($c) )*) <- then (|| $e)))\n\n";
    // Expanded tokens are re-spaced, so compare without whitespace.
    let t = |call: &str| -> String {
        transpile(&format!("{G}fn main$:\n    let v: Vec<i32> = ({call}) <- collect$\n")).split_whitespace().collect()
    };
    // No condition: the range is not cut short.
    assert!(t("g~ x for x in 0..4").contains("(0..4).into_iter()"), "{}", t("g~ x for x in 0..4"));
    // One, then several: each condition is a whole expression.
    assert!(t("g~ x for x in 0..6 if x > 3").contains("true&&(x>3)"));
    let two = t("g~ x for x in 0..10 if x % 2 == 0 if x > 3");
    assert!(two.contains("true&&(x%2==0)&&(x>3)"), "{two}");
    // A repetition that begins with a fragment still takes one atom per
    // element: `$( $x:expr ),*` is unchanged.
    let lst = transpile("macro_rules~ lst\n    ($( ($x:expr) ),*) => do: [$( $x ),*]\n\nfn main$:\n    let v = lst~ 1, 2 + 3, 4\n");
    assert!(lst.contains("[1, 2 + 3, 4]"), "{lst}");
}

/// A `$` a macro's transcriber wrote carries a span inside the macro's
/// *definition*. The parens it becomes used to anchor the emitter's gap
/// copying like a hand-written `$`, so `(|| $x)$` followed by more source
/// pasted everything from the definition down to the call into the output:
/// the expansion, then the original line again, unexpanded (found
/// 2026-09-21 -- a comprehension's levels are closures, so `g~` hit it on
/// every multi-level call written on one line). Only a source `$` anchors.
#[test]
fn an_expanded_dollar_does_not_copy_source_between_definition_and_call() {
    let src = "macro_rules~ r\n    (($x:expr)) => do: (|| $x)$\n\nfn main$:\n    let v = (r~ 7) + 5\n";
    let out = transpile(src);
    assert!(!out.contains("r~"), "the original call leaked into the output:\n{out}");
    assert!(!out.contains("macro_rules"), "{out}");
    // A hand-written `$` keeps its source spacing as before.
    assert!(transpile("fn main$:\n    let v = f$ + 5\n").contains("f() + 5"));
}

/// `g~`, Harsh's comprehension, and its shorthands, from the prelude
/// (the user's rulings, 2026-09-20/21). One closure per `for`; the `if`s
/// that follow a `for` fold into that closure's one condition under
/// `bool::then`; every level but the innermost is flattened. Lazy, and
/// `for` means `IntoIterator`, as it does everywhere in the language.
#[test]
fn the_prelude_comprehension_and_its_shorthands() {
    let flat = |src: &str| -> String { transpile(src).split_whitespace().collect() };
    // Available with no definition and no `use`.
    let one = flat("fn main$:\n    let v: Vec<i32> = (g~ x * 2 for x in 0..5 if x > 1 if x < 4) <- collect$\n");
    assert!(one.contains("(0..5).into_iter().flat_map(move|x|((true&&(x>1)&&(x<4)).then(||x*2)))"), "{one}");
    // Two levels: the outer is flattened, the inner is not.
    let two = flat("fn main$:\n    let v: Vec<i32> = (g~ x + y for x in 0..3 for y in 0..3 if y > x) <- collect$\n");
    assert!(two.contains(".flatten()"), "{two}");
    assert!(!two.contains("g~"), "a recursive expansion leaked its call:\n{two}");
    // The shorthands collect; their turbofish survives expansion.
    for (call, want) in [
        ("list~ x for x in 0..3", "collect::<Vec<_>>()"),
        ("set~ x for x in 0..3", "collect::<std::collections::HashSet<_>>()"),
        ("dict~ x => x for x in 0..3", "collect::<std::collections::HashMap<_,_>>()"),
    ] {
        let out = flat(&format!("fn main$:\n    let v = {call}\n"));
        assert!(out.contains(want), "{call}: {out}");
    }
}

/// A file's own macro shadows the prelude's, as Rust's prelude is shadowed;
/// `hrs_std.NAME~` always reaches the prelude's, and the prelude's own
/// macros recurse through that path so a user's `g` is never picked up from
/// inside one. A prelude name used with `!` is some Rust macro, not an error.
#[test]
fn a_files_own_macro_shadows_the_prelude() {
    let own = "macro_rules~ g\n    (($x:expr)) => do: $x * 100\n\n";
    assert!(transpile(&format!("{own}fn main$:\n    let a = g~ 3\n")).contains("3 * 100"));
    let q = transpile(&format!("{own}fn main$:\n    let v: Vec<i32> = (hrs_std.g~ x for x in 0..3) <- collect$\n"));
    assert!(q.contains("flat_map") && !q.contains("hrs_std"), "{q}");
    let l = transpile(&format!("{own}fn main$:\n    let v = list~ x for x in 0..3\n"));
    assert!(l.contains("flat_map") && !l.contains("* 100"), "list~ must not reach the user's g:\n{l}");
    let rust = "macro_rules! g { ($x:expr) => { $x + 1 }; }\n\nfn main$:\n    let v: Vec<i32> = (hrs_std.g~ x for x in 0..3) <- collect$\n    let b = g! 1\n";
    assert!(transpile(rust).contains("g!(1)"));
}

/// A comma inside a type's generic list belongs to the list, not to the
/// parameter group (2026-09-21). The rule composes from bricks applied
/// recursively -- the lexer's atomic tokens, bracket groups, and now `<…>` --
/// and the last brick is safe only in a parameter list, where after `:` a
/// type follows and `<` can only open, never compare. Until this, the forward
/// direction refused `(r: Result<i32, String>)` and `hrs-from` silently split
/// Rust's `fn f(r: Result<i32, String>)` into `(r: Result<i32) (String>)`.
#[test]
fn a_comma_inside_generics_belongs_to_the_generic_list() {
    for (src, want) in [
        ("fn f (r: Result<i32, String>) -> i32:\n    0\n", "fn f(r: Result<i32, String>)"),
        ("fn f (a: i32) (m: HashMap<String, i32>) -> i32:\n    a\n", "fn f(a: i32, m: HashMap<String, i32>)"),
        ("fn f (m: HashMap<String, Vec<i32>>) -> i32:\n    0\n", "fn f(m: HashMap<String, Vec<i32>>)"),
        ("fn f (it: impl Iterator<Item = (i32, i32)>) -> i32:\n    0\n", "fn f(it: impl Iterator<Item = (i32, i32)>)"),
        // Spacing carries no meaning in a type: the lexer gives the same `<`.
        ("fn f (r: Result <i32, String>) -> i32:\n    0\n", "fn f(r: Result <i32, String>)"),
    ] {
        assert!(transpile(src).contains(want), "{src}\n{}", transpile(src));
    }
    // A comma between two parameters is still refused: that brick is unchanged.
    assert!(layout_err("fn f (a: i32, b: i32) -> i32:\n    a\n").contains("one parameter"));
    // And the converter no longer splits inside the list.
    for (rust, want) in [
        ("fn f(r: Result<i32, String>) -> i32 { 0 }\n", "fn f (r: Result<i32, String>) -> i32:"),
        ("fn f(a: i32, m: HashMap<String, i32>) -> i32 { a }\n", "fn f (a: i32) (m: HashMap<String, i32>) -> i32:"),
        ("fn f(m: HashMap<String, Vec<i32>>) -> i32 { 0 }\n", "fn f (m: HashMap<String, Vec<i32>>) -> i32:"),
    ] {
        let h = convert(rust);
        assert!(h.contains(want), "{rust}\n{h}");
    }
}

/// A repetition with no separator whose body begins with a fragment is
/// juxtaposition: each round takes one atom, and the repetition ends where
/// what follows it begins (2026-09-21, found by the first matrix literal).
/// Before, the follow set made `$( $x:expr )*` span a whole row, emitting
/// `1.0(2.0)`, and once that was fixed a round took the row's `;` as one more
/// element -- a one-atom `expr` accepts any single token.
#[test]
fn a_juxtaposed_repetition_takes_one_atom_per_round_and_stops_at_what_follows() {
    let m = "macro_rules~ m\n    ([ $( $( ($x:expr) )* );* ]) => do: vec! $( (vec! $( ($x) )*) )*\n\n";
    let out: String = transpile(&format!("{m}fn main$:\n    let a = m~ [1.0 2.0; 3.0 4.0]\n")).split_whitespace().collect();
    assert!(out.contains("vec!(vec!(1.0,2.0),vec!(3.0,4.0))"), "{out}");
    // A negative entry is one element, as Julia's `[1 -2]` is two.
    let neg: String = transpile(&format!("{m}fn main$:\n    let a = m~ [1.0 -2.0; 3.0 4.0]\n")).split_whitespace().collect();
    assert!(neg.contains("vec!(vec!(1.0,-2.0),vec!(3.0,4.0))"), "{neg}");
    // The separated and the literal-led repetitions are unchanged.
    assert!(transpile("macro_rules~ l\n    ($( ($x:expr) ),*) => do: [$( $x ),*]\n\nfn main$:\n    let v = l~ 1, 2 + 3, 4\n").contains("[1, 2 + 3, 4]"));
}

/// Julia's matrix and vector literals, from the prelude (2026-09-21):
/// `m~ [1 2; 3 4]` -- spaces between entries, `;` between rows -- and
/// `v~ [1, 2, 3]`, commas making a vector as in Julia. They expand to the
/// `hrs_std` crate's types, where `*` is the matrix product, as in Julia.
#[test]
fn the_prelude_matrix_and_vector_literals() {
    let flat = |src: &str| -> String { transpile(src).split_whitespace().collect() };
    let m = flat("fn main$:\n    let a = m~ [1.0 -2.0; 3.0 4.0]\n");
    assert!(m.contains("hrs_std::Matrix::from_rows(vec!(vec!(1.0,-2.0),vec!(3.0,4.0)))"), "{m}");
    let v = flat("fn main$:\n    let x = v~ [1.0, 2.0, 3.0]\n");
    assert!(v.contains("hrs_std::Vector::from_vec(vec!(1.0,2.0,3.0))"), "{v}");
    // A file's own `m` shadows the prelude's, and needs no crate.
    let own = transpile("macro_rules~ m\n    (($x:expr)) => do: $x + 1\n\nfn main$:\n    let a = m~ 1\n");
    assert!(own.contains("1 + 1") && !own.contains("hrs_std"), "{own}");
}

/// A project that calls `m~` or `v~` without depending on `hrs_std` is told
/// which line to add, instead of rustc failing on an unresolved crate.
#[test]
fn a_matrix_literal_names_the_crate_it_needs() {
    assert_eq!(harsh_lang::driver::uses_hrs_std("fn main$:\n    let a = m~ [1 2]\n"), Some(("m~".into(), 2)));
    assert_eq!(harsh_lang::driver::uses_hrs_std("fn main$:\n    let x = v~ [1, 2]\n"), Some(("v~".into(), 2)));
    // `g~` needs nothing; a variable named `m` is not a call; a shadowing
    // definition takes the name away from the prelude.
    assert_eq!(harsh_lang::driver::uses_hrs_std("fn main$:\n    let m = 1\n    let s = list~ x for x in 0..3\n"), None);
    assert_eq!(harsh_lang::driver::uses_hrs_std("macro_rules~ m\n    (($x:expr)) => do: $x\n\nfn main$:\n    let a = m~ 1\n"), None);
    // A dotted operator and a function marked `<>` are written to Rust as
    // `hrs_std::DOT` and `hrs_std::each!`: they need the crate too.
    assert_eq!(harsh_lang::driver::uses_hrs_std("use std.collections.*\n\nfn f (a: &M) -> M:\n    a .* a\n"), Some((".*".into(), 4)));
    assert_eq!(harsh_lang::driver::uses_hrs_std("fn f (a: &M) -> M:\n    f64.sqrt<> a\n"), Some(("sqrt<>".into(), 2)));
    assert_eq!(harsh_lang::driver::uses_hrs_std("use std.collections.*\n\nfn f$:\n    let v = xs <- collect<Vec<_>>$\n"), None);
}

/// An array literal passed as an argument is isolated: brackets always index
/// in Harsh (the user's rule, 2026-09-11), so `f [1, 2]` would be an index of
/// `f`. The converter wrote it bare; the self-host round trip caught it on
/// `s.starts_with(['=', '.'])`, in code written minutes earlier (2026-09-21).
#[test]
fn converter_isolates_an_array_literal_argument() {
    for (rust, harsh, back) in [
        ("fn g() -> i32 { let b = f([1, 2]); b }\n", "f ([1, 2])", "f([1, 2])"),
        ("fn g() -> i32 { let b = f([1, 2], 3); b }\n", "f ([1, 2]) 3", "f([1, 2], 3)"),
        ("fn g(s: &str) -> bool { s.starts_with(['=', '.']) }\n", "starts_with (['=', '.'])", "starts_with(['=', '.'])"),
    ] {
        let h = convert(rust);
        assert!(h.contains(harsh), "{rust}\n{h}");
        assert!(transpile(&h).contains(back), "{}", transpile(&h));
    }
    // An indexed name is not an array literal and is unaffected: it still
    // comes back as the same call.
    assert!(transpile(&convert("fn g(v: Vec<i32>) -> i32 { f(v[0]) }\n")).contains("f(v[0])"));
}

/// Inside `m~ [ … ]` a line break separates rows as `;` does -- Julia's
/// grammar for its matrix literal (the user's request, 2026-09-21). Only the
/// prelude's `m`: a file's own macro, and every other bracket list, see a
/// line break as nothing at all.
#[test]
fn a_matrix_literal_may_take_one_row_per_line() {
    let flat = |src: &str| -> String { transpile(src).split_whitespace().collect() };
    let lines = flat("fn main$:\n    let b =\n        m~ [1 2 3\n            4 5 6]\n");
    let semis = flat("fn main$:\n    let b = m~ [1 2 3; 4 5 6]\n");
    assert!(lines.contains("from_rows(vec!(vec!(1,2,3),vec!(4,5,6)))"), "{lines}");
    assert_eq!(lines, semis, "the two layouts are the same matrix");
    // A `;` at the end of a line is not doubled.
    let both = flat("fn main$:\n    let b =\n        m~ [1 2 3;\n            4 5 6]\n");
    assert!(both.contains("vec!(vec!(1,2,3),vec!(4,5,6))"), "{both}");
    // A file's own `m` gets no rows: the line break is nothing to it.
    let own = flat("macro_rules~ m\n    ([ $( ($x:expr) )* ]) => do: [$( $x ),*]\n\nfn main$:\n    let b =\n        m~ [1 2\n            3 4]\n");
    assert!(own.contains("[1,2,3,4]"), "{own}");
}

/// Julia's grammar for its matrix literal, not a list of shapes (the user's
/// ruling, 2026-09-21, correcting the Matlab forms Claude had added the same
/// morning): a space is `hcat`, `;` or a line break is `vcat`, `,` makes the
/// entries of a vector, and every entry is a block -- a nested bracket built
/// by the same rules. All-number rows take the direct path, `from_rows`.
#[test]
fn a_matrix_literal_follows_julias_grammar() {
    let flat = |src: &str| -> String { transpile(src).split_whitespace().collect() };
    let at = |call: &str| flat(&format!("fn main$:\n    let a =\n        {call}\n"));
    // Numbers: rows by `;` or by line, entries by spaces.
    for call in ["m~ [1 3 4; 5 6 2]", "m~ [1 3 4\n            5 6 2]"] {
        assert!(at(call).contains("from_rows(vec!(vec!(1,3,4),vec!(5,6,2)))"), "{call}: {}", at(call));
    }
    // A comma list of numbers is a column.
    assert!(at("m~ [1, 2, 3]").contains("from_rows(vec!(vec!(1),vec!(2),vec!(3)))"));
    // Blocks: side by side is `hcat`, stacked is `vcat`, each built by `m~`.
    let cols = at("m~ [[1, 5] [3, 6] [4, 2]]");
    assert!(cols.contains("hrs_std::vcat(vec!(hrs_std::hcat(vec!(") && cols.contains("vec!(vec!(1),vec!(5))"), "{cols}");
    let stacked = at("m~ [[1 3 4]\n            [5 6 2]]");
    assert!(stacked.contains("hrs_std::vcat(vec!(hrs_std::hcat(vec!(") && stacked.matches("hcat").count() == 2, "{stacked}");
    // A block may be a matrix: concatenation. A paren group's parens go once.
    let joined = at("m~ [(&a) (&a)]");
    assert!(joined.contains("hrs_std::block(&a)") && !joined.contains("block((&a))"), "{joined}");
    // Julia's refusals, in Julia's terms.
    assert!(at("m~ [[1 3 4], [5 6 2]]").contains("commasneverconcatenate"));
    assert!(flat("fn main$:\n    let v = v~ [1 2 3]\n").contains("spacesmakearow"));
    // A vector: a comma list, or one entry per line -- the same tokens.
    assert_eq!(
        flat("fn main$:\n    let v = v~ [1, 2, 3]\n"),
        flat("fn main$:\n    let v =\n        v~ [1\n            2\n            3]\n").replace("letv=", "letv=")
    );
}

/// A panic names the Harsh line, not the generated one (2026-09-21): `hrs
/// run` and `hrs test` read the program's stderr and put every
/// `target/hrs/main.rs:L:C` back through the source map. Here through the
/// real binary and a real map; the position is the same token Rust named.
#[test]
fn a_panic_is_put_back_on_its_harsh_line() {
    let dir = std::env::temp_dir().join(format!("hrs-panic-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("target/hrs")).unwrap();
    std::fs::write(dir.join("src/main.hrs"), "fn main$:\n    let v: Vec<i32> = vec!$\n    println! \"{}\" v[3]\n").unwrap();
    let ok = std::process::Command::new(env!("CARGO_BIN_EXE_hrs"))
        // Absolute paths, as the driver writes them into its maps.
        .arg(dir.join("src/main.hrs"))
        .arg("-o")
        .arg(dir.join("target/hrs/main.rs"))
        .arg("--map")
        .arg(dir.join("target/hrs/main.map.json"))
        .current_dir(&dir)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let generated = std::fs::read_to_string(dir.join("target/hrs/main.rs")).unwrap();
    let line = generated.lines().position(|l| l.contains("v[3]")).unwrap() + 1;
    let col = generated.lines().nth(line - 1).unwrap().find("v[3]").unwrap() + 1;
    let places = harsh_lang::driver::PanicPlaces::load(&dir, &[dir.join("target/hrs/main.map.json")]);
    let said = format!("thread 'main' panicked at target/hrs/main.rs:{line}:{col}:");
    assert_eq!(places.rewrite(&said), "thread 'main' panicked at src/main.hrs:3:19:");
    // A backtrace frame, with Rust's `./` prefix, and text the maps do not
    // cover, are handled and left alone respectively.
    assert!(places.rewrite(&format!("  at ./target/hrs/main.rs:{line}:{col}")).contains("./src/main.hrs:3:19"));
    assert_eq!(places.rewrite("no place here: main.rs"), "no place here: main.rs");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A macro never sees a comment, in Rust or in Harsh (2026-09-21). A
/// comment on one of a `~` call's lines used to be captured into a spanning
/// fragment -- `1..20 // only a is available` as one expression -- and the
/// one-line expansion was commented out from there on. Found writing the
/// Book's annotated comprehension, which is this test.
#[test]
fn a_comment_inside_a_macro_call_is_not_part_of_it() {
    let src = "fn main$:\n    let triples =\n        list~ (a, b, c)\n            for a in 1..20        // only a is available for the condition\n            for b in a..20        // only a, b are available for the condition\n            for c in b..20        // a, b, c are available for the condition\n            if a * a + b * b == c * c\n";
    let out = transpile(src);
    assert!(!out.contains("//"), "a comment reached the expansion:\n{out}");
    let flat: String = out.split_whitespace().collect();
    assert!(flat.contains("(1..20).into_iter().flat_map(") && flat.contains("(a*a+b*b==c*c)"), "{flat}");
}

/// Julia's dotted operators (ruled 2026-09-21): `.*` is `* hrs_std::DOT *`, a
/// pure substitution, so Rust's precedence is left to do Julia's work.
#[test]
fn a_dotted_operator_is_a_pair_of_operators_round_a_dot() {
    let out = transpile("use std.collections.*\n\nfn f$:\n    let c = &a .* &b\n    let d = &a + &a.*&b ./ 2.0 .- x\n    let e = (g a) .+ h$ .* 3\n");
    assert!(out.contains("use std::collections::*;"), "a `use` glob is not an operator:\n{out}");
    assert!(out.contains("let c = &a * hrs_std::DOT * &b;"), "{out}");
    assert!(out.contains("let d = &a + &a * hrs_std::DOT *&b / hrs_std::DOT / 2.0 - hrs_std::DOT - x;"), "{out}");
    assert!(out.contains("let e = (g(a)) + hrs_std::DOT + h() * hrs_std::DOT * 3;"), "{out}");
    // A path, a range and a float are still what they were.
    let out = transpile("fn f$:\n    let r = a.b * 1.0 - c..d\n");
    assert!(out.contains("let r = a::b * 1.0 - c..d;"), "{out}");
}

/// `f<>` -- apply to each (the user's mark, 2026-09-21): a macro head, and the
/// rest is the application rule. The pipes see `f<>` as one function.
#[test]
fn a_function_marked_each_becomes_the_each_macro() {
    let src = "fn f$:\n    let r = f64.sqrt<> a\n    let p = f64.powf<> a 2.0\n    let q = (|x| x * x)<> a\n    let s = a |> f64.abs<> |> f64.sqrt<> |> total\n    let u = relu<> (&a * &b) .+ 1.0\n    println! \"{}\" (relu<> a)\n";
    let out = transpile(src);
    for want in [
        "let r = hrs_std::each!(f64::sqrt, a);",
        "let p = hrs_std::each!(f64::powf, a, 2.0);",
        "let q = hrs_std::each!(|x| x * x, a);",
        "let s = total(hrs_std::each!(f64::sqrt, hrs_std::each!(f64::abs, a)));",
        "let u = hrs_std::each!(relu, &a * &b) + hrs_std::DOT + 1.0;",
        "println!(\"{}\", hrs_std::each!(relu, a))",
    ] {
        assert!(out.contains(want), "missing `{want}` in:\n{out}");
    }
    // Generics and comparisons are untouched.
    let out = transpile("fn f$:\n    let v = xs <- collect<Vec<_>>$\n    let b = a < c && c > d\n");
    assert!(out.contains("xs.collect::<Vec<_>>()") && out.contains("a < c && c > d"), "{out}");
}

#[test]
fn the_each_mark_is_written_tight_and_rusts_empty_generics_are_dropped() {
    let toks = harsh_lang::lex::lex("fn f$:\n    let r = g <> a\n").expect("lex");
    let err = harsh_lang::layout::build(toks).expect_err("a spaced `<>` must be refused");
    assert!(err.msg.contains("written tight against it: `g<> a`"), "{}", err.msg);
    // Rust's `f::<>(x)` and `P<>` mean nothing; in Harsh they would mean
    // *apply to each*, so the converter drops them -- the one carve-out from
    // "Harsh is a superset of Rust".
    let harsh = convert("fn main() { let p: P<> = P(1); let x = sq::<>(3.0); }\n");
    assert!(!harsh.contains("<>"), "{harsh}");
    assert!(harsh.contains("let p: P = P 1") && harsh.contains("let x = sq 3.0"), "{harsh}");
}

/// A top-level comma in an index makes a tuple (ruled 2026-09-21): Julia's
/// `a[1, 2]`, through Rust's one-argument `Index`.
#[test]
fn a_comma_in_an_index_makes_a_tuple() {
    let src = "fn notify<T, U> (item: &T) (other: U) [where T: Clone, U: Copy]:\n    let x = a[1, 1]\n    a[0, 1] = 50\n    let v = &a[0..2, ..]\n    let w = a [1.., ..=1] <- copy$\n    let n = grid[i + 1, g j][0, 0]\n    let arr = [1, 2]\n    let t = total ([4, 5])\n    let one = a[(2, 3)]\n    let plain = xs[4]\n    println! \"{} {}\" a[0, 0] (&a[0, ..])\n    for p in [(1, 2), (3, 4)]:\n        show p\n";
    let out = transpile(src);
    for want in [
        "where T: Clone, U: Copy {",
        "let x = a[(1, 1)];",
        "a[(0, 1)] = 50;",
        "let v = &a[(0..2, ..)];",
        "[(1.., ..=1)].copy();",
        "let n = grid[(i + 1, g(j))][(0, 0)];",
        "let arr = [1, 2];",
        "let t = total([4, 5]);",
        "let one = a[(2, 3)];",
        "let plain = xs[4];",
        "println!(\"{} {}\", a[(0, 0)], &a[(0, ..)]);",
        "for p in [(1, 2), (3, 4)] {",
    ] {
        assert!(out.contains(want), "missing `{want}` in:\n{out}");
    }
}

/// Found by `round_trip_own_source` on 2026-09-21, in code written minutes
/// before: three shapes the converter wrote wrongly, one of them silently.
#[test]
fn converter_carries_a_block_operand_and_a_literal_closure_body() {
    let same = |rust: &str| {
        let harsh = convert(rust);
        let back = transpile(&harsh);
        assert_eq!(norm_tokens(rust), norm_tokens(&back), "through:\n{harsh}\nback:\n{back}");
        harsh
    };
    // A `match` as an operand with more after it. The `&& t < 9` used to be
    // written as a statement of its own, and transpiled without complaint.
    same("fn f(k: i32, t: i32) -> bool {\n    let ok = t > 0\n        && match k {\n            1 => true,\n            _ => false,\n        }\n        && t < 9;\n    ok\n}\n");
    // The same with a bracket inside a character literal before the `match`:
    // the depth scan read `'['` as an opener and wrote the retired `match x:`.
    let harsh = same("fn f(toks: &[Token], k: usize) -> bool {\n    let ok = toks[k].kind == Tk::Open('[')\n        && match toks[k - 1].kind {\n            Tk::Ident => true,\n            _ => false,\n        }\n        && k > 0;\n    ok\n}\n");
    assert!(harsh.contains("kind\\"), "a match is opened by `\\`:\n{harsh}");
    // A struct literal as a closure's body, across lines: the `|` that closes
    // the parameters was taken for a pattern's, and the literal for a block.
    let harsh = same("fn f(xs: Vec<i32>, t: &T) -> Vec<Token> {\n    xs.into_iter()\n        .map(|(kind, text)| Token {\n            ctx: t.ctx,\n            kind,\n            text: text.to_string(),\n        })\n        .collect()\n}\n");
    assert!(harsh.contains("Token\\") && !harsh.contains("do:"), "{harsh}");
    // A struct pattern in a closure's parameters is still a pattern.
    same("fn f(ps: Vec<Point>) -> Vec<i32> {\n    ps.into_iter().map(|Point { x, y }| x + y).collect()\n}\n");
}

/// A1 (ruled 2026-09-22): a bracket after a macro's bang is its first
/// argument, an array; the idiom is juxtaposition, and a stream that is not a
/// list of expressions goes in braces. The prelude's `m~` and `v~` were
/// written in the old spelling and are pinned here through their output.
#[test]
fn a_bracket_after_a_bang_is_an_array_argument() {
    let out = transpile("fn f$:\n    let a = vec! 1 2 3\n    let b = vec! (x + 1) (g y)\n    let c = vec!$\n    let d = vec! { 0u8; 4 }\n    let e = vec! ([1, 2, 3])\n    let g = m! [a, b] c\n    let h = f (vec! 1 2)\n");
    for want in [
        "let a = vec!(1, 2, 3);",
        "let b = vec!(x + 1, g(y));",
        "let c = vec!();",
        "let d = vec! { 0u8; 4 };",
        "let e = vec!([1, 2, 3]);",
        "let g = m!([a, b], c);",
        "let h = f(vec!(1, 2));",
    ] {
        assert!(out.contains(want), "missing `{want}` in:\n{out}");
    }
    let out = transpile("fn f$:\n    let a = m~ [1 2; 3 4]\n    let v = v~ [1.0, 2.0]\n");
    assert!(out.contains("hrs_std::Matrix::from_rows(vec!(vec!(1, 2), vec!(3, 4)))"), "{out}");
    assert!(out.contains("hrs_std::Vector::from_vec(vec!(1.0, 2.0))"), "{out}");
    // The converter's side: a list is juxtaposed, `;` goes in braces.
    let harsh = convert("fn f(n: usize) {\n    let a = vec![1, 2];\n    let b: Vec<i32> = vec![];\n    let c = vec![usize::MAX; n];\n    let d = vec![(1, 2), (3, 4)];\n    let e = vec![Some(1), None];\n}\n");
    for want in ["let a = vec! 1 2", "vec!$", "let c = vec! { usize::MAX; n }", "let d = vec! ((1, 2)) ((3, 4))", "let e = vec! (Some 1) None"] {
        assert!(harsh.contains(want), "missing `{want}` in:\n{harsh}");
    }
}

/// A `~` call's stream belongs to its macro (3.1.1): no rule for Rust's `!`
/// macros may judge it. Found 2026-09-22 on the raw path the editor and the
/// formatter's guard read, where the mark is already normalised to `!`: the
/// new bracket refusal fired on `filled~ [0u8; 2]`, and the older tuple rule
/// had always fired on the guide's own `pair~ (1, 2)`.
#[test]
fn a_tilde_stream_is_never_judged_by_the_bang_rules() {
    for src in [
        "fn f$:\n    let z = filled~ [0u8; 2]\n",
        "fn f$:\n    let p = pair~ (1, 2)\n",
        "fn f$:\n    let p = g (pair~ (1, 2))\n",
    ] {
        let toks = harsh_lang::lex::lex(src).expect("lex");
        if let Err(e) = harsh_lang::layout::build(toks) {
            panic!("{src:?} refused on the raw path: {}", e.msg);
        }
    }
}

/// Found 2026-09-22 while migrating `vec!` (A1): a block argument followed by
/// another block argument ended the application -- `f (P\\ x = 1) (Q\\ y = 2)`
/// was `f(P { .. })(Q { .. })`, a call on the result. Plain arguments after a
/// block argument always worked; a second block did not. Pre-existing, and on
/// the migration's path: a list of literals is now juxtaposed.
#[test]
fn a_block_argument_may_be_followed_by_another() {
    let out = transpile("fn f$:\n    let a = g (|x|: x) (|y|: y)\n    let b = g (P\\ x = 1) (Q\\ y = 2)\n    let c = g (P\\ x = 1) y (Q\\ y = 2)\n    let d = g y (P\\ x = 1) (Q\\ y = 2)\n    let e = vec! (P\\ x = 1) (Q\\ y = 2) (R\\ z = 3)\n");
    let flat: String = out.split_whitespace().collect();
    for want in [
        "leta=g(|x|{x},|y|{y});",
        "letb=g(P{x:1,},Q{y:2,});",
        "letc=g(P{x:1,},y,Q{y:2,});",
        "letd=g(y,P{x:1,},Q{y:2,});",
        "lete=vec!(P{x:1},Q{y:2,},R{z:3,});",
    ] {
        assert!(flat.contains(want), "missing `{want}` in:\n{out}");
    }
    // Across lines, one argument per continuation line.
    let out: String = transpile("fn f$:\n    let v =\n        vec!\n            (P\\\n                x = 1)\n            (P\\ x = 2)\n").split_whitespace().collect();
    assert!(out.contains("letv=vec!(P{x:1},P{x:2,});"), "{out}");
}

/// A literal cannot be applied (ruled 2026-09-22, A3). A literal after an
/// operator, `=`, a keyword or an opener is a head; one after a name, a
/// literal, a closer or a macro's bang is an argument, and fine.
#[test]
fn a_literal_cannot_be_applied() {
    for (src, shown) in [
        ("fn f$:\n    let b = assert_eq! a + 1 b\n", "`1 b`"),
        ("fn f$:\n    let x = 1 b\n", "`1 b`"),
        ("fn f$:\n    let s = \"s\" x\n", "`\"s\" x`"),
    ] {
        let toks = harsh_lang::lex::lex(src).expect("lex");
        let msg = harsh_lang::layout::build(toks).expect_err(src).msg;
        assert!(msg.contains("a literal cannot be applied") && msg.contains(shown), "{src:?}: {msg}");
    }
    let out = transpile("fn f$:\n    let c = assert_eq! (a + 1) b\n    let d = f 1 2\n    let e = println! \"{}\" 1\n    let g = f (x) 1 \"a\"\n    let h = 1 as u8\n");
    for want in ["assert_eq!(a + 1, b)", "f(1, 2)", "println!(\"{}\", 1)", "f(x, 1, \"a\")", "1 as u8"] {
        assert!(out.contains(want), "missing `{want}` in:\n{out}");
    }
}

/// Harsh's turbofish `.<T>` is a path step, so a turbofished function
/// applies: `parse_kv.<String> "x"` was `parse_kv::<String> "x"`, the
/// argument left behind (found running C1 of the edge sheet, 2026-09-22).
#[test]
fn a_turbofished_function_applies() {
    let out = transpile("fn f$:\n    let a = parse_kv.<String> \"x\"\n    let b = m.parse.<i32> \"x\"\n    let c = g.<T> a b\n    let d = size_of.<Vec<Vec<u8>>>$\n    let e = Vec.<i32>.with_capacity 4\n    let k = a < b && c > d\n");
    for want in [
        "parse_kv::<String>(\"x\")",
        "m::parse::<i32>(\"x\")",
        "g::<T>(a, b)",
        "size_of::<Vec<Vec<u8>>>()",
        "Vec::<i32>::with_capacity(4)",
        "a < b && c > d",
    ] {
        assert!(out.contains(want), "missing `{want}` in:\n{out}");
    }
}

/// The converter writes the holes an author would (the user's algorithm,
/// 2026-09-23): the DSL as written, every piece of Rust code in a hole, a
/// literal as it is. Found by shape, knowing no DSL -- and each shape pinned
/// here, because the round trip proves the program unchanged, not a hole's
/// place: twice a misplaced hole passed it.
#[test]
fn converter_writes_holes_in_a_dsl_body() {
    let harsh = convert("fn main() {\n    let v = view! {\n        <button\n            type=\"button\"\n            class:active=move || current.get() == i\n            aria-label=format!(\"Show review {}\", i + 1)\n            on:click=move |_| set_current.set(i)\n        ></button>\n        <p class=\"x\">{\"literal\"}</p>\n        <ul>{items.iter().map(|x| view! { <li>{x.name.clone()}</li> }).collect_view()}</ul>\n    };\n}\n");
    for want in [
        "type=\"button\"\n",
        "class:active=@: move || current <- get$ == i :@\n",
        "aria-label=@: format! \"Show review {}\" (i + 1) :@\n",
        "on:click=@: move |_| set_current <- set i :@\n",
        "<p class=\"x\">{\"literal\"}</p>",
        "view! { <li>{@: x <- name <- clone$ :@}</li> }",
    ] {
        assert!(harsh.contains(want), "\n  expected: {want:?}\n  got:\n{harsh}");
    }
    assert!(!harsh.contains("class:@:"), "a namespaced attribute read as a tree's:\n{harsh}");
    // An `if` expression inside a value is part of it, braces and all.
    let harsh = convert("fn main() {\n    let v = view! { <p class=if c { \"a\" } else { \"b\" }>\"x\"</p> };\n}\n");
    let flat: String = harsh.split_whitespace().collect();
    assert!(flat.contains("class=@:ifc:\"a\"else:\"b\":@>"), "{harsh}");
    assert!(harsh_lang::driver::transpile_str(&harsh).unwrap().contains("class=if c {"), "{harsh}");
    // What the converter cannot write as Harsh exactly stops it, named by
    // line: a chain off an `if`, which it isolates in parentheses, is such a
    // piece today.
    let err = harsh_lang::unbrace::convert("fn main() {\n    let v = view! {\n        <p class=if c { \"a\" } else { \"b\" }.to_string()>\"x\"</p>\n    };\n}\n").unwrap_err();
    assert!(err.contains("line 3") && err.contains("could not be written as Harsh"), "{err}");
}

/// `select!`-style arms, `pattern = future => handler,`: the pattern, the
/// future and the handler are Rust and become holes; `_`, `else` and a
/// literal stay as written (2026-09-23).
#[test]
fn converter_writes_holes_in_select_arms() {
    let rust = "async fn race(rx: Receiver<u8>) -> u8 {\n    tokio::select! {\n        n = slow(\"slow\") => n,\n        Some(v) = rx.recv() => {\n            println!(\"{v}\");\n            v\n        }\n        _ = sleep(Duration::from_millis(5)) => 0,\n        else => 1,\n    }\n}\n";
    let harsh = convert(rust);
    for want in [
        "n = @: slow \"slow\" :@ => @: n :@,",
        "@: Some v :@ = @: rx <- recv$ :@ => {",
        "_ = @: sleep (Duration.from_millis 5) :@ => 0,",
        "else => 1,",
    ] {
        assert!(harsh.contains(want), "\n  expected: {want:?}\n  got:\n{harsh}");
    }
    let back = transpile(&harsh);
    assert_eq!(norm_tokens(rust), norm_tokens(&back), "back:\n{back}");
}

/// A macro called with `(…)` or `[…]` whose stream is not a list of
/// expressions is a DSL: braces delimit it in Harsh, its Rust in holes, and
/// only the delimiter changes (the user's rule, 2026-09-23). A list stays
/// juxtaposed. Until then `sql!(SELECT name FROM t)` was written in isolating
/// parens and came back `sql!(SELECT(name, FROM, t))`, silently.
#[test]
fn converter_writes_a_dsl_stream_in_braces() {
    let rust = "fn main() {\n    let a = vec![1, 2, 3];\n    let b = matches!(x, Some(1) | None);\n    let d = sql!(SELECT name FROM users WHERE id = 1);\n    let e = html!(<p class=\"x\">{name}</p>);\n    let f = json!({ \"id\": 1, \"tags\": [\"a\", \"b\"] });\n    let g = json!({ \"id\": user.id, \"ok\": true });\n    let h = 1;\n}\n";
    let harsh = convert(rust);
    for want in [
        "let a = vec! 1 2 3",
        "let b = matches! x (Some 1 | None)",
        "let d = sql! {SELECT name FROM users WHERE id = 1}",
        "let e = html! {<p class=\"x\">{@: name :@}</p>}",
        "let f = json! {{ \"id\": 1, \"tags\": [\"a\", \"b\"] }}",
        "let g = json! {{ \"id\": @: user <- id :@, \"ok\": true }}",
    ] {
        assert!(harsh.contains(want), "\n  expected: {want:?}\n  got:\n{harsh}");
    }
    let back = transpile(&harsh);
    assert_eq!(norm_tokens(rust), norm_tokens(&back), "back:\n{back}");
    assert!(!back.contains("SELECT("), "{back}");
    // Nested JSON: arrays of objects are structure; the holes are the leaves
    // (sr-auto's `ai.rs`, where the whole array was first taken for one hole).
    let rust = "fn main() {\n    let body = json!({ \"parts\": [{ \"text\": prompt() }], \"n\": [1, 2] });\n}\n";
    let harsh = convert(rust);
    assert!(harsh.contains("\"parts\": [{ \"text\": @: prompt$ :@ }], \"n\": [1, 2]"), "{harsh}");
    assert_eq!(norm_tokens(rust), norm_tokens(&transpile(&harsh)));
}

/// A closure's prototype ends itself, and `:` opens its body. A return type
/// closing two generic lists at once, `Option<Vec<u8>>`, had its body isolated
/// as if it were an argument -- `(do: …)`, which transpiled to a turbofish in
/// the return type (the `>>` token, 2026-09-23).
#[test]
fn converter_writes_a_closure_body_beneath_its_prototype() {
    for rt in ["usize", "Vec<u8>", "Option<Vec<String>>", "HashMap<String, Vec<Option<u8>>>"] {
        let rust = format!("fn main() {{\n    let f = |s: &str| -> {rt} {{\n        let v = g(s);\n        v\n    }};\n}}\n");
        let harsh = convert(&rust);
        assert!(harsh.contains(&format!("-> {rt}:\n")) && !harsh.contains("(do:"), "{rt}:\n{harsh}");
        let back = transpile(&harsh);
        assert_eq!(norm_tokens(&rust), norm_tokens(&back), "{rt} back:\n{back}");
    }
}

/// A `~` macro's author expands to valid Harsh, holes included; the
/// transpiler reads an expansion exactly as it reads a file (the user's rule,
/// 2026-09-23). An expansion that produced a Rust macro's brace body is read
/// again as source -- its holes transpiled -- with the file's own brace bodies
/// and Rust zones put back first.
#[test]
fn an_expansion_is_read_as_source() {
    let out = transpile("macro_rules! double {\n    ($x:expr) => { $x * 2 };\n}\n\nmacro_rules~ filled\n    ([ ($elem:expr) ; ($n:expr) ]) => do:\n        vec! { @: $elem :@; @: $n :@ }\n\nfn main$:\n    let t = view! { <p>{@: 1 + 1 :@}</p> }\n    let z = filled~ [Vec.<u8>.new$; 2]\n    println! \"{:?} {}\" z (double! 3)\n");
    let flat: String = out.split_whitespace().collect();
    for want in ["macro_rules!double{($x:expr)=>{$x*2};}", "letz=vec!{Vec::<u8>::new();2};", "view!{<p>{1+1}</p>}", "double!(3)"] {
        assert!(flat.contains(want), "missing `{want}` in:\n{out}");
    }
}

/// Harsh's own proc macros, the registration (ruling 17, 2026-09-24): the
/// plan's `hello_macro` is ordinary Harsh with a mark, and the mark is
/// Harsh's -- it never reaches Rust. The Rust is exactly the plan's, line for
/// line (the mark's line left empty, so every line below keeps its number and
/// the map holds), and the formatter leaves the file as written.
#[test]
fn a_proc_macro_is_ordinary_harsh_with_a_mark() {
    let harsh = "use hrs_proc_macro.TokenStream\n\
                 \n\
                 #[proc_macro~]\n\
                 pub fn hello_macro (input: TokenStream) -> TokenStream:\n\
                 \x20   let input_str = input <- to_string$\n\
                 \x20   let output = format! \"\\\"Hello, {}!\\\"\" input_str\n\
                 \x20   output <- parse$ <- unwrap$\n";
    let rust = "use hrs_proc_macro::TokenStream;\n\
                \n\
                \n\
                pub fn hello_macro(input: TokenStream) -> TokenStream {\n\
                \x20   let input_str = input.to_string();\n\
                \x20   let output = format!(\"\\\"Hello, {}!\\\"\", input_str);\n\
                \x20   output.parse().unwrap()\n\
                }\n";
    assert_eq!(transpile(harsh), rust);
    assert_eq!(harsh_lang::fmt::format(harsh), harsh, "hrs fmt must leave a proc macro's mark alone");
    let regs = harsh_lang::procmac::prepare(harsh).unwrap().1;
    assert_eq!(regs.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["hello_macro"]);
}

/// Every `~` expands in Harsh and none reaches Rust (the user, 2026-09-24).
/// A derive and an attribute-like macro are both built (0.1.29, and this
/// batch): without the macro among the project's proc macros, each is an
/// error naming it -- never a `derive!` or `name!` for rustc.
#[test]
fn a_derive_or_attribute_needs_its_macro() {
    let table: &[(&str, &str, &str)] = &[
        ("derive", "#[derive~ MyMacro]\nstruct P\n    x: i32\n", "no derive macro `MyMacro`"),
        ("derive beside Rust's", "#[derive Debug]\n#[derive~ MyMacro]\nstruct P\n    x: i32\n", "no derive macro `MyMacro`"),
        ("attribute", "#[my_attr~ GET \"/\"]\nfn f$:\n    ()\n", "no attribute macro `my_attr`"),
        ("bare attribute", "#[my_attr~]\nfn f$:\n    ()\n", "no attribute macro `my_attr`"),
    ];
    for (what, src, phrase) in table {
        // The driver's path: the refusal lives before the lexer, where
        // `layout_err` does not reach.
        let msg = harsh_lang::driver::transpile_str(src).expect_err(what);
        assert!(msg.contains(phrase), "{what}: {msg}");
    }
    // Rust's own attributes are Rust's and pass through as ever.
    let out = transpile("#[derive Debug Clone]\n#[tokio.main]\nfn f$:\n    ()\n");
    assert!(out.starts_with("#[derive(Debug, Clone)]\n#[tokio::main]\n"), "{out}");
}

/// **Known converter gap, open (found 2026-09-24 by the self-host, in
/// `src/procmac.rs`).** An `if` used as a struct literal's field value, with a
/// string continued over lines (`"…\`) in one branch, comes out of `hrs-from`
/// in inline braces spanning two lines -- `m = if a {"x \` / ` y" <- into$ }
/// else {…}` -- which the transpiler refuses ("a block over several lines is
/// written with `:` or `do:`"). The same `if` in a `let` converts. Ignored so
/// that every `cargo test` reports it until it is fixed; remove the `ignore`
/// with the fix, and `procmac.rs` may then build its message in place again.
#[test]
fn converter_writes_an_if_with_a_continued_string_as_a_field_value() {
    let rust = "fn f(a: bool) -> E {\n    E { m: if a { \"x \\\n y\".into() } else { \"y\".into() } }\n}\n";
    let harsh = convert(rust);
    let back = harsh_lang::driver::transpile_str(&harsh).unwrap_or_else(|e| panic!("{e}\n--- harsh:\n{harsh}"));
    assert_eq!(norm_tokens(&back), norm_tokens(rust), "\n--- harsh:\n{harsh}");
}

/// A mark written inline before its item, as any attribute may be: blanked,
/// it left the item first on its line and over-indented, and was refused
/// (found 2026-09-24, building derives). The item now takes the mark's column.
#[test]
fn an_inline_proc_macro_mark_leaves_the_item_where_it_stood() {
    let harsh = "use hrs_proc_macro.TokenStream\n\n#[proc_macro~] pub fn hi (i: TokenStream) -> TokenStream:\n    i\n\nfn main$:\n    ()\n";
    let rust = transpile(harsh);
    assert_eq!(rust, "use hrs_proc_macro::TokenStream;\n\npub fn hi(i: TokenStream) -> TokenStream {\n    i\n}\n\nfn main() {\n    ()\n}\n");
}

/// **Fixed 2026-09-24** (`layout::decl_header` skips the line's leading
/// attributes). An attribute written inline before a `struct` whose fields
/// are beneath --
/// `#[derive Debug] struct P` / `    x: i32` -- emits `struct P\n    x {\n
/// i32,\n}`, invalid Rust; on its own line the attribute works, and inline
/// before a `fn` it works. Harsh is a superset of Rust, so the inline form
/// is owed. Ignored so every `cargo test` reports it until it is fixed.
#[test]
fn an_inline_attribute_before_a_struct_keeps_its_fields() {
    let rust = transpile("#[derive Debug] struct P\n    x: i32\n");
    assert_eq!(norm_tokens(&rust), norm_tokens("#[derive(Debug)] struct P { x: i32, }"));
}

/// **Known converter gap, open (found 2026-09-24 by the self-host, in
/// `mac::expand_derive`).** A guard arm followed by an arm whose value is a
/// `match`: `hrs-from` writes the inner `match` with `:` and its arms on one
/// line (`Some _ => match g h:` / `Some c => h = c + 1, None => break,`),
/// which the transpiler refuses. Without the guard it converts. Ignored so
/// every `cargo test` reports it until it is fixed.
#[test]
fn converter_writes_a_match_after_a_guard_arm() {
    let rust = "fn f(v: &[i32]) -> usize {\n    let mut h = 0;\n    loop {\n        match v.get(h) {\n            Some(t) if *t > 0 => h += 1,\n            Some(_) => match g(h) {\n                Some(c) => h = c + 1,\n                None => break,\n            },\n            None => return 0,\n        }\n    }\n    h\n}\nfn g(i: usize) -> Option<usize> { Some(i) }\n";
    let harsh = convert(rust);
    let back = harsh_lang::driver::transpile_str(&harsh).unwrap_or_else(|e| panic!("{e}\n--- harsh:\n{harsh}"));
    assert_eq!(norm_tokens(&back), norm_tokens(rust), "\n--- harsh:\n{harsh}");
}

/// **Known converter gap, open (found 2026-09-24 by the self-host, in
/// `driver.rs`).** An attribute between two runs of doc comments -- valid
/// Rust -- is converted so that the item beneath is over-indented, and the
/// transpiler refuses it. With the docs first and the attribute right above
/// the item, it converts. Ignored so every `cargo test` reports it until it
/// is fixed.
#[test]
fn converter_keeps_an_item_after_an_attribute_between_doc_comments() {
    let rust = "struct P;\nimpl P {\n    /// First.\n    #[inline]\n    /// Second.\n    pub fn f(&self) -> i32 {\n        1\n    }\n}\n";
    let harsh = convert(rust);
    let back = harsh_lang::driver::transpile_str(&harsh).unwrap_or_else(|e| panic!("{e}\n--- harsh:\n{harsh}"));
    assert_eq!(norm_tokens(&back), norm_tokens(rust), "\n--- harsh:\n{harsh}");
}

/// An item may be a macro call, as in Rust: at item level `compile_error!
/// "msg"` was left unapplied, `compile_error! "msg";` (found 2026-09-24,
/// when a derive emitted one). Now applied as in a block; the isolated and
/// brace forms were already right.
#[test]
fn a_macro_call_at_item_level_is_applied() {
    let rust = transpile("struct P\n    x: i32\ncompile_error! \"top-level\"\nmy_items! a b\ncompile_error! (\"isolated\")\nthread_local! { static X: u8 = 0 }\n");
    assert!(rust.contains("compile_error!(\"top-level\");"), "{rust}");
    assert!(rust.contains("my_items!(a, b);"), "{rust}");
    assert!(rust.contains("compile_error!(\"isolated\");"), "{rust}");
    assert!(rust.contains("thread_local! { static X: u8 = 0 }"), "{rust}");
}

/// A declaration's where clause stands on its header line (found
/// 2026-09-24, writing `hrs_syn`): `struct W<T> [where T: Clone]` transpiles
/// to Rust's; the function's own-line form beneath a struct's header had been
/// emitted as a field, invalid Rust, and is now refused, naming the form.
#[test]
fn a_structs_where_clause_stands_on_its_header_line() {
    let rust = transpile("struct W<T> [where T: Clone]\n    t: T\n");
    assert_eq!(norm_tokens(&rust), norm_tokens("struct W<T> where T: Clone { t: T, }"));
    let e = layout_err("struct W<T>\n    [where T: Clone]\n    t: T\n");
    assert!(e.contains("stands on its header line"), "{e}");
}

/// **Known converter gap, open (found 2026-09-24 by the self-host, in
/// `procmac.rs`).** An `if` expression as a macro argument,
/// `format!` with its arguments one per line, `if c { a } else { b }` among them: `hrs-from` writes block `if`s
/// inside the parentheses, and the `else` loses its `if`. Ignored so every
/// `cargo test` reports it until it is fixed.
#[test]
fn converter_writes_an_if_as_a_macro_argument() {
    // The arguments one per line, as rustfmt lays out a long call; on one
    // line the same call converts.
    let rust = "fn f(c: bool) -> String {\n    format!(\n        \"{} {}\",\n        if c { \"a\" } else { \"b\" },\n        if c { \"c\" } else { \"d\" }\n    )\n}\n";
    let harsh = convert(rust);
    let back = harsh_lang::driver::transpile_str(&harsh).unwrap_or_else(|e| panic!("{e}\n--- harsh:\n{harsh}"));
    assert_eq!(norm_tokens(&back), norm_tokens(rust), "\n--- harsh:\n{harsh}");
}

/// Harsh declarative macros, the matcher (the user, 2026-09-25):
/// 1. a metavariable is written `($name:spec)`, its parentheses part of the
///    construct as `$( … )*`'s are the repetition's; a bare `$name:spec` is
///    refused, in repetitions too, naming the form;
/// 2. every other token in a matcher is literal, further parentheses too;
/// 3. a specifier gives its capture meaning: `:expr` sees `(1+2)`, `((1+2))`
///    as the same expression, while `:ident` is one identifier token, as in
///    Rust, so `(adding)` is no `:ident`;
/// 4. the transcriber stays bare.
#[test]
fn a_metavariable_is_written_in_its_parentheses_and_the_rest_is_literal() {
    let pair = |matcher: &str, call: &str| -> Result<String, String> {
        let src = format!("macro_rules~ pair\n    ({matcher}) => do:\n        ($k, $v)\n\nfn main$:\n    let p = pair~ {call}\n");
        harsh_lang::driver::transpile_str(&src)
    };
    // The user's table.
    assert!(pair("($k:expr) => ($v:expr)", "\"key\" => (3+4)").is_ok(), "row 1");
    assert!(pair("(($k:expr)) => ($v:expr)", "\"key\" => (3+4)").is_err(), "row 2: the outer `( )` is literal");
    assert!(pair("(($k:expr)) => ($v:expr)", "(\"key\") => (3+4)").is_ok(), "row 3");
    assert!(pair("(($k:expr)) => ($v:expr)", "((\"key\")) => (3+4)").is_ok(), "row 4");
    // Grouping parentheses are the expression's: all three are one `:expr`.
    for v in ["(1+2)", "((1+2))", "(((1+2)))"] {
        assert!(pair("($k:expr) => ($v:expr)", &format!("1 => {v}")).is_ok(), "{v}");
    }
    // The user's three examples, with the rule as it now stands.
    let flat = |s: &str| s.split_whitespace().collect::<String>();
    let apply = |matcher: &str, call: &str| {
        let src = format!(
            "macro_rules~ apply\n    ({matcher}) => do:\n        $v $( $x )*\n\n\
             fn adding (x:i32) (y:i32) -> i32 do:\n    x + y\n\nfn main$:\n    let x = apply~ {call}\n"
        );
        harsh_lang::driver::transpile_str(&src)
    };
    let e = apply("($v:ident) $( ($x:expr) )*", "(adding) (4) (5)").expect_err("`(adding)` is no `:ident`");
    assert!(e.contains("no arm of `apply~` matches"), "{e}");
    let e = apply("$v:ident $( $x:expr )*", "adding 4 5").expect_err("bare metavariables");
    assert!(e.contains("a metavariable is written in its parentheses: `($v:ident)`"), "{e}");
    let rust = apply("($v:ident) $( ($x:expr) )*", "adding 4 5").expect("matches");
    assert!(flat(&rust).contains("letx=adding(4,5)"), "{rust}");
    let rust = apply("($v:ident) $( ($x:expr) )*", "adding (4) (5)").expect("matches");
    // `(4)` is captured as written; in Harsh, isolation parens around one
    // argument, so the Rust is the same call.
    assert!(flat(&rust).contains("letx=adding(4,5)"), "{rust}");
}

/// The converter's "`while … matches!` in a nested `match`" bug (found
/// 2026-09-10, reproduction in the handover): the inner match's last arm kept
/// its `,` and the outer arm's `},` was dropped, which the transpiler refused.
/// Fixed by 2026-09-25's brace look-back stopping at an arm's `=>`: it now
/// converts, and the Harsh transpiles back to the same Rust.
#[test]
fn a_while_matches_inside_a_nested_match_converts_and_reads_back() {
    let rust = "fn f(v: &[i32], last: usize) -> usize {\n    let opener = match v.first() {\n        Some(i) if *i > 0 => 1,\n        _ => match v.last() {\n            Some(t) if *t == 2 => {\n                let mut h = last;\n                while h > 0 && matches!(v[h - 1], 1 | 2) && v[h - 1] == v[h] {\n                    h -= 1;\n                }\n                h\n            }\n            _ => last,\n        },\n    };\n    opener + 1\n}\n";
    let harsh = convert(rust);
    let back = harsh_lang::driver::transpile_str(&harsh).unwrap_or_else(|e| panic!("{e}\n--- harsh:\n{harsh}"));
    // (`norm_tokens` treats a `,` after a block-bodied arm as the layout's,
    // meaningless: the round trip is exact.)
    assert_eq!(norm_tokens(rust), norm_tokens(&back), "\n--- harsh:\n{harsh}");
}

/// Several `if`/`else` arguments in a row, isolated, with a plain argument
/// between and a chain after a group (2026-09-25): each is the call's next
/// argument -- the tail after an `else:` branch continues the argument list,
/// as a tail after a literal did -- and `) <- len$` is still a chain.
#[test]
fn several_if_arguments_in_a_row_are_one_argument_list() {
    let rust = transpile("fn f (c: bool) -> String:\n    format!\n        \"{} {} {}\"\n        (\n            if c:\n                \"a\"\n            else:\n                \"b\"\n        )\n        7\n        (\n            if c:\n                \"c\"\n            else:\n                \"d\"\n        )\n\nfn g (xs: Vec<i32>) -> usize:\n    (\n        if true:\n            xs\n        else:\n            vec! 1\n    ) <- len$\n");
    assert_eq!(
        norm_tokens(&rust),
        norm_tokens("fn f(c: bool) -> String { format!(\"{} {} {}\", if c { \"a\" } else { \"b\" }, 7, if c { \"c\" } else { \"d\" }) } fn g(xs: Vec<i32>) -> usize { (if true { xs } else { vec!(1) }).len() }")
    );
}

/// Rust's top level holds items only, so a statement there is an error that
/// says so (the user, 2026-09-27: a top-level `let x = f 3 2` used to pass
/// through as Harsh, and the website's Converter showed broken Rust).
#[test]
fn a_statement_at_the_top_level_is_refused() {
    let refused = [
        ("let x = f 3 2\n", "a `let` at the top level"),
        ("for i in 0..3:\n    i\n", "a `for` at the top level"),
        ("while true:\n    break\n", "a `while` at the top level"),
        ("loop:\n    break\n", "a `loop` at the top level"),
        ("if true:\n    1\n", "a `if` at the top level"),
        ("match 1\\\n    _ => 2\n", "a `match` at the top level"),
        ("return\n", "a `return` at the top level"),
        ("x = 3\n", "an expression at the top level"),
        ("f 3 2\n", "an expression at the top level"),
        ("42\n", "an expression at the top level"),
    ];
    for (src, want) in refused {
        let e = harsh_lang::driver::transpile_str(src).expect_err(src);
        assert!(e.contains(want) && e.contains("belongs inside a function"), "{src:?}: {e}");
        // rustc's hint for a global, on a `let` only.
        assert_eq!(e.contains("`const` or `static`"), src.starts_with("let "), "{src:?}: {e}");
    }
}

/// Everything Rust allows at the top level stays allowed: items with their
/// attributes, inner attributes, and macro calls, which may expand to items.
#[test]
fn the_top_level_keeps_every_item() {
    let accepted = [
        "#![allow(dead_code)]\nfn main$:\n    let x = 1\n",
        "use std.collections.HashMap\n",
        "pub fn f x: i32 -> i32:\n    x\n",
        "struct P\n    x: i32\n",
        "enum E\n    A\n    B\n",
        "impl P:\n    fn new$ -> P:\n        P: x = 0\n",
        "trait T:\n    fn t$\n",
        "mod m:\n    pub fn g$:\n        ()\n",
        "const N: i32 = 3\n",
        "static S: &str = \"s\"\n",
        "type Id = u32\n",
        "unsafe fn u$:\n    ()\n",
        "async fn a$:\n    ()\n",
        "#[derive Debug]\nstruct Q\n    y: i32\n",
        "thread_local! \\\n    static C: i32 = 0\n",
    ];
    for src in accepted {
        if let Err(e) = harsh_lang::driver::transpile_str(src) {
            assert!(!e.contains("at the top level"), "{src:?} refused as a statement: {e}");
        }
    }
}

/// Wrapped in a function -- as the Jupyter kernel does with a cell, and the
/// doc-example harness with an example -- the same statements are at home.
#[test]
fn a_statement_inside_a_function_is_fine() {
    let rust = harsh_lang::driver::transpile_str("fn f x: i32 -> i32:\n    x\n\nfn __cell$:\n    let y = f 3\n").unwrap();
    assert!(rust.contains("let y = f(3);"), "{rust}");
}

/// Rust -> Harsh -> Rust through the converter and back, as the website's
/// Converter does it: the Harsh, and the Rust it gives back.
fn there_and_back(rust: &str) -> (String, String) {
    let harsh = harsh_lang::driver::convert_str(rust).unwrap_or_else(|e| panic!("convert: {e}\n{rust}"));
    let back = harsh_lang::driver::transpile_str(&harsh).unwrap_or_else(|e| panic!("transpile: {e}\n{harsh}"));
    (harsh, back)
}

/// An empty record variant stays `Home {}`: it once became `Home` over a `()`
/// line, and came back `Home { (), }` (found 2026-09-26, converting the
/// website back; fixed 2026-09-27).
#[test]
fn converter_keeps_an_empty_record_variant() {
    let (harsh, back) = there_and_back("enum Route {\n    Home {},\n    Page { id: u32 },\n}\n");
    assert!(harsh.contains("    Home {}\n"), "{harsh}");
    assert!(back.contains("Home {},") && !back.contains("()"), "{back}");
}

/// An empty struct keeps its braces on its line, and the item after it starts
/// a line of its own: it once glued on, `struct Empty { } fn g$`, and the
/// next function's body was misread (found and fixed 2026-09-27).
#[test]
fn converter_keeps_an_empty_struct_on_its_own_line() {
    let (harsh, back) = there_and_back("struct Empty {}\nfn f() -> u8 {\n    let e = Empty {};\n    3\n}\n");
    assert!(harsh.starts_with("struct Empty { }\nfn f$ -> u8:\n"), "{harsh}");
    assert!(back.contains("fn f() -> u8 {") && back.contains("let e = Empty { };"), "{back}");
}

/// Two shapes recorded as converter gaps that convert and read back today:
/// kept so, by these tests.
#[test]
fn converter_reads_a_tuple_field_after_an_index_and_an_if_as_an_argument() {
    let (harsh, back) = there_and_back("fn main() {\n    let v = vec![(1, 2), (3, 4)];\n    let x = v[0].1;\n    println!(\"{}\", x);\n}\n");
    assert!(harsh.contains("let x = v[0].1") && back.contains("let x = v[0].1;"), "{harsh}\n{back}");
    let (_, back) = there_and_back("fn main() {\n    let n = 3;\n    println!(\"{}\", if n > 2 { \"big\" } else { \"small\" });\n}\n");
    assert!(back.contains("println!(\"{}\", if n") && back.contains("\"small\""), "{back}");
}

/// A declaration's header keeps its `:` -- supertraits, a bound in `impl<T:
/// ..>` -- even with no body beneath: the marker trait and its blanket impl,
/// a Rust idiom, were misread as inline blocks until 2026-09-27.
#[test]
fn a_marker_trait_and_its_blanket_impl() {
    let rust = harsh_lang::driver::transpile_str("use std.fmt.( Debug, Display)\n\ntrait Loggable: Debug + Display + Clone\nimpl<T: Debug + Display + Clone> Loggable for T\n\ntrait Named: Debug\n    fn name (&self) -> String\n").unwrap();
    assert!(rust.contains("trait Loggable: Debug + Display + Clone {}"), "{rust}");
    assert!(rust.contains("impl<T: Debug + Display + Clone> Loggable for T {}"), "{rust}");
    assert!(rust.contains("trait Named: Debug {\n    fn name(&self) -> String;"), "{rust}");
    // An empty enum -- a type with no values -- too; a unit struct keeps `;`.
    let rust = harsh_lang::driver::transpile_str("enum Void\nstruct Unit\n").unwrap();
    assert!(rust.contains("enum Void {}") && rust.contains("struct Unit;"), "{rust}");
    // Written with Rust's braces already, it keeps one pair (the 0.1.44 rule
    // added a second, found 2026-09-28 by *By Example*'s custom error).
    let rust = harsh_lang::driver::transpile_str("impl std.error.Error for E {}\n").unwrap();
    assert!(rust.contains("impl std::error::Error for E {}") && !rust.contains("{} {}"), "{rust}");
}

/// A declaration with no body, several parameters and no return type -- a
/// trait method, a function in an `extern "C"` block -- takes Rust's comma
/// list, even when a parameter's type holds a `:` of its own (2026-09-28).
#[test]
fn a_bodiless_declaration_with_several_parameters() {
    let rust = harsh_lang::driver::transpile_str("use std.ffi.c_void\n\nextern \"C\"\n    fn qsort (base: *mut c_void) (n: usize) (compare: extern \"C\" fn (*const c_void) (*const c_void) -> i32)\n\ntrait Canvas\n    fn plot (&mut self) (x: i32) (y: i32)\n").unwrap();
    assert!(rust.contains("fn qsort(base: *mut c_void, n: usize, compare: extern \"C\" fn(*const c_void, *const c_void) -> i32);"), "{rust}");
    assert!(rust.contains("fn plot(&mut self, x: i32, y: i32);"), "{rust}");
}


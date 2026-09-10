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
    let toks = hrust::lex::lex(harsh).expect("lex");
    let tree = hrust::layout::build(toks).expect("layout");
    let mut em = hrust::emit::Emitter::new(harsh);
    em.program(&tree);
    em.out
}

fn convert(rust: &str) -> String {
    hrust::unbrace::convert(rust).expect("convert")
}

/// Token stream with whitespace, comments and optional trailing commas removed.
fn norm_tokens(s: &str) -> Vec<String> {
    let mut out: Vec<String> = hrust::lex::lex_rust(s)
        .expect("lex_rust")
        .into_iter()
        .filter(|t| !t.is_comment())
        .map(|t| t.text)
        .collect();
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
                let toks = hrust::lex::lex(&src)
                    .unwrap_or_else(|e| panic!("{}: {}", path.display(), e.msg));
                hrust::layout::build(toks)
                    .unwrap_or_else(|e| panic!("{}: {}", path.display(), e.msg));
                n += 1;
            }
        }
    }
    assert!(n > 40, "only {n} example files found");
}

#[test]
fn rejects_rust_spellings() {
    assert!(hrust::lex::lex("fn main$:\n    let x = a::b\n").is_err(), "`::` must be rejected");
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
        ("impl A:\n    fn g self -> i32:\n        1\n", "fn g(self) -> i32"),
        // A bare parameter ends at a `[where …]` clause as it does at `->`.
        ("fn g<T> x: &T\n    [where T: Clone]:\n    1\n", "fn g<T>(x: &T) where T: Clone {"),
    ]);
}

#[test]
fn block_separators() {
    check(&[
        ("struct P\n    x: i32\n    y: i32\n", "x: i32,"),
        ("enum E\n    A\n    B\n", "A,"),
        ("trait T:\n    fn a (&self) -> i32\n", "fn a(&self) -> i32;"),
        ("fn main$:\n    let x = do:\n        1\n", "let x = {"),
        ("fn main$:\n    match x:\n        A => 1\n        B => 2\n", "A => 1,"),
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
        "fn d$:\n    let z = vec![0; 4]\n    let w = vec![0; 4]\n    q z w\n",
        "fn e$:\n    if c:\n        f$;\n    g$;\n",
        "use std.fmt\nfn f$:\n    ()\n",
        "struct P\n    x: i32\n    y: i32\n",
        "fn g$ -> i32:\n    match x:\n        A => 1\n        B => 2\n",
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
    let toks = hrust::lex::lex("fn m$:\n    let a = matches! (x, Some(n) if n > 1)\n").expect("lex");
    assert!(hrust::layout::build(toks).is_err(), "the tuple spelling must be rejected");
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
        "fn g$ -> i32:\n    match x:\n        A => 1;\n        B => 2\n",
        "struct P\n    x: i32;\n    y: i32\n",
        "enum E\n    A;\n    B\n",
    ] {
        let toks = hrust::lex::lex(src).expect("lex");
        assert!(hrust::layout::build(toks).is_err(), "should reject: {src:?}");
    }
}

/// A comma block inserts its own commas, so a written one is a second
/// spelling and is rejected rather than emitted doubled.
#[test]
fn rejects_comma_in_comma_blocks() {
    for src in [
        "fn g$ -> i32:\n    match x:\n        A => 1,\n        B => 2\n",
        "fn g$ -> i32:\n    match x:\n        A => 1\n        B => 2,\n",
        "struct P\n    x: i32,\n    y: i32\n",
        "enum E\n    A,\n    B\n",
    ] {
        let toks = hrust::lex::lex(src).expect("lex");
        assert!(hrust::layout::build(toks).is_err(), "should reject: {src:?}");
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
        "trait T:\n    fn f (a: i32, b: i32) -> i32;\n",
        "impl S:\n    fn m (&self, x: i32) -> i32:\n        x\n",
    ] {
        let toks = hrust::lex::lex(src).expect("lex");
        assert!(hrust::layout::build(toks).is_err(), "should reject: {src:?}");
    }
    check(&[
        ("fn t ((a, b): (i32, i32)) -> i32:\n    a + b\n", "fn t((a, b): (i32, i32)) -> i32"),
        ("fn p (g: fn i32 i32 -> i32) (v: i32) -> i32:\n    g v v\n", "fn p(g: fn(i32, i32) -> i32, v: i32) -> i32"),
        ("impl S:\n    fn m (&self) (x: i32) -> i32:\n        x\n", "fn m(&self, x: i32) -> i32"),
    ]);
}

/// The converter writes one group per parameter, drops a trailing comma, and
/// leaves tuple patterns and `fn` pointer types alone.
#[test]
fn converter_splits_parameter_groups() {
    let got = hrust::unbrace::convert(
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
    let toks = hrust::lex::lex("use std.io:\n    Read\n    Write\n").expect("lex");
    assert!(hrust::layout::build(toks).is_err());
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
            "fn f$ -> i32:\n    let p = P\\ x = 1\n    match p:\n        P\\ x => x\n",
            "match p {",
        ),
        (
            "fn f (p: P) -> i32:\n    let q = P\\ x = 1\n    if p == q:\n        1\n    else:\n        0\n",
            "if p == q {",
        ),
        // No brace in the scrutinee: no parens added.
        ("fn f (a: i32) -> i32:\n    match a:\n        1 => 2\n", "match a {"),
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
        let toks = hrust::lex::lex(src).expect("lex");
        assert!(hrust::layout::build(toks).is_err(), "should reject: {src:?}");
    }
    // A genuine tuple argument to a macro is still expressible.
    let toks = hrust::lex::lex("fn m$:\n    dbg! ((a, b))\n").expect("lex");
    assert!(hrust::layout::build(toks).is_ok());
}

/// A colon that is not the last token opens an inline block, closed by the end
/// of the logical line, by `else`, or by a separator that is not its own. Items
/// inside carry the separator the indented form would have inserted.
#[test]
fn inline_blocks() {
    check(&[
        ("fn a (t: bool) -> i32:\n    if t: 1 else: 2\n", "if t {"),
        ("fn b (x: E) -> i32:\n    match x: A => 1, B => 2\n", "A => 1,"),
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
            "fn b (x: E) -> i32:\n    match x: A => 1, B => 2\n",
            "fn b (x: E) -> i32:\n    match x:\n        A => 1\n        B => 2\n",
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
        let toks = hrust::lex::lex(&src).expect("lex");
        let err = hrust::layout::build(toks).err().expect("must be rejected");
        assert!(err.msg.contains(needle), "{}", err.msg);
    }
    // `$` as a macro metavariable is untouched.
    let got = transpile("macro_rules! pair:\n    ($a:expr) => { $a }\n");
    assert!(got.contains("$a"), "{got}");
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

/// `macro_rules! name:` is a definition; the name is not an argument.
#[test]
fn macro_rules_definition() {
    let got = transpile("macro_rules! pair:\n    ($a:expr) => { $a }\n");
    assert!(got.contains("macro_rules! pair {"), "{got}");
    assert!(got.contains("($a:expr) => { $a };"), "{got}");
    // The arms take `;` from the newline; a written one is rejected, as a
    // written `,` is after a match arm.
    assert!(layout_err("macro_rules! pair:\n    ($a:expr) => { $a };\n").contains("macro arm"));
}

/// Mixing the inline and multi-line `else` forms orphans the second `else`,
/// because the inline block closes at the end of its line.
#[test]
fn rejects_orphaned_else() {
    let src = "fn b (n: i32) -> i32:\n    if n == 1: 10\n    else: if n == 2: 20\n    else: 30\n";
    let toks = hrust::lex::lex(src).expect("lex");
    assert!(hrust::layout::build(toks).is_err());

    // The same chain on one line, and the fully indented form, both work.
    for ok in [
        "fn b (n: i32) -> i32:\n    if n == 1: 10 else: if n == 2: 20 else: 30\n",
        "fn b (n: i32) -> i32:\n    if n == 1:\n        10\n    else if n == 2:\n        20\n    else:\n        30\n",
    ] {
        let toks = hrust::lex::lex(ok).expect("lex");
        assert!(hrust::layout::build(toks).is_ok(), "{ok:?}");
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
    let toks = hrust::lex::lex(bare).expect("lex");
    let err = hrust::layout::build(toks).err().expect("must be rejected");
    assert!(err.msg.contains("isolate the closure"), "{}", err.msg);

    // The isolated form is the fix, and a bare closure with nothing after it
    // is still fine.
    for ok in [
        "fn f (v: Vec<i32>) -> usize:\n    v <- iter$ <- map (|x|:\n        x * 2)\n      <- count$\n",
        "fn main$:\n    v <- sort_by |a, b|:\n        a <- cmp b\n    g$\n",
    ] {
        let toks = hrust::lex::lex(ok).expect("lex");
        assert!(hrust::layout::build(toks).is_ok(), "{ok:?}");
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
    let toks = hrust::lex::lex(src).expect("lex");
    match hrust::layout::build(toks) {
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
    let m = layout_err("fn f x: i32 -> i32:\n    match x: 1 => do: g$; h$, _ => 0\n");
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
        ("fn f x: i32 -> i32:\n    match x: 1 => { g$; h$ }, _ => 0\n", "1 => { g(); h() },"),
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
    let back = hrust::unbrace::convert("fn f() -> i32 { g(()) + h::<i32>() }\n").expect("convert");
    assert!(back.contains("fn f$ -> i32:"), "{back}");
    assert!(back.contains("g () + h<i32>$") || back.contains("g () + h.<i32>$") || back.contains("h<i32>$"), "{back}");
}

/// A lone parameter may be bare; with more, every one is a group.
#[test]
fn rejects_bare_parameter_followed_by_group() {
    for (src, ok) in [
        ("impl C:\n    fn top &self -> i32:\n        1\n", true),
        ("impl C:\n    fn top (&self) -> i32:\n        1\n", true),
        ("impl C:\n    fn add (&self) (k: i32) -> i32:\n        k\n", true),
        ("fn f g: fn i32:\n    g 1\n", true),
        ("impl C:\n    fn add &self (k: i32) -> i32:\n        k\n", false),
        ("impl C:\n    fn add &mut self (k: i32):\n        k\n", false),
        ("fn add a: i32 (b: i32) -> i32:\n    a\n", false),
    ] {
        let toks = hrust::lex::lex(src).expect("lex");
        let r = hrust::layout::build(toks);
        if ok {
            assert!(r.is_ok(), "{src}: {:?}", r.err());
        } else {
            let err = r.err().expect("must be rejected");
            assert!(err.msg.contains("every one is a group"), "{}", err.msg);
        }
    }
    let err = hrust::layout::build(hrust::lex::lex("impl C:\n    fn add &self (k: i32):\n        k\n").unwrap()).err().unwrap();
    assert!(err.msg.contains("`(&self) (`"), "{}", err.msg);
}

/// A macro's brace body is Harsh in one line, as every brace is: the
/// substitutions apply and so does juxtaposition. (The earlier rule that
/// nothing inside juxtaposes was withdrawn with the space rule.)
#[test]
fn macro_brace_body_is_harsh() {
    check(&[
        ("fn m$:\n    let z = m! { f a b }\n", "m! { f(a, b) }"),
        ("fn m$:\n    let y = quote! { fn #name$ -> u32 { #body } }\n", "quote! { fn #name() -> u32 { #body } }"),
        ("fn m$:\n    let y = quote! { fn #name (a: T) (b: U) -> u32 { #body } }\n", "quote! { fn #name(a: T, b: U) -> u32 { #body } }"),
        ("fn m$:\n    let s = tokio.select! { n = slow \"slow\" => n, }\n", "tokio::select! { n = slow(\"slow\") => n, }"),
    ]);
    // A name and its group are separated by a space here as everywhere.
    assert!(layout_err("fn m$:\n    let b = m! { matches!(a, b) }\n").contains("separated by a space"));
    // `!(a && b)` is negation, Rust's, kept.
    check(&[("fn m$:\n    let b = !(a && c)\n", "let b = !(a && c);")]);
}

/// Rule 2: a macro `do:` block is a Harsh block. Lines with a top-level `=>`
/// are arms and take `,`; the rest are statements; `$x` is an atom; a
/// repetition alone on its line holds statements, each with its `;`, and may
/// span lines as `$(` / body / `)*`; a value-producing transcriber is a `do:`
/// block inside the arm, as Rust's `{ { .. } }`.
#[test]
fn macro_do_block_is_harsh() {
    check(&[
        (
            "fn m$:\n    let w = tokio.select! do:\n        n = slow \"slow\" => n\n        n = fast \"fast\" => n\n    w\n",
            "let w = tokio::select! {\n        n = slow(\"slow\") => n,\n        n = fast(\"fast\") => n,\n    };",
        ),
        ("fn m$:\n    let w = tokio.select! do:\n        n = slow \"slow\" => do:\n            n\n    w\n", "n = slow(\"slow\") => {\n            n\n        },"),
        (
            "macro_rules! my_vec:\n    ( $( ($x:expr) )* ) => do:\n        do:\n            let mut v = Vec.new$\n            $(v <- push $x)*\n            v\n",
            "( $($x:expr ),* ) => {\n        {\n            let mut v = Vec::new();\n            $(v.push($x);)*\n            v\n        }\n    };",
        ),
        (
            "macro_rules! my_vec:\n    ( $( ($x:expr) )* ) => do:\n        do:\n            let mut v = Vec.new$\n            $(\n                let x = $x\n                v <- push x\n            )*\n            v\n",
            "$(\n                let x = $x;\n                v.push(x);\n            )*\n            v",
        ),
        ("macro_rules! twice:\n    ($e:expr) => do: $e * 2\n", "($e:expr) => {\n        $e * 2\n    };"),
        ("fn m$:\n    let v = my_vec! 1 2 3\n    let m = hashmap! (\"a\" => 1) (\"b\" => 2)\n", "my_vec!(1, 2, 3);\n    let m = hashmap!(\"a\" => 1, \"b\" => 2)"),
    ]);
}

/// Rules 4 and 5, and the same brick in a transcriber: a parenthesised
/// fragment is one parameter (or argument), a sequence is a comma list, a
/// repetition of them is a comma-separated repetition. Bare tokens, `tt`
/// forwarding included, are Rust's. Bracketed and braced matchers are Rust's.
#[test]
fn macro_matchers_are_parameter_groups() {
    check(&[
        ("macro_rules! m:\n    ( ($a:expr) ($b:expr) ) => do: $a\n", "($a:expr, $b:expr ) => {"),
        ("macro_rules! m:\n    ( $( ($k:expr => $v:expr) )* ) => do: 0\n", "( $($k:expr => $v:expr ),* ) => {"),
        ("macro_rules! m:\n    ( ($h:expr) $( ($t:expr) )* ) => do: $h + m! $( ($t) )*\n", "($h:expr, $($t:expr ),* ) => {\n        $h + m!($($t ),*)"),
        ("macro_rules! m:\n    ( add ($a:expr) ($b:expr) ) => do: $a\n", "( add, $a:expr, $b:expr ) => {"),
        ("macro_rules! m:\n    ( $x:expr ) => do: $x\n", "( $x:expr ) => {"),
        ("macro_rules! m:\n    () => do: 0\n", "() => {"),
        ("macro_rules! log:\n    ( ($level:expr) $($arg:tt)* ) => do:\n        println! \"[{}] {}\" $level (format! $($arg)*)\n", "($level:expr, $($arg:tt)* ) => {\n        println!(\"[{}] {}\", $level, format!($($arg)*))"),
        ("macro_rules! filled:\n    [ $elem:expr ; $n:expr ] => do:\n        vec! [$elem; $n]\n", "[ $elem:expr ; $n:expr ] => {\n        vec! [$elem; $n]\n    };"),
        ("fn m$:\n    let v = filled! [0u8; 4]\n    log! \"info\" \"x = {}\" 1\n", "filled! [0u8; 4];\n    log!(\"info\", \"x = {}\", 1)"),
    ]);
}

/// Rule 1: a macro `do:` block whose first line begins with `<` is markup,
/// copied through with the token substitutions; every `{ .. }` is Harsh, a
/// hole may span lines and holds layout, and its `}` closes on the generated
/// block's line so the line count is kept.
#[test]
fn hsx_markup_with_harsh_holes() {
    let src = "fn m$:\n    view! do:\n        <div class=\"app\">\n            <p>{count}</p>\n            <button on:click={move |_| set_count <- update (|n| *n += 1)}>\"+\"</button>\n            <button on:click={move |_|:\n                let step = 2\n                set_count <- update (|n| *n += step)\n            }>\"++\"</button>\n            <Greeting name={\"World\" <- to_string$} />\n            <a href=(\"/\") class=\"x\">\"home\"</a>\n            <leptos_router.A href=\"/\">\"home\"</leptos_router.A>\n        </div>\n";
    let got = transpile(src);
    for want in [
        "view! {\n",
        "<div class=\"app\">\n",
        "<p>{count}</p>\n",
        "<button on:click={move |_| set_count.update(|n| *n += 1)}>\"+\"</button>",
        "{move |_| {\n",
        "let step = 2;\n",
        "set_count.update(|n| *n += step)\n",
        "}}>\"++\"</button>",
        "<Greeting name={\"World\".to_string()} />",
        "<a href=\"/\" class=\"x\">\"home\"</a>",
        "<leptos_router::A href=\"/\">\"home\"</leptos_router::A>",
        "</div>\n    }\n",
    ] {
        assert!(got.contains(want), "\n  expected: {want:?}\n  got:\n{got}");
    }
    // Markup lines are not statements: a `type=` attribute on its own line is
    // not a misplaced `type` item.
    let src = "fn m$:\n    view! do:\n        <input\n            type=\"text\"\n            prop:value={name}\n        />\n";
    assert!(transpile(src).contains("type=\"text\""));
}

/// The macro rules' errors.
#[test]
fn rejects_bad_macro_spellings() {
    for (src, needle) in [
        ("macro_rules! twice:\n    ($e:expr) => $e * 2\n", "transcriber is a block"),
        ("macro_rules! pair:\n    ($a:expr) => { $a };\n", "macro arm"),
        ("macro_rules! pair:\n    ($a:expr) => { $a },\n", "macro arm"),
        ("macro_rules! m:\n    ( ($x:expr) ) => do:\n        do:\n            $(v <- push $x);*\n", "no separator"),
        ("macro_rules! m:\n    $a\n", "expected a macro arm"),
    ] {
        assert!(layout_err(src).contains(needle), "{src:?}: {}", layout_err(src));
    }
}

/// The converter side of the macro rules: `macro_rules!` bodies become the
/// `:` form with rule-4 matchers, `view! { .. }` becomes `view! do:` with
/// every attribute value and hole converted as Harsh, and both round-trip
/// back to the Rust they came from.
#[test]
fn converter_macros() {
    let rust = "macro_rules! hashmap {\n    ( $( $k:expr => $v:expr ),* ) => {\n        {\n            let mut m = HashMap::new();\n            $( m.insert($k, $v); )*\n            m\n        }\n    };\n}\n\nmacro_rules! add {\n    ($a:expr, $b:expr) => { $a + $b };\n    ($h:expr, $($t:tt)*) => { $h + add!($($t)*) };\n}\n\nmacro_rules! filled {\n    [ $elem:expr ; $n:expr ] => { vec![$elem; $n] };\n}\n\nfn main() {\n    let m = hashmap!(\"a\" => 1, \"b\" => 2);\n    let s = add!(1, 2, 3);\n    let z = filled![0u8; 4];\n}\n";
    let harsh = convert(rust);
    for want in [
        "macro_rules! hashmap:\n    ($( ($k:expr => $v:expr) )*) => do:\n        do:\n            let mut m = HashMap.new$\n            $(m <- insert $k $v)*\n            m\n",
        "macro_rules! add:\n    (($a:expr) ($b:expr)) => { $a + $b }\n    (($h:expr) $($t:tt)*) => { $h + add! $($t)* }\n",
        "macro_rules! filled:\n    [ $elem:expr ; $n:expr ] => { vec![$elem; $n] }\n",
        "hashmap! (\"a\" => 1) (\"b\" => 2)",
        "add! 1 2 3",
    ] {
        assert!(harsh.contains(want), "\n  expected: {want:?}\n  got:\n{harsh}");
    }
    let back = transpile(&harsh);
    assert!(same_tokens(rust, &back), "{}", first_diff(rust, &back));

    let rust = "fn main() {\n    view! {\n        <div class=\"app\">\n            <p>{count}</p>\n            <button on:click=move |_| set_count.update(|n| *n += 1)>\"+\"</button>\n            <Greeting name={\"World\".to_string()} />\n            <input type=\"text\" prop:value=name.get() on:input=move |ev| set_name(event_target_value(&ev)) />\n            <leptos_router::A href=\"/\">\"home\"</leptos_router::A>\n        </div>\n    }\n}\n";
    let harsh = convert(rust);
    for want in [
        "    view! do:\n        <div class=\"app\">\n            <p>{count}</p>\n",
        "on:click={move |_| set_count <- update (|n| * n += 1)}>\"+\"</button>",
        "<Greeting name={\"World\" <- to_string$} />",
        "prop:value={name <- get$} on:input={move |ev| set_name (event_target_value (&ev))} />",
        "<leptos_router.A href=\"/\">\"home\"</leptos_router.A>\n        </div>\n",
    ] {
        assert!(harsh.contains(want), "\n  expected: {want:?}\n  got:\n{harsh}");
    }
    // Back to Rust: identical but for the braces the holes gained, RSX's own
    // block-valued attribute, which rstml reads as the same value.
    let back = transpile(&harsh);
    let strip = |s: &str| s.replace("={", "=").replace("}>", ">").replace("} ", " ");
    assert!(same_tokens(&strip(rust), &strip(&back)), "{}", first_diff(&strip(rust), &strip(&back)));
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

/// Rule 6: a macro `do:` block whose first line is `name:` or `name = ..` is
/// a brace tree. `name:` opens an element, `name = value` is an attribute
/// written `name: value,` with a hole's braces dropped, a child keeps its
/// braces, `for`/`if` headers are the DSL's and are copied, and only
/// attributes (and spreads) take commas.
#[test]
fn brace_tree_macro_body() {
    let src = "fn m$:\n    rsx! do:\n        div:\n            class = \"app\"\n            onclick = {move |_| count <- set (count$ + 1)}\n            \"Hello {count}\"\n            for item in items <- iter$:\n                li: \"{item}\"\n            if count > 2:\n                p: \"many\"\n            Button:\n                onclick = {move |_|:\n                    let n = count * 2\n                    reset n\n                }\n                \"Reset\"\n            {count}\n            input:\n                type = \"text\"\n                ..attrs\n";
    let got = transpile(src);
    for want in [
        "rsx! {\n        div {\n            class: \"app\",\n            onclick: move |_| count.set(count() + 1),\n            \"Hello {count}\"\n            for item in items.iter() {\n                li { \"{item}\" }\n            }\n            if count > 2 {\n                p { \"many\" }\n            }\n            Button {\n                onclick: move |_| {\n",
        "let n = count * 2;\n",
        "reset(n)\n",
        "},\n                \"Reset\"\n            }\n            {count}\n            input {\n                type: \"text\",\n                ..attrs,\n            }\n        }\n    }\n",
    ] {
        assert!(got.contains(want), "\n  expected: {want:?}\n  got:\n{got}");
    }
}

/// Rule 6, converter side, and a lexer gap it exposed: raw identifiers.
#[test]
fn converter_brace_tree() {
    let rust = "fn main() {\n    rsx! {\n        div {\n            class: \"app\",\n            onclick: move |_| count.set(count() + 1),\n            \"Hello {count}\"\n            for item in items.iter() {\n                li { \"{item}\" }\n            }\n            Button {\n                onclick: move |_| reset(),\n                \"Reset\"\n            }\n            {count}\n            input { r#type: \"text\", ..attrs }\n        }\n    }\n}\n";
    let harsh = convert(rust);
    for want in [
        "    rsx! do:\n        div:\n            class = \"app\"\n            onclick = {move |_| count <- set (count$ + 1)}\n            \"Hello {count}\"\n            for item in items <- iter$:\n                li:\n                    \"{item}\"\n            Button:\n                onclick = {move |_| reset$}\n                \"Reset\"\n            {count}\n            input:\n                r#type = \"text\"\n                ..attrs\n",
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
        // Rule 1 in brace form, and a body that opens with a hole.
        ("fn m$:\n    let v = view! { <p class=\"x\">{count}</p> }\n    v\n", "view! { <p class=\"x\">{count}</p> }"),
        ("fn m$:\n    view! do:\n        {panel}\n        <button type=\"button\" class=\"x\">\"+\"</button>\n", "view! {\n                {panel}\n                <button type=\"button\" class=\"x\">\"+\"</button>\n    }"),
        // A comment line first in the body does not hide the markup.
        ("fn m$:\n    view! do:\n        // the sheet\n        <Stylesheet id=\"leptos\" href=\"/x\"/>\n", "// the sheet\n                <Stylesheet id=\"leptos\" href=\"/x\"/>"),
        // A markup block inside a group keeps its tail.
        ("fn m$:\n    let v = (0..3) <- map (|_| view! do:\n            <i class=\"x\"></i>\n    ) <- collect_view$\n    v\n", "}).collect_view();"),
        // A paren block inside a paren block, markup inside that.
        (
            "fn m$:\n    let dots = (len > 1) <- then (||:\n            (0..len) <- map (|i|:\n                    view! do:\n                        <button\n                            type=\"button\"\n                        ></button>\n            ) <- collect_view$\n    )\n    dots\n",
            "(0..len).map(|i| {\n            view! {\n",
        ),
        // A `):` tail opens the header's own block.
        ("fn m$:\n    if xs <- iter$ <- any (|x|:\n            *x > 1\n    ):\n        return 1\n    2\n", "if xs.iter().any(|x| {\n        *x > 1\n    }) {\n        return 1\n    }"),
        // A chain hanging off a block expression, isolated in a group.
        ("fn m$:\n    let b =\n        (\n            if c:\n                vec! [1]\n            else:\n                y <- clone$\n        ) <- into_iter$ <- count$\n    b\n", "}).into_iter().count();"),
        // A brace group is never an atom: `json! ({ .. })` is isolated.
        ("fn m$:\n    let body = json! ({\"a\": 1})\n    body\n", "json!({\"a\": 1})"),
    ]);
    // Markup lines in brace form are not statements.
    // (A `view! { .. }` over several lines is now a `do:` block; the
    // one-line brace form keeps its place.)
    let toks = hrust::lex::lex("fn m$:\n    let v = (0..3) <- map (|_| { view! { <input type=\"text\" /> } })\n").expect("lex");
    assert!(hrust::layout::build(toks).is_ok());
}

/// The converter on the same shapes.
#[test]
fn converter_leptos_shapes() {
    let rust = "#[component]\npub fn Card(\n    /// The service.\n    service: Service,\n    /// Stagger.\n    #[prop(default = 0.0)]\n    delay: f32,\n) -> impl IntoView {\n    let v = items.into_iter().map(|item| {\n        let y = item;\n        view! { <li aria-label=\"x\">{y}</li> }\n    }).collect_view();\n    let r = (extra > 0).then(|| { view! { <li>{extra}</li> } });\n    let b = if c { vec![1] } else { y.clone() }.into_iter().count();\n    {\n        let z = 1;\n        f(z);\n    }\n    leptos_routes(&opts, routes, { let o = o.clone(); move || shell(o.clone()) });\n    v\n}\n";
    let harsh = convert(rust);
    for want in [
        "pub fn Card\n    /// The service.\n    (service: Service)\n    /// Stagger.\n    (#[prop (default = 0.0)] delay: f32)\n    -> impl IntoView:\n",
        "<- map (|item|:\n            let y = item\n            view! do:\n                <li aria-label=\"x\">{y}</li>\n    ) <- collect_view$\n",
        "<- then (||:\n            view! do:\n                <li>{extra}</li>\n    )\n",
        "    let b = (\n        if c:\n            vec![1]\n        else:\n            y <- clone$\n    ) <- into_iter$ <- count$\n",
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

/// From the crate's first real build: a paren block's tail that opens a
/// markup block, and include paths re-based for the generated tree.
#[test]
fn leptos_build_findings() {
    check(&[(
        "fn m$:\n    let brand_name =\n        (\n            if c:\n                vec! [a]\n            else:\n                b\n        ) <- into_iter$ <- map (|line| view! do:\n                               <span class=\"x\">{line}</span>\n                           ) <- collect_view$\n    brand_name\n",
        "}).into_iter().map(|line| view! {\n                                       <span class=\"x\">{line}</span>\n    }).collect_view();",
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
    hrust::driver::transpile_one(&std::fs::read_to_string(&hrs).unwrap(), &hrs, &rs, &map).expect("transpile");
    let out = std::fs::read_to_string(&rs).unwrap();
    assert!(out.contains("include_str!(\"../../content/site.toml\")"), "{out}");
    assert!(out.contains("include_bytes!(\"../../src/data.bin\")"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
    // A single file written beside its source keeps its paths.
    check(&[("fn m$:\n    let s = include_str! \"../content/site.toml\"\n    s\n", "include_str!(\"../content/site.toml\")")]);
}

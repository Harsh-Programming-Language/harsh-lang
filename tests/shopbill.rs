//! What building ShopBill found (HARSH-ISSUES.md, S1-S6), and the rulings of
//! 2026-10-05 that came from it: one test each, so they stay fixed.

fn t(src: &str) -> Result<String, String> {
    harsh_lang::driver::transpile_str(src).map_err(|e| e.to_string())
}

fn flat(s: &str) -> String {
    s.split_whitespace().collect()
}

/// S1: a literal inside a tuple argument, isolated, keeps every `)`.
#[test]
fn s1_a_literal_in_a_tuple_argument() {
    let r = t("struct P\n    x: i32\nfn main$:\n    let mut v: Vec<(i32, P)> = Vec.new$\n    v <- push (1, (P\\ x = 1))\n").unwrap();
    assert!(flat(&r).contains("v.push((1,P{x:1,}))"), "{r}");
}

/// S2: a multi-line literal as an argument, isolated.
#[test]
fn s2_a_multi_line_literal_argument() {
    let r = t("struct C\n    id: u32\nfn main$:\n    let mut v: Vec<C> = Vec.new$\n    v <- push (C\\\n        id = 1\n    )\n").unwrap();
    assert!(flat(&r).contains("v.push(C{id:1,})"), "{r}");
}

/// S4: a chain broken inside a moved `let x = match` block formats, and
/// formats once.
#[test]
fn s4_a_chain_in_a_moved_block_formats() {
    let src = "async fn f (slug: &str) -> Result<(), String>:\n    let x = match labor_op\\\n        Some op => do:\n            let row: Option<(String, String)> =\n                sqlx.query_as \"SELECT 1\"\n                    <- bind (op <- clone$) <- fetch_optional (&mut *tx) <- await <- map_err text ?\n            (Some op, 1)\n        None => do:\n            (None, 2)\n    tx <- commit$ <- await <- map_err text ?\n    Ok ()\n";
    let (out, backed_out) = harsh_lang::fmt::format_checked(src);
    assert!(!backed_out);
    assert!(out.contains("                        <- fetch_optional (&mut *tx)\n"), "{out}");
    assert_eq!(harsh_lang::fmt::format(&out), out);
}

/// S5 and the constructor rule: after a constructor the files declare (or
/// `Some`, `Ok`, `Err`), a single argument needs no parentheses; after any
/// other name `&` is bit-and.
#[test]
fn s5_constructors_take_a_single_argument() {
    let r = t("fn row$ -> Option<String>: None\nfn main$:\n    let x = 1\n    let Some mut line = row$ else:\n        return\n    let a = Some &x\n    let n = 2\n    let b = n & x\n    ()\n").unwrap();
    assert!(r.contains("let Some(mut line) = row()") && r.contains("Some(&x)") && r.contains("n & x"), "{r}");
    // A constructor is one the files declare (or the prelude's), never a
    // matter of case: a function in UpperCamelCase keeps bit-and, and a
    // declared tuple struct applies.
    let r = t("struct Wrap<'a> (&'a i32)\n#[allow non_snake_case]\nfn MyFunction x: i32 -> i32: x\nfn main$:\n    let x = 1\n    let w = Wrap &x\n    let b = MyFunction & x\n    ()\n").unwrap();
    assert!(r.contains("Wrap(&x)") && r.contains("MyFunction & x"), "{r}");
}

/// S6: a record literal inside a juxtaposed `vec!` argument.
#[test]
fn s6_a_record_literal_in_vec() {
    let r = t("enum R\n    A\n    B\n        s: String\nfn main$:\n    let v = vec!\n        ((\"a\", R.A))\n        ((\"b\", R.B\\ s = String.from \"x\"))\n    ()\n").unwrap();
    assert!(flat(&r).contains("R::B{s:String::from(\"x\")}"), "{r}");
}

/// Ranges are atoms (2026-10-05): `f 0..3` applies; ends that are not atoms
/// keep the range an operator.
#[test]
fn ranges_of_atoms_are_atoms() {
    let r = t("fn f r: std.ops.Range<i32> -> i32: 1\nfn main$:\n    let n = 5\n    let a = f 0..3\n    let b = f (n + 1..9)\n    ()\n").unwrap();
    assert!(r.contains("f(0..3)") && r.contains("f(n + 1..9)"), "{r}");
}

/// Rule 1 for holes: a hole's code is Harsh, so a mark in its strings is text.
#[test]
fn a_mark_in_a_holes_string_is_text() {
    let r = t("use dioxus.prelude.*\n\nfn App$ -> Element:\n    rsx! {\n        p { {@: format! \"a:@b\" :@} }\n    }\n").unwrap();
    assert!(r.contains("format!(\"a:@b\")"), "{r}");
}

/// A bare lone parameter before `do:` keeps its `)`.
#[test]
fn a_bare_parameter_before_do() {
    let r = t("fn process & mut x: &mut i32 do:\n    ()\n").unwrap();
    assert!(r.contains("fn process(& mut x: &mut i32) {"), "{r}");
}

/// A tuple index applies to one atom: a call whose result is indexed is
/// isolated in Harsh, `(f src).0`, and written `f(src).0` in Rust -- both ways.
#[test]
fn a_tuple_index_on_a_calls_result() {
    let r = t("fn f s: &str -> (String, bool):\n    (s <- to_string$, true)\nfn main$:\n    let a = (f \"x\").0\n    ()\n").unwrap();
    assert!(r.contains("let a = f(\"x\").0;"), "{r}");
    let h = harsh_lang::driver::convert_str("fn f(s: &str) -> (String, bool) { (s.to_string(), true) }\nfn main() { let a = f(\"x\").0; }\n").unwrap();
    assert!(h.contains("(f \"x\").0"), "{h}");
}

/// `thread_local! { … }` converts: the hole covers the expression only (Rust's
/// parser reads `Cell<bool> = …` as no expression, 2026-10-06), the `}`
/// ends its line, and the next item starts a statement, not a continuation.
#[test]
fn thread_local_converts_and_returns() {
    let rust = "use std::cell::Cell;\n\nthread_local! {\n    /// A flag.\n    static FLAG: Cell<bool> = Cell::new(false);\n}\n\n/// Reads it.\npub fn read() -> bool {\n    FLAG.with(|f| f.get())\n}\n";
    let h = harsh_lang::driver::convert_str(rust).unwrap();
    assert!(h.contains("static FLAG: Cell<bool> = @: Cell.new false :@;"), "{h}");
    assert!(h.contains("}\n/// Reads it.\npub fn read$ -> bool:"), "{h}");
    assert!(t(&h).is_ok(), "{h}");
}

/// A statement after an attribute converts like any other: its calls are
/// juxtaposed, not left as a comma list (one tuple argument in Harsh).
#[test]
fn a_statement_after_an_attribute_converts() {
    let h = harsh_lang::driver::convert_str("fn g(a: i32, b: i32) -> i32 { a }\nfn f(a: i32, b: i32) {\n    #[allow(unused)]\n    let y = g(a, b);\n}\n").unwrap();
    assert!(h.contains("let y = g a b"), "{h}");
}

/// An indexed group as an argument is isolated: `show (pair s).0` would
/// index `show`'s result, so it is refused, naming `((pair s).0)`.
#[test]
fn an_indexed_group_argument_is_isolated() {
    let pre = "fn pair s: &str -> (String, usize): (s <- to_uppercase$, s <- len$)\nfn show s: String -> String: s\nfn main$:\n";
    let e = t(&format!("{pre}    let a = show (pair \"x\").0\n")).unwrap_err();
    assert!(e.contains("Isolate the whole argument: `((pair \"x\").0)`"), "{e}");
    let r = t(&format!("{pre}    let a = show ((pair \"x\").0)\n    println! \"{{}}\" ((pair \"x\").0)\n")).unwrap();
    assert!(r.contains("show(pair(\"x\").0)") && r.contains("println!(\"{}\", pair(\"x\").0)"), "{r}");
}


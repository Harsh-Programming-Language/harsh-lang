//! How the converter finds Rust in a macro's body (the user's rule,
//! 2026-10-06): a hole may start after a top-level `:` or `=`, inside a lone
//! brace pair, or in a format slot; it ends at the segment's end; it covers
//! the longest stretch that Rust's parser reads as one expression. Real macro
//! bodies, each converted, transpiled back and compared token for token.

fn flat(s: &str) -> String {
    s.lines().filter(|l| !l.starts_with("// Generated")).collect::<String>().split_whitespace().collect()
}

fn round_trip(rust: &str) -> (String, String) {
    let h = harsh_lang::driver::convert_str(rust).unwrap();
    let back = harsh_lang::driver::transpile_str(&h).unwrap();
    (h, back)
}

#[test]
fn rsx_values_become_holes() {
    let rust = "fn app() -> Element {\n    let mut n = use_signal(|| 0);\n    rsx! {\n        button { onclick: move |_| n.set(n() + 1), \"+\" }\n        p { {n() * 2} }\n    }\n}\n";
    let (h, back) = round_trip(rust);
    assert!(h.contains("onclick: @: move |_| n <- set (n$ + 1) :@,"), "{h}");
    assert_eq!(flat(&back), flat(rust), "{h}");
}

#[test]
fn thread_local_holes_cover_the_expression_only() {
    let rust = "thread_local! {\n    static A: Cell<bool> = Cell::new(false);\n    static B: RefCell<Vec<u8>> = RefCell::new(Vec::new());\n}\n";
    let (h, back) = round_trip(rust);
    assert!(h.contains("static A: Cell<bool> = @: Cell.new false :@;"), "{h}");
    assert!(h.contains("static B: RefCell<Vec<u8>> = @: RefCell.new Vec.new$ :@;"), "{h}");
    assert_eq!(flat(&back), flat(rust), "{h}");
}

#[test]
fn quick_error_clauses_stay_verbatim() {
    let rust = "quick_error! {\n    #[derive(Debug)]\n    pub enum IoWrapper {\n        Io(err: io::Error) {\n            from()\n            description(\"io error\")\n            display(\"I/O error: {}\", err)\n            cause(err)\n        }\n    }\n}\n";
    let (h, back) = round_trip(rust);
    assert!(!h.contains("@:"), "{h}");
    assert_eq!(flat(&back), flat(rust), "{h}");
}

#[test]
fn json_values_become_holes() {
    let rust = "fn f(x: i32) -> serde_json::Value {\n    json!({ \"a\": x + 1, \"list\": [1, 2, x] })\n}\n";
    let (h, back) = round_trip(rust);
    assert!(h.contains("\"a\": @: x + 1 :@,") && h.contains("\"list\": @: [1, 2, x] :@"), "{h}");
    // The call's delimiter is the one difference: a DSL takes braces in Harsh.
    assert_eq!(flat(&back).replace("json!{{", "json!({").replace("}}}", "})}"), flat(rust), "{h}");
}

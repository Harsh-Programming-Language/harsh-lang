//! Enter and Tab inside a hole whose code starts on the `@:` line (B2, the
//! user, 2026-10-04): the editor aligns with that code, as `hrs fmt` and the
//! transpiler do.

#[test]
fn enter_after_a_hole_opening_a_block_goes_one_level_in_from_its_code() {
    let src = "use dioxus.prelude.*\n\nfn App$ -> Element:\n    let n = 3\n    rsx! {\n        p {\n            {@: if n > 2:\n";
    let c = harsh_lang::columns::columns(src, 7);
    // `if` is at column 16: the body at 20, and Tab offers 16 for `else:`.
    assert_eq!(c.default, 20, "{c:?}");
    assert!(c.legal.contains(&16) && c.legal.contains(&20), "{c:?}");
}

#[test]
fn enter_after_a_hole_continuing_a_chain_stays_at_its_code() {
    let src = "fn main$:\n    rsx! {\n        p {\n            {@: v <- iter$\n";
    let c = harsh_lang::columns::columns(src, 4);
    assert_eq!(c.default, 16, "{c:?}");
}

#[test]
fn a_closed_hole_changes_nothing() {
    let src = "fn main$:\n    rsx! {\n        p { {@: n$ :@} }\n";
    let c = harsh_lang::columns::columns(src, 3);
    assert_ne!(c.default, 16, "{c:?}");
}

const HEAD: &str = "use dioxus.prelude.*\n\nfn App$ -> Element:\n    let n = 3\n    let v = vec! 1 2 3\n    rsx! {\n        p {\n";
const TAIL: &str = "        }\n    }\n";

/// The three layouts are one: each transpiles to the same Rust.
#[test]
fn the_three_hole_layouts_transpile_alike() {
    let one = format!("{HEAD}            {{@:\n                if n > 2:\n                    \"many\"\n                else:\n                    \"few\"\n            :@}}\n{TAIL}");
    let two = format!("{HEAD}            {{@: if n > 2:\n                    \"many\"\n                else:\n                    \"few\"\n            :@}}\n{TAIL}");
    let three = format!("{HEAD}            {{@: if n > 2:\n                    \"many\"\n                else:\n                    \"few\" :@}}\n{TAIL}");
    let t = |s: &str| harsh_lang::driver::transpile_str(s).map(|r| r.split_whitespace().collect::<String>());
    let (a, b, c) = (t(&one).unwrap(), t(&two).unwrap(), t(&three).unwrap());
    assert_eq!(a, b);
    assert_eq!(b, c);
    // A closure's body under `onclick: @: move |_|:` is measured from its line.
    let closure = format!("{HEAD}            button {{\n                onclick: @: move |_|:\n                    println! \"hi\"\n                :@,\n            }}\n{TAIL}");
    assert!(t(&closure).is_ok());
}

/// `hrs fmt` writes layout #3 and one space inside the marks (B7 (a), B2).
#[test]
fn the_formatter_writes_layout_three() {
    let one = format!("{HEAD}            {{@:\n                if n > 2:\n                    \"many\"\n                else:\n                    \"few\"\n            :@}}\n            {{ @:n$ + 1:@ }}\n{TAIL}");
    let f = harsh_lang::fmt::format(&one);
    assert!(f.contains("            {@: if n > 2:\n                    \"many\"\n                else:\n                    \"few\" :@}"), "{f}");
    assert!(f.contains("{@: n$ + 1 :@}"), "{f}");
    assert_eq!(harsh_lang::fmt::format(&f), f, "not idempotent");
    // A last line ending in a comment keeps `:@` on its own line.
    let c = format!("{HEAD}            {{@:\n                if n > 2:\n                    \"many\"\n                else:\n                    \"few\"  // fallback\n            :@}}\n{TAIL}");
    assert!(!harsh_lang::fmt::format(&c).contains("// fallback :@"));
}

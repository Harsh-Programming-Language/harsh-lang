// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! What writing the developer book found in 0.7.2 (its chapter 45), fixed in
//! 0.7.3: one test each.

use std::collections::HashSet;

fn t(src: &str) -> Result<String, String> {
    harsh_lang::driver::transpile_str(src).map_err(|e| e.to_string())
}

fn flat(s: &str) -> String {
    s.split_whitespace().collect()
}

/// A plain byte string has escapes: `b"\""` ends at its second quote.
#[test]
fn a_byte_string_has_escapes() {
    let r = t("fn main$:\n    let a = b\"\\\"\"\n    let n = a <- len$\n    println! \"{n}\"\n").unwrap();
    assert!(r.contains("let a = b\"\\\"\";") && r.contains("a.len()"), "{r}");
}

/// The arity scan stops at a bare parameter's function: `show`'s header no
/// longer runs on into `add`, so a pipe into `add` knows its two parameters.
#[test]
fn a_bare_parameter_ends_its_functions_arity() {
    let r = t("fn show s: String:\n    s\n\nfn add (a: i32) (b: i32) -> i32: a + b\n\nfn main$:\n    let f = 1 |> add\n    println! \"{}\" (f 2)\n").unwrap();
    assert!(flat(&r).contains("letf=move|__hrs1|add(1,__hrs1);"), "{r}");
}

/// `>>` closes two generic lists in a signature, so the groups after it are
/// joined.
#[test]
fn a_double_closer_ends_a_functions_generics() {
    let r = t("fn f<T: Into<Vec<u8>>> (a: T) (b: u8) -> u8: b\n").unwrap();
    assert!(flat(&r).contains("fnf<T:Into<Vec<u8>>>(a:T,b:u8)->u8"), "{r}");
}

/// A function with its body on its line is checked as one with its body
/// beneath: a comma list, or `()`, is refused.
#[test]
fn an_inline_body_does_not_excuse_the_parameters() {
    let e = t("fn f (a: i32, b: i32) -> i32: a\n").unwrap_err();
    assert!(e.contains("a parameter group holds one parameter"), "{e}");
    let e = t("fn g () -> i32: 1\n").unwrap_err();
    assert!(e.contains("`fn g$`"), "{e}");
    assert!(t("fn h (a: i32) (b: i32) -> i32: a + b\n").is_ok());
}

/// A `macro_rules!` with a short name stays where it was written.
#[test]
fn a_short_macro_name_stays_in_place() {
    let r = t("fn main$:\n    println! \"{}\" m!$\n\nmacro_rules! m { () => { 1 } }\n").unwrap();
    let main = r.find("fn main").unwrap();
    let def = r.find("macro_rules! m").unwrap();
    assert!(main < def, "{r}");
}

/// The tree shaker keeps an inner attribute, and the `use` after it.
#[test]
fn the_shaker_keeps_an_inner_attribute() {
    let files = vec![("lib.rs".to_string(), "#![forbid(unsafe_code)]\nuse std::fmt::Debug;\npub fn a<T: Debug>(t: T) {}\npub fn b() {}\n".to_string())];
    let roots: HashSet<String> = ["a".to_string()].into_iter().collect();
    let out = &harsh_lang::shake::shake(&files, &roots)[0].1;
    assert!(out.contains("#![forbid(unsafe_code)]") && out.contains("use std::fmt::Debug;"), "{out}");
    assert!(out.contains("pub fn a") && !out.contains("pub fn b"), "{out}");
}

/// `hrs` lists `hrs expand` and the multi-file mode in its usage.
#[test]
fn the_usage_names_every_command() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_hrs")).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(text.contains("hrs expand <file.hrs>") && text.contains("hrs [--check] <a.hrs> <b.hrs>"), "{text}");
}

/// Enter and `hrs fmt` share one anchor rule: inside `compute (1 + 1) (`
/// the group's contents go one unit past `compute`, not past the `(`.
#[test]
fn the_editor_and_the_formatter_share_the_anchor() {
    let src = "fn main$:\n    let r = compute (1 + 1) (\n";
    let c = harsh_lang::columns::columns(src, 2);
    assert_eq!(c.closer, Some(12));
    assert_eq!(c.default, 16);
}

/// The converter: a dereference and a negation are tight, a product spaced;
/// an index on a call's result is isolated, and the round trip holds.
#[test]
fn the_converter_writes_unary_operators_tight_and_isolates_an_indexed_call() {
    let rust = "fn f(v: &mut Vec<i32>, m: &mut i32, a: bool) -> i32 {\n    *m += 1;\n    let p = 2 * *m;\n    let q = !a;\n    h(v)[0] + p + v[g(1)]\n}\n";
    let harsh = harsh_lang::driver::convert_str(rust).unwrap();
    for want in ["*m += 1", "let p = 2 * *m", "let q = !a", "(h v)[0] + p + v[g 1]"] {
        assert!(harsh.contains(want), "{want}\n{harsh}");
    }
    let back = t(&harsh).unwrap();
    assert_eq!(flat(&back), flat(rust), "{back}");
}

/// A group indexed after it is applied once: `(g v)[f x]`.
#[test]
fn an_indexed_group_applies_its_index_once() {
    let r = t("fn main$:\n    let a = (g v)[f x]\n").unwrap();
    assert!(flat(&r).contains("leta=g(v)[f(x)];"), "{r}");
}

/// A file re-read from a `~` expansion keeps its map: the token right after
/// a one-digit body placeholder, `{/*Z0*/}` (8 bytes), maps to its own place
/// -- it was dropped, the placeholder taken as 9 bytes.
#[test]
fn a_token_right_after_a_short_placeholder_keeps_its_place() {
    let d = std::env::temp_dir().join(format!("harsh-findings-anchor-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let src = "macro_rules~ filled\n    ([ ($elem:expr) ; ($n:expr) ]) => do:\n        vec! { @: $elem :@; @: $n :@ }\n\nfn main$:\n    let t = (m! {x}, 1)\n    let z = filled~ [0u8; 2]\n";
    let (hrs, rs, map) = (d.join("a.hrs"), d.join("a.rs"), d.join("a.map.json"));
    std::fs::write(&hrs, src).unwrap();
    harsh_lang::driver::transpile_one(src, &hrs, &rs, &map).unwrap();
    let out = std::fs::read_to_string(&rs).unwrap();
    let m = harsh_lang::remap::SourceMap::load(map.to_str().unwrap()).unwrap();
    // The `,` right after the body, in the Rust and in the Harsh.
    let gen_at = out.find("{x},").unwrap() + "{x}".len();
    let gen_line = out[..gen_at].matches('\n').count() + 1;
    let gen_col = gen_at - out[..gen_at].rfind('\n').map_or(0, |k| k + 1) + 1;
    let src_at = src.find("{x},").unwrap() + "{x}".len();
    let src_col = src_at - src[..src_at].rfind('\n').map_or(0, |k| k + 1) + 1;
    assert_eq!(m.locate(gen_line, gen_col), Some((6, src_col)), "{out}");
}

/// A one-line DSL body followed by more on its line: the spaces that padded
/// its placeholder do not reach the Rust.
#[test]
fn a_one_line_body_leaves_no_padding_behind() {
    let r = t("fn main$:\n    let t = (view! { <p>{@: 1 + 1 :@}</p> }, 1)\n").unwrap();
    assert!(r.contains("(view! { <p>{1 + 1}</p> }, 1)"), "{r}");
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Doc examples are Harsh: `hrs` writes the Rust rustdoc compiles, `hrs-from`
//! writes the Harsh an author reads, and a fence that names another language
//! is prose and is left alone.

use hrust::emit::MapEntry;

/// Transpile as the driver does: the emitter, then the doc-example pass.
fn transpile(harsh: &str) -> String {
    let toks = hrust::lex::lex(harsh).expect("lex");
    let tree = hrust::layout::build(toks).expect("layout");
    let mut em = hrust::emit::Emitter::new(harsh);
    em.program(&tree);
    hrust::docex::to_rust_in(&mut em.out, &mut em.map, harsh).expect("doc example");
    em.out
}

/// Convert as `hrs-from` does: the converter, the formatter, then the
/// doc-example pass.
fn convert(rust: &str) -> String {
    let mut out = hrust::fmt::format(&hrust::unbrace::convert(rust).expect("convert"));
    hrust::docex::to_harsh_in(&mut out);
    out
}

const HARSH: &str = "\
/// Adds one.
///
/// ```
/// let a = 5
/// let b = my_crate.add_one a
///
/// assert_eq! 6 b
/// ```
pub fn add_one (x: i32) -> i32:
    x + 1
";

const RUST: &str = "\
/// Adds one.
///
/// ```
/// let a = 5;
/// let b = my_crate::add_one(a);
///
/// assert_eq!(6, b);
/// ```
pub fn add_one(x: i32) -> i32 {
    x + 1
}
";

#[test]
fn doc_example_is_transpiled() {
    assert_eq!(transpile(HARSH), RUST);
}

#[test]
fn doc_example_is_converted_back() {
    assert_eq!(convert(RUST), HARSH);
}

#[test]
fn doc_example_round_trips() {
    assert_eq!(transpile(&convert(RUST)), RUST);
}

/// A fence that names another language is prose: rustdoc does not compile it,
/// so neither direction touches it.
#[test]
fn a_tagged_fence_is_left_alone() {
    let harsh = "\
/// A note.
///
/// ```text
/// a::b(c)
/// ```
pub fn f$:
    ()
";
    let rust = transpile(harsh);
    assert!(rust.contains("/// a::b(c)"), "{rust}");
    let mut back = rust.clone();
    hrust::docex::to_harsh_in(&mut back);
    assert!(back.contains("/// a::b(c)"), "{back}");
}

/// rustdoc's hidden lines are compiled and not shown. The `#` is its mark,
/// not Harsh's, so it is taken off before the line is read and put back after.
#[test]
fn a_hidden_line_keeps_its_marker() {
    let harsh = "\
/// A note.
///
/// ```
/// # let a = 1
/// println! \"{}\" a
/// ```
pub fn f$:
    ()
";
    let rust = transpile(harsh);
    assert!(rust.contains("/// # let a = 1;"), "{rust}");
    assert!(rust.contains("/// println!(\"{}\", a);"), "{rust}");
}

/// The example is code, so a mistake in it is a transpile error -- pointing at
/// the line in the `.hrs`, not at the generated Rust.
#[test]
fn a_broken_example_is_an_error_on_its_own_line() {
    let harsh = "\
/// A note.
///
/// ```
/// let a = f::g
/// ```
pub fn f$:
    ()
";
    let toks = hrust::lex::lex(harsh).expect("lex");
    let tree = hrust::layout::build(toks).expect("layout");
    let mut em = hrust::emit::Emitter::new(harsh);
    em.program(&tree);
    let mut map: Vec<MapEntry> = std::mem::take(&mut em.map);
    let err = hrust::docex::to_rust_in(&mut em.out, &mut map, harsh)
        .expect_err("`::` is not Harsh");
    let line = harsh[..err.0.lo as usize].matches('\n').count() + 1;
    assert_eq!(line, 4, "the error is on the example's own line");
}

/// The example is transpiled inside a wrapper, so its last line is a statement
/// like the others: without that, its `;` would be dropped as a block's tail
/// and rustdoc's own wrapper would read it as what `main` returns.
#[test]
fn the_last_line_of_an_example_keeps_its_semicolon() {
    let harsh = "\
/// A note.
///
/// ```
/// my_crate.f$
/// ```
pub fn f$:
    ()
";
    assert!(transpile(harsh).contains("/// my_crate::f();"), "{}", transpile(harsh));
}

/// Found by `round_trip_own_source` when this module was written: an inline
/// literal inside a tuple ran to the tuple's `)` and took the other elements
/// as shorthand fields -- a valid literal, so a silent wrong program.
#[test]
fn a_literal_in_a_tuple_is_isolated() {
    let rust = "\
struct P {
    x: i32,
}

fn f(n: i32, msg: String) -> (P, String) {
    (P { x: n }, msg)
}
";
    let harsh = convert(rust);
    assert!(harsh.contains("((P\\ x = n), msg)"), "{harsh}");
    assert_eq!(transpile(&harsh).contains("P { x: n }"), false, "still a tuple");
    let back = transpile(&harsh);
    assert!(back.contains(", msg)"), "{back}");
}

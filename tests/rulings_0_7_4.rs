// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The user's rulings of 2026-10-12, built in 0.7.4: H9's long one-link
//! chains, Enter after a call still waiting for arguments, and holes in a
//! DSL string's format slots.

fn fmt(src: &str) -> String {
    harsh_lang::fmt::format(src)
}

/// H9: a one-link chain with no arguments stays on its line, whatever its
/// length -- neither broken nor moved after its `=`.
#[test]
fn a_one_link_chain_without_arguments_stays_on_its_line() {
    let src = "fn main$:\n    let label = message <- subject_line_for_privilege_log_with_redactions_applied$\n";
    assert_eq!(fmt(src), src);
}

/// H9: a one-link chain made long by long arguments moves after its `=`
/// and lists its arguments beneath the method, one unit past it -- both at
/// once; one that fits stays whole.
#[test]
fn a_one_link_chain_with_long_arguments_lists_them_beneath_the_method() {
    let src = "fn main$:\n    let label = message <- subject_line_for_privilege_log (first_custodian_name_argument) (second_matter_reference)\n";
    let want = "fn main$:\n    let label =\n        message <- subject_line_for_privilege_log\n                       (first_custodian_name_argument)\n                       (second_matter_reference)\n";
    assert_eq!(fmt(src), want);
    assert_eq!(fmt(want), want, "not idempotent");
    // Just past the width: moved and listed together, never moved alone.
    let src = "fn main$:\n    let mid = msg <- subject_line first_custodian_name_argument second_ref\n";
    let want = "fn main$:\n    let mid =\n        msg <- subject_line\n                   first_custodian_name_argument\n                   second_ref\n";
    assert_eq!(fmt(src), want);
    // It fits: one line.
    let src = "fn main$:\n    let mid = msg <- subject first_custodian_name_argument b\n";
    assert_eq!(fmt(src), src);
    // A statement: the arguments beneath, nothing to move.
    let src = "fn main$:\n    message <- send_to_reviewer_queue_for_second_pass first_custodian_name_argument second_matter_reference\n";
    let want = "fn main$:\n    message <- send_to_reviewer_queue_for_second_pass\n                   first_custodian_name_argument\n                   second_matter_reference\n";
    assert_eq!(fmt(src), want);
}

/// Enter after a call to a declared function given fewer arguments than it
/// takes lands one unit past the callee; a pipe makes the call a partial,
/// complete, and the cursor stays at the statement's column.
#[test]
fn enter_after_a_call_waiting_for_arguments() {
    let decl = "fn add (a: i32) (b: i32) -> i32: a + b\n\nfn main$:\n";
    let src = format!("{decl}    let s = add 1\n");
    let c = harsh_lang::columns::columns(&src, 4);
    assert_eq!(c.default, 16, "{c:?}");
    let src = format!("{decl}    let p = 3 |> add\n");
    let c = harsh_lang::columns::columns(&src, 4);
    assert_eq!(c.default, 4, "{c:?}");
    let src = format!("{decl}    let s = add 1 2\n");
    let c = harsh_lang::columns::columns(&src, 4);
    assert_eq!(c.default, 4, "{c:?}");
}

/// `hrs-from` writes the Rust in a DSL string's format slots as holes: a
/// plain name, a position and `{{` stay; a spec stays outside the hole; the
/// round trip is exact, strings included.
#[test]
fn the_converter_writes_format_slots_as_holes() {
    let rust = "fn main() {\n    let v = rsx! {\n        p { \"Hello {name.to_uppercase()}\" }\n        p { class: \"row {kind}\", \"{count} of {items.len():>4} {{literal}} {x:?} {0}\" }\n        p { \"{a + b}\" }\n    };\n    let w = view! { <p>\"Use {name.to_uppercase()} here\"</p> };\n}\n";
    let harsh = harsh_lang::driver::convert_str(rust).unwrap();
    for want in [
        "\"Hello {@: name <- to_uppercase$ :@}\"",
        "\"row {kind}\"",
        "\"{count} of {@: items <- len$ :@:>4} {{literal}} {x:?} {0}\"",
        "\"{@: a + b :@}\"",
        "\"Use {@: name <- to_uppercase$ :@} here\"",
    ] {
        assert!(harsh.contains(want), "{want}\n{harsh}");
    }
    let back = harsh_lang::driver::transpile_str(&harsh).unwrap();
    let strings = |s: &str| s.split('"').skip(1).step_by(2).map(String::from).collect::<Vec<_>>();
    assert_eq!(strings(&back), strings(rust), "{back}");
    // `hrs fmt` leaves a slot's spec where it is: `:@:` inside a hole closes it.
    let src = "fn main$:\n    let v = rsx! { p { \"{@: items <- len$ :@:>4}\" } }\n";
    assert_eq!(fmt(src), src);
}

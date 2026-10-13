// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Where the holes go in the DSLs of popular libraries (0.7.5): each
//! library's own Rust, in `examples/dsl/NAME.rs` -- compiled against the
//! real crate when the catalogue was written -- and the Harsh an author
//! writes for it, `NAME.hrs`, which the guide, the Book and *By Example*
//! show. Three checks each: the author's Harsh gives the library's Rust;
//! `hrs-from` converts that Rust and the round trip holds; and the converter
//! puts its holes where the library takes Rust.

use std::path::PathBuf;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/dsl")
}

fn read(name: &str) -> String {
    std::fs::read_to_string(dir().join(name)).unwrap()
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
    // A unit macro's call ending a block is one statement with or without
    // its `;` -- `{ println!(..); }` and `{ println!(..) }`: the converter
    // writes no `()` after it (2026-09-30), so the `;` does not come back.
    let mut i = 0;
    while i < out.len() {
        if out[i] == ";" && i > 0 && out[i - 1] == ")" && out.get(i + 1).map(String::as_str) == Some("}") {
            let mut d = 0i32;
            let mut j = i - 1;
            loop {
                match out[j].as_str() {
                    ")" | "]" | "}" => d += 1,
                    "(" | "[" | "{" => d -= 1,
                    _ => {}
                }
                if d == 0 || j == 0 {
                    break;
                }
                j -= 1;
            }
            if j >= 2 && out[j - 1] == "!" && harsh_lang::rules::UNIT_MACROS.contains(&out[j - 2].as_str()) {
                out.remove(i);
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Every hole's code, its whitespace collapsed, in order.
fn holes_of(harsh: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = harsh;
    while let Some(a) = rest.find("@:") {
        let after = &rest[a + 2..];
        // The matching `:@`, counting nested holes.
        let (mut d, mut i, b) = (1, 0, after.as_bytes());
        while i + 1 < b.len() {
            if &b[i..i + 2] == b"@:" {
                d += 1;
                i += 2;
                continue;
            }
            if &b[i..i + 2] == b":@" {
                d -= 1;
                if d == 0 {
                    break;
                }
                i += 2;
                continue;
            }
            i += 1;
        }
        out.push(after[..i].split_whitespace().collect::<Vec<_>>().join(" "));
        rest = &after[i.min(after.len())..];
    }
    out
}

/// The author's Harsh gives the library's Rust; `hrs-from` writes holes
/// where the library takes Rust, and its Harsh comes back the same.
fn check(name: &str, holes: &[&str]) {
    let rust = read(&format!("{name}.rs"));
    let harsh = read(&format!("{name}.hrs"));
    let from_author = harsh_lang::driver::transpile_str(&harsh).unwrap_or_else(|e| panic!("{name}.hrs: {e}"));
    assert_eq!(norm_tokens(&from_author), norm_tokens(&rust), "{name}.hrs:\n{from_author}");
    let converted = harsh_lang::driver::convert_str(&rust).unwrap_or_else(|e| panic!("{name}.rs: {e}"));
    for want in holes {
        assert!(converted.contains(want), "{name}: expected {want:?} in\n{converted}");
    }
    // The converter's holes are the author's, one for one (the binding
    // rule, 2026-10-12): the same stretches, whatever their layout.
    assert_eq!(holes_of(&converted), holes_of(&harsh), "{name}: the converter's holes differ from the author's:\n{converted}");
    let back = harsh_lang::driver::transpile_str(&converted).unwrap_or_else(|e| panic!("{name}, converted: {e}\n{converted}"));
    assert_eq!(norm_tokens(&back), norm_tokens(&rust), "{name}, converted:\n{converted}");
}

#[test]
fn serde_json() {
    check("serde_json", &[
        "\"name\": @: name <- to_uppercase$ :@,",
        "\"count\": @: langs <- len$ + 1 :@,",
        "\"main\": @: langs <- first$ :@ }]",
        "\"age\": 36,",
    ]);
}

#[test]
fn tokio_select() {
    check("tokio_select", &[
        "@: Some v :@ = @: rx <- recv$ :@ => @: v * 2 :@,",
        "_ = @: sleep (Duration.from_secs 1) :@ => @: do:",
    ]);
}

#[test]
fn thread_local() {
    check("thread_local", &[
        "static LOG: RefCell<Vec<String>> = @: RefCell.new (Vec.with_capacity 8) :@;",
    ]);
}

#[test]
fn lazy_static() {
    check("lazy_static", &[
        "static ref NAMES: HashMap<u32, &'static str> = @: do:",
        "static ref COUNT: usize = @: NAMES <- len$ * 2 :@;",
    ]);
}

#[test]
fn bitflags() {
    check("bitflags", &[
        "pub struct Perms: u32 {",
        "const READ = 0b001;",
        "const ALL = @: Self.READ <- bits$ | Self.WRITE <- bits$ :@;",
    ]);
}

#[test]
fn tracing() {
    check("tracing", &[
        "(user = %user)",
        "(?items)",
    ]);
}

#[test]
fn maud() {
    check("maud", &[
        "p.intro { \"Items: \" (@: items <- len$ :@) }",
        "a href=(@: format! \"/items/{}\" (items <- len$) :@) { \"next\" }",
        "li { (@: item <- to_uppercase$ :@) }",
        "@if @: items <- is_empty$ :@ {",
    ]);
}

#[test]
fn lists() {
    check("lists", &[
        "let v = vec! 1 2 3",
        "matches! (v <- last$) (Some n if *n > 2)",
    ]);
}

#[test]
fn leptos() {
    check("leptos", &[
        "on:click=@: move |_| set_count <- update (|n| *n += 1) :@",
        "class:big={@: move || count <- get$ > 10 :@}",
        "{@: move || count <- get$ + 1 :@}",
        "<Show when=@: move || count <- get$ % 2 == 0 :@ fallback=|| view! { <p>\"odd\"</p> }>",
    ]);
}

#[test]
fn dioxus() {
    check("dioxus", &[
        "onclick: @: move |_| count += 1 :@,",
        "disabled: @: count$ <= 0 :@,",
        "\"Twice: {@: count$ * 2 :@}\"",
        "for item in @: items <- iter$ :@ {",
        "if @: count$ > 3 :@ {",
        "class: @: if count$ > 10",
    ]);
}

#[test]
fn yew() {
    check("yew", &[
        "<div class={@: classes! \"app\" ((*counter > 5) <- then (|| \"big\")) :@}>",
        "<p>{@: format! \"Twice: {}\" (*counter * 2) :@}</p>",
        "{ for @: (0..*counter) <- map",
        "if @: *counter > 3 :@ { <p>{ \"big\" }</p> }",
    ]);
}

/// The guide to macros shows every example as it is here, Harsh and Rust:
/// a change to one without the other fails.
#[test]
fn the_guide_shows_the_examples_as_tested() {
    let guide = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/MACROS.md")).unwrap();
    for e in std::fs::read_dir(dir()).unwrap() {
        let p = e.unwrap().path();
        let text = std::fs::read_to_string(&p).unwrap();
        let block = format!("\n{}\n```", text.trim_end());
        assert!(guide.contains(&block), "docs/MACROS.md does not show {} as it is", p.display());
    }
}

/// The Book and *By Example* run the same examples as projects with the
/// real crates; their copies are these files.
#[test]
fn the_books_run_the_examples_as_tested() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let book = root.join("book/sources/src/23_advanced");
    let rbe = root.join("by-example/sources/src/07_harsh");
    // The public tree ships the books rendered, not their sources.
    if !book.is_dir() || !rbe.is_dir() {
        return;
    }
    for (copy, name) in [
        (book.join("dsl_json/src/main.hrs"), "serde_json"),
        (book.join("dsl_thread_local.hrs"), "thread_local"),
        (book.join("dsl_lazy/src/main.hrs"), "lazy_static"),
        (book.join("dsl_flags/src/main.hrs"), "bitflags"),
        (book.join("dsl_maud/src/main.hrs"), "maud"),
        (book.join("dsl_tracing/src/main.hrs"), "tracing"),
        (rbe.join("dsl_json/src/main.hrs"), "serde_json"),
        (rbe.join("dsl_maud/src/main.hrs"), "maud"),
    ] {
        assert_eq!(std::fs::read_to_string(&copy).unwrap(), read(&format!("{name}.hrs")), "{} differs from examples/dsl/{name}.hrs", copy.display());
    }
}

/// A stretch built only from a glob import's names is left as written, and
/// the conversion says so (the user's choice, 2026-10-12): Leptos's
/// `fallback=|| view! { … }`; Yew's `classes!("btn", "big")`.
#[test]
fn a_stretch_of_glob_names_is_left_with_a_warning() {
    let (_, warnings) = harsh_lang::driver::convert_str_warned(&read("leptos.rs")).unwrap();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].starts_with("line 15: `|| view! { <p>\"odd\"</p> }`"), "{warnings:?}");
    let rust = "use yew::prelude::*;\nfn badge() -> Html {\n    html! { <span class={classes!(\"btn\", \"big\")}>{ \"new\" }</span> }\n}\n";
    let (harsh, warnings) = harsh_lang::driver::convert_str_warned(rust).unwrap();
    assert!(harsh.contains("class={classes!(\"btn\", \"big\")}") && warnings.len() == 1, "{harsh}\n{warnings:?}");
    // Text the DSL owns binds nothing: no hole and no warning.
    let rust = "quick_error! {\n    pub enum E {\n        Io(err: std::io::Error) {\n            from()\n            display(\"I/O error: {}\", err)\n        }\n    }\n}\n";
    let (harsh, warnings) = harsh_lang::driver::convert_str_warned(rust).unwrap();
    assert!(!harsh.contains("@:") && warnings.is_empty(), "{harsh}\n{warnings:?}");
    // An operator is a binding: SQL's `price * 2` is a hole.
    let rust = "fn f() {\n    let rows = sql! { SELECT name FROM orders WHERE total = price * 2 };\n}\n";
    let harsh = harsh_lang::driver::convert_str(rust).unwrap();
    assert!(harsh.contains("WHERE total = @: price * 2 :@"), "{harsh}");
}

/// A single word is exempt everywhere; a name the DSL binds counts.
#[test]
fn a_single_word_is_no_hole_and_a_dsl_binding_counts() {
    let rust = "fn f(items: Vec<Item>, count: u32) {\n    let v = view! {\n        <p>{count}</p>\n        <For each=move || items.clone() key=|i| i.id let:item>\n            <li>{item.name.clone()}</li>\n        </For>\n    };\n    let r = rsx! { for row in rows.iter() { li { \"{row.title}\" } } };\n    tokio::select! { n = slow() => n, Some(v) = rx.recv() => v.len() }\n}\n";
    let harsh = harsh_lang::driver::convert_str(rust).unwrap();
    for want in ["<p>{count}</p>", "<li>{@: item <- name <- clone$ :@}</li>", "\"{@: row <- title :@}\"", "=> n,", "=> @: v <- len$ :@"] {
        assert!(harsh.contains(want), "{want}\n{harsh}");
    }
}

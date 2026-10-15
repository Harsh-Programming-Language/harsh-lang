// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! What building Docreview (BenView) found and 0.7.2 still did (its
//! HARSH-ISSUES.md, H11-H13), and what its round 75 found in 0.8.0
//! (H14-H16, fixed in 0.8.1): one test each, so they stay fixed.

use std::process::Command;

fn t(src: &str) -> String {
    harsh_lang::driver::transpile_str(src).unwrap()
}

fn flat(s: &str) -> String {
    s.split_whitespace().collect()
}

/// H11: `hrs fmt` spaces a hole's marks, and a `:@` inside one of the hole's
/// strings is text: it became `"from :@{domain}"`, a changed string.
#[test]
fn h11_fmt_leaves_a_mark_inside_a_holes_string() {
    let src = "fn main$:\n    let domain = \"a.com\"\n    rsx! { p { query: @:format! \"from:@{domain}\":@ } }\n";
    let out = harsh_lang::fmt::format(src);
    assert!(out.contains("query: @: format! \"from:@{domain}\" :@ }"), "{out}");
    assert_eq!(flat(&t(&out)), flat(&t(src)));
    // The same across lines: the hole's code read as one body.
    let src = "fn main$:\n    rsx! { p { query: @:\n        format! \"from:@{d} // :@\"\n    :@ } }\n";
    let out = harsh_lang::fmt::format(src);
    assert!(out.contains("\"from:@{d} // :@\""), "{out}");
}

/// H12: the project's top level is in `export/`, files as well as
/// folders, so `dx` run there (`hrs -- dx`) finds `Dioxus.toml` -- but never
/// the manifests `hrs` writes there itself. Since 0.8.2 (ShopBill's S7) a
/// mirror -- real folders, files hard-linked -- not symbolic links; since
/// 0.9.0 hidden files too (all but `.git`, `.hg`, `.svn` and `.hrs`).
#[test]
fn h12_top_level_files_are_linked_into_the_generated_project() {
    let d = std::env::temp_dir().join(format!("harsh-docreview-h12-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let hrs = env!("CARGO_BIN_EXE_hrs");
    assert!(Command::new(hrs).args(["new", "app"]).current_dir(&d).status().unwrap().success());
    let root = d.join("app");
    std::fs::write(root.join("Dioxus.toml"), "[application]\nname = \"app\"\n").unwrap();
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::write(root.join(".secret"), "x").unwrap();
    let out = Command::new(hrs).args(["-c", "metadata", "--no-deps", "--format-version", "1"]).current_dir(&root).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let gen = root.join("export");
    for name in ["Dioxus.toml", "assets"] {
        let m = std::fs::symlink_metadata(gen.join(name)).unwrap();
        assert!(!m.file_type().is_symlink(), "{name} is a symbolic link, not mirrored");
    }
    assert!(gen.join("assets").is_dir());
    assert_eq!(std::fs::read_to_string(gen.join("Dioxus.toml")).unwrap(), "[application]\nname = \"app\"\n");
    for name in ["Cargo.toml", "Hrs.toml"] {
        if let Ok(m) = std::fs::symlink_metadata(gen.join(name)) {
            assert!(!m.file_type().is_symlink(), "{name} must be hrs's own, not a link");
        }
    }
    assert!(gen.join(".secret").is_file(), "a hidden file was not mirrored");
    assert!(!gen.join(".hrs").exists() && !gen.join("src/main.hrs").exists());
}

/// H13: a record variant's fields are not constructors -- only the variants
/// one level under the `enum` are -- so a local named like a field keeps `&`
/// as bit-and. Variants declared inline after `\` are constructors too.
#[test]
fn h13_a_record_variants_fields_are_not_constructors() {
    let r = t("enum Details\n    Email\n        first: u8\n        messages: u32\n    Empty\n\nfn f (x: u8) -> u8:\n    let first = x\n    first & 0x7f\n");
    assert!(r.contains("first & 0x7f"), "{r}");
    let r = t("enum Shape\\ Empty, Wrap u8, (Named\\ name: u8)\n\nfn f (x: u8) -> Shape:\n    let s = Wrap &x\n    s\n");
    assert!(flat(&r).contains("Wrap(&x)"), "{r}");
    // The record variant's own name is still one.
    let r = t("enum E\n    Rec\n        a: u8\n\nfn f (x: u8) -> u8:\n    let a = x\n    a & 1\n");
    assert!(r.contains("a & 1"), "{r}");
}

/// Rule 0 and stability, for every formatter test below: the formatted
/// source transpiles to the same Rust as the original (an arm's value moved
/// under its `=>` aside, which the formatter's own check counts as the same
/// arm), and formatting it again changes nothing.
fn formats_to_the_same(src: &str) -> String {
    let (out, backed) = harsh_lang::fmt::format_checked(src);
    assert!(!backed, "left as written:\n{src}");
    let arms = |s: String| s.replace("=>{", "=>").replace("},", ",");
    assert_eq!(arms(flat(&t(&out))), arms(flat(&t(src))), "\n{out}");
    assert_eq!(harsh_lang::fmt::format(&out), out, "not stable:\n{out}");
    out
}

/// H14: a parenthesised call right after a range's `..` is applied, in an
/// index, a `let`, a `for` and an argument; it was left `..(split n)`.
#[test]
fn h14_a_call_after_a_ranges_dots_is_applied() {
    let src = "fn split (n: usize) -> usize: n / 2\n\nfn take (r: std.ops.Range<usize>) -> usize: r <- len$\n\nfn main$:\n    let v = vec! 1 2 3 4\n    let n = v <- len$\n    let a = &v[..(split n)]\n    let b = &v[(split n)..]\n    let c = &v[0..(split n)]\n    let d = ..=(split 8)\n    for i in 0..(split 8):\n        println! \"{i}\"\n    let x = take 0..(split n)\n";
    let rust = flat(&t(src));
    for want in ["&v[..(split(n))]", "&v[(split(n))..]", "&v[0..(split(n))]", "..=(split(8))", "foriin0..(split(8))", "take(0..(split(n)))"] {
        assert!(rust.contains(&flat(want)), "{want}\n{rust}");
    }
    // An argument that starts with a group is that group, as before:
    // `f (x)..` is `f(x)..`, the form `hrs-from` writes.
    assert!(flat(&t("fn main$:\n    let r = min (a - b)..\n")).contains("min(a-b).."));
}

/// H15: `&=` and `|=` are one token each; `hrs fmt` wrote `b & = 3`, and
/// its own check passed it.
#[test]
fn h15_fmt_keeps_compound_bit_assignments() {
    let src = "fn f (mut b: u32) -> u32:\n    b &= 3\n    b |= 4\n    b ^= 1\n    b &= !2\n    b\n";
    assert_eq!(harsh_lang::fmt::format(src), src);
    assert!(t(src).contains("b &= 3;") && t(src).contains("b |= 4;"));
}

/// H16: what `hrs fmt` 0.8.0 left as written in 18 of Docreview's files --
/// each formatted now, to the same Rust, stable.
#[test]
fn h16_arguments_listed_in_a_moved_block_stay_arguments() {
    // `let next = match` moves after its `=`; the listed arguments took the
    // callee's column and became statements.
    let src = "fn iso (y: i64) (m: i64) (d: i64) -> String: format! \"{y}-{m}-{d}\"\n\nfn step (mode: &'static str) (by: i64) -> String:\n    let next = match mode\\\n        \"month\" => do:\n            let index = by * 12\n            iso (index <- div_euclid 12_000_000_000) ((index <- rem_euclid 12_000_000) + 1) 1\n        _ => String.new$\n    next\n";
    let out = formats_to_the_same(src);
    assert!(out.contains("                iso\n                    (index <- div_euclid 12_000_000_000)\n"), "{out}");
}

#[test]
fn h16_an_inline_if_is_the_authors_line() {
    // A value moved after the `:` left `else:` behind it; a chain broken at
    // its `<-` mixed the condition and both values.
    let src = "fn f (s: &str) (doc: &str) -> String:\n    let title =\n        if s <- is_empty$: format! \"={}\" (doc <- trim_start_matches '=' <- to_string$) else: doc <- to_string$ <- to_uppercase$\n    title\n";
    assert_eq!(formats_to_the_same(src), src);
}

#[test]
fn h16_a_chain_in_a_group_after_a_long_receiver() {
    // `Some (PathBuf.from home` / `<- join ..`: the links went to column 0 + 4.
    let src = "use std.path.PathBuf\n\nfn t (home: String) -> Option<PathBuf>:\n    Some (PathBuf.from home <- join \".config\" <- join \"d\" <- join \"t\")\n";
    let out = formats_to_the_same(src);
    assert!(out.contains("    Some (PathBuf.from home\n        <- join \".config\"\n"), "{out}");
}

#[test]
fn h16_a_fields_value_stays_in_its_literal() {
    // `detail = line <- detail <- clone$` was broken after the field's `=`.
    let src = "fn f (line: &Line) (action: String) -> LogLine:\n    let log = LogLine\\ action = action, detail = line <- detail <- clone$, ..LogLine.default$\n    log\n";
    let out = formats_to_the_same(src);
    assert!(out.contains("detail = line <- detail <- clone$\n"), "{out}");
}

#[test]
fn h16_one_run_is_enough() {
    // Each was changed again by a second run: a chain after a call whose
    // arguments are listed; a path starting with `crate`; a literal's chain
    // measured from `let`; a closure body set deep under its `(`.
    for src in [
        "async fn f (to: String) -> Result<(), E>:\n    crate.server.mail.send_the_message_now (String.from \"a long subject for this message\") (&to) <- await <- map_err E.new ?\n    Ok ()\n",
        "async fn f (user: &User) -> bool:\n    let seat = crate.server.licence.require_ma (user <- id) <- await <- is_ok$\n    seat\n",
        "async fn f (pending: String) (code: String) -> Request:\n    let request = ActivateFinish\\ pending, code = code <- trim$ <- to_string$ <- to_owned$\n    request\n",
        "fn f (item: Option<u32>) (places: &[(u32, u64, u64)]) -> Option<u64>:\n    item <- and_then (|id|:\n        let (_, offset, length) = places <- iter$ <- find (|(item, _, _)| *item == id) ?\n        Some (*offset + *length)\n    )\n",
    ] {
        formats_to_the_same(src);
    }
}

#[test]
fn h16_a_file_of_views_takes_the_layout() {
    // A file with `rsx!` bodies kept its lines whenever the layout added
    // some: its bodies now go back where their placeholders stand.
    let src = "fn view (corpus: Corpus) -> Element:\n    let here = match corpus <- matter <- read$ <- as_ref$\\\n        Some m => format! \"{} ({})\" (m <- role <- label$) (m <- matter <- name <- clone$ <- to_uppercase$)\n        None => String.from \"no matter open\"\n    rsx! {\n        div { class: \"here\",\n            p { \"{here}\" }\n        }\n    }\n";
    let out = formats_to_the_same(src);
    assert!(out.contains("    let here =\n        match corpus"), "{out}");
    assert!(out.ends_with("    rsx! {\n        div { class: \"here\",\n            p { \"{here}\" }\n        }\n    }\n"), "{out}");
}

/// Round 76: a chain that is an operand of `||` is the author's, as any
/// operand chain is; the logical or was taken for an empty closure's bars,
/// and each operand's chain was broken in place, mid-expression. An empty
/// closure's `||` still starts its body.
#[test]
fn a_chain_in_a_logical_or_is_the_authors() {
    let src = "fn f (cs: Vec<C>) (words: String) -> Vec<C>:\n    cs <- into_iter$ <- filter (|c| words <- is_empty$ || c <- name <- to_lowercase$ <- contains (&words)) <- collect$\n";
    let out = formats_to_the_same(src);
    assert!(out.contains("|c| words <- is_empty$ || c <- name <- to_lowercase$ <- contains (&words))"), "{out}");
    let src = "fn g$ -> u32:\n    let h = std.thread.spawn (|| a <- b$ <- c$ <- d$)\n    1\n";
    let out = formats_to_the_same(src);
    assert!(out.contains("(|| a <- b$\n"), "{out}");
}

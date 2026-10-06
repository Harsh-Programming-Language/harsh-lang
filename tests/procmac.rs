// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Harsh's own proc macros end to end (ruling 17, `docs/dev/PROC-MACRO-PLAN.md`,
//! step 5 of the path): a macro crate written in Harsh, a consumer that
//! calls it, and `hrs run` / `hrs build` -- real cargo builds, as
//! `tests/remap.rs` runs `hrs` and `rustc`. The expander's behaviour is
//! pinned more finely, and without cargo, in `mac::proc_tests`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A fresh directory for one test, removed when the test is done.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("harsh-procmac-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // By its real path, which is what `hrs` reports: on macOS the
        // temporary folder `/var/folders/…` is a symlink to
        // `/private/var/folders/…`, and three tests comparing paths failed on
        // the user's Mac for it (found 2026-09-25).
        Scratch(fs::canonicalize(&dir).unwrap())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// The runtime crate in this tree, by absolute path.
fn runtime() -> String {
    format!("{}/hrs_proc_macro", env!("CARGO_MANIFEST_DIR"))
}

/// A proc-macro crate `hello` under `dir` whose `src/lib.hrs` is `lib`.
fn macro_crate(dir: &Path, lib: &str, marked: bool) -> PathBuf {
    let root = dir.join("hello");
    let meta = if marked { "\n[package.metadata.harsh]\nproc-macro = true\n" } else { "" };
    write(
        &root.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [lib]\npath = \"target/src/lib.rs\"\n\n\
             [dependencies]\nhrs_proc_macro = {{ path = \"{}\" }}\n{meta}",
            runtime()
        ),
    );
    write(&root.join("src/lib.hrs"), lib);
    root
}

/// A binary crate `app` under `dir` whose `src/main.hrs` is `main`, using
/// the macro crate beside it when `uses` is set.
fn consumer(dir: &Path, main: &str, uses: bool) -> PathBuf {
    let root = dir.join("app");
    let meta = if uses { "\n[package.metadata.harsh]\nproc-macros = [\"../hello\"]\n" } else { "" };
    write(
        &root.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [[bin]]\nname = \"app\"\npath = \"target/src/main.rs\"\n{meta}"
        ),
    );
    write(&root.join("src/main.hrs"), main);
    root
}

fn hrs(cmd: &str, dir: &Path) -> Output {
    hrs_args(&[cmd], dir)
}

fn hrs_args(args: &[&str], dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hrs"))
        .args(args)
        .current_dir(dir)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTFLAGS")
        .output()
        .expect("run hrs")
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

const HELLO: &str = "use hrs_proc_macro.TokenStream

#[proc_macro~]
pub fn hello_macro (input: TokenStream) -> TokenStream:
    let input_str = input <- to_string$
    let output = format! \"\\\"Hello, {}!\\\"\" input_str
    output <- parse$ <- unwrap$
";

/// Plan test 1: the plan's macro, written in Harsh, called as
/// `hello_macro~ world`. The generated Rust holds no macro of ours.
#[test]
fn hello_macro_prints_hello_world() {
    let s = Scratch::new("hello");
    macro_crate(&s.0, HELLO, true);
    let app = consumer(&s.0, "fn main$:\n    println! \"{}\" (hello_macro~ world)\n", true);
    let out = hrs("run", &app);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim_end(), "Hello, world!");
    let rust = fs::read_to_string(app.join("target/src/main.rs")).unwrap();
    assert_eq!(rust, "fn main() {\n    println!(\"{}\", \"Hello, world!\")\n}\n");

    // `hrs expand` finds the file's project from the file, not from where it
    // is run, and shows the proc macro's expansion (step 6).
    let file = app.join("src/main.hrs");
    let out = hrs_args(&["expand", &file.display().to_string()], &s.0);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "fn main$:\n    println! \"{}\" (\"Hello, world!\")\n");

    // Outside any project the call is left as written, and said to be.
    let lone = s.0.join("lone.hrs");
    fs::copy(&file, &lone).unwrap();
    let out = hrs_args(&["expand", &lone.display().to_string()], &s.0);
    let all = text(&out);
    assert!(all.contains("(hello_macro~ world)") && all.contains("note: `hello_macro~` is left as written"), "{all}");
}

/// Plan test 2: a macro that produces bad Harsh, or fails, is an error that
/// names the macro and the call -- whoever finds it: the lexer (`::`), the
/// macro itself (a panic), or rustc (`let x =`, which lays out, and whose
/// error is mapped back to the call).
#[test]
fn a_bad_expansion_names_the_macro_and_the_call() {
    let s = Scratch::new("bad");
    let lib = format!(
        "{HELLO}
#[proc_macro~]
pub fn dangling (_input: TokenStream) -> TokenStream:
    \"let x =\" <- parse$ <- unwrap$

#[proc_macro~]
pub fn rusty (_input: TokenStream) -> TokenStream:
    \"std::mem::drop 1\" <- parse$ <- unwrap$

#[proc_macro~]
pub fn boom (_input: TokenStream) -> TokenStream:
    panic! \"not yet\"
"
    );
    macro_crate(&s.0, &lib, true);
    // Each error's own location line, `--> …/main.hrs:3:…`, is checked: the
    // remapper's note repeats the call's line whatever the error points at,
    // so `main.hrs:3:` anywhere in the output would prove nothing (a
    // mutation that dropped the map relocation left the error at `main.hrs:5:1`,
    // past the end of the file, and a looser check still passed).
    let cases: &[(&str, &[&str])] = &[
        ("dangling~ oops", &["--> MAIN:3:", "in the expansion of `dangling~`"]),
        // Since `hrs_proc_macro` 0.2.0 a macro's own `parse` lexes, so text
        // that is not Harsh fails inside the macro -- as a Rust macro's
        // `parse().unwrap()` panics -- and is reported with the reason.
        ("rusty~ oops", &["`rusty~` failed: panicked: called `Result::unwrap()` on an `Err` value: `::` is not valid in Harsh", "--> MAIN:3:"]),
        ("boom~ oops", &["`boom~` failed: panicked: not yet", "--> MAIN:3:"]),
    ];
    for (call, wanted) in cases {
        let app = consumer(&s.0, &format!("fn main$:\n    let n = 1\n    {call}\n    println! \"{{}}\" n\n"), true);
        let out = hrs("run", &app);
        let all = text(&out);
        assert!(!out.status.success(), "{call}: should fail\n{all}");
        let main = app.join("src/main.hrs").display().to_string();
        for w in *wanted {
            let w = w.replace("MAIN", &main);
            assert!(all.contains(&w), "{call}: expected {w:?} in\n{all}");
        }
    }
}

/// Plan test 3: a crate that says it is a proc-macro crate registers at
/// least one macro; a crate that does not say so registers none; and a
/// registration lives in the crate's root.
#[test]
fn a_macro_crate_must_register_and_only_a_macro_crate_may() {
    let s = Scratch::new("refused");
    let root = macro_crate(&s.0, "pub fn plain$ -> i32:\n    1\n", true);
    let all = text(&hrs("build", &root));
    assert!(all.contains("no `pub fn` in") && all.contains("is marked `#[proc_macro~]`"), "{all}");

    let root = macro_crate(&s.0, HELLO, false);
    let all = text(&hrs("build", &root));
    assert!(all.contains("not a proc-macro crate") && all.contains("proc-macro = true"), "{all}");

    let root = macro_crate(&s.0, "pub mod more\n", true);
    write(&root.join("src/more.hrs"), HELLO);
    let all = text(&hrs("build", &root));
    assert!(all.contains("registered in the crate's root, `src/lib.hrs`") && all.contains("more.hrs"), "{all}");

    // And a consumer pointing at a crate that is not a macro crate.
    macro_crate(&s.0, "pub fn plain$ -> i32:\n    1\n", false);
    let app = consumer(&s.0, "fn main$:\n    println! \"{}\" 1\n", true);
    let all = text(&hrs("build", &app));
    assert!(all.contains("`../hello` is not a Harsh proc-macro crate"), "{all}");
}

/// Plan test 4: without `proc-macros` in its manifest, a consumer knows no
/// such macro; the call goes to rustc as `hello_macro!`, which cannot find
/// it -- today's behaviour, pinned so it does not become a silent one.
#[test]
fn a_consumer_that_does_not_list_the_crate_gets_rustcs_error() {
    let s = Scratch::new("unlisted");
    macro_crate(&s.0, HELLO, true);
    let app = consumer(&s.0, "fn main$:\n    println! \"{}\" (hello_macro~ world)\n", false);
    let out = hrs("build", &app);
    let all = text(&out);
    assert!(!out.status.success(), "{all}");
    assert!(all.contains("cannot find macro `hello_macro`"), "{all}");
}

/// A derive written in Harsh, end to end (0.1.29): `#[derive~ Describe]`
/// beside Rust's `#[derive Debug]`, its helper `#[describe skip]` read by the
/// derive and gone from the Rust, its `impl` added after the item. Mirrors
/// Rust's derive, as checked with rustc on 2026-09-24.
#[test]
fn a_derive_written_in_harsh_adds_its_impl_and_consumes_its_helper() {
    let s = Scratch::new("derive");
    let lib = "use hrs_proc_macro.TokenStream\n\n/// `impl NAME` with `describe$`, naming the fields -- all but those marked\n/// `#[describe skip]`.\n#[proc_macro_derive~ Describe (attributes describe)]\npub fn describe (input: TokenStream) -> TokenStream:\n    let text = input <- to_string$\n    let mut name = String.new$\n    let mut fields: Vec<String> = Vec.new$\n    let mut skip = false\n    for line in text <- lines$:\n        let line = line <- trim$\n        if line == \"#[describe skip]\":\n            skip = true\n        else if let Some rest = line <- strip_prefix \"struct \":\n            name = rest <- to_string$\n        else if let Some (field, _) = line <- split_once ':':\n            if !skip:\n                fields <- push (field <- to_string$)\n            skip = false\n    let listed = fields <- join \", \"\n    let out = format! \"impl {}\\n    pub fn describe$ -> &'static str:\\n        \\\"{} {{ {} }}\\\"\" name name listed\n    out <- parse$ <- unwrap$\n";
    macro_crate(&s.0, lib, true);
    let app = consumer(&s.0, "#[derive Debug]\n#[derive~ Describe]\nstruct Point\n    x: i32\n    #[describe skip]\n    secret: i32\n    y: i32\n\nfn main$:\n    let p = Point\\ x = 1, secret = 7, y = 2\n    println! \"{}\" (Point.describe$)\n    println! \"{:?}\" p\n    let total = p <- x + p <- secret + p <- y\n    println! \"{}\" total\n", true);
    let out = hrs("run", &app);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "Point { x, y }\nPoint { x: 1, secret: 7, y: 2 }\n10\n");
    let rust = fs::read_to_string(app.join("target/src/main.rs")).unwrap();
    assert!(rust.starts_with("#[derive(Debug)]\n\nstruct Point {\n    x: i32,\n\n    secret: i32,\n    y: i32,\n}\nimpl Point {\n"), "{rust}");
    assert!(!rust.contains("describe skip") && !rust.contains("derive!"), "{rust}");

    // `hrs expand` shows the item unchanged but for its helper, and the impl.
    let file = app.join("src/main.hrs");
    let shown = hrs_args(&["expand", &file.display().to_string()], &s.0);
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(shown.contains("struct Point") && shown.contains("impl Point") && !shown.contains("derive~"), "{shown}");

    // A derive that fails is reported at its attribute, naming it.
    let broken = format!(
        "{}\n#[proc_macro_derive~ Broken]\npub fn broken (_input: TokenStream) -> TokenStream:\n    panic! \"no fields\"\n\n\
         #[proc_macro_derive~ Wrong]\npub fn wrong (_input: TokenStream) -> TokenStream:\n    \"impl P\\n    pub fn n$ -> i32:\\n        \\\"text\\\"\" <- parse$ <- unwrap$\n",
        lib
    );
    macro_crate(&s.0, &broken, true);
    let app = consumer(&s.0, "#[derive~ Broken]\nstruct P\n    x: i32\n\nfn main$:\n    ()\n", true);
    let all = text(&hrs("build", &app));
    let main = app.join("src/main.hrs").display().to_string();
    assert!(all.contains("the derive `Broken` failed: panicked: no fields") && all.contains(&format!("--> {main}:1:")), "{all}");

    // rustc's error inside what a derive produced lands on the derive,
    // naming it as written.
    let app = consumer(&s.0, "#[derive~ Wrong]\nstruct P\n    x: i32\n\nfn main$:\n    ()\n", true);
    let all = text(&hrs("build", &app));
    assert!(all.contains("mismatched types") && all.contains(&format!("--> {main}:1:")), "{all}");
    assert!(all.contains("in the expansion of `#[derive~ Wrong]`"), "{all}");
}

/// Step 2 of Harsh's `syn`/`quote` (0.1.30): errors at the programmer's
/// token, by Rust's own mechanism. A token a macro copies from its input keeps
/// its origin, so rustc's error on it lands there (`keep~` returns its input:
/// the `+` at column 21, not the call at 13); and a macro rejects code by
/// emitting `compile_error!` with an input token's span (`NoBad` points at the
/// field `bad`).
#[test]
fn an_error_lands_on_the_programmers_token() {
    let s = Scratch::new("spans");
    macro_crate(&s.0, "use hrs_proc_macro.{Ident, Literal, Place, Punct, Spacing, TokenStream, TokenTree}\n\n#[proc_macro~]\npub fn keep (input: TokenStream) -> TokenStream:\n    input\n\n/// Refuses a field called `bad`, pointing at it.\n#[proc_macro_derive~ NoBad]\npub fn no_bad (input: TokenStream) -> TokenStream:\n    for t in input <- iter$:\n        if let TokenTree.Ident i = t:\n            if *i == \"bad\":\n                let span = i <- span$\n                let mut bang = TokenTree.from (Punct.new '!' Spacing.Alone)\n                bang <- set_place Place.Tight\n                let mut text = TokenTree.from (Literal.string \"a field may not be called `bad`\")\n                text <- set_span span\n                let parts = vec! (TokenTree.from (Ident.new \"compile_error\" span)) bang text\n                return parts <- into_iter$ <- collect$\n    TokenStream.new$\n", true);
    let app = consumer(&s.0, "fn main$:\n    let n: i32 = 2\n    let s = keep~ n + \"text\"\n    println! \"{}\" s\n", true);
    let all = text(&hrs("build", &app));
    let main = app.join("src/main.hrs").display().to_string();
    assert!(all.contains(&format!("--> {main}:3:21")) && all.contains("in the expansion of `keep~`"), "{all}");

    let app = consumer(&s.0, "#[derive~ NoBad]\nstruct P\n    good: i32\n    bad: i32\n\nfn main$:\n    ()\n", true);
    let all = text(&hrs("build", &app));
    assert!(all.contains("error: a field may not be called `bad`") && all.contains(&format!("--> {main}:4:5")), "{all}");
}

/// Step 3 of Harsh's `syn`/`quote`: the user's `HelloMacro` (the Rust Book's),
/// written as a Harsh derive with `quote~` -- the template laid out as the
/// output reads, `#name` interpolated -- end to end.
#[test]
fn the_hello_macro_derive_written_with_quote() {
    let s = Scratch::new("quote");
    let root = s.0.join("hello");
    write(
        &root.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"target/src/lib.rs\"\n\n\
             [dependencies]\nhrs_proc_macro = {{ path = \"{}\" }}\nhrs_quote = {{ path = \"{}/hrs_quote\" }}\n\n\
             [package.metadata.harsh]\nproc-macro = true\n",
            runtime(),
            env!("CARGO_MANIFEST_DIR")
        ),
    );
    write(&root.join("src/lib.hrs"), "use hrs_proc_macro.TokenStream\nuse hrs_quote.quote\n\n#[proc_macro_derive~ HelloMacro]\npub fn hello_macro_derive (input: TokenStream) -> TokenStream:\n    let trees: Vec<_> = input <- into_iter$ <- collect$\n    let at = trees <- iter$ <- position (|t| t <- to_string_tree$ == \"struct\") <- unwrap$\n    let name = trees[at + 1] <- clone$\n    let expanded = quote~ do:\n        impl HelloMacro for #name\n            fn hello_macro$:\n                println! \"Hello, Macro! My name is {}!\" (stringify! #name)\n    expanded\n");
    let app = consumer(&s.0, "trait HelloMacro\n    fn hello_macro$\n\n#[derive~ HelloMacro]\nstruct Pancakes\n\n#[derive~ HelloMacro]\nstruct Waffles\n\nfn main$:\n    Pancakes.hello_macro$\n    Waffles.hello_macro$\n", true);
    let out = hrs("run", &app);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "Hello, Macro! My name is Pancakes!\nHello, Macro! My name is Waffles!\n");
}

/// Step 4 of Harsh's `syn`/`quote`: repetition. A derive written in Harsh
/// repeats inline (`vec! #(#labels)*`) and as lines (one `push_str` per
/// field, the repetition beginning a line).
#[test]
fn a_derive_repeats_inline_and_as_lines() {
    let s = Scratch::new("repeat");
    let root = s.0.join("hello");
    write(
        &root.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"target/src/lib.rs\"\n\n\
             [dependencies]\nhrs_proc_macro = {{ path = \"{}\" }}\nhrs_quote = {{ path = \"{}/hrs_quote\" }}\n\n\
             [package.metadata.harsh]\nproc-macro = true\n",
            runtime(),
            env!("CARGO_MANIFEST_DIR")
        ),
    );
    write(&root.join("src/lib.hrs"), "use hrs_proc_macro.{TokenStream, TokenTree}\nuse hrs_quote.quote\n\n/// `field_names$` and `show$` for a struct, from its fields.\n#[proc_macro_derive~ Fields]\npub fn fields (input: TokenStream) -> TokenStream:\n    let trees: Vec<TokenTree> = input <- into_iter$ <- collect$\n    let at = trees <- iter$ <- position (|t| t <- to_string_tree$ == \"struct\") <- unwrap$\n    let name = trees[at + 1] <- clone$\n    // A field is a name followed by `:`.\n    let fields: Vec<TokenTree> =\n        (at + 2 .. trees <- len$ - 1)\n            <- filter (|&i| trees[i + 1] <- to_string_tree$ == \":\")\n            <- map (|i| trees[i] <- clone$)\n            <- collect$\n    let labels: Vec<String> = fields <- iter$ <- map (|f| f <- to_string_tree$) <- collect$\n    quote~ do:\n        impl #name\n            pub fn field_names$ -> Vec<&'static str>:\n                vec! #(#labels)*\n            pub fn show (&self) -> String:\n                let mut out = String.new$\n                #(out <- push_str (&format! \"{}={} \" #labels (self <- #fields)))*\n                out\n");
    let app = consumer(&s.0, "#[derive~ Fields]\nstruct Point\n    x: i32\n    y: i32\n    z: i32\n\nfn main$:\n    let p = Point\\ x = 1, y = 2, z = 3\n    println! \"{:?}\" (Point.field_names$)\n    println! \"{}\" (p <- show$)\n", true);
    let out = hrs("run", &app);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "[\"x\", \"y\", \"z\"]\nx=1 y=2 z=3 \n");
}

/// Step 5 of Harsh's `syn`/`quote`: the Rust Book's `HelloMacro`, written in
/// Harsh line for line -- `parse_macro_input!` into a `DeriveInput`,
/// `impl_hello_macro`, `split_for_impl`, `quote~` -- on a plain struct and a
/// generic one.
#[test]
fn the_rust_books_hello_macro_with_hrs_syn() {
    let s = Scratch::new("syn");
    let root = s.0.join("hello");
    write(
        &root.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"target/src/lib.rs\"\n\n\
             [dependencies]\nhrs_proc_macro = {{ path = \"{}\" }}\nhrs_quote = {{ path = \"{m}/hrs_quote\" }}\nhrs_syn = {{ path = \"{m}/hrs_syn\" }}\n\n\
             [package.metadata.harsh]\nproc-macro = true\n",
            runtime(),
            m = env!("CARGO_MANIFEST_DIR")
        ),
    );
    write(&root.join("src/lib.hrs"), "use hrs_proc_macro.TokenStream\nuse hrs_quote.quote\nuse hrs_syn.{parse_macro_input, DeriveInput}\n\n#[proc_macro_derive~ HelloMacro]\npub fn hello_macro_derive (input: TokenStream) -> TokenStream:\n    // Construct a representation of Harsh code as a syntax tree\n    // that we can manipulate.\n    let ast = parse_macro_input! { input as DeriveInput }\n\n    // Build the trait implementation.\n    impl_hello_macro (&ast)\n\nfn impl_hello_macro (ast: &DeriveInput) -> TokenStream:\n    let name = &ast <- ident\n    let (impl_generics, ty_generics, where_clause) = ast <- generics <- split_for_impl$\n    let generated = quote~ do:\n        impl#impl_generics HelloMacro for #name#ty_generics #where_clause\n            fn hello_macro$:\n                println! \"Hello, Macro! My name is {}!\" (stringify! #name)\n    generated\n");
    let app = consumer(&s.0, "trait HelloMacro\n    fn hello_macro$\n\n#[derive~ HelloMacro]\nstruct Pancakes\n\n#[derive~ HelloMacro]\nstruct Wrapper<T: Clone>\n    value: T\n\nfn main$:\n    Pancakes.hello_macro$\n    Wrapper.<i32>.hello_macro$\n    let w = Wrapper\\ value = 1\n    println! \"{}\" (w <- value)\n", true);
    let out = hrs("run", &app);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "Hello, Macro! My name is Pancakes!\nHello, Macro! My name is Wrapper!\n1\n");
    let rust = fs::read_to_string(app.join("target/src/main.rs")).unwrap();
    assert!(rust.contains("impl<T: Clone> HelloMacro for Wrapper<T> {"), "{rust}");
}

/// Harsh's attribute-like macros: the user's `route` example (2026-09-24),
/// `#[route~ POST "/api/v1/submit"]` on a function -- the macro receives the
/// arguments as written and the item, parses the function with `hrs_syn`'s
/// `ItemFn`, and its output replaces the function.
#[test]
fn an_attribute_macro_written_in_harsh_replaces_its_function() {
    let s = Scratch::new("attribute");
    let root = s.0.join("hello");
    write(
        &root.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"target/src/lib.rs\"\n\n\
             [dependencies]\nhrs_proc_macro = {{ path = \"{}\" }}\nhrs_quote = {{ path = \"{m}/hrs_quote\" }}\nhrs_syn = {{ path = \"{m}/hrs_syn\" }}\n\n\
             [package.metadata.harsh]\nproc-macro = true\n",
            runtime(),
            m = env!("CARGO_MANIFEST_DIR")
        ),
    );
    write(&root.join("src/lib.hrs"), "use hrs_proc_macro.TokenStream\nuse hrs_quote.quote\nuse hrs_syn.{parse_macro_input, ItemFn}\n\n#[proc_macro_attribute~]\npub fn route (attr: TokenStream) (item: TokenStream) -> TokenStream:\n    let route = attr <- to_string$\n    let f = parse_macro_input! { item as ItemFn }\n    let vis = &f <- vis\n    let sig = &f <- sig\n    let name = f <- ident <- to_string$\n    let body = &f <- block\n    quote~ do:\n        #vis #sig:\n            println! \"[ROUTE LOG] Dispatched handler '{}' for {}\" #name #route\n            #body\n");
    let app = consumer(&s.0, "#[route~ POST \"/api/v1/submit\"]\nfn handle_submit$:\n    println! \"Processing payload...\"\n\nfn main$:\n    handle_submit$\n", true);
    let out = hrs("run", &app);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "[ROUTE LOG] Dispatched handler 'handle_submit' for POST \"/api/v1/submit\"\nProcessing payload...\n"
    );
    let rust = fs::read_to_string(app.join("target/src/main.rs")).unwrap();
    assert!(!rust.contains("route"), "{rust}");
}

/// A different `hrs` retranspiles a project once (found 2026-09-24: after an
/// upgrade, a project kept the old version's Rust until its sources
/// changed). `target/src/.hrs-stamp` records which `hrs` last transpiled it;
/// a stamp from another makes every file stale once.
#[test]
fn a_different_hrs_retranspiles_once() {
    let s = Scratch::new("stamp");
    let app = consumer(&s.0, "fn main$:\n    println! \"hi\"\n", false);
    // A source written in the same clock tick as its output counts as stale
    // (`>=`, the safe side); a person's edit and build are further apart.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let first = |o: &Output| text(o).lines().find(|l| l.starts_with("hrs:")).unwrap_or("").to_string();
    assert!(first(&hrs("build", &app)).contains("transpiled 1 of 1"));
    assert!(first(&hrs("build", &app)).contains("up to date"));
    fs::write(app.join("target/src/.hrs-stamp"), "0.0.1 0").unwrap();
    assert!(first(&hrs("build", &app)).contains("transpiled 1 of 1"));
    assert!(first(&hrs("build", &app)).contains("up to date"));
}

/// `hrs export` is pure Rust (the user, 2026-09-25): no
/// `[package.metadata.harsh]` in the exported manifest, and a Harsh macro
/// crate is refused -- it has no Rust meaning.
#[test]
fn export_is_pure_rust() {
    let s = Scratch::new("export");
    macro_crate(&s.0, HELLO, true);
    let app = consumer(&s.0, "fn main$:\n    println! \"{}\" (hello_macro~ world)\n", true);
    let out_dir = s.0.join("exported");
    let out = hrs_args(&["export", &out_dir.display().to_string()], &app);
    assert!(out.status.success(), "{}", text(&out));
    let manifest = fs::read_to_string(out_dir.join("Cargo.toml")).unwrap();
    assert!(!manifest.contains("harsh"), "{manifest}");
    let rust = fs::read_to_string(out_dir.join("src/main.rs")).unwrap();
    assert!(rust.contains("\"Hello, world!\"") && !rust.contains("hello_macro"), "{rust}");
    let refused = hrs_args(&["export", &s.0.join("x").display().to_string()], &s.0.join("hello"));
    assert!(!refused.status.success() && text(&refused).contains("a Harsh macro crate is not exported"), "{}", text(&refused));
}

/// The standard distribution in `src/dist.rs` is the four crates as they
/// are in the tree (`docs/dev/gen-dist.py` writes it).
#[test]
fn the_distribution_is_current() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (path, text) in harsh_lang::dist::FILES {
        let on_disk = fs::read_to_string(root.join(path)).unwrap_or_default();
        assert!(*text == on_disk, "{path} differs from src/dist.rs: run python3 docs/dev/gen-dist.py");
    }
    for c in harsh_lang::driver::DISTRIBUTION {
        for f in fs::read_dir(root.join(c).join("src")).unwrap() {
            let p = format!("{c}/src/{}", f.unwrap().file_name().to_string_lossy());
            assert!(harsh_lang::dist::FILES.iter().any(|(q, _)| *q == p), "{p} is not in src/dist.rs: run python3 docs/dev/gen-dist.py");
        }
    }
}

/// Harsh's standard distribution ships inside `hrs` (the user, 2026-09-25):
/// a macro crate asks for `hrs_proc_macro = "0.2"`, `hrs_quote = "0.1"`,
/// `hrs_syn = "0.1"` and an app for `hrs_std = "0.1"`, by version, as Rust
/// asks for `syn` and `quote` -- none of them on crates.io, all served by
/// what `hrs` carries. (The user's first test of 0.1.30 failed here.)
#[test]
fn the_standard_distribution_needs_no_registry() {
    let s = Scratch::new("dist");
    let root = s.0.join("hello");
    write(
        &root.join("Cargo.toml"),
        "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"target/src/lib.rs\"\n\n\
         [dependencies]\nhrs_proc_macro = \"0.2\"\nhrs_quote = \"0.1\"\nhrs_syn = \"0.1\"\n\n[package.metadata.harsh]\nproc-macro = true\n",
    );
    write(&root.join("src/lib.hrs"), "use hrs_proc_macro.TokenStream\nuse hrs_quote.quote\nuse hrs_syn.{parse_macro_input, DeriveInput}\n\n#[proc_macro_derive~ HelloMacro]\npub fn hello_macro_derive (input: TokenStream) -> TokenStream:\n    // Construct a representation of Harsh code as a syntax tree\n    // that we can manipulate.\n    let ast = parse_macro_input! { input as DeriveInput }\n\n    // Build the trait implementation.\n    impl_hello_macro (&ast)\n\nfn impl_hello_macro (ast: &DeriveInput) -> TokenStream:\n    let name = &ast <- ident\n    let generated = quote~ do:\n        impl HelloMacro for #name\n            fn hello_macro$:\n                println! \"Hello, Macro! My name is {}!\" (stringify! #name)\n    generated\n");
    let app = consumer(&s.0, "trait HelloMacro\n    fn hello_macro$\n\n#[derive~ HelloMacro]\nstruct Pancakes\n\nfn main$:\n    Pancakes.hello_macro$\n    let a = m~ [1.0_f64 2.0; 3.0 4.0]\n    println! \"{:?}\" (a <- size$)\n", true);
    let manifest = fs::read_to_string(app.join("Cargo.toml")).unwrap();
    // 0.3.0: Harsh's dependencies are in Hrs.toml, beside Cargo.toml.
    fs::write(app.join("Cargo.toml"), &manifest).unwrap();
    fs::write(app.join("Hrs.toml"), "[dependencies]\nhrs_std = \"0.1\"\n").unwrap();
    let out = hrs("run", &app);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "Hello, Macro! My name is Pancakes!\n(2, 2)\n");
    assert!(!text(&out).contains("was not used in the crate graph"), "{}", text(&out));
}

/// A version the standard distribution does not satisfy is named plainly --
/// cargo's own answer sends the reader to crates.io (found 2026-09-25: the
/// Book's `proc_hello` still asked for `hrs_proc_macro = "0.1"`).
#[test]
fn a_version_the_distribution_cannot_serve_says_what_it_ships() {
    let s = Scratch::new("oldver");
    let root = s.0.join("hello");
    write(
        &root.join("Cargo.toml"),
        "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"target/src/lib.rs\"\n\n\
         [dependencies]\nhrs_proc_macro = \"0.1\"\n\n[package.metadata.harsh]\nproc-macro = true\n",
    );
    write(&root.join("src/lib.hrs"), HELLO);
    let app = consumer(&s.0, "fn main$:\n    println! \"{}\" (hello_macro~ world)\n", true);
    let out = hrs("run", &app);
    assert!(!out.status.success());
    let t = text(&out);
    assert!(t.contains("this hrs ships hrs_proc_macro 0.2.0") && t.contains("ask for \"0.2\""), "{t}");
    assert!(!t.contains("crates.io index"), "{t}");
}

/// A Harsh app using a Harsh library by path (found 2026-09-25: it never
/// worked -- `hrs` did not transpile the library, though the Book's chapter
/// 17 said it did): the library is transpiled first, as a dependency.
#[test]
fn a_harsh_app_uses_a_harsh_library_by_path() {
    let s = Scratch::new("pathdep");
    write(&s.0.join("add_one/Cargo.toml"), "[package]\nname = \"add_one\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"target/src/lib.rs\"\n");
    write(&s.0.join("add_one/src/lib.hrs"), "pub fn add_one (x: i32) -> i32:\n    x + 1\n");
    write(&s.0.join("adder/Cargo.toml"), "[package]\nname = \"adder\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"adder\"\npath = \"target/src/main.rs\"\n\n[dependencies]\nadd_one = { path = \"../add_one\" }\n");
    write(&s.0.join("adder/src/main.hrs"), "fn main$:\n    println! \"{}\" (add_one.add_one 41)\n");
    let out = hrs("run", &s.0.join("adder"));
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "42\n");
}

/// Re-exporting a Harsh macro (the user, 2026-09-24, option (a);
/// `PACKAGING.md` section 5), the Rust Book's arrangement: `hello_macro`
/// defines the trait and says `pub use hello_macro_derive.HelloMacro`; `app`
/// depends on `hello_macro` alone and derives it. The `pub use` is absent
/// from `hello_macro`'s Rust.
#[test]
fn a_library_reexports_a_harsh_derive_to_its_users() {
    let s = Scratch::new("reexport");
    write(&s.0.join("hello_macro_derive/Cargo.toml"), "[package]\nname = \"hello_macro_derive\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"target/src/lib.rs\"\n\n[dependencies]\nhrs_proc_macro = \"0.2\"\nhrs_quote = \"0.1\"\nhrs_syn = \"0.1\"\n\n[package.metadata.harsh]\nproc-macro = true\n");
    write(&s.0.join("hello_macro_derive/src/lib.hrs"), "use hrs_proc_macro.TokenStream\nuse hrs_quote.quote\nuse hrs_syn.{parse_macro_input, DeriveInput}\n\n#[proc_macro_derive~ HelloMacro]\npub fn hello_macro_derive (input: TokenStream) -> TokenStream:\n    // Construct a representation of Harsh code as a syntax tree\n    // that we can manipulate.\n    let ast = parse_macro_input! { input as DeriveInput }\n\n    // Build the trait implementation.\n    impl_hello_macro (&ast)\n\nfn impl_hello_macro (ast: &DeriveInput) -> TokenStream:\n    let name = &ast <- ident\n    let generated = quote~ do:\n        impl HelloMacro for #name\n            fn hello_macro$:\n                println! \"Hello, Macro! My name is {}!\" (stringify! #name)\n    generated\n");
    write(&s.0.join("hello_macro/Cargo.toml"), "[package]\nname = \"hello_macro\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"target/src/lib.rs\"\n\n[package.metadata.harsh]\nproc-macros = [\"../hello_macro_derive\"]\n");
    write(&s.0.join("hello_macro/src/lib.hrs"), "pub trait HelloMacro\n    fn hello_macro$\n\npub use hello_macro_derive.HelloMacro\n");
    write(&s.0.join("app/Cargo.toml"), "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"app\"\npath = \"target/src/main.rs\"\n\n[dependencies]\nhello_macro = { path = \"../hello_macro\" }\n");
    write(&s.0.join("app/src/main.hrs"), "use hello_macro.HelloMacro\n\n#[derive~ HelloMacro]\nstruct Pancakes\n\nfn main$:\n    Pancakes.hello_macro$\n");
    let out = hrs("run", &s.0.join("app"));
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "Hello, Macro! My name is Pancakes!\n");
    let rust = fs::read_to_string(s.0.join("hello_macro/target/src/lib.rs")).unwrap();
    assert!(!rust.contains("hello_macro_derive"), "{rust}");
}

/// `hrs export` carries no Harsh crate (the user, 2026-09-25;
/// `PACKAGING.md` section 4): code using `hrs_std` gets it as its own module,
/// `matrix`, with `nalgebra` and `num-traits` in place of `hrs_std`, and no
/// word of Harsh anywhere; code without matrices gets neither. (Built with
/// cargo alone and compared with `hrs run` by hand, 2026-09-25: the Book's
/// `broadcasting` and `slicing` identical, no warnings -- not here, where
/// compiling nalgebra would add minutes to every run.)
#[test]
fn export_takes_hrs_std_in_as_its_own_code() {
    let s = Scratch::new("exportstd");
    let app = consumer(&s.0, "fn main$:\n    let a = m~ [1.0_f64 2.0; 3.0 4.0]\n    println! \"{:?}\" (a <- size$)\n", false);
    let manifest = fs::read_to_string(app.join("Cargo.toml")).unwrap();
    fs::write(app.join("Cargo.toml"), manifest).unwrap();
    fs::write(app.join("Hrs.toml"), "[dependencies]\nhrs_std = \"0.1\"\n").unwrap();
    let out_dir = s.0.join("exported");
    let out = hrs_args(&["export", &out_dir.display().to_string()], &app);
    assert!(out.status.success(), "{}", text(&out));
    let m = fs::read_to_string(out_dir.join("Cargo.toml")).unwrap();
    assert!(m.contains("nalgebra") && m.contains("num-traits") && !m.contains("hrs_std"), "{m}");
    assert!(out_dir.join("src/matrix/mod.rs").is_file());
    let main = fs::read_to_string(out_dir.join("src/main.rs")).unwrap();
    assert!(main.contains("mod matrix;") && main.contains("crate::matrix::Matrix"), "{main}");
    for f in walk(&out_dir) {
        let t = fs::read_to_string(&f).unwrap_or_default();
        assert!(!t.contains("hrs_std") && !t.to_lowercase().contains("harsh"), "{}", f.display());
    }
    // Without matrices: no module, no dependency.
    let plain = consumer(&s.0.join("plain"), "fn main$:\n    println! \"hi\"\n", false);
    let out_dir = s.0.join("exported-plain");
    let out = hrs_args(&["export", &out_dir.display().to_string()], &plain);
    assert!(out.status.success(), "{}", text(&out));
    assert!(!out_dir.join("src/matrix").exists());
    assert!(!fs::read_to_string(out_dir.join("Cargo.toml")).unwrap().contains("nalgebra"));
}

/// The website's Playground sends one file to the Rust Playground, which
/// has nalgebra and num-traits but not `hrs_std` (the user, 2026-09-26):
/// `single_file` puts the part of `hrs_std` a program uses after it, as an
/// inline `mod hrs_std { … }`, its modules nested, and leaves the program's
/// lines where they were. Code without matrices is returned as it is. (Built
/// with cargo alone and run by hand, 2026-09-26: all nine programs of the
/// Book's chapter 16, packed this way, printed what the Book shows --
/// against nalgebra 0.33, as here compiling nalgebra would add minutes.)
#[test]
fn single_file_carries_the_hrs_std_it_uses() {
    use harsh_lang::driver::{single_file, transpile_str};
    let plain = transpile_str("fn main$:\n    println! \"hi\"\n").unwrap();
    assert_eq!(single_file(&plain), plain);

    let rust = transpile_str("fn main$:\n    let v = v~ [1, 3, 4]\n    println! \"{}\" v\n").unwrap();
    let one = single_file(&rust);
    // The program first, line for line; only its paths changed.
    let program: Vec<&str> = rust.trim_end().lines().collect();
    let sent: Vec<&str> = one.lines().take(program.len()).collect();
    assert_eq!(sent.len(), program.len());
    for (a, b) in program.iter().zip(&sent) {
        assert_eq!(a.replace("hrs_std::", "crate::hrs_std::"), *b, "\n{one}");
    }
    // Then the module, whole: nested, no declarations left, no tests.
    assert!(one.contains("\nmod hrs_std {\n"), "{one}");
    for m in ["broadcast", "slicing", "refs"] {
        assert!(!one.contains(&format!("mod {m};")), "`mod {m};` left in:\n{one}");
    }
    assert!(!one.contains("#[cfg(test)]"), "{one}");
    assert!(one.contains("pub struct Vector"), "{one}");
    assert!(harsh_lang::lex::lex_rust(&one).is_ok());
    let opens = one.matches('{').count();
    assert_eq!(opens, one.matches('}').count());
}

/// `hrs dist <dir>` writes Harsh's standard distribution as `hrs build`
/// writes it into a project -- every file the distribution holds, with the
/// same text (`hrs_proc_macro`'s manifest without its development `path`) --
/// and a second run changes nothing. The Jupyter kernel gives the `hrs_std`
/// it writes to evcxr (the user, 2026-09-26).
#[test]
fn hrs_dist_writes_the_standard_distribution() {
    let dir = std::env::temp_dir().join(format!("harsh-dist-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let run = || Command::new(env!("CARGO_BIN_EXE_hrs")).arg("dist").arg(&dir).output().unwrap();
    let out = run();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    for (path, text) in harsh_lang::dist::FILES {
        let want = if *path == "hrs_proc_macro/Cargo.toml" { text.replace(", path = \"..\"", "") } else { text.to_string() };
        assert_eq!(fs::read_to_string(dir.join(path)).unwrap(), want, "{path}");
    }
    assert!(dir.join("hrs_std/src/lib.rs").is_file());
    // A second run writes nothing: the files' times stay as they were.
    let stamp = fs::metadata(dir.join("hrs_std/src/lib.rs")).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    assert!(run().status.success());
    assert_eq!(fs::metadata(dir.join("hrs_std/src/lib.rs")).unwrap().modified().unwrap(), stamp);
    // Without a folder: the usage, and a failure.
    let bare = Command::new(env!("CARGO_BIN_EXE_hrs")).arg("dist").output().unwrap();
    assert!(!bare.status.success());
    let _ = fs::remove_dir_all(&dir);
}

/// The website's Converter (Rust to Harsh) and `hrs-from` give the same
/// Harsh, byte for byte: both call `driver::convert_str` -- the converter,
/// the formatter when the result transpiles, the doc examples (2026-09-27).
#[test]
fn convert_str_is_what_hrs_from_writes() {
    let rust = "/// Adds one.\n///\n/// ```\n/// let x = add_one(1);\n/// assert_eq!(x, 2);\n/// ```\npub fn add_one(x: i32) -> i32 {\n    x + 1\n}\n\nfn main() {\n    let v: Vec<i32> = (1..4).map(|x| add_one(x)).collect();\n    println!(\"{:?}\", v);\n}\n";
    let dir = std::env::temp_dir().join(format!("harsh-convert-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.rs");
    fs::write(&file, rust).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_hrs-from")).arg(&file).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let from_cli = String::from_utf8(out.stdout).unwrap();
    let from_lib = harsh_lang::driver::convert_str(rust).unwrap();
    assert_eq!(from_cli, from_lib);
    // And the Harsh is Harsh: it transpiles, and the doc example was converted.
    assert!(harsh_lang::driver::transpile_str(&from_lib).is_ok(), "{from_lib}");
    assert!(from_lib.contains("fn add_one"), "{from_lib}");
    let _ = fs::remove_dir_all(&dir);
}

/// Every sample on the website survives a round trip, both ways, as the
/// Converter makes it (found 2026-09-27: a screenshot of the Converter showed
/// `from_row_slice(3, 2, …)` come back as `3 3 32`; the tools, driven every
/// way, never did -- this pins that they do not). Rust → Harsh → Rust gives
/// the same tokens; Harsh → Rust → Harsh → Rust gives the same Rust; and a
/// second Rust → Harsh changes nothing.
#[test]
fn every_website_sample_round_trips_both_ways() {
    use harsh_lang::driver::{convert_str, transpile_str};
    let src = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("site/src/samples.hrs")).unwrap();
    let mut samples: Vec<(String, String)> = Vec::new();
    let mut rest = src.as_str();
    while let Some(i) = rest.find("pub const ") {
        rest = &rest[i + "pub const ".len()..];
        let name = rest[..rest.find(':').unwrap()].to_string();
        let open = rest.find("r#\"").unwrap() + 3;
        let close = rest[open..].find("\"#").unwrap() + open;
        samples.push((name, rest[open..close].to_string()));
        rest = &rest[close..];
    }
    let tight = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let mut checked = 0;
    for (name, text) in &samples {
        if name.ends_with("_RUST") && !text.contains("list~") {
            let harsh = convert_str(text).unwrap_or_else(|e| panic!("{name}: {e}"));
            let back = transpile_str(&harsh).unwrap_or_else(|e| panic!("{name} back: {e}\n{harsh}"));
            // Same program: the same characters, spacing and line breaks aside
            // (the converter writes `vec!(…)` for `vec![…]`, and a trailing
            // comma may go).
            let norm = |s: &str| tight(s).replace("vec![", "vec!(").replace("],)", "])").replace("]);", "));").replace(",)", ")");
            assert_eq!(norm(&back), norm(text), "{name}: Rust → Harsh → Rust changed the program\n{harsh}\n{back}");
            assert_eq!(convert_str(&back).unwrap(), harsh, "{name}: a second Rust → Harsh differs");
            checked += 1;
        }
        if name.ends_with("_HARSH") {
            let rust = transpile_str(text).unwrap_or_else(|e| panic!("{name}: {e}"));
            let harsh = convert_str(&rust).unwrap_or_else(|e| panic!("{name} to Harsh: {e}\n{rust}"));
            let again = transpile_str(&harsh).unwrap_or_else(|e| panic!("{name} again: {e}\n{harsh}"));
            // Spacing aside: the converter writes a deref as `* counts`, the
            // same Rust as `*counts`.
            assert_eq!(tight(&again), tight(&rust), "{name}: Harsh → Rust → Harsh → Rust changed the program\n{harsh}");
            checked += 1;
        }
    }
    assert!(checked >= 6, "only {checked} samples checked");
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().map_or(false, |n| n != "target") {
                out.extend(walk(&p));
            }
        } else {
            out.push(p);
        }
    }
    out
}

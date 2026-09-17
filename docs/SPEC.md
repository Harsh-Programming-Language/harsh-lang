# Harsh — implementation notes

The user-facing language reference is `docs/LANGUAGE.md`. This file covers the internals: how the layout pass decides what it decides, and why each rule is shaped the way it is.

## Pipeline

`lex.rs` produces a flat token stream with byte spans, recording for each token whether it is first on its physical line and what that line's indentation is. It knows nothing about blocks.

`layout.rs` merges physical lines into logical lines, then builds a tree of blocks. This is the only structural knowledge in the system.

`emit.rs` walks the tree and writes Rust, copying inter-token whitespace verbatim from the original source (except the gap before a call paren, which follows Rust's spacing — see below) and substituting individual tokens. It records one source-map entry per token.

`docex.rs` runs last, over the emitted text: a fenced block in a `///` or `//!` comment is code — rustdoc compiles and runs it — so it is Harsh, and it is transpiled here and written back into the comment, with the map shifted by what each rewrite changes. It runs after the emitter rather than inside it because a comment is trivia to every other pass, and that is worth keeping. The reverse, `to_harsh_in`, runs over `hrs-from`'s output. Both work on whole lines of a doc comment, which the emitter copies verbatim, so each example is found in the output by its own text.

`bin/hrs-remap.rs` reads a cargo `--message-format=json` stream and re-renders diagnostics against the original source.

## Layout rules

- **A logical line whose last significant token is `:` or `=>` opens a block.** The `:` is consumed and replaced by `{`; the `=>` is kept and `{` appended after it.
- **A more-indented line in any other position is a continuation of the current logical line.** This is what makes multi-line method chains work with no leading-dot rule, and it is why the type-annotation `:` never conflicts with the block-opening `:` — annotation colons are always mid-line.
- **Parens isolate, or raise precedence; they neither open nor close a block.** A block may be written inside a group — a closure's `(|p|: …)`, a paren block with a line-final `:` and `)` as its tail — and it owns no token outside the group that contains its opener, so it ends where the group ends. A physical line inside an open `(` indents past the line that opened the group, or is the `)` that closes it; a line at the opener's column or left of it is an error. An unmatched `(` that runs to the end of a header is the last argument of the application before it: `fold 0 (|acc, x|:` is `fold(0, |acc, x| {`, and the tail's `)` closes the list.
- **Braces hold a block on one line.** A `{ .. }` that opens and closes on different lines is an error naming `:` / `do:`, except a hole in markup or a tree, and a brace group handed as an argument. A block in operand position is `(do:` .. `)`; a block as a value is `= do:`; an empty function body is `()`.
- **Structs and enums.** A declaration's body follows its name with no mark (`struct Point`, `enum Truth`, a record variant's bare name); inline, `\` marks the field list: `struct Point\ x: f64, y: f64`. A tuple payload is an application: `struct Meters f64`, `Tuple i32 String`. A literal is always marked `\` — `Point\` with `field = value` lines beneath, or inline `Point\ x = 1, y = 2` reading to its line's end or its isolating `)` — and so is a pattern, `Point\ x, ..`. Braces build nothing: `Point { x: 1 }` is an error; `struct Point:` and `Point:` are errors naming the forms above.
- Inside `[` or `{`, layout is suppressed entirely: brackets and braces are Rust's. Their *contents* are Harsh: juxtaposition applies inside a brace (a macro's `{ … }` body included) and inside an attribute's `#[ … ]`, which is an application.
- **One syntax for applying.** A name and the group isolating its argument are separated by a space; `f(x)` tight is an error, everywhere — patterns, declarations, attributes, macros, types. `Fn`, `FnMut`, `FnOnce` and a `fn` in type position are applications (`Fn i32 -> i32`, `fn (x: i32) (y: f64)`, `FnOnce$`). `extern "…":` opens a block of items. A `!` glued to a name is a macro bang; a bare `!(…)` is negation and is Rust's.
- **Optional grouping is not emitted.** Parens around the whole of a statement or the whole of a value after `=` / `=>` are dropped; a tuple (a top-level comma), the unit `()`, and a group that shares its expression with other tokens are kept.
- **A closure is a prototype, `|x|`, followed by a body:** a block in any of its forms — `:`, `do:`, `{ … }` — inline or indented, or an inline expression. `|x|:` and `|x| do:` both emit `|x| { … }`; `do:` emits exactly one pair of braces wherever it stands.
- Blank lines are insignificant; up to two consecutive are preserved in the output.
- Comment-only lines float to the next real line and never participate in layout decisions or receive a separator.
- Tabs advance to the next multiple of four columns. Mixing tabs and spaces is accepted but is a candidate lint.

## Token substitutions

| hrs | Rust | Notes |
|---|---|---|
| `.` | `::` | path separator |
| `<-` | `.` | field access and method call; surrounding spaces dropped, line breaks kept |
| `.(` … `)` | `::{` … `}` | use-tree group; `.(` is not valid Rust in any position, so the rewrite is unambiguous |
| `IDENT<…>` before a path `.` | `IDENT::<…>::` | turbofish; required because `Vec<i32>::new()` is a parse error in expression position, and turbofish is legal in type position too, so no position tracking is needed |
| `do:` | `{` | the `do` marker is dropped; Rust reserves the keyword but has no syntax for it |

Carve-outs where `.` stays a `.`:

- Preceded by a numeric literal → float (`1.0`, `2.5e3`).
- Followed by a digit → tuple index (`t.0`), safe because a path segment can never begin with a digit.
- Doubled → range or rest pattern (`..`, `..=`, `..base`, `Foo { x, .. }`).
The dot substitution is unconditional. An earlier version rejected a `.` following a value, on the grounds that `::` cannot legally appear there; that special case was removed. A `.` after a value can only arise from `::` after a value in Rust, which is not valid Rust, so the converter cannot produce one. The only source is a typo, which rustc reports and the remapper renders against the original source.

The lexer has two modes. In Harsh, `.` is the path separator and `::` is a syntax error; in Rust, which `hrs-from` reads, `.` is field access and `::` is the path separator. Everything else -- literals, comments, operators, indentation tracking -- is shared.

Rejecting `::` in Harsh matters more than it looks. Ten instances had accumulated across the two hand-written examples while nothing was checking, and the converter output had none, which is the same pattern the permissive dot rule produced: hand-written files drift toward Rust spelling unless the lexer refuses it.

## Block kinds and separators

The indent stack carries a block kind derived from the header line. Classification splits the header at the first top-level `=`; if there is a right-hand side it is classified from that, otherwise from the left. This keeps `-> impl IntoResponse:` from being read as an impl block while still recognising `let x = match y:`.

| Kind | Header keyword | Separator |
|---|---|---|
| `Stmts` | `fn`, `do`, `if`, `else`, `while`, `for`, `loop`, `unsafe`, and the default | `;` |
| `Fields` | `struct`, `enum`, `union` | `,` including trailing |
| `Arms` | `match` | `,` including trailing |
| `Items` | `impl`, `trait`, `mod` | `;` on bodyless declarations only |
| `UseTree` | `use` | `,` including trailing |

Within a `Stmts` block:

- Every logical line gets `;` except the last.
- A line beginning with `let`, `use`, `const`, `static`, `type`, `mod`, `extern`, or `return` always gets `;`, including when last.
- An explicit `;` is preserved and suppresses the last-line exemption.
- A nested block gets `;` after its closing brace only when its header began with `let`, `const`, `static`, `type`, or `use`.
- Attribute lines never take a separator and never count as the block's tail.

Within an `Items` block, a non-block line is a declaration without a body — a trait method signature, an associated type or const, a `use`, or an item macro — and all of them terminate with `;`. Lines that do have a body arrive as blocks and are unaffected. This generalisation replaced an earlier keyword whitelist that missed trait method signatures.

## Source map

The emitter records one entry per token: `[gen_lo, gen_hi, src_lo, src_hi]`, all byte offsets. Whitespace between tokens is copied verbatim rather than regenerated, so the mapping is close to an identity and diagnostics land on the right column, not merely the right line.

`hrs-remap` remaps `byte_start`/`byte_end` for every span in every message and sub-message, then re-renders. rustc's `rendered` field is discarded, because it has the generated file's paths and line numbers baked in.

Offsets with no entry — inserted braces and separators — resolve to the next real token.

A rewritten doc example is one exception to the identity: its body is replaced wholesale, so entries after it are shifted by the length difference and offsets inside it resolve to the next token after the comment. A mistake *in* an example is caught before rustc ever sees it, and is reported against the `.hrs` line directly.

Two facts about rustc's JSON output that the design depends on, both verified rather than assumed:

- `byte_start`/`byte_end` are byte offsets, while `column_start`/`column_end` are character counts. They diverge on any line containing non-ASCII, so the map is keyed on bytes.
- `include!` preserves the included file's path and line numbers with `expansion` set to null, so a build.rs-generated layout would not break span mapping. This was checked because the opposite is widely assumed.

## Juxtaposed application

`juxt.rs` computes **expression regions** per logical line rather than transforming everywhere and carving out exceptions. That is the whole difficulty: adjacent identifiers mean application in an expression, but something else in a type (`&mut Formatter`), a declaration (`let mut sum`) or a signature.

- Patterns juxtapose — `if let Some p = prev`, `Location.Tup a b`, `Query params` as a destructured parameter.
- Type positions do not — a tuple-struct or variant payload, field types, signatures.
- A `fn` header exposes only its parameter groups; every other header juxtaposes whole, since a match arm's pattern and a scrutinee are both ordinary applications.
- A closure is an argument atom only where unambiguous: parameters ending a block header. Elsewhere `|` and `||` stay operators. An early version recognised them everywhere and read `a || b` as an empty closure.
- A `<` is a generic argument list only when the matching `>` is followed by a call or a path step; otherwise it is a comparison. Without that, `s < to && a.lo > b` was read as generics.

`<|` and `|>` are rewritten in `juxt::rewrite_pipes`, once per logical line in `layout::build_with`, before anything else sees the stream. Each pipe expression — atoms `|>` function `<|` atoms — becomes a juxtaposition run, `callee left-atoms holes right-atoms`, preceded by `move |holes| ` when the count falls short of the function's arity; juxtaposition then wraps the run into a call, so the emitter, the source map and the separator rules never see a pipe. The arity table (`collect_arities`, project-wide, `fn` declarations only, plus partials bound by `let` on earlier lines) is read here and nowhere else. Unknown arity means a plain call with the atoms given. The first token of every piece that left source order is marked synthetic, so the emitter writes a single space instead of copying the source gap that preceded it; its span still maps.

## Rust to Harsh

`unbrace.rs` runs the pipeline backwards. Brace matching is free, since the lexer already tracks bracket depth, so the whole problem reduces to one classification per `{`.

Two decisions keep it safe rather than clever:

- Only braces at paren/bracket depth zero are considered. A `{` inside a call is a closure body or a struct literal in argument position, and Harsh suppresses layout inside brackets anyway.
- When classification is not clear-cut, the braces are left alone. Braces are legal everywhere in Harsh, so the fallback is always valid output, just less idiomatic.

Classification walks back from the `{` to the start of its segment -- the run of tokens since the last `;`, `{`, or `}` at the same depth -- and looks for a block keyword. Special cases: `::` immediately before means a use tree; an identifier followed by `!` and an opening bracket means a macro; a trailing `=>` means a match arm; an empty body keeps its braces, since Harsh rejects a block opener with nothing under it.

Semicolons are dropped except on the final statement of a block, where dropping one would turn a discarded value into a tail expression. That rule is semantics-preserving without any type information.

## Self-hosting as the test oracle

The transpiler's own source is the corpus. Converting every module to Harsh, transpiling back, and building the result gives an automated correctness test that hand-written examples cannot match. The current state is a fixed point: the round-tripped transpiler compiles, and its binaries produce byte-identical output to the original on all four examples and on the converter itself.

Bugs this loop found that the examples did not:

- `_ => {}` -- an empty block converted to a header with no body, which Harsh rejects.
- `if !flag {` -- read as a macro invocation, because an identifier followed by `!` looked like a macro bang. Fixed by requiring an opening bracket after the `!`.
- `return match x:` -- classified as a statement block rather than an arm list, because classification stopped at the first identifier instead of scanning for the first *recognised* keyword.
- `let x = if c: .. else: ..` -- the `;` landed after the `if` block, orphaning the `else`. The keyword and the assignment flag now inherit through the chain.
- `chain_kw = if c: .. else: ..` -- plain assignment, not `let`, so the trailing `;` was missed. A top-level `=` in the header is now the signal.
- `self.i += if c: .. else: ..` -- same, for compound assignment operators.
- `if let Some(p) = x:` -- contains a top-level `=` but the block is not a value. An `=` now only counts when it precedes the block keyword.
- `use a::{b, c}` -- ends with `}` and still needs its `;`.
- `pub const X: [&str; 3] = [..]` -- lost its `;`, because the leading-keyword check did not skip modifiers.
- `b'\n'` -- byte char literals were lexed as an identifier plus a char.
- `serde_json::from_str::<T>(..)` -- a generic call with no trailing path got no turbofish. Rust rejects `a < b > (c)` as a chained comparison, so turbofishing before `(` is unambiguous.
- `fn parse_all<'a, I> (..)` -- the above then wrongly turbofished *definition* sites, where generics are declared rather than applied.

## Verification status

Three example programs transpile, compile, and run:

- `examples/hello.hrs` — axum server. Attributes, `async`/`.await`, macros, pattern-destructured extractor parameters, multi-line use trees. Both routes serve correctly.
- `examples/edge.hrs` — match with inline and block arm bodies, `impl`, `enum`, tuple indexing, turbofish, `do:` as an expression, `if`/`else`, doc comments.
- `examples/general.hrs` — std only. Trait with required and default methods, `impl Trait for Type`, `fmt::Display`, generic function with lifetime parameter and multi-line `where` clause, nested modules, closures in iterator chains, `Result` with early return, `while let`, `if let`, `const` and `static`.

Diagnostic remapping verified on three simultaneous errors across three lines, including inside a nested match arm and through a `<-` rewrite, all landing on the correct column.

## Known gaps

- Indented closure bodies cannot work, since a closure body inside a call sits within brackets where layout is suppressed. The brace form works.
- `macro_rules~` definitions are untested.
- `else if` chains beyond one level are untested.
- Nested `.( … )` groups more than one level deep in a use tree are handled but only lightly exercised.
- `else` emits on its own line after the closing brace. Valid, but rustfmt would join it.
- No lint yet for `.` where `<-` was meant on an identifier receiver. The rustc error remaps to the right column but the message talks about paths.

## Not in stage one

- Juxtaposed application at call sites (`f x y`). Needs an expression parser, plus the `$` rule, the no-commas rule, and the `<-` precedence rule. Parameter declarations are curried already, since that region is bounded and needs no parsing.
- `Point:` struct literals. Needs expression-position knowledge.
- `Router <- new$` for type paths. Needs name resolution to distinguish a type from a value.

## Call-paren style in generated Rust

Decided: generated Rust follows Rust's own spacing around a call paren, whatever the `.hrs` writes. A `(` that follows a callee — a name, a closing bracket, a turbofish `>`, a macro's `!` — is emitted tight: `map(`, `iter()`, `Some(1)`, `format!(`, `v.get(0).unwrap()`; and the invented `(` of a bare parameter list sits against the function name, `fn g(name: &str)`. A `(` after a keyword keeps its space, as rustfmt writes it: `if (a)`, `return (x)`; the keyword list is `rules::SPACED_KEYWORDS`. Only that one gap changes; every other inter-token gap and every line break is still copied from the source, so the source map is unaffected. The `.hrs` keeps its idiom — `f (x)` reads as "f applied to one isolated thing" — and the converter still writes it that way. Rustfmt on the output was never an option: it moves every byte offset and breaks the map. Test: `call_parens_follow_rust_spacing`.

## `$` — apply to nothing

`$` is a suffix meaning *apply to nothing*: `f$` is `f()`, `fn f$` is `fn f()`. It is tight, like a macro's `!`; a spaced `$` is an error. Because `$` carries this meaning, `()` in Harsh is only ever the unit value: `f ()` is `f(())`, `Ok ()` is `Ok(())`, and a `()` group in a `fn` declaration is an error naming `fn name$`.

Implementation: `juxt::rewrite_dollar` runs on every logical line before `rewrite_pipes` and replaces a `$` that is tight after a name, `)`, `>`, `>>`, a tuple index or `!` with two synthetic tokens `(` `)` carrying the `$`'s span and the `Token.dollar` flag. `juxt::atom_end` absorbs a `dollar` pair into the atom before it (`dollar_end`), so `f$` is one atom wherever it stands — argument, head, pipe side — and no other rule knows `$` exists: the pipes see a call, `fn_param_groups` sees an empty group, the emitter writes `()`. The `dollar` flag is separate from `synthetic` because the pipes mark *moved* tokens synthetic while keeping their real spans, and the emitter must not copy gaps after those; after a `$` pair it must. `layout::check_fn_params` rejects a source `()` group and a `$` beside other groups. A `$` that is a prefix (`$a`, `$(`) is a macro metavariable and passes through; a line containing `macro_rules` is skipped entirely, as by the pipes.

The converter writes `$` for an empty call or parameter list (`unbrace::empty_params`, the call path, and a fallback in the token loop for callees the call path does not recognise) and `()` for a unit argument; an empty call is atomic as an argument. Test: `dollar_applies_to_nothing`; the round trip over the transpiler's own source covers the rest.

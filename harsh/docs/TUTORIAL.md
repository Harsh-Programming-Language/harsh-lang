# Building a language on top of Rust

*What was used to build Harsh, and what it taught.*

This is a working account rather than a compiler course. It covers the techniques that carried the project, the ones that were tried and abandoned, and the handful of lessons that generalise beyond this particular language.

---

## Part one — the architecture

### The decision that made everything else possible

The original transpiler was a conventional front end: lexer, recursive-descent parser, semantic AST, emitter that reconstructed Rust from the tree. It handled a subset of Rust and would have needed to grow indefinitely to handle the rest.

It was replaced with a **layout transformation**. The transpiler understands exactly one thing — which lines are nested inside which — and copies every token it does not explicitly rewrite.

That single decision is why generics, lifetimes, `impl Trait`, `async`/`.await`, macros, closures, const generics, and every future Rust syntax addition work without the transpiler knowing they exist. It never has to keep up with Rust, because it never had to catch up in the first place.

The cost is that it cannot do anything requiring semantics. That turned out to constrain almost nothing worth having.

### The pipeline

```
lex.rs      tokens with byte spans; two modes differing only in what `.` means
layout.rs   physical lines -> logical lines -> a tree of blocks; validation
juxt.rs     expression regions; juxtaposed application; the pipe rewrite (which is also partial application)
rules.rs    keyword tables shared by both directions
emit.rs     Harsh -> Rust, plus the source map
unbrace.rs  Rust -> Harsh
remap.rs    rustc/clippy diagnostics -> Harsh source positions
driver.rs   project walk, incremental build, cargo invocation, watch mode
```

Both directions live in one crate deliberately. They must agree on the carve-out list and the block-kind table, and separate crates would let those drift. That is not hypothetical: a duplicated `ALWAYS_SEMI` constant in `emit.rs` once shadowed the shared one in `rules.rs`, so edits to the shared table silently did nothing. The round-trip test found it; nothing else would have.

### Two layout rules do all the work

- A logical line whose last significant token is `:` opens a block.
- A more-indented line in any **other** position continues the current logical line.

The second rule is the interesting one. Python would reject a multi-line method chain; this rule accepts it with no leading-dot convention and no explicit continuation marker. It also means the block-opening `:` can never be confused with a type annotation, because annotation colons are always mid-line.

Everything structural in the language follows from those two sentences.

### Spans and the source map

Every token carries byte offsets into the original source, and the emitter records one map entry per token. That granularity is what makes diagnostics land on the correct **column** rather than merely the correct line.

The decision that made it nearly free: **whitespace between tokens is copied from the source rather than regenerated.** `>>` never becomes `> >`, multi-line chains keep their shape, and the mapping is close to an identity. A pretty-printer would have destroyed all of that for no gain, since generated Rust is a build artifact nobody reads.

Two facts about rustc's JSON output that the design depends on, both verified rather than assumed:

- `byte_start`/`byte_end` are byte offsets while `column_start`/`column_end` are character counts. They diverge on any line with non-ASCII, so the map is keyed on bytes.
- `include!` preserves the included file's path and line numbers with `expansion` set to null. This contradicts the common assumption and mattered for the project-layout decision.

---

## Part two — the techniques

### Empirical verification over reasoning

The single most productive habit. Confident claims that turned out wrong when tested:

| Claim | Reality |
|---|---|
| `Vec<i32>::new()` works in expression position | Parse error; turbofish required |
| Turbofish is only valid in expression position | Valid in type position too — which made the fix context-free |
| `include!` breaks span mapping | It preserves paths and line numbers exactly |
| Indented closure bodies are impossible | Three of four forms already worked |
| Closures cost something at runtime | LLVM aliased them to the direct call: `.set via_closure, direct` |
| `do`, `block`, `begin` are equivalent keyword choices | Only `do` is reserved in Rust; the others are live identifiers |

Each was settled by a probe taking under a minute. The pattern is worth naming: **when a claim is decidable by running something, run it.** Reasoning about a compiler's behaviour is strictly worse than asking the compiler.

### Behavioural probes

```bash
# Is `<-` a real token, or does it collide with comparison?
echo 'fn main(){ let x=5i32; let _ = x<-1; }' > t.rs && rustc t.rs
# error: unexpected token `<-`
# help: if you meant a comparison against a negative value, add a space
```

That answer shaped an operator choice. Similar one-liners settled which keywords are reserved, whether tuples implement `Index`, whether `a < |x| x` parses, and whether closures survive optimisation.

### Assembly as the arbiter

When a performance claim came up — do partial applications cost stack frames? — the way to settle it was to look:

```bash
rustc -O --crate-type=lib --emit=asm -o a.s a.rs
```

The output contained `.set via_closure, direct` and `.set via_nested, direct`. Not similar code — the same symbol. That killed a proposed optimisation pass outright, saving a session's work on something with no payoff.

### Self-hosting as a test oracle

The transpiler's own source is the corpus. Convert every module to Harsh, transpile back, compare token streams:

```rust
#[test]
fn round_trip_own_source() {
    for (name, rust) in corpus() {
        let harsh = convert(&rust);
        let back = transpile(&harsh);
        assert!(same_tokens(&rust, &back), "{name}");
    }
}
```

This is the highest-leverage thing in the project. See the lessons below.

### Verify from a clean extract

Every packaged archive was extracted to an empty directory and the test suite run there. This caught things the working tree hid:

- An archive built from stale sources — five tests failed on extract that passed in place.
- `examples/` breaking `cargo test` in a fresh checkout, because Cargo treats that directory as example *targets* and choked on a dot in a filename. Fixed with `autoexamples = false`.
- A stale `.hrs` reference file left over from before a syntax change, no longer valid input.

The working tree lies. The extract does not.

### Verify the layout before committing to it

The project-layout question stayed open for most of the project. It was settled by trying the preferred option rather than reasoning about it:

```toml
[[bin]]
name = "myapp"
path = "target/hrs/main.rs"
```

Cargo accepts arbitrary target paths, `/target` is already gitignored, dependencies stay in one manifest, and `mod` works across generated files. Five minutes of testing replaced weeks of intermittent deliberation.

---

## Part three — the lessons

### Permissive was wrong every single time

This is the one that generalises furthest.

Three separate bugs came from making the lexer accommodating, and each drifted into inconsistency in files written by hand — while the converter, which has no choice but to apply rules uniformly, never drifted once. Strictness cost nothing and was the only thing that kept one spelling.

The three:

1. **The impossible-path carve-out.** A `.` after a literal or closing bracket was allowed to stay a `.`, on the grounds that `::` could not appear there. Justified as paste-compatibility. It delivered roughly one dot in four surviving a paste, unpredictably — worse than uniform failure, because the errors were scattered and the message talked about undeclared crates.

2. **`::` passing through unchanged.** Ten instances accumulated across two hand-written example files while nothing was checking. The converter's output had zero.

3. **Macro parentheses tolerated.** Rust call syntax written into a macro was accepted and silently reinterpreted as a tuple argument.

Each time the fix was the same: reject it. And each time the objection to rejecting it — "but then pasted Rust breaks" — dissolved once the converter existed. **There is a tool for translating; the language does not need to be forgiving.**

The deeper point is about who enforces consistency. A permissive rule relies on discipline, and discipline fails silently. A strict rule is enforced by the machine on every build.

### Keep the round trip byte-exact

Converting the transpiler's own source to Harsh and back caught roughly twenty bugs the examples never reached.

A sample, none of which hand-written tests would have found:

- `pub const X = …` silently lost its `;`, because the leading-keyword check did not skip modifiers.
- `let x = if c: … else: …` put the `;` after the `if`, orphaning the `else`.
- `chain_kw = if …` and `self.i += if …` missed their `;` — plain and compound assignment were not recognised as value contexts.
- `if let Some(p) = x:` then over-corrected the other way, since it contains a top-level `=` that binds a pattern rather than assigning.
- `return match x:` was classified as a statement block, so its arms got `;` instead of `,`.
- `if !flag {` was read as a macro invocation, because an identifier followed by `!` looked like a macro bang.
- `if a && matches!(…) {` was classified as a **macro body**, because a macro appeared somewhere earlier in the condition.
- `if expr && …` was misread as an `if let`, because the item contained a `let` further down in its body.
- A duplicated constant shadowing the shared one, so edits to the shared table did nothing.

The pattern: **these are all cases where a rule was almost right.** Hand-written examples exercise the cases you thought of. Real code exercises the cases you did not.

Two properties make it work. It must be **byte-exact** — the moment "expected differences" are tolerated, real differences hide underneath them. And the corpus must be **real code**, not code written to demonstrate features.

### Isolating parentheses resolve ambiguity

Whenever two readings were possible, the answer was parentheses rather than a context-sensitive rule. `f (|x| body)` for a closure in an ambiguous position, `f ((a, b))` where a tuple must be distinguished, `(add 10) 7` to apply a returned closure, `[where T: Clone]` so a bound's colon cannot open a block.

This is Haskell's answer and it held throughout. The alternative — a rule that decides based on surroundings — is where languages accumulate the special cases nobody can remember.

### Do not patch in a tight loop

A regression got in during a stretch of rapid fixing: `a || b` was read as an empty-parameter closure, because closures had just been made recognisable everywhere rather than only where unambiguous.

The failure mode is characteristic. Each individual patch looked reasonable; the third one broke something the first two depended on. **When two or three fixes in a row fail, stop and measure.** The round-trip test names the file and the first differing token — reading that is faster than another guess.

### Confident wrongness is the expensive kind

The corrections that mattered most all followed the same shape: a claim that something was impossible, overturned by being asked to check.

- "Macros must keep parentheses permanently" — they juxtapose identically to functions.
- "Indented closure bodies are not possible" — three of four forms already worked.
- "Partial application needs name resolution" — not when written as a function returning a closure, and not when `$` counts the currying splits explicitly.
- "Juxtaposition needs a real expression parser" — it needed expression *regions* computed per line.

Each was stated with more confidence than the evidence supported. The tell, in retrospect: **an impossibility claim made without having tried it.** The habit worth building is to notice that shape and go and check.

### Simple recursive rules beat enumerated cases

The `$` deferred-argument marker is the clearest example. The first design tried to enumerate arities and needed a symbol table. The rule that shipped is one sentence — *each `$` is one currying split* — and needs no arity knowledge at all:

```
sub3$ 100 20   ->  move |p1| sub3(100, 20, p1)
sub3$$ 100     ->  move |p1| move |p2| sub3(100, p1, p2)
```

Too many `$` becomes a type error rustc reports, remapped to the source. The transpiler counts nothing.

(History: `$` was later rebuilt with project-wide arity, and later still folded into the pipes — `|>` fills from the left, `<|` from the right — so that one mechanism defers. The lesson stands; the operator does not.)

The same shape appears in the inline-block separators: instead of new syntax per block kind, the inline form writes the separator the indented form would have inserted. One rule, three constructs, and a test asserting both forms emit identical Rust.

### Reuse where the languages agree; rewrite only what differs

For VSCode, the grammar is a few dozen lines: `{"include": "source.rust"}` plus four operators. Keywords, strings, types, lifetimes and macros come free because none of them changed.

For Zed, tree-sitter is mandatory and highlighting derives from the parse tree, so a parse error removes highlighting entirely rather than degrading it. But the reuse still applies at the grammar level — the token set is Rust's, and only block structure and the new operators needed writing.

Knowing **which layer** the reuse lives at is the whole skill. At the token level it was near-total; at the parser level it was partial; assuming either without checking would have wasted the effort.

### Delete the optimisation that measurement kills

A proposal to eliminate closures via descriptor tracking and dataflow analysis would have taken about a session, added a symbol table to a transpiler that has none, and weakened the source map.

One assembly dump ended it: the compiler had already aliased the closure forms to the direct call. **Measure before optimising** is old advice; what is worth adding is that measuring also protects the architecture. The optimisation's real cost was not the week — it was that the transpiler would have stopped being a layout transformation.

---

## Part four — practical notes

### Environment

- rustc 1.75 from a package manager, with no rustup available. axum pinned to 0.7 as a result, using `:name` route syntax.
- `autoexamples = false` in `Cargo.toml`, or Cargo tries to compile `examples/*.generated.rs` as crate targets.
- Watch mode uses mtime polling rather than filesystem events: no dependency, no platform differences, and editors write in stages anyway.

### The acceptance test that mattered

Not unit tests on the layout engine — those pass while the thing is broken. The real bar was: transpile a real axum server, have `cargo build` succeed, run it, and curl both routes. That single program exercises attributes, generics, macros, `async`/`.await`, and pattern-destructured parameters at once, and it is the smallest program that does.

### Documentation drifts faster than code

The syntax guide fell three features behind while the implementation moved, and at one point documented as valid something that had become a syntax error. A reader arriving cold would have been actively misled.

The fix was a `docs/dev/HANDOVER.md` that restates the current state in compressed form and says plainly which other documents are stale. **Write the handover as though the next reader has none of your context, because eventually that reader is you.**

---

## The short version

1. Verify against a real compiler rather than asserting. Impossibility claims especially.
2. Prefer strict to permissive. Permissiveness relies on discipline, and discipline fails silently.
3. Round-trip real code and keep it byte-exact. Tolerating "expected differences" hides real ones.
4. Isolating parentheses resolve ambiguity better than context-sensitive rules.
5. One change at a time; when fixes stop working, measure instead of guessing.
6. Simple recursive rules beat enumerated cases, and usually need less machinery.
7. Measure before optimising — it protects the architecture as much as the schedule.
8. Verify from a clean extract. The working tree lies.

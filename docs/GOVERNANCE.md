# Governance

## Who decides

Language design decisions rest with the project lead, Bahiminin Benoit Dah, who designed the language. That covers syntax, semantics, and what does and does not belong in the language.

This is deliberate and is not expected to change. A language needs a coherent point of view more than it needs consensus, and Harsh has one: it is a **layout transformation over Rust**, not a new language. Every proposal is measured against that.

Implementation, tooling, documentation and bug fixes are open to anyone, and the bar there is ordinary code review.

## What the project will not do

These are settled, and proposals to change them will be declined:

- **No semantic analysis in the transpiler.** No type knowledge, no name resolution. This is why generics, lifetimes, `impl Trait`, and future Rust syntax all work without the transpiler knowing they exist. It is the single most valuable property Harsh has.
- **The arity table is for the pipes alone.** Deferring the rest of a function's parameters needs the parameter *count* of that function, so the transpiler collects those counts — never types — from `fn` declarations across the project. That table is read by `rewrite_pipes` and by nothing else, and proposals to read it elsewhere will be declined. A second reader would make it a symbol table, and a symbol table is the first step toward the transpiler doing semantic analysis. If that is ever the right call, it is decided as its own question, on its own merits, not as a side effect of a feature that found the table convenient. Structurally: `collect_arities` produces the table, `layout::build_with` receives it, and `juxt::rewrite_pipes` is its only consumer.
- **A rule holds in every form it applies to.** The reason Harsh can be learned is that a small set of syntax is enough and the rest is deduced from the pattern: a written `;` discards a block's tail value, so it does so in the indented block, the inline `do:`, the inline `if` and the match arm alike — the day the inline form dropped it (2026-09-11), that was a bug, not a quirk to document. Every exception a rule acquires is one more thing to memorise and one more place the pattern stops predicting; the test for a proposed exception is whether the learner could have guessed it, and if not, it is the rule that is wrong. In the author's words: simple repeated patterns make a language predictable; one learns a small set of the syntax and deduces the rest.
- **No divergence from Rust semantics.** Harsh changes how Rust is written, never what it means. Anything that would make a Harsh program behave differently from the Rust it generates is out of scope.
- **One spelling is preferred, not enforced.** Where two syntaxes would express the same construct, the guide names the idiom; alternatives exist where they serve a purpose — the three block forms (`:` indented, `do:`, `{ … }` for several statements on one line), `do:` after a return type for the eye, `|x|:` and `|x| do:` for a closure's body — and are documented as such. A spelling is made a syntax error only for a reason: consistency, so that two files for one program cannot drift (Rust's `::`, the comma parameter list, a written `,` after a match arm), or safety, where the spelling would otherwise *compile to something else*. Permissiveness in the lexer has produced every consistency bug this project has had; permissiveness in the syntax has not.
- **Leave to rustc what rustc rejects.** A poor spelling that produces Rust which does not compile needs no Harsh-side error: rustc's message lands on the `.hrs` line through the remapper, and the transpiler stays small. Harsh must catch only the poor spelling that would otherwise compile and mean something else — `fn add &self (k: i32)` once emitted valid, wrong Rust silently, and that is the kind of error worth a rule. The test for a proposed Harsh-side error is therefore: *would the bad spelling otherwise compile?*

## Proposing a change

Open an issue describing the problem before the solution. A syntax proposal should say:

- What you cannot currently express, or can only express awkwardly.
- What the rule is, stated in one or two sentences.
- Which existing rule it interacts with, and whether any construct becomes ambiguous.
- Whether it can be done as a token rewrite or needs the transpiler to understand expressions.

That last question decides most proposals on its own.

## Standard of evidence

Claims about Rust's behaviour are verified against a real compiler, not asserted from memory. Several confident assumptions during this project's development turned out to be wrong when tested — that `Vec<i32>::new()` works in expression position, that `include!` breaks span mapping, that closures cost something at runtime. A one-line probe settles these.

Behavioural changes need a test. The round trip over the transpiler's own source (`tests/roundtrip.rs`) is the main safety net and must stay byte-exact.

## Forks

The MPL guarantees your right to fork, and that will not change. If you disagree with a direction, forking is a legitimate response and no permission is needed.

The only requirement is the name: a fork must be called something else. See `docs/TRADEMARK.md`.

## Contributor agreement

Contributions require signing off under the terms in `CONTRIBUTING.md`, which keeps the project able to relicense in future. Without that, a project with many contributors is permanently frozen on its original licence, which is a worse outcome for everyone.

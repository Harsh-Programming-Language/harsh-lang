# Contributing

## Before you start

For a **bug fix**, open a pull request directly.

For a **language change**, open an issue first. `docs/GOVERNANCE.md` describes what the project will and will not do, and what a proposal needs to say. Several kinds of change are settled and will be declined — reading that file first will save you the work.

## The contributor agreement

By submitting a contribution you agree to the terms below. Add a `Signed-off-by:` line to your commits (`git commit -s`) to indicate this.

1. You certify that you wrote the contribution, or otherwise have the right to submit it under these terms.
2. You license your contribution to the project under the Mozilla Public License 2.0, the same licence as the rest of the code.
3. You additionally grant the project lead the right to distribute your contribution under a different licence in future, provided that licence is approved by the Open Source Initiative.

Point 3 is the only unusual one, and it exists for a specific reason: without it, a project with many contributors can never change licence, because every contributor would have to agree. That has stranded real projects. The restriction to OSI-approved licences means this cannot be used to take the project closed.

If you would rather not agree to point 3, say so in your pull request. Small fixes can usually be taken another way.

## Working on the code

```
cargo build
cargo test          # 24 tests; all must pass
./check.sh          # builds, transpiles, runs and round-trips everything
```

The test that matters most is `round_trip_own_source`: it converts every module of the transpiler to Harsh, transpiles it back, and compares token streams. It is currently **byte-exact**, and it must stay that way. It has caught roughly twenty bugs that hand-written examples never reached, including a duplicated constant that silently shadowed the shared one.

When something breaks, that test names the file and the first differing token. Read it before reasoning about the cause.

## What good changes look like

- **Verify, do not assert.** Claims about Rust's behaviour are checked against a real compiler. A one-line probe settles most questions faster than an argument does.
- **Prefer strict to permissive.** Every consistency bug this project has had came from making the lexer accommodating. There is a converter (`hrs-from`) for anything that needs translating; the language itself does not need to be forgiving.
- **Isolating parentheses resolve ambiguity.** Where two readings are possible, that is the answer, rather than a context-sensitive rule.
- **One change at a time.** A regression got in during a stretch of rapid patching. If two or three fixes in a row fail, stop and measure rather than continuing.

## Adding a language rule

A rule that changes emitted output needs:

- A test in `tests/roundtrip.rs` covering it.
- A test covering what it now rejects, if it rejects anything.
- An entry in `docs/LANGUAGE.md`, which is the user-facing reference.
- A note in `docs/SPEC.md` if it affects the internals.
- The round trip still byte-exact, and all examples still building and running.

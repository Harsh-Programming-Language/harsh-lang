# A Harsh lexer for Rouge (GitLab's highlighter)

GitLab colours code with [Rouge](https://github.com/rouge-ruby/rouge). Until
Rouge knows Harsh, Harsh blocks are fenced `rust harsh` and coloured as Rust.
This folder is a Rouge lexer for Harsh, ready to submit as a pull request to
`rouge-ruby/rouge`. Once Rouge releases it and GitLab updates its Rouge, a
fence can say `harsh` and `.hrs` files are coloured as Harsh.

The lexer is Rust's with Harsh's own tokens added: `<-`, `f$`, the pipes
`|>` and `<|`, `do:`, a literal's `\`, the turbofish `.<T>`, Harsh macros
`name~` and their metavariables `($x:expr)`, Rust macros applied like
functions (`println! "{}" x`), and a macro's brace body lexed as the DSL's
text, with its holes `@: … :@` lexed as Harsh (`@@:` is a literal `@:`).

## The files, where they go in Rouge's repository

| Here | In `rouge-ruby/rouge` |
|---|---|
| `lib/rouge/lexers/harsh.rb` | the same path |
| `lib/rouge/demos/harsh` | the same path |
| `spec/lexers/harsh_spec.rb` | the same path |
| `spec/visual/samples/harsh` | the same path |

Add one line to `docs/Languages.md`, in alphabetical order:

    - Harsh (`harsh`)

## Checked

Against Rouge's `main` of 2026-10-13: the Harsh spec (10 tests) and Rust's
pass, and so does Rouge's whole general suite (522 tests, which lex every
demo and guess every lexer), with no error token in the visual sample.
Rouge has no popularity rule for new lexers; a maintainer reviews the pull
request. `bundle exec rake` runs the same suite in a checkout with its gems.

## To submit (yours)

1. Fork `rouge-ruby/rouge`, copy the four files and the `Languages.md` line.
2. `bundle exec rake` in the fork.
3. Open the pull request: "Add Harsh lexer", linking https://harsh-lang.com/.

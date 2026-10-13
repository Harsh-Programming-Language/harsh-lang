# Harsh in GitHub's Linguist

GitHub colours code, and counts a repository's languages, with
[Linguist](https://github.com/github-linguist/linguist). Until Linguist
knows Harsh, a `.hrs` file is shown as plain text, and a fenced block is
coloured as whatever its fence names (`rust harsh` gives Rust's colours).

## When Linguist will take it

Linguist adds a language only once it is used widely on GitHub. Its rule
(CONTRIBUTING.md, *Language extension and filename usage requirements*,
read 2026-10-13):

- **at least 2,000 `.hrs` files indexed by GitHub Search in the last year**,
  forks excluded (the threshold for an extension that occurs more than once
  per repository);
- spread across many `user/repo` pairs: if the language's own authors make
  up much of the results, they are filtered out with `-user:…`.

The search to watch, and to give in the pull request:
`path:*.hrs NOT is:fork`. No language in Linguist uses `.hrs` today.

Very new languages are closed on sight, so the pull request waits until the
count is there.

## The entry, ready for that day

In `lib/linguist/languages.yml`, in alphabetical order (Linguist's script
assigns the `language_id`):

```yaml
Harsh:
  type: programming
  color: "#6fb3d2"
  extensions:
  - ".hrs"
  tm_scope: source.harsh
  ace_mode: rust
  codemirror_mode: rust
  codemirror_mime_type: text/x-rustsrc
```

- **Colour:** Harsh's accent blue, `--acc` in the website's pages. Any
  colour is the user's to choose.
- **Grammar:** `tm_scope: source.harsh` is the VS Code extension's TextMate
  grammar, `editors/vscode-harsh/syntaxes/harsh.tmLanguage.json`. Linguist
  takes a grammar from a public repository of its own
  (`script/add-grammar https://github.com/…`), so the grammar needs one,
  like `tree-sitter-harsh` has.
- **Samples:** two or three real `.hrs` files under `samples/Harsh/`, for
  the classifier; the Book's programs serve.

## Meanwhile

A repository can colour its own `.hrs` files as Rust on GitHub with a
`.gitattributes` line, `*.hrs linguist-language=Rust`. That also counts
them as Rust in the repository's language bar, so it is the user's choice,
repository by repository.

# The Harsh website

The site for **Harsh — Rust without the braces**, written in Harsh: a
[Dioxus](https://dioxuslabs.com) app whose `rsx!` trees carry Harsh in
holes, built to a static site and published on GitHub Pages by the
repository's workflow (`.github/workflows/pages.yml`).

```
src/          the site, in Harsh (src/**.hrs)
assets/       the stylesheet, the logo, the favicon
public/       the Book, By Example and the guide, as this tree renders them
```

The Converter page is the transpiler itself, this tree's `harsh-lang` library,
compiled to WebAssembly with the site.

## Building it

From the repository root, with `hrs` built or installed
(`cargo install --path .`), the Dioxus CLI (`cargo install dioxus-cli`) and
the wasm target (`rustup target add wasm32-unknown-unknown`):

```
cd site
hrs cargo metadata --no-deps > /dev/null    # transpile src/**.hrs to target/hrs/
dx serve                                    # then http://localhost:8080
```

`dx` copies `public/` into its output (`public_dir` in `Dioxus.toml`), so
the Learn page's links to the Book, *By Example* and the guide work under
`dx serve` as they do on Pages.

## Publishing

See [`DEPLOY.md`](DEPLOY.md): the one-time settings on GitHub, what
each deploy does and how to check it landed, and how the domain is set up.
In short, a push to `main` publishes the site to **https://harsh-lang.com/**.

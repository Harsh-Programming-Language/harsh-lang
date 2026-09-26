# Publishing the website

The site in `site/` is published to **GitHub Pages** by the workflow
`.github/workflows/pages.yml`, from the GitHub mirror of the repository. It
is free for a public repository: a soft limit of 100 GB of traffic a month
and 1 GB of site, which throttle rather than bill; the build runs on GitHub
Actions, free for public repositories.

First at the free address,
**https://harsh-programming-language.github.io/harsh-lang/**; later, at a
domain of your own (part C).

## A. Once, before the first deploy

1. **Let the mirror carry the workflow.** The GitLab push mirror
   authenticates to GitHub with a fine-grained token that has *Contents:
   read and write*. GitHub refuses a push that adds or changes a file under
   `.github/workflows/` unless the token also has **Workflows: read and
   write**. On GitHub: Settings → Developer settings → Personal access
   tokens → Fine-grained tokens → the mirror's token → add *Workflows: read
   and write* (or make a new token with both, and paste it into GitLab:
   Settings → Repository → Mirroring repositories → the GitHub mirror →
   edit).

2. **Turn Pages on, with Actions as its source.** On the GitHub repository
   `Harsh-Programming-Language/harsh-lang`: Settings → Pages → *Build and
   deployment* → Source: **GitHub Actions**.

3. **Commit the site's lock file** (recommended). After `dx serve` has run
   on your Mac, `site/Cargo.lock` exists. Committing it makes every deploy
   build the same versions you tested, instead of the newest ones that day.
   (`site/target` and `site/dist` are ignored; the lock file is not.)

## B. Every deploy

Push to `main` on GitLab, as always. The mirror carries the commit to
GitHub, and the workflow runs when `site/`, `src/`, `Cargo.toml` or the
workflow itself changed (or by hand: GitHub → Actions → *Publish the site*
→ *Run workflow*). It:

1. installs Rust and the wasm target, then `hrs` from the same commit;
2. installs the Dioxus CLI, pinned to the newest **0.7.x**, matching the
   site's `dioxus = "0.7"`;
3. transpiles the site's Harsh, runs `dx build --release --platform web`;
4. checks that the Book, *By Example* and the guide are in the output
   (they come from `site/public/`), adds `404.html` so every route loads,
   and deploys.

The first build takes several minutes; later ones are faster, from cache.

**Check it landed:** GitHub → Actions shows the run green, with the page's
address under *deploy*. Open it, then check a few things by hand:

- the home page, and a click to Learn → the Book (the books are static
  files, not routes -- a 404 there means `public/` was not copied);
- the Converter: type Harsh, see Rust;
- the Playground: Run the word count (`the: 3`, `and: 2`, `cat: 1`), then a
  matrix program -- `v~ [1, 3, 4]` printed -- which is the check that
  `hrs_std` builds against the Rust Playground's nalgebra;
- reload on a page other than the home page: it must load (that is the
  `404.html`).

**If the run fails**, the step that failed says where:

- *Install hrs from this checkout*: the same commit fails `cargo install
  --path .` locally too; fix it in the language.
- *Build the site*: a Dioxus or Rust error, the same one `dx build` gives
  on your Mac.
- *Assemble the pages*, at `find target/dx …`: the Dioxus CLI changed where
  it writes its output. Run `dx build --release --platform web` on the Mac,
  find the folder holding `index.html`, and adjust the path in the workflow.
- the push never reached GitHub: the mirror's token lacks *Workflows* (A.1);
  GitLab's mirror settings show the error.

## C. Later: a domain of your own

With a domain -- say `harsh-lang.org` -- the site moves from
`…github.io/harsh-lang/` to the domain's root. In this order:

1. **The site: remove the prefix.** In `site/Dioxus.toml`, delete the line
   `base_path = "harsh-lang"`. (It exists only because a project site lives
   under `/harsh-lang/`; at a domain's root there is no prefix. Locally,
   `dx serve` then serves at `http://127.0.0.1:8080/` too.) Commit and push.

2. **Verify the domain with GitHub first** (protects it from being claimed
   by someone else's Pages site). GitHub: your profile's Settings → Pages →
   *Add a domain*, enter it, and add the TXT record GitHub shows you at your
   DNS provider; wait for *Verified*.

3. **DNS, at your registrar or DNS provider** -- for the apex domain:

   | Type | Name | Value |
   |---|---|---|
   | A | `@` | `185.199.108.153` |
   | A | `@` | `185.199.109.153` |
   | A | `@` | `185.199.110.153` |
   | A | `@` | `185.199.111.153` |
   | AAAA (optional, IPv6) | `@` | `2606:50c0:8000::153`, `…8001::153`, `…8002::153`, `…8003::153` |
   | CNAME | `www` | `harsh-programming-language.github.io` |

   Remove any default record your provider created for `@` first. Or, for
   a subdomain only (`www.harsh-lang.org`), the CNAME alone.

4. **Tell GitHub.** Repository → Settings → Pages → *Custom domain*: enter
   the domain, Save. No `CNAME` file is needed: with a site deployed by a
   workflow, GitHub ignores one.

5. **HTTPS.** Once GitHub has issued the certificate (from minutes to an
   hour or so), tick **Enforce HTTPS** on the same page.

6. **Check.** `dig harsh-lang.org +noall +answer -t A` shows the four
   addresses; the site loads at `https://harsh-lang.org/` and at `www.`;
   the old `…github.io/harsh-lang/` address redirects to the domain.

## Later, when it matters

- **Search engines.** The site renders in the browser, so a crawler that
  runs no JavaScript sees little. Static generation (`dx build --ssg`)
  writes each page's HTML at build time; it is the first improvement once
  the site is live.
- **Another host.** If traffic ever outgrows GitHub Pages, the same build
  output (`dist/`) can be served by Cloudflare's static hosting, which
  meters no bandwidth; only the workflow's last step changes.

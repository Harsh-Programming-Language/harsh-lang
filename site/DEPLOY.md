# Publishing the website

The site in `site/` is published to **GitHub Pages** by the workflow
`.github/workflows/pages.yml`, from the GitHub mirror of the repository. It
is free for a public repository: a soft limit of 100 GB of traffic a month
and 1 GB of site, which throttle rather than bill; the build runs on GitHub
Actions, free for public repositories.

Live at **https://harsh-lang.com/** since 2026-09-26 (part C tells how the
domain is set up). The old address,
`https://harsh-programming-language.github.io/harsh-lang/`, redirects there.

## A. Once, before the first deploy

1. **Let the mirror carry the workflow.** The GitLab push mirror
   authenticates to GitHub with a fine-grained token that has *Contents:
   read and write*. GitHub refuses a push that adds or changes a file under
   `.github/workflows/` unless the token also has **Workflows: read and
   write**. On GitHub: Settings → Developer settings → Personal access
   tokens → Fine-grained tokens → **Generate new token**: resource owner
   `Harsh-Programming-Language`, *Only select repositories* → `harsh-lang`,
   permissions **Contents: Read and write** and **Workflows: Read and
   write** (*Metadata: Read* comes by itself). **Not** *Actions*: that one
   governs workflow runs, not workflow files. Choose a long expiration and
   note the date: an expired token stops the mirror.

   GitLab cannot edit a saved mirror's password, so a new token means a new
   mirror row: GitLab → Settings → Repository → Mirroring repositories →
   delete the old row → **Add new**: URL
   `https://github.com/Harsh-Programming-Language/harsh-lang.git`, direction
   *Push*, *Username and Password* -- the GitHub username, the token as the
   password -- then **Update now**.

   What a token without the permission looks like, in the mirror row's
   *Error* tooltip: `refusing to allow a Personal Access Token to create or
   update workflow .github/workflows/pages.yml without workflow scope`.

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
3. transpiles the site's Harsh, runs `dx bundle --web --ssg --features ssg` -- every page pre-rendered (part D);
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
  it writes its output. Run the build of part D on the Mac (`dx bundle --web --ssg --features ssg`),
  find the folder holding `index.html`, and adjust the path in the workflow.
- the push never reached GitHub: the mirror's token lacks *Workflows* (A.1);
  GitLab's mirror settings show the error.

## C. The domain, harsh-lang.com

Set up on 2026-09-26, at IONOS. What it took, in order, for the record and
for the next domain:

1. **Verify the domain with GitHub first** (so no one else can attach it to
   a Pages site). GitHub, signed in as `Harsh-Programming-Language`: the
   profile's Settings → Pages → *Add a domain* → `harsh-lang.com`; the TXT
   record it shows goes into IONOS (host name: only the part before
   `.harsh-lang.com`); then *Verify*. **Keep that TXT record for good.**

2. **DNS at IONOS** (the domain → DNS). IONOS's *Default Site* service owns
   the domain's first `A`, `AAAA` and `_dep_ws_mutex` records; they cannot be
   deleted, only disabled -- adding or editing a record for `@` offers to
   disable the service: accept. **IONOS allows one A record per host name**
   (each new one replaces the last), so the site has one of GitHub's four
   addresses, which is enough. The records, now:

   | Type | Host name | Points to |
   |---|---|---|
   | A | `@` | `185.199.108.153` |
   | CNAME | `www` | `harsh-programming-language.github.io` |
   | TXT | `_github-pages-challenge-Harsh-Programming-Language` | GitHub's code |

   Plus IONOS's Mail records (MX, SPF, DMARC, DKIM, autodiscover) and
   `_domainconnect`, left alone: they do not touch the website. GitHub's
   four addresses are in its documentation, *Managing a custom domain for
   your GitHub Pages site*; `dig harsh-programming-language.github.io +short`
   shows the same four.

   Check against IONOS's own nameserver, not a cache:
   `dig NS harsh-lang.com +short` names them; then
   `dig @ns1046.ui-dns.com harsh-lang.com +noall +answer -t A`.

3. **The site at the domain's root.** `site/Dioxus.toml` has no
   `base_path`: a github.io *project* address lives under `/harsh-lang/`
   and needs one; a domain's root does not.

4. **Attach the domain to the site.** The **repository's** Settings → Pages
   (not the profile's) → *Custom domain* → `harsh-lang.com` → *Save*; wait
   for *DNS check successful*. No `CNAME` file: a site deployed by a
   workflow ignores it. Then **Run workflow** once, so a build made after
   both changes is deployed.

5. **Wait.** Even with everything right, GitHub's servers answered "Site
   not found" (`Server: GitHub.com`, 404) for a while after the domain was
   attached, while the old address already redirected to it. It cleared on
   its own. If it has not after half an hour: *Remove* the custom domain,
   add it again, *Save*, and *Run workflow*.

6. **HTTPS.** When the certificate is issued, *Enforce HTTPS* stops being
   greyed out on the same page: tick it.

7. **Afterwards**, raise the records' TTL in IONOS from 1 minute (used while
   setting up) to an hour.

**Checking, at any time:**

```sh
curl -sI http://harsh-lang.com | grep -i -E '^(HTTP|server)'        # 200 (or 301 to https), GitHub.com
curl -sI https://harsh-programming-language.github.io/harsh-lang/ | grep -i -E '^(HTTP|location)'   # 301 to the domain
```

A 404 with `Server: nginx` means an old IONOS address, still cached; one
with `Server: GitHub.com` and "Site not found" means step 4 or 5.

## D. Static generation (the deploy, since 2026-09-27)

Pages built in the browser show search engines almost nothing, and show
visitors nothing until the WebAssembly has loaded. The site is published with
every page pre-rendered as HTML: Dioxus 0.7 does it as a *fullstack* app --
its CLI runs the app, asks it for the list of pages (the server function
`static_routes`, in `src/main.hrs`), and saves each rendered page (`dx bundle
--web --ssg`). The browser shows the HTML at once; the WebAssembly loads after
and makes the Converter, the Playground and the jumbotron interactive.

The `build` job adds the `server` feature the fullstack build needs to its
own copy of `Cargo.toml`, builds, and checks each page's HTML holds its
content -- the home page's "Rust without the braces", Learn's "The Harsh
Book", Install's "rust-analyzer" -- before publishing. Tried first on the
user's Mac ("blazingly fast"), then made the deploy by his decision.

**On your Mac**, the same build (a plain `dx serve` stays the web app alone):

```sh
cd site
hrs build
cp Cargo.toml Cargo.toml.orig
sed -i '' '/^ssg = /a\
server = ["dioxus/server", "ssg"]
' Cargo.toml
dx bundle --web --ssg --features ssg
mv Cargo.toml.orig Cargo.toml
public=$(find target/dx -type d -name public | head -n 1)
grep -c "Rust without the braces" "$public/index.html"   # 1 or more: pre-rendered
grep -c "The Harsh Book" "$public/learn/index.html"
python3 -m http.server -d "$public" 8080                  # the pages still work as the app
```

## Later, when it matters

- **Search engines.** The site renders in the browser, so a crawler that
  runs no JavaScript sees little. Static generation (`dx build --ssg`)
  writes each page's HTML at build time; it is the first improvement now
  that the site is live.
- **Another host.** If traffic ever outgrows GitHub Pages, the same build
  output (`dist/`) can be served by Cloudflare's static hosting, which
  meters no bandwidth; only the workflow's last step changes.

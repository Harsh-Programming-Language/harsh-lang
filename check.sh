#!/usr/bin/env bash
# Build the toolchain, transpile every example, compile and run each one, and
# verify the round trip through hrs-from. Requires cargo and a network-reachable
# crates.io on first run (for serde_json, and for axum in the hello example).
set -euo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
WORK="${TMPDIR:-/tmp}/hrs-check"
rm -rf "$WORK" && mkdir -p "$WORK"

say() { printf '\n\033[1m%s\033[0m\n' "$*"; }

say "Building harsh-lang"
cargo build --release
BIN="$ROOT/target/release"

say "The library without features"
# hrs_proc_macro, and after it the parsing crates, build on the pure
# language: lexer, layout, expander, emitter, no dependencies. It stopped
# building that way once without anyone noticing (found 2026-09-24).
cargo check --lib --no-default-features --quiet
( cd hrs_proc_macro && cargo test --quiet 2>&1 | grep -E "^test result" )
( cd hrs_quote && cargo test --quiet 2>&1 | grep -E "^test result" )
( cd hrs_syn && cargo test --quiet 2>&1 | grep -E "^test result" )

say "The website transpiles and is formatted"
# site/ is a Harsh project (Dioxus); its Rust is never built here -- the
# container's Rust is too old for Dioxus -- but every .hrs must transpile
# and match the formatter. `hrs cargo metadata` transpiles and builds nothing.
( cd site && "$BIN/hrs" cargo metadata --no-deps --format-version 1 > /dev/null && "$BIN/hrs" fmt --check > /dev/null && echo "  site/src: $(find src -name '*.hrs' | wc -l | tr -d ' ') files transpiled, formatted" )

say "The website highlights as the books do"
# site/src/highlight.hrs is a port of docs/build.py's highlighter (the user's
# choice, 2026-09-26); every code block of the books must come out identical.
python3 site/check-highlight.py

say "The website's editor keys"
# Enter, Tab, brackets and pairs in site/src/components/editor.hrs, driven by
# simulated keys against the language's own columns (skipped without node).
python3 site/check-editor.py

say "The website's samples fit their panes"
# Every line of a code sample is at most 52 characters, indentation
# included: 430px at 13.5px monospace, the width each side-by-side pane
# gives its text (the user's rule, 2026-09-26; site/assets/main.css).
python3 - site/src/samples.hrs <<'PY'
import re, sys
text = open(sys.argv[1]).read()
bad = [(name, n, len(line))
       for name, body in re.findall(r'pub const (\w+): &str = r#"(.*?)\n"#', text, re.S)
       for n, line in enumerate(body.split('\n'), 1) if len(line) > 52]
for name, n, w in bad:
    print(f"  {name}, line {n}: {w} characters, over 52")
if bad:
    sys.exit(1)
print("  every sample line within 52 characters")
PY

say "Transpiling examples"
for f in general edge params brackets; do
    "$BIN/hrs" "examples/$f.hrs" -o "$WORK/$f.rs" --map "$WORK/$f.map.json"
    echo "  $f.hrs -> $f.rs"
done

say "Compiling and running each example"
for f in general edge params brackets; do
    mkdir -p "$WORK/$f-app/src"
    cat > "$WORK/$f-app/Cargo.toml" <<EOF
[package]
name = "${f}_app"
version = "0.1.0"
edition = "2021"
EOF
    cp "$WORK/$f.rs" "$WORK/$f-app/src/main.rs"
    ( cd "$WORK/$f-app" && cargo build --quiet && ./target/debug/${f}_app > "$WORK/$f.out" )
    echo "  $f: ok ($(wc -l < "$WORK/$f.out") lines of output)"
done

say "Round trip: Rust -> Harsh -> Rust"
for f in general edge params brackets; do
    "$BIN/hrs-from" "$WORK/$f.rs" -o "$WORK/$f.back.hrs"
    "$BIN/hrs" "$WORK/$f.back.hrs" -o "$WORK/$f.back.rs"
    if diff -q "$WORK/$f.rs" "$WORK/$f.back.rs" > /dev/null; then
        echo "  $f: byte-identical"
    elif diff -q <(tr -d ' \n' < "$WORK/$f.rs") <(tr -d ' \n' < "$WORK/$f.back.rs") > /dev/null; then
        # The converter re-flows continuation lines and drops blank lines;
        # the tokens must still be the same.
        echo "  $f: identical up to whitespace (the converter re-flows layout)"
    else
        echo "  $f: DIFFERS beyond whitespace"
        diff "$WORK/$f.rs" "$WORK/$f.back.rs" | head -20
        exit 1
    fi
done

say "Self-host: convert harsh-lang's own source and rebuild it"
cp -r "$ROOT/src" "$ROOT/Cargo.toml" "$ROOT/Cargo.lock" "$WORK/" 2>/dev/null || true
mkdir -p "$WORK/selfhost" && cp -r "$ROOT/src" "$ROOT/Cargo.toml" "$ROOT/Cargo.lock" "$WORK/selfhost/"
( cd "$WORK/selfhost"
  for f in src/*.rs src/bin/*.rs; do
      "$BIN/hrs-from" "$f" -o "${f%.rs}.hrs"
      "$BIN/hrs" "${f%.rs}.hrs" -o "$f"
  done
  cargo build --release --quiet
  echo "  self-hosted build: ok"
  for f in general edge params brackets; do
      ./target/release/hrs "$ROOT/examples/$f.hrs" -o "$WORK/sh-$f.rs"
      if diff -q "$WORK/sh-$f.rs" "$WORK/$f.rs" > /dev/null; then
          echo "  self-hosted $f output: identical"
      else
          echo "  self-hosted $f output: DIFFERS"
      fi
  done )

say "Diagnostic remapping"
mkdir -p "$WORK/diag/src"
cat > "$WORK/diag/Cargo.toml" <<'EOF'
[package]
name = "diag"
version = "0.1.0"
edition = "2021"
EOF
sed 's|let mut n: i64 = 0|let mut n: \&str = 0|' examples/general.hrs > "$WORK/bad.hrs"
"$BIN/hrs" "$WORK/bad.hrs" -o "$WORK/diag/src/main.rs" --map "$WORK/diag/map.json"
( cd "$WORK/diag" && cargo build --message-format=json 2>/dev/null \
    | "$BIN/hrs-remap" --map map.json || true )

say "Export: a Harsh project as a plain, formatted Rust crate"
rm -rf "$WORK/exp" && (cd "$WORK" && "$BIN/hrs" new exp > /dev/null)
cp examples/general.hrs "$WORK/exp/src/main.hrs"
(cd "$WORK/exp" && "$BIN/hrs" export > "$WORK/export.log" 2>&1) || { cat "$WORK/export.log"; exit 1; }
grep -q "exported 1 file" "$WORK/export.log" || { cat "$WORK/export.log"; exit 1; }
[ -f "$WORK/exp/target/export/src/main.rs" ] || { echo "no exported main.rs"; exit 1; }
grep -q "target/hrs" "$WORK/exp/target/export/Cargo.toml" && { echo "exported manifest still points at target/hrs"; exit 1; }
(cd "$WORK/exp/target/export" && cargo run --quiet > "$WORK/export.out" 2>&1) || { cat "$WORK/export.out"; exit 1; }
if diff -q "$WORK/export.out" "$WORK/general.out" > /dev/null 2>&1; then
    echo "  exported crate builds with cargo alone and matches general's output"
else
    echo "  exported crate output differs from general's"; diff "$WORK/export.out" "$WORK/general.out" | head; exit 1
fi
grep -q "formatted with cargo fmt" "$WORK/export.log" && echo "  formatted with cargo fmt" || echo "  (rustfmt not installed here; export left unformatted)"

say "All checks complete. Artifacts in $WORK"

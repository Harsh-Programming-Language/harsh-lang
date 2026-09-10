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

say "Building hrust"
cargo build --release
BIN="$ROOT/target/release"

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

say "Self-host: convert hrust's own source and rebuild it"
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

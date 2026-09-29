# Beneath std

*Original: [Beneath std](https://doc.rust-lang.org/nomicon/beneath-std.html)*

Everything so far used the standard library. Code for bare metal, kernels or
embedded devices uses `#![no_std]`: only `core` (and `alloc`, if an allocator
is provided). Such a program must define what happens on a panic — a
function marked `#[panic_handler]` — and, as a binary, its own entry point.

Harsh transpiles such code as it does any other — `#![no_std]` is an
attribute like the rest — but a `no_std` binary cannot be built and run the
way this book runs its programs, so this page has none. The original shows
the pieces; with them written in Harsh's spelling, `hrs build` for an embedded
target works as `cargo build` does.

## 12.1 #[panic_handler]

*Original: [#\[panic_handler\]](https://doc.rust-lang.org/nomicon/panic-handler.html)*

A `no_std` program has no panic machinery of its own: exactly one function in
the final binary must be marked `#[panic_handler]`, taking a
`&core::panic::PanicInfo` and never returning (`-> !`). It may loop, reset the
device, or report over a serial port. In Harsh it is written as any function —
`#[panic_handler]` above `fn panic (info: &PanicInfo) -> !:` — with its body
beneath.

That is the end of the dark arts. Use them sparingly, keep them in small
modules, and write down, next to every `unsafe:`, why it is sound.

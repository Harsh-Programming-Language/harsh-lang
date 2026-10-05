# Uninitialized Memory

Memory that has been allocated but not yet given a value. Reading it is
undefined behaviour; safe Rust makes that impossible, and unsafe code must
keep it so. The original chapter:
[Working With Uninitialized Memory](https://doc.rust-lang.org/nomicon/uninitialized.html).

## 5.1 Checked

*Original: [Checked](https://doc.rust-lang.org/nomicon/checked-uninit.html)*

A variable may be declared before it is given a value, but the compiler
tracks every path and refuses a read that might come first.

```rust harsh
fn main$:
    let x: i32
    let ready = std.env.args$ <- count$ > 5
    if ready:
        x = 1
    // Refused: on the other path, `x` was never given a value.
    println! "{x}"
```

```text
error[E0381]: used binding `x` is possibly-uninitialized
  --> checked.hrs:7:15
   |
 2 |     let x: i32
   |         - binding declared here but left uninitialized
 5 |         x = 1
   |         ----- binding initialized here in some conditions
 7 |     println! "{x}"
   |               ^^^ `x` used here but it is possibly-uninitialized
```

**In Harsh:** the error points at the `.hrs` line of the read.

## 5.2 Drop Flags

*Original: [Drop Flags](https://doc.rust-lang.org/nomicon/drop-flags.html)*

When a value is initialised on some paths only, or moved out on some, the
compiler keeps a hidden *drop flag* to know at run time whether to drop it.

```rust harsh
struct Loud
    name: &'static str

impl Drop for Loud
    fn drop (&mut self):
        println! "drop {}" (self <- name)

fn main$:
    let decide = std.env.args$ <- count$ < 5
    let a
    if decide:
        a = Loud\ name = "a"
    let b = Loud\ name = "b"
    let c = Loud\ name = "c"
    // `c` moved out on this path only: its drop flag decides at the end.
    if decide:
        drop c
    println! "end of main"
    let _keep = &b
```

```text
drop c
end of main
drop b
drop a
```

**In Harsh:** the same; the output shows which values were dropped, and when.

## 5.3 Unchecked

*Original: [Unchecked](https://doc.rust-lang.org/nomicon/unchecked-uninit.html)*

To initialise memory piece by piece — an array element by element, a buffer
from C — use `MaybeUninit<T>`: it holds possibly-uninitialised memory, and
`assume_init` is your promise that every part is now valid.

```rust harsh
use std.mem.MaybeUninit

fn main$:
    // Memory for four Strings, not yet any String in it.
    let mut slots: [MaybeUninit<String>; 4] = std.array.from_fn (|_| MaybeUninit.uninit$)
    for (i, slot) in slots <- iter_mut$ <- enumerate$:
        let _ = slot <- write (format! "item {i}")
    // Our promise: every slot was written.
    let items: [String; 4] = slots <- map (|s| unsafe: s <- assume_init$)
    println! "{:?}" items
```

```text
["item 0", "item 1", "item 2", "item 3"]
```

**In Harsh:** each write is `unsafe:`-free here, since `MaybeUninit.write`
is safe; only the final `assume_init` needs the promise.

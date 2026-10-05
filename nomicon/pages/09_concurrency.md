# Concurrency

Safe Rust makes data races impossible; it does not make every race
impossible. Unsafe code that shares memory between threads must uphold the
same guarantee. The original chapter:
[Concurrency and Parallelism](https://doc.rust-lang.org/nomicon/concurrency.html).

## 8.1 Races

*Original: [Races](https://doc.rust-lang.org/nomicon/races.html)*

A *data race* — two threads touching the same memory unsynchronised, at least
one writing — is undefined behaviour, and safe Rust prevents it. A *race
condition* — a result depending on timing — is only a bug, and safe Rust
allows it. Atomics and locks are how threads share memory without a data race.

```rust harsh
use std.sync.atomic.( AtomicUsize, Ordering)
use std.thread

fn main$:
    let counter = AtomicUsize.new 0
    thread.scope (|s|:
        for _ in 0..4:
            let _ = s <- spawn (||:
                for _ in 0..1000:
                    let _ = counter <- fetch_add 1 Ordering.Relaxed
            )
    )
    // Four threads, no data race: always 4000.
    println! "{}" (counter <- load Ordering.Relaxed)
```

```text
4000
```

**In Harsh:** the threads are spawned in a scope, so they may borrow the
counter; each adds with an atomic `fetch_add`.

## 8.2 Send and Sync

*Original: [Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)*

`Send`: a value may move to another thread. `Sync`: a reference to it may be
shared between threads. The compiler derives both from a type's fields; raw
pointers are neither, so a type built on them states its own, with an
`unsafe impl` — a promise.

```rust harsh
use std.thread

// A raw pointer is neither Send nor Sync: the wrapper promises it is safe to
// send, because the data outlives the thread and nothing else touches it.
struct Pointer (*mut i32)
unsafe impl Send for Pointer

fn main$:
    let mut value = 41
    let p = Pointer (&mut value as *mut i32)
    thread.scope (|s|:
        let _ = s <- spawn (move ||:
            let p = p
            unsafe:
                *p.0 += 1
        )
    )
    println! "{value}"
```

```text
42
```

**In Harsh:** `unsafe impl Send for Pointer` is one line with no body.

## 8.3 Atomics

*Original: [Atomics](https://doc.rust-lang.org/nomicon/atomics.html)*

Atomics carry an *ordering*: `Relaxed` orders nothing but the atomic itself;
`Release` on a store and `Acquire` on the load that sees it make everything
written before the store visible after the load; `SeqCst` adds a single total
order. Most synchronisation is a Release/Acquire pair.

```rust harsh
use std.sync.atomic.( AtomicBool, AtomicU64, Ordering)
use std.thread

static DATA: AtomicU64 = AtomicU64.new 0
static READY: AtomicBool = AtomicBool.new false

fn main$:
    thread.scope (|s|:
        let _ = s <- spawn (||:
            DATA <- store 42 Ordering.Relaxed
            // Release: everything before this store is visible to the
            // thread whose Acquire load sees it.
            READY <- store true Ordering.Release
        )
        let _ = s <- spawn (||:
            while !READY <- load Ordering.Acquire:
                std.hint.spin_loop$
            println! "data = {}" (DATA <- load Ordering.Relaxed)
        )
    )
```

```text
data = 42
```

**In Harsh:** `Ordering.Release`, `Ordering.Acquire` — paths with dots.

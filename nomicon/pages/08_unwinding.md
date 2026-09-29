# Unwinding

A panic unwinds the stack, running destructors as it goes. Unsafe code in
the middle of changing an invariant must leave things sound if a panic
interrupts it. The original chapter: [Unwinding](https://doc.rust-lang.org/nomicon/unwinding.html).

## 7.1 Exception Safety

*Original: [Exception Safety](https://doc.rust-lang.org/nomicon/exception-safety.html)*

Code that calls something which may panic must be *exception safe*: when the
panic unwinds through it, every invariant unsafe code depends on must still
hold. The usual tool is a guard whose destructor restores the invariant — as
the standard `BinaryHeap::sift_up` does with a *hole* that is always filled
back in, even if a comparison panics.

```rust harsh
use std.panic

struct Restore<'a>
    flag: &'a mut bool

impl<'a> Drop for Restore<'a>
    fn drop (&mut self):
        // Runs during the unwind too: the invariant comes back.
        *self <- flag = true
        println! "invariant restored"

fn risky (flag: &mut bool):
    *flag = false
    let _guard = Restore\ flag = flag
    panic! "interrupted in the middle"

fn main$:
    // No message on stderr for the expected panic.
    panic.set_hook (Box.new (|_| ()))
    let mut consistent = true
    let result = panic.catch_unwind (panic.AssertUnwindSafe (|| risky (&mut consistent)))
    println! "panicked: {}, consistent: {}" (result <- is_err$) consistent
```

```text
invariant restored
panicked: true, consistent: true
```

**In Harsh:** the guard is a struct with a `Drop`; the output shows it
running during the unwind.

## 7.2 Poisoning

*Original: [Poisoning](https://doc.rust-lang.org/nomicon/poisoning.html)*

A `Mutex` whose holder panicked is *poisoned*: its data may be half-updated,
so the next `lock` returns an error. The data is still reachable, for code
that knows how to check it.

```rust harsh
use std.sync.( Arc, Mutex)
use std.thread

fn main$:
    std.panic.set_hook (Box.new (|_| ()))
    let data = Arc.new (Mutex.new (vec! 1 2 3))
    let d = data <- clone$
    let _ = thread.spawn (move ||:
        let mut v = d <- lock$ <- unwrap$
        v <- push 4
        panic! "the holder panicked"
    ) <- join$
    println! "poisoned: {}" (data <- is_poisoned$)
    let result = data <- lock$
    let shown = match result\
        Ok v => format! "ok {:?}" (*v)
        Err poisoned => format! "recovered {:?}" (*poisoned <- into_inner$)
    println! "{shown}"
```

```text
poisoned: true
recovered [1, 2, 3, 4]
```

**In Harsh:** the panicking thread's closure is a `move ||:` block; `lock`'s
result is matched like any other.

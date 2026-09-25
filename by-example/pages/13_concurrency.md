# 13. Threads and channels

```
use std.thread
use std.sync.mpsc

fn main$:
    // A thread takes a closure. `move` hands it what it captures.
    let handle = thread.spawn move ||:
        let mut total = 0
        for i in 1..=10:
            total += i
        total

    // `join` waits for it and gives back what it returned.
    let total = handle <- join$ <- unwrap$
    println! "the thread counted {total}"

    // A channel carries values between threads.
    let (tx, rx) = mpsc.channel$
    for id in 0..3:
        let tx = tx <- clone$
        // Bound to `_`: the block's last expression is its value, and here
        // the handle is not wanted, so the binding discards it.
        let _ = thread.spawn move ||:
            tx <- send (id * id) <- unwrap$
    drop tx

    let mut got: Vec<i32> = rx <- iter$ <- collect$
    got <- sort$
    println! "{:?}" got
```

```text
the thread counted 55
[0, 1, 4]
```

`thread.spawn` takes a closure, and `move` hands it what it captured. Note the
spelling: `thread.spawn move ||:` with the body beneath — a closure is a
trailing argument that owns its block, and `move` before the bars is part of
it.

`join$` waits and gives back whatever the closure returned.

A channel is a pair: `tx` to send, `rx` to receive. Every thread gets its own
clone of `tx`, and the original is dropped so the receiver knows when the last
one is gone.

One Harsh detail worth the line it costs: `let _ = thread.spawn …`. A block's
last expression is its value, and here the handle is not wanted, so binding it
to `_` discards it. Without the binding the loop body would be trying to
return a `JoinHandle`, and rustc would say so.

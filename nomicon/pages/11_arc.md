# Implementing Arc and Mutex

*Original: [Implementing Arc and Mutex](https://doc.rust-lang.org/nomicon/arc-mutex/arc-and-mutex.html)*

The original's chapter, despite its title, implements `Arc`; this companion
follows its sections, with the program under *Final Code*.

## 10.1 Arc

*Original: [Arc](https://doc.rust-lang.org/nomicon/arc-mutex/arc.html)*

A pointer to a heap allocation holding the value and an atomic count of
owners, shared across threads.

### 10.1.1 Layout

*Original: [Layout](https://doc.rust-lang.org/nomicon/arc-mutex/arc-layout.html)*

`MyArc<T>` holds a `NonNull<ArcInner<T>>` — the count and the data together —
and a `PhantomData<ArcInner<T>>`, telling the drop checker it owns an
`ArcInner<T>`. It is `Send` and `Sync` only when `T` is both.

### 10.1.2 Base Code

*Original: [Base Code](https://doc.rust-lang.org/nomicon/arc-mutex/arc-base.html)*

`new` boxes the inner value with a count of one and keeps the raw pointer;
`Deref` reaches the data through it.

### 10.1.3 Cloning

*Original: [Cloning](https://doc.rust-lang.org/nomicon/arc-mutex/arc-clone.html)*

Cloning increments the count, `Relaxed` — a new owner needs no
synchronisation with the others — and aborts if the count nears overflow.

### 10.1.4 Dropping

*Original: [Dropping](https://doc.rust-lang.org/nomicon/arc-mutex/arc-drop.html)*

Dropping decrements, `Release`; the owner that brings the count to zero
issues an `Acquire` fence, so it sees every other owner's writes, and frees
the allocation.

### 10.1.5 Final Code

*Original: [Final Code](https://doc.rust-lang.org/nomicon/arc-mutex/arc-final.html)*

```rust harsh
use std.marker.PhantomData
use std.ops.Deref
use std.ptr.NonNull
use std.sync.atomic.( self, AtomicUsize, Ordering)
use std.thread

struct ArcInner<T>
    rc: AtomicUsize
    data: T

pub struct MyArc<T>
    ptr: NonNull<ArcInner<T>>
    phantom: PhantomData<ArcInner<T>>

// Shared across threads: sound when T may be both sent and shared.
unsafe impl<T: Sync + Send> Send for MyArc<T>
unsafe impl<T: Sync + Send> Sync for MyArc<T>

impl<T> MyArc<T>
    pub fn new (data: T) -> MyArc<T>:
        let boxed = Box.new (ArcInner\ rc = AtomicUsize.new 1, data = data)
        MyArc\ ptr = NonNull.new (Box.into_raw boxed) <- unwrap$, phantom = PhantomData

    pub fn count (this: &Self) -> usize:
        let inner = unsafe: this <- ptr <- as_ref$
        inner <- rc <- load Ordering.Acquire

impl<T> Deref for MyArc<T>
    type Target = T
    fn deref (&self) -> &T:
        let inner = unsafe: self <- ptr <- as_ref$
        &inner <- data

impl<T> Clone for MyArc<T>
    fn clone (&self) -> MyArc<T>:
        let inner = unsafe: self <- ptr <- as_ref$
        // Relaxed: a new owner needs no synchronisation with the others.
        let old = inner <- rc <- fetch_add 1 Ordering.Relaxed
        if old >= isize.MAX as usize:
            std.process.abort$
        MyArc\ ptr = self <- ptr, phantom = PhantomData

impl<T> Drop for MyArc<T>
    fn drop (&mut self):
        let inner = unsafe: self <- ptr <- as_ref$
        let old = inner <- rc <- fetch_sub 1 Ordering.Release
        if old != 1:
            return
        // The last owner: see every other owner's writes, then free.
        atomic.fence Ordering.Acquire
        unsafe: drop (Box.from_raw (self <- ptr <- as_ptr$))

struct Payload
    text: String

impl Drop for Payload
    fn drop (&mut self):
        println! "payload freed"

fn main$:
    let shared = MyArc.new (Payload\ text = String.from "shared text")
    let mut handles = Vec.new$
    for i in 0..3:
        let mine = shared <- clone$
        let read = move || format! "thread {i} read {:?}" (mine <- text)
        handles <- push (thread.spawn read)
    for h in handles:
        println! "{}" (h <- join$ <- unwrap$)
    println! "owners left: {}" (MyArc.count (&shared))
```

```text
thread 0 read "shared text"
thread 1 read "shared text"
thread 2 read "shared text"
owners left: 1
payload freed
```

**In Harsh:** the counts' orderings read as they are named; the value is
shared by three threads, and freed once, after the last owner is gone.

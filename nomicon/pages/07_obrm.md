# Ownership Based Resource Management

A value owns its resources — memory, files, locks — and releases them when it
is dropped. It is the pattern everything else in Rust rests on. The original
chapter: [OBRM](https://doc.rust-lang.org/nomicon/obrm.html).

## 6.1 Constructors

*Original: [Constructors](https://doc.rust-lang.org/nomicon/constructors.html)*

There is exactly one way to create a value: name its type and give every
field. `new` functions, `Default`, `Clone` are ordinary code built on that one
way; nothing runs implicitly — no copy constructor, no move constructor. A
move is always a plain copy of the bytes, after which the source is no longer
used.

There is no program on this page: there is nothing hidden to show.

## 6.2 Destructors

*Original: [Destructors](https://doc.rust-lang.org/nomicon/destructors.html)*

`Drop::drop` runs when a value goes out of scope, then its fields are dropped
in declaration order; local variables are dropped in reverse order of
declaration. `mem::forget` and `ManuallyDrop` stop a destructor from running.

```rust harsh
use std.mem.ManuallyDrop

struct Loud
    name: &'static str

impl Drop for Loud
    fn drop (&mut self):
        println! "drop {}" (self <- name)

struct Pair
    first: Loud
    second: Loud

fn main$:
    let _a = Loud\ name = "local a"
    let _b = Loud\ name = "local b"
    let _p = Pair\ first = Loud\ name = "field first", second = Loud\ name = "field second"
    // Never dropped: forgotten, and kept from dropping.
    std.mem.forget (Loud\ name = "forgotten")
    let _m = ManuallyDrop.new (Loud\ name = "manual")
    println! "end of main"
```

```text
end of main
drop field first
drop field second
drop local b
drop local a
```

**In Harsh:** the order of the output is the order the rules predict.

## 6.3 Leaking

*Original: [Leaking](https://doc.rust-lang.org/nomicon/leaking.html)*

Leaking is *safe*: `mem::forget` does it on purpose, and two `Rc`s pointing at
each other do it by accident. So unsafe code must never rely on a destructor
running for soundness — only for tidiness. The original's examples show what
that costs: `Vec::drain` sets the length to zero before it starts, so a
forgotten `Drain` leaks rather than exposes moved-out elements; and the early
`thread::scoped` API was removed because a forgotten guard let a thread
outlive what it borrowed — which `thread::scope`, used in this book, avoids.

```rust harsh
use std.cell.RefCell
use std.rc.( Rc, Weak)

struct Node
    next: RefCell<Option<Rc<Node>>>
    back: RefCell<Weak<Node>>

fn main$:
    // A cycle of strong pointers: neither count ever reaches zero.
    let a = Rc.new (Node\ next = RefCell.new None, back = RefCell.new (Weak.new$))
    let b = Rc.new (Node\ next = RefCell.new (Some (a <- clone$)), back = RefCell.new (Weak.new$))
    *a <- next <- borrow_mut$ = Some (b <- clone$)
    println! "strong: a {} b {} -- a leak when both go" (Rc.strong_count (&a)) (Rc.strong_count (&b))
    // Break it: a Weak pointer back does not keep its target alive.
    *a <- next <- borrow_mut$ = None
    *a <- back <- borrow_mut$ = Rc.downgrade (&b)
    println! "strong: a {} b {}" (Rc.strong_count (&a)) (Rc.strong_count (&b))
```

```text
strong: a 2 b 2 -- a leak when both go
strong: a 2 b 1
```

**In Harsh:** `Weak` breaks the cycle; the counts show it.

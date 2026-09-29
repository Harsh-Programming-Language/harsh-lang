# Meet Safe and Unsafe

Safe Rust guarantees there is no undefined behaviour: no dangling reference,
no data race, no read of uninitialised memory. Unsafe Rust is the same
language with a few more abilities, whose correctness the compiler can no
longer check — you promise it instead. The original chapter:
[Meet Safe and Unsafe](https://doc.rust-lang.org/nomicon/meet-safe-and-unsafe.html).

## 1.1 How Safe and Unsafe Interact

*Original: [How Safe and Unsafe Interact](https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html)*

`unsafe` marks a contract. On a function, it says the caller must uphold
conditions the compiler cannot check; on a block, it says *you* have checked
them. On a trait, it says implementers must uphold an invariant; on an
`impl`, that you have. Safe code must never be able to cause undefined
behaviour, however it calls a safe function — that is the safe function's
author's job.

```rust harsh
// A safe function around an unsafe operation: the check makes it sound.
fn get (xs: &[i32]) (i: usize) -> Option<i32>:
    if i < xs <- len$:
        // Sound: `i` was checked against the length just above.
        Some (unsafe: *xs <- get_unchecked i)
    else:
        None

// An unsafe function: the caller promises what the function cannot check.
unsafe fn first_unchecked (xs: &[i32]) -> i32:
    *xs <- get_unchecked 0

fn main$:
    let xs = [10, 20, 30]
    println! "{:?} {:?}" (get (&xs) 1) (get (&xs) 9)
    // We promise: `xs` is not empty.
    println! "{}" (unsafe: first_unchecked (&xs))
```

```text
Some(20) None
10
```

**In Harsh:** `unsafe fn` is declared like any function; `unsafe:` opens a
block, and a short one fits on its line: `unsafe: *xs <- get_unchecked i`.

## 1.2 What Unsafe Can Do

*Original: [What Unsafe Can Do](https://doc.rust-lang.org/nomicon/what-unsafe-does.html)*

Only a few things: dereference a raw pointer; call an `unsafe` function
(including C functions); implement an `unsafe` trait; access or modify a
`static mut`; access a `union`'s fields. Everything else — the borrow
checker, the type system — still applies inside `unsafe`. What unsafe code
must never do is the original's other list, of undefined behaviours: a
dangling or unaligned dereference, breaking the aliasing rules, a data race,
producing an invalid value (a `bool` that is not 0 or 1, an enum with no
such variant), among others.

```rust harsh
static mut COUNTER: u32 = 0

union IntOrFloat
    i: u32
    f: f32

fn main$:
    // 1. Dereference a raw pointer.
    let x = 5
    let p = &x as *const i32
    println! "{}" (unsafe: *p)
    // 2. Call an unsafe function.
    let v = vec! 1 2 3
    println! "{}" (unsafe: *v <- get_unchecked 2)
    // 3. Access a `static mut` (one thread only here).
    unsafe:
        COUNTER += 1
        println! "{}" COUNTER
    // 4. Read a union's field.
    let u = IntOrFloat\ f = 1.0
    println! "{:#x}" (unsafe: u <- i)
```

```text
5
3
1
0x3f800000
```

**In Harsh:** each power reads as in Rust; the raw-pointer dereference is
`*p`, isolated in parentheses where it is an argument, `(*p)`. Mind its
precedence: a prefix operator takes the whole chain after it, so `*p <- field`
dereferences the *field*; to reach through the pointer, write `(*p) <-
field`. The Book's appendix *Precedence* has the full table.

## 1.3 Working with Unsafe

*Original: [Working with Unsafe](https://doc.rust-lang.org/nomicon/working-with-unsafe.html)*

Unsafe code depends on invariants that *safe* code around it can break: a
`Vec`'s safe method that changes `len` wrongly is as dangerous as the unsafe
read that trusts it. So the boundary of safety is the **module**: keep the
fields that unsafe code relies on private, and let only the module's own
code touch them.

```rust harsh
mod stack
    // `len` is private: only this module's code can change it, so the
    // unsafe read below can trust it.
    pub struct Stack
        items: [i32; 8]
        len: usize

    impl Stack
        pub fn new$ -> Self:
            Self\ items = [0; 8], len = 0

        pub fn push (&mut self) (x: i32) -> bool:
            if self <- len == 8:
                return false
            self <- items[self <- len] = x
            self <- len += 1
            true

        pub fn top (&self) -> Option<i32>:
            if self <- len == 0:
                None
            else:
                // Sound because `len` is always <= 8, which only this
                // module can break.
                Some (unsafe: *self <- items <- get_unchecked (self <- len - 1))

fn main$:
    let mut s = stack.Stack.new$
    s <- push 1
    s <- push 2
    println! "{:?}" (s <- top$)
```

```text
Some(2)
```

**In Harsh:** the module and its private fields are the same `mod` and the
same absence of `pub`.

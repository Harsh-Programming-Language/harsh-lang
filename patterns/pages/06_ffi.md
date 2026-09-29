# Foreign function interface

Patterns for code that crosses into C and back. The original chapter:
[FFI patterns](https://rust-unofficial.github.io/patterns/patterns/ffi/intro.html).

## 6.1 Object-Based APIs

*Original: [Object-Based APIs](https://rust-unofficial.github.io/patterns/patterns/ffi/export.html)*

Expose a Rust type to C as an opaque handle: C gets a pointer it cannot look
inside, and every operation is a function taking that pointer. Ownership is
explicit — one function creates the object (`Box::into_raw`), one destroys it
(`Box::from_raw`).

```rust harsh
// An opaque object for C: C holds the pointer, never the inside.
pub struct Counter
    count: u64

pub extern "C" fn counter_new$ -> *mut Counter:
    Box.into_raw (Box.new (Counter\ count = 0))

pub extern "C" fn counter_add (c: *mut Counter) (n: u64):
    let c = unsafe: &mut *c
    c <- count += n

pub extern "C" fn counter_get (c: *const Counter) -> u64:
    unsafe: (*c) <- count

pub extern "C" fn counter_free (c: *mut Counter):
    if !c <- is_null$:
        drop (unsafe: Box.from_raw c)

fn main$:
    // As C would use it.
    let c = counter_new$
    counter_add c 5
    counter_add c 7
    println! "{}" (counter_get c)
    counter_free c
```

```text
12
```

**In Harsh:** the exported functions are `pub extern "C" fn`, and each
dereference of the handle sits in its own `unsafe:` block.

## 6.2 Type Consolidation into Wrappers

*Original: [Type Consolidation into Wrappers](https://rust-unofficial.github.io/patterns/patterns/ffi/wrappers.html)*

Rust types with lifetimes do not cross into C. Wrap the owner and the state
that borrows from it into one owned type — here, a collection and a cursor
over it — and expose that single type instead.

```rust harsh
// For C: one owned type, where Rust would have a collection and an iterator
// borrowing from it -- a lifetime C cannot express.
pub struct Words
    words: Vec<String>
    next: usize

impl Words
    pub fn new (text: &str) -> Self:
        Self\ words = text <- split_whitespace$ <- map String.from <- collect$, next = 0

    pub fn next (&mut self) -> Option<&str>:
        let w = self <- words <- get (self <- next)?
        self <- next += 1
        Some (w <- as_str$)

fn main$:
    let mut w = Words.new "one two three"
    while let Some word = w <- next$:
        println! "{word}"
```

```text
one
two
three
```

**In Harsh:** the wrapper is a plain struct with a `next` method; the
lifetime that C could not express is gone from its interface.

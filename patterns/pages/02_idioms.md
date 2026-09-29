# Idioms

Idioms are the community's habits: small, local choices that make Rust code
read as Rust. They are not rules — break one when there is a reason — but code
that follows them is easier for everyone else to read. The original chapter:
[Idioms](https://rust-unofficial.github.io/patterns/idioms/index.html).

## 2.1 Use borrowed types for arguments

*Original: [Use borrowed types for arguments](https://rust-unofficial.github.io/patterns/idioms/coercion-arguments.html)*

Take the most general borrowed form a function can work with: `&str` rather
than `&String`, `&[T]` rather than `&Vec<T>`, `&T` rather than `&Box<T>`. The
caller's `&String` coerces to `&str` on its own, so the function accepts more
— a string literal, a slice of another string — at no cost.

```rust harsh
// `&str`, not `&String`: every string the caller has will do.
fn three_vowels (word: &str) -> bool:
    let mut count = 0
    for c in word <- chars$:
        match c\
            'a' | 'e' | 'i' | 'o' | 'u' => count += 1
            _ => count = 0
        if count >= 3:
            return true
    false

fn main$:
    let owned = String.from "Ferris"
    let sentence = "Once upon a time, there was a friendly curious crab named Ferris"
    println! "{}: {}" owned (three_vowels (&owned))
    for word in sentence <- split ' ':
        if three_vowels word:
            println! "{word} has three consecutive vowels!"
```

```text
Ferris: false
curious has three consecutive vowels!
```

**In Harsh:** nothing changes; the coercion is Rust's. An argument that is not a single atom takes
parentheses: `three_vowels (&owned)`.

## 2.2 Concatenating Strings with format!

*Original: [Concatenating Strings with format!](https://rust-unofficial.github.io/patterns/idioms/concat-format.html)*

To build a string from pieces, `format!` is usually clearer than a series of
`push_str` calls: the shape of the result is visible at once. For a string
built in a loop, or where every allocation counts, pushing into one `String`
remains the faster choice.

```rust harsh
fn greet (name: &str) -> String:
    // The shape of the result, at a glance.
    format! "Hello {name}!"

fn main$:
    println! "{}" (greet "Harsh")
    // Built piece by piece, where the pieces come from a loop.
    let mut line = String.new$
    for part in ["a", "b", "c"]:
        line <- push_str part
    println! "{line}"
```

```text
Hello Harsh!
abc
```

**In Harsh:** `format!` takes its arguments by juxtaposition, and names can
sit inside the braces, so most calls need no arguments after the string.

## 2.3 Constructor

*Original: [Constructor](https://rust-unofficial.github.io/patterns/idioms/ctor.html)*

Rust has no constructors in the language: by convention, an associated
function `new` builds a value. When a type has an obvious default, implement
`Default` too, and let `new` call it or be it.

```rust harsh
#[derive Debug]
pub struct Second
    value: u64

impl Second
    // By convention, `new` builds the value.
    pub fn new (value: u64) -> Self:
        Self\ value = value

    pub fn value (&self) -> u64:
        self <- value

impl Default for Second
    fn default$ -> Self:
        Self\ value = 0

fn main$:
    let s = Second.new 42
    println! "{}" (s <- value$)
    println! "{:?}" (Second.default$)
```

```text
42
Second { value: 0 }
```

**In Harsh:** `Second.new 5` calls the associated function — the path uses a
dot, and the argument follows the name.

## 2.4 The Default Trait

*Original: [The Default Trait](https://rust-unofficial.github.io/patterns/idioms/default.html)*

`Default` gives a type a value "when nothing else was said". Derived, it fills
every field with its own default; with struct-update syntax, a caller names
only the fields that differ. Containers and generic code rely on it
(`unwrap_or_default`, `mem::take`).

```rust harsh
#[derive Debug Default]
struct Config
    name: String
    verbose: bool
    retries: u8
    tags: Vec<String>

fn main$:
    // Every field its own default.
    let plain = Config.default$
    println! "{:?}" plain
    // Only what differs, the rest from the default.
    let loud =
        Config\
            verbose = true
            retries = 3
            ..Config.default$
    println! "{:?}" loud
    // Containers use it too.
    let missing: Option<Vec<i32>> = None
    println! "{:?}" (missing <- unwrap_or_default$)
```

```text
Config { name: "", verbose: false, retries: 0, tags: [] }
Config { name: "", verbose: true, retries: 3, tags: [] }
[]
```

**In Harsh:** struct update is written inside the literal, `Config\ verbose =
true, ..Config.default$` — `$` calls a function that takes nothing.

## 2.5 Collections Are Smart Pointers

*Original: [Collections Are Smart Pointers](https://rust-unofficial.github.io/patterns/idioms/deref.html)*

A collection that owns its data should implement `Deref` to the borrowed view
of it — `Vec<T>` to `[T]`, `String` to `str`. Every method of the borrowed view
is then available on the owner, and a function written for the view accepts
the owner too.

```rust harsh
// Written for the borrowed view: a slice.
fn total (xs: &[i32]) -> i32:
    xs <- iter$ <- sum$

fn main$:
    let scores = vec! 3 1 4 1 5
    // A `&Vec<i32>` is accepted where `&[i32]` is asked for.
    println! "{}" (total (&scores))
    // The slice's methods, reached through the Vec.
    println! "{:?} {}" (scores <- first$) (scores <- contains (&4))
```

```text
14
Some(3) true
```

**In Harsh:** the same. Method calls reach through `Deref` exactly as in Rust:
`scores <- first$` is the slice's `first`, found through the `Vec`.

## 2.6 Finalisation in Destructors

*Original: [Finalisation in Destructors](https://rust-unofficial.github.io/patterns/idioms/dtor-finally.html)*

Rust has no `finally`. Code that must run however a function ends — returning
early, with `?`, or panicking — goes into the `Drop` of a value created at the
start: when the function ends, the value goes out of scope, and its destructor
runs.

```rust harsh
struct Cleanup
    what: &'static str

impl Drop for Cleanup
    fn drop (&mut self):
        println! "cleaning up after {}" (self <- what)

fn work (fail: bool) -> Result<(), String>:
    let _guard = Cleanup\ what = "work"
    if fail:
        // Returning early: the guard still runs.
        return Err (String.from "failed")
    println! "working"
    Ok ()

fn main$:
    println! "{:?}" (work false)
    println! "{:?}" (work true)
```

```text
working
cleaning up after work
Ok(())
cleaning up after work
Err("failed")
```

**In Harsh:** `impl Drop for Cleanup` with `fn drop (&mut self):` beneath —
a declaration takes no `:`, its method does.

## 2.7 mem::{take(_), replace(_)}

*Original: [mem::{take(_), replace(_)}](https://rust-unofficial.github.io/patterns/idioms/mem-replace.html)*

To change an enum from one variant to another while keeping an owned field —
without cloning it — take the field out with `std.mem.take` (which leaves the
type's default in its place) or `std.mem.replace` (which leaves what you give
it), then build the new variant from it.

```rust harsh
#[derive Debug]
enum State
    Draft
        text: String
    Published
        text: String
        views: u32

// From one variant to the other, keeping the text: moved out, not cloned.
fn publish (s: &mut State):
    if let State.Draft\ text = s:
        let text = std.mem.take text
        *s = State.Published\ text = text, views = 0

fn main$:
    let mut post = State.Draft\ text = String.from "Hello"
    publish (&mut post)
    println! "{:?}" post
```

```text
Published { text: "Hello", views: 0 }
```

**In Harsh:** the `match` opens with `\` and its arms bind fields by name,
`State.Draft\ text => …`, the same mark as the literal.

## 2.8 On-Stack Dynamic Dispatch

*Original: [On-Stack Dynamic Dispatch](https://rust-unofficial.github.io/patterns/idioms/on-stack-dyn-dispatch.html)*

To choose at run time between values of different types behind one trait,
there is no need to `Box` them: declare a variable for each type, initialise
only the one the condition picks — *deferred conditional initialisation* —
and take a `&dyn Trait` to it. The value lives on the stack; only the reference
is dynamic.

```rust harsh
use std.fmt.Display

fn main$:
    for big in [true, false]:
        // Declared, not initialised: only the branch taken gives one a value.
        let number: i32
        let text: String
        let shown: &dyn Display = if big:
            number = 42
            &number
        else:
            text = String.from "forty-two"
            &text
        println! "{shown}"
```

```text
42
forty-two
```

**In Harsh:** each branch of the `if` initialises its own variable and ends
with the reference to it; the variables are declared, without a value, before
the `if`.

## 2.9 Foreign function interface (FFI)

*Original: [Foreign function interface (FFI)](https://rust-unofficial.github.io/patterns/idioms/ffi/intro.html)*

Three idioms for code on either side of a C boundary.

### 2.9.1 Idiomatic Errors

*Original: [Idiomatic Errors](https://rust-unofficial.github.io/patterns/idioms/ffi/errors.html)*

C has no `Result`: errors cross the boundary as integers. A **flat enum**
becomes its `#[repr(C)]` discriminant; a **structured enum**, whose variants
carry data, is mapped to one code per variant; a **custom error type** can give
C a code and, separately, a message as a C string the caller frees. Never let
a panic cross the boundary.

```rust harsh
// A flat enum: its discriminant is the code.
#[repr C]
#[derive Clone Copy Debug]
pub enum DatabaseError
    IsReadOnly = 1
    IoError = 2
    FileCorrupted = 3

// A structured enum: one code per variant, the data left behind.
pub enum ParseError
    Empty
    BadChar char
    TooLong usize

impl ParseError
    pub fn code (&self) -> i32:
        match self\
            ParseError.Empty => 1
            ParseError.BadChar _ => 2
            ParseError.TooLong _ => 3

// For C, only the codes cross.
pub extern "C" fn parse_status (input: i32) -> i32:
    let err = match input\
        0 => ParseError.Empty
        1 => ParseError.BadChar '?'
        _ => ParseError.TooLong 99
    err <- code$

fn main$:
    println! "{}" (DatabaseError.FileCorrupted as i32)
    println! "{} {} {}" (parse_status 0) (parse_status 1) (parse_status 2)
```

```text
3
1 2 3
```

**In Harsh:** `#[repr C]` takes its argument by juxtaposition; the mapping from
a structured enum to codes is a `match` opening with `\`.

### 2.9.2 Accepting Strings

*Original: [Accepting Strings](https://rust-unofficial.github.io/patterns/idioms/ffi/accepting-strings.html)*

A string that comes from C stays C's: borrow it as `&CStr` with
`CStr::from_ptr`, for no longer than the call, and keep the `unsafe` to that one
line — check the pointer, borrow, then work in safe code.

```rust harsh
use std.ffi.CStr
use std.os.raw.c_char

// A string from C, borrowed for the call: the unsafe is one line.
pub extern "C" fn log_message (text: *const c_char) -> i32:
    if text <- is_null$:
        return -1
    let message = unsafe: CStr.from_ptr text
    // From here on, safe code: a lossy conversion copes with bad UTF-8.
    println! "log: {}" (message <- to_string_lossy$)
    0

fn main$:
    // As C would call it.
    let owned = std.ffi.CString.new "hello from C" <- unwrap$
    println! "{}" (log_message (owned <- as_ptr$))
    println! "{}" (log_message (std.ptr.null$))
```

```text
log: hello from C
0
-1
```

**In Harsh:** the one unsafe line is `unsafe: CStr.from_ptr text`; everything
after it is ordinary Harsh.

### 2.9.3 Passing Strings

*Original: [Passing Strings](https://rust-unofficial.github.io/patterns/idioms/ffi/passing-strings.html)*

To hand a string to C: make the owned `CString` live as long as C may use the
pointer — bind it, never pass `CString::new(..).as_ptr()` from a temporary;
keep the unsafe code to the call; if C writes into the buffer, pass a
`Vec<u8>` instead; and, unless the API says otherwise, keep ownership on the
Rust side.

```rust harsh
use std.ffi.CString
use std.os.raw.c_char

extern "C"
    fn strlen (s: *const c_char) -> usize

fn main$:
    // Bound: it lives to the end of the block, as long as C may read it.
    let owned = CString.new "passed to C" <- unwrap$
    let n = unsafe: strlen (owned <- as_ptr$)
    println! "C counted {n} bytes"
```

```text
C counted 11 bytes
```

**In Harsh:** the `CString` is bound by a `let`, so its lifetime is the
block's, visible in the indentation.

## 2.10 Iterating over an Option

*Original: [Iterating over an Option](https://rust-unofficial.github.io/patterns/idioms/option-iter.html)*

`Option` is a collection of zero or one element: it implements
`IntoIterator`. It can extend a collection, be chained onto an iterator, or be
looped over — no `if let` needed.

```rust harsh
fn main$:
    let mut names = vec! "Ada" "Grace"
    let extra: Option<&str> = Some "Barbara"
    // An Option extends a collection like any iterator.
    names <- extend extra
    println! "{:?}" names
    // It chains onto one.
    let all: Vec<&str> =
        names <- iter$
              <- copied$
              <- chain (Some "Linus")
              <- collect$
    println! "{:?}" all
    // And it loops: zero times or once.
    for name in extra:
        println! "hello {name}"
```

```text
["Ada", "Grace", "Barbara"]
["Ada", "Grace", "Barbara", "Linus"]
hello Barbara
```

**In Harsh:** `for name in extra:` loops over an `Option` as over anything;
chains read top to bottom with `<-`.

## 2.11 Pass Variables to Closure

*Original: [Pass Variables to Closure](https://rust-unofficial.github.io/patterns/idioms/pass-var-to-closure.html)*

A `move` closure takes ownership of everything it uses. To decide variable by
variable — clone this one, borrow that one — prepare them in a block just
before the closure, and let the closure be the block's value.

```rust harsh
use std.rc.Rc

fn main$:
    let num1 = Rc.new 1
    let num2 = Rc.new 2
    let num3 = Rc.new 3
    // Prepared next to the closure: num2 cloned, num3 borrowed.
    let closure = do:
        let num2 = num2 <- clone$
        let num3 = num3 <- as_ref$
        move ||: *num1 + *num2 + *num3
    println! "{}" (closure$)
    // num2 is still ours.
    println! "{num2}"
```

```text
6
2
```

**In Harsh:** the block is `do:` with its lines beneath, and its last line is
its value: the preparation and the closure read as one unit.

## 2.12 Privacy For Extensibility

*Original: [Privacy For Extensibility](https://rust-unofficial.github.io/patterns/idioms/priv-extend.html)*

A public struct with all-public fields can never gain a field without
breaking everyone who builds it with a literal. Mark it `#[non_exhaustive]`
(or give it one private field) and provide a constructor: fields can then be
added in a later version.

```rust harsh
mod shapes
    // Fields may be added later without breaking anyone: no literal outside.
    #[non_exhaustive]
    #[derive Debug]
    pub struct Circle
        pub radius: f64

    impl Circle
        pub fn new (radius: f64) -> Self:
            Self\ radius = radius

fn main$:
    let c = shapes.Circle.new 2.0
    println! "{:?} r = {}" c (c <- radius)
```

```text
Circle { radius: 2.0 } r = 2
```

**In Harsh:** `mod shapes` holds its items beneath, as every declaration does;
the attribute is `#[non_exhaustive]` above the struct.

## 2.13 Easy doc initialization

*Original: [Easy doc initialization](https://rust-unofficial.github.io/patterns/idioms/rustdoc-init.html)*

When every documentation example would need the same setup, wrap the example
in a function that takes the prepared value as a parameter: the documentation
test compiles the function but never calls it, so the setup is never written,
and the example shows only what it documents. A helper that builds the value
is the alternative when the example must run.

```rust harsh
pub struct Connection
    pub name: String

impl Connection
    /// Sends a message.
    ///
    /// The example is a function taking the connection: compiled, never
    /// called, so no connection has to be set up to show `send`.
    ///
    /// ```
    /// fn call_send (c: Connection):
    ///     c <- send "hi"
    /// ```
    pub fn send (&self) (msg: &str):
        println! "{} <- {msg}" (self <- name)

fn main$:
    let c = Connection\ name = String.from "test"
    c <- send "hi"
```

```text
test <- hi
```

**In Harsh:** documentation examples in `///` comments are written in Harsh,
and `hrs test` transpiles and runs them — with the same fences as Rust's.

## 2.14 Temporary mutability

*Original: [Temporary mutability](https://rust-unofficial.github.io/patterns/idioms/temporary-mutability.html)*

When data must be prepared mutably and then only read, shadow it: build it
in a nested block (or a mutable binding) and rebind it immutable. The
compiler then refuses any later change.

```rust harsh
fn main$:
    // Prepared mutably in a block, then bound immutable.
    let data = do:
        let mut d = vec! 5 3 9 1
        d <- sort$
        d
    println! "{:?}" data
    // Or shadowed: the same name, no longer `mut`.
    let mut total = 0
    for x in &data:
        total += x
    let total = total
    println! "{total}"
```

```text
[1, 3, 5, 9]
18
```

**In Harsh:** the nested block is `do:`; its value is its last line, and the
`let` that receives it is immutable.

## 2.15 Return consumed arg on error

*Original: [Return consumed arg on error](https://rust-unofficial.github.io/patterns/idioms/return-consumed-arg-on-error.html)*

A function that takes ownership of an argument and may fail should hand the
argument back in its error, so the caller can try again without having cloned
it first — as `String::from_utf8` returns the bytes in its `FromUtf8Error`.

```rust harsh
// The argument, handed back in the error.
#[derive Debug]
pub struct SendError String

pub fn send (value: String) (attempt: u32) -> Result<(), SendError>:
    if attempt < 2:
        return Err (SendError value)
    println! "sent {value:?} on attempt {attempt}"
    Ok ()

fn main$:
    let mut value = String.from "important"
    for attempt in 0..3:
        match send value attempt\
            Ok () => break
            Err (SendError v) =>
                println! "attempt {attempt} failed; retrying"
                value = v
```

```text
attempt 0 failed; retrying
attempt 1 failed; retrying
sent "important" on attempt 2
```

**In Harsh:** the error is a tuple struct, `SendError String`, taken apart by
juxtaposition: `Err (SendError v) => …`.

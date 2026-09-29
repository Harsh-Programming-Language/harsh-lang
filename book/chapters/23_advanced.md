# 23. Advanced features

Everything in this chapter is something a Rust program can go a long way without. Each is here because sooner or later you meet it in a library's source or an error message, and because knowing the edges of the language is part of knowing the language. In order: `unsafe`, the parts of the trait system chapter 10 skipped, the corners of the type system, functions as values, macros — Harsh's and Rust's, yours and imported — and a last word on parentheses.

## 23.1 Unsafe Rust

Every guarantee so far — no dangling references, no data races, no out-of-bounds reads — is enforced by the compiler *refusing programs it cannot prove safe*. Some correct programs cannot be proved safe: talking to the operating system, implementing a data structure with raw pointers, calling C. For those, `unsafe` marks a region where the programmer takes over the proof:

```rust harsh
fn main$:
    let mut num = 5
    let r1 = &num as *const i32          // raw pointers can be made in safe code
    let r2 = &mut num as *mut i32

    unsafe:                              // ...but only dereferenced in an unsafe block
        println! "r1 is: {}" (*r1)
        *r2 = 6
        println! "r2 is: {}" (*r2)
```

```text
r1 is: 5
r2 is: 6
```

A *raw pointer* — `*const T` or `*mut T` — may be created anywhere; it is *dereferencing* one that is unsafe, since nothing guarantees it points at a live value, so that happens inside `unsafe:`. Five things need the keyword: dereferencing a raw pointer, calling an unsafe function, accessing a mutable static, implementing an unsafe trait, and accessing a union's fields. Everything else — the borrow checker, the type checker — stays on inside the block. `unsafe` does not turn Rust off; it turns off five checks, and marks where.

```rust harsh
use std.slice

unsafe fn dangerous$:
    println! "this function is unsafe to call"

// A safe function wrapping unsafe code: the contract is checked, then trusted.

fn split_at_mut (values: &mut [i32]) (mid: usize) -> (&mut [i32], &mut [i32]):
    let len = values <- len$
    let ptr = values <- as_mut_ptr$
    assert! (mid <= len)

    unsafe:
        (slice.from_raw_parts_mut ptr mid, slice.from_raw_parts_mut
                                               (ptr <- add mid)
                                               (len - mid))

fn main$:
    unsafe:
        dangerous$

    let mut v = vec! 1 2 3 4 5 6
    let (a, b) = split_at_mut (&mut v) 3
    a[0] = 10
    b[0] = 40
    println! "{v:?}"
```

```text
this function is unsafe to call
[10, 2, 3, 40, 5, 6]
```

`unsafe fn dangerous$` is a function whose *caller* must uphold some contract, and may only be called from an `unsafe:` block. `split_at_mut` is the pattern that matters: a *safe* function that uses unsafe code inside, after checking the contract itself. Two mutable slices into one vector cannot be expressed to the borrow checker, so the function takes a raw pointer, asserts that `mid` is in range, and builds the slices from raw parts inside `unsafe:` — and its callers never see the keyword, because the function has done the reasoning and stands behind it. This is how the standard library is written, and the discipline to copy: keep `unsafe` small, wrap it in a safe interface, and write down the invariant it depends on.

```rust harsh
extern "C"                             // a foreign block: a layout block like any other
    fn abs (input: i32) -> i32           // a foreign function has no body; the `;` is supplied

static mut COUNTER: u32 = 0             // a mutable global: reading or writing it is unsafe

fn add_to_count inc: u32:
    unsafe:
        COUNTER += inc

fn main$:
    unsafe:
        println! "Absolute value of -3 according to C: {}" (abs (-3))

    add_to_count 3

    unsafe:
        println! "COUNTER: {COUNTER}"
```

```text
Absolute value of -3 according to C: 3
COUNTER: 3
```

`extern "C":` declares functions from another language — a layout block like any other, each foreign signature on its own line with no body — and calling one is unsafe, since Rust cannot check what C does. A `static mut` is a mutable global, and every access to one is unsafe, because two threads could race on it; a `static` without `mut` is safe, and is the usual form. (The `(-3)` is isolated: a negative literal is an operator expression.)

## 23.2 Advanced traits

### Associated types

Chapter 13 showed `Iterator`'s `type Item`. Here is a type implementing it:

```rust harsh
struct Counter
    count: u32

impl Iterator for Counter
    type Item = u32                       // the associated type: what `next` yields

    fn next (&mut self) -> Option<Self.Item>:
        if self <- count < 5:
            self <- count += 1
            Some (self <- count)
        else:
            None

fn main$:
    let sum: u32 =
        (Counter\ count = 0)
            <- zip ((Counter\ count = 0) <- skip 1)
            <- map (|(a, b)| a * b)
            <- filter (|x| x % 3 == 0)
            <- sum$

    println! "{sum}"
```

```text
18
```

`type Item = u32` inside the `impl` fixes what this iterator yields, and `next` returns `Option<Self.Item>`. An associated type is like a type parameter that the *implementation* chooses once, rather than the caller choosing at each use: `Counter` is an iterator of `u32` and nothing else, and callers never write `Iterator<u32>`. That is the difference from generics, and the reason `Iterator` uses one — a type is an iterator over one thing. Having implemented `next`, `Counter` gets every adaptor for free, and the chain in `main` (`zip`, `map`, `filter`, `sum$`) is the proof.

### Operator overloading

`+` is a trait, `Add`, and a type implements it to be addable:

```rust harsh
use std.ops.Add

#[derive Debug Copy Clone PartialEq]
struct Point
    x: i32
    y: i32

impl Add for Point
    type Output = Point

    fn add (self) (other: Point) -> Point:
        Point\ x = self <- x + other <- x, y = self <- y + other <- y

struct Millimeters u32
struct Meters u32

// The default type parameter, `Rhs = Self`, overridden: add a different type.
impl Add<Meters> for Millimeters
    type Output = Millimeters

    fn add (self) (other: Meters) -> Millimeters:
        Millimeters (self.0 + (other.0 * 1000))

fn main$:
    println! "{:?}" ((Point\ x = 1, y = 0) + (Point\ x = 2, y = 3))

    let Millimeters total = Millimeters 500 + Meters 2
    println! "{total}"
```

```text
Point { x: 3, y: 3 }
2500
```

`impl Add for Point` supplies `add` and `type Output`, and `Point + Point` calls it. `Add<Rhs = Self>` has a *default type parameter* — the right-hand side is the same type unless you say otherwise — and `impl Add<Meters> for Millimeters` says otherwise: a millimetre value plus a metre value. All the operators are traits in `std.ops`, and this is the whole mechanism.

### Same name, several traits

Two traits may define a method with the same name, and a type may implement both, and have an inherent method of that name as well:

```rust harsh
trait Pilot
    fn fly (&self)

trait Wizard
    fn fly (&self)

struct Human

impl Pilot for Human
    fn fly (&self):
        println! "This is your captain speaking."

impl Wizard for Human
    fn fly (&self):
        println! "Up!"

impl Human
    fn fly (&self):
        println! "*waving arms furiously*"

trait Animal
    fn baby_name$ -> String

struct Dog

impl Dog
    fn baby_name$ -> String:
        String.from "Spot"

impl Animal for Dog
    fn baby_name$ -> String:
        String.from "puppy"

fn main$:
    let person = Human
    person <- fly$                       // the inherent method wins
    Pilot.fly (&person)                  // a trait's method, by its path
    Wizard.fly (&person)
    println! "A baby dog is called a {}" (Dog.baby_name$)
    println! "A baby dog is called a {}" (<Dog as Animal>.baby_name$)   // fully qualified
```

```text
*waving arms furiously*
This is your captain speaking.
Up!
A baby dog is called a Spot
A baby dog is called a puppy
```

`person <- fly$` calls the inherent method — the type's own wins. A trait's version is called through the trait's path with the receiver as the first argument, `Pilot.fly (&person)`. When the method has no `self` — `baby_name$` is an associated function — there is no receiver to go by, and the *fully qualified* form names both the type and the trait: `<Dog as Animal>.baby_name$`, "Dog's implementation of Animal's baby_name". The angle brackets are Rust's and pass through; the `.` is Harsh's path dot. You will write this rarely and read it in error messages often.

### Supertraits and the newtype pattern

A trait may require another:

```rust harsh
use std.fmt

// A supertrait: OutlinePrint requires Display, and may use it.
trait OutlinePrint: fmt.Display
    fn outline_print (&self):
        let output = self <- to_string$
        let len = output <- len$
        println! "{}" ("*" <- repeat (len + 4))
        println! "*{}*" (" " <- repeat (len + 2))
        println! "* {output} *"
        println! "*{}*" (" " <- repeat (len + 2))
        println! "{}" ("*" <- repeat (len + 4))

struct Point
    x: i32
    y: i32

impl fmt.Display for Point
    fn fmt (&self) (f: &mut fmt.Formatter) -> fmt.Result:
        write! f "({}, {})" (self <- x) (self <- y)

impl OutlinePrint for Point {}
// The newtype pattern: a local wrapper lets us implement a foreign trait on a foreign type.
struct Wrapper (Vec<String>)

impl fmt.Display for Wrapper
    fn fmt (&self) (f: &mut fmt.Formatter) -> fmt.Result:
        write! f "[{}]" (self.0 <- join ", ")

fn main$:
    (Point\ x = 1, y = 3) <- outline_print$

    let w = Wrapper (vec! (String.from "hello") (String.from "world"))
    println! "w = {w}"
```

```text
**********
*        *
* (1, 3) *
*        *
**********
w = [hello, world]
```

`trait OutlinePrint: fmt.Display:` — the first `:` declares the *supertrait*, the second opens the block. Any type implementing `OutlinePrint` must implement `Display`, and `outline_print`'s default may therefore call `to_string$`. `Point` implements `Display` and then `OutlinePrint` with an empty `impl … {}` — Rust's braces for an empty body, since there is nothing to lay out.

The second half is the *newtype* pattern, the answer to chapter 10's orphan rule. `Display` and `Vec` are both foreign, so `impl Display for Vec<String>` is not allowed; wrap the vector in a local tuple struct, `Wrapper (Vec<String>)`, and implement `Display` for that. The wrapper costs nothing at run time and `self.0` reaches the vector. It is also the way to give a value a distinct type — `Millimeters` and `Meters` above — so the compiler keeps units apart that are the same `u32` underneath.

## 23.3 Advanced types

```rust harsh
use std.fmt

// A type alias: a synonym, not a new type.
type Kilometers = i32
type Thunk = Box<dyn Fn$ + Send + 'static>

fn takes_long_type f: Thunk:
    f$

// The never type: `!` for something that does not return.
fn bar$ -> !:
    panic! "never returns"

fn main$:
    let x: i32 = 5
    let y: Kilometers = 5
    println! "x + y = {}" (x + y)              // the same type, so they add

    let f: Thunk = Box.new (|| println! "hi")
    takes_long_type f

    // `continue`, `panic!` and `loop` have type `!`, so a match arm may use them
    // where a value of any type is expected:
    let guess = "3"

    let n: u32 =
        match guess <- trim$ <- parse$\
            Ok num => num
            Err _ => bar$

    println! "{n}"

    // Dynamically sized types must sit behind a pointer: `str` is one, `&str` is
    // pointer plus length. A generic `T` is implicitly `T: Sized`; relax it with `?Sized`:
    fn generic<T: ?Sized + fmt.Debug> t: &T:
        println! "{t:?}"

    generic "a str, unsized"
```

```text
x + y = 10
hi
3
"a str, unsized"
```

A *type alias* is a name for an existing type, not a new one: `Kilometers` *is* `i32`, so the two add, and the alias buys nothing but readability — which is exactly what `Thunk` buys for a long trait-object type written many times. The *never type* `!` is the type of an expression that does not return: `panic!`, `continue`, `loop` without `break`, `process.exit`. It is why a `match` arm can be `Err _ => bar$` next to `Ok num => num` — `!` coerces to any type, so the arms agree — and why `continue` worked in chapter 2's guessing game.

A *dynamically sized type* is one whose size is not known at compile time: `str` (not `&str`), `[T]`, `dyn Trait`. They can only be used behind a pointer that carries the size — `&str` is a pointer and a length, `Box<dyn Trait>` a pointer and a vtable. Every generic `T` is implicitly `T: Sized`; `T: ?Sized` relaxes that, and then `T` must be used through a reference, as `generic` does.

## 23.4 Functions and closures as values

```rust harsh
fn add_one x: i32 -> i32:
    x + 1

// `fn i32 -> i32` is a function pointer type: takes any fn or non-capturing closure.
fn do_twice (f: fn i32 -> i32) (arg: i32) -> i32:
    f arg + f arg

// Returning a closure: an opaque `impl Fn`, or a boxed trait object when the
// concrete type varies.
fn returns_closure$ -> impl Fn i32 -> i32:
    |x| x + 1

fn returns_initialized_closure init: i32 -> Box<dyn Fn i32 -> i32>:
    if init > 0: Box.new (move |x| x + init) else: Box.new (move |x| x - init)

#[derive Debug]
enum Status
    Value u32
    Stop

fn main$:
    println! "{}" (do_twice add_one 5)

    // A tuple-struct or variant constructor is a function too: pass it where a closure is wanted.
    let strings: Vec<String> =
        [1, 2, 3] <- iter$
                  <- map (ToString.to_string)
                  <- collect$
    println! "{strings:?}"

    let statuses: Vec<Status> =
        (0u32..3) <- map Status.Value <- collect$
    println! "{statuses:?} {:?}" Status.Stop

    let handlers = vec! returns_closure$ returns_closure$

    for h in handlers:
        println! "{}" (h 1)

    let f = returns_initialized_closure 10
    println! "{}" (f 1)
```

```text
12
["1", "2", "3"]
[Value(0), Value(1), Value(2)] Stop
2
2
11
```

`fn(i32) -> i32` is a *function pointer* type, and `do_twice` takes one: any named function, or a closure that captures nothing. Prefer the `impl Fn` bounds of chapter 13 in your own signatures, since they accept capturing closures too; `fn` is for interfacing with code that needs a plain pointer, C among it. The two `map` calls show a useful fact: every tuple-struct or enum-variant constructor *is* a function, so `map Status.Value` builds a `Status` from each number, and `map (ToString.to_string)` applies a trait method by its path.

Returning a closure needs a type for it, and a closure's type has no name. `impl Fn(i32) -> i32` in return position is the usual answer: the caller gets *some* closure. When different calls return different closures — the `if` returns one of two, with different captures — they are different types and `impl Fn` cannot name both; box them as `Box<dyn Fn(i32) -> i32>`, a trait object, and the type is the same either way. Chapter 21's rule, applied to closures.

## 23.5 Harsh macros (~)

A macro is code that writes code before your program is compiled. Which macro you are looking at depends on two things: whether it is written in Harsh or in Rust — the mark says so, `name~` or `name!` — and whether it is yours or comes from a library. This section is about the macros you write in Harsh; 23.6 is about the ones you write in Rust inside a Harsh project, which are still yours to write; 23.7 and 23.8 are about the ones you import, from Harsh libraries and from Rust ones.

A macro written `name~` is **Harsh's**: it is defined with `macro_rules~`, its matchers and its body are Harsh, and `hrs` unfolds it *into Harsh* before anything is transpiled — so what it produces is ordinary Harsh, meeting the rules of this book. A macro written `name!` is **Rust's**: `println!`, `vec!`, `format!` and every macro a Rust crate exports. You call it with Harsh syntax, and Rust unfolds it.

Harsh's own macros come in two families, as Rust's do. A **declarative** macro is a set of patterns and what each expands to. A **procedural** macro is a program: an ordinary function that receives the call's tokens and returns the code to put in its place. Both are called `name~`, so a call reads the same whichever kind implements it, and a macro may change from one to the other without touching its callers.

### Declarative macros

A declarative Harsh macro is a set of patterns and what each expands to:

```rust harsh
// A declarative macro: pattern-matched at compile time. Both sides are
// written in Harsh. The matcher is a parameter list -- one group per fragment,
// a repetition of groups for a list -- and the transcriber is a `do:` block.
#[macro_export]
macro_rules~ my_vec
    ( $( ($x:expr) )* ) => do:
        do:
            let mut temp_vec = Vec.new$
            $(temp_vec <- push $x)*
            temp_vec

// Two arms, and a repetition that spans lines. The inner `do:` makes the
// expansion a block with a value, as the Rust `{ { .. } }` would.
macro_rules~ sum
    () => do: 0
    ( ($h:expr) $( ($t:expr) )* ) => do:
        $h + sum~ $( $t )*

fn main$:
    let v: Vec<u32> = my_vec~ 1 2 3
    println! "{v:?}"
    println! "{}" (sum~ 1 2 3 4)
```

```text
[1, 2, 3]
10
```

The macro unfolds **into Harsh**, before anything is transpiled: `my_vec~ 1 2 3` becomes the block the transcriber describes, and the generated Rust holds no macro at all — only what the expansion came to. You can see that middle step with `hrs expand`, and any mistake in a macro shows up as a mistake in the Harsh it produced, on lines you can read.

Two rules come with that, and both are Rust's. A name the macro introduces is its own: a `let tmp` in a transcriber never captures the caller's `tmp`, and where both would stand in one scope the macro's is the one that moves. And a name the *caller* is meant to use must be passed in — `($name:ident)` — since a macro cannot invent a binding for someone else's code.

A call to a Harsh macro is a *stream*, not a function call: everything after `name~` to the end of the line reaches the matcher exactly as written — commas, parens, all of it — so a Harsh macro juxtaposes its arguments, `my_macro~ (1+3) "more"`, and writes commas only where its DSL spells a construct that owns them. A repetition in a matcher reads like a regular expression: `$( … )*` zero or more, `$( … )+` one or more, `$( … )?` at most once, with the separator written just before the marker — `$( ($x:expr) ),*` is expressions separated by commas. A trailing separator the caller may or may not write is a second, optional repetition of the bare comma: `$( ($x:expr) ),* $(,)?`. A call inside a larger expression is isolated, `((twice~ 4), 0)`, as any application is.

Both sides of the macro are written in Harsh, and both follow rules you already know. The matcher `( $( ($x:expr) )* )` is a parameter list: one group per fragment, and a repetition of groups is a list of them — the same brick as `fn f (a: T) (b: U)`, applied a third time. The call `my_vec! 1 2 3` is an ordinary application, one atom per argument. The transcriber is a `do:` block; its statements take their `;` from the layout, `$x` is an atom like any other so `push $x` is a call, and a repetition `$( … )*` that is the whole of its line repeats a statement. The inner `do:` is there because a macro's expansion is a run of tokens, not a block: for the expansion to *be* a block with a value, the block must be written, as Rust writes `{ { … } }`. `sum!` shows an arm with nothing to match and a repetition that spans lines, `$(` on one line, its body beneath, `)*` back under it. On the call side, `$( ($t) )*` in an argument position is a list of arguments, the mirror of the matcher; a repetition of bare tokens, `$($arg)*`, is copied as it stands and is how a macro forwards `tt`s.

> **Harsh —** A matcher may hold brackets or braces inside the arm's own parentheses — `([ ($elem:expr) ; ($n:expr) ])` — and their tokens are matched exactly as written, because that is the shape the call side has too: `filled~ [0u8; 4]`. A metavariable is always written in its parentheses, `($elem:expr)`; every other token of a matcher is literal. The same licence covers a transcriber whose output is a language of its own rather than Harsh.

### Procedural macros

A procedural macro is a program: a function that receives tokens and returns the code to put in their place. Harsh's are written in Harsh, in a crate of their own marked `proc-macro = true` under `[package.metadata.harsh]`, which the crate that calls them lists by path; `hrs` runs them when it transpiles the code that calls them. Each is a `pub fn` from a `TokenStream` to a `TokenStream`, marked with an attribute that names its kind, as Rust's are — `#[some_attribute~]` above `pub fn some_name (input: TokenStream) -> TokenStream:`, in the Rust Book's words — and there are three kinds, below: custom derive, attribute-like, and function-like.

Two differences from a declarative macro come with running a program. A procedural macro is not hygienic: a name it introduces is visible to the caller, since it produced text rather than patterns. And it lives in a separate crate, because `hrs` must build and run it before it can transpile the code that calls it; `hrs build` does that for you, and does it again only when the macro changes.

#### Custom derive Macros

A custom derive works as Rust's does, and the Rust Book's own example reads almost line for line in Harsh. The trait, `HelloMacro`, is its user's; the derive, in a crate of its own, parses the item it is given into a syntax tree with `hrs_syn` — Harsh's `syn` — and builds the implementation with `quote~` — Harsh's `quote`:

`hello_macro_derive/Cargo.toml`

```text
[dependencies]
hrs_proc_macro = "0.2"
hrs_quote = "0.1"
hrs_syn = "0.1"

[package.metadata.harsh]
proc-macro = true
```

`hello_macro_derive/src/lib.hrs`

```rust harsh
use hrs_proc_macro.TokenStream
use hrs_quote.quote
use hrs_syn.{parse_macro_input, DeriveInput}

#[proc_macro_derive~ HelloMacro]
pub fn hello_macro_derive (input: TokenStream) -> TokenStream:
    // Construct a representation of Harsh code as a syntax tree
    // that we can manipulate.
    let ast = parse_macro_input! { input as DeriveInput }

    // Build the trait implementation.
    impl_hello_macro (&ast)

fn impl_hello_macro (ast: &DeriveInput) -> TokenStream:
    let name = &ast <- ident
    let generated = quote~ do:
        impl HelloMacro for #name
            fn hello_macro$:
                println! "Hello, Macro! My name is {}!" (stringify! #name)
    generated
```

`app/Cargo.toml`

```text
[package.metadata.harsh]
proc-macros = ["../hello_macro_derive"]
```

`app/src/main.hrs`

```rust harsh
trait HelloMacro
    fn hello_macro$

#[derive~ HelloMacro]
struct Pancakes

fn main$:
    Pancakes.hello_macro$
```

```text
$ cd app && hrs run
Hello, Macro! My name is Pancakes!
```

`parse_macro_input!` turns the tokens into a `DeriveInput`: the item's attributes, its visibility, its name, its generics, and its fields or variants. `quote~ do:` takes a template beneath it, laid out as the output should read, and `#name` puts the value of `name` in its place. What the derive returns is added after the item, which it cannot change. It is registered with `#[proc_macro_derive~ HelloMacro]` on a `pub fn`, and called `#[derive~ HelloMacro]` above a `struct`, an `enum` or a `union`.

A derive usually works field by field. This one lists a struct's fields, leaving out those marked with a *helper attribute* it declares — `(attributes describe)` in its registration, as Rust's derive declares `attributes(describe)`:

`describe/Cargo.toml`

```text
[dependencies]
hrs_proc_macro = "0.2"
hrs_quote = "0.1"
hrs_syn = "0.1"

[package.metadata.harsh]
proc-macro = true
```

`describe/src/lib.hrs`

```rust harsh
use hrs_proc_macro.TokenStream
use hrs_quote.quote
use hrs_syn.{parse_macro_input, Data, DeriveInput, Error, Field}

/// `describe$`: the type's name and its fields -- all but those marked
/// `#[describe skip]`.
#[proc_macro_derive~ Describe (attributes describe)]
pub fn describe (input: TokenStream) -> TokenStream:
    let ast = parse_macro_input! { input as DeriveInput }
    let name = &ast <- ident
    let fields = match &ast <- data\
        Data.Struct s => &s <- fields
        _ => return (Error.new (name <- span$) "`Describe` is for a struct") <- to_compile_error$
    let shown: Vec<String> =
        fields <- iter$
               <- filter (|f| !skipped f)
               <- map name_of
               <- collect$
    quote~ do:
        impl #name
            pub fn describe$ -> String:
                let fields: Vec<&str> = vec! #(#shown)*
                format! "{} {{ {} }}" (stringify! #name) (fields <- join ", ")

// A field marked `#[describe]` is left out.
fn skipped (f: &Field) -> bool:
    f <- attrs
      <- iter$
      <- any (|a| a <- is "describe")

fn name_of (f: &Field) -> String:
    f <- ident
      <- as_ref$
      <- unwrap$
      <- to_string$
```

`app/Cargo.toml`

```text
[package.metadata.harsh]
proc-macros = ["../describe"]
```

`app/src/main.hrs`

```rust harsh
#[derive Debug]
#[derive~ Describe]
struct Point
    x: i32
    #[describe skip]
    secret: i32
    y: i32

fn main$:
    let p =
        Point\
            x = 1
            secret = 7
            y = 2
    println! "{}" (Point.describe$)
    println! "{:?}" p
    println! "{}" (p <- x + p <- secret + p <- y)
```

```text
$ cd app && hrs run
Point { x, y }
Point { x: 1, secret: 7, y: 2 }
10
```

`#(#shown)*` is a repetition, from `quote`: it repeats what it encloses once for each element of `shown`, here each field's name, as a string. The helper, `#[describe skip]`, is the derive's to read, and is gone from the item once the derive has run, since the compiler would not know what it means. The derive receives the item as written, with every derive attribute removed — its own and Rust's alike — which is why `#[derive Debug]` sits beside it and still works. Several derives run in the order written, each on the same item.

#### Attribute-Like Macros

An attribute-like macro defines an attribute of its own, and may stand on any item — a function, a module, a `struct` — where a derive stands only on types. What it returns *replaces* the item. The Rust Book's example is a web framework's `route`, which here logs each call of the handler it stands on:

`web/Cargo.toml`

```text
[dependencies]
hrs_proc_macro = "0.2"
hrs_quote = "0.1"
hrs_syn = "0.1"

[package.metadata.harsh]
proc-macro = true
```

`web/src/lib.hrs`

```rust harsh
use hrs_proc_macro.TokenStream
use hrs_quote.quote
use hrs_syn.{parse_macro_input, ItemFn}

/// Logs each call of the function it stands on, with the route it serves.
#[proc_macro_attribute~]
pub fn route (attr: TokenStream) (item: TokenStream) -> TokenStream:
    let route = attr <- to_string$
    let f = parse_macro_input! { item as ItemFn }
    let vis = &f <- vis
    let sig = &f <- sig
    let name = f <- ident <- to_string$
    let body = &f <- block
    quote~ do:
        #vis #sig:
            println! "[ROUTE LOG] Dispatched handler '{}' for {}" #name #route
            #body
```

`app/Cargo.toml`

```text
[package.metadata.harsh]
proc-macros = ["../web"]
```

`app/src/main.hrs`

```rust harsh
#[route~ POST "/api/v1/submit"]
fn handle_submit$:
    println! "Processing payload..."

fn main$:
    handle_submit$
```

```text
$ cd app && hrs run
[ROUTE LOG] Dispatched handler 'handle_submit' for POST "/api/v1/submit"
Processing payload...
```

The function is marked `#[proc_macro_attribute~]` and takes two streams, as Rust's does: the attribute's arguments as written — `POST "/api/v1/submit"` — and the item. With no arguments, `#[route~]`, the first is empty. `hrs_syn`'s `ItemFn` parses the function into its signature, name and body, and the template rebuilds it with the logging line at the top. The macro receives the item with every other attribute it carries, those above its own and those below, and must give back whatever of them it wants to keep, since its output is all that remains. Attribute-like macros run before derives, so a derive on the same item sees what they made of it.

#### Function-Like Macros

A function-like macro is called like a declarative one, `name~ stream`. It is an ordinary Harsh function marked `#[proc_macro~]`, taking the call's tokens and returning the code to put in their place:

`hello/Cargo.toml`

```text
[dependencies]
hrs_proc_macro = "0.2"

[package.metadata.harsh]
proc-macro = true
```

`hello/src/lib.hrs`

```rust harsh
use hrs_proc_macro.TokenStream

#[proc_macro~]
pub fn hello_macro (input: TokenStream) -> TokenStream:
    let input_str = input <- to_string$
    let output = format! "\"Hello, {}!\"" input_str
    output <- parse$ <- unwrap$
```

`app/Cargo.toml`

```text
[package.metadata.harsh]
proc-macros = ["../hello"]
```

`app/src/main.hrs`

```rust harsh
fn main$:
    println! "{}" (hello_macro~ world)
    let greeting = hello_macro~ Harsh readers
    println! "{}" greeting
```

```text
$ cd app && hrs run
Hello, world!
Hello, Harsh readers!
```

`hello_macro~ Harsh readers` hands the function its tokens as text, `Harsh readers`, exactly as written; what it returns is Harsh, which `hrs` reads and puts where the call stood, at the call's column. So the macro runs when `hrs` transpiles the caller, not when the program runs, and the generated Rust holds no macro at all — only the string it produced. `hrs expand` shows a file with its proc macros expanded; if a macro returns something that is not Harsh, or fails, the error names the macro and points at the call.

## 23.6 Rust macros (!)

The macros Rust lets you write are still yours to write in a Harsh project, and they remain Rust macros, written in Rust. Their reference is the Rust Book's chapter on them, [*Macros*](https://doc.rust-lang.org/stable/book/ch20-05-macros.html); each part below links to its section, and adds only what is particular to Harsh — where the code lives, and how Harsh calls it. A Rust macro is always called with Rust's mark, `!`, by the rules of 23.8.

### Declarative macros

The Rust Book: [*Declarative Macros for General Metaprogramming*](https://doc.rust-lang.org/stable/book/ch20-05-macros.html#declarative-macros-for-general-metaprogramming).

In Harsh, a `macro_rules!` may be written inside a Harsh file. It is a zone of Rust there: `hrs` copies it out byte for byte, its braces delimit it, and its body is laid out as Rust, so nothing inside it is Harsh — `#[macro_export]` and the rest mean what they mean in Rust. Only the definition is Rust; its calls are Harsh calls, `square! n`, `square! (n + 1)`:

```rust harsh
macro_rules! square {
    ($x:expr) => { $x * $x };
}

fn main$:
    let n = 7
    println! "{}" (square! n)
    println! "{}" (square! (n + 1))
```

```text
49
64
```

### Procedural macros

The Rust Book: [*Procedural Macros for Generating Code from Attributes*](https://doc.rust-lang.org/stable/book/ch20-05-macros.html#procedural-macros-for-generating-code-from-attributes).

In Harsh, a Rust procedural macro lives where Rust puts it, in a Rust crate that says `proc-macro = true` under `[lib]`, written in Rust — with `syn` and `quote` if you like. A Harsh crate lists it under `[dependencies]` as it would any crate, and brings its macros in with `use`: `use hello_macro_derive.HelloMacro`. `hrs` leaves the calls to rustc, which runs the macro as it runs any.

#### Custom derive Macros

The Rust Book: [*Custom `derive` Macros*](https://doc.rust-lang.org/stable/book/ch20-05-macros.html#custom-derive-macros).

In Harsh, a derive is called by juxtaposition, `#[derive HelloMacro]`, beside Rust's own `#[derive Debug]` and Harsh's `#[derive~ …]` if you like. Here is the Rust Book's `HelloMacro`, used from Harsh: the trait is declared in Harsh, the derive written in Rust — reading the type's name from its tokens, where the Rust Book's parses them with `syn` and builds its output with `quote`:

`hello_macro_derive/Cargo.toml`

```text
[lib]
proc-macro = true
```

`hello_macro_derive/src/lib.rs`

```rs
use proc_macro::TokenStream;

#[proc_macro_derive(HelloMacro)]
pub fn hello_macro_derive(input: TokenStream) -> TokenStream {
    // The item's name, read from its tokens; `syn` would parse it properly.
    let text = input.to_string();
    let name = text.split_whitespace().skip_while(|w| *w != "struct").nth(1).unwrap();
    let name = name.trim_end_matches(';');
    format!(
        "impl HelloMacro for {name} {{ fn hello_macro() {{ println!(\"Hello, Macro! My name is {name}!\"); }} }}"
    )
    .parse()
    .unwrap()
}
```

`app/Cargo.toml`

```text
[dependencies]
hello_macro_derive = { path = "../hello_macro_derive" }
```

`app/src/main.hrs`

```rust harsh
use hello_macro_derive.HelloMacro

trait HelloMacro
    fn hello_macro$

#[derive HelloMacro]
struct Pancakes

#[derive HelloMacro]
struct Waffles

fn main$:
    Pancakes.hello_macro$
    Waffles.hello_macro$
```

```text
$ cd app && hrs run
Hello, Macro! My name is Pancakes!
Hello, Macro! My name is Waffles!
```

#### Attribute-Like Macros

The Rust Book: [*Attribute-Like Macros*](https://doc.rust-lang.org/stable/book/ch20-05-macros.html#attribute-like-macros).

In Harsh, an attribute-like macro is applied as any attribute is, its arguments juxtaposed: `#[route GET "/"]` above `fn index$:` hands the macro two arguments, `GET` and `"/"`, and `#[route]` none. It receives them and the item, and what it returns replaces the item. Parentheses isolate one argument of several tokens, as anywhere: `#[route GET "/" (some_attr = some_value)]` passes three. Around the whole list they would make it one tuple — `#[route (GET, "/")]` hands the macro a single argument, not two.

#### Function-Like Macros

The Rust Book: [*Function-Like Macros*](https://doc.rust-lang.org/stable/book/ch20-05-macros.html#function-like-macros).

In Harsh, a function-like macro whose stream is a list of values is applied like a function, its arguments juxtaposed: `name! a b`. One whose stream is a language of its own — the Rust Book's `sql!` — takes it in braces, with Harsh in holes where it needs any: `sql! { SELECT * FROM posts WHERE id=1 }`. Both are 23.8's rules.

## 23.7 Harsh DSLs (~)

A library written in Harsh exports macros written in Harsh, declarative or procedural, and everything about them is Harsh: each call follows the grammar the library's guide describes, and what it expands to is Harsh, meeting the rules of this book. There is nothing to learn beyond that guide.

The first such library comes with Harsh: the prelude's `g~`, `list~`, `set~` and `dict~` (chapter 15) and `m~` and `v~` (chapter 16) need no `use`. A crate of Harsh procedural macros is listed by path under `proc-macros`, as in 23.5.

## 23.8 Rust DSLs (!)

Most macros you call come from Rust libraries: `println!` and `vec!` from the standard library, a web framework's `view!`, serde's `#[derive Serialize]`, Tokio's `#[tokio.main]`. You call them by Harsh's rules. A macro whose stream is a list of values is applied like a function, its arguments juxtaposed and isolated as any argument is: `vec! 1 2 3`, `matches! a (n if n > 3)`. A derive or an attribute applies by juxtaposition inside its brackets: `#[derive Debug Serialize]`, `#[route "/api/:id"]`, `#[tokio.main (flavor = "multi_thread")]`.

A Rust macro that takes a language of its own — the markup of a web framework's `view!`, the element tree of Dioxus's `rsx!`, a query — takes it in braces, and what is inside the braces is that language, written exactly as its documentation shows. Harsh reads none of it. The only Harsh inside is what you mark: a hole opens with `@:` and always closes with `:@`, and between the two is ordinary Harsh, transpiled in place:

```rust harsh
// A stand-in for a UI framework's `view!`: it swallows the markup and yields
// unit, so this file compiles without a dependency. Only the spelling of the
// call is the point here.
macro_rules! view {
    ( $($t:tt)* ) => { () };
}

fn main$:
    let count = 3
    let items = vec! 1 2 3
    view! {
        <div class="app">
            <p>{count}</p>
            <button on:click={@: move |_| println! "{}" (count + 1) :@}>"+"</button>
            <ul>
                <For each={@: move || items <- clone$ :@} let:item>
                    <li>{item * 2}</li>
                </For>
            </ul>
        </div>
    }
    println! "rendered {count}"
```

```text
rendered 3
```

The markup's own `{count}` needs no mark — a name is the same in both languages — while the handler, written in Harsh, sits in a hole. A hole may span lines: `@:` on the line where it opens, the Harsh beneath at that line's indentation, `:@` where it ends. The same convention serves a tree of braces as it serves markup, because Harsh never learns either:

```rust harsh
// A stand-in for Dioxus's `rsx!`, as `view!` above: it swallows the tree and
// yields unit.
macro_rules! rsx {
    ( $($t:tt)* ) => { () };
}

fn main$:
    let count = 3
    let items = vec! 1 2 3
    rsx! {
        div {
            class: "app",
            onclick: @: move |_| println! "{}" (count + 1) :@,
            "Hello {count}"
            for item in items {
                li { "{item}" }
            }
            Button {
                onclick: @: move |_|:
                    let n = count * 2
                    println! "{n}"
                :@,
                "Reset"
            }
        }
    }
    println! "rendered {count}"
```

```text
rendered 3
```

`for item in items { … }` there is `rsx!`'s own, not Harsh's: inside the braces the macro's grammar rules, and a hole is only for what you want written in Harsh. A hole may hold a macro call with braces of its own, and holes inside those.

Holes follow a few conventions. A hole always closes, with `:@`, even at the end of a line. A literal `@:` in the macro's own language is written `@@:`. `hrs-from` writes the holes when it converts a Rust file, turning each piece of Rust inside a body into Harsh. And a Harsh macro whose expansion contains such a call writes the holes too, since what it produces is Harsh.

## 23.9 Parentheses, once and for all

Every use of parentheses in this book has been one of a short list, and the list is worth stating now that all of it has been seen:

```rust harsh
fn double x: i32 -> i32:
    x * 2

fn main$:
    let a = (1 + 2) * 3          // precedence: needed, and kept
    let b = (1 + 2)              // grouping: optional, and harmless
    let c = (double 4)           // the same: a whole value in parens is just the value
    let t = (1, 2)               // a tuple: the comma is what makes it one
    let u = ()                   // the unit value
    let d = double (a + b)       // isolating one argument
    println! "{a} {b} {c} {t:?} {u:?} {d}"
```

```text
9 3 8 (1, 2) () 24
```

> **Harsh —** Parentheses do three things. They **make a tuple** — the comma does it, and `()` with nothing inside is the unit value. They **set precedence** — `(1 + 2) * 3` — which is mandatory and kept. And they **group** — around a whole value, a whole statement, or one argument of an application — which is optional, costs nothing, and is how an argument that is more than one token is marked as one: `double (a + b)`. Nothing else: parentheses never pass an argument list, and a name written tight against a `(` is an error naming the space. Inside a macro's one-line braces the same rules apply, since a macro's body is Harsh — the one earlier exemption for brace bodies was withdrawn when it proved to be a second syntax in disguise.

## 23.10 What you have

`unsafe:` for five operations the compiler cannot check, wrapped in safe functions that do the checking; `extern "C"` for foreign functions. Associated types for a trait with one choice per implementation; operator traits in `std.ops` with default type parameters; disambiguation by trait path and `<Type as Trait>`; supertraits; the newtype pattern for the orphan rule and for distinct types. Aliases, `!`, and `?Sized`. `fn` pointers, constructors as functions, `impl Fn` and `Box<dyn Fn>` for returned closures. Macros by who wrote them: Harsh's own, `macro_rules~` and procedural macros — function-like and derives — written as ordinary Harsh functions; Rust's, `macro_rules!` in a Harsh file and a proc-macro crate beside it; a Rust library's, called by Harsh's rules, its own language in braces with Harsh holes; and a Harsh library's, Harsh throughout.

Next, and last: a multithreaded web server, built from the standard library alone — the book's closing project.

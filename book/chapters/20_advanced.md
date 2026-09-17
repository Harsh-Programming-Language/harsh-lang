# 20. Advanced features

Everything in this chapter is something a Rust program can go a long way without. Each is here because sooner or later you meet it in a library's source or an error message, and because knowing the edges of the language is part of knowing the language. In order: `unsafe`, the parts of the trait system chapter 10 skipped, the corners of the type system, functions as values, and macros.

## 20.1 Unsafe Rust

Every guarantee so far — no dangling references, no data races, no out-of-bounds reads — is enforced by the compiler *refusing programs it cannot prove safe*. Some correct programs cannot be proved safe: talking to the operating system, implementing a data structure with raw pointers, calling C. For those, `unsafe` marks a region where the programmer takes over the proof:

```
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

```
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

    let mut v = vec! [1, 2, 3, 4, 5, 6]
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

```
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

## 20.2 Advanced traits

### Associated types

Chapter 13 showed `Iterator`'s `type Item`. Here is a type implementing it:

```
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

```
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

```
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

```
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

    let w = Wrapper (vec! [String.from "hello", String.from "world"])
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

## 20.3 Advanced types

```
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

    let n: u32 = match guess <- trim$ <- parse$:
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

## 20.4 Functions and closures as values

```
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

    let handlers = vec! [returns_closure$, returns_closure$]

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

Returning a closure needs a type for it, and a closure's type has no name. `impl Fn(i32) -> i32` in return position is the usual answer: the caller gets *some* closure. When different calls return different closures — the `if` returns one of two, with different captures — they are different types and `impl Fn` cannot name both; box them as `Box<dyn Fn(i32) -> i32>`, a trait object, and the type is the same either way. Chapter 18's rule, applied to closures.

## 20.5 Macros

A macro is code that writes code at compile time. `println!`, `vec!` and `#[derive]` are macros; the `!` and the `#[…]` are how you tell. A *declarative* macro is defined with `macro_rules~` as a set of patterns and what each expands to:

```
// A declarative macro: pattern-matched at compile time. Both sides are
// written in Harsh. The matcher is a parameter list -- one group per fragment,
// a repetition of groups for a list -- and the transcriber is a `do:` block.
#[macro_export]
macro_rules~ my_vec:
    ( $( ($x:expr) )* ) => do:
        do:
            let mut temp_vec = Vec.new$
            $(temp_vec <- push $x)*
            temp_vec

// Two arms, and a repetition that spans lines. The inner `do:` makes the
// expansion a block with a value, as the Rust `{ { .. } }` would.
macro_rules~ sum:
    () => do: 0
    ($h:expr) => do: $h
    ( ($h:expr) $( ($t:expr) )* ) => do:
        $h + sum~ $( ($t) )*

fn main$:
    let v: Vec<u32> = my_vec~ 1 2 3
    println! "{v:?}"
    println! "{}" (sum~ 1 2 3 4)
```

```text
[1, 2, 3]
10
```

Both sides of the macro are written in Harsh, and both follow rules you already know. The matcher `( $( ($x:expr) )* )` is a parameter list: one group per fragment, and a repetition of groups is a list of them — the same brick as `fn f (a: T) (b: U)`, applied a third time. The call `my_vec! 1 2 3` is an ordinary application, one atom per argument. The transcriber is a `do:` block; its statements take their `;` from the layout, `$x` is an atom like any other so `push $x` is a call, and a repetition `$( … )*` that is the whole of its line repeats a statement. The inner `do:` is there because a macro's expansion is a run of tokens, not a block: for the expansion to *be* a block with a value, the block must be written, as Rust writes `{ { … } }`. `sum!` shows an arm with nothing to match and a repetition that spans lines, `$(` on one line, its body beneath, `)*` back under it. On the call side, `$( ($t) )*` in an argument position is a list of arguments, the mirror of the matcher; a repetition of bare tokens, `$($arg)*`, is copied as it stands and is how a macro forwards `tt`s.

> **Harsh —** A matcher written in brackets or braces rather than parentheses — `[ $elem:expr ; $n:expr ]` — is left exactly as written, because that is the shape the call side has too: `vec! [0u8; 4]`. The same licence covers a transcriber whose output is a language of its own rather than Harsh.

A macro whose body is markup — the `view!` of a web framework — is written the same way, with one rule of its own: when the first line of the `do:` block begins with `<`, the lines are markup and are copied through, and the contents of every `{ … }` are Harsh:

```
// A stand-in for a UI framework's `view!`: it swallows the markup and yields
// unit, so this file compiles without a dependency. Only the spelling of the
// call is the point here.
macro_rules! view {
    ( $($t:tt)* ) => { () };
}

fn main$:
    let count = 3
    let items = vec! [1, 2, 3]
    view! do:
        <div class="app">
            <p>{count}</p>
            <button on:click={move |_| println! "{}" (count + 1)}>"+"</button>
            <ul>
                <For each={move || items <- clone$} let:item>
                    <li>{item * 2}</li>
                </For>
            </ul>
        </div>
    println! "rendered {count}"
```

```text
rendered 3
```

A hole may hold a whole expression, `{move |_| …}`, and it may span lines: `{move |_|:` with the closure's body beneath and `}` closing it. The framework's own rules for markup are unchanged, because Harsh never reads it; only the holes are its business.

Dioxus writes its interface not as markup but as a tree of braces — an element is a name and a brace body holding its attributes and children — and a tree of braces is what Harsh's layout is. So it is written as one. When the first line of a macro's `do:` block is a name followed by `:`, the block is a brace tree: `div:` opens an element, an attribute takes its value through `=` (since `:` opens blocks here) and is emitted `class: "app",`, a string or a `{ … }` is a child, and `for` and `if` are the framework's own, their bodies trees again. As in markup, the Harsh is in the holes:

```
// A stand-in for Dioxus's `rsx!`, as `view!` above: it swallows the tree and
// yields unit.
macro_rules! rsx {
    ( $($t:tt)* ) => { () };
}

fn main$:
    let count = 3
    let items = vec! [1, 2, 3]
    rsx! do:
        div:
            class = "app"
            onclick = {move |_| println! "{}" (count + 1)}
            "Hello {count}"
            for item in items:
                li: "{item}"
            Button\
                onclick = {move |_|:
                    let n = count * 2
                    println! "{n}"
                }
                "Reset"
    println! "rendered {count}"
```

```text
rendered 3
```

What the tree becomes follows what `rsx!` itself accepts, read from its source: attributes are separated by commas and elements and text nodes are not, and an event handler is recognised by its leading `move` or `|` — which is why an attribute's hole loses its braces on the way out while a child's keeps them.

The other kind, *procedural* macros — `#[derive Debug]`, attribute macros, function-like macros that parse arbitrary tokens — are functions that take a token stream and return one, and live in a crate of their own with `proc-macro = true` in its `Cargo.toml`. Writing one is a project rather than a page: the `syn` crate parses the tokens into a syntax tree, `quote` turns a template back into tokens, and the function in between is ordinary Rust. When you need one, those two crates and their examples are the place to start.

### Parentheses, once and for all

Every use of parentheses in this book has been one of a short list, and the list is worth stating now that all of it has been seen:

```
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

## 20.6 What you have

`unsafe:` for five operations the compiler cannot check, wrapped in safe functions that do the checking; `extern "C"` for foreign functions. Associated types for a trait with one choice per implementation; operator traits in `std.ops` with default type parameters; disambiguation by trait path and `<Type as Trait>`; supertraits; the newtype pattern for the orphan rule and for distinct types. Aliases, `!`, and `?Sized`. `fn` pointers, constructors as functions, `impl Fn` and `Box<dyn Fn>` for returned closures. `macro_rules~` written in Harsh on both sides, markup and brace-tree macros with Harsh holes, and where procedural macros come from.

Next, and last: a multithreaded web server, built from the standard library alone — the book's closing project.

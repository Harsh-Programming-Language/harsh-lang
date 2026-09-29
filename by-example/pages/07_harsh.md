# 7. What Harsh adds

Everything so far has been Rust with the braces taken out. This page is the
part that is Harsh's own.

## 7.1 The pipes

```rust harsh
fn sub (a: i32) (b: i32) (c: i32) -> i32:
    a - b - c

fn double n: i32 -> i32:
    n * 2

fn main$:
    // `|>` fills a function's parameters from the left.
    println! "{}" (10 5 1 |> sub)

    // Fewer arguments than it takes: what comes back is a closure waiting
    // for the rest. This is partial application, and it is flat -- no
    // nesting, no `move |x| move |y|`.
    let from_ten = 10 |> sub
    println! "{}" (from_ten 5 1)

    // `<|` fills from the right instead.
    let take_one = sub <| 1
    println! "{}" (take_one 10 5)

    // Both at once: `1 |> sub <| 3` puts 1 first and 3 last.
    let middle = 10 |> sub <| 1
    println! "{}" (middle 5)

    // Chained, a pipe reads left to right: the value, then what happens next.
    println! "{}" (3 |> double |> double)
```

```text
4
4
4
4
12
```

`|>` fills a function's parameters from the left, `<|` from the right. When
the count reaches what the function takes, it is simply a call. When it falls
short, what comes back is a closure waiting for the rest — partial
application, and a flat one: a single closure over the remaining parameters,
not a nest of them.

The arity comes from the declaration, which `hrs` has already read, so nothing
has to be spelled out at the call.

Left of `|>` is a run of atoms, all of them arguments. To pipe the *result* of
an application, isolate it: `(f a) |> g`.

## 7.2 Partial application

```rust harsh
fn label (prefix: &str) (name: &str) (suffix: &str) -> String:
    format! "{prefix}{name}{suffix}"

fn main$:
    // `|>` fills the parameters from the left.
    println! "{}" ("Dr. " "Ada" "," |> label)
    // `<|` fills them from the right: this waits for a prefix.
    let with_phd = label <| "Ada" " PhD"
    println! "{}" (with_phd "Prof. ")
    // Both together leave a hole in the middle: a function of the name.
    let formal = "Dr. " |> label <| "."
    println! "{}" (formal "Ada")
    println! "{}" (formal "Grace")
```

```text
Dr. Ada,
Prof. Ada PhD
Dr. Ada.
Dr. Grace.
```

Give a function fewer values than it takes and the result is a function
waiting for the rest. `|>` fills parameters from the left, `<|` from the
right, and both together leave a hole in the middle: `"Dr. " |> label <| "."`
is a function of the name alone. There is no placeholder; the hole is what is
left. Chapter 14 of the Book has the whole story.

## 7.3 Macros

Harsh's macros are organised by who wrote them: in Harsh or in Rust -- the
mark says so, `name~` or `name!` -- and yours or imported from a library.
The Book's sections 23.5 to 23.8 take each part in full, and follow the Rust
Book's own chapter on macros.

### Harsh macros (~)

Your own, written in Harsh.

#### Declarative macros

```rust harsh
// A Harsh macro is matched and expanded by `hrs`, in Harsh, before anything
// is transpiled. The mark is `~`.
macro_rules~ greet
    (($who:expr)) => do:
        println! "hello, {}" $who

// A repetition takes `*`, `+` or `?`, exactly as Rust's does.
macro_rules~ list_of
    ($( ($x:expr) )*) => do: [$( $x ),*]

fn main$:
    greet~ "world"

    // The arguments are a token stream: everything to the end of the line,
    // and every line indented beneath it.
    let small = list_of~ 1 2 3
    let bigger =
        list_of~ 1 2 3
                     4 5 6
    println! "{:?} {:?}" small bigger

    // To chain on the result, isolate the call.
    println! "{}" ((list_of~ 1 2 3) <- len$)
```

```text
hello, world
[1, 2, 3] [1, 2, 3, 4, 5, 6]
3
```

`macro_rules~` defines a macro that `hrs` expands itself, in Harsh, before
anything is transpiled — so the generated Rust holds no macro at all, and what
a macro produces is Harsh you could have written by hand.

A call passes a **token stream**, not arguments: everything after `name~` to
the end of the line, and every line indented deeper beneath it. Nothing in
that stream is interpreted — a comma is the DSL's, a `\` is the DSL's — which
is why a macro can invent a grammar of its own. The stream ends at the close
of a group the call sits inside, so `(list_of~ 1 2 3) <- len$` chains on the
result.

#### Procedural macros

Programs, written in Harsh in a crate of their own, mirroring Rust's three
forms; each needs two crates to show, so the Book teaches them (23.5).

##### Custom derive Macros

A `pub fn` marked `#[proc_macro_derive~ Describe]`, called `#[derive~ Describe]`
above a `struct`, an `enum` or a `union`: it receives the item and adds code
after it.

##### Attribute-Like Macros

`#[proc_macro_attribute~]` on a `pub fn` taking two streams, the attribute's
arguments and the item, called `#[route~ GET "/"]` or `#[route~]`, and
replacing the item (the Book's 23.5 has the Rust Book's `route` example).

##### Function-Like Macros

A `pub fn` marked `#[proc_macro~]`, called like a declarative macro,
`hello_macro~ world`: it receives `world` and returns the Harsh that takes the
call's place.

### Rust macros (!)

Your own, written in Rust, still supported (23.6).

#### Declarative macros

A `macro_rules!` in a Harsh file is a zone of Rust, copied as written; its
calls are Harsh, `square! n`.

#### Procedural macros

A Rust proc-macro crate beside your Harsh crates, brought in with `use`.

##### Custom derive Macros

`proc_macro_derive`, called `#[derive Name]`; the Book's 23.6 has the Rust
Book's `HelloMacro` used from Harsh.

##### Attribute-Like Macros

`proc_macro_attribute`, two streams, called `#[route GET "/"]` -- Rust's
`#[route(GET, "/")]` -- replacing the item.

##### Function-Like Macros

`proc_macro`, called `name! args`.

### Harsh DSLs (~)

Imported from Harsh libraries, Harsh throughout; the library's guide is the
reference. The prelude's `g~` and `m~` are the first (pages 16 and 17).

### Rust DSLs (!)

Imported from Rust libraries, called by Harsh's rules: `vec! 1 2 3`,
`#[derive Debug Serialize]`, `#[tokio.main]`; a stream that is a language of
its own goes in braces, with Harsh in holes, `@: … :@` (23.8).

## Precedence

Application binds tighter than anything but an atom: `add 1 2 * 10` adds,
then multiplies. The chain `<-` comes next, then `?` and the prefix operators
— so `!v <- is_empty$` is "not empty", and `(*r) <- len$` dereferences `r`
before calling. Rust's operators follow in Rust's order, then the pipes. A
negative or dereferenced argument is parenthesised: `f (-1)`, `f (*x)`. The
full table is the Book's appendix *Precedence*.

```rust harsh
struct Switch<'a>
    flag: &'a mut bool

fn add (a: i32) (b: i32) -> i32:
    a + b

fn double (x: i32) -> i32:
    x * 2

fn main$:
    let v = vec! 3 1 2
    // Application first: `add 1 2`, then `* 10`.
    println! "{}" (add 1 2 * 10)
    // A negative argument is parenthesised.
    println! "{}" (double (-4))
    // A chain before an operator: the length, plus one.
    println! "{}" (v <- len$ + 1)
    // A prefix operator takes the whole chain: not (v is empty).
    println! "{}" (!v <- is_empty$)
    // An application before a chain: `add 1 2`, then its method.
    println! "{}" (add 1 2 <- pow 2)
    // Dereference first, then the method: parenthesised.
    let r = &v
    println! "{}" ((*r) <- len$)
    // Through a field: `*s <- flag` is the bool the field points to.
    let mut on = false
    let mut s = Switch\ flag = &mut on
    *s <- flag = true
    println! "{on}"
    // Pipes, left to right; several values fill several parameters.
    println! "{}" (3 |> double |> double)
    println! "{}" (1 2 |> add)
    // To pipe a call's result, parenthesise the call.
    println! "{}" ((add 1 2) |> double)
    // Fewer values than parameters: a partial application.
    let add10 = 10 |> add
    println! "{}" (add10 5)
    // The backward pipe, right to left.
    println! "{}" (double <| double <| 5)
```

```text
30
-8
4
true
9
3
true
12
3
6
15
20
```

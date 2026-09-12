# 10. Generic types, traits and lifetimes

Three mechanisms let one piece of code serve many types, and this chapter is all three. *Generics* are placeholders for types: a `Vec<T>` is one definition that is a vector of anything. *Traits* say what a type can do, so a generic function can require it: "any `T` that can be compared". And *lifetimes* are generics over how long a reference is valid, which is the question chapter 4 left open. They arrive together because they are used together, and because the last of them is the one people fear, unnecessarily, so it is best met right after the first two make the shape familiar.

## 10.1 Generics

### Removing duplication

Two functions that differ only in a type:

```
fn largest_i32 list: &[i32] -> &i32:
    let mut largest = &list[0]

    for item in list:
        if item > largest:
            largest = item

    largest

fn largest_char list: &[char] -> &char:
    let mut largest = &list[0]

    for item in list:
        if item > largest:
            largest = item

    largest

fn main$:
    println! "{}" (largest_i32 (&[34, 50, 25, 100, 65]))
    println! "{}" (largest_char (&['y', 'm', 'a', 'q']))
```

```text
100
y
```

Same body, twice, for `i32` and for `char`, and there would be a third for `f64`. The duplication is in the *type*, so the fix is a parameter for the type:

```
fn largest<T> list: &[T] -> &T:
    let mut largest = &list[0]

    for item in list:
        if item > largest:
            largest = item

    largest

fn main$:
    println! "{}" (largest (&[34, 50, 25, 100, 65]))
```

```text
error[E0369]: binary operation `>` cannot be applied to type `&T`
  --> largest_generic_err.hrs:5:17
   |
 5 |         if item > largest:
   |            ---- &T
   |                 ^
   |                   ------- &T
   = help: consider restricting type parameter `T` (hrs 1:13)
```

`fn largest<T> list: &[T] -> &T` — the `<T>` after the name declares a type parameter, and then `T` is used where the type would be. It does not compile, and the error is the chapter's first lesson: `>` is not defined for *every* type, so the compiler will not let a function that takes *any* `T` compare two of them. It has to be told which `T`s are allowed, and the help says how: restrict the type parameter.

```
fn largest<T: PartialOrd> list: &[T] -> &T:
    let mut largest = &list[0]

    for item in list:
        if item > largest:
            largest = item

    largest

fn main$:
    println! "{}" (largest (&[34, 50, 25, 100, 65]))
    println! "{}" (largest (&['y', 'm', 'a', 'q']))
```

```text
100
y
```

`<T: PartialOrd>` — `T` may be any type that implements the trait `PartialOrd`, which is the trait that provides `>`. Now the body may compare, and the function works for `i32` and `char` and anything else that can be ordered, with one definition. Generics in Rust are checked at the definition, not at each use: a generic function has to make sense for *every* type its bounds allow, which is why the unbounded version was rejected and why, once it compiles, no call can break it. (At compile time each use is *monomorphized* — a copy is generated per concrete type — so there is no runtime cost for the abstraction.)

### Generic structs and methods

A struct's fields can be generic too:

```
#[derive Debug]
struct Point<T>
    x: T
    y: T

impl<T> Point<T>:
    fn x (&self) -> &T:
        &self <- x

impl Point<f32>:
    fn distance_from_origin (&self) -> f32:
        (self <- x <- powi 2 + self <- y <- powi 2) <- sqrt$

#[derive Debug]
struct Mixed<T, U>
    x: T
    y: U

fn main$:
    let integer = Point\ x = 5, y = 10
    let float = Point\ x = 1.0, y = 4.0
    println! "{:?} {:?} {}" integer float (integer <- x$)
    println! "{}" (float <- distance_from_origin$)

    let m = Mixed\ x = 5, y = 4.0
    println! "{m:?}"
```

```text
Point { x: 5, y: 10 } Point { x: 1.0, y: 4.0 } 5
4.1231055
Mixed { x: 5, y: 4.0 }
```

`struct Point<T>` with fields of type `T`; `Point\ x = 5, y = 10` is a `Point<i32>` and `Point\ x = 1.0, y = 4.0` a `Point<f64>`, inferred. Methods on a generic type are declared in `impl<T> Point<T>:` — the `<T>` after `impl` says *this block is generic over T* — and a method for one specific type is `impl Point<f32>:`, so `distance_from_origin` exists only on `Point<f32>`. When two fields may differ in type, two parameters, `Mixed<T, U>`.

Both fields of `Point<T>` are the same `T`, and the compiler holds you to it:

```
struct Point<T>
    x: T
    y: T

fn main$:
    let wont_work = Point\ x = 5, y = 4.0
    let _ = wont_work
```

```text
error[E0308]: mismatched types
  --> generic_mismatch.hrs:6:39
   |
 6 |     let wont_work = Point\ x = 5, y = 4.0
   |                                       ^^^ expected integer, found floating-point number
```

## 10.2 Traits

A trait is a set of methods a type may implement — an interface, a protocol, whichever word your last language used. It is defined with `trait`, and a type implements it with `impl … for`:

```
pub trait Summary:
    fn summarize (&self) -> String

pub struct NewsArticle
    pub headline: String
    pub location: String
    pub author: String

impl Summary for NewsArticle:
    fn summarize (&self) -> String:
        format!
            "{}, by {} ({})"
            (self <- headline)
            (self <- author)
            (self <- location)

pub struct Tweet
    pub username: String
    pub content: String

impl Summary for Tweet:
    fn summarize (&self) -> String:
        format! "{}: {}" (self <- username) (self <- content)

fn main$:
    let tweet =
        Tweet\
            username = String.from "horse_ebooks"
            content = String.from "of course, as you probably already know, people"
    println! "1 new tweet: {}" (tweet <- summarize$)

    let article =
        NewsArticle\
            headline = String.from "Penguins win the Stanley Cup Championship!"
            location = String.from "Pittsburgh, PA, USA"
            author = String.from "Iceburgh"
    println! "New article available! {}" (article <- summarize$)
```

```text
1 new tweet: horse_ebooks: of course, as you probably already know, people
New article available! Penguins win the Stanley Cup Championship!, by Iceburgh (Pittsburgh, PA, USA)
```

`pub trait Summary:` declares one method by its signature, `fn summarize (&self) -> String`, with no body — that line ends at the return type. `impl Summary for NewsArticle:` gives the body for one type, `impl Summary for Tweet:` for another, and after that `tweet <- summarize$` and `article <- summarize$` are ordinary method calls. The trait is what makes the two types interchangeable to any code that only needs `summarize`.

One rule to know: you may implement a trait for a type only if the trait or the type is defined in your crate. `Summary` for `Vec<T>` — fine, `Summary` is yours; `Display` for `Tweet` — fine, `Tweet` is yours; `Display` for `Vec<T>` — not allowed, since both belong to someone else and two crates could disagree. It is called the *orphan rule* and it is what keeps trait implementations unambiguous across the whole ecosystem.

### Default implementations

A trait method may have a body, used by any type that does not supply its own:

```
pub trait Summary:
    fn summarize_author (&self) -> String

    fn summarize (&self) -> String:
        format! "(Read more from {}...)" (self <- summarize_author$)

pub struct Tweet
    pub username: String
    pub content: String

impl Summary for Tweet:
    fn summarize_author (&self) -> String:
        format! "@{}" (self <- username)

fn main$:
    let tweet =
        Tweet\
            username = String.from "horse_ebooks"
            content = String.from "of course, as you probably already know, people"
    println! "1 new tweet: {}" (tweet <- summarize$)
```

```text
1 new tweet: (Read more from @horse_ebooks...)
```

`summarize` has a default that calls `summarize_author`, which has none; `Tweet` implements only the required one and gets the other free. A default may call the required methods, which is how a trait can offer a lot of behaviour and demand a little.

### Traits as parameters

A function that takes "anything summarizable" has four spellings, all in this example:

```
use std.fmt.Display

pub trait Summary:
    fn summarize (&self) -> String

pub struct Tweet
    pub username: String

impl Summary for Tweet:
    fn summarize (&self) -> String:
        format! "@{}" (self <- username)

impl Display for Tweet:
    fn fmt (&self) (f: &mut std.fmt.Formatter) -> std.fmt.Result:
        write! f "tweet by {}" (self <- username)

// `impl Trait` in a parameter: any type that implements Summary

pub fn notify item: &impl Summary:
    println! "Breaking news! {}" (item <- summarize$)

// The same, spelled out as a bound on a type parameter
pub fn notify2<T: Summary> item: &T:
    println! "Breaking news! {}" (item <- summarize$)

// Two bounds with `+`
pub fn notify3 item: &(impl Summary + Display):
    println! "{item} — {}" (item <- summarize$)

// A `where` clause, for when the bounds get long
pub fn notify4<T> item: &T
    [where T: Summary + Display]:
    println! "{item} — {}" (item <- summarize$)

fn main$:
    let t = Tweet\ username = String.from "horse_ebooks"
    notify (&t)
    notify2 (&t)
    notify3 (&t)
    notify4 (&t)
```

```text
Breaking news! @horse_ebooks
Breaking news! @horse_ebooks
tweet by horse_ebooks — @horse_ebooks
tweet by horse_ebooks — @horse_ebooks
```

`item: &impl Summary` is the short form: a reference to some type that implements `Summary`. `notify2<T: Summary> item: &T` is the same thing with the type parameter named, which you need when two parameters must be the *same* type, or the name is used elsewhere. `&(impl Summary + Display)` requires two traits — `+` joins bounds, and the group is parenthesised so the `+` cannot be read as part of the parameter. And `[where T: Summary + Display]` moves the bounds out of the signature to a line of their own, which is what you do when they get long. The brackets are Harsh's: a `where` bound has a `:` in it, and the brackets keep that colon from opening a block; they are stripped on the way out.

`Tweet` also implements `Display` here, by writing `fmt` — that is the trait behind `{}`, the one chapter 5's error said `Rectangle` lacked. `write! f "…" args` writes into the formatter, and the method's two groups are `(&self)` and `(f: &mut std.fmt.Formatter)`.

> **Harsh —** A `where` clause on one line needs nothing: `fn f<T> (x: T) -> T where T: Clone:`. Over several lines it is written in brackets, `[where T: Summary + Display]` on its own line under the signature, because a bound's `:` would otherwise be read as opening a block; the brackets keep the clause part of the header and are not emitted, and the body indents as it would without them.


### Returning a trait

`impl Trait` works in return position too:

```
pub trait Summary:
    fn summarize (&self) -> String

pub struct Tweet
    pub username: String

impl Summary for Tweet:
    fn summarize (&self) -> String:
        format! "@{}" (self <- username)

fn returns_summarizable$ -> impl Summary:
    Tweet\ username = String.from "horse_ebooks"

fn main$:
    println! "{}" (returns_summarizable$ <- summarize$)
```

```text
@horse_ebooks
```

"This function returns *some* type that implements `Summary`" — the caller can call `summarize$` and nothing else. Useful when the concrete type is long or unnameable (closures and iterators, chapter 13). One limit: the function must return one concrete type; a function that returns a `Tweet` on one branch and a `NewsArticle` on another cannot use `impl Summary`, and needs a trait object (chapter 18).

### Conditional methods

A method can exist only when the type parameter meets a bound:

```
use std.fmt.Display

struct Pair<T>
    x: T
    y: T

impl<T> Pair<T>:
    fn new (x: T) (y: T) -> Self:
        Self\ x, y

// Only a Pair whose T can be compared and displayed gets this method.

impl<T: Display + PartialOrd> Pair<T>:
    fn cmp_display (&self):
        if self <- x >= self <- y:
            println! "The largest member is x = {}" (self <- x)

        else:
            println! "The largest member is y = {}" (self <- y)

fn main$:
    let p = Pair.new 3 7
    p <- cmp_display$

    // `to_string` exists on every type that implements Display — a blanket impl in std.
    let s: String = 42 <- to_string$
    println! "{s}"
```

```text
The largest member is y = 7
42
```

`impl<T> Pair<T>:` gives every `Pair` a `new`; `impl<T: Display + PartialOrd> Pair<T>:` gives `cmp_display` only to pairs whose elements can be compared and printed. The standard library uses the same device on a larger scale: `impl<T: Display> ToString for T` implements `ToString` for *every* type that implements `Display`, which is why `42 <- to_string$` works — a *blanket implementation*, and a large part of why traits compose.

## 10.3 Lifetimes

Every reference has a *lifetime*: the region of the program during which it is valid. Usually the compiler works it out and you write nothing — every `&` so far had a lifetime you never saw. It has to be written only when the compiler cannot tell how the lifetimes of several references relate, and the classic case is a function that returns one of two borrowed arguments:

```
fn longest (x: &str) (y: &str) -> &str:
    if x <- len$ > y <- len$: x else: y

fn main$:
    println! "{}" (longest "abcd" "xyz")
```

```text
error[E0106]: missing lifetime specifier
  --> longest_err.hrs:1:35
   |
 1 | fn longest (x: &str) (y: &str) -> &str:
   |                ----
   |                          ----
   |                                   ^ expected named lifetime parameter
   = help: this function's return type contains a borrowed value, but the signature does not say whether it is borrowed from `x` or `y`
   = help: consider introducing a named lifetime parameter (hrs 1:12)
```

Read the help: the return type is a borrowed value, and the signature does not say whether it is borrowed from `x` or from `y`. The compiler needs to know, because the caller's borrow checker needs to know how long the returned reference may be used — as long as `x` lives, as long as `y` lives, or only as long as both do. The signature has to say. The syntax for saying it is a *lifetime parameter*:

```
fn longest<'a> (x: &'a str) (y: &'a str) -> &'a str:
    if x <- len$ > y <- len$: x else: y

fn main$:
    let string1 = String.from "abcd"
    let string2 = "xyz"
    let result = longest (string1 <- as_str$) string2
    println! "The longest string is {result}"
```

```text
The longest string is abcd
```

`fn longest<'a> (x: &'a str) (y: &'a str) -> &'a str` — `'a` is declared after the name like a type parameter (the apostrophe marks it as a lifetime), and then `&'a str` means "a reference with lifetime `'a`". Putting the same `'a` on both inputs and the output says: the returned reference lives as long as the *shorter* of the two inputs. That is the whole meaning. Lifetime annotations do not change how long anything lives; they describe the relationship, so that the checker can verify calls against it.

And it does verify:

```
fn longest<'a> (x: &'a str) (y: &'a str) -> &'a str:
    if x <- len$ > y <- len$: x else: y

fn main$:
    let string1 = String.from "long string is long"
    let result

    do:
        let string2 = String.from "xyz"
        result = longest (string1 <- as_str$) (string2 <- as_str$)

    println! "The longest string is {result}"
```

```text
error[E0597]: `string2` does not live long enough
  --> longest_scope.hrs:10:48
   |
 9 |         let string2 = String.from "xyz"
   |             ------- binding `string2` declared here
10 |         result = longest (string1 <- as_str$) (string2 <- as_str$)
   |                                                ^^^^^^^ borrowed value does not live long enough
12 |     println! "The longest string is {result}"
   |     - `string2` dropped here while still borrowed
   |                                     -------- borrow later used here
```

`string2` lives only inside the `do:` block; `result` is used after it; and `longest`'s signature says `result` cannot outlive the shorter of its inputs. So the compiler rejects the program, and — this is the part to appreciate — it does so *at the call*, from the signature alone, without looking inside `longest`. That is what the annotation bought: a function's borrowing behaviour is part of its interface, checked at each use.

### Lifetimes in structs

Chapter 5's `&str` field had this error. A struct that holds a reference declares a lifetime, and the struct cannot outlive what it borrows:

```
#[derive Debug]
struct ImportantExcerpt<'a>
    part: &'a str

impl<'a> ImportantExcerpt<'a>:
    fn level (&self) -> i32:
        3

    fn announce_and_return_part (&self) (announcement: &str) -> &str:
        println! "Attention please: {announcement}"
        self <- part

fn main$:
    let novel = String.from "Call me Ishmael. Some years ago..."
    let first_sentence =
        novel <- split '.'
              <- next$
              <- expect "Could not find a '.'"
    let i = ImportantExcerpt\ part = first_sentence

    println! "{i:?} {}" (i <- level$)
    println! "{}" (i <- announce_and_return_part "hi")
```

```text
ImportantExcerpt { part: "Call me Ishmael" } 3
Attention please: hi
Call me Ishmael
```

`struct ImportantExcerpt<'a>` with `part: &'a str` means an `ImportantExcerpt` is valid only while the text it points into is. `impl<'a> ImportantExcerpt<'a>:` declares the lifetime for the methods, which then mostly do not mention it — `level` takes `&self` and returns an `i32`; `announce_and_return_part` returns a `&str` whose lifetime the compiler works out from `&self`, by the rules below.

### Elision

If every reference needed a written lifetime the language would be unusable, so three rules fill them in when they can, and you write one only when they cannot:

```
// One input lifetime: the output gets it. Written out, this is
// fn first_word<'a> s: &'a str -> &'a str.
fn first_word s: &str -> &str:
    let bytes = s <- as_bytes$

    for (i, &item) in bytes <- iter$ <- enumerate$:
        if item == b' ':
            return &s[0..i]

    &s[..]

// A reference that lives for the whole program.
static GREETING: &'static str = "I have a static lifetime."

fn main$:
    println! "{}" (first_word "hello world")
    println! "{GREETING}"
```

```text
hello
I have a static lifetime.
```

1. Each reference parameter gets its own lifetime.
2. If there is exactly one input lifetime, the output gets it — which is why chapter 4's `first_word s: &str -> &str` needed nothing.
3. If one of the inputs is `&self`, the output gets `self`'s lifetime — which is why `announce_and_return_part` needed nothing.

`longest` had two inputs and no `self`, so no rule applied and the signature had to say. That is the entire theory of when you write lifetimes. `'static` is the one named lifetime you will see in the wild: the whole program, which every string literal has, since the text lives in the binary.

### All three at once

```
use std.fmt.Display

fn longest_with_an_announcement<'a, T> (x: &'a str) (y: &'a str) (ann: T) -> &'a str
    [where T: Display]:
    println! "Announcement! {ann}"
    if x <- len$ > y <- len$: x else: y

fn main$:
    println! "{}" (longest_with_an_announcement "abcd" "xyz" "today")
```

```text
Announcement! today
abcd
```

A lifetime, a type parameter, a bound in a `[where …]` clause, and three parameters: everything in this chapter in one signature, and nothing in it is new.

## 10.4 What you have

`<T>` after a name makes a function, struct or `impl` generic; `T: Trait` restricts it, and the body may only do what the bounds allow. `trait` declares methods, with or without defaults; `impl Trait for Type` supplies them; `&impl Trait`, `T: Trait`, `+`, and `[where …]` take them as parameters and `impl Trait` returns one. The orphan rule keeps implementations unambiguous. A lifetime `'a` names how long a reference is valid, is written only when elision's three rules cannot fill it in, and lets the checker verify each call from the signature alone.

Next: writing automated tests — which is the natural use of everything so far, and where `panic` is a feature.

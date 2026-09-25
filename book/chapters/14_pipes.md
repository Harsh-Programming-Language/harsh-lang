# 14. Pipes and partial application

Harsh adds three things to Rust that Rust has no syntax for, and this chapter and the next two are about them. The first is the pipes: `|>` hands a value to a function, `<|` does the same from the other side, and together they give *partial application* — calling a function with only some of its arguments and getting back a function that waits for the rest. They are borrowed from the functional languages, F#, OCaml and Elm, where they are how most code is written.

## 14.1 The pipes

Everything so far has reached into a value with `<-`: a method on the left of the arrow's data. The pipes go the other way. `|>` takes the value on its left and hands it to the *function* on its right; `<|` does the same from the other side. Where a chain says *this value, then this method on it*, a pipe says *this value, into this function*:

```
fn tokenize text: &str -> Vec<String>:
    text <- split_whitespace$
         <- map (|w| w <- to_lowercase$)
         <- collect$

fn count words: &Vec<String> -> usize:
    words <- len$

fn shout text: &str -> String:
    text <- to_uppercase$

fn main$:
    let text = "the cat sat on the mat"

    // a value flows left to right, into one function after another
    let n = text |> tokenize |> (|w| count (&w))
    println! "{n} words"

    // the same, right to left
    let m = (|w| count (&w)) <| tokenize <| text
    println! "{m} words"

    // a method reaches into a value; a pipe hands a value to a function
    let a = text <- to_uppercase$
    let b = text |> shout
    println! "{a} / {b}"
```

```text
6 words
6 words
THE CAT SAT ON THE MAT / THE CAT SAT ON THE MAT
```

> **Harsh —** `text |> tokenize` is `tokenize` applied to `text`, and the chain reads on: the next `|>` applies the closure to what came out. `<|` is the mirror — `f <| g <| x` applies `g` to `x` and `f` to the result. Both sides of a pipe are *atoms*: a value, a name, an isolated group. That is the one place the pipes and the arrow part company. Chapter 2 said an application binds tighter than `<-` — `tokenize text <- len$` applies `tokenize` first — but `tokenize text |> count` does *not* apply it first: every atom on a pipe's side is an argument, so that line hands `count` two of them, `tokenize` and `text`. To pipe a *result*, isolate it: `(tokenize text) |> count`. The reason is the pipes' own feature — a pipe may carry several values, `2.0 0.5 |> scale` — and a rule that let one of them be an application would have to guess where it ended. So a chain like `raw <- clone$` is isolated before it goes in, `(raw <- clone$) |> trim_ws`, and a closure is isolated the same way, `|> (|w| count (&w))` — the function a pipe applies is one atom too.

## 14.2 Partial application

The pipes do one more thing, and it is the thing the arrow cannot do. A pipe may carry several values — `2.0 0.5 3.0 |> scale` — and when it carries *fewer* than the function takes, the missing ones are **deferred**: the result is a closure waiting for the rest.

```
fn scale (factor: f64) (offset: f64) (x: f64) -> f64:
    x * factor + offset

fn main$:
    // every parameter given: a plain call
    let y = 2.0 0.5 3.0 |> scale
    println! "{y}"

    // fewer given: what is missing is deferred, and the result is a closure
    let f = 2.0 0.5 |> scale          // waits for x
    println! "{}" (f 3.0)

    let g = scale <| 10.0             // filled from the right: waits for factor and offset
    println! "{}" (g 2.0 0.5)

    let h = 2.0 |> scale <| 10.0      // both sides: the hole is in the middle
    println! "{}" (h 0.5)

    // a partial is a value like any other
    let doubled: Vec<f64> =
        vec! 1.0 2.0 3.0
            <- into_iter$
            <- map (2.0 0.0 |> scale)
            <- collect$
    println! "{doubled:?}"
```

```text
6.5
6.5
20.5
20.5
[2.0, 4.0, 6.0]
```

> **Harsh —** `|>` fills a function's parameters from the left, `<|` from the right, and the two together leave a hole in the middle. The result is one flat closure over whatever was not filled — a value like any other: bind it, pass it to `map`, return it from a function. There is no placeholder token; the hole is what is left. Harsh knows how many parameters `scale` takes because `scale` is declared in your project; for a function it cannot see into — the standard library, a crate you depend on — a pipe is a plain call with the values you gave, and the compiler says so if the count was wrong. Give a project function *more* than it takes and Harsh itself refuses:

```
fn sub (a: i32) (b: i32) (c: i32) -> i32:
    a - b - c

fn main$:
    let n = 1 2 3 4 |> sub
    println! "{n}"
```

```text
error: `sub` takes 3 parameter(s) and 4 were piped in
  --> pipe_too_many.hrs:5:21
   |
  5|     let n = 1 2 3 4 |> sub
   |                     ^^
```

## 14.3 From the right, and a hole in the middle

`|>` fills a function's parameters from the left; `<|` fills them from the right. Used together they fill both ends and leave the middle open:

```
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

`label` takes a prefix, a name and a suffix. `"Dr. " "Ada" "," |> label` fills all three and calls it. `label <| "Ada" " PhD"` fills the last two — the name and the suffix, in order — and waits for the prefix. And `"Dr. " |> label <| "."` fills the first from the left and the last from the right: what is left is a function of the name, and `formal` can be applied to one name after another. No placeholder marks the hole; the hole is simply whatever was not filled.

## 14.4 Pipelines

A pipe's result is a value, so pipes chain: each stage's output is the next stage's input, left to right, and a partial application makes a stage out of a function that needed more than one argument:

```
fn trim_ws s: String -> String:
    s <- trim$ <- to_string$

fn shout s: String -> String:
    s <- to_uppercase$

fn wrap (left: &str) (right: &str) (s: String) -> String:
    format! "{left}{s}{right}"

fn main$:
    let raw = String.from "   hello, harsh   "

    // a pipeline: each stage's result is the next stage's argument
    let out = (raw <- clone$) |> trim_ws |> shout |> ("[" "]" |> wrap)
    println! "{out}"

    // the same pipeline as a value, with the argument left out
    let banner = |s: String|:
        s |> trim_ws |> shout |> ("<" ">" |> wrap)
    println! "{}" (banner raw)
```

```text
[HELLO, HARSH]
<HELLO, HARSH>
```

`("[" "]" |> wrap)` is `wrap` with its first two parameters filled — a function of one `String` — and so it is a stage like `trim_ws` and `shout`. `banner` is the whole pipeline as a closure: the same stages, with the argument left for later. Read the line aloud: *raw, trimmed, shouted, wrapped*.

## 14.5 Partials as arguments

A partial application is a value, so it goes wherever a function may go — above all, into an iterator's adaptors:

```
fn scale (factor: f64) (x: f64) -> f64:
    factor * x

fn clamp (lo: f64) (hi: f64) (x: f64) -> f64:
    x <- max lo <- min hi

fn main$:
    let readings = vec! 0.4 1.2 2.6 3.1
    // A partial application is a function value: hand it to `map`.
    let doubled: Vec<f64> =
        readings <- iter$
                 <- map (|&x| (2.0 |> scale) x)
                 <- collect$
    let bounded: Vec<f64> =
        doubled <- iter$
                <- map (|&x| (0.0 5.0 |> clamp) x)
                <- collect$
    println! "{doubled:?}"
    println! "{bounded:?}"
```

```text
[0.8, 2.4, 5.2, 6.2]
[0.8, 2.4, 5.0, 5.0]
```

`2.0 |> scale` is "multiply by two", and `0.0 5.0 |> clamp` is "keep between 0 and 5", each built from a general function by fixing its first arguments. A predicate works the same way: in the next program `2.0 |> above` is "greater than two", passed to `filter` as it is:

```
fn parse (line: &str) -> Option<f64>:
    line <- trim$
         <- parse$
         <- ok$

fn above (limit: f64) (x: &f64) -> bool:
    *x > limit

fn mean (xs: &[f64]) -> f64:
    xs <- iter$ <- sum.<f64>$ / (xs <- len$ as f64)

fn main$:
    let raw = "3.5\n oops \n7.25\n1.0\n9.5\n"
    // A small data pipeline: parse, keep what parsed, filter, summarise.
    let values: Vec<f64> =
        raw <- lines$
            <- filter_map parse
            <- filter (2.0 |> above)
            <- collect$
    println! "{values:?}"
    println! "mean {:.2}" ((&values) |> mean)
```

```text
[3.5, 7.25, 9.5]
mean 6.75
```

Parse, keep what parsed, keep what is large enough, summarise: the chain reads as the steps. Note `((&values) |> mean)`. `&values` is two tokens, `&` and `values`, so it is isolated to make it one atom before it enters the pipe — the same rule as any argument with an operator in it.

## 14.6 When a pipe beats a closure

A closure that only forwards its argument is a partial application spelled the long way:

```
fn discount (rate: f64) (price: f64) -> f64:
    price * (1.0 - rate)

fn main$:
    let prices = vec! 10.0 25.0 40.0

    // a closure that only forwards its argument...
    let a: Vec<f64> =
        prices <- iter$
               <- map (|p| discount 0.2 (*p))
               <- collect$
    // ...says nothing a partial does not say shorter
    let b: Vec<f64> =
        prices <- iter$
               <- copied$
               <- map (0.2 |> discount)
               <- collect$
    println! "{a:?} {b:?}"

    // where the closure earns its place: the argument is transformed on the way in
    let c: Vec<f64> =
        prices <- iter$
               <- map (|p| discount 0.2 (p + 5.0))
               <- collect$
    println! "{c:?}"
```

```text
[8.0, 20.0, 32.0] [8.0, 20.0, 32.0]
[12.0, 24.0, 36.0]
```

`|p| discount 0.2 (*p)` names `p` twice to say what `0.2 |> discount` says once. When the closure transforms its argument on the way in — `p + 5.0` — it is doing work the pipe cannot, and it stays. That is the whole rule: reach for a partial when the argument passes through untouched, and a closure when it does not.

## 14.7 What you have

`|>` hands the values on its left to the function on its right; `<|` hands the values on its right to the function on its left. Each side of a pipe is a list of atoms, so anything with an operator in it — a chain, an `&`, a closure — is isolated first. Give a function fewer values than it takes and the rest are deferred: `|>` fills from the left, `<|` from the right, both at once leave a hole in the middle, and the result is a closure over what was not filled — a value to bind, pass to `map` or `filter`, or return. Give it more and Harsh refuses. Harsh counts the parameters of the functions in your project; for functions it cannot see, a pipe is a plain call.

Next: generator comprehensions — building a collection by saying what goes in it.

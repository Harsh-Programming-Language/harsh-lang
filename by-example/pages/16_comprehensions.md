# 16. Generator comprehensions

## 16.1 g~

```
fn main$:
    // A comprehension: the value, then where it comes from.
    let squares: Vec<i32> = (g~ x * x for x in 1..6) <- collect$
    println! "{:?}" squares

    // `if`s filter; each belongs to the `for` it follows.
    let odd_big: Vec<i32> = (g~ x for x in 0..20 if x % 2 == 1 if x > 10) <- collect$
    println! "{:?}" odd_big

    // Several `for`s nest, and an inner condition sees the outer name.
    let pairs: Vec<(i32, i32)> = (g~ (x, y) for x in 1..4 for y in 1..4 if y > x) <- collect$
    println! "{:?}" pairs
```

```text
[1, 4, 9, 16, 25]
[11, 13, 15, 17, 19]
[(1, 2), (1, 3), (2, 3)]
```

`g~` is Harsh's comprehension, and it is always available — it lives in the
prelude, so there is nothing to `use`. It reads the way the idea reads: the
value first, then `for` a pattern `in` an iterable, then any conditions.

Each `if` belongs to the `for` it follows, so a condition can use every name
bound up to that point — `if y > x` sees both — and a condition on the outer
loop filters before the inner one runs.

`g~` produces an iterator, and it is lazy: nothing is computed until something
asks for the values. `for` means what it means everywhere in the language, so
it takes the iterable by value; write `xs <- iter$` to borrow instead.

## 16.2 Conditions, and several `for`s

```
fn main$:
    // `if`s filter, and every `if` belongs to the `for` before it.
    let picked: Vec<i32> = (g~ x for x in 0..30 if x % 3 == 0 if x % 2 == 1) <- collect$
    println! "{picked:?}"

    // A condition sees every name bound so far: `y > x` uses both.
    let rising: Vec<(i32, i32)> = (g~ (x, y) for x in 1..4 for y in 1..4 if y > x) <- collect$
    println! "{rising:?}"
```

```text
[3, 9, 15, 21, 27]
[(1, 2), (1, 3), (2, 3)]
```

Each `if` belongs to the `for` before it, so a condition sees every name
bound up to that point — `y > x` uses both — and runs as early as it can.

## 16.3 Where a condition may go

```
fn main$:
    let triples =
        list~ (a, b, c)
            for a in 1..20        // only a is available for the condition
            for b in a..20        // only a, b are available for the condition
            for c in b..20        // a, b, c are available for the condition
            if a * a + b * b == c * c
    println! "{triples:?}"
```

```text
[(3, 4, 5), (5, 12, 13), (6, 8, 10), (8, 15, 17), (9, 12, 15)]
```

Every `for` makes one more name available, and a condition can use only the
names bound up to its place — so a test goes after the last `for` whose name it
uses. Attached to `for a`, the same test cannot see `b`:

```
fn main$:
    // The test needs b and c, but it is attached to `for a`,
    // where neither exists yet.
    let triples =
        list~ (a, b, c)
            for a in 1..20 if a * a + b * b == c * c
            for b in a..20
            for c in b..20
    println! "{triples:?}"
```

```text
error[E0425]: cannot find value `b` in this scope
 --> too_early.hrs:4:80
  |
4 | ....flat_map(move | a | ((true &&(a * a + b * b == c * c)).then(|| ((a .. 20).into_iter().flat_map(move | b | ((true).then(|| ((b .. 20)....
  |                                           ^ help: a local variable with a similar name exists: `a`

error[E0425]: cannot find value `b` in this scope
 --> too_early.hrs:4:84
  |
4 | ...t_map(move | a | ((true &&(a * a + b * b == c * c)).then(|| ((a .. 20).into_iter().flat_map(move | b | ((true).then(|| ((b .. 20).into...
  |                                           ^ help: a local variable with a similar name exists: `a`

error[E0425]: cannot find value `c` in this scope
 --> too_early.hrs:4:89
  |
4 | ...(move | a | ((true &&(a * a + b * b == c * c)).then(|| ((a .. 20).into_iter().flat_map(move | b | ((true).then(|| ((b .. 20).into_iter...
  |                                           ^ help: a local variable with a similar name exists: `a`

error[E0425]: cannot find value `c` in this scope
 --> too_early.hrs:4:93
  |
4 | ...e | a | ((true &&(a * a + b * b == c * c)).then(|| ((a .. 20).into_iter().flat_map(move | b | ((true).then(|| ((b .. 20).into_iter().f...
  |                                           ^ help: a local variable with a similar name exists: `a`

error: aborting due to 4 previous errors

For more information about this error, try `rustc --explain E0425`.
```

## 16.4 Collecting: list~, set~, dict~

```
use std.collections.HashMap
use std.collections.HashSet

fn main$:
    // `g~` is lazy -- a plain iterator. The shorthands collect it.
    let v = list~ x * 10 for x in 1..4
    println! "{:?}" v

    let remainders: HashSet<i32> = set~ x % 3 for x in 0..10
    let mut seen: Vec<_> = remainders <- into_iter$ <- collect$
    seen <- sort$
    println! "{:?}" seen

    // `dict~` takes `key => value`.
    let lengths: HashMap<&str, usize> = dict~ w => (w <- len$) for w in ["sun", "moon"]
    let mut pairs: Vec<_> = lengths <- into_iter$ <- collect$
    pairs <- sort$
    println! "{:?}" pairs
```

```text
[10, 20, 30]
[0, 1, 2]
[("moon", 4), ("sun", 3)]
```

The shorthands collect a comprehension into a `Vec`, a `HashSet` or a
`HashMap`. `dict~` takes its entry as `key => value`. Or keep `g~` and let the
type decide — `let v: Vec<_> = (g~ …) <- collect$` — which is the same thing
written out.

## 16.5 Laying one out

```
fn main$:
    // Every line indented beneath the call belongs to it, so a long one is
    // laid out one clause per line. Pythagorean triples:
    let triples =
        list~ (a, b, c)
            for a in 1..20
            for b in a..20
            for c in b..20 if a * a + b * b == c * c
    println! "{:?}" triples
```

```text
[(3, 4, 5), (5, 12, 13), (6, 8, 10), (8, 15, 17), (9, 12, 15)]
```

A call takes the rest of its line **and every line indented beneath it**, so a
comprehension with several clauses is written one clause per line. Nothing
changes but the layout: the same tokens reach the macro either way.

If you define a macro of your own called `g` (or `list`, `set`, `dict`), yours
wins in that file. The prelude's is still there as `hrs_std.g~`.

## 16.6 Lazy, and by value

```
fn main$:
    // `g~` is an iterator: nothing runs until something asks for values.
    // So an endless source is fine, as long as something stops asking.
    let first: Vec<u64> =
        (g~ n * n for n in 1.. if n % 7 == 3) <- take 4 <- collect$
    println! "{first:?}"

    // `for` takes its iterable by value, as it does everywhere in Rust.
    // Borrow to keep the collection.
    let words = vec! (String.from "pipe") (String.from "matrix")
    let lengths: Vec<usize> = (g~ (w <- len$) for w in (words <- iter$)) <- collect$
    println! "{lengths:?} -- and still {words:?}"
```

```text
[9, 100, 289, 576]
[4, 6] -- and still ["pipe", "matrix"]
```

`g~` is an iterator: nothing runs until something asks, so an endless
source is fine when something stops asking. `for` takes its iterable by
value; give it a borrow, `(words <- iter$)`, to keep the collection.

## 16.7 What it stands for

```
fn main$:
    let wanted: Vec<i32> =
        (g~ x * 10 + y
            for x in (0..6) if x % 2 == 0 if x > 0
            for y in (0..6) if y > x if y % 2 == 1
        ) <- collect$

    // The same by hand: one `flat_map` per `for`, each level's conditions
    // folded into one test, and the outer level flattened.
    let inner =
        |x: i32| (0..6) <- into_iter$
            <- flat_map (move |y| ((true && (y > x) && (y % 2 == 1)) <- then (|| x * 10 + y)))
    let written: Vec<i32> =
        (0..6) <- into_iter$
               <- flat_map (move |x| ((true && (x % 2 == 0) && (x > 0)) <- then (|| inner x)))
               <- flatten$
               <- collect$

    println! "{wanted:?} {}" (wanted == written)
```

```text
[23, 25, 45] true
```

A generator comprehension is a chain written another way: each `for` a
`flat_map`, its conditions folded into one test under `then`, every level but
the innermost flattened. The `flatten$` is part of the generator, so it is
never written by hand; the `collect$` is not, because a generator is lazy —
write it, or use `list~`. Chapter 15 of the Book shows the whole expansion.

```
fn main$:
    let scores = vec! 72 45 91 60 88

    // A comprehension...
    let a: Vec<i32> = (g~ s + 5 for s in (scores <- iter$) if *s >= 60) <- collect$
    // ...and the chain it stands for.
    let b: Vec<i32> =
        scores <- iter$
               <- filter (|s| **s >= 60)
               <- map (|s| s + 5)
               <- collect$
    println! "{a:?} {}" (a == b)
```

```text
[77, 96, 65, 93] true
```

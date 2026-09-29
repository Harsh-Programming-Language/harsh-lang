# Appendix: Precedence

When an expression mixes several forms, which one binds first? In most of
Harsh the answer is Rust's, operator for operator. Harsh adds three forms of
its own — applying a function by juxtaposition, `f a b`; reaching into a
value with `<-`; and the pipes — and this appendix places them among the
rest. The program at the end shows each rule at work, with its output.

## The levels

From the tightest-binding to the loosest:

| # | Level | Forms | Example, and what it means |
|---|---|---|---|
| 1 | **Atoms** | a literal, a name, a path `a.b`, anything in parentheses, a call with no arguments `f$`, an index `v[i]` | `f$` calls `f` with nothing |
| 2 | **Application** | `f a b` — each argument an atom; anything else in parentheses | `f (-1) (&y) (a + b)`: three arguments |
| 3 | **Member chain `<-`** | a field `x <- field`, a method `x <- m$` or `x <- m a b`, read left to right; each step takes its own arguments | `T.new "s" <- run (&mut p)`: build, then call `run` on the result |
| 4 | **`?`** | applies to the whole call or chain before it | `x <- get i?`: the `?` is on what `get` returns |
| 5 | **Prefix operators** | dereference `*`, borrow `&` and `&mut`, negation `-` and `!` — each on the whole chain after it | `!x <- is_empty$`: not (`x` is empty) |
| 6 | `as` | | `x <- ptr$ as usize + 1`: the cast, then the addition |
| 7 | `*` `/` `%` | | |
| 8 | `+` `-` | | `f a * 2 == g b`: two calls, a product, a comparison |
| 9 | `<<` `>>` | | |
| 10 | `&` | | |
| 11 | `^` | | |
| 12 | `\|` | | |
| 13 | **Comparisons** | `==` `!=` `<` `>` `<=` `>=` — never chained | `x <- len$ >= y <- len$`: two lengths compared |
| 14 | `&&`, then `\|\|` | | |
| 15 | **Ranges** | `..` `..=` | `(1..=10) <- map f`: the range needs parentheses before a chain |
| 16 | **Pipes** | `\|>` and `<\|`, a form of their own (below) | `3 \|> double \|> double` |
| 17 | **Assignment** | `=` `+=` `-=` … | |
| 18 | **To the end of the line** | a closure `\|x\| …`, an inline `if c: a else: b`, `do:`, a struct literal `P\\ a = 1, b = 2` | |

Levels 6 to 17 are Rust's order unchanged. What is Harsh's own is at the top
— application binds tighter than anything but an atom, and `<-` next — and
the pipes.

## Dereference

A prefix operator takes the whole chain after it, and since fields and
methods are both written with `<-`, dereference needs care:

| Harsh | What it means |
|---|---|
| `*p <- field` | the **field**, dereferenced |
| `(*p) <- field` | **`p`** dereferenced, then its field |
| `*p <- m$` | what `m` **returns**, dereferenced |
| `(*p) <- m$` | `m` called on what `p` points to |
| `*x <- get i?` | the `?` first, then the dereference |
| `**x` | twice |
| `*x + 1` | the dereference, then the addition |
| `*s <- flag = true` | assigns through the **field** — a `&mut bool` held by `s` |
| `f (*x)` | a dereferenced argument, parenthesised |
| `f *x` | **not a call**: `f` multiplied by `x` |

To reach through a raw pointer to a field, the parentheses are required:
`(*c) <- count`, as *The Harshonomicon* writes it.

## Pipes

A pipe is a form of its own, with strict edges, so that it can never be read
two ways:

- On its **left**, only atoms — the values it carries. Several values fill
  several parameters: `1 2 |> add` is `add` given 1 and 2. Fewer than the
  function takes make a **partial application**: `10 |> add` is a function
  waiting for the second number (chapter 14).
- On its **right**, one function; after it only another pipe may follow.
  `a |> f + 1` is refused: write `(a |> f) + 1`.
- `|>` reads left to right, `a |> f |> g`: `f` first, then `g`. `<|` reads
  right to left, `f <| g <| x`: `g` first, then `f`.

## Traps

Each of these compiles, or is refused with a message, and each has been
written by mistake while this book's companions were written:

- **`f -1` is not a call:** it is `f` minus 1. Write `f (-1)`.
- **`f *x` is not a call:** it is `f` times `x`. Write `f (*x)`.
- **`f x |> g` pipes two values,** `f` and `x`, into `g`. To pipe the result
  of `f x`, write `(f x) |> g`.
- **`f a.b` passes the path `a.b`,** not a field: fields are `a <- b`.
- **`x <- g` is a field, `x <- g$` a call.**
- **`*p <- field` dereferences the field,** not `p`: write `(*p) <- field`.
- **A struct literal written inline inside another** takes the outer one's
  remaining fields: bind the inner one first, or parenthesise it.

## The rules at work

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

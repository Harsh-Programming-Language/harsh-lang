# 13. Closures and iterators

Two features Rust took from the functional languages, and the two that most change how daily code reads. A *closure* is a function you write inline, that can use the variables around it. An *iterator* is a value that yields a sequence one item at a time, with a library of *adaptors* — `map`, `filter` and the rest — that take a closure each. Together they replace most `for` loops with a chain that says what is wanted rather than how to loop for it. Harsh has layout for both, and this is the chapter where its chain and closure forms earn their keep.

## 13.1 Closures

A closure captures its environment. Here one reads a struct field it was not passed:

@@ giveaway

`unwrap_or_else` takes a closure that produces the default: `(|| self <- most_stocked$)`, no parameters, a body that uses `self` from the enclosing method. Nothing was passed in; the closure *captured* `self`. A function could not do this — `fn` cannot see the variables of the function that defines it — and that is the difference: a closure is a function plus the environment it was written in. The closure is isolated in parentheses here because it is an argument with an inline body; it is the one case in the argument rule where the parentheses mark "this whole thing, closure and body, is one argument".

### Forms

A closure is `|parameters| body`, and the body is an expression or a block:

@@ closure_forms

Four spellings of `x + 1`. `add_one_v2` annotates everything and has a block body under a `:`; `v3` is the short form, inferred types and an expression; `v4` is inferred with a block. The `:` after the parameter list opens the body's block exactly as it does after a function signature, and the body indents from the closure. Annotations are optional because a closure is usually short and used once, close to where its types are obvious; the compiler infers them from the first use, and holds the closure to it:

@@ closure_infer !error

`example_closure` was called with a `String`, so that is its parameter type, and the second call with `5` is a type error. A closure has *one* signature, inferred once; it is not generic.

### Capturing

A closure captures each variable it uses in the least demanding way that works — by shared reference, by mutable reference, or by value — and the borrow checker treats the closure as holding that borrow from its creation to its last use:

@@ capture_borrow

`only_borrows` reads `list`, so it holds `&list`, and `list` can still be printed before and after the call, since shared borrows coexist. Now a closure that writes:

@@ capture_mut

`borrows_mutably` pushes, so it holds `&mut list` from `let` to the last call — and notice there is no `println!` between the two, because one would need `&list` while the mutable borrow is live, and chapter 4's rule would refuse it. The closure is `let mut` because calling it mutates its capture.

To make a closure take ownership of what it uses — needed when it will outlive the current function, as a closure handed to a new thread will — write `move`:

@@ capture_move

`move ||` moves `list` into the closure; the thread may then run after `main`'s frame is gone and still own its data. Without `move`, the closure would borrow `list`, and the compiler would reject the program because the thread might outlive the borrow. (Threads are chapter 16; this is a preview of why `move` exists.)

### `Fn`, `FnMut`, `FnOnce`

How a closure captures determines which of three traits it implements, and every function that takes a closure says which it needs:

- `FnOnce`: can be called at least once. Every closure implements it; a closure that *moves* a captured value out of its body implements *only* it, since after one call the value is gone.
- `FnMut`: can be called repeatedly and may mutate its captures.
- `Fn`: can be called repeatedly and touches nothing mutably — or captures nothing at all.

The type of a closure is written as the trait applied to its parameter types, the return after `->`:

@@ fn_types

@harsh `Fn i32 -> i32` is a closure taking one `i32` and returning one; `Fn (i32) (i32) -> i32` takes two, each parameter one atom, a group when it is more than a bare name; `FnOnce$` takes none, exactly as a call with none is written. The same spelling serves `impl Fn` in a parameter or a return type and `dyn Fn` inside a `Box`. There is nothing else to learn here: a closure type is an application like every other.

`unwrap_or_else` takes `FnOnce`, the most permissive, because it calls the closure at most once. `sort_by_key` calls its closure once per comparison, so it asks for `FnMut`:

@@ sort_by_key

`(|r| r <- width)` reads a field and returns it: it is `Fn`, which is also `FnMut`, so it qualifies. Here is one that does not:

@@ fn_once_err !error

The closure pushes `value` — a `String`, moved out of the capture — onto a vector. That can happen once, so the closure is `FnOnce` only, and `sort_by_key` needs to call it many times; the error names both facts. Mutating a capture *without* moving it is fine:

@@ fn_mut

`num_sort_operations += 1` mutates a captured counter, so the closure is `FnMut`, which is what was asked for, and the sort reports how many times it looked. The block-bodied closure is the trailing form from chapter 12 — `list <- sort_by_key |r|:` and the body beneath — with the argument list closing after the block.

## 13.2 Iterators

An iterator produces a sequence of values, one at a time, on request. The whole of the `Iterator` trait that matters is one method:

```
pub trait Iterator:
    type Item
    fn next (&mut self) -> Option<Self.Item>
```

`next` returns `Some item` until the sequence is finished and `None` after; `Item` is an *associated type* (chapter 20) naming what it yields. Everything else is built on `next`:

@@ iter_next

`v1 <- iter$` makes an iterator over references to the vector's elements — `Some (&1)`, a reference, because `iter` borrows — and each `next$` advances it, which is why `v1_iter` is `mut`. A `for` loop is exactly this: it calls `next` until `None`, and does not need the `mut` because it takes the iterator by value and does its own advancing. Three ways to make an iterator from a collection: `iter$` yields `&T`, `iter_mut$` yields `&mut T`, and `into_iter$` consumes the collection and yields `T`.

### Consumers and adaptors

Methods on iterators come in two kinds. A *consumer* calls `next` until the end and produces something:

@@ iter_sum

`sum$` drives the iterator to exhaustion and adds. `collect$` gathers into a collection (the type must be written or inferable); `count$`, `max$`, `min$`, `last$`, `find`, `any`, `all` and `fold` are consumers too. An *adaptor* takes one iterator and returns another that transforms its items as they pass:

@@ iter_map

`map (|x| x + 1)` yields each item plus one; `collect$` at the end consumes the result. That last step is not optional:

@@ iter_lazy

An adaptor alone does nothing, because iterators are *lazy*: `map` builds an iterator that will add one *when asked*, and nothing asks. The compiler warns that the value is unused ("iterators are lazy and do nothing unless consumed" — the warnings are off in the book's build, but you will see it). Every chain therefore ends in a consumer, or a `for`.

### Closures that capture their environment

An adaptor's closure can use variables from outside, which is most of their power:

@@ iter_filter

`filter (|s| s <- size == shoe_size)` keeps the shoes whose size equals a variable of the enclosing function; `into_iter$` consumes the vector so the kept shoes are moved into the result rather than copied. The function is three links on three lines — the vertical chain from the language guide's layout section, every `<-` aligned under the first — and reads top to bottom as "the shoes, filtered, collected".

### Chains

Adaptors compose, and this is where the chain layout matters:

@@ iter_chain

Filter, map, collect; map then sum; `find` for the first match; `any` for a yes-or-no; `enumerate$ <- skip 4` to number the items and drop the first four. Each closure is one atom in parentheses, and when the chain grows past a line it goes vertical. `any (|&w| w == "fig")` uses a `&w` pattern in the closure's parameter to take the `&&str` the iterator yields down to a `&str` — a pattern, as in a `for` or a `match`. Read the chains aloud and they are English.

### Laying a chain out

The layout has one rule for chains, and the formatter applies it, so you rarely decide it yourself:

@@ chain_layout

@harsh One or two links stay on their line when the line fits in 72 columns; three or more go vertical, one `<-` per line, every arrow under the first. A `=` always ends its line before a vertical chain begins, never starts one. A closure whose body is a block is written as a *paren block*: the `(` ends the arrow's line, the parameters sit on their own line, the body beneath them, and the `)` closes on a line of its own at the closure's column — the parentheses are transparent to the layout, and the block inside ends where they end. `hrs fmt` puts all of this where it belongs; write it however it comes out of your fingers, save, and read the result.

## 13.3 The pipes

Everything so far has reached into a value with `<-`: a method on the left of the arrow's data. The pipes go the other way. `|>` takes the value on its left and hands it to the *function* on its right; `<|` does the same from the other side. Where a chain says *this value, then this method on it*, a pipe says *this value, into this function*:

@@ pipe_basic

@harsh `text |> tokenize` is `tokenize` applied to `text`, and the chain reads on: the next `|>` applies the closure to what came out. `<|` is the mirror — `f <| g <| x` applies `g` to `x` and `f` to the result. Both sides of a pipe are *atoms*: a value, a name, an isolated group. That is the one place the pipes and the arrow part company. Chapter 2 said an application binds tighter than `<-` — `tokenize text <- len$` applies `tokenize` first — but `tokenize text |> count` does *not* apply it first: every atom on a pipe's side is an argument, so that line hands `count` two of them, `tokenize` and `text`. To pipe a *result*, isolate it: `(tokenize text) |> count`. The reason is the pipes' own feature — a pipe may carry several values, `2.0 0.5 |> scale` — and a rule that let one of them be an application would have to guess where it ended. So a chain like `raw <- clone$` is isolated before it goes in, `(raw <- clone$) |> trim_ws`, and a closure is isolated the same way, `|> (|w| count (&w))` — the function a pipe applies is one atom too.

### Partial application

The pipes do one more thing, and it is the thing the arrow cannot do. A pipe may carry several values — `2.0 0.5 3.0 |> scale` — and when it carries *fewer* than the function takes, the missing ones are **deferred**: the result is a closure waiting for the rest.

@@ pipe_partial

@harsh `|>` fills a function's parameters from the left, `<|` from the right, and the two together leave a hole in the middle. The result is one flat closure over whatever was not filled — a value like any other: bind it, pass it to `map`, return it from a function. There is no placeholder token; the hole is what is left. Harsh knows how many parameters `scale` takes because `scale` is declared in your project; for a function it cannot see into — the standard library, a crate you depend on — a pipe is a plain call with the values you gave, and the compiler says so if the count was wrong. Give a project function *more* than it takes and Harsh itself refuses:

@@ pipe_too_many !error

### Pipelines

A pipe's result is a value, so pipes chain: each stage's output is the next stage's input, left to right, and a partial application makes a stage out of a function that needed more than one argument:

@@ pipe_pipeline

`("[" "]" |> wrap)` is `wrap` with its first two parameters filled — a function of one `String` — and so it is a stage like `trim_ws` and `shout`. `banner` is the whole pipeline as a closure: the same stages, with the argument left for later. Read the line aloud: *raw, trimmed, shouted, wrapped*.

### When a pipe beats a closure

A closure that only forwards its argument is a partial application spelled the long way:

@@ pipe_vs_closure

`|p| discount 0.2 (*p)` names `p` twice to say what `0.2 |> discount` says once. When the closure transforms its argument on the way in — `p + 5.0` — it is doing work the pipe cannot, and it stays. That is the whole rule: reach for a partial when the argument passes through untouched, and a closure when it does not.

## 13.4 Improving `minigrep`

Chapter 12's program used `clone` in `Config.build` and a `for` loop with a `push` in `search`; iterators remove both:

@@ minigrep body poem.txt

`Config.build` now takes `impl Iterator<Item = String>` — the argument iterator itself, straight from `env.args$` — and pulls from it with `next$`: the program name is skipped, then `let Some query = args <- next$ else:` takes the query or returns the error. No vector, no indexes, no clones: each `String` is moved out of the iterator into the `Config`. And `search` is one expression, `contents <- lines$ <- filter (…) <- collect$`, three lines that say exactly what the loop did in six. Same output, same tests.

### Performance

The natural worry is that a chain of closures must be slower than a loop. It is not: Rust compiles adaptors and their closures down to the same machine code as the hand-written loop — sometimes better, since the compiler can see the whole chain — and the standard library's own benchmarks show no difference. This is the *zero-cost abstraction* Rust promises: what you do not use costs nothing, and what you do use costs what it would have cost by hand. Write the chain; it is the clearer of the two and not the slower.

## 13.5 What you have

Closures capture by reference, mutable reference or value, `move` to force the last; they are `Fn`, `FnMut` or `FnOnce` by what they do with their captures, and a function taking one says which it needs. Iterators produce items through `next`; `iter$`, `iter_mut$`, `into_iter$` make them; adaptors like `map` and `filter` are lazy and take closures; consumers like `sum$` and `collect$` run the chain; `for` is a consumer too. Harsh lays a chain out vertically with the arrows aligned and a closure's block body under its parameters. `|>` and `<|` hand a value to a function, from the left or the right; with fewer values than the function takes they defer the rest as a flat closure, which is how a partial application is written.

Next: cargo — profiles, documentation, publishing, workspaces — before the second half of the book turns to smart pointers and concurrency.

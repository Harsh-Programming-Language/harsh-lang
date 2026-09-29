# Harsh Design Patterns

*A Harsh companion to [Rust Design Patterns](https://rust-unofficial.github.io/patterns/) — its idioms, patterns and
anti-patterns, in the same order, with every program written in Harsh and
built and run to produce the output you see.*

A design pattern is a solution that has worked before, to a problem that keeps
coming back: a way to arrange code that other programmers will recognise. Rust
has its own, because it is not an object-oriented language: ownership, the
borrow checker, traits and closures make some classic patterns unnecessary and
call for others. *Rust Design Patterns*, a community book, collects them in
three families — **idioms**, the habits of the community; **design patterns**,
arrangements that solve a recurring problem; and **anti-patterns**, arrangements
that look like solutions and create problems.

Harsh is Rust with another layout, so every one of them holds in Harsh as it
is. What this companion adds is how each one is written in Harsh, and where
Harsh changes the picture: pipes and partial application make several patterns
a line long, comprehensions replace some loops outright, and the layout rules
make a builder or a command list read top to bottom. Where Harsh changes
nothing, the page says so and keeps short.

## How to read it

Each page gives the idea in a few sentences, a program in Harsh with its real
output, what Harsh changes, and a link to the original page — the place for
the full discussion, the trade-offs and the history, which this companion does
not repeat. If you have not read *The Harsh Programming Language* yet, read it
first: this book assumes you can read Harsh.

The book follows the original's order: the idioms; the design patterns —
behavioural, creational, structural, and for foreign functions; the
anti-patterns; functional programming; and, to close, the design principles
behind them all.

*Rust Design Patterns* is by its contributors, under the Mozilla Public
License 2.0, as is this companion; see ATTRIBUTION.md.

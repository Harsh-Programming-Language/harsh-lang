# The Harshonomicon

*A Harsh companion to [The Rustonomicon](https://doc.rust-lang.org/nomicon/) — the dark arts of unsafe Rust,
in the same order, with every program written in Harsh and built and run to
produce the output you see.*

*The Rustonomicon* is the book for the moment safe Rust is not enough: when
you write a data structure the borrow checker cannot follow, call into C, or
need to know exactly how a value sits in memory. It explains what `unsafe`
allows, what it does not, and the contracts you take on when you use it.

Harsh is Rust with another layout, and `unsafe` in Harsh is `unsafe` in Rust:
the same rules, the same undefined behaviour, the same compiler checking what
it can. Nothing in this book is a Harsh rule. What the companion adds is how
unsafe code reads in Harsh — `unsafe:` opening a block like any keyword, raw
pointers used with `<-` like any value — in programs you can run, each short,
each showing one rule at work.

Its chapters and sections are numbered as the original's, so a section here
and its original share a number. **This companion is shorter than its
original, on purpose.** Much of *The
Rustonomicon*'s value is its careful explanation of why each rule exists;
each page here states the rule in a few sentences of its own and links to the
original section. Before you write unsafe code for real, read the original.

Every program in this book is **well-defined**: none relies on undefined
behaviour, even to illustrate it. Where a page is about something that must
not be done, it shows the compiler refusing it, or the safe form.

*The Rustonomicon* is by the Rust Project developers, licensed under the
Apache License 2.0; this companion is under the Mozilla Public License 2.0, as
Harsh is. See ATTRIBUTION.md.

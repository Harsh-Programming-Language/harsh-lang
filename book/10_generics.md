# 10. Generic types, traits and lifetimes

Three mechanisms let one piece of code serve many types, and this chapter is all three. *Generics* are placeholders for types: a `Vec<T>` is one definition that is a vector of anything. *Traits* say what a type can do, so a generic function can require it: "any `T` that can be compared". And *lifetimes* are generics over how long a reference is valid, which is the question chapter 4 left open. They arrive together because they are used together, and because the last of them is the one people fear, unnecessarily, so it is best met right after the first two make the shape familiar.

## 10.1 Generics

### Removing duplication

Two functions that differ only in a type:

@@ largest_dup

Same body, twice, for `i32` and for `char`, and there would be a third for `f64`. The duplication is in the *type*, so the fix is a parameter for the type:

@@ largest_generic_err !error

`fn largest<T> list: &[T] -> &T` — the `<T>` after the name declares a type parameter, and then `T` is used where the type would be. It does not compile, and the error is the chapter's first lesson: `>` is not defined for *every* type, so the compiler will not let a function that takes *any* `T` compare two of them. It has to be told which `T`s are allowed, and the help says how: restrict the type parameter.

@@ largest_generic

`<T: PartialOrd>` — `T` may be any type that implements the trait `PartialOrd`, which is the trait that provides `>`. Now the body may compare, and the function works for `i32` and `char` and anything else that can be ordered, with one definition. Generics in Rust are checked at the definition, not at each use: a generic function has to make sense for *every* type its bounds allow, which is why the unbounded version was rejected and why, once it compiles, no call can break it. (At compile time each use is *monomorphized* — a copy is generated per concrete type — so there is no runtime cost for the abstraction.)

### Generic structs and methods

A struct's fields can be generic too:

@@ generic_struct

`struct Point<T>` with fields of type `T`; `Point\ x = 5, y = 10` is a `Point<i32>` and `Point\ x = 1.0, y = 4.0` a `Point<f64>`, inferred. Methods on a generic type are declared in `impl<T> Point<T>:` — the `<T>` after `impl` says *this block is generic over T* — and a method for one specific type is `impl Point<f32>:`, so `distance_from_origin` exists only on `Point<f32>`. When two fields may differ in type, two parameters, `Mixed<T, U>`.

Both fields of `Point<T>` are the same `T`, and the compiler holds you to it:

@@ generic_mismatch !error

## 10.2 Traits

A trait is a set of methods a type may implement — an interface, a protocol, whichever word your last language used. It is defined with `trait`, and a type implements it with `impl … for`:

@@ trait_def

`pub trait Summary:` declares one method by its signature, `fn summarize (&self) -> String`, with no body — that line ends at the return type. `impl Summary for NewsArticle:` gives the body for one type, `impl Summary for Tweet:` for another, and after that `tweet <- summarize$` and `article <- summarize$` are ordinary method calls. The trait is what makes the two types interchangeable to any code that only needs `summarize`.

One rule to know: you may implement a trait for a type only if the trait or the type is defined in your crate. `Summary` for `Vec<T>` — fine, `Summary` is yours; `Display` for `Tweet` — fine, `Tweet` is yours; `Display` for `Vec<T>` — not allowed, since both belong to someone else and two crates could disagree. It is called the *orphan rule* and it is what keeps trait implementations unambiguous across the whole ecosystem.

### Default implementations

A trait method may have a body, used by any type that does not supply its own:

@@ trait_default

`summarize` has a default that calls `summarize_author`, which has none; `Tweet` implements only the required one and gets the other free. A default may call the required methods, which is how a trait can offer a lot of behaviour and demand a little.

### Traits as parameters

A function that takes "anything summarizable" has four spellings, all in this example:

@@ trait_param

`item: &impl Summary` is the short form: a reference to some type that implements `Summary`. `notify2<T: Summary> item: &T` is the same thing with the type parameter named, which you need when two parameters must be the *same* type, or the name is used elsewhere. `&(impl Summary + Display)` requires two traits — `+` joins bounds, and the group is parenthesised so the `+` cannot be read as part of the parameter. And `[where T: Summary + Display]` moves the bounds out of the signature to a line of their own, which is what you do when they get long. The brackets are Harsh's: a `where` bound has a `:` in it, and the brackets keep that colon from opening a block; they are stripped on the way out.

`Tweet` also implements `Display` here, by writing `fmt` — that is the trait behind `{}`, the one chapter 5's error said `Rectangle` lacked. `write! f "…" args` writes into the formatter, and the method's two groups are `(&self)` and `(f: &mut std.fmt.Formatter)`.

@harsh A `where` clause on one line needs nothing: `fn f<T> (x: T) -> T where T: Clone:`. Over several lines it is written in brackets, `[where T: Summary + Display]` on its own line under the signature, because a bound's `:` would otherwise be read as opening a block; the brackets keep the clause part of the header and are not emitted, and the body indents as it would without them.


### Returning a trait

`impl Trait` works in return position too:

@@ trait_return

"This function returns *some* type that implements `Summary`" — the caller can call `summarize$` and nothing else. Useful when the concrete type is long or unnameable (closures and iterators, chapter 13). One limit: the function must return one concrete type; a function that returns a `Tweet` on one branch and a `NewsArticle` on another cannot use `impl Summary`, and needs a trait object (chapter 18).

### Conditional methods

A method can exist only when the type parameter meets a bound:

@@ blanket

`impl<T> Pair<T>:` gives every `Pair` a `new`; `impl<T: Display + PartialOrd> Pair<T>:` gives `cmp_display` only to pairs whose elements can be compared and printed. The standard library uses the same device on a larger scale: `impl<T: Display> ToString for T` implements `ToString` for *every* type that implements `Display`, which is why `42 <- to_string$` works — a *blanket implementation*, and a large part of why traits compose.

## 10.3 Lifetimes

Every reference has a *lifetime*: the region of the program during which it is valid. Usually the compiler works it out and you write nothing — every `&` so far had a lifetime you never saw. It has to be written only when the compiler cannot tell how the lifetimes of several references relate, and the classic case is a function that returns one of two borrowed arguments:

@@ longest_err !error

Read the help: the return type is a borrowed value, and the signature does not say whether it is borrowed from `x` or from `y`. The compiler needs to know, because the caller's borrow checker needs to know how long the returned reference may be used — as long as `x` lives, as long as `y` lives, or only as long as both do. The signature has to say. The syntax for saying it is a *lifetime parameter*:

@@ longest

`fn longest<'a> (x: &'a str) (y: &'a str) -> &'a str` — `'a` is declared after the name like a type parameter (the apostrophe marks it as a lifetime), and then `&'a str` means "a reference with lifetime `'a`". Putting the same `'a` on both inputs and the output says: the returned reference lives as long as the *shorter* of the two inputs. That is the whole meaning. Lifetime annotations do not change how long anything lives; they describe the relationship, so that the checker can verify calls against it.

And it does verify:

@@ longest_scope !error

`string2` lives only inside the `do:` block; `result` is used after it; and `longest`'s signature says `result` cannot outlive the shorter of its inputs. So the compiler rejects the program, and — this is the part to appreciate — it does so *at the call*, from the signature alone, without looking inside `longest`. That is what the annotation bought: a function's borrowing behaviour is part of its interface, checked at each use.

### Lifetimes in structs

Chapter 5's `&str` field had this error. A struct that holds a reference declares a lifetime, and the struct cannot outlive what it borrows:

@@ struct_lifetime

`struct ImportantExcerpt<'a>` with `part: &'a str` means an `ImportantExcerpt` is valid only while the text it points into is. `impl<'a> ImportantExcerpt<'a>:` declares the lifetime for the methods, which then mostly do not mention it — `level` takes `&self` and returns an `i32`; `announce_and_return_part` returns a `&str` whose lifetime the compiler works out from `&self`, by the rules below.

### Elision

If every reference needed a written lifetime the language would be unusable, so three rules fill them in when they can, and you write one only when they cannot:

@@ elision

1. Each reference parameter gets its own lifetime.
2. If there is exactly one input lifetime, the output gets it — which is why chapter 4's `first_word s: &str -> &str` needed nothing.
3. If one of the inputs is `&self`, the output gets `self`'s lifetime — which is why `announce_and_return_part` needed nothing.

`longest` had two inputs and no `self`, so no rule applied and the signature had to say. That is the entire theory of when you write lifetimes. `'static` is the one named lifetime you will see in the wild: the whole program, which every string literal has, since the text lives in the binary.

### All three at once

@@ all_together

A lifetime, a type parameter, a bound in a `[where …]` clause, and three parameters: everything in this chapter in one signature, and nothing in it is new.

## 10.4 What you have

`<T>` after a name makes a function, struct or `impl` generic; `T: Trait` restricts it, and the body may only do what the bounds allow. `trait` declares methods, with or without defaults; `impl Trait for Type` supplies them; `&impl Trait`, `T: Trait`, `+`, and `[where …]` take them as parameters and `impl Trait` returns one. The orphan rule keeps implementations unambiguous. A lifetime `'a` names how long a reference is valid, is written only when elision's three rules cannot fill it in, and lets the checker verify each call from the signature alone.

Next: writing automated tests — which is the natural use of everything so far, and where `panic` is a feature.

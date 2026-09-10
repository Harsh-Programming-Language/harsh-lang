# 18. Object-oriented features

Is Rust object-oriented? It has objects in the sense that matters — data with methods, encapsulated behind a public interface — and it has polymorphism, through traits. It does not have inheritance, on purpose, and this chapter is about what it offers instead: *trait objects*, which let a collection hold values of different types and call the same method on each, and the design consequences of choosing that over a class hierarchy.

## 18.1 Encapsulation

A struct with private fields and public methods is an object in the classic sense:

@@ encapsulation

`list` and `average` are private; `add`, `remove` and `average` are the interface; `update_average` is an implementation detail no caller can see. The cached average can never be stale, because every path that changes the list goes through a method that recomputes it — and the representation can change (a `HashSet`, say) without any caller noticing. This is chapter 7's privacy doing the job classes do elsewhere, and it needs no new feature.

## 18.2 Trait objects

Chapter 8 held several types in one `Vec` by wrapping them in an enum, which works when the set of types is known when you write the enum. A GUI library cannot know what components its users will define. It needs "a vector of anything that can draw itself":

@@ trait_object

`Vec<Box<dyn Draw>>` is that vector. `dyn Draw` is a *trait object*: some type, unknown at compile time, that implements `Draw`. It must sit behind a pointer — `Box`, `&`, `Rc` — because different types have different sizes and the vector's elements must all be the same size, and a pointer is. The `for` loop calls `component <- draw$` on each, and the right implementation is found *at run time* through a table of method pointers the trait object carries — *dynamic dispatch*, as against the compile-time dispatch of generics. `Button` is defined here and `SelectBox` could be defined by a user of the library; `Screen` needs to know neither.

The compiler still checks that every element can draw:

@@ not_draw !error

A `String` does not implement `Draw`, so it cannot become a `Box<dyn Draw>`, and the error says so at the point of the cast. Duck typing with a type checker: anything goes in, provided it has the method, and "provided" is verified.

### Trait objects or generics

The same screen could be written with a type parameter:

@@ generic_vs_dyn

`Screen<T: Draw>` holds a `Vec<T>` — a vector of *one* type that implements `Draw`, chosen per screen at compile time, with no boxing and static dispatch. That is the faster and the more restrictive choice: a `Screen<Button>` cannot hold a `SelectBox`. Use generics when all the elements are the same type, which is the common case and the compiler will optimise it fully; use `dyn` when they are not, and pay the pointer and the indirect call, which is small. The choice is one word in the type and it is a real design decision, not a style one.

One limit: not every trait can be a trait object. The method signatures must not mention `Self` as a return type or have type parameters — the compiler needs to know the method's calling convention without knowing the concrete type. `Clone` is the usual casualty (`fn clone (&self) -> Self`); a trait like that is *not object safe* and the error says so when you try.

## 18.3 The state pattern

An object-oriented design pattern, done in Rust to see what fits and what does not. A blog post is a draft, then pending review, then published; its behaviour — what `content` returns, what `approve` does — depends on which. In the classic pattern each state is an object, and the post delegates to whichever it holds:

@@ state_pattern

`Post` holds `Option<Box<dyn State>>`; `Draft`, `PendingReview` and `Published` are unit structs implementing `State`; `request_review` and `approve` each *consume* the current state (`self: Box<Self>` — a method that takes the box by value, which is how a state can hand back a different one) and return the next. `Post`'s methods use `take$` to move the state out of the `Option`, call the transition, and put the result back — the `Option` exists so that the state can be moved out of a `&mut self` without leaving a hole. `content` has a default that returns `""`, overridden only by `Published`. `Post` knows nothing about the transitions; adding a state means adding a type, not editing a `match`.

It works, and it has the pattern's usual costs: the states know about each other, a transition that is invalid for a state has to be written as "return `self`", and nothing stops a caller from asking a draft for its content. Rust offers a different shape — encode the states as *types*:

@@ state_types

Now `Post.new$` returns a `DraftPost`, which has `add_text` and `request_review` and *no* `content` method; `request_review` consumes it and returns a `PendingReviewPost`, which has only `approve`; and only a `Post` has `content`. Each transition is `let post = post <- …` — shadowing, chapter 3 — because each is a new type. An invalid transition is not a no-op or a runtime error; it does not compile, because the method does not exist on that type. The workflow is in the type signatures, where a reader finds it and the compiler enforces it.

That is the chapter's real lesson. Rust can express the object-oriented patterns, and sometimes they are right; but a pattern that exists to make invalid states unrepresentable at run time is often better expressed as types that make them unrepresentable at compile time. Chapter 9 said the same about `Guess`. Reach for the type first.

## 18.4 What you have

Structs with private fields and public methods are encapsulated objects. `Box<dyn Trait>` is a trait object — any type implementing the trait, dispatched at run time, sized by the pointer — for collections of mixed types, checked at the cast. Generics for one type chosen at compile time. `self: Box<Self>` for a method that consumes a boxed value and returns a replacement. And the state pattern, done twice: as trait objects, and as types that make the invalid transitions uncompilable.

Next: patterns — every place they appear and every form they take, since you have been using them since chapter 2.

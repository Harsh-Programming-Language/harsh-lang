# 18. Object-oriented features

Is Rust object-oriented? It has objects in the sense that matters — data with methods, encapsulated behind a public interface — and it has polymorphism, through traits. It does not have inheritance, on purpose, and this chapter is about what it offers instead: *trait objects*, which let a collection hold values of different types and call the same method on each, and the design consequences of choosing that over a class hierarchy.

## 18.1 Encapsulation

A struct with private fields and public methods is an object in the classic sense:

```
pub struct AveragedCollection
    list: Vec<i32>
    average: f64

impl AveragedCollection
    pub fn new$ -> Self:
        Self\ list = vec! [], average = 0.0

    pub fn add (&mut self) (value: i32):
        self <- list <- push value
        self <- update_average$

    pub fn remove (&mut self) -> Option<i32>:
        let result = self <- list <- pop$

        match result:
            Some value => do:
                self <- update_average$
                Some value
            None => None

    pub fn average (&self) -> f64:
        self <- average

    fn update_average (&mut self):
        let total: i32 =
            self <- list
                 <- iter$
                 <- sum$
        self <- average = total as f64 / self <- list <- len$ as f64

fn main$:
    let mut c = AveragedCollection.new$
    c <- add 3
    c <- add 5
    println! "{}" (c <- average$)
    c <- remove$
    println! "{}" (c <- average$)
```

```text
4
3
```

`list` and `average` are private; `add`, `remove` and `average` are the interface; `update_average` is an implementation detail no caller can see. The cached average can never be stale, because every path that changes the list goes through a method that recomputes it — and the representation can change (a `HashSet`, say) without any caller noticing. This is chapter 7's privacy doing the job classes do elsewhere, and it needs no new feature.

## 18.2 Trait objects

Chapter 8 held several types in one `Vec` by wrapping them in an enum, which works when the set of types is known when you write the enum. A GUI library cannot know what components its users will define. It needs "a vector of anything that can draw itself":

```
pub trait Draw
    fn draw (&self)

pub struct Screen
    pub components: Vec<Box<dyn Draw>>      // any type that implements Draw, boxed

impl Screen
    pub fn run (&self):
        for component in self <- components <- iter$:
            component <- draw$

pub struct Button
    pub width: u32
    pub height: u32
    pub label: String

impl Draw for Button
    fn draw (&self):
        println!
            "button {}x{} '{}'"
            (self <- width)
            (self <- height)
            (self <- label)

struct SelectBox
    width: u32
    height: u32
    options: Vec<String>

impl Draw for SelectBox
    fn draw (&self):
        println!
            "select box {}x{} {:?}"
            (self <- width)
            (self <- height)
            (self <- options)

fn main$:
    let screen =
        Screen\
            components =
                vec! [
                    (Box.new (SelectBox\
                        width = 75
                        height = 10
                        options = vec! [String.from "Yes", String.from "Maybe", String.from "No"]
                     )),
                         (Box.new (Button\
                             width = 50
                             height = 10
                             label = String.from "OK"
                          )),
                          ]
    screen <- run$
```

```text
select box 75x10 ["Yes", "Maybe", "No"]
button 50x10 'OK'
```

`Vec<Box<dyn Draw>>` is that vector. `dyn Draw` is a *trait object*: some type, unknown at compile time, that implements `Draw`. It must sit behind a pointer — `Box`, `&`, `Rc` — because different types have different sizes and the vector's elements must all be the same size, and a pointer is. The `for` loop calls `component <- draw$` on each, and the right implementation is found *at run time* through a table of method pointers the trait object carries — *dynamic dispatch*, as against the compile-time dispatch of generics. `Button` is defined here and `SelectBox` could be defined by a user of the library; `Screen` needs to know neither.

The compiler still checks that every element can draw:

```
pub trait Draw
    fn draw (&self)

pub struct Screen
    pub components: Vec<Box<dyn Draw>>

fn main$:
    let screen =
        Screen\
            components = vec! [Box.new (String.from "Hi")]
    let _ = screen
```

```text
error[E0277]: the trait bound `String: Draw` is not satisfied
  --> not_draw.hrs:10:32
   |
10 |             components = vec! [Box.new (String.from "Hi")]
   |                                ^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `Draw` is not implemented for `String`
   = help: this trait has no implementations, consider adding one (hrs 1:1)
   = note: required for the cast from `Box<String>` to `Box<dyn Draw>`
```

A `String` does not implement `Draw`, so it cannot become a `Box<dyn Draw>`, and the error says so at the point of the cast. Duck typing with a type checker: anything goes in, provided it has the method, and "provided" is verified.

### Trait objects or generics

The same screen could be written with a type parameter:

```
pub trait Draw
    fn draw (&self)

// Generic: one T for the whole screen, chosen at compile time, no boxing.
pub struct Screen<T: Draw>
    pub components: Vec<T>

impl<T> Screen<T> [where T: Draw]
    pub fn run (&self):
        for component in self <- components <- iter$:
            component <- draw$

struct Label String

impl Draw for Label
    fn draw (&self):
        println! "label '{}'" (self.0)

fn main$:
    let screen = Screen\ components = vec! [Label (String.from "a"), Label (String.from "b")]
    screen <- run$
```

```text
label 'a'
label 'b'
```

`Screen<T: Draw>` holds a `Vec<T>` — a vector of *one* type that implements `Draw`, chosen per screen at compile time, with no boxing and static dispatch. That is the faster and the more restrictive choice: a `Screen<Button>` cannot hold a `SelectBox`. Use generics when all the elements are the same type, which is the common case and the compiler will optimise it fully; use `dyn` when they are not, and pay the pointer and the indirect call, which is small. The choice is one word in the type and it is a real design decision, not a style one.

One limit: not every trait can be a trait object. The method signatures must not mention `Self` as a return type or have type parameters — the compiler needs to know the method's calling convention without knowing the concrete type. `Clone` is the usual casualty (`fn clone (&self) -> Self`); a trait like that is *not object safe* and the error says so when you try.

## 18.3 The state pattern

An object-oriented design pattern, done in Rust to see what fits and what does not. A blog post is a draft, then pending review, then published; its behaviour — what `content` returns, what `approve` does — depends on which. In the classic pattern each state is an object, and the post delegates to whichever it holds:

```
pub struct Post
    state: Option<Box<dyn State>>
    content: String

impl Post
    pub fn new$ -> Post:
        Post\ state = Some (Box.new Draft), content = String.new$

    pub fn add_text (&mut self) (text: &str):
        self <- content <- push_str text

    pub fn content (&self) -> &str:
        self <- state
             <- as_ref$
             <- unwrap$
             <- content self

    pub fn request_review (&mut self):
        if let Some s = self <- state <- take$:
            self <- state = Some (s <- request_review$)

    pub fn approve (&mut self):
        if let Some s = self <- state <- take$:
            self <- state = Some (s <- approve$)

trait State
    fn request_review (self: Box<Self>) -> Box<dyn State>
    fn approve (self: Box<Self>) -> Box<dyn State>

    fn content<'a> (&self) (_post: &'a Post) -> &'a str:
        ""

struct Draft

impl State for Draft
    fn request_review (self: Box<Self>) -> Box<dyn State>:
        Box.new PendingReview

    fn approve (self: Box<Self>) -> Box<dyn State>:
        self

struct PendingReview

impl State for PendingReview
    fn request_review (self: Box<Self>) -> Box<dyn State>:
        self

    fn approve (self: Box<Self>) -> Box<dyn State>:
        Box.new Published

struct Published

impl State for Published
    fn request_review (self: Box<Self>) -> Box<dyn State>:
        self

    fn approve (self: Box<Self>) -> Box<dyn State>:
        self

    fn content<'a> (&self) (post: &'a Post) -> &'a str:
        &post <- content

fn main$:
    let mut post = Post.new$
    post <- add_text "I ate a salad for lunch today"
    println! "draft: '{}'" (post <- content$)
    post <- request_review$
    println! "pending: '{}'" (post <- content$)
    post <- approve$
    println! "published: '{}'" (post <- content$)
```

```text
draft: ''
pending: ''
published: 'I ate a salad for lunch today'
```

`Post` holds `Option<Box<dyn State>>`; `Draft`, `PendingReview` and `Published` are unit structs implementing `State`; `request_review` and `approve` each *consume* the current state (`self: Box<Self>` — a method that takes the box by value, which is how a state can hand back a different one) and return the next. `Post`'s methods use `take$` to move the state out of the `Option`, call the transition, and put the result back — the `Option` exists so that the state can be moved out of a `&mut self` without leaving a hole. `content` has a default that returns `""`, overridden only by `Published`. `Post` knows nothing about the transitions; adding a state means adding a type, not editing a `match`.

It works, and it has the pattern's usual costs: the states know about each other, a transition that is invalid for a state has to be written as "return `self`", and nothing stops a caller from asking a draft for its content. Rust offers a different shape — encode the states as *types*:

```
// The same workflow with the states as types: an invalid transition does not compile.
pub struct Post
    content: String

pub struct DraftPost
    content: String

pub struct PendingReviewPost
    content: String

impl Post
    pub fn new$ -> DraftPost:
        DraftPost\ content = String.new$

    pub fn content (&self) -> &str:
        &self <- content

impl DraftPost
    pub fn add_text (&mut self) (text: &str):
        self <- content <- push_str text

    pub fn request_review self -> PendingReviewPost:
        PendingReviewPost\ content = self <- content

impl PendingReviewPost
    pub fn approve self -> Post:
        Post\ content = self <- content

fn main$:
    let mut post = Post.new$
    post <- add_text "I ate a salad for lunch today"

    let post = post <- request_review$     // a DraftPost has no `content` method to misuse
    let post = post <- approve$
    println! "{}" (post <- content$)
```

```text
I ate a salad for lunch today
```

Now `Post.new$` returns a `DraftPost`, which has `add_text` and `request_review` and *no* `content` method; `request_review` consumes it and returns a `PendingReviewPost`, which has only `approve`; and only a `Post` has `content`. Each transition is `let post = post <- …` — shadowing, chapter 3 — because each is a new type. An invalid transition is not a no-op or a runtime error; it does not compile, because the method does not exist on that type. The workflow is in the type signatures, where a reader finds it and the compiler enforces it.

That is the chapter's real lesson. Rust can express the object-oriented patterns, and sometimes they are right; but a pattern that exists to make invalid states unrepresentable at run time is often better expressed as types that make them unrepresentable at compile time. Chapter 9 said the same about `Guess`. Reach for the type first.

## 18.4 What you have

Structs with private fields and public methods are encapsulated objects. `Box<dyn Trait>` is a trait object — any type implementing the trait, dispatched at run time, sized by the pointer — for collections of mixed types, checked at the cast. Generics for one type chosen at compile time. `self: Box<Self>` for a method that consumes a boxed value and returns a replacement. And the state pattern, done twice: as trait objects, and as types that make the invalid transitions uncompilable.

Next: patterns — every place they appear and every form they take, since you have been using them since chapter 2.

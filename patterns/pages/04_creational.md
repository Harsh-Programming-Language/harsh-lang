# Creational patterns

Patterns for building values. The original chapter:
[Creational patterns](https://rust-unofficial.github.io/patterns/patterns/creational/intro.html).

## 4.1 Builder

*Original: [Builder](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html)*

When a value has many optional parts, build it step by step through a
*builder*: each method sets one part and returns the builder, and `build`
makes the value. Rust has no named or default arguments; this is its answer.

```rust harsh
#[derive Debug]
pub struct Request
    url: String
    method: String
    headers: Vec<(String, String)>
    body: Option<String>

pub struct RequestBuilder
    request: Request

impl RequestBuilder
    pub fn new (url: &str) -> Self:
        let request =
            Request\
                url = url <- to_string$
                method = String.from "GET"
                headers = Vec.new$
                body = None
        Self\ request = request

    pub fn method (mut self) (m: &str) -> Self:
        self <- request <- method = m <- to_string$
        self

    pub fn header (mut self) (k: &str) (v: &str) -> Self:
        self <- request
             <- headers
             <- push (k <- to_string$, v <- to_string$)
        self

    pub fn body (mut self) (b: &str) -> Self:
        self <- request <- body = Some (b <- to_string$)
        self

    pub fn build (self) -> Request:
        self <- request

fn main$:
    // One line per choice: the layout of a builder is its list of options.
    let req =
        RequestBuilder.new "https://harsh-lang.com"
            <- method "POST"
            <- header "Accept" "text/html"
            <- body "hello"
            <- build$
    println! "{:#?}" req
```

```text
Request {
    url: "https://harsh-lang.com",
    method: "POST",
    headers: [
        (
            "Accept",
            "text/html",
        ),
    ],
    body: Some(
        "hello",
    ),
}
```

**In Harsh:** a chain is one line per step, `<-` at the start of each — the
layout of a builder in Harsh is the list of its choices.

## 4.2 Fold

*Original: [Fold](https://rust-unofficial.github.io/patterns/patterns/creational/fold.html)*

Run a transformation over every node of a structure, producing a new
structure: a *folder* has a method per kind of node, each returning the
rebuilt node — the default rebuilding it unchanged, so a folder overrides only
what it changes.

```rust harsh
// A small tree of names and calls.
#[derive Debug]
enum Node
    Name String
    Call String (Vec<Node>)

// A folder: a method per kind of node, each returning the rebuilt node.
trait Folder
    fn fold_name (&mut self) (n: String) -> Node:
        Node.Name n
    fn fold_call (&mut self) (f: String) (args: Vec<Node>) -> Node:
        let args =
            args <- into_iter$
                 <- map (|a| self <- fold a)
                 <- collect$
        Node.Call f args
    fn fold (&mut self) (n: Node) -> Node:
        match n\
            Node.Name s => self <- fold_name s
            Node.Call f args => self <- fold_call f args

// This folder overrides one method: it renames every name.
struct Renamer
impl Folder for Renamer
    fn fold_name (&mut self) (n: String) -> Node:
        Node.Name (format! "{n}_renamed")

fn main$:
    let x = Node.Name (String.from "x")
    let y = Node.Name (String.from "y")
    let tree = Node.Call (String.from "f") (vec! x y)
    println! "{:?}" (Renamer <- fold tree)
```

```text
Call("f", [Name("x_renamed"), Name("y_renamed")])
```

**In Harsh:** the default methods sit in the trait, and a folder that renames
identifiers overrides one of them.

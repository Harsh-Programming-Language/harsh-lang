# 5. Structs

A *struct* is a type you define: a name for a bundle of values that belong together — a user, a rectangle, a request. Chapter 4's rules apply to a struct exactly as to a `String`: a struct owns its fields, is moved on assignment unless it is `Copy`, and is borrowed with `&`. This chapter is the definitions, the ways of building a value, and the methods that give a type its behaviour.

There are three forms of struct, and they are three equals, not one standard form and two odd ones. A **record struct** has named fields — Rust's own documentation calls this one simply "a struct", so that is what you will see it called elsewhere. A **tuple struct** has fields by position, with no names of their own. A **unit-like struct** has no fields at all. Every one of them can be declared, built and taken apart, and chapter 6 will show that each variant of an enum is one of these three.

This is also the first chapter where Harsh has a shape of its own to show. A record struct's fields are lines under the heading, one per line, with no commas and no braces, the way a function's body sits under its signature; a tuple struct's field types follow its name the way arguments follow a function; a unit-like struct is its name and nothing more. A *value* of a record struct is laid out as its declaration is, with a `\` after the name to say that a field list follows — the one mark this chapter adds.

## 5.1 The three forms

@@ three_forms

@harsh A record struct's fields may sit beneath the name, one per line, or on the declaration's own line after a `\` — `struct Size\ w: f64, h: f64` — and the same choice exists for a value: `Point\ x = 1.0, y = 2.0` on one line, or `Size\` with a field per line beneath. Neither is the preferred form; the block reads better when the fields are many or their names long, the inline form when the type is small, and this book uses whichever the example reads best in. A tuple struct is the name *applied* to its field types, `struct Meters f64`, exactly as `Meters 5.5` applies it to a value. A unit-like struct is a name, and `Origin` is also its only value.

### Record structs

@@ define

The definition is `struct User` and four indented fields, each `name: Type`. Nothing marks the body: only a name can follow `struct`, so a deeper line can only be a field. An *instance* is the struct's name, a `\`, and every field given a value with `=` — a line each here, or all on one line after the mark, `User\ active = true, …`. Order does not matter, but every field must be present, and the `\` is what says the name is not finished: a bare `User` is already a complete expression. Inline, the field list reaches the end of its line, or the `)` of the group it is written in — so `Point\ x = 1, msg` is one literal with the shorthand field `msg`, with or without parens around it, and the compiler is the one to say whether `msg` is a field `Point` has. When a literal is one element of a tuple, give it its own parens: `((Point\ x = 1), msg)`. A field is read with `<-`: `user1 <- email`. The whole instance is mutable or it is not — `let mut user1` — and then a field is assigned with `<-` on the left of `=`. There is no way to mark a single field mutable, and this is deliberate: mutability is a property of the binding, and a reader who sees `let user1` knows the entire value is fixed.

Notice the parentheses around `(user1 <- username)` when it is handed to `println!`. A field access has an operator in it, so, like every other operator expression, it is isolated when it is an argument; `println! "{}" user1 <- username` would read as `println!` applied to `user1`, followed by an arrow the macro does not want. The rule is chapter 3's and chapter 4's, applied again.

### Building a struct in a function

A function that returns a struct is the ordinary way to construct one with defaults:

@@ build_user

`username,` and `email,` inside the literal are the *field init shorthand*: when the variable has the same name as the field, the field name alone does both jobs. Not required, but universal in real code, and it is why parameters in constructors tend to be named after fields.

### Building one instance from another

Often a new instance is an old one with a few fields changed. The *struct update syntax*, `..user1`, says "and every field I did not mention comes from `user1`":

@@ update

It must be last in the literal. And it moves: `..user1` copies the `Copy` fields (`active`, `sign_in_count`) and *moves* the rest (`username`), exactly as `let x = user1.username` would. After it, `user1` is partly gone — `user1 <- active` is still usable, `user1 <- username` is not:

@@ update_move !error

The compiler tracks moves per field. That is the chapter 4 rule at a finer grain, and the error says which field went where.

### Tuple structs

A tuple struct has a name and positional fields with no names of their own — a tuple with a type:

@@ tuple_structs

`struct Color i32 i32 i32` declares: the field types follow the name, juxtaposed, the way arguments follow a function. `Color 0 0 0` constructs, by juxtaposition, three arguments — the same rule as a function call, because in Rust a tuple struct's name *is* a constructor function. Fields are `black.0`, `black.1`: a tuple index keeps its dot and is part of the name, so `black.0` is an atom and needs no parentheses as an argument. And a value is taken apart with a pattern of the same shape as the constructor, `let Point x y z = origin`. `Color` and `Point` have the same fields and are different types; a function taking a `Color` will not accept a `Point`, which is the point of naming them.

### Unit-like structs

A struct can have no fields at all:

@@ unit_struct

`struct AlwaysEqual` is the whole definition — nothing after the name — and `AlwaysEqual` is also its only value. Such a type exists to carry behaviour (chapter 10 puts traits on one) rather than data.

### Ownership of a struct's data

Every `String` in `User` is owned by the instance: when the instance is dropped, its strings are. That is why the fields were `String` and not `&str`. Try the reference:

@@ str_field !error

A struct holding a reference must say how long the referenced data lives — the *lifetime* the error asks for — so that the compiler can check the struct never outlives what it points at. That is chapter 10; until then, structs own their data, which is the common case and the one that needs no annotation.

## 5.2 An example program

Here is a small program built three times, to show what a struct does for readability. Compute the area of a rectangle, first with two separate variables:

@@ rect_vars

It works, and `area` takes two parameters that are related — they are one rectangle's width and height — without saying so. Group them in a tuple:

@@ rect_tuple

Now one argument, but `dimensions.0` and `dimensions.1` are worse than `width` and `height`: the reader has to know which is which, and so does the next person who edits it. Name them:

@@ rect_struct

`area` now takes `&Rectangle` — a borrow, so `main` keeps `rect1` — and reads `rectangle <- width * rectangle <- height`. The code says what it means. Application binds tighter than `<-`, so the two accesses are the operands of `*` and nothing needs isolating on that line; only the call `(area (&rect1))` in the `println!` does, twice, for the two operators in it.

### Printing a struct

`println!` with `{}` knows how to print numbers and strings; it does not know how to print a `Rectangle`, and it says so:

@@ no_debug !error

`{}` uses the `Display` trait, which is for user-facing output, and Rust will not guess what a user-facing rectangle looks like. The `{:?}` in the message is the *debug* format, which can be derived automatically — read the compiler's note, which tells you exactly what to add:

@@ derive_debug

`#[derive Debug]` is an *attribute*: a line above the definition that asks the compiler to generate an implementation of `Debug` for the type. Inside its brackets an attribute is an application like any other — `derive` applied to `Debug`, and to more, `#[derive Debug Clone PartialEq]`; an attribute that takes a keyed value isolates it, `#[cfg (feature = "fast")]`. `{:?}` prints on one line; `{:#?}` pretty-prints with each field on its own. You will derive `Debug` on almost every struct you write. There is also `dbg!`, a macro that prints an expression *and its value* to standard error along with the file and line, and hands the value back — `dbg! (30 * scale)` inside a larger expression is how you look at an intermediate result without restructuring the code.

## 5.3 Methods

A *method* is a function that belongs to a type. It is defined inside an `impl` block and its first parameter is `self`, the instance it is called on:

@@ method_area

`impl Rectangle:` opens the block that holds the type's methods. `fn area (&self) -> u32:` is `area` with one parameter, `&self` — a borrow of the instance, in its own group — and the body reads the fields through it. The call is `rect1 <- area$`: the arrow selects the method on `rect1`, `$` applies it to nothing, and `self` is `&rect1`. `rect1 <- area$` is the whole call, so as an argument it is isolated like any expression with operators in it.

`&self` is the choice you make most: read, do not consume. `&mut self` when the method changes the instance; plain `self` — taking ownership — rarely, for a method that transforms the value into something else and does not want the old one used again. The three are chapter 4's three ways of passing a value, applied to the receiver.

A method may have the same name as a field. `rect1 <- width$` is the method, `rect1 <- width` the field; the `$` tells them apart. The usual reason to do this is a *getter*: the field private, the method public, so the value can be read but not set from outside the module — chapter 7.

### More parameters

After `self`, a method's parameters are groups like any function's:

@@ method_params

`fn can_hold (&self) (other: &Rectangle)` — the receiver, then one parameter. The call, `rect1 <- can_hold (&rect2)`, applies the method to one argument, isolated because of its `&`.

### Associated functions

A function in an `impl` block that does *not* take `self` is an *associated function*: it belongs to the type but not to an instance. Constructors are the classic case:

@@ assoc_fn

`fn square size: u32 -> Self:` — `Self` inside an `impl` is an alias for the type, so this returns a `Rectangle`. It is called with the path form, `Rectangle.square 3`: a dot, because this is a path into the type's namespace, not a method on a value. `String.from` and `String.new$` are the same thing, and now you know what they are. A type may have several `impl` blocks, as here; there is no reason to split them in a program this size, but it is legal, and chapter 10 shows when it is useful.

## 5.4 What you have

A struct is named fields under a heading; an instance is the name, a `\`, and its fields with `=`, with shorthand and `..other` to fill it. Tuple structs are constructed and matched by juxtaposition; a unit struct is a name alone. Structs own their data until chapter 10 says how they may borrow it. `#[derive Debug]` and `{:?}` to see one. `impl` holds the methods, `&self` reads, `&mut self` changes, `self` consumes, and `rect1 <- area$` calls; a function without `self` is associated with the type and called through its path, `Rectangle.square 3`.

Next: enums — types whose value is one of several named shapes — and `match`, which you met in chapter 2 and can now understand.

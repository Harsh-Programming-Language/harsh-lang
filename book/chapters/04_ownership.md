# 4. Ownership

This is the chapter the book exists for. Ownership is the idea that is Rust's alone: the reason it needs no garbage collector and no manual `free`, the reason the compiler will refuse programs that every other language accepts, and the reason those programs, once accepted, do not have the bugs the others do. Harsh changes nothing here. There is no notation to learn in this chapter beyond what you have, and every example is a conversation with the compiler about who is responsible for a value. Read the errors as carefully as the programs; they are the lesson.

## 4.1 What ownership is

### The stack, the heap, and the question

A running program keeps values in two places. The **stack** holds values whose size is known at compile time — an `i32`, a `bool`, a fixed array — and it is fast because it is simple: push on the way in, pop on the way out, in strict order. The **heap** holds values whose size is not known in advance or may change — the text of a string typed by the user, a list that grows — and it is slower because someone has to find a free region, record where it is, and later give it back.

That "give it back" is the whole problem. Give the memory back too early and something still using it reads garbage; give it back twice and the allocator's bookkeeping is corrupted; never give it back and the program leaks. Garbage-collected languages solve this by never letting you free anything and periodically finding what is unreachable. C solves it by trusting you. Rust solves it with a rule the compiler can check.

### The rules

- Every value has exactly one **owner**: the variable that holds it.
- When the owner goes out of scope, the value is **dropped** — its memory is freed, immediately and deterministically.
- There is only ever one owner at a time, so a value is freed exactly once.

Scope is the ordinary thing: a variable is valid from where it is declared until the end of the block that declares it.

```
fn main$:
    do:
        let s = "hello"          // s is valid from here
        println! "{}" s          // and usable until the block ends
    // the block is over: s is gone, and there is nothing to free

    let t = String.from "hello"  // a String owns memory on the heap
    println! "{}" t
    // main ends here: t goes out of scope and its memory is freed
```

```text
hello
hello
```

`s` holds a string *literal*, text baked into the program, which lives on the stack and costs nothing to drop. `t` holds a `String`, which owns a buffer on the heap. `String.from` allocates it; when `t` goes out of scope at the end of `main`, Rust frees it. You wrote no `free`, and none was needed, because the compiler knows exactly where `t`'s scope ends and puts the call there itself. That is ownership from the value's point of view: an owner, and a drop at the end of the owner's scope.

`String` is the type you reach for when text is not fixed at compile time — it can grow:

```
fn main$:
    let mut s = String.from "hello"
    s <- push_str ", world"       // append to the heap buffer
    s <- push '!'                 // append one character
    println! "{}" s
    println! "{} bytes" (s <- len$)
```

```text
hello, world!
13 bytes
```

`push_str` and `push` append to the buffer, reallocating if it is full. `s <- len$` is isolated in parentheses because it is an argument to `println!` and is not a single atom — the same rule chapter 3 gave for an index expression.

### Move

Now the rule bites. What happens when a `String` is assigned to a second variable?

```
fn main$:
    let s1 = String.from "hello"
    let s2 = s1                   // s1 is moved into s2
    println! "{}, world" s1       // s1 is no longer valid
```

```text
error[E0382]: borrow of moved value: `s1`
  --> move.hrs:4:26
   |
 2 |     let s1 = String.from "hello"
   |         -- move occurs because `s1` has type `String`, which does not implement the `Copy` trait
 3 |     let s2 = s1                   // s1 is moved into s2
   |              -- value moved here
 4 |     println! "{}, world" s1       // s1 is no longer valid
   |                          ^^ value borrowed here after move
   = help: consider cloning the value if the performance cost is acceptable (hrs 3:16)
```

The line `let s2 = s1` did not copy the string. A `String` is, on the stack, three words — a pointer to the heap buffer, a length, and a capacity — and assignment copies those three words, so that `s1` and `s2` would both point at the same buffer. If both were owners, both would free it when they went out of scope, and that is the double-free the rules exist to prevent. So Rust does the only safe thing: it declares that the assignment **moved** the value, that `s2` is now the owner, and that `s1` is no longer valid. Using `s1` afterwards is the error you see — and read it, because it tells you where the move happened, why (`String` does not implement `Copy`), and what to do if you meant to keep both.

This is the moment people come to Rust from anywhere else and feel the floor shift: assignment is not a copy. It is a transfer of responsibility. Once you have it, most of the chapter follows from it.

### Clone

When you actually want two strings, say so:

```
fn main$:
    let s1 = String.from "hello"
    let s2 = s1 <- clone$       // a second, independent copy of the heap data
    println! "s1 = {}, s2 = {}" s1 s2
```

```text
s1 = hello, s2 = hello
```

`clone` allocates a second buffer and copies the bytes into it; now there are two owners of two values, and each will be freed once. It is a method call, so it is visible in the code, which is the point — anything that copies heap memory is expensive enough that Rust wants you to write it down. When you see `<- clone$` you know something possibly costly is happening; when you do not, you know nothing is.

### Copy

Integers did not behave this way in chapter 3, and they still do not:

```
fn main$:
    let x = 5
    let y = x                     // an integer is copied, not moved
    println! "x = {}, y = {}" x y
```

```text
x = 5, y = 5
```

An `i32` is a value that lives entirely on the stack, has no buffer to free, and is as cheap to copy as to move — the two operations are the same four bytes. Types like that implement a trait called `Copy`, and for a `Copy` type assignment copies and the original stays valid. All the scalars are `Copy`, and so are tuples and arrays made only of `Copy` types. Nothing that owns heap memory is, and a type cannot be `Copy` if dropping it does work. The error above named this exactly: `String` does not implement `Copy`, so assignment moved it.

### Ownership and functions

Passing a value to a function is the same as assigning it to the parameter, and follows the same rule:

```
fn takes_ownership some_string: String:
    println! "{}" some_string
    // some_string goes out of scope here and is freed

fn makes_copy some_integer: i32:
    println! "{}" some_integer

fn main$:
    let s = String.from "hello"
    takes_ownership s             // s moves into the function…

    let x = 5
    makes_copy x                  // x is copied into the function…
    println! "{}" x               // …so x is still here
    println! "{}" s               // …but s is not
```

```text
error[E0382]: borrow of moved value: `s`
  --> fn_move.hrs:15:19
   |
 9 |     let s = String.from "hello"
   |         - move occurs because `s` has type `String`, which does not implement the `Copy` trait
10 |     takes_ownership s             // s moves into the function…
   |                     - value moved here
15 |     println! "{}" s               // …but s is not
   |                   ^ value borrowed here after move
   = note: consider changing this parameter type in function `takes_ownership` to borrow instead if owning the value isn't necessary (hrs 1:33)
   = help: consider cloning the value if the performance cost is acceptable (hrs 10:22)
```

`s` moved into `takes_ownership`, was printed, and was dropped when that function returned — so by the time `main` tries to print it, it is gone, and the compiler says so. `x` was copied into `makes_copy`, so `main` still has it. Notice the compiler's note: it has already worked out that `takes_ownership` did not need to own the string, and suggests borrowing instead. That suggestion is the next section; for now, see that the rule is uniform. There is no special case for function calls. A value goes where its owner goes.

Returning a value moves it too — out of the function and into whatever receives it:

```
fn gives_ownership$ -> String:
    let some_string = String.from "yours"
    some_string                   // moved out to the caller

fn takes_and_gives_back a_string: String -> String:
    a_string                      // moved in, moved back out

fn main$:
    let s1 = gives_ownership$
    let s2 = String.from "hello"
    let s3 = takes_and_gives_back s2
    println! "{} {}" s1 s3
```

```text
yours hello
```

`gives_ownership` creates a `String` and hands it to `s1`. `s2` moves into `takes_and_gives_back` and comes out again as `s3`; `s2` is gone, and the string it named now belongs to `s3`. The value was never copied, only passed along. Nothing was freed until `main` ends, when `s1` and `s3` go out of scope and each drops what it owns.

This is correct and it is also tedious. A function that only wants to *look* at a `String` — measure it, say — has to give it back or the caller loses it, and giving it back means returning it alongside the real answer:

```
fn calculate_length s: String -> (String, usize):
    let length = s <- len$
    (s, length)                   // hand the String back along with the answer

fn main$:
    let s1 = String.from "hello"
    let (s2, len) = calculate_length s1
    println! "the length of '{}' is {}" s2 len
```

```text
the length of 'hello' is 5
```

It works. It is also nobody's idea of a good time, and Rust has a better one.

## 4.2 References and borrowing

A **reference** lets a function use a value without owning it. Write `&s1` and you get a reference to `s1`; the function receives `&String` — "a reference to a String" — and when its parameter goes out of scope nothing is freed, because the parameter never owned anything:

```
fn calculate_length s: &String -> usize:
    s <- len$
    // s goes out of scope, but it never owned the String, so nothing is freed

fn main$:
    let s1 = String.from "hello"
    let len = calculate_length (&s1)
    println! "the length of '{}' is {}" s1 len
```

```text
the length of 'hello' is 5
```

Creating a reference is called **borrowing**, and the word is chosen with care: you have the value for a while, you give it back, and the owner was the owner throughout. `s1` is still valid after the call because it never left. The `&` appears once in the signature, `s: &String`, and once at the call, `(&s1)` — where it is isolated in parentheses because `&s1` contains an operator and an argument must be one atom. Chapter 3's index rule again: if it has an operator in it, wrap it.

### Mutable references

A borrowed value cannot be changed through the borrow:

```
fn change some_string: &String:
    some_string <- push_str ", world"

fn main$:
    let s = String.from "hello"
    change (&s)
```

```text
error[E0596]: cannot borrow `*some_string` as mutable, as it is behind a `&` reference
  --> borrow_mut_err.hrs:2:5
   |
 2 |     some_string <- push_str ", world"
   |     ^^^^^^^^^^^ `some_string` is a `&` reference, so the data it refers to cannot be borrowed as mutable
   = help: consider changing this to be a mutable reference (hrs 1:25)
```

A plain `&` reference is a promise to only read. To change something through a reference you need a **mutable reference**, `&mut`, and the value you borrow from has to be `mut` in the first place:

```
fn change some_string: &mut String:
    some_string <- push_str ", world"

fn main$:
    let mut s = String.from "hello"
    change (&mut s)
    println! "{}" s
```

```text
hello, world
```

Three `mut`s: the variable is declared mutable, the borrow is taken mutably, and the parameter's type says so. That is not redundancy; each one is a statement at a different place — the declaration, the call, the signature — and each is what its reader needs to know.

### The one rule about mutable references

Here is the restriction that makes borrowing safe, and it is the compiler's most-argued-with error:

```
fn main$:
    let mut s = String.from "hello"
    let r1 = &mut s
    let r2 = &mut s
    println! "{}, {}" r1 r2
```

```text
error[E0499]: cannot borrow `s` as mutable more than once at a time
  --> two_mut.hrs:4:14
   |
 3 |     let r1 = &mut s
   |              ------ first mutable borrow occurs here
 4 |     let r2 = &mut s
   |              ^^^^^^ second mutable borrow occurs here
 5 |     println! "{}, {}" r1 r2
   |                       -- first borrow later used here
```

**At any one time, a value may have either one mutable reference or any number of immutable ones — never both, never two mutable.** The reason is *data races*: two paths that can write the same memory, or one that writes while another reads, with nothing to order them. In a garbage-collected language that is a bug you find at runtime, sometimes. Rust makes it a compile error, always, by refusing to let two mutable references to one value exist at the same time.

The scope of a reference is what matters, so the borrows only conflict while both are alive. Give the first one a block of its own and the second is fine:

```
fn main$:
    let mut s = String.from "hello"

    do:
        let r1 = &mut s
        r1 <- push_str " there"
    // r1 is gone, so a new mutable borrow is fine

    let r2 = &mut s
    r2 <- push_str "!"
    println! "{}" s
```

```text
hello there!
```

Mixing is refused for the same reason. Readers were promised nothing would change under them; a writer breaks that promise:

```
fn main$:
    let mut s = String.from "hello"
    let r1 = &s
    let r2 = &s
    let r3 = &mut s
    println! "{}, {}, and {}" r1 r2 r3
```

```text
error[E0502]: cannot borrow `s` as mutable because it is also borrowed as immutable
  --> mixed_borrow.hrs:5:14
   |
 3 |     let r1 = &s
   |              -- immutable borrow occurs here
 5 |     let r3 = &mut s
   |              ^^^^^^ mutable borrow occurs here
 6 |     println! "{}, {}, and {}" r1 r2 r3
   |                               -- immutable borrow later used here
```

A reference's scope, though, is not its enclosing block. It runs from where the reference is created to **the last place it is used**. So this compiles, because `r1` and `r2` are not used after the first `println!`, and the compiler can see that:

```
fn main$:
    let mut s = String.from "hello"
    let r1 = &s
    let r2 = &s
    println! "{} and {}" r1 r2

    // r1 and r2 are not used after this point, so their borrow is over
    let r3 = &mut s
    r3 <- push_str "!"
    println! "{}" r3
```

```text
hello and hello
hello!
```

The compiler tracks where each borrow ends by use, not by block, and this is what makes the rule livable rather than merely correct. When you get one of these errors, look at the last use of the earlier borrow, because that is where its scope ends, and often the fix is to reorder two lines so the borrows do not overlap.

### Dangling references

In languages with pointers it is possible to hand out a pointer to memory that is then freed — a *dangling* pointer, the classic source of crashes and worse. Rust guarantees this cannot happen: a reference is never allowed to outlive the value it points to. Try to return a reference to a local and see:

```
fn dangle$ -> &String:
    let s = String.from "hello"
    &s                            // a reference to s…

fn main$:
    let reference_to_nothing = dangle$
    // …but s was freed when dangle returned
```

```text
error[E0106]: missing lifetime specifier
  --> dangle.hrs:1:15
   |
 1 | fn dangle$ -> &String:
   |               ^ expected named lifetime parameter
   = help: this function's return type contains a borrowed value, but there is no value for it to be borrowed from
   = help: consider using the `'static` lifetime (hrs 1:16)
```

`s` is created inside `dangle`, so it is dropped when `dangle` returns; a reference to it would point at freed memory. The compiler's message is about *lifetimes*, a word chapter 10 explains in full — for now, read the first `help`: the return type is a borrowed value and there is nothing in this function it could be borrowed *from*. The fix is to return the `String` itself, moving it out to the caller:

```
fn no_dangle$ -> String:
    let s = String.from "hello"
    s                             // move the String out instead

fn main$:
    let s = no_dangle$
    println! "{}" s
```

```text
hello
```

Ownership moves out; nothing dangles; the caller owns the string and will drop it.

To sum up this section in two lines: at any time, either one mutable reference or any number of immutable ones; and a reference must always point to a valid value. Everything the compiler said above is one of those two rules, applied.

## 4.3 Slices

A **slice** is a reference to a contiguous part of a collection rather than the whole thing. It exists to solve a problem that references alone leave open, so here is the problem first. Say we want the first word of a string. Without slices, the natural answer is an *index* — the position where the first word ends:

```
fn first_word s: &String -> usize:
    let bytes = s <- as_bytes$

    for (i, &item) in bytes <- iter$ <- enumerate$:
        if item == b' ':
            return i

    s <- len$

fn main$:
    let mut s = String.from "hello world"
    let word = first_word (&s)    // word = 5
    s <- clear$                 // s is now "", but word is still 5
    println! "{}" word            // 5 is the answer to a question nobody can ask now
```

```text
5
```

This compiles and it prints `5`, and it is wrong in a way the compiler cannot see: `word` is a `usize`, a plain number with no connection to `s`, so when `s` is cleared, `word` goes on holding a meaning that no longer means anything. The bug is that we have two values that must agree — the string and an offset into it — and nothing enforces the agreement.

Two things about the function itself. `as_bytes` gives the string's bytes, `iter$ <- enumerate$` walks them with their indices, and `(i, &item)` in the `for` destructures each pair — the `&` in the pattern takes the byte out of the reference the iterator yields. And the `if` returns early with `return i`; the function's last line, `s <- len$`, is the value when no space is found.

### String slices

A string slice is a reference to part of a `String`:

```
fn main$:
    let s = String.from "hello world"
    let hello = &s[0..5]
    let world = &s[6..11]
    let from_start = &s[..5]      // same as 0..5
    let to_end = &s[6..]          // same as 6..s.len()
    let whole = &s[..]
    println! "{} {} {} {} {}" hello world from_start to_end whole
```

```text
hello world hello world hello world
```

`&s[0..5]` is a reference to the five bytes starting at index 0 — the range's end is excluded, as in chapter 3. Under the hood it is a pointer into `s`'s buffer plus a length. The start may be dropped when it is 0, the end when it is the length, and both when it is the whole string. The type of every one of these is `&str`, pronounced "string slice", and it is the type of a string literal too: `"hello"` is a `&str` pointing into the program's own binary.

Now `first_word` can return the word itself, tied to the string it came from:

```
fn first_word s: &str -> &str:
    let bytes = s <- as_bytes$

    for (i, &item) in bytes <- iter$ <- enumerate$:
        if item == b' ':
            return &s[0..i]

    &s[..]

fn main$:
    let s = String.from "hello world"
    let word = first_word (&s)    // a &String coerces to &str
    println! "{}" word

    let literal = "hello world"   // a literal is already a &str
    println! "{}" (first_word literal)
    println! "{}" (first_word (&literal[6..]))
```

```text
hello
hello
world
```

The signature takes `&str`, not `&String`, and that is the more useful signature: a `&String` converts to a `&str` automatically — you see it happen at `first_word (&s)` — so the function accepts a `String`, a literal, and a slice of either. Write your functions over `&str` and everyone can call them. (The last call slices a literal: `&literal[6..]` is itself a `&str`, and it is isolated as an argument by the usual rule.)

And now the bug from the start of the section is caught, by the borrowing rule we already have:

```
fn first_word s: &str -> &str:
    let bytes = s <- as_bytes$

    for (i, &item) in bytes <- iter$ <- enumerate$:
        if item == b' ':
            return &s[0..i]

    &s[..]

fn main$:
    let mut s = String.from "hello world"
    let word = first_word (&s)
    s <- clear$                 // error: word still borrows s
    println! "{}" word
```

```text
error[E0502]: cannot borrow `s` as mutable because it is also borrowed as immutable
  --> slice_stale.hrs:13:5
   |
12 |     let word = first_word (&s)
   |                            -- immutable borrow occurs here
13 |     s <- clear$                 // error: word still borrows s
   |     ^^^^^^^^^^^ mutable borrow occurs here
14 |     println! "{}" word
   |                   ---- immutable borrow later used here
```

`word` is a slice of `s`, which is an immutable borrow of `s`; `clear` needs a mutable borrow to empty the string; the two cannot coexist while `word` is still used. The index version compiled and lied. The slice version does not compile, and the error names the line. This is the pattern of the whole chapter: an idea that was a bug in one form becomes a type error in another, and the compiler catches at build time what the earlier form would have caught in production, or not at all.

### Other slices

Slices are not only for strings. Part of an array is `&[i32]`, a slice of integers, and it works the same way:

```
fn main$:
    let a = [1, 2, 3, 4, 5]
    let middle: &[i32] = &a[1..4]
    println! "{:?}" middle
    println! "{}" (middle <- len$)
```

```text
[2, 3, 4]
3
```

`&[T]` is the slice type for any element type, and it is what you take as a parameter when a function wants "some contiguous `T`s" without caring whether they came from an array or a `Vec`.

## 4.4 What you have

One owner per value, dropped at the end of its scope. Assignment and function calls **move** unless the type is `Copy`; `clone` when you want a real copy. `&` borrows without owning, `&mut` borrows to change, and at any time there is either one `&mut` or any number of `&` — measured from creation to last use. A reference can never outlive what it refers to. And a slice is a borrow of part of something, which is why the compiler can tell when the something changes underneath it.

Nothing in this chapter was Harsh. That is the thesis: the notation was quiet, and Rust was all there was to learn. Next, a way to give your own types a shape — structs.

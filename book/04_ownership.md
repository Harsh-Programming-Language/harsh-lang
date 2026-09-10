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

@@ scope

`s` holds a string *literal*, text baked into the program, which lives on the stack and costs nothing to drop. `t` holds a `String`, which owns a buffer on the heap. `String.from` allocates it; when `t` goes out of scope at the end of `main`, Rust frees it. You wrote no `free`, and none was needed, because the compiler knows exactly where `t`'s scope ends and puts the call there itself. That is ownership from the value's point of view: an owner, and a drop at the end of the owner's scope.

`String` is the type you reach for when text is not fixed at compile time — it can grow:

@@ string_grow

`push_str` and `push` append to the buffer, reallocating if it is full. `s <- len$` is isolated in parentheses because it is an argument to `println!` and is not a single atom — the same rule chapter 3 gave for an index expression.

### Move

Now the rule bites. What happens when a `String` is assigned to a second variable?

@@ move !error

The line `let s2 = s1` did not copy the string. A `String` is, on the stack, three words — a pointer to the heap buffer, a length, and a capacity — and assignment copies those three words, so that `s1` and `s2` would both point at the same buffer. If both were owners, both would free it when they went out of scope, and that is the double-free the rules exist to prevent. So Rust does the only safe thing: it declares that the assignment **moved** the value, that `s2` is now the owner, and that `s1` is no longer valid. Using `s1` afterwards is the error you see — and read it, because it tells you where the move happened, why (`String` does not implement `Copy`), and what to do if you meant to keep both.

This is the moment people come to Rust from anywhere else and feel the floor shift: assignment is not a copy. It is a transfer of responsibility. Once you have it, most of the chapter follows from it.

### Clone

When you actually want two strings, say so:

@@ clone

`clone` allocates a second buffer and copies the bytes into it; now there are two owners of two values, and each will be freed once. It is a method call, so it is visible in the code, which is the point — anything that copies heap memory is expensive enough that Rust wants you to write it down. When you see `<- clone$` you know something possibly costly is happening; when you do not, you know nothing is.

### Copy

Integers did not behave this way in chapter 3, and they still do not:

@@ copy

An `i32` is a value that lives entirely on the stack, has no buffer to free, and is as cheap to copy as to move — the two operations are the same four bytes. Types like that implement a trait called `Copy`, and for a `Copy` type assignment copies and the original stays valid. All the scalars are `Copy`, and so are tuples and arrays made only of `Copy` types. Nothing that owns heap memory is, and a type cannot be `Copy` if dropping it does work. The error above named this exactly: `String` does not implement `Copy`, so assignment moved it.

### Ownership and functions

Passing a value to a function is the same as assigning it to the parameter, and follows the same rule:

@@ fn_move !error

`s` moved into `takes_ownership`, was printed, and was dropped when that function returned — so by the time `main` tries to print it, it is gone, and the compiler says so. `x` was copied into `makes_copy`, so `main` still has it. Notice the compiler's note: it has already worked out that `takes_ownership` did not need to own the string, and suggests borrowing instead. That suggestion is the next section; for now, see that the rule is uniform. There is no special case for function calls. A value goes where its owner goes.

Returning a value moves it too — out of the function and into whatever receives it:

@@ fn_return

`gives_ownership` creates a `String` and hands it to `s1`. `s2` moves into `takes_and_gives_back` and comes out again as `s3`; `s2` is gone, and the string it named now belongs to `s3`. The value was never copied, only passed along. Nothing was freed until `main` ends, when `s1` and `s3` go out of scope and each drops what it owns.

This is correct and it is also tedious. A function that only wants to *look* at a `String` — measure it, say — has to give it back or the caller loses it, and giving it back means returning it alongside the real answer:

@@ tuple_return

It works. It is also nobody's idea of a good time, and Rust has a better one.

## 4.2 References and borrowing

A **reference** lets a function use a value without owning it. Write `&s1` and you get a reference to `s1`; the function receives `&String` — "a reference to a String" — and when its parameter goes out of scope nothing is freed, because the parameter never owned anything:

@@ borrow

Creating a reference is called **borrowing**, and the word is chosen with care: you have the value for a while, you give it back, and the owner was the owner throughout. `s1` is still valid after the call because it never left. The `&` appears once in the signature, `s: &String`, and once at the call, `(&s1)` — where it is isolated in parentheses because `&s1` contains an operator and an argument must be one atom. Chapter 3's index rule again: if it has an operator in it, wrap it.

### Mutable references

A borrowed value cannot be changed through the borrow:

@@ borrow_mut_err !error

A plain `&` reference is a promise to only read. To change something through a reference you need a **mutable reference**, `&mut`, and the value you borrow from has to be `mut` in the first place:

@@ borrow_mut

Three `mut`s: the variable is declared mutable, the borrow is taken mutably, and the parameter's type says so. That is not redundancy; each one is a statement at a different place — the declaration, the call, the signature — and each is what its reader needs to know.

### The one rule about mutable references

Here is the restriction that makes borrowing safe, and it is the compiler's most-argued-with error:

@@ two_mut !error

**At any one time, a value may have either one mutable reference or any number of immutable ones — never both, never two mutable.** The reason is *data races*: two paths that can write the same memory, or one that writes while another reads, with nothing to order them. In a garbage-collected language that is a bug you find at runtime, sometimes. Rust makes it a compile error, always, by refusing to let two mutable references to one value exist at the same time.

The scope of a reference is what matters, so the borrows only conflict while both are alive. Give the first one a block of its own and the second is fine:

@@ two_mut_scoped

Mixing is refused for the same reason. Readers were promised nothing would change under them; a writer breaks that promise:

@@ mixed_borrow !error

A reference's scope, though, is not its enclosing block. It runs from where the reference is created to **the last place it is used**. So this compiles, because `r1` and `r2` are not used after the first `println!`, and the compiler can see that:

@@ borrow_ends

The compiler tracks where each borrow ends by use, not by block, and this is what makes the rule livable rather than merely correct. When you get one of these errors, look at the last use of the earlier borrow, because that is where its scope ends, and often the fix is to reorder two lines so the borrows do not overlap.

### Dangling references

In languages with pointers it is possible to hand out a pointer to memory that is then freed — a *dangling* pointer, the classic source of crashes and worse. Rust guarantees this cannot happen: a reference is never allowed to outlive the value it points to. Try to return a reference to a local and see:

@@ dangle !error

`s` is created inside `dangle`, so it is dropped when `dangle` returns; a reference to it would point at freed memory. The compiler's message is about *lifetimes*, a word chapter 10 explains in full — for now, read the first `help`: the return type is a borrowed value and there is nothing in this function it could be borrowed *from*. The fix is to return the `String` itself, moving it out to the caller:

@@ no_dangle

Ownership moves out; nothing dangles; the caller owns the string and will drop it.

To sum up this section in two lines: at any time, either one mutable reference or any number of immutable ones; and a reference must always point to a valid value. Everything the compiler said above is one of those two rules, applied.

## 4.3 Slices

A **slice** is a reference to a contiguous part of a collection rather than the whole thing. It exists to solve a problem that references alone leave open, so here is the problem first. Say we want the first word of a string. Without slices, the natural answer is an *index* — the position where the first word ends:

@@ first_word_index

This compiles and it prints `5`, and it is wrong in a way the compiler cannot see: `word` is a `usize`, a plain number with no connection to `s`, so when `s` is cleared, `word` goes on holding a meaning that no longer means anything. The bug is that we have two values that must agree — the string and an offset into it — and nothing enforces the agreement.

Two things about the function itself. `as_bytes` gives the string's bytes, `iter$ <- enumerate$` walks them with their indices, and `(i, &item)` in the `for` destructures each pair — the `&` in the pattern takes the byte out of the reference the iterator yields. And the `if` returns early with `return i`; the function's last line, `s <- len$`, is the value when no space is found.

### String slices

A string slice is a reference to part of a `String`:

@@ slices

`&s[0..5]` is a reference to the five bytes starting at index 0 — the range's end is excluded, as in chapter 3. Under the hood it is a pointer into `s`'s buffer plus a length. The start may be dropped when it is 0, the end when it is the length, and both when it is the whole string. The type of every one of these is `&str`, pronounced "string slice", and it is the type of a string literal too: `"hello"` is a `&str` pointing into the program's own binary.

Now `first_word` can return the word itself, tied to the string it came from:

@@ first_word

The signature takes `&str`, not `&String`, and that is the more useful signature: a `&String` converts to a `&str` automatically — you see it happen at `first_word (&s)` — so the function accepts a `String`, a literal, and a slice of either. Write your functions over `&str` and everyone can call them. (The last call slices a literal: `&literal[6..]` is itself a `&str`, and it is isolated as an argument by the usual rule.)

And now the bug from the start of the section is caught, by the borrowing rule we already have:

@@ slice_stale !error

`word` is a slice of `s`, which is an immutable borrow of `s`; `clear` needs a mutable borrow to empty the string; the two cannot coexist while `word` is still used. The index version compiled and lied. The slice version does not compile, and the error names the line. This is the pattern of the whole chapter: an idea that was a bug in one form becomes a type error in another, and the compiler catches at build time what the earlier form would have caught in production, or not at all.

### Other slices

Slices are not only for strings. Part of an array is `&[i32]`, a slice of integers, and it works the same way:

@@ array_slice

`&[T]` is the slice type for any element type, and it is what you take as a parameter when a function wants "some contiguous `T`s" without caring whether they came from an array or a `Vec`.

## 4.4 What you have

One owner per value, dropped at the end of its scope. Assignment and function calls **move** unless the type is `Copy`; `clone` when you want a real copy. `&` borrows without owning, `&mut` borrows to change, and at any time there is either one `&mut` or any number of `&` — measured from creation to last use. A reference can never outlive what it refers to. And a slice is a borrow of part of something, which is why the compiler can tell when the something changes underneath it.

Nothing in this chapter was Harsh. That is the thesis: the notation was quiet, and Rust was all there was to learn. Next, a way to give your own types a shape — structs.

# 15. Smart pointers

A reference, `&T`, points at a value it does not own. A *smart pointer* is a struct that points at a value and *does* own it, with some rule about that ownership: `Box<T>` puts a value on the heap; `Rc<T>` lets several owners share one; `RefCell<T>` moves the borrow rules from compile time to run time. `String` and `Vec` are smart pointers too, by this definition — they own heap memory and know its length. What makes the three in this chapter worth a chapter is that between them they build the data structures that seem impossible under chapter 4's rules: recursive lists, shared graphs, values changed through a shared reference.

## 15.1 `Box<T>`

A `Box` is the simplest: one value, on the heap, owned by the box:

@@ box_basic

`Box.new 5` allocates and returns the box; `b` is used like the `i32` it holds, and freed when `b` goes out of scope. On its own that is pointless — an `i32` is happier on the stack — and boxes are for three situations: a type whose size is not known at compile time, a large value you want to move without copying, and a value you own but only care about the trait it implements (chapter 18). The first is the classic:

@@ cons_err !error

A *cons list* — each element holds a value and the rest of the list — is the recursive structure of the functional languages. Written directly it is a type error: a `List` contains a `List` contains a `List`, so the compiler cannot say how many bytes one takes. The help says exactly what to do: put the recursion behind a pointer, whose size is known.

@@ cons_box

`Cons (i32, Box<List>)` is an `i32` and a pointer, a fixed size, and the rest of the list is on the heap behind it. `use List.(Cons, Nil)` brings the variants in so the construction reads as `Cons 1 (Box.new (Cons 2 …))` — each `Cons` applied to a value and a boxed tail, each argument that is a call isolated. `Box` does nothing but own and point, which is why it is the pointer to reach for first.

## 15.2 `Deref`: treating a pointer like a reference

`*b` on a `Box` gives the value inside, as `*r` on a reference does. That works because `Box` implements the `Deref` trait, and you can implement it for a type of your own:

@@ deref

`MyBox<T>` is a tuple struct with one field; `impl Deref for MyBox<T>` says what `*` does: `deref` returns a reference to the field, and `*y` is sugar for `*(y.deref())`. The `type Target = T` line is an associated type — the trait needs to know what a `MyBox<T>` derefs *to*.

The second half is *deref coercion*: `hello` takes `&str`, and is passed `&m`, a `&MyBox<String>`. The compiler applies `Deref` as many times as needed to make the types meet — `&MyBox<String>` to `&String` to `&str` — so the call compiles as written. The last line spells out what the coercion did: `&(*m)[..]`. This is the mechanism behind every `&String` that was passed where a `&str` was wanted since chapter 4; it was never a special case, only `Deref`.

## 15.3 `Drop`: code that runs on cleanup

A smart pointer's other half is what happens when it goes away. The `Drop` trait is a method the compiler calls when a value goes out of scope:

@@ drop

`impl Drop for CustomSmartPointer` with `fn drop (&mut self)` — the body runs at the end of the owner's scope, in reverse order of creation, which is why `d` is dropped after `c` would have been. `Box` uses `Drop` to free its heap memory, `File` to close the file, a lock guard to release the lock. You cannot call `x <- drop$` yourself — that would leave `x` in scope, to be dropped again — but you can call the function `drop x`, which takes the value and ends it now; the example does, and `c`'s message appears before the last `println!`. Deterministic cleanup with no `finally` and no garbage collector is what `Drop` gives, and it is most of the reason Rust needs no `defer`.

## 15.4 `Rc<T>`: shared ownership

Chapter 4 said a value has exactly one owner. Sometimes that is the wrong model — a node in a graph belongs to every edge that reaches it — and `Rc<T>`, the *reference-counted* pointer, is how Rust expresses it:

@@ rc

`a` is a list held in an `Rc`; `b` and `c` are lists that each *share* `a` as their tail, by `Rc.clone (&a)`. `Rc.clone` does not copy the list; it increments a count and returns another pointer to the same allocation, and `Rc.strong_count` shows the count going 1, 2, 3 and back to 2 when `c` is dropped. The value is freed when the count reaches zero — when the last owner is gone — and that is the whole rule. Two things to know: `Rc.clone` is the conventional spelling precisely because it is *not* a deep copy, so a reader can tell the cheap clones from the expensive ones; and `Rc` is for a single thread — chapter 16 has `Arc` for the rest.

An `Rc<T>` only hands out *shared* references to its value. Several owners and one of them writing would be the data race of chapter 4, so through an `Rc` the value is read-only. Which leaves the question of how to change something that is shared.

## 15.5 `RefCell<T>`: borrowing checked at run time

The borrow rules — one `&mut` or many `&`, never both — are enforced by the compiler, on what it can prove:

@@ refcell_err !error

`RefCell<T>` enforces the same rules *at run time*: `borrow$` gives a shared reference and `borrow_mut$` a mutable one, each counted while it lives, and a violation is a panic rather than a compile error. The value may then be mutated through something that is itself only shared — an `Rc`, say:

@@ refcell

`value` is an `Rc<RefCell<i32>>` shared by three lists, and `*value <- borrow_mut$ += 10` writes through it: `borrow_mut$` returns a guard, `*` reaches the number, and the guard's `Drop` releases the borrow at the end of the statement. All three lists see `15`. This is *interior mutability*: a value that looks immutable from outside — `a`, `b`, `c` are not `mut` — and is mutated inside, under the borrow rules, checked as it happens.

And checked they are:

@@ refcell_panic !panic

Two `borrow_mut$` guards alive at once is exactly what the compiler forbids for `&mut`, and `RefCell` panics with `already borrowed`. The rules did not change; only when they are checked did. Use `RefCell` when you know the code respects the rules and the compiler cannot see it — a value mutated through shared handles, a mock object recording calls in a test — and accept that the check has moved from build time to the first run that violates it.

## 15.6 Reference cycles

`Rc` frees when the count reaches zero; two `Rc`s that point at each other never reach zero, and the memory leaks. Rust does not prevent this — a leak is safe, just wasteful — so structures with pointers in both directions use a `Weak<T>` for one direction. A weak pointer does not count toward ownership, and to use it you `upgrade$` it into an `Option<Rc<T>>` that is `None` if the value is gone:

@@ weak

A tree: a `Node` owns its children (`Rc`) and *knows* its parent (`Weak`). `leaf` starts with no parent; inside the block `branch` is made with `leaf` as a child, and `leaf`'s parent is set with `Rc.downgrade (&branch)` — a weak pointer, so `branch`'s strong count stays 1 and its weak count becomes 1. When the block ends `branch` is dropped, the strong count hits zero, the node is freed, and `leaf`'s `upgrade$` afterwards returns `None`: the parent is gone and the child knows it. No cycle, no leak, and the counts printed at each step show exactly why.

## 15.7 What you have

`Box<T>` owns one heap value and makes a recursive type finite. `Deref` makes a pointer usable as a reference and drives the coercion from `&String` to `&str`. `Drop` runs cleanup at the end of scope, in order, and `drop x` runs it early. `Rc<T>` gives a value several owners and frees it when the last is gone; `RefCell<T>` moves the borrow rules to run time so a shared value can be changed; together they make shared mutable structures; and `Weak<T>` breaks the cycles that would otherwise leak.

Next: concurrency — threads, channels and the `Mutex`, and how the ownership rules make data races a compile error.

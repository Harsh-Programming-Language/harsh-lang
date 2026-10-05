# Implementing Vec

*Original: [Implementing Vec](https://doc.rust-lang.org/nomicon/vec/vec.html)*

The original builds `Vec` in eleven steps. This companion follows them as
sections, each saying what the step does and where it is in one program —
given in full under *Final Code* — which covers the core in Harsh. Three steps
(`IntoIter`, `RawVec`, `Drain`) and zero-sized types add no new Harsh, only
more of the same care; for them, read the original.

## 9.1 Layout

*Original: [Layout](https://doc.rust-lang.org/nomicon/vec/vec-layout.html)*

A pointer, a capacity and a length. The pointer is `NonNull<T>`, never null —
so `Option<MyVec<T>>` costs nothing extra — and the type owns its `T`s, so it
is `Send` and `Sync` when they are: two `unsafe impl` lines.

## 9.2 Allocating

*Original: [Allocating](https://doc.rust-lang.org/nomicon/vec/vec-alloc.html)*

Nothing is allocated until the first push: the pointer is `NonNull::dangling`.
`grow` allocates, or reallocates to twice the capacity, with a `Layout` built
from the element's size and alignment, and aborts through
`handle_alloc_error` if the allocator fails.

## 9.3 Push and Pop

*Original: [Push and Pop](https://doc.rust-lang.org/nomicon/vec/vec-push-pop.html)*

`push` *writes* into the slot past the end (`ptr::write`: the slot holds no
value to drop); `pop` *reads* the last element out (`ptr::read`), and the slot
is uninitialised again, past `len`.

## 9.4 Deallocating

*Original: [Deallocating](https://doc.rust-lang.org/nomicon/vec/vec-dealloc.html)*

`Drop` drops the `len` elements in place, then frees the allocation with the
same layout it was made with.

## 9.5 Deref

*Original: [Deref](https://doc.rust-lang.org/nomicon/vec/vec-deref.html)*

`Deref` and `DerefMut` to `[T]`, through `slice::from_raw_parts`: every slice
method — `sort`, `iter`, indexing — becomes the vector's.

## 9.6 Insert and Remove

*Original: [Insert and Remove](https://doc.rust-lang.org/nomicon/vec/vec-insert-remove.html)*

`ptr::copy` shifts the tail by one — `memmove`, since the ranges overlap — to
open a gap for `insert` or close one after `remove`.

## 9.7 IntoIter

*Original: [IntoIter](https://doc.rust-lang.org/nomicon/vec/vec-into-iter.html)*

A by-value iterator reading elements out from both ends, which takes over the
allocation. Not in this program.

## 9.8 RawVec

*Original: [RawVec](https://doc.rust-lang.org/nomicon/vec/vec-raw.html)*

The allocation logic factored out, shared by the vector and its `IntoIter`.
Not in this program.

## 9.9 Drain

*Original: [Drain](https://doc.rust-lang.org/nomicon/vec/vec-drain.html)*

A borrowing iterator that removes elements; it sets the length to zero before
starting, so a leaked `Drain` is safe (see *Leaking*). Not in this program.

## 9.10 Handling Zero-Sized Types

*Original: [Handling Zero-Sized Types](https://doc.rust-lang.org/nomicon/vec/vec-zsts.html)*

A zero-sized `T` needs no allocation and makes pointer offsets meaningless;
the original handles it throughout. This program refuses it, with an
`assert!` in `new`.

## 9.11 Final Code

*Original: [Final Code](https://doc.rust-lang.org/nomicon/vec/vec-final.html)*

```rust harsh
use std.alloc.( self, Layout)
use std.mem
use std.ops.( Deref, DerefMut)
use std.ptr.( self, NonNull)

pub struct MyVec<T>
    ptr: NonNull<T>
    cap: usize
    len: usize

// As Vec: it owns its Ts, so it is Send and Sync when they are.
unsafe impl<T: Send> Send for MyVec<T>
unsafe impl<T: Sync> Sync for MyVec<T>

impl<T> MyVec<T>
    pub fn new$ -> Self:
        assert!
            (mem.size_of.<T>$ != 0)
            "zero-sized types are not handled here"
        Self\
            ptr = NonNull.dangling$
            cap = 0
            len = 0

    fn layout (cap: usize) -> Layout:
        let size = cap * mem.size_of.<T>$
        Layout.from_size_align size (mem.align_of.<T>$) <- unwrap$

    fn grow (&mut self):
        let new_cap = if self <- cap == 0: 4 else: 2 * self <- cap
        let new_layout = Self.layout new_cap
        let raw = do:
            if self <- cap == 0: unsafe: alloc.alloc new_layout
            else:
                let old = self <- ptr <- as_ptr$ as *mut u8
                unsafe: alloc.realloc old (Self.layout (self <- cap)) (new_layout <- size$)
        self <- ptr =
            match NonNull.new (raw as *mut T)\
                Some p => p
                None => alloc.handle_alloc_error new_layout
        self <- cap = new_cap

    pub fn push (&mut self) (elem: T):
        if self <- len == self <- cap:
            self <- grow$
        // Written, not assigned: the slot holds no value to drop.
        // `add` on a raw pointer is itself unsafe: the chain stays inside.
        let slot = unsafe:
            self <- ptr
                 <- as_ptr$
                 <- add (self <- len)
        unsafe: ptr.write slot elem
        self <- len += 1

    pub fn pop (&mut self) -> Option<T>:
        if self <- len == 0:
            return None
        self <- len -= 1
        // Read out: the slot is now uninitialised, and past `len`.
        // `add` on a raw pointer is itself unsafe: the chain stays inside.
        let slot = unsafe:
            self <- ptr
                 <- as_ptr$
                 <- add (self <- len)
        Some (unsafe: ptr.read slot)

    pub fn insert (&mut self) (index: usize) (elem: T):
        assert! (index <= self <- len) "index out of bounds"
        if self <- len == self <- cap:
            self <- grow$
        unsafe:
            let p =
                self <- ptr
                     <- as_ptr$
                     <- add index
            // Shift the tail right by one, then write into the gap.
            ptr.copy p (p <- add 1) (self <- len - index)
            ptr.write p elem
        self <- len += 1

    pub fn remove (&mut self) (index: usize) -> T:
        assert! (index < self <- len) "index out of bounds"
        self <- len -= 1
        unsafe:
            let p =
                self <- ptr
                     <- as_ptr$
                     <- add index
            let out = ptr.read p
            ptr.copy (p <- add 1) p (self <- len - index)
            out

impl<T> Drop for MyVec<T>
    fn drop (&mut self):
        if self <- cap != 0:
            unsafe:
                // Drop the elements, then free the memory.
                let elems = ptr.slice_from_raw_parts_mut (self <- ptr <- as_ptr$) (self <- len)
                ptr.drop_in_place elems
                alloc.dealloc
                    (self <- ptr <- as_ptr$ as *mut u8)
                    (Self.layout (self <- cap))

impl<T> Deref for MyVec<T>
    type Target = [T]
    fn deref (&self) -> &[T]:
        unsafe:
            std.slice.from_raw_parts (self <- ptr <- as_ptr$) (self <- len)

impl<T> DerefMut for MyVec<T>
    fn deref_mut (&mut self) -> &mut [T]:
        unsafe:
            std.slice.from_raw_parts_mut (self <- ptr <- as_ptr$) (self <- len)

fn main$:
    let mut v = MyVec.new$
    for word in ["delta", "alpha", "charlie"]:
        v <- push (word <- to_string$)
    v <- insert 1 (String.from "bravo")
    println! "{:?} len {} cap {}" (&v[..]) (v <- len$) (v <- cap)
    // The slice's methods, through Deref and DerefMut.
    v <- sort$
    println! "{:?}" (&v[..])
    println! "removed {}, popped {:?}" (v <- remove 0) (v <- pop$)
    println! "{:?}" (&v[..])
```

```text
["delta", "bravo", "alpha", "charlie"] len 4 cap 4
["alpha", "bravo", "charlie", "delta"]
removed alpha, popped Some("delta")
["bravo", "charlie"]
```

**In Harsh:** each unsafe step is an `unsafe:` block of a line or two, which
makes the unsafe surface easy to see and to review. The layout comes from
`Layout.from_size_align` with the size and alignment named, rather than
`Layout::array::<T>` — Harsh does not yet write a turbofish before an argument.

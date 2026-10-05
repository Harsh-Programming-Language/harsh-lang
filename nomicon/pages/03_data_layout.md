# Data Layout

How values sit in memory: their size, their alignment, the order of their
fields. Unsafe code that reads or writes memory directly depends on it. The
original chapter: [Data Representation](https://doc.rust-lang.org/nomicon/data.html).

## 2.1 repr(Rust)

*Original: [repr(Rust)](https://doc.rust-lang.org/nomicon/repr-rust.html)*

Every type has a size and an alignment. By default, Rust may reorder a
struct's fields to reduce padding, and promises nothing else about their
order: two structs with the same fields may be laid out differently.

```rust harsh
use std.mem

// Written loosely: a byte, a u32, a byte.
struct Loose
    a: u8
    b: u32
    c: u8

// The same fields, in C's order.
#[repr C]
struct InOrder
    a: u8
    b: u32
    c: u8

fn main$:
    // Rust may reorder the fields to pack them; C's order pads.
    println!
        "Loose:   size {} align {}"
        (mem.size_of.<Loose>$)
        (mem.align_of.<Loose>$)
    println!
        "InOrder: size {} align {}"
        (mem.size_of.<InOrder>$)
        (mem.align_of.<InOrder>$)
```

```text
Loose:   size 8 align 4
InOrder: size 12 align 4
```

**In Harsh:** `mem.size_of<Loose>$` — the generic list follows the name, and
the transpiler writes Rust's turbofish.

## 2.2 Exotically Sized Types

*Original: [Exotically Sized Types](https://doc.rust-lang.org/nomicon/exotic-sizes.html)*

Not every type has a known, non-zero size. *Dynamically sized* types — `[T]`,
`str`, `dyn Trait` — live behind pointers that carry their length or vtable,
so those pointers are twice as wide. *Zero-sized* types take no space at all;
*empty* types, like an enum with no variants, cannot even be built.

```rust harsh
use std.mem

struct Nothing
enum Void

trait Shape
    fn area (&self) -> f64

fn main$:
    // Zero-sized: no space at all.
    println! "()      {}" (mem.size_of.<()>$)
    println! "Nothing {}" (mem.size_of.<Nothing>$)
    println! "Void    {}" (mem.size_of.<Void>$)
    // A pointer to a dynamically sized type carries its length or vtable.
    println! "&u8        {}" (mem.size_of.<&u8>$)
    println! "&[u8]      {}" (mem.size_of.<&[u8]>$)
    println! "&str       {}" (mem.size_of.<&str>$)
    println! "&dyn Shape {}" (mem.size_of.<&dyn Shape>$)
```

```text
()      0
Nothing 0
Void    0
&u8        8
&[u8]      16
&str       16
&dyn Shape 16
```

**In Harsh:** an empty enum is its header alone, `enum Void`, like any
declaration with nothing beneath.

## 2.3 Other reprs

*Original: [Other reprs](https://doc.rust-lang.org/nomicon/other-reprs.html)*

`#[repr(C)]` lays out fields in order, as C does — required for FFI.
`#[repr(u8)]` (and the other integers) fixes an enum's discriminant.
`#[repr(transparent)]` makes a one-field wrapper laid out exactly as its
field. `#[repr(packed)]` (or `packed(n)`) removes padding, at the cost of
unaligned fields; `#[repr(align(n))]` raises a type's alignment.

```rust harsh
use std.mem

// A fixed discriminant, one byte.
#[repr u8]
#[derive Clone Copy Debug]
enum Color
    Red = 1
    Green = 2

// A wrapper laid out exactly as its field.
#[repr transparent]
struct Meters f64

// No padding: the u32 may be unaligned.
#[repr C packed]
struct Packed
    a: u8
    b: u32

fn main$:
    println! "{} {}" (mem.size_of.<Color>$) (Color.Green as u8)
    println! "{} {}" (mem.size_of.<Meters>$) (mem.size_of.<f64>$)
    println! "{}" (mem.size_of.<Packed>$)
```

```text
1 2
8 8
5
```

**In Harsh:** attribute arguments juxtapose, `#[repr C]`, `#[repr u8]`,
`#[repr transparent]`.

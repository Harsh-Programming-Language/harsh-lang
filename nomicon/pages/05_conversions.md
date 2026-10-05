# Type Conversions

The ways a value changes type: implicitly, with `as`, or by reinterpreting
its bits. The original chapter: [Type Conversions](https://doc.rust-lang.org/nomicon/conversions.html).

## 4.1 Coercions

*Original: [Coercions](https://doc.rust-lang.org/nomicon/coercions.html)*

Some conversions happen on their own, at specific places: `&mut T` to `&T`,
`&String` to `&str` through `Deref`, an array to a slice, a concrete type to
`dyn Trait`. They are never applied to satisfy a trait bound.

```rust harsh
use std.fmt.Display

fn count (s: &str) -> usize:
    s <- len$

fn total (xs: &[i32]) -> i32:
    xs <- iter$ <- sum$

fn main$:
    let owned = String.from "harsh"
    // &String to &str, through Deref.
    println! "{}" (count (&owned))
    // An array to a slice.
    println! "{}" (total (&[1, 2, 3]))
    // A concrete type to a trait object.
    let shown: &dyn Display = &42
    println! "{shown}"
```

```text
5
6
42
```

**In Harsh:** the same places, the same conversions.

## 4.2 The Dot Operator

*Original: [The Dot Operator](https://doc.rust-lang.org/nomicon/dot-operator.html)*

A method call finds its method by trying the receiver, then `&receiver`,
then `&mut receiver`, then dereferencing and trying again. This is
convenient and occasionally surprising: `x.clone()` on a `&T` where `T` is not
`Clone` clones the *reference*.

```rust harsh
#[derive Clone Debug]
struct Named
    name: String

struct NotClone

fn main$:
    let n = Named\ name = String.from "a"
    let r = &n
    // `Named` is Clone: r <- clone$ is a Named.
    let copy: Named = r <- clone$
    println! "{:?}" copy
    // `NotClone` is not: r2 <- clone$ clones the reference itself.
    let x = NotClone
    let r2 = &x
    let also_ref: &NotClone = r2 <- clone$
    println! "{}" (std.ptr.eq r2 also_ref)
```

```text
Named { name: "a" }
true
```

**In Harsh:** `<-` is the dot and follows the same search. Writing the trait's
function, `T.clone x`, says which one you mean — found while writing *Harsh
Design Patterns*, where it mattered.

## 4.3 Casts

*Original: [Casts](https://doc.rust-lang.org/nomicon/casts.html)*

`as` converts between primitive types explicitly. Numeric casts truncate or
sign-extend; a float to an integer saturates (and NaN becomes 0); pointer
casts change the type the pointer claims. None of them is undefined
behaviour — but a pointer cast is only as good as what it points to.

```rust harsh
fn main$:
    // Integer to a smaller integer: truncates. (A literal that does not fit
    // is refused outright, by the `overflowing_literals` lint.)
    let big: i32 = 300
    println! "{}" (big as u8)
    // Signed to unsigned: the same bits.
    println! "{}" (-1i32 as u32)
    // Float to integer: saturates; NaN becomes 0.
    println! "{} {} {}" (3.9f64 as i32) (1e20f64 as i32) (f64.NAN as i32)
    // A reference to a raw pointer, and to an integer address.
    let x = 7
    let p = &x as *const i32
    println! "{}" ((p as usize) % (std.mem.align_of.<i32>$))
```

```text
44
4294967295
3 2147483647 0
0
```

**In Harsh:** `as` binds as in Rust, `(big as u8)`.

## 4.4 Transmutes

*Original: [Transmutes](https://doc.rust-lang.org/nomicon/transmutes.html)*

`mem::transmute` reinterprets the bits of one type as another of the same
size. It is the most dangerous tool there is: almost every use has a safer
alternative (`f32::to_bits`, `from_ne_bytes`, pointer casts), which the next
program compares it with.

```rust harsh
fn main$:
    let x = 1.5f32
    // The dangerous tool...
    let bits: u32 = unsafe: std.mem.transmute x
    // ...and the safe one that does the same.
    println! "{:#x} {:#x}" bits (x <- to_bits$)
    let bytes = 0x12345678u32 <- to_be_bytes$
    println! "{:x?}" bytes
    println! "{:#x}" (u32.from_be_bytes bytes)
```

```text
0x3fc00000 0x3fc00000
[12, 34, 56, 78]
0x12345678
```

**In Harsh:** `mem.transmute<f32, u32> x` would need the turbofish before an
argument, which the transpiler does not yet write — one more reason to reach
for `to_bits`. The program names the types in a binding instead.

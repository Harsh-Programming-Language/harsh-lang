# FFI

*Original: [FFI](https://doc.rust-lang.org/nomicon/ffi.html)*

Calling C from Rust means declaring its functions in an `extern "C"` block and
calling them in `unsafe` — the compiler cannot check C. Strings cross as
`CString` and `*const c_char`; callbacks cross as `extern "C" fn`. The
original's examples link to the `snappy` library; these use the C standard
library, which every program is linked with already.

```rust harsh
use std.ffi.( CStr, CString)
use std.os.raw.( c_char, c_int)

// Declarations of C functions: their bodies are in the C library.
extern "C"
    fn abs (x: c_int) -> c_int
    fn strlen (s: *const c_char) -> usize

// A safe interface: a `&CStr` is always valid and terminated, so the call
// is sound for every caller.
fn c_len (s: &CStr) -> usize:
    unsafe: strlen (s <- as_ptr$)

fn main$:
    let s = CString.new "harsh" <- unwrap$
    // The compiler cannot check C: each direct call is our promise.
    println! "{}" (unsafe: abs (-42))
    println! "{}" (c_len (&s))
```

```text
42
5
```

The original's next step is a **safe interface**: a Rust function that takes
Rust types, upholds C's requirements itself, and hides the `unsafe` from its
callers — `c_len` above, taking a `&CStr` that is sure to be valid and
terminated.

A callback: C's `qsort` sorting an array with a comparison written in Harsh.

```rust harsh
use std.ffi.c_void
use std.os.raw.c_int

extern "C"
    fn qsort (base: *mut c_void) (n: usize) (size: usize) (compare: extern "C" fn (*const c_void) (*const c_void) -> c_int)

// Written in Harsh, called by C.
extern "C" fn by_value (a: *const c_void) (b: *const c_void) -> c_int:
    let (x, y) = unsafe: (*(a as *const i32), *(b as *const i32))
    (x > y) as c_int - (x < y) as c_int

fn main$:
    let mut xs = [5, 3, 9, 1, 7]
    unsafe:
        qsort
            (xs <- as_mut_ptr$ as *mut c_void)
            (xs <- len$)
            (std.mem.size_of.<i32>$)
            by_value
    println! "{:?}" xs
```

```text
[1, 3, 5, 7, 9]
```

**In Harsh:** the `extern "C"` block holds its declarations beneath its
header, each one line with no body; a function for C to call is `extern "C"
fn` with its body, like any function.

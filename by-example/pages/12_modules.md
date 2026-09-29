# 12. Modules

```rust harsh
// A module groups items. Its body follows the header, with no mark.
mod shapes
    // Items are private to the module unless they say `pub`.
    pub struct Circle
        pub radius: f64

    impl Circle
        pub fn area (&self) -> f64:
            3.14159 * (self <- radius) * (self <- radius)

    pub mod units
        pub const NAME: &str = "cm"

// `use` brings a path into scope; `.` is the path separator.
use shapes.Circle
use shapes.units.NAME

fn main$:
    let c = Circle\ radius = 2.0
    println! "{:.2} square {}" (c <- area$) NAME

    // Or written out in full, without a `use`.
    println! "{}" shapes.units.NAME
```

```text
12.57 square cm
cm
```

A `mod` groups items, and its body follows the header with no mark, like every
other item body. Everything inside is private unless it says `pub`.

`.` is the path separator, so Rust's `shapes::units::NAME` is
`shapes.units.NAME`, and `use shapes.Circle` brings it into scope. A grouped
import is written with parentheses: `use std.io.(Read, Write)`.

Privacy is checked, so reaching a field that is not `pub` is refused:

```rust harsh
mod counter
    pub struct Counter
        // Not `pub`: only this module may touch it.
        value: i32

    impl Counter
        pub fn new$ -> Counter:
            Counter\ value = 0

fn main$:
    let c = counter.Counter.new$
    println! "{}" (c <- value)
```

```text
error[E0616]: field `value` of struct `Counter` is private
  --> privacy.hrs:18:22
   |
18 |     println!("{}", c.value)
   |                      ^^^^^ private field

error: aborting due to previous error

For more information about this error, try `rustc --explain E0616`.
```

In a real project the modules live in files rather than in one: `src/main.hrs`
plus `src/shapes.hrs`, and `mod shapes` in the main file. `hrs build`
transpiles the tree and cargo compiles it.

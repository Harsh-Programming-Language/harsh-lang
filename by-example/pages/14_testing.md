# 14. Tests

```rust harsh
fn add (a: i32) (b: i32) -> i32:
    a + b

fn divide (a: i32) (b: i32) -> Result<i32, String>:
    if b == 0:
        return Err (String.from "divide by zero")
    Ok (a / b)

// A test module is compiled only when testing.
#[cfg (test)]
mod tests
    use super.*

    #[test]
    fn addition_works$:
        assert_eq! (add 2 2) 4

    #[test]
    fn division_reports_its_error$:
        assert! (divide 1 0 <- is_err$)
        assert_eq! (divide 10 2) (Ok 5)

    #[test]
    fn a_message_of_your_own$:
        let n = add 2 2
        assert! (n > 0) "expected a positive number, got {}" n
```

```text
$ hrs test
running 3 tests
test tests::a_message_of_your_own ... ok
test tests::addition_works ... ok
test tests::division_reports_its_error ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished
```

A test is a function marked `#[test]`, usually inside a
`#[cfg (test)] mod tests`, which is compiled only when testing. `use super.*`
brings in what the file above defines.

Three things read differently from Rust, and all three are the ordinary rules:

- `#[cfg (test)]` — an attribute is an application, so its argument is
  isolated in parentheses when it is not a single atom.
- `fn addition_works$` — a test takes no parameters, so it takes `$`.
- `assert_eq! (add 2 2) 4` — a macro's arguments juxtapose, and `add 2 2` is
  an application, so it is isolated.

`hrs test` runs them, passing the arguments through to cargo.

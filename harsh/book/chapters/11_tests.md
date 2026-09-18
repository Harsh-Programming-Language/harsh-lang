# 11. Writing automated tests

The compiler checks a great deal — types, ownership, exhaustive matches — and none of it says whether `add 2 2` is `4`. That is what tests are for: functions that call your code and check what comes back, run by `cargo test` (or `hrs test`, which transpiles and then does the same), and reported. This chapter is how to write them, what the assertion macros do, and how to run some of them. Every example here is a test file, and the output shown is the test runner's report.

## 11.1 Anatomy of a test

A test is a function with `#[test]` above it. It passes if it returns, and fails if it panics:

```
pub fn add (left: u64) (right: u64) -> u64:
    left + right

#[cfg test]
mod tests
    use super.*

    #[test]
    fn it_works$:
        let result = add 2 2
        assert_eq! result 4
```

```text
$ cargo test
running 1 test
test tests::it_works ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Everything after `#[cfg test]` is the conventional shape. `mod tests:` is an ordinary module; the attribute above it says *compile this only when testing*, so the tests cost nothing in the real build. `use super.*` brings the enclosing module's items — `add` — into the test module, which is a child and would otherwise see nothing. Then `#[test]` on a function makes it a test, and `assert_eq! result 4` checks that its two arguments are equal, panicking with both values if they are not. The report names each test by its module path and tallies at the end.

Here is what failure looks like:

```
#[cfg test]
mod tests
    #[test]
    fn exploration$:
        assert_eq! (2 + 2) 4

    #[test]
    fn another$:
        panic! "Make this test fail"
```

```text
$ cargo test
running 2 tests
test tests::another ... FAILED
test tests::exploration ... ok
failures:
---- tests::another stdout ----
thread 'tests::another' panicked at failing.hrs:10:
Make this test fail
failures:
    tests::another
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

`another` panicked, so it failed; the runner prints each failing test's output and message under `failures:`, then the tally. One failure fails the run — `cargo test` exits nonzero — which is what a build pipeline wants.

## 11.2 The assertion macros

`assert!` takes something that must be true:

```
#[derive Debug]
struct Rectangle
    width: u32
    height: u32

impl Rectangle
    fn can_hold (&self) (other: &Rectangle) -> bool:
        self <- width > other <- width && self <- height > other <- height

#[cfg test]
mod tests
    use super.*

    #[test]
    fn larger_can_hold_smaller$:
        let larger = Rectangle\ width = 8, height = 7
        let smaller = Rectangle\ width = 5, height = 1
        assert! (larger <- can_hold (&smaller))

    #[test]
    fn smaller_cannot_hold_larger$:
        let larger = Rectangle\ width = 8, height = 7
        let smaller = Rectangle\ width = 5, height = 1
        assert! (!smaller <- can_hold (&larger))
```

```text
$ cargo test
running 2 tests
test tests::larger_can_hold_smaller ... ok
test tests::smaller_cannot_hold_larger ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Both tests call a method and assert on the result: the first that it holds, the second that it does not — `!smaller <- can_hold (&larger)` negates the call, and the whole expression is isolated as the macro's one argument. `assert!` on a `false` panics with the text of the expression.

`assert_eq!` and `assert_ne!` compare two values and, on failure, print both — which is why they beat `assert! (a == b)`, whose failure would only say "false":

```
pub fn add_two a: u64 -> u64:
    a + 3        // a bug

#[cfg test]
mod tests
    use super.*

    #[test]
    fn it_adds_two$:
        let result = add_two 2
        assert_eq! result 4
```

```text
$ cargo test
running 1 test
test tests::it_adds_two ... FAILED
failures:
---- tests::it_adds_two stdout ----
thread 'tests::it_adds_two' panicked at assert_eq_fail.hrs:12:
assertion `left == right` failed
  left: 5
 right: 4
failures:
    tests::it_adds_two
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

`left: 5, right: 4` — the function under test has a bug, and the report shows what it produced and what was expected without a `println!` in sight. The two arguments must implement `PartialEq` (for `==`) and `Debug` (for the printing); for your own types that is `#[derive PartialEq Debug]`.

### A message of your own

Every assertion macro takes extra arguments after the required ones, and they are a format string and its values, printed on failure:

```
pub fn greeting name: &str -> String:
    String.from "Hello!"      // forgot the name

#[cfg test]
mod tests
    use super.*

    #[test]
    fn greeting_contains_name$:
        let result = greeting "Carol"
        assert!
            (result <- contains "Carol")
            "Greeting did not contain name, value was `{result}`"
```

```text
$ cargo test
running 1 test
test tests::greeting_contains_name ... FAILED
failures:
---- tests::greeting_contains_name stdout ----
thread 'tests::greeting_contains_name' panicked at custom_message.hrs:12:
Greeting did not contain name, value was `Hello!`
failures:
    tests::greeting_contains_name
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

When a test fails a week after it was written, "assertion failed" is not enough; "did not contain name, value was `Hello!`" says what to look at. The message is worth writing whenever the failing values alone would not explain the failure.

### Testing that something panics

Sometimes the correct behaviour *is* a panic — chapter 9's `Guess.new` on a bad value — and a test asserts that it happens:

```
pub struct Guess
    value: i32

impl Guess
    pub fn new value: i32 -> Guess:
        if value < 1:
            panic! "Guess value must be greater than or equal to 1, got {value}."

        else if value > 100:
            panic! "Guess value must be less than or equal to 100, got {value}."

        Guess\ value

#[cfg test]
mod tests
    use super.*

    #[test]
    #[should_panic (expected = "less than or equal to 100")]
    fn greater_than_100$:
        Guess.new 200;
```

```text
$ cargo test
running 1 test
test tests::greater_than_100 - should panic ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`#[should_panic]` inverts the meaning of a panic: the test passes if the body panics and fails if it returns. On its own it is imprecise — any panic passes, including one from a bug in the wrong place — so `expected = "…"` narrows it to a panic whose message contains that text. The report marks it `- should panic`.

### `Result` in tests

A test may return a `Result` instead of panicking:

```
#[cfg test]
mod tests
    #[test]
    fn it_works$ -> Result<(), String>:
        let result = 2 + 2

        if result == 4:
            Ok ()

        else:
            Err (String.from "two plus two does not equal four")
```

```text
$ cargo test
running 1 test
test tests::it_works ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`Ok ()` passes and `Err` fails with the error printed. The point is `?`: a test that calls fallible code can use `?` on every step and fail cleanly on the first `Err`, instead of unwrapping each one. (Such a test cannot be `#[should_panic]`; to assert that something returns an `Err`, use `assert! (value <- is_err$)`.)

## 11.3 Running tests

`cargo test` builds the test binary and runs every test, by default in parallel on several threads — so tests must not depend on each other or on shared state such as a file they all write; `cargo test -- --test-threads=1` runs them one at a time when they must. Output printed by a passing test is captured and hidden; `-- --show-output` shows it.

A name after `cargo test` runs only the tests whose path contains it:

```
pub fn add_two a: u64 -> u64:
    a + 2

#[cfg test]
mod tests
    use super.*

    #[test]
    fn add_two_and_two$:
        assert_eq! 4 (add_two 2)

    #[test]
    fn add_three_and_two$:
        assert_eq! 5 (add_two 3)

    #[test]
    fn one_hundred$:
        assert_eq! 102 (add_two 100)

    #[test]
    #[ignore]
    fn expensive_test$:
        // takes an hour; run with `cargo test -- --ignored`
        assert_eq! 2 (add_two 0)
```

```text
$ cargo test add
running 2 tests
test tests::add_three_and_two ... ok
test tests::add_two_and_two ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out
```

Three tests match `add`, `one_hundred` does not and is *filtered out*, and `expensive_test` was never a candidate: `#[ignore]` excludes a test from the default run, and `cargo test -- --ignored` runs only those. That is the tool for the slow tests you want to keep but not wait for on every save.

## 11.4 Where tests live

Rust distinguishes *unit* tests, which live in the file with the code and test it in detail, from *integration* tests, which live outside the crate and use its public API the way a user would.

Unit tests are what this chapter has shown: a `#[cfg test] mod tests:` at the bottom of each file. Because it is a child module of the code it tests, it can see private items:

```
pub fn add_two a: u64 -> u64:
    internal_adder a 2

fn internal_adder (left: u64) (right: u64) -> u64:
    left + right

#[cfg test]
mod tests
    use super.*

    #[test]
    fn internal$:
        // a private function is still visible to its own module's tests
        assert_eq! 4 (internal_adder 2 2)
```

```text
$ cargo test
running 1 test
test tests::internal ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`internal_adder` is not `pub` and the test calls it anyway — child modules see their ancestors, and privacy is about the boundary of a crate, not about hiding code from its own tests. Whether private functions *should* be tested is a matter of taste; Rust does not make the choice for you.

Integration tests are files in a `tests/` directory beside `src/`, each its own crate that `use`s your library like an external one; `cargo test` builds and runs every file there, and only the public API is reachable, which is the point. A binary crate — one with `src/main.hrs` and no library — cannot be tested this way, since there is nothing to `use`; the usual arrangement is a thin `main.hrs` calling into a `lib.hrs` that holds the logic, and the integration tests target the library. Harsh changes none of this: `hrs test` transpiles `src/` and `tests/` alike and hands `cargo test` the result.

## 11.5 What you have

`#[test]` makes a function a test; it passes by returning and fails by panicking, which `assert!`, `assert_eq!` and `assert_ne!` do with useful messages and your own on top. `#[should_panic (expected = "…")]` for the cases that must panic; a `Result`-returning test for `?`. `#[cfg test] mod tests:` with `use super.*` at the bottom of a file, seeing everything in it; `tests/` for the public API. `cargo test name` filters, `#[ignore]` defers, `-- --test-threads=1` serialises.

Next: an I/O project — a small `grep` — that uses everything so far in one program, and is the first thing in this book you might actually run.

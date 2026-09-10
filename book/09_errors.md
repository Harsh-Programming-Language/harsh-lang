# 9. Error handling

Rust has two kinds of error and refuses to blur them. An *unrecoverable* error is a bug — an index past the end, an invariant broken — and the program *panics*: it prints a message and stops. A *recoverable* error is an event the program should expect — a file that is not there, text that is not a number — and it is a *value*, of type `Result<T, E>`, which the caller has to look at. There are no exceptions, and no error can be silently ignored, because an ignored `Result` is an unused value the compiler warns about and an unhandled one does not type-check. This chapter is the two kinds, the `?` operator that makes the recoverable kind pleasant, and the judgement of which to use.

## 9.1 Unrecoverable errors with `panic!`

You can panic on purpose:

@@ panic !panic

The program printed its first line, reached `panic!`, printed the message with the file and line, and exited. That is the whole mechanism. By default the panic *unwinds*: it walks back up the stack dropping every value on the way, so files close and memory is freed, and then the process ends. (A backtrace — the chain of calls that led here — is available with `RUST_BACKTRACE=1` in the environment; the book leaves it off.)

Most panics are not written; they are hit:

@@ panic_index !panic

Chapter 8's index rule, seen from the other side. In C this reads whatever happens to be at that address — a *buffer over-read*, the root of a large fraction of security holes. Rust checks, and panics, and the panic is not the bug: the index was. The panic is the report.

## 9.2 Recoverable errors with `Result`

Opening a file can fail for reasons that are nobody's bug. So `File.open` returns a `Result`:

```
enum Result<T, E>
    Ok T
    Err E
```

`Ok` carries the value when the operation worked, `Err` the error when it did not, and the caller takes it apart with `match`, exactly as it would an `Option`:

@@ result_match !panic

There is no `hello.txt`, so the `Err` arm ran and the program panicked with the error inside it. This is a recoverable error handled by choosing not to recover — which is fine for a program that cannot do anything without the file — and the point is that the choice was made in the code, visibly, rather than by the runtime.

### Matching on the kind of error

An `io.Error` says what went wrong, and a program can act on that:

@@ result_kinds

`error <- kind$` is an `ErrorKind`, an enum: on `NotFound`, create the file; on anything else, panic. Three `match`es nested, one inside the arm of the last — each `=> match … :` opens its arms on the lines beneath, and the layout keeps them apart where braces would have been a thicket. (The last line deletes the file the example created, so the book stays tidy.)

### `unwrap` and `expect`

`match` on every `Result` is verbose, and there are two shortcuts for "give me the value or panic":

@@ unwrap_expect !panic

`unwrap$` is the `match` above with a generic message. `expect` is the same with *your* message:

@@ expect !panic

Prefer `expect`: when it fires, the message says what you were assuming, and that is what you need to know first. Both are right when the `Err` case really cannot happen — a literal that you know parses — or in a quick program where stopping is the correct response. In library code they are a smell, since the library does not know what the caller wants done.

### Propagating errors

The usual answer is that the function that hit the error is not the one that knows what to do about it; it should hand the error up. That is *propagating*, and written by hand it looks like this:

@@ propagate

The function returns `Result<String, io.Error>`; each step that can fail is matched, and on `Err` the function returns it. The caller then decides. This is correct and it is half `match`; Rust has an operator for it.

### The `?` operator

`?` after a `Result` means: if it is `Ok`, give me the value; if it is `Err`, return it from this function right now.

@@ question

The same function in three lines. `File.open "hello.txt"?` is the file or an early return; `<- read_to_string (&mut username)?` likewise. The `?` binds tighter than `<-`, as it does in Rust, so it applies to the `open` before the chain continues — and the chain runs across the `?`, which reads naturally once you see it as "open, if that worked read into `username`, if that worked carry on". `?` also converts the error on the way out, through the `From` trait: a function returning `Result<_, MyError>` can use `?` on any error that `MyError` knows how to be made from, and chapter 10 shows `From`.

`?` works on `Option` too, returning `None` early:

@@ question_option

`text <- lines$ <- next$?` is the first line, or `None` if there are no lines; then `<- chars$ <- last$` is the last character of it, or `None` if the line is empty. Two ways to have nothing, one expression. What `?` cannot do is mix: a `?` on an `Option` inside a function returning `Result` is an error, and the other way round. Convert first.

`?` can only be used in a function whose return type it fits — which means not in a `main` that returns nothing:

@@ question_in_main !error

The compiler says exactly what is missing: `main` returns `()`, and `?` needs somewhere to send the error. `main` may return a `Result`:

@@ main_result

`fn main$ -> Result<(), Box<dyn Error>>:` — a `main` that either succeeds with nothing or fails with some error. `Box<dyn Error>` is "any error at all", a trait object (chapter 18) that `?` can convert any error into, so this `main` can use `?` on a file operation and a parse in the same body. `Ok ()` at the end is the success value — `()` is the unit value, and it is passed to `Ok` as an ordinary argument. If `main` returns an `Err`, the program prints it and exits with a nonzero code, which is what a shell expects of a failed program.

## 9.3 To panic or not

The rule of thumb: panic when the program has reached a state it was not written to be in — a bug — and return a `Result` when the failure is something the caller can be expected to handle. A few sharper edges of that rule:

- In examples, prototypes and tests, `unwrap` and `expect` are fine; a `Result` that you handle carefully in a ten-line demo obscures the demo. In tests, a panic is *how* a test fails.
- When you have information the compiler does not — you know the literal parses, you know the index is in range because you just checked the length — `expect` with a message saying why is the honest spelling.
- When the failure is expected in normal operation — a missing file, bad input, a network that is down — it is a `Result`, and the type signature is the documentation that it can happen.
- When a function's *contract* is broken — it was called with an argument it documented as invalid — panic. The caller has a bug, and continuing would spread it.

That last point is what types are for, and a type can carry a check so that no caller has to repeat it:

@@ guess_validate !panic

`Guess.new` checks the range once; `value` is private, so a `Guess` cannot be made any other way; every function that takes a `Guess` can therefore trust it without checking. Making an invalid state impossible to construct is the strongest form of error handling, because the error has nowhere to occur.

## 9.4 What you have

`panic!` for bugs; a panic unwinds, prints, and stops. `Result<T, E>` for failures the caller should handle: `match` it, `unwrap$` or `expect "why"` to panic on `Err`, or `?` to return it up — on `Result` or `Option`, in any function whose return type it fits, including a `main` that returns `Result<(), Box<dyn Error>>`. Choose by asking whether the failure is the program's fault or the world's, and let a type's constructor make the invalid states unreachable.

Next: generics, traits and lifetimes — the three mechanisms by which one piece of code serves many types, and the chapter where the lifetime error from chapter 4 finally gets its answer.

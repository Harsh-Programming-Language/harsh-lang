# 17. Async

Chapter 16's threads are the right tool when the work is computation. When the work is *waiting* — for a network reply, a file, a timer — a thread spends its time blocked, and a program with ten thousand connections cannot have ten thousand threads. `async` is Rust's answer: functions that can pause at the points where they would wait, and a *runtime* that runs many of them on a few threads, switching between them at those points. This chapter is `async`/`await`, tasks, joining and racing futures, and the one rule about blocking. The examples use the `tokio` runtime, because the language provides the syntax and the `Future` trait but deliberately no runtime; each example is a small project with a dependency, run with `hrs run`.

## 17.1 `async` and `await`

@@ hello

`async fn say_hello` returns not a `String` but a *future* — a value that will produce a `String` when driven to completion. `<- await` on a future drives it: if it is ready, the value; if it must wait, the current function *pauses*, the runtime runs something else, and this function resumes later, on the same line. `sleep (…) <- await` is the waiting point here, and while it waits no thread is blocked. `#[tokio.main]` turns `main` into the runtime's entry: it builds the runtime and runs `main`'s future to completion. The attribute's path is written with Harsh's dot, `tokio.main`, like any path.

Two things to hold: an `async fn` does nothing until awaited — calling it builds the future and returns immediately — and `await` can only appear inside an `async` function or block, because only those can pause.

## 17.2 Tasks

A *task* is the async counterpart of a thread: a future handed to the runtime to run independently:

@@ tasks

`tokio.spawn async:` — `spawn` takes a future, and `async:` opens a block that is one, with its body beneath, laid out as a closure's would be. The two loops run interleaved on one thread: each `sleep <- await` is a point where the runtime switches to the other. `handle <- await` waits for the task, as `join$` did for a thread. The order of the lines is fixed here because a `current_thread` runtime switches only at awaits, deterministically; on a multi-threaded runtime it would vary, as chapter 16's did.

### Joining futures

Two futures can also be run to completion *together*, without spawning:

@@ join

`async:` blocks assigned to `let` are futures that have not started. `tokio.join! fut1 fut2` polls both, alternating whenever one waits, and finishes when both have — returning a tuple of their results, discarded here with a `;`. The difference from `spawn`: a joined future runs inside the current task and can borrow from it, where a spawned task is independent and must own what it uses (`move`, chapter 16 again).

## 17.3 Message passing

Chapter 16's channel has an async twin, whose `recv` is a future:

@@ channel

Three futures joined: two producers sending with delays and a receiver looping `while let Some value = rx <- recv$ <- await:`. The producers are `async move:` blocks — `move` so that each owns its transmitter — and the receiver ends when both transmitters have been dropped, which happens when the producer blocks finish. The messages interleave at the awaits, and because the runtime is single-threaded and the delays are equal, the order is fixed. With threads this program needed `Arc` for nothing and a `join` for each thread; here it is three blocks and one `join!`.

## 17.4 Racing and timeouts

Sometimes the first future to finish is the one you want:

@@ race

`tokio.select!` polls its arms and returns the first to complete, dropping the others — `fast` wins, and `slow` is cancelled mid-sleep. Its body is written like any Harsh block, `tokio.select! do:` with the arms beneath: a line with a `=>` in it is an arm and takes the comma Rust wants, and `slow "slow"` is a call as everywhere else. `tokio.time.timeout` is the common special case — a future and a limit, `Err` if the limit comes first — and here it does.

## 17.5 The one rule

An async program shares its threads among all its tasks, so a task that *blocks* — a CPU-bound loop, a `std.thread.sleep`, a synchronous file read — stalls every other task on that thread. The rule is: never block in async code. When the work is blocking, hand it to a thread:

@@ blocking

`spawn_blocking` runs the closure on a thread from a pool kept for the purpose, and returns a future that resolves to the closure's result; the runtime keeps turning while the thread works. The closure is a trailing `||:` with its body beneath, as in chapter 13. Everything in the standard library that blocks — `std.fs`, `std.thread.sleep`, `Mutex` held across an await — has this shape in async code: either an async version from the runtime (`tokio.fs`, `tokio.time.sleep`, `tokio.sync.Mutex`) or a `spawn_blocking`.

## 17.6 What you have

`async fn` returns a future; `<- await` drives one and pauses the caller; a runtime — `#[tokio.main]` — drives `main`. `tokio.spawn async:` for an independent task, `async:` and `async move:` blocks as values, `tokio.join!` to run several to completion together, `tokio.select! do:` to take the first, `timeout` for a limit, async channels for messages between tasks, and `spawn_blocking` for anything that would block. Harsh lays out an `async:` block as it does a closure's body; everything else is Rust's.

Next: the object-oriented features Rust has and the ones it deliberately lacks — and trait objects, which is how a `Vec` holds values of different types.

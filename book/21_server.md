# 21. Final project: a multithreaded web server

The last chapter builds a web server from the standard library alone: a TCP listener, HTTP by hand, a thread pool, and a graceful shutdown. It uses nearly everything in the book — closures, trait objects, channels, `Arc<Mutex<T>>`, `Drop`, `Option.take` — and is the kind of program that in most languages would be a framework. Each stage here is a complete project that runs to completion: the server spawns its own clients, serves a fixed number of requests, and stops, so that the book can show the run. Point a browser at the listening address instead and it is a real server.

## 21.1 A single-threaded server

@@ single

`TcpListener.bind "127.0.0.1:0"` — port 0 means "any free port", and `local_addr$` says which; a real server binds `127.0.0.1:7878` and you open it in a browser. `listener <- incoming$` is an iterator of connections, `take 3` stops it after three, and each is handed to `handle_connection`.

`handle_connection` reads the first line of the request — `GET / HTTP/1.1` — through a `BufReader`, matches it against the three paths it knows, and writes a response: a status line, a `Content-Length` header, a blank line, the file. That is HTTP, or enough of it. The `/sleep` path sleeps first, to stand in for a slow request. `(status_line, filename)` comes out of a `match` whose arms are tuples, with the slow arm a `do:` block; and the request line is matched as a `&str` slice, `&request_line[..]`, because string literals are `&str` and the patterns must have the value's type.

The client is a thread that connects three times in a row and prints each status. Watch the order of the log at the end: the third request could not start until the second's sleep was over, because a single thread served them one at a time. A browser tab waiting behind someone else's slow request is what that feels like.

## 21.2 A thread pool

Spawning a thread per connection would fix that and open a denial-of-service hole — a thousand connections, a thousand threads. A *thread pool* is a fixed number of threads that take jobs from a queue, and it is the shape of most servers:

@@ pool

`ThreadPool` is in `lib.hrs`, so it can be tested and reused; `main.hrs` uses it. Reading the library top to bottom:

- A `Job` is a boxed closure, `Box<dyn FnOnce() + Send + 'static>`: a trait object, because each job is a different closure type; `FnOnce`, because it runs once; `Send` and `'static`, because it crosses to another thread and may outlive the caller. A type alias names it once.
- `ThreadPool.new` makes a channel, wraps the *receiver* in `Arc<Mutex<…>>` — one receiver, shared by every worker, locked to take a job — and spawns `size` workers each holding a clone of the `Arc`. `assert! (size > 0)` is the documented panic.
- `execute` boxes the closure and sends it. The bound is in a `[where …]` clause; `self <- sender <- as_ref$ <- unwrap$` reaches the sender inside the `Option`.
- A `Worker` is a thread in a `loop`: lock the receiver, `recv$` a job, *release the lock* (the guard is dropped at the end of the `let` statement — which is why `recv$` is a separate statement from the `match`, so that other workers can take jobs while this one runs), then run it. `Err` from `recv$` means the sender is gone, and the loop ends.
- `Drop for ThreadPool` is the graceful shutdown: `take$` the sender out of its `Option` and drop it, so every worker's next `recv$` returns `Err` and it exits its loop; then `join$` each worker's thread, taken out of *its* `Option`. The two `Option`s exist because `drop` has only `&mut self` and cannot move a field out of it — `take$` moves the value and leaves `None`, which is chapter 18's trick with the post's state.

`main` now starts four clients *at once*, hands each connection to `pool <- execute move ||:`, and `drop pool` at the end waits for the workers. The slow request no longer delays the others — the clients' statuses arrive in the order the threads finish, which is why the served log is sorted before printing. The four `worker N shut down` lines are the graceful shutdown doing exactly what it says.

Everything in this program is a chapter you have read: channels and `Arc<Mutex<T>>` from 16, `Box<dyn FnOnce>` from 18, `Drop` and `take$` from 15 and 18, the `[where …]` bound from 10, `move ||:` from 13. That is the book's argument, made one last time: the notation stayed out of the way, and Rust was what you learned.

## 21.3 Where to go

The Rust Book this one follows has a chapter of appendices — keywords, operators, derivable traits, the tools — which are Rust's and not repeated here. For Harsh itself, the *language guide* is the reference for every construct in every form, and it is short, because Harsh is short. For Rust, the standard library documentation is the next book: `std` is large and well written, and after twenty-one chapters you can read any page of it. Build something. The compiler will tell you when you are wrong, and you now know how to read what it says.

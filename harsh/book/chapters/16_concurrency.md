# 16. Fearless concurrency

Concurrent code — several things happening at once, on threads — is where most languages' worst bugs live: data races, where two threads touch one value with no ordering; deadlocks; values used after the thread that owned them is gone. Rust makes the first of those a compile error, using nothing new: the ownership and borrowing rules of chapter 4, plus two traits that say which types may cross a thread boundary. The result is what the Rust community calls *fearless* concurrency — not that it is easy, but that the compiler catches the class of mistake that is otherwise found in production at three in the morning. This chapter is threads, message passing, shared state, and the two traits.

## 16.1 Threads

`thread.spawn` runs a closure on a new thread and returns a handle:

```
use std.thread
use std.time.Duration

fn main$:
    let handle = thread.spawn ||:
        for i in 1..4:
            println! "hi number {i} from the spawned thread!"
            thread.sleep (Duration.from_millis 1)

    println! "main is waiting"
    handle <- join$ <- unwrap$      // block until the spawned thread finishes
    println! "main is done"
```

```text
hi number 1 from the spawned thread!
main is waiting
hi number 2 from the spawned thread!
hi number 3 from the spawned thread!
main is done
```

The spawned thread prints three lines while the main thread carries on — the two are running at once, and without the `join` the main thread would reach its end and the process would exit, taking the spawned thread with it, however far it had got. `handle <- join$` blocks until the thread finishes, and `<- unwrap$` handles the case where the thread panicked. The closure is `||:` with its body beneath, the trailing block form; here it captures nothing.

Usually it captures something, and this is where ownership meets threads:

```
use std.thread

fn main$:
    let v = vec! [1, 2, 3]

    let handle = thread.spawn ||:
        println! "Here's a vector: {v:?}"

    handle <- join$ <- unwrap$
```

```text
error[E0373]: closure may outlive the current function, but it borrows `v`, which is owned by the current function
  --> move_err.hrs:6:31
   |
 6 |     let handle = thread.spawn ||:
   |                               ^^ may outlive borrowed value `v`
 7 |         println! "Here's a vector: {v:?}"
   |                                     - `v` is borrowed here
   = note: function requires argument type to outlive `'static` (hrs 6:18)
   = help: to force the closure to take ownership of `v` (and any other referenced variables), use the `move` keyword (hrs 6:31)
```

The closure borrows `v`, and the compiler asks a question the language forces: how long will the thread run? It cannot know — the thread might outlive `main`, and then the borrow would dangle. So a closure passed to `spawn` must own everything it uses, and the help says how:

```
use std.thread

fn main$:
    let v = vec! [1, 2, 3]

    let handle = thread.spawn move ||:
        println! "Here's a vector: {v:?}"

    handle <- join$ <- unwrap$
```

```text
Here's a vector: [1, 2, 3]
```

`move ||:` — chapter 13's `move`, moving `v` into the closure, which then owns it for as long as the thread runs. This is the one place `move` is not optional, and the error when it is missing is precise.

## 16.2 Message passing

One way to share data between threads is not to share it: send it. A *channel* has a transmitter and a receiver, and a value sent down it is moved from one thread to the other:

```
use std.sync.mpsc
use std.thread

fn main$:
    let (tx, rx) = mpsc.channel$

    thread.spawn move ||:
        let val = String.from "hi"
        tx <- send val <- unwrap$

    let received = rx <- recv$ <- unwrap$
    println! "Got: {received}"
```

```text
Got: hi
```

`mpsc.channel$` — *multiple producer, single consumer* — returns the pair `(tx, rx)`. The spawned thread moves `tx` in, makes a `String`, and `tx <- send val` moves the string into the channel; `rx <- recv$` blocks the main thread until something arrives and hands it over. The string was on the spawned thread and is now on the main one, and no lock was taken because at no moment did both have it.

That "moved" is enforced:

```
use std.sync.mpsc
use std.thread

fn main$:
    let (tx, rx) = mpsc.channel$

    thread.spawn move ||:
        let val = String.from "hi"
        tx <- send val <- unwrap$
        println! "val is {val}"        // val was moved into the channel

    let received = rx <- recv$ <- unwrap$
    println! "Got: {received}"
```

```text
error[E0382]: borrow of moved value: `val`
  --> channel_moved.hrs:10:26
   |
 8 |         let val = String.from "hi"
   |             --- move occurs because `val` has type `String`, which does not implement the `Copy` trait
 9 |         tx <- send val <- unwrap$
   |                    --- value moved here
10 |         println! "val is {val}"        // val was moved into the channel
   |                          ^^^^^ value borrowed here after move
   = help: consider cloning the value if the performance cost is acceptable (hrs 9:23)
```

`send` took `val`; using it afterwards is chapter 4's error, and it is the right error, because the receiving thread may have modified or dropped the value by now. In a language where sending is a copy of a pointer, this compiles and the two threads quietly share a string.

Several producers, one consumer:

```
use std.sync.mpsc
use std.thread
use std.time.Duration

fn main$:
    let (tx, rx) = mpsc.channel$
    let tx1 = tx <- clone$

    thread.spawn move ||:
        for val in ["hi", "from", "the", "thread"]:
            tx1 <- send (String.from val) <- unwrap$
            thread.sleep (Duration.from_millis 1)

    thread.spawn move ||:
        for val in ["more", "messages", "for", "you"]:
            tx <- send (String.from val) <- unwrap$
            thread.sleep (Duration.from_millis 1)

    let mut received: Vec<String> = rx <- iter$ <- collect$    // ends when every sender is dropped
    received <- sort$
    println! "{received:?}"
```

```text
["for", "from", "hi", "messages", "more", "the", "thread", "you"]
```

`tx <- clone$` makes a second transmitter for the second thread. `rx <- iter$` yields each message as it arrives and ends when *every* transmitter has been dropped — both threads finish, their `tx`s go out of scope, the iterator stops, and `collect$` has all eight words. (They are sorted before printing because the two threads' messages interleave in an order the scheduler decides, and the book's output has to be the same every build.)

## 16.3 Shared state

The other way is to share the data and take turns. A `Mutex` — *mutual exclusion* — holds a value and gives out access to one thread at a time:

```
use std.sync.Mutex

fn main$:
    let m = Mutex.new 5

    do:
        let mut num = m <- lock$ <- unwrap$   // blocks until the lock is free
        *num = 6
    // the guard was dropped at the end of the block, releasing the lock

    println! "m = {m:?}"
```

```text
m = Mutex { data: 6, poisoned: false, .. }
```

`m <- lock$` blocks until the lock is free and returns a *guard* that derefs to the value: `*num = 6` writes through it. The guard implements `Drop`, and dropping it releases the lock — which happens at the end of the `do:` block here, and would happen at the end of any scope, so a lock cannot be forgotten. This is chapter 15's `Deref` and `Drop` doing the work: the API that makes a mutex hard to misuse is made of two traits.

To share a mutex between threads, it needs several owners, and chapter 15's answer to that was `Rc`:

```
use std.rc.Rc
use std.sync.Mutex
use std.thread

fn main$:
    let counter = Rc.new (Mutex.new 0)
    let mut handles = vec! []

    for _ in 0..10:
        let counter = Rc.clone (&counter)

        let handle = thread.spawn move ||:
            let mut num = counter <- lock$ <- unwrap$
            *num += 1

        handles <- push handle

    for handle in handles:
        handle <- join$ <- unwrap$

    println! "Result: {}" (*counter <- lock$ <- unwrap$)
```

```text
error[E0277]: `Rc<Mutex<i32>>` cannot be sent between threads safely
  --> mutex_rc_err.hrs:12:35
   |
12 |         let handle = thread.spawn move ||:
   |                      ------------ required by a bound introduced by this call
   |                                   ^^^^^^^^ `Rc<Mutex<i32>>` cannot be sent between threads safely
   |                                   ------- within this `{closure@mutex_rc_err.hrs:12:36: 12:43}`
   = help: within `{closure@mutex_rc_err.hrs:12:36: 12:43}`, the trait `Send` is not implemented for `Rc<Mutex<i32>>`
   = note: required because it's used within this closure (hrs 12:35)
   = note: required by a bound in `spawn`
```

Refused. `Rc` counts references without any synchronisation — two threads incrementing the count at once would corrupt it — and so `Rc<T>` is not `Send`, and `spawn` requires `Send`. The type system knows which types are safe to move across threads, and this one is not. The thread-safe reference count is `Arc`, *atomic* `Rc`, with the same API:

```
use std.sync.(Arc, Mutex)
use std.thread

fn main$:
    let counter = Arc.new (Mutex.new 0)
    let mut handles = vec! []

    for _ in 0..10:
        let counter = Arc.clone (&counter)

        let handle = thread.spawn move ||:
            let mut num = counter <- lock$ <- unwrap$
            *num += 1

        handles <- push handle

    for handle in handles:
        handle <- join$ <- unwrap$

    println! "Result: {}" (*counter <- lock$ <- unwrap$)
```

```text
Result: 10
```

`Arc.new (Mutex.new 0)`, `Arc.clone (&counter)` for each thread, `move` the clone into the closure, lock, increment; join all ten; read the result. Ten threads incremented one counter and the answer is ten, and any attempt to write the program without the lock — an `Arc<i32>` and `+= 1` — would not compile, because `Arc` only hands out shared references. The pattern `Arc<Mutex<T>>` is the standard shape of shared mutable state across threads, and every piece of it is a chapter you have read.

## 16.4 `Send` and `Sync`

The rejection of `Rc` came from two *marker traits* — traits with no methods, that a type implements to make a claim:

- `Send`: the type may be *moved* to another thread. Almost everything is `Send`; `Rc<T>` is the standard exception, and raw pointers.
- `Sync`: the type may be *referenced* from several threads at once — `&T` is `Send`. `Mutex<T>` is `Sync`; `RefCell<T>` is not, since its borrow counting is not thread-safe.

Both are implemented automatically for any type made entirely of `Send`/`Sync` parts, so you rarely write them; you meet them as the bound `spawn` puts on its closure, and as the error when a type does not qualify. They are the whole of Rust's concurrency guarantee: the rules of chapter 4 decide who may touch what, and these two traits decide which types may cross the line, and the compiler checks both.

## 16.5 What you have

`thread.spawn` with a `move` closure and `join$` on the handle. Channels — `mpsc.channel$`, `send` moves a value across, `recv$` and `rx <- iter$` receive, clone the transmitter for more producers — for sharing by not sharing. `Mutex` for taking turns, with a guard that unlocks on drop; `Arc` for owning one across threads; `Arc<Mutex<T>>` as the pattern. `Send` and `Sync`, which are what make the compiler refuse the unsafe versions.

Next: `async` — concurrency without threads, for the programs that wait on the network more than they compute.

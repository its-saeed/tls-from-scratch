# Lesson 1: Futures by Hand

## Real-life analogy: the buzzer

You're at a burger joint. You order at the counter and they hand you a buzzer.

```
You:     "Can I have a burger?"
Counter: "Not ready yet. Here's a buzzer." (Poll::Pending + Waker)
You:     Go sit down, check your phone, chat with friends.
         ...
Buzzer:  *BZZZ* (waker.wake())
You:     Walk to counter. "Is my burger ready?"
Counter: "Yes, here it is!" (Poll::Ready(burger))
```

Without the buzzer (no waker), you'd have to keep walking to the counter every 10 seconds asking "is it ready yet?" — wasteful. Without async (blocking), you'd stand frozen at the counter unable to do anything until the burger is done.

The `Future` trait is the burger order. The `Waker` is the buzzer. The executor (runtime) is you, managing multiple buzzer orders at once.

## What is a Future?

A future is a value that might not be ready yet. It's Rust's core async abstraction — **the single most important trait in async Rust**:

```rust
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

pub enum Poll<T> {
    Ready(T),    // "Here's your result"
    Pending,     // "Not ready yet, I'll buzz you"
}
```

That's the entire trait. One method: `poll`. When called:
- Return `Poll::Ready(value)` → the result is available, we're done
- Return `Poll::Pending` → not ready yet, will notify via waker when ready

### The three parts of poll()

```
fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>
         ─────────────────  ──────────────────────    ────────────────────
                │                     │                        │
                │                     │                        │
         Pin<&mut Self>          Context                   Poll<T>
         "I promise not        "Here's a waker            "Ready or
          to move this          (buzzer) so you            Pending?"
          future in memory"     can notify me"
```

- **`Pin<&mut Self>`** — we'll cover this in Lesson 5. For now: it prevents the future from being moved in memory. Ignore it for simple futures.
- **`Context`** — contains the `Waker`. The future uses `cx.waker()` to notify the runtime when it should be polled again.
- **`Poll<T>`** — the result: either done (`Ready`) or not yet (`Pending`).

## What `.await` does

When you write:

```rust
let value = some_future.await;
```

The compiler transforms it into roughly:

```rust
loop {
    match some_future.poll(cx) {
        Poll::Ready(value) => break value,     // done! use the value
        Poll::Pending => yield_to_runtime(),   // give control back, wait for wake
    }
}
```

`.await` = "keep polling until ready, yielding control between attempts."

### Visualizing the poll cycle

```
Executor                          Future (CountdownFuture { count: 3 })
   │                                 │
   ├── poll() ──────────────────►   │  count=3 → decrement → count=2
   │  ◄── Pending ──────────────────┤  wake_by_ref() → "poll me again"
   │                                 │
   ├── poll() ──────────────────►   │  count=2 → decrement → count=1
   │  ◄── Pending ──────────────────┤  wake_by_ref() → "poll me again"
   │                                 │
   ├── poll() ──────────────────►   │  count=1 → decrement → count=0
   │  ◄── Pending ──────────────────┤  wake_by_ref() → "poll me again"
   │                                 │
   ├── poll() ──────────────────►   │  count=0 → done!
   │  ◄── Ready(()) ───────────────┤  future is complete
   │                                 │
   │  (never poll again)             │
```

## The contract

Three rules that futures MUST follow:

1. **Don't poll after Ready** — once a future returns `Ready`, it's done. Polling it again is undefined behavior. The result has been consumed.

2. **Pending MUST wake** — if you return `Pending`, you MUST arrange for `cx.waker().wake()` to be called eventually. Otherwise the executor will never poll you again and the task hangs forever. This is the most common async bug.

3. **Poll should be cheap** — do a small amount of work, then return. Don't block the thread (no `std::thread::sleep`, no blocking I/O). If you block inside `poll`, you freeze the entire executor.

```
Rule 2 visualized — what happens if you forget to wake:

Executor                          Future
   │                                │
   ├── poll() ────────────────►    │
   │  ◄── Pending ─────────────────┤  forgot to call wake()!
   │                                │
   │  ... executor waits ...        │  ... future waits ...
   │  ... nobody wakes anybody ...  │  ... nobody wakes anybody ...
   │                                │
   │  💀 DEADLOCK — task hangs forever
```

## What a Waker actually is (preview)

You'll build one from scratch in Lesson 3. For now, the key idea:

```
┌─────────────────────────────────────────────────────────┐
│  Waker = a callback handle                              │
│                                                         │
│  waker.wake()      → tells the executor: "re-poll me!"  │
│  waker.wake_by_ref() → same, without consuming the waker│
│  waker.clone()     → copy it, store it for later         │
│                                                         │
│  Internally: a function pointer + data pointer           │
│  The executor provides the implementation                │
│  The future just calls .wake() — doesn't know the details│
└─────────────────────────────────────────────────────────┘
```

For this lesson's exercises, we'll use a **noop waker** — a waker that does nothing when called. This is enough for manual polling in a loop.

## Noop waker (use this for exercises)

Since Rust 1.85+, you can use:

```rust
use std::task::Waker;
let waker = Waker::noop();
```

If your Rust version is older:

```rust
use std::task::{RawWaker, RawWakerVTable, Waker};

fn noop_waker() -> Waker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, no_op, no_op, no_op);
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}
```

## Exercises

### Exercise 1: CountdownFuture

Implement a future that counts down from N to 0. Each `poll` decrements the counter and returns `Pending`. When it hits 0, return `Ready(())`.

```rust
struct CountdownFuture {
    count: u32,
}

impl Future for CountdownFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        // TODO:
        // if count > 0: decrement, call cx.waker().wake_by_ref(), return Pending
        // if count == 0: return Ready(())
    }
}
```

Why `wake_by_ref()` when we return `Pending`? Because without it, the executor doesn't know to poll us again. In a real future, you'd only wake when something actually happens (data arrives, timer fires). Here, we always want to be re-polled immediately — so we wake every time.

### Exercise 2: ReadyFuture

Implement a future that immediately returns a value on first poll:

```rust
struct ReadyFuture<T>(Option<T>);
```

- First poll: take the value out of the `Option`, return `Ready(value)`
- The `Option` ensures the value is only returned once

This is what `std::future::ready(42)` does internally.

### Exercise 3: DelayFuture

Implement a future that returns `Pending` for a specified number of polls, then returns `Ready` with a message:

```rust
struct DelayFuture {
    polls_remaining: u32,
    message: String,
}
```

This simulates a future that takes "time" (polls) to complete, like waiting for I/O.

### Exercise 4: Poll manually

Don't use any executor. Use the noop waker to manually poll futures in a loop:

```rust
let waker = Waker::noop();
let mut cx = Context::from_waker(&waker);
let mut future = CountdownFuture { count: 5 };
let mut pinned = std::pin::pin!(future);

loop {
    match pinned.as_mut().poll(&mut cx) {
        Poll::Ready(()) => { println!("Done!"); break; }
        Poll::Pending => { println!("Not ready yet..."); }
    }
}
```

See the state change with each poll. This is literally what an executor does — just a poll loop.

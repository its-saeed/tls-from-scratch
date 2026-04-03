# Lesson 2: State Machines

## Real-life analogy: the vending machine

A vending machine is a state machine:

```
┌─────────┐  insert coin    ┌──────────┐   press button  ┌────────────┐
│  Idle   │ ──────────────► │ HasCoin  │ ──────────────► │ Dispensing │
│         │                 │          │                 │            │
└─────────┘                 └──────────┘                 └─────┬──────┘
     ▲                                                         │
     │                    item drops out                       │
     └─────────────────────────────────────────────────────────┘
```

At any moment, the machine is in one state. An event causes a transition to the next state. It never skips states or goes backwards unexpectedly.

`async fn` works the same way. Each `.await` is a state transition. The compiler turns your sequential code into an enum where each variant is a state.

## What `async fn` compiles to

When you write:

```rust
async fn fetch_data() -> String {
    let url = build_url().await;         // await #1
    let response = http_get(url).await;  // await #2
    response.body
}
```

The compiler generates something like:

```rust
enum FetchData {
    // State 0: haven't started yet
    Start,
    // State 1: waiting for build_url() to complete
    // Holds the sub-future for build_url
    WaitingForUrl {
        build_url_future: BuildUrlFuture,
    },
    // State 2: got the url, waiting for http_get() to complete
    // Holds `url` (needed later) and the sub-future for http_get
    WaitingForResponse {
        http_get_future: HttpGetFuture,
    },
    // State 3: done
    Done,
}
```

And implements `Future` for it:

```rust
impl Future for FetchData {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<String> {
        loop {
            match self.as_mut().get_mut() {
                FetchData::Start => {
                    // Create the sub-future for build_url
                    *self = FetchData::WaitingForUrl {
                        build_url_future: build_url(),
                    };
                }
                FetchData::WaitingForUrl { build_url_future } => {
                    match Pin::new(build_url_future).poll(cx) {
                        Poll::Pending => return Poll::Pending,  // not ready, yield
                        Poll::Ready(url) => {
                            // Transition to next state
                            *self = FetchData::WaitingForResponse {
                                http_get_future: http_get(url),
                            };
                        }
                    }
                }
                FetchData::WaitingForResponse { http_get_future } => {
                    match Pin::new(http_get_future).poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(response) => {
                            *self = FetchData::Done;
                            return Poll::Ready(response.body);
                        }
                    }
                }
                FetchData::Done => panic!("polled after completion"),
            }
        }
    }
}
```

### Visualizing the state transitions

```
poll #1:  Start → WaitingForUrl → poll build_url → Pending
                                                     ↑ return to executor

poll #2:  WaitingForUrl → poll build_url → Ready(url)
          → transition to WaitingForResponse
          → poll http_get → Pending
                              ↑ return to executor

poll #3:  WaitingForResponse → poll http_get → Ready(response)
          → transition to Done
          → return Ready(response.body)
```

Each `poll()` call resumes exactly where the last one left off. No stack needed — the enum variant holds all the state.

## Why this matters

### Memory: enum vs thread stack

```
Thread:
┌────────────────────┐
│  Stack: 8 MB       │  Fixed allocation, mostly empty.
│  (99% wasted)      │  Every thread gets this whether it
│                    │  needs it or not.
└────────────────────┘

Async state machine:
┌────────────────────┐
│  Enum: ~100 bytes  │  Size = largest variant.
│  (nothing wasted)  │  Only stores what the current
│                    │  state actually needs.
└────────────────────┘

10,000 tasks:
  Threads:    10,000 × 8 MB   = 80 GB
  Async:      10,000 × 100 B  = 1 MB
```

### The compiler does the hard work

Writing state machines by hand is tedious and error-prone. `async`/`.await` gives you:
- **Readability** of sequential code
- **Performance** of hand-written state machines
- **Safety** guaranteed by the compiler

## A simpler example: `add_slowly`

Let's desugar a simple async function step by step:

```rust
// The async version (what you write):
async fn add_slowly(a: u32, b: u32) -> u32 {
    let x = yield_once(a).await;   // yields once, returns a
    let y = yield_once(b).await;   // yields once, returns b
    x + y
}
```

Where `yield_once` is a future that returns `Pending` once, then `Ready(value)`:

```rust
struct YieldOnce<T> {
    value: Option<T>,
    yielded: bool,
}
```

The state machine for `add_slowly`:

```
           ┌──────────────────┐
           │   State: Start   │  holds: a, b
           │                  │
           └────────┬─────────┘
                    │ create YieldOnce(a), poll it → Pending
                    ▼
           ┌──────────────────┐
           │ State: YieldingA │  holds: b, yield_future_a
           │                  │
           └────────┬─────────┘
                    │ poll yield_future_a → Ready(x)
                    │ create YieldOnce(b), poll it → Pending
                    ▼
           ┌──────────────────┐
           │ State: YieldingB │  holds: x, yield_future_b
           │                  │
           └────────┬─────────┘
                    │ poll yield_future_b → Ready(y)
                    │ compute x + y
                    ▼
           ┌──────────────────┐
           │   State: Done    │  return Ready(x + y)
           └──────────────────┘
```

Notice: each state only holds what's needed going forward. State `YieldingB` holds `x` (needed for the final addition) but NOT `a` (already consumed).

## See what the compiler generates

```sh
# Install cargo-expand
cargo install cargo-expand

# Write a simple async fn and expand it
cargo expand --bin 2-state-machines 2>/dev/null | head -100
```

The output is verbose but you'll see an enum with variants matching the await points.

## Exercises

### Exercise 1: YieldOnce future

Implement `YieldOnce<T>` — a future that returns `Pending` on the first poll (and wakes), then `Ready(value)` on the second poll. This simulates one async operation completing.

### Exercise 2: Manual AddSlowly state machine

Implement `AddSlowly` as an enum with the states shown above. Implement `Future` for it by hand — match on the current state, poll sub-futures, transition states.

Run it with `poll_to_completion` from Lesson 1 and verify it returns the correct sum.

### Exercise 3: Async version comparison

Write the same logic as `async fn add_slowly` using actual `async`/`.await`. Run both (your hand-written state machine and the async version). Verify they produce the same result.

### Exercise 4: Future sizes

Print the size of various futures:

```rust
async fn no_awaits() -> u32 { 42 }
async fn one_await() -> u32 { yield_once(42).await }
async fn holds_big_data() -> u32 {
    let buf = [0u8; 1024];
    yield_once(0).await;
    buf[0] as u32
}
```

Use `std::mem::size_of_val(&future)`. Compare the sizes — the future that holds `[u8; 1024]` across an await will be ~1KB larger.

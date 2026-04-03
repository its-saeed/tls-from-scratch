# Lesson 3: Wakers & Waking

## Real-life analogy: notification systems

Think about all the ways you get notified in daily life:

```
Scenario                    Blocking (bad)              Waker (good)
─────────────────────────────────────────────────────────────────────
Pizza delivery              Stand at the door            Doorbell rings
                            staring at the street

Laundry                     Sit in front of the          Washer beeps
                            machine watching it spin     when done

Doctor's office             Stand at the counter         They call
                            asking "is it my turn?"      your name

Package delivery            Refresh tracking page        Push notification
                            every 5 seconds              "delivered!"
```

In all cases, the **waker pattern** lets you do other things while waiting. The notification system (doorbell, beep, name call, push notification) is the `Waker`.

In async Rust:
- Your future is you (waiting for pizza)
- The executor is your brain (managing all your tasks)
- The waker is the doorbell
- `wake()` = doorbell rings → your brain says "go check the door"

## The problem

When a future returns `Pending`, the executor needs to know **when** to poll it again.

```
Option A: Busy-polling (wasteful)

  Executor: "Are you ready?"  →  Future: "No"
  Executor: "Are you ready?"  →  Future: "No"
  Executor: "Are you ready?"  →  Future: "No"
  Executor: "Are you ready?"  →  Future: "No"
  Executor: "Are you ready?"  →  Future: "YES!"

  (CPU at 100% doing nothing useful)
```

```
Option B: Wakers (efficient)

  Executor: "Here's my number (waker). Call me when ready."
  Future: (stores the waker, waits for I/O)
  ... executor goes to sleep or handles other tasks ...
  Future: (I/O ready!) waker.wake() → "Hey executor, poll me!"
  Executor: "Oh! Let me check." → Future: "Ready!"

  (CPU at 0% while waiting)
```

## How Waker works internally

A `Waker` is built from a `RawWaker` — basically two pointers:

```
┌─────────────────────────────────────────────────────────┐
│  RawWaker                                               │
│                                                         │
│  ┌──────────────────┐   ┌───────────────────────────┐   │
│  │ data: *const ()  │   │ vtable: &RawWakerVTable   │   │
│  │                  │   │                           │   │
│  │ Points to the    │   │ Function pointers:        │   │
│  │ task/executor    │   │   clone()  → copy waker   │   │
│  │ state. Could be  │   │   wake()   → notify exec  │   │
│  │ an Arc<Task>,    │   │   wake_by_ref() → same    │   │
│  │ a task ID, etc.  │   │   drop()   → cleanup      │   │
│  └──────────────────┘   └───────────────────────────┘   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

The vtable is like a trait object — function pointers that define behavior. Different executors provide different vtable implementations:

```
Noop executor:     wake() = do nothing
Thread executor:   wake() = thread::unpark(thread_handle)
Tokio:             wake() = push task to scheduler queue
```

The future doesn't know or care how the waker works internally. It just calls `waker.wake()` and trusts that the executor will re-poll.

## Building a Waker from scratch

### Step 1: Define the vtable functions

```rust
// Each function receives the `data` pointer from RawWaker
unsafe fn clone(data: *const ()) -> RawWaker {
    // Create a copy of the waker (increment refcount, clone Arc, etc.)
}
unsafe fn wake(data: *const ()) {
    // Notify the executor: "re-poll the task associated with this data"
}
unsafe fn wake_by_ref(data: *const ()) {
    // Same as wake, but doesn't consume the waker
}
unsafe fn drop(data: *const ()) {
    // Clean up (decrement refcount, free memory, etc.)
}
```

### Step 2: Create the vtable

```rust
static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
```

### Step 3: Create the waker

```rust
let raw_waker = RawWaker::new(data_ptr, &VTABLE);
let waker = unsafe { Waker::from_raw(raw_waker) };
```

## The three wakers you'll build

### 1. Noop waker
All vtable functions do nothing. Useful for testing — you manually poll in a loop.

```
wake() → (does nothing)
Used in: Lesson 1 exercises, simple tests
```

### 2. Counting waker
`wake()` increments an atomic counter. Lets you verify how many times a future called `wake`.

```
wake() → counter.fetch_add(1)
Used in: testing that futures wake correctly
```

### 3. Thread-parking waker
`wake()` calls `thread::unpark()` on a specific thread. The executor thread parks itself after `Pending`, then the waker unparks it.

```
Executor thread          Future                    Waker
     │                     │                         │
     ├── poll() ────────►  │                         │
     │  ◄── Pending ───────┤  stores waker           │
     │                     │                         │
     ├── thread::park()    │                         │
     │   (sleeping)        │                         │
     │                     │  (I/O ready)            │
     │                     ├── waker.wake() ────────►│
     │                     │                         ├── thread::unpark()
     │  ◄── (wakes up!) ──────────────────────────── │
     │                     │                         │
     ├── poll() ────────►  │                         │
     │  ◄── Ready(val) ────┤                         │
```

This is how real single-threaded executors work. You'll use this pattern in Lesson 4.

## Exercises

### Exercise 1: Noop waker

Build a noop waker from `RawWaker` + vtable where all functions do nothing. Use it to manually poll the `CountdownFuture` from Lesson 1. Print each poll result.

### Exercise 2: Counting waker

Build a waker that stores an `Arc<AtomicU32>` as the data pointer. Each `wake()` call increments the counter. Poll a `CountdownFuture(5)` and verify the waker was called 5 times.

Hints:
- `Arc::into_raw(arc)` converts `Arc<T>` to `*const T` (which you can cast to `*const ()`)
- `Arc::from_raw(ptr)` converts back (for clone and drop)
- Be careful with reference counting — `clone` should increment, `drop` should decrement

### Exercise 3: Thread-parking waker

Build a waker that calls `thread.unpark()` when `wake()` is called.

1. Get the current thread handle: `std::thread::current()`
2. Store it in the waker's data pointer
3. `wake()` calls `thread_handle.unpark()`
4. In your poll loop: after `Pending`, call `std::thread::park()`
5. The waker's `wake()` will unpark you, and you'll poll again

This is a real executor pattern. Test it with `CountdownFuture` — but note: the future calls `wake_by_ref()` synchronously during `poll()`, so `park()` will immediately return (the thread was already unparked before it parks). This is fine — `park` returns immediately if there's a pending unpark.

### Exercise 4: Waker contract verification

Create a future that intentionally **doesn't** call `wake()` when returning `Pending`. Use your thread-parking waker. Show that the executor hangs — `thread::park()` blocks forever because nobody calls `unpark()`. This demonstrates why Rule 2 ("Pending must wake") exists.

Interrupt with Ctrl+C after a few seconds.

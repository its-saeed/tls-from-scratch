# Lesson 13: Channels

> **Prerequisites**: Lesson 4 (Tasks), Lesson 10 (Task Scheduling). Channels are how tasks communicate — they need wakers to notify receivers.

## Real-life analogy: the mailbox

```
Oneshot channel = a one-time letter:
┌────────────────┐                      ┌────────────────┐
│  Sender        │   sends ONE letter   │  Receiver      │
│                │ ────────────────────► │                │
│  "Here's your  │                      │  Waits at      │
│   blood test   │                      │  mailbox       │
│   result"      │                      │  until letter  │
│                │                      │  arrives       │
└────────────────┘                      └────────────────┘
  Used once, then both sides are done.

MPSC channel = a mailbox with multiple senders:
┌────────────────┐
│  Sender A      │──┐
└────────────────┘  │
┌────────────────┐  │    ┌────────────────┐
│  Sender B      │──┼───►│  Receiver      │
└────────────────┘  │    │  (one mailbox) │
┌────────────────┐  │    └────────────────┘
│  Sender C      │──┘
└────────────────┘
  Multiple producers, single consumer.
  Messages queue up if receiver is busy.
```

The key difference from `std::sync::mpsc`: async channels **wake** the receiver when a message arrives instead of blocking a thread.

## How async channels work

The core pattern: shared state + waker.

```
┌────────────────────────────────────────────────┐
│  Shared State (Arc<Mutex<Inner>>)              │
│                                                │
│  queue: VecDeque<T>     ← messages waiting     │
│  rx_waker: Option<Waker> ← receiver's waker   │
│  closed: bool           ← sender dropped?      │
│                                                │
│  Sender writes:                                │
│    1. Lock inner                               │
│    2. Push message to queue                    │
│    3. If rx_waker is Some → wake it            │
│                                                │
│  Receiver reads:                               │
│    1. Lock inner                               │
│    2. Pop from queue → got message? Ready      │
│    3. Queue empty? Store waker, return Pending │
│                                                │
└────────────────────────────────────────────────┘
```

### The sequence

```
Sender                  Shared State              Receiver
  │                        │                         │
  │                        │          poll() ◄───────┤
  │                        │  queue empty             │
  │                        │  store waker ◄───────────┤
  │                        │                Pending ──►
  │                        │                         │
  │  send("hello") ──────►│                         │
  │  push to queue         │                         │
  │  wake receiver ────────┼──► waker.wake()         │
  │                        │                         │
  │                        │          poll() ◄───────┤
  │                        │  pop "hello"             │
  │                        │         Ready("hello") ──►
```

## Oneshot channel

The simplest async channel: one message, one sender, one receiver.

```rust
struct Inner<T> {
    value: Option<T>,
    rx_waker: Option<Waker>,
    closed: bool,
}

struct Sender<T> {
    inner: Arc<Mutex<Inner<T>>>,
}

struct Receiver<T> {
    inner: Arc<Mutex<Inner<T>>>,
}
```

### Sender::send

```rust
impl<T> Sender<T> {
    fn send(self, value: T) -> Result<(), T> {
        let mut inner = self.inner.lock().unwrap();
        if inner.closed {
            return Err(value);  // receiver dropped
        }
        inner.value = Some(value);
        if let Some(waker) = inner.rx_waker.take() {
            waker.wake();  // notify receiver
        }
        Ok(())
    }
}
```

Note: `send` consumes `self` — you can only send once.

### Receiver as a Future

```rust
impl<T: Unpin> Future for Receiver<T> {
    type Output = Result<T, RecvError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(value) = inner.value.take() {
            Poll::Ready(Ok(value))
        } else if inner.closed {
            Poll::Ready(Err(RecvError))  // sender dropped without sending
        } else {
            inner.rx_waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
```

### Usage

```rust
let (tx, rx) = oneshot::channel();
spawn(async move { tx.send(42).unwrap(); });
let value = rx.await.unwrap();  // 42
```

## MPSC channel

Multiple senders, one receiver. Messages buffer up in a queue.

Key differences from oneshot:
- **Queue** instead of single value
- **Sender is Clone** — track how many senders exist
- **Bounded vs unbounded** — bounded adds backpressure (Lesson 23)
- **All senders drop** → channel closed

## Closed channel detection

When the sender drops without sending, the receiver should be woken with an error:

```rust
impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        inner.closed = true;
        if let Some(waker) = inner.rx_waker.take() {
            waker.wake();  // wake receiver so it sees the closure
        }
    }
}
```

## Exercises

### Exercise 1: Oneshot channel

Implement `oneshot::channel()` → `(Sender<T>, Receiver<T>)`.
- Sender has `send(value)` that consumes self
- Receiver implements `Future`
- Test: send 42 from one task, receive in another

### Exercise 2: Oneshot drop detection

Test that dropping the sender without sending wakes the receiver with an error:
```rust
let (tx, rx) = oneshot::channel::<i32>();
drop(tx);
assert!(rx.await.is_err());
```

### Exercise 3: MPSC channel

Implement `mpsc::channel()` → `(Sender<T>, Receiver<T>)`.
- Sender is Clone, has `send(value)`
- Receiver has `async fn recv() → Option<T>` (None when all senders dropped)
- Test: 3 senders each send 10 messages, receiver collects all 30

### Exercise 4: Bounded MPSC

Add capacity to your MPSC: `mpsc::channel(cap)`.
- `send()` returns Pending when the queue is full
- `recv()` wakes one blocked sender when it pops a message
- Test: channel(2), send 3 messages — third blocks until receiver pops one

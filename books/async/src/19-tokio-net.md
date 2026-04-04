# Lesson 19: tokio::net — TcpListener, TcpStream, and the Reactor

> **Prerequisites**: Lessons 8-9 (async I/O, reactor), Lesson 17 (I/O driver).

## Real-life analogy: phone vs walkie-talkie

```
TCP = Phone call                       UDP = Walkie-talkie
┌─────────────────────┐                ┌─────────────────────┐
│ 1. Dial (connect)   │                │ 1. Press button     │
│ 2. Ring (SYN/ACK)   │                │ 2. Talk (send_to)   │
│ 3. "Hello?" (est.)  │                │ 3. Release button   │
│ 4. Conversation     │                │                     │
│ 5. "Bye" (FIN)      │                │ No connection.      │
│                      │                │ No guarantee.       │
│ Reliable, ordered,   │                │ Fast, unordered,    │
│ bidirectional stream │                │ fire-and-forget     │
└─────────────────────┘                └─────────────────────┘
```

## How tokio::net wraps mio

```
Your code                  Tokio                         OS
─────────────────────────────────────────────────────────────────
TcpStream::connect()  →  mio::net::TcpStream::connect()  →  connect(2)
stream.read(&buf)     →  poll_read()                      →  read(2)
  │                        │
  │  if WouldBlock:        │
  │  ┌─────────────────┐   │
  │  │ Register with    │   │
  │  │ I/O driver       │   │
  │  │ (mio::Registry)  │   │
  │  │ Return Pending   │   │
  │  └─────────────────┘   │
  │                        │
  │  Later, epoll/kqueue   │
  │  says "fd ready" ──────┤
  │  → wake task           │
  │  → poll_read() again   │
  │  → data available!     │
```

## TcpListener accept loop

The fundamental server pattern:

```rust
use tokio::net::TcpListener;

let listener = TcpListener::bind("0.0.0.0:8080").await?;
loop {
    let (stream, addr) = listener.accept().await?;
    tokio::spawn(async move {
        handle_connection(stream, addr).await;
    });
}
```

Internally, `accept()` calls `poll_accept()` which:
1. Tries `mio::TcpListener::accept()` (non-blocking)
2. On `WouldBlock` → registers with reactor, returns `Pending`
3. Reactor wakes task when a new connection arrives
4. `accept()` retries → succeeds this time

## Splitting a TcpStream

```
                    TcpStream
                   ┌─────────┐
                   │  read   │
                   │  write  │
                   │  (one   │
                   │   fd)   │
                   └────┬────┘
                        │
          ┌─────────────┴─────────────┐
          │  split() (borrowed)        │  into_split() (owned)
          ▼                            ▼
  ┌──────────────┐              ┌──────────────┐
  │  ReadHalf<'_>│              │OwnedReadHalf │ ← Send + 'static
  │  WriteHalf<'_>│              │OwnedWriteHalf│ ← Send + 'static
  └──────────────┘              └──────────────┘
  Same task only               Different tasks OK
```

- **`split()`** — borrows the stream, returns `ReadHalf` + `WriteHalf`. Not `Send`. Use within one task.
- **`into_split()`** — consumes the stream, returns owned halves. `Send`. Use when reader and writer are in different tasks.

```rust
// Two tasks: one reads, one writes
let (reader, writer) = stream.into_split();
tokio::spawn(read_loop(reader));
tokio::spawn(write_loop(writer));
```

## UDP sockets

```rust
let socket = tokio::net::UdpSocket::bind("0.0.0.0:3000").await?;
// No connection — just send/receive datagrams
socket.send_to(b"ping", "127.0.0.1:4000").await?;

let mut buf = [0u8; 1024];
let (len, addr) = socket.recv_from(&mut buf).await?;
```

## Exercises

### Exercise 1: TCP echo with split

Write a TCP echo server that uses `into_split()` to put reading and writing in separate tasks. The reader reads lines and sends them through a `tokio::sync::mpsc` channel to the writer.

### Exercise 2: UDP ping-pong

Create two UDP sockets. Socket A sends "ping" to Socket B. Socket B replies "pong". Print the round-trip time.

### Exercise 3: Connection counter

Build a TCP server that tracks total connections with an `Arc<AtomicUsize>`. Each new connection prints "Connection #N from {addr}". Echo data back, then increment the counter on disconnect.

### Exercise 4: Multi-client chat (simple)

TCP server where each client's message is broadcast to all others. Use `into_split()` + a shared `Vec<OwnedWriteHalf>` behind a `tokio::sync::Mutex`.

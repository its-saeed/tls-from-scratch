# Lesson 0: Why Async?

## Real-life analogy: the restaurant

Imagine a restaurant with 100 tables.

**Thread-per-connection model** = one waiter per table:

```
Table 1 → Waiter 1 (stands at table, waits for customer to decide)
Table 2 → Waiter 2 (stands at table, waits for food from kitchen)
Table 3 → Waiter 3 (stands at table, waits for customer to finish)
...
Table 100 → Waiter 100
```

Each waiter does nothing most of the time — they're **blocked** waiting. You need 100 waiters (expensive), and they're mostly idle. If 200 guests show up, you can't serve them.

**Event-driven model** = one waiter, many tables:

```
Waiter checks Table 1 → "still reading menu, skip"
Waiter checks Table 2 → "food is ready!" → serves it
Waiter checks Table 3 → "wants to order!" → takes order, sends to kitchen
Waiter checks Table 4 → "still eating, skip"
...
```

One waiter handles all tables by only doing work when a table needs something. The waiter never stands idle — they keep circling. This is **event-driven I/O**.

The buzzer system at a fast-food restaurant is even more accurate:
1. You order (register interest)
2. You sit down and do other things (non-blocking)
3. Buzzer vibrates (event notification — kqueue/epoll)
4. You pick up your food (handle the ready event)

## The problem: one thread per connection

The traditional server model:

```rust
loop {
    let stream = listener.accept();
    std::thread::spawn(|| handle(stream));  // one thread per client
}
```

Each thread costs real resources:

```
┌────────────────────────────────────────────────────┐
│              Thread Memory Layout                  │
│                                                    │
│  ┌──────────────┐  Each thread gets its own stack  │
│  │ Stack: 8 MB  │  allocated by the OS.            │
│  │              │  Most of it is never used —      │
│  │  (mostly     │  the thread is just waiting      │
│  │   empty,     │  on read() or write().           │
│  │   waiting)   │                                  │
│  │              │                                  │
│  └──────────────┘                                  │
│                                                    │
│  10,000 threads × 8 MB = 80 GB virtual memory      │
│  (actual RSS is lower, but overhead is still real) │
└────────────────────────────────────────────────────┘
```

### See it yourself

Check your system's default thread stack size:

```sh
# macOS
ulimit -s          # prints stack size in KB (usually 8192 = 8 MB)

# Linux
ulimit -s          # usually 8192 KB
cat /proc/sys/kernel/threads-max   # max threads the kernel allows
```

Check how many threads a process is using:

```sh
# macOS: count threads of a process
ps -M <pid> | wc -l

# Linux: count threads of a process
ls /proc/<pid>/task | wc -l

# Or for any process by name
ps -eLf | grep <process-name> | wc -l
```

### The C10K problem

In 1999, Dan Kegel asked: "how do you handle 10,000 concurrent connections?" With one thread per connection, you can't — you hit OS limits on memory, thread count, and context switching overhead.

```
Connections     Threads     Memory (8MB stack)    Context switches/sec
──────────────────────────────────────────────────────────────────────
100             100         800 MB                 ~10,000
1,000           1,000       8 GB                   ~100,000
10,000          10,000      80 GB                  ~1,000,000 (OS melts)
100,000         ???         impossible             impossible
```

Modern servers need to handle 100K-1M+ connections. Threads don't scale.

## The solution: event-driven I/O

Instead of blocking a thread per connection, use **one thread** that watches all connections:

```
┌──────────────────────────────────────────────────────────────┐
│                    Event-Driven Server                       │
│                                                              │
│  ┌─────────┐                                                 │
│  │ kqueue  │ ← register: "notify me when socket 5 is ready"  │
│  │ /epoll  │ ← register: "notify me when socket 8 is ready"  │
│  │         │ ← register: "notify me when socket 12 is ready" │
│  └────┬────┘                                                 │
│       │                                                      │
│       ▼  wait() — blocks until ANY socket is ready           │
│                                                              │
│  "Socket 8 is readable!"                                     │
│       │                                                      │
│       ▼  read from socket 8, process, respond                │
│       │                                                      │
│       ▼  back to wait()                                      │
│                                                              │
│  One thread. 100,000 connections. ~10 MB memory.             │
└──────────────────────────────────────────────────────────────┘
```

### See the event system yourself

```sh
# macOS: see kqueue in action
# Start a simple server (Python for quick demo)
python3 -c "
import socket
s = socket.socket()
s.bind(('127.0.0.1', 9999))
s.listen()
print('listening...')
conn, _ = s.accept()
print('connected:', conn.recv(1024))
" &

# Find its PID and trace the syscalls
lsof -i :9999                          # find which process is on port 9999
pgrep -f "python3"                     # or find PID by process name
sudo dtruss -p $(pgrep -f "python3") 2>&1 | grep -E 'kevent|kqueue|read|accept'
# You'll see: kqueue(), kevent() — the OS event notification API
```

```sh
# Linux: see epoll in action
strace -e epoll_wait,epoll_ctl,accept,read python3 -c "
import socket
s = socket.socket()
s.bind(('127.0.0.1', 9999))
s.listen()
s.accept()
"
# You'll see: epoll_create, epoll_ctl (register), epoll_wait (block until event)
```

### Blocking vs non-blocking: what actually happens

When you call `read()` on a **blocking** socket:

```
Your code                           OS Kernel
    │                                 │
    ├── read(fd) ──────────────────►  │
    │   (your thread is FROZEN)       │  waiting for data...
    │   (can't do anything else)      │  still waiting...
    │                                 │  data arrives!
    │  ◄── returns data ──────────────┤
    │                                 │
```

When you call `read()` on a **non-blocking** socket:

```
Your code                           OS Kernel
    │                                 │
    ├── read(fd) ──────────────────►  │
    │  ◄── WouldBlock (instantly) ────┤  no data yet
    │                                 │
    │  (go do other work!)            │
    │                                 │
    ├── read(fd) ──────────────────►  │
    │  ◄── WouldBlock (instantly) ────┤  still no data
    │                                 │
    │  ... later, after kqueue says   │
    │      "fd is ready" ...          │
    │                                 │
    ├── read(fd) ──────────────────►  │
    │  ◄── returns data ──────────────┤  data was ready!
```

See it yourself:

```sh
# Set a socket to non-blocking and watch the WouldBlock errors
python3 -c "
import socket
s = socket.socket()
s.setblocking(False)
try:
    s.connect(('example.com', 80))
except BlockingIOError as e:
    print(f'Non-blocking connect: {e}')  # Operation would block
"
```

## Where async fits

Writing event-driven code by hand is painful — you end up with callback spaghetti:

```rust
// Callback hell (event-driven without async)
socket.on_readable(|data| {
    process(data, |result| {
        socket.on_writable(|_| {
            socket.write(result, |_| {
                // deeply nested, hard to follow
            });
        });
    });
});
```

Rust's `async`/`.await` gives you event-driven performance with sequential-looking code:

```rust
// Same logic, but readable
async fn handle(stream: TcpStream) {
    let data = stream.read().await;   // yields to runtime, doesn't block thread
    let result = process(data);
    stream.write(result).await;       // yields to runtime, doesn't block thread
}
```

The compiler transforms this into a state machine (Lesson 2). The runtime (tokio) manages the event loop (Lesson 8). You write simple code, get scalable performance.

### The mental model

```
┌─────────────────────────────────────────────────────┐
│                What you write                       │
│                                                     │
│  async fn handle(stream: TcpStream) {               │
│      let data = stream.read().await;                │
│      stream.write(data).await;                      │
│  }                                                  │
└───────────────────┬─────────────────────────────────┘
                    │ compiler transforms
                    ▼
┌───────────────────────────────────────────────────────┐
│              What the compiler generates              │
│                                                       │
│  A state machine enum:                                │
│    State::Reading  → poll read, if not ready: Pending │
│    State::Writing  → poll write, if not ready: Pending│
│    State::Done     → return Ready(())                 │
└───────────────────┬───────────────────────────────────┘
                    │ runtime drives
                    ▼
┌─────────────────────────────────────────────────────┐
│              What the runtime does                  │
│                                                     │
│  loop {                                             │
│      events = kqueue.wait();                        │
│      for event in events {                          │
│          task = lookup(event.fd);                   │
│          task.poll(); // advance the state machine  │
│      }                                              │
│  }                                                  │
└─────────────────────────────────────────────────────┘
```

## The cost of async

Async isn't free. The trade-off:

```
                    Threads              Async
─────────────────────────────────────────────────────
Memory per task     2-8 MB (stack)       ~100 bytes (future struct)
Max connections     ~10K                 ~1M
Context switch      OS (expensive)       Userspace (cheap)
Code complexity     Simple               Pin, lifetimes, cancellation
Debugging           Good stack traces    Confusing stack traces
Ecosystem           Everything works     Need async versions of libs
CPU-bound work      Natural              Must use spawn_blocking
```

**Use async when**: many concurrent I/O operations (web servers, proxies, chat, databases)

**Don't use async when**: CPU-bound work, simple scripts, low concurrency, prototyping

## Exercises

### Exercise 1: Thread overhead benchmark

Spawn 10,000 threads that each `std::thread::sleep(Duration::from_secs(1))`. Measure:
- Wall time: `std::time::Instant::now()` before and after
- Peak memory: check with `ps` or `Activity Monitor` while running

Then do the same with 10,000 `tokio::spawn` tasks using `tokio::time::sleep`. Compare both.

Useful commands while the benchmark runs:
```sh
# macOS: check memory of your process
ps -o pid,rss,vsz,comm -p <pid>
# rss = actual memory used (KB), vsz = virtual memory (KB)

# Linux
cat /proc/<pid>/status | grep -E 'VmRSS|VmSize|Threads'
```

### Exercise 2: Max threads

Keep spawning `std::thread::spawn` in a loop until it fails. Print the count and the error.

```sh
# Check your system limits
ulimit -u    # max user processes
sysctl kern.num_taskthreads  # macOS max threads per process
```

### Exercise 3: Blocking vs non-blocking syscalls

Write two programs that connect to a TCP server and read data:
1. Using `std::net::TcpStream` (blocking)
2. Using `std::net::TcpStream` with `set_nonblocking(true)`

Trace the syscalls:
```sh
# macOS
sudo dtruss -f ./target/debug/my-binary 2>&1 | grep -E 'read|recvfrom|kevent'

# Linux
strace -e read,recvfrom,epoll_wait ./target/debug/my-binary
```

In the blocking version, you'll see `read()` that takes seconds to return.
In the non-blocking version, you'll see `read()` returning immediately with EAGAIN/EWOULDBLOCK.

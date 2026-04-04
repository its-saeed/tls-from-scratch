# Project 3: HTTP Load Tester (mini wrk/hey)

> **Combines**: Lessons 18-22 (tokio::sync, tokio::net, task-locals, graceful shutdown, tracing).

## What you'll build

A CLI tool that hammers an HTTP endpoint, controls concurrency with a Semaphore, handles Ctrl+C gracefully, propagates request IDs via task-locals, and reports latency percentiles.

## Architecture

```
┌───────────────────────────────────────────────────────────────┐
│  load-tester --url http://example.com --requests 100 -c 10   │
└──────────────────────────┬────────────────────────────────────┘
                           │
                    ┌──────▼──────┐
                    │  CLI Parser │ (clap)
                    │  --url      │
                    │  --requests │
                    │  --concurrency│
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
     ┌────────▼───┐  ┌────▼─────┐  ┌──▼──────────┐
     │ Semaphore  │  │ Shutdown │  │ Result       │
     │ (c permits)│  │ (Notify) │  │ Collector    │
     │            │  │ Ctrl+C   │  │ (Mutex<Vec>) │
     └────────┬───┘  └────┬─────┘  └──┬──────────┘
              │            │           │
              └────────────┼───────────┘
                           │
              ┌────────────▼────────────────┐
              │  for each request 1..N:      │
              │    tokio::spawn {            │
              │      acquire semaphore       │
              │      set task-local req_id   │
              │      select! {               │
              │        shutdown => return    │
              │        send_request => {     │
              │          record latency      │
              │          record status       │
              │        }                     │
              │      }                       │
              │    }                         │
              └────────────┬────────────────┘
                           │
                    ┌──────▼──────┐
                    │  Report     │
                    │  p50, p90   │
                    │  p99, max   │
                    │  throughput │
                    │  status map │
                    └─────────────┘
```

## CLI interface

```
load-tester --url https://example.com/api --requests 1000 --concurrency 50
```

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--url` | `-u` | required | Target URL |
| `--requests` | `-n` | 100 | Total requests to send |
| `--concurrency` | `-c` | 10 | Max concurrent requests |

## Sample output

```
Target: https://example.com/api
Requests: 1000, Concurrency: 50

Running...  [1000/1000] done

Results:
  Total:      1000 requests
  Succeeded:  985
  Failed:     15
  Duration:   2.34s
  Throughput: 427.35 req/s

Latency:
  p50:    4.2ms
  p90:   12.1ms
  p99:   45.3ms
  max:   102.7ms

Status codes:
  200: 985
  503: 15
```

## Key implementation details

### Concurrency with Semaphore

```rust
let sem = Arc::new(Semaphore::new(concurrency));
for i in 0..total_requests {
    let permit = sem.clone().acquire_owned().await?;
    tokio::spawn(async move {
        let result = send_request(&url).await;
        drop(permit); // release slot
        result
    });
}
```

### Latency percentiles

```rust
fn percentile(sorted: &[Duration], p: f64) -> Duration {
    let idx = ((sorted.len() as f64) * p / 100.0) as usize;
    sorted[idx.min(sorted.len() - 1)]
}
```

### Graceful Ctrl+C

```rust
let shutdown = Arc::new(Notify::new());
tokio::spawn({
    let s = shutdown.clone();
    async move {
        tokio::signal::ctrl_c().await.ok();
        s.notify_waiters();
    }
});
```

### Making HTTP requests with raw TCP

Since we don't have `reqwest`, we use raw `TcpStream` with minimal HTTP/1.1:

```rust
async fn http_get(url: &str) -> Result<(u16, Duration)> {
    let start = Instant::now();
    let mut stream = TcpStream::connect((host, port)).await?;
    stream.write_all(format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes()).await?;
    // Read response, parse status code
    Ok((status_code, start.elapsed()))
}
```

## Exercises

### Exercise 1: Basic load tester

Implement the full load tester with Semaphore-based concurrency, latency collection, and percentile reporting. Use raw TCP for HTTP requests.

### Exercise 2: Ctrl+C graceful shutdown

Add `tokio::signal::ctrl_c()` handling. On Ctrl+C, stop spawning new requests, let in-flight ones finish, then print partial results.

### Exercise 3: Live progress reporting

Print a progress line that updates every 100ms showing completed/total requests and current throughput. Use a separate task with `tokio::time::interval`.

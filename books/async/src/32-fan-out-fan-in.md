# Pattern 4: Fan-out / Fan-in

## Real-life analogy: the research team

```
Professor assigns papers to read:
  ┌──────────┐
  │Professor │
  │(dispatch)│
  └────┬─────┘
       │ assigns
  ┌────┼────────────┐
  │    │            │
  ▼    ▼            ▼
┌────┐ ┌────┐ ┌────┐
│TA 1│ │TA 2│ │TA 3│   ← fan-out: work distributed
│read│ │read│ │read│
│ 10 │ │ 10 │ │ 10 │
└──┬─┘ └──┬─┘ └──┬─┘
   │      │      │
   └──────┼──────┘
          │ summaries
          ▼
   ┌──────────┐
   │Professor │              ← fan-in: results collected
   │(collect) │
   │write     │
   │report    │
   └──────────┘

30 papers read in parallel, not sequentially.
Total time: time of slowest TA, not sum of all.
```

## The pattern

One task distributes work to N workers, another collects results:

```
┌──────────────────────────────────────────────────────────┐
│  Fan-out / Fan-in                                        │
│                                                          │
│  ┌──────────┐                                            │
│  │Dispatcher│                                            │
│  └────┬─────┘                                            │
│       │ fan-out                                          │
│  ┌────┼────────────┐                                     │
│  │    │            │                                     │
│  ▼    ▼            ▼                                     │
│ ┌──┐ ┌──┐ ┌──┐ ┌──┐                                     │
│ │W1│ │W2│ │W3│ │W4│  ← N workers (concurrent)           │
│ └┬─┘ └┬─┘ └┬─┘ └┬─┘                                     │
│  │    │    │    │                                        │
│  └────┼────┼────┘                                        │
│       │    │ fan-in                                      │
│  ┌────▼────▼───┐                                         │
│  │  Collector  │                                         │
│  └─────────────┘                                         │
└──────────────────────────────────────────────────────────┘
```

## In Rust: JoinSet

```rust
use tokio::task::JoinSet;

let urls = vec!["https://a.com", "https://b.com", "https://c.com"];
let mut set = JoinSet::new();

// Fan-out: spawn one task per URL
for url in urls {
    set.spawn(async move {
        reqwest::get(url).await
    });
}

// Fan-in: collect results as they complete
while let Some(result) = set.join_next().await {
    match result {
        Ok(Ok(response)) => println!("Got: {}", response.status()),
        Ok(Err(e)) => println!("Request failed: {e}"),
        Err(e) => println!("Task panicked: {e}"),
    }
}
```

## Concurrency limiting

Without a limit, fan-out can overwhelm the target:

```
Fan-out 10,000 HTTP requests simultaneously:
  → target server returns 429 Too Many Requests
  → or your machine runs out of file descriptors

Solution: Semaphore limits concurrent workers
```

```rust
let semaphore = Arc::new(Semaphore::new(50)); // max 50 concurrent

for url in urls {
    let permit = semaphore.clone().acquire_owned().await.unwrap();
    set.spawn(async move {
        let result = fetch(url).await;
        drop(permit); // release the slot
        result
    });
}
```

## When to use

- **Parallel HTTP requests** — fetch 100 URLs, collect results
- **Batch processing** — process 10,000 records, N at a time
- **Map-reduce** — transform items in parallel, aggregate results
- **Health checks** — ping N services, report which are up

## When NOT to use

- **Sequential dependencies** — if step 2 depends on step 1's output, use a pipeline instead
- **Single resource** — if all workers hit the same bottleneck (one database), parallelism doesn't help
- **Ordering matters** — fan-in collects in completion order, not submission order

## Code exercise: Web Crawler

Build a concurrent web crawler:

```
┌──────────┐
│ Seed URL │
│ list     │
└────┬─────┘
     │ fan-out (max 10 concurrent)
┌────┼──────────────────┐
│    │         │        │
▼    ▼         ▼        ▼
┌──┐ ┌──┐ ┌──┐ ┌──┐
│F1│ │F2│ │F3│ │F4│   fetch pages
└┬─┘ └┬─┘ └┬─┘ └┬─┘
 │    │    │    │     fan-in
 └────┼────┼────┘
      ▼
┌──────────┐
│ Results  │
│ - URL    │
│ - status │
│ - size   │
│ - time   │
└──────────┘
```

**Requirements**:
1. Read a list of URLs (from a file or hardcoded)
2. Fetch each URL concurrently (fan-out)
3. Limit concurrency to 10 with a semaphore
4. Collect results (fan-in): URL, HTTP status, response size, latency
5. Print a summary table when all are done

**Starter code**:

```rust
use tokio::task::JoinSet;
use tokio::sync::Semaphore;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct CrawlResult {
    url: String,
    status: u16,
    size: usize,
    latency: Duration,
}

async fn fetch(url: &str) -> CrawlResult {
    let start = Instant::now();
    // TODO: make HTTP request (use tokio::net::TcpStream + raw HTTP, or reqwest)
    // Return CrawlResult
    todo!()
}

#[tokio::main]
async fn main() {
    let urls = vec![
        "http://example.com",
        "http://httpbin.org/get",
        "http://httpbin.org/delay/2",
        // add more
    ];
    let semaphore = Arc::new(Semaphore::new(10));
    let mut set = JoinSet::new();

    for url in urls {
        let sem = semaphore.clone();
        set.spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            fetch(url).await
        });
    }

    // TODO: collect results, print summary table
}
```

**Expected output**:
```
URL                          Status  Size     Latency
─────────────────────────────────────────────────────
http://example.com           200     1256B    120ms
http://httpbin.org/get       200     432B     89ms
http://httpbin.org/delay/2   200     312B     2045ms

Total: 3 URLs, 3 success, 0 failed
```

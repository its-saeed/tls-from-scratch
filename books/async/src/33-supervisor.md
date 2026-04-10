# Pattern 5: Supervisor Tree

## Real-life analogy: the corporate hierarchy

```
CEO (root supervisor)
  │
  ├── VP Engineering (supervisor)
  │     ├── Team Lead Backend (supervisor)
  │     │     ├── Developer 1 (worker) ← quits
  │     │     ├── Developer 2 (worker)     Team Lead hires
  │     │     └── Developer 3 (worker)     a replacement
  │     │
  │     └── Team Lead Frontend (supervisor)
  │           ├── Developer 4 (worker)
  │           └── Developer 5 (worker)
  │
  └── VP Operations (supervisor)
        └── SRE Team Lead (supervisor)
              ├── Oncall 1 (worker)
              └── Oncall 2 (worker) ← quits → Team Lead hires replacement

When a developer quits (task crashes):
  - Their team lead (supervisor) hires a replacement (restart)
  - The VP doesn't even know it happened
  - If the team lead quits → VP restarts the team lead + all their reports
```

## The pattern

A supervisor monitors child tasks and restarts them when they fail:

```
┌──────────────────────────────────────────────────────────┐
│  Supervisor Tree                                         │
│                                                          │
│  ┌───────────┐                                           │
│  │ Supervisor│                                           │
│  │           │                                           │
│  │ children: │                                           │
│  │  [W1, W2] │                                           │
│  └─────┬─────┘                                           │
│        │                                                 │
│    ┌───┴───┐                                             │
│    │       │                                             │
│    ▼       ▼                                             │
│  ┌───┐  ┌───┐                                            │
│  │W1 │  │W2 │  ← W2 panics!                             │
│  └───┘  └───┘                                            │
│              │                                           │
│    Supervisor detects W2 exit                             │
│    Logs the failure                                      │
│    Spawns a NEW W2                                       │
│              │                                           │
│           ┌──▼──┐                                        │
│           │W2'  │  ← replacement, same job               │
│           └─────┘                                        │
│                                                          │
│  Restart strategies:                                     │
│    one-for-one: restart only the crashed child            │
│    one-for-all: restart ALL children (for dependent tasks)│
│    rest-for-one: restart crashed + all after it           │
└──────────────────────────────────────────────────────────┘
```

```rust
async fn supervisor(num_workers: usize) {
    let mut set = JoinSet::new();

    // Spawn initial workers
    for id in 0..num_workers {
        set.spawn(worker(id));
    }

    // Monitor and restart
    loop {
        match set.join_next().await {
            Some(Ok(())) => {
                println!("Worker finished normally");
            }
            Some(Err(e)) => {
                println!("Worker crashed: {e}. Restarting...");
                set.spawn(worker(next_id()));
            }
            None => break, // all workers done, supervisor exits
        }
    }
}

async fn worker(id: usize) {
    loop {
        // do work...
        // might panic!
    }
}
```

## Erlang's "let it crash" philosophy

```
Traditional approach:              Erlang/supervisor approach:
  fn process() {                     fn process() {
    if error {                         // just do the work
      handle_error();                  // if something goes wrong,
      recover();                       // let it crash
      retry();                         // the supervisor will restart us
      log();                         }
      // 50 lines of error handling
    }
  }

  Complex, error-prone.              Simple, robust.
  Every function handles its          Errors bubble up to supervisor.
  own errors.                         Supervisor has ONE job: restart.
```

## When to use

- **Long-running services** — web servers, message brokers, game servers
- **Worker pools** — N workers processing a queue; crashed ones are replaced
- **Unreliable external dependencies** — task talks to flaky API; crashes get restarted
- **Fault isolation** — one bad request crashes one task, not the whole server

## When NOT to use

- **Short-lived programs** — CLI tools, scripts (just exit on error)
- **Errors that should propagate** — if the whole program should stop on error
- **Debugging** — supervisors can mask bugs by restarting endlessly (add restart limits)

## Restart limits

Prevent infinite restart loops:

```rust
let mut restart_count = 0;
let max_restarts = 5;
let reset_interval = Duration::from_secs(60);
let mut window_start = Instant::now();

// In the supervisor loop:
if window_start.elapsed() > reset_interval {
    restart_count = 0;
    window_start = Instant::now();
}
restart_count += 1;
if restart_count > max_restarts {
    eprintln!("Too many restarts in {}s. Giving up.", reset_interval.as_secs());
    break;
}
```

## Code exercise: Resilient Worker Pool

Build a supervisor that manages a pool of workers:

```
┌──────────────┐
│  Supervisor  │
│              │
│  max_restart:│
│  5 per 60s   │
└──────┬───────┘
       │
  ┌────┼────┐
  │    │    │
  ▼    ▼    ▼
┌──┐ ┌──┐ ┌──┐
│W1│ │W2│ │W3│  ← workers process jobs from a shared queue
└──┘ └──┘ └──┘
```

**Requirements**:
1. Supervisor spawns 3 workers
2. Each worker pulls jobs from a shared `mpsc` channel
3. Workers randomly "crash" (panic) on some jobs
4. Supervisor detects the crash and spawns a replacement
5. Replacement worker connects to the same job channel
6. After 5 restarts in 60 seconds, supervisor gives up and exits
7. Remaining jobs are processed by surviving workers

**Starter code**:

```rust
use tokio::task::JoinSet;
use tokio::sync::mpsc;
use std::time::{Duration, Instant};

async fn worker(id: usize, mut jobs: mpsc::Receiver<String>) {
    while let Some(job) = jobs.recv().await {
        // Simulate random crashes
        if rand::random::<f32>() < 0.1 {
            panic!("Worker {id} crashed on job: {job}");
        }
        println!("[Worker {id}] processed: {job}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::main]
async fn main() {
    // TODO: create job channel, spawn workers, supervise
}
```

**Expected output**:
```
[Worker 0] processed: job-1
[Worker 1] processed: job-2
[Worker 2] processed: job-3
Worker 1 crashed: panicked. Restarting... (restart 1/5)
[Worker 3] processed: job-4
[Worker 0] processed: job-5
...
All jobs processed. Total restarts: 3
```

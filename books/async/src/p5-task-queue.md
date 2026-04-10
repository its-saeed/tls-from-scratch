# Capstone: Distributed Task Queue

> **Prerequisites**: All patterns from Course 5. This project combines every pattern into one system.

## What you're building

A job processing system — like a mini Celery, Sidekiq, or Bull. Clients submit jobs via HTTP, workers process them concurrently, a supervisor keeps workers alive, and a dashboard shows live status.

```sh
# Start the system:
cargo run -p async-lessons --bin p5-task-queue

# Submit jobs:
curl -X POST http://127.0.0.1:8080/jobs -d '{"type":"resize","file":"img.png"}'
curl -X POST http://127.0.0.1:8080/jobs -d '{"type":"email","to":"bob@example.com"}'

# Check status:
curl http://127.0.0.1:8080/status
# {"pending": 5, "running": 3, "completed": 42, "failed": 1, "workers": 4}

# Dashboard (in terminal):
# Workers: 4/4 alive
# Queue: 5 pending, 3 running, 42 completed, 1 failed
# Throughput: 14 jobs/sec
# Last failure: "email to bob@example.com — timeout" (2s ago, auto-retried)
```

## Architecture

Every pattern has a role:

```
┌──────────────────────────────────────────────────────────────┐
│  Distributed Task Queue                                      │
│                                                              │
│  ┌───────────────────────────────────────────────┐           │
│  │  API Server (task-per-connection)              │           │
│  │  Accept HTTP requests → enqueue jobs           │           │
│  │  GET /status → query state actor               │           │
│  └─────────────────┬─────────────────────────────┘           │
│                    │ submit job                               │
│                    ▼                                          │
│  ┌───────────────────────────────────────────────┐           │
│  │  State Manager (actor model)                   │           │
│  │  Owns: job queue, status map, counters         │           │
│  │  No locks — processes messages sequentially     │           │
│  │  Messages: Enqueue, Dequeue, UpdateStatus,     │           │
│  │            GetStats                            │           │
│  └─────────────────┬─────────────────────────────┘           │
│                    │ job ready                                │
│                    ▼                                          │
│  ┌───────────────────────────────────────────────┐           │
│  │  Dispatcher (pipeline)                         │           │
│  │  Reads jobs from state manager                 │           │
│  │  Routes to appropriate worker                  │           │
│  │  Handles retries for failed jobs               │           │
│  └─────────────────┬─────────────────────────────┘           │
│                    │ distribute                               │
│              ┌─────┼─────┐                                   │
│              │     │     │                                   │
│              ▼     ▼     ▼                                   │
│  ┌──────────────────────────────────────────────┐            │
│  │  Workers (fan-out/fan-in)                     │            │
│  │  N worker tasks process jobs concurrently     │            │
│  │  CPU-heavy jobs use spawn_blocking            │            │
│  │  Report results back to state manager         │            │
│  └──────────────────────────────────────────────┘            │
│              │     │     │                                   │
│              │ supervised by                                 │
│              ▼                                               │
│  ┌──────────────────────────────────────────────┐            │
│  │  Supervisor (supervisor tree)                 │            │
│  │  Monitors workers via JoinSet                 │            │
│  │  Restarts crashed workers                     │            │
│  │  Max 5 restarts per minute                    │            │
│  └──────────────────────────────────────────────┘            │
│                                                              │
│  ┌───────────────────────────────────────────────┐           │
│  │  Event Bus (pub-sub)                           │           │
│  │  Events: JobEnqueued, JobStarted,              │           │
│  │          JobCompleted, JobFailed,              │           │
│  │          WorkerCrashed, WorkerRestarted        │           │
│  │                                                │           │
│  │  Subscribers:                                  │           │
│  │    Dashboard → live terminal output            │           │
│  │    Logger → write events to file               │           │
│  │    Alerter → Slack notification on failures    │           │
│  └───────────────────────────────────────────────┘           │
└──────────────────────────────────────────────────────────────┘
```

## Pattern map

```
Component        Pattern               Why
─────────────────────────────────────────────────────────
API Server       Task-per-Connection   Each HTTP request = one task
State Manager    Actor                 Owns all mutable state, no locks
Dispatcher       Pipeline              Route jobs through stages
Workers          Fan-out/Fan-in        N workers process concurrently
Supervisor       Supervisor Tree       Restart crashed workers
Events           Event Bus             Dashboard, logger, alerter subscribe
```

## Job lifecycle

```
   Client POSTs job
        │
        ▼
   API task sends Enqueue message to State Actor
        │
        ▼
   State Actor: job status = Pending, pushes to queue
        │
        ▼
   Dispatcher: pulls from queue, assigns to available worker
        │
        ▼
   State Actor: job status = Running
        │
        ▼
   Worker processes the job
        │
   ┌────┴────┐
   │         │
   ▼         ▼
 Success    Failure
   │         │
   ▼         ▼
 State:    State: Failed
 Completed  Dispatcher: retry? (max 3 attempts)
             │
             └── if retries exhausted → Dead Letter Queue
```

## Implementation guide

### Step 1: Define the job types

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Job {
    id: String,
    job_type: String,
    payload: serde_json::Value,
    attempts: u32,
    max_retries: u32,
}

#[derive(Clone, Debug)]
enum JobStatus {
    Pending,
    Running { worker_id: usize },
    Completed { result: String },
    Failed { error: String, retries_left: u32 },
}
```

### Step 2: State Manager Actor

```rust
enum StateMsg {
    Enqueue { job: Job, reply: oneshot::Sender<String> },
    Dequeue { reply: oneshot::Sender<Option<Job>> },
    UpdateStatus { job_id: String, status: JobStatus },
    GetStats { reply: oneshot::Sender<Stats> },
}

async fn state_actor(mut rx: mpsc::Receiver<StateMsg>, event_tx: broadcast::Sender<Event>) {
    let mut queue: VecDeque<Job> = VecDeque::new();
    let mut statuses: HashMap<String, JobStatus> = HashMap::new();

    while let Some(msg) = rx.recv().await {
        match msg {
            StateMsg::Enqueue { job, reply } => {
                let id = job.id.clone();
                statuses.insert(id.clone(), JobStatus::Pending);
                queue.push_back(job);
                event_tx.send(Event::JobEnqueued { id: id.clone() }).ok();
                reply.send(id).ok();
            }
            // TODO: handle other messages
            _ => todo!(),
        }
    }
}
```

### Step 3: Worker + Supervisor

```rust
async fn worker(id: usize, state: StateHandle, event_tx: broadcast::Sender<Event>) {
    loop {
        let job = state.dequeue().await;
        match job {
            Some(job) => {
                event_tx.send(Event::JobStarted { id: job.id.clone(), worker: id }).ok();
                match process_job(&job).await {
                    Ok(result) => {
                        state.update_status(job.id, JobStatus::Completed { result }).await;
                    }
                    Err(e) => {
                        state.update_status(job.id, JobStatus::Failed {
                            error: e.to_string(),
                            retries_left: job.max_retries - job.attempts,
                        }).await;
                    }
                }
            }
            None => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
}
```

### Step 4: API Server

```rust
// Minimal HTTP server (raw TCP, parse HTTP manually)
async fn handle_http(stream: TcpStream, state: StateHandle) {
    // Parse: POST /jobs → enqueue
    // Parse: GET /status → get stats
    // Return JSON responses
}
```

### Step 5: Event Bus + Dashboard

```rust
tokio::spawn(async move {
    let mut rx = event_tx.subscribe();
    let mut stats = Stats::default();

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(Event::JobCompleted { .. }) => stats.completed += 1,
                    Ok(Event::JobFailed { .. }) => stats.failed += 1,
                    // ...
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                print_dashboard(&stats);
            }
        }
    }
});
```

## Exercises

### Exercise 1: Basic queue

Implement State Actor + Workers + Dispatcher. Submit 100 jobs, process them with 4 workers. Print completion count.

### Exercise 2: Add supervisor

Workers randomly crash (1% chance). Supervisor restarts them. All 100 jobs should still complete.

### Exercise 3: Add HTTP API

Accept jobs via `POST /jobs`, return status via `GET /status`. Test with `curl`.

### Exercise 4: Add event bus + dashboard

Broadcast events, subscribe with a dashboard that prints live stats every second.

### Exercise 5: Retry with backoff

Failed jobs are retried up to 3 times with exponential backoff (1s, 2s, 4s). After 3 failures → dead letter queue.

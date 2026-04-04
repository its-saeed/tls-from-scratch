# Lesson 20: Task-Local Storage — tokio::task::LocalKey

> **Prerequisites**: Lesson 4 (tasks), Lesson 14 (work-stealing).

## Real-life analogy: name tags at a conference

```
┌───────────────────────────────────────────────────────────────┐
│  Conference Venue                                             │
│                                                               │
│  Thread-locals = room assignments:                            │
│    Room A has whiteboard "Project X"                          │
│    Room B has whiteboard "Project Y"                          │
│    If attendee MOVES rooms → sees wrong whiteboard!           │
│                                                               │
│  Task-locals = name tags on each person:                      │
│    Alice wears "Request #42"                                  │
│    Bob wears "Request #99"                                    │
│    No matter which room they walk into,                       │
│    their name tag follows them.                               │
│                                                               │
│  In async: tasks migrate between OS threads (rooms).          │
│  Thread-locals follow the room. Task-locals follow the task.  │
└───────────────────────────────────────────────────────────────┘
```

## The problem with thread-locals in async

```
Thread 1              Thread 2
┌────────────┐        ┌────────────┐
│ TLS: "abc" │        │ TLS: "xyz" │
│            │        │            │
│ Task A     │        │            │
│  reads TLS │        │            │
│  → "abc"   │        │            │
│  .await    │───────►│ Task A     │   (work-stealing moved it!)
│            │        │  reads TLS │
│            │        │  → "xyz"   │   WRONG! Expected "abc"
└────────────┘        └────────────┘
```

In a multi-thread tokio runtime, a task can resume on **any** worker thread after `.await`. Thread-local values belong to the thread, not the task.

## task_local! to the rescue

```rust
tokio::task_local! {
    static REQUEST_ID: String;
}

async fn handle_request(id: String) {
    REQUEST_ID.scope(id, async {
        // Value is set for the duration of this future
        do_work().await;        // survives .await
        do_more_work().await;   // still correct
    }).await;
}

async fn do_work() {
    REQUEST_ID.with(|id| {
        println!("Processing request: {id}");
    });
}
```

## Scoping rules

```
REQUEST_ID.scope("req-42", async {
    │
    │  REQUEST_ID.with(|id| ...)    → Ok("req-42")
    │
    │  REQUEST_ID.scope("req-99", async {   // nested: shadows outer
    │      │
    │      │  REQUEST_ID.with(|id| ...)  → Ok("req-99")
    │      │
    │  }).await;
    │
    │  REQUEST_ID.with(|id| ...)    → Ok("req-42")  (restored)
    │
}).await;

// Outside any scope:
REQUEST_ID.with(|id| ...)           → PANIC!
REQUEST_ID.try_with(|id| ...)       → Err(AccessError)
```

## Key limitations

| Limitation | Why |
|-----------|-----|
| No `set()` method | Values are immutable within a scope |
| Not inherited by child tasks | `tokio::spawn` creates a fresh context |
| Must use `.scope()` | Cannot set from outside an async context |
| One value per scope | Nesting shadows, does not merge |

### Child tasks do NOT inherit

```rust
REQUEST_ID.scope("req-42".into(), async {
    tokio::spawn(async {
        // PANIC! REQUEST_ID is not set here.
        REQUEST_ID.with(|id| println!("{id}"));
    }).await;
}).await;
```

You must explicitly pass values to child tasks via `.scope()` or function arguments.

## Common patterns

### Request ID propagation

```rust
task_local! { static REQ_ID: u64; }

async fn middleware(req_id: u64, handler: impl Future<Output = ()>) {
    REQ_ID.scope(req_id, handler).await;
}

async fn log_something() {
    REQ_ID.with(|id| println!("[req={id}] doing something"));
}
```

## Exercises

### Exercise 1: Task-local survives .await

Define a `REQUEST_ID` task-local. Set it with `.scope()`, call an async function that reads it after an `.await` point. Verify the value is correct.

### Exercise 2: Isolation between tasks

Spawn 10 tasks, each with a unique task-local value. Each task prints its value after a `yield_now()`. Verify no task sees another's value.

### Exercise 3: Child task does NOT inherit

Demonstrate that `tokio::spawn` inside a `.scope()` does NOT have access to the parent's task-local. Use `try_with` to show it returns `Err`.

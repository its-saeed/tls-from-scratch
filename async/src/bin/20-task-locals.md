# Lesson 19: Task-Local Storage — tokio::task::LocalKey

## What you'll learn

- Why thread-locals break in async code (tasks migrate between threads)
- How `task_local!` provides per-task storage
- Scoping task-local values with `.scope()`
- Practical patterns: request IDs, trace context, database transactions

## Key concepts

### The problem with thread-locals

In a multi-thread runtime, a task may resume on a different OS thread after an `.await`. Thread-local storage follows the thread, not the task, so values appear to change randomly.

### task_local! macro

```rust
tokio::task_local! {
    static REQUEST_ID: String;
}

async fn handle_request(id: String) {
    REQUEST_ID.scope(id, async {
        // anywhere inside this scope:
        REQUEST_ID.with(|id| println!("request: {id}"));
        do_work().await; // task-local survives .await
    }).await;
}
```

### Scoping rules

- `LocalKey::scope(value, future)` — sets the value for the duration of the future
- `LocalKey::with(|v| ...)` — access the current value (panics if not in scope)
- `LocalKey::try_with(|v| ...)` — returns `Err` if not in scope

### Common patterns

- **Request ID propagation** — set once at request entry, read in all handlers
- **Trace context** — propagate span context through async call chains
- **Database transaction** — pass transaction handle without threading through every function

### Limitations

- Cannot be set from outside `.scope()` — no `set()` method
- The value is not shared between parent and spawned child tasks
- Each `.scope()` creates a new binding; nesting shadows the outer one

## Exercises

1. Define a `REQUEST_ID` task-local and verify it survives across `.await` points
2. Spawn 10 tasks each with a unique task-local value; print from inside each to confirm isolation
3. Demonstrate that `std::thread_local!` gives wrong results in multi-thread Tokio
4. Build a middleware-like pattern that sets a request ID task-local before calling a handler
5. Show that a child task spawned with `tokio::spawn` does NOT inherit the parent's task-local

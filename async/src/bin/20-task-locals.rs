// Lesson 20: Task-Local Storage
//
// tokio::task::LocalKey — per-task context that survives .await.
// Run with: cargo run -p async-lessons --bin 20-task-locals -- <command>
//
// Commands:
//   scope-demo       Show task-local surviving across .await points
//   isolation        Show task-locals are isolated between tasks
//   no-inherit       Show child tasks do NOT inherit parent's task-local
//   all              Run all demos

use clap::{Parser, Subcommand};

tokio::task_local! {
    static REQUEST_ID: String;
}

#[derive(Parser)]
#[command(name = "task-locals", about = "Lesson 20: Task-local storage")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Demo task-local scope surviving .await points
    ScopeDemo,
    /// Demo isolation between concurrent tasks
    Isolation,
    /// Demo that spawned child tasks do NOT inherit task-locals
    NoInherit,
    /// Run all demos
    All,
}

async fn demo_scope() {
    println!("=== Task-Local Scope Demo ===");
    println!("A task-local value follows the task across .await points.\n");

    REQUEST_ID.scope("req-42".to_string(), async {
        // Before .await
        REQUEST_ID.with(|id| println!("  Before .await: REQUEST_ID = {id}"));

        // Yield to runtime — task might move to another thread
        tokio::task::yield_now().await;

        // After .await — value is still correct
        REQUEST_ID.with(|id| println!("  After .await:  REQUEST_ID = {id}"));

        // Call a nested async function
        inner_work().await;
    }).await;

    println!();
    println!("Takeaway: task-locals follow the task, not the thread.");
    println!("They survive any number of .await points within the scope.");
}

async fn inner_work() {
    tokio::task::yield_now().await;
    REQUEST_ID.with(|id| {
        println!("  In inner_work: REQUEST_ID = {id}");
    });
}

async fn demo_isolation() {
    println!("=== Task-Local Isolation Demo ===");
    println!("Each task has its own independent task-local value.\n");

    let mut handles = vec![];
    for i in 0..5 {
        handles.push(tokio::spawn(async move {
            REQUEST_ID.scope(format!("task-{i}"), async move {
                // Yield to encourage thread migration
                tokio::task::yield_now().await;

                REQUEST_ID.with(|id| {
                    let thread = std::thread::current().id();
                    println!("  Task {i} on {thread:?}: REQUEST_ID = {id}");
                    assert_eq!(id, &format!("task-{i}"), "Task saw wrong value!");
                });
            }).await;
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    println!();
    println!("Takeaway: each task's scope is independent.");
    println!("Even if tasks share a thread, they see their own values.");
}

async fn demo_no_inherit() {
    println!("=== No Inheritance Demo ===");
    println!("Child tasks spawned with tokio::spawn do NOT get parent's task-local.\n");

    REQUEST_ID.scope("parent-req-1".to_string(), async {
        REQUEST_ID.with(|id| println!("  Parent: REQUEST_ID = {id}"));

        let child = tokio::spawn(async {
            // try_with returns Err because this is a new task context
            let result = REQUEST_ID.try_with(|id| id.clone());
            match result {
                Ok(ref id) => println!("  Child: found REQUEST_ID = {id} (unexpected)"),
                Err(_) => println!("  Child: REQUEST_ID not set! (expected)"),
            }
            result.is_err()
        });

        let was_missing = child.await.unwrap();
        assert!(was_missing, "Child should NOT inherit parent task-local");

        // To pass context to a child, set it explicitly:
        let parent_id = REQUEST_ID.with(|id| id.clone());
        let child2 = tokio::spawn(async move {
            REQUEST_ID.scope(parent_id, async {
                REQUEST_ID.with(|id| {
                    println!("  Child2 (explicit): REQUEST_ID = {id}");
                });
            }).await;
        });
        child2.await.unwrap();
    }).await;

    println!();
    println!("Takeaway: tokio::spawn creates a fresh task — no task-local inheritance.");
    println!("You must explicitly pass values into child tasks via .scope() or args.");
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::ScopeDemo => demo_scope().await,
        Command::Isolation => demo_isolation().await,
        Command::NoInherit => demo_no_inherit().await,
        Command::All => {
            demo_scope().await;
            println!("\n");
            demo_isolation().await;
            println!("\n");
            demo_no_inherit().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn task_local_survives_await() {
        REQUEST_ID.scope("test-req".to_string(), async {
            tokio::task::yield_now().await;
            REQUEST_ID.with(|id| assert_eq!(id, "test-req"));
        }).await;
    }

    #[tokio::test]
    async fn child_task_does_not_inherit() {
        REQUEST_ID.scope("parent".to_string(), async {
            let handle = tokio::spawn(async {
                REQUEST_ID.try_with(|_| ()).is_err()
            });
            assert!(handle.await.unwrap(), "Child should not inherit task-local");
        }).await;
    }
}

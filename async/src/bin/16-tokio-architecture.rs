// Lesson 16: Tokio Architecture
//
// Explore runtime builder, current_thread vs multi_thread, and the driver system.
// Run with: cargo run -p async-lessons --bin 16-tokio-architecture -- <command>
//
// Commands:
//   current-thread     Run tasks on a single-threaded runtime
//   multi-thread       Run tasks on a multi-threaded runtime, show thread distribution
//   missing-driver     Show what happens without enable_time()
//   handle             Spawn from outside the runtime using Handle
//   all                Run all demos

use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "tokio-architecture", about = "Lesson 16: Tokio Architecture")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run tasks on current_thread runtime
    CurrentThread,
    /// Run tasks on multi_thread, show thread distribution
    MultiThread,
    /// Show what happens without enable_time()
    MissingDriver,
    /// Spawn from outside the runtime via Handle
    Handle,
    /// Run all demos
    All,
}

fn demo_current_thread() {
    println!("=== current_thread Runtime ===");
    println!("All tasks run on one thread. No Send requirement.");
    println!();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let thread_ids: Arc<Mutex<Vec<thread::ThreadId>>> = Arc::new(Mutex::new(Vec::new()));

    rt.block_on(async {
        let mut handles = vec![];
        for i in 0..10 {
            let ids = thread_ids.clone();
            handles.push(tokio::task::spawn_local(async move {
                ids.lock().unwrap().push(thread::current().id());
                i * i
            }));
        }
        for h in handles {
            let _ = h.await;
        }
    });

    let ids = thread_ids.lock().unwrap();
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    println!("  Tasks ran: {}", ids.len());
    println!("  Unique threads: {} (should be 1)", unique.len());
    println!();
    println!("Takeaway: current_thread = one thread, all tasks interleave.");
    println!("Use spawn_local (not spawn) — no Send required.");
}

fn demo_multi_thread() {
    println!("=== multi_thread Runtime ===");
    println!("Tasks distributed across worker threads via work-stealing.");
    println!();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    let thread_ids: Arc<Mutex<Vec<thread::ThreadId>>> = Arc::new(Mutex::new(Vec::new()));

    rt.block_on(async {
        let mut handles = vec![];
        for _ in 0..100 {
            let ids = thread_ids.clone();
            handles.push(tokio::spawn(async move {
                // Small work to encourage distribution
                tokio::task::yield_now().await;
                ids.lock().unwrap().push(thread::current().id());
            }));
        }
        for h in handles {
            let _ = h.await;
        }
    });

    let ids = thread_ids.lock().unwrap();
    let mut counts: HashMap<thread::ThreadId, usize> = HashMap::new();
    for id in ids.iter() {
        *counts.entry(*id).or_default() += 1;
    }

    println!("  Tasks ran: {}", ids.len());
    println!("  Thread distribution:");
    for (id, count) in &counts {
        println!("    {:?}: {} tasks", id, count);
    }
    println!();
    println!("Takeaway: work-stealing balances load across {} threads.", counts.len());
}

fn demo_missing_driver() {
    println!("=== Missing Driver Demo ===");
    println!("What happens when you forget enable_time()?");
    println!();

    // Runtime with I/O but NO timer
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()  // no enable_time()!
        .build()
        .unwrap();

    println!("  Created runtime with enable_io() only (no enable_time()).");
    println!("  Trying tokio::time::sleep...");
    println!();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rt.block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        });
    }));

    match result {
        Ok(()) => println!("  Unexpectedly succeeded (timer might be auto-enabled)"),
        Err(e) => {
            let msg = e.downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap_or("unknown panic");
            println!("  PANICKED: {msg}");
        }
    }

    println!();
    println!("Takeaway: enable_time() creates the timer driver.");
    println!("Without it, tokio::time::sleep panics. Use enable_all() to be safe.");
}

fn demo_handle() {
    println!("=== Handle: Spawn from Outside ===");
    println!("Using Handle to spawn tasks from a non-runtime thread.");
    println!();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    let handle = rt.handle().clone();
    let result = Arc::new(Mutex::new(None::<String>));
    let r = result.clone();

    // Spawn from a std thread
    let join = std::thread::spawn(move || {
        println!("  [std thread] spawning task via handle...");
        let h = handle.spawn(async move {
            let tid = thread::current().id();
            format!("hello from tokio worker {:?}", tid)
        });
        // Block on the handle to get the result
        handle.block_on(async {
            let msg = h.await.unwrap();
            *r.lock().unwrap() = Some(msg);
        });
    });

    join.join().unwrap();

    let msg = result.lock().unwrap().take().unwrap();
    println!("  [main] got: {msg}");
    println!();
    println!("Takeaway: Handle is Send + Sync + Clone.");
    println!("You can spawn tokio tasks from any thread.");

    rt.shutdown_timeout(std::time::Duration::from_secs(1));
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::CurrentThread => demo_current_thread(),
        Command::MultiThread => demo_multi_thread(),
        Command::MissingDriver => demo_missing_driver(),
        Command::Handle => demo_handle(),
        Command::All => {
            demo_current_thread();
            println!("\n");
            demo_multi_thread();
            println!("\n");
            demo_missing_driver();
            println!("\n");
            demo_handle();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn current_thread_runs() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let val = rt.block_on(async { 42 });
        assert_eq!(val, 42);
    }

    #[test]
    fn multi_thread_runs() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let val = rt.block_on(async {
            tokio::spawn(async { 42 }).await.unwrap()
        });
        assert_eq!(val, 42);
    }
}

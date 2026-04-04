// Lesson 21: Graceful Shutdown
//
// CancellationToken patterns, signal handling, and the drain pattern.
// Run with: cargo run -p async-lessons --bin 21-graceful-shutdown -- <command>
//
// Commands:
//   signal-demo      Demo catching Ctrl+C with tokio::signal
//   shutdown-notify  Demo Notify-based shutdown of multiple workers
//   drain-demo       Demo drain pattern with in-flight counter
//   all              Run non-blocking demos

use clap::{Parser, Subcommand};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

#[derive(Parser)]
#[command(name = "graceful-shutdown", about = "Lesson 21: Graceful shutdown patterns")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Demo catching Ctrl+C (press Ctrl+C to trigger)
    SignalDemo,
    /// Demo Notify-based shutdown of 5 worker tasks
    ShutdownNotify,
    /// Demo drain pattern: wait for in-flight tasks to complete
    DrainDemo,
    /// Run automated demos (no Ctrl+C needed)
    All,
}

async fn demo_signal() {
    println!("=== Signal Handling Demo ===");
    println!("Press Ctrl+C to trigger shutdown.\n");

    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();

    // Background work
    let worker = tokio::spawn(async move {
        loop {
            c.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await.unwrap();
    println!("\n  Ctrl+C received!");
    worker.abort();
    let count = counter.load(Ordering::SeqCst);
    println!("  Worker completed {count} iterations before shutdown.");
    println!();
    println!("Takeaway: tokio::signal::ctrl_c() is an async future.");
    println!("Combine with select! to race shutdown against server work.");
}

async fn demo_shutdown_notify() {
    println!("=== Notify-Based Shutdown Demo ===");
    println!("5 workers, each doing periodic work. Shutdown after 500ms.\n");

    let shutdown = Arc::new(Notify::new());
    let mut handles = vec![];

    for i in 0..5 {
        let shutdown = shutdown.clone();
        handles.push(tokio::spawn(async move {
            let mut iterations = 0u32;
            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        println!("  Worker {i}: shutting down after {iterations} iterations");
                        return iterations;
                    }
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {
                        iterations += 1;
                    }
                }
            }
        }));
    }

    // Let workers run for a bit
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Signal shutdown
    println!("  --- Sending shutdown signal ---");
    shutdown.notify_waiters(); // wake ALL waiting tasks

    let mut total = 0u32;
    for h in handles {
        total += h.await.unwrap();
    }
    println!("  Total iterations across all workers: {total}");
    println!();
    println!("Takeaway: Notify::notify_waiters() wakes all tasks waiting on notified().");
    println!("Each worker uses select! to check for shutdown alongside its main work.");
}

async fn demo_drain() {
    println!("=== Drain Pattern Demo ===");
    println!("Track in-flight tasks, wait for all to complete on shutdown.\n");

    let shutdown = Arc::new(Notify::new());
    let in_flight = Arc::new(AtomicUsize::new(0));
    let all_done = Arc::new(Notify::new());

    let mut handles = vec![];

    // Spawn "requests" that take varying amounts of time
    for i in 0..10 {
        let shutdown = shutdown.clone();
        let in_flight = in_flight.clone();
        let all_done = all_done.clone();

        handles.push(tokio::spawn(async move {
            in_flight.fetch_add(1, Ordering::SeqCst);

            // Simulate work — some tasks are slow
            let work_time = Duration::from_millis(50 + (i * 30) as u64);

            tokio::select! {
                _ = shutdown.notified() => {
                    // Even on shutdown, finish current work (graceful)
                    println!("  Task {i}: got shutdown, finishing current work...");
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                _ = tokio::time::sleep(work_time) => {
                    println!("  Task {i}: completed normally");
                }
            }

            let remaining = in_flight.fetch_sub(1, Ordering::SeqCst) - 1;
            if remaining == 0 {
                all_done.notify_one();
            }
        }));
    }

    // Trigger shutdown after 200ms
    tokio::time::sleep(Duration::from_millis(200)).await;
    println!("  --- Triggering shutdown ---");
    let active = in_flight.load(Ordering::SeqCst);
    println!("  In-flight tasks: {active}");
    shutdown.notify_waiters();

    // Wait for drain with a hard timeout
    let drain_result = tokio::time::timeout(
        Duration::from_secs(5),
        all_done.notified(),
    ).await;

    match drain_result {
        Ok(()) => println!("  All tasks drained successfully."),
        Err(_) => println!("  TIMEOUT: forced shutdown after 5 seconds."),
    }

    for h in handles {
        let _ = h.await;
    }

    println!();
    println!("Takeaway: the drain pattern has three phases:");
    println!("  1. Signal shutdown (notify_waiters)");
    println!("  2. Wait for in-flight tasks to finish");
    println!("  3. Hard timeout as a safety net");
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::SignalDemo => demo_signal().await,
        Command::ShutdownNotify => demo_shutdown_notify().await,
        Command::DrainDemo => demo_drain().await,
        Command::All => {
            demo_shutdown_notify().await;
            println!("\n");
            demo_drain().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn notify_wakes_all_workers() {
        let shutdown = Arc::new(Notify::new());
        let count = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..5 {
            let shutdown = shutdown.clone();
            let count = count.clone();
            handles.push(tokio::spawn(async move {
                shutdown.notified().await;
                count.fetch_add(1, Ordering::SeqCst);
            }));
        }

        // Give tasks time to start waiting
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(10)).await;

        shutdown.notify_waiters();

        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(count.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn drain_completes_before_timeout() {
        let in_flight = Arc::new(AtomicUsize::new(1));
        let all_done = Arc::new(Notify::new());

        let in_f = in_flight.clone();
        let done = all_done.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if in_f.fetch_sub(1, Ordering::SeqCst) == 1 {
                done.notify_one();
            }
        });

        let result = tokio::time::timeout(
            Duration::from_secs(2),
            all_done.notified(),
        ).await;
        assert!(result.is_ok(), "Drain should complete before timeout");
    }
}

// Project 4: Async Job Queue
//
// Combines lessons 23-28: backpressure, cancellation, bridging, streams, pooling, testing.
// Run with: cargo run -p async-lessons --bin p4-job-queue -- <command>

use clap::{Parser, Subcommand};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};

#[derive(Parser)]
#[command(name = "job-queue", about = "Project 4: Async Job Queue")]
struct Cli { #[command(subcommand)] command: Command }

#[derive(Subcommand)]
enum Command {
    /// Run the job queue demo
    Run,
    /// Run the architecture description
    Architecture,
    All,
}

/// A job to be processed.
struct Job {
    id: usize,
    payload: String,
}

/// Result of a processed job.
#[derive(Debug)]
struct JobResult {
    job_id: usize,
    output: String,
}

fn demo_architecture() {
    println!("=== Async Job Queue Architecture ===");
    println!();
    println!("  Producer → [bounded channel] → Dispatcher → [worker pool] → Results");
    println!();
    println!("  ┌──────────┐     ┌──────────────┐     ┌──────────────┐");
    println!("  │ Producer │────►│ Job Channel  │────►│ Dispatcher   │");
    println!("  │          │     │ (bounded: 10)│     │              │");
    println!("  └──────────┘     └──────────────┘     └──────┬───────┘");
    println!("                                                │");
    println!("                          ┌─────────────────────┼─────────────────┐");
    println!("                          ▼                     ▼                 ▼");
    println!("                   ┌──────────┐          ┌──────────┐      ┌──────────┐");
    println!("                   │ Worker 0 │          │ Worker 1 │      │ Worker 2 │");
    println!("                   │ (semaphore│          │          │      │          │");
    println!("                   │  limited) │          │          │      │          │");
    println!("                   └─────┬────┘          └─────┬────┘      └─────┬────┘");
    println!("                         │                     │                 │");
    println!("                         └─────────┬───────────┴─────────────────┘");
    println!("                                   ▼");
    println!("                            ┌──────────────┐");
    println!("                            │ Result Chan  │");
    println!("                            └──────────────┘");
    println!();
    println!("  Key patterns:");
    println!("    - Bounded channel: backpressure (Lesson 23)");
    println!("    - Semaphore: limit concurrent workers (Lesson 23)");
    println!("    - spawn_blocking: CPU-heavy jobs (Lesson 25)");
    println!("    - Graceful shutdown: drain in-flight (Lesson 21)");
    println!("    - Cancellation-safe recv (Lesson 24)");
}

fn demo_run() {
    println!("=== Job Queue Demo ===");
    println!();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let (job_tx, mut job_rx) = mpsc::channel::<Job>(10);
        let (result_tx, mut result_rx) = mpsc::channel::<JobResult>(10);
        let semaphore = Arc::new(Semaphore::new(3)); // max 3 concurrent workers

        // Producer: submit 10 jobs
        let producer = tokio::spawn(async move {
            for i in 0..10 {
                println!("  [producer] submitting job {i}");
                job_tx.send(Job {
                    id: i,
                    payload: format!("data-{i}"),
                }).await.unwrap();
            }
            println!("  [producer] all jobs submitted");
        });

        // Dispatcher: take jobs, spawn workers (limited by semaphore)
        let dispatcher = tokio::spawn(async move {
            while let Some(job) = job_rx.recv().await {
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let tx = result_tx.clone();
                tokio::spawn(async move {
                    let _permit = permit; // hold until done

                    // Simulate CPU work with spawn_blocking
                    let output = tokio::task::spawn_blocking(move || {
                        std::thread::sleep(Duration::from_millis(100));
                        format!("processed-{}", job.payload)
                    }).await.unwrap();

                    println!("  [worker] completed job {}", job.id);
                    let _ = tx.send(JobResult { job_id: job.id, output }).await;
                });
            }
        });

        // Collector: gather results
        let collector = tokio::spawn(async move {
            let mut results = vec![];
            // Wait for all 10 results
            for _ in 0..10 {
                if let Some(result) = result_rx.recv().await {
                    results.push(result);
                }
            }
            results
        });

        producer.await.unwrap();
        drop(dispatcher); // stop accepting new jobs

        let results = collector.await.unwrap();
        println!();
        println!("  Collected {} results:", results.len());
        for r in &results {
            println!("    job {}: {}", r.job_id, r.output);
        }
        println!();
        println!("Takeaway: bounded channels provide backpressure,");
        println!("semaphore limits concurrency, spawn_blocking handles CPU work.");
    });
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Run => demo_run(),
        Command::Architecture => demo_architecture(),
        Command::All => { demo_architecture(); println!("\n"); demo_run(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn job_queue_processes_all() {
        let (tx, mut rx) = mpsc::channel::<Job>(5);
        let (result_tx, mut result_rx) = mpsc::channel::<JobResult>(5);

        // Submit 3 jobs
        for i in 0..3 {
            tx.send(Job { id: i, payload: format!("test-{i}") }).await.unwrap();
        }
        drop(tx);

        // Process
        while let Some(job) = rx.recv().await {
            let tx = result_tx.clone();
            tokio::spawn(async move {
                let _ = tx.send(JobResult {
                    job_id: job.id,
                    output: format!("done-{}", job.payload),
                }).await;
            });
        }
        drop(result_tx);

        // Collect
        let mut count = 0;
        while let Some(_) = result_rx.recv().await { count += 1; }
        assert_eq!(count, 3);
    }
}

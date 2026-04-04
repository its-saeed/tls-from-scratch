// Project 3: HTTP Load Tester (mini wrk/hey)
//
// Combines lessons 18-22: Semaphore, tokio::net, task-locals, graceful shutdown.
// Run with: cargo run -p async-lessons --bin p3-load-tester -- <command>
//
// Commands:
//   run --url <URL> -n <count> -c <concurrency>    Run the load test
//   demo                                            Run against a built-in echo server
//   report-demo                                     Show sample percentile calculation

use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, Semaphore};

tokio::task_local! {
    static REQUEST_ID: u64;
}

#[derive(Parser)]
#[command(name = "load-tester", about = "Project 3: HTTP Load Tester")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run load test against a URL (host:port format)
    Run {
        /// Target address (host:port)
        #[arg(long, short = 'u')]
        url: String,
        /// Path to request
        #[arg(long, default_value = "/")]
        path: String,
        /// Total number of requests
        #[arg(long, short = 'n', default_value = "100")]
        requests: usize,
        /// Max concurrent requests
        #[arg(long, short = 'c', default_value = "10")]
        concurrency: usize,
    },
    /// Run a demo against a built-in test server
    Demo,
    /// Show percentile calculation on sample data
    ReportDemo,
}

#[derive(Debug, Clone)]
struct RequestResult {
    status: u16,
    latency: Duration,
    error: Option<String>,
}

async fn send_request(addr: &str, path: &str) -> RequestResult {
    let start = Instant::now();
    match send_request_inner(addr, path).await {
        Ok(status) => RequestResult {
            status,
            latency: start.elapsed(),
            error: None,
        },
        Err(e) => RequestResult {
            status: 0,
            latency: start.elapsed(),
            error: Some(e.to_string()),
        },
    }
}

async fn send_request_inner(
    addr: &str,
    path: &str,
) -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
    let mut stream = TcpStream::connect(addr).await?;

    // Extract host from addr
    let host = addr.split(':').next().unwrap_or(addr);
    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await?;
    stream.shutdown().await.ok();

    let mut response = vec![0u8; 4096];
    let n = stream.read(&mut response).await?;
    let response_str = String::from_utf8_lossy(&response[..n]);

    // Parse status code from "HTTP/1.1 200 OK"
    let status = response_str
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(0);

    Ok(status)
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((sorted.len() as f64) * p / 100.0) as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn print_report(results: &[RequestResult], total_duration: Duration) {
    let total = results.len();
    let succeeded = results.iter().filter(|r| r.error.is_none() && r.status > 0).count();
    let failed = total - succeeded;

    let mut latencies: Vec<Duration> = results
        .iter()
        .filter(|r| r.error.is_none())
        .map(|r| r.latency)
        .collect();
    latencies.sort();

    let throughput = if total_duration.as_secs_f64() > 0.0 {
        total as f64 / total_duration.as_secs_f64()
    } else {
        0.0
    };

    println!("\nResults:");
    println!("  Total:      {total} requests");
    println!("  Succeeded:  {succeeded}");
    println!("  Failed:     {failed}");
    println!("  Duration:   {:.2?}", total_duration);
    println!("  Throughput: {throughput:.2} req/s");

    if !latencies.is_empty() {
        println!("\nLatency:");
        println!("  p50:  {:>8.2?}", percentile(&latencies, 50.0));
        println!("  p90:  {:>8.2?}", percentile(&latencies, 90.0));
        println!("  p99:  {:>8.2?}", percentile(&latencies, 99.0));
        println!("  max:  {:>8.2?}", latencies.last().unwrap());
    }

    // Status code distribution
    let mut status_counts: HashMap<u16, usize> = HashMap::new();
    for r in results {
        if r.status > 0 {
            *status_counts.entry(r.status).or_default() += 1;
        }
    }
    if !status_counts.is_empty() {
        println!("\nStatus codes:");
        let mut codes: Vec<_> = status_counts.into_iter().collect();
        codes.sort_by_key(|(code, _)| *code);
        for (code, count) in codes {
            println!("  {code}: {count}");
        }
    }
}

async fn run_load_test(addr: String, path: String, total: usize, concurrency: usize) {
    println!("Target: {addr}{path}");
    println!("Requests: {total}, Concurrency: {concurrency}\n");

    let sem = Arc::new(Semaphore::new(concurrency));
    let results: Arc<Mutex<Vec<RequestResult>>> = Arc::new(Mutex::new(Vec::with_capacity(total)));
    let completed = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(Notify::new());
    let cancelled = Arc::new(AtomicBool::new(false));

    // Ctrl+C handler
    let s = shutdown.clone();
    let c = cancelled.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\n  Ctrl+C received, stopping...");
        c.store(true, Ordering::SeqCst);
        s.notify_waiters();
    });

    let start = Instant::now();
    let mut handles = vec![];

    for i in 0..total {
        if cancelled.load(Ordering::SeqCst) {
            break;
        }

        let permit = sem.clone().acquire_owned().await.unwrap();
        let addr = addr.clone();
        let path = path.clone();
        let results = results.clone();
        let completed = completed.clone();
        let shutdown = shutdown.clone();

        handles.push(tokio::spawn(async move {
            let result = REQUEST_ID.scope(i as u64, async {
                tokio::select! {
                    _ = shutdown.notified() => {
                        RequestResult {
                            status: 0,
                            latency: Duration::ZERO,
                            error: Some("cancelled".into()),
                        }
                    }
                    result = send_request(&addr, &path) => result
                }
            }).await;

            drop(permit);
            let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
            if done % 20 == 0 || done == total {
                print!("\r  Progress: [{done}/{total}]");
            }
            results.lock().unwrap().push(result);
        }));
    }

    for h in handles {
        let _ = h.await;
    }

    let total_duration = start.elapsed();
    println!();

    let results = results.lock().unwrap();
    print_report(&results, total_duration);
}

async fn demo_with_builtin_server() {
    println!("=== Load Tester Demo ===");
    println!("Starting a built-in HTTP server, then load testing it.\n");

    // Start a simple HTTP server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    println!("  Test server on {addr}");

    tokio::spawn(async move {
        loop {
            if let Ok((mut stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    let _ = stream.read(&mut buf).await;
                    // Simulate variable latency
                    let delay = (std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .subsec_nanos()
                        % 10) as u64;
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
        }
    });

    // Give server a moment to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    run_load_test(addr.to_string(), "/".to_string(), 50, 10).await;

    println!("\nTakeaway: Semaphore limits in-flight requests.");
    println!("Ctrl+C triggers graceful shutdown via Notify.");
    println!("Task-locals carry request IDs through the async call chain.");
}

fn demo_report() {
    println!("=== Percentile Report Demo ===");
    println!("Showing how percentile calculation works on sample data.\n");

    let sample_latencies: Vec<Duration> = (1..=100)
        .map(|i| Duration::from_millis(i))
        .collect();

    println!("  100 requests with latencies from 1ms to 100ms:");
    println!("  p50:  {:>8.2?}", percentile(&sample_latencies, 50.0));
    println!("  p90:  {:>8.2?}", percentile(&sample_latencies, 90.0));
    println!("  p99:  {:>8.2?}", percentile(&sample_latencies, 99.0));
    println!("  max:  {:>8.2?}", sample_latencies.last().unwrap());
    println!();
    println!("Takeaway: percentile = sorted_latencies[N * p / 100].");
    println!("p99 means 99% of requests were faster than this value.");
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Run { url, path, requests, concurrency } => {
            run_load_test(url, path, requests, concurrency).await;
        }
        Command::Demo => demo_with_builtin_server().await,
        Command::ReportDemo => demo_report(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_calculation() {
        let data: Vec<Duration> = (1..=100).map(|i| Duration::from_millis(i)).collect();
        assert_eq!(percentile(&data, 50.0), Duration::from_millis(51));
        assert_eq!(percentile(&data, 90.0), Duration::from_millis(91));
        assert_eq!(percentile(&data, 99.0), Duration::from_millis(100));
    }

    #[tokio::test]
    async fn load_test_against_echo_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    tokio::spawn(async move {
                        let mut buf = [0u8; 512];
                        let _ = stream.read(&mut buf).await;
                        let resp = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
                        let _ = stream.write_all(resp.as_bytes()).await;
                    });
                }
            }
        });

        tokio::time::sleep(Duration::from_millis(20)).await;

        let sem = Arc::new(Semaphore::new(5));
        let results: Arc<Mutex<Vec<RequestResult>>> = Arc::new(Mutex::new(vec![]));
        let mut handles = vec![];

        for _ in 0..10 {
            let permit = sem.clone().acquire_owned().await.unwrap();
            let addr_str = addr.to_string();
            let results = results.clone();
            handles.push(tokio::spawn(async move {
                let r = send_request(&addr_str, "/").await;
                drop(permit);
                results.lock().unwrap().push(r);
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let results = results.lock().unwrap();
        let ok_count = results.iter().filter(|r| r.status == 200).count();
        assert_eq!(ok_count, 10, "All requests should succeed");
    }
}

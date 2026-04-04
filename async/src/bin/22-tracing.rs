// Lesson 22: Tracing & Debugging Async Code
//
// Demonstrates structured logging concepts without requiring tracing crate.
// Run with: cargo run -p async-lessons --bin 22-tracing -- <command>
//
// Commands:
//   spans          Demo span-like structured logging with manual context
//   event-counter  Demo counting events by level
//   request-ids    Demo request ID propagation through async call chains
//   all            Run all demos

use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};


#[derive(Parser)]
#[command(name = "tracing", about = "Lesson 22: Tracing & debugging async code")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Demo span-like structured context propagation
    Spans,
    /// Demo counting events by severity level
    EventCounter,
    /// Demo request ID flowing through async call chain
    RequestIds,
    /// Run all demos
    All,
}

// ---- Simulated structured logging (no tracing crate needed) ----

tokio::task_local! {
    static SPAN_CONTEXT: String;
}

fn log_event(level: &str, span: &str, msg: &str, fields: &[(&str, &str)]) {
    let fields_str: Vec<String> = fields.iter().map(|(k, v)| format!("{k}={v}")).collect();
    let fields_display = if fields_str.is_empty() {
        String::new()
    } else {
        format!(" {{{}}}", fields_str.join(", "))
    };
    let now = chrono_lite_ts();
    println!("  {now} {level:>5} [{span}]{fields_display} {msg}");
}

fn chrono_lite_ts() -> String {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = elapsed.as_secs() % 100_000;
    let millis = elapsed.subsec_millis();
    format!("{secs:05}.{millis:03}")
}

async fn demo_spans() {
    println!("=== Structured Span Demo ===");
    println!("Simulating tracing spans: context flows through async calls.\n");

    for req_id in 1..=3 {
        let span = format!("handle_request(id={req_id})");
        SPAN_CONTEXT.scope(span.clone(), async move {
            log_event("INFO", &span, "request received", &[("method", "GET")]);

            let db_span = format!("{span} > db_query");
            SPAN_CONTEXT.scope(db_span.clone(), async move {
                // Simulate async DB query
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                log_event("DEBUG", &db_span, "query complete", &[("rows", "42")]);
            }).await;

            log_event("INFO", &span, "response sent", &[("status", "200")]);
        }).await;
    }

    println!();
    println!("Takeaway: spans give hierarchical context to log events.");
    println!("With the real tracing crate, #[instrument] does this automatically.");
    println!("Child spans inherit parent context across .await points.");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

struct EventCounter {
    counts: Mutex<HashMap<Level, u64>>,
}

impl EventCounter {
    fn new() -> Self {
        Self { counts: Mutex::new(HashMap::new()) }
    }

    fn record(&self, level: Level) {
        *self.counts.lock().unwrap().entry(level).or_default() += 1;
    }

    fn report(&self) {
        let counts = self.counts.lock().unwrap();
        println!("  Event counts:");
        for level in [Level::Debug, Level::Info, Level::Warn, Level::Error] {
            let count = counts.get(&level).copied().unwrap_or(0);
            let label = format!("{:?}", level);
            println!("    {label:>5}: {count}");
        }
    }
}

async fn demo_event_counter() {
    println!("=== Event Counter Demo ===");
    println!("Counting events by severity level (like a tracing Layer).\n");

    let counter = Arc::new(EventCounter::new());

    // Simulate application activity
    let c = counter.clone();
    let mut handles = vec![];

    for i in 0..20 {
        let c = c.clone();
        handles.push(tokio::spawn(async move {
            // Simulate different event levels
            c.record(Level::Info);
            if i % 3 == 0 { c.record(Level::Debug); }
            if i % 5 == 0 { c.record(Level::Warn); }
            if i % 10 == 0 { c.record(Level::Error); }
            tokio::task::yield_now().await;
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    counter.report();
    println!();
    println!("Takeaway: a tracing Layer can intercept all events and do");
    println!("custom processing — counting, filtering, exporting to services.");
    println!("Layers compose: fmt + filter + counter all work together.");
}

async fn demo_request_ids() {
    println!("=== Request ID Propagation Demo ===");
    println!("Each request gets an ID that flows through all async calls.\n");

    static NEXT_ID: AtomicU64 = AtomicU64::new(1);

    async fn handle_request() {
        let req_id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let ctx = format!("req-{req_id}");

        SPAN_CONTEXT.scope(ctx.clone(), async move {
            log_event("INFO", &ctx, "started processing", &[]);

            // Simulate calling into nested services
            authenticate().await;
            fetch_data().await;

            log_event("INFO", &ctx, "done", &[]);
        }).await;
    }

    async fn authenticate() {
        SPAN_CONTEXT.with(|ctx| {
            log_event("DEBUG", ctx, "authenticating user", &[("method", "token")]);
        });
        tokio::task::yield_now().await;
        SPAN_CONTEXT.with(|ctx| {
            log_event("DEBUG", ctx, "auth success", &[]);
        });
    }

    async fn fetch_data() {
        SPAN_CONTEXT.with(|ctx| {
            log_event("DEBUG", ctx, "fetching from DB", &[("table", "users")]);
        });
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        SPAN_CONTEXT.with(|ctx| {
            log_event("DEBUG", ctx, "fetch complete", &[("rows", "7")]);
        });
    }

    // Simulate 3 concurrent requests
    let mut handles = vec![];
    for _ in 0..3 {
        handles.push(tokio::spawn(handle_request()));
    }
    for h in handles {
        h.await.unwrap();
    }

    println!();
    println!("Takeaway: request IDs in spans let you grep logs for one request.");
    println!("With tracing, this happens automatically via span fields.");
    println!("Even interleaved concurrent requests are easy to follow.");
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Spans => demo_spans().await,
        Command::EventCounter => demo_event_counter().await,
        Command::RequestIds => demo_request_ids().await,
        Command::All => {
            demo_spans().await;
            println!("\n");
            demo_event_counter().await;
            println!("\n");
            demo_request_ids().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn event_counter_counts_correctly() {
        let counter = EventCounter::new();
        counter.record(Level::Info);
        counter.record(Level::Info);
        counter.record(Level::Error);

        let counts = counter.counts.lock().unwrap();
        assert_eq!(counts.get(&Level::Info), Some(&2));
        assert_eq!(counts.get(&Level::Error), Some(&1));
        assert_eq!(counts.get(&Level::Debug), None);
    }

    #[tokio::test]
    async fn span_context_propagates_across_await() {
        SPAN_CONTEXT.scope("test-span".to_string(), async {
            tokio::task::yield_now().await;
            SPAN_CONTEXT.with(|ctx| {
                assert_eq!(ctx, "test-span");
            });
        }).await;
    }
}

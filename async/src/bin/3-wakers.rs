// Lesson 3: Wakers & Waking
//
// Build Wakers from scratch using RawWaker + vtable.
// Run with: cargo run -p async-lessons --bin 3-wakers -- <command>
//
// Commands:
//   noop               Build a noop waker, poll CountdownFuture manually
//   counting <n>       Build a counting waker, verify wake count
//   thread-park <n>    Build a thread-parking waker, real executor pattern
//   deadlock           Show what happens when a future forgets to wake
//   all                Run all demos (except deadlock)

use clap::{Parser, Subcommand};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

// ============================================================
// CountdownFuture (reused from Lesson 1)
// ============================================================

struct CountdownFuture {
    count: u32,
}

impl Future for CountdownFuture {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.count == 0 {
            Poll::Ready(())
        } else {
            self.count -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

// ============================================================
// A "forgetful" future that doesn't call wake (for deadlock demo)
// ============================================================

struct ForgetfulFuture {
    count: u32,
}

impl Future for ForgetfulFuture {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.count == 0 {
            Poll::Ready(())
        } else {
            self.count -= 1;
            // BUG: we return Pending but DON'T call wake!
            // The executor will park and nobody will unpark it → deadlock
            Poll::Pending
        }
    }
}

// ============================================================
// Waker #1: Noop Waker
// ============================================================

/// Build a waker where all operations do nothing.
/// Useful for manual polling in a loop.
///
/// TODO: Implement the four vtable functions (all no-ops) and construct the waker.
///   - clone: return a new RawWaker with same data and vtable
///   - wake: do nothing
///   - wake_by_ref: do nothing
///   - drop: do nothing
fn build_noop_waker() -> Waker {
    todo!("Implement build_noop_waker")
}

// ============================================================
// Waker #2: Counting Waker
// ============================================================

/// Build a waker that increments an Arc<AtomicU32> each time wake() is called.
/// This lets you verify that a future calls wake the correct number of times.
///
/// TODO: Implement the vtable functions:
///   - clone: Arc::from_raw the ptr, clone it, Arc::into_raw both, return new RawWaker
///   - wake: Arc::from_raw the ptr, fetch_add(1), (Arc drops, decrementing refcount)
///   - wake_by_ref: same as wake but don't consume — use Arc::from_raw then Arc::into_raw
///   - drop: Arc::from_raw the ptr (lets it drop, decrementing refcount)
///
/// Hints:
///   - Arc::into_raw(arc) → *const AtomicU32 → cast to *const ()
///   - Arc::from_raw(ptr as *const AtomicU32) → recovers the Arc
///   - After Arc::from_raw, you OWN the Arc — if you don't want to drop it,
///     call Arc::into_raw again to "forget" it
fn build_counting_waker(counter: Arc<AtomicU32>) -> Waker {
    todo!("Implement build_counting_waker")
}

// ============================================================
// Waker #3: Thread-Parking Waker
// ============================================================

/// Build a waker that unparks a specific thread when wake() is called.
/// This is the pattern used by real single-threaded executors.
///
/// TODO: Implement the vtable functions:
///   - clone: Arc::from_raw → clone → Arc::into_raw both → new RawWaker
///   - wake: Arc::from_raw → thread.unpark()
///   - wake_by_ref: same, but don't consume the Arc
///   - drop: Arc::from_raw (lets it drop)
///
/// The data pointer stores an Arc<std::thread::Thread>.
fn build_thread_waker(thread: std::thread::Thread) -> Waker {
    todo!("Implement build_thread_waker")
}

// ============================================================
// CLI
// ============================================================

#[derive(Parser)]
#[command(name = "wakers", about = "Lesson 3: Wakers & Waking")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build a noop waker, poll CountdownFuture manually
    Noop,
    /// Build a counting waker, verify wake() was called N times
    Counting { n: u32 },
    /// Build a thread-parking waker, real executor pattern
    ThreadPark { n: u32 },
    /// Show what happens when a future forgets to call wake
    Deadlock,
    /// Run all demos (except deadlock)
    All,
}

fn demo_noop() {
    println!("=== Noop Waker ===");
    println!("Building a waker where wake() does nothing.");
    println!("We manually poll in a loop — the waker is just a formality.");
    println!();

    let waker = build_noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut future = CountdownFuture { count: 3 };
    let mut pinned = unsafe { Pin::new_unchecked(&mut future) };
    let mut polls = 0;

    loop {
        polls += 1;
        match pinned.as_mut().poll(&mut cx) {
            Poll::Pending => println!("  [poll #{polls}] Pending — waker.wake() called (does nothing)"),
            Poll::Ready(()) => {
                println!("  [poll #{polls}] Ready!");
                break;
            }
        }
    }
    println!();
    println!("Takeaway: noop waker works for manual polling, but a real executor");
    println!("needs wake() to actually DO something (like unpark a thread).");
}

fn demo_counting(n: u32) {
    println!("=== Counting Waker ===");
    println!("Building a waker that counts how many times wake() is called.");
    println!();

    let counter = Arc::new(AtomicU32::new(0));
    let waker = build_counting_waker(counter.clone());
    let mut cx = Context::from_waker(&waker);
    let mut future = CountdownFuture { count: n };
    let mut pinned = unsafe { Pin::new_unchecked(&mut future) };

    loop {
        match pinned.as_mut().poll(&mut cx) {
            Poll::Pending => {}
            Poll::Ready(()) => break,
        }
    }

    let wake_count = counter.load(Ordering::SeqCst);
    println!("  CountdownFuture({n}) called wake() {wake_count} times");
    assert_eq!(wake_count, n, "Should wake exactly N times");
    println!("  ✓ Correct! Each Pending returned one wake() call.");
    println!();
    println!("Takeaway: counting wakers let you verify the wake contract in tests.");
}

fn demo_thread_park(n: u32) {
    println!("=== Thread-Parking Waker ===");
    println!("Building a waker that unparks the current thread.");
    println!("After Pending, the executor parks (sleeps). wake() unparks it.");
    println!();

    let waker = build_thread_waker(std::thread::current());
    let mut cx = Context::from_waker(&waker);
    let mut future = CountdownFuture { count: n };
    let mut pinned = unsafe { Pin::new_unchecked(&mut future) };
    let mut polls = 0;

    loop {
        polls += 1;
        match pinned.as_mut().poll(&mut cx) {
            Poll::Pending => {
                println!("  [poll #{polls}] Pending — parking thread...");
                std::thread::park();
                println!("  [poll #{polls}] ...unparked by waker!");
            }
            Poll::Ready(()) => {
                println!("  [poll #{polls}] Ready!");
                break;
            }
        }
    }
    println!();
    println!("Takeaway: this is how real executors work.");
    println!("  1. Poll future → Pending");
    println!("  2. Park thread (sleep, 0% CPU)");
    println!("  3. wake() → unpark → poll again");
    println!("This is the foundation for block_on() in Lesson 4.");
}

fn demo_deadlock() {
    println!("=== Deadlock Demo ===");
    println!("A future that returns Pending WITHOUT calling wake().");
    println!("The executor parks... and nobody unparks it. Deadlock!");
    println!();
    println!("Press Ctrl+C to exit after a few seconds.");
    println!();

    let waker = build_thread_waker(std::thread::current());
    let mut cx = Context::from_waker(&waker);
    let mut future = ForgetfulFuture { count: 3 };
    let mut pinned = unsafe { Pin::new_unchecked(&mut future) };

    let mut polls = 0;
    loop {
        polls += 1;
        match pinned.as_mut().poll(&mut cx) {
            Poll::Pending => {
                println!("  [poll #{polls}] Pending — parking thread...");
                println!("  [poll #{polls}] (Future forgot to wake! This will hang forever)");
                std::thread::park();
                // We'll never reach here — nobody calls unpark
                println!("  [poll #{polls}] ...unparked! (you won't see this)");
            }
            Poll::Ready(()) => {
                println!("  [poll #{polls}] Ready!");
                break;
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Noop => demo_noop(),
        Command::Counting { n } => demo_counting(n),
        Command::ThreadPark { n } => demo_thread_park(n),
        Command::Deadlock => demo_deadlock(),
        Command::All => {
            demo_noop();
            println!("\n");
            demo_counting(5);
            println!("\n");
            demo_thread_park(3);
            println!();
            println!("(Skipping deadlock demo — run it manually with 'deadlock' command)");
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_waker_polls_to_completion() {
        let waker = build_noop_waker();
        let mut cx = Context::from_waker(&waker);
        let mut future = CountdownFuture { count: 3 };
        let mut pinned = unsafe { Pin::new_unchecked(&mut future) };

        assert!(pinned.as_mut().poll(&mut cx).is_pending());
        assert!(pinned.as_mut().poll(&mut cx).is_pending());
        assert!(pinned.as_mut().poll(&mut cx).is_pending());
        assert!(pinned.as_mut().poll(&mut cx).is_ready());
    }

    #[test]
    fn counting_waker_counts_correctly() {
        let counter = Arc::new(AtomicU32::new(0));
        let waker = build_counting_waker(counter.clone());
        let mut cx = Context::from_waker(&waker);
        let mut future = CountdownFuture { count: 7 };
        let mut pinned = unsafe { Pin::new_unchecked(&mut future) };

        loop {
            match pinned.as_mut().poll(&mut cx) {
                Poll::Pending => {}
                Poll::Ready(()) => break,
            }
        }
        assert_eq!(counter.load(Ordering::SeqCst), 7);
    }

    #[test]
    fn thread_waker_completes() {
        let waker = build_thread_waker(std::thread::current());
        let mut cx = Context::from_waker(&waker);
        let mut future = CountdownFuture { count: 5 };
        let mut pinned = unsafe { Pin::new_unchecked(&mut future) };

        loop {
            match pinned.as_mut().poll(&mut cx) {
                Poll::Pending => std::thread::park(),
                Poll::Ready(()) => break,
            }
        }
        // If we get here, it didn't deadlock — the waker worked.
    }
}

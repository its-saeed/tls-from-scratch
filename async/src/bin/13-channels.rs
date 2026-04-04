// Lesson 13: Channels
//
// Build async oneshot and mpsc channels with waker integration.
// Run with: cargo run -p async-lessons --bin 13-channels -- <command>
//
// Commands:
//   oneshot          Demo oneshot channel: send + receive
//   oneshot-drop     Demo sender drop detection
//   mpsc             Demo mpsc channel: multiple senders
//   all              Run all demos

use clap::{Parser, Subcommand};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

// ============================================================
// Oneshot channel
// ============================================================

mod oneshot {
    use super::*;

    struct Inner<T> {
        value: Option<T>,
        rx_waker: Option<Waker>,
        closed: bool,
    }

    pub struct Sender<T> {
        inner: Arc<Mutex<Inner<T>>>,
    }

    pub struct Receiver<T> {
        inner: Arc<Mutex<Inner<T>>>,
    }

    #[derive(Debug, PartialEq)]
    pub struct RecvError;

    pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
        let inner = Arc::new(Mutex::new(Inner {
            value: None,
            rx_waker: None,
            closed: false,
        }));
        (
            Sender { inner: inner.clone() },
            Receiver { inner },
        )
    }

    impl<T> Sender<T> {
        /// Send a value. Consumes the sender (can only send once).
        ///
        /// TODO: Implement this.
        ///   1. Lock inner
        ///   2. If closed (receiver dropped): return Err(value)
        ///   3. Store value
        ///   4. If rx_waker is Some: wake it
        ///   5. Return Ok(())
        pub fn send(self, value: T) -> Result<(), T> {
            todo!("Implement oneshot::Sender::send")
        }
    }

    impl<T> Drop for Sender<T> {
        fn drop(&mut self) {
            let mut inner = self.inner.lock().unwrap();
            inner.closed = true;
            if let Some(waker) = inner.rx_waker.take() {
                waker.wake();
            }
        }
    }

    impl<T: Unpin> Future for Receiver<T> {
        type Output = Result<T, RecvError>;

        /// TODO: Implement this.
        ///   1. Lock inner
        ///   2. If value is Some: take it, return Ready(Ok(value))
        ///   3. If closed: return Ready(Err(RecvError))
        ///   4. Else: store waker, return Pending
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            todo!("Implement oneshot::Receiver::poll")
        }
    }

    impl<T> Drop for Receiver<T> {
        fn drop(&mut self) {
            let mut inner = self.inner.lock().unwrap();
            inner.closed = true;
        }
    }
}

// ============================================================
// MPSC channel
// ============================================================

mod mpsc {
    use super::*;

    struct Inner<T> {
        queue: VecDeque<T>,
        rx_waker: Option<Waker>,
        sender_count: usize,
    }

    pub struct Sender<T> {
        inner: Arc<Mutex<Inner<T>>>,
    }

    pub struct Receiver<T> {
        inner: Arc<Mutex<Inner<T>>>,
    }

    pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
        let inner = Arc::new(Mutex::new(Inner {
            queue: VecDeque::new(),
            rx_waker: None,
            sender_count: 1,
        }));
        (
            Sender { inner: inner.clone() },
            Receiver { inner },
        )
    }

    impl<T> Clone for Sender<T> {
        fn clone(&self) -> Self {
            self.inner.lock().unwrap().sender_count += 1;
            Sender { inner: self.inner.clone() }
        }
    }

    impl<T> Sender<T> {
        /// Send a value on the channel.
        ///
        /// TODO: Implement this.
        ///   1. Lock inner
        ///   2. Push value to queue
        ///   3. If rx_waker is Some: wake it
        pub fn send(&self, value: T) {
            todo!("Implement mpsc::Sender::send")
        }
    }

    impl<T> Drop for Sender<T> {
        fn drop(&mut self) {
            let mut inner = self.inner.lock().unwrap();
            inner.sender_count -= 1;
            if inner.sender_count == 0 {
                if let Some(waker) = inner.rx_waker.take() {
                    waker.wake();
                }
            }
        }
    }

    impl<T> Receiver<T> {
        /// Receive a value. Returns None when all senders are dropped.
        ///
        /// This is NOT a Future itself — it returns a RecvFuture.
        pub fn recv(&mut self) -> RecvFuture<'_, T> {
            RecvFuture { receiver: self }
        }
    }

    pub struct RecvFuture<'a, T> {
        receiver: &'a mut Receiver<T>,
    }

    impl<'a, T> Future for RecvFuture<'a, T> {
        type Output = Option<T>;

        /// TODO: Implement this.
        ///   1. Lock inner
        ///   2. Pop from queue → Some(value)? return Ready(Some(value))
        ///   3. Queue empty + sender_count == 0? return Ready(None) (channel closed)
        ///   4. Else: store waker, return Pending
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
            todo!("Implement mpsc::RecvFuture::poll")
        }
    }
}

// ============================================================
// Simple executor (from Lesson 10)
// ============================================================

struct Task {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        self.queue.lock().unwrap().push_back(self.clone());
    }
}

struct Executor {
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
}

impl Executor {
    fn new() -> Self {
        Self { queue: Arc::new(Mutex::new(VecDeque::new())) }
    }

    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            queue: self.queue.clone(),
        });
        self.queue.lock().unwrap().push_back(task);
    }

    fn run(&self) {
        loop {
            let task = self.queue.lock().unwrap().pop_front();
            match task {
                Some(task) => {
                    let waker = Waker::from(task.clone());
                    let mut cx = Context::from_waker(&waker);
                    let mut future = task.future.lock().unwrap();
                    let _ = future.as_mut().poll(&mut cx);
                }
                None => break,
            }
        }
    }
}

// ============================================================
// CLI
// ============================================================

#[derive(Parser)]
#[command(name = "channels", about = "Lesson 13: Channels")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Demo oneshot channel
    Oneshot,
    /// Demo sender drop detection
    OneshotDrop,
    /// Demo mpsc channel with multiple senders
    Mpsc,
    /// Run all demos
    All,
}

fn demo_oneshot() {
    println!("=== Oneshot Channel ===");
    println!("One sender, one receiver, one message.");
    println!();

    // TODO: uncomment when oneshot is implemented
    // let executor = Executor::new();
    // let (tx, rx) = oneshot::channel();
    //
    // executor.spawn(async move {
    //     tx.send(42).unwrap();
    //     println!("  [sender] sent 42");
    // });
    //
    // executor.spawn(async move {
    //     let value = rx.await.unwrap();
    //     println!("  [receiver] got {value}");
    //     assert_eq!(value, 42);
    // });
    //
    // executor.run();

    println!("TODO: implement oneshot::Sender::send and Receiver::poll");
    println!();
    println!("Takeaway: oneshot is the simplest async channel.");
    println!("send() stores value + wakes receiver. Receiver polls for value.");
}

fn demo_oneshot_drop() {
    println!("=== Oneshot Drop Detection ===");
    println!("Dropping sender without sending → receiver gets error.");
    println!();

    // TODO: uncomment when implemented
    // let executor = Executor::new();
    // let (tx, rx) = oneshot::channel::<i32>();
    //
    // executor.spawn(async move {
    //     drop(tx);
    //     println!("  [sender] dropped without sending");
    // });
    //
    // executor.spawn(async move {
    //     match rx.await {
    //         Ok(v) => println!("  [receiver] unexpected value: {v}"),
    //         Err(_) => println!("  [receiver] got RecvError — sender dropped!"),
    //     }
    // });
    //
    // executor.run();

    println!("TODO: implement oneshot, then uncomment the demo.");
    println!();
    println!("Takeaway: Drop on sender wakes receiver with an error.");
}

fn demo_mpsc() {
    println!("=== MPSC Channel ===");
    println!("Multiple senders, one receiver.");
    println!();

    // TODO: uncomment when implemented
    // let executor = Executor::new();
    // let (tx, mut rx) = mpsc::channel();
    //
    // for i in 0..3 {
    //     let tx = tx.clone();
    //     executor.spawn(async move {
    //         tx.send(i * 10);
    //         println!("  [sender {i}] sent {}", i * 10);
    //     });
    // }
    // drop(tx); // drop original sender
    //
    // executor.spawn(async move {
    //     let mut values = vec![];
    //     while let Some(v) = rx.recv().await {
    //         values.push(v);
    //     }
    //     println!("  [receiver] got {} messages: {:?}", values.len(), values);
    // });
    //
    // executor.run();

    println!("TODO: implement mpsc::Sender::send and RecvFuture::poll");
    println!();
    println!("Takeaway: MPSC channels let multiple tasks send to one receiver.");
    println!("When all senders drop, recv() returns None.");
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Oneshot => demo_oneshot(),
        Command::OneshotDrop => demo_oneshot_drop(),
        Command::Mpsc => demo_mpsc(),
        Command::All => {
            demo_oneshot();
            println!("\n");
            demo_oneshot_drop();
            println!("\n");
            demo_mpsc();
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
    fn oneshot_channel_creation() {
        let (_tx, _rx) = oneshot::channel::<i32>();
        // Should compile and not panic
    }

    #[test]
    fn mpsc_channel_creation() {
        let (_tx, _rx) = mpsc::channel::<i32>();
    }

    #[test]
    fn mpsc_sender_clone() {
        let (tx, _rx) = mpsc::channel::<i32>();
        let _tx2 = tx.clone();
        let _tx3 = tx.clone();
    }
}

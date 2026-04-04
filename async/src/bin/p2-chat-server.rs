// Project 2: Multi-threaded Chat Server
//
// A chat server built on YOUR async runtime. No tokio.
// Combines: reactor, scheduler, async I/O, timers, channels, select.
//
// Run with: cargo run -p async-lessons --bin p2-chat-server -- <command>
//
// Commands:
//   run              Start the chat server on 127.0.0.1:8080
//   test             Run a quick self-test with simulated clients

use clap::{Parser, Subcommand};

// ============================================================
// Chat Protocol
// ============================================================

/// Events sent from client tasks to the broker.
#[derive(Debug)]
enum Event {
    /// A new client connected.
    Join {
        id: usize,
        nick: String,
        // inbox: Sender<String>,  // channel to send messages to this client
    },
    /// A client disconnected.
    Leave { id: usize },
    /// A client sent a message.
    Message { id: usize, text: String },
    /// A client changed their nickname.
    NickChange { id: usize, new_nick: String },
}

// ============================================================
// Broker: owns the client map, fans out messages
// ============================================================

/// The broker task owns all client state.
/// It receives Events and broadcasts messages.
///
/// TODO: Implement the broker loop.
///
/// ```rust
/// async fn broker(mut events: Receiver<Event>) {
///     let mut clients: HashMap<usize, ClientHandle> = HashMap::new();
///
///     while let Some(event) = events.recv().await {
///         match event {
///             Event::Join { id, nick, inbox } => {
///                 broadcast(&clients, &format!("{} joined", nick));
///                 clients.insert(id, ClientHandle { nick, inbox });
///             }
///             Event::Leave { id } => {
///                 if let Some(handle) = clients.remove(&id) {
///                     broadcast(&clients, &format!("{} left", handle.nick));
///                 }
///             }
///             Event::Message { id, text } => {
///                 if let Some(handle) = clients.get(&id) {
///                     let msg = format!("{}: {}", handle.nick, text);
///                     broadcast_except(&clients, id, &msg);
///                 }
///             }
///             Event::NickChange { id, new_nick } => {
///                 if let Some(handle) = clients.get_mut(&id) {
///                     let old = std::mem::replace(&mut handle.nick, new_nick.clone());
///                     broadcast(&clients, &format!("{} is now {}", old, new_nick));
///                 }
///             }
///         }
///     }
/// }
/// ```

// ============================================================
// Client task: read loop + inbox
// ============================================================

/// Each client gets its own task that:
///   1. Reads lines from the TCP stream
///   2. Sends events to the broker
///   3. Receives messages from inbox and writes to stream
///
/// Uses select! to handle both directions:
///
/// ```rust
/// async fn client_task(
///     id: usize,
///     stream: AsyncTcpStream,
///     events_tx: Sender<Event>,
///     mut inbox: Receiver<String>,
/// ) {
///     let (reader, writer) = stream.split();
///     let mut lines = BufReader::new(reader);
///
///     loop {
///         select! {
///             line = lines.read_line() => {
///                 match line {
///                     Ok(line) if line.is_empty() => break, // EOF
///                     Ok(line) => {
///                         if line.starts_with("/nick ") {
///                             let nick = line[6..].trim().to_string();
///                             events_tx.send(Event::NickChange { id, new_nick: nick });
///                         } else if line.starts_with("/quit") {
///                             break;
///                         } else {
///                             events_tx.send(Event::Message { id, text: line });
///                         }
///                     }
///                     Err(_) => break,
///                 }
///             }
///             msg = inbox.recv() => {
///                 if let Some(msg) = msg {
///                     writer.write_all(msg.as_bytes()).await;
///                     writer.write_all(b"\n").await;
///                 }
///             }
///         }
///     }
///
///     events_tx.send(Event::Leave { id });
/// }
/// ```

// ============================================================
// Server: accept loop
// ============================================================

/// The main server loop:
///
/// ```rust
/// async fn server() {
///     let listener = AsyncTcpListener::bind("127.0.0.1:8080").await;
///     let (events_tx, events_rx) = mpsc::channel();
///
///     spawn(broker(events_rx));
///
///     let mut next_id = 0;
///     loop {
///         let (stream, addr) = listener.accept().await;
///         let id = next_id;
///         next_id += 1;
///         let nick = format!("user-{}", id);
///
///         let (inbox_tx, inbox_rx) = mpsc::channel();
///         events_tx.send(Event::Join { id, nick, inbox: inbox_tx });
///
///         let tx = events_tx.clone();
///         spawn(client_task(id, stream, tx, inbox_rx));
///     }
/// }
/// ```

// ============================================================
// CLI
// ============================================================

#[derive(Parser)]
#[command(name = "chat-server", about = "Project 2: Chat Server")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the chat server
    Run,
    /// Run a self-test with simulated clients
    Test,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Run => {
            println!("=== Chat Server ===");
            println!("TODO: implement using your runtime from Lessons 9-15.");
            println!();
            println!("The server should:");
            println!("  1. Accept connections on 127.0.0.1:8080");
            println!("  2. Spawn a broker task that owns the client map");
            println!("  3. Spawn a client task per connection");
            println!("  4. Broadcast messages to all other clients");
            println!("  5. Support /nick, /who, /quit commands");
            println!();
            println!("Test with: nc 127.0.0.1 8080 (multiple terminals)");
            println!();
            println!("Architecture:");
            println!("  Accept loop → spawn client tasks → events → broker → broadcast");
            // TODO: block_on(server());
        }
        Command::Test => {
            println!("=== Chat Server Self-Test ===");
            println!("TODO: implement the server, then:");
            println!("  1. Spawn server in background");
            println!("  2. Connect 3 clients");
            println!("  3. Client A sends 'hello'");
            println!("  4. Assert clients B and C receive 'user-0: hello'");
            println!("  5. Client B sends '/nick bob'");
            println!("  6. Client B sends 'hi'");
            println!("  7. Assert clients A and C receive 'bob: hi'");
        }
    }
}

# Project: Encrypted File Transfer

> **Prerequisites**: Lesson 2 (Encryption), Lesson 5 (HKDF), Lesson 9-10 (Encrypted + Authenticated Echo Server). This project extends the echo server to transfer files.

## What is this?

`scp` lets you copy files between machines over SSH. You're building the same thing — but using the crypto you built in earlier lessons instead of SSH.

```
┌──────────────────────────────────────────────────────────┐
│  The problem:                                            │
│                                                          │
│  You want to send a file to another machine.             │
│                                                          │
│  Option 1: netcat (nc)                                   │
│    Fast. Simple. ZERO encryption.                        │
│    Anyone on the network can see your file.              │
│                                                          │
│  Option 2: scp / sftp                                    │
│    Encrypted via SSH. But requires SSH setup on both     │
│    machines, user accounts, authorized_keys...           │
│                                                          │
│  Option 3: your tool (this project)                      │
│    Encrypted with your mini-TLS (Lessons 9-10).          │
│    One binary on each side. No SSH, no accounts.         │
│    Authenticated — receiver proves identity.             │
│    Integrity — SHA-256 verifies the file wasn't corrupted│
└──────────────────────────────────────────────────────────┘
```

## What you're building

```sh
# Terminal 1 — receiver listens:
cargo run -p tls --bin p5-transfer -- receive --port 9000 --key server.key
# Listening on 0.0.0.0:9000...

# Terminal 2 — sender connects and sends a file:
cargo run -p tls --bin p5-transfer -- send \
  --host 127.0.0.1:9000 \
  --server-pubkey abc123... \
  my-file.tar.gz

# Output:
# Connected to 127.0.0.1:9000
# Server authenticated ✓
# Sending: my-file.tar.gz (4.2 MB)
# [████████████████████████] 100%  4.2 MB  2.1s  2.0 MB/s
# SHA-256: a1b2c3d4e5f6...
# Transfer complete ✓
```

## Architecture

```
Sender                                  Receiver
  │                                        │
  │── DH public key (32B) ────────────────►│
  │◄── DH public key (32B) ────────────────│  Handshake
  │◄── signature (64B) ────────────────────│  (Lessons 9-10)
  │                                        │
  │  verify signature ✓                    │
  │  derive c2s_key, s2c_key               │
  │                                        │
  │── [len][encrypted metadata] ──────────►│  Step 1: what file?
  │                                        │  filename, size, SHA-256
  │                                        │
  │── [len][encrypted chunk 1] ───────────►│  Step 2: file data
  │── [len][encrypted chunk 2] ───────────►│  4KB chunks
  │── [len][encrypted chunk 3] ───────────►│  counter nonces
  │── ...                                  │
  │── [len][encrypted final chunk] ───────►│
  │                                        │
  │◄── [len][encrypted "OK" or "ERR"] ─────│  Step 3: verification
  │                                        │  receiver checks SHA-256
```

## Try it with existing tools

```sh
# === netcat: file transfer WITHOUT encryption ===

# Terminal 1 (receiver):
nc -l 9000 > received.txt

# Terminal 2 (sender):
echo "secret document content" > secret.txt
nc 127.0.0.1 9000 < secret.txt

# The file transferred — but ANYONE on the network could read it.
# Your tool does the same thing, but encrypted.
```

```sh
# === scp: file transfer WITH encryption (SSH) ===
scp myfile.txt user@server:/tmp/

# This uses SSH (which uses TLS-like crypto internally).
# You're building the crypto layer yourself.
```

```sh
# === Verify file integrity with SHA-256 ===

# Create a test file:
dd if=/dev/urandom of=testfile.bin bs=1024 count=4096 2>/dev/null
# Created a 4MB random file

# Compute its hash:
shasum -a 256 testfile.bin
# a1b2c3d4...  testfile.bin

# This is what your tool sends as metadata — the receiver
# recomputes the hash after receiving all chunks and compares.
```

## Implementation guide

### Step 0: Project setup

```sh
touch tls/src/bin/p5-transfer.rs
```

Add to `tls/Cargo.toml` (if not already there):

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

CLI skeleton:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "transfer", about = "Encrypted file transfer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Send a file
    Send {
        /// Host:port to connect to
        #[arg(long)]
        host: String,
        /// Server's public key (hex)
        #[arg(long)]
        server_pubkey: String,
        /// File to send
        file: String,
    },
    /// Receive a file
    Receive {
        /// Port to listen on
        #[arg(long, default_value = "9000")]
        port: u16,
        /// Path to server identity key
        #[arg(long)]
        key: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Send { host, server_pubkey, file } => todo!(),
        Command::Receive { port, key } => todo!(),
    }
}
```

### Step 1: Reuse the handshake from Lesson 10

You already built this. Copy (or import) the handshake code:

```rust
/// Sender side: connect, do DH, verify server signature
fn sender_handshake(stream: &mut TcpStream, server_pubkey: &[u8; 32])
    -> (ChaCha20Poly1305, ChaCha20Poly1305)
{
    // 1. Generate ephemeral X25519 DH key pair
    // 2. Send our DH public key (32 bytes)
    // 3. Read server's ephemeral DH public key (32 bytes) — NOT the identity key
    // 4. Read server's Ed25519 signature (64 bytes) over its DH public key
    // 5. Verify signature using server_pubkey (the identity key passed as argument)
    // 6. Compute shared secret = DH(our_secret, server_dh_public)
    // 7. Derive c2s_key and s2c_key via HKDF
    // Return (c2s_cipher, s2c_cipher)
    todo!("Reuse handshake from Lesson 10")
}

/// Receiver side: accept, do DH, sign
fn receiver_handshake(stream: &mut TcpStream, identity_key: &SigningKey)
    -> (ChaCha20Poly1305, ChaCha20Poly1305)
{
    // Mirror of sender_handshake
    todo!("Reuse handshake from Lesson 10")
}
```

Test: connect sender to receiver, verify "handshake complete" prints on both sides.

### Step 2: Compute file metadata

Before sending any data, tell the receiver what to expect:

```rust
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::Read;

fn compute_file_metadata(path: &str) -> (String, u64, String) {
    let mut file = File::open(path).expect("can't open file");
    let file_size = file.metadata().unwrap().len();
    let filename = std::path::Path::new(path)
        .file_name().unwrap()
        .to_str().unwrap()
        .to_string();

    // Compute SHA-256 of entire file
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = file.read(&mut buf).unwrap();
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    let hash = hex::encode(hasher.finalize());

    (filename, file_size, hash)
}
```

Test:

```rust
let (name, size, hash) = compute_file_metadata("testfile.bin");
println!("File: {name}, Size: {size}, SHA-256: {hash}");

// Compare with shasum:
// shasum -a 256 testfile.bin
```

### Step 3: Send metadata as first encrypted message

```rust
fn send_metadata(
    stream: &mut TcpStream,
    cipher: &ChaCha20Poly1305,
    filename: &str,
    file_size: u64,
    sha256: &str,
    nonce_counter: &mut u64,
) {
    let metadata = serde_json::json!({
        "filename": filename,
        "size": file_size,
        "sha256": sha256,
    });
    let json_bytes = metadata.to_string().into_bytes();
    send_encrypted(stream, cipher, &json_bytes, *nonce_counter);
    *nonce_counter += 1;
}
```

Here, `send_encrypted` is your framing function from the echo server: `[2-byte length][12-byte nonce][ciphertext + tag]`. But now the nonce comes from a counter (Lesson 12) instead of random.

```rust
fn counter_nonce(counter: u64) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[4..12].copy_from_slice(&counter.to_be_bytes());
    nonce
}

fn send_encrypted(
    stream: &mut TcpStream,
    cipher: &ChaCha20Poly1305,
    data: &[u8],
    nonce_counter: u64,
) {
    let nonce = counter_nonce(nonce_counter);
    let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce), data).unwrap();
    let len = ciphertext.len() as u16;
    stream.write_all(&len.to_be_bytes()).unwrap();
    stream.write_all(&ciphertext).unwrap();
    // Note: nonce is NOT sent — receiver derives it from the same counter
}
```

### Step 4: Send file in chunks

```rust
fn send_file(
    stream: &mut TcpStream,
    cipher: &ChaCha20Poly1305,
    path: &str,
    file_size: u64,
    nonce_counter: &mut u64,
) {
    let mut file = File::open(path).unwrap();
    let mut buf = [0u8; 4096];
    let mut sent: u64 = 0;

    loop {
        let n = file.read(&mut buf).unwrap();
        if n == 0 { break; }

        send_encrypted(stream, cipher, &buf[..n], *nonce_counter);
        *nonce_counter += 1;
        sent += n as u64;

        // Progress
        let pct = (sent as f64 / file_size as f64 * 100.0) as u32;
        eprint!("\r  Sent: {} / {} bytes ({}%)", sent, file_size, pct);
    }
    eprintln!();
}
```

### Step 5: Receive and verify

The receiver mirrors the sender:

```rust
fn receive_file(
    stream: &mut TcpStream,
    cipher: &ChaCha20Poly1305,
    nonce_counter: &mut u64,
) {
    // 1. Receive metadata
    let metadata_bytes = recv_encrypted(stream, cipher, *nonce_counter);
    *nonce_counter += 1;
    let metadata: serde_json::Value = serde_json::from_slice(&metadata_bytes).unwrap();

    let filename = metadata["filename"].as_str().unwrap();
    let expected_size = metadata["size"].as_u64().unwrap();
    let expected_hash = metadata["sha256"].as_str().unwrap();

    println!("Receiving: {filename} ({expected_size} bytes)");

    // 2. Receive chunks, write to file, compute hash
    let mut output = File::create(filename).unwrap();
    let mut hasher = Sha256::new();
    let mut received: u64 = 0;

    while received < expected_size {
        let chunk = recv_encrypted(stream, cipher, *nonce_counter);
        *nonce_counter += 1;
        hasher.update(&chunk);
        output.write_all(&chunk).unwrap();
        received += chunk.len() as u64;

        let pct = (received as f64 / expected_size as f64 * 100.0) as u32;
        eprint!("\r  Received: {} / {} bytes ({}%)", received, expected_size, pct);
    }
    eprintln!();

    // 3. Verify hash
    let actual_hash = hex::encode(hasher.finalize());
    if actual_hash == expected_hash {
        println!("SHA-256 verified ✓");
        println!("File saved: {filename}");
    } else {
        eprintln!("SHA-256 MISMATCH!");
        eprintln!("  Expected: {expected_hash}");
        eprintln!("  Got:      {actual_hash}");
        std::fs::remove_file(filename).ok();
        eprintln!("File deleted — transfer corrupted.");
    }
}
```

The `recv_encrypted` function reads the 2-byte length, reads that many bytes of ciphertext, and decrypts with the counter nonce:

```rust
fn recv_encrypted(
    stream: &mut TcpStream,
    cipher: &ChaCha20Poly1305,
    nonce_counter: u64,
) -> Vec<u8> {
    let mut len_buf = [0u8; 2];
    stream.read_exact(&mut len_buf).unwrap();
    let len = u16::from_be_bytes(len_buf) as usize;

    let mut ciphertext = vec![0u8; len];
    stream.read_exact(&mut ciphertext).unwrap();

    let nonce = counter_nonce(nonce_counter);
    cipher.decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .expect("Decryption failed — corrupted data or wrong counter")
}
```

**Key design choice**: the nonce is NOT sent on the wire. Both sides maintain the same counter independently. This saves 12 bytes per chunk AND prevents an attacker from manipulating nonces.

### Step 6: Wire it all together

```rust
fn cmd_send(host: &str, server_pubkey_hex: &str, file_path: &str) {
    let mut stream = TcpStream::connect(host).unwrap();
    let server_pubkey: [u8; 32] = hex::decode(server_pubkey_hex).unwrap().try_into().unwrap();
    let (c2s_cipher, _s2c_cipher) = sender_handshake(&mut stream, &server_pubkey);
    println!("Connected, server authenticated ✓");

    let (filename, file_size, sha256) = compute_file_metadata(file_path);
    println!("Sending: {filename} ({file_size} bytes, SHA-256: {sha256})");

    let mut nonce = 0u64;
    send_metadata(&mut stream, &c2s_cipher, &filename, file_size, &sha256, &mut nonce);
    send_file(&mut stream, &c2s_cipher, file_path, file_size, &mut nonce);

    println!("Transfer complete ✓");
}

fn cmd_receive(port: u16, key_path: &str) {
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).unwrap();
    println!("Listening on 0.0.0.0:{port}...");

    let identity_key = load_identity_key(key_path);
    let (mut stream, addr) = listener.accept().unwrap();
    println!("Connection from {addr}");

    let (c2s_cipher, _s2c_cipher) = receiver_handshake(&mut stream, &identity_key);
    println!("Handshake complete ✓");

    let mut nonce = 0u64;
    receive_file(&mut stream, &c2s_cipher, &mut nonce);
}
```

### Step 7: Test it end-to-end

```sh
# Generate server identity key:
cargo run -p tls --bin 10-genkey
# Public key: dd8c3c76...

# Create a test file:
dd if=/dev/urandom of=testfile.bin bs=1024 count=1024 2>/dev/null
shasum -a 256 testfile.bin
# a1b2c3d4...

# Terminal 1 — receiver:
cargo run -p tls --bin p5-transfer -- receive --port 9000 --key server_identity.key

# Terminal 2 — sender:
cargo run -p tls --bin p5-transfer -- send \
  --host 127.0.0.1:9000 \
  --server-pubkey dd8c3c76... \
  testfile.bin

# Expected output (sender):
#   Connected, server authenticated ✓
#   Sending: testfile.bin (1048576 bytes)
#   Sent: 1048576 / 1048576 bytes (100%)
#   Transfer complete ✓

# Expected output (receiver):
#   Listening on 0.0.0.0:9000...
#   Connection from 127.0.0.1:54321
#   Handshake complete ✓
#   Receiving: testfile.bin (1048576 bytes)
#   Received: 1048576 / 1048576 bytes (100%)
#   SHA-256 verified ✓
#   File saved: testfile.bin

# Verify the received file matches:
shasum -a 256 testfile.bin
# Should match the original hash
```

## Security properties

```
What this tool provides:
  ✓ Confidentiality    — file data is encrypted (ChaCha20-Poly1305)
  ✓ Integrity          — SHA-256 hash verifies file wasn't corrupted
  ✓ Authentication     — server proves identity via Ed25519 signature
  ✓ Replay defense     — counter nonces prevent chunk replay/reorder
  ✓ Tamper detection   — AEAD tag on each chunk detects modification

What it does NOT provide:
  ✗ Client authentication — receiver doesn't verify who's sending
  ✗ Resume after disconnect — must restart from scratch
  ✗ Multiple files in one session
  ✗ Compression
```

## Exercises

### Exercise 1: Basic transfer

Implement all steps above. Transfer a 1MB file, verify SHA-256 matches on both sides.

### Exercise 2: Progress bar

Show a visual progress bar with speed and ETA:

```
  testfile.bin  [██████████░░░░░░░░░░] 52%  524KB/1MB  1.2MB/s  ETA 0.4s
```

Hint: `\r` carriage return overwrites the current line.

### Exercise 3: Multiple files

Accept multiple file paths. Send metadata for each, then data. The receiver creates all files:

```sh
cargo run -p tls --bin p5-transfer -- send \
  --host 127.0.0.1:9000 \
  --server-pubkey abc... \
  file1.txt file2.pdf file3.tar.gz
```

### Exercise 4: Resume interrupted transfer

If the connection drops mid-transfer:
1. Receiver saves how many bytes/chunks were received
2. On reconnect, receiver tells sender "I have chunks 0-47"
3. Sender skips those chunks, continues from chunk 48

This requires a "resume" protocol message after the handshake.

# Project: Encrypted File Transfer

> **Prerequisites**: Lessons 9-10 (Encrypted + Authenticated Echo Server). This project extends the echo server to transfer files.

## What you're building

A mini `scp` — send files securely between two machines using your hand-built TLS protocol.

```sh
# Receiver listens:
cargo run -p tls --bin p5-transfer -- receive --port 9000 --key server.key

# Sender connects and sends:
cargo run -p tls --bin p5-transfer -- send --host 127.0.0.1:9000 \
  --server-pubkey abc123... my-file.tar.gz
# Sending my-file.tar.gz (4.2 MB)...
# [████████████████████████] 100% — 2.1s
# SHA-256: a1b2c3d4...
# Transfer complete ✓
```

## Architecture

```
Sender                                  Receiver
  │                                        │
  │── DH public key (32B) ───────────────►│  Handshake
  │◄── DH public key (32B) ──────────────│  (same as Lesson 9)
  │◄── signature (64B) ──────────────────│  (authentication from Lesson 10)
  │                                        │
  │  derive c2s_key, s2c_key               │
  │                                        │
  │── [len][encrypted metadata] ─────────►│  filename + file size + hash
  │                                        │
  │── [len][encrypted chunk 1] ──────────►│  4KB chunks
  │── [len][encrypted chunk 2] ──────────►│
  │── [len][encrypted chunk 3] ──────────►│
  │── ...                                  │
  │── [len][encrypted final chunk] ───────►│
  │                                        │
  │◄── [len][encrypted "OK" or "ERR"] ───│  Receiver verifies SHA-256
```

## Protocol

### Metadata message (first encrypted message)

```json
{
  "filename": "backup.tar.gz",
  "size": 4200000,
  "sha256": "a1b2c3d4e5f6..."
}
```

### Data chunks

File is split into 4KB chunks. Each chunk is encrypted individually with an incrementing nonce (Lesson 12 replay defense). The receiver reassembles and verifies the SHA-256 hash.

## Implementation guide

### Step 1: Reuse the handshake from Lesson 10

Copy the DH key exchange + authentication code from Lesson 10. After the handshake, you have `c2s_key` and `s2c_key`.

### Step 2: Send metadata

```rust
let metadata = serde_json::json!({
    "filename": filename,
    "size": file_size,
    "sha256": hex::encode(sha256_hash),
});
send_encrypted(&mut stream, &c2s_key, metadata.to_string().as_bytes());
```

### Step 3: Send file in chunks

```rust
let mut file = File::open(path)?;
let mut buf = [0u8; 4096];
let mut nonce_counter = 1u64; // 0 was metadata

loop {
    let n = file.read(&mut buf)?;
    if n == 0 { break; }
    send_encrypted_with_nonce(&mut stream, &c2s_key, &buf[..n], nonce_counter);
    nonce_counter += 1;
}
```

### Step 4: Receive and verify

```rust
// Receive metadata
let metadata: Value = serde_json::from_slice(&recv_encrypted(&mut stream, &c2s_key)?)?;
let expected_hash = metadata["sha256"].as_str().unwrap();
let expected_size = metadata["size"].as_u64().unwrap();

// Receive chunks, compute hash
let mut hasher = Sha256::new();
let mut total = 0u64;

while total < expected_size {
    let chunk = recv_encrypted(&mut stream, &c2s_key)?;
    hasher.update(&chunk);
    output_file.write_all(&chunk)?;
    total += chunk.len() as u64;
}

let actual_hash = hex::encode(hasher.finalize());
assert_eq!(actual_hash, expected_hash, "File integrity check failed!");
```

## Try it with existing tools

```sh
# Compare with scp (uses SSH/TLS under the hood):
scp myfile.txt user@server:/tmp/

# Or with netcat (NO encryption — plaintext!):
# Receiver: nc -l 9000 > received.txt
# Sender:   nc 127.0.0.1 9000 < myfile.txt
# Your tool does what netcat does, but encrypted.
```

## Exercises

### Exercise 1: Basic transfer

Implement send/receive for a single file. Verify SHA-256 matches on both sides.

### Exercise 2: Progress bar

Show transfer progress: bytes sent, percentage, speed (MB/s), ETA.

### Exercise 3: Multiple files

Send a list of files (or a directory) in one session. Send metadata for each, then the data.

### Exercise 4: Resume interrupted transfer

If the connection drops mid-transfer, the receiver reports how many bytes it received. On reconnect, the sender skips already-sent chunks. This requires storing state on both sides.

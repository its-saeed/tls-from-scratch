# Lesson 7: Encrypted Echo Server (no authentication)

## What we're building

A TCP echo server and client that communicate over an encrypted channel. This combines Lessons 2, 4, and 5 into a working protocol:

1. **Key exchange** (Lesson 4): X25519 Diffie-Hellman
2. **Key derivation** (Lesson 5): HKDF to produce two independent keys
3. **Encrypted messaging** (Lesson 2): ChaCha20-Poly1305 with length-prefixed framing

This is essentially a simplified TLS session — without authentication (that's Lesson 8).

## The protocol

```
Client                                    Server
  │                                         │
  │──── client_public (32 bytes) ─────────►│   Handshake: raw bytes,
  │◄──── server_public (32 bytes) ─────────│   no framing needed (fixed size)
  │                                         │
  │  shared = DH(my_secret, their_public)   │   Both sides compute independently
  │  c2s_key = HKDF(shared, "c2s")          │   Both derive the same two keys
  │  s2c_key = HKDF(shared, "s2c")          │
  │                                         │
  │── [2B len][12B nonce][ciphertext] ────►│   Encrypted with c2s_key
  │◄── [2B len][12B nonce][ciphertext] ────│   Encrypted with s2c_key
  │                                         │
```

### Why two different keys?

If both directions used the same key, an attacker could **reflect** messages: capture an encrypted message from client→server and send it back to the client. The client would successfully decrypt it (same key) and think the server sent it.

With separate keys: a message encrypted with `c2s_key` can only be decrypted by someone who has `c2s_key`. If an attacker reflects it back to the client, the client tries to decrypt with `s2c_key` — it fails.

### Message format

Each encrypted message on the wire looks like:

```
┌─────────┬──────────┬────────────────────────────┐
│ 2 bytes │ 12 bytes │ N + 16 bytes               │
│ length  │  nonce   │ ciphertext + auth tag       │
└─────────┴──────────┴────────────────────────────┘
         └── length covers nonce + ciphertext ──┘
```

The 2-byte length prefix tells the receiver how many bytes to read. Without it, TCP is a byte stream — the receiver has no way to know where one message ends and the next begins.

## Real-world scenario: Alice and Bob's encrypted chat

Alice and Bob want to chat privately. Eve is monitoring the network.

### The handshake

1. Alice generates an ephemeral X25519 key pair
2. Alice sends her 32-byte public key to Bob
3. Bob generates an ephemeral X25519 key pair
4. Bob sends his 32-byte public key to Alice
5. Both compute: `shared_secret = DH(my_secret, their_public)`
6. Both derive: `c2s_key = HKDF(shared, "c2s")` and `s2c_key = HKDF(shared, "s2c")`

Eve sees both public keys on the wire. She cannot compute the shared secret (Lesson 4).

### Encrypted communication

7. Alice types "meet at 3pm"
8. Alice generates a random 12-byte nonce
9. Alice encrypts: `ciphertext = ChaCha20Poly1305(c2s_key, nonce, "meet at 3pm")`
10. Alice sends: `[length][nonce][ciphertext]`
11. Bob reads the length, reads that many bytes, splits nonce and ciphertext
12. Bob decrypts: `plaintext = ChaCha20Poly1305(c2s_key, nonce, ciphertext)` → "meet at 3pm"
13. Bob echoes back, encrypted with `s2c_key`

Eve sees: `[0x00 0x1d][random 12 bytes][random-looking bytes]`. She can see the message length (29 bytes = 12 nonce + 13 plaintext + 4... wait, actually 12 + 11 + 16 = 39). She knows a message was sent and its approximate size, but not the content.

### What Eve CAN still learn (traffic analysis)

Even with encryption, Eve can observe:
- **When** messages are sent (timing)
- **How large** each message is (length prefix is plaintext)
- **How many** messages are exchanged
- **Who** is talking to whom (IP addresses)

TLS has the same limitation. This is why some protocols add padding to obscure message sizes.

## The vulnerability: no authentication

This implementation is vulnerable to man-in-the-middle attacks.

```
Alice ←──DH──→ Mallory ←──DH──→ Bob (Server)
       key_A_M           key_M_B
```

Mallory intercepts Alice's public key, does her own DH with Alice (`key_A_M`) and a separate DH with Bob (`key_M_B`). She decrypts Alice's messages with `key_A_M`, reads them, re-encrypts with `key_M_B`, and forwards to Bob.

Lesson 8 fixes this by having the server sign its public key, proving its identity.

## Comparison with real TLS

| Feature | Our implementation | TLS 1.3 |
|---------|-------------------|---------|
| Key exchange | X25519 | X25519 or P-256 |
| Key derivation | HKDF-SHA256 | HKDF-SHA256 or SHA384 |
| Encryption | ChaCha20-Poly1305 | ChaCha20-Poly1305 or AES-GCM |
| Authentication | None | Certificates + signatures |
| Nonce | Random per message | Counter (sequence number) |
| Handshake | 1-RTT (2 messages) | 1-RTT (2 flights) |
| Session resumption | No | 0-RTT with PSK |
| Record framing | 2-byte length | 2-byte length + type + version |

Our protocol is structurally similar to TLS 1.3 — just stripped down to the essentials.

## Exercises

### Exercise 1: Encrypted echo (implemented in 7-echo-server.rs and 7-echo-client.rs)
Build the server and client as described above. Type messages in the client, see them echoed back.

### Exercise 2: Graceful disconnection
The current implementation panics when the client disconnects. Make `recv_encrypted` return a `Result` and handle EOF gracefully — server prints "client disconnected" and waits for a new connection.

### Exercise 3: Counter nonce
Replace random nonces with a counter. Each side maintains a `u64` counter starting at 0, incremented after each message. Encode it as the last 8 bytes of the 12-byte nonce (first 4 bytes = 0). This is what TLS does — it guarantees uniqueness without relying on randomness.

### Exercise 4: Bidirectional chat
Modify the client to not just send-then-receive, but handle both directions concurrently. Use threads or async: one thread reads from stdin and sends, another reads from the server and prints. This makes it a real chat application.

### Exercise 5: Wireshark capture
Run the echo server/client and capture traffic with:
```sh
sudo tcpdump -i lo0 port 7878 -w capture.pcap
```
Open in Wireshark. You'll see the TCP stream with:
- First 32 bytes: client's DH public key (plaintext)
- Next 32 bytes: server's DH public key (plaintext)
- Everything after: encrypted messages (random-looking)

Compare this with a plaintext TCP echo server — the difference is visible.

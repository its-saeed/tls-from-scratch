# Lesson 13: TLS Handshake Deep Dive

> **Prerequisites**: Lessons 9-12 (you've built a mini-TLS). Now see how the real TLS 1.3 handshake works.

## Real-life analogy: the diplomatic meeting

Two diplomats meeting for the first time:

```
Step 1 — Introductions (ClientHello / ServerHello):
  "I speak English, French, or German"    → cipher suites
  "I prefer to meet at my embassy"        → extensions (SNI)
  "Here's my proposed meeting protocol"   → key exchange group
  "Let's use this secret handshake"       → key share

Step 2 — Credentials (Certificate):
  "Here's my diplomatic passport"         → server certificate
  "Signed by the UN"                      → CA chain
  "Here's proof I'm really me"            → CertificateVerify (signature)

Step 3 — Agreement (Finished):
  "I confirm everything we discussed"     → handshake transcript hash
  "Let's begin the real conversation"     → application data
```

All of this happens in **one round trip** in TLS 1.3.

## The TLS 1.3 handshake

```
Client                                           Server
  │                                                │
  │──── ClientHello ─────────────────────────────►│
  │  • Protocol version: TLS 1.3                   │
  │  • Random: 32 bytes                            │
  │  • Cipher suites: [ChaCha20, AES-256-GCM]     │
  │  • Key share: X25519 public key                │
  │  • SNI: "example.com"                          │
  │  • ALPN: ["h2", "http/1.1"]                    │
  │                                                │
  │◄──── ServerHello ─────────────────────────────│
  │  • Cipher suite: ChaCha20-Poly1305 (chosen)   │
  │  • Key share: X25519 public key (server's)     │
  │                                                │
  │  ══════ ENCRYPTED FROM HERE ══════════════════ │
  │  (keys derived from DH shared secret)          │
  │                                                │
  │◄──── EncryptedExtensions ─────────────────────│
  │  • ALPN: "h2" (chosen)                         │
  │                                                │
  │◄──── Certificate ─────────────────────────────│
  │  • Server's X.509 certificate chain            │
  │                                                │
  │◄──── CertificateVerify ───────────────────────│
  │  • Signature over handshake transcript         │
  │                                                │
  │◄──── Finished ────────────────────────────────│
  │  • HMAC over handshake transcript              │
  │                                                │
  │──── Finished ─────────────────────────────────►│
  │  • HMAC over handshake transcript              │
  │                                                │
  │◄═══════════ Application Data ═══════════════► │
```

### Key insight: 1-RTT

TLS 1.3 completes the handshake in **one round trip** (1-RTT). The client sends its key share in ClientHello — no need to wait for the server to pick a group first. Compare with TLS 1.2 which needed 2-RTT.

## Cipher suite negotiation

The client offers a list; the server picks one:

```
Client offers:                        Server picks:
  TLS_CHACHA20_POLY1305_SHA256         ✓ (selected)
  TLS_AES_256_GCM_SHA384
  TLS_AES_128_GCM_SHA256
```

A TLS 1.3 cipher suite specifies:
- **AEAD cipher**: ChaCha20-Poly1305 or AES-GCM (your Lesson 2)
- **Hash**: SHA-256 or SHA-384 (your Lesson 1)
- **Key exchange**: always ephemeral DH (your Lesson 4) — not part of the cipher suite name

```sh
# See what cipher suites a server supports:
echo | openssl s_client -connect google.com:443 2>/dev/null | grep "Cipher"
# Cipher    : TLS_AES_256_GCM_SHA384

# List all TLS 1.3 cipher suites your OpenSSL supports:
openssl ciphers -v -tls1_3
```

## Extensions

Extensions carry additional information in the handshake:

### SNI (Server Name Indication)

```
ClientHello extension:
  server_name: "example.com"
```

Tells the server which hostname the client wants. Essential for virtual hosting — one IP serving multiple HTTPS sites.

```sh
# Connect with explicit SNI:
openssl s_client -connect 93.184.216.34:443 -servername example.com

# Connect WITHOUT SNI — might get wrong cert or error:
openssl s_client -connect 93.184.216.34:443
```

**Privacy note**: SNI is sent in **plaintext** in ClientHello. Anyone watching the network sees which site you're connecting to. Encrypted Client Hello (ECH) aims to fix this.

### ALPN (Application-Layer Protocol Negotiation)

```
ClientHello extension:
  alpn: ["h2", "http/1.1"]

ServerHello extension:
  alpn: "h2"
```

Negotiates the application protocol. Used for HTTP/2 (`h2`) vs HTTP/1.1 upgrade.

```sh
# Request HTTP/2 via ALPN:
openssl s_client -connect google.com:443 -alpn h2 2>/dev/null | grep "ALPN"
# ALPN protocol: h2
```

### Key share

The client sends its DH public key directly in ClientHello (TLS 1.3's big improvement over 1.2):

```
ClientHello extension:
  key_share: x25519 public key (32 bytes)

ServerHello extension:
  key_share: x25519 public key (32 bytes)
```

Both sides compute the shared secret immediately. No extra round trip.

## The key schedule

After DH, the shared secret goes through HKDF (your Lesson 5) to derive all session keys:

```
DH shared secret
       │
       ▼
  HKDF-Extract(salt=0, shared_secret) → handshake_secret
       │
       ├── HKDF-Expand("c hs traffic", transcript_hash)
       │   → client_handshake_key + IV
       │
       ├── HKDF-Expand("s hs traffic", transcript_hash)
       │   → server_handshake_key + IV
       │
       └── HKDF-Extract(handshake_secret, 0) → master_secret
              │
              ├── HKDF-Expand("c ap traffic", transcript_hash)
              │   → client_application_key + IV
              │
              └── HKDF-Expand("s ap traffic", transcript_hash)
                  → server_application_key + IV
```

**Handshake keys** encrypt the Certificate, CertificateVerify, and Finished messages.
**Application keys** encrypt the actual data (HTTP requests, etc.).

The **transcript hash** is a hash of all handshake messages sent so far. This binds the keys to the specific handshake — an attacker can't mix and match messages from different handshakes.

## The handshake transcript

Every handshake message is hashed together:

```
transcript_hash = SHA-256(
    ClientHello ||
    ServerHello ||
    EncryptedExtensions ||
    Certificate ||
    CertificateVerify ||
    server Finished
)
```

This hash appears in:
- **Key derivation**: the transcript is an input to HKDF-Expand
- **CertificateVerify**: the server signs the transcript hash
- **Finished**: both sides HMAC the transcript hash

If an attacker modifies ANY handshake message, the transcript hash changes, and everything fails: keys don't match, signatures don't verify, Finished messages don't validate.

## Watch a real handshake

```sh
# Full handshake trace:
openssl s_client -connect example.com:443 -msg 2>&1 | head -50
# Shows raw bytes of each handshake message

# Wireshark-style breakdown:
openssl s_client -connect example.com:443 -state 2>&1 | grep "SSL_connect"
# SSL_connect:before SSL initialization
# SSL_connect:SSLv3/TLS write client hello
# SSL_connect:SSLv3/TLS read server hello
# SSL_connect:SSLv3/TLS read change cipher spec
# ...

# Detailed certificate chain:
openssl s_client -connect example.com:443 -showcerts 2>/dev/null | \
  openssl x509 -text -noout | head -30

# See the negotiated parameters:
echo | openssl s_client -connect example.com:443 2>/dev/null | \
  grep -E "Protocol|Cipher|Server public key|Peer signing"
```

```sh
# Capture a handshake with tcpdump and view in Wireshark:
sudo tcpdump -i en0 -w tls-handshake.pcap host example.com and port 443 &
curl -s https://example.com > /dev/null
kill %1

# Open tls-handshake.pcap in Wireshark:
# Filter: tls.handshake
# You'll see ClientHello, ServerHello, etc. with all extensions decoded
```

## TLS 1.3 vs 1.2

```
                        TLS 1.2              TLS 1.3
──────────────────────────────────────────────────────
Round trips              2-RTT               1-RTT
Key exchange             RSA or DHE          DHE only (forward secrecy)
Cipher suites            ~100+               5 (simplified)
Encryption starts        After Finished      After ServerHello
Static RSA               Allowed             Removed
Compression              Allowed             Removed (CRIME attack)
Renegotiation            Allowed             Removed
0-RTT resumption         No                  Yes (with replay risk)
```

## Exercises

### Exercise 1: Trace a handshake

Use `openssl s_client -state -connect google.com:443` to see each handshake step. Identify: ClientHello, ServerHello, Certificate, Finished. What cipher suite was negotiated?

### Exercise 2: SNI experiment

Connect to a shared hosting server (like Cloudflare) with different SNI values:
```sh
openssl s_client -connect 104.16.0.0:443 -servername example.com
openssl s_client -connect 104.16.0.0:443 -servername different-site.com
```
Compare the certificates returned. Same IP, different certs — SNI in action.

### Exercise 3: Cipher suite restriction

In your tokio-rustls server (Lesson 14), configure it to only accept ChaCha20-Poly1305. Connect with a client that only offers AES-GCM. The handshake should fail. Then allow both — it should succeed.

### Exercise 4: Transcript binding

In your mini-TLS (Lesson 10), modify the authenticated echo server to sign the **full transcript** (both DH public keys concatenated) instead of just the server's DH key. Show that a replayed ServerHello from a different session is rejected because the transcript doesn't match.

### Exercise 5: Wireshark capture

Capture a TLS handshake with `tcpdump`, open in Wireshark. Identify each message type. Observe that everything after ServerHello is encrypted — you can see the handshake structure but not Certificate or Finished contents.

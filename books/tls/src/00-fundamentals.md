# Lesson 0: Cryptography Fundamentals

> **The Story of Alice's Bookstore**
>
> Alice runs a small online bookstore from her apartment. She sells rare books and ships worldwide. Her customers type their credit card numbers into her website, send her messages, and download digital books.
>
> One day, her friend Bob — a security researcher — sits down with her at a coffee shop.
>
> *"Alice, your website sends everything in plaintext. Anyone on this Wi-Fi can see your customers' credit cards."*
>
> *"What? How?"*
>
> *"Let me show you the problems — and how cryptography solves each one."*
>
> This is where our story begins. Each lesson solves one of Alice's problems. By the end, her bookstore will be fully secure.

Before writing any code, you need to understand the vocabulary and core concepts. Every lesson that follows builds on these ideas.

## Real-life analogy: sending a secret letter

Imagine you need to send a secret letter across town. Every cryptographic concept maps to a part of this scenario:

```
┌──────────────────────────────────────────────────────────────┐
│  The Secret Letter Problem                                   │
│                                                              │
│  You (Alice) want to send a letter to Bob.                   │
│  The mail carrier (the network) might read it.               │
│                                                              │
│  Confidentiality → put the letter in a locked box            │
│  Integrity       → seal the box with tamper-evident tape     │
│  Authentication  → stamp it with your wax seal               │
│                                                              │
│  Key             → the key to the locked box                 │
│  Hash            → a fingerprint of the letter               │
│  Signature       → your wax seal (only you have the ring)    │
│  Nonce           → a unique serial number on each box        │
│  Certificate     → a passport proving you are Alice          │
│                                                              │
│  Without these:                                              │
│    Anyone can read the letter (no confidentiality)           │
│    Anyone can change the letter (no integrity)               │
│    Anyone can pretend to be you (no authentication)          │
└──────────────────────────────────────────────────────────────┘
```

## The three goals of cryptography

### 1. Confidentiality

Only the intended recipient can read the message. Everyone else sees random noise.

```
Alice writes: "meet at 3pm"
Alice encrypts: "meet at 3pm" → "x7#kQ!9pL@"
Eve intercepts: "x7#kQ!9pL@" (meaningless)
Bob decrypts: "x7#kQ!9pL@" → "meet at 3pm"
```

**Without confidentiality**: anyone on the network (ISP, router, attacker on the same Wi-Fi) can read your emails, passwords, bank transfers, medical records.

### 2. Integrity

The message hasn't been modified in transit. If anyone changes even one bit, the recipient detects it.

```
Alice sends: "transfer $100 to Bob"
Eve intercepts, modifies: "transfer $999 to Eve"
Bob receives it — but integrity check FAILS → message rejected
```

**Without integrity**: an attacker can silently alter messages. Change a dollar amount, modify a software update to include malware, alter DNS responses to redirect traffic.

### 3. Authentication

You know who you're talking to. The sender is who they claim to be.

```
Alice receives a message claiming to be from her bank.
Authentication check: is this really from the bank, or from an attacker pretending to be the bank?
```

**Without authentication**: phishing, man-in-the-middle attacks, impersonation. An attacker sets up a fake bank website — without authentication, your browser can't tell the difference.

## Core terminology

### Plaintext and ciphertext

- **Plaintext**: the original, readable data. Doesn't have to be text — could be a file, image, or any bytes.
- **Ciphertext**: the encrypted, unreadable version. Looks like random bytes. Same length as plaintext (roughly).
- **Encryption**: plaintext → ciphertext (using a key)
- **Decryption**: ciphertext → plaintext (using a key)

```
plaintext: "hello world"
     │
     ▼ encrypt(key)
ciphertext: 0x7a3f8b2e1c...
     │
     ▼ decrypt(key)
plaintext: "hello world"
```

### Keys

A key is a secret value that controls encryption and decryption. Without the key, decryption is computationally impossible.

- **Symmetric key**: one key for both encryption and decryption. Both sides must share the same key.
- **Asymmetric key pair**: two keys — a public key (shared openly) and a private key (kept secret). What one encrypts, only the other can decrypt.

```
Symmetric:
  Alice and Bob both have key K
  Alice: encrypt(K, plaintext) → ciphertext
  Bob:   decrypt(K, ciphertext) → plaintext

Asymmetric:
  Bob has: public key (shared) + private key (secret)
  Alice: encrypt(Bob_public, plaintext) → ciphertext
  Bob:   decrypt(Bob_private, ciphertext) → plaintext
```

### Cipher

An algorithm that performs encryption and decryption. Examples:
- **AES** (Advanced Encryption Standard): symmetric, block cipher
- **ChaCha20**: symmetric, stream cipher
- **RSA**: asymmetric

The cipher is public — security comes from the key, not from keeping the algorithm secret. This is **Kerckhoffs's principle**: a system should be secure even if everything about it is public knowledge except the key.

### Hash / Digest

A fixed-size fingerprint of any data. One-way — you can't reverse it.

```
SHA-256("hello") → 2cf24dba5fb0a30e...  (always 32 bytes)
SHA-256("hello ") → 98ea6e4f216f2fb4... (completely different)
```

Used for: integrity verification, password storage, key derivation, digital signatures.

### Nonce

"Number used once." A value that must never repeat with the same key. Used in encryption to ensure that encrypting the same plaintext twice produces different ciphertext.

```
encrypt(key, nonce=1, "hello") → 0x8a3f...
encrypt(key, nonce=2, "hello") → 0x2b7c...  (different!)
```

If you reuse a nonce with the same key, the encryption breaks — an attacker can recover plaintext. This is one of the most common crypto implementation mistakes.

### Digital signature

The asymmetric equivalent of a handwritten signature. Proves who created a message and that it hasn't been modified.

```
Alice signs:   signature = sign(Alice_private_key, message)
Bob verifies:  verify(Alice_public_key, message, signature) → true/false
```

If the message changes, verification fails. If someone else tries to sign, they can't produce a valid signature without Alice's private key.

### MAC (Message Authentication Code)

A MAC is a short piece of data (a **tag**) that proves two things about a message:
1. **Integrity** — the message wasn't modified
2. **Authenticity** — the message came from someone who knows the secret key

Think of it as a tamper-evident seal that only works with a secret:

```
Creating a MAC:
  Alice has: message + shared_key
  Alice computes: tag = MAC(shared_key, "transfer $100")
  Alice sends: message + tag

Verifying a MAC:
  Bob has: message + tag + shared_key
  Bob computes: expected_tag = MAC(shared_key, "transfer $100")
  Bob checks: tag == expected_tag?
    YES → message is authentic and unmodified
    NO  → message was tampered with or wrong key

Attacker (no key):
  Eve intercepts: message + tag
  Eve changes message to "transfer $900"
  Eve can't compute new tag (doesn't have the key)
  Bob checks → tag doesn't match → REJECTED
```

**MAC vs Hash**: a plain hash (Lesson 1) proves integrity but NOT authenticity — anyone can compute `SHA-256("transfer $900")` and replace the hash. A MAC requires the secret key, so only key holders can produce valid tags.

**MAC vs Signature**: a MAC uses a **symmetric** key (both sides share the same key). A signature uses an **asymmetric** key pair. MAC is faster but doesn't prove *which* key holder created it (both sides have the same key). Signatures prove exactly who signed.

```
           MAC                    Signature
           ───                    ─────────
Key:       shared symmetric key   asymmetric key pair
Creates:   anyone with the key    only private key holder
Verifies:  anyone with the key    anyone with the public key
Proves:    "a key holder made it" "THIS person made it"
Speed:     fast                   slower
Used in:   TLS record integrity   TLS handshake authentication
```

**HMAC**: the most common MAC construction — built from a hash function (e.g., HMAC-SHA256). "Hash-based MAC." Used extensively in TLS for key derivation (HKDF, Lesson 5) and handshake integrity.

### AEAD (Authenticated Encryption with Associated Data)

Modern encryption that provides both confidentiality AND integrity in one operation. You get:
- **Ciphertext**: encrypted data (confidentiality)
- **Authentication tag**: proves the ciphertext wasn't modified (integrity)

"Associated data" is metadata that's authenticated but not encrypted (e.g., a message header).

```
encrypt(key, nonce, plaintext, associated_data) → (ciphertext, tag)
decrypt(key, nonce, ciphertext, tag, associated_data) → plaintext OR error
```

If anyone modifies the ciphertext, the tag, or the associated data, decryption fails.

## Authentication vs Authorization

These are different concepts that are often confused:

- **Authentication** (AuthN): "Who are you?" — verifying identity
  - Example: logging in with username/password, presenting a certificate
- **Authorization** (AuthZ): "What are you allowed to do?" — verifying permissions
  - Example: "user X can read this file but not write to it"

TLS handles **authentication** (proving the server is who it claims to be). It does NOT handle authorization — that's the application's job.

```
TLS handshake: "I am server.example.com" (authentication)
Application:   "User alice can access /admin" (authorization)
```

## Trust models

How do you decide to trust a public key?

### Direct trust (pinned keys)
You manually verify and store the public key. Simple but doesn't scale.
- Example: SSH `known_hosts`, WireGuard peer configuration

### Web of trust
People vouch for each other's keys. Decentralized but messy.
- Example: PGP/GPG key signing parties

### Certificate authority (CA)
A trusted third party vouches for public keys by signing certificates. Hierarchical and scalable.
- Example: HTTPS (Let's Encrypt, DigiCert sign server certificates)

### Trust on first use (TOFU)
Accept the key the first time you see it, alert if it changes.
- Example: SSH ("The authenticity of host can't be established... continue?")

## Forward secrecy

If an attacker records all your encrypted traffic today, and steals your long-term key next year, can they decrypt the recorded traffic?

- **Without forward secrecy**: Yes. The long-term key decrypts everything.
- **With forward secrecy**: No. Each session used ephemeral keys that were destroyed. The long-term key can't help.

TLS 1.3 mandates forward secrecy by requiring ephemeral Diffie-Hellman key exchange.

## The cast of characters

Cryptography literature uses standard names:

| Name | Role |
|------|------|
| **Alice** | Initiator (usually the client) |
| **Bob** | Responder (usually the server) |
| **Eve** | Eavesdropper — passively listens to traffic |
| **Mallory** | Active attacker — can modify, inject, and replay messages |
| **Trent** | Trusted third party (e.g., a Certificate Authority) |

## Common attacks

```
Attack              Who         What they do             Defense
─────────────────────────────────────────────────────────────────────
Eavesdropping       Eve         Listens to traffic       Encryption
Man-in-the-middle   Mallory     Intercepts + modifies    Authentication
Replay              Mallory     Re-sends old messages    Nonces / sequence #
Tampering           Mallory     Modifies ciphertext      AEAD / MAC
Downgrade           Mallory     Forces weak crypto       Signed handshake
```

### Eavesdropping (passive)
Eve listens to network traffic. Defeated by encryption (confidentiality).

```sh
# See how easy eavesdropping is on unencrypted traffic:
# Terminal 1: start a plaintext HTTP server
python3 -m http.server 8000 &

# Terminal 2: capture traffic
sudo tcpdump -i lo0 port 8000 -A 2>/dev/null &

# Terminal 3: make a request
curl http://127.0.0.1:8000/

# tcpdump shows EVERYTHING in plaintext — the full HTTP request and response.
# This is why HTTPS exists.
kill %1 %2 2>/dev/null
```

### Man-in-the-middle / MITM (active)
Mallory sits between Alice and Bob, impersonating each to the other. Defeated by authentication.

```
Alice ←──────→ Mallory ←──────→ Bob
  "Hi Bob"   →  reads it  →  "Hi Bob"
  "Hi Alice" ←  reads it  ←  "Hi Alice"

Both think they're talking to each other.
Mallory reads and modifies everything.
```

### Replay attack
Mallory records a valid encrypted message and sends it again later. Defeated by sequence numbers or timestamps.

```
Alice sends:  "transfer $100" (encrypted, valid)
Mallory records it.
... 1 hour later ...
Mallory sends the same bytes again.
Server processes it → $100 transferred AGAIN.
```

### Tampering
Mallory modifies an encrypted message in transit. Defeated by integrity checks (AEAD, MAC).

### Downgrade attack
Mallory forces Alice and Bob to use weaker crypto than they'd normally choose. Defeated by signing the handshake negotiation.

## See it in the real world

Every concept in this lesson is happening right now on your machine:

```sh
# See a real TLS handshake — every concept in action:
echo | openssl s_client -connect google.com:443 2>/dev/null | head -20
# You'll see: certificate chain (authentication), cipher suite (encryption),
# protocol version, key exchange algorithm

# See WHICH cipher suite was negotiated:
echo | openssl s_client -connect google.com:443 2>/dev/null | grep "Cipher"
# Example: TLS_AES_256_GCM_SHA384
# That's: AEAD cipher (AES-GCM) + hash (SHA384)

# See the certificate (authentication):
echo | openssl s_client -connect google.com:443 2>/dev/null | \
  openssl x509 -noout -subject -issuer
# subject: CN = *.google.com  ← who they claim to be
# issuer: CN = GTS CA 1C3     ← who vouches for them (CA)

# See forward secrecy in action:
echo | openssl s_client -connect google.com:443 2>/dev/null | grep "Server Temp Key"
# Server Temp Key: X25519  ← ephemeral DH key exchange = forward secrecy
```

```sh
# See encryption protecting YOUR traffic right now:
# Capture some HTTPS traffic:
sudo tcpdump -i en0 -c 10 host google.com and port 443 -w /tmp/tls.pcap 2>/dev/null &
curl -s https://google.com > /dev/null
sleep 2 && kill %1 2>/dev/null

# Look at the raw bytes — all encrypted:
tcpdump -r /tmp/tls.pcap -X 2>/dev/null | tail -20
# You see hex garbage — that's AEAD encryption at work.
# Without the key, nobody can read it. Not your ISP, not the Wi-Fi owner.
```

```sh
# See HMAC (integrity) — on your own machine:
# Your SSH known_hosts uses HMAC to hash hostnames:
cat ~/.ssh/known_hosts | head -3
# Some lines start with |1|... — that's HMAC-hashed hostnames

# Package managers verify integrity with hashes:
# macOS:
shasum -a 256 $(which ls)
# The OS verified this hash when the binary was installed
```

## How TLS uses all of this

```
TLS Handshake:
  1. Negotiate cipher suite          (which algorithms to use)
  2. Key exchange (DH)               (confidentiality + forward secrecy)
  3. Server certificate              (authentication via CA trust model)
  4. Server signature                (proves server has the private key)
  5. Key derivation (HKDF)           (derive session keys)
  6. Finished messages               (integrity of handshake — MAC)

TLS Record Protocol:
  7. AEAD encryption of data         (confidentiality + integrity)
  8. Sequence number nonces           (replay defense)
```

```
Every concept → a lesson:

  Concept              Lesson    What you'll build
  ──────────────────────────────────────────────────
  Hash                 1         SHA-256 file hasher
  Symmetric encryption 2         ChaCha20-Poly1305
  Signatures           3         Ed25519 sign/verify
  Key exchange         4         X25519 Diffie-Hellman
  Key derivation       5         HKDF from shared secret
  Password KDFs        6         Argon2 / PBKDF2
  Certificates         7         X.509 parsing
  Cert generation      8         Build a CA with rcgen
  Mini-TLS             9-12      Encrypted echo server
  TLS handshake        13        Protocol deep dive
  Real TLS             14-15     tokio-rustls + HTTPS
```

Every concept in this lesson maps to a specific part of TLS. The following lessons implement each piece in Rust.

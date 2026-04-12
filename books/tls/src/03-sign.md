# Lesson 3: Asymmetric Crypto & Signatures (Ed25519)

> **Alice's Bookstore — Chapter 3**
>
> Alice starts emailing order confirmations to customers. One day, a customer named Dave calls:
>
> *"I got an email from 'Alice's Bookstore' saying my order was cancelled and asking me to re-enter my credit card on a new link. Is that real?"*
>
> It wasn't. Someone — let's call her Mallory — sent a fake email pretending to be Alice. The email looked identical to Alice's real ones. Dave almost entered his credit card on Mallory's phishing site.
>
> *"How can my customers tell that a message is really from ME and not a fake?"*
>
> Bob explains: *"You need a digital signature. You sign your messages with a private key that only you have. Anyone can verify the signature using your public key. Mallory can't forge it because she doesn't have your private key."*
>
> *"So it's like a wax seal that only I can stamp?"*
>
> *"Exactly."*

## Real-life analogy: the wax seal

In medieval times, kings sealed letters with a wax stamp pressed from a unique signet ring:

```
King's ring (private key):
  Only the king has it. Never leaves his finger.

Wax impression (signature):
  Anyone can SEE it and verify it matches the king's seal.
  Nobody can FORGE it without the ring.

Royal seal catalog (public key):
  Everyone knows what the king's seal looks like.
  They compare the wax impression against the catalog.

  ┌──────────┐     ┌───────────┐     ┌───────────┐
  │ Letter   │     │ Wax seal  │     │ Catalog   │
  │ "attack  │ +   │ (made with│     │ (king's   │
  │  at dawn"│     │  ring)    │     │  known    │
  └──────────┘     └───────────┘     │  seal)    │
       │                │             └─────┬─────┘
       └────────┬───────┘                   │
                ▼                           ▼
         Does the seal match?          Compare!
         Was the letter modified?      ✓ or ✗
```

Digital signatures work the same way: sign with private key, verify with public key.

## The problem symmetric crypto can't solve

In Lesson 2, both sides need the same key. But how do you share it? You can't send it over the network — anyone watching would see it. You can't encrypt it — you'd need another key for that (chicken-and-egg).

Asymmetric crypto solves this with **key pairs**:
- **Private key**: kept secret, never leaves your machine
- **Public key**: given to everyone

## Two uses of key pairs

```
                    Private Key              Public Key
                    (secret)                 (shared with everyone)
─────────────────────────────────────────────────────────────────
Encryption:         decrypt                  encrypt
                    Only you can read        Anyone can send you
                    messages to you          encrypted messages

Signatures:         sign                     verify
                    Only you can sign        Anyone can check
                    (proves authorship)      your signature
```

### 1. Encryption (less common in modern TLS)
- Encrypt with someone's public key → only their private key can decrypt
- Used in older TLS (RSA key exchange), but NOT in TLS 1.3

### 2. Digital signatures (critical in TLS)
- Sign with your private key → anyone with your public key can verify
- Proves two things:
  - **Authenticity**: "this message was created by the private key holder"
  - **Integrity**: "this message hasn't been modified since signing"

## Try it yourself

```sh
# Generate an Ed25519 key pair with OpenSSL:
openssl genpkey -algorithm Ed25519 -out private.pem
openssl pkey -in private.pem -pubout -out public.pem

# Look at the keys:
cat private.pem   # PEM-encoded private key
cat public.pem    # PEM-encoded public key

# Sign a file:
echo "important document" > doc.txt
openssl pkeyutl -sign -inkey private.pem -in doc.txt -out doc.sig

# Verify the signature:
openssl pkeyutl -verify -pubin -inkey public.pem -in doc.txt -sigfile doc.sig
# Signature Verified Successfully

# Tamper with the document and verify again:
echo "modified document" > doc.txt
openssl pkeyutl -verify -pubin -inkey public.pem -in doc.txt -sigfile doc.sig
# Signature Verification Failure
```

```sh
# See SSH host keys (Ed25519 is typically one of them):
ls -la /etc/ssh/ssh_host_*key*
# ssh_host_ed25519_key      ← private key (permissions: 600)
# ssh_host_ed25519_key.pub  ← public key

# See your SSH known_hosts (server public keys you've trusted):
cat ~/.ssh/known_hosts | head -3

# See your own SSH public key:
cat ~/.ssh/id_ed25519.pub 2>/dev/null || echo "No Ed25519 SSH key found"

# Generate one if you don't have it:
# ssh-keygen -t ed25519
```

## Ed25519

A modern signature algorithm based on elliptic curves (Curve25519). Designed by Daniel Bernstein.

```
sign(private_key, message) → signature (64 bytes)
verify(public_key, message, signature) → true/false
```

```
┌─────────────────────────────────────────────────────────┐
│  Ed25519 at a glance                                    │
│                                                         │
│  Private key:   32 bytes                                │
│  Public key:    32 bytes (derived from private key)     │
│  Signature:     64 bytes                                │
│  Speed:         ~15,000 signatures/second               │
│  Deterministic: yes (no random nonce needed)            │
│                                                         │
│  Used by: SSH, WireGuard, Signal, TLS, git signing,    │
│           cargo, age encryption, minisign               │
└─────────────────────────────────────────────────────────┘
```

Key property: **deterministic**. Same key + same message → same signature every time. Unlike ECDSA, there's no random nonce — which means no nonce reuse bugs (recall the PS3 disaster from Lesson 2).

## Real-world scenarios

### Alice signs a software release

Alice publishes open-source software. Users need to verify downloads are genuinely from Alice, not an attacker who compromised the download mirror.

1. Alice generates an Ed25519 key pair. Publishes her public key on her website.
2. Alice builds version 2.0, signs the binary: `sign(alice_private, binary) → sig`
3. Alice uploads `binary` and `sig` to the download mirror
4. Bob downloads both. He has Alice's public key from her website.
5. Bob runs `verify(alice_public, binary, sig)` → success
6. An attacker modifies the binary on the mirror. Bob downloads it.
7. Bob runs `verify(alice_public, modified_binary, sig)` → **FAILS**

Bob knows the binary was tampered with. This is exactly how `apt` (Debian/Ubuntu) and `cargo` verify packages.

### Bob authenticates to a server (SSH)

When you run `ssh server.com`, the server proves its identity:

1. Server has a long-term Ed25519 key pair (in `/etc/ssh/ssh_host_ed25519_key`)
2. During SSH handshake, server signs session data with its private key
3. Client verifies the signature against the server's known public key (in `~/.ssh/known_hosts`)
4. If verification fails → "WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!"

This prevents MITM attacks: an attacker can't forge the server's signature without its private key.

### How TLS uses signatures

During the TLS handshake:
1. Server sends its ephemeral DH public key (for key exchange, Lesson 4)
2. Server signs the handshake transcript (all messages so far) with its long-term private key
3. Client verifies the signature using the server's public key from the certificate (Lesson 6)
4. If the signature is valid → the client knows the DH public key genuinely came from the server
5. An attacker can't forge this because they don't have the server's private key

Without this signature, an attacker could substitute their own DH public key (man-in-the-middle attack).

### Ed25519 vs RSA vs ECDSA

```
Algorithm    Key size     Sig size    Speed        Nonce risk
──────────────────────────────────────────────────────────────
RSA-2048     256 bytes    256 bytes   Slow         No
ECDSA P-256  64 bytes     64 bytes    Medium       YES (fatal!)
Ed25519      32 bytes     64 bytes    Fast         No (deterministic)
```

- **RSA**: oldest, huge keys, being phased out. Still used by many CAs.
- **ECDSA**: smaller keys, but has a dangerous nonce. If the random nonce leaks or is reused, the **private key can be recovered**. This happened to Sony's PS3 signing key (2010).
- **Ed25519**: smallest keys, fastest, deterministic (no nonce footgun). The modern choice.

```sh
# Benchmark signing speed on your machine:
openssl speed ed25519 ecdsa rsa2048 2>/dev/null | grep -E 'sign|verify'
```

### How TLS uses signatures

```
TLS Handshake:
  ┌────────┐                              ┌────────┐
  │ Client │                              │ Server │
  └───┬────┘                              └───┬────┘
      │                                       │
      │◄── server's DH public key ────────────│
      │◄── server's certificate ──────────────│
      │◄── signature over handshake ──────────│  ← signed with server's
      │                                       │    private key
      │                                       │
      │  Client verifies:                     │
      │    1. Certificate → trusted CA?       │
      │    2. Signature → matches public key? │
      │    3. Both pass → server is genuine   │
      │                                       │
      │  Without signature:                   │
      │    Attacker substitutes own DH key    │
      │    → man-in-the-middle!               │
```

## Exercises

### Exercise 1: Sign and verify (implemented in 3-sign.rs)
Generate a key pair, sign a message, verify it. Then modify the message and show verification fails.

### Exercise 2: Sign multiple messages
Sign three different messages with the same key. Verify each with the corresponding message. Then try verifying message 1's signature against message 2 — it should fail. Each signature is bound to its specific message.

### Exercise 3: Key separation
Generate two different key pairs. Sign the same message with both. Show that:
- Key A's signature verifies with Key A's public key
- Key A's signature does NOT verify with Key B's public key
- Key B's signature does NOT verify with Key A's public key

This demonstrates that signatures are bound to both the message AND the signer's identity.

### Exercise 4: Detached signatures (real-world pattern)
Simulate a software release workflow:
1. Create a "binary" (any byte array)
2. Sign it, save the signature to a separate "file" (`Vec<u8>`)
3. In a separate function (simulating a different machine), load the "binary" and "signature", verify against a hardcoded public key

### Exercise 5: Verify on the command line

Generate keys and sign a file entirely from the CLI, then verify in your Rust program:

```sh
# Generate key pair:
openssl genpkey -algorithm Ed25519 -out key.pem
openssl pkey -in key.pem -pubout -out pub.pem

# Sign:
echo -n "verify me" > msg.txt
openssl pkeyutl -sign -inkey key.pem -in msg.txt -out msg.sig

# Now write Rust code that reads pub.pem and msg.sig, verifies msg.txt
```

This bridges the CLI tools with your Rust code — the same keys and signatures work in both.

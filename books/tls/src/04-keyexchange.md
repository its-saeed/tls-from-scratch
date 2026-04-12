# Lesson 4: Diffie-Hellman Key Exchange (X25519)

> **Alice's Bookstore — Chapter 4**
>
> Alice now understands encryption (Lesson 2) — she can scramble data so Eve can't read it. But there's a chicken-and-egg problem:
>
> *"To encrypt, my customer and I need the same secret key. But how do we agree on a key? If I send it over the network, Eve sees it. If I encrypt it... I need a key to encrypt the key. It's turtles all the way down!"*
>
> Bob smiles: *"This is the most elegant trick in cryptography. You and the customer can agree on a shared secret by exchanging messages in public — and even if Eve records every single byte you send, she STILL can't figure out the secret."*
>
> *"That sounds impossible."*
>
> *"It's called Diffie-Hellman. Let me show you with paint."*

## Real-life analogy: mixing paint

Alice and Bob want to agree on a shared secret color. Eve is watching everything they send.

```
Public:  Both agree on base color: YELLOW

Alice (secret: RED)              Bob (secret: BLUE)
  │                                │
  │ mix RED + YELLOW → ORANGE      │ mix BLUE + YELLOW → GREEN
  │                                │
  ├──── sends ORANGE ────────────►│
  │◄──── sends GREEN ─────────────┤
  │                                │
  │ mix GREEN + RED → BROWN        │ mix ORANGE + BLUE → BROWN
  │                                │
  └── shared secret: BROWN ────────┘── shared secret: BROWN

Eve sees: YELLOW, ORANGE, GREEN
Eve CANNOT unmix paint to get BROWN
```

This is Diffie-Hellman. Replace "colors" with "math" and it's the real thing.

## The core problem

Alice and Bob want to encrypt their communication (Lesson 2), but they need a shared secret key. They can't send it in plaintext — Eve is watching the network. They can't encrypt it — that requires a key they don't have yet (chicken-and-egg).

## The magic trick (paint analogy)

1. Alice and Bob publicly agree on a base color: **yellow**
2. Alice picks a secret color: **red**. Mixes red + yellow → **orange**. Sends orange to Bob.
3. Bob picks a secret color: **blue**. Mixes blue + yellow → **green**. Sends green to Alice.
4. Alice mixes Bob's green + her secret red → **brown**
5. Bob mixes Alice's orange + his secret blue → **same brown**

Eve sees yellow, orange, and green — but can't unmix paint to get brown. That's Diffie-Hellman.

## The math (simplified)

With numbers and modular arithmetic:

```
Public parameters: p = 23 (prime), g = 5 (generator)

Alice                               Bob
picks secret a = 6                  picks secret b = 15

A = g^a mod p                      B = g^b mod p
A = 5^6 mod 23 = 8                 B = 5^15 mod 23 = 19

sends A = 8 ──────────────────►    receives A = 8
receives B = 19 ◄──────────────    sends B = 19

shared = B^a mod p                 shared = A^b mod p
       = 19^6 mod 23 = 2                 = 8^15 mod 23 = 2
```

Both get **2**. Why?
```
Alice: B^a = (g^b)^a = g^(b*a) mod p
Bob:   A^b = (g^a)^b = g^(a*b) mod p
a*b == b*a, so they're equal.
```

Eve sees g=5, p=23, A=8, B=19. To find the shared secret, she'd need to solve `5^a mod 23 = 8` for `a` — the **discrete logarithm problem**. With small numbers it's trivial, but with 256-bit numbers it's computationally infeasible.

## X25519: the modern version

Instead of `g^a mod p`, X25519 uses **elliptic curve point multiplication**:
- Secret key `a` = random 32 bytes
- Public key `A` = `a * G` (multiply base point G on Curve25519 by scalar a)
- Shared secret = `a * B` = `a * (b * G)` = `b * (a * G)` = `b * A`

Same principle, different math. Elliptic curves give equivalent security with much smaller keys (32 bytes vs 2048+ bytes for classic DH).

## Try it yourself

```sh
# Generate an X25519 key pair with OpenSSL:
openssl genpkey -algorithm X25519 -out alice_private.pem
openssl pkey -in alice_private.pem -pubout -out alice_public.pem

openssl genpkey -algorithm X25519 -out bob_private.pem
openssl pkey -in bob_private.pem -pubout -out bob_public.pem

# Derive the shared secret (Alice's side):
openssl pkeyutl -derive -inkey alice_private.pem \
  -peerkey bob_public.pem -out shared_alice.bin

# Derive the shared secret (Bob's side):
openssl pkeyutl -derive -inkey bob_private.pem \
  -peerkey alice_public.pem -out shared_bob.bin

# Verify they match:
xxd shared_alice.bin
xxd shared_bob.bin
# Same 32 bytes! Alice and Bob derived the same secret.
```

```sh
# See what key exchange a real TLS connection uses:
echo | openssl s_client -connect google.com:443 2>/dev/null | grep -i "Server Temp Key"
# Server Temp Key: X25519, 253 bits

# See the full handshake showing key exchange:
echo | openssl s_client -connect example.com:443 -state 2>&1 | grep -i "key"
```

```sh
# Verify DH with Python (small numbers, for learning):
python3 -c "
p, g = 23, 5
a, b = 6, 15  # secrets

A = pow(g, a, p)  # Alice's public: 5^6 mod 23 = 8
B = pow(g, b, p)  # Bob's public: 5^15 mod 23 = 19

shared_alice = pow(B, a, p)  # 19^6 mod 23 = 2
shared_bob   = pow(A, b, p)  # 8^15 mod 23 = 2

print(f'Alice public: {A}, Bob public: {B}')
print(f'Alice shared: {shared_alice}, Bob shared: {shared_bob}')
print(f'Match: {shared_alice == shared_bob}')
"
```

## Real-world scenarios

### Alice and Bob establish an encrypted chat session

Alice and Bob have never communicated before. They want to set up end-to-end encryption.

1. Alice generates an ephemeral X25519 key pair: `(alice_secret, alice_public)`
2. Bob generates an ephemeral X25519 key pair: `(bob_secret, bob_public)`
3. Alice sends `alice_public` (32 bytes) to Bob over the internet
4. Bob sends `bob_public` (32 bytes) to Alice over the internet
5. Alice computes: `shared = alice_secret.dh(bob_public)` → 32-byte secret
6. Bob computes: `shared = bob_secret.dh(alice_public)` → same 32-byte secret
7. Both use this shared secret as an encryption key (or derive keys via HKDF, Lesson 5)
8. Both destroy their ephemeral secrets

Eve recorded all traffic. She has `alice_public` and `bob_public`. She cannot compute the shared secret.

### Forward secrecy in TLS

Every TLS connection generates fresh ephemeral keys:

1. Monday: Client and server do DH → `shared_1`. Encrypt traffic. Destroy ephemeral keys.
2. Tuesday: Client and server do DH → `shared_2`. Encrypt traffic. Destroy ephemeral keys.
3. Wednesday: Attacker compromises the server's long-term private key.

The attacker recorded Monday's and Tuesday's encrypted traffic. Can they decrypt it? **No.** The ephemeral DH keys are gone. `shared_1` and `shared_2` can never be reconstructed. This is **forward secrecy**.

Without ephemeral DH (old RSA key exchange): the attacker uses the long-term key to decrypt ALL past traffic. This is why TLS 1.3 removed RSA key exchange entirely.

```
With ephemeral DH (TLS 1.3):        Without (old RSA):
  Mon: DH → key_1 → destroyed         Mon: RSA decrypt → key_1
  Tue: DH → key_2 → destroyed         Tue: RSA decrypt → key_2
  Wed: attacker gets long-term key     Wed: attacker gets RSA key
       ↓                                    ↓
  Can decrypt Mon traffic? NO           Can decrypt Mon? YES
  Can decrypt Tue traffic? NO           Can decrypt Tue? YES
  Keys are gone forever.                All past traffic exposed.
```

### WireGuard's Noise protocol

WireGuard uses X25519 for both:
- **Static keys**: long-term identity (like a certificate)
- **Ephemeral keys**: per-session (forward secrecy)

The handshake does multiple DH operations: static-static, static-ephemeral, ephemeral-ephemeral. This gives authentication AND forward secrecy in one round trip.

## The man-in-the-middle problem

DH alone does NOT authenticate. Mallory (attacker) can intercept:

```
Alice                   Mallory                  Bob
  │                        │                       │
  ├── alice_pub ──────────►│                       │
  │                        ├── mallory_pub1 ──────►│
  │                        │◄── bob_pub ───────────┤
  │◄── mallory_pub2 ──────┤                       │
  │                        │                       │
  │ shared_AM              │ shared_AM, shared_MB  │ shared_MB
  │ (Alice↔Mallory)        │ (can read EVERYTHING) │ (Mallory↔Bob)
  │                        │                       │
  │ Thinks she's           │ Decrypts, reads,      │ Thinks he's
  │ talking to Bob         │ re-encrypts, forwards │ talking to Alice
```

Mallory does two separate key exchanges. She reads everything. Neither side knows.

This is why Lessons 3 and 7 (signatures and certificates) are necessary — they authenticate the DH public keys.

## Exercises

### Exercise 1: Key exchange (implemented in 4-keyexchange.rs)
Simulate Alice and Bob. Generate ephemeral keys, exchange public keys, compute shared secrets. Print both — they must match.

### Exercise 2: Ephemeral means unique
Run the key exchange three times. Print the shared secret each time. All three should be different — demonstrating that each session gets a unique key.

### Exercise 3: Wrong public key
Alice does DH with Bob's public key. Charlie does DH with Bob's public key using a DIFFERENT secret. Show that Alice and Charlie get different shared secrets — only the matching pair produces the same result.

### Exercise 4: Simulate man-in-the-middle
Implement Mallory intercepting the exchange:
1. Alice generates keys, sends `alice_public` to Mallory (thinking it's Bob)
2. Mallory generates her own keys, sends `mallory_public` to Alice (pretending to be Bob)
3. Mallory sends `mallory_public2` to Bob (pretending to be Alice)
4. Bob sends `bob_public` to Mallory
5. Mallory now has two different shared secrets: one with Alice, one with Bob
6. Show that Alice's shared secret != Bob's shared secret (they're not talking to each other)

# Design Challenges

These challenges test your ability to **combine** cryptographic primitives into real solutions. There's no single correct answer — the goal is to think through the design, identify trade-offs, and build a working prototype.

## How to approach each challenge

1. **Read the scenario** — understand what the system needs to do
2. **List the requirements** — what properties must hold (confidentiality, integrity, authentication?)
3. **Pick your primitives** — which lessons apply? (hashing, encryption, signatures, key exchange, certificates?)
4. **Draw the protocol** — what messages are exchanged? What does each party store?
5. **Implement** — build a working prototype in Rust
6. **Break it** — try to attack your own design. What assumptions did you make?

---

## Beginner (after Phases 1-2)

### Challenge 1: Tamper-Proof Audit Log

**Scenario**: You run a financial service. Regulators require that your audit logs cannot be modified after the fact. An employee with database access should not be able to silently delete or change a log entry.

**Requirements**:
- Each log entry contains: timestamp, action, user, data
- Each entry is linked to the previous one (if any entry is modified, everything after it becomes invalid)
- Anyone can verify the log hasn't been tampered with

**Constraints**:
- The log is stored in a plain file or database (no special hardware)
- You cannot use a blockchain network

<details>
<summary>Hints</summary>

- Lesson 1 (Hashing): each entry includes the hash of the previous entry
- This creates a **hash chain** — modifying entry N changes its hash, which breaks entry N+1's chain
- This is how Git works internally (each commit hashes the previous commit)
- This is also the core idea behind blockchains

</details>

**Design questions**:
- What happens if the attacker modifies the LAST entry? (There's no next entry to break)
- How could you publish the latest hash somewhere immutable (a tweet, a newspaper ad, a blockchain)?
- What if you need to verify just one entry without checking the entire chain? (→ Merkle tree)

**Verification**: modify any entry in the middle of the log. Show that verification fails from that point forward.

---

### Challenge 2: Password Breach Checker

**Scenario**: You want to check if your password appeared in a data breach (like haveibeenpwned.com). But you don't want to send your password to anyone — not even the checking service.

**Requirements**:
- The service has a database of 600 million breached password hashes
- You check if your password is breached WITHOUT revealing it to the service
- The service doesn't learn your password, even partially

**Constraints**:
- You cannot download the entire 600M hash database (it's 30GB)
- You must send something to the service

<details>
<summary>Hints</summary>

- Lesson 1 (Hashing): hash your password with SHA-1 (that's what haveibeenpwned uses)
- Send only the first 5 hex characters of the hash (k-anonymity)
- The service returns ALL hashes that start with those 5 characters (~500 hashes)
- You check locally if your full hash is in the returned list

</details>

**Design questions**:
- The service sees the first 5 chars of your hash. How much does this reveal? (1/16^5 = 1 in ~1M)
- Could the service be malicious and log the prefix + your IP to narrow it down?
- Try it yourself: `curl https://api.pwnedpasswords.com/range/$(echo -n "password" | shasum -a 1 | cut -c1-5)`

---

### Challenge 3: Sealed Bid Auction

**Scenario**: Three companies are bidding on a government contract. Each must submit a bid without knowing the others' bids. After the deadline, all bids are revealed simultaneously. No one can change their bid after seeing others'.

**Requirements**:
- Bids are secret until the reveal phase
- No bid can be changed after submission
- Everyone can verify that the revealed bid matches the committed bid

**Constraints**:
- There's no trusted third party
- The reveal happens on a public channel

<details>
<summary>Hints</summary>

- Lesson 1 (Hashing): commitment scheme — commit = hash(bid + random_nonce)
- Each bidder publishes their commitment before the deadline
- After deadline, each reveals bid + nonce
- Everyone verifies: hash(bid + nonce) == commitment

</details>

**Design questions**:
- Why is the random nonce needed? (Without it, someone could hash all possible bid amounts and find yours)
- What if a bidder refuses to reveal? (They forfeit, but the auction can still proceed)
- What if two bidders collude — one reveals first, the other adjusts?

---

### Challenge 4: Secure Dead Drop

**Scenario**: Alice wants to leave an encrypted message for Bob on a public server (like a pastebin). They've never communicated before, but Bob has published his public key on his personal website.

**Requirements**:
- Only Bob can read the message
- The server cannot read the message
- Alice doesn't need Bob to be online when she sends

**Constraints**:
- No real-time key exchange (Bob is offline)
- The message sits on a public server anyone can see

<details>
<summary>Hints</summary>

- Lesson 4 (Key Exchange): Alice generates an ephemeral X25519 key pair
- Alice computes shared_secret = DH(alice_ephemeral_secret, bob_public)
- Lesson 5 (HKDF): derive encryption key from shared secret
- Lesson 2 (Encryption): encrypt the message with ChaCha20-Poly1305
- Alice posts: her ephemeral public key + encrypted message
- Bob computes: shared_secret = DH(bob_secret, alice_ephemeral_public)
- Bob derives the same key and decrypts

</details>

**Design questions**:
- This is called ECIES (Elliptic Curve Integrated Encryption Scheme). Where is it used in practice?
- What if Alice wants Bob to know the message is from her? (Add a signature — Lesson 3)
- What if the server modifies the ephemeral public key? (Bob derives wrong key → decryption fails → integrity via AEAD)

---

### Challenge 5: File Deduplication Without Reading Content

**Scenario**: You're building a cloud storage service. When two users upload the same file, you want to store it only once (deduplication). But you want to detect duplicates WITHOUT the server reading file contents.

**Requirements**:
- Server detects duplicate files
- Server never sees file contents (files are encrypted client-side)
- Different users uploading the same file → stored once

**Constraints**:
- Files are encrypted before upload
- Random encryption keys would make identical files look different

<details>
<summary>Hints</summary>

- Lesson 1 (Hashing): derive the encryption key FROM the file content: `key = HKDF(SHA-256(file_content))`
- Same file → same hash → same key → same ciphertext
- Upload: `(file_hash, encrypted_content)` — server deduplicates by hash
- This is called **convergent encryption**

</details>

**Design questions**:
- Security problem: if the server knows the file might be "secret-document.pdf", they can hash it and check. How bad is this?
- What about files that differ by one byte? No dedup possible.
- Real-world: Dropbox uses a form of this. What are the privacy implications?

---

## Intermediate (after Phase 3)

### Challenge 6: Multi-Signature Contract

**Scenario**: A company requires 3 of 5 board members to sign a document before it's valid (like a multi-sig Bitcoin wallet). No single person can approve alone.

**Requirements**:
- 5 board members each have an Ed25519 key pair
- A document is valid only with 3+ signatures
- Anyone can verify the document is properly signed

**Constraints**:
- No trusted central server
- Board members sign independently (not at the same time)

<details>
<summary>Hints</summary>

- Lesson 3 (Signatures): each member signs the document independently
- Attach all signatures to the document: `[(pubkey_1, sig_1), (pubkey_2, sig_2), ...]`
- Verifier checks: at least 3 valid signatures from known board member public keys
- Store the list of authorized public keys somewhere trusted

</details>

**Design questions**:
- How do you handle a board member leaving? (Revoke their public key)
- What if the document is modified after 2 of 3 sign? (Remaining signature fails — integrity)
- Could you do threshold signatures (one combined 64-byte signature instead of 3 separate ones)?

---

### Challenge 7: Multiplayer Game Anti-Cheat

**Scenario**: Two players play rock-paper-scissors over the network. Neither should be able to cheat by seeing the other's move before committing their own.

**Requirements**:
- Both commit to their move simultaneously
- After both commit, both reveal
- Cheating (changing your move after seeing the other's) is detectable

**Constraints**:
- No trusted server — peer-to-peer only
- Network latency means messages aren't truly simultaneous

<details>
<summary>Hints</summary>

- Lesson 1 (Hashing): commit = hash(move + random_nonce)
- Phase 1: both send their commitment (hash)
- Phase 2: both reveal move + nonce
- Phase 3: both verify opponent's commitment matches
- If anyone changed their move, the hash won't match

</details>

**Design questions**:
- What if player B sees player A's commitment and tries every possible move (only 3 options) to find a match? (That's why the nonce is essential — makes it impossible to reverse)
- Can this scale to poker? (52 cards = more complex, needs mental poker protocol)
- What if one player refuses to reveal? (They forfeit after a timeout)

---

### Challenge 8: Whistleblower Drop

**Scenario**: A journalist runs a secure drop server. Sources can submit documents anonymously. The source needs to verify they're talking to the real journalist (not law enforcement impersonating them), but the journalist must NOT know who the source is.

**Requirements**:
- Source verifies the journalist's identity (one-way authentication)
- Journalist cannot identify the source
- Messages can't be replayed
- Submitted documents are encrypted in transit

**Constraints**:
- Source does NOT have a key pair (anonymous)
- Only the journalist has identity keys

<details>
<summary>Hints</summary>

- Lesson 10 (Authenticated Echo): one-way authentication — journalist signs, source verifies
- Lesson 4 (Key Exchange): ephemeral DH for encryption (source generates throwaway keys)
- Lesson 12 (Replay Defense): sequence numbers prevent replay
- Source generates a fresh ephemeral key each session → no linkability between sessions

</details>

**Design questions**:
- How is this similar to SecureDrop (used by NY Times, Washington Post)?
- What metadata can still leak? (IP address, timing, file size)
- How would Tor help here?

---

### Challenge 9: Secure Software Update Pipeline

**Scenario**: You ship IoT thermostats. Each device checks for firmware updates over the internet. A compromised update could brick millions of devices or install malware.

**Requirements**:
- Device verifies the update is from your company (not an attacker)
- Update hasn't been tampered with
- Update is newer than the installed version (prevent rollback)
- Update is encrypted in transit

**Constraints**:
- Devices have limited CPU/memory
- Devices are in customers' homes (physical access possible)
- The update server might be compromised

<details>
<summary>Hints</summary>

- Lesson 3 (Signatures): sign the firmware with your company's Ed25519 key
- Lesson 1 (Hashing): include firmware hash in a signed metadata file
- Lesson 7 (Certificates): embed the company's public key in the device at factory
- Version number in signed metadata prevents rollback
- Lesson 14 (Real TLS): encrypt transit with TLS

</details>

**Design questions**:
- What if your signing key is compromised? (Key rotation — embed next key in current update)
- What if the update server is hacked? (Attacker can serve old signed updates — that's why version checking matters)
- What if someone captures and replays an old update to rollback a device?

---

### Challenge 10: Exam Integrity System

**Scenario**: Students take an online exam. You want to prevent cheating by ensuring: (1) the professor can't see answers before the deadline, (2) students can't change answers after the deadline, (3) all answers are revealed simultaneously.

**Requirements**:
- Students encrypt their answers before the deadline
- Professor cannot decrypt until after the deadline
- Students cannot modify their answers post-deadline
- Professor can verify each student's answer wasn't changed

**Constraints**:
- Students don't trust the professor (might peek early)
- Professor doesn't trust the students (might change answers)

<details>
<summary>Hints</summary>

- Phase 1 (before deadline): student encrypts answers with a random key, submits ciphertext + hash of key
- Phase 2 (after deadline): student reveals the key
- Professor decrypts with the key, verifies hash matches
- Student can't change answer (ciphertext already submitted)
- Professor can't peek (doesn't have the key until reveal)

</details>

**Design questions**:
- What if a student doesn't reveal their key? (Marked as zero — same as not submitting)
- What if the professor colludes with a student to leak the exam early? (Separate problem — this protocol only handles answer integrity)
- How would you add anonymity? (Student submits through a mix network)

---

## Advanced (after Phases 4-5)

### Challenge 11: Encrypted DNS Resolver

**Scenario**: Your ISP logs every DNS query you make (they can see you visited `bank.com` even with HTTPS). Build a DNS client that sends queries over TLS to a trusted resolver.

**Requirements**:
- DNS queries are encrypted in transit (your ISP can't see them)
- The resolver is authenticated (you verify its certificate)
- Responses can't be tampered with

**Constraints**:
- Must follow the DNS-over-TLS (DoT) spec: TLS on port 853
- The DNS wire format is binary (not HTTP)

<details>
<summary>Hints</summary>

- Lesson 14 (tokio-rustls): wrap a TCP connection to `1.1.1.1:853` with TLS
- DNS wire format: 2-byte length prefix + DNS message (similar to your tunnel framing!)
- Send a DNS query for `example.com`, parse the response
- Verify: compare results with `dig example.com @1.1.1.1`

</details>

**Design questions**:
- What metadata still leaks? (The IP of the DNS resolver — your ISP sees you're using 1.1.1.1)
- How does DNS-over-HTTPS (DoH) differ? (Uses HTTP/2 on port 443, looks like normal web traffic)
- Could you combine this with encrypted SNI (ECH) for full privacy?

---

### Challenge 12: API Rate Limiter with Client Certificates

**Scenario**: You run a paid API. Free users get 100 requests/hour, Pro users get 10,000. You want to authenticate and rate-limit using mTLS — the client's certificate contains their tier.

**Requirements**:
- Clients authenticate with certificates signed by your CA
- The certificate contains the tier (free/pro/enterprise) in a custom extension or the CN
- Server reads the tier and enforces rate limits
- No API keys, no tokens — just the cert

**Constraints**:
- Client certs are issued by your CA (Lesson 8)
- Tier cannot be changed by the client (it's signed by the CA)

<details>
<summary>Hints</summary>

- P7 (CA): issue certs with CN like `free:alice` or `pro:bob`
- Lesson 11 (mTLS): require client cert on the server
- After handshake, extract CN from client cert → parse tier
- Rate limit based on tier

</details>

**Design questions**:
- What if a free user shares their cert+key with others? (Track cert serial number, revoke if abused)
- How do you upgrade a user from free to pro? (Issue new cert, revoke old one)
- How does this compare to JWT tokens? (Certs are verified at TLS level — no application code needed)

---

### Challenge 13: Secure File Sharing with Expiring Links

**Scenario**: Alice uploads a file. She gets a link she can share with anyone. The link works for 24 hours. The server stores the file but CANNOT read its contents.

**Requirements**:
- File is encrypted client-side before upload
- The decryption key is in the link fragment (never sent to server)
- Link expires after 24 hours
- Server stores ciphertext only

**Constraints**:
- Server must not have the decryption key
- Anyone with the link can decrypt (no pre-shared keys)

<details>
<summary>Hints</summary>

- Lesson 2 (Encryption): encrypt file with a random key client-side
- Upload ciphertext to server, get a file ID
- Link format: `https://share.example.com/files/abc123#key=hexencodedkey`
- The `#fragment` is never sent to the server (browser rule)
- Server handles expiry (delete after 24h)
- Recipient's browser downloads ciphertext, extracts key from fragment, decrypts

</details>

**Design questions**:
- This is how Firefox Send worked (before Mozilla shut it down). Why did they shut it down?
- What if the server is malicious and serves modified JavaScript? (Could steal the key)
- How would you add password protection on top? (PBKDF2 on the password → wrap the file key)

---

### Challenge 14: Forward-Secret Chat (Double Ratchet)

**Scenario**: Alice and Bob chat. Each message should use a unique key. If message #47's key leaks, messages #1-46 and #48+ remain secure. This is how Signal works.

**Requirements**:
- Each message is encrypted with a different key
- Compromising one message key reveals nothing about others
- Keys automatically "ratchet" forward after each message

**Constraints**:
- No server holds any keys
- Must work asynchronously (Alice sends 5 messages before Bob reads any)

<details>
<summary>Hints</summary>

- Lesson 4 (DH): periodically do new DH exchanges to ratchet the root key
- Lesson 5 (HKDF): derive message keys from a chain: `chain_key_n+1 = HKDF(chain_key_n, "chain")`
- Each message key is derived then discarded
- New DH exchange → new root key → new chain → all old keys are unrecoverable

</details>

**Design questions**:
- How many DH ratchet steps per message? (Signal: every time the sender changes, not every message)
- What about message ordering? (Signal uses message counters within each chain)
- This is the Signal Protocol — used by Signal, WhatsApp, Facebook Messenger. Read the spec.

---

### Challenge 15: Cryptocurrency Wallet

**Scenario**: Build a simple wallet: generate key pairs, create "transactions" (Alice sends 10 coins to Bob), sign them, verify them.

**Requirements**:
- Key generation: Ed25519 key pair per user
- Transaction: `{from: pubkey, to: pubkey, amount: u64, signature: sig}`
- Signature covers: from + to + amount (prevents tampering)
- Wallet file encrypted with password (Lesson 6)

**Constraints**:
- No actual blockchain — just the signing/verification layer
- Focus on the cryptographic operations, not consensus

<details>
<summary>Hints</summary>

- Lesson 3 (Signatures): sign(sender_private, hash(from || to || amount))
- Lesson 6 (Password KDF): encrypt the wallet (private keys) with Argon2 + ChaCha20
- Lesson 1 (Hashing): transaction ID = hash of the transaction

</details>

**Design questions**:
- What prevents double-spending? (In real crypto: a blockchain ledger. Here: just the signature layer)
- What if someone steals the wallet file? (Password-encrypted, but attacker can brute-force offline)
- How does this compare to Bitcoin (secp256k1 + SHA-256) or Ethereum (secp256k1 + Keccak)?

# Project: Password Manager Vault

> **Prerequisites**: Lesson 2 (ChaCha20-Poly1305), Lesson 6 (Password-Based KDFs). Encrypt data at rest using a master password.

## What you're building

A CLI password manager that encrypts a JSON vault with a master password. Like a mini 1Password/Bitwarden.

```sh
# Initialize a new vault:
cargo run -p tls --bin p4-vault -- init
# Enter master password: ********
# Created vault.enc

# Add an entry:
cargo run -p tls --bin p4-vault -- add github
# Enter master password: ********
# Username: alice
# Password: s3cret123
# Saved.

# Retrieve an entry:
cargo run -p tls --bin p4-vault -- get github
# Enter master password: ********
# Username: alice
# Password: s3cret123

# List all entries:
cargo run -p tls --bin p4-vault -- list
# Enter master password: ********
# github, gmail, ssh-server
```

## How it works

```
Master password: "correct horse battery staple"
        │
        ▼
┌──────────────────────────────┐
│ Argon2id(password, salt)     │  Lesson 6: intentionally slow
│ → 32-byte encryption key     │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│ ChaCha20-Poly1305            │  Lesson 2: AEAD encryption
│ encrypt(key, nonce, vault)   │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│ vault.enc file on disk:      │
│ [16B salt][12B nonce][cipher]│
│                              │
│ Without the master password, │
│ this file is random bytes.   │
└──────────────────────────────┘
```

## File format

```
Bytes  0-15:  Argon2 salt (random, unique per vault)
Bytes 16-27:  ChaCha20-Poly1305 nonce (random, unique per save)
Bytes 28+:    Encrypted JSON + 16-byte auth tag

The JSON inside (decrypted):
{
  "entries": {
    "github": { "username": "alice", "password": "s3cret123" },
    "gmail":  { "username": "alice@gmail.com", "password": "p4ssw0rd" }
  }
}
```

## Implementation guide

### Step 1: Derive key from password

```rust
use argon2::Argon2;

fn derive_key(password: &[u8], salt: &[u8; 16]) -> [u8; 32] {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password, salt, &mut key)
        .unwrap();
    key
}
```

### Step 2: Encrypt the vault

```rust
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Aead, Nonce, Key};

fn encrypt_vault(key: &[u8; 32], plaintext: &[u8]) -> (Vec<u8>, [u8; 12]) {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext).unwrap();
    (ciphertext, nonce_bytes)
}
```

### Step 3: Save to disk

```rust
fn save_vault(path: &str, salt: &[u8; 16], nonce: &[u8; 12], ciphertext: &[u8]) {
    let mut file = Vec::new();
    file.extend_from_slice(salt);      // 16 bytes
    file.extend_from_slice(nonce);     // 12 bytes
    file.extend_from_slice(ciphertext); // N bytes
    std::fs::write(path, file).unwrap();
}
```

### Step 4: Load and decrypt

```rust
fn load_vault(path: &str, password: &[u8]) -> String {
    let data = std::fs::read(path).unwrap();
    let salt: [u8; 16] = data[..16].try_into().unwrap();
    let nonce: [u8; 12] = data[16..28].try_into().unwrap();
    let ciphertext = &data[28..];

    let key = derive_key(password, &salt);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let plaintext = cipher.decrypt(Nonce::from_slice(&nonce), ciphertext)
        .expect("Wrong password or corrupted vault");
    String::from_utf8(plaintext).unwrap()
}
```

## Security considerations

```
✓ Password → Argon2 → key (slow, memory-hard, brute-force resistant)
✓ Each save uses a fresh random nonce (no nonce reuse)
✓ AEAD auth tag detects tampering AND wrong password
✓ Salt is unique per vault (no rainbow tables)

✗ Master password in memory while running (use zeroize crate)
✗ No clipboard integration (passwords shown in terminal)
✗ No key stretching for vault-to-vault migration
✗ No multi-device sync
```

## Exercises

### Exercise 1: Basic vault

Implement init, add, get, list commands. Verify: wrong password gives "Wrong password" error (AEAD tag mismatch), not garbled output.

### Exercise 2: Password generator

Add a `generate` command that creates random passwords:
```sh
cargo run -p tls --bin p4-vault -- generate --length 20 --symbols
# Generated: k9$mP2@xL5#nQ8wR3&jY
```

### Exercise 3: Export/import

Add export (decrypt vault → JSON) and import (JSON → encrypted vault). Useful for backup/migration.

### Exercise 4: Zeroize sensitive data

Use the `zeroize` crate to wipe the master password and derived key from memory after use:
```rust
use zeroize::Zeroize;
let mut key = derive_key(&password, &salt);
// ... use key ...
key.zeroize(); // overwrites memory with zeros
```

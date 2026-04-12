# Project: Password Manager Vault

> **Prerequisites**: Lesson 2 (ChaCha20-Poly1305), Lesson 6 (Password-Based KDFs). This project directly applies what you just learned.

## What is a password vault?

Every time you create an account on a website, you need a unique, strong password. Nobody can remember 200 random passwords. A password manager solves this:

```
┌──────────────────────────────────────────────────────────┐
│  The problem:                                            │
│                                                          │
│  github.com      → need a password                       │
│  gmail.com       → need a different password             │
│  bank.com        → need another different password       │
│  ... × 200 sites                                         │
│                                                          │
│  Reusing passwords? One breach exposes all accounts.     │
│  Writing them down? Paper can be stolen/lost.            │
│                                                          │
│  The solution: password manager                          │
│                                                          │
│  One master password → unlocks a vault                   │
│  Vault contains all your passwords, encrypted            │
│  The vault file is useless without the master password   │
└──────────────────────────────────────────────────────────┘
```

This is how KeePass, 1Password, Bitwarden, and LastPass work. You're building a simplified version.

## What you're building

A CLI password manager. One file on disk, encrypted with your master password:

```sh
# Create a new vault:
cargo run -p tls --bin p4-vault -- init
# Enter master password: ********
# Created vault.enc

# Store a password:
cargo run -p tls --bin p4-vault -- add github
# Enter master password: ********
# Username: alice
# Password: s3cret!@#456
# Saved.

# Retrieve it later:
cargo run -p tls --bin p4-vault -- get github
# Enter master password: ********
# Username: alice
# Password: s3cret!@#456

# List all entries:
cargo run -p tls --bin p4-vault -- list
# Enter master password: ********
# github, gmail, ssh-server

# Wrong password:
cargo run -p tls --bin p4-vault -- list
# Enter master password: wrong-password
# Error: wrong password or corrupted vault
```

## How it works — the big picture

Two lessons combine:

```
Lesson 6 (Password KDF):          Lesson 2 (Encryption):
  master password → Argon2 → key     key + vault → encrypt → ciphertext

Together:
  "correct horse battery staple"
          │
          ▼
  ┌──────────────────────┐
  │ Argon2id             │   Slow on purpose (Lesson 6)
  │ password + salt → key│   Attacker can't brute-force
  └──────────┬───────────┘
             │
             ▼ 32-byte key
  ┌──────────────────────┐
  │ ChaCha20-Poly1305    │   AEAD encryption (Lesson 2)
  │ key + nonce + data   │   Confidentiality + integrity
  │ → ciphertext + tag   │
  └──────────┬───────────┘
             │
             ▼
  ┌──────────────────────┐
  │ vault.enc on disk    │   Random bytes without the password
  │ salt | nonce | cipher│
  └──────────────────────┘
```

## The vault file format

The `.enc` file is a simple binary format:

```
┌────────────┬────────────┬──────────────────────────────┐
│ Salt       │ Nonce      │ Encrypted JSON + auth tag    │
│ 16 bytes   │ 12 bytes   │ variable length              │
└────────────┴────────────┴──────────────────────────────┘

Salt:   random, generated once when vault is created
        Used by Argon2 to derive the key
        Not secret — just prevents rainbow tables

Nonce:  random, generated fresh every time the vault is saved
        Used by ChaCha20 for encryption
        Ensures re-saving with the same password produces different ciphertext

The JSON inside (after decryption):
{
  "entries": {
    "github": { "username": "alice", "password": "s3cret!@#456" },
    "gmail":  { "username": "alice@gmail.com", "password": "Tr0ub4d0r&3" }
  }
}
```

## Try it with existing tools first

```sh
# See how openssl encrypts a file with a password (uses PBKDF2 internally):
echo '{"github": {"user": "alice", "pass": "s3cret"}}' > vault.json
openssl enc -aes-256-cbc -salt -pbkdf2 -in vault.json -out vault.enc
# enter password

# Decrypt:
openssl enc -aes-256-cbc -d -salt -pbkdf2 -in vault.enc
# enter same password → JSON appears

# Wrong password:
openssl enc -aes-256-cbc -d -salt -pbkdf2 -in vault.enc
# enter wrong password → "bad decrypt" error

# Look at the encrypted file — random bytes:
xxd vault.enc | head -5

rm vault.json vault.enc
```

That's what we're building in Rust — but with Argon2 (stronger than PBKDF2) and ChaCha20-Poly1305 (modern AEAD).

## Implementation guide

### Step 0: Project setup

```sh
touch tls/src/bin/p4-vault.rs
```

Dependencies (add to `tls/Cargo.toml`):

```toml
argon2 = "0.5"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rpassword = "7"  # for reading passwords without echoing to terminal
```

CLI skeleton:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "vault", about = "Encrypted password vault")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Path to the vault file
    #[arg(long, default_value = "vault.enc")]
    vault: String,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new empty vault
    Init,
    /// Add a new entry
    Add { name: String },
    /// Retrieve an entry
    Get { name: String },
    /// List all entry names
    List,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Init => todo!(),
        Command::Add { name } => todo!(),
        Command::Get { name } => todo!(),
        Command::List => todo!(),
    }
}
```

```sh
cargo run -p tls --bin p4-vault -- --help
```

### Step 1: Read password from terminal (without echoing)

When you type a password, it shouldn't appear on screen:

```rust
fn ask_password(prompt: &str) -> String {
    rpassword::prompt_password(prompt).unwrap()
}

fn ask_input(prompt: &str) -> String {
    eprint!("{prompt}");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}
```

Test it:

```rust
fn main() {
    let password = ask_password("Master password: ");
    println!("You typed {} characters (not shown)", password.len());
}
```

```sh
cargo run -p tls --bin p4-vault -- init
# Master password: (you type, nothing appears)
# You typed 12 characters (not shown)
```

### Step 2: Derive encryption key from password

This is Lesson 6 in action:

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

Test it:

```rust
fn main() {
    let salt: [u8; 16] = rand::random();
    let key = derive_key(b"my-password", &salt);
    println!("Salt: {}", hex::encode(salt));
    println!("Key:  {}", hex::encode(key));

    // Same password + same salt = same key (deterministic):
    let key2 = derive_key(b"my-password", &salt);
    assert_eq!(key, key2);
    println!("Deterministic: ✓");

    // Different password = different key:
    let key3 = derive_key(b"wrong-password", &salt);
    assert_ne!(key, key3);
    println!("Different password = different key: ✓");
}
```

### Step 3: Encrypt and decrypt the vault

This is Lesson 2 in action:

```rust
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Aead, Nonce, Key};

fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> ([u8; 12], Vec<u8>) {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce_bytes: [u8; 12] = rand::random();
    let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .expect("encryption failed");
    (nonce_bytes, ciphertext)
}

fn decrypt(key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher.decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| "Wrong password or corrupted vault".to_string())
}
```

**Key insight**: when the password is wrong, Argon2 produces a different key, and ChaCha20's AEAD tag verification fails. The user sees "wrong password" — not garbled data. AEAD gives us this for free.

### Step 4: Save and load the vault file

```rust
fn save_vault(path: &str, salt: &[u8; 16], nonce: &[u8; 12], ciphertext: &[u8]) {
    let mut file_data = Vec::with_capacity(16 + 12 + ciphertext.len());
    file_data.extend_from_slice(salt);       // bytes 0-15
    file_data.extend_from_slice(nonce);      // bytes 16-27
    file_data.extend_from_slice(ciphertext); // bytes 28+
    std::fs::write(path, &file_data).unwrap();
}

fn load_vault(path: &str) -> ([u8; 16], [u8; 12], Vec<u8>) {
    let data = std::fs::read(path).expect("Can't read vault file");
    assert!(data.len() >= 28, "Vault file too small — corrupted?");

    let salt: [u8; 16] = data[..16].try_into().unwrap();
    let nonce: [u8; 12] = data[16..28].try_into().unwrap();
    let ciphertext = data[28..].to_vec();
    (salt, nonce, ciphertext)
}
```

Test the round-trip:

```sh
cargo run -p tls --bin p4-vault -- init
# Master password: ********
# Created vault.enc

ls -la vault.enc
# 44 bytes (16 salt + 12 nonce + 2 JSON "{}" + 16 auth tag = 46... close)

xxd vault.enc | head -3
# Random-looking bytes — vault is encrypted
```

### Step 5: The vault data structure

```rust
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default)]
struct Vault {
    entries: HashMap<String, Entry>,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    username: String,
    password: String,
}
```

### Step 6: Wire it all together

```rust
/// Open an existing vault (ask password, decrypt, parse JSON)
fn open_vault(path: &str) -> (Vault, [u8; 16]) {
    let password = ask_password("Master password: ");
    let (salt, nonce, ciphertext) = load_vault(path);
    let key = derive_key(password.as_bytes(), &salt);
    let plaintext = decrypt(&key, &nonce, &ciphertext)
        .expect("Wrong password or corrupted vault");
    let vault: Vault = serde_json::from_slice(&plaintext).unwrap();
    (vault, salt)
}

/// Save vault (re-encrypt with same salt, fresh nonce)
fn save(path: &str, vault: &Vault, salt: &[u8; 16], password: &str) {
    let key = derive_key(password.as_bytes(), salt);
    let json = serde_json::to_vec(vault).unwrap();
    let (nonce, ciphertext) = encrypt(&key, &json);
    save_vault(path, salt, &nonce, &ciphertext);
}
```

Now implement each command:

```rust
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Init => {
            let password = ask_password("Master password: ");
            let confirm = ask_password("Confirm password: ");
            if password != confirm {
                eprintln!("Passwords don't match!");
                return;
            }
            let salt: [u8; 16] = rand::random();
            let vault = Vault::default();
            save(&cli.vault, &vault, &salt, &password);
            println!("Created {}", cli.vault);
        }
        Command::Add { name } => {
            let password = ask_password("Master password: ");
            let (mut vault, salt) = open_vault(&cli.vault);
            let username = ask_input("Username: ");
            let entry_password = ask_input("Password: ");
            vault.entries.insert(name.clone(), Entry { username, password: entry_password });
            save(&cli.vault, &vault, &salt, &password);
            println!("Saved entry: {name}");
        }
        Command::Get { name } => {
            let (vault, _) = open_vault(&cli.vault);
            match vault.entries.get(&name) {
                Some(entry) => {
                    println!("Username: {}", entry.username);
                    println!("Password: {}", entry.password);
                }
                None => eprintln!("Entry '{name}' not found"),
            }
        }
        Command::List => {
            let (vault, _) = open_vault(&cli.vault);
            for name in vault.entries.keys() {
                println!("{name}");
            }
        }
    }
}
```

### Step 7: Test it end-to-end

```sh
# Create vault:
cargo run -p tls --bin p4-vault -- init
# Master password: test123
# Confirm password: test123
# Created vault.enc

# Add entries:
cargo run -p tls --bin p4-vault -- add github
# Master password: test123
# Username: alice
# Password: gh-s3cret!

cargo run -p tls --bin p4-vault -- add gmail
# Master password: test123
# Username: alice@gmail.com
# Password: gm-p@ssw0rd

# List:
cargo run -p tls --bin p4-vault -- list
# Master password: test123
# github
# gmail

# Get:
cargo run -p tls --bin p4-vault -- get github
# Master password: test123
# Username: alice
# Password: gh-s3cret!

# Wrong password:
cargo run -p tls --bin p4-vault -- list
# Master password: wrongpassword
# Error: Wrong password or corrupted vault

# Inspect the file — just random bytes:
xxd vault.enc | head -5
```

## Security considerations

```
What this vault gets RIGHT:
  ✓ Argon2id: brute-forcing the master password is impractical
  ✓ Random salt: prevents rainbow table attacks
  ✓ Fresh nonce per save: re-saving doesn't reuse nonces
  ✓ AEAD tag: wrong password → clean error, not garbled data
  ✓ AEAD tag: tampered file → clean error, not corrupted data

What a REAL password manager also does:
  ✗ Zeroize keys in memory after use (zeroize crate)
  ✗ Lock the vault after a timeout
  ✗ Clipboard integration (copy password, auto-clear after 30s)
  ✗ Browser extension for auto-fill
  ✗ Sync across devices (Bitwarden uses a server, KeePass uses file sync)
  ✗ Backup/recovery (what if you forget the master password?)
```

## Exercises

### Exercise 1: Basic vault

Implement all the steps above. Test with the commands shown in Step 7.

### Exercise 2: Password generator

Add a `generate` command that creates random passwords:

```sh
cargo run -p tls --bin p4-vault -- generate --length 20 --symbols
# Generated: k9$mP2@xL5#nQ8wR3&jY

# Or generate and save in one step:
cargo run -p tls --bin p4-vault -- add github --generate --length 16
# Master password: ********
# Username: alice
# Generated password: xK7mN2pQ9rW4tY6a
# Saved.
```

### Exercise 3: Export / import

Add export (decrypt → JSON file) and import (JSON file → encrypted vault):

```sh
cargo run -p tls --bin p4-vault -- export --out backup.json
# Master password: ********
# Exported 5 entries to backup.json (PLAINTEXT — delete after use!)

cargo run -p tls --bin p4-vault -- import --in backup.json
# New master password: ********
# Imported 5 entries.
```

### Exercise 4: Zeroize sensitive data

The master password and derived key sit in memory while the program runs. Use the `zeroize` crate to wipe them immediately after use:

```rust
use zeroize::Zeroize;

let mut password = ask_password("Master password: ");
let mut key = derive_key(password.as_bytes(), &salt);
// ... use key ...
key.zeroize();       // [0, 0, 0, 0, ... 0]
password.zeroize();  // ""
// Memory no longer contains sensitive data
```

Why this matters: if the process crashes or is swapped to disk, the password/key could be recovered from a memory dump.

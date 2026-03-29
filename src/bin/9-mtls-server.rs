// Lesson 9: Mutual TLS (mTLS)
// Both client and server authenticate each other using Ed25519 signatures.

use std::io::{Read, Write};
use std::net::TcpListener;

use ed25519_dalek::SigningKey;
use tls_from_scratch::common;
use x25519_dalek::{EphemeralSecret, PublicKey};

fn main() {
    // TODO:
    // 1. Load server identity key from "server_identity.key"
    // 2. Accept TCP connection
    // 3. Read client DH public key (32 bytes)
    // 4. Generate ephemeral DH key, send server DH public key (32 bytes)
    // 5. Sign server DH public key with identity key, send signature (64 bytes)
    // 6. Read client signature (64 bytes)
    // 7. Read client identity public key (32 bytes) — or load from a trusted keys file
    // 8. Verify client signature over client DH public key
    // 9. Compute shared secret, derive keys
    // 10. Encrypted echo loop
    todo!()
}

// Lesson 9: Mutual TLS (mTLS)
// Both client and server authenticate each other using Ed25519 signatures.

use std::io::{self, Read, Write};
use std::net::TcpStream;

use ed25519_dalek::SigningKey;
use tls_from_scratch::common;
use x25519_dalek::{EphemeralSecret, PublicKey};

fn main() {
    // TODO:
    // 1. Load client identity key from "client_identity.key"
    // 2. Connect to server
    // 3. Generate ephemeral DH key, send client DH public key (32 bytes)
    // 4. Read server DH public key (32 bytes)
    // 5. Read server signature (64 bytes), verify against known server public key
    // 6. Sign client DH public key with client identity key, send signature (64 bytes)
    // 7. Send client identity public key (32 bytes) — so server knows who we are
    // 8. Compute shared secret, derive keys
    // 9. Interactive encrypted chat loop
    todo!()
}

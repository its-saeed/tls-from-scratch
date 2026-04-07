// Lesson 10: Replay Attack Defense
// Uses sequence numbers in nonces to prevent message replay and reordering.

use std::io::{self, Read, Write};
use std::net::TcpStream;

use tls::common;
use x25519_dalek::{EphemeralSecret, PublicKey};

fn main() {
    // TODO:
    // 1. Do the authenticated handshake (same as Lesson 8)
    // 2. Derive keys
    // 3. Maintain send counter and receive counter (both start at 0)
    // 4. Encrypt each message with send counter as nonce, increment
    // 5. Decrypt each response, verify receive counter matches, increment
    // 6. No more random nonces — deterministic and verifiable
    todo!()
}

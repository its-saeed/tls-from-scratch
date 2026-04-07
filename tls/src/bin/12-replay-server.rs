// Lesson 10: Replay Attack Defense
// Uses sequence numbers in nonces to prevent message replay and reordering.

use std::io::{Read, Write};
use std::net::TcpListener;

use tls::common;
use x25519_dalek::{EphemeralSecret, PublicKey};

fn main() {
    // TODO:
    // 1. Do the authenticated handshake (same as Lesson 8)
    // 2. Derive keys
    // 3. Maintain a receive counter (starts at 0)
    // 4. For each received message:
    //    a. Decrypt using counter-based nonce (not random)
    //    b. Verify the nonce matches expected sequence number
    //    c. Reject if sequence number is <= last seen (replay/reorder)
    //    d. Increment counter
    // 5. For each sent message:
    //    a. Encrypt using send counter as nonce
    //    b. Increment send counter
    todo!()
}

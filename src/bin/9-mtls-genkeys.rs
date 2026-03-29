// Lesson 9: Generate identity keys for both client and server.

use ed25519_dalek::SigningKey;
use std::fs;

fn main() {
    // Server key
    let server_key = SigningKey::generate(&mut rand_core::OsRng);
    fs::write("server_identity.key", server_key.to_bytes()).unwrap();
    println!("Server private key saved to server_identity.key");
    println!("Server public key: {}", hex::encode(server_key.verifying_key().to_bytes()));

    println!();

    // Client key
    let client_key = SigningKey::generate(&mut rand_core::OsRng);
    fs::write("client_identity.key", client_key.to_bytes()).unwrap();
    println!("Client private key saved to client_identity.key");
    println!("Client public key: {}", hex::encode(client_key.verifying_key().to_bytes()));
}

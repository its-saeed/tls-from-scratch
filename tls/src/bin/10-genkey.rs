// Lesson 8: Generate a long-term Ed25519 identity key pair for the server.
// The private key is saved to a file. The public key is printed as hex
// so the client can hardcode or configure it.

use ed25519_dalek::SigningKey;
use std::fs;

fn main() {
    let signing_key = SigningKey::generate(&mut rand_core::OsRng);
    let verifying_key = signing_key.verifying_key();

    fs::write("server_identity.key", signing_key.to_bytes()).unwrap();
    println!("Private key saved to server_identity.key");
    println!("Public key: {}", hex::encode(verifying_key.to_bytes()));
}

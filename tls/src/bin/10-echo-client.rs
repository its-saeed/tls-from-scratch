use std::{io::{self, Read, Write}, net::TcpStream};

use ed25519_dalek::VerifyingKey;
use tls::common;
use x25519_dalek::{EphemeralSecret, PublicKey};

fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:7878").unwrap();
    let client_secret = EphemeralSecret::random_from_rng(rand_core::OsRng);
    let client_pub = PublicKey::from(&client_secret);
    stream.write_all(client_pub.as_bytes()).unwrap();
    let mut server_pub = [0u8; 32];
    stream.read_exact(&mut server_pub).unwrap();
    let server_pub = PublicKey::from(server_pub);
    let mut sig_bytes = [0u8; 64];
    stream.read_exact(&mut sig_bytes).unwrap();
    let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
    let known_pub = hex::decode("dd8c3c76bf81163f497ea58187eeed89ffa80a1b8dbfe16763612131db4e2c07").unwrap();
    let verifier = VerifyingKey::from_bytes(&known_pub.try_into().unwrap()).unwrap();
    verifier.verify_strict(server_pub.as_bytes(), &signature).expect("server authentication failed");
    println!("server authenticated");

    let secret = client_secret.diffie_hellman(&server_pub);
    let (client_cipher, server_cipher) = common::derive_keys(secret.as_bytes());
    loop {
        let mut msg = String::new();
        io::stdin().read_line(&mut msg).unwrap();
        common::send_encrypted(&mut stream, &client_cipher, msg.as_bytes());
        match common::recv_encrypted(&mut stream, &server_cipher) {
            Ok(msg) =>  println!("{}", String::from_utf8(msg).unwrap()),
            Err(_) => {
                println!("disconnected");
                break;
            }
        }
    }
}

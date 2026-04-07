use std::{io::{Read, Write}, net::TcpListener};

use tls::common;
use x25519_dalek::{EphemeralSecret, PublicKey};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let (mut stream, _) = listener.accept().unwrap();
    let mut client_pub = [0u8; 32];
    stream.read_exact(&mut client_pub).unwrap();
    let client_pub = PublicKey::from(client_pub);
    let server_secret = EphemeralSecret::random_from_rng(rand_core::OsRng);
    let server_pub = PublicKey::from(&server_secret);
    stream.write_all(server_pub.as_bytes()).unwrap();
    let secret = server_secret.diffie_hellman(&client_pub);
    let (client_cipher, server_cipher) = common::derive_keys(secret.as_bytes());
    loop {
        match common::recv_encrypted(&mut stream, &client_cipher) {
            Ok(msg) => {
                println!("{}", String::from_utf8(msg.clone()).unwrap());
                common::send_encrypted(&mut stream, &server_cipher, &msg);
            }
            Err(_) => {
                println!("client disconnected");
                break;
            }
        }
    }
}

use std::{io::{self, Read, Write}, net::TcpStream};

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

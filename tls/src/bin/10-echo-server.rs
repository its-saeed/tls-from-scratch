use std::{io::{self, Read, Write}, net::{TcpListener, TcpStream}};

use chacha20poly1305::ChaCha20Poly1305;
use ed25519_dalek::Signer;
use tls::common;
use x25519_dalek::{EphemeralSecret, PublicKey};

fn load_identity_key() -> ed25519_dalek::SigningKey {
    let key_bytes = std::fs::read("server_identity.key").unwrap();
    ed25519_dalek::SigningKey::from_bytes(&key_bytes.try_into().unwrap())
}

struct Server {
    listener: TcpListener,
    client: Option<TcpStream>,
    client_cipher: Option<ChaCha20Poly1305>,
    server_cipher: Option<ChaCha20Poly1305>,
}

impl Server {
    fn bind(addr: &str) -> Self {
        Self {
            listener: TcpListener::bind(addr).unwrap(),
            client: None,
            client_cipher: None,
            server_cipher: None,
        }
    }

    fn accept(&mut self) {
        self.client = Some(self.listener.accept().unwrap().0);
    }

    fn negotiate(&mut self) {
        if let Some(ref mut stream) = self.client {
            let mut client_pub = [0u8; 32];
            stream.read_exact(&mut client_pub).unwrap();
            let client_pub = PublicKey::from(client_pub);
            let server_secret = EphemeralSecret::random_from_rng(rand_core::OsRng);
            let server_pub = PublicKey::from(&server_secret);
            let identity_key = load_identity_key();
            let server_identity = identity_key.sign(server_pub.as_bytes());
            stream.write_all(server_pub.as_bytes()).unwrap();
            stream.write_all(&server_identity.to_bytes()).unwrap();

            let secret = server_secret.diffie_hellman(&client_pub);
            let (client_cipher, server_cipher) = common::derive_keys(secret.as_bytes());
            self.client_cipher = Some(client_cipher);
            self.server_cipher = Some(server_cipher);
        } else {
            panic!("no client connected");
        }
    }
    fn recv(&mut self) -> Result<Vec<u8>, io::Error> {
        if let Some(ref mut stream) = self.client {
            if let Some(ref client_cipher) = self.client_cipher {
                common::recv_encrypted(stream, client_cipher)
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "no client cipher"))
            }
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "no client"))
        }
    }

    fn send(&mut self, msg: &[u8]) -> Result<(), io::Error> {
        if let Some(ref mut stream) = self.client {
            if let Some(ref server_cipher) = self.server_cipher {
                Ok(common::send_encrypted(stream, server_cipher, msg))
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "no server cipher"))
            }
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "no client"))
        }
    }
}

fn main() {
    let mut server = Server::bind("127.0.0.1:7878");
    server.accept();
    server.negotiate();
    loop {
        match server.recv() {
            Ok(msg) => {
                println!("{}", String::from_utf8(msg.clone()).unwrap());
                server.send(&msg).unwrap();
            }
            Err(_) => {
                println!("client disconnected");
                break;
            }
        }
    }
}

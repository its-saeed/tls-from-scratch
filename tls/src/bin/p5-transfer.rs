use std::{io::Write, net::TcpStream};

use chacha20poly1305::ChaCha20Poly1305;
use clap::{Parser, Subcommand};
use x25519_dalek::{EphemeralSecret, PublicKey};

#[derive(Parser)]
#[command(name = "transfer", about = "Encrypted file transfer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Send a file
    Send {
        /// Host:port to connect to
        #[arg(long)]
        host: String,
        /// Server's public key (hex)
        #[arg(long)]
        server_pubkey: String,
        /// File to send
        file: String,
    },
    /// Receive a file
    Receive {
        /// Port to listen on
        #[arg(long, default_value = "9000")]
        port: u16,
        /// Path to server identity key
        #[arg(long)]
        key: String,
    },
}

fn sender_handshake(stream: &mut TcpStream, server_pubkey: &[u8; 32])
    -> anyhow::Result<(ChaCha20Poly1305, ChaCha20Poly1305)>
{
    // 1. Generate ephemeral DH key
    let key = EphemeralSecret::random_from_rng(rand_core::OsRng);
    let pubkey = PublicKey::from(&key);
    stream.write_all(pubkey.as_bytes())?;
    todo!()
}
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Send { host, server_pubkey, file } => todo!(),
        Command::Receive { port, key } => todo!(),
    }
}

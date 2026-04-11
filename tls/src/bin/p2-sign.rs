use clap::{Parser, Subcommand};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey, ed25519::signature::SignerMut};

#[derive(Parser)]
#[command(name = "sign-tool", about = "Sign and verify files with Ed25519")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a new Ed25519 key pair
    Keygen { key_path: String },
    /// Sign a file
    Sign {
        #[arg(long)]
        key_path: String,
        #[arg(long)]
        file_path: String,
    },
    /// Verify a file's signature
    Verify {
        #[arg(long)]
        pubkey: String,
        #[arg(long)]
        file: String,
        #[arg(long)]
        signature: String,
    },
}

fn generate_keypair(key_path: &str) {
    let signing_key = SigningKey::generate(&mut rand_core::OsRng);
    let public_key = signing_key.verifying_key();
    let _ = std::fs::write(key_path, signing_key.to_bytes());
    let _ = std::fs::write(format!("{}.pub", key_path), public_key.to_bytes());

    println!("Private key saved to: {key_path}");
    println!("Public key saved to:  {key_path}.pub");
    println!("Public key (hex):     {}", hex::encode(public_key.to_bytes()));
}

fn sign_file(key_path: &str, file_path: &str) {
    let key_bytes: [u8; 32] = std::fs::read(key_path).expect("Can't read keyfile").try_into().unwrap();
    let mut signing_key = SigningKey::from_bytes(&key_bytes);
    let file_data = std::fs::read(file_path).expect("Can't read filep data");

    let signature = signing_key.sign(&file_data);
    let sig_path = format!("{file_path}.sig");
    std::fs::write(&sig_path, signature.to_bytes()).unwrap();

    println!("Signed: {file_path}");
    println!("Signature: {sig_path} ({} bytes)", signature.to_bytes().len());
    println!("Signature (hex): {}", hex::encode(signature.to_bytes()));
}

fn verify(pubkey: &str, file: &str, signature: &str) {
    let pub_bytes: [u8; 32] = std::fs::read(pubkey).expect("Can't read public key").try_into().unwrap();
    let verifying_key = VerifyingKey::from_bytes(&pub_bytes).expect("Failed to create the public key");

    let file_data = std::fs::read(file).unwrap();
    let sig_bytes: [u8; 64] = std::fs::read(signature).expect("Can't read signature").try_into().unwrap();
    let signature = Signature::from_bytes(&sig_bytes);

    match verifying_key.verify_strict(&file_data, &signature) {
        Ok(()) => {
            println!("✓ Signature valid");
            println!("  File: {file}");
            println!("  Signed by: {}", hex::encode(pub_bytes));
        }
        Err(e) => {
            println!("✗ Signature INVALID");
            println!("  File: {file}");
            println!("  Error: {e}");
            std::process::exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Command::Keygen { key_path } => {
            generate_keypair(key_path);
        }
        Command::Sign { key_path, file_path} => {
            sign_file(key_path, file_path);
        }
        Command::Verify { pubkey, file, signature } => {
            verify(pubkey, file, signature)
        }
    }
}

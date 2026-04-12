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
        #[arg(long = "timestamp")]
        include_timestamp: bool
    },
    /// Verify a file's signature
    Verify {
        #[arg(long)]
        pubkey: String,
        #[arg(long)]
        file: String,
        #[arg(long)]
        signature: String,
        #[arg(long)]
        max_age_secs: Option<u64>
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

fn sign_file(key_path: &str, file_path: &str, include_timestamp: bool) {
    let key_bytes: [u8; 32] = std::fs::read(key_path).expect("Can't read keyfile").try_into().unwrap();
    let mut signing_key = SigningKey::from_bytes(&key_bytes);
    let file_data = std::fs::read(file_path).expect("Can't read filep data");

    let timestamp = if include_timestamp {
        Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs())
    } else {
        None
    };

    let mut to_sign = Vec::new();
    if let Some(ts) = timestamp {
        println!("{}", hex::encode(ts.to_be_bytes()));
        to_sign.extend_from_slice(&ts.to_be_bytes());
    }

    to_sign.extend_from_slice(&file_data);

    let signature = signing_key.sign(&to_sign);
    let sig_path = format!("{file_path}.sig");

    let mut signature = signature.to_vec();
    signature.extend_from_slice(&timestamp.unwrap().to_be_bytes());
    std::fs::write(&sig_path, &signature).unwrap();

    println!("Signed: {file_path}");
    println!("Signature: {sig_path} ({} bytes)", signature.len());
    println!("Signature (hex): {}", hex::encode(signature));
}

fn verify(pubkey: &str, file: &str, signature: &str, max_age_secs: Option<u64>) -> Result<(), String> {
    let pub_bytes: [u8; 32] = std::fs::read(pubkey).expect("Can't read public key").try_into().unwrap();
    let verifying_key = VerifyingKey::from_bytes(&pub_bytes).expect("Failed to create the public key");

    let file_data = std::fs::read(file).unwrap();
    let sig_file_bytes = std::fs::read(signature).unwrap();
    let (signature_bytes, timestamp ) = sig_file_bytes.split_at(64);
    //let sig_bytes: [u8; 64] = std::fs::read(signature).expect("Can't read signature").try_into().unwrap();
    let signature = Signature::from_bytes(&signature_bytes.try_into().unwrap());

    let mut file_data_with_ts = timestamp.to_vec();
    file_data_with_ts.extend_from_slice(&file_data);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    println!("{}", hex::encode(timestamp));
    let timestamp = u64::from_be_bytes(timestamp.try_into().unwrap());
    if  timestamp > now { return Err("timestamp is in the future".into()); }
    if now - timestamp > max_age_secs.unwrap() {
        return Err(format!("signature expired ({} seconds old)", now - timestamp));
    }
    // Verify signature
    verifying_key.verify_strict(&file_data_with_ts, &signature)
        .map_err(|_| "signature invalid".to_string())?;

    // Check timestamp
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Command::Keygen { key_path } => {
            generate_keypair(key_path);
        }
        Command::Sign { key_path, file_path, include_timestamp} => {
            sign_file(key_path, file_path, *include_timestamp);
        }
        Command::Verify { pubkey, file, signature, max_age_secs } => {
            verify(pubkey, file, signature, max_age_secs.to_owned()).unwrap()
        }
    }
}

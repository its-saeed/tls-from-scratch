use clap::{Parser, Subcommand};
use hmac::Mac;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a TOTP code
    Generate { secret: String },
}

fn decode_secret(secret: &str) -> Vec<u8> {
    data_encoding::BASE32.decode(secret.as_bytes()).expect("Invalid base32 secret!")
}

fn current_time_step() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() / 30
}

fn hmac_sha1(secret: &[u8], time_step: u64) -> [u8; 20] {
    let mut mac = hmac::Hmac::<sha1::Sha1>::new_from_slice(secret).unwrap();
    mac.update(&time_step.to_be_bytes());
    let result = mac.finalize().into_bytes();
    let mut output = [0u8; 20];
    output.copy_from_slice(&result);
    output
}

fn truncate(hmac_result: &[u8; 20]) -> u32 {
    // The last nibble (4 bits) determines the offset
    let offset = (hmac_result[19] & 0x0F) as usize;
    // offset is 0-15, and we read 4 bytes, so max index is 15+3=18 (within 20)

    // Extract 4 bytes at that offset, mask the high bit
    u32::from_be_bytes([
        hmac_result[offset] & 0x7F,  // & 0x7F clears the sign bit
        hmac_result[offset + 1],
        hmac_result[offset + 2],
        hmac_result[offset + 3],
    ])
}

fn generate_totp(secret_base32: &str, time_step: u64) -> u32 {
    let secret = decode_secret(secret_base32);
    let hmac = hmac_sha1(&secret, time_step);
    let truncated = truncate(&hmac);
    truncated % 1_000_000  // 6 digits
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate { secret } => {
            let step = current_time_step();
            let totp = generate_totp(&secret, step);
            println!("TOTP: {}", totp);
        }
    }
}

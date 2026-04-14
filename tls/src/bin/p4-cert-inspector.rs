use std::sync::Arc;

use clap::{Parser, Subcommand};
use rustls::{ClientConfig, RootCertStore};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use x509_parser::prelude::{FromDer, X509Certificate};

#[derive(Parser)]
#[command(name = "cert-inspector", about = "Inspect TLS certificates of any website")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Inspect a site's certificate chain
    Inspect {
        /// Domain name (e.g., google.com)
        host: String,
        /// Port (default: 443)
        #[arg(long, default_value = "443")]
        port: u16,
    },
    /// Check certificate expiry for multiple domains
    CheckExpiry {
        /// Domain names
        hosts: Vec<String>,
    },
}

async fn tls_connect(host: String, port: u16) -> Result<tokio_rustls::client::TlsStream<TcpStream>, Box<dyn std::error::Error>> {
    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let tcp = TcpStream::connect(format!("{host}:{port}")).await?;
    let server_name = host.try_into()?;
    let tls = connector.connect(server_name, tcp).await?;
    Ok(tls)
}

async fn inspect_cert(domain: String, port: u16) {
    println!("==============={domain}===============");
    let tls = tls_connect(domain.clone(), port).await.unwrap();
    println!("connected to {domain}");
    let (_, conn) = tls.get_ref();
    println!("Protocol: {:?}", conn.protocol_version().unwrap());
    println!("Cipher:   {:?}", conn.negotiated_cipher_suite().unwrap());

    let cert = conn.peer_certificates().expect("server didn't send certificates.");
    for (i, c) in cert.iter().enumerate() {
        print_cert(i, c);
    }

}

fn print_cert(index: usize, der: &[u8]) {
    let (_, cert) = X509Certificate::from_der(der).expect("failed to parse cert");
    print_sans(&cert);
    println!("  [{}] {}", index, cert.subject());
    println!("      Issuer:  {}", cert.issuer());
    println!("      Valid:   {} to {}",
        cert.validity().not_before,
        cert.validity().not_after);

    // Check if it's a CA certificate
    if let Some(bc) = cert.basic_constraints().ok().flatten() {
        if bc.value.ca {
            println!("      Type:    CA certificate");
        }
    }

    // Self-signed?
    if cert.subject() == cert.issuer() {
        println!("      Note:    Self-signed (root CA or self-signed cert)");
    }

    let days = days_until_expiry(&cert);
    if days < 0 {
        println!("      ⚠ EXPIRED {} days ago!", -days);
    } else if days < 30 {
        println!("      ⚠ Expires in {} days (renew soon!)", days);
    } else {
        println!("      Expires in {} days", days);
    }
}

fn days_until_expiry(cert: &X509Certificate) -> i64 {
    let not_after = cert.validity().not_after.timestamp();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    (not_after - now) / 86400
}

fn print_sans(cert: &X509Certificate) {
    if let Ok(Some(san_ext)) = cert.subject_alternative_name() {
        let names: Vec<String> = san_ext.value.general_names.iter()
            .filter_map(|name| match name {
                x509_parser::extensions::GeneralName::DNSName(dns) => {
                    Some(dns.to_string())
                }
                x509_parser::extensions::GeneralName::IPAddress(ip) => {
                    Some(format!("IP:{:?}", ip))
                }
                _ => None,
            })
            .collect();

        if !names.is_empty() {
            println!("      SANs:    {}", names.join(", "));
        }
    }
}

#[tokio::main]
async fn main() {

    let cli = Cli::parse();
    match cli.command {
        Command::Inspect { host, port } => inspect_cert(host, port).await,
        Command::CheckExpiry { hosts } => {
            let handles = hosts.iter().map(|host| {
                async {
                    inspect_cert(host.clone(), 443).await
                }
            }).collect::<Vec<_>>();
            for handle in handles {
                handle.await;
            }
        },
    }
}

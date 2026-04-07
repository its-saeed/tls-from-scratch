// Lesson 6: Certificates and Trust (X.509)
//
// Problem: DH key exchange is vulnerable to man-in-the-middle (MITM).
// Without authentication, an attacker (Mallory) can intercept:
//   Alice ←DH→ Mallory ←DH→ Bob
// Mallory does separate key exchanges with each side, decrypts and re-encrypts.
//
// Solution: certificates. A certificate binds a public key to an identity:
//   "Public key 0x3a8f... belongs to server.example.com"
//   Signed by a Certificate Authority (CA) that the client trusts.
//
// Chain of trust:
//   Root CA (pre-trusted, built into OS/browser)
//     └─ signs → Intermediate CA certificate
//                  └─ signs → Server certificate
//
// X.509: the standard certificate format. Contains:
//   - Subject name (e.g. CN=localhost)
//   - Public key
//   - Issuer name
//   - Validity period
//   - Issuer's signature
//
// Self-signed certificates: the certificate signs itself (acts as its own CA).
// Fine for private infrastructure — you manually trust the cert on the client.

use std::io::BufReader;

fn main() {
    let f = std::fs::File::open("server.crt").unwrap();
    let certs = rustls_pemfile::certs(&mut BufReader::new(f)).collect::<Result<Vec<_>, _>>().unwrap();
    let der_bytes = &certs[0];
    let (_, cert) = x509_parser::parse_x509_certificate(der_bytes).unwrap();
    println!("Subject: {}", cert.subject());
    println!("Public key algorithm: {}", cert.public_key().algorithm.algorithm);
}

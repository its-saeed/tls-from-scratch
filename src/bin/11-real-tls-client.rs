// Lesson 11: Real TLS with tokio-rustls
// Replace our hand-built protocol with production TLS.

fn main() {
    // TODO:
    // 1. Load the server's CA certificate (server.crt) as a trusted root
    // 2. Create a rustls ClientConfig with the trusted root
    // 3. Connect via TCP, wrap with TlsConnector
    // 4. The TlsStream implements AsyncRead + AsyncWrite
    // 5. Send messages, read echoed responses
    //
    // The key insight: all the crypto we built by hand in Lessons 1-8
    // (DH key exchange, HKDF, ChaCha20, signatures, certificates)
    // happens automatically inside rustls. We just read/write plaintext.
    todo!()
}

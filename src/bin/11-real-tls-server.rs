// Lesson 11: Real TLS with tokio-rustls
// Replace our hand-built protocol with production TLS.

fn main() {
    // TODO:
    // 1. Load server certificate (server.crt) and private key (server.key)
    // 2. Create a rustls ServerConfig with the cert/key
    // 3. Wrap TcpListener::accept() with TlsAcceptor
    // 4. The TlsStream implements AsyncRead + AsyncWrite — use it like a normal stream
    // 5. Echo loop: read from TLS stream, write back
    //
    // Dependencies needed:
    //   tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "io-util"] }
    //   tokio-rustls = "0.26"
    //   rustls = "0.23"
    //   rustls-pemfile = "2"  (already have this)
    todo!()
}

// Lesson 12: Build a simple HTTPS client
// Connect to a real website using TLS, send an HTTP GET, print the response.

fn main() {
    // TODO:
    // 1. Create a rustls ClientConfig with the system's trusted root certificates
    //    Use webpki-roots or rustls-native-certs to load system CA store
    // 2. Connect TCP to a real server (e.g., "example.com:443")
    // 3. Wrap with TlsConnector, using SNI hostname "example.com"
    // 4. Send a raw HTTP/1.1 GET request:
    //    "GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n"
    // 5. Read the response until EOF, print it
    // 6. Print the negotiated cipher suite and TLS version
    //
    // Dependencies needed:
    //   webpki-roots = "0.26"  (or rustls-native-certs for system CA store)
    //
    // This demonstrates: your knowledge of TLS applied to real internet traffic.
    // The same handshake you built by hand is happening inside rustls,
    // verified against the same CA hierarchy you learned about in Lesson 6.
    todo!()
}

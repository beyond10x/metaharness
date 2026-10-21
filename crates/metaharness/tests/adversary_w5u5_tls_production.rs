//! Adversary cases, wave 5 unit 5 pass 1, against `story:loopback-tls-leg-tested`.
//!
//! An integration test links `metaharness` **without** `cfg(test)`, so the upstream TLS leg these
//! cases drive is the one a real run executes: the `#[cfg(not(test))]` branch of `connect` and the
//! compiled-in roots. The unit's own Vectors 7 and 8 live inside the crate, so they compile the
//! `cfg(test)` branch instead, and a change confined to the production branch is invisible to
//! them. Every upstream here is on 127.0.0.1, every certificate is generated in memory, and the
//! custody token is a fake.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use metaharness::protocol::Kind;
use metaharness::{CredentialCustody, LoopbackProxy};

/// The fake custody token. Its bytes must never be seen by an upstream the proxy cannot trust.
const TOKEN: &str = "adversary-w5u5-custody-token";

/// A custody over a credential file with the vendor's shape and [`TOKEN`] in it.
fn custody(dir: &Path) -> Arc<CredentialCustody> {
    let path = dir.join(".credentials.json");
    let body = serde_json::json!({
        "claudeAiOauth": {
            "accessToken": TOKEN,
            "refreshToken": "fake-refresh-token",
            "expiresAt": 4_102_444_800_000_i64,
            "refreshTokenExpiresAt": 4_102_444_800_000_i64,
            "scopes": ["user:inference"],
            "subscriptionType": "fake",
            "rateLimitTier": "fake",
        }
    });
    std::fs::write(&path, serde_json::to_vec(&body).unwrap()).unwrap();
    Arc::new(CredentialCustody::open(Kind::Claude, &path).unwrap())
}

/// What one upstream connection saw.
#[derive(Debug, Default)]
struct Observed {
    /// Every byte the proxy put on the wire, before any decryption.
    raw: Vec<u8>,
    /// The upstream's own handshake outcome, when it spoke TLS.
    handshake: Option<Result<(), String>>,
    /// Application bytes the upstream decrypted after a completed handshake.
    plaintext: Vec<u8>,
}

/// A socket that keeps a copy of every byte read from it.
struct Recording {
    socket: TcpStream,
    raw: Arc<Mutex<Vec<u8>>>,
}

impl Read for Recording {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.socket.read(buf)?;
        self.raw.lock().unwrap().extend_from_slice(&buf[..read]);
        Ok(read)
    }
}

impl Write for Recording {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.socket.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.socket.flush()
    }
}

/// Listen on 127.0.0.1, take at most one connection within 10 s, and hand it to `behaviour`.
///
/// A proxy that never connects yields an empty [`Observed`], which every case below refuses.
fn one_connection<F>(behaviour: F) -> (u16, JoinHandle<Observed>)
where
    F: FnOnce(TcpStream) -> Observed + Send + 'static,
{
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let handle = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match listener.accept() {
                Ok((socket, _)) => {
                    socket.set_nonblocking(false).unwrap();
                    socket
                        .set_read_timeout(Some(Duration::from_secs(10)))
                        .unwrap();
                    return behaviour(socket);
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Observed::default();
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return Observed::default(),
            }
        }
    });
    (port, handle)
}

/// A TLS upstream presenting `config`'s certificate. On a completed handshake it reads the
/// request head the proxy sends and answers 200; on a failed one it records the failure and stops.
fn tls_upstream(config: Arc<rustls::ServerConfig>) -> impl FnOnce(TcpStream) -> Observed {
    move |socket| {
        let raw = Arc::new(Mutex::new(Vec::new()));
        let session = rustls::ServerConnection::new(config).unwrap();
        let mut tls = rustls::StreamOwned::new(
            session,
            Recording {
                socket,
                raw: Arc::clone(&raw),
            },
        );
        let mut handshake = Ok(());
        while tls.conn.is_handshaking() {
            if let Err(error) = tls.conn.complete_io(&mut tls.sock) {
                handshake = Err(error.to_string());
                break;
            }
        }
        let mut plaintext = Vec::new();
        if handshake.is_ok() {
            let mut buffer = [0u8; 4096];
            loop {
                match tls.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => {
                        plaintext.extend_from_slice(&buffer[..read]);
                        if plaintext.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }
                }
            }
            let _ = tls
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
            tls.conn.send_close_notify();
            let _ = tls.flush();
        }
        let _ = tls.sock.socket.shutdown(Shutdown::Both);
        let raw = raw.lock().unwrap().clone();
        Observed {
            raw,
            handshake: Some(handshake),
            plaintext,
        }
    }
}

/// An "https" upstream that answers in cleartext HTTP before the proxy has said anything, then
/// records every byte the proxy sends until it closes.
fn cleartext_upstream(mut socket: TcpStream) -> Observed {
    let _ =
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
    let _ = socket.flush();
    let mut raw = Vec::new();
    let _ = socket.read_to_end(&mut raw);
    Observed {
        raw,
        ..Observed::default()
    }
}

/// An "https" upstream that reads the `ClientHello`, closes its sending side mid-handshake, and
/// records every byte the proxy sends until it closes too.
fn drops_mid_handshake(mut socket: TcpStream) -> Observed {
    let mut raw = vec![0u8; 16 * 1024];
    let first = socket.read(&mut raw).unwrap_or(0);
    raw.truncate(first);
    let _ = socket.shutdown(Shutdown::Write);
    let _ = socket.read_to_end(&mut raw);
    Observed {
        raw,
        ..Observed::default()
    }
}

/// A certificate authority generated for one test, held in memory only.
fn test_ca(name: &str) -> rcgen::CertifiedIssuer<'static, rcgen::KeyPair> {
    let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, name);
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![
        rcgen::KeyUsagePurpose::KeyCertSign,
        rcgen::KeyUsagePurpose::CrlSign,
    ];
    let key = rcgen::KeyPair::generate().unwrap();
    rcgen::CertifiedIssuer::self_signed(params, key).unwrap()
}

/// A TLS server configuration for 127.0.0.1, signed by `ca`, speaking only `versions`.
fn server_for_loopback(
    ca: &rcgen::CertifiedIssuer<'static, rcgen::KeyPair>,
    versions: &[&'static rustls::SupportedProtocolVersion],
) -> Arc<rustls::ServerConfig> {
    let key = rcgen::KeyPair::generate().unwrap();
    let mut params = rcgen::CertificateParams::new(vec!["127.0.0.1".to_string()]).unwrap();
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
    let leaf = params.signed_by(&key, ca).unwrap();
    let private = rustls::pki_types::PrivateKeyDer::Pkcs8(key.serialize_der().into());
    Arc::new(
        rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(versions)
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![leaf.der().clone()], private)
        .unwrap(),
    )
}

/// A `POST /v1/messages` carrying `auth`.
fn messages_request(auth: &str) -> String {
    let body = r#"{"model":"claude-opus-5","messages":[]}"#;
    format!(
        "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1\r\n{auth}\r\n\
         anthropic-version: 2023-06-01\r\ncontent-type: application/json\r\n\
         Content-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// Speak one request to the proxy and read the whole answer.
fn ask(port: u16, request: &str) -> (String, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    stream.flush().unwrap();
    let mut all = Vec::new();
    stream.read_to_end(&mut all).unwrap();
    let text = String::from_utf8_lossy(&all).to_string();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    (head.to_string(), body.to_string())
}

/// Run one request through a proxy built by the **public** constructor at an `https` upstream on
/// `port`, and return the child's view, what the upstream observed, and the forwarded count.
fn through_production(
    port: u16,
    upstream: JoinHandle<Observed>,
) -> (String, String, Observed, u64) {
    let dir = tempfile::tempdir().unwrap();
    let proxy = LoopbackProxy::start(
        &format!("https://127.0.0.1:{port}"),
        custody(dir.path()),
        "adversary",
    )
    .unwrap();
    let (head, body) = ask(
        proxy.port(),
        &messages_request(&format!("Authorization: Bearer {}", proxy.placeholder())),
    );
    let forwarded = proxy.report().forwarded;
    proxy.shutdown();
    let observed = upstream.join().unwrap();
    (head, body, observed, forwarded)
}

/// What every refusal shares: the child gets the proxy's own 502 without the token, the proxy
/// spoke TLS (so the case is not passing on a proxy that never connected), no cleartext
/// credential crossed the wire, and nothing counted as forwarded.
fn assert_refused_without_the_credential(
    case: &str,
    head: &str,
    body: &str,
    observed: &Observed,
    forwarded: u64,
) {
    assert!(
        head.starts_with("HTTP/1.1 502"),
        "{case}: the proxy must answer the child itself, got: {head}\n{body}"
    );
    assert!(
        !body.contains(TOKEN),
        "{case}: the refusal must not carry the credential: {body}"
    );
    assert!(
        observed.raw.starts_with(&[0x16, 0x03]),
        "{case}: the proxy must open the https leg with a TLS handshake record, saw {} bytes: {:02x?}",
        observed.raw.len(),
        &observed.raw[..observed.raw.len().min(8)]
    );
    let raw = String::from_utf8_lossy(&observed.raw);
    assert!(
        !raw.contains(TOKEN) && !raw.to_ascii_lowercase().contains("authorization"),
        "{case}: no credential may cross the wire in cleartext"
    );
    assert!(
        observed.plaintext.is_empty(),
        "{case}: the upstream must decrypt nothing, decrypted: {}",
        String::from_utf8_lossy(&observed.plaintext)
    );
    assert_eq!(forwarded, 0, "{case}: nothing was forwarded");
}

/// The production build, TLS 1.3: an issuer outside the compiled-in roots is refused in the
/// handshake and the upstream never completes its side, so no application data — and no token —
/// was ever encrypted to it.
#[test]
fn the_production_build_refuses_an_issuer_outside_the_compiled_in_roots_over_tls13() {
    let ca = test_ca("adversary w5u5 CA, TLS 1.3");
    let (port, upstream) = one_connection(tls_upstream(server_for_loopback(
        &ca,
        &[&rustls::version::TLS13],
    )));
    let (head, body, observed, forwarded) = through_production(port, upstream);
    assert_refused_without_the_credential("TLS 1.3", &head, &body, &observed, forwarded);
    assert!(
        matches!(observed.handshake, Some(Err(_))),
        "TLS 1.3: the upstream's handshake must not complete: {:?}",
        observed.handshake
    );
    assert!(
        body.contains("certificate"),
        "TLS 1.3: the refusal names the certificate: {body}"
    );
}

/// The same refusal when the upstream offers only TLS 1.2, where the certificate arrives in the
/// server's first flight and the client's key exchange follows it.
#[test]
fn the_production_build_refuses_an_issuer_outside_the_compiled_in_roots_over_tls12() {
    let ca = test_ca("adversary w5u5 CA, TLS 1.2");
    let (port, upstream) = one_connection(tls_upstream(server_for_loopback(
        &ca,
        &[&rustls::version::TLS12],
    )));
    let (head, body, observed, forwarded) = through_production(port, upstream);
    assert_refused_without_the_credential("TLS 1.2", &head, &body, &observed, forwarded);
    assert!(
        matches!(observed.handshake, Some(Err(_))),
        "TLS 1.2: the upstream's handshake must not complete: {:?}",
        observed.handshake
    );
    assert!(
        body.contains("certificate"),
        "TLS 1.2: the refusal names the certificate: {body}"
    );
}

/// An `https` upstream that answers in cleartext HTTP is not a downgrade: the proxy fails the
/// handshake and the token never goes out in the clear.
#[test]
fn a_cleartext_answer_on_the_https_leg_is_refused_and_no_credential_crosses() {
    let (port, upstream) = one_connection(cleartext_upstream);
    let (head, body, observed, forwarded) = through_production(port, upstream);
    assert_refused_without_the_credential("cleartext answer", &head, &body, &observed, forwarded);
}

/// An `https` upstream that closes mid-handshake gets a 502 and no credential, before or after.
#[test]
fn an_upstream_that_closes_mid_handshake_gets_no_credential() {
    let (port, upstream) = one_connection(drops_mid_handshake);
    let (head, body, observed, forwarded) = through_production(port, upstream);
    assert_refused_without_the_credential(
        "mid-handshake close",
        &head,
        &body,
        &observed,
        forwarded,
    );
}

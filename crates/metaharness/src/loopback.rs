//! The loopback provider: metaharness is the API endpoint the inner harness talks to.
//!
//! LP-1 and LP-2 of `docs/design/loopback-provider-v0.1.md`. The child is launched with
//! `ANTHROPIC_BASE_URL` pointed at [`LoopbackHandle::base_url`] and a **placeholder** credential
//! ([`LoopbackHandle::placeholder`]); its scratch home contains no credential file at all. The
//! real token lives in exactly one place — [`crate::CredentialCustody`], on the metaharness side
//! of the socket — and is attached to each request on its way out.
//!
//! # What this changes about the hermetic claim
//!
//! H6 today reads "credentials are one file, copied". Under this proxy it becomes **"no
//! credential in the child at all"**, and that is attestable from the launch values rather than
//! asserted: the plan carries no credential copy, and the env carries a string that is worthless
//! anywhere but this port. The design's correction to the original proposal is exactly this — the
//! child never holds the real token, because a token injected into the child's environment would
//! still age inside the isolation directory.
//!
//! # The three behaviours, and why each is shaped the way it is
//!
//! 1. **The placeholder is checked, then destroyed.** A request is accepted in either spelling —
//!    `Authorization: Bearer <placeholder>` or `x-api-key: <placeholder>`, because Claude Code was
//!    observed sending `x-api-key` against a custom base (model-adapter design, verified) and the
//!    bearer spelling is the one `ANTHROPIC_AUTH_TOKEN` documents. **Both** headers are stripped
//!    before forwarding, so no spelling of the placeholder survives the hop. A request that does
//!    not carry it is answered 401 here and **never forwarded**: the port is on 127.0.0.1, where
//!    every other process on the machine can reach it, and forwarding an unauthenticated request
//!    would spend the operator's subscription for whoever asked.
//! 2. **Everything else is relayed verbatim.** No path allowlist — a vendor endpoint nobody
//!    catalogued must not break the run — and no header curation: `anthropic-version` and
//!    `anthropic-beta` are the upstream's own contract and a proxy that "tidied" them would
//!    silently change the API the child is calling. Only the hop-by-hop headers, `Host` and
//!    `Content-Length` are rewritten, because those describe *this* hop and not the child's
//!    request. Response bytes are piped as they arrive, flushed per chunk: an SSE body that were
//!    buffered to completion would turn a streaming harness into a batch one, and nothing would
//!    report it as a fault.
//! 3. **A 401 is answered by refreshing custody and retrying once.** The child sees the 401 only
//!    if the retry fails too, so the child never attempts its own OAuth refresh — which it could
//!    not do anyway, holding only a placeholder. See [`crate::custody`] for the v1 narrowing:
//!    "refresh" is a **re-read of the operator's live file under an exclusive lock**, never an
//!    OAuth dance metaharness performs. That is what removes V-LP5's rotation race — a party that
//!    never writes the credential cannot invalidate anybody's session — and it is why
//!    [`crate::CustodyError::StillStale`] (the re-read came back byte-identical) is the
//!    "refresh failed" signal rather than a transport error.
//!
//! # Deliberate limits of this build, so nobody reads them as properties
//!
//! * **One upstream connection per request**, `Connection: close` both ways. It costs a TLS
//!   handshake per request against a real vendor host. It is taken because connection reuse plus
//!   `Content-Length`/chunked framing plus a mid-stream retry is three interacting state machines,
//!   and this build would rather be slow than subtly wrong about where a response ends.
//! * **The TLS leg is unexercised by the tests here.** Every vector in this module runs against a
//!   plain-HTTP fake upstream, on purpose: they must cost nothing and need no network. The rustls
//!   path is therefore *built, not verified* — it flips to verified when V-LP2 runs one live
//!   session through it, and until then this comment is the label.
//! * **A request body arriving `Transfer-Encoding: chunked` from the child is refused by name**
//!   (400) rather than relayed. The body must be buffered — otherwise the 401 retry has nothing
//!   left to resend — and a chunked *request* decoder is machinery no observed Claude Code request
//!   needs. It is refused loudly so the gap cannot be mistaken for support.
//! * **Nothing here logs a request or response body.** The design makes body logging opt-in and
//!   stored with the run's transcript; this build implements neither, and counts instead
//!   ([`ProxyReport`]).

use std::fmt::Write as _;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::custody::{CredentialCustody, CustodyError};

/// How long the child gets to finish sending a request before its connection is abandoned.
const CHILD_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// How long the upstream TCP connect may take.
const UPSTREAM_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
/// How long an upstream stream may be silent before the relay gives up.
///
/// Generous, because a long tool-using turn can be quiet for minutes and a proxy that cut it
/// would look exactly like a model that stopped answering.
const UPSTREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(300);
/// The relay's copy buffer. Small on purpose: it is also the streaming granularity.
const RELAY_CHUNK: usize = 16 * 1024;
/// The most of an upstream error body this proxy will hold in memory while it retries.
const MAX_BUFFERED_ERROR_BODY: usize = 1024 * 1024;
/// The most header lines a request may carry before it is treated as an attack rather than a call.
const MAX_HEAD_LINES: usize = 200;

/// Headers that describe *this* hop and must not be carried to the next one (RFC 9110 § 7.6.1),
/// plus the non-standard `proxy-connection` that some clients still emit.
const HOP_BY_HOP: [&str; 9] = [
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "proxy-connection",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// The per-run loopback proxy.
///
/// A namespace rather than a value: the running proxy is [`LoopbackHandle`], and a caller holding
/// one has everything it needs — the base URL to hand the child, the placeholder to hand the
/// child, the counters, and the way to stop it.
pub struct LoopbackProxy;

impl LoopbackProxy {
    /// Bind 127.0.0.1 on an ephemeral port and serve one run's traffic.
    ///
    /// `upstream` is scheme + host, optionally with a port: `https://api.anthropic.com`, or
    /// `http://127.0.0.1:8931` for a local gateway or a test's fake upstream. A path is refused
    /// rather than joined, because a base with a path and a child that constructs `/v1/messages`
    /// disagree about the result and the failure would show up as a vendor 404.
    ///
    /// The port is ephemeral and the placeholder carries a random nonce: one run cannot use
    /// another's endpoint even by accident, and no other process on the machine can guess the
    /// placeholder in order to spend the operator's subscription through this port.
    ///
    /// # Errors
    ///
    /// The upstream could not be parsed, the loopback port could not be bound, the accept thread
    /// could not be started, or the OS entropy source refused.
    pub fn start(
        upstream: &str,
        custody: Arc<CredentialCustody>,
        run_id: &str,
    ) -> io::Result<LoopbackHandle> {
        let upstream = parse_upstream(upstream)?;
        let placeholder = placeholder_for(run_id, &nonce()?);
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        let counters = Arc::new(Counters::default());
        let stopping = Arc::new(AtomicBool::new(false));
        let proxy = Arc::new(Proxy {
            upstream,
            custody,
            placeholder: placeholder.clone(),
            counters: Arc::clone(&counters),
        });
        let accept = {
            let stopping = Arc::clone(&stopping);
            std::thread::Builder::new()
                .name(format!("mh-loopback-{port}"))
                .spawn(move || accept_loop(&listener, &proxy, &stopping))?
        };
        Ok(LoopbackHandle {
            port,
            placeholder,
            counters,
            stopping,
            accept: Some(accept),
        })
    }
}

/// A running per-run proxy. Dropping it stops it.
#[derive(Debug)]
pub struct LoopbackHandle {
    port: u16,
    placeholder: String,
    counters: Arc<Counters>,
    stopping: Arc<AtomicBool>,
    accept: Option<JoinHandle<()>>,
}

impl LoopbackHandle {
    /// What the child's `ANTHROPIC_BASE_URL` is set to.
    ///
    /// Plain HTTP and loopback-only, which is the design's own choice: the hop the operator wants
    /// to be able to inspect should not be encrypted, and it never leaves the machine.
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// The placeholder credential the child authenticates with: `mh-run-<run_id>-<nonce>`.
    ///
    /// It names the run, so a request arriving here can be attributed without a session table,
    /// and it is worthless anywhere else — which is the point of putting it in the child instead
    /// of the token.
    #[must_use]
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// The port this run's proxy is listening on.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The counters, readable while the run is still going.
    #[must_use]
    pub fn report(&self) -> ProxyReport {
        self.counters.report()
    }

    /// Stop accepting and join the accept thread.
    ///
    /// Connections already being relayed are **not** joined: an SSE stream can outlive the run
    /// that started it by a long time, and a shutdown that waited for one would hang the caller.
    ///
    /// **When this returns the listening socket is closed**, and that is a guarantee rather than a
    /// race: the accept thread *owns* the [`TcpListener`], so the join below is what drops it. The
    /// port **number** is a different thing and is nobody's afterwards — any process on the box may
    /// bind the ephemeral number this run has just released, within microseconds and without
    /// knowing it was ours. So "is something listening there?" is not a question about this proxy,
    /// and anything asserting this guarantee has to be written knowing that (`builder`'s
    /// `port_stops_accepting`, which carries the measurements).
    pub fn shutdown(mut self) {
        self.stop();
    }

    /// The idempotent half of [`LoopbackHandle::shutdown`], so `Drop` can share it.
    fn stop(&mut self) {
        if let Some(accept) = self.accept.take() {
            self.stopping.store(true, Ordering::SeqCst);
            // The accept call is blocking, so it is woken by giving it one last connection to
            // return from; it sees the flag and leaves. Nothing is written on this socket.
            let _ = TcpStream::connect(("127.0.0.1", self.port));
            let _ = accept.join();
        }
    }
}

impl Drop for LoopbackHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// What the proxy did, in four numbers.
///
/// Counted rather than logged: the design makes body logging opt-in and this build implements
/// none of it, so these are what a run can say about its own wire without carrying content.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProxyReport {
    /// Requests that carried the placeholder and were relayed upstream.
    pub forwarded: u64,
    /// Requests whose placeholder did not match: answered 401 here, never forwarded.
    pub refused: u64,
    /// Upstream 401s that triggered a custody re-read and one retry.
    pub refresh_and_retry: u64,
    /// Custody re-reads that came back byte-identical — the `auth.expired` signal.
    pub refresh_failed: u64,
}

/// The counters, in the form the serving threads share them.
#[derive(Debug, Default)]
struct Counters {
    forwarded: AtomicU64,
    refused: AtomicU64,
    refresh_and_retry: AtomicU64,
    refresh_failed: AtomicU64,
}

impl Counters {
    fn report(&self) -> ProxyReport {
        ProxyReport {
            forwarded: self.forwarded.load(Ordering::SeqCst),
            refused: self.refused.load(Ordering::SeqCst),
            refresh_and_retry: self.refresh_and_retry.load(Ordering::SeqCst),
            refresh_failed: self.refresh_failed.load(Ordering::SeqCst),
        }
    }
}

/// Everything a connection thread needs, shared by `Arc`.
#[derive(Debug)]
struct Proxy {
    upstream: Upstream,
    custody: Arc<CredentialCustody>,
    placeholder: String,
    counters: Arc<Counters>,
}

/// Where forwarded requests go.
#[derive(Debug, Clone)]
struct Upstream {
    tls: bool,
    host: String,
    port: u16,
    /// What goes in `Host:` — the authority exactly as the operator wrote it.
    authority: String,
}

/// Split `scheme://host[:port]` into something connectable.
fn parse_upstream(raw: &str) -> io::Result<Upstream> {
    let raw = raw.trim();
    let (scheme, rest) = raw.split_once("://").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("the upstream {raw} has no scheme: give http:// or https://"),
        )
    })?;
    let tls = match scheme.to_ascii_lowercase().as_str() {
        "https" => true,
        "http" => false,
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("the upstream scheme {other} is not http or https"),
            ));
        }
    };
    let authority = rest.trim_end_matches('/');
    if authority.contains('/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "the upstream {raw} carries a path: the child builds its own paths, so a base \
                 with one would produce a URL neither side intended. Give scheme://host[:port]"
            ),
        ));
    }
    if authority.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("the upstream {raw} names no host"),
        ));
    }
    let (host, port) = split_authority(authority, tls)?;
    Ok(Upstream {
        tls,
        host,
        port,
        authority: authority.to_string(),
    })
}

/// `host`, `host:port` or `[::1]:port` into the two pieces a socket needs.
fn split_authority(authority: &str, tls: bool) -> io::Result<(String, u16)> {
    let default = if tls { 443 } else { 80 };
    let bad_port = |text: &str| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("the upstream port {text} is not a port number"),
        )
    };
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, tail) = rest.split_once(']').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("the upstream {authority} opens an IPv6 literal and never closes it"),
            )
        })?;
        let port = match tail.strip_prefix(':') {
            Some(text) => text.parse().map_err(|_| bad_port(text))?,
            None => default,
        };
        return Ok((host.to_string(), port));
    }
    match authority.rsplit_once(':') {
        Some((host, text)) => Ok((host.to_string(), text.parse().map_err(|_| bad_port(text))?)),
        None => Ok((authority.to_string(), default)),
    }
}

/// 12 bytes of OS entropy, hex.
///
/// Not the clock and not the pid: the proxy listens where every process on the machine can reach
/// it, so a guessable placeholder is a way to spend the operator's subscription.
fn nonce() -> io::Result<String> {
    let mut bytes = [0u8; 12];
    getrandom::fill(&mut bytes)
        .map_err(|error| io::Error::other(format!("the OS entropy source refused: {error}")))?;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    Ok(out)
}

/// `mh-run-<run_id>-<nonce>`, with the run id reduced to what can live in a header value.
///
/// Reduced rather than rejected: a run id is chosen by the embedder for its own reasons, and a
/// proxy that refused to start over a space in it would be refusing the wrong thing.
fn placeholder_for(run_id: &str, nonce: &str) -> String {
    let safe: String = run_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("mh-run-{safe}-{nonce}")
}

/// Accept until told to stop, one detached thread per connection.
fn accept_loop(listener: &TcpListener, proxy: &Arc<Proxy>, stopping: &Arc<AtomicBool>) {
    for incoming in listener.incoming() {
        if stopping.load(Ordering::SeqCst) {
            break;
        }
        let Ok(stream) = incoming else {
            // A failed accept is not a reason to stop serving a run; a hot loop over one would be
            // worse than the failure, hence the pause.
            std::thread::sleep(Duration::from_millis(5));
            continue;
        };
        let proxy = Arc::clone(proxy);
        // Detached: a relay outlives the accept that started it, and an accept loop that joined
        // its children would stop accepting for the length of the longest stream.
        if std::thread::Builder::new()
            .name("mh-loopback-conn".to_string())
            .spawn(move || serve(&stream, &proxy))
            .is_err()
        {
            // Out of threads. There is nothing to relay the request with and nothing useful to
            // say on a socket we cannot service; the child sees the connection close.
        }
    }
}

/// One connection: one request, one answer, then close.
fn serve(stream: &TcpStream, proxy: &Proxy) {
    let _ = stream.set_nodelay(true);
    let _ = stream.set_read_timeout(Some(CHILD_REQUEST_TIMEOUT));
    let _ = stream.set_write_timeout(Some(UPSTREAM_IDLE_TIMEOUT));
    let mut child = stream;

    let request = {
        let mut reader = BufReader::new(stream);
        match read_request(&mut reader) {
            Ok(Some(request)) => request,
            // A connection opened and closed without asking anything — a probe, or the child
            // deciding not to speak. There is nothing to answer.
            Ok(None) => return,
            Err(error) if error.kind() == io::ErrorKind::Unsupported => {
                let _ = answer(
                    &mut child,
                    400,
                    "Bad Request",
                    "invalid_request_error",
                    &error.to_string(),
                );
                let _ = stream.shutdown(Shutdown::Write);
                return;
            }
            Err(_) => {
                // A request this proxy could not parse gets no status: any code invented here
                // would be a claim about a message nobody understood. The connection closes.
                let _ = stream.shutdown(Shutdown::Both);
                return;
            }
        }
    };

    if !presents_placeholder(&request.headers, &proxy.placeholder) {
        proxy.counters.refused.fetch_add(1, Ordering::SeqCst);
        let _ = answer(
            &mut child,
            401,
            "Unauthorized",
            "authentication_error",
            "metaharness loopback proxy: this request did not carry this run's placeholder \
             credential, so it was refused here and never forwarded upstream",
        );
        let _ = stream.shutdown(Shutdown::Write);
        return;
    }

    exchange(&request, proxy, &mut child);
    let _ = stream.shutdown(Shutdown::Write);
}

/// The authenticated half: attach the real token, forward, and handle a 401 once.
fn exchange<W: Write>(request: &Request, proxy: &Proxy, child: &mut W) {
    let token = match proxy.custody.bearer() {
        Ok(token) => token,
        Err(error) => {
            let _ = answer(
                child,
                502,
                "Bad Gateway",
                "api_error",
                &format!("metaharness loopback proxy: {error}"),
            );
            return;
        }
    };

    let mut first = match attempt(proxy, request, &token) {
        Ok(response) => response,
        Err(error) => {
            let _ = answer(
                child,
                502,
                "Bad Gateway",
                "api_error",
                &format!(
                    "metaharness loopback proxy: the upstream {} could not be reached: {error}",
                    proxy.upstream.authority
                ),
            );
            return;
        }
    };
    proxy.counters.forwarded.fetch_add(1, Ordering::SeqCst);

    if first.head.status != 401 {
        let _ = relay(child, &first.head, &mut first.body);
        return;
    }

    // The 401's own body is buffered before anything is written to the child, because it is the
    // answer the child gets if the refresh produces nothing — and by then this connection is gone.
    let stale_body = read_body(&mut first.body, &first.head, MAX_BUFFERED_ERROR_BODY)
        .unwrap_or_else(|_| Vec::new());
    let stale_head = first.head.clone();
    drop(first);
    after_401(request, proxy, child, &token, &stale_head, &stale_body);
}

/// Refresh custody and retry once, or relay the upstream's own 401.
fn after_401<W: Write>(
    request: &Request,
    proxy: &Proxy,
    child: &mut W,
    stale: &str,
    stale_head: &Head,
    stale_body: &[u8],
) {
    match proxy.custody.refreshed(stale) {
        Ok(fresh) => {
            proxy
                .counters
                .refresh_and_retry
                .fetch_add(1, Ordering::SeqCst);
            match attempt(proxy, request, &fresh) {
                Ok(mut second) => {
                    let _ = relay(child, &second.head, &mut second.body);
                }
                Err(error) => {
                    let _ = answer(
                        child,
                        502,
                        "Bad Gateway",
                        "api_error",
                        &format!(
                            "metaharness loopback proxy: the refreshed retry could not reach {}: \
                             {error}",
                            proxy.upstream.authority
                        ),
                    );
                }
            }
        }
        Err(CustodyError::StillStale) => {
            // The one case the design names: nothing refreshed the file, so the upstream's own
            // 401 is the truest thing to hand the child. Retrying the same token would spin.
            proxy.counters.refresh_failed.fetch_add(1, Ordering::SeqCst);
            let _ = replay(child, stale_head, stale_body);
        }
        Err(error) => {
            // Custody itself broke mid-run (the file vanished, or another process left it in a
            // shape this build does not read). That is not "the credential expired" and must not
            // be counted as one.
            let _ = answer(
                child,
                502,
                "Bad Gateway",
                "api_error",
                &format!("metaharness loopback proxy: {error}"),
            );
        }
    }
}

/// One forwarded attempt: connect, send, read the response head.
///
/// The body is left unread on purpose — the caller decides whether to pipe it or buffer it, and
/// a streaming body must not be touched before that decision.
fn attempt(proxy: &Proxy, request: &Request, token: &str) -> io::Result<UpstreamResponse> {
    let mut body = BufReader::new(connect(&proxy.upstream)?);
    write_upstream_request(body.get_mut(), request, token, &proxy.upstream)?;
    let head = read_head(&mut body)?;
    Ok(UpstreamResponse { head, body })
}

/// An upstream response whose head has been read and whose body has not.
struct UpstreamResponse {
    head: Head,
    body: BufReader<Box<dyn ReadWrite + Send>>,
}

/// A blocking byte stream in both directions: a plain socket, or rustls over one.
trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

/// Open the upstream leg, TLS when the scheme said so.
fn connect(upstream: &Upstream) -> io::Result<Box<dyn ReadWrite + Send>> {
    let mut last = io::Error::other(format!("{} resolved to no address", upstream.authority));
    let mut socket = None;
    for address in (upstream.host.as_str(), upstream.port).to_socket_addrs()? {
        match TcpStream::connect_timeout(&address, UPSTREAM_CONNECT_TIMEOUT) {
            Ok(open) => {
                socket = Some(open);
                break;
            }
            Err(error) => last = error,
        }
    }
    let socket = socket.ok_or(last)?;
    socket.set_nodelay(true)?;
    socket.set_read_timeout(Some(UPSTREAM_IDLE_TIMEOUT))?;
    socket.set_write_timeout(Some(UPSTREAM_IDLE_TIMEOUT))?;
    if !upstream.tls {
        return Ok(Box::new(socket));
    }
    let name = rustls::pki_types::ServerName::try_from(upstream.host.clone()).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a TLS server name: {error}", upstream.host),
        )
    })?;
    let session = rustls::ClientConnection::new(tls_config(), name).map_err(io::Error::other)?;
    Ok(Box::new(rustls::StreamOwned::new(session, socket)))
}

/// The one TLS client configuration, built once.
///
/// Mozilla's trust anchors compiled in rather than the platform store, so the proxy trusts the
/// same roots on a laptop and in CI; `ring` rather than `aws-lc-rs` so the build needs no cmake.
fn tls_config() -> Arc<rustls::ClientConfig> {
    static CONFIG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    Arc::clone(CONFIG.get_or_init(|| {
        let roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        Arc::new(
            rustls::ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .expect("ring supports rustls's default protocol versions")
                .with_root_certificates(roots)
                .with_no_client_auth(),
        )
    }))
}

/// One request as the child sent it, with its body already in memory.
///
/// Buffered rather than streamed because of the 401 retry: a request whose body had been piped
/// upstream has nothing left to resend, and the retry is the feature.
#[derive(Debug, Clone)]
struct Request {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    /// Whether the child framed a body at all, so a `Content-Length: 0` POST stays one.
    framed_body: bool,
}

/// A response status line and its headers.
#[derive(Debug, Clone)]
struct Head {
    status: u16,
    reason: String,
    headers: Vec<(String, String)>,
}

/// Read one HTTP/1.1 request, body included. `None` at a clean end of stream.
fn read_request<R: BufRead>(reader: &mut R) -> io::Result<Option<Request>> {
    let Some(lines) = read_head_lines(reader)? else {
        return Ok(None);
    };
    let mut lines = lines.into_iter();
    let start = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "an empty request line"))?;
    let mut parts = start.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "a request line with no method"))?
        .to_string();
    let target = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "a request line with no target"))?
        .to_string();
    let headers: Vec<(String, String)> = lines.filter_map(|line| split_header(&line)).collect();

    if header(&headers, "transfer-encoding").is_some() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "metaharness loopback proxy: this build does not accept a chunked request body. The \
             body must be buffered so an upstream 401 can be retried with a refreshed token, and \
             no observed vendor request needs chunked upload",
        ));
    }
    let length = match header(&headers, "content-length") {
        Some(text) => Some(text.trim().parse::<usize>().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Content-Length {text} is not a length"),
            )
        })?),
        None => None,
    };
    let mut body = vec![0u8; length.unwrap_or(0)];
    reader.read_exact(&mut body)?;
    Ok(Some(Request {
        method,
        target,
        headers,
        body,
        framed_body: length.is_some(),
    }))
}

/// Read one HTTP/1.1 response head.
fn read_head<R: BufRead>(reader: &mut R) -> io::Result<Head> {
    let lines = read_head_lines(reader)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "the upstream closed without answering",
        )
    })?;
    let mut lines = lines.into_iter();
    let start = lines.next().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "an empty upstream status line")
    })?;
    let mut parts = start.splitn(3, ' ');
    let _version = parts.next();
    let status = parts
        .next()
        .and_then(|text| text.parse::<u16>().ok())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("the upstream status line {start} carries no status code"),
            )
        })?;
    let reason = parts.next().unwrap_or("").to_string();
    Ok(Head {
        status,
        reason,
        headers: lines.filter_map(|line| split_header(&line)).collect(),
    })
}

/// The lines up to the blank one, `None` when the peer closed before sending any.
///
/// Lossy UTF-8 rather than a hard failure: header bytes are ASCII by specification, and a
/// malformed byte in one header is not a reason to drop a request the upstream would have
/// answered.
fn read_head_lines<R: BufRead>(reader: &mut R) -> io::Result<Option<Vec<String>>> {
    let mut lines: Vec<String> = Vec::new();
    loop {
        let mut raw = Vec::new();
        if reader.read_until(b'\n', &mut raw)? == 0 {
            if lines.is_empty() {
                return Ok(None);
            }
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the head ended without a blank line",
            ));
        }
        let line = String::from_utf8_lossy(&raw)
            .trim_end_matches(['\r', '\n'])
            .to_string();
        if line.is_empty() {
            return Ok(Some(lines));
        }
        if lines.len() >= MAX_HEAD_LINES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("more than {MAX_HEAD_LINES} header lines"),
            ));
        }
        lines.push(line);
    }
}

/// `Name: value` into its two halves, or nothing when the line is not a header.
fn split_header(line: &str) -> Option<(String, String)> {
    let (name, value) = line.split_once(':')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), value.trim().to_string()))
}

/// The first value of a header, case-insensitively.
fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

/// Does this request carry this run's placeholder, in either spelling?
///
/// Both spellings are accepted because both are observed vendor behaviour: Claude Code sends
/// `x-api-key` against a custom base (model-adapter design, verified), and `ANTHROPIC_AUTH_TOKEN`
/// is documented as a bearer.
fn presents_placeholder(headers: &[(String, String)], placeholder: &str) -> bool {
    headers
        .iter()
        .filter(|(name, _)| is_placeholder_header(name))
        .any(|(_, value)| same_secret(presented_token(value), placeholder))
}

/// The credential out of an `Authorization` or `x-api-key` value.
fn presented_token(value: &str) -> &str {
    let value = value.trim();
    match value.split_once(' ') {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("bearer") => rest.trim(),
        _ => value,
    }
}

/// Compare two secrets without an early exit.
///
/// The placeholder is the only thing between a local process and the operator's subscription, and
/// this loop runs once per request: a comparison that returned at the first differing byte would
/// leak its prefix to anything that can time it.
fn same_secret(presented: &str, expected: &str) -> bool {
    let (presented, expected) = (presented.as_bytes(), expected.as_bytes());
    if presented.len() != expected.len() {
        return false;
    }
    presented
        .iter()
        .zip(expected)
        .fold(0u8, |accumulated, (a, b)| accumulated | (a ^ b))
        == 0
}

/// Is this header one of the two spellings of the child's credential?
fn is_placeholder_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("authorization") || name.eq_ignore_ascii_case("x-api-key")
}

/// Does this header describe this hop rather than the message?
fn is_hop_by_hop(name: &str) -> bool {
    HOP_BY_HOP
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

/// Send the child's request upstream with the real credential on it.
fn write_upstream_request<W: Write>(
    out: &mut W,
    request: &Request,
    token: &str,
    upstream: &Upstream,
) -> io::Result<()> {
    let mut head = String::with_capacity(512);
    let _ = write!(head, "{} {} HTTP/1.1\r\n", request.method, request.target);
    let _ = write!(head, "Host: {}\r\n", upstream.authority);
    let _ = write!(head, "Authorization: Bearer {token}\r\n");
    head.push_str("Connection: close\r\n");
    for (name, value) in &request.headers {
        // Everything the child sent, except what describes this hop and except the placeholder in
        // either spelling. `anthropic-version` and `anthropic-beta` ride through untouched: they
        // are the contract between the child and the upstream, not between the child and us.
        if is_hop_by_hop(name)
            || is_placeholder_header(name)
            || name.eq_ignore_ascii_case("host")
            || name.eq_ignore_ascii_case("content-length")
        {
            continue;
        }
        let _ = write!(head, "{name}: {value}\r\n");
    }
    if request.framed_body || !request.body.is_empty() {
        let _ = write!(head, "Content-Length: {}\r\n", request.body.len());
    }
    head.push_str("\r\n");
    out.write_all(head.as_bytes())?;
    out.write_all(&request.body)?;
    out.flush()
}

/// How a body's end is decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Framing {
    /// Exactly this many bytes.
    Length(usize),
    /// Chunk-framed; this proxy relays the framing verbatim.
    Chunked,
    /// Until the connection closes.
    ToEnd,
}

/// Read a response's framing out of its headers.
fn framing(head: &Head) -> Framing {
    if header(&head.headers, "transfer-encoding")
        .is_some_and(|value| value.to_ascii_lowercase().contains("chunked"))
    {
        return Framing::Chunked;
    }
    match header(&head.headers, "content-length").and_then(|text| text.trim().parse::<usize>().ok())
    {
        Some(length) => Framing::Length(length),
        None => Framing::ToEnd,
    }
}

/// Relay an upstream response to the child as it arrives.
///
/// The head is written and flushed before a byte of body is read, and every chunk of body is
/// flushed as it lands. That is the whole of the SSE requirement: a relay that flushed once at
/// the end would still be byte-correct and would have destroyed streaming.
fn relay<W: Write, R: BufRead>(child: &mut W, head: &Head, body: &mut R) -> io::Result<()> {
    write_child_head(child, head)?;
    child.flush()?;
    match framing(head) {
        Framing::Length(length) => copy_exact(body, child, length),
        Framing::Chunked | Framing::ToEnd => copy_to_end(body, child),
    }
}

/// Write a response this proxy has already buffered whole.
///
/// The original framing headers are dropped and replaced by a `Content-Length`: the bytes in hand
/// are decoded, and claiming they are still chunked would hand the child something it cannot
/// parse.
fn replay<W: Write>(child: &mut W, head: &Head, body: &[u8]) -> io::Result<()> {
    let mut rewritten = Head {
        status: head.status,
        reason: head.reason.clone(),
        headers: head
            .headers
            .iter()
            .filter(|(name, _)| !name.eq_ignore_ascii_case("content-length"))
            .filter(|(name, _)| !name.eq_ignore_ascii_case("transfer-encoding"))
            .cloned()
            .collect(),
    };
    rewritten
        .headers
        .push(("Content-Length".to_string(), body.len().to_string()));
    write_child_head(child, &rewritten)?;
    child.write_all(body)?;
    child.flush()
}

/// The status line and headers this proxy hands the child.
fn write_child_head<W: Write>(child: &mut W, head: &Head) -> io::Result<()> {
    let mut out = String::with_capacity(512);
    let _ = write!(out, "HTTP/1.1 {} {}\r\n", head.status, head.reason);
    for (name, value) in &head.headers {
        // `Transfer-Encoding` survives, because a chunked body is relayed with its framing
        // intact; every other hop-by-hop header is this connection's business and not the
        // child's.
        if is_hop_by_hop(name) && !name.eq_ignore_ascii_case("transfer-encoding") {
            continue;
        }
        let _ = write!(out, "{name}: {value}\r\n");
    }
    out.push_str("Connection: close\r\n\r\n");
    child.write_all(out.as_bytes())
}

/// A JSON error from the proxy itself, in the vendor's error shape so the child's own parser
/// understands it, and naming metaharness so the operator knows who answered.
fn answer<W: Write>(
    child: &mut W,
    status: u16,
    reason: &str,
    kind: &str,
    message: &str,
) -> io::Result<()> {
    let body = serde_json::json!({
        "type": "error",
        "error": { "type": kind, "message": message },
    })
    .to_string();
    let mut out = String::with_capacity(body.len() + 160);
    let _ = write!(out, "HTTP/1.1 {status} {reason}\r\n");
    out.push_str("Content-Type: application/json\r\n");
    let _ = write!(out, "Content-Length: {}\r\n", body.len());
    out.push_str("Connection: close\r\n\r\n");
    out.push_str(&body);
    child.write_all(out.as_bytes())?;
    child.flush()
}

/// Read a whole body into memory, whatever its framing, up to `limit`.
fn read_body<R: BufRead>(body: &mut R, head: &Head, limit: usize) -> io::Result<Vec<u8>> {
    match framing(head) {
        Framing::Length(length) => {
            let mut out = vec![0u8; length.min(limit)];
            body.read_exact(&mut out)?;
            Ok(out)
        }
        Framing::Chunked => read_chunked(body, limit),
        Framing::ToEnd => {
            let mut out = Vec::new();
            body.take(limit as u64).read_to_end(&mut out)?;
            Ok(out)
        }
    }
}

/// Decode a chunk-framed body into the bytes it carries.
fn read_chunked<R: BufRead>(body: &mut R, limit: usize) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let mut line = Vec::new();
        if body.read_until(b'\n', &mut line)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "a chunked body ended without its terminating chunk",
            ));
        }
        let text = String::from_utf8_lossy(&line);
        // A chunk size may carry extensions after a semicolon; nothing here uses them.
        let size_text = text
            .trim()
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let size = usize::from_str_radix(&size_text, 16).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{size_text} is not a chunk size"),
            )
        })?;
        if size == 0 {
            // Trailers, then the blank line that ends the body.
            loop {
                let mut trailer = Vec::new();
                if body.read_until(b'\n', &mut trailer)? == 0 {
                    break;
                }
                if String::from_utf8_lossy(&trailer).trim().is_empty() {
                    break;
                }
            }
            return Ok(out);
        }
        if out.len() + size > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("a chunked body exceeded {limit} bytes"),
            ));
        }
        let start = out.len();
        out.resize(start + size, 0);
        body.read_exact(&mut out[start..])?;
        let mut crlf = [0u8; 2];
        body.read_exact(&mut crlf)?;
    }
}

/// Copy exactly `length` bytes, flushing as they arrive.
fn copy_exact<R: Read, W: Write>(from: &mut R, to: &mut W, length: usize) -> io::Result<()> {
    let mut buffer = [0u8; RELAY_CHUNK];
    let mut left = length;
    while left > 0 {
        let want = left.min(buffer.len());
        let read = from.read(&mut buffer[..want])?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the upstream closed before its Content-Length was satisfied",
            ));
        }
        to.write_all(&buffer[..read])?;
        to.flush()?;
        left -= read;
    }
    Ok(())
}

/// Copy until the upstream closes, flushing as bytes arrive.
fn copy_to_end<R: Read, W: Write>(from: &mut R, to: &mut W) -> io::Result<()> {
    let mut buffer = [0u8; RELAY_CHUNK];
    loop {
        match from.read(&mut buffer) {
            Ok(0) => return to.flush(),
            Ok(read) => {
                to.write_all(&buffer[..read])?;
                to.flush()?;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            // A TLS peer that closes without `close_notify` surfaces as `UnexpectedEof`. For a
            // close-delimited body that is the end of the body, not a fault to hand the child
            // halfway through a stream it was reading fine.
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return to.flush(),
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::mpsc::{Receiver, channel};

    /// The three events the streaming vector pushes through the proxy.
    const SSE_EVENTS: [&str; 3] = [
        "event: message_start\ndata: {\"type\":\"message_start\"}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\"}\n\n",
        "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
    ];

    /// A credential file with the vendor's shape and a token no account has ever held.
    fn fake_credential(dir: &Path, token: &str) -> PathBuf {
        let path = dir.join(".credentials.json");
        write_fake_credential(&path, token);
        path
    }

    fn write_fake_credential(path: &Path, token: &str) {
        let body = serde_json::json!({
            "claudeAiOauth": {
                "accessToken": token,
                "refreshToken": "fake-refresh-token",
                "expiresAt": 4_102_444_800_000_i64,
                "refreshTokenExpiresAt": 4_102_444_800_000_i64,
                "scopes": ["user:inference"],
                "subscriptionType": "fake",
                "rateLimitTier": "fake",
            }
        });
        std::fs::write(path, serde_json::to_vec(&body).expect("a credential body"))
            .expect("the fake credential");
    }

    fn custody_over(dir: &Path, token: &str) -> (PathBuf, Arc<CredentialCustody>) {
        let path = fake_credential(dir, token);
        let custody = CredentialCustody::open(&path).expect("the fake credential opens");
        (path, Arc::new(custody))
    }

    /// One request the fake upstream saw, exactly as it came off the wire.
    #[derive(Debug, Clone)]
    struct Seen {
        method: String,
        target: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    impl Seen {
        fn header(&self, name: &str) -> Option<&str> {
            super::header(&self.headers, name)
        }
    }

    /// What the fake upstream does with one connection.
    enum Reply {
        /// A complete, `Content-Length`-framed answer.
        Fixed { status: u16, body: String },
        /// Rewrite the credential file, *then* answer 401 — so the re-read the proxy performs
        /// next finds a token that really did change, without the test having to guess when.
        RotateThen401 { credential: PathBuf, fresh: String },
        /// An SSE body whose second and third events are withheld until the child has read the
        /// first. A proxy that buffered would deadlock here, which `stalled` records.
        Sse {
            resume: Receiver<()>,
            stalled: Arc<AtomicBool>,
        },
    }

    /// An upstream that answers from a script and records what it was asked.
    struct FakeUpstream {
        port: u16,
        seen: Arc<Mutex<Vec<Seen>>>,
        stopping: Arc<AtomicBool>,
    }

    impl FakeUpstream {
        fn serving(replies: Vec<Reply>) -> Self {
            let listener = TcpListener::bind(("127.0.0.1", 0)).expect("a fake upstream port");
            let port = listener.local_addr().expect("its address").port();
            let seen = Arc::new(Mutex::new(Vec::new()));
            let stopping = Arc::new(AtomicBool::new(false));
            let queue = Arc::new(Mutex::new(VecDeque::from(replies)));
            {
                let seen = Arc::clone(&seen);
                let stopping = Arc::clone(&stopping);
                std::thread::spawn(move || {
                    for incoming in listener.incoming() {
                        if stopping.load(Ordering::SeqCst) {
                            break;
                        }
                        let Ok(stream) = incoming else { break };
                        let reply = queue.lock().expect("the script").pop_front();
                        let seen = Arc::clone(&seen);
                        std::thread::spawn(move || fake_connection(&stream, &seen, reply));
                    }
                });
            }
            Self {
                port,
                seen,
                stopping,
            }
        }

        fn base(&self) -> String {
            format!("http://127.0.0.1:{}", self.port)
        }

        fn authority(&self) -> String {
            format!("127.0.0.1:{}", self.port)
        }

        fn requests(&self) -> Vec<Seen> {
            self.seen.lock().expect("the record").clone()
        }
    }

    impl Drop for FakeUpstream {
        fn drop(&mut self) {
            self.stopping.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", self.port));
        }
    }

    fn fake_connection(stream: &TcpStream, seen: &Mutex<Vec<Seen>>, reply: Option<Reply>) {
        let request = {
            let mut reader = BufReader::new(stream);
            // The same reader the proxy uses: the fake upstream speaks the dialect the proxy
            // writes, so a malformed forward fails here rather than being absorbed.
            match read_request(&mut reader) {
                Ok(Some(request)) => request,
                _ => return,
            }
        };
        seen.lock().expect("the record").push(Seen {
            method: request.method.clone(),
            target: request.target.clone(),
            headers: request.headers.clone(),
            body: request.body.clone(),
        });
        let mut out = stream;
        match reply {
            Some(Reply::Fixed { status, body }) => fixed(&mut out, status, &body),
            Some(Reply::RotateThen401 { credential, fresh }) => {
                write_fake_credential(&credential, &fresh);
                fixed(
                    &mut out,
                    401,
                    r#"{"type":"error","error":{"type":"expired"}}"#,
                );
            }
            Some(Reply::Sse { resume, stalled }) => sse(&mut out, &resume, &stalled),
            None => fixed(
                &mut out,
                500,
                r#"{"error":"the script ran out of replies"}"#,
            ),
        }
        let _ = stream.shutdown(Shutdown::Both);
    }

    fn fixed<W: Write>(out: &mut W, status: u16, body: &str) {
        let head = format!(
            "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n",
            if status == 200 { "OK" } else { "Error" },
            body.len()
        );
        out.write_all(head.as_bytes()).expect("the fake head");
        out.write_all(body.as_bytes()).expect("the fake body");
        out.flush().expect("the fake flush");
    }

    fn sse<W: Write>(out: &mut W, resume: &Receiver<()>, stalled: &AtomicBool) {
        let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
                    Cache-Control: no-store\r\nConnection: close\r\n\r\n";
        out.write_all(head.as_bytes()).expect("the fake head");
        out.flush().expect("the fake flush");
        out.write_all(SSE_EVENTS[0].as_bytes())
            .expect("the first event");
        out.flush().expect("the fake flush");
        if resume.recv_timeout(Duration::from_secs(5)).is_err() {
            stalled.store(true, Ordering::SeqCst);
        }
        for event in &SSE_EVENTS[1..] {
            out.write_all(event.as_bytes()).expect("a later event");
            out.flush().expect("the fake flush");
        }
    }

    /// A `POST /v1/messages` with the vendor's own headers on it.
    fn messages_request(auth: &str) -> String {
        let body = r#"{"model":"claude-opus-5","messages":[]}"#;
        format!(
            "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1\r\n{auth}\r\n\
             anthropic-version: 2023-06-01\r\nanthropic-beta: oauth-2025-04-20\r\n\
             content-type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
    }

    /// Speak one request to the proxy and read the whole answer.
    fn ask(port: u16, request: &str) -> (String, Vec<u8>) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("the proxy port");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("a bounded wait");
        stream
            .write_all(request.as_bytes())
            .expect("the whole request");
        stream.flush().expect("the flush");
        let mut all = Vec::new();
        stream.read_to_end(&mut all).expect("the whole answer");
        let separator = all
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("a head and a body, separated");
        (
            String::from_utf8_lossy(&all[..separator]).to_string(),
            all[separator + 4..].to_vec(),
        )
    }

    /// Vector 1, bearer spelling. The upstream must see the custody token and no trace of the
    /// placeholder — a proxy that forwarded both would leave the child's credential in the
    /// vendor's logs and could authenticate as the wrong thing.
    #[test]
    fn a_bearer_placeholder_is_swapped_for_the_custody_token() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let (_, custody) = custody_over(dir.path(), "fake-upstream-token");
        let upstream = FakeUpstream::serving(vec![Reply::Fixed {
            status: 200,
            body: r#"{"ok":true}"#.to_string(),
        }]);
        let proxy =
            LoopbackProxy::start(&upstream.base(), custody, "bearer-swap").expect("a proxy");

        let auth = format!("Authorization: Bearer {}", proxy.placeholder());
        let (head, body) = ask(proxy.port(), &messages_request(&auth));

        assert!(
            head.starts_with("HTTP/1.1 200 OK"),
            "the child must get the upstream's own answer, got: {head}"
        );
        assert_eq!(body, br#"{"ok":true}"#, "the body is relayed verbatim");
        let seen = upstream.requests();
        assert_eq!(seen.len(), 1, "exactly one request reached the upstream");
        assert_eq!(
            seen[0].header("authorization"),
            Some("Bearer fake-upstream-token"),
            "the upstream must see the custody token, not the placeholder"
        );
        assert_eq!(
            seen[0].header("x-api-key"),
            None,
            "both spellings of the child's credential are stripped, not just the one it used"
        );
        assert!(
            !seen[0]
                .headers
                .iter()
                .any(|(_, value)| value.contains(proxy.placeholder())),
            "no header may still carry the placeholder: {:?}",
            seen[0].headers
        );
        assert_eq!(
            seen[0].header("anthropic-version"),
            Some("2023-06-01"),
            "the vendor's own headers ride through uncurated, or the proxy has silently changed \
             which API the child is calling"
        );
        assert_eq!(seen[0].header("anthropic-beta"), Some("oauth-2025-04-20"));
        assert_eq!(
            seen[0].header("host"),
            Some(upstream.authority().as_str()),
            "Host names the upstream, not the loopback port the child dialled"
        );
        assert_eq!(seen[0].body, br#"{"model":"claude-opus-5","messages":[]}"#);
        assert_eq!(proxy.report().forwarded, 1);
        proxy.shutdown();
    }

    /// Vector 1, `x-api-key` spelling — the one Claude Code was observed sending against a custom
    /// base (model-adapter design, verified). Same outcome, or the loopback provider is unusable
    /// with the harness it was built for.
    #[test]
    fn an_x_api_key_placeholder_is_swapped_for_a_bearer_of_the_custody_token() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let (_, custody) = custody_over(dir.path(), "fake-upstream-token");
        let upstream = FakeUpstream::serving(vec![Reply::Fixed {
            status: 200,
            body: r#"{"ok":true}"#.to_string(),
        }]);
        let proxy = LoopbackProxy::start(&upstream.base(), custody, "key-swap").expect("a proxy");

        let auth = format!("x-api-key: {}", proxy.placeholder());
        let (head, _) = ask(proxy.port(), &messages_request(&auth));

        assert!(head.starts_with("HTTP/1.1 200 OK"), "got: {head}");
        let seen = upstream.requests();
        assert_eq!(
            seen[0].header("authorization"),
            Some("Bearer fake-upstream-token"),
            "the x-api-key spelling is authenticated and re-spelled as the bearer the upstream \
             expects"
        );
        assert_eq!(
            seen[0].header("x-api-key"),
            None,
            "the placeholder's own header is removed, not forwarded alongside the real one"
        );
        proxy.shutdown();
    }

    /// Vector 2. The port is on 127.0.0.1 where every process on the machine can reach it, so a
    /// request without the placeholder must die here — forwarding it would spend the operator's
    /// subscription for whoever asked.
    #[test]
    fn a_request_without_the_placeholder_is_refused_here_and_never_forwarded() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let (_, custody) = custody_over(dir.path(), "fake-upstream-token");
        let upstream = FakeUpstream::serving(vec![Reply::Fixed {
            status: 200,
            body: r#"{"ok":true}"#.to_string(),
        }]);
        let proxy = LoopbackProxy::start(&upstream.base(), custody, "refusal").expect("a proxy");

        let (head, body) = ask(
            proxy.port(),
            &messages_request("Authorization: Bearer mh-run-refusal-000000000000000000000000"),
        );

        assert!(
            head.starts_with("HTTP/1.1 401 Unauthorized"),
            "a mismatched placeholder is answered 401, got: {head}"
        );
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("metaharness"),
            "the refusal names who refused, or the operator debugs the vendor instead: {text}"
        );
        assert!(
            upstream.requests().is_empty(),
            "the upstream must never have been contacted"
        );
        let report = proxy.report();
        assert_eq!(report.refused, 1, "the refusal is counted");
        assert_eq!(report.forwarded, 0, "and is not counted as a forward");
        proxy.shutdown();
    }

    #[test]
    fn a_request_with_no_credential_at_all_is_refused_the_same_way() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let (_, custody) = custody_over(dir.path(), "fake-upstream-token");
        let upstream = FakeUpstream::serving(vec![Reply::Fixed {
            status: 200,
            body: "{}".to_string(),
        }]);
        let proxy = LoopbackProxy::start(&upstream.base(), custody, "bare").expect("a proxy");

        let (head, _) = ask(proxy.port(), &messages_request("x-unrelated: 1"));

        assert!(head.starts_with("HTTP/1.1 401"), "got: {head}");
        assert!(
            upstream.requests().is_empty(),
            "an unauthenticated request is not a forwardable one"
        );
        assert_eq!(proxy.report().refused, 1);
        proxy.shutdown();
    }

    /// Vector 3. The fake upstream refuses to write events 2 and 3 until the child has read event
    /// 1, so a proxy that buffered the body to completion deadlocks and sets `stalled`. Byte
    /// identity alone would not catch that — a fully buffered relay is byte-identical too.
    #[test]
    fn a_streaming_body_reaches_the_child_incrementally_and_byte_identical() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let (_, custody) = custody_over(dir.path(), "fake-upstream-token");
        let (resume, resume_rx) = channel();
        let stalled = Arc::new(AtomicBool::new(false));
        let upstream = FakeUpstream::serving(vec![Reply::Sse {
            resume: resume_rx,
            stalled: Arc::clone(&stalled),
        }]);
        let proxy = LoopbackProxy::start(&upstream.base(), custody, "stream").expect("a proxy");

        let stream = TcpStream::connect(("127.0.0.1", proxy.port())).expect("the proxy port");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("a bounded wait");
        let auth = format!("Authorization: Bearer {}", proxy.placeholder());
        (&stream)
            .write_all(messages_request(&auth).as_bytes())
            .expect("the request");
        let mut reader = BufReader::new(&stream);

        let head = read_head(&mut reader).expect("the relayed head");
        assert_eq!(
            head.status, 200,
            "the stream's head arrives before its body"
        );
        assert_eq!(
            header(&head.headers, "content-type"),
            Some("text/event-stream"),
            "the upstream's content type is relayed, or the child will not treat it as a stream"
        );

        let mut first = vec![0u8; SSE_EVENTS[0].len()];
        reader
            .read_exact(&mut first)
            .expect("the first event, before the upstream has written the rest");
        assert_eq!(String::from_utf8_lossy(&first), SSE_EVENTS[0]);

        resume.send(()).expect("release the upstream");
        let mut rest = Vec::new();
        reader.read_to_end(&mut rest).expect("the remaining events");

        assert!(
            !stalled.load(Ordering::SeqCst),
            "the upstream waited 5s for the child to read event 1 and gave up: the proxy buffered \
             the stream instead of piping it"
        );
        let whole = [first, rest].concat();
        assert_eq!(
            String::from_utf8_lossy(&whole),
            SSE_EVENTS.concat(),
            "the body must arrive byte-identical, framing included"
        );
        assert_eq!(proxy.report().forwarded, 1);
        proxy.shutdown();
    }

    /// Vector 4. The upstream rotates the credential file and answers 401; the proxy re-reads,
    /// retries once, and the child sees only the 200. The child holds a placeholder, so if it
    /// ever saw the 401 it would have no way to recover from it.
    #[test]
    fn an_upstream_401_is_refreshed_and_retried_once_and_the_child_never_sees_it() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let (path, custody) = custody_over(dir.path(), "fake-upstream-token-stale");
        let upstream = FakeUpstream::serving(vec![
            Reply::RotateThen401 {
                credential: path,
                fresh: "fake-upstream-token-fresh".to_string(),
            },
            Reply::Fixed {
                status: 200,
                body: r#"{"ok":true}"#.to_string(),
            },
        ]);
        let proxy = LoopbackProxy::start(&upstream.base(), custody, "retry").expect("a proxy");

        let auth = format!("Authorization: Bearer {}", proxy.placeholder());
        let (head, body) = ask(proxy.port(), &messages_request(&auth));

        assert!(
            head.starts_with("HTTP/1.1 200 OK"),
            "the retry's answer is what the child gets, got: {head}"
        );
        assert_eq!(body, br#"{"ok":true}"#);
        let seen = upstream.requests();
        assert_eq!(seen.len(), 2, "one attempt and exactly one retry");
        assert_eq!(
            seen[0].header("authorization"),
            Some("Bearer fake-upstream-token-stale"),
            "the first attempt carried the token custody held at the time"
        );
        assert_eq!(
            seen[1].header("authorization"),
            Some("Bearer fake-upstream-token-fresh"),
            "the retry carried the re-read token, or the refresh changed nothing"
        );
        assert_eq!(
            seen[1].body, seen[0].body,
            "the retry resends the child's body, which is why the request is buffered at all"
        );
        let report = proxy.report();
        assert_eq!(report.refresh_and_retry, 1);
        assert_eq!(report.refresh_failed, 0);
        assert_eq!(
            report.forwarded, 1,
            "one child request, however many attempts"
        );
        proxy.shutdown();
    }

    /// Vector 5. Nothing rewrote the file, so the re-read is byte-identical: that is the whole of
    /// the "refresh failed" signal in v1, and the child gets the upstream's own 401 rather than a
    /// metaharness paraphrase of it.
    #[test]
    fn a_refresh_that_reads_the_same_token_relays_the_upstreams_own_401() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let (_, custody) = custody_over(dir.path(), "fake-upstream-token-stale");
        let upstream = FakeUpstream::serving(vec![Reply::Fixed {
            status: 401,
            body: r#"{"type":"error","error":{"type":"authentication_error"}}"#.to_string(),
        }]);
        let proxy = LoopbackProxy::start(&upstream.base(), custody, "stale").expect("a proxy");

        let auth = format!("Authorization: Bearer {}", proxy.placeholder());
        let (head, body) = ask(proxy.port(), &messages_request(&auth));

        assert!(head.starts_with("HTTP/1.1 401"), "got: {head}");
        assert_eq!(
            body, br#"{"type":"error","error":{"type":"authentication_error"}}"#,
            "the upstream's own reason reaches the child, not a substitute"
        );
        assert_eq!(
            upstream.requests().len(),
            1,
            "a refresh that produced nothing must not be retried, or the proxy spins on a token \
             the upstream already rejected"
        );
        let report = proxy.report();
        assert_eq!(report.refresh_failed, 1);
        assert_eq!(report.refresh_and_retry, 0);
        proxy.shutdown();
    }

    /// Vector 6. No path allowlist: a vendor endpoint nobody catalogued must not break the run,
    /// so method, path and query go through as written.
    #[test]
    fn an_uncatalogued_path_is_forwarded_generically_with_its_query() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let (_, custody) = custody_over(dir.path(), "fake-upstream-token");
        let upstream = FakeUpstream::serving(vec![
            Reply::Fixed {
                status: 200,
                body: r#"{"data":[]}"#.to_string(),
            },
            Reply::Fixed {
                status: 200,
                body: r#"{"anything":true}"#.to_string(),
            },
        ]);
        let proxy = LoopbackProxy::start(&upstream.base(), custody, "generic").expect("a proxy");
        let placeholder = proxy.placeholder().to_string();

        let (models_head, models_body) = ask(
            proxy.port(),
            &format!(
                "GET /v1/models?limit=5 HTTP/1.1\r\nHost: 127.0.0.1\r\n\
                 Authorization: Bearer {placeholder}\r\n\r\n"
            ),
        );
        let payload = r#"{"free":"form"}"#;
        let (other_head, other_body) = ask(
            proxy.port(),
            &format!(
                "POST /anything/else HTTP/1.1\r\nHost: 127.0.0.1\r\nx-api-key: {placeholder}\r\n\
                 Content-Length: {}\r\n\r\n{payload}",
                payload.len()
            ),
        );

        assert!(models_head.starts_with("HTTP/1.1 200"), "{models_head}");
        assert_eq!(models_body, br#"{"data":[]}"#);
        assert!(other_head.starts_with("HTTP/1.1 200"), "{other_head}");
        assert_eq!(other_body, br#"{"anything":true}"#);

        let seen = upstream.requests();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].method, "GET");
        assert_eq!(
            seen[0].target, "/v1/models?limit=5",
            "the query survives, or a paginated endpoint silently returns the wrong page"
        );
        assert_eq!(seen[1].method, "POST");
        assert_eq!(seen[1].target, "/anything/else");
        assert_eq!(seen[1].body, payload.as_bytes());
        assert_eq!(proxy.report().forwarded, 2);
        proxy.shutdown();
    }

    /// The placeholder names the run, which is what makes a request arriving on this port
    /// attributable without a session table.
    #[test]
    fn the_placeholder_names_the_run_and_carries_a_nonce() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let (_, custody) = custody_over(dir.path(), "fake-upstream-token");
        let upstream = FakeUpstream::serving(Vec::new());
        let one =
            LoopbackProxy::start(&upstream.base(), Arc::clone(&custody), "run-7").expect("a proxy");
        let two = LoopbackProxy::start(&upstream.base(), custody, "run-7").expect("another");

        assert!(
            one.placeholder().starts_with("mh-run-run-7-"),
            "got {}",
            one.placeholder()
        );
        assert_ne!(
            one.placeholder(),
            two.placeholder(),
            "two runs with the same id must not share a placeholder, or one run's child can spend \
             through the other's port"
        );
        assert!(
            one.base_url().starts_with("http://127.0.0.1:"),
            "the base is loopback and plain HTTP, so the hop stays inspectable and local: {}",
            one.base_url()
        );
        assert_ne!(one.port(), two.port(), "one port per run");
        one.shutdown();
        two.shutdown();
    }

    /// A chunked request body is refused by name rather than dropped or half-forwarded: the body
    /// must be buffered for the 401 retry, and a silent gap here would look like a vendor fault.
    #[test]
    fn a_chunked_request_body_is_refused_by_name() {
        let dir = tempfile::TempDir::new().expect("a directory");
        let (_, custody) = custody_over(dir.path(), "fake-upstream-token");
        let upstream = FakeUpstream::serving(Vec::new());
        let proxy = LoopbackProxy::start(&upstream.base(), custody, "chunked").expect("a proxy");

        let (head, body) = ask(
            proxy.port(),
            &format!(
                "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1\r\n\
                 Authorization: Bearer {}\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
                proxy.placeholder()
            ),
        );

        assert!(head.starts_with("HTTP/1.1 400"), "got: {head}");
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("metaharness") && text.contains("chunked"),
            "the refusal names metaharness and what it will not do: {text}"
        );
        proxy.shutdown();
    }

    #[test]
    fn an_upstream_is_parsed_into_a_host_a_port_and_the_authority_to_send() {
        let https = parse_upstream("https://api.anthropic.com").expect("a vendor base");
        assert!(https.tls);
        assert_eq!(https.port, 443, "https defaults to 443");
        assert_eq!(https.authority, "api.anthropic.com");

        let local = parse_upstream("http://127.0.0.1:8931").expect("a local base");
        assert!(!local.tls);
        assert_eq!(local.port, 8931);
        assert_eq!(local.authority, "127.0.0.1:8931");

        let six = parse_upstream("http://[::1]:9000").expect("an IPv6 base");
        assert_eq!(six.host, "::1", "the brackets are framing, not the host");
        assert_eq!(six.port, 9000);

        assert_eq!(
            parse_upstream("http://127.0.0.1/")
                .expect("a trailing slash is not a path")
                .authority,
            "127.0.0.1"
        );
    }

    /// A base with a path would be joined to the child's own path and produce a URL neither side
    /// intended — which surfaces as a vendor 404 and gets debugged in the wrong place.
    #[test]
    fn an_upstream_with_a_path_or_no_scheme_is_refused_naming_the_reason() {
        let path = parse_upstream("https://gateway.example/v1").expect_err("a path is refused");
        assert_eq!(path.kind(), io::ErrorKind::InvalidInput);
        assert!(path.to_string().contains("path"), "{path}");

        let bare = parse_upstream("api.anthropic.com").expect_err("a scheme is required");
        assert!(bare.to_string().contains("scheme"), "{bare}");

        let odd = parse_upstream("ftp://example.com").expect_err("only http and https");
        assert!(odd.to_string().contains("ftp"), "{odd}");
    }

    #[test]
    fn the_secret_comparison_is_length_safe_and_exact() {
        assert!(same_secret("mh-run-a-1", "mh-run-a-1"));
        assert!(!same_secret("mh-run-a-1", "mh-run-a-2"));
        assert!(
            !same_secret("mh-run-a-1", "mh-run-a-1x"),
            "a prefix of the placeholder is not the placeholder"
        );
        assert!(!same_secret("", "mh-run-a-1"));
    }

    #[test]
    fn a_credential_is_read_out_of_either_header_spelling() {
        assert_eq!(presented_token("Bearer abc"), "abc");
        assert_eq!(
            presented_token("bearer abc"),
            "abc",
            "the scheme is case-insensitive on the wire"
        );
        assert_eq!(
            presented_token("abc"),
            "abc",
            "x-api-key carries the credential bare"
        );
    }
}

//! PKCE, state and cancellable callback validation without gameplay coupling.

use oteryn_foundation::{BoundedText, Cancellable, CancellationToken, cancellable};
use sha2::{Digest, Sha256};
use std::fmt::{self, Debug, Display, Formatter};
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

const MIN_ENTROPY_BYTES: usize = 32;
const MAX_ENTROPY_BYTES: usize = 96;
const MAX_CALLBACK_QUERY_BYTES: usize = 4096;
const MAX_CALLBACK_REQUEST_LINE_BYTES: usize = 8192;
const CALLBACK_READ_TIMEOUT: Duration = Duration::from_secs(5);
const CALLBACK_PATH: &str = "/callback";

#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl Debug for SecretString {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretString([REDACTED])")
    }
}

impl Display for SecretString {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityError {
    EntropyLengthOutOfRange,
    InvalidState,
    InvalidCallback,
    StateMismatch,
    Cancelled,
    Timeout,
    CallbackClosed,
    /// The loopback redirect listener could not be bound.
    Listener,
}

impl Display for IdentityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EntropyLengthOutOfRange => {
                "Identity entropy length is outside the accepted range"
            }
            Self::InvalidState => "Identity state is invalid",
            Self::InvalidCallback => "Identity callback is invalid",
            Self::StateMismatch => "Identity callback state does not match",
            Self::Cancelled => "Identity flow was cancelled",
            Self::Timeout => "Identity callback timed out",
            Self::CallbackClosed => "Identity callback channel closed",
            Self::Listener => "Identity loopback listener could not be bound",
        })
    }
}

impl std::error::Error for IdentityError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkceMaterial {
    verifier: SecretString,
    challenge: String,
}

impl PkceMaterial {
    pub fn from_entropy(entropy: &[u8]) -> Result<Self, IdentityError> {
        if !(MIN_ENTROPY_BYTES..=MAX_ENTROPY_BYTES).contains(&entropy.len()) {
            return Err(IdentityError::EntropyLengthOutOfRange);
        }
        let verifier = base64_url_no_pad(entropy);
        let challenge = base64_url_no_pad(&Sha256::digest(verifier.as_bytes()));
        Ok(Self {
            verifier: SecretString::new(verifier),
            challenge,
        })
    }

    #[must_use]
    pub const fn verifier(&self) -> &SecretString {
        &self.verifier
    }

    #[must_use]
    pub fn challenge(&self) -> &str {
        &self.challenge
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateNonce(BoundedText);

impl StateNonce {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentityError> {
        BoundedText::new(value, 128)
            .map(Self)
            .map_err(|_error| IdentityError::InvalidState)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationCode(SecretString);

impl AuthorizationCode {
    #[must_use]
    pub const fn expose(&self) -> &SecretString {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackResult {
    pub code: AuthorizationCode,
    pub state: StateNonce,
}

impl CallbackResult {
    pub fn parse_query(query: &str) -> Result<Self, IdentityError> {
        if query.len() > MAX_CALLBACK_QUERY_BYTES {
            return Err(IdentityError::InvalidCallback);
        }
        let mut code = None;
        let mut state = None;
        for pair in query.split('&') {
            let (key, value) = pair.split_once('=').ok_or(IdentityError::InvalidCallback)?;
            let decoded = percent_decode(value)?;
            match key {
                "code" if code.is_none() => code = Some(decoded),
                "state" if state.is_none() => state = Some(decoded),
                "code" | "state" => return Err(IdentityError::InvalidCallback),
                _ => {}
            }
        }
        let code = code.ok_or(IdentityError::InvalidCallback)?;
        let state = StateNonce::new(state.ok_or(IdentityError::InvalidCallback)?)?;
        if code.is_empty() || code.len() > 2048 || code.chars().any(char::is_control) {
            return Err(IdentityError::InvalidCallback);
        }
        Ok(Self {
            code: AuthorizationCode(SecretString::new(code)),
            state,
        })
    }
}

#[derive(Debug, Clone)]
pub struct IdentityFlow {
    expected_state: StateNonce,
    timeout: Duration,
}

impl IdentityFlow {
    #[must_use]
    pub const fn new(expected_state: StateNonce, timeout: Duration) -> Self {
        Self {
            expected_state,
            timeout,
        }
    }

    pub async fn await_callback(
        &self,
        receiver: &mut mpsc::Receiver<String>,
        cancellation: CancellationToken,
    ) -> Result<AuthorizationCode, IdentityError> {
        let receive = async { receiver.recv().await.ok_or(IdentityError::CallbackClosed) };
        let bounded = tokio::time::timeout(self.timeout, cancellable(cancellation, receive))
            .await
            .map_err(|_elapsed| IdentityError::Timeout)?;
        let query = match bounded {
            Cancellable::Completed(result) => result?,
            Cancellable::Cancelled => return Err(IdentityError::Cancelled),
        };
        let callback = CallbackResult::parse_query(&query)?;
        if callback.state != self.expected_state {
            return Err(IdentityError::StateMismatch);
        }
        Ok(callback.code)
    }
}

/// The native client's OAuth redirect target: a listener on `127.0.0.1` with an OS-chosen port
/// (RFC 8252 §7.3). It forwards only the first `/callback` request's query to an [`IdentityFlow`],
/// which checks the exact `state`, and stops listening once the flow ends.
#[derive(Debug)]
pub struct LoopbackRedirect {
    listener: TcpListener,
    address: SocketAddr,
}

impl LoopbackRedirect {
    pub async fn bind() -> Result<Self, IdentityError> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(|_error| IdentityError::Listener)?;
        let address = listener
            .local_addr()
            .map_err(|_error| IdentityError::Listener)?;
        Ok(Self { listener, address })
    }

    #[must_use]
    pub fn redirect_uri(&self) -> String {
        format!("http://{}{CALLBACK_PATH}", self.address)
    }

    /// Serves the redirect until `flow` accepts or rejects the first callback, times out or is
    /// cancelled. The listener is closed on return.
    pub async fn await_code(
        self,
        flow: &IdentityFlow,
        cancellation: CancellationToken,
    ) -> Result<AuthorizationCode, IdentityError> {
        let (sender, mut receiver) = mpsc::channel(1);
        let listener = self.listener;
        let server = tokio::spawn(async move {
            loop {
                let Ok((stream, _peer)) = listener.accept().await else {
                    continue;
                };
                if let Some(query) = serve_redirect(stream).await {
                    let _ = sender.send(query).await;
                    return;
                }
            }
        });
        let result = flow.await_callback(&mut receiver, cancellation).await;
        server.abort();
        result
    }
}

/// Answers one loopback request. Returns the query of a `GET /callback?...` request.
async fn serve_redirect(mut stream: TcpStream) -> Option<String> {
    let request_line = tokio::time::timeout(CALLBACK_READ_TIMEOUT, read_request_line(&mut stream))
        .await
        .ok()
        .flatten();
    let query = request_line.as_deref().and_then(|line| {
        let target = line.strip_prefix("GET ")?.strip_suffix(" HTTP/1.1")?;
        target
            .strip_prefix(CALLBACK_PATH)?
            .strip_prefix('?')
            .map(str::to_owned)
    });
    let (status, body) = if query.is_some() {
        ("200 OK", "Sign-in received. You can return to Oteryn.")
    } else {
        ("404 Not Found", "Not found.")
    };
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: {}\r\ncache-control: no-store\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = tokio::time::timeout(CALLBACK_READ_TIMEOUT, async {
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;
    })
    .await;
    query
}

async fn read_request_line(stream: &mut TcpStream) -> Option<String> {
    let mut line = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        line.extend_from_slice(&chunk[..read]);
        if let Some(end) = line.windows(2).position(|pair| pair == b"\r\n") {
            line.truncate(end);
            return String::from_utf8(line).ok();
        }
        if line.len() > MAX_CALLBACK_REQUEST_LINE_BYTES {
            return None;
        }
    }
}

fn base64_url_no_pad(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        output.push(char::from(TABLE[usize::from(first >> 2)]));
        output.push(char::from(
            TABLE[usize::from(((first & 0x03) << 4) | (second >> 4))],
        ));
        if chunk.len() > 1 {
            output.push(char::from(
                TABLE[usize::from(((second & 0x0f) << 2) | (third >> 6))],
            ));
        }
        if chunk.len() > 2 {
            output.push(char::from(TABLE[usize::from(third & 0x3f)]));
        }
    }
    output
}

fn percent_decode(value: &str) -> Result<String, IdentityError> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let high = bytes
                    .get(index + 1)
                    .copied()
                    .ok_or(IdentityError::InvalidCallback)?;
                let low = bytes
                    .get(index + 2)
                    .copied()
                    .ok_or(IdentityError::InvalidCallback)?;
                decoded.push((hex(high)? << 4) | hex(low)?);
                index += 3;
            }
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).map_err(|_error| IdentityError::InvalidCallback)
}

fn hex(value: u8) -> Result<u8, IdentityError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(IdentityError::InvalidCallback),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_is_deterministic_and_redacted() -> Result<(), IdentityError> {
        let material = PkceMaterial::from_entropy(&[7_u8; 32])?;
        assert!(material.challenge().len() >= 43);
        assert_eq!(
            format!("{:?}", material.verifier()),
            "SecretString([REDACTED])"
        );
        Ok(())
    }

    #[test]
    fn pkce_rejects_entropy_below_minimum() {
        assert_eq!(
            PkceMaterial::from_entropy(&[7_u8; 31]),
            Err(IdentityError::EntropyLengthOutOfRange)
        );
    }

    #[test]
    fn pkce_accepts_minimum_entropy_with_exact_verifier_length() -> Result<(), IdentityError> {
        let material = PkceMaterial::from_entropy(&[7_u8; 32])?;
        assert_eq!(material.verifier().expose().len(), 43);
        Ok(())
    }

    #[test]
    fn pkce_accepts_maximum_entropy_with_exact_verifier_length() -> Result<(), IdentityError> {
        let material = PkceMaterial::from_entropy(&[7_u8; 96])?;
        assert_eq!(material.verifier().expose().len(), 128);
        Ok(())
    }

    #[test]
    fn pkce_rejects_entropy_above_maximum() {
        assert_eq!(
            PkceMaterial::from_entropy(&[7_u8; 97]),
            Err(IdentityError::EntropyLengthOutOfRange)
        );
    }

    async fn get(redirect_uri: &str, path: &str) -> Result<String, std::io::Error> {
        let address = redirect_uri
            .trim_start_matches("http://")
            .trim_end_matches(CALLBACK_PATH);
        let mut stream = TcpStream::connect(address).await?;
        stream
            .write_all(format!("GET {path} HTTP/1.1\r\nhost: {address}\r\n\r\n").as_bytes())
            .await?;
        let mut response = String::new();
        stream.read_to_string(&mut response).await?;
        Ok(response)
    }

    #[test]
    fn loopback_redirect_returns_the_code_for_the_exact_state()
    -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let redirect = LoopbackRedirect::bind().await?;
            let uri = redirect.redirect_uri();
            assert!(uri.starts_with("http://127.0.0.1:"));
            assert!(uri.ends_with("/callback"));
            let flow = IdentityFlow::new(StateNonce::new("state-1")?, Duration::from_secs(5));
            let waiting = tokio::spawn({
                let flow = flow.clone();
                async move { redirect.await_code(&flow, CancellationToken::new()).await }
            });
            let not_found = get(&uri, "/favicon.ico").await?;
            assert!(not_found.starts_with("HTTP/1.1 404"));
            let accepted = get(&uri, "/callback?code=abc&state=state-1").await?;
            assert!(accepted.starts_with("HTTP/1.1 200"));
            let code = waiting.await??;
            assert_eq!(code.expose().expose(), "abc");
            assert!(get(&uri, "/callback?code=abc&state=state-1").await.is_err());

            let redirect = LoopbackRedirect::bind().await?;
            let uri = redirect.redirect_uri();
            let waiting =
                tokio::spawn(
                    async move { redirect.await_code(&flow, CancellationToken::new()).await },
                );
            get(&uri, "/callback?code=abc&state=other").await?;
            assert_eq!(waiting.await?, Err(IdentityError::StateMismatch));
            Ok(())
        })
    }

    #[test]
    fn callback_requires_exact_state() -> Result<(), IdentityError> {
        let callback = CallbackResult::parse_query("code=a%2Db&state=state-1")?;
        assert_eq!(callback.state.as_str(), "state-1");
        assert_eq!(callback.code.expose().expose(), "a-b");
        Ok(())
    }
}

//! PKCE, state and cancellable callback validation without gameplay coupling.

use oteryn_foundation::{BoundedText, Cancellable, CancellationToken, cancellable};
use sha2::{Digest, Sha256};
use std::fmt::{self, Debug, Display, Formatter};
use std::time::Duration;
use tokio::sync::mpsc;

const MIN_ENTROPY_BYTES: usize = 32;
const MAX_CALLBACK_QUERY_BYTES: usize = 4096;

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
    InsufficientEntropy,
    InvalidState,
    InvalidCallback,
    StateMismatch,
    Cancelled,
    Timeout,
    CallbackClosed,
}

impl Display for IdentityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InsufficientEntropy => "Identity entropy is insufficient",
            Self::InvalidState => "Identity state is invalid",
            Self::InvalidCallback => "Identity callback is invalid",
            Self::StateMismatch => "Identity callback state does not match",
            Self::Cancelled => "Identity flow was cancelled",
            Self::Timeout => "Identity callback timed out",
            Self::CallbackClosed => "Identity callback channel closed",
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
        if entropy.len() < MIN_ENTROPY_BYTES {
            return Err(IdentityError::InsufficientEntropy);
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
    fn callback_requires_exact_state() -> Result<(), IdentityError> {
        let callback = CallbackResult::parse_query("code=a%2Db&state=state-1")?;
        assert_eq!(callback.state.as_str(), "state-1");
        assert_eq!(callback.code.expose().expose(), "a-b");
        Ok(())
    }
}

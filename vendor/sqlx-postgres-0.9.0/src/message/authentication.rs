use std::str::from_utf8;

use memchr::memchr;
use sqlx_core::bytes::Buf;
use sqlx_core::net::OwnedBytes as Bytes;

use crate::error::Error;
use crate::message::PgDecode;

use crate::message::{BackendMessage, BackendMessageFormat};
use base64::prelude::{Engine as _, BASE64_STANDARD};
// On startup, the server sends an appropriate authentication request message,
// to which the frontend must reply with an appropriate authentication
// response message (such as a password).

// For all authentication methods except GSSAPI, SSPI and SASL, there is at
// most one request and one response. In some methods, no response at all is
// needed from the frontend, and so no authentication request occurs.

// For GSSAPI, SSPI and SASL, multiple exchanges of packets may
// be needed to complete the authentication.

// <https://www.postgresql.org/docs/devel/protocol-flow.html#id-1.10.5.7.3>
// <https://www.postgresql.org/docs/devel/protocol-message-formats.html>

#[derive(Debug)]
pub enum Authentication {
    /// The authentication exchange is successfully completed.
    Ok,

    /// The frontend must now send a [PasswordMessage] containing the
    /// password in clear-text form.
    CleartextPassword,

    /// The frontend must now send a [PasswordMessage] containing the
    /// password (with user name) encrypted via MD5, then encrypted
    /// again using the 4-byte random salt.
    Md5Password(AuthenticationMd5Password),

    /// The frontend must now initiate a SASL negotiation,
    /// using one of the SASL mechanisms listed in the message.
    ///
    /// The frontend will send a [SaslInitialResponse] with the name
    /// of the selected mechanism, and the first part of the SASL
    /// data stream in response to this.
    ///
    /// If further messages are needed, the server will
    /// respond with [Authentication::SaslContinue].
    Sasl(AuthenticationSasl),

    /// This message contains challenge data from the previous step of SASL negotiation.
    ///
    /// The frontend must respond with a [SaslResponse] message.
    SaslContinue(AuthenticationSaslContinue),

    /// SASL authentication has completed with additional mechanism-specific
    /// data for the client.
    ///
    /// The server will next send [Authentication::Ok] to
    /// indicate successful authentication.
    SaslFinal(AuthenticationSaslFinal),
}

impl BackendMessage for Authentication {
    const FORMAT: BackendMessageFormat = BackendMessageFormat::Authentication;

    fn decode_body(mut buf: Bytes) -> Result<Self, Error> {
        if buf.len() < 4 {
            return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
        }
        Ok(match buf.get_u32() {
            0 => {
                if !buf.is_empty() {
                    return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
                }
                Authentication::Ok
            }

            3 => {
                if !buf.is_empty() {
                    return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
                }
                Authentication::CleartextPassword
            }

            5 => {
                if buf.len() != 4 {
                    return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
                }
                let mut salt = [0; 4];
                buf.copy_to_slice(&mut salt);

                Authentication::Md5Password(AuthenticationMd5Password { salt })
            }

            10 => {
                let mut tail = &buf[..];
                loop {
                    let nul = memchr(0, tail)
                        .ok_or_else(|| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
                    std::str::from_utf8(&tail[..nul])
                        .map_err(|_| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
                    tail = &tail[nul + 1..];
                    if nul == 0 {
                        if !tail.is_empty() {
                            return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
                        }
                        break;
                    }
                }
                Authentication::Sasl(AuthenticationSasl(buf))
            }
            11 => Authentication::SaslContinue(AuthenticationSaslContinue::decode(buf)?),
            12 => Authentication::SaslFinal(AuthenticationSaslFinal::decode(buf)?),

            _ => return Err(Error::Io(std::io::ErrorKind::InvalidData.into())),
        })
    }
}

/// Body of [Authentication::Md5Password].
#[derive(Debug)]
pub struct AuthenticationMd5Password {
    pub salt: [u8; 4],
}

/// Body of [Authentication::Sasl].
#[derive(Debug)]
pub struct AuthenticationSasl(Bytes);

impl AuthenticationSasl {
    #[inline]
    pub fn mechanisms(&self) -> SaslMechanisms<'_> {
        SaslMechanisms(&self.0)
    }
}

/// An iterator over the SASL authentication mechanisms provided by the server.
pub struct SaslMechanisms<'a>(&'a [u8]);

impl<'a> Iterator for SaslMechanisms<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.0.is_empty() && self.0[0] == b'\0' {
            return None;
        }

        let mechanism = memchr(b'\0', self.0).and_then(|nul| from_utf8(&self.0[..nul]).ok())?;

        self.0 = &self.0[(mechanism.len() + 1)..];

        Some(mechanism)
    }
}

#[derive(Debug)]
pub struct AuthenticationSaslContinue {
    pub salt: Vec<u8>,
    pub iterations: u32,
    pub nonce: String,
    pub message: String,
    _allocation: crate::statement::AllocationLease,
}

impl PgDecode for AuthenticationSaslContinue {
    fn decode(buf: Bytes) -> Result<Self, Error> {
        std::str::from_utf8(&buf).map_err(|_| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
        let mut seen = 0u8;
        let mut reserve = buf.len();
        for item in buf.split(|b| *b == b',') {
            if item.len() < 2 || item[1] != b'=' {
                return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
            }
            let bit = match item[0] {
                b'r' => 1,
                b's' => 2,
                b'i' => 4,
                b'm' => return Err(Error::Io(std::io::ErrorKind::InvalidData.into())),
                _ => 0,
            };
            if bit != 0 && seen & bit != 0 {
                return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
            }
            seen |= bit;
            let extra = match item[0] {
                b'r' => item.len() - 2,
                b's' => base64::decoded_len_estimate(item.len() - 2),
                _ => 0,
            };
            reserve = reserve
                .checked_add(extra)
                .ok_or_else(crate::statement::allocation_denied)?;
        }
        if seen != 7 {
            return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
        }
        let allocation = crate::statement::AllocationLease::reserve(buf.budget(), reserve)?;
        let mut iterations: u32 = 0;
        let mut salt = Vec::new();
        let mut nonce = Bytes::new();

        // [Example]
        // r=/z+giZiTxAH7r8sNAeHr7cvpqV3uo7G/bJBIJO3pjVM7t3ng,s=4UV68bIkC8f9/X8xH7aPhg==,i=4096

        for item in buf.split(|b| *b == b',') {
            let key = item[0];
            let value = &item[2..];

            match key {
                b'r' => {
                    nonce = buf.slice_ref(value);
                }

                b'i' => {
                    iterations = std::str::from_utf8(value)
                        .ok()
                        .and_then(|value| value.parse().ok())
                        .filter(|value| *value > 0)
                        .ok_or_else(|| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
                }

                b's' => {
                    salt = BASE64_STANDARD
                        .decode(value)
                        .map_err(|_| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
                }

                _ => {}
            }
        }

        Ok(Self {
            iterations,
            salt,
            nonce: from_utf8(&nonce).map_err(Error::protocol)?.to_owned(),
            message: from_utf8(&buf).map_err(Error::protocol)?.to_owned(),
            _allocation: allocation,
        })
    }
}

#[derive(Debug)]
pub struct AuthenticationSaslFinal {
    pub verifier: Vec<u8>,
    _allocation: crate::statement::AllocationLease,
}

impl PgDecode for AuthenticationSaslFinal {
    fn decode(buf: Bytes) -> Result<Self, Error> {
        let mut reserve = 0usize;
        let mut seen = false;
        for item in buf.split(|b| *b == b',') {
            if item.len() < 2 || item[1] != b'=' {
                return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
            }
            if item[0] == b'e' {
                return Err(Error::Io(std::io::ErrorKind::PermissionDenied.into()));
            }
            if item[0] == b'v' {
                if seen {
                    return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
                }
                seen = true;
                reserve = base64::decoded_len_estimate(item.len() - 2);
            }
        }
        if !seen {
            return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
        }
        let allocation = crate::statement::AllocationLease::reserve(buf.budget(), reserve)?;
        let mut verifier = Vec::new();

        for item in buf.split(|b| *b == b',') {
            let key = item[0];
            let value = &item[2..];

            if let b'v' = key {
                verifier = BASE64_STANDARD
                    .decode(value)
                    .map_err(|_| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
            }
        }

        Ok(Self {
            verifier,
            _allocation: allocation,
        })
    }
}

#[cfg(test)]
mod custody_tests {
    use super::*;
    use crate::statement::custody_test_support::Ledger;
    #[test]
    fn scram_challenge_and_verifier_are_charged_and_reject_malformed_attributes() {
        let budget = Ledger::new(usize::MAX);
        let input =
            Bytes::try_copy_from_slice(b"r=client-server,s=c2FsdA==,i=4096", budget.clone())
                .unwrap();
        let challenge = AuthenticationSaslContinue::decode(input).unwrap();
        assert_eq!(challenge.salt, b"salt");
        assert_eq!(challenge.iterations, 4096);
        assert!(budget.held() > 0);
        drop(challenge);
        assert_eq!(budget.held(), 0);
        for bytes in [
            &b"r=x,s=eA=="[..],
            &b"r=x,s=eA==,i=0"[..],
            &b"r=x,r=y,s=eA==,i=1"[..],
            &b"r=x,s=eA==,i=1junk"[..],
        ] {
            assert!(AuthenticationSaslContinue::decode(Bytes::copy_from_slice(bytes)).is_err());
        }
        let input = Bytes::try_copy_from_slice(b"v=eA==", budget.clone()).unwrap();
        let baseline = budget.held();
        budget.limit(baseline);
        assert!(AuthenticationSaslFinal::decode(input).is_err());
        assert_eq!(budget.held(), 0);
        assert!(AuthenticationSaslFinal::decode(Bytes::from_static(b"e=server-error")).is_err());
    }
}

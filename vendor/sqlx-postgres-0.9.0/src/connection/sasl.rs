use crate::connection::stream::PgStream;
use crate::error::Error;
use crate::message::{Authentication, AuthenticationSasl, SaslInitialResponse, SaslResponse};
use crate::rt;
use crate::PgConnectOptions;
use hmac::{Hmac, Mac};
use hmac::{HmacReset, KeyInit};
use sha2::{Digest, Sha256};
use stringprep::saslprep;

use base64::prelude::{Engine as _, BASE64_STANDARD};
use rand::RngExt;

const GS2_HEADER: &str = "n,,";
const CHANNEL_ATTR: &str = "c";
const USERNAME_ATTR: &str = "n";
const CLIENT_PROOF_ATTR: &str = "p";
const NONCE_ATTR: &str = "r";

pub(crate) async fn authenticate(
    stream: &mut PgStream,
    options: &PgConnectOptions,
    data: AuthenticationSasl,
) -> Result<(), Error> {
    if stream.resource_budget().is_some() {
        return authenticate_owned(stream, options, data).await;
    }
    let mut has_sasl = false;
    let mut has_sasl_plus = false;
    let mut unknown = Vec::new();

    for mechanism in data.mechanisms() {
        match mechanism {
            "SCRAM-SHA-256" => {
                has_sasl = true;
            }

            "SCRAM-SHA-256-PLUS" => {
                has_sasl_plus = true;
            }

            _ => {
                unknown.push(mechanism.to_owned());
            }
        }
    }

    if !has_sasl_plus && !has_sasl {
        return Err(err_protocol!(
            "unsupported SASL authentication mechanisms: {}",
            unknown.join(", ")
        ));
    }

    // channel-binding = "c=" base64
    let mut channel_binding = format!("{CHANNEL_ATTR}=");
    BASE64_STANDARD.encode_string(GS2_HEADER, &mut channel_binding);

    // "n=" saslname ;; Usernames are prepared using SASLprep.
    let username = format!("{}={}", USERNAME_ATTR, options.username);
    let username = match saslprep(&username) {
        Ok(v) => v,
        Err(error) => {
            return Err(Error::Configuration(
                format!("Failed to saslprep username: {:?}", error).into(),
            ))
        }
    };

    // nonce = "r=" c-nonce [s-nonce] ;; Second part provided by server.
    let nonce = gen_nonce();

    // client-first-message-bare = [reserved-mext ","] username "," nonce ["," extensions]
    let client_first_message_bare = format!("{username},{nonce}");

    let client_first_message = format!("{GS2_HEADER}{client_first_message_bare}");

    stream
        .send(SaslInitialResponse {
            response: &client_first_message,
            plus: false,
        })
        .await?;

    let cont = match stream.recv_expect().await? {
        Authentication::SaslContinue(data) => data,

        auth => {
            return Err(err_protocol!(
                "expected SASLContinue but received {:?}",
                auth
            ));
        }
    };

    // Normalize(password):
    let password = options.password.as_deref().unwrap_or_default();
    let password = match saslprep(password) {
        Ok(v) => v,
        Err(error) => {
            return Err(Error::Configuration(
                format!("Failed to saslprep password: {:?}", error).into(),
            ))
        }
    };

    // SaltedPassword := Hi(Normalize(password), salt, i)
    let salted_password = hi(&password, &cont.salt, cont.iterations).await?;

    // ClientKey := HMAC(SaltedPassword, "Client Key")
    let mut mac = Hmac::<Sha256>::new_from_slice(&salted_password).map_err(Error::protocol)?;
    mac.update(b"Client Key");

    let client_key = mac.finalize().into_bytes();

    // StoredKey := H(ClientKey)
    let stored_key = Sha256::digest(client_key);

    // client-final-message-without-proof
    let client_final_message_wo_proof = format!(
        "{channel_binding},r={nonce}",
        channel_binding = channel_binding,
        nonce = &cont.nonce
    );

    // AuthMessage := client-first-message-bare + "," + server-first-message + "," + client-final-message-without-proof
    let auth_message = format!(
        "{client_first_message_bare},{server_first_message},{client_final_message_wo_proof}",
        client_first_message_bare = client_first_message_bare,
        server_first_message = cont.message,
        client_final_message_wo_proof = client_final_message_wo_proof
    );

    // ClientSignature := HMAC(StoredKey, AuthMessage)
    let mut mac = Hmac::<Sha256>::new_from_slice(&stored_key).map_err(Error::protocol)?;
    mac.update(auth_message.as_bytes());

    let client_signature = mac.finalize().into_bytes();

    // ClientProof := ClientKey XOR ClientSignature
    let client_proof: Vec<u8> = client_key
        .iter()
        .zip(client_signature.iter())
        .map(|(&a, &b)| a ^ b)
        .collect();

    // ServerKey := HMAC(SaltedPassword, "Server Key")
    let mut mac = Hmac::<Sha256>::new_from_slice(&salted_password).map_err(Error::protocol)?;
    mac.update(b"Server Key");

    let server_key = mac.finalize().into_bytes();

    // ServerSignature := HMAC(ServerKey, AuthMessage)
    let mut mac = Hmac::<Sha256>::new_from_slice(&server_key).map_err(Error::protocol)?;
    mac.update(auth_message.as_bytes());

    // client-final-message = client-final-message-without-proof "," proof
    let mut client_final_message = format!("{client_final_message_wo_proof},{CLIENT_PROOF_ATTR}=");
    BASE64_STANDARD.encode_string(client_proof, &mut client_final_message);

    stream.send(SaslResponse(&client_final_message)).await?;

    let data = match stream.recv_expect().await? {
        Authentication::SaslFinal(data) => data,

        auth => {
            return Err(err_protocol!("expected SASLFinal but received {:?}", auth));
        }
    };

    // authentication is only considered valid if this verification passes
    mac.verify_slice(&data.verifier).map_err(Error::protocol)?;

    Ok(())
}

// nonce is a sequence of random printable bytes
fn gen_nonce() -> String {
    let mut rng = rand::rng();
    let count = rng.random_range(64..128);

    // printable = %x21-2B / %x2D-7E
    // ;; Printable ASCII except ",".
    // ;; Note that any "printable" is also
    // ;; a valid "value".
    let nonce: String = std::iter::repeat(())
        .map(|()| {
            let mut c = rng.random_range(0x21u8..0x7F);

            while c == 0x2C {
                c = rng.random_range(0x21u8..0x7F);
            }

            c
        })
        .take(count)
        .map(|c| c as char)
        .collect();

    format!("{NONCE_ATTR}={nonce}")
}

// Hi(str, salt, i):
async fn hi<'a>(s: &'a str, salt: &'a [u8], iter_count: u32) -> Result<[u8; 32], Error> {
    let mut mac = HmacReset::<Sha256>::new_from_slice(s.as_bytes()).map_err(Error::protocol)?;

    mac.update(salt);
    mac.update(&1u32.to_be_bytes());

    let mut u = mac.finalize_reset().into_bytes();
    let mut hi = u;

    for i in 1..iter_count {
        mac.update(u.as_slice());
        u = mac.finalize_reset().into_bytes();
        hi = hi.iter().zip(u.iter()).map(|(&a, &b)| a ^ b).collect();

        // For large iteration counts, this process can take a long time and block the event loop.
        // It was measured as taking ~50ms for 4096 iterations (the default) on a developer machine.
        // If we want to yield every 10-100us (as generally advised for tokio), then we can yield
        // every 5 iterations which should be every ~50us.
        if i % 5 == 0 {
            rt::yield_now().await;
        }
    }

    Ok(hi.into())
}

/// All dynamic SCRAM strings have exact destination capacity and external custody.
struct AuthText {
    text: String,
    _allocation: sqlx_core::net::resource_budget::ResourceReservation,
}
impl std::ops::Deref for AuthText {
    type Target = str;
    fn deref(&self) -> &str {
        &self.text
    }
}
impl AuthText {
    fn concat(
        parts: &[&str],
        budget: &std::sync::Arc<dyn sqlx_core::net::resource_budget::ResourceBudget>,
    ) -> Result<Self, Error> {
        let size = parts
            .iter()
            .try_fold(0usize, |size, part| size.checked_add(part.len()))
            .ok_or_else(crate::statement::allocation_denied)?;
        let allocation =
            sqlx_core::net::resource_budget::ResourceReservation::try_new(budget.clone(), size)
                .map_err(|_| crate::statement::allocation_denied())?;
        let mut text = String::with_capacity(size);
        for part in parts {
            text.push_str(part);
        }
        Ok(Self {
            text,
            _allocation: allocation,
        })
    }
}

struct PreparedAuthText<'a> {
    text: std::borrow::Cow<'a, str>,
    _allocation: sqlx_core::net::resource_budget::ResourceReservation,
}
impl std::ops::Deref for PreparedAuthText<'_> {
    type Target = str;
    fn deref(&self) -> &str {
        &self.text
    }
}
fn prepare_auth_text<'a>(
    text: &'a str,
    budget: &std::sync::Arc<dyn sqlx_core::net::resource_budget::ResourceBudget>,
) -> Result<PreparedAuthText<'a>, Error> {
    // stringprep 0.1.5 borrows printable ASCII. The pinned Unicode 17 NFKC
    // fully decomposed table expands one scalar into at most 18 scalars.
    // For N expanded scalars, each growing Vec has old+new <= 3*max(N,8)
    // elements: decomposition (u8,char), recomposition char, UTF-8 output
    // (at most four bytes/scalar). Stable sort scratch needs <= N pairs.
    // This finite peak is acquired before normalization, including its result.
    let bytes = if text.bytes().all(|b| b.is_ascii() && !b.is_ascii_control()) {
        0
    } else {
        let count = text
            .chars()
            .count()
            .checked_mul(18)
            .ok_or_else(crate::statement::allocation_denied)?
            .max(8);
        let pair = std::mem::size_of::<(u8, char)>();
        count
            .checked_mul(3 * (pair + std::mem::size_of::<char>() + 4) + pair)
            .ok_or_else(crate::statement::allocation_denied)?
    };
    let allocation =
        sqlx_core::net::resource_budget::ResourceReservation::try_new(budget.clone(), bytes)
            .map_err(|_| crate::statement::allocation_denied())?;
    let text = saslprep(text).map_err(|_| Error::Io(std::io::ErrorKind::InvalidInput.into()))?;
    Ok(PreparedAuthText {
        text,
        _allocation: allocation,
    })
}

async fn authenticate_owned(
    stream: &mut PgStream,
    options: &PgConnectOptions,
    data: AuthenticationSasl,
) -> Result<(), Error> {
    use rand::SeedableRng;
    let budget = stream
        .resource_budget()
        .expect("owned authentication")
        .clone();
    if !data
        .mechanisms()
        .any(|mechanism| mechanism == "SCRAM-SHA-256")
    {
        return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
    }
    let username = AuthText::concat(&["n=", &options.username], &budget)?;
    let username = prepare_auth_text(&username, &budget)?;
    // Per-attempt CSPRNG state is inline. Avoid rand::rng()'s process/thread
    // Rc allocation and lifetime while preserving the nonce alphabet/length.
    let mut rng = rand::rngs::StdRng::try_from_rng(&mut rand::rngs::SysRng)
        .map_err(|_| Error::Io(std::io::ErrorKind::Other.into()))?;
    let count = rng.random_range(64..128);
    let mut nonce = [0u8; 129];
    nonce[..2].copy_from_slice(b"r=");
    for byte in &mut nonce[2..count + 2] {
        loop {
            let value = rng.random_range(0x21u8..0x7f);
            if value != b',' {
                *byte = value;
                break;
            }
        }
    }
    let nonce = std::str::from_utf8(&nonce[..count + 2]).expect("ASCII nonce");
    let first_bare = AuthText::concat(&[&username, ",", nonce], &budget)?;
    let first = AuthText::concat(&[GS2_HEADER, &first_bare], &budget)?;
    stream
        .send(SaslInitialResponse {
            response: &first,
            plus: false,
        })
        .await?;
    let cont = match stream.recv_expect().await? {
        Authentication::SaslContinue(data) => data,
        _ => return Err(Error::Io(std::io::ErrorKind::InvalidData.into())),
    };
    if !cont.nonce.starts_with(&nonce[2..]) || cont.nonce.len() <= nonce.len() - 2 {
        return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
    }
    let password = prepare_auth_text(options.password.as_deref().unwrap_or_default(), &budget)?;
    let salted_password = hi(&password, &cont.salt, cont.iterations).await?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&salted_password)
        .map_err(|_| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
    mac.update(b"Client Key");
    let client_key = mac.finalize().into_bytes();
    let stored_key = Sha256::digest(client_key);
    let final_without_proof = AuthText::concat(&["c=biws,r=", &cont.nonce], &budget)?;
    let auth_message = AuthText::concat(
        &[&first_bare, ",", &cont.message, ",", &final_without_proof],
        &budget,
    )?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&stored_key)
        .map_err(|_| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
    mac.update(auth_message.as_bytes());
    let signature = mac.finalize().into_bytes();
    let mut proof = [0u8; 32];
    for ((out, key), signature) in proof.iter_mut().zip(client_key).zip(signature) {
        *out = key ^ signature;
    }
    let mut encoded = [0u8; 44];
    BASE64_STANDARD
        .encode_slice(proof, &mut encoded)
        .expect("32-byte proof encodes into 44 bytes");
    let encoded = std::str::from_utf8(&encoded).expect("base64 ASCII");
    let final_message = AuthText::concat(&[&final_without_proof, ",p=", encoded], &budget)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&salted_password)
        .map_err(|_| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
    mac.update(b"Server Key");
    let server_key = mac.finalize().into_bytes();
    let mut mac = Hmac::<Sha256>::new_from_slice(&server_key)
        .map_err(|_| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
    mac.update(auth_message.as_bytes());
    stream.send(SaslResponse(&final_message)).await?;
    let data = match stream.recv_expect().await? {
        Authentication::SaslFinal(data) => data,
        _ => return Err(Error::Io(std::io::ErrorKind::InvalidData.into())),
    };
    mac.verify_slice(&data.verifier)
        .map_err(|_| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
    Ok(())
}

#[cfg(test)]
mod custody_tests {
    use super::*;
    use crate::statement::custody_test_support::Ledger;
    #[test]
    fn auth_text_is_precharged_and_normalization_preserves_unicode() {
        let ledger = Ledger::new(5);
        let budget: std::sync::Arc<dyn sqlx_core::net::resource_budget::ResourceBudget> =
            ledger.clone();
        assert!(AuthText::concat(&["abc", "def"], &budget).is_err());
        assert_eq!(ledger.held(), 0);
        ledger.limit(6);
        let text = AuthText::concat(&["abc", "def"], &budget).unwrap();
        assert_eq!(&*text, "abcdef");
        assert_eq!(ledger.held(), 6);
        drop(text);
        assert_eq!(ledger.held(), 0);
        assert!(prepare_auth_text("\u{fdFA}", &budget).is_err());
        ledger.limit(usize::MAX);
        let normalized = prepare_auth_text("\u{fdFA}", &budget).unwrap();
        assert_eq!(&*normalized, &*saslprep("\u{fdFA}").unwrap());
        assert!(ledger.held() > normalized.len());
        drop(normalized);
        assert_eq!(ledger.held(), 0);
        let ascii = prepare_auth_text("ordinary-password", &budget).unwrap();
        assert_eq!(ledger.held(), 0);
        assert_eq!(&*ascii, "ordinary-password");
    }
}

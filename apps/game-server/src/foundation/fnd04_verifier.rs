use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericDateError {
    Malformed,
    NotYetValid,
    Expired,
}

pub const MAX_COMPACT_JWS_BYTES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fnd04VerificationError {
    Malformed,
    AuthenticationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactJws {
    protected_header_segment: String,
    payload_segment: String,
    signature_segment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectedHeader {
    kid: String,
    typ: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedTrustContext {
    keys: BTreeMap<String, [u8; 32]>,
}

impl FixedTrustContext {
    pub fn new<I, K>(keys: I) -> Self
    where
        I: IntoIterator<Item = (K, [u8; 32])>,
        K: Into<String>,
    {
        Self {
            keys: keys
                .into_iter()
                .map(|(key_id, public_key)| (key_id.into(), public_key))
                .collect(),
        }
    }
}

pub fn parse_protected_header(
    compact_jws: &CompactJws,
) -> Result<ProtectedHeader, Fnd04VerificationError> {
    let protected_header = decode_canonical_base64url(&compact_jws.protected_header_segment, 512)?;
    let value: serde_json::Value =
        serde_json::from_slice(&protected_header).map_err(|_| Fnd04VerificationError::Malformed)?;
    let object = value.as_object().ok_or(Fnd04VerificationError::Malformed)?;
    if object.len() != 3
        || !object.contains_key("alg")
        || !object.contains_key("kid")
        || !object.contains_key("typ")
    {
        return Err(Fnd04VerificationError::Malformed);
    }

    let alg = object
        .get("alg")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.is_ascii())
        .ok_or(Fnd04VerificationError::Malformed)?;
    let kid = object
        .get("kid")
        .and_then(serde_json::Value::as_str)
        .filter(|value| {
            (1..=64).contains(&value.len())
                && value.is_ascii()
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        })
        .ok_or(Fnd04VerificationError::Malformed)?;
    let typ = object
        .get("typ")
        .and_then(serde_json::Value::as_str)
        .filter(|value| {
            (1..=64).contains(&value.len())
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_graphic() && !byte.is_ascii_whitespace())
        })
        .ok_or(Fnd04VerificationError::Malformed)?;

    if alg != "Ed25519" {
        return Err(Fnd04VerificationError::AuthenticationFailed);
    }

    Ok(ProtectedHeader {
        kid: kid.to_owned(),
        typ: typ.to_owned(),
    })
}

pub fn verify_compact_signature(
    compact_jws: &CompactJws,
    protected_header: &ProtectedHeader,
    trust_context: &FixedTrustContext,
) -> Result<(), Fnd04VerificationError> {
    let public_key = trust_context
        .keys
        .get(&protected_header.kid)
        .ok_or(Fnd04VerificationError::AuthenticationFailed)?;
    let verifying_key = VerifyingKey::from_bytes(public_key)
        .map_err(|_| Fnd04VerificationError::AuthenticationFailed)?;
    let signature = decode_canonical_base64url(&compact_jws.signature_segment, 64)?;
    let signature: [u8; 64] = signature
        .try_into()
        .map_err(|_| Fnd04VerificationError::AuthenticationFailed)?;
    let signing_input = format!(
        "{}.{}",
        compact_jws.protected_header_segment, compact_jws.payload_segment
    );
    verifying_key
        .verify_strict(signing_input.as_bytes(), &Signature::from_bytes(&signature))
        .map_err(|_| Fnd04VerificationError::AuthenticationFailed)
}

pub fn parse_compact_jws(token: &str) -> Result<CompactJws, Fnd04VerificationError> {
    if token.len() > MAX_COMPACT_JWS_BYTES || !token.is_ascii() {
        return Err(Fnd04VerificationError::Malformed);
    }

    let mut segments = token.split('.');
    let Some(protected_header_segment) = segments.next() else {
        return Err(Fnd04VerificationError::Malformed);
    };
    let Some(payload_segment) = segments.next() else {
        return Err(Fnd04VerificationError::Malformed);
    };
    let Some(signature_segment) = segments.next() else {
        return Err(Fnd04VerificationError::Malformed);
    };
    if segments.next().is_some()
        || protected_header_segment.is_empty()
        || payload_segment.is_empty()
        || signature_segment.is_empty()
    {
        return Err(Fnd04VerificationError::Malformed);
    }

    let protected_header = decode_canonical_base64url(protected_header_segment, 512)?;
    let payload = decode_canonical_base64url(payload_segment, 3_072)?;
    decode_canonical_base64url(signature_segment, MAX_COMPACT_JWS_BYTES)?;
    validate_bounded_json_object(&protected_header)?;
    validate_bounded_json_object(&payload)?;

    Ok(CompactJws {
        protected_header_segment: protected_header_segment.to_owned(),
        payload_segment: payload_segment.to_owned(),
        signature_segment: signature_segment.to_owned(),
    })
}

fn decode_canonical_base64url(
    segment: &str,
    maximum_decoded_bytes: usize,
) -> Result<Vec<u8>, Fnd04VerificationError> {
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|_| Fnd04VerificationError::Malformed)?;
    if decoded.len() > maximum_decoded_bytes
        || base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&decoded) != segment
    {
        return Err(Fnd04VerificationError::Malformed);
    }
    Ok(decoded)
}

fn validate_bounded_json_object(input: &[u8]) -> Result<(), Fnd04VerificationError> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    BoundedJsonObject { depth: 1 }
        .deserialize(&mut deserializer)
        .map_err(|_| Fnd04VerificationError::Malformed)?;
    deserializer
        .end()
        .map_err(|_| Fnd04VerificationError::Malformed)
}

struct BoundedJsonObject {
    depth: u8,
}

impl<'de> DeserializeSeed<'de> for BoundedJsonObject {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(BoundedJsonVisitor { depth: self.depth })
    }
}

struct BoundedJsonValue {
    depth: u8,
}

impl<'de> DeserializeSeed<'de> for BoundedJsonValue {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(BoundedJsonVisitor { depth: self.depth })
    }
}

struct BoundedJsonVisitor {
    depth: u8,
}

impl<'de> Visitor<'de> for BoundedJsonVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded JSON")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        if self.depth > 2 {
            return Err(de::Error::custom("JSON nesting limit exceeded"));
        }

        let mut members = BTreeSet::new();
        while let Some(member) = map.next_key::<String>()? {
            if !members.insert(member) {
                return Err(de::Error::custom("duplicate JSON member"));
            }
            map.next_value_seed(BoundedJsonValue {
                depth: self.depth.saturating_add(1),
            })?;
        }
        Ok(())
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        if self.depth > 2 {
            return Err(de::Error::custom("JSON nesting limit exceeded"));
        }

        while sequence
            .next_element_seed(BoundedJsonValue {
                depth: self.depth.saturating_add(1),
            })?
            .is_some()
        {}
        Ok(())
    }
}

pub struct NumericDate;

impl NumericDate {
    pub fn validate(now: i64, iat: i64, nbf: i64, exp: i64) -> Result<(), NumericDateError> {
        let nbf_lower_bound = iat.checked_sub(1).ok_or(NumericDateError::Malformed)?;
        let nbf_upper_bound = iat.checked_add(1).ok_or(NumericDateError::Malformed)?;
        if !(nbf_lower_bound..=nbf_upper_bound).contains(&nbf) {
            return Err(NumericDateError::Malformed);
        }

        let lifetime = exp.checked_sub(iat).ok_or(NumericDateError::Malformed)?;
        if exp <= iat || lifetime > 30 {
            return Err(NumericDateError::Malformed);
        }

        let latest_accepted_not_before = now.checked_add(5).ok_or(NumericDateError::Malformed)?;
        if latest_accepted_not_before < nbf {
            return Err(NumericDateError::NotYetValid);
        }

        let earliest_accepted_expiry = now.checked_sub(5).ok_or(NumericDateError::Malformed)?;
        if earliest_accepted_expiry >= exp {
            return Err(NumericDateError::Expired);
        }

        let issue_age = iat
            .checked_sub(now)
            .and_then(i64::checked_abs)
            .ok_or(NumericDateError::Malformed)?;
        if issue_age > 35 {
            return Err(NumericDateError::Malformed);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FixedTrustContext, Fnd04VerificationError, NumericDate, NumericDateError,
        parse_compact_jws, parse_protected_header, verify_compact_signature,
    };
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn numeric_date_extremes_fail_closed_without_panicking() {
        for value in [i64::MIN, i64::MAX] {
            let result =
                std::panic::catch_unwind(|| NumericDate::validate(value, value, value, value));

            assert!(matches!(result, Ok(Err(NumericDateError::Malformed))));
        }
    }

    #[test]
    fn compact_jws_requires_bounded_exactly_three_ascii_segments() {
        for token in [
            String::new(),
            "e30.e30".to_owned(),
            "e30.e30.AA.extra".to_owned(),
            "a".repeat(4_097),
            "e30.é30.AA".to_owned(),
        ] {
            assert_eq!(
                parse_compact_jws(&token),
                Err(Fnd04VerificationError::Malformed)
            );
        }

        assert!(parse_compact_jws("e30.e30.AA").is_ok());
    }

    #[test]
    fn compact_jws_rejects_noncanonical_base64url_segments() {
        for token in ["e30=.e30.AA", "e30.e30.A+", "e30.e30.A"] {
            assert_eq!(
                parse_compact_jws(token),
                Err(Fnd04VerificationError::Malformed)
            );
        }
    }

    #[test]
    fn compact_jws_rejects_invalid_utf8_and_duplicate_json_members() {
        for token in ["__8.e30.AA", "e30.eyJhIjoxLCJhIjoyfQ.AA"] {
            assert_eq!(
                parse_compact_jws(token),
                Err(Fnd04VerificationError::Malformed)
            );
        }
    }

    #[test]
    fn protected_header_enforces_exact_members_and_algorithm_precedence()
    -> Result<(), Fnd04VerificationError> {
        let malformed = compact_token(r#"{"alg":"Ed25519","kid":"fresh"}"#);
        let non_exact_algorithm = compact_token(r#"{"alg":"EdDSA","kid":"fresh","typ":"x"}"#);
        let valid = compact_token(r#"{"alg":"Ed25519","kid":"fresh","typ":"x"}"#);

        let malformed_header = parse_compact_jws(&malformed)?;
        let non_exact_algorithm_header = parse_compact_jws(&non_exact_algorithm)?;
        let valid_header = parse_compact_jws(&valid)?;

        assert_eq!(
            parse_protected_header(&malformed_header),
            Err(Fnd04VerificationError::Malformed)
        );
        assert_eq!(
            parse_protected_header(&non_exact_algorithm_header),
            Err(Fnd04VerificationError::AuthenticationFailed)
        );
        assert!(parse_protected_header(&valid_header).is_ok());
        Ok(())
    }

    #[test]
    fn signature_validation_only_uses_verifier_fixed_trust_keys()
    -> Result<(), Fnd04VerificationError> {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let header = r#"{"alg":"Ed25519","kid":"fresh","typ":"x"}"#;
        let encoded_header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
        let signing_input = format!("{encoded_header}.e30");
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(signing_key.sign(signing_input.as_bytes()).to_bytes());
        let token = format!("{signing_input}.{signature}");
        let compact_jws = parse_compact_jws(&token)?;
        let protected_header = parse_protected_header(&compact_jws)?;
        let trusted = FixedTrustContext::new([("fresh", signing_key.verifying_key().to_bytes())]);
        let untrusted = FixedTrustContext::new([("other", signing_key.verifying_key().to_bytes())]);

        assert!(verify_compact_signature(&compact_jws, &protected_header, &trusted).is_ok());
        assert_eq!(
            verify_compact_signature(&compact_jws, &protected_header, &untrusted),
            Err(Fnd04VerificationError::AuthenticationFailed)
        );
        Ok(())
    }

    fn compact_token(header: &str) -> String {
        let encoded_header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
        format!("{encoded_header}.e30.AA")
    }
}

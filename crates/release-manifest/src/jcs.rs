//! RFC 8785 through upstream serde_json_canonicalizer 0.4.1 (MIT).
//! Qualified source: evik42/serde-json-canonicalizer@47e1a6b41b9e17f960c3fdaa47dcc291d499c7ca.
//! Do not use upstream `pipe`: parsing into Value would discard duplicate keys.

use serde::{Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Bounds,
    Invalid,
    NonCanonical,
}

/// Callers supply a bounded closed schema with duplicate/unknown-field rejection.
/// This function confers no signature, identity or authority qualification.
pub fn encode<T: Serialize>(value: &T, maximum: usize) -> Result<Vec<u8>, Error> {
    let bytes = serde_json_canonicalizer::to_vec(value).map_err(|_| Error::Invalid)?;
    if bytes.len() > maximum {
        return Err(Error::Bounds);
    }
    Ok(bytes)
}

/// Use only after authenticating the exact bytes at a signed consumer boundary.
/// Deserialize directly into the closed schema; never first into a JSON Value.
pub fn decode<T: DeserializeOwned + Serialize>(bytes: &[u8], maximum: usize) -> Result<T, Error> {
    if bytes.is_empty() || bytes.len() > maximum {
        return Err(Error::Bounds);
    }
    // serde_json's recursion limit stays enabled (128). Closed domain schemas
    // below have far smaller depth and no recursive or unbounded JSON values.
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let value = T::deserialize(&mut decoder).map_err(|_| Error::Invalid)?;
    decoder.end().map_err(|_| Error::Invalid)?;
    if encode(&value, maximum)? != bytes {
        return Err(Error::NonCanonical);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Closed {
        count: String,
    }

    #[test]
    fn cross_language_rfc8785_ecmascript_numbers() {
        // RFC8785 Appendix B IEEE754 vectors, also used by cyberphone's
        // ECMAScript reference implementation and upstream's Rust tests.
        for (bits, expected) in [
            (0x0000000000000000, "0"),
            (0x8000000000000000, "0"),
            (0x0000000000000001, "5e-324"),
            (0x7fefffffffffffff, "1.7976931348623157e+308"),
            (0x4340000000000000, "9007199254740992"),
            (0x4430000000000000, "295147905179352830000"),
        ] {
            assert_eq!(
                encode(&f64::from_bits(bits), 128).expect("vector"),
                expected.as_bytes()
            );
        }
    }

    #[test]
    fn cross_language_utf16_property_sort_and_escaping() {
        // RFC8785 §3.2.3: UTF16 ordering differs from UTF8/Unicode scalar order.
        let value: serde_json::Value = serde_json::from_str(
            r#"{"€":"Euro Sign","\r":"Carriage Return","דּ":"Hebrew Letter Dalet With Dagesh","1":"One","😀":"Emoji: Grinning Face","\u0080":"Control","ö":"Latin Small Letter O With Diaeresis"}"#
        ).expect("reference input");
        let expected = "{\"\\r\":\"Carriage Return\",\"1\":\"One\",\"\u{80}\":\"Control\",\"ö\":\"Latin Small Letter O With Diaeresis\",\"€\":\"Euro Sign\",\"😀\":\"Emoji: Grinning Face\",\"דּ\":\"Hebrew Letter Dalet With Dagesh\"}";
        assert_eq!(
            encode(&value, 1024).expect("reference output"),
            expected.as_bytes()
        );
    }

    #[test]
    fn closed_decode_rejects_duplicate_unknown_noncanonical_and_oversize() {
        for bytes in [
            br#"{"count":"1","count":"1"}"#.as_slice(),
            br#"{"count":"1","extra":0}"#,
            br#"{ "count":"1"}"#,
            br#"{"count":"1"} "#,
        ] {
            assert!(decode::<Closed>(bytes, 128).is_err());
        }
        assert!(decode::<Closed>(br#"{"count":"1"}"#, 2).is_err());
        assert!(decode::<Closed>(br#"{"count":"1"}"#, 128).is_ok());
    }
}

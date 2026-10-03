//! CHARM source-only JSON adapters. The caller bounds source bytes with Content limits;
//! these adapters preserve numerical literals and reject ambiguous members. No activation.

use super::ContentError;
use serde::{
    Deserialize,
    de::{self, DeserializeOwned, MapAccess, Visitor},
};
use serde_json::value::RawValue;
use std::collections::BTreeMap;

fn invalid() -> ContentError {
    ContentError::InvalidArtifact("invalid exact Charm source JSON")
}

/// Integer hundredths, bounded to u32; the effect owner validates its applicable range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExactPercent(pub(crate) u32);

impl<'de> Deserialize<'de> for ExactPercent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        exact_hundredths(raw.get())
            .map(Self)
            .map_err(de::Error::custom)
    }
}

fn exact_hundredths(token: &str) -> Result<u32, ContentError> {
    let mut parts = token.split('.');
    let whole = parts.next().ok_or_else(invalid)?;
    let fraction = parts.next();
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || (whole.len() > 1 && whole.starts_with('0'))
    {
        return Err(invalid());
    }
    let whole: u32 = whole.parse().map_err(|_| invalid())?;
    let fraction = match fraction {
        None => 0,
        Some(value)
            if !value.is_empty()
                && value.len() <= 2
                && value.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            value.parse::<u32>().map_err(|_| invalid())? * if value.len() == 1 { 10 } else { 1 }
        }
        _ => return Err(invalid()),
    };
    whole
        .checked_mul(100)
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(invalid)
}

/// Duplicate detection precedes map insertion. The owning closed shape consumes known
/// members with `take`, then requires `ensure_empty` to reject unknown parameters.
pub(crate) struct CharmSourceMembers(pub(crate) BTreeMap<String, Box<RawValue>>);
impl<'de> Deserialize<'de> for CharmSourceMembers {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MembersVisitor;
        impl<'de> Visitor<'de> for MembersVisitor {
            type Value = CharmSourceMembers;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("one Charm source object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut members = BTreeMap::new();
                while let Some((key, value)) = access.next_entry::<String, Box<RawValue>>()? {
                    if members.insert(key, value).is_some() {
                        return Err(de::Error::custom("duplicate Charm source member"));
                    }
                }
                Ok(CharmSourceMembers(members))
            }
        }
        deserializer.deserialize_map(MembersVisitor)
    }
}

pub(crate) fn take<T: DeserializeOwned>(
    members: &mut CharmSourceMembers,
    key: &str,
) -> Result<T, ContentError> {
    let raw = members.0.remove(key).ok_or_else(invalid)?;
    serde_json::from_str(raw.get()).map_err(|_| invalid())
}

pub(crate) fn ensure_empty(members: &CharmSourceMembers) -> Result<(), ContentError> {
    if members.0.is_empty() {
        Ok(())
    } else {
        Err(invalid())
    }
}

#[cfg(test)]
#[path = "charm_source_json_tests.rs"]
mod tests;

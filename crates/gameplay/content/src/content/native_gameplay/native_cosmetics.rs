//! D1093 canonical authoring admission. The outer native artifact owns issuance/currentness;
//! per-field promotion evidence is mandatory data and never inferred from source observations.
use super::cosmetics_loader::{ProposedCosmeticsBinding, compile_candidate};
use super::{PinnedGameplayBytes, hex, invalid};
use crate::content::ContentError;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) const MAX_BYTES: usize = 16 * 1024 * 1024;
const SCHEMA: &str = "OTERYN_NATIVE_COSMETICS/v1";
const UNAVAILABLE: &[u8] =
    b"{\"schema\":\"OTERYN_NATIVE_COSMETICS/v1\",\"availability\":\"unavailable\"}";

/// Explicit legacy-version policy, not an empty-byte sentinel or a fabricated fallback.
/// The ordinary pin qualification still checks these real bytes and their SHA256.
pub(super) fn unavailable() -> PinnedGameplayBytes {
    PinnedGameplayBytes {
        bytes: UNAVAILABLE.to_vec(),
        sha256: hex(crate::content::digest::sha256(UNAVAILABLE)),
    }
}

#[derive(Deserialize)]
#[serde(tag = "availability", deny_unknown_fields)]
enum Section {
    #[serde(rename = "unavailable")]
    Unavailable { schema: String },
    #[serde(rename = "authored")]
    Authored {
        schema: String,
        authoring: Box<Authoring>,
        source_manifest: String,
        source_shards: Vec<Shard>,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authoring {
    schema: String,
    profile: Profile,
    client_version: String,
    document: StrictValue,
    promotions: Vec<Promotion>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    kind: ProfileKind,
    revision: String,
    mount_speed_bonus: u16,
}
#[derive(Deserialize)]
enum ProfileKind {
    #[serde(rename = "reference")]
    Reference,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Promotion {
    path: String,
    value: StrictValue,
    state: PromotionState,
    decision: String,
    source_kind: PromotionSource,
    source: Citation,
}
#[derive(Deserialize)]
enum PromotionState {
    #[serde(rename = "PROMOTED")]
    Promoted,
}
#[derive(Deserialize)]
enum PromotionSource {
    #[serde(rename = "STRUCTURED_REFERENCE_DATA")]
    StructuredReference,
    #[serde(rename = "OWNER_DECISION")]
    OwnerDecision,
    #[serde(rename = "ACCEPTED_REFERENCE_EVIDENCE")]
    AcceptedReference,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Citation {
    reference: String,
    revision: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Shard {
    path: String,
    json: String,
}

/// Reject duplicate members before normalizing the authored document or promotion values.
struct StrictValue(Value);
impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Value;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("JSON with unique members and integer numbers")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Value, E> {
                Ok(Value::Bool(v))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Value, E> {
                Ok(v.into())
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Value, E> {
                Ok(v.into())
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Value, E> {
                Ok(v.into())
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
                let mut values = Vec::new();
                while let Some(v) = a.next_element::<StrictValue>()? {
                    values.push(v.0);
                }
                Ok(Value::Array(values))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((k, v)) = a.next_entry::<String, StrictValue>()? {
                    if values.insert(k, v.0).is_some() {
                        return Err(serde::de::Error::custom("duplicate cosmetics member"));
                    }
                }
                Ok(Value::Object(values))
            }
        }
        d.deserialize_any(Visitor).map(Self)
    }
}

fn text(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.bytes().all(|v| (b' '..=b'~').contains(&v))
        && !value.eq_ignore_ascii_case("UNKNOWN")
}
fn canonical_document(value: &Value) -> Result<(), ContentError> {
    fn ascii(value: &Value) -> bool {
        match value {
            Value::String(v) => {
                v.bytes().all(|v| (b' '..=b'~').contains(&v)) && !v.eq_ignore_ascii_case("UNKNOWN")
            }
            Value::Array(values) => values.iter().all(ascii),
            Value::Object(values) => values.values().all(ascii),
            _ => true,
        }
    }
    let key = |value: &Value| -> bool {
        value.as_str().is_some_and(|key| {
            key.len() <= 160
                && key.strip_prefix("oteryn:").is_some_and(|local| {
                    local
                        .as_bytes()
                        .first()
                        .is_some_and(|v| v.is_ascii_lowercase() || v.is_ascii_digit())
                        && local.bytes().all(|v| {
                            v.is_ascii_lowercase()
                                || v.is_ascii_digit()
                                || matches!(v, b'.' | b'_' | b'/' | b':' | b'-')
                        })
                })
        })
    };
    if !ascii(value) || !key(&value["palette"]["key"]) {
        return Err(invalid("cosmetics canonical text/key profile"));
    }
    for family in ["outfits", "mounts"] {
        let records = value[family]
            .as_array()
            .ok_or_else(|| invalid("cosmetics records"))?;
        if records.len() > super::MAX_RECORDS || records.iter().any(|r| !key(&r["key"])) {
            return Err(invalid("cosmetics canonical record bounds/keys"));
        }
        if family == "outfits"
            && records.iter().any(|r| {
                r["unlock_sources"]
                    .as_array()
                    .is_none_or(|v| v.len() > super::MAX_RECORDS)
            })
        {
            return Err(invalid("cosmetics canonical unlock-source bounds"));
        }
    }
    Ok(())
}
fn digest(value: &str) -> Result<[u8; 32], ContentError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
    {
        return Err(invalid("cosmetics source digest"));
    }
    let mut result = [0; 32];
    for (out, pair) in result.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        let pair = std::str::from_utf8(pair).map_err(|_| invalid("cosmetics digest UTF8"))?;
        *out = u8::from_str_radix(pair, 16).map_err(|_| invalid("cosmetics digest hex"))?;
    }
    if result == [0; 32] {
        return Err(invalid("cosmetics zero source digest"));
    }
    Ok(result)
}

fn promotion_paths(document: &Value) -> Result<BTreeSet<String>, ContentError> {
    let mut paths: BTreeSet<_> = [
        "/source_manifest_sha256",
        "/source_appearances_sha256",
        "/palette",
        "/fallbacks/female",
        "/fallbacks/male",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    if document.get("grant_mappings").is_some() {
        paths.insert("/grant_mappings".to_owned());
    }
    for (family, fields) in [
        (
            "outfits",
            &[
                "key",
                "revision",
                "female",
                "male",
                "addon_count",
                "starter_female",
                "starter_male",
                "starter_addons_female",
                "starter_addons_male",
                "premium",
                "unlock_sources",
                "source",
            ][..],
        ),
        (
            "mounts",
            &[
                "key",
                "revision",
                "look",
                "speed_bonus",
                "premium",
                "source",
            ][..],
        ),
    ] {
        let records = document[family]
            .as_array()
            .ok_or_else(|| invalid("cosmetics records"))?;
        for index in 0..records.len() {
            for field in fields {
                paths.insert(format!("/{family}/{index}/{field}"));
            }
        }
    }
    Ok(paths)
}

pub(super) fn decode(bytes: &[u8]) -> Result<Option<super::NativeCosmeticsView>, ContentError> {
    super::bounded(bytes, MAX_BYTES)?;
    let section: Section = serde_json::from_slice(bytes)
        .map_err(|_| invalid("cosmetics section JSON or unpromoted source"))?;
    let (schema, authoring, source_manifest, source_shards) = match section {
        Section::Authored {
            schema,
            authoring,
            source_manifest,
            source_shards,
        } => (schema, authoring, source_manifest, source_shards),
        Section::Unavailable { schema } => {
            return if schema == SCHEMA {
                Ok(None)
            } else {
                Err(invalid("cosmetics unavailable schema"))
            };
        }
    };
    let authoring = *authoring;
    if schema != SCHEMA
        || authoring.schema != "OTERYN_CHARACTER_COSMETICS_AUTHORING/v1"
        || authoring.document.0["schema"] != "OTERYN_CHARACTER_COSMETICS/v1"
        || !text(&authoring.client_version, 128)
        || authoring.profile.mount_speed_bonus != 10
    {
        return Err(invalid("cosmetics authoring/profile discriminator"));
    }
    crate::content::DefinitionRevisionRef::new(&authoring.profile.revision)?;
    let mut document = authoring.document.0;
    if !text(&authoring.profile.revision, 128) || authoring.promotions.len() > 65536 {
        return Err(invalid("cosmetics canonical profile/promotion bounds"));
    }
    canonical_document(&document)?;
    let expected_paths = promotion_paths(&document)?;
    let mut seen = BTreeSet::new();
    for promotion in authoring.promotions {
        let _ = (&promotion.state, &promotion.source_kind);
        if !expected_paths.contains(&promotion.path)
            || !seen.insert(promotion.path.clone())
            || document.pointer(&promotion.path) != Some(&promotion.value.0)
            || !text(&promotion.decision, 1024)
            || !text(&promotion.source.reference, 1024)
            || !text(&promotion.source.revision, 128)
        {
            return Err(invalid(
                "cosmetics missing, duplicated or mismatched promotion",
            ));
        }
        digest(&promotion.source.sha256)?;
    }
    if seen != expected_paths {
        return Err(invalid("cosmetics incomplete field promotion"));
    }
    if document["mounts"]
        .as_array()
        .is_none_or(|mounts| mounts.iter().any(|m| m["speed_bonus"] != 10))
    {
        return Err(invalid("cosmetics Reference ruleset speed"));
    }
    if document["palette"]["colours"].as_array().map(Vec::len) != Some(133) {
        return Err(invalid("cosmetics Reference palette size"));
    }
    let content_revision = document["content_revision"]
        .as_str()
        .ok_or_else(|| invalid("cosmetics content revision"))?
        .to_owned();
    let keys = |family: &str| -> Result<super::Arc<[String]>, ContentError> {
        let rows = document[family]
            .as_array()
            .ok_or_else(|| invalid("cosmetics records"))?;
        let mut keys = BTreeSet::new();
        for row in rows {
            let key = row["key"]
                .as_str()
                .ok_or_else(|| invalid("cosmetics definition key"))?;
            keys.insert(key.to_owned());
        }
        Ok(keys.into_iter().collect::<Vec<_>>().into())
    };
    let outfit_keys = keys("outfits")?;
    let mount_keys = keys("mounts")?;
    let appearances_sha256 = digest(
        document["source_appearances_sha256"]
            .as_str()
            .ok_or_else(|| invalid("cosmetics appearances digest"))?,
    )?;
    // Only the schema discriminator is normalized for the existing, single source-bound
    // compiler. No source ID, sex, starter, premium, palette or fallback value is inferred.
    document["schema"] = "OTERYN_CHARACTER_COSMETICS_CANDIDATE/v1".into();
    let document = serde_json::to_vec(&document).map_err(|_| invalid("cosmetics normalization"))?;
    let shards: Vec<_> = source_shards
        .iter()
        .map(|s| (s.path.as_str(), s.json.as_bytes()))
        .collect();
    compile_candidate(
        &document,
        ProposedCosmeticsBinding {
            document_sha256: crate::content::digest::sha256(&document),
            content_revision: &content_revision,
            manifest_sha256: crate::content::digest::sha256(source_manifest.as_bytes()),
            appearances_sha256,
            client_version: &authoring.client_version,
        },
        source_manifest.as_bytes(),
        &shards,
    )
    .map(|content| {
        Some(super::NativeCosmeticsView {
            // Parsing supplies immutable data only. The enclosing native decoder below
            // binds this field to the complete outer artifact before exposing its state.
            source_digest: crate::content::digest::sha256(bytes),
            content: super::Arc::new(content),
            outfit_keys,
            mount_keys,
        })
    })
    .map_err(|_| invalid("cosmetics source-bound content"))
}

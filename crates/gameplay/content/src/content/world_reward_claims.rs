//! CHEST-PLACE-BIND-1 (ARCH-WORLD-CONTENT-SERVE-1 §1.4, §1.5): the served set of RewardClaims and
//! the binding of each placement to exactly one entry of a World's base map.
//!
//! [`bind_reward_claims`] is a pure function of a [`RewardClaimBase`], the reward-claim shards and the
//! admitted Item registry. It writes nothing and returns the claims a World serves, with each
//! placement's canonical [`PlacementKey`], bound cell and bundle `placement_key`, plus the
//! placements that did not bind and the records left out, so the caller can gate boot on it.

use crate::content::{
    ContentError, DefinitionFamily, DefinitionRevisionRef, PlacementKey, ProductionKey,
    RewardClaimItem, RewardClaimPlacement, TypedDefinitionRef,
};
use oteryn_world_bundle::bundle;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;

/// The `unique` attribute of a top-level entry and the appearance id of its palette key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UniqueEntry {
    pub unique: u16,
    pub appearance: Option<u16>,
}

/// Read-only base-map data used to bind reward placements.
pub trait RewardClaimBase {
    type Tile<'a>: RewardClaimTile
    where
        Self: 'a;

    fn tile(&self, x: u16, y: u16, floor: i8) -> Option<Self::Tile<'_>>;
}

/// The payload depths and unique attributes of one base-map tile.
pub trait RewardClaimTile {
    fn depths(&self) -> &[u8];
    fn unique(&self, ordinal: u8) -> Option<UniqueEntry>;
}

/// The RewardClaim shards, embedded as the quest document is: the plain shards, then the authored
/// variant shards. Keys and the exact W15b native container projection are served;
/// other source dispositions remain closed.
pub const REWARD_CLAIM_SHARDS: [&str; 5] = [
    include_str!(
        "../../../../../content/interactions/reward_claims/reward-claims-00000-00099.json"
    ),
    include_str!(
        "../../../../../content/interactions/reward_claims/reward-claims-00100-00199.json"
    ),
    include_str!(
        "../../../../../content/interactions/reward_claims/reward-claims-00200-00230.json"
    ),
    include_str!(
        "../../../../../content/interactions/reward_claims/reward-claim-variants-00000-00099.json"
    ),
    include_str!(
        "../../../../../content/interactions/reward_claims/reward-claim-variants-00100-00104.json"
    ),
];

/// Why a placement is not bound (§1.4). Each is exactly one reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnboundReason {
    /// The placement has no `crystalserver` legacy unique id.
    NoCrystalUniqueId,
    /// The project position has no native cell.
    CellOutOfBounds,
    /// No top-level entry on the cell has the unique id.
    NoEntry,
    /// Two or more top-level entries on the cell have the unique id.
    AmbiguousEntry,
    /// The entry's palette appearance id is absent or differs from `appearance_tibia_id`.
    AppearanceMismatch,
}

/// Why a record is not a served candidate (§1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LeftOut {
    /// An authored variant record other than a key reward.
    Variant,
    /// `readiness` is not `ready` (`WAITING_*`).
    NotReady,
    /// A field or shape the server artifact does not encode (`WAITING_UNENCODED_FIELD`).
    UnencodedField,
    /// A reward item without an admitted, materializable Item definition.
    ItemNotAdmitted,
}

/// The native cell a placement bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundCell {
    pub x: u16,
    pub y: u16,
    pub floor: i8,
}

/// A placement bound to one entry of the base map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundPlacement {
    /// The canonical placement, keyed `oteryn:placement/<claim key sans prefix>/<index>`.
    pub placement: RewardClaimPlacement,
    pub cell: BoundCell,
    /// The compiler's `placement_key` of the entry. In memory only: it names the entry only
    /// together with the bundle digest (ADR-0021 §4.2).
    pub bundle_placement_key: u64,
    /// Fresh direct children of the reward container, in source order. Every
    /// child is part of the same claim; an empty vector is the legacy leaf path.
    pub contents: Vec<RewardContainerItem>,
    /// Source-authored text on a leaf reward; this is not player writing.
    pub root_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardContainerItem {
    pub item: TypedDefinitionRef,
    pub count: u32,
    pub key_number: Option<u16>,
    pub text: Option<String>,
}

/// A claim whose placements are all bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedClaim {
    pub key: ProductionKey,
    pub revision: DefinitionRevisionRef,
    pub placements: Vec<BoundPlacement>,
}

/// A placement of a served candidate that did not bind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnboundPlacement {
    pub claim: String,
    pub index: usize,
    pub reason: UnboundReason,
}

/// The result of one binding pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BindingReport {
    pub served: Vec<ServedClaim>,
    pub unbound: Vec<UnboundPlacement>,
    pub left_out: Vec<(String, LeftOut)>,
}

impl BindingReport {
    /// The claims with at least one unbound placement, by the reason of their first one.
    pub fn unbound_claims_by_reason(&self) -> BTreeMap<UnboundReason, usize> {
        let mut seen = BTreeMap::new();
        for entry in &self.unbound {
            seen.entry(entry.claim.as_str()).or_insert(entry.reason);
        }
        let mut counts = BTreeMap::new();
        for reason in seen.into_values() {
            *counts.entry(reason).or_insert(0) += 1;
        }
        counts
    }
}

/// A shard the reader cannot decode. The shards are embedded, so this refuses the whole pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimsError(pub &'static str);

impl fmt::Display for ClaimsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "reward claim shard is malformed: {}", self.0)
    }
}

impl std::error::Error for ClaimsError {}

impl From<ContentError> for ClaimsError {
    fn from(_: ContentError) -> Self {
        Self("a key or revision is not a valid Content identifier")
    }
}

/// [`bind_reward_claims`] over the embedded plain shards.
pub fn bind_embedded_reward_claims(
    base: &impl RewardClaimBase,
    admitted: &dyn Fn(&TypedDefinitionRef) -> bool,
) -> Result<BindingReport, ClaimsError> {
    bind_reward_claims(base, &REWARD_CLAIM_SHARDS, admitted)
}

/// The served set and binding report of `shards` over `base` (§1.4, §1.5). `admitted` says
/// whether an Item definition is admitted and materializable.
pub fn bind_reward_claims(
    base: &impl RewardClaimBase,
    shards: &[&str],
    admitted: &dyn Fn(&TypedDefinitionRef) -> bool,
) -> Result<BindingReport, ClaimsError> {
    let mut report = BindingReport::default();
    for shard in shards {
        let shard: Value =
            serde_json::from_str(shard).map_err(|_| ClaimsError("not a JSON document"))?;
        let records = shard
            .get("records")
            .and_then(Value::as_array)
            .ok_or(ClaimsError("no records"))?;
        for record in records {
            let definition = record
                .get("definition")
                .and_then(Value::as_object)
                .ok_or(ClaimsError("record without a definition"))?;
            let key = definition
                .get("identity")
                .and_then(|identity| identity.get("key"))
                .and_then(Value::as_str)
                .ok_or(ClaimsError("record without an identity key"))?;
            match candidate(definition, admitted)? {
                Err(left_out) => report.left_out.push((key.to_owned(), left_out)),
                Ok(candidate) => bind_candidate(base, key, &candidate, &mut report)?,
            }
        }
    }
    Ok(report)
}

struct Candidate {
    revision: String,
    placements: Vec<CandidatePlacement>,
}

struct CandidatePlacement {
    appearance: u64,
    cell: Option<BoundCell>,
    crystal_unique_id: Option<u64>,
    item: TypedDefinitionRef,
    count: u32,
    quest_transition: Option<String>,
    key_number: Option<u16>,
    achievement: Option<String>,
    contents: Vec<RewardContainerItem>,
    root_text: Option<String>,
}

const DEFINITION_FIELDS: [&str; 6] = [
    "claim",
    "identity",
    "provenance",
    "quest",
    "readiness",
    "placements",
];
/// The fields an authored key variant adds to [`DEFINITION_FIELDS`].
const KEY_VARIANT_FIELDS: [&str; 5] = [
    "data_holds",
    "definition_profile",
    "native_admission",
    "source_variant",
    "native_contents",
];
/// The one lowering a key variant waits for, which KEY-1 implements.
const KEY_REWARD_LOWERING: &str = "KEY_REWARD_NATIVE_LOWERING_NOT_IMPLEMENTED";
/// The authored binding of a key reward: `canary:door-key/<decimal number>`.
const KEY_BINDING_PREFIX: &str = "canary:door-key/";
const PLACEMENT_FIELDS: [&str; 4] = [
    "appearance_tibia_id",
    "quest_transition",
    "reward",
    "source_binding",
];

fn only(value: &Value, allowed: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|map| map.keys().all(|key| allowed.contains(&key.as_str())))
}

/// Whether the record is a served candidate; the outer error is a malformed shard.
fn candidate(
    definition: &serde_json::Map<String, Value>,
    admitted: &dyn Fn(&TypedDefinitionRef) -> bool,
) -> Result<Result<Candidate, LeftOut>, ClaimsError> {
    let key_variant = definition.contains_key("definition_profile");
    let native_contents = native_container_placements(definition);
    if key_variant && !waits_only_for_key_lowering(definition) && native_contents.is_none() {
        return Ok(Err(LeftOut::Variant));
    }
    if !key_variant && definition.get("readiness").and_then(Value::as_str) != Some("ready") {
        return Ok(Err(LeftOut::NotReady));
    }
    let whole = Value::Object(definition.clone());
    let claim = definition.get("claim");
    let encoded = definition.keys().all(|key| {
        DEFINITION_FIELDS.contains(&key.as_str())
            || (key_variant && KEY_VARIANT_FIELDS.contains(&key.as_str()))
    }) && claim.is_some_and(|claim| {
        only(claim, &["per", "repeat"])
            && claim.get("per").and_then(Value::as_str) == Some("character")
            && claim
                .get("repeat")
                .is_some_and(|repeat| only(repeat, &["kind"]))
            && claim
                .get("repeat")
                .and_then(|repeat| repeat.get("kind"))
                .and_then(Value::as_str)
                == Some("once")
    });
    let placements = whole
        .get("placements")
        .and_then(Value::as_array)
        .ok_or(ClaimsError("record without placements"))?;
    let revision = definition
        .get("identity")
        .and_then(|identity| identity.get("revision"))
        .and_then(Value::as_str)
        .ok_or(ClaimsError("record without an identity revision"))?;
    let mut parsed = Vec::with_capacity(placements.len());
    let mut all_encoded = encoded && !placements.is_empty();
    if native_contents.is_some_and(|recipes| recipes.len() != placements.len()) {
        return Ok(Err(LeftOut::UnencodedField));
    }
    for (index, placement) in placements.iter().enumerate() {
        let parsed_placement = match native_contents {
            Some(recipes) => candidate_container_placement(placement, &recipes[index])?,
            None => candidate_placement(placement, key_variant)?,
        };
        match parsed_placement {
            Some(placement) => parsed.push(placement),
            None => all_encoded = false,
        }
    }
    if !all_encoded {
        return Ok(Err(LeftOut::UnencodedField));
    }
    if parsed.iter().any(|placement| {
        !admitted(&placement.item)
            || placement
                .contents
                .iter()
                .any(|child| !admitted(&child.item))
    }) {
        return Ok(Err(LeftOut::ItemNotAdmitted));
    }
    Ok(Ok(Candidate {
        revision: revision.to_owned(),
        placements: parsed,
    }))
}

/// Whether an authored variant waits for nothing but the key reward lowering: its data is
/// complete, it holds no data, grants and charges nothing, and its one pending lowering is
/// [`KEY_REWARD_LOWERING`].
fn waits_only_for_key_lowering(definition: &serde_json::Map<String, Value>) -> bool {
    let text =
        |value: Option<&Value>, expected: &str| value.and_then(Value::as_str) == Some(expected);
    let empty = |value: Option<&Value>| value.and_then(Value::as_array).is_some_and(Vec::is_empty);
    let variant = definition.get("source_variant");
    let field = |name: &str| variant.and_then(|variant| variant.get(name));
    let lowering = field("native_lowering");
    text(definition.get("definition_profile"), "authored_variant_v1")
        && text(definition.get("native_admission"), "WAITING_IMPLEMENTATION")
        && text(definition.get("readiness"), "waiting_implementation")
        && empty(definition.get("data_holds"))
        && empty(field("achievement_grants"))
        && empty(field("charge_dispositions"))
        && text(field("source_data").and_then(|data| data.get("status")), "COMPLETE")
        && text(lowering.and_then(|lowering| lowering.get("status")), "WAITING_IMPLEMENTATION")
        && lowering
            .and_then(|lowering| lowering.get("pending_implementation"))
            .and_then(Value::as_array)
            .is_some_and(|pending| {
                matches!(pending.as_slice(), [only] if only.as_str() == Some(KEY_REWARD_LOWERING))
            })
}

/// The key number of a `canary:door-key/<number>` binding, in 1..=65,535 (`WORLDINT0-RL-10`).
fn key_number(binding: &str) -> Option<u16> {
    let digits = binding
        .strip_prefix(KEY_BINDING_PREFIX)
        .or_else(|| (binding == "crystalserver:door-key/3002").then_some("3002"))?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u16>().ok().filter(|number| *number != 0)
}

/// The source DATA packet is immutable evidence. Only the additional authored
/// native projection admits a filled container; missing source or Item facts
/// still refuse the entire claim (no partial rewards).
fn native_container_placements(definition: &serde_json::Map<String, Value>) -> Option<&Vec<Value>> {
    if definition.get("definition_profile")?.as_str()? != "authored_variant_v1"
        || definition.get("native_admission")?.as_str()? != "WAITING_IMPLEMENTATION"
        || definition.get("readiness")?.as_str()? != "waiting_implementation"
        || !definition.get("data_holds")?.as_array()?.is_empty()
        || definition
            .get("source_variant")?
            .get("source_data")?
            .get("status")?
            .as_str()?
            != "COMPLETE"
    {
        return None;
    }
    let projection = definition.get("native_contents")?;
    if !only(
        projection,
        &["schema", "revision", "source_sha256", "placements"],
    ) || projection.get("schema")?.as_str()? != "OTERYN_NATIVE_CONTAINER_REWARD/v1"
        || projection.get("revision")?.as_str()? != "201189f871d23597b82caf88fd611454a2ddefb8"
        || projection.get("source_sha256")?.as_str()?
            != "b073d546930a60adf511677f932e6e0d37df3a230313dc85c4b9253742325757"
    {
        return None;
    }
    projection
        .get("placements")?
        .as_array()
        .filter(|entries| !entries.is_empty())
}

fn candidate_container_placement(
    placement: &Value,
    recipe: &Value,
) -> Result<Option<CandidatePlacement>, ClaimsError> {
    let bad = ClaimsError("native container recipe is malformed");
    if !only(
        placement,
        &[
            "appearance_tibia_id",
            "quest_transition",
            "reward",
            "source_binding",
            "achievement_grant",
        ],
    ) || !only(
        recipe,
        &[
            "unique_id",
            "line",
            "root",
            "capacity",
            "contents",
            "completion",
            "achievement",
            "root_text",
        ],
    ) {
        return Ok(None);
    }
    let reward = placement.get("reward").ok_or(bad.clone())?;
    let achievement = match (
        placement.get("achievement_grant"),
        recipe.get("achievement"),
    ) {
        (None, None) => None,
        (Some(grant), Some(reference)) => {
            let expected = serde_json::json!({"family": "Achievement", "key": "oteryn:achievement/annihilator", "revision": "1"});
            let expected_grant = serde_json::json!({"state": "KNOWN", "value": {
                "achievement": expected, "owner": "Achievement", "request": "grant"
            }});
            if grant != &expected_grant
                || reference != &expected
                || !recipe
                    .get("unique_id")
                    .and_then(Value::as_u64)
                    .is_some_and(|uid| (6085..=6088).contains(&uid))
                || recipe.get("completion")
                    != Some(
                        &serde_json::json!({"kind": "storage", "key": "Storage.Quest.U7_24.TheAnnihilator.Reward"}),
                    )
            {
                return Ok(None);
            }
            Some("oteryn:achievement/annihilator".to_owned())
        }
        _ => return Ok(None),
    };
    let mut ordinary = placement.clone();
    ordinary
        .as_object_mut()
        .ok_or(bad.clone())?
        .remove("achievement_grant");
    if recipe.get("capacity").is_some_and(Value::is_null) {
        // Leaf alternatives and written books use the same once-claim owner.
        if recipe.get("contents") != Some(&serde_json::json!([]))
            || reward.get("items")
                != Some(&serde_json::json!([{"item": recipe["root"], "count": 1}]))
        {
            return Ok(None);
        }
        let key_leaf = recipe.get("unique_id").and_then(Value::as_u64) == Some(6093)
            && only(reward, &["items", "key_binding"])
            && reward.get("key_binding").and_then(Value::as_str)
                == Some("crystalserver:door-key/3002");
        let root_text = match recipe.get("root_text") {
            Some(body) => {
                let Some(body) = body.as_str() else {
                    return Ok(None);
                };
                if achievement.is_some()
                    || !only(reward, &["items", "written_text"])
                    || !authored_text_matches(
                        body,
                        reward.get("written_text"),
                        &recipe["root"],
                        recipe.get("unique_id").and_then(Value::as_u64),
                    )
                {
                    return Ok(None);
                }
                Some(body.to_owned())
            }
            None if achievement.is_some() && only(reward, &["items"]) => None,
            None if achievement.is_none() && key_leaf => None,
            None => return Ok(None),
        };
        if !key_leaf {
            ordinary["reward"] = serde_json::json!({"items": reward["items"]});
        }
        let Some(mut result) = candidate_placement(&ordinary, key_leaf)? else {
            return Ok(None);
        };
        if result.crystal_unique_id != recipe.get("unique_id").and_then(Value::as_u64) {
            return Ok(None);
        }
        result.achievement = achievement;
        result.root_text = root_text;
        return Ok(Some(result));
    }
    if recipe.get("root_text").is_some() {
        return Ok(None);
    }
    if !only(
        reward,
        &["container", "items", "key_binding", "written_text"],
    ) || reward.get("container") != recipe.get("root")
    {
        return Ok(None);
    }
    let capacity = recipe
        .get("capacity")
        .and_then(Value::as_u64)
        .ok_or(bad.clone())?;
    let rows = recipe
        .get("contents")
        .and_then(Value::as_array)
        .ok_or(bad.clone())?;
    let source_rows = reward
        .get("items")
        .and_then(Value::as_array)
        .ok_or(bad.clone())?;
    if !(1..=65535).contains(&capacity)
        || rows.is_empty()
        || rows.len() > capacity.min(20) as usize
        || rows.len() != source_rows.len()
    {
        return Ok(None);
    }
    let key = reward
        .get("key_binding")
        .and_then(Value::as_str)
        .and_then(key_number);
    if reward.get("key_binding").is_some() && key.is_none() {
        return Ok(None);
    }
    // Reuse the strict singleton placement decoder for the actual root.
    let mut root = ordinary.clone();
    root["reward"] = serde_json::json!({"items": [{"item": recipe["root"], "count": 1}]});
    let Some(mut result) = candidate_placement(&root, false)? else {
        return Ok(None);
    };
    if result.crystal_unique_id != recipe.get("unique_id").and_then(Value::as_u64) {
        return Ok(None);
    }
    result.achievement = achievement;
    for (row, source) in rows.iter().zip(source_rows) {
        if !only(row, &["item", "count", "key_number", "text"])
            || !only(source, &["item", "count"])
            || row.get("item") != source.get("item")
            || row.get("count") != source.get("count")
        {
            return Ok(None);
        }
        let number = match row.get("key_number") {
            None => None,
            Some(value) => {
                let Some(number) = value
                    .as_u64()
                    .and_then(|v| u16::try_from(v).ok())
                    .filter(|v| *v > 0)
                else {
                    return Ok(None);
                };
                if Some(number) != key || row.get("count").and_then(Value::as_u64) != Some(1) {
                    return Ok(None);
                }
                Some(number)
            }
        };
        let mut child = ordinary.clone();
        child["reward"] = serde_json::json!({"items": [source]});
        let Some(child) = candidate_placement(&child, false)? else {
            return Ok(None);
        };
        if child.count > 100 {
            return Ok(None);
        }
        // Only donor Item types marked `key` receive keyAction. A missing key
        // number is as invalid as assigning it to a cookie or another reward.
        let donor_key = matches!(
            child.item.key().as_str(),
            "oteryn:item.tibia.i2970" | "oteryn:item.tibia.i2971" | "oteryn:item.tibia.i2972"
        );
        if number != key.filter(|_| donor_key) {
            return Ok(None);
        }
        let text = match row.get("text") {
            None => None,
            Some(body) => {
                let Some(body) = body.as_str() else {
                    return Ok(None);
                };
                if child.count != 1
                    || result.crystal_unique_id != Some(6195)
                    || !authored_text_matches(
                        body,
                        reward.get("written_text"),
                        &row["item"],
                        result.crystal_unique_id,
                    )
                {
                    return Ok(None);
                }
                Some(body.to_owned())
            }
        };
        result.contents.push(RewardContainerItem {
            item: child.item,
            count: child.count,
            key_number: number,
            text,
        });
    }
    if result
        .contents
        .iter()
        .filter(|child| child.text.is_some())
        .count()
        != usize::from(reward.get("written_text").is_some())
    {
        return Ok(None);
    }
    Ok(Some(result))
}

/// Exact source book identities and normalized text digests. The established
/// chest authoring strips outer LF; no other text or carrier is admitted.
fn authored_text_matches(
    body: &str,
    written: Option<&Value>,
    item: &Value,
    uid: Option<u64>,
) -> bool {
    let Some(written) = written else { return false };
    let (key, expected_digest) = match uid {
        Some(6013) => (
            "oteryn:item.tibia.i2821",
            "bdeaf6e287079dc867ef991a0d8b2afefae3ffaac3e5c12f189b943581f80263",
        ),
        Some(6112) => (
            "oteryn:item.tibia.i2821",
            "9ea894f92d1b6e4cf60555cd08f5e552b8c5ce59c11c13eb6d35f7ff07abb036",
        ),
        Some(6179) => (
            "oteryn:item.tibia.i2815",
            "3d95b7c96da6a14a36118fbbd5a8397239e7e3cc9931c5f60f65d136bd72ff27",
        ),
        Some(6187) => (
            "oteryn:item.tibia.i2821",
            "53dec5b27efa9bdef83a186a77b25230554dbade9ae2ce10cf1858d6a328bff1",
        ),
        Some(6195) => (
            "oteryn:item.tibia.i2816",
            "484f70e524471dc25e2dc695deae9e697aa11497a1d6d7d6d32b2e79f3d7f476",
        ),
        Some(6196) => (
            "oteryn:item.tibia.i2821",
            "50918d1f490444e7ecc11a36c2a48b6e1c6ee9229931abb59c69b1042df50b9f",
        ),
        Some(6202) => (
            "oteryn:item.tibia.i2836",
            "cd5d63f500cf56283e976d96162ed57678dd8544fd2a174d32744747fe607e11",
        ),
        Some(6203) => (
            "oteryn:item.tibia.i2816",
            "d066a264757f85fdf15d081a61e5554426884420ac6e67dcb8c4ddc82821c70d",
        ),
        Some(6214) => (
            "oteryn:item.tibia.i2820",
            "41bc61280d076bacf1f1af4d5566c0fecec27a1ef58224a87c1b1e13050bbaf7",
        ),
        Some(6250) => (
            "oteryn:item.tibia.i5956",
            "dd0900f6022f0c8f804fa153ab0c101015fd1d6d5c34b5d39cbc5c510b1c4328",
        ),
        Some(6272) => (
            "oteryn:item.tibia.i8117",
            "28974209eb9e879c26e4f9c92c6ea10918ebaef9d7dcb4a783c15a2bb69824c2",
        ),
        Some(6296) => (
            "oteryn:item.tibia.i11699",
            "ab5fcc7f25cc5e5039b1cdd1cf3dca12fc6e55bcfb9fa43ac0c6d79fc8e06cc4",
        ),
        Some(6500) => (
            "oteryn:item.tibia.i2821",
            "96ac3309e4bc4f53b6d1d45e86b907e5a7ad05475e823960e666e1916a9b4ec0",
        ),
        _ => return false,
    };
    let digest = Sha256::digest(body.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let expected_ref =
        serde_json::json!({"length":body.chars().count(),"placeholders":[],"sha256":digest});
    !body.is_empty()
        && body.len() <= 7996
        && body.chars().count() <= 4000
        && !body.contains('\0')
        && item.get("key").and_then(Value::as_str) == Some(key)
        && digest == expected_digest
        && only(written, &["item", "text_ref"])
        && written.get("item") == Some(item)
        && written.get("text_ref") == Some(&expected_ref)
}

/// `None` for a placement with a field or shape the server artifact does not encode. A key
/// variant's placement must carry a key binding of one item; no other placement may.
fn candidate_placement(
    placement: &Value,
    key_variant: bool,
) -> Result<Option<CandidatePlacement>, ClaimsError> {
    let bad = ClaimsError("placement is not in the shard shape");
    if !only(placement, &PLACEMENT_FIELDS) {
        return Ok(None);
    }
    let appearance = placement
        .get("appearance_tibia_id")
        .and_then(Value::as_u64)
        .ok_or(bad.clone())?;
    let binding = placement.get("source_binding").ok_or(bad.clone())?;
    let reward = placement.get("reward").ok_or(bad.clone())?;
    let reward_fields: &[&str] = if key_variant {
        &["items", "key_binding"]
    } else {
        &["items"]
    };
    if !only(binding, &["legacy_unique_ids", "project_position"]) || !only(reward, reward_fields) {
        return Ok(None);
    }
    let key_number = match reward.get("key_binding") {
        None if key_variant => return Ok(None),
        None => None,
        Some(binding) => {
            let binding = binding.as_str().ok_or(bad.clone())?;
            let Some(number) = key_number(binding) else {
                return Ok(None);
            };
            Some(number)
        }
    };
    let items = reward
        .get("items")
        .and_then(Value::as_array)
        .ok_or(bad.clone())?;
    let [entry] = items.as_slice() else {
        return Ok(None);
    };
    if !only(entry, &["count", "item"]) {
        return Ok(None);
    }
    let count = entry
        .get("count")
        .and_then(Value::as_u64)
        .and_then(|count| u32::try_from(count).ok())
        .filter(|count| *count > 0)
        .ok_or(bad.clone())?;
    // A key is one non-stackable item.
    if key_number.is_some() && count != 1 {
        return Ok(None);
    }
    let item = entry.get("item").ok_or(bad.clone())?;
    let text = |field: &str| item.get(field).and_then(Value::as_str).ok_or(bad.clone());
    if text("family")? != "Item" {
        return Ok(None);
    }
    let item = TypedDefinitionRef::new(
        DefinitionFamily::Item,
        ProductionKey::new(text("key")?)?,
        DefinitionRevisionRef::new(text("revision")?)?,
    );
    let crystal_unique_id = binding
        .get("legacy_unique_ids")
        .and_then(Value::as_array)
        .ok_or(bad.clone())?
        .iter()
        .find(|id| id.get("server").and_then(Value::as_str) == Some("crystalserver"))
        .map(|id| {
            id.get("unique_id")
                .and_then(Value::as_u64)
                .ok_or(bad.clone())
        })
        .transpose()?;
    let quest_transition = placement
        .get("quest_transition")
        .map(|key| key.as_str().map(str::to_owned).ok_or(bad.clone()))
        .transpose()?;
    let position = binding.get("project_position").ok_or(bad)?;
    if reward.get("key_binding").and_then(Value::as_str) == Some("crystalserver:door-key/3002")
        && (crystal_unique_id != Some(6093)
            || item.key().as_str() != "oteryn:item.tibia.i2969"
            || position != &serde_json::json!({"x":32802,"y":31576,"z":7}))
    {
        return Ok(None);
    }
    Ok(Some(CandidatePlacement {
        appearance,
        cell: native_cell(position),
        crystal_unique_id,
        item,
        count,
        quest_transition,
        key_number,
        achievement: None,
        contents: Vec::new(),
        root_text: None,
    }))
}

/// The native `(x, y, floor = -z)` of a project position (ADR-0021 §4.3), `None` when it has no
/// native cell.
fn native_cell(position: &Value) -> Option<BoundCell> {
    let coordinate = |axis| position.get(axis).and_then(Value::as_i64);
    let x = u16::try_from(coordinate("x")?).ok()?;
    let y = u16::try_from(coordinate("y")?).ok()?;
    let floor = i8::try_from(coordinate("z")?.checked_neg()?).ok()?;
    (-15..=0)
        .contains(&floor)
        .then_some(BoundCell { x, y, floor })
}

fn bind_candidate(
    base: &impl RewardClaimBase,
    key: &str,
    candidate: &Candidate,
    report: &mut BindingReport,
) -> Result<(), ClaimsError> {
    let local = key
        .strip_prefix("oteryn:")
        .ok_or(ClaimsError("claim key is not in the oteryn namespace"))?;
    let mut bound = Vec::with_capacity(candidate.placements.len());
    let mut unbound = Vec::new();
    for (index, placement) in candidate.placements.iter().enumerate() {
        match bind_placement(base, placement) {
            Ok((cell, bundle_placement_key)) => bound.push(BoundPlacement {
                placement: RewardClaimPlacement {
                    placement: PlacementKey::new(&format!("oteryn:placement/{local}/{index}"))?,
                    items: vec![RewardClaimItem {
                        item: placement.item.clone(),
                        count: placement.count,
                    }],
                    achievement: placement.achievement.clone(),
                    quest_transition: placement.quest_transition.clone(),
                    key_number: placement.key_number,
                },
                cell,
                bundle_placement_key,
                contents: placement.contents.clone(),
                root_text: placement.root_text.clone(),
            }),
            Err(reason) => unbound.push(UnboundPlacement {
                claim: key.to_owned(),
                index,
                reason,
            }),
        }
    }
    if unbound.is_empty() {
        report.served.push(ServedClaim {
            key: ProductionKey::new(key)?,
            revision: DefinitionRevisionRef::new(&candidate.revision)?,
            placements: bound,
        });
    } else {
        report.unbound.extend(unbound);
    }
    Ok(())
}

/// §1.4: the one top-level entry on the cell whose `unique` is the CrystalServer id and whose
/// palette appearance id is the placement's.
fn bind_placement(
    base: &impl RewardClaimBase,
    placement: &CandidatePlacement,
) -> Result<(BoundCell, u64), UnboundReason> {
    let id = placement
        .crystal_unique_id
        .ok_or(UnboundReason::NoCrystalUniqueId)?;
    let cell = placement.cell.ok_or(UnboundReason::CellOutOfBounds)?;
    let tile = base
        .tile(cell.x, cell.y, cell.floor)
        .ok_or(UnboundReason::NoEntry)?;
    let top_level = tile.depths().iter().filter(|depth| **depth == 0).count();
    let mut found = None;
    for ordinal in 0..top_level {
        let ordinal = u8::try_from(ordinal).map_err(|_| UnboundReason::NoEntry)?;
        let Some(entry) = tile.unique(ordinal) else {
            continue;
        };
        if u64::from(entry.unique) != id {
            continue;
        }
        if found.replace((ordinal, entry)).is_some() {
            return Err(UnboundReason::AmbiguousEntry);
        }
    }
    let (ordinal, entry) = found.ok_or(UnboundReason::NoEntry)?;
    if entry.appearance.map(u64::from) != Some(placement.appearance) {
        return Err(UnboundReason::AppearanceMismatch);
    }
    let key =
        bundle::placement_key(cell.floor, cell.x, cell.y, ordinal).ok_or(UnboundReason::NoEntry)?;
    Ok((cell, key))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod container_tests {
    use super::*;

    fn authored(suffix: &str) -> Option<Value> {
        for shard in REWARD_CLAIM_SHARDS {
            let document: Value = serde_json::from_str(shard).expect("embedded shard");
            for record in document["records"].as_array().expect("records") {
                if record["definition"]["identity"]["key"]
                    .as_str()
                    .is_some_and(|key| key.ends_with(suffix))
                {
                    return Some(record["definition"].clone());
                }
            }
        }
        None
    }

    #[test]
    fn container_projection_requires_every_exact_source_child_and_admitted_item() {
        let source = authored("kv.bear_chest1").expect("bear source");
        let parsed = candidate(source.as_object().expect("definition"), &|_| true)
            .expect("decoded")
            .expect("native container");
        assert_eq!(
            parsed.placements[0].item.key().as_str(),
            "oteryn:item.tibia.i2853"
        );
        assert_eq!(parsed.placements[0].contents.len(), 2);
        for mutation in [
            "missing child",
            "changed count",
            "changed root",
            "source digest",
        ] {
            let mut changed = source.clone();
            let recipe = &mut changed["native_contents"]["placements"][0];
            match mutation {
                "missing child" => {
                    recipe["contents"].as_array_mut().expect("contents").pop();
                }
                "changed count" => recipe["contents"][0]["count"] = 11.into(),
                "changed root" => recipe["root"]["key"] = "oteryn:item.tibia.i2854".into(),
                _ => changed["native_contents"]["source_sha256"] = "unproved".into(),
            }
            assert!(
                candidate(changed.as_object().expect("definition"), &|_| true)
                    .expect("decoded")
                    .is_err(),
                "{mutation}"
            );
        }
        assert!(matches!(
            candidate(source.as_object().expect("definition"), &|item| item
                .key()
                .as_str()
                != "oteryn:item.tibia.i3447"),
            Ok(Err(LeftOut::ItemNotAdmitted))
        ));
    }

    #[test]
    fn key_attributes_cannot_be_dropped_or_assigned_to_other_contents() {
        let source = authored("emperors_cookies.rewards.cookies").expect("key source");
        let parsed = candidate(source.as_object().expect("definition"), &|_| true)
            .expect("decoded")
            .expect("native key container");
        assert_eq!(parsed.placements[0].contents[0].key_number, Some(3801));
        assert_eq!(parsed.placements[0].contents[1].key_number, None);
        for index in [0, 1] {
            let mut changed = source.clone();
            let child = &mut changed["native_contents"]["placements"][0]["contents"][index];
            if index == 0 {
                child.as_object_mut().expect("child").remove("key_number");
            } else {
                child["key_number"] = 3801.into();
            }
            assert!(
                candidate(changed.as_object().expect("definition"), &|_| true)
                    .expect("decoded")
                    .is_err()
            );
        }
    }

    #[test]
    fn shared_storage_choices_keep_the_grant_and_all_four_placements() {
        let source = authored("the_annihilator.reward").expect("shared source");
        let parsed = candidate(source.as_object().expect("definition"), &|_| true)
            .expect("decoded")
            .expect("native shared choice");
        assert_eq!(parsed.placements.len(), 4);
        assert_eq!(
            parsed
                .placements
                .iter()
                .map(|p| p.contents.len())
                .collect::<Vec<_>>(),
            [0, 0, 0, 1]
        );
        assert!(
            parsed
                .placements
                .iter()
                .all(|p| p.achievement.as_deref() == Some("oteryn:achievement/annihilator"))
        );
        let mut changed = source;
        changed["placements"][3]
            .as_object_mut()
            .expect("placement")
            .remove("achievement_grant");
        assert!(
            candidate(changed.as_object().expect("definition"), &|_| true)
                .expect("decoded")
                .is_err()
        );
    }

    #[test]
    fn written_leaf_keeps_the_exact_normalized_text_and_carrier() {
        let source = authored("kv.powerbolt2").expect("written leaf");
        let parsed = candidate(source.as_object().expect("definition"), &|_| true)
            .expect("decoded")
            .expect("native written leaf");
        assert_eq!(
            parsed.placements[0].root_text.as_deref().map(str::len),
            Some(1505)
        );
        assert!(parsed.placements[0].contents.is_empty());
        for mutation in ["missing", "trailing LF", "changed", "carrier", "digest"] {
            let mut changed = source.clone();
            match mutation {
                "missing" => {
                    changed["native_contents"]["placements"][0]
                        .as_object_mut()
                        .expect("recipe")
                        .remove("root_text");
                }
                "trailing LF" => {
                    let text = changed["native_contents"]["placements"][0]["root_text"]
                        .as_str()
                        .expect("body")
                        .to_owned()
                        + "\n";
                    changed["native_contents"]["placements"][0]["root_text"] = text.into();
                }
                "changed" => {
                    changed["native_contents"]["placements"][0]["root_text"] = "invented".into()
                }
                "carrier" => {
                    changed["placements"][0]["reward"]["written_text"]["item"]["key"] =
                        "oteryn:item.tibia.i2816".into()
                }
                _ => {
                    changed["placements"][0]["reward"]["written_text"]["text_ref"]["sha256"] =
                        "invented".into()
                }
            }
            assert!(
                candidate(changed.as_object().expect("definition"), &|_| true)
                    .expect("decoded")
                    .is_err(),
                "{mutation}"
            );
        }
    }

    #[test]
    fn crystal_key3002_is_bound_only_to_its_exact_source_leaf() {
        let source = authored("quest.key.id3002").expect("Crystal key");
        let parsed = candidate(source.as_object().expect("definition"), &|_| true)
            .expect("decoded")
            .expect("native Crystal key");
        assert_eq!(parsed.placements[0].key_number, Some(3002));
        for mutation in ["number", "item", "quantity", "uid", "position"] {
            let mut changed = source.clone();
            let placement = &mut changed["placements"][0];
            match mutation {
                "number" => {
                    placement["reward"]["key_binding"] = "crystalserver:door-key/3003".into()
                }
                "item" => {
                    placement["reward"]["items"][0]["item"]["key"] =
                        "oteryn:item.tibia.i3031".into()
                }
                "quantity" => placement["reward"]["items"][0]["count"] = 2.into(),
                "uid" => {
                    for uid in placement["source_binding"]["legacy_unique_ids"]
                        .as_array_mut()
                        .expect("ids")
                    {
                        if uid["server"] == "crystalserver" {
                            uid["unique_id"] = 6094.into();
                        }
                    }
                }
                _ => placement["source_binding"]["project_position"]["x"] = 32803.into(),
            }
            assert!(
                candidate(changed.as_object().expect("definition"), &|_| true)
                    .expect("decoded")
                    .is_err(),
                "{mutation}"
            );
        }
    }

    #[test]
    fn authored_book_text_requires_exact_source_body_and_carrier() {
        let source = authored("dreamers_challenge.bplong_sword").expect("book source");
        let parsed = candidate(source.as_object().expect("definition"), &|_| true)
            .expect("decoded")
            .expect("native text container");
        let children = &parsed.placements[0].contents;
        assert_eq!(children.len(), 4);
        assert_eq!(children[0].text.as_deref().map(str::len), Some(478));
        assert!(children[1..].iter().all(|child| child.text.is_none()));
        for mutation in [
            "missing",
            "changed",
            "wrong carrier",
            "unbound text",
            "wrong digest",
        ] {
            let mut changed = source.clone();
            match mutation {
                "missing" => {
                    changed["native_contents"]["placements"][0]["contents"][0]
                        .as_object_mut()
                        .expect("child")
                        .remove("text");
                }
                "changed" => {
                    changed["native_contents"]["placements"][0]["contents"][0]["text"] =
                        "invented".into()
                }
                "wrong carrier" => {
                    let text = changed["native_contents"]["placements"][0]["contents"][0]
                        .as_object_mut()
                        .expect("child")
                        .remove("text")
                        .expect("body");
                    changed["native_contents"]["placements"][0]["contents"][1]["text"] = text;
                }
                "unbound text" => {
                    changed["placements"][0]["reward"]
                        .as_object_mut()
                        .expect("reward")
                        .remove("written_text");
                }
                _ => {
                    changed["placements"][0]["reward"]["written_text"]["text_ref"]["sha256"] =
                        "invented".into()
                }
            }
            assert!(
                candidate(changed.as_object().expect("definition"), &|_| true)
                    .expect("decoded")
                    .is_err(),
                "{mutation}"
            );
        }
    }
}

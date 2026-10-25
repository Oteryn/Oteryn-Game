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
/// variant shards. Of the variants, this reader serves only key rewards (KEY-1); every other
/// variant is left out.
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
const KEY_VARIANT_FIELDS: [&str; 4] = [
    "data_holds",
    "definition_profile",
    "native_admission",
    "source_variant",
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
    if key_variant && !waits_only_for_key_lowering(definition) {
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
    for placement in placements {
        match candidate_placement(placement, key_variant)? {
            Some(placement) => parsed.push(placement),
            None => all_encoded = false,
        }
    }
    if !all_encoded {
        return Ok(Err(LeftOut::UnencodedField));
    }
    if parsed.iter().any(|placement| !admitted(&placement.item)) {
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
    let digits = binding.strip_prefix(KEY_BINDING_PREFIX)?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u16>().ok().filter(|number| *number != 0)
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
    Ok(Some(CandidatePlacement {
        appearance,
        cell: native_cell(position),
        crystal_unique_id,
        item,
        count,
        quest_transition,
        key_number,
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
                    achievement: None,
                    quest_transition: placement.quest_transition.clone(),
                    key_number: placement.key_number,
                },
                cell,
                bundle_placement_key,
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

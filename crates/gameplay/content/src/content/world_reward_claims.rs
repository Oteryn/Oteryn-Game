//! CHEST-PLACE-BIND-1 (ARCH-WORLD-CONTENT-SERVE-1 §1.4, §1.5): the served set of RewardClaims and
//! the binding of each placement to exactly one entry of a World's base map.
//!
//! [`bind_reward_claims`] is a pure function of a [`RewardClaimBase`], the reward-claim shards and the
//! admitted Item registry. It writes nothing and returns the claims a World serves, with each
//! placement's canonical [`PlacementKey`], bound cell and bundle `placement_key`, plus the
//! placements that did not bind and the records left out, so the caller can gate boot on it.

mod native_state;

use crate::content::{
    ContentError, DefinitionFamily, DefinitionRevisionRef, PlacementKey, ProductionKey,
    RewardClaimItem, RewardClaimPlacement, TypedDefinitionRef,
};
use oteryn_world_bundle::bundle;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[path = "native_reward_claims.rs"]
mod native_authored;
#[path = "native_container_fluids.rs"]
mod native_container_fluids;
const CONTAINER_NATIVE_SOURCE_SHA256: &str =
    "b073d546930a60adf511677f932e6e0d37df3a230313dc85c4b9253742325757";

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
    /// Palette appearance of an exact top-level native entry, including entries without a UID.
    fn appearance(&self, ordinal: u8) -> Option<u16> {
        self.unique(ordinal)?.appearance
    }
}

mod native_aliases;

/// The RewardClaim shards, embedded as the quest document is: the plain shards, then the authored
/// variant shards. Keys and the exact W15b native container projection are served;
/// other source dispositions remain closed.
pub const REWARD_CLAIM_SHARDS: [&str; 6] = [
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
    include_str!("../../../../../content/interactions/reward_claims/engclaim-native.json"),
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
    /// QUEST-FINISH owner scope excludes Dawnport from the Newhaven runtime.
    OwnerExcluded,
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
    pub root_fluid_subtype: Option<u8>,
    pub root_charges_remaining: Option<u32>,
    /// Optional typed quest reward owner for native inventory exchanges/allocations.
    pub reward_callback: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardContainerItem {
    /// Initial fluid in one freshly minted variable-fluid vessel.
    pub fluid_subtype: Option<u8>,
    pub item: TypedDefinitionRef,
    pub count: u32,
    pub key_number: Option<u16>,
    pub text: Option<String>,
    pub charges_remaining: Option<u32>,
}

/// A claim whose native placements are all bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedClaim {
    pub key: ProductionKey,
    pub revision: DefinitionRevisionRef,
    pub authority_digest: [u8; 32],
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
    bind_reward_claims_with_aliases(
        base,
        &REWARD_CLAIM_SHARDS,
        admitted,
        native_aliases::load()?,
    )
}

/// The served set and binding report of `shards` over `base` (§1.4, §1.5). `admitted` says
/// whether an Item definition is admitted and materializable.
pub fn bind_reward_claims(
    base: &impl RewardClaimBase,
    shards: &[&str],
    admitted: &dyn Fn(&TypedDefinitionRef) -> bool,
) -> Result<BindingReport, ClaimsError> {
    bind_reward_claims_with_aliases(base, shards, admitted, &[])
}

fn bind_reward_claims_with_aliases(
    base: &impl RewardClaimBase,
    shards: &[&str],
    admitted: &dyn Fn(&TypedDefinitionRef) -> bool,
    aliases: &[native_aliases::NativePlacementAlias],
) -> Result<BindingReport, ClaimsError> {
    let mut report = BindingReport::default();
    let mut matched_aliases = std::collections::BTreeSet::new();
    let mut native_keys = BTreeSet::new();
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
            for (index, alias) in aliases
                .iter()
                .enumerate()
                .filter(|(_, alias)| alias.claim == key)
            {
                if !alias.validates(definition) {
                    return Err(ClaimsError(
                        "native alias does not match its original source placement",
                    ));
                }
                matched_aliases.insert(index);
            }
            if definition.get("definition_profile").and_then(Value::as_str)
                == Some("native_authored_v1")
            {
                native_keys.insert(key.to_owned());
            }
            match candidate(definition, admitted)? {
                Err(left_out) => report.left_out.push((key.to_owned(), left_out)),
                Ok(candidate) => {
                    let terms = serde_json::json!({"identity": definition.get("identity"), "claim": definition.get("claim"), "placements": definition.get("placements")});
                    let digest = Sha256::digest(
                        serde_json::to_vec(&terms).map_err(|_| ClaimsError("authority terms"))?,
                    )
                    .into();
                    bind_candidate(base, key, &candidate, digest, &mut report, aliases)?;
                }
            }
        }
    }
    // A native placement cannot acquire a second value authority under another
    // claim key (or an existing source claim). Refuse every affected whole claim.
    if !native_keys.is_empty() {
        let mut entries = BTreeMap::<u64, Vec<(usize, usize)>>::new();
        for (claim_index, claim) in report.served.iter().enumerate() {
            for (placement_index, placement) in claim.placements.iter().enumerate() {
                entries
                    .entry(placement.bundle_placement_key)
                    .or_default()
                    .push((claim_index, placement_index));
            }
        }
        let mut refused = BTreeSet::new();
        for bindings in entries.values().filter(|bindings| bindings.len() > 1) {
            if bindings
                .iter()
                .any(|(index, _)| native_keys.contains(report.served[*index].key.as_str()))
            {
                for &(index, placement) in bindings {
                    refused.insert(index);
                    report.unbound.push(UnboundPlacement {
                        claim: report.served[index].key.as_str().to_owned(),
                        index: placement,
                        reason: UnboundReason::AmbiguousEntry,
                    });
                }
            }
        }
        report.served = report
            .served
            .into_iter()
            .enumerate()
            .filter_map(|(index, claim)| (!refused.contains(&index)).then_some(claim))
            .collect();
    }
    if matched_aliases.len() != aliases.len() {
        return Err(ClaimsError("native alias names an unknown source claim"));
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
    native_ordinal: Option<u8>,
    extra_items: Vec<RewardClaimItem>,
    reward_callback: Option<String>,
    item: TypedDefinitionRef,
    count: u32,
    additional_items: Vec<RewardClaimItem>,
    quest_transition: Option<String>,
    key_number: Option<u16>,
    achievement: Option<String>,
    contents: Vec<RewardContainerItem>,
    root_text: Option<String>,
    root_fluid_subtype: Option<u8>,
    root_charges_remaining: Option<u32>,
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
    if definition
        .get("quest")
        .and_then(|quest| quest.get("key"))
        .and_then(Value::as_str)
        == Some("canary:quest/dawnport_quest")
    {
        return Ok(Err(LeftOut::OwnerExcluded));
    }
    if definition.get("definition_profile").and_then(Value::as_str) == Some("native_authored_v1") {
        return native_authored::candidate(definition, admitted);
    }
    let key_variant = definition.contains_key("definition_profile");
    let medusa = crystal_medusa_reconciliation(definition);
    let reconciled = native_state::reconciled(definition);
    let native_contents = native_container_placements(definition);
    if definition.contains_key("native_contents") && native_contents.is_none() {
        return Ok(Err(LeftOut::UnencodedField));
    }
    if key_variant && !waits_only_for_key_lowering(definition) && native_contents.is_none() {
        return Ok(Err(LeftOut::Variant));
    }
    if !key_variant
        && !reconciled
        && definition.get("readiness").and_then(Value::as_str) != Some("ready")
    {
        return Ok(Err(LeftOut::NotReady));
    }
    let whole = Value::Object(definition.clone());
    let claim = definition.get("claim");
    let encoded = definition.keys().all(|key| {
        DEFINITION_FIELDS.contains(&key.as_str())
            || (reconciled && key == "native_contents")
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
            Some(recipes) if medusa => {
                let mut native = placement.clone();
                native["reward"]
                    .as_object_mut()
                    .ok_or(ClaimsError("Medusa reward is malformed"))?
                    .remove("written_text");
                candidate_container_placement(&native, &recipes[index])?
            }
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
                .additional_items
                .iter()
                .any(|item| !admitted(&item.item))
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
fn crystal_medusa_reconciliation(definition: &serde_json::Map<String, Value>) -> bool {
    if definition
        .get("identity")
        .and_then(|value| value.get("key"))
        .and_then(Value::as_str)
        != Some("oteryn:reward-claim.quest.u7_3.the_medusa_quest")
    {
        return false;
    }
    // Crystal201189f chest.lua:2017-2024 and handler:42-49: the fox text
    // belongs to6179/book2815. Canary47dfd handler:40-47,119-123 attaches it
    // to all Medusa children. Only this complete, exact Crystal projection
    // resolves that SOURCE-only conflict; no other hold or carrier is erased.
    let item = |number: u16| {
        serde_json::json!({"family":"Item",
        "key":format!("oteryn:item.tibia.i{number}"),"revision":"definition-r1"})
    };
    let contents: Vec<_> = [3436, 3567, 3324]
        .into_iter()
        .map(|number| serde_json::json!({"item":item(number),"count":1}))
        .collect();
    let Some(projection) = definition.get("native_contents") else {
        return false;
    };
    projection.get("source_difference").and_then(Value::as_str)
        == Some("CRYSTAL_MEDUSA_WITHOUT_CANARY_TEXT")
        && definition.get("readiness").and_then(Value::as_str) == Some("waiting_data")
        && definition.get("data_holds")
            == Some(&serde_json::json!([
            {"category":"source","code":"SOURCE_WRITTEN_TEXT_CARRIER_UNRESOLVED","placement_index":0}]))
        && definition
            .get("source_variant")
            .and_then(|value| value.get("source_data"))
            == Some(&serde_json::json!({"status":"WAITING_SOURCE","missing":[
                {"code":"SOURCE_WRITTEN_TEXT_CARRIER_UNRESOLVED","placement_index":0}]}))
        && definition.get("placements")
            == Some(&serde_json::json!([{
            "appearance_tibia_id":1983,"source_binding":{
                "project_position":{"x":33049,"y":32399,"z":10},
                "legacy_unique_ids":[{"server":"canary","unique_id":6183},
                                     {"server":"crystalserver","unique_id":6183}]},
            "reward":{"container":item(2853),"items":contents,"written_text":{
                "item":null,"text_ref":{"length":70,"placeholders":[],
                    "sha256":"3d95b7c96da6a14a36118fbbd5a8397239e7e3cc9931c5f60f65d136bd72ff27"}}}}]))
        && projection.get("placements")
            == Some(&serde_json::json!([{
            "unique_id":6183,"line":2017,"root":item(2853),"capacity":8,"contents":contents,
            "completion":{"kind":"storage","key":"Storage.Quest.U7_3.TheMedusaQuest"}}]))
}

fn native_container_placements(definition: &serde_json::Map<String, Value>) -> Option<&Vec<Value>> {
    let medusa = crystal_medusa_reconciliation(definition);
    let reconciled = native_state::reconciled(definition);
    if !reconciled
        && (definition.get("definition_profile")?.as_str()? != "authored_variant_v1"
            || definition.get("native_admission")?.as_str()? != "WAITING_IMPLEMENTATION"
            || (!medusa
                && (definition.get("readiness")?.as_str()? != "waiting_implementation"
                    || !definition.get("data_holds")?.as_array()?.is_empty()
                    || definition
                        .get("source_variant")?
                        .get("source_data")?
                        .get("status")?
                        .as_str()?
                        != "COMPLETE")))
    {
        return None;
    }
    let projection = definition.get("native_contents")?;
    if !only(
        projection,
        &[
            "schema",
            "revision",
            "source_sha256",
            "placements",
            "source_difference",
        ],
    ) || projection.get("schema")?.as_str()? != "OTERYN_NATIVE_CONTAINER_REWARD/v1"
        || projection.get("revision")?.as_str()? != "201189f871d23597b82caf88fd611454a2ddefb8"
        || projection.get("source_sha256")?.as_str()? != CONTAINER_NATIVE_SOURCE_SHA256
        || (projection.get("source_difference").is_some() && !medusa && !reconciled)
    {
        return None;
    }
    let placements = projection.get("placements")?.as_array()?;
    if placements.is_empty()
        || (!reconciled
            && placements.iter().any(|recipe| {
                recipe.get("root_charges_remaining").is_some()
                    || recipe.get("root_fluid_subtype").is_some()
                    || recipe
                        .get("contents")
                        .and_then(Value::as_array)
                        .is_some_and(|children| {
                            children.iter().any(|child| {
                                child.get("fluid_subtype").is_some()
                                    || child.get("charges_remaining").is_some()
                            })
                        })
            }))
    {
        return None;
    }
    Some(placements)
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
            "root_charges_remaining",
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
            None if achievement.is_none()
                && (key_leaf
                    || recipe.get("root_charges_remaining").and_then(Value::as_u64) == Some(1)) =>
            {
                None
            }
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
        result.root_charges_remaining = recipe
            .get("root_charges_remaining")
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok());
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
    for (ordinal, (row, source)) in rows.iter().zip(source_rows).enumerate() {
        if !only(
            row,
            &[
                "item",
                "count",
                "key_number",
                "text",
                "fluid_subtype",
                "charges_remaining",
            ],
        ) || !only(source, &["item", "count"])
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
        // Exact source-selected state is checked by native_state before this
        // decoder is entered. Literal water additionally retains the raw donor
        // constructor proof; the selected milk/wine choices keep their own witness.
        let fluid_subtype = row
            .get("fluid_subtype")
            .and_then(Value::as_u64)
            .map(|n| u8::try_from(n).map_err(|_| bad.clone()))
            .transpose()?;
        if fluid_subtype.is_some() && (child.count != 1 || number.is_some() || text.is_some())
            || (fluid_subtype == Some(1)
                && native_container_fluids::source_subtype(row, result.crystal_unique_id, ordinal)
                    != Some(Some(1)))
        {
            return Ok(None);
        }
        result.contents.push(RewardContainerItem {
            fluid_subtype,
            item: child.item,
            count: child.count,
            key_number: number,
            text,
            charges_remaining: row
                .get("charges_remaining")
                .and_then(Value::as_u64)
                .map(|n| u32::try_from(n).map_err(|_| bad.clone()))
                .transpose()?,
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

/// Decode one plain reward leaf without discarding its source identity or quantity.
fn candidate_reward_item(entry: &Value) -> Result<Option<RewardClaimItem>, ClaimsError> {
    let bad = ClaimsError("reward item is not in the shard shape");
    if !only(entry, &["count", "item"]) {
        return Ok(None);
    }
    let count = entry
        .get("count")
        .and_then(Value::as_u64)
        .and_then(|count| u32::try_from(count).ok())
        .filter(|count| *count > 0)
        .ok_or(bad.clone())?;
    let item = entry.get("item").ok_or(bad.clone())?;
    let text = |field: &str| item.get(field).and_then(Value::as_str).ok_or(bad.clone());
    if text("family")? != "Item" {
        return Ok(None);
    }
    Ok(Some(RewardClaimItem {
        count,
        item: TypedDefinitionRef::new(
            DefinitionFamily::Item,
            ProductionKey::new(text("key")?)?,
            DefinitionRevisionRef::new(text("revision")?)?,
        ),
    }))
}

/// A key variant remains a single key; plain rewards admit a bounded item vector.
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
    if !only(
        binding,
        &["legacy_unique_ids", "project_position", "native_ordinal"],
    ) || !only(reward, reward_fields)
    {
        return Ok(None);
    }
    // Authoring must select exactly one namespace; a missing source UID never
    // silently becomes an authored selector.
    let native_ordinal = match binding.get("native_ordinal") {
        Some(value) => {
            if binding.get("legacy_unique_ids").is_some() {
                return Ok(None);
            }
            let Some(ordinal) = value.as_u64().and_then(|n| u8::try_from(n).ok()) else {
                return Ok(None);
            };
            Some(ordinal)
        }
        None => None,
    };
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
    let Some((entry, rest)) = items.split_first() else {
        return Ok(None);
    };
    if items.len() > 20 || (key_variant && !rest.is_empty()) {
        return Ok(None);
    }
    let Some(first) = candidate_reward_item(entry)? else {
        return Ok(None);
    };
    let item = first.item;
    let count = first.count;
    if key_number.is_some() && count != 1 {
        return Ok(None);
    }
    let mut additional_items = Vec::with_capacity(rest.len());
    for entry in rest {
        let Some(item) = candidate_reward_item(entry)? else {
            return Ok(None);
        };
        additional_items.push(item);
    }
    let crystal_unique_id = if native_ordinal.is_some() {
        None
    } else {
        binding
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
            .transpose()?
    };
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
        native_ordinal,
        extra_items: Vec::new(),
        reward_callback: None,
        item,
        count,
        additional_items,
        quest_transition,
        key_number,
        achievement: None,
        contents: Vec::new(),
        root_text: None,
        root_fluid_subtype: None,
        root_charges_remaining: None,
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
    authority_digest: [u8; 32],
    report: &mut BindingReport,
    aliases: &[native_aliases::NativePlacementAlias],
) -> Result<(), ClaimsError> {
    let local = key
        .strip_prefix("oteryn:")
        .ok_or(ClaimsError("claim key is not in the oteryn namespace"))?;
    let mut bound = Vec::with_capacity(candidate.placements.len());
    let mut unbound = Vec::new();
    for (index, placement) in candidate.placements.iter().enumerate() {
        let binding = match bind_placement(base, placement) {
            Err(UnboundReason::NoEntry) => {
                if let Some(alias) = aliases.iter().find(|alias| alias.applies(key, placement)) {
                    alias.bind(base, placement)
                } else {
                    Err(UnboundReason::NoEntry)
                }
            }
            other => other,
        };
        match binding {
            Ok((cell, bundle_placement_key)) => bound.push(BoundPlacement {
                placement: RewardClaimPlacement {
                    placement: PlacementKey::new(&format!("oteryn:placement/{local}/{index}"))?,
                    items: std::iter::once(RewardClaimItem {
                        item: placement.item.clone(),
                        count: placement.count,
                    })
                    .chain(placement.extra_items.iter().cloned())
                    .chain(placement.additional_items.iter().cloned())
                    .collect(),
                    achievement: placement.achievement.clone(),
                    quest_transition: placement.quest_transition.clone(),
                    key_number: placement.key_number,
                },
                cell,
                bundle_placement_key,
                contents: placement.contents.clone(),
                root_text: placement.root_text.clone(),
                root_fluid_subtype: placement.root_fluid_subtype,
                root_charges_remaining: placement.root_charges_remaining,
                reward_callback: placement.reward_callback.clone(),
            }),
            Err(reason) => unbound.push(UnboundPlacement {
                claim: key.to_owned(),
                index,
                reason,
            }),
        }
    }
    // Foreign-only selectors stay unbound without suppressing the claim's
    // native sources. Every native selector must still bind successfully.
    if !bound.is_empty()
        && unbound
            .iter()
            .all(|placement| placement.reason == UnboundReason::NoCrystalUniqueId)
    {
        report.served.push(ServedClaim {
            key: ProductionKey::new(key)?,
            revision: DefinitionRevisionRef::new(&candidate.revision)?,
            authority_digest,
            placements: bound,
        });
    }
    report.unbound.extend(unbound);
    Ok(())
}

/// Resolve one top-level entry by its source UID or explicit authored ordinal.
/// Both selectors require the declared cell and palette appearance.
fn bind_placement(
    base: &impl RewardClaimBase,
    placement: &CandidatePlacement,
) -> Result<(BoundCell, u64), UnboundReason> {
    if placement.native_ordinal.is_none() && placement.crystal_unique_id.is_none() {
        return Err(UnboundReason::NoCrystalUniqueId);
    }
    let cell = placement.cell.ok_or(UnboundReason::CellOutOfBounds)?;
    let tile = base
        .tile(cell.x, cell.y, cell.floor)
        .ok_or(UnboundReason::NoEntry)?;
    let top_level = tile.depths().iter().filter(|depth| **depth == 0).count();
    if let Some(ordinal) = placement.native_ordinal {
        if usize::from(ordinal) >= top_level {
            return Err(UnboundReason::NoEntry);
        }
        if tile.appearance(ordinal).map(u64::from) != Some(placement.appearance) {
            return Err(UnboundReason::AppearanceMismatch);
        }
        let key = bundle::placement_key(cell.floor, cell.x, cell.y, ordinal)
            .ok_or(UnboundReason::NoEntry)?;
        return Ok((cell, key));
    }
    let id = placement
        .crystal_unique_id
        .ok_or(UnboundReason::NoCrystalUniqueId)?;
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

    #[test]
    fn source_charge_and_fluid_projections_reach_the_native_candidate() {
        for suffix in [
            "kv.deeperfibula4",
            "quest.u7_2.the_queen_of_the_banshees.reward.stone_skin_amulet",
            "quest.u15_24.targuna.silver_amulet_chest",
            "quest.pre_u6_0.ornamented_shield.rewards.ornamented_shield",
            "kv.ornamentedshield",
            "kv.goblintemple2",
            "quest.u7_9.dreamers_challenge.chests_wine",
            "kv.present_box",
        ] {
            let source = authored(suffix).expect("source recipe");
            let parsed = candidate(source.as_object().expect("definition"), &|_| true)
                .expect("decoded")
                .expect(suffix);
            if suffix == "kv.ornamentedshield" {
                // The source instance has one charge; the Item definition stays at 150.
                let necklace = parsed.placements[0]
                    .contents
                    .iter()
                    .find(|c| c.item.key().as_str() == "oteryn:item.tibia.i3083")
                    .expect("exact necklace");
                assert_eq!(necklace.count, 1);
                assert_eq!(necklace.charges_remaining, Some(1));
                let profiles: Value = serde_json::from_slice(include_bytes!(
                    "../../../../../content/items/definitions/spell-native-profiles.json"
                ))
                .expect("pinned Item facts");
                let item = profiles["records"]
                    .as_array()
                    .expect("profiles")
                    .iter()
                    .find(|r| {
                        r["production_definition"]["production_key"] == "oteryn:item.tibia.i3083"
                    })
                    .expect("exact source Item");
                assert_eq!(
                    item["semantics"]["charges"],
                    serde_json::json!({"state":"KNOWN","value":{"count":{"state":"KNOWN","value":150}}})
                );
            } else {
                assert!(
                    parsed
                        .placements
                        .iter()
                        .all(|placement| placement.root_charges_remaining == Some(1)
                            || placement
                                .contents
                                .iter()
                                .any(|child| child.charges_remaining == Some(1)
                                    || child.fluid_subtype.is_some())),
                    "{suffix}: explicit instance state must survive projection"
                );
            }
            let mut changed = source.clone();
            changed["native_contents"]
                .as_object_mut()
                .expect("projection")
                .remove("source_difference");
            if suffix == "kv.ornamentedshield" {
                changed["native_contents"]["placements"][0]["contents"][2]["charges_remaining"] =
                    1.into();
                changed["data_holds"] = serde_json::json!([{"category":"source","code":"SOURCE_CHARGE_MISMATCH","placement_index":0,"reward_field":"items","reward_index":2}]);
            }
            assert!(
                candidate(changed.as_object().expect("definition"), &|_| true)
                    .expect("decoded")
                    .is_err(),
                "{suffix}: source choice is mandatory"
            );
            for field in ["schema", "source_sha256"] {
                let mut changed = source.clone();
                changed["native_contents"][field] = "invalid".into();
                assert!(
                    candidate(changed.as_object().expect("definition"), &|_| true)
                        .expect("decoded")
                        .is_err(),
                    "{suffix}: invalid {field} must not fall back to default state"
                );
            }
        }
    }

    #[test]
    fn ornamented_shield_preserves_the_exact_source_charge_choice() {
        // Crystal201189f chest UID6063 passes raw1: one charge, while the
        // independent Item definition retains its full 150-charge default.
        let source = authored("kv.ornamentedshield").expect("Ornamented Shield source");
        let parsed = candidate(source.as_object().expect("definition"), &|_| true)
            .expect("decoded")
            .expect("exact source-charge recipe");
        assert_eq!(parsed.placements.len(), 1);
        let child = &parsed.placements[0].contents[2];
        assert_eq!(child.item.key().as_str(), "oteryn:item.tibia.i3083");
        assert_eq!(child.count, 1);
        assert_eq!(child.charges_remaining, Some(1));
        assert_eq!(
            source["native_contents"]["source_difference"],
            "EXACT_SOURCE_CHARGES_NOT_DEFINITION_DEFAULTS"
        );
        for field in ["schema", "source_sha256"] {
            let mut changed = source.clone();
            changed["native_contents"][field] = "invalid".into();
            assert!(
                candidate(changed.as_object().expect("definition"), &|_| true)
                    .expect("decoded")
                    .is_err(),
                "invalid {field}"
            );
        }
    }

    #[test]
    fn medusa_uses_exact_crystal_bag_without_canary_text_carrier_guess() {
        let source = authored("quest.u7_3.the_medusa_quest").expect("Medusa source");
        let parsed = candidate(source.as_object().expect("definition"), &|_| true)
            .expect("decoded")
            .expect("complete Crystal bag");
        assert_eq!(parsed.placements[0].contents.len(), 3);
        for mutation in ["sibling hold", "carrier", "count", "difference"] {
            let mut changed = source.clone();
            match mutation {
                "sibling hold" => changed["data_holds"]
                    .as_array_mut()
                    .expect("holds")
                    .push(serde_json::json!({"category":"item","code":"ITEM_SEMANTICS_MISSING"})),
                "carrier" => {
                    changed["placements"][0]["reward"]["written_text"]["item"] =
                        changed["placements"][0]["reward"]["items"][0]["item"].clone()
                }
                "count" => changed["placements"][0]["reward"]["items"][0]["count"] = 2.into(),
                "difference" => changed["native_contents"]["source_difference"] = "guessed".into(),
                _ => unreachable!(),
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
    fn owner_scope_excludes_dawnport_rewards_even_when_items_are_admitted() {
        for suffix in ["quest.key.id0010", "quest.u10_55.dawnport.torn_log_book"] {
            let definition = authored(suffix).expect("Dawnport source claim retained");
            assert!(
                candidate(definition.as_object().expect("definition"), &|_| true)
                    .expect("well-formed source claim")
                    .is_err(),
                "{suffix}"
            );
        }
    }

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
    fn multiple_top_level_rewards_retain_every_source_item() {
        let source = authored("kv.bear_chest1").expect("source fixture");
        let mut placement = source["placements"][0].clone();
        // Exercise the plain, top-level reward shape rather than a container.
        placement["reward"]
            .as_object_mut()
            .expect("reward")
            .remove("container");
        let parsed = candidate_placement(&placement, false)
            .expect("valid reward shape")
            .expect("both top-level items must be encoded");
        assert_eq!(parsed.item.key().as_str(), "oteryn:item.tibia.i3447");
        assert_eq!(parsed.count, 12);
        assert_eq!(parsed.additional_items.len(), 1);
        assert_eq!(
            parsed.additional_items[0].item.key().as_str(),
            "oteryn:item.tibia.i3031"
        );
        assert_eq!(parsed.additional_items[0].count, 40);
        placement["reward"]["items"][1]["count"] = serde_json::json!(0);
        assert!(candidate_placement(&placement, false).is_err());
    }

    #[test]
    fn top_level_reward_vectors_are_nonempty_and_bounded() {
        let source = authored("kv.bear_chest1").expect("source fixture");
        let mut placement = source["placements"][0].clone();
        placement["reward"]
            .as_object_mut()
            .expect("reward")
            .remove("container");
        let item = placement["reward"]["items"][0].clone();
        for count in [0, 1, 20, 21] {
            placement["reward"]["items"] = Value::Array(vec![item.clone(); count]);
            assert_eq!(
                candidate_placement(&placement, false)
                    .expect("valid shape")
                    .is_some(),
                (1..=20).contains(&count),
                "{count} reward items"
            );
        }
    }

    #[test]
    fn itemhold_q_charge_choices_reach_the_native_reader() {
        let ornament = authored("kv.ornamentedshield").expect("Ornamented Shield source");
        let ornament = candidate(ornament.as_object().expect("definition"), &|_| true)
            .expect("decoded")
            .expect("qualified native container");
        let necklace = ornament.placements[0]
            .contents
            .iter()
            .find(|row| row.item.key().as_str() == "oteryn:item.tibia.i3083")
            .expect("one necklace with the exact source charge state");
        assert_eq!(necklace.count, 1);
        assert_eq!(necklace.charges_remaining, Some(1));
        let power = authored("kv.powerring2").expect("Power Ring source");
        let power = candidate(power.as_object().expect("definition"), &|_| true)
            .expect("decoded")
            .expect("qualified full-charge leaf");
        assert_eq!(
            power.placements[0].item.key().as_str(),
            "oteryn:item.tibia.i3056"
        );
        assert_eq!(power.placements[0].count, 1);
        assert_eq!(power.placements[0].root_charges_remaining, None);
    }

    #[test]
    fn source_selected_fluid_projections_require_the_exact_subtype() -> Result<(), String> {
        for (key, child_index, count, subtype) in [
            ("quest.u7_9.dreamers_challenge.chests_wine", 1, 4, 2),
            ("kv.goblintemple2", 2, 1, 9),
        ] {
            let source = authored(key).ok_or("source-selected fluid claim")?;
            let parsed = candidate(source.as_object().ok_or("definition")?, &|_| true)
                .map_err(|e| format!("{e:?}"))?
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(parsed.placements.len(), count);
            for placement in &parsed.placements {
                assert_eq!(placement.contents.len(), 3);
                assert_eq!(placement.contents[child_index].fluid_subtype, Some(subtype));
            }
            for subtype in [None, Some(0), Some(1), Some(15)] {
                let mut changed = source.clone();
                let child =
                    &mut changed["native_contents"]["placements"][0]["contents"][child_index];
                if let Some(subtype) = subtype {
                    child["fluid_subtype"] = serde_json::json!(subtype);
                } else {
                    child
                        .as_object_mut()
                        .ok_or("child")?
                        .remove("fluid_subtype");
                }
                assert!(
                    candidate(changed.as_object().ok_or("definition")?, &|_| true)
                        .map_err(|e| format!("{e:?}"))?
                        .is_err()
                );
            }
        }
        Ok(())
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

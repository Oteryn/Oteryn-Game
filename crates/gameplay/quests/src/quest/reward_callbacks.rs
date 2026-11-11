//! Bounded NPC exchanges composed by the fenced quest writer, never by dialogue text.
use super::world_callbacks::Storage;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod capture_tests;
mod carried;
mod carried_bounds;
#[cfg(test)]
mod carried_exchange_tests;
mod colours_peer;
mod corpse;
mod inventory;
mod published_markers;
mod questpc7_registry_bounds;
#[cfg(test)]
mod questpc8_exchange_tests;
pub use corpse::CorpseTarget;
#[cfg(test)]
mod corpse_tests;
#[cfg(test)]
mod peer_tests;
#[cfg(test)]
mod qready1_craft_tests;
#[cfg(test)]
mod recipe_tests;
mod state_ops;
mod temporal_binding;
pub use inventory::InventoryRequirement;
#[cfg(test)]
mod inventory_tests;
mod temporal_payment;
#[cfg(test)]
mod temporal_payment_tests;
pub use state_ops::{StateCondition, StateMutation, StateOperation};

fn is_false(value: &bool) -> bool {
    !*value
}

pub const PROVENANCE: &str = "crystal-9f5a72c-quest-rewards-v1";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemAmount {
    pub item: String,
    pub count: u16,
    /// A freshly minted variable-fluid vessel; never an intake matching shortcut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fluid_subtype: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemReward {
    /// Recheck the live account gate before a fresh value grant.
    #[serde(default, skip_serializing_if = "is_false")]
    pub premium: bool,
    pub item: String,
    pub count: u16,
    #[serde(default)]
    pub contents: Vec<ItemAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_number: Option<u16>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backpack_outputs: Vec<ItemAmount>,
    /// Literal initial text on the root or its ordered container children.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<RewardText>,
    /// Independent top-level items committed in the same quest receipt.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional: Vec<ItemAmount>,
}

impl ItemReward {
    /// Validate all initial text before constructing a durable MINT plan.
    pub fn valid_literal_text(&self) -> bool {
        self.text
            .as_ref()
            .is_none_or(|text| text.valid_for(self.count, &self.contents))
    }
}

mod reward_text;
pub use reward_text::{RewardContentText, RewardText};

#[cfg(test)]
mod literal_text_tests;
#[cfg(test)]
mod native_claim_tests;
#[cfg(test)]
mod reward_contents_text_tests;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutfitReward {
    pub outfit: String,
    /// Base entitlement is always included. Bits 1 and 2 earn the respective addons.
    pub addons: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sex: Option<CreationSex>,
}

/// Exact source price; only the durable fee owner may consume the funds.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonetaryFee {
    pub gold: u64,
    pub include_bank: bool,
}

/// Immutable creation-sex eligibility, independent of selected appearance.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CreationSex {
    Female,
    Male,
}

/// A bounded, source-authored write through another quest's existing owner.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QuestParticipant {
    pub quest: String,
    pub storage: Vec<Storage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_ops: Vec<StateOperation>,
}

impl QuestParticipant {
    pub fn tracks(&self) -> impl Iterator<Item = &str> {
        self.storage
            .iter()
            .map(|s| s.track.as_str())
            .chain(self.state_ops.iter().map(|s| s.track.as_str()))
    }
}

/// A read-only prerequisite: its original quest retains ownership and writes.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TrackRequirement {
    pub track: String,
    pub minimum: i64,
    pub maximum: i64,
}

impl TrackRequirement {
    pub fn holds(&self, catalogue: &QuestStateCatalogue, stored: &BTreeMap<String, i64>) -> bool {
        catalogue.track(&self.track).is_some_and(|track| {
            QuestComparison::Between(self.minimum, self.maximum)
                .holds(stored.get(&self.track).copied().unwrap_or(track.initial), 0)
        })
    }
}

/// Current native actor conditions admitted for one source command.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ActorConditionRequirement {
    Fire,
    Drunk,
}

pub use super::map_marks::{MapMarkKind as QuestMapMarkIcon, QuestMapMark};

/// Inclusive native bounds for an exact self-target carried quest action.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelfTarget {
    pub minimum: super::world_callbacks::Position,
    pub maximum: super::world_callbacks::Position,
}

impl SelfTarget {
    pub fn contains(&self, x: i32, y: i32, floor: i16) -> bool {
        floor == -i16::from(self.minimum.floor)
            && (i32::from(self.minimum.x)..=i32::from(self.maximum.x)).contains(&x)
            && (i32::from(self.minimum.y)..=i32::from(self.maximum.y)).contains(&y)
    }

    fn valid(&self) -> bool {
        self.minimum.floor == self.maximum.floor
            && (-15..=0).contains(&self.minimum.floor)
            && self.minimum.x <= self.maximum.x
            && self.minimum.y <= self.maximum.y
            && self.maximum.x - self.minimum.x <= 16
            && self.maximum.y - self.minimum.y <= 16
    }
}

/// One paid source occurrence advances the existing Character achievement owner.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AchievementProgress {
    pub key: String,
    pub target: u32,
}

/// Immutable evidence survives a source track being reset for another task run.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct ReceiptPredicate {
    pub transition: String,
    pub track: String,
    pub value: i64,
}

/// Immutable native RewardClaim authority. The shared map binder proves the
/// semantic digest before the physical USE owner may call the reward writer.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeClaimBinding {
    pub key: String,
    pub revision: String,
    pub authority_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardCallback {
    #[serde(default, skip_serializing_if = "is_false")]
    pub completes: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub repeatable: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_claims: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counter: Option<RewardCounter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirements: Option<RewardRequirements>,
    /// Personal display fact created atomically with this dialogue progress receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_mark: Option<super::map_marks::QuestMapMark>,
    pub key: String,
    pub quest: String,
    pub npc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_claim: Option<NativeClaimBinding>,
    /// Ordinary carried USE binding; NPC dialogue cannot execute this callback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carried_item: Option<String>,
    /// Explicit selected-instance exchange; legacy tools retain their no-grant contract.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub carried_exchange: bool,
    /// Exact second carried operand; its selected instance must accompany the ingress.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carried_target: Option<String>,
    /// Read-only condition on the selected other live player, checked atomically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer_requires: Option<TrackRequirement>,
    /// Exact native species; live unless a source corpse predicate is provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creature_target: Option<String>,
    /// Exact current self entity, inside the source-authored native area.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_target: Option<SelfTarget>,
    /// Read-only source corpse sampling retains its native decay and contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corpse_target: Option<CorpseTarget>,
    /// Remove the exact living target only after its tool/progress receipt commits.
    #[serde(default, skip_serializing_if = "is_false")]
    pub capture_creature: bool,
    /// Sample the exact original fresh Ground corpse instead of a live creature.
    #[serde(default, skip_serializing_if = "is_false")]
    pub fresh_corpse: bool,
    pub keywords: Vec<String>,
    pub topic: u16,
    pub next_topic: u16,
    /// Complete this owning quest in the same receipt as its final dialogue.
    pub storage: Vec<Storage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_ops: Vec<StateOperation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<QuestParticipant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_sex: Option<CreationSex>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub requires_premium: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires_completed_quests: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires_outfits: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires_addons: Vec<OutfitReward>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forbids_outfits: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires_actor_conditions: Vec<ActorConditionRequirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forbids_addons: Vec<OutfitReward>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<TrackRequirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_receipts: Vec<ReceiptPredicate>,
    pub take: Vec<ItemAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee: Option<MonetaryFee>,
    /// Carried evidence checked under the same locks, without consuming it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present: Vec<ItemAmount>,
    /// Inclusive carried-quantity guards, rechecked before a fresh reward write.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inventory: Vec<InventoryRequirement>,
    pub give: Option<ItemReward>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub per_occurrence_mint: bool,
    pub outfit: Option<OutfitReward>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outfits: Vec<OutfitReward>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exact_take: bool,
    pub achievement: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub achievement_progress: Vec<AchievementProgress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub magic_effects: Vec<super::npc_magic::Cue>,
    pub reply: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failure_reply: Vec<String>,
    pub sources: Vec<String>,
}

impl RewardCallback {
    /// Syntax-only reset shape. Current save authority is resolved by the writer;
    /// the catalogue separately validates owned tracks and their initial values.
    pub(crate) fn is_save_restart_only(&self) -> bool {
        if self.key != super::shared_world_progress::save_event_restart_key(&self.quest)
            || self.storage.is_empty()
            || self
                .storage
                .iter()
                .any(|state| state.preserve || state.increment || state.at_least)
        {
            return false;
        }
        let pure = serde_json::from_value::<Self>(serde_json::json!({
            "key":self.key,"quest":self.quest,"npc":self.npc,"keywords":self.keywords,
            "topic":self.topic,"next_topic":self.next_topic,"storage":self.storage,
            "take":[],"give":null,"outfit":null,"achievement":null,
            "reply":self.reply,"sources":self.sources
        }));
        pure.is_ok_and(|callback| callback == *self)
    }
}

#[cfg(test)]
mod fresh_corpse_tests;
#[cfg(test)]
mod ice_paint_tests;

impl OutfitReward {
    pub fn grant_key(&self, addon: u8) -> String {
        match self.sex {
            None => format!("{}/quest-grant/{addon}", self.outfit),
            Some(sex) => format!(
                "{}/quest-grant/{}/{addon}",
                self.outfit,
                match sex {
                    CreationSex::Female => "female",
                    CreationSex::Male => "male",
                }
            ),
        }
    }
}
impl RewardCallback {
    fn valid_carried_inputs(&self) -> bool {
        let selected = |key: &str| {
            self.take
                .iter()
                .any(|input| input.item == key && input.count == 1 && input.fluid_subtype.is_none())
        };
        match (&self.carried_item, &self.carried_target) {
            (Some(source), Some(target)) => {
                source != target
                    && selected(source)
                    && selected(target)
                    && self.give.is_some()
                    && self.peer_requires.is_none()
                    && self.creature_target.is_none()
            }
            (None, Some(_)) => false,
            (Some(source), None) => {
                matches!(self.take.as_slice(), [input]
                if input.item == *source && input.count == 1 && input.fluid_subtype.is_none())
                    && (self.give.is_none() || self.creature_target.is_some())
            }
            (None, None) => true,
        }
    }
    pub fn cosmetic_targets(&self) -> impl Iterator<Item = &OutfitReward> {
        self.outfit.iter().chain(&self.outfits)
    }
    /// Paid exchanges and exact carried recipes repeat under distinct CommandRefs.
    pub fn is_paid_exchange(&self) -> bool {
        let guarded_sale = self.repeatable
            && !self.completes
            && self.carried_item.is_none()
            && self.world_claim.is_none()
            && self.state_ops.is_empty()
            && self.storage.iter().all(|state| state.preserve);
        ((self.storage.is_empty() && self.state_ops.is_empty())
            || self.carried_target.is_some()
            || guarded_sale)
            && self.participants.is_empty()
            && !self.take.is_empty()
            && self.give.is_some()
    }
    fn value_only_guarded(&self) -> bool {
        // Repeatable source trades spend exact admitted inputs and produce a
        // native backpack MINT in the same fenced transaction.
        temporal_payment::guarded(self)
            || (self.repeatable
                && (!self.take.is_empty() || self.fee.is_some())
                && (self.give.is_some() || self.experience.is_some()))
            || (!self.take.is_empty() && self.give.is_some())
            || self.cosmetic_targets().any(|target| {
                (target.addons == 0
                    && target.sex.is_none()
                    && self.forbids_outfits.contains(&target.outfit))
                    || self.forbids_addons.iter().any(|guard| {
                        target.outfit == guard.outfit
                            && target.addons & guard.addons != 0
                            && (target.sex.is_none()
                                || target.sex == guard.sex.or(self.creation_sex))
                    })
            })
    }
}

/// Source-bound track metadata; writable declarations name the exact existing owner.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackDeclaration {
    #[serde(default)]
    writable: bool,
    key: String,
    quest: String,
    initial: i64,
    min: i64,
    max: i64,
    sources: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    #[serde(default)]
    item_disguises: Vec<super::item_disguises::Rule>,
    #[serde(default)]
    tracks: Vec<TrackDeclaration>,
    callbacks: Vec<RewardCallback>,
}

const DOCUMENTS: &[&str] = &[
    include_str!("../../../../../content/quests/shared-reward-callbacks.json"),
    include_str!("../../../../../content/quests/npc-callbacks/fibula.json"),
    include_str!("../../../../../content/quests/npc-callbacks/l2-ready-stories.json"),
    include_str!("../../../../../content/quests/npc-callbacks/l6-stories.json"),
    include_str!("../../../../../content/quests/npc-callbacks/l13-outfits.json"),
    include_str!("../../../../../content/quests/npc-callbacks/questpc7-programmes.json"),
    include_str!("../../../../../content/quests/chosen-reward-callbacks.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m13-guardcatcher.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m09-rookie.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m13-rotworm-sample.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m13-goblin-capture.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m16-targuna-marker.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m20-cursed-crystal-recipes.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m56-cursed-crystal-wax.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m39-stories.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m13-slug-sample.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m13-siflind-paint.json"),
    // Keep the shared document at DOCUMENTS[0]; Oramond is the final extension.
    include_str!("../../../../../content/quests/npc-callbacks/oramond-native-roots.json"),
    include_str!("../../../../../content/quests/npc-callbacks/qnpcfx-m10.json"),
    include_str!("../../../../../content/quests/npc-callbacks/qnpcfx-koshei.json"),
    include_str!("../../../../../content/quests/native-reward-callbacks/engclaim.json"),
];

/// The current immutable catalogue's explicit authored callbacks plus the retained source owners.
pub fn load_for(catalogue: &QuestStateCatalogue) -> Result<Vec<RewardCallback>, String> {
    let mut callbacks = load()?;
    if let Some(bindings) = catalogue.authored_bindings() {
        callbacks.extend(bindings.reward_callbacks().cloned());
    }
    if callbacks.len() > super::authored_bindings::MAX_BOUND_STAGES
        || callbacks
            .iter()
            .map(|c| &c.key)
            .collect::<BTreeSet<_>>()
            .len()
            != callbacks.len()
    {
        return Err("authored/base reward binding collision or bound".into());
    }
    Ok(callbacks)
}

/// NPC offer selection excludes donor-unavailable missions. Raw callbacks remain
/// available to immutable receipt reconciliation and catalogue validation.
pub fn offered_for(catalogue: &QuestStateCatalogue) -> Result<Vec<RewardCallback>, String> {
    let mut callbacks = load_for(catalogue)?;
    callbacks.retain(super::authored_bindings::personal_event_offered);
    Ok(callbacks)
}

pub fn load() -> Result<Vec<RewardCallback>, String> {
    embedded_document().map(|document| document.callbacks)
}

fn embedded_document() -> Result<Document, String> {
    let mut document = parse_documents(DOCUMENTS)?;
    document
        .callbacks
        .extend(super::world_change_journals::npc_callbacks()?);
    validate_document_with_capacity(document, 256 * DOCUMENTS.len())
}

pub(super) fn item_disguises() -> Result<Vec<super::item_disguises::Rule>, String> {
    parse_documents(DOCUMENTS).map(|document| document.item_disguises)
}

fn parse_documents(sources: &[&str]) -> Result<Document, String> {
    let maximum = questpc7_registry_bounds::combined_capacity(sources.len())?;
    let mut combined = Document {
        schema: 1,
        item_disguises: Vec::new(),
        tracks: Vec::new(),
        callbacks: Vec::new(),
    };
    if sources.is_empty() || sources.len() > DOCUMENTS.len() {
        return Err("reward callback source bounds".into());
    }
    for source in sources {
        let document: Document = serde_json::from_str(source).map_err(|e| e.to_string())?;
        if document.callbacks.len() > 256 || document.tracks.len() > 256 {
            return Err("reward callback bounds".into());
        }
        if document.schema != 1 {
            return Err("reward callback schema".into());
        }
        if document.callbacks.len() > questpc7_registry_bounds::PER_DOCUMENT
            || document.tracks.len() > questpc7_registry_bounds::PER_DOCUMENT
        {
            return Err("reward callback bounds".into());
        }
        combined.item_disguises.extend(document.item_disguises);
        combined.tracks.extend(document.tracks);
        combined.callbacks.extend(document.callbacks);
    }
    colours_peer::append_after_opening(&mut combined)?;
    validate_document_with_capacity(combined, maximum)
}

pub(crate) fn validate_callbacks(callbacks: &[RewardCallback]) -> Result<(), String> {
    parse(&serde_json::json!({"schema":1,"callbacks":callbacks}).to_string()).map(|_| ())
}

pub(crate) fn valid_item_reward(reward: &ItemReward) -> bool {
    let item = |key: &str, count: u16| {
        key.len() <= 256
            && key.strip_prefix("oteryn:item.").is_some_and(|tail| {
                !tail.is_empty()
                    && tail.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"._/:-".contains(&b)
                    })
            })
            && (1..=100).contains(&count)
    };
    item(&reward.item, reward.count)
        && reward.valid_literal_text()
        && reward.additional.len() <= 19
        && (reward.additional.is_empty() || reward.backpack_outputs.is_empty())
        && reward
            .additional
            .iter()
            .all(|entry| item(&entry.item, entry.count) && entry.fluid_subtype.is_none())
        && reward.key_number.is_none_or(|key| {
            key != 0
                && reward.count == 1
                && reward.contents.is_empty()
                && reward.backpack_outputs.is_empty()
        })
        && (reward.contents.is_empty() || reward.backpack_outputs.is_empty())
        && reward.backpack_outputs.len() <= 19
        && reward.contents.len() <= 20
        && reward
            .contents
            .iter()
            .chain(&reward.backpack_outputs)
            .all(|entry| {
                item(&entry.item, entry.count)
                    && !entry
                        .fluid_subtype
                        .is_some_and(|fluid| fluid == 0 || entry.count != 1)
            })
}

fn parse(text: &str) -> Result<Vec<RewardCallback>, String> {
    parse_documents(&[text]).map(|document| document.callbacks)
}

fn external_track_owners() -> Result<&'static [(String, String)], String> {
    static OWNERS: std::sync::OnceLock<Result<Vec<(String, String)>, String>> =
        std::sync::OnceLock::new();
    OWNERS
        .get_or_init(|| {
            let mut owners = Vec::new();
            for callback in super::world_callbacks::load()? {
                for storage in callback.storage {
                    if !storage.read_only {
                        owners.push((callback.quest.clone(), storage.track));
                    }
                }
            }
            for objective in super::death_objectives::load()? {
                owners.push((objective.quest.clone(), objective.stage.clone()));
                owners.push((objective.quest.clone(), objective.counter.clone()));
            }
            Ok(owners)
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

fn load_document() -> Result<Document, String> {
    parse_documents(DOCUMENTS)
}
#[cfg(test)]
fn parse_document(text: &str) -> Result<Document, String> {
    parse_documents(&[text])
}
fn validate_document_with_capacity(
    document: Document,
    capacity: usize,
) -> Result<Document, String> {
    if !super::item_disguises::valid(&document.item_disguises) {
        return Err("invalid quest item disguises".into());
    }
    let mut keys = BTreeSet::new();
    let slug = |value: &str, prefix: &str, alphabet: &[u8], maximum: usize| {
        value.len() <= maximum
            && value.strip_prefix(prefix).is_some_and(|tail| {
                !tail.is_empty()
                    && tail.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || alphabet.contains(&b)
                    })
            })
    };
    if document.schema != 1
        || document.callbacks.len() > capacity
        || document.tracks.len() > capacity
    {
        return Err("reward callback bounds".into());
    }
    let mut declarations = BTreeSet::new();
    // Journal callbacks are validated while their owner catalogue is initializing.
    // Only writable declarations need cross-catalogue ownership discovery.
    let external = if document.tracks.iter().any(|track| track.writable) {
        external_track_owners()?
    } else {
        &[]
    };
    for track in &document.tracks {
        if !slug(&track.key, "oteryn:", b"._/:-", 128)
            || !slug(&track.quest, "oteryn:", b"._/:-", 128)
            || !declarations.insert(&track.key)
            || track.min > track.initial
            || track.initial > track.max
            || track.sources.is_empty()
            || track.sources.len() > 8
            || track.sources.iter().any(|s| s.is_empty() || s.len() > 1024)
            || if track.writable {
                let owners: Vec<_> = document
                    .callbacks
                    .iter()
                    .flat_map(|c| {
                        c.storage
                            .iter()
                            .map(|s| (&c.quest, s.track.as_str()))
                            .chain(c.state_ops.iter().map(|s| (&c.quest, s.track.as_str())))
                            .chain(
                                c.participants
                                    .iter()
                                    .flat_map(|p| p.tracks().map(|key| (&p.quest, key))),
                            )
                    })
                    .chain(
                        external
                            .iter()
                            .map(|(quest, track)| (quest, track.as_str())),
                    )
                    .filter(|(_, key)| *key == track.key)
                    .map(|(owner, _)| owner)
                    .collect();
                owners.is_empty()
                    || owners
                        .iter()
                        .any(|owner| owner.as_str() != track.quest.as_str())
            } else {
                !document.callbacks.iter().any(|c| {
                    c.requires
                        .iter()
                        .chain(c.peer_requires.iter())
                        .any(|r| r.track == track.key)
                }) || document.callbacks.iter().any(|c| {
                    c.storage
                        .iter()
                        .map(|s| s.track.as_str())
                        .chain(c.state_ops.iter().map(|s| s.track.as_str()))
                        .chain(c.participants.iter().flat_map(QuestParticipant::tracks))
                        .any(|key| key == track.key)
                })
            }
        {
            return Err(format!(
                "invalid reward prerequisite declaration {}",
                track.key
            ));
        }
    }
    for c in &document.callbacks {
        if c.requirements.as_ref().is_some_and(|r| !r.valid())
            || c.give.as_ref().is_some_and(|g| !g.valid())
        {
            return Err(format!("invalid lane reward extension {}", c.key));
        }
        if c.give
            .as_ref()
            .is_some_and(|reward| !valid_item_reward(reward))
            || (c.per_occurrence_mint
                && (c.give.is_none()
                    || c.cosmetic_targets().next().is_some()
                    || c.achievement.is_some()
                    || !c.achievement_progress.is_empty()))
            || c.experience
                .is_some_and(|amount| !(1..=super::QUESTGATE0_RL_05).contains(&amount))
        {
            return Err(format!("invalid reward value extension {}", c.key));
        }
        let tracks: BTreeSet<_> = c.storage.iter().map(|s| &s.track).collect();
        let op_keys: BTreeSet<_> = c.state_ops.iter().map(|s| &s.track).collect();
        let participant_keys: BTreeSet<_> = c
            .participants
            .iter()
            .flat_map(|p| {
                p.storage
                    .iter()
                    .map(|s| &s.track)
                    .chain(p.state_ops.iter().map(|s| &s.track))
            })
            .collect();
        let participant_count: usize = c
            .participants
            .iter()
            .map(|p| p.storage.len() + p.state_ops.len())
            .sum();
        if (!c.participants.is_empty() && participant_key(&c.key, 0).len() > 128)
            || c.participants.len() > 3
            || c.participants
                .iter()
                .map(|p| &p.quest)
                .collect::<BTreeSet<_>>()
                .len()
                != c.participants.len()
            || op_keys.len() != c.state_ops.len()
            || !op_keys.is_disjoint(&tracks)
            || c.state_ops.iter().any(|s| !s.valid())
            || participant_keys.len() != participant_count
            || !participant_keys.is_disjoint(&op_keys)
            || !participant_keys.is_disjoint(&tracks)
            || c.requires
                .iter()
                .any(|r| participant_keys.contains(&r.track) || op_keys.contains(&r.track))
            || c.storage.len()
                + c.state_ops.len()
                + participant_count
                + usize::from(c.counter.is_some())
                > super::QUESTSTATE0_RL_02
            || c.participants.iter().any(|p| {
                p.quest == c.quest
                    || !slug(&p.quest, "oteryn:", b"._/:-", 128)
                    || (p.storage.is_empty() && p.state_ops.is_empty())
                    // Foreign participant receipts retain their separate eight-track contract.
                    || p.storage.len() + p.state_ops.len() > 8
                    || p.state_ops.iter().any(|s| !s.valid())
                    || p.storage
                        .iter()
                        .any(|s| !slug(&s.track, "oteryn:", b"._/:-", 128) || s.minimum > s.maximum)
            })
        {
            return Err(format!("invalid reward participants {}", c.key));
        }
        let inputs: BTreeSet<_> = c.take.iter().map(|s| &s.item).collect();
        let evidence: BTreeSet<_> = c.present.iter().map(|s| &s.item).collect();
        let requires: BTreeSet<_> = c.requires.iter().map(|s| &s.track).collect();
        if c.map_mark
            .as_ref()
            .is_some_and(|mark| !mark.valid() || c.carried_item.is_some())
            || !(slug(&c.key, "oteryn:quest-reward/", b"_/-", 128)
                || (c.is_save_restart_only()
                    && slug(
                        &c.key,
                        super::shared_world_progress::SAVE_EVENT_RESTART_PREFIX,
                        b"._/-",
                        128,
                    )))
            || c.self_target.as_ref().is_some_and(|area| {
                !area.valid()
                    || c.carried_item.is_none()
                    || c.carried_target.is_some()
                    || c.peer_requires.is_some()
                    || c.creature_target.is_some()
                    || c.storage.iter().all(|s| s.preserve)
            })
            || c.corpse_target
                .as_ref()
                .is_some_and(|target| !target.valid(c))
            || (c.fresh_corpse && c.creature_target.is_none())
            || (c.capture_creature && (c.fresh_corpse || c.creature_target.is_none()))
            || c.creature_target.as_ref().is_some_and(|key| {
                c.carried_item.is_none()
                    || c.peer_requires.is_some()
                    || c.give.as_ref().is_some_and(|give| {
                        give.count != 1 || !give.contents.is_empty() || !give.additional.is_empty()
                    })
                    || !slug(key, "oteryn:creature.", b"_", 160)
            })
            || c.peer_requires.as_ref().is_some_and(|requirement| {
                c.carried_item.is_none()
                    || c.storage.iter().all(|state| state.preserve)
                    || !slug(&requirement.track, "oteryn:", b"._/:-", 128)
                    || requirement.minimum > requirement.maximum
            })
            || c.required_claims.len() > 8
            || c.required_claims.iter().collect::<BTreeSet<_>>().len() != c.required_claims.len()
            || c.required_claims
                .iter()
                .any(|key| !slug(key, "oteryn:reward-claim.", b"._/:-", 128))
            || (c.repeatable
                && (!((c.fee.is_some() && c.give.is_some())
                    || (!c.take.is_empty() && (c.give.is_some() || c.experience.is_some())))
                    || c.outfit.is_some()
                    || !c.outfits.is_empty()
                    || c.achievement.is_some()
                    || c.completes
                    || !c.state_ops.is_empty()
                    || !c.participants.is_empty()
                    || c.counter.is_some()
                    || !c.achievement_progress.is_empty()
                    || c.storage.iter().any(|s| !s.preserve)))
            || c.give.as_ref().is_some_and(|r| {
                r.key_number.is_some_and(|key| {
                    key == 0
                        || r.count != 1
                        || !r.contents.is_empty()
                        || !r.backpack_outputs.is_empty()
                })
            })
            || !slug(&c.quest, "oteryn:", b"._/:-", 128)
            || !keys.insert(&c.key)
            || match &c.world_claim {
                None => !slug(&c.npc, "oteryn:npc.", b"_", 128),
                Some(binding) => {
                    !c.npc.is_empty()
                        || !c.keywords.is_empty()
                        || c.topic != 0
                        || c.next_topic != 0
                        || c.carried_item.is_some()
                        || c.fee.is_some()
                        || c.storage.is_empty()
                        || !slug(&binding.key, "oteryn:reward-claim.", b"._/-", 128)
                        || binding.revision.is_empty()
                        || binding.revision.len() > 128
                        || !binding
                            .revision
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"._/:-".contains(&b))
                        || binding.authority_sha256.len() != 64
                        || !binding
                            .authority_sha256
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                }
            }
            || if c.carried_exchange {
                !c.valid_carried_use()
                    || !carried_bounds::valid(c)
                    || c.carried_target.is_some()
                    || c.peer_requires.is_some()
                    || c.creature_target.is_some()
                    || c.self_target.is_some()
                    || c.corpse_target.is_some()
            } else {
                !c.valid_carried_inputs()
            }
            || c.carried_item.is_some()
                && (c.fee.is_some() || !c.requires_actor_conditions.is_empty())
            || c.forbids_outfits.len() > 8
            || c.forbids_outfits.iter().collect::<BTreeSet<_>>().len() != c.forbids_outfits.len()
            || c.forbids_outfits
                .iter()
                .any(|key| !slug(key, "oteryn:outfit.", b"_", 128))
            || (c.storage.is_empty() && c.state_ops.is_empty() && !c.value_only_guarded())
            || c.magic_effects.len() > 8
            || c.magic_effects.iter().any(|cue| cue.effect_id().is_none())
            || c.storage.len() > super::QUESTSTATE0_RL_02
            || c.minimum_level == Some(0)
            || tracks.len() != c.storage.len()
            || c.requires_actor_conditions.len() > 8
            || c.requires_actor_conditions
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != c.requires_actor_conditions.len()
            || c.forbids_addons.len() > 8
            || c.forbids_addons
                .iter()
                .map(|r| (&r.outfit, r.addons, r.sex))
                .collect::<BTreeSet<_>>()
                .len()
                != c.forbids_addons.len()
            || c.forbids_addons.iter().any(|r| {
                !slug(&r.outfit, "oteryn:outfit.", b"_", 128) || !(1..=3).contains(&r.addons)
            })
            || c.minimum_level == Some(0)
            || c.requires_completed_quests.len() > 8
            || c.requires_completed_quests
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != c.requires_completed_quests.len()
            || c.requires_completed_quests
                .iter()
                .any(|key| key == &c.quest || !slug(key, "oteryn:quest.", b"._", 128))
            || c.requires_addons.len() > 8
            || c.requires_addons
                .iter()
                .map(|r| (&r.outfit, r.addons, r.sex))
                .collect::<BTreeSet<_>>()
                .len()
                != c.requires_addons.len()
            || c.requires_addons.iter().any(|r| {
                !slug(&r.outfit, "oteryn:outfit.", b"_", 128) || !(1..=3).contains(&r.addons)
            })
            || c.requires_outfits.len() > 8
            || c.requires_outfits.iter().collect::<BTreeSet<_>>().len() != c.requires_outfits.len()
            || c.requires_outfits
                .iter()
                .any(|key| !slug(key, "oteryn:outfit.", b"_", 128))
            || c.requires.len() > 8
            || requires.len() != c.requires.len()
            || !requires.is_disjoint(&tracks)
            || c.requires
                .iter()
                .any(|r| !slug(&r.track, "oteryn:", b"._/:-", 128) || r.minimum > r.maximum)
            || c.storage
                .iter()
                .any(|s| !slug(&s.track, "oteryn:", b"._/:-", 128) || s.minimum > s.maximum)
            || c.counter.as_ref().is_some_and(|counter| {
                !slug(&counter.track, "oteryn:quest-progress/", b"._/:-", 256)
                    || !(1..=1_000_000_000_000).contains(&counter.maximum)
                    || c.completes
                    || tracks.contains(&counter.track)
                    || op_keys.contains(&counter.track)
                    || participant_keys.contains(&counter.track)
                    || requires.contains(&counter.track)
            })
            || (!c.repeatable
                && c.counter.is_none()
                && !c.value_only_guarded()
                && !c.is_save_restart_only()
                && c.storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set)))
            || c.required_receipts.len() > 8
            || c.required_receipts.iter().collect::<BTreeSet<_>>().len()
                != c.required_receipts.len()
            || c.required_receipts.iter().any(|requirement| {
                !super::valid_quest_key(&requirement.transition)
                    || !super::valid_quest_key(&requirement.track)
                    || requirement.transition == c.key
            })
            || c.present.len() > 8
            || evidence.len() != c.present.len()
            || c.present
                .iter()
                .any(|s| !slug(&s.item, "oteryn:item.", b"._/:-", 256) || s.count == 0)
            || c.fee.as_ref().is_some_and(|fee| {
                fee.gold == 0
                    || fee.gold > 1_000_000_000_000
                    || c.take.iter().any(|item| {
                        matches!(
                            item.item.as_str(),
                            "oteryn:item.tibia.i3031"
                                | "oteryn:item.tibia.i3035"
                                | "oteryn:item.tibia.i3043"
                        )
                    })
            })
            || c.present.iter().any(|item| item.fluid_subtype.is_some())
            || c.take.len() > 16
            || inputs.len() != c.take.len()
            || c.take.iter().any(|s| {
                !slug(&s.item, "oteryn:item.", b"._/:-", 256)
                    || (s.count == 0 || s.fluid_subtype.is_some())
            })
            || c.give
                .as_ref()
                .is_some_and(|reward| !valid_item_reward(reward))
            || (!c.outfits.is_empty() && c.outfit.is_some())
            || c.outfits.len() > 4
            || c.cosmetic_targets()
                .map(|r| (&r.outfit, r.addons, r.sex))
                .collect::<BTreeSet<_>>()
                .len()
                != c.cosmetic_targets().count()
            || c.cosmetic_targets()
                .any(|r| !slug(&r.outfit, "oteryn:outfit.", b"_", 160) || r.addons > 3)
            || (c.keywords.is_empty() && c.world_claim.is_none())
            || c.keywords.len() > 8
            || c.keywords.iter().collect::<BTreeSet<_>>().len() != c.keywords.len()
            || c.keywords.iter().any(|s| {
                s.is_empty() || s.len() > 64 || !s.is_ascii() || s != &s.to_ascii_lowercase()
            })
            || c.experience
                .is_some_and(|amount| !(1..=100_000_000).contains(&amount))
            || (c.repeatable
                && ((c.take.is_empty() && c.fee.is_none())
                    || (c.give.is_none() && c.experience.is_none())
                    || c.cosmetic_targets().next().is_some()
                    || c.achievement.is_some()
                    || !c.achievement_progress.is_empty()))
            || c.inventory.len() > 8
            || c.inventory
                .iter()
                .map(|rule| &rule.item)
                .collect::<BTreeSet<_>>()
                .len()
                != c.inventory.len()
            || c.inventory.iter().any(|rule| {
                !slug(&rule.item, "oteryn:item.", b"._/:-", 256) || rule.minimum > rule.maximum
            })
            || c.failure_reply.len() > 8
            || c.failure_reply.iter().any(|line| {
                line.is_empty() || line.len() > 512 || line.chars().any(char::is_control)
            })
            || c.reply.is_empty()
            || c.achievement_progress.len() > 8
            || c.achievement_progress
                .iter()
                .map(|p| &p.key)
                .collect::<BTreeSet<_>>()
                .len()
                != c.achievement_progress.len()
            || c.achievement_progress
                .iter()
                .any(|p| !slug(&p.key, "oteryn:achievement/", b"_", 160) || p.target == 0)
            || c.reply.len() > 8
            || c.reply
                .iter()
                .any(|s| s.is_empty() || s.len() > 512 || s.chars().any(char::is_control))
            || c.achievement
                .as_ref()
                .is_some_and(|key| !slug(key, "oteryn:achievement/", b"_/-", 128))
            || c.sources.len() > 8
            || c.sources.iter().any(|s| s.is_empty() || s.len() > 1024)
            || c.sources.is_empty()
        {
            return Err(format!("invalid reward callback {}", c.key));
        }
    }
    Ok(document)
}

// Ungated published callbacks retain their original Set encoding. For newly
// gated callbacks, reuse PC3's bound Set so changing admission/value terms also
// changes the owning unfinished quest pin; speech/citations are not reward terms.
/// Optional binding of newly gated admission/value terms; absent legacy gates keep old hashes.
pub fn gated_binding(callback: &RewardCallback) -> Result<Option<[u8; 32]>, String> {
    if callback.minimum_level.is_none()
        && callback.inventory.is_empty()
        && callback.world_claim.is_none()
        && callback.peer_requires.is_none()
        && callback.creature_target.is_none()
        && callback.map_mark.is_none()
        && !(callback.repeatable && callback.is_paid_exchange() && !callback.storage.is_empty())
        && callback.carried_target.is_none()
        && !temporal_payment::guarded(callback)
        && callback.self_target.is_none()
        && !callback
            .key
            .starts_with("oteryn:quest-reward/worldchg-m31/")
    {
        return Ok(None);
    }
    use sha2::{Digest, Sha256};
    let mut terms = serde_json::to_value(callback).map_err(|e| e.to_string())?;
    let terms = terms.as_object_mut().ok_or("reward terms")?;
    terms.remove("reply");
    terms.remove("failure_reply");
    terms.remove("sources");
    let bytes = serde_json::to_vec(terms).map_err(|e| e.to_string())?;
    Ok(Some(Sha256::digest(bytes).into()))
}

/// Recipe receipts have their own selected-input proof, separate from one-input tools.
/// Legacy tool plans retain their exact serialized shape.
pub fn durable_plan(callback: &RewardCallback) -> Result<serde_json::Value, String> {
    let mut value = serde_json::to_value(callback).map_err(|e| e.to_string())?;
    if callback.carried_target.is_some() {
        let object = value.as_object_mut().ok_or("recipe plan")?;
        let source = object.remove("carried_item").ok_or("recipe source")?;
        object.insert("recipe_source".into(), source);
    }
    Ok(value)
}

pub fn participant_key(callback: &str, index: usize) -> String {
    format!("{callback}/participant/{index}")
}

fn value_plan(callback: &RewardCallback) -> Result<Vec<u8>, String> {
    let legacy = serde_json::to_vec(&(
        &callback.key,
        &callback.quest,
        &callback.required_receipts,
        &callback.npc,
        &callback.keywords,
        callback.topic,
        callback.next_topic,
        &callback.storage,
        &callback.take,
        &callback.present,
        &callback.give,
        &callback.outfit,
        &callback.achievement,
        callback.completes,
        callback.experience,
    ))
    .map_err(|e| e.to_string())?;
    let current =
        if callback.state_ops.is_empty() && callback.fee.is_none() && !callback.requires_premium {
            legacy
        } else {
            serde_json::to_vec(&(
                "oteryn:quest-reward-extended-v1",
                legacy,
                &callback.state_ops,
                &callback.fee,
                callback.requires_premium,
            ))
            .map_err(|error| error.to_string())?
        };
    let current = if callback.requires_completed_quests.is_empty() {
        current
    } else {
        serde_json::to_vec(&(
            "oteryn:quest-reward-completed-parents-v1",
            current,
            &callback.requires_completed_quests,
        ))
        .map_err(|error| error.to_string())?
    };
    if callback.per_occurrence_mint {
        serde_json::to_vec(&("oteryn:quest-reward-occurrence-mint-v1", current))
            .map_err(|error| error.to_string())
    } else {
        Ok(current)
    }
}

/// Declare authored bounds before world and death adapters validate their stages.
/// Their complete rewards and provenance are still installed by `extend` afterward.
pub(crate) fn declare_tracks(
    catalogue: QuestStateCatalogue,
) -> Result<QuestStateCatalogue, String> {
    extend_callbacks(catalogue, Vec::new(), load_document()?.tracks)
}

pub fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let document = embedded_document()?;
    extend_document(catalogue, document)
}

#[cfg(test)]
fn extend_fixture_callbacks(
    catalogue: QuestStateCatalogue,
    callbacks: &[RewardCallback],
) -> Result<QuestStateCatalogue, String> {
    extend_document(
        catalogue,
        Document {
            schema: 1,
            item_disguises: Vec::new(),
            tracks: Vec::new(),
            callbacks: callbacks.to_vec(),
        },
    )
}

fn extend_document(
    catalogue: QuestStateCatalogue,
    document: Document,
) -> Result<QuestStateCatalogue, String> {
    extend_callbacks(catalogue, document.callbacks, document.tracks)
}
fn extend_callbacks(
    catalogue: QuestStateCatalogue,
    callbacks: Vec<RewardCallback>,
    declarations: Vec<TrackDeclaration>,
) -> Result<QuestStateCatalogue, String> {
    let npc_legacy = catalogue.npc_legacy.clone();
    let published = published_markers::keys()?;
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    let explicit: BTreeSet<_> = declarations.iter().map(|d| d.key.clone()).collect();
    for declaration in declarations {
        let declared = QuestTrack {
            key: declaration.key,
            quest: declaration.quest,
            initial: declaration.initial,
            min: declaration.min,
            max: declaration.max,
        };
        if let Some(existing) = tracks.iter().find(|t| t.key == declared.key) {
            if existing != &declared {
                return Err(format!(
                    "conflicting reward prerequisite declaration {}",
                    declared.key
                ));
            }
        } else {
            tracks.push(declared);
        }
    }
    let requires: Vec<_> = callbacks
        .iter()
        .flat_map(|c| c.requires.iter().chain(c.peer_requires.iter()))
        .collect();
    for callback in &callbacks {
        if let Some(counter) = &callback.counter {
            if !(1..=1_000_000_000_000).contains(&counter.maximum) {
                return Err(format!("invalid reward counter {}", counter.track));
            }
            if let Some(track) = tracks.iter().find(|t| t.key == counter.track) {
                if track.quest != callback.quest
                    || track.initial != 0
                    || track.min != 0
                    || track.max != counter.maximum
                {
                    return Err(format!("foreign reward counter {}", counter.track));
                }
            } else {
                tracks.push(QuestTrack {
                    key: counter.track.clone(),
                    quest: callback.quest.clone(),
                    initial: 0,
                    min: 0,
                    max: counter.maximum,
                });
            }
        }
        for operation in &callback.state_ops {
            let track = tracks
                .iter()
                .find(|track| track.key == operation.track)
                .ok_or_else(|| format!("undeclared reward operation {}", operation.track))?;
            if track.quest != callback.quest
                || !operation.valid()
                || ((matches!(operation.effect, StateMutation::SetNow)
                    || matches!(
                        operation.from,
                        StateCondition::ElapsedAtLeast { .. }
                            | StateCondition::ElapsedLessThan { .. }
                    ))
                    && (track.initial != 0 || track.min != 0 || track.max != i64::MAX))
            {
                return Err(format!(
                    "foreign or invalid reward operation {}",
                    operation.track
                ));
            }
        }
        for (index, participant) in callback.participants.iter().enumerate() {
            for state in &participant.storage {
                let Some(track) = tracks.iter().find(|t| t.key == state.track) else {
                    return Err(format!("unknown participant track {}", state.track));
                };
                if track.quest != participant.quest
                    || state.minimum < track.min
                    || state.maximum > track.max
                    || !(track.min..=track.max).contains(&state.set)
                {
                    return Err(format!(
                        "foreign or unbounded participant track {}",
                        state.track
                    ));
                }
            }
            if participant.state_ops.iter().any(|op| {
                tracks
                    .iter()
                    .find(|t| t.key == op.track)
                    .is_none_or(|t| t.quest != participant.quest)
            }) {
                return Err(format!("foreign participant operation {}", callback.key));
            }
            transitions.push(QuestTransition {
                world_change: None,
                key: participant_key(&callback.key, index),
                quest: participant.quest.clone(),
                effects: participant
                    .storage
                    .iter()
                    .map(|s| QuestEffect {
                        track: s.track.clone(),
                        from: QuestComparison::Between(s.minimum, s.maximum),
                        effect: if callback
                            .counter
                            .as_ref()
                            .is_some_and(|counter| counter.track == s.track)
                        {
                            QuestEffectKind::Add(1)
                        } else if s.preserve {
                            QuestEffectKind::Add(0)
                        } else {
                            QuestEffectKind::Set(s.set)
                        },
                    })
                    .chain(
                        participant
                            .state_ops
                            .iter()
                            .map(StateOperation::quest_effect),
                    )
                    .collect(),
                completes: false,
                experience: None,
            });
        }
        for requirement in &callback.required_receipts {
            if !catalogue
                .transition(&requirement.transition)
                .is_some_and(|transition| transition.quest == callback.quest)
                || !catalogue.track(&requirement.track).is_some_and(|track| {
                    track.quest == callback.quest
                        && (track.min..=track.max).contains(&requirement.value)
                })
            {
                return Err(format!(
                    "foreign or invalid receipt predicate {}",
                    callback.key
                ));
            }
        }

        // Value participants are part of the pinned quest contract. Reply copy and
        // source citations remain journal metadata and do not invalidate progress.
        let value_plan = value_plan(callback)?;
        let digest: String = Sha256::digest(value_plan)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if (!callback.storage.is_empty() || !callback.state_ops.is_empty())
            && published_markers::needs_marker(callback, &published)
        {
            transitions.push(QuestTransition {
                world_change: None,
                key: format!("oteryn:quest-reward-rule/{digest}"),
                quest: callback.quest.clone(),
                effects: vec![QuestEffect {
                    track: callback
                        .storage
                        .first()
                        .map(|storage| storage.track.clone())
                        .or_else(|| {
                            callback
                                .state_ops
                                .first()
                                .map(|operation| operation.track.clone())
                        })
                        .ok_or("reward storage/operation")?,
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Computed,
                }],
                completes: false,
                experience: None,
            });
        }
        for s in &callback.storage {
            if let Some(track) = tracks.iter_mut().find(|t| t.key == s.track) {
                if track.quest != callback.quest {
                    return Err(format!("foreign reward track {}", s.track));
                }
                if explicit.contains(&s.track)
                    && (s.minimum < track.min
                        || s.maximum > track.max
                        || !(track.min..=track.max).contains(&s.set))
                {
                    return Err(format!(
                        "unbounded explicitly declared reward track {}",
                        s.track
                    ));
                }
                if !explicit.contains(&s.track) {
                    track.max = track.max.max(s.maximum).max(s.set);
                    track.min = track.min.min(s.minimum);
                }
            } else {
                tracks.push(QuestTrack {
                    key: s.track.clone(),
                    quest: callback.quest.clone(),
                    initial: -1,
                    min: s.minimum.min(-1).min(s.set),
                    max: s.maximum.max(s.set),
                });
            }
        }
        transitions.push(primary_transition(callback)?);
    }
    if let Some(required) = requires
        .iter()
        .find(|r| !tracks.iter().any(|t| t.key == r.track))
    {
        return Err(format!("undeclared reward prerequisite {}", required.track));
    }
    let mut updated = QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))?;
    updated.npc_legacy = npc_legacy;
    updated.predecessors = catalogue.predecessors;
    Ok(updated)
}

/// Recover the complete reward definition captured before Citizen NPC activation.
/// The current loader still installs NPC prerequisites before active rewards.
pub(super) fn retain_outfit_reward_predecessor(
    before_npc: QuestStateCatalogue,
    active: &mut QuestStateCatalogue,
) -> Result<(), String> {
    const QUEST: &str = "oteryn:quest.outfit_and_addon_quests";
    if before_npc.content_revision != active.content_revision {
        return Err("outfit reward predecessor content revision".into());
    }
    let before = (
        before_npc
            .definition_hash(QUEST)
            .ok_or("outfit reward predecessor missing definition")?,
        before_npc
            .tracks
            .values()
            .filter(|track| track.quest == QUEST)
            .map(|track| (track.key.clone(), track.initial))
            .collect::<BTreeMap<_, _>>(),
    );
    if active
        .definition_hash(QUEST)
        .is_none_or(|hash| hash == before.0)
    {
        return Err("outfit reward predecessor missing NPC successor".into());
    }
    // Foreign rewards can write Outfit tracks through exact participant effects.
    // Use the same validated source and constructor as the active reward owner.
    let document = embedded_document()?;
    let callbacks = document
        .callbacks
        .into_iter()
        .filter(|callback| {
            callback.quest == QUEST
                || callback
                    .participants
                    .iter()
                    .any(|participant| participant.quest == QUEST)
        })
        .collect();
    let complete = extend_callbacks(before_npc, callbacks, document.tracks)?;
    let complete = (
        complete
            .definition_hash(QUEST)
            .ok_or("outfit reward predecessor missing reward definition")?,
        complete
            .tracks
            .values()
            .filter(|track| track.quest == QUEST)
            .map(|track| (track.key.clone(), track.initial))
            .collect::<BTreeMap<_, _>>(),
    );
    if complete.0 == before.0
        || before
            .1
            .iter()
            .any(|(key, value)| complete.1.get(key) != Some(value))
    {
        return Err("outfit reward predecessor changed existing defaults".into());
    }
    let registered = active
        .npc_legacy
        .get(QUEST)
        .ok_or("outfit NPC predecessor missing")?;
    let retained = active
        .predecessors
        .get(QUEST)
        .and_then(|entries| entries.iter().find(|entry| entry.hash == before.0));
    if retained.is_some_and(|entry| entry.defaults != before.1) {
        return Err("outfit reward predecessor conflicting defaults".into());
    }
    if registered == &complete {
        return retained
            .map(|_| ())
            .ok_or_else(|| "outfit reward predecessor missing retained capture".into());
    }
    if registered != &before {
        return Err("outfit NPC predecessor differs from captured catalogue".into());
    }
    // Keep the exact declaration-only pin already accepted by the current loader,
    // including its own absent-track defaults. No stored rows or pins are changed.
    if retained.is_none() {
        active.predecessors.entry(QUEST.into()).or_default().push(
            super::NpcDefinitionPredecessor {
                hash: before.0,
                defaults: before.1,
            },
        );
    }
    active.npc_legacy.insert(QUEST.into(), complete);
    Ok(())
}

/// Exact primary state operation shared by lowering and bound admission validation.
pub fn primary_transition(callback: &RewardCallback) -> Result<QuestTransition, String> {
    let binding = gated_binding(callback)?;
    // A guarded exchange may only read progress while still binding admission.
    let bound_effect = callback
        .storage
        .iter()
        .position(|state| !state.preserve)
        .or_else(|| (!callback.storage.is_empty()).then_some(0));
    let stamp_binding = binding.filter(|_| temporal_payment::guarded(callback));
    Ok(QuestTransition {
        world_change: None,
        key: callback.key.clone(),
        quest: callback.quest.clone(),
        effects: callback
            .storage
            .iter()
            .enumerate()
            .map(|(index, state)| QuestEffect {
                track: state.track.clone(),
                from: QuestComparison::Between(state.minimum, state.maximum),
                effect: if state.preserve {
                    match binding.filter(|_| Some(index) == bound_effect) {
                        Some(binding) => QuestEffectKind::AddWithBinding { delta: 0, binding },
                        None => QuestEffectKind::Add(0),
                    }
                } else {
                    match (
                        state.at_least,
                        binding.filter(|_| Some(index) == bound_effect),
                    ) {
                        (true, Some(binding)) => QuestEffectKind::RaiseToWithBinding {
                            value: state.set,
                            binding,
                        },
                        (false, Some(binding)) => QuestEffectKind::SetWithBinding {
                            value: state.set,
                            binding,
                        },
                        (true, None) => QuestEffectKind::RaiseTo(state.set),
                        (false, None) => QuestEffectKind::Set(state.set),
                    }
                },
            })
            .chain(
                callback
                    .state_ops
                    .iter()
                    .map(|operation| temporal_binding::stamp(operation, stamp_binding)),
            )
            .chain(callback.counter.iter().map(|counter| QuestEffect {
                track: counter.track.clone(),
                from: QuestComparison::Between(0, counter.maximum - 1),
                effect: QuestEffectKind::Add(1),
            }))
            .collect(),
        completes: callback.completes,
        experience: callback.experience,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn reward_storage_supports_one_atomic_sixteen_track_plan() -> Result<(), String> {
        let mut callback = super::parse(include_str!(
            "../../../../../content/quests/shared-reward-callbacks.json"
        ))?
        .into_iter()
        .next()
        .ok_or("callback")?;
        callback.storage = (0..16)
            .map(|i| super::Storage {
                track: format!("oteryn:quest-progress/atomic/{i}"),
                minimum: 0,
                maximum: 0,
                set: 1,
                set_if: None,
                add: None,
                preserve: false,
                at_least: false,
                increment: false,
                read_only: false,
            })
            .collect();
        let document = |callback: &super::RewardCallback| {
            serde_json::json!({"schema": 1, "callbacks": [callback]}).to_string()
        };
        assert!(super::parse(&document(&callback)).is_ok());
        callback.storage.push(super::Storage {
            track: "oteryn:quest-progress/atomic/16".into(),
            minimum: 0,
            maximum: 0,
            set: 1,
            set_if: None,
            add: None,
            preserve: false,
            at_least: false,
            increment: false,
            read_only: false,
        });
        assert!(super::parse(&document(&callback)).is_err());
        Ok(())
    }

    #[test]
    fn paid_repeatable_key_sales_require_price_preserved_guards_and_one_key() -> Result<(), String>
    {
        let source = include_str!("../../../../../content/quests/npc-callbacks/fibula.json");
        let mut value: serde_json::Value =
            serde_json::from_str(source).map_err(|e| e.to_string())?;
        let index = value["callbacks"]
            .as_array()
            .ok_or("callbacks")?
            .iter()
            .position(|c| c["key"] == "oteryn:quest-reward/fibula-key-repeat")
            .ok_or("key sale")?;
        assert!(parse(&value.to_string()).is_ok());
        for edit in 0..7 {
            let mut invalid = value.clone();
            let c = &mut invalid["callbacks"][index];
            match edit {
                0 => c["fee"] = serde_json::Value::Null,
                1 => c["fee"]["gold"] = 0.into(),
                2 => c["give"]["key_number"] = 0.into(),
                3 => c["give"]["count"] = 2.into(),
                4 => {
                    c["give"]["contents"] =
                        serde_json::json!([{"item":"oteryn:item.tibia.i3031","count":1}])
                }
                5 => c["storage"][0]["preserve"] = false.into(),
                _ => c["achievement"] = "oteryn:achievement/backpack_tourist".into(),
            }
            assert!(
                parse(&invalid.to_string()).is_err(),
                "invalid paid sale {edit}"
            );
        }
        value["callbacks"][index]["repeatable"] = false.into();
        assert!(
            parse(&value.to_string()).is_err(),
            "ordinary rewards still need a once-only guard"
        );
        Ok(())
    }

    #[test]
    fn completed_parent_requirements_are_bounded_unique_and_read_only() -> Result<(), String> {
        let original = super::load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-reward/l13-rift-base")
            .ok_or("Rift base")?;
        assert_eq!(
            original.requires_completed_quests,
            ["oteryn:quest.ferumbras_ascension_quest"]
        );
        let original_plan = super::value_plan(&original)?;
        let mut changed = original.clone();
        changed.requires_completed_quests.clear();
        assert_ne!(original_plan, super::value_plan(&changed)?);
        changed.requires_completed_quests = vec!["oteryn:quest.another_parent".into()];
        assert_ne!(original_plan, super::value_plan(&changed)?);
        for parents in [
            vec![original.quest.clone()],
            vec!["oteryn:quest.INVALID".into()],
            vec!["oteryn:quest.parent".into(); 2],
            (0..9).map(|i| format!("oteryn:quest.parent_{i}")).collect(),
        ] {
            let mut row = serde_json::to_value(&original).map_err(|e| e.to_string())?;
            row["requires_completed_quests"] = serde_json::json!(parents);
            assert!(
                super::parse(&serde_json::json!({"schema":1,"callbacks":[row]}).to_string())
                    .is_err()
            );
        }
        Ok(())
    }

    #[test]
    fn level_and_positive_addon_guards_preserve_one_shot_bases() -> Result<(), String> {
        let original = super::load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-reward/l11-dream-warrior-base")
            .ok_or("level base")?;
        assert!(original.storage.is_empty());
        for edit in 0..5 {
            let mut row = serde_json::to_value(&original).map_err(|e| e.to_string())?;
            match edit {
                0 => row["minimum_level"] = 0.into(),
                1 => row["forbids_outfits"] = serde_json::json!([]),
                2 => row["forbids_outfits"] = serde_json::json!(["oteryn:outfit.knight"]),
                3 => {
                    row["requires_addons"] =
                        serde_json::json!([{"outfit":"oteryn:outfit.knight","addons":0}])
                }
                _ => row["outfit"]["sex"] = "male".into(),
            }
            assert!(
                super::parse(&serde_json::json!({"schema":1,"callbacks":[row]}).to_string())
                    .is_err(),
                "invalid level/base guard {edit}"
            );
        }
        Ok(())
    }
    #[test]
    fn carried_quest_tools_bind_exactly_one_input_and_negative_ownership() -> Result<(), String> {
        let original = super::load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-reward/l10-brotherhood-use-follower")
            .ok_or("carried source")?;
        for edit in 0..5 {
            let mut row = serde_json::to_value(&original).map_err(|e| e.to_string())?;
            match edit {
                0 => row["carried_item"] = "oteryn:item.tibia.i769".into(),
                1 => row["take"] = serde_json::json!([]),
                2 => row["take"][0]["count"] = 2.into(),
                3 => {
                    row["give"] =
                        serde_json::json!({"item":"oteryn:item.tibia.i768","count":1,"contents":[]})
                }
                _ => {
                    row["forbids_outfits"] = serde_json::json!([
                        "oteryn:outfit.brotherhood",
                        "oteryn:outfit.brotherhood"
                    ])
                }
            }
            assert!(
                super::parse(&serde_json::json!({"schema":1,"callbacks":[row]}).to_string())
                    .is_err(),
                "invalid carried tool {edit}"
            );
        }
        Ok(())
    }
    #[test]
    fn paid_value_only_sources_require_cost_mint_and_bounded_progress() -> Result<(), String> {
        let original = load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-reward/l9-afflicted-pouch")
            .ok_or("paid source")?;
        assert!(original.storage.is_empty());
        assert!(original.is_paid_exchange());
        for edit in 0..4 {
            let mut callback = serde_json::to_value(&original).map_err(|e| e.to_string())?;
            match edit {
                0 => callback["take"] = serde_json::json!([]),
                1 => callback["give"] = serde_json::Value::Null,
                2 => callback["achievement_progress"][0]["target"] = 0.into(),
                _ => {
                    let progress = callback["achievement_progress"][0].clone();
                    callback["achievement_progress"]
                        .as_array_mut()
                        .ok_or("progress")?
                        .push(progress);
                }
            }
            assert!(
                parse(&serde_json::json!({"schema":1,"callbacks":[callback]}).to_string()).is_err(),
                "invalid paid source case {edit}"
            );
        }
        Ok(())
    }
    use super::*;
    #[test]
    fn participant_authoring_rejects_owner_confusion_duplicates_and_invalid_sex()
    -> Result<(), String> {
        let callbacks = load()?;
        let original = callbacks
            .iter()
            .find(|c| !c.participants.is_empty())
            .ok_or("participant source")?;
        for edit in 0..5 {
            let mut callback = serde_json::to_value(original).map_err(|e| e.to_string())?;
            match edit {
                0 => callback["participants"][0]["quest"] = callback["quest"].clone(),
                1 => {
                    callback["participants"][0]["storage"][0]["track"] =
                        callback["storage"][0]["track"].clone()
                }
                2 => callback["creation_sex"] = "selected_appearance".into(),
                3 => {
                    let p = callback["participants"][0].clone();
                    callback["participants"]
                        .as_array_mut()
                        .ok_or("array")?
                        .push(p);
                }
                _ => {
                    let maximum = callback["participants"][0]["storage"][0]["maximum"]
                        .as_i64()
                        .ok_or("maximum")?;
                    callback["participants"][0]["storage"][0]["minimum"] =
                        maximum.checked_add(1).ok_or("maximum overflow")?.into();
                }
            }
            let doc = serde_json::json!({"schema":1,"callbacks":[callback]});
            assert!(
                parse(&doc.to_string()).is_err(),
                "invalid participant case{edit}"
            );
        }
        // Optional fields preserve legacy source/receipt serialization.
        let legacy = callbacks
            .iter()
            .find(|c| c.participants.is_empty() && c.creation_sex.is_none())
            .ok_or("legacy")?;
        let encoded = serde_json::to_value(legacy).map_err(|e| e.to_string())?;
        assert!(encoded.get("participants").is_none());
        assert!(encoded.get("creation_sex").is_none());
        Ok(())
    }

    #[test]
    fn actor_and_unowned_addon_guards_fail_closed() -> Result<(), String> {
        let original: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/shared-reward-callbacks.json"
        ))
        .map_err(|e| e.to_string())?;
        for guard in [
            serde_json::json!({"requires_actor_conditions":["fire","fire"]}),
            serde_json::json!({"requires_actor_conditions":["invented"]}),
            serde_json::json!({"forbids_addons":[{"outfit":"oteryn:outfit.knight","addons":0}]}),
            serde_json::json!({"forbids_addons":[{"outfit":"oteryn:outfit.knight","addons":1},{"outfit":"oteryn:outfit.knight","addons":1}]}),
        ] {
            let mut doc = original.clone();
            for (key, value) in guard.as_object().ok_or("guard")? {
                doc["callbacks"][0][key] = value.clone();
            }
            assert!(parse(&doc.to_string()).is_err());
        }
        let legacy = load()?
            .into_iter()
            .find(|c| c.requires_actor_conditions.is_empty() && c.forbids_addons.is_empty())
            .ok_or("legacy")?;
        let value = serde_json::to_value(legacy).map_err(|e| e.to_string())?;
        assert!(value.get("requires_actor_conditions").is_none());
        assert!(value.get("forbids_addons").is_none());
        Ok(())
    }

    #[test]
    fn native_eligibility_guards_are_bounded_and_preserve_legacy_plans() -> Result<(), String> {
        let original: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/shared-reward-callbacks.json"
        ))
        .map_err(|e| e.to_string())?;
        for guard in [
            serde_json::json!({"requires_premium":"yes"}),
            serde_json::json!({"requires_outfits":["oteryn:outfit.beggar","oteryn:outfit.beggar"]}),
            serde_json::json!({"requires_outfits":["untrusted"]}),
        ] {
            let mut doc = original.clone();
            let edits = guard.as_object().ok_or("guards")?;
            for (key, value) in edits {
                doc["callbacks"][0][key] = value.clone();
            }
            assert!(
                parse(&doc.to_string()).is_err(),
                "invalid native eligibility guard"
            );
        }
        let legacy = load()?
            .into_iter()
            .find(|c| !c.requires_premium && c.requires_outfits.is_empty())
            .ok_or("legacy")?;
        let value = serde_json::to_value(legacy).map_err(|e| e.to_string())?;
        assert!(value.get("requires_premium").is_none());
        assert!(value.get("requires_outfits").is_none());
        Ok(())
    }

    #[test]
    fn state_operations_and_writable_declarations_fail_closed() -> Result<(), String> {
        let source = include_str!("../../../../../content/quests/shared-reward-callbacks.json");
        let original: serde_json::Value =
            serde_json::from_str(source).map_err(|e| e.to_string())?;
        let index = original["callbacks"]
            .as_array()
            .ok_or("callbacks")?
            .iter()
            .position(|c| c.get("state_ops").is_some())
            .ok_or("operations")?;
        for case in 0..4 {
            let mut document = original.clone();
            let callback = &mut document["callbacks"][index];
            match case {
                0 => callback["state_ops"][0]["track"] = callback["storage"][0]["track"].clone(),
                1 => {
                    callback["state_ops"][0]["from"] =
                        serde_json::json!({"kind":"between","minimum":2,"maximum":1})
                }
                2 => callback["state_ops"][0]["effect"] = serde_json::json!({"kind":"execute_sql"}),
                _ => {
                    let declaration = document["tracks"]
                        .as_array_mut()
                        .ok_or("tracks")?
                        .iter_mut()
                        .find(|t| t["writable"] == true)
                        .ok_or("writable")?;
                    declaration["writable"] = false.into();
                }
            }
            assert!(
                parse(&document.to_string()).is_err(),
                "invalid operation/declaration case{case}"
            );
        }
        let legacy = load()?
            .into_iter()
            .find(|c| c.state_ops.is_empty() && c.participants.is_empty())
            .ok_or("legacy")?;
        assert!(
            serde_json::to_value(legacy)
                .map_err(|e| e.to_string())?
                .get("state_ops")
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn readonly_prerequisite_uses_declared_initial_and_fresh_stored_value() -> Result<(), String> {
        let catalogue = QuestStateCatalogue::new(
            "test",
            vec![QuestTrack {
                key: "oteryn:parent".into(),
                quest: "oteryn:quest.parent".into(),
                initial: 7,
                min: 0,
                max: 8,
            }],
            vec![],
        )
        .map_err(|e| format!("{e:?}"))?;
        let requirement = TrackRequirement {
            track: "oteryn:parent".into(),
            minimum: 7,
            maximum: 7,
        };
        let mut stored = BTreeMap::new();
        assert!(requirement.holds(&catalogue, &stored));
        stored.insert(requirement.track.clone(), 6);
        assert!(!requirement.holds(&catalogue, &stored));
        stored.insert(requirement.track.clone(), 7);
        assert!(requirement.holds(&catalogue, &stored));
        assert!(
            !TrackRequirement {
                track: "oteryn:missing".into(),
                ..requirement
            }
            .holds(&catalogue, &stored)
        );
        Ok(())
    }

    #[test]
    fn qengine_reward_callbacks_are_bounded_and_never_repeatable_value_grants() -> Result<(), String>
    {
        let source = include_str!("../../../../../content/quests/shared-reward-callbacks.json");
        let callbacks = load()?;
        assert!(callbacks.len() >= 6);
        for edit in 0..6 {
            let mut value: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            match edit {
                0 => {
                    value["callbacks"][0]["take"] =
                        serde_json::json!([{"item":"oteryn:item.tibia.i139","count":0}])
                }
                1 => {
                    value["callbacks"][0]["outfit"] =
                        serde_json::json!({"outfit":"oteryn:outfit.pirate","addons":4})
                }
                2 => value["callbacks"][0]["storage"] = serde_json::json!([]),
                3 => {
                    value["callbacks"][0]["storage"][0]["set"] =
                        value["callbacks"][0]["storage"][0]["minimum"].clone()
                }
                4 => value["callbacks"][0]["unknown"] = true.into(),
                _ => {
                    value["callbacks"][0]["take"] = serde_json::json!([{"item":"oteryn:item.tibia.i139","count":1},{"item":"oteryn:item.tibia.i139","count":1}])
                }
            }
            assert!(parse(&value.to_string()).is_err(), "case {edit}");
        }
        for (field, invalid) in [
            ("npc", "oteryn:npc."),
            ("npc", "oteryn:npc.morgan?"),
            ("quest", "oteryn:quest.BAD"),
            ("key", "oteryn:quest-reward/bad.key"),
        ] {
            let mut value: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            value["callbacks"][0][field] = invalid.into();
            assert!(parse(&value.to_string()).is_err(), "{field}={invalid}");
        }
        let mut value: serde_json::Value =
            serde_json::from_str(source).map_err(|e| e.to_string())?;
        value["callbacks"][0]["take"] =
            serde_json::json!([{"item":"oteryn:item.tibia.i3031","count":1000}]);
        assert!(
            parse(&value.to_string()).is_ok(),
            "inputs may span many stacks"
        );
        for present in [
            serde_json::json!([{"item":"oteryn:item.tibia.i9041","count":0}]),
            serde_json::json!([{"item":"oteryn:item.tibia.i9041","count":1},{"item":"oteryn:item.tibia.i9041","count":1}]),
        ] {
            let mut invalid: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            invalid["callbacks"][0]["present"] = present;
            assert!(parse(&invalid.to_string()).is_err());
        }
        let track = value["callbacks"][0]["storage"][0]["track"].clone();
        for required in [
            serde_json::json!([{"track":track,"minimum":0,"maximum":1}]),
            serde_json::json!([{"track":"oteryn:parent","minimum":2,"maximum":1}]),
            serde_json::json!([{"track":"oteryn:parent","minimum":0,"maximum":1},{"track":"oteryn:parent","minimum":0,"maximum":1}]),
        ] {
            let mut invalid: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            invalid["callbacks"][0]["requires"] = required;
            assert!(parse(&invalid.to_string()).is_err());
        }
        for edit in 0..3 {
            let mut invalid: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            match edit {
                0 => {
                    let duplicate = invalid["tracks"][0].clone();
                    invalid["tracks"]
                        .as_array_mut()
                        .ok_or("tracks")?
                        .push(duplicate);
                }
                1 => invalid["tracks"][0]["initial"] = serde_json::json!(i64::MAX),
                _ => invalid["tracks"][0]["key"] = serde_json::json!("oteryn:unused"),
            }
            assert!(
                parse(&invalid.to_string()).is_err(),
                "invalid declaration {edit}"
            );
        }
        super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        Ok(())
    }
    #[test]
    fn gated_terms_bind_level_and_value_but_not_display_text() -> Result<(), String> {
        let original = load()?
            .into_iter()
            .find(|c| c.minimum_level == Some(30))
            .ok_or("gated callback")?;
        let binding = gated_binding(&original)?.ok_or("binding")?;
        let mut changed = original.clone();
        changed.minimum_level = Some(31);
        assert_ne!(gated_binding(&changed)?, Some(binding));
        let mut changed = original.clone();
        changed.requires_premium = !changed.requires_premium;
        assert_ne!(gated_binding(&changed)?, Some(binding));
        let mut changed = original.clone();
        changed.reply = vec!["Different wording".into()];
        changed.sources = vec!["Different citation".into()];
        assert_eq!(gated_binding(&changed)?, Some(binding));
        let mut changed = original;
        changed.minimum_level = None;
        assert_eq!(gated_binding(&changed)?, None);
        let mut invalid = serde_json::to_value(changed).map_err(|e| e.to_string())?;
        invalid["minimum_level"] = 0.into();
        assert!(parse(&serde_json::json!({"schema":1,"callbacks":[invalid]}).to_string()).is_err());
        Ok(())
    }

    #[test]
    fn published_ungated_quest_hash_baseline() -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        for (key, expected) in [
            (
                "oteryn:quest.outfit_and_addon_quests",
                "bd44ba2f572315de58bc53958ee1bf398a7039ae5badf3e796017965d051f6c8",
            ),
            // Includes the source seal callbacks, packed counter and Siflind vial grant.
            (
                "oteryn:quest.the_ice_islands_quest",
                // M47 binds four source counters plus the durable roll; absence stays -1.
                "80dc73861594fa37edd7c46c38a7734cc9c84889fa2bcc89e9af08a49ff002f0",
            ),
            (
                "oteryn:quest.the_spirit_will_get_you_quest",
                "0df1f07a25989a00e27d16a890e8598556f37bcb08deb2a3b2feab8958747414",
            ),
        ] {
            let hash = lowered
                .catalogue()
                .definition_hash(key)
                .ok_or("published quest")?;
            // Citizen source scopes promote the shared quest through the existing
            // exact-predecessor adapter; its published hash must remain loadable.
            let hash = if key == "oteryn:quest.outfit_and_addon_quests" {
                let legacy = lowered
                    .catalogue()
                    .npc_legacy
                    .get(key)
                    .ok_or("published Citizen predecessor")?
                    .0;
                assert_ne!(
                    hash, legacy,
                    "source scopes must bind the active definition"
                );
                legacy
            } else {
                hash
            };
            assert_eq!(
                hash.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                expected,
                "published ungated quest {key}"
            );
        }
        Ok(())
    }

    #[test]
    fn registered_shards_preserve_each_original_record_bound() -> Result<(), String> {
        // Shared, QuestPC7, other registered shards, M43 peer, Kingsday journal.
        assert_eq!(load()?.len(), 207 + 59 + 121 + 1 + 1);
        let mut document: serde_json::Value =
            serde_json::from_str(DOCUMENTS[0]).map_err(|error| error.to_string())?;
        let callbacks = document["callbacks"].as_array_mut().ok_or("callbacks")?;
        let first = callbacks.first().ok_or("first callback")?.clone();
        while callbacks.len() <= questpc7_registry_bounds::PER_DOCUMENT {
            callbacks.push(first.clone());
        }
        assert_eq!(
            parse(&document.to_string()).err().as_deref(),
            Some("reward callback bounds")
        );
        assert_eq!(
            parse_documents(&[DOCUMENTS[0]; DOCUMENTS.len() + 1])
                .err()
                .as_deref(),
            Some("reward callback document bounds")
        );
        Ok(())
    }

    #[test]
    fn authored_shards_complete_only_the_selected_quest_and_preserve_old_plans()
    -> Result<(), String> {
        let legacy = parse(DOCUMENTS[0])?;
        let added_keys = [
            "oteryn:quest-reward/l2-wizard-flowers",
            "oteryn:quest-reward/l2-wizard-flowers-start",
            "oteryn:quest-reward/l2-wizard-legs",
            "oteryn:quest-reward/l2-wizard-mail",
            "oteryn:quest-reward/l2-wizard-shield",
            "oteryn:quest-reward/l2-wizard-skull",
            "oteryn:quest-reward/l2-wizard-start",
            "oteryn:quest-reward/m36-rock-steady-confirm",
            "oteryn:quest-reward/m36-rock-steady-first",
            "oteryn:quest-reward/m36-rock-steady-second",
            "oteryn:quest-reward/m36-rock-steady-start",
            "oteryn:quest-reward/m36-rock-steady-weapon",
        ];
        let added: BTreeSet<_> = legacy
            .iter()
            .map(|callback| callback.key.as_str())
            .filter(|key| added_keys.contains(key))
            .collect();
        assert_eq!(added, BTreeSet::from(added_keys));
        assert_eq!(legacy.len(), 207);
        let original: Vec<_> = legacy
            .iter()
            .filter(|callback| !added_keys.contains(&callback.key.as_str()))
            .collect();
        assert_eq!(original.len(), 195);
        let programmes = parse(include_str!(
            "../../../../../content/quests/npc-callbacks/questpc7-programmes.json"
        ))?;
        assert_eq!(programmes.len(), 59);
        let published = published_markers::keys()?;
        for callback in &programmes {
            assert!(callback.key.starts_with("oteryn:quest-reward/questpc7-"));
            assert!(published.contains(&callback.key));
            assert!(legacy.iter().all(|old| old.key != callback.key));
        }
        for callback in original {
            let crown = callback.key == "oteryn:quest-reward/l4-royal-costume-crown";
            assert_eq!(callback.completes, crown);
            assert_eq!(
                serde_json::to_value(callback)
                    .map_err(|e| e.to_string())?
                    .get("completes")
                    .is_some(),
                crown
            );
            if crown {
                assert_eq!(callback.quest, "oteryn:quest.royal_costume_outfits_quest");
                assert_eq!(
                    (callback.storage[0].minimum, callback.storage[0].set),
                    (2, 3)
                );
            }
        }
        let callbacks = load()?;
        let completion = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-reward/l6-spirit-confession")
            .ok_or("authored completion")?;
        assert!(completion.completes);
        assert_eq!(
            completion.quest,
            "oteryn:quest.the_spirit_will_get_you_quest"
        );
        assert!(completion.take.is_empty());
        assert_eq!(completion.present[0].count, 3);
        let catalogue = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            catalogue
                .catalogue()
                .transition(&completion.key)
                .ok_or("completion transition")?
                .completes
        );
        assert!(parse_documents(&[DOCUMENTS[0], DOCUMENTS[0]]).is_err());
        let mut malformed: serde_json::Value =
            serde_json::from_str(DOCUMENTS[1]).map_err(|e| e.to_string())?;
        malformed["callbacks"][0]["completes"] = "yes".into();
        assert!(parse(&malformed.to_string()).is_err());
        Ok(())
    }
}

impl ItemReward {
    pub(super) fn valid(&self) -> bool {
        valid_item_reward(self)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardCounter {
    pub track: String,
    pub maximum: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CarriedItemRequirement {
    pub item: String,
    pub count: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_source: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardRequirements {
    pub min_level: u32,
    pub premium: bool,
    #[serde(default)]
    pub completed_quests: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owned_cosmetics: Vec<OutfitReward>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owned_items: Vec<CarriedItemRequirement>,
}

#[cfg(test)]
mod receipt_test_base;

impl RewardRequirements {
    pub(super) fn valid(&self) -> bool {
        let slug = |value: &str, prefix: &str, maximum| {
            value.len() <= maximum
                && value.strip_prefix(prefix).is_some_and(|tail| {
                    !tail.is_empty()
                        && tail.bytes().all(|b| {
                            b.is_ascii_lowercase() || b.is_ascii_digit() || b"._/:-".contains(&b)
                        })
                })
        };
        self.min_level <= 1_000_000
            && self.completed_quests.len() <= 8
            && self.completed_quests.iter().collect::<BTreeSet<_>>().len()
                == self.completed_quests.len()
            && self
                .completed_quests
                .iter()
                .all(|key| slug(key, "oteryn:quest.", 128))
            && self.owned_items.len() <= 8
            && self
                .owned_items
                .iter()
                .map(|i| &i.item)
                .collect::<BTreeSet<_>>()
                .len()
                == self.owned_items.len()
            && self.owned_items.iter().all(|i| {
                slug(&i.item, "oteryn:item.", 256)
                    && (1..=100).contains(&i.count)
                    && i.map_source
                        .is_none_or(|key| key <= 0xffffffffffff && ((key >> 8) & 255) <= 15)
            })
            && self.owned_cosmetics.len() <= 8
            && self
                .owned_cosmetics
                .iter()
                .map(|o| &o.outfit)
                .collect::<BTreeSet<_>>()
                .len()
                == self.owned_cosmetics.len()
            && self.owned_cosmetics.iter().all(|o| {
                o.addons <= 3
                    && o.outfit.len() <= 160
                    && o.outfit.strip_prefix("oteryn:outfit.").is_some_and(|tail| {
                        !tail.is_empty()
                            && tail
                                .bytes()
                                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                    })
            })
    }
}
#[cfg(test)]
mod lane_tests {
    use super::*;
    #[test]
    fn reward_extension_preserves_native_npc_legacy_bindings() -> Result<(), String> {
        let loaded = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            !loaded.catalogue().npc_legacy.is_empty(),
            "adding rewards must preserve the native NPC source binding owner"
        );
        Ok(())
    }

    #[test]
    fn reward_extension_keeps_registered_predecessor_lookup_exact() -> Result<(), String> {
        let loaded = super::super::loader::parse_quest_state(
            include_str!("../../../../../content/quests/missions/quest-state.json"),
            "reward-predecessors",
        )
        .map_err(|error| format!("{error:?}"))?;
        let mut base = loaded.catalogue().clone();
        let fixture: BTreeMap<String, String> = serde_json::from_str(include_str!(
            "npc_rules/fixtures/m04_pre_completion_hashes.json"
        ))
        .map_err(|error| error.to_string())?;
        for (quest, text) in fixture {
            let mut hash = [0u8; 32];
            for (index, byte) in hash.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
                    .map_err(|error| error.to_string())?;
            }
            base.predecessors.insert(
                quest,
                vec![super::super::NpcDefinitionPredecessor {
                    hash,
                    defaults: BTreeMap::new(),
                }],
            );
        }
        let expected = base.predecessors.clone();
        assert_eq!(expected.len(), 2);
        let extended = extend_document(
            base,
            parse_document(include_str!(
                "../../../../../content/quests/npc-callbacks/fibula.json"
            ))?,
        )?;
        assert!(
            extended
                .transition("oteryn:quest-reward/fibula-key-repeat")
                .is_some(),
            "the real reward extension must add its callback"
        );
        assert_eq!(extended.predecessors, expected);
        for (quest, entries) in &expected {
            for entry in entries {
                assert_eq!(
                    super::super::npc_rules::legacy_defaults(&extended, quest, &entry.hash),
                    Some(&entry.defaults)
                );
                let mut invalid = entry.hash;
                invalid[0] ^= 1;
                assert!(
                    super::super::npc_rules::legacy_defaults(&extended, quest, &invalid).is_none()
                );
                assert!(
                    super::super::npc_rules::legacy_defaults(
                        &extended,
                        "oteryn:quest.unknown_predecessor",
                        &entry.hash,
                    )
                    .is_none()
                );
            }
        }
        Ok(())
    }

    #[test]
    fn reward_amount_is_part_of_the_quest_definition_hash() -> Result<(), String> {
        let before = parse(include_str!(
            "../../../../../content/quests/chosen-reward-callbacks.json"
        ))?;
        let mut changed = before.clone();
        changed
            .first_mut()
            .and_then(|c| c.give.as_mut())
            .ok_or("reward")?
            .count = 51;
        let empty = receipt_test_base::catalogue;
        let before = extend_callbacks(
            empty()?,
            before,
            parse_document(include_str!(
                "../../../../../content/quests/chosen-reward-callbacks.json"
            ))?
            .tracks,
        )?;
        let changed = extend_callbacks(
            empty()?,
            changed,
            parse_document(include_str!(
                "../../../../../content/quests/chosen-reward-callbacks.json"
            ))?
            .tracks,
        )?;
        assert_ne!(
            before.definition_hash(super::super::grizzly_adams::QUEST),
            changed.definition_hash(super::super::grizzly_adams::QUEST),
            "a changed payout must not reuse a running quest's pinned value plan"
        );
        Ok(())
    }

    #[test]
    fn reward_receipt_predicates_require_the_same_quest_and_declared_value() -> Result<(), String> {
        let base = receipt_test_base::catalogue()?;
        // Include every receipt-gated callback and its production receipt owner;
        // unrelated participant prerequisites are covered separately.
        let callbacks: Vec<_> = load()?
            .into_iter()
            .filter(|callback| !callback.required_receipts.is_empty())
            .collect();
        assert!(!callbacks.is_empty());
        let extend_receipts = |base| extend_callbacks(base, callbacks.clone(), vec![]);
        extend_receipts(base.clone())?;
        let transition = "oteryn:quest-transition/grizzly/report-boss-1";
        let track = super::super::grizzly_adams::storage_track(34100);
        let mut foreign_transition = base.clone();
        foreign_transition
            .transitions
            .get_mut(transition)
            .ok_or("report")?
            .quest = "oteryn:quest.foreign".into();
        assert!(extend_receipts(foreign_transition).is_err());
        let mut foreign_track = base.clone();
        foreign_track
            .tracks
            .get_mut(&track)
            .ok_or("boss track")?
            .quest = "oteryn:quest.foreign".into();
        assert!(extend_receipts(foreign_track).is_err());
        let mut bounded = base;
        bounded.tracks.get_mut(&track).ok_or("boss track")?.max = 2;
        assert!(extend_receipts(bounded).is_err());
        Ok(())
    }
}

#[cfg(test)]
mod lane_counter_tests {
    use super::*;
    #[test]
    fn service_counter_uses_real_add_owner_and_refuses_at_its_bound() -> Result<(), String> {
        let mut callback = load()?
            .into_iter()
            .find(|c| c.key.ends_with("questpc7-friends-royal-steel"))
            .ok_or("service")?;
        callback.counter.as_mut().ok_or("counter")?.maximum = 2;
        let counter = callback.counter.as_ref().ok_or("counter")?.track.clone();
        let parent = callback.storage[0].track.clone();
        let catalogue = extend_fixture_callbacks(
            QuestStateCatalogue::new("counter-test", vec![], vec![])
                .map_err(|e| format!("{e:?}"))?,
            std::slice::from_ref(&callback),
        )?;
        let transition = catalogue.transition(&callback.key).ok_or("transition")?;
        assert_eq!(
            *transition,
            primary_transition(&callback)?,
            "bound admission must validate the same counter operation as lowering"
        );
        let mut stored = BTreeMap::from([(parent.clone(), 1)]);
        for value in 1..=2 {
            let changes = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|e| format!("{e:?}"))?;
            let change = changes
                .iter()
                .find(|c| c.track == counter)
                .ok_or("missing counted receipt")?;
            assert_eq!((change.before, change.after), (value - 1, value));
            assert!(
                changes
                    .iter()
                    .any(|c| c.track == parent && c.before == 1 && c.after == 1)
            );
            stored.insert(counter.clone(), value);
        }
        assert!(
            catalogue.evaluate(transition, &stored, 0).is_err(),
            "exhausted counter cannot overflow or consume another input"
        );
        for maximum in [0, 1_000_000_000_001] {
            let mut invalid = callback.clone();
            invalid.counter.as_mut().ok_or("counter")?.maximum = maximum;
            assert!(
                parse(&serde_json::json!({"schema":1,"callbacks":[invalid]}).to_string()).is_err()
            );
        }
        callback.counter.as_mut().ok_or("counter")?.track = parent;
        assert!(
            parse(&serde_json::json!({"schema":1,"callbacks":[callback]}).to_string()).is_err(),
            "counter cannot take over its prerequisite track"
        );
        Ok(())
    }
    #[test]
    fn npc_rewards_reject_mixed_or_unbounded_backpack_outputs() -> Result<(), String> {
        let mut document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/shared-reward-callbacks.json"
        ))
        .map_err(|error| error.to_string())?;
        document["callbacks"][0]["outfit"] = serde_json::Value::Null;
        let valid = serde_json::json!({"item":"oteryn:item.tibia.i2853","count":1,
            "contents":[],"backpack_outputs":[{"item":"oteryn:item.tibia.i3034","count":1}]});
        document["callbacks"][0]["give"] = valid.clone();
        let parsed = parse(&document.to_string());
        assert!(parsed.is_ok(), "bounded backpack delivery: {parsed:?}");
        let mut mixed = valid.clone();
        mixed["contents"] = serde_json::json!([{"item":"oteryn:item.tibia.i3031","count":1}]);
        let mut excessive = valid.clone();
        excessive["backpack_outputs"] = serde_json::json!(vec![
            serde_json::json!({"item":"oteryn:item.tibia.i3034","count":1});
            20
        ]);
        let mut empty_quantity = valid;
        empty_quantity["backpack_outputs"][0]["count"] = serde_json::json!(0);
        for invalid in [mixed, excessive, empty_quantity] {
            document["callbacks"][0]["give"] = invalid;
            assert!(parse(&document.to_string()).is_err());
        }
        Ok(())
    }
    #[test]
    fn quest_fees_preserve_free_callback_identity_and_reject_coin_double_charge()
    -> Result<(), String> {
        let source = include_str!("../../../../../content/quests/shared-reward-callbacks.json");
        let mut document: serde_json::Value =
            serde_json::from_str(source).map_err(|error| error.to_string())?;
        let free = parse(source)?.remove(0);
        assert!(free.fee.is_none());
        assert!(
            serde_json::to_value(&free)
                .map_err(|error| error.to_string())?
                .get("fee")
                .is_none()
        );
        document["callbacks"][0]["fee"] = serde_json::json!({"gold":100,"include_bank":true});
        assert!(parse(&document.to_string()).is_ok());
        for gold in [0_u64, 1_000_000_000_001] {
            let mut invalid = document.clone();
            invalid["callbacks"][0]["fee"]["gold"] = gold.into();
            assert!(parse(&invalid.to_string()).is_err());
        }
        for coin in [3031, 3035, 3043] {
            let mut invalid = document.clone();
            invalid["callbacks"][0]["take"] = serde_json::json!([
                {"item":format!("oteryn:item.tibia.i{coin}"),"count":1}
            ]);
            assert!(parse(&invalid.to_string()).is_err());
        }
        Ok(())
    }

    fn state_ops_callback() -> Result<RewardCallback, String> {
        serde_json::from_value(serde_json::json!({
            "key":"oteryn:quest-reward/state_ops_test/start","quest":"oteryn:quest.state_ops_test",
            "npc":"oteryn:npc.test","keywords":["start"],"topic":0,"next_topic":0,
            "storage":[{"track":"oteryn:quest-progress/state_ops_test/stage","minimum":-1,"maximum":-1,"set":1}],
            "take":[],"give":null,"outfit":null,"achievement":null,"reply":["Start."],"sources":["test:state-ops"]
        })).map_err(|error|error.to_string())
    }

    fn clock_declaration() -> TrackDeclaration {
        TrackDeclaration {
            writable: true,
            key: "oteryn:quest-progress/state_ops_test/clock".into(),
            quest: "oteryn:quest.state_ops_test".into(),
            initial: 0,
            min: 0,
            max: i64::MAX,
            sources: vec!["test:clock".into()],
        }
    }

    fn callback_hash(
        callback: RewardCallback,
        declarations: Vec<TrackDeclaration>,
    ) -> Result<String, String> {
        let quest = callback.quest.clone();
        let catalogue = extend_callbacks(
            QuestStateCatalogue::empty("content-1").ok_or("empty catalogue")?,
            vec![callback],
            declarations,
        )?;
        Ok(catalogue
            .definition_hash(&quest)
            .ok_or("definition hash")?
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }

    #[test]
    fn free_legacy_hash_is_stable_and_fees_premium_operations_and_guards_are_bound()
    -> Result<(), String> {
        let legacy = state_ops_callback()?;
        // Snapshot computed from the pre-port reward tuple and version2 quest encoding.
        let baseline = callback_hash(legacy.clone(), Vec::new())?;
        assert_eq!(
            baseline,
            "4a17ff7834109f75619e0aac06f022d0485ef3a7f9ca89973479c244c4d7ffea"
        );
        let encoded = serde_json::to_value(&legacy).map_err(|error| error.to_string())?;
        for omitted in ["state_ops", "fee", "requires_premium"] {
            assert!(encoded.get(omitted).is_none());
        }
        let mut paid = legacy.clone();
        paid.fee = Some(MonetaryFee {
            gold: 50,
            include_bank: false,
        });
        let first = callback_hash(paid.clone(), Vec::new())?;
        assert_ne!(first, baseline);
        paid.fee.as_mut().ok_or("fee")?.gold = 51;
        assert_ne!(callback_hash(paid.clone(), Vec::new())?, first);
        paid.fee = Some(MonetaryFee {
            gold: 50,
            include_bank: true,
        });
        assert_ne!(callback_hash(paid, Vec::new())?, first);
        let mut premium = legacy.clone();
        premium.requires_premium = true;
        assert_ne!(callback_hash(premium, Vec::new())?, baseline);
        let clock = clock_declaration();
        let mut operation = legacy;
        operation.state_ops.push(StateOperation {
            track: clock.key.clone(),
            from: StateCondition::ElapsedLessThan { seconds: 1800 },
            effect: StateMutation::Add { value: 0 },
        });
        let with_guard = callback_hash(operation.clone(), vec![clock.clone()])?;
        operation.state_ops[0].from = StateCondition::ElapsedLessThan { seconds: 1801 };
        assert_ne!(
            callback_hash(operation.clone(), vec![clock.clone()])?,
            with_guard
        );
        operation.state_ops[0].from = StateCondition::ElapsedLessThan { seconds: 1800 };
        operation.state_ops[0].effect = StateMutation::SetNow;
        assert_ne!(callback_hash(operation, vec![clock])?, with_guard);
        Ok(())
    }

    #[test]
    fn state_operations_require_exact_writable_owned_declarations() -> Result<(), String> {
        let mut callback = state_ops_callback()?;
        let clock = clock_declaration();
        callback.state_ops.push(StateOperation {
            track: clock.key.clone(),
            from: StateCondition::Any,
            effect: StateMutation::SetNow,
        });
        assert!(callback_hash(callback.clone(), Vec::new()).is_err());
        callback_hash(callback.clone(), vec![clock.clone()])?;
        let mut foreign = clock.clone();
        foreign.quest = "oteryn:quest.other".into();
        assert!(callback_hash(callback.clone(), vec![foreign]).is_err());
        let mut untyped = clock.clone();
        untyped.initial = -1;
        untyped.min = -1;
        assert!(callback_hash(callback.clone(), vec![untyped]).is_err());
        let serialized = serde_json::to_value(&callback).map_err(|error| error.to_string())?;
        let mut document = serde_json::json!({"schema":1,"tracks":[{"writable":true,"key":clock.key,"quest":clock.quest,"initial":0,"min":0,"max":i64::MAX,"sources":["test:clock"]}],"callbacks":[serialized]});
        parse_document(&document.to_string())?;
        for edit in 0..4 {
            let mut invalid = document.clone();
            match edit {
                0 => invalid["tracks"][0]["writable"] = false.into(),
                1 => invalid["tracks"][0]["quest"] = "oteryn:quest.other".into(),
                2 => invalid["tracks"][0]["initial"] = (-1).into(),
                _ => {
                    invalid["callbacks"][0]["state_ops"][0]["track"] =
                        invalid["callbacks"][0]["storage"][0]["track"].clone()
                }
            }
            assert!(
                parse_document(&invalid.to_string()).is_err(),
                "invalid declaration/operator case {edit}"
            );
        }
        document["tracks"] = serde_json::json!([]);
        let undeclared = parse_document(&document.to_string())?;
        assert!(
            extend_callbacks(
                QuestStateCatalogue::empty("content-1").ok_or("empty")?,
                undeclared.callbacks,
                undeclared.tracks
            )
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn published_marker_compatibility_keeps_new_value_terms_bound() -> Result<(), String> {
        let published = published_markers::keys()?;
        let callback = parse(DOCUMENTS[0])?
            .into_iter()
            .find(|callback| !callback.storage.is_empty() && callback.give.is_some())
            .ok_or("published item callback")?;
        assert!(!published_markers::needs_marker(&callback, &published));
        let before = callback_hash(callback.clone(), Vec::new())?;
        let mut occurrence = callback.clone();
        occurrence.per_occurrence_mint = true;
        assert!(published_markers::needs_marker(&occurrence, &published));
        assert_ne!(callback_hash(occurrence, Vec::new())?, before);
        let mut carried = callback.clone();
        carried.carried_exchange = true;
        assert!(published_markers::needs_marker(&carried, &published));
        let mut outputs = callback.clone();
        outputs
            .give
            .as_mut()
            .ok_or("give")?
            .backpack_outputs
            .push(ItemAmount {
                item: "oteryn:item.tibia.i3031".into(),
                count: 1,
                fluid_subtype: None,
            });
        assert!(published_markers::needs_marker(&outputs, &published));
        let mut receipt = callback;
        receipt.required_receipts.push(ReceiptPredicate {
            transition: "oteryn:quest-reward/fixture".into(),
            track: "oteryn:quest-progress/fixture/stage".into(),
            value: 1,
        });
        assert!(published_markers::needs_marker(&receipt, &published));
        let chosen = parse(include_str!(
            "../../../../../content/quests/chosen-reward-callbacks.json"
        ))?
        .into_iter()
        .next()
        .ok_or("chosen")?;
        assert!(published_markers::needs_marker(&chosen, &published));
        assert!(published_markers::needs_marker(
            &state_ops_callback()?,
            &published
        ));
        Ok(())
    }

    #[test]
    fn occurrence_mint_is_explicit_portable_and_part_of_the_value_hash() -> Result<(), String> {
        let mut callback = load()?
            .into_iter()
            .find(|c| c.key.ends_with("/venore-daily/accept"))
            .ok_or("accept")?;
        let legacy = value_plan(&callback)?;
        let encoded = serde_json::to_value(&callback).map_err(|e| e.to_string())?;
        assert!(encoded.get("per_occurrence_mint").is_none());
        callback.per_occurrence_mint = true;
        assert_ne!(legacy, value_plan(&callback)?);
        let document = serde_json::json!({"schema":1,"callbacks":[callback]});
        assert!(parse(&document.to_string()).is_ok());
        for case in 0..3 {
            let mut bad = document.clone();
            match case {
                0 => bad["callbacks"][0]["give"] = serde_json::Value::Null,
                1 => {
                    bad["callbacks"][0]["outfit"] =
                        serde_json::json!({"outfit":"oteryn:outfit.revenant","addons":0})
                }
                _ => bad["callbacks"][0]["per_occurrence_mint"] = "true".into(),
            }
            assert!(
                parse(&bad.to_string()).is_err(),
                "occurrence option case {case}"
            );
        }
        callback.per_occurrence_mint = false;
        assert_eq!(legacy, value_plan(&callback)?);
        Ok(())
    }
}

#[cfg(test)]
mod world_track_owner_tests {
    use super::*;

    #[test]
    fn world_only_track_declarations_preserve_exact_quest_ownership() -> Result<(), String> {
        let source = include_str!("../../../../../content/quests/chosen-reward-callbacks.json");
        let document = parse_document(source)?;
        assert!(
            document
                .tracks
                .iter()
                .any(|track| track.key == "oteryn:quest-progress/chosen/winterlight/stage")
        );
        let mut wrong: serde_json::Value =
            serde_json::from_str(source).map_err(|e| e.to_string())?;
        let track = wrong["tracks"]
            .as_array_mut()
            .ok_or("tracks")?
            .iter_mut()
            .find(|track| track["key"] == "oteryn:quest-progress/chosen/winterlight/stage")
            .ok_or("stage")?;
        track["quest"] = "oteryn:quest.authored.venore_daily_tasks_quest".into();
        let wrong = serde_json::to_string(&wrong).map_err(|e| e.to_string())?;
        assert!(
            parse_document(&wrong).is_err(),
            "world/death ownership cannot be reassigned by a declaration"
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "reward_callbacks/venore_tests.rs"]
mod venore_tests;

#[cfg(test)]
mod qnpcfx_tests;

#[cfg(test)]
mod m10_magic_tests;

#[cfg(test)]
#[path = "reward_callbacks/readable_text_tests.rs"]
mod readable_text_tests;

#[cfg(test)]
#[path = "reward_callbacks/map_mark_tests.rs"]
mod map_mark_tests;

#[cfg(test)]
mod multiple_rewards_tests;

#[cfg(test)]
mod outfit_reward_predecessor_tests {
    use super::*;

    const QUEST: &str = "oteryn:quest.outfit_and_addon_quests";

    fn fixture() -> Result<(QuestStateCatalogue, QuestStateCatalogue), String> {
        let lowered = super::super::loader::parse_quest_state(
            include_str!("../../../../../content/quests/missions/quest-state.json"),
            "outfit-predecessor",
        )
        .map_err(|error| format!("{error:?}"))?;
        let bindings = lowered
            .catalogue()
            .tracks
            .keys()
            .filter_map(|key| {
                lowered
                    .source_track(key)
                    .map(|source| (key.clone(), source.to_owned()))
            })
            .collect();
        let before = declare_tracks(lowered.catalogue().clone())?;
        let active = super::super::npc_rules::extend(before.clone(), &bindings)?;
        Ok((before, active))
    }

    fn complete_rewards(before: &QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
        let document = embedded_document()?;
        let owned = document
            .callbacks
            .iter()
            .filter(|callback| callback.quest == QUEST)
            .count();
        assert_eq!(owned, 112);
        let participants = document
            .callbacks
            .iter()
            .flat_map(|callback| {
                callback
                    .participants
                    .iter()
                    .enumerate()
                    .filter(|(_, participant)| participant.quest == QUEST)
                    .map(move |(index, _)| participant_key(&callback.key, index))
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            participants,
            BTreeSet::from([
                "oteryn:quest-reward/l5-norseman-base/participant/0".into(),
                "oteryn:quest-reward/l5-norseman-first-reward/participant/0".into(),
                "oteryn:quest-reward/l5-norseman-second-reward/participant/0".into(),
                "oteryn:quest-reward/l6-citizen-rook-backpack/participant/0".into(),
                "oteryn:quest-reward/l6-citizen-rook-backpack-start/participant/0".into(),
            ])
        );
        extend_callbacks(
            before.clone(),
            document
                .callbacks
                .into_iter()
                .filter(|callback| {
                    callback.quest == QUEST
                        || callback
                            .participants
                            .iter()
                            .any(|participant| participant.quest == QUEST)
                })
                .collect(),
            document.tracks,
        )
    }

    #[test]
    fn outfit_reward_predecessors_keep_full_pins_defaults_and_active_definitions()
    -> Result<(), String> {
        let (before, mut active) = fixture()?;
        let old_active = active.clone();
        let declared = active
            .npc_legacy
            .get(QUEST)
            .ok_or("captured Outfit")?
            .clone();
        let rewards = complete_rewards(&before)?;
        let full_hash = rewards.definition_hash(QUEST).ok_or("full Outfit")?;
        let full_defaults = rewards
            .tracks
            .values()
            .filter(|track| track.quest == QUEST)
            .map(|track| (track.key.clone(), track.initial))
            .collect::<BTreeMap<_, _>>();
        let hex = |hash: &[u8; 32]| {
            hash.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        assert_eq!(
            hex(&declared.0),
            "8ae9fff068d9af51c1be88777aad3ac82e5c6551314a5ac185b17ce766a6ef1f"
        );
        assert_eq!(
            hex(&full_hash),
            "bd44ba2f572315de58bc53958ee1bf398a7039ae5badf3e796017965d051f6c8"
        );
        assert_eq!((declared.1.len(), full_defaults.len()), (51, 52));
        assert!(
            declared
                .1
                .iter()
                .all(|(key, value)| full_defaults.get(key) == Some(value))
        );
        let navigator = "oteryn:quest-progress/navigator";
        assert!(!declared.1.contains_key(navigator));
        assert_eq!(full_defaults.get(navigator), Some(&-1));

        retain_outfit_reward_predecessor(before.clone(), &mut active)?;
        assert_eq!(
            active.npc_legacy.get(QUEST),
            Some(&(full_hash, full_defaults.clone()))
        );
        for (hash, defaults) in [(&declared.0, &declared.1), (&full_hash, &full_defaults)] {
            assert_eq!(
                super::super::npc_rules::legacy_defaults(&active, QUEST, hash),
                Some(defaults)
            );
            let mut altered = *hash;
            altered[0] ^= 1;
            assert!(super::super::npc_rules::legacy_defaults(&active, QUEST, &altered).is_none());
            for foreign in [
                "oteryn:quest.the_ice_islands_quest",
                "oteryn:quest.foreign",
                "oteryn:quest.dawnport_quest",
            ] {
                assert!(super::super::npc_rules::legacy_defaults(&active, foreign, hash).is_none());
            }
            // The durable consumers fill only missing rows with these defaults.
            let track = rewards
                .tracks
                .values()
                .find(|track| {
                    track.quest == QUEST
                        && defaults.contains_key(&track.key)
                        && track.initial != track.max
                })
                .ok_or("existing progress track")?;
            let key = &track.key;
            let mut stored = BTreeMap::from([(key.clone(), track.max)]);
            for (key, value) in defaults {
                stored.entry(key.clone()).or_insert(*value);
            }
            assert_eq!(stored.get(key), Some(&track.max));
            assert!(
                defaults
                    .iter()
                    .filter(|(other, _)| *other != key)
                    .all(|(key, value)| stored.get(key) == Some(value))
            );
        }

        let mut without_compatibility = active.clone();
        without_compatibility.npc_legacy = old_active.npc_legacy.clone();
        without_compatibility.predecessors = old_active.predecessors.clone();
        assert_eq!(without_compatibility, old_active);
        assert_eq!(
            active.npc_legacy.keys().collect::<Vec<_>>(),
            old_active.npc_legacy.keys().collect::<Vec<_>>()
        );
        let mut old_other = old_active.npc_legacy.clone();
        let mut new_other = active.npc_legacy.clone();
        old_other.remove(QUEST);
        new_other.remove(QUEST);
        assert_eq!(new_other, old_other);
        let mut old_other = old_active.predecessors.clone();
        let mut new_other = active.predecessors.clone();
        old_other.remove(QUEST);
        new_other.remove(QUEST);
        assert_eq!(new_other, old_other);
        let once = active.clone();
        retain_outfit_reward_predecessor(before, &mut active)?;
        assert_eq!(active, once);
        Ok(())
    }

    #[test]
    fn outfit_reward_predecessor_refuses_foreign_or_incomplete_capture_atomically()
    -> Result<(), String> {
        let (before, active) = fixture()?;
        let declared = active
            .npc_legacy
            .get(QUEST)
            .ok_or("captured Outfit")?
            .clone();
        let mut accepted = active.clone();
        retain_outfit_reward_predecessor(before.clone(), &mut accepted)?;
        for case in 0..7 {
            let mut invalid = active.clone();
            match case {
                0 => {
                    invalid.npc_legacy.remove(QUEST);
                }
                1 => {
                    invalid.npc_legacy.get_mut(QUEST).ok_or("capture")?.0[0] ^= 1;
                }
                2 => {
                    *invalid
                        .npc_legacy
                        .get_mut(QUEST)
                        .ok_or("capture")?
                        .1
                        .values_mut()
                        .next()
                        .ok_or("default")? += 1;
                }
                3 => {
                    let mut defaults = declared.1.clone();
                    defaults.insert("oteryn:quest-progress/navigator".into(), 0);
                    invalid.predecessors.entry(QUEST.into()).or_default().push(
                        super::super::NpcDefinitionPredecessor {
                            hash: declared.0,
                            defaults,
                        },
                    );
                }
                4 => {
                    invalid
                        .npc_legacy
                        .insert(QUEST.into(), accepted.npc_legacy[QUEST].clone());
                }
                5 => {
                    invalid.content_revision = "foreign-content".into();
                }
                _ => {
                    invalid.hashes.insert(QUEST.into(), declared.0);
                }
            }
            let unchanged = invalid.clone();
            assert!(
                retain_outfit_reward_predecessor(before.clone(), &mut invalid).is_err(),
                "case {case}"
            );
            assert_eq!(invalid, unchanged, "case {case}");
        }
        Ok(())
    }
}

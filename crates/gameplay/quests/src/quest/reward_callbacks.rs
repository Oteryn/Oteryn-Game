//! Bounded NPC exchanges composed by the fenced quest writer, never by dialogue text.
use super::world_callbacks::Storage;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

mod state_ops;
pub use state_ops::{StateCondition, StateMutation, StateOperation};

/// The same bounded complete Item output is admitted for NPC and world rewards.
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
        && reward.contents.len() <= 20
        && reward.backpack_outputs.len() <= 19
        && (reward.contents.is_empty() || reward.backpack_outputs.is_empty())
        && reward
            .contents
            .iter()
            .chain(&reward.backpack_outputs)
            .all(|entry| item(&entry.item, entry.count))
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn reward_binding(callback: &RewardCallback) -> Result<[u8; 32], String> {
    // Speech and source citations are not durable reward terms. Every value and
    // admission participant is: changing one must invalidate an old quest pin.
    let mut bytes = serde_json::to_vec(&(
        &callback.key,
        &callback.quest,
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
        &callback.blessing,
        callback.experience,
        callback.premium,
        callback.completes,
    ))
    .map_err(|error| error.to_string())?;
    // Preserve every published binding for callbacks without a level gate.
    if let Some(level) = callback.minimum_level {
        bytes.extend_from_slice(b"\0minimum_level\0");
        bytes.extend_from_slice(&level.to_be_bytes());
    }
    let mut digest = Sha256::new();
    digest.update(bytes);
    // Empty optional predicates retain the original binding for unrelated
    // callbacks. Nonempty terms extend it in a disjoint domain (JSON contains
    // no literal NUL), so old in-flight receipts keep their exact definitions.
    if !callback.equipped_slots.is_empty() || callback.repeatable_check {
        digest.update(b"\0quest-reward-equipment-check-v1\0");
        digest.update(
            serde_json::to_vec(&(&callback.equipped_slots, callback.repeatable_check))
                .map_err(|error| error.to_string())?,
        );
    }
    if let Some(amount) = callback.health_loss {
        digest.update(b"\0quest-reward-health-loss-v1\0");
        digest.update(amount.to_be_bytes());
    }
    if !callback.present_any.is_empty() {
        digest.update(b"\0quest-reward-present-any-v1\0");
        digest
            .update(serde_json::to_vec(&callback.present_any).map_err(|error| error.to_string())?);
    }
    if !callback.absent.is_empty() {
        digest.update(b"\0quest-reward-owned-absence-v1\0");
        digest.update(serde_json::to_vec(&callback.absent).map_err(|error| error.to_string())?);
    }
    if let Some(fee) = &callback.fee {
        digest.update(b"\0quest-reward-fee-v1\0");
        digest.update(serde_json::to_vec(fee).map_err(|error| error.to_string())?);
    }
    if let Some(item) = &callback.carried_item {
        digest.update(b"\0quest-reward-carried-use-v1\0");
        digest.update(item.as_bytes());
    }
    if callback.repeatable {
        digest.update(b"\0quest-reward-repeatable-v1\0");
    }
    if !callback.state_ops.is_empty()
        || !callback.participants.is_empty()
        || callback.creation_sex.is_some()
        || callback.requires_premium
        || !callback.requires_outfits.is_empty()
        || !callback.requires_addons.is_empty()
        || !callback.forbids_outfits.is_empty()
        || !callback.requires_actor_conditions.is_empty()
        || !callback.forbids_addons.is_empty()
        || !callback.requires.is_empty()
        || !callback.outfits.is_empty()
        || callback.exact_take
        || !callback.achievement_progress.is_empty()
    {
        digest.update(b"\0quest-reward-additional-terms-v1\0");
        digest.update(
            serde_json::to_vec(&(
                &callback.state_ops,
                &callback.participants,
                callback.creation_sex,
                callback.requires_premium,
                &callback.requires_outfits,
                &callback.requires_addons,
                &callback.forbids_outfits,
                &callback.requires_actor_conditions,
                &callback.forbids_addons,
                &callback.requires,
                &callback.outfits,
                callback.exact_take,
                &callback.achievement_progress,
            ))
            .map_err(|e| e.to_string())?,
        );
    }
    Ok(digest.finalize().into())
}

pub const PROVENANCE: &str = "crystal-9f5a72c-quest-rewards-v1";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemAmount {
    pub item: String,
    pub count: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemReward {
    pub item: String,
    pub count: u16,
    #[serde(default)]
    pub contents: Vec<ItemAmount>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backpack_outputs: Vec<ItemAmount>,
}

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

/// One paid source occurrence advances the existing Character achievement owner.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AchievementProgress {
    pub key: String,
    pub target: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardCallback {
    pub key: String,
    pub quest: String,
    pub npc: String,
    /// Ordinary carried USE binding; NPC dialogue cannot execute this callback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carried_item: Option<String>,
    pub keywords: Vec<String>,
    pub topic: u16,
    pub next_topic: u16,
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
    pub take: Vec<ItemAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee: Option<MonetaryFee>,
    /// Carried evidence checked under the same locks, without consuming it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present: Vec<ItemAmount>,
    pub give: Option<ItemReward>,
    pub outfit: Option<OutfitReward>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outfits: Vec<OutfitReward>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exact_take: bool,
    pub achievement: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub achievement_progress: Vec<AchievementProgress>,
    pub reply: Vec<String>,
    pub sources: Vec<String>,
    /// Positive carried costs permit a fresh command occurrence of this exchange.
    #[serde(default, skip_serializing_if = "is_false")]
    pub repeatable: bool,
    /// Awarded by the existing fenced Quest XP obligation owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<i64>,
    /// At least one complete carried requirement must hold; inputs remain untouched.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present_any: Vec<ItemAmount>,
    /// Definitions that must have no live instance in complete owned custody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub absent: Vec<String>,
    /// Read-only canonical durable equipment slots, checked under the Character root lock.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub equipped_slots: Vec<u8>,
    /// A repeated source check may never grant or consume durable value.
    #[serde(default, skip_serializing_if = "is_false")]
    pub repeatable_check: bool,
    /// A canonical blessing Service, granted through the existing blessing owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blessing: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub premium: bool,
    /// Source health loss, staged and installed by the existing live vitals owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_loss: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub completes: bool,
}

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
    pub fn cosmetic_targets(&self) -> impl Iterator<Item = &OutfitReward> {
        self.outfit.iter().chain(&self.outfits)
    }
    /// Storage-free paid exchanges can repeat under distinct CommandRefs.
    pub fn is_paid_exchange(&self) -> bool {
        self.storage.is_empty()
            && self.state_ops.is_empty()
            && self.participants.is_empty()
            && !self.take.is_empty()
            && self.give.is_some()
    }
    fn value_only_guarded(&self) -> bool {
        // Repeatable source trades spend exact admitted inputs and produce a
        // native backpack MINT in the same fenced transaction.
        self.repeatable_check
            || self.repeatable
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
    tracks: Vec<TrackDeclaration>,
    callbacks: Vec<RewardCallback>,
}

pub fn load() -> Result<Vec<RewardCallback>, String> {
    parse(include_str!(
        "../../../../../content/quests/shared-reward-callbacks.json"
    ))
}

fn parse(text: &str) -> Result<Vec<RewardCallback>, String> {
    parse_document(text).map(|document| document.callbacks)
}

fn validate_additional_terms(c: &RewardCallback) -> Result<(), String> {
    let slug = |value: &str, prefix: &str, alphabet: &[u8], maximum: usize| {
        value.len() <= maximum
            && value.strip_prefix(prefix).is_some_and(|tail| {
                !tail.is_empty()
                    && tail.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || alphabet.contains(&b)
                    })
            })
    };

    let alternatives: BTreeSet<_> = c.present_any.iter().map(|s| &s.item).collect();
    let absent: BTreeSet<_> = c.absent.iter().collect();
    let refill = c.absent.len() == 1
        && c.give.as_ref().is_some_and(|give| {
            c.absent.contains(&give.item)
                && give.contents.is_empty()
                && give.backpack_outputs.is_empty()
        });

    if ((c.repeatable || c.repeatable_check) && c.storage.is_empty())
        || c.storage
            .iter()
            .any(|s| s.increment.is_some() || s.set_when_at_least.is_some())
        || (c.repeatable
            && ((c.take.is_empty() && !refill)
                || c.give.is_none()
                || c.fee.is_some()
                || c.outfit.is_some()
                || !c.outfits.is_empty()
                || c.achievement.is_some()
                || !c.achievement_progress.is_empty()
                || c.blessing.is_some()
                || c.experience.is_some()
                || c.health_loss.is_some()
                || c.completes
                || c.repeatable_check))
        || (c.repeatable_check
            && (!c.take.is_empty()
                || c.fee.is_some()
                || c.give.is_some()
                || c.outfit.is_some()
                || !c.outfits.is_empty()
                || c.achievement.is_some()
                || !c.achievement_progress.is_empty()
                || c.blessing.is_some()
                || c.experience.is_some()
                || c.health_loss.is_some()
                || c.completes
                || c.storage
                    .iter()
                    .any(|s| !s.preserve && (s.minimum != s.maximum || s.set != s.minimum))))
        || c.equipped_slots.len() > 9
        || c.equipped_slots.iter().collect::<BTreeSet<_>>().len() != c.equipped_slots.len()
        || c.equipped_slots
            .iter()
            .any(|s| !(1..=10).contains(s) || *s == 9)
        || c.health_loss == Some(0)
        || c.experience
            .is_some_and(|v| !(1..=super::QUESTGATE0_RL_05).contains(&v))
        || c.absent.len() > 8
        || absent.len() != c.absent.len()
        || (!c.absent.is_empty() && c.give.is_none())
        || c.absent.iter().any(|key| {
            !slug(key, "oteryn:item.", b"._/:-", 256)
                || c.take
                    .iter()
                    .chain(&c.present)
                    .chain(&c.present_any)
                    .any(|item| &item.item == key)
        })
        || (c.repeatable && refill && (!c.participants.is_empty() || !c.state_ops.is_empty()))
        || c.present_any.len() > 8
        || alternatives.len() != c.present_any.len()
        || c.present_any
            .iter()
            .any(|s| !slug(&s.item, "oteryn:item.", b"._/:-", 256) || s.count == 0)
        || c.blessing
            .as_ref()
            .is_some_and(|key| !slug(key, "oteryn:service.blessing.", b"_", 160))
    {
        return Err(format!("invalid additional reward terms {}", c.key));
    }
    Ok(())
}

fn parse_document(text: &str) -> Result<Document, String> {
    let document: Document = serde_json::from_str(text).map_err(|e| e.to_string())?;
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
    if document.schema != 1 || document.callbacks.len() > 512 || document.tracks.len() > 256 {
        return Err("reward callback bounds".into());
    }
    let mut declarations = BTreeSet::new();
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
                    .filter(|(_, key)| *key == track.key)
                    .map(|(owner, _)| owner)
                    .collect();
                owners.is_empty()
                    || owners
                        .iter()
                        .any(|owner| owner.as_str() != track.quest.as_str())
            } else {
                !document
                    .callbacks
                    .iter()
                    .any(|c| c.requires.iter().any(|r| r.track == track.key))
                    || document.callbacks.iter().any(|c| {
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
        validate_additional_terms(c)?;
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
            || c.storage.len() + c.state_ops.len() + participant_count > 8
            || c.participants.iter().any(|p| {
                p.quest == c.quest
                    || !slug(&p.quest, "oteryn:", b"._/:-", 128)
                    || (p.storage.is_empty() && p.state_ops.is_empty())
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
        if !slug(&c.key, "oteryn:quest-reward/", b"_/-", 128)
            || !slug(&c.quest, "oteryn:", b"._/:-", 128)
            || !keys.insert(&c.key)
            || !slug(&c.npc, "oteryn:npc.", b"_", 128)
            || c.carried_item.as_ref().is_some_and(|item| {
                if c.repeatable {
                    !c.take
                        .iter()
                        .any(|take| &take.item == item && take.count == 1)
                        || c.give.is_none()
                        || c.fee.is_some()
                        || c.health_loss.is_some()
                        || !c.requires_actor_conditions.is_empty()
                } else {
                    c.take
                        != [ItemAmount {
                            item: item.clone(),
                            count: 1,
                        }]
                        || c.give.is_some()
                        || c.fee.is_some()
                        || !c.requires_actor_conditions.is_empty()
                }
            })
            || c.forbids_outfits.len() > 8
            || c.forbids_outfits.iter().collect::<BTreeSet<_>>().len() != c.forbids_outfits.len()
            || c.forbids_outfits
                .iter()
                .any(|key| !slug(key, "oteryn:outfit.", b"_", 128))
            || (c.storage.is_empty() && c.state_ops.is_empty() && !c.value_only_guarded())
            || c.storage.len() > 8
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
            || (!c.value_only_guarded()
                && c.storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set)))
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
            || c.take.len() > 16
            || inputs.len() != c.take.len()
            || c.take
                .iter()
                .any(|s| !slug(&s.item, "oteryn:item.", b"._/:-", 256) || s.count == 0)
            || c.give.as_ref().is_some_and(|r| !valid_item_reward(r))
            || (!c.outfits.is_empty() && c.outfit.is_some())
            || c.outfits.len() > 4
            || c.cosmetic_targets()
                .map(|r| (&r.outfit, r.addons, r.sex))
                .collect::<BTreeSet<_>>()
                .len()
                != c.cosmetic_targets().count()
            || c.cosmetic_targets()
                .any(|r| !slug(&r.outfit, "oteryn:outfit.", b"_", 160) || r.addons > 3)
            || c.keywords.is_empty()
            || c.keywords.len() > 8
            || c.keywords.iter().collect::<BTreeSet<_>>().len() != c.keywords.len()
            || c.keywords.iter().any(|s| {
                s.is_empty() || s.len() > 64 || !s.is_ascii() || s != &s.to_ascii_lowercase()
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

pub fn participant_key(callback: &str, index: usize) -> String {
    format!("{callback}/participant/{index}")
}

pub fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    let document = parse_document(include_str!(
        "../../../../../content/quests/shared-reward-callbacks.json"
    ))?;
    for declaration in document.tracks {
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
    let callbacks = document.callbacks;
    let requires: Vec<_> = callbacks.iter().flat_map(|c| &c.requires).collect();
    for callback in &callbacks {
        let binding = callback
            .key
            .starts_with("oteryn:quest-reward/questpc3/")
            .then(|| reward_binding(callback))
            .transpose()?;
        let bound_effect = callback.storage.iter().position(|s| !s.preserve);
        for operation in &callback.state_ops {
            if tracks
                .iter()
                .find(|t| t.key == operation.track)
                .is_none_or(|t| t.quest != callback.quest)
            {
                return Err(format!(
                    "undeclared or foreign reward operation {}",
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
                        effect: if s.preserve {
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
        for s in &callback.storage {
            if let Some(track) = tracks.iter_mut().find(|t| t.key == s.track) {
                if track.quest != callback.quest {
                    return Err(format!("foreign reward track {}", s.track));
                }
                track.max = track.max.max(s.maximum).max(s.set);
                track.min = track.min.min(s.minimum);
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
        transitions.push(QuestTransition {
            world_change: None,
            key: callback.key.clone(),
            quest: callback.quest.clone(),
            effects: callback
                .storage
                .iter()
                .enumerate()
                .map(|(index, s)| QuestEffect {
                    track: s.track.clone(),
                    from: QuestComparison::Between(s.minimum, s.maximum),
                    effect: if s.preserve {
                        QuestEffectKind::Add(0)
                    } else if Some(index) == bound_effect
                        && let Some(binding) = binding
                    {
                        QuestEffectKind::SetWithBinding {
                            value: s.set,
                            binding,
                        }
                    } else {
                        QuestEffectKind::Set(s.set)
                    },
                })
                .chain(callback.state_ops.iter().map(StateOperation::quest_effect))
                .collect(),
            completes: callback.completes,
            experience: callback.experience,
        });
    }
    if let Some(required) = requires
        .iter()
        .find(|r| !tracks.iter().any(|t| t.key == r.track))
    {
        return Err(format!("undeclared reward prerequisite {}", required.track));
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
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
        // Use the original single-track canonical fixture, independently of
        // newly merged programmes that sort before it and change two tracks.
        let fixture = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-reward/deepling-helmet")
            .ok_or("single-track reward fixture")?;

        assert!(callbacks.len() >= 6);
        for edit in 0..6 {
            let mut value: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            value["callbacks"][0] = serde_json::to_value(fixture).map_err(|e| e.to_string())?;
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
}

#[cfg(test)]
mod additional_terms_tests {
    use super::*;
    #[test]
    fn owned_absence_refills_are_bounded_exact_and_disjoint() -> Result<(), String> {
        let original = load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-reward/questpc3/beginning/lost-coat")
            .ok_or("refill callback")?;
        let document =
            |row: &RewardCallback| serde_json::json!({"schema":1,"callbacks":[row]}).to_string();
        assert!(parse(&document(&original)).is_ok());
        for edit in 0..6 {
            let mut callback = original.clone();
            match edit {
                0 => callback.absent.push(callback.absent[0].clone()),
                1 => callback.absent = vec!["oteryn:item.tibia.i3270".into()],
                2 => callback.present.push(ItemAmount {
                    item: callback.absent[0].clone(),
                    count: 1,
                }),
                3 => callback.take.push(ItemAmount {
                    item: callback.absent[0].clone(),
                    count: 1,
                }),
                4 => callback.experience = Some(1),
                _ => callback.give = None,
            }
            assert!(parse(&document(&callback)).is_err(), "absence edit {edit}");
        }
        let mut ordinary = original.clone();
        ordinary.repeatable = false;
        ordinary.absent.clear();
        assert_ne!(reward_binding(&original)?, reward_binding(&ordinary)?);
        Ok(())
    }

    #[test]
    fn merged_catalogue_has_a_closed_512_callback_budget() -> Result<(), String> {
        let original = load()?.into_iter().next().ok_or("callback fixture")?;
        let mut rows = Vec::new();
        for index in 0..513 {
            let mut callback = original.clone();
            callback.key = format!("oteryn:quest-reward/capacity-{index}");
            rows.push(callback);
        }
        let full = serde_json::json!({"schema":1,"callbacks":&rows[..512]}).to_string();
        assert_eq!(parse(&full)?.len(), 512);
        let overflow = serde_json::json!({"schema":1,"callbacks":rows}).to_string();
        assert!(parse(&overflow).is_err());
        Ok(())
    }

    #[test]
    fn carried_exchanges_bind_selection_and_refuse_free_or_unselected_inputs() -> Result<(), String>
    {
        let callback = load()?
            .into_iter()
            .find(|c| c.carried_item.is_some())
            .ok_or("carried exchange")?;
        let document =
            |c: &RewardCallback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
        assert!(parse(&document(&callback)).is_ok());
        let mut other = callback.clone();
        other.carried_item = callback
            .take
            .iter()
            .find(|take| Some(&take.item) != callback.carried_item.as_ref())
            .map(|take| take.item.clone());
        assert!(other.carried_item.is_some());
        assert!(parse(&document(&other)).is_ok());
        assert_ne!(reward_binding(&callback)?, reward_binding(&other)?);
        for mutation in 0..5 {
            let mut invalid = callback.clone();
            match mutation {
                0 => invalid.take.clear(),
                1 => invalid.give = None,
                2 => invalid.carried_item = Some("oteryn:item.tibia.i3270".into()),
                3 => invalid.take.iter_mut().for_each(|take| take.count = 2),
                _ => {
                    invalid.fee = Some(MonetaryFee {
                        gold: 1,
                        include_bank: false,
                    })
                }
            }
            assert!(
                parse(&document(&invalid)).is_err(),
                "carried exchange mutation {mutation}"
            );
        }
        Ok(())
    }
    #[test]
    fn legacy_callbacks_keep_their_existing_reward_binding() -> Result<(), String> {
        // Frozen before equipment/repeatable-check fields existed. Empty defaults
        // must not invalidate unrelated in-flight reward definitions and receipts.
        let text = r#"{"schema":1,"callbacks":[{"key":"oteryn:quest-reward/test","quest":"oteryn:quest.test","npc":"oteryn:npc.test","keywords":["yes"],"topic":0,"next_topic":0,"storage":[{"track":"oteryn:track","minimum":3,"maximum":3,"set":4}],"take":[],"give":null,"outfit":null,"achievement":null,"reply":["Reply."],"sources":["fixture"]}]}"#;
        let callback = parse(text)?.into_iter().next().ok_or("callback")?;
        assert_eq!(
            reward_binding(&callback)?,
            [
                207, 41, 110, 39, 135, 189, 244, 75, 82, 120, 174, 202, 42, 96, 136, 154, 182, 250,
                189, 203, 104, 136, 202, 26, 238, 6, 117, 167, 127, 82, 231, 202
            ]
        );
        Ok(())
    }

    #[test]
    fn monetary_fees_are_bounded_source_bound_and_excluded_from_value_free_checks()
    -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.fee.is_some())
            .ok_or("paid callback")?;
        let document =
            |c: &RewardCallback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
        let original = reward_binding(&callback)?;
        let mut changed = callback.clone();
        changed.fee.as_mut().ok_or("fee")?.gold += 1;
        assert_ne!(reward_binding(&changed)?, original);
        assert!(parse(&document(&changed)).is_ok());
        changed = callback.clone();
        changed.fee.as_mut().ok_or("fee")?.include_bank = true;
        assert_ne!(reward_binding(&changed)?, original);
        changed = callback.clone();
        changed.fee = None;
        assert_ne!(reward_binding(&changed)?, original);
        for gold in [0, 1_000_000_000_001] {
            changed = callback.clone();
            changed.fee.as_mut().ok_or("fee")?.gold = gold;
            assert!(parse(&document(&changed)).is_err());
        }
        changed = callback.clone();
        changed.take.push(ItemAmount {
            item: "oteryn:item.tibia.i3031".into(),
            count: 1,
        });
        assert!(parse(&document(&changed)).is_err());
        changed = callback;
        changed.repeatable_check = true;
        changed.give = None;
        changed.outfit = None;
        changed.achievement = None;
        changed.blessing = None;
        changed.experience = None;
        changed.health_loss = None;
        changed.completes = false;
        changed.take.clear();
        for storage in &mut changed.storage {
            storage.preserve = true;
        }
        assert!(parse(&document(&changed)).is_err());
        changed.fee = None;
        assert!(parse(&document(&changed)).is_ok());
        Ok(())
    }
    #[test]
    fn alternative_evidence_is_bounded_distinct_positive_and_content_bound() -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|callback| !callback.present_any.is_empty())
            .ok_or("alternative callback")?;
        let document = |callback: &RewardCallback| {
            serde_json::json!({"schema": 1, "callbacks": [callback]}).to_string()
        };
        let original = reward_binding(&callback)?;
        let mut changed = callback.clone();
        changed.present_any[0].count += 1;
        assert_ne!(reward_binding(&changed)?, original);
        assert!(parse(&document(&changed)).is_ok());
        changed.present_any[0].count = 0;
        assert!(parse(&document(&changed)).is_err());
        changed = callback.clone();
        changed.present_any.push(changed.present_any[0].clone());
        assert!(parse(&document(&changed)).is_err());
        changed.present_any = (1..=9)
            .map(|id| ItemAmount {
                item: format!("oteryn:item.tibia.i{id}"),
                count: 1,
            })
            .collect();
        assert!(parse(&document(&changed)).is_err());
        changed = callback.clone();
        changed.present_any[0].item = "unqualified-input".into();
        assert!(parse(&document(&changed)).is_err());
        changed.present_any.clear();
        assert_ne!(reward_binding(&changed)?, original);
        assert!(parse(&document(&changed)).is_ok());
        Ok(())
    }

    #[test]
    fn health_loss_is_positive_once_only_and_content_bound() -> Result<(), String> {
        let mut callback = load()?
            .into_iter()
            .find(|callback| callback.health_loss.is_some())
            .ok_or("health callback")?;
        let original = reward_binding(&callback)?;
        callback.health_loss = Some(21);
        assert_ne!(reward_binding(&callback)?, original);
        let document = |callback: &RewardCallback| {
            serde_json::json!({"schema": 1, "callbacks": [callback]}).to_string()
        };
        callback.health_loss = Some(0);
        assert!(parse(&document(&callback)).is_err());
        callback.health_loss = Some(20);
        callback.repeatable_check = true;
        for storage in &mut callback.storage {
            storage.maximum = storage.minimum;
            storage.set = storage.minimum;
        }
        assert!(parse(&document(&callback)).is_err());
        callback.health_loss = None;
        assert!(parse(&document(&callback)).is_ok());
        Ok(())
    }

    #[test]
    fn repeatable_equipment_checks_are_bounded_value_free_and_content_bound() -> Result<(), String>
    {
        let mut callback = load()?.into_iter().next().ok_or("callback")?;
        callback.take.clear();
        callback.give = None;
        callback.outfit = None;
        callback.achievement = None;
        callback.experience = None;
        callback.blessing = None;
        callback.completes = false;
        callback.repeatable_check = true;
        callback.equipped_slots = vec![4];
        for storage in &mut callback.storage {
            storage.preserve = false;
            storage.set = storage.minimum;
            storage.maximum = storage.minimum;
        }
        let accepts = |c: &RewardCallback| {
            let value = serde_json::json!({"schema":1, "callbacks":[c]});
            parse(&value.to_string()).is_ok()
        };
        assert!(accepts(&callback));
        let mut level_bound = callback.clone();
        level_bound.minimum_level = Some(0);
        assert!(!accepts(&level_bound));
        level_bound.minimum_level = Some(30);
        assert!(accepts(&level_bound));
        assert_ne!(reward_binding(&level_bound)?, reward_binding(&callback)?);
        let mut mutating = callback.clone();
        mutating.storage[0].maximum = mutating.storage[0].minimum + 1;
        assert!(!accepts(&mutating));
        let original_binding = reward_binding(&callback)?;
        for slots in [vec![0], vec![9], vec![11], vec![4, 4]] {
            let mut invalid = callback.clone();
            invalid.equipped_slots = slots;
            assert!(!accepts(&invalid));
        }
        for field in [
            "take",
            "give",
            "outfit",
            "achievement",
            "experience",
            "blessing",
            "completes",
        ] {
            let mut invalid = serde_json::to_value(&callback).map_err(|e| e.to_string())?;
            invalid[field] = match field {
                "take" => serde_json::json!([{"item":"oteryn:item.tibia.i3562","count":1}]),
                "give" => serde_json::json!({"item":"oteryn:item.tibia.i3270","count":1}),
                "outfit" => serde_json::json!({"outfit":"oteryn:outfit.citizen","addons":0}),
                "achievement" => serde_json::json!("oteryn:achievement/test"),
                "experience" => serde_json::json!(100),
                "blessing" => serde_json::json!("oteryn:service.blessing.test"),
                "completes" => serde_json::json!(true),
                _ => return Err("unexpected test field".into()),
            };
            let text = serde_json::json!({"schema":1,"callbacks":[invalid]}).to_string();
            assert!(
                parse(&text).is_err(),
                "repeated checks cannot include {field}"
            );
        }
        callback.equipped_slots = vec![1];
        assert_ne!(reward_binding(&callback)?, original_binding);
        callback.equipped_slots = vec![4];
        callback.repeatable_check = false;
        assert_ne!(reward_binding(&callback)?, original_binding);
        assert!(!accepts(&callback));
        Ok(())
    }

    #[test]
    fn reward_value_and_premium_terms_change_the_quest_definition_hash() -> Result<(), String> {
        let callback = load()?.into_iter().next().ok_or("callback")?;
        let make = |callback: RewardCallback| -> Result<QuestStateCatalogue, String> {
            let binding = reward_binding(&callback)?;
            let tracks = callback
                .storage
                .iter()
                .map(|s| QuestTrack {
                    key: s.track.clone(),
                    quest: callback.quest.clone(),
                    initial: -1,
                    min: s.minimum.min(-1).min(s.set),
                    max: s.maximum.max(s.set),
                })
                .collect();
            let effects = callback
                .storage
                .iter()
                .map(|s| QuestEffect {
                    track: s.track.clone(),
                    from: QuestComparison::Between(s.minimum, s.maximum),
                    effect: QuestEffectKind::SetWithBinding {
                        value: s.set,
                        binding,
                    },
                })
                .collect();
            QuestStateCatalogue::new(
                "reward-binding",
                tracks,
                vec![QuestTransition {
                    world_change: None,
                    key: callback.key,
                    quest: callback.quest,
                    effects,
                    completes: false,
                    experience: None,
                }],
            )
            .map_err(|e| format!("{e:?}"))
        };
        let original = make(callback.clone())?.definition_hash(&callback.quest);
        let mut changed = callback.clone();
        changed.blessing = Some("oteryn:service.blessing.the_spiritual_shielding".into());
        assert_ne!(make(changed)?.definition_hash(&callback.quest), original);
        let mut changed = callback.clone();
        changed.premium = !changed.premium;
        assert_ne!(make(changed)?.definition_hash(&callback.quest), original);
        let mut changed = callback.clone();
        changed.minimum_level = Some(30);
        assert_ne!(make(changed)?.definition_hash(&callback.quest), original);
        Ok(())
    }

    #[test]
    fn qengine_reward_callbacks_are_bounded_and_never_repeatable_value_grants() -> Result<(), String>
    {
        let source = include_str!("../../../../../content/quests/shared-reward-callbacks.json");
        let callbacks = load()?;
        assert!(callbacks.len() >= 6);
        for amount in [0, -1, super::super::QUESTGATE0_RL_05 + 1] {
            let mut invalid: serde_json::Value =
                serde_json::from_str(source).map_err(|error| error.to_string())?;
            invalid["callbacks"][0]["experience"] = amount.into();
            assert!(parse(&invalid.to_string()).is_err(), "invalid XP {amount}");
        }
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
                    let storage = value["callbacks"][0]["storage"]
                        .as_array_mut()
                        .ok_or("storage fixture")?;
                    for field in storage {
                        field["set"] = field["minimum"].clone();
                    }
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
        super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        Ok(())
    }
}

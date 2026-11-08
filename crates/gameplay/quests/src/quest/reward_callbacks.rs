//! Bounded NPC exchanges composed by the fenced quest writer, never by dialogue text.
use super::world_callbacks::Storage;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod capture_tests;
mod colours_peer;
#[cfg(test)]
mod peer_tests;
#[cfg(test)]
mod recipe_tests;
mod state_ops;
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
    /// Literal source-authored text initialized by this reward's existing MINT.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Independent top-level items committed in the same quest receipt.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional: Vec<ItemAmount>,
}

impl ItemReward {
    pub(super) fn valid_literal_text(&self) -> bool {
        self.text.as_deref().is_none_or(|text| {
            self.count == 1
                && self.contents.is_empty()
                && !text.is_empty()
                && text.len() <= 7_996
                && text.chars().count() <= 4_000
                && !text.contains('\0')
        })
    }
}

#[cfg(test)]
mod literal_text_tests;

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
    /// Personal display fact created atomically with this dialogue progress receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_mark: Option<super::map_marks::QuestMapMark>,
    pub key: String,
    pub quest: String,
    pub npc: String,
    /// Ordinary carried USE binding; NPC dialogue cannot execute this callback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carried_item: Option<String>,
    /// Exact second carried operand; its selected instance must accompany the ingress.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carried_target: Option<String>,
    /// Read-only condition on the selected other live player, checked atomically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer_requires: Option<TrackRequirement>,
    /// Exact native live creature selected by a carried quest tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creature_target: Option<String>,
    /// Remove the exact living target only after its tool/progress receipt commits.
    #[serde(default, skip_serializing_if = "is_false")]
    pub capture_creature: bool,
    pub keywords: Vec<String>,
    pub topic: u16,
    pub next_topic: u16,
    /// Complete this owning quest in the same receipt as its final dialogue.
    #[serde(default, skip_serializing_if = "is_false")]
    pub completes: bool,
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
        (!self.take.is_empty() && self.give.is_some())
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

const DOCUMENTS: &[&str] = &[
    include_str!("../../../../../content/quests/shared-reward-callbacks.json"),
    include_str!("../../../../../content/quests/npc-callbacks/l6-stories.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m13-rotworm-sample.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m13-goblin-capture.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m16-targuna-marker.json"),
    include_str!("../../../../../content/quests/npc-callbacks/m20-cursed-crystal-recipes.json"),
    // Keep the shared document at DOCUMENTS[0]; Oramond is the final extension.
    include_str!("../../../../../content/quests/npc-callbacks/oramond-native-roots.json"),
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

pub fn load() -> Result<Vec<RewardCallback>, String> {
    parse_documents(DOCUMENTS).map(|document| document.callbacks)
}

fn parse_documents(sources: &[&str]) -> Result<Document, String> {
    let mut combined = Document {
        schema: 1,
        tracks: Vec::new(),
        callbacks: Vec::new(),
    };
    for source in sources {
        let document: Document = serde_json::from_str(source).map_err(|e| e.to_string())?;
        if document.schema != 1 {
            return Err("reward callback schema".into());
        }
        combined.tracks.extend(document.tracks);
        combined.callbacks.extend(document.callbacks);
    }
    colours_peer::append_after_opening(&mut combined)?;
    validate_document(combined)
}

pub(crate) fn validate_callbacks(callbacks: &[RewardCallback]) -> Result<(), String> {
    parse(&serde_json::json!({"schema":1,"callbacks":callbacks}).to_string()).map(|_| ())
}

fn parse(text: &str) -> Result<Vec<RewardCallback>, String> {
    parse_documents(&[text]).map(|document| document.callbacks)
}

fn validate_document(document: Document) -> Result<Document, String> {
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
    let item =
        |item: &str, count| slug(item, "oteryn:item.", b"._/:-", 256) && (1..=100).contains(&count);
    if document.schema != 1 || document.callbacks.len() > 256 || document.tracks.len() > 256 {
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
            || c.storage.len() + c.state_ops.len() + participant_count > super::QUESTSTATE0_RL_02
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
            || !slug(&c.key, "oteryn:quest-reward/", b"_/-", 128)
            || (c.capture_creature && c.creature_target.is_none())
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
            || !slug(&c.quest, "oteryn:", b"._/:-", 128)
            || !keys.insert(&c.key)
            || !slug(&c.npc, "oteryn:npc.", b"_", 128)
            || !c.valid_carried_inputs()
            || c.carried_item.is_some()
                && (c.fee.is_some() || !c.requires_actor_conditions.is_empty())
            || c.forbids_outfits.len() > 8
            || c.forbids_outfits.iter().collect::<BTreeSet<_>>().len() != c.forbids_outfits.len()
            || c.forbids_outfits
                .iter()
                .any(|key| !slug(key, "oteryn:outfit.", b"_", 128))
            || (c.storage.is_empty() && c.state_ops.is_empty() && !c.value_only_guarded())
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
            || c.present.iter().any(|item| item.fluid_subtype.is_some())
            || c.take.len() > 16
            || inputs.len() != c.take.len()
            || c.take.iter().any(|s| {
                !slug(&s.item, "oteryn:item.", b"._/:-", 256)
                    || (s.count == 0 || s.fluid_subtype.is_some())
            })
            || c.give.as_ref().is_some_and(|r| {
                !item(&r.item, r.count)
                    || !r.valid_literal_text()
                    || r.additional.len() > 19
                    || r.additional
                        .iter()
                        .any(|s| !item(&s.item, s.count) || s.fluid_subtype.is_some())
                    || r.contents.len() > 20
                    || r.contents.iter().any(|s| {
                        !item(&s.item, s.count)
                            || s.fluid_subtype
                                .is_some_and(|fluid| fluid == 0 || s.count != 1)
                    })
            })
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

// Ungated published callbacks retain their original Set encoding. For newly
// gated callbacks, reuse PC3's bound Set so changing admission/value terms also
// changes the owning unfinished quest pin; speech/citations are not reward terms.
/// Optional binding of newly gated admission/value terms; absent legacy gates keep old hashes.
pub fn gated_binding(callback: &RewardCallback) -> Result<Option<[u8; 32]>, String> {
    if callback.minimum_level.is_none()
        && callback.peer_requires.is_none()
        && callback.creature_target.is_none()
        && callback.map_mark.is_none()
        && callback.carried_target.is_none()
    {
        return Ok(None);
    }
    use sha2::{Digest, Sha256};
    let mut terms = serde_json::to_value(callback).map_err(|e| e.to_string())?;
    let terms = terms.as_object_mut().ok_or("reward terms")?;
    terms.remove("reply");
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

pub fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    let document = parse_documents(DOCUMENTS)?;
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
    let requires: Vec<_> = callbacks
        .iter()
        .flat_map(|c| c.requires.iter().chain(c.peer_requires.iter()))
        .collect();
    for callback in &callbacks {
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
        transitions.push(primary_transition(callback)?);
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

/// Exact primary state operation shared by lowering and bound admission validation.
pub fn primary_transition(callback: &RewardCallback) -> Result<QuestTransition, String> {
    let binding = gated_binding(callback)?;
    let bound_effect = callback.storage.iter().position(|state| !state.preserve);
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
                    QuestEffectKind::Add(0)
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
            .chain(callback.state_ops.iter().map(StateOperation::quest_effect))
            .collect(),
        completes: callback.completes,
        experience: None,
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
                preserve: false,
                at_least: false,
                increment: false,
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
            preserve: false,
            at_least: false,
            increment: false,
        });
        assert!(super::parse(&document(&callback)).is_err());
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
            (
                "oteryn:quest.the_ice_islands_quest",
                // M47 adds the canonical four-track native mead transition.
                "ce91fb477a2477dfa81674b40bb86149ee17e775b0d1ee623bfac428ae3e7fe4",
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
            assert_eq!(
                hash.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                expected,
                "published ungated quest {key}"
            );
        }
        Ok(())
    }

    #[test]
    fn authored_shards_complete_only_the_selected_quest_and_preserve_old_plans()
    -> Result<(), String> {
        let legacy = parse(DOCUMENTS[0])?;
        for callback in &legacy {
            assert!(!callback.completes);
            assert!(
                serde_json::to_value(callback)
                    .map_err(|e| e.to_string())?
                    .get("completes")
                    .is_none()
            );
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

#[cfg(test)]
#[path = "reward_callbacks/readable_text_tests.rs"]
mod readable_text_tests;

#[cfg(test)]
#[path = "reward_callbacks/map_mark_tests.rs"]
mod map_mark_tests;

#[cfg(test)]
mod multiple_rewards_tests;

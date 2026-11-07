//! Draft finite donor-derived acquisition programs. No Lua or external program executes.
#[path = "source_program/paid.rs"]
pub mod paid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sex {
    Female,
    Male,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocked {
    InvalidProgram,
    MissingSourceFact,
    ArithmeticBound,
    UnsupportedEffect,
}

/// Values come only from the durable owner's locked source-qualified snapshot.
pub trait Facts {
    fn now(&self) -> i64;
    fn actor_condition(&self, _condition: ActorCondition) -> Result<bool, Blocked> {
        Err(Blocked::MissingSourceFact)
    }
    /// Unknown binding is an error; a known track with no stored row is -1.
    fn track(&self, storage: &str) -> Result<i64, Blocked>;
    fn inventory(&self, item: u32) -> Result<u32, Blocked>;
    fn premium(&self) -> Result<bool, Blocked>;
    fn sex(&self) -> Sex;
    /// Exact admitted source look identity, not a canonical-family approximation.
    fn owned_look_addon(&self, look: u32, addon: u8) -> Result<bool, Blocked>;
    fn kv_bool(&self, scope: &str, key: &str) -> Result<bool, Blocked>;
    fn level(&self) -> u32;
    fn owner_points(&self, owner: &str) -> Result<i64, Blocked>;
    fn owned_mount(&self, id: u16) -> Result<bool, Blocked>;
    fn selected_integer(&self, name: &str) -> Option<i64>;
    fn hazard_can_select(&self, owner: &str, selection: i64) -> Result<bool, Blocked>;
    fn world_storage(&self, id: u32, missing: i64) -> Result<i64, Blocked>;
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    schema: String,
    revision: String,
    pub bindings: Vec<StorageBinding>,
    pub items: Vec<ItemBinding>,
    pub looks: Vec<LookBinding>,
    pub mounts: Vec<MountBinding>,
    pub npcs: Vec<Npc>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageBinding {
    pub storage: String,
    pub key: String,
    pub quest: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemBinding {
    pub id: u32,
    pub definition: Definition,
}
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub family: String,
    pub production_key: String,
    pub revision_ref: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LookBinding {
    pub look: u32,
    pub sex: String,
    pub outfit_key: String,
    pub addon_count: u8,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountBinding {
    pub id: u16,
    pub look: u32,
    pub key: String,
}
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub key: String,
    pub outfit_key: String,
    pub look: u32,
    pub sex: String,
    pub addon: u8,
    pub paired: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Npc {
    pub npc: String,
    pub source: Source,
    pub greeting: String,
    pub farewell: String,
    pub owned_tracks: Vec<String>,
    pub transitions: Vec<Transition>,
    #[serde(default)]
    pub current_tibia_source_corrections: Option<serde_json::Value>,
    #[serde(default)]
    pub upstream_owner: Option<serde_json::Value>,
}
impl Npc {
    fn requires_quest_composition(&self, program: &Program) -> bool {
        self.transitions.iter().any(|transition| {
            transition.effects.iter().any(|effect| match effect {
                Effect::SetTrack { storage, value } if !self.owned_tracks.contains(storage) => {
                    program.binding(storage).is_none_or(|binding| {
                        !super::source_quest_bindings::covers(
                            &transition.source_key,
                            storage,
                            binding,
                            value,
                        )
                    })
                }
                _ => false,
            })
        })
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub path: String,
    pub sha256: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub id: String,
    pub source_key: String,
    pub grants: Vec<Grant>,
    pub source_lines: [u32; 2],
    pub keywords: Vec<String>,
    #[serde(default)]
    pub keyword_guard: Option<Words>,
    #[serde(default)]
    pub keyword_capture: Option<KeywordCapture>,
    #[serde(default)]
    pub reply_template: Option<ReplyTemplate>,
    pub topic_before: Option<u16>,
    pub topic_after: u16,
    pub guards: Guard,
    pub effects: Vec<Effect>,
    pub reply: String,
    #[serde(default)]
    pub reply_lines: Vec<String>,
    #[serde(default)]
    pub trigger: Trigger,
}
#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    #[default]
    Say,
    Greet,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeywordCapture {
    #[serde(rename = "type")]
    pub kind: CaptureKind,
    pub name: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureKind {
    PositiveInteger,
}
#[path = "source_program_reply_template.rs"]
mod reply_template;
pub use reply_template::{HazardReplyFacts, ReplyTemplate};
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ActorCondition {
    Drunk,
}
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Guard {
    All(All),
    Any(Any),
    Not(Not),
    Leaf(Leaf),
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct All {
    all: Vec<Guard>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Any {
    any: Vec<Guard>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Not {
    not: Box<Guard>,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Leaf {
    ActorCondition {
        condition: ActorCondition,
        present: bool,
    },
    Track {
        storage: String,
        op: Compare,
        value: Operand,
    },
    Inventory {
        item: u32,
        op: Compare,
        value: i64,
    },
    Sex {
        op: Compare,
        value: String,
    },
    Premium {
        op: Compare,
        value: bool,
    },
    OwnedLookAddon {
        look: u32,
        addon: u8,
        present: bool,
    },
    LevelGe {
        value: u32,
    },
    OwnerPoints {
        owner: String,
        op: Compare,
        value: i64,
    },
    OwnedMount {
        id: u16,
        present: bool,
    },
    Elapsed {
        storage: String,
        minimum_seconds: i64,
    },
    SelectedInteger {
        name: String,
        op: Compare,
        value: i64,
    },
    HazardOwnerCanSelect {
        owner: String,
        selection: String,
    },
    WorldStorage {
        storage: u32,
        op: Compare,
        value: Operand,
        missing: i64,
    },
    Kv {
        scope: String,
        key: String,
        op: Compare,
        value: bool,
    },
}
#[derive(Debug, Deserialize)]
pub enum Compare {
    EQ,
    NE,
    LT,
    LE,
    GT,
    GE,
}
impl Compare {
    fn test(&self, a: i64, b: i64) -> bool {
        match self {
            Self::EQ => a == b,
            Self::NE => a != b,
            Self::LT => a < b,
            Self::LE => a <= b,
            Self::GT => a > b,
            Self::GE => a >= b,
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Operand {
    Integer(i64),
    Expression(Expression),
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Expression {
    Literal {
        value: i64,
    },
    Now,
    NowPlus {
        seconds: i64,
    },
    Add {
        storage: String,
        amount: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        clamp_min: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        clamp_max: Option<i64>,
    },
}
impl Operand {
    pub fn value(&self, s: &impl Facts) -> Result<i64, Blocked> {
        match self {
            Self::Integer(v) => Ok(*v),
            Self::Expression(e) => e.value(s),
        }
    }
}
impl Expression {
    pub fn value(&self, s: &impl Facts) -> Result<i64, Blocked> {
        let bounded = |n: Option<i64>| n.ok_or(Blocked::ArithmeticBound);
        match self {
            Self::Literal { value } => Ok(*value),
            Self::Now => Ok(s.now()),
            Self::NowPlus { seconds } => bounded(s.now().checked_add(*seconds)),
            Self::Add {
                storage,
                amount,
                clamp_min,
                clamp_max,
            } => {
                let mut n = bounded(s.track(storage)?.checked_add(*amount))?;
                if let Some(min) = clamp_min {
                    n = n.max(*min)
                }
                if let Some(max) = clamp_max {
                    n = n.min(*max)
                }
                Ok(n)
            }
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Words {
    All(AllWords),
    Any(AnyWords),
    Not(NotWords),
    Contains(Contains),
    Equals(ExactWords),
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllWords {
    all: Vec<Words>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnyWords {
    any: Vec<Words>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotWords {
    not: Box<Words>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contains {
    contains: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactWords {
    equals: String,
    #[serde(default = "default_case_sensitive")]
    case_sensitive: bool,
}
fn default_case_sensitive() -> bool {
    true
}
impl Words {
    fn test(&self, text: &str, contains: fn(&str, &str) -> bool) -> bool {
        match self {
            Self::All(v) => v.all.iter().all(|v| v.test(text, contains)),
            Self::Any(v) => v.any.iter().any(|v| v.test(text, contains)),
            Self::Not(v) => !v.not.test(text, contains),
            Self::Contains(v) => contains(text, &v.contains),
            Self::Equals(v) => {
                if v.case_sensitive {
                    text == v.equals
                } else {
                    text.eq_ignore_ascii_case(&v.equals)
                }
            }
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    SetTrack {
        storage: String,
        value: Expression,
    },
    ConsumeItem {
        item: u32,
        count: u32,
    },
    MintItem {
        item: u32,
        count: u32,
    },
    ConsumeMoney {
        gold: u64,
    },
    GrantLookAddon {
        look: u32,
        addon: u8,
    },
    MagicEffect {
        id: u16,
    },
    Achievement {
        name: String,
    },
    AchievementProgress {
        name: String,
        target: u32,
    },
    CloseConversation,
    ConsumeMoneyBank {
        gold: u64,
    },
    GrantMount {
        id: u16,
    },
    TextMessage {
        class: String,
        text: String,
    },
    MintStoreDecorationKit {
        source_target: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source_action: Option<u32>,
        requires_free_store_inbox_entry: bool,
    },
    ConsumeOwnerPoints {
        owner: String,
        amount: u32,
    },
    SetHazardLevel {
        owner: String,
        selection: String,
    },
    SetKv {
        scope: String,
        key: String,
        value: bool,
    },
    SpikeUpperKillAccept,
    SpikeUpperKillReport,
    ConditionDamage {
        condition: String,
        delayed: bool,
        ticks: u32,
        interval_ms: u64,
        damage_per_tick: u32,
    },
}
impl Guard {
    pub fn test(&self, s: &impl Facts) -> Result<bool, Blocked> {
        match self {
            Self::All(v) => {
                for g in &v.all {
                    if !g.test(s)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Self::Any(v) => {
                for g in &v.any {
                    if g.test(s)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Self::Not(v) => Ok(!v.not.test(s)?),
            Self::Leaf(g) => Ok(match g {
                Leaf::ActorCondition { condition, present } => {
                    s.actor_condition(*condition)? == *present
                }
                Leaf::Track { storage, op, value } => op.test(s.track(storage)?, value.value(s)?),
                Leaf::Inventory { item, op, value } => {
                    op.test(i64::from(s.inventory(*item)?), *value)
                }
                Leaf::Sex { op, value } => {
                    let expected = match value.as_str() {
                        "female" => Sex::Female,
                        "male" => Sex::Male,
                        _ => return Err(Blocked::InvalidProgram),
                    };
                    op.test(i64::from(s.sex() == expected), 1)
                }
                Leaf::Premium { op, value } => op.test(i64::from(s.premium()?), i64::from(*value)),
                Leaf::OwnedLookAddon {
                    look,
                    addon,
                    present,
                } => s.owned_look_addon(*look, *addon)? == *present,
                Leaf::LevelGe { value } => s.level() >= *value,
                Leaf::OwnerPoints { owner, op, value } => op.test(s.owner_points(owner)?, *value),
                Leaf::OwnedMount { id, present } => s.owned_mount(*id)? == *present,
                Leaf::Elapsed {
                    storage,
                    minimum_seconds,
                } => s
                    .now()
                    .checked_sub(s.track(storage)?)
                    .is_some_and(|elapsed| elapsed >= *minimum_seconds),
                Leaf::SelectedInteger { name, op, value } => s
                    .selected_integer(name)
                    .is_some_and(|selected| op.test(selected, *value)),
                Leaf::HazardOwnerCanSelect { owner, selection } => {
                    match s.selected_integer(selection) {
                        Some(value) => s.hazard_can_select(owner, value)?,
                        None => false,
                    }
                }
                Leaf::WorldStorage {
                    storage,
                    op,
                    value,
                    missing,
                } => op.test(s.world_storage(*storage, *missing)?, value.value(s)?),
                Leaf::Kv {
                    scope,
                    key,
                    op,
                    value,
                } => op.test(i64::from(s.kv_bool(scope, key)?), i64::from(*value)),
            }),
        }
    }
    fn bounded(&self, depth: usize) -> bool {
        if depth > 8 {
            return false;
        }
        match self {
            Self::All(v) => v.all.len() <= 64 && v.all.iter().all(|v| v.bounded(depth + 1)),
            Self::Any(v) => v.any.len() <= 64 && v.any.iter().all(|v| v.bounded(depth + 1)),
            Self::Not(v) => v.not.bounded(depth + 1),
            Self::Leaf(_) => true,
        }
    }
}
impl Program {
    pub fn parse(bytes: &[u8]) -> Result<Self, Blocked> {
        if bytes.len() > 2 * 1024 * 1024 {
            return Err(Blocked::InvalidProgram);
        }
        let p: Self = serde_json::from_slice(bytes).map_err(|_| Blocked::InvalidProgram)?;
        if p.schema != "OTERYN_NPC_COSMETIC_TRANSITIONS/v1"
            || p.revision != "201189f871d23597b82caf88fd611454a2ddefb8"
            || p.npcs.len() > 64
        {
            return Err(Blocked::InvalidProgram);
        }
        let mut npcs = BTreeSet::new();
        let mut total = 0;
        for n in &p.npcs {
            let mut transitions = BTreeSet::new();
            total += n.transitions.len();
            if !npcs.insert(&n.npc)
                || !n.npc.starts_with("oteryn:npc.")
                || n.source.sha256.len() != 64
                || !n.source.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                || !n.source.path.starts_with("data-global/npc/")
            {
                return Err(Blocked::InvalidProgram);
            }
            for t in &n.transitions {
                if !transitions.insert(&t.id)
                    || t.id.len() > 128
                    || t.id.is_empty()
                    || t.source_lines[0] == 0
                    || t.source_lines[1] < t.source_lines[0]
                    || t.keywords.len() > 32
                    || t.keywords.iter().any(|s| s.is_empty() || s.len() > 128)
                    || t.reply.len() > 2048
                    || t.reply.chars().any(char::is_control)
                    || t.reply_lines.len() > 32
                    || t.reply_lines.iter().any(|line| {
                        line.is_empty() || line.len() > 2048 || line.chars().any(char::is_control)
                    })
                    || t.reply_template.as_ref().is_some_and(|template| {
                        !template.valid_for(&t.reply)
                            || template
                                .storage()
                                .is_some_and(|storage| p.binding(storage).is_none())
                    })
                    || t.effects.len() > 64
                    || !t.guards.bounded(0)
                {
                    return Err(Blocked::InvalidProgram);
                }
            }
        }
        if total > 4096 {
            return Err(Blocked::InvalidProgram);
        }
        Ok(p)
    }
    /// Preserve source branch order. Missing authoritative facts fail closed rather
    /// than falling through to a later, less restrictive grant branch.
    pub fn choose<'a>(
        &'a self,
        npc: &str,
        text: &str,
        topic: u16,
        s: &impl Facts,
        contains: fn(&str, &str) -> bool,
    ) -> Result<Option<&'a Transition>, Blocked> {
        self.choose_for_trigger(npc, text, topic, s, contains, Trigger::Say)
    }
    pub fn choose_for_trigger<'a>(
        &'a self,
        npc: &str,
        text: &str,
        topic: u16,
        s: &impl Facts,
        contains: fn(&str, &str) -> bool,
        trigger: Trigger,
    ) -> Result<Option<&'a Transition>, Blocked> {
        let Some(n) = self.npcs.iter().find(|n| n.npc == npc) else {
            return Ok(None);
        };
        for t in &n.transitions {
            if t.trigger != trigger {
                continue;
            }
            if t.topic_before.is_some_and(|before| before != topic) {
                continue;
            }
            // A registered Greet callback receives no message argument; the
            // native conversation owner supplies its genuine fresh trigger.
            let words = trigger == Trigger::Greet
                || t.keyword_guard.as_ref().map_or_else(
                    || t.keywords.is_empty() || t.keywords.iter().any(|k| contains(text, k)),
                    |w| w.test(text, contains),
                );
            if words && t.guards.test(s)? {
                return Ok(Some(t));
            }
        }
        Ok(None)
    }
}

impl Program {
    pub fn supports(&self, npc: &str) -> bool {
        self.npc(npc)
            .is_some_and(|npc| !npc.requires_quest_composition(self))
    }
    pub fn binding(&self, symbol: &str) -> Option<&StorageBinding> {
        self.bindings.iter().find(|b| b.storage == symbol)
    }
    pub fn npc(&self, key: &str) -> Option<&Npc> {
        self.npcs.iter().find(|n| n.npc == key)
    }
    pub fn recognizes(
        &self,
        npc: &str,
        text: &str,
        topic: u16,
        contains: fn(&str, &str) -> bool,
    ) -> bool {
        self.npc(npc).is_some_and(|n| {
            if n.requires_quest_composition(self) {
                return false;
            }
            n.transitions.iter().any(|t| {
                if t.trigger != Trigger::Say {
                    return false;
                }
                t.topic_before.is_none_or(|before| before == topic)
                    && t.keyword_guard.as_ref().map_or_else(
                        || t.keywords.is_empty() || t.keywords.iter().any(|k| contains(text, k)),
                        |w| w.test(text, contains),
                    )
            })
        })
    }
}
pub const EMBEDDED: &str =
    include_str!("../../../../../../content/cosmetics/npc-acquisitions.json");
pub fn embedded() -> Result<&'static Program, Blocked> {
    static PROGRAM: std::sync::OnceLock<Result<Program, Blocked>> = std::sync::OnceLock::new();
    PROGRAM
        .get_or_init(|| Program::parse(EMBEDDED.as_bytes()))
        .as_ref()
        .map_err(|e| *e)
}
pub fn digest() -> [u8; 32] {
    use sha2::Digest;
    sha2::Sha256::digest(EMBEDDED.as_bytes()).into()
}
pub fn supported(npc: &str) -> bool {
    embedded().is_ok_and(|p| p.supports(npc))
}

/// Add only source-owned cosmetic tracks missing from the admitted catalogue.
/// Existing quest definitions and transitions are retained; this grants no state.
pub(crate) fn extend_catalogue(
    catalogue: super::super::QuestStateCatalogue,
) -> Result<super::super::QuestStateCatalogue, String> {
    let document = embedded().map_err(|error| format!("source programme: {error:?}"))?;
    let owned: std::collections::BTreeSet<_> = document
        .npcs
        .iter()
        .flat_map(|npc| npc.owned_tracks.iter())
        .collect();
    let mut additions: std::collections::BTreeMap<String, Vec<super::super::QuestTrack>> =
        std::collections::BTreeMap::new();
    for binding in &document.bindings {
        if !owned.contains(&binding.storage) {
            continue;
        }
        if let Some(track) = catalogue.track(&binding.key) {
            if track.quest != binding.quest {
                return Err(format!("source track owner drift: {}", binding.key));
            }
            continue;
        }
        if catalogue.definition_hash(&binding.quest).is_some() {
            return Err(format!(
                "source track would change an existing Quest definition: {}",
                binding.key
            ));
        }
        additions
            .entry(binding.quest.clone())
            .or_default()
            .push(super::super::QuestTrack {
                key: binding.key.clone(),
                quest: binding.quest.clone(),
                initial: -1,
                min: -1,
                max: i64::MAX,
            });
    }
    let mut output = catalogue;
    for tracks in additions.into_values() {
        output = output
            .with_quest(tracks, Vec::new())
            .map_err(|error| format!("source acquisition component: {error:?}"))?;
    }
    Ok(output)
}

#[cfg(test)]
mod programme_tests {
    use super::*;
    #[test]
    fn actual_closed_source_programme_has_unique_qualified_identities() -> Result<(), String> {
        let _: Program = serde_json::from_str(EMBEDDED)
            .map_err(|error| format!("strict programme serde: {error}"))?;
        let program = embedded().map_err(|error| format!("{error:?}"))?;
        // The56 permanent-grant source census is independent of the linked
        // programme. Rental/dead/reference-only owners remain excluded; Gnomilly
        // is one explicitly separate11-transition acquisition support input.
        let captured: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../tools/content-schema/cosmetics-authoring/npc-source-transitions.json"
        ))
        .map_err(|e| e.to_string())?;
        let nodes = captured["npcs"].as_array().ok_or("captured NPC census")?;
        assert_eq!(nodes.len(), 56);
        let excluded = [
            "oteryn:npc.appaloosa",
            "oteryn:npc.palomino",
            "oteryn:npc.walter_jaeger",
            "oteryn:npc.flickering_soul",
        ];
        let mut expected = std::collections::BTreeMap::new();
        for node in nodes {
            let key = node["npc"].as_str().ok_or("captured NPC identity")?;
            if !excluded.contains(&key) {
                let count = node["transitions"]
                    .as_array()
                    .ok_or("captured transitions")?
                    .len();
                assert!(
                    expected.insert(key, count).is_none(),
                    "unique raw source identity"
                );
            }
        }
        assert_eq!(
            expected.len(),
            52,
            "ordinary permanent acquisition identities"
        );
        let support: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../tools/content-schema/cosmetics-authoring/gnomilly-source-support.json"
        )).map_err(|e|e.to_string())?;
        let support = support["npcs"].as_array().ok_or("support census")?;
        assert_eq!(support.len(), 1);
        assert_eq!(support[0]["npc"], "oteryn:npc.gnomilly");
        let count = support[0]["transitions"]
            .as_array()
            .ok_or("support transitions")?
            .len();
        assert_eq!(count, 11);
        assert!(expected.insert("oteryn:npc.gnomilly", count).is_none());
        assert_eq!(
            expected.values().sum::<usize>(),
            475,
            "closed donor transition census"
        );
        let actual: std::collections::BTreeMap<_, _> = program
            .npcs
            .iter()
            .map(|npc| (npc.npc.as_str(), npc.transitions.len()))
            .collect();
        assert_eq!(program.npcs.len(), expected.len());
        assert_eq!(
            actual, expected,
            "every independently authored NPC and transition survives linking"
        );
        for npc in &program.npcs {
            assert!(
                ![
                    "oteryn:npc.appaloosa",
                    "oteryn:npc.palomino",
                    "oteryn:npc.walter_jaeger",
                    "oteryn:npc.flickering_soul"
                ]
                .contains(&npc.npc.as_str())
            );
            for transition in &npc.transitions {
                assert!(transition.source_key.starts_with(&format!(
                    "oteryn:npc-cosmetic/{}/",
                    npc.npc.trim_start_matches("oteryn:npc.")
                )));
                assert!(transition.grants.len() <= 2);
                for grant in &transition.grants {
                    assert!(program.looks.iter().any(|look| look.look == grant.look
                        && look.sex == grant.sex
                        && look.outfit_key == grant.outfit_key));
                }
            }
        }
        Ok(())
    }
    #[test]
    fn unknown_effect_and_extra_authority_fields_fail_closed() -> Result<(), String> {
        let mut document: serde_json::Value =
            serde_json::from_str(EMBEDDED).map_err(|error| error.to_string())?;
        document["npcs"][0]["transitions"][0]["effects"] =
            serde_json::json!([{"kind":"arbitrary_quest_completion"}]);
        assert_eq!(
            Program::parse(&serde_json::to_vec(&document).map_err(|error| error.to_string())?)
                .err(),
            Some(Blocked::InvalidProgram)
        );
        let mut document: serde_json::Value =
            serde_json::from_str(EMBEDDED).map_err(|error| error.to_string())?;
        document["npcs"][0]["transitions"][0]["unlocked_account"] = serde_json::json!(true);
        assert_eq!(
            Program::parse(&serde_json::to_vec(&document).map_err(|error| error.to_string())?)
                .err(),
            Some(Blocked::InvalidProgram)
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "source_program_policy_tests.rs"]
mod source_policy_oracle_tests;

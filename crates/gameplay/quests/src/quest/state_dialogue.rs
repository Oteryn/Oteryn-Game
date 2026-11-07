//! Bounded state-only NPC branches over the existing atomic QuestState writer.
//! Every guard is also a locked transition comparison, including guard-only tracks.
use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};
use serde::Deserialize;
use serde::de::IgnoredAny;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

const TEXT: &str = include_str!("../../../../../content/dialogues/quests/npc-state-stories.json");
const SOURCE: &str =
    "https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    default_messages_source: Source,
    #[serde(rename = "npcs")]
    _npcs: IgnoredAny,
    branches: Vec<Branch>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    url: String,
    sha256: String,
}

impl Source {
    fn valid(&self) -> bool {
        self.url.starts_with(SOURCE)
            && self.url.contains(".lua#L")
            && self.sha256.len() == 64
            && self.sha256.bytes().all(|b| b.is_ascii_hexdigit())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Guard {
    track: String,
    op: String,
    value: i64,
}

impl Guard {
    fn comparison(&self) -> Result<QuestComparison, String> {
        Ok(match self.op.as_str() {
            "eq" => QuestComparison::Eq(self.value),
            "ne" => QuestComparison::Ne(self.value),
            "lt" => QuestComparison::Lt(self.value),
            "le" => QuestComparison::Le(self.value),
            "gt" => QuestComparison::Gt(self.value),
            "ge" => QuestComparison::Ge(self.value),
            _ => return Err("unknown NPC state comparison".into()),
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Write {
    track: String,
    value: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Branch {
    pub key: String,
    quest: String,
    npc: String,
    keyword: String,
    topic: u16,
    pub next_topic: u16,
    guards: Vec<Guard>,
    writes: Vec<Write>,
    pub reply: Vec<String>,
    source: Source,
}

impl Branch {
    pub fn transition(&self) -> Option<String> {
        (!self.writes.is_empty()).then(|| self.key.clone())
    }

    fn effects(&self, catalogue: &QuestStateCatalogue) -> Result<Vec<QuestEffect>, String> {
        let mut effects = BTreeMap::new();
        for write in &self.writes {
            if effects
                .insert(
                    write.track.clone(),
                    QuestEffect {
                        track: write.track.clone(),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::Set(write.value),
                    },
                )
                .is_some()
            {
                return Err("duplicate NPC state write".into());
            }
        }
        let mut guards = BTreeSet::new();
        for guard in &self.guards {
            if !guards.insert(&guard.track) {
                return Err("duplicate NPC state guard".into());
            }
            effects
                .entry(guard.track.clone())
                .or_insert_with(|| QuestEffect {
                    track: guard.track.clone(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Add(0),
                })
                .from = guard.comparison()?;
        }
        for effect in effects.values() {
            let track = catalogue
                .track(&effect.track)
                .ok_or("unknown NPC state track")?;
            if track.quest != self.quest {
                return Err("foreign NPC state track".into());
            }
        }
        Ok(effects.into_values().collect())
    }
}

pub struct Book {
    document: Document,
}

pub fn load() -> Result<&'static Book, &'static str> {
    static BOOK: OnceLock<Result<Book, String>> = OnceLock::new();
    BOOK.get_or_init(|| {
        let document: Document = serde_json::from_str(TEXT).map_err(|e| e.to_string())?;
        let mut keys = BTreeSet::new();
        if document.schema != 1
            || !document.default_messages_source.valid()
            || document.branches.len() > 256
        {
            return Err("NPC state dialogue bounds".into());
        }
        for branch in &document.branches {
            if !branch.key.starts_with("oteryn:quest-transition/npcfix/")
                || !super::valid_quest_key(&branch.key)
                || !super::valid_quest_key(&branch.quest)
                || !super::valid_quest_key(&branch.npc)
                || !keys.insert(&branch.key)
                || !branch.npc.starts_with("oteryn:npc.")
                || branch.keyword.is_empty()
                || branch.keyword.len() > 128
                || !branch.source.valid()
                || branch.guards.len() > super::QUESTSTATE0_RL_02
                || branch.writes.len() > super::QUESTSTATE0_RL_02
                || branch.reply.is_empty()
                || branch.reply.len() > 16
                || branch.reply.iter().any(|line| {
                    line.is_empty() || line.len() > 1024 || line.chars().any(char::is_control)
                })
            {
                return Err("invalid NPC state branch".into());
            }
            for guard in &branch.guards {
                guard.comparison()?;
            }
        }
        Ok(Book { document })
    })
    .as_ref()
    .map_err(String::as_str)
}

impl Book {
    pub fn owns(&self, npc: &str) -> bool {
        self.document.branches.iter().any(|b| b.npc == npc)
    }

    pub fn respond(
        &self,
        npc: &str,
        message: &str,
        topic: u16,
        tracks: &BTreeMap<String, i64>,
        catalogue: &QuestStateCatalogue,
    ) -> Option<&Branch> {
        self.document.branches.iter().find(|branch| {
            branch.npc == npc
                && branch.topic == topic
                && super::classic_talk::contains(message, &branch.keyword)
                && branch.guards.iter().all(|guard| {
                    catalogue.track(&guard.track).is_some_and(|track| {
                        guard.comparison().is_ok_and(|comparison| {
                            comparison.holds(
                                tracks.get(&guard.track).copied().unwrap_or(track.initial),
                                0,
                            )
                        })
                    })
                })
        })
    }
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut transitions = catalogue.transitions.clone();
    for branch in &load()?.document.branches {
        // Validate even a text-only branch's track ownership and guard shape.
        let effects = branch.effects(&catalogue)?;
        if branch.writes.is_empty() {
            continue;
        }
        let transition = QuestTransition {
            world_change: None,
            key: branch.key.clone(),
            quest: branch.quest.clone(),
            effects,
            completes: false,
            experience: None,
        };
        if transitions.insert(branch.key.clone(), transition).is_some() {
            return Err("duplicate NPC state transition".into());
        }
    }
    QuestStateCatalogue::new(
        catalogue.content_revision,
        catalogue.tracks.into_values().collect(),
        transitions.into_values().collect(),
    )
    .map_err(|e| format!("{e:?}"))
}

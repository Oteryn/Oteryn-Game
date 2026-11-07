//! Source-qualified storage participants captured by the existing NPC trade owner.
use super::world_callbacks::Storage;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const PREFIX: &str = "oteryn:quest-trade/";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TradeCallback {
    pub key: String,
    pub quest: String,
    pub npc: String,
    pub side: Side,
    pub items: Vec<String>,
    pub unit_price: u64,
    pub offer_count: u32,
    pub minimum_units: u32,
    pub storage: Vec<Storage>,
    pub sources: Vec<String>,
}
impl TradeCallback {
    pub fn binding(&self) -> Result<[u8; 32], String> {
        let bytes = serde_json::to_vec(&(
            &self.key,
            &self.quest,
            &self.npc,
            self.side,
            &self.items,
            self.unit_price,
            self.offer_count,
            self.minimum_units,
            &self.storage,
        ))
        .map_err(|e| e.to_string())?;
        Ok(Sha256::new()
            .chain_update(b"quest-trade-callback-v1\0")
            .chain_update(bytes)
            .finalize()
            .into())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    callbacks: Vec<TradeCallback>,
}
fn key(value: &str, prefix: &str) -> bool {
    value.len() <= 128
        && value.strip_prefix(prefix).is_some_and(|tail| {
            !tail.is_empty()
                && tail
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._/:-".contains(&b))
        })
}
fn parse(text: &str) -> Result<Vec<TradeCallback>, String> {
    let document: Document = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if document.schema != 1 || document.callbacks.len() > 128 {
        return Err("trade callback bounds".into());
    }
    let mut keys = BTreeSet::new();
    let mut selectors = BTreeSet::new();
    for c in &document.callbacks {
        if !key(&c.key, PREFIX)
            || !keys.insert(&c.key)
            || !key(&c.quest, "oteryn:quest.")
            || !key(&c.npc, "oteryn:npc.")
            || c.items.is_empty()
            || c.items.len() > 8
            || c.unit_price == 0
            || c.unit_price > 1_000_000_000
            || c.offer_count == 0
            || c.offer_count > 100
            || c.minimum_units == 0
            || c.storage.is_empty()
            || c.storage.len() > 8
            || c.storage
                .iter()
                .map(|s| &s.track)
                .collect::<BTreeSet<_>>()
                .len()
                != c.storage.len()
            || c.storage
                .iter()
                .any(|s| !key(&s.track, "oteryn:") || s.minimum > s.maximum)
            || !c
                .storage
                .iter()
                .any(|s| !s.preserve && !(s.minimum..=s.maximum).contains(&s.set))
            || c.sources.is_empty()
            || c.sources.len() > 8
            || c.sources.iter().any(|s| s.is_empty() || s.len() > 1024)
        {
            return Err(format!("invalid trade callback {}", c.key));
        }
        for item in &c.items {
            // One command has one obligation identity. Reject overlapping source selectors.
            if !key(item, "oteryn:item.") || !selectors.insert((&c.npc, c.side, item)) {
                return Err(format!("ambiguous trade callback {}", c.key));
            }
        }
    }
    Ok(document.callbacks)
}
pub fn load() -> Result<Vec<TradeCallback>, String> {
    parse(include_str!(
        "../../../../../content/quests/shared-trade-callbacks.json"
    ))
}
pub fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for c in load()? {
        let binding = c.binding()?;
        let bound = c
            .storage
            .iter()
            .position(|s| !s.preserve)
            .ok_or("trade effect")?;
        for s in &c.storage {
            if let Some(track) = tracks.iter_mut().find(|t| t.key == s.track) {
                if track.quest != c.quest {
                    return Err(format!("foreign trade track {}", s.track));
                }
                track.min = track.min.min(s.minimum);
                track.max = track.max.max(s.maximum).max(s.set);
            } else {
                tracks.push(QuestTrack {
                    key: s.track.clone(),
                    quest: c.quest.clone(),
                    initial: -1,
                    min: s.minimum.min(-1).min(s.set),
                    max: s.maximum.max(s.set),
                });
            }
        }
        transitions.push(QuestTransition {
            key: c.key,
            quest: c.quest,
            effects: c
                .storage
                .iter()
                .enumerate()
                .map(|(i, s)| QuestEffect {
                    track: s.track.clone(),
                    from: QuestComparison::Between(s.minimum, s.maximum),
                    effect: if s.preserve {
                        QuestEffectKind::Add(0)
                    } else if i == bound {
                        QuestEffectKind::SetWithBinding {
                            value: s.set,
                            binding,
                        }
                    } else {
                        QuestEffectKind::Set(s.set)
                    },
                })
                .collect(),
            completes: false,
            experience: None,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trade_terms_are_bound_and_source_selectors_are_unambiguous() -> Result<(), String> {
        let rule = load()?.into_iter().next().ok_or("rule")?;
        let document = |rules: Vec<TradeCallback>| {
            serde_json::json!({"schema":1,"callbacks":rules}).to_string()
        };
        let mut changed = rule.clone();
        changed.unit_price += 1;
        assert_ne!(rule.binding()?, changed.binding()?);
        assert!(parse(&document(vec![changed])).is_ok());
        let mut duplicate = rule.clone();
        duplicate.key.push_str("-other");
        assert!(parse(&document(vec![rule.clone(), duplicate])).is_err());
        let mut changed = rule.clone();
        changed.items.push(changed.items[0].clone());
        assert!(parse(&document(vec![changed])).is_err());
        for zero in 0..3 {
            let mut changed = rule.clone();
            match zero {
                0 => changed.unit_price = 0,
                1 => changed.offer_count = 0,
                _ => changed.minimum_units = 0,
            }
            assert!(parse(&document(vec![changed])).is_err());
        }
        let mut unchanged = rule.clone();
        for condition in &mut unchanged.storage {
            condition.set = condition.minimum;
        }
        assert!(parse(&document(vec![unchanged])).is_err());
        Ok(())
    }
}

//! Exact append-only definition predecessors, reconstructed from ordered source cohorts.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DefinitionPredecessor {
    pub hash: [u8; 32],
    pub defaults: BTreeMap<String, i64>,
}

impl QuestStateCatalogue {
    /// Admit only additions: every previous track and transition must remain identical.
    /// The hashes retained here come from a validated predecessor, never a supplied save.
    pub(super) fn with_additive_extension(&self, mut next: Self) -> Result<Self, String> {
        if self.content_revision != next.content_revision
            || self
                .tracks
                .iter()
                .any(|(key, track)| next.tracks.get(key) != Some(track))
            || self
                .transitions
                .iter()
                .any(|(key, transition)| next.transitions.get(key) != Some(transition))
        {
            return Err("catalogue layer changes or removes an existing definition".into());
        }
        next.npc_legacy.clone_from(&self.npc_legacy);
        next.predecessors.clone_from(&self.predecessors);
        for (quest, hash) in &self.hashes {
            if next.hashes.get(quest) == Some(hash) {
                continue;
            }
            let history = next.predecessors.entry(quest.clone()).or_default();
            if !history.iter().any(|entry| &entry.hash == hash) {
                history.push(DefinitionPredecessor {
                    hash: *hash,
                    defaults: self
                        .tracks
                        .values()
                        .filter(|track| &track.quest == quest)
                        .map(|track| (track.key.clone(), track.initial))
                        .collect(),
                });
            }
        }
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const QUEST: &str = "oteryn:quest.history-test";
    fn base() -> Result<QuestStateCatalogue, String> {
        QuestStateCatalogue::new(
            "history-1",
            vec![QuestTrack {
                key: "oteryn:quest-progress/history-test/old".into(),
                quest: QUEST.into(),
                initial: -1,
                min: -1,
                max: 10,
            }],
            vec![QuestTransition {
                key: "oteryn:quest-transition/history-test/old".into(),
                quest: QUEST.into(),
                effects: vec![QuestEffect {
                    track: "oteryn:quest-progress/history-test/old".into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(1),
                }],
                completes: false,
                experience: None,
            }],
        )
        .map_err(|e| format!("{e:?}"))
    }
    fn addition(base: &QuestStateCatalogue, suffix: &str) -> Result<QuestStateCatalogue, String> {
        let mut tracks = base.tracks.values().cloned().collect::<Vec<_>>();
        tracks.push(QuestTrack {
            key: format!("oteryn:quest-progress/history-test/{suffix}"),
            quest: QUEST.into(),
            initial: -1,
            min: -1,
            max: 1,
        });
        QuestStateCatalogue::new(
            base.content_revision.clone(),
            tracks,
            base.transitions.values().cloned().collect(),
        )
        .map_err(|e| format!("{e:?}"))
    }
    #[test]
    fn append_history_preserves_every_exact_predecessor_and_old_absent_defaults()
    -> Result<(), String> {
        let original = base()?;
        let original_hash = original.definition_hash(QUEST).ok_or("original hash")?;
        let mut npc = addition(&original, "npc")?;
        npc.npc_legacy.insert(
            QUEST.into(),
            (
                original_hash,
                original
                    .tracks
                    .values()
                    .map(|t| (t.key.clone(), t.initial))
                    .collect(),
            ),
        );
        let npc_hash = npc.definition_hash(QUEST).ok_or("NPC hash")?;
        let doors = npc.with_additive_extension(addition(&npc, "doors")?)?;
        let door_hash = doors.definition_hash(QUEST).ok_or("door hash")?;
        let room = doors.with_additive_extension(addition(&doors, "room")?)?;
        for (hash, count) in [(original_hash, 1), (npc_hash, 2), (door_hash, 3)] {
            let defaults = super::super::npc_rules::legacy_defaults(&room, QUEST, &hash)
                .ok_or("known predecessor")?;
            assert_eq!(defaults.len(), count);
            assert!(defaults.values().all(|v| *v == -1));
        }
        assert!(super::super::npc_rules::legacy_defaults(&room, QUEST, &[0; 32]).is_none());
        assert!(
            super::super::npc_rules::legacy_defaults(&room, "oteryn:quest.other", &npc_hash)
                .is_none()
        );
        Ok(())
    }
    #[test]
    fn append_history_refuses_changed_bounds_effects_missing_rules_and_revisions()
    -> Result<(), String> {
        let base = base()?;
        for kind in 0..4 {
            let mut next = addition(&base, "new")?;
            match kind {
                0 => {
                    next.tracks
                        .get_mut("oteryn:quest-progress/history-test/old")
                        .ok_or("track")?
                        .max = 9
                }
                1 => {
                    next.transitions
                        .get_mut("oteryn:quest-transition/history-test/old")
                        .ok_or("transition")?
                        .effects[0]
                        .effect = QuestEffectKind::Set(2)
                }
                2 => next.transitions.clear(),
                _ => next.content_revision = "different".into(),
            }
            assert!(base.with_additive_extension(next).is_err());
        }
        Ok(())
    }
    #[test]
    fn source_layers_preserve_the_actual_npc13_tibia_tales_definition() -> Result<(), String> {
        let loaded = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = loaded.catalogue();
        let hex = "8c6494650b3d22ccd66d6893c77f8b0ad16b06dfcd69725316150c1be49ee084";
        let bytes = (0..32)
            .map(|at| u8::from_str_radix(&hex[at * 2..at * 2 + 2], 16).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        let hash: [u8; 32] = bytes.try_into().map_err(|_| "hash length")?;
        let previous = super::super::loader::load_embedded_quest_state_at_layer("content-1", 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            previous
                .catalogue()
                .definition_hash("oteryn:quest.tibia_tales"),
            Some(hash)
        );
        assert!(super::super::loader::load_embedded_quest_state_at_layer("content-1", 3).is_err());
        let defaults =
            super::super::npc_rules::legacy_defaults(catalogue, "oteryn:quest.tibia_tales", &hash)
                .ok_or("actual prior definition missing")?;
        for suffix in ["01", "02", "03"] {
            let key = format!(
                "oteryn:quest-progress/quest/u11_50/hidden_threats/corym_works_door{suffix}"
            );
            assert!(!defaults.contains_key(&key));
            assert_eq!(catalogue.track(&key).ok_or("new flag")?.initial, -1);
        }
        assert!(
            catalogue
                .predecessors
                .keys()
                .all(|q| q == "oteryn:quest.tibia_tales")
        );
        assert_ne!(
            catalogue.definition_hash("oteryn:quest.tibia_tales"),
            Some(hash)
        );
        Ok(())
    }
}

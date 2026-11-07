use super::*;
use crate::map::facts::{QuestCallbackProjection, QuestSceneMutation};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_quest_scene(
        &self,
        map: &BundleMap,
        callback: &Callback,
        close: bool,
    ) -> Result<Option<QuestCallbackProjection>, UseDisposition> {
        let Some(scene) = &callback.effects.scene else {
            return Ok(None);
        };
        let edits = if close { &scene.close } else { &scene.open };
        let evacuations = if close {
            &scene.evacuate_close
        } else {
            &scene.evacuate_open
        };
        if evacuations.iter().any(|m| {
            [m.from, m.players_to, m.creatures_to]
                .into_iter()
                .any(|position| map.facts.house_tile(tile(position)))
        }) {
            return Err(UseDisposition::Rejected);
        }
        let mut mutations = Vec::new();
        for edit in edits {
            let at = tile(edit.position);
            if map.facts.house_tile(at) {
                return Err(UseDisposition::Rejected);
            }
            let base = map
                .overlay
                .base()
                .tile(at.x, at.y, at.floor)
                .ok_or(UseDisposition::Rejected)?;
            let source = if let Some(appearance) = edit.remove {
                let mut candidates = base
                    .ids()
                    .iter()
                    .zip(base.depths())
                    .filter(|(_, depth)| **depth == 0)
                    .enumerate()
                    .filter_map(|(ordinal, (id, _))| {
                        let ordinal = u8::try_from(ordinal).ok()?;
                        let key = placement_key(at, ordinal)?;
                        let facts = map.facts.base_entry(at, ordinal, *id)?;
                        (facts.appearance_id == appearance && !map.facts.base_hidden(key))
                            .then_some((key, map.facts.object_revision(key)))
                    });
                let first = candidates.next();
                if candidates.next().is_some() {
                    return Err(UseDisposition::Rejected);
                }
                if first.is_none() {
                    if edit.allow_missing {
                        continue;
                    }
                    return Err(UseDisposition::Rejected);
                }
                first
            } else {
                None
            };
            let created = edit
                .create
                .map(|appearance| {
                    map.facts
                        .movement_item_facts(appearance)
                        .filter(|facts| !facts.pickupable)
                        .ok_or(UseDisposition::Rejected)
                })
                .transpose()?;
            // Restoring a removed source object retains its placement identity and
            // revision; it does not manufacture another overlay object on every toggle.
            if source.is_none()
                && edit.remove.is_none()
                && let (Some(facts), Some(passable)) = (created, edit.passable)
            {
                let mut hidden = base
                    .ids()
                    .iter()
                    .zip(base.depths())
                    .filter(|(_, depth)| **depth == 0)
                    .enumerate()
                    .filter_map(|(ordinal, (id, _))| {
                        let ordinal = u8::try_from(ordinal).ok()?;
                        let key = placement_key(at, ordinal)?;
                        let original = map.facts.base_entry(at, ordinal, *id)?;
                        (original.appearance_id == facts.appearance_id
                            && map.facts.base_hidden(key))
                        .then_some((key, map.facts.object_revision(key)))
                    });
                if let Some((key, revision)) = hidden.next() {
                    if hidden.next().is_some() {
                        return Err(UseDisposition::Rejected);
                    }
                    mutations.push(QuestSceneMutation::Restore {
                        key,
                        revision,
                        facts,
                        passable,
                    });
                    continue;
                }
            }
            mutations.push(match (source, created, edit.passable) {
                (Some((key, revision)), Some(facts), Some(passable)) => {
                    QuestSceneMutation::Replace {
                        key,
                        revision,
                        facts,
                        passable,
                    }
                }
                (Some((key, revision)), None, None) => QuestSceneMutation::Remove { key, revision },
                (None, Some(facts), Some(passable)) => QuestSceneMutation::Create {
                    position: at,
                    facts,
                    passable,
                },
                _ => return Err(UseDisposition::Rejected),
            });
        }
        Ok(Some(QuestCallbackProjection::Scene {
            mutations,
            remove_self: scene.remove_self,
        }))
    }
}

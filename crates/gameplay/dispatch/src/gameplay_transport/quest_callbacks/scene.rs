use super::*;
use crate::map::facts::{QuestCallbackProjection, QuestSceneField, QuestSceneMutation};

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
        let mut oven_edits;
        let edits = if callback.key == "oteryn:quest-callback/authored-outlaw-camp-oven" {
            let [source, destination] = edits.as_slice() else {
                return Err(UseDisposition::Rejected);
            };
            let source_tile =
                crate::map::view::compose_tile(&map.overlay, &map.facts, tile(source.position))
                    .map_err(|_| UseDisposition::Rejected)?
                    .ok_or(UseDisposition::Rejected)?;
            let Some(appearance) = source_tile
                .entries
                .iter()
                .rev()
                .filter(|entry| !entry.facts.pickupable)
                .map(|entry| entry.facts.appearance_id)
                .next()
                .filter(|appearance| matches!(appearance, 2535 | 2536))
            else {
                // Crystal toggles the switch even if the oven is out of position.
                return Ok(None);
            };
            oven_edits = [source.clone(), destination.clone()];
            oven_edits[0].remove = Some(appearance);
            oven_edits[1].create = Some(appearance);
            oven_edits.as_slice()
        } else {
            edits.as_slice()
        };
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
                    if edit.create.is_none() {
                        let visible = map.facts.movement_items(at);
                        let mut added = visible.iter().filter_map(|item| match item.source {
                            crate::map::view::EntrySource::Added(entry)
                                if item.facts.appearance_id == appearance =>
                            {
                                Some(entry)
                            }
                            _ => None,
                        });
                        if let Some(entry) = added.next() {
                            if added.next().is_some() {
                                return Err(UseDisposition::Rejected);
                            }
                            mutations.push(QuestSceneMutation::RemoveCreated {
                                position: at,
                                entry,
                            });
                            continue;
                        }
                    }
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
                    field: self.prepare_outlaw_fire_field(map, callback, edit.position, facts)?,
                },
                _ => return Err(UseDisposition::Rejected),
            });
        }
        Ok(Some(QuestCallbackProjection::Scene {
            mutations,
            remove_self: scene.remove_self,
        }))
    }

    fn prepare_outlaw_fire_field(
        &self,
        map: &BundleMap,
        callback: &Callback,
        position: Position,
        facts: EntryFacts,
    ) -> Result<Option<QuestSceneField>, UseDisposition> {
        use oteryn_gameplay_quests::quest::outlaw_power_burn;
        if !outlaw_power_burn::shared_input(callback) || facts.appearance_id != 2118 {
            return Ok(None);
        }
        let mut stages = Vec::new();
        // Crystal Item::transform starts decay on the ring's replacement. The
        // separate Game.createItem call leaves its field in the initial form.
        if position == outlaw_power_burn::RING_POSITION {
            let mut deadline = self.owner_now().get();
            for (duration, appearance) in [
                (200_000_000, Some(2119)),
                (148_000_000, Some(2120)),
                (98_000_000, None),
            ] {
                deadline = deadline
                    .checked_add(duration)
                    .ok_or(UseDisposition::Exhausted)?;
                let next = if let Some(appearance) = appearance {
                    Some(
                        map.facts
                            .movement_item_facts(appearance)
                            .ok_or(UseDisposition::Rejected)?,
                    )
                } else {
                    None
                };
                stages.push((deadline, next));
            }
        }
        Ok(Some(QuestSceneField { stages }))
    }
}

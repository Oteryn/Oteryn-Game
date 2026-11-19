use super::*;
use crate::map::facts::{QuestCallbackProjection, QuestSceneMutation};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::SceneEdit;

pub(super) fn matches_delayed_scene_opening(
    map: &BundleMap,
    callback: &Callback,
    position: TilePos,
    ordinal: u8,
) -> bool {
    let Some(scene) = &callback.effects.scene else {
        return true;
    };
    let Some([opening, _]) = callback.lever else {
        return true;
    };
    if scene.delayed_open.is_none() {
        return true;
    }
    // Crystal 201189f8, actions_habitat_grass.lua:439-468. Both lever
    // appearances retain the same AID; only this one starts the terrain turn.
    map.overlay
        .base()
        .tile(position.x, position.y, position.floor)
        .and_then(|base| {
            base.ids()
                .iter()
                .zip(base.depths())
                .filter(|(_, depth)| **depth == 0)
                .nth(usize::from(ordinal))
                .map(|(id, _)| *id)
        })
        .and_then(|id| map.facts.base_entry(position, ordinal, id))
        .is_some_and(|entry| entry.appearance_id == opening)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_delayed_scene_spawn<'a>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        callback: &'a Callback,
    ) -> Result<Option<(&'a str, i64, MovementLocalPosition)>, UseDisposition> {
        let Some(spawn) = callback
            .effects
            .scene
            .as_ref()
            .and_then(|scene| scene.delayed_open.as_ref())
            .and_then(|delayed| delayed.spawn.as_ref())
        else {
            return Ok(None);
        };
        let at = local(spawn.position);
        // Crystal 201189f8, actions_habitat_grass.lua:439-449. The creature
        // enters on lever use and the later landscape does not admit it again.
        if map.facts.house_tile(tile(spawn.position))
            || !super::super::encounter_phases::placement(self.movement_cells(), runtime, at)
        {
            return Err(UseDisposition::Rejected);
        }
        let native = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or(UseDisposition::Rejected)?;
        native
            .install_companion_policies(runtime)
            .map_err(|_| UseDisposition::Rejected)?;
        let health = runtime
            .companion_policy(&spawn.creature)
            .map_err(|_| UseDisposition::Rejected)?
            .maximum_health;
        let prepared = (spawn.creature.as_str(), health, at);
        runtime
            .preflight_quest_creature_spawn(&[prepared])
            .map_err(|_| UseDisposition::Rejected)?;
        Ok(Some(prepared))
    }

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
        let mutations = prepare_edits(map, edits, false)?;
        if !close && let Some(delayed) = &scene.delayed_open {
            let due = self
                .owner_now()
                .get()
                .checked_add(u64::from(delayed.delay_ms) * 1_000)
                .ok_or(UseDisposition::Exhausted)?;
            return Ok(Some(QuestCallbackProjection::DelayedScene {
                due,
                immediate: mutations,
                delayed: prepare_edits(map, &delayed.edits, true)?,
            }));
        }
        Ok(Some(QuestCallbackProjection::Scene {
            mutations,
            remove_self: scene.remove_self,
        }))
    }
}

fn prepare_edits(
    map: &BundleMap,
    edits: &[SceneEdit],
    fixed_only: bool,
) -> Result<Vec<QuestSceneMutation>, UseDisposition> {
    let mut mutations = Vec::with_capacity(edits.len());
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
                    (facts.appearance_id == appearance
                        && !map.facts.base_hidden(key)
                        && (!fixed_only || !facts.pickupable && !facts.bound))
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
                    (original.appearance_id == facts.appearance_id && map.facts.base_hidden(key))
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
            (Some((key, revision)), Some(facts), Some(passable)) => QuestSceneMutation::Replace {
                key,
                revision,
                facts,
                passable,
            },
            (Some((key, revision)), None, None) => QuestSceneMutation::Remove { key, revision },
            (None, Some(facts), Some(passable)) => QuestSceneMutation::Create {
                position: at,
                facts,
                passable,
            },
            _ => return Err(UseDisposition::Rejected),
        });
    }
    Ok(mutations)
}

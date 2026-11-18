//! Habitat terrain swaps are scheduled only after the authoritative lever USE.
use super::*;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::zamulosh::{HABITATS, Habitat};

#[derive(Clone, Copy)]
pub(super) struct PendingHabitat {
    due: u64,
    scope: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    map: [u8; 32],
}

pub(super) fn is_habitat_callback(key: &str) -> bool {
    let key = key.strip_suffix("/reset").unwrap_or(key);
    HABITATS.iter().any(|source| source.key == key)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn schedule_zamulosh_habitat(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        map: &BundleMap,
        callback: &Callback,
    ) {
        let Some((index, source)) = HABITATS
            .iter()
            .enumerate()
            .find(|(_, source)| source.key == callback.key)
        else {
            return;
        };
        let Some(due) = self.owner_now().get().checked_add(2_000_000) else {
            return;
        };
        let Ok(mut pending) = self.states.quest_callbacks.habitats.lock() else {
            return;
        };
        if pending[index].is_some() {
            return;
        }
        pending[index] = Some(PendingHabitat {
            due,
            scope: runtime.encounter_owner_scope(),
            map: map.overlay.base().digest(),
        });
        drop(pending);
        self.spawn_lovely_habitat_creature(runtime, source);
        states.owner_wake.notify_one();
    }

    fn spawn_lovely_habitat_creature(&self, runtime: &mut ChannelRuntimeV1, source: &Habitat) {
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return;
        };
        if native.install_companion_policies(runtime).is_err() {
            return;
        }
        let Ok(policy) = runtime.companion_policy(source.creature) else {
            return;
        };
        let spawn = MovementLocalPosition {
            x: i32::from(source.spawn.0),
            y: i32::from(source.spawn.1),
            floor: 12,
        };
        let _ =
            runtime.commit_quest_creature_spawn(&[(source.creature, policy.maximum_health, spawn)]);
    }

    pub(in crate::gameplay_transport) async fn drain_zamulosh_terrain(&self) {
        let Some(map) = self.bundle_map() else { return };
        let _permit = match self.spell_lane_permit().await {
            Some(permit) => permit,
            None => return,
        };
        let runtime = self.runtime.lock().await;
        if runtime.owner_fence().is_err() {
            return;
        }
        let scope = runtime.encounter_owner_scope();
        let digest = map.overlay.base().digest();
        let now = self.owner_now().get();
        let Ok(mut pending) = self.states.quest_callbacks.habitats.lock() else {
            return;
        };
        for (slot, source) in pending.iter_mut().zip(&HABITATS) {
            let Some(timer) = *slot else { continue };
            if timer.scope != scope || timer.map != digest {
                *slot = None;
            } else if now >= timer.due {
                let origin = crate::map::overlay::TilePos {
                    x: source.origin.0,
                    y: source.origin.1,
                    floor: -12,
                };
                let _ = map.facts.publish_habitat_mirror(
                    map.overlay.base(),
                    &map.spell_tiles,
                    origin,
                    source.counterpart_x,
                );
                *slot = None;
            }
        }
    }
}

use crate::durability::quest_state::{CommittedQuestTransition, zamulosh::HabitatReset};
use oteryn_gameplay_quests::quest::zamulosh::{HABITAT_COUNT, HABITAT_RESET_SECONDS};

#[derive(Clone, Copy)]
pub(super) struct PendingHabitatReset {
    input: HabitatReset,
    timer: PendingHabitat,
    source: &'static Habitat,
}

/// Applies the same appearance guard used by the source Lua. Basin positions sit
/// outside the terrain area, so its later scenery swap cannot own the basin.
fn transform_basin(map: &BundleMap, source: &Habitat, before: u16, after: u16) {
    let at = crate::map::overlay::TilePos {
        x: source.basin.0,
        y: source.basin.1,
        floor: -12,
    };
    let Some(base) = map.overlay.base().tile(at.x, at.y, at.floor) else {
        return;
    };
    let mut basins = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, (&id, _))| {
            use crate::map::view::MapFacts;
            let ordinal = u8::try_from(ordinal).ok()?;
            let key = placement_key(at, ordinal)?;
            let facts = map.facts.base_entry(at, ordinal, id)?;
            (facts.appearance_id == before && !facts.pickupable && !map.facts.base_hidden(key))
                .then_some(key)
        });
    let Some(key) = basins.next() else { return };
    if basins.next().is_some() || map.facts.house_tile(at) {
        return;
    }
    let Some(replacement) = map.facts.quest_callback_replacement(key, after) else {
        return;
    };
    if map.facts.apply_quest_callback(
        key,
        map.facts.object_revision(key),
        replacement,
        &[],
        &[],
        false,
        None,
    ) {
        map.facts.emit_movement_effect(at, 38);
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn commit_zamulosh_basin_use(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        map: &BundleMap,
        callback: &Callback,
        committed: &CommittedQuestTransition,
    ) {
        let Some(source) = HABITATS.iter().find(|source| source.key == callback.key) else {
            return;
        };
        transform_basin(map, source, 11114, 22196);
        let QuestCause::Use(command) = committed.cause else {
            return;
        };
        if !committed
            .changes
            .iter()
            .any(|change| change.track == HABITAT_COUNT && change.after >= 7)
        {
            return;
        }
        let Some(input) = HabitatReset::from_use(command, source.key) else {
            return;
        };
        let Some(due) = self
            .owner_now()
            .get()
            .checked_add(HABITAT_RESET_SECONDS * 1_000_000)
        else {
            return;
        };
        let Ok(mut pending) = self.states.quest_callbacks.habitat_resets.lock() else {
            return;
        };
        if pending.len() >= 256 || pending.iter().any(|item| item.input == input) {
            return;
        }
        pending.push(PendingHabitatReset {
            input,
            source,
            timer: PendingHabitat {
                due,
                scope: runtime.encounter_owner_scope(),
                map: map.overlay.base().digest(),
            },
        });
        states.owner_wake.notify_one();
    }

    pub(in crate::gameplay_transport) async fn drain_zamulosh_resets(&self) {
        let (Some(map), Some(catalogue)) = (self.bundle_map(), self.quest_catalogue.as_ref())
        else {
            return;
        };
        let Some(_permit) = self.spell_lane_permit().await else {
            return;
        };
        let scope = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err() {
                return;
            }
            runtime.encounter_owner_scope()
        };
        let digest = map.overlay.base().digest();
        let now = self.owner_now().get();
        let candidates = {
            let Ok(mut pending) = self.states.quest_callbacks.habitat_resets.lock() else {
                return;
            };
            pending.retain(|item| item.timer.scope == scope && item.timer.map == digest);
            pending
                .iter()
                .copied()
                .filter(|item| item.timer.due <= now)
                .take(8)
                .collect::<Vec<_>>()
        };
        for item in candidates {
            let session = item.input.session();
            let completed = match self.current_quest_fence(session).await {
                Ok(Some(fence)) => {
                    let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
                    match slot.commit_zamulosh_habitat_reset(
                        self.root, self.character, self.holder, fence, item.input,
                        std::sync::Arc::clone(catalogue),
                    ).await {
                        Ok(QuestTransitionOutcome::Committed(ref result) | QuestTransitionOutcome::AlreadyCommitted(ref result)) => {
                            self.apply_committed_quest_copy(session, false, result);
                            self.schedule_quest_refresh(session);
                            true
                        }
                        Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected) => false,
                        _ => continue,
                    }
                }
                Ok(None) => false,
                Err(()) => continue,
            };
            if completed {
                let runtime = self.runtime.lock().await;
                if runtime.owner_fence().is_ok() && runtime.encounter_owner_scope() == scope {
                    transform_basin(map, item.source, 22196, 11114);
                }
            }
            if let Ok(mut pending) = self.states.quest_callbacks.habitat_resets.lock() {
                pending.retain(|other| other.input != item.input);
            }
        }
    }
}

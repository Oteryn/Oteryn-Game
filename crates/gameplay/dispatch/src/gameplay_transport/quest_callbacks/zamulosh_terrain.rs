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

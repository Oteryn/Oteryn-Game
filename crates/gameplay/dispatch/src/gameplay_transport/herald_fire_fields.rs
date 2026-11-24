//! The Herald's source attack runs on the private Instance AI clock.
use std::sync::Arc;

use super::ComposedFreshAdmission;
use crate::content::bosses::activity_template::herald::{HeraldFireFieldAttack, HeraldRedFloors};
use crate::durability::{
    creature_source_items::{HeraldFieldClear, HeraldFieldRefresh},
    spell_owner_commit::SpellLanePermit,
};

#[derive(Debug, Default)]
pub(super) struct HeraldFieldSchedule {
    next_cast_us: Option<u64>,
    pending: Option<Arc<HeraldFieldRefresh>>,
    floor_reset: Option<Arc<HeraldFloorReset>>,
}

impl HeraldFieldSchedule {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.pending
            .as_ref()
            .map(|refresh| refresh.due_us())
            .or(self.next_cast_us)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn herald_activity_room(
        &self,
    ) -> Option<&crate::content::bosses::activity_template::QualifiedActivityRoomTemplate> {
        let scope = self.bundle_map()?.overlay.runtime_scope();
        if !matches!(scope, crate::foundation::RuntimeScopeRefV1::Instance { .. }) {
            return None;
        }
        let generation = self.runtime.lock().await.binding().scope_generation().get();
        let room = self.activity_runtimes.get(scope, generation)?;
        let key = room.template.as_ref()?;
        self.activity_runtimes
            .template(&key.key, &key.revision)
            .filter(|template| template.has_herald_callbacks())
    }

    pub(super) async fn drain_herald_fire_field_attack(
        &self,
        permit: &SpellLanePermit,
        room: &crate::content::QualifiedNativeEntryRoom,
        now_us: u64,
    ) -> bool {
        let Some(map) = self.bundle_map() else {
            return true;
        };
        let scope = map.overlay.runtime_scope();
        if !matches!(scope, crate::foundation::RuntimeScopeRefV1::Instance { .. }) {
            return true;
        }
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return true;
        };
        let due = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err() || runtime.binding().runtime_scope() != scope {
                return true;
            }
            let Some(owner) = self
                .activity_runtimes
                .get(scope, runtime.binding().scope_generation().get())
            else {
                return true;
            };
            let Some(key) = owner.template.as_ref() else {
                return true;
            };
            let Some(template) = self.activity_runtimes.template(&key.key, &key.revision) else {
                return true;
            };
            let Some(source) = template.herald_fire_field_attack() else {
                return true;
            };
            let mut states = self.states.spell_states.lock().await;
            let schedule = &mut states.herald_field_schedule;
            if let Some(refresh) = &schedule.pending {
                if now_us < refresh.due_us() {
                    return true;
                }
                Arc::clone(refresh)
            } else {
                if !runtime.activity_herald_is_alive() {
                    schedule.next_cast_us = None;
                    return true;
                }
                let Ok(census) = runtime.positioned_melee_census() else {
                    return true;
                };
                if !census.iter().any(|(_, _, session)| session.is_some()) {
                    schedule.next_cast_us = None;
                    return true;
                }
                let Some(next_cast_us) = schedule.next_cast_us else {
                    schedule.next_cast_us = now_us.checked_add(HeraldFireFieldAttack::INTERVAL_US);
                    return true;
                };
                if now_us < next_cast_us {
                    return true;
                }
                let Some(actor) = runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|creature| {
                        runtime.matches_live_creature_identity(
                            creature.actor,
                            HeraldFireFieldAttack::CREATURE.as_bytes(),
                        )
                    })
                    .map(|creature| creature.actor)
                else {
                    return true;
                };
                let refresh = match HeraldFieldRefresh::from_live_cast(
                    &runtime, room, native, source, actor, now_us,
                ) {
                    Ok(refresh) => refresh,
                    Err(error) => {
                        super::operator_event(&format!(
                            "herald_fire_field_prepare_deferred error={error:?}"
                        ));
                        return true;
                    }
                };
                // The Instance BundleMap owns a separate outbox from the origin Channel.
                for tile in source
                    .cells()
                    .filter_map(crate::content::bundle_spell_tiles::tile_pos)
                {
                    let _ = map
                        .facts
                        .emit_movement_effect(tile, HeraldFireFieldAttack::EFFECT);
                }
                schedule.pending = Some(Arc::new(refresh));
                schedule.next_cast_us = now_us.checked_add(HeraldFireFieldAttack::INTERVAL_US);
                return true;
            }
        };
        if let Err(error) = self
            .root
            .commit_herald_field_refresh(permit, self.character, self.holder, &due, now_us)
            .await
        {
            super::operator_event(&format!(
                "herald_fire_field_refresh_deferred error={error:?}"
            ));
            return false;
        }
        self.states.loop_ground.invalidate();
        // Retain the exact cast until its committed Ground state protects the next HP turn.
        if super::loop_move::refresh_herald_rune_fields(self)
            .await
            .is_none()
        {
            return false;
        }
        self.states
            .spell_states
            .lock()
            .await
            .herald_field_schedule
            .pending = None;
        true
    }
}

type Plate = (u64, crate::map::view::EntryFacts);
#[derive(Debug)]
struct HeraldFloorReset {
    clear: HeraldFieldClear,
    plates: [Plate; 3],
}

fn red_floors(map: &crate::map::boot::BundleMap) -> Option<[Plate; 3]> {
    use crate::map::{
        overlay::TilePos,
        view::{self, EntrySource},
    };
    let plate = |(x, y)| {
        let tile =
            view::compose_tile(&map.overlay, &map.facts, TilePos { x, y, floor: -15 }).ok()??;
        tile.entries.into_iter().find_map(|entry| {
            let EntrySource::Base { placement_key, .. } = entry.source else {
                return None;
            };
            matches!(
                entry.facts.appearance_id,
                HeraldRedFloors::UNSTEPPED | HeraldRedFloors::STEPPED
            )
            .then_some((placement_key, entry.facts))
        })
    };
    let [first, second, third] = HeraldRedFloors::PLATES.map(plate);
    Some([first?, second?, third?])
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn stage_herald_red_floor_step(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        states: &mut super::actor_spell::ChannelSpellStates,
        source: HeraldRedFloors,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        step: crate::foundation::MovementPositionSnapshot,
    ) -> bool {
        use crate::map::{facts::MovementSpeech, view::MapFacts};
        let at = step.position();
        let Some(pos) = crate::content::bundle_spell_tiles::tile_pos(crate::content::LogicalCell {
            x: at.x,
            y: at.y,
            z: i32::from(at.floor),
        }) else {
            return false;
        };
        let Some(index) = HeraldRedFloors::PLATES
            .iter()
            .position(|cell| pos.floor == -15 && *cell == (pos.x, pos.y))
        else {
            return false;
        };
        let Some(map) = self.bundle_map() else {
            return false;
        };
        let Some(plates) = red_floors(map) else {
            return false;
        };
        let (key, mut floor) = plates[index];
        if floor.appearance_id != HeraldRedFloors::UNSTEPPED {
            return false;
        }
        let Ok(clear) =
            HeraldFieldClear::from_committed_step(runtime, source, actor, session, step)
        else {
            return false;
        };
        floor.appearance_id = HeraldRedFloors::STEPPED;
        if !map.facts.apply_quest_callback(
            key,
            map.facts.object_revision(key),
            floor,
            &[],
            &[],
            false,
            None,
        ) {
            return false;
        }
        map.facts.emit_movement_speech(MovementSpeech {
            speaker: actor,
            position: pos,
            talk_type: 36,
            text: "Click!".into(),
            captured_name: self.states.chat.speaker_name(session).and_then(|name| {
                oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(&name).ok()
            }),
        });
        let mut plates = plates;
        plates[index].1 = floor;
        if plates
            .iter()
            .all(|(_, facts)| facts.appearance_id == HeraldRedFloors::STEPPED)
        {
            states.herald_field_schedule.floor_reset =
                Some(Arc::new(HeraldFloorReset { clear, plates }));
            return true;
        }
        false
    }

    pub(super) async fn drain_herald_floor_reset(&self, permit: &SpellLanePermit) -> bool {
        use crate::map::view::{EntrySource, MapFacts};
        let reset = self
            .states
            .spell_states
            .lock()
            .await
            .herald_field_schedule
            .floor_reset
            .clone();
        let Some(reset) = reset else { return true };
        let Some(map) = self.bundle_map() else {
            return false;
        };
        if let Err(error) = self
            .root
            .commit_herald_field_clear(permit, self.character, self.holder, &reset.clear)
            .await
        {
            super::operator_event(&format!("herald_red_floor_clear_deferred error={error:?}"));
            return false;
        }
        self.states.loop_ground.invalidate();
        if super::loop_move::refresh_herald_rune_fields(self)
            .await
            .is_none()
        {
            return false;
        }
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.owner_fence().is_err()
            || map.overlay.runtime_scope() != runtime.binding().runtime_scope()
        {
            return false;
        }
        let [(key, floor), second, third] = reset.plates.map(|(key, mut floor)| {
            floor.appearance_id = HeraldRedFloors::UNSTEPPED;
            (key, floor)
        });
        if !map.facts.apply_quest_callback(
            key,
            map.facts.object_revision(key),
            floor,
            &[],
            &[(second.0, second.1, true), (third.0, third.1, true)],
            false,
            None,
        ) {
            return false;
        }
        for (pos, entry) in map.facts.movement_item_entries() {
            if pos.floor == -15
                && (32487..=32504).contains(&pos.x)
                && (32649..=32664).contains(&pos.y)
                && matches!(entry.facts.appearance_id, 2118..=2120)
                && let EntrySource::Added(item) = entry.source
            {
                let _ = map
                    .facts
                    .remove_movement_item(pos, item.get(), entry.facts.appearance_id);
            }
        }
        for creature in runtime.visible_entities().creatures {
            let at = creature.position;
            if at.floor == 15
                && at.x.abs_diff(32496) <= 12
                && at.y.abs_diff(32656) <= 12
                && !runtime.matches_live_creature_identity(
                    creature.actor,
                    HeraldFireFieldAttack::CREATURE.as_bytes(),
                )
            {
                let _ = runtime.remove_encounter_actor(creature.actor);
            }
        }
        states.herald_field_schedule.floor_reset = None;
        true
    }
}

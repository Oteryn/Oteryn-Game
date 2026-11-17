//! Native ladder USE shares the sealed rope/stair movement owner.
use super::ComposedFreshAdmission;
use super::connection::{UseCommand, UseOutcome};
use crate::foundation::ExactActorRef;
use crate::map::boot::BundleMap;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_native_bundle_ladder(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        let key = super::world_map::resolve_map_target(
            &super::world_map::MapViewSource {
                overlay: &map.overlay,
                facts: &map.facts.pinned(),
                content_generation: self
                    .runtime
                    .lock()
                    .await
                    .content_pin()
                    .client_artifact_digest(),
                reset_epoch: self.reset_epoch,
            },
            target,
            |_| None::<()>,
        )
        .ok()?;
        let (pos, ordinal) = crate::map::view::placement_of(key.placement_key)?;
        let base = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
        let id = *base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, d)| **d == 0)
            .nth(usize::from(ordinal))?
            .0;
        let entry = map.facts.base_entry(pos, ordinal, id)?;
        if !crate::map::floor::is_ladder_appearance(entry.appearance_id) {
            return None;
        }
        self.use_resolved_ladder(
            actor,
            command,
            map,
            pos,
            crate::map::floor::LadderTarget::Base {
                key: key.placement_key,
                revision: target.expected_revision,
            },
        )
        .await
    }

    pub(super) async fn use_ladder_handle(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: super::item_view::ItemKey,
    ) -> Option<UseOutcome> {
        let super::item_view::ItemKey::MapAdded(id) = item else {
            return None;
        };
        let map = self.bundle_map()?;
        let Some((pos, entry_id)) = map.facts.movement_item(id) else {
            return Some(UseOutcome {
                disposition: UseDisposition::StaleState,
                ..UseOutcome::rejected()
            });
        };
        let entry = map
            .facts
            .movement_items(pos)
            .into_iter()
            .find(|entry| entry.source == crate::map::view::EntrySource::Added(entry_id))?;
        if !crate::map::floor::is_ladder_appearance(entry.facts.appearance_id) {
            return None;
        }
        self.use_resolved_ladder(
            actor,
            command,
            map,
            pos,
            crate::map::floor::LadderTarget::Added { id },
        )
        .await
    }

    async fn use_resolved_ladder(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        pos: crate::map::overlay::TilePos,
        target: crate::map::floor::LadderTarget,
    ) -> Option<UseOutcome> {
        let Some(_permit) = self.spell_lane_permit().await else {
            return Some(UseOutcome::rejected());
        };
        if !self.refresh_pvp_locks(actor, command.game_session_id).await {
            return Some(UseOutcome::rejected());
        }
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let session = command.game_session_id;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
            || !self.states.player_trades.physical(session, None)
        {
            return Some(UseOutcome::rejected());
        }
        if !self
            .states
            .world_gate
            .lock()
            .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
        {
            return Some(UseOutcome {
                disposition: UseDisposition::Exhausted,
                ..UseOutcome::rejected()
            });
        }
        let expected = runtime.read_actor_position(actor).ok()?;
        let blocking = crate::map::floor::BundleDoors(&map.facts);
        let blocked = super::pvp_live::pz_blocked(&states, actor, session, self.owner_now());
        let proof = crate::map::floor::prepare_bundle_ladder(
            &runtime,
            &self.movement_cells,
            map,
            actor,
            session,
            expected,
            crate::content::LogicalCell {
                x: i32::from(pos.x),
                y: i32::from(pos.y),
                z: -i32::from(pos.floor),
            },
            target,
            blocked,
            &blocking,
        );
        let disposition = match proof {
            Ok(proof) => {
                use oteryn_gameplay_spell_rules::cast::{STAIRHOP_COOLDOWN_KEY, STAIRHOP_MICROS};
                let Some(until) = self.owner_now().checked_add(STAIRHOP_MICROS).ok() else {
                    return Some(UseOutcome::rejected());
                };
                if runtime.commit_source_step(proof).is_err() {
                    UseDisposition::Rejected
                } else {
                    if let Some(state) = states.get_mut(&runtime, actor, session) {
                        state
                            .cooldowns
                            .groups
                            .insert(STAIRHOP_COOLDOWN_KEY.into(), until);
                    }
                    self.states.chat.close_out_of_range(&runtime, session);
                    UseDisposition::Committed
                }
            }
            Err(_) => UseDisposition::NotPossible,
        };
        let entered = (disposition == UseDisposition::Committed)
            .then(|| runtime.read_actor_position(actor).ok())
            .flatten();
        drop(states);
        drop(runtime);
        if let Some(entered) = entered {
            self.step_quest_callback(&_permit, actor, session, expected, entered)
                .await;
        }
        Some(UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        })
    }
}

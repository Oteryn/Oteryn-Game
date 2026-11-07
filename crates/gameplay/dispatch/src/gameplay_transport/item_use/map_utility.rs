//! Crystal 9f5a72c ladder_up.lua: dropdown 435 descends at the item's x/y.
//! Map identity, current actor control and the sealed native landing proof remain authoritative.
use super::*;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::map::view::EntrySource;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_map_utility(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        command: UseCommand,
        key: ItemKey,
    ) -> UseDisposition {
        let session = command.game_session_id;
        let optional_pvp = self.special_movement_policy(permit, actor, session).await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let result = (|| -> Result<UseDisposition, UseDisposition> {
            if !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|f| f.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, session)
                || !self.states.player_trades.physical(session, None)
            {
                return Err(UseDisposition::Rejected);
            }
            let at = super::super::bundle_map_position(&mut runtime, self.movement_cells, actor)
                .ok_or(UseDisposition::Rejected)?;
            let facts = at.map.facts.pinned();
            let source = super::super::world_map::MapViewSource {
                overlay: &at.map.overlay,
                facts: &facts,
                content_generation: at.content_generation,
                reset_epoch: self.reset_epoch,
            };
            use super::super::world_map::MapHandleTarget;
            let target = super::super::world_map::resolve_map_handle(&source, key)?;
            let position = match target {
                MapHandleTarget::Base { pos, .. } | MapHandleTarget::Added { pos, .. } => pos,
                _ => return Err(UseDisposition::NothingToUse),
            };
            let tile = crate::map::view::compose_tile(&at.map.overlay, &facts, position)
                .map_err(|_| UseDisposition::Rejected)?
                .ok_or(UseDisposition::StaleState)?;
            let entry = tile
                .entries
                .iter()
                .find(|entry| match (target, entry.source) {
                    (
                        MapHandleTarget::Base { placement_key, .. },
                        EntrySource::Base {
                            placement_key: actual,
                            ..
                        },
                    ) => placement_key == actual,
                    (MapHandleTarget::Added { id, .. }, EntrySource::Added(actual)) => {
                        id == actual.get()
                    }
                    _ => false,
                })
                .ok_or(UseDisposition::StaleState)?;
            if entry.facts.appearance_id != 435 {
                return Err(UseDisposition::NothingToUse);
            }
            if at.at.floor != i16::from(position.floor)
                || at.at.x.abs_diff(i32::from(position.x)) > 1
                || at.at.y.abs_diff(i32::from(position.y)) > 1
            {
                return Err(UseDisposition::TooFar);
            }
            let expected = runtime
                .read_actor_position(actor)
                .map_err(|_| UseDisposition::Rejected)?;
            let destination = crate::foundation::MovementLocalPosition {
                x: i32::from(position.x),
                y: i32::from(position.y),
                floor: i16::from(position.floor)
                    .checked_neg()
                    .and_then(|z| z.checked_add(1))
                    .ok_or(UseDisposition::NotPossible)?,
            };
            let blocked =
                super::super::pvp_live::pz_blocked(&states, actor, session, self.owner_now());
            let closed = crate::map::floor::BundleDoors(at.map.facts.doors());
            let proof = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                self.movement_cells,
                actor,
                session,
                expected,
                destination,
                blocked,
                &closed,
            )
            .map_err(|_| UseDisposition::NotPossible)?;
            let snapshot = runtime
                .commit_source_step(proof)
                .map_err(|_| UseDisposition::Rejected)?;
            states.stage_special_movement_step(
                &runtime,
                at.map,
                actor,
                session,
                self.owner_now().get(),
                Some(expected.position()),
                snapshot.position(),
                optional_pvp,
            );
            let underwater = super::super::paced_movement::underwater_drown::underwater;
            states.stage_underwater_step(
                &runtime,
                actor,
                session,
                self.owner_now().get(),
                underwater(at.map, expected.position()),
                underwater(at.map, snapshot.position()),
            );
            Ok(UseDisposition::Committed)
        })();
        let disposition = result.unwrap_or_else(|refusal| refusal);
        if let Some(gate) = states.item_uses.gate(&runtime, actor, session) {
            gate.settle(command.command_id, disposition, &[]);
        }
        disposition
    }
}

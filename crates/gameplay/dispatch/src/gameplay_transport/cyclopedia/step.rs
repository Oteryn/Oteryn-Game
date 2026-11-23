//! Discovery runs on the accepted movement path, including clients with no map window open.
use super::*;
use crate::foundation::{ExactActorRef, GameSessionId, MovementPositionSnapshot};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn cyclopedia_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) {
        if self.cyclopedia_content().is_none() {
            return;
        }
        let Some(current) =
            super::super::character_gameplay_fence::current(self, session, false).await
        else {
            return;
        };
        let mut slot = self.revision_sequencer.acquire(current.character_id).await;
        let area_at = {
            let mut runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if current.runtime_scope != binding.runtime_scope()
                || current.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || !runtime
                    .read_actor_position(actor)
                    .is_ok_and(|position| position == entered)
            {
                return;
            }
            let Some(position) =
                super::super::bundle_map_position(&mut runtime, self.movement_cells(), actor)
            else {
                return;
            };
            let at = Point {
                x: position.at.x,
                y: position.at.y,
                floor: position.at.floor,
            };
            content::admit_at(self, at).map(|area| (area, at, position.map))
        };
        let Some((area, at, map)) = area_at else {
            return;
        };
        let binding: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:cyclopedia-step:v1")
            .chain_update(actor.placement_identity())
            .chain_update(session.as_bytes())
            .chain_update(entered.revision().to_be_bytes())
            .chain_update(self.reset_epoch.to_be_bytes())
            .finalize()
            .into();
        let mut compact = [0; 8];
        compact.copy_from_slice(&binding[..8]);
        let Ok(id) = CommandId::new(u64::from_be_bytes(compact).max(1)) else {
            return;
        };
        let occurrence =
            super::super::npc_trade_command::derived(b"cyclopedia:step", session, id.get());
        let request = MapRequest {
            command: CommandRef::new(session, id),
            action: MapAction::Find,
            binding,
            occurrence,
            bundle: area.digest,
            epoch: self.reset_epoch,
            area: area.area,
            legacy_area: area.legacy_area,
            subarea: area.subarea,
            subareas: area.subareas,
            milestone_subareas: discovery_roster(map),
            pool: area.pool,
            at,
            fee: None,
            transaction: occurrence,
            event: occurrence,
            now: super::super::npc_trade_command::unix_ms_now(),
        };
        if slot.cursor(self.root, self.character).await.is_err() {
            return;
        }
        match slot
            .cyclopedia(
                self.root,
                self.character,
                self.holder,
                current,
                request,
                &Premium(None),
            )
            .await
        {
            Ok(commit) => {
                if commit.discovered_now {
                    self.states.tutorial_sessions.notice(
                        session,
                        oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(
                            "Subarea discovered! All seven points of interest found.".into(),
                        ),
                    );
                }
            }
            Err(_) => super::super::operator_event("cyclopedia_step_unavailable"),
        }
    }
}

//! Discovery runs on the accepted movement path, including clients with no map window open.
use super::*;
use crate::durability::DurabilityError;
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
        let retry = super::super::AdmissionRetryBudget::for_command(self.root);
        // STEP has already committed. Give a temporarily busy holder time to return
        // before the next movement can make this landing stale.
        if retry
            .retry_command_guard(async || {
                self.credit_cyclopedia_landing(actor, session, entered)
                    .await
            })
            .await
            .is_err()
        {
            super::super::operator_event("cyclopedia_step_unavailable");
        }
    }

    async fn credit_cyclopedia_landing(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> Result<(), DurabilityError> {
        let Some(current) =
            super::super::character_gameplay_fence::current_once(self, session, false).await?
        else {
            return Ok(());
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
                return Ok(());
            }
            let Some(position) =
                super::super::bundle_map_position(&mut runtime, self.movement_cells(), actor)
            else {
                return Ok(());
            };
            let at = Point {
                x: position.at.x,
                y: position.at.y,
                floor: position.at.floor,
            };
            content::admit_at(self, at).map(|area| (area, at))
        };
        let Some((area, at)) = area_at else {
            return Ok(());
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
            return Ok(());
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
            pool: area.pool,
            at,
            fee: None,
            transaction: occurrence,
            event: occurrence,
            now: super::super::npc_trade_command::unix_ms_now(),
        };
        let before = match slot.cursor(self.root, self.character).await {
            Ok(before) => before,
            Err(crate::durability::character_authority::CharacterAuthorityError::Unavailable(
                error,
            )) => {
                return Err(error);
            }
            Err(_) => return Ok(()),
        };
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
            Ok(bytes) => {
                if wire::decode_retained_result(&bytes).is_ok_and(|result| {
                    result.discovered && result.found == 7 && result.revision > before.get()
                }) {
                    self.states.tutorial_sessions.notice(
                        session,
                        oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(
                            "Subarea discovered! All seven points of interest found.".into(),
                        ),
                    );
                }
            }
            Err(CharacterProgressionError::Unavailable(error)) => return Err(error),
            Err(_) => super::super::operator_event("cyclopedia_step_unavailable"),
        }
        Ok(())
    }
}

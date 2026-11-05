//! A cancelled ordinary USE keeps its prepared owner effect until its exact
//! receipt is known. Reconciliation never executes the item action again.
use super::*;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::AssignmentState;
use crate::foundation::{GameSessionId, GameSessionState, RuntimeScopeRefV1};
use oteryn_gameplay_items::novelty_items::NoveltyKind;
use oteryn_protocol_oteryn::world_object::UseDisposition;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn reconcile_pending_item_effects(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        if !self
            .states
            .spell_states
            .lock()
            .await
            .pending_item_effects
            .iter()
            .any(|memo| memo.actor == actor && memo.request.command.game_session_id() == session)
        {
            return true;
        }
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return false;
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return false;
        };
        let (fence, request, cosmetic) = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            let Some(memo) = states.pending_item_effects.iter().find(|memo| {
                memo.actor == actor && memo.request.command.game_session_id() == session
            }) else {
                return true;
            };
            // Control loss may already be marked, but the same actor, session
            // and qualified scope must still own the retained successor.
            let current = match memo.fence.runtime_scope {
                RuntimeScopeRefV1::Channel { .. } => {
                    runtime.binding().runtime_scope() == memo.fence.runtime_scope
                        && runtime.binding().scope_generation()
                            == memo.fence.scope_ownership_generation
                        && runtime.player_control_facts(actor, session).is_ok()
                }
                RuntimeScopeRefV1::Instance { .. } => {
                    runtime
                        .house_actor_state(actor, session)
                        .is_ok_and(|house| {
                            house.scope == memo.fence.runtime_scope
                                && house.generation == memo.fence.scope_ownership_generation
                        })
                        && runtime.house_player_control_facts(actor, session).is_ok()
                }
            };
            if !current {
                return false;
            }
            (
                memo.fence,
                memo.request.clone(),
                memo.effects.kind == NoveltyKind::Cosmetic,
            )
        };
        let content = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        // The existing owner takes the original command's transaction advisory
        // lock before reading history. Absence therefore proves that its former
        // writer has ended without a commit; it is safe to release the memo.
        let result = self
            .root
            .consume_transform_detailed(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &content,
                true,
            )
            .await;
        match result {
            Ok(Some(receipt)) => {
                // History establishes the outcome, never current permission to install
                // its prepared live successor. Known refusals and absence need no install.
                if receipt.disposition == UseDisposition::Committed
                    && !self.pending_item_owner_current(actor, fence).await
                {
                    return false;
                }
                if cosmetic {
                    self.finish_cosmetic_item(actor, &request, receipt)
                        .await
                        .is_ok()
                } else {
                    self.finish_novelty_item(actor, &request, receipt)
                        .await
                        .is_ok()
                }
            }
            Ok(None) => {
                self.states
                    .spell_states
                    .lock()
                    .await
                    .release_novelty_effect(actor, &request);
                true
            }
            Err(_) => false,
        }
    }

    async fn pending_item_owner_current(
        &self,
        actor: ExactActorRef,
        original: CurrentCharacterItemFence,
    ) -> bool {
        if self
            .root
            .require_current_node_registration(self.holder)
            .await
            .is_err()
        {
            return false;
        }
        let Ok(Some(assignment)) = self
            .root
            .read_runtime_scope_assignment(original.runtime_scope)
            .await
        else {
            return false;
        };
        if assignment.state != AssignmentState::Assigned
            || assignment.scope != original.runtime_scope
            || assignment.ownership_generation != original.scope_ownership_generation.get()
            || assignment.holder != Some(self.holder.fact())
        {
            return false;
        }
        let Ok((current, _)) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(original.game_session_id)
            .await
        else {
            return false;
        };
        if !matches!(
            current.session_state(),
            GameSessionState::Active | GameSessionState::Reconnectable
        ) {
            return false;
        }
        let Some(current) = super::super::item_fence_of(current) else {
            return false;
        };
        // Ordinary same-session reconnect advances only connection generation.
        // A new lease, Character or scope cannot inherit this retained successor.
        if current.character_id != original.character_id
            || current.game_session_id != original.game_session_id
            || current.character_lease_generation != original.character_lease_generation
            || current.runtime_scope != original.runtime_scope
            || current.scope_ownership_generation != original.scope_ownership_generation
            || current.connection_generation.get() < original.connection_generation.get()
        {
            return false;
        }
        let runtime = self.runtime.lock().await;
        runtime.owner_fence().is_ok()
            && runtime.binding().runtime_scope() == current.runtime_scope
            && runtime.binding().scope_generation() == current.scope_ownership_generation
            && runtime
                .bound_attacker_lease(actor, current.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == current.character_id.as_bytes()
                        && lease.generation() == current.character_lease_generation
                })
    }
}

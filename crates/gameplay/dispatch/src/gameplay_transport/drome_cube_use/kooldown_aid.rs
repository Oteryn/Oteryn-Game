//! W09: ordinary USE reaches the instant Kooldown Aid owner before the Cube owner.
//! A historical receipt is returned without resetting later spells. Only a retained,
//! exact PREPARE may apply the instant effect after a known commit, once, in the item lane.

use super::super::*;
use crate::durability::item_use::concoction::kooldown_aid::KOOLDOWN_AID_KEY;

const RESET_TEXT: &str = "Your spells are no longer on cooldown.";

impl ComposedFreshAdmission<'_, '_, '_> {
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn lose_next_kooldown_ack() {
        test_faults::lose_next_ack();
    }

    pub(in crate::gameplay_transport) async fn use_kooldown_aid_impl(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if intent.use_with.is_some()
            || intent.use_with_map_item.is_some()
            || intent.use_with_position.is_some()
            || !matches!(
                intent.target,
                UseTarget::Item(_) | UseTarget::ItemByDefinition(_)
            )
        {
            return Ok(None);
        }
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        if fence.game_session_id != command.game_session_id {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let binding: [u8; 32] =
            Sha256::digest(encode_item_use_intent(intent).map_err(|_| ())?).into();
        let Some(_permit) = self.spell_lane_permit().await else {
            return self.drome_lane_unavailable(actor, command).await;
        };
        let pending = {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            states
                .item_uses
                .gate(&runtime, actor, command.game_session_id)
                .and_then(|gate| gate.cube_pending.clone())
        };
        let retry = AdmissionRetryBudget::for_command(self.root);
        match retry_cube_read(&retry, "kooldown_aid_receipt", || async {
            self.root
                .read_kooldown_aid_receipt(self.character, fence, command_ref, binding)
                .await
        })
        .await
        {
            Ok(true) => {
                self.finish_kooldown_aid(actor, command, binding, UseDisposition::Committed)
                    .await?;
                return Ok(Some(outcome(UseDisposition::Committed)));
            }
            Ok(false) => {}
            Err(ItemUseError::Unavailable(_)) => return Err(()),
            Err(_) => return Ok(Some(outcome(UseDisposition::Rejected))),
        }
        if let Some(pending) = pending.as_ref()
            && pending.request.kind == ItemUseKind::Concoction
            && pending.request.command_id == command.command_id
        {
            if pending.binding() != Some(binding) {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            if !self.cube_actor_live(actor, command.game_session_id).await {
                return Err(());
            }
            return self
                .commit_kooldown_attempt(actor, command, pending.request.clone(), binding)
                .await
                .map(|result| Some(outcome(result)));
        }
        let native = match self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        {
            Some(native) => native,
            None => return Ok(None),
        };
        let index = {
            let runtime = self.runtime.lock().await;
            match pinned_index(&runtime, self.active_generation) {
                Some(index) => index,
                None => return Ok(None),
            }
        };
        let wanted = match intent.target {
            UseTarget::ItemByDefinition(id) => {
                let Some((key, revision)) = index.definition_at(id) else {
                    return Ok(None);
                };
                if key != KOOLDOWN_AID_KEY {
                    return Ok(None);
                }
                Some(TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.into(),
                    revision_ref: revision.into(),
                })
            }
            _ => None,
        };
        let inventory = match retry_item_use_holder(&retry, async || {
            self.root
                .read_item_use_inventory(self.character, self.holder, fence)
                .await
        })
        .await
        {
            Ok(inventory) => inventory,
            Err(ItemUseError::Unavailable(_)) => return Err(()),
            Err(_) => return Ok(Some(outcome(UseDisposition::Rejected))),
        };
        let Some(bag) = inventory.backpack.as_ref() else {
            return Ok(None);
        };
        let entries = || {
            bag.entries
                .iter()
                .chain(inventory.nested.iter().map(|(_, entry)| entry))
        };
        let resolved = match source {
            Some(ItemKey::Instance(id)) => {
                entries().find(|entry| entry.item.item_instance_id == id)
            }
            _ => None,
        };
        if wanted.is_none()
            && resolved.is_none_or(|entry| entry.item.definition.production_key != KOOLDOWN_AID_KEY)
        {
            return Ok(None);
        }
        if pending.is_some() {
            return Ok(Some(outcome(UseDisposition::Exhausted)));
        }
        if !self.cube_actor_live(actor, command.game_session_id).await {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let reserved = {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let Some(gate) = states
                .item_uses
                .gate(&runtime, actor, command.game_session_id)
            else {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            if let Err(refusal) = gate.admit(command.command_id, self.owner_ms()) {
                return Ok(Some(outcome(refusal)));
            }
            gate.unspendable.clone()
        };
        let source_id = if let Some(wanted) = wanted {
            match inventory.first_unreserved(&wanted, &reserved) {
                Ok(id) => id,
                Err(_) => return Ok(Some(outcome(UseDisposition::NothingToUse))),
            }
        } else {
            match resolved {
                Some(entry) => entry.item.item_instance_id,
                None => return Ok(Some(outcome(UseDisposition::StaleState))),
            }
        };
        let Some(entry) = entries().find(|entry| entry.item.item_instance_id == source_id) else {
            return Ok(Some(outcome(UseDisposition::StaleState)));
        };
        if entry.item.definition.family != "Item"
            || entry.item.definition.production_key != KOOLDOWN_AID_KEY
            || native
                .item_policy(KOOLDOWN_AID_KEY, &entry.item.definition.revision_ref)
                .is_none()
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(source_id))
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let request = ItemUseRequest {
            kind: ItemUseKind::Concoction,
            command_id: command.command_id,
            content_revision: inventory.content_revision.clone(),
            ruleset_revision: inventory.ruleset_revision.clone(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            transaction_id: crate::operator_files::uuid_v7().map_err(|_| ())?,
            event_id: crate::operator_files::uuid_v7().map_err(|_| ())?,
            occurred_at_unix_ms: super::super::super::unix_ms()?,
            server_build_id: SERVER_BUILD_ID.into(),
            source_item_instance_id: source_id,
            source_definition: entry.item.definition.clone(),
            source_quantity_before: entry.item.quantity,
            flask: ItemUseFlaskPlan::None,
        };
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let Some(gate) = states
                .item_uses
                .gate(&runtime, actor, command.game_session_id)
            else {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            if let Err(refusal) = gate.prepare_concoction(request.clone(), binding, self.owner_ms())
            {
                return Ok(Some(outcome(refusal)));
            }
        }
        self.commit_kooldown_attempt(actor, command, request, binding)
            .await
            .map(|result| Some(outcome(result)))
    }

    async fn commit_kooldown_attempt(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        request: ItemUseRequest,
        binding: [u8; 32],
    ) -> Result<UseDisposition, ()> {
        let fence = command.item_fence.ok_or(())?;
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let retry = AdmissionRetryBudget::for_command(self.root);
        let written = retry_item_use_holder(&retry, async || {
            self.root
                .commit_kooldown_aid(self.character, self.holder, fence, request.clone(), binding)
                .await
        })
        .await;
        #[cfg(test)]
        if matches!(
            &written,
            Ok(crate::durability::item_use::ItemUseOutcome::Written { .. })
        ) && test_faults::take_lost_ack()
        {
            self.mark_cube_unknown(actor, command).await;
            return Err(());
        }
        let disposition = match written {
            Ok(_) => UseDisposition::Committed,
            Err(error) => {
                match self
                    .root
                    .read_kooldown_aid_receipt(self.character, fence, command_ref, binding)
                    .await
                {
                    Ok(true) => UseDisposition::Committed,
                    _ if item_use_commit_unknown(&error) => {
                        // The write may have committed even when its receipt is unavailable.
                        self.mark_cube_unknown(actor, command).await;
                        return Err(());
                    }
                    _ => match error {
                        ItemUseError::Cooldown => UseDisposition::Exhausted,
                        ItemUseError::StaleState => UseDisposition::StaleState,
                        _ => UseDisposition::Rejected,
                    },
                }
            }
        };
        self.finish_kooldown_aid(actor, command, binding, disposition)
            .await?;
        Ok(disposition)
    }

    /// No await between applying the runtime effect and releasing its retained PREPARE.
    /// Historical reads alone never reset later spells or release another command's source.
    async fn finish_kooldown_aid(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        binding: [u8; 32],
        disposition: UseDisposition,
    ) -> Result<(), ()> {
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let ours = states
                .item_uses
                .gate(&runtime, actor, command.game_session_id)
                .and_then(|gate| gate.cube_pending.as_ref())
                .is_some_and(|pending| {
                    pending.request.kind == ItemUseKind::Concoction
                        && pending.request.command_id == command.command_id
                        && pending.binding() == Some(binding)
                });
            if !ours {
                return Ok(());
            }
        }
        if disposition != UseDisposition::Committed {
            self.finish_cube(actor, command, disposition).await;
            return Ok(());
        }
        let fence = command.item_fence.ok_or(())?;
        // Re-prove the current item fence even when the receipt's source is consumed.
        // Fresh-admission reconciliation also requires admission-origin receipts;
        // it is not the item-only continuation contract.
        self.root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let applied = self.apply_kooldown_aid(
            &runtime,
            &mut states,
            actor,
            command.game_session_id,
            command.command_id,
        )?;
        drop(states);
        drop(runtime);
        if applied {
            self.kooldown_aid_notice(command.game_session_id);
        }
        Ok(())
    }

    pub(in crate::gameplay_transport) fn finish_resumed_kooldown_aid(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        states: &mut crate::gameplay_transport::actor_spell::ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let command = states
            .item_uses
            .0
            .iter()
            .find(|(present, held, _)| *present == actor && *held == session)
            .and_then(|(_, _, gate)| gate.cube_pending.as_ref())
            .filter(|pending| pending.recovered_concoction)
            .map(|pending| pending.request.command_id);
        command.is_some_and(|command| {
            self.apply_kooldown_aid(runtime, states, actor, session, command) == Ok(true)
        })
    }

    fn apply_kooldown_aid(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        states: &mut crate::gameplay_transport::actor_spell::ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        command: u64,
    ) -> Result<bool, ()> {
        if runtime.owner_fence().is_err()
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || states.is_dead(actor)
            || !states.spell_idle(runtime, actor, session)
            || pinned_index(runtime, self.active_generation).is_none()
        {
            return Err(());
        }
        let actor_index = states
            .actors
            .iter()
            .position(|(present, held, _)| *present == actor && *held == session)
            .ok_or(())?;
        // The source-qualified reset helper is intentionally independent of casting Focus:
        // it must not spend Harmony, heal, arm a stance or rearm any spell cooldown.
        if !states
            .item_uses
            .gate(runtime, actor, session)
            .is_some_and(|gate| gate.finish_cube(command, UseDisposition::Committed))
        {
            return Ok(false);
        }
        let (_, _, state) = states.actors.get_mut(actor_index).ok_or(())?;
        let before = state.cooldowns.clone();
        clear_kooldown_cooldowns(&mut state.cooldowns, self.owner_now());
        states.record_cooldown_changes(
            runtime,
            actor,
            session,
            &before,
            &states.actors[actor_index].2.cooldowns,
            None,
        );
        states.owner_wake.notify_one();
        Ok(true)
    }

    pub(in crate::gameplay_transport) fn kooldown_aid_notice(&self, session: GameSessionId) {
        self.states.tutorial_sessions.notice(
            session,
            oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(RESET_TEXT.into()),
        );
    }
}

#[cfg(test)]
pub(in crate::gameplay_transport) mod test_faults {
    use std::cell::Cell;
    thread_local! { static LOSE_ACK: Cell<bool> = const { Cell::new(false) }; }
    pub(crate) fn lose_next_ack() {
        LOSE_ACK.set(true);
    }
    pub(super) fn take_lost_ack() -> bool {
        LOSE_ACK.replace(false)
    }
}

/// Crystal 201189f8 player.cpp:10468-10487 expires individual and group spell
/// conditions without a book/Harmony filter. It does not remove other conditions.
/// Preserve the map entries and expire them at the current semantic time; movement
/// pacification shares this owner but is not a spell-group condition.
fn clear_kooldown_cooldowns(
    cooldowns: &mut crate::spell::Cooldowns,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) {
    for deadline in cooldowns.spells.values_mut() {
        *deadline = now;
    }
    for (key, deadline) in &mut cooldowns.groups {
        if key != crate::spell::STAIRHOP_COOLDOWN_KEY {
            *deadline = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::clear_kooldown_cooldowns;
    use crate::spell::{Cooldowns, STAIRHOP_COOLDOWN_KEY};
    use oteryn_simulation_determinism::SemanticTimeMicros;

    #[test]
    fn kooldown_expires_all_spell_entries_but_keeps_movement_pacification() {
        let now = SemanticTimeMicros::from_micros(10);
        let future = SemanticTimeMicros::from_micros(1_000);
        let mut cooldowns = Cooldowns {
            spells: [("known".into(), future), ("not_in_book".into(), future)].into(),
            groups: [
                ("attack".into(), future),
                ("healing".into(), future),
                (STAIRHOP_COOLDOWN_KEY.into(), future),
            ]
            .into(),
        };
        clear_kooldown_cooldowns(&mut cooldowns, now);
        assert_eq!(cooldowns.spells.len(), 2);
        assert!(cooldowns.spells.values().all(|deadline| *deadline == now));
        assert_eq!(cooldowns.groups.get("attack"), Some(&now));
        assert_eq!(cooldowns.groups.get("healing"), Some(&now));
        assert_eq!(cooldowns.groups.get(STAIRHOP_COOLDOWN_KEY), Some(&future));
        assert_eq!(cooldowns.groups.len(), 3);
        let once = cooldowns.clone();
        clear_kooldown_cooldowns(&mut cooldowns, now);
        assert_eq!(cooldowns, once);
    }
}

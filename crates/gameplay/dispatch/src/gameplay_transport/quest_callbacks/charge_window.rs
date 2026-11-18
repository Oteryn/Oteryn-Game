//! Crystal actions_holy_water.lua:105-135 shares a charge window across every caster.
use super::*;
use crate::durability::quest_world_callback::charge_window::{ChargeObservation, ChargeState};
use crate::map::facts::QuestCallbackProjection;
use oteryn_gameplay_quests::quest::world_callbacks::ChargeWindow;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

pub(super) fn matches_current(
    map: &BundleMap,
    at: TilePos,
    ordinal: u8,
    charge: &ChargeWindow,
) -> bool {
    map.overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .and_then(|base| {
            base.ids()
                .iter()
                .zip(base.depths())
                .filter(|(_, depth)| **depth == 0)
                .nth(usize::from(ordinal))
                .map(|(id, _)| *id)
        })
        .and_then(|id| map.facts.base_entry(at, ordinal, id))
        .is_some_and(|entry| charge.accepts(entry.appearance_id))
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn prepare_quest_charge(
        &self,
        permit: &SpellLanePermit,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
        at: TilePos,
        appearance: u16,
        cause: QuestCause,
        fence: Option<CurrentCharacterGameplayFence>,
        selected: Option<&([u8; 16], Vec<u8>)>,
    ) -> Option<UseDisposition> {
        let charge = callback.effects.charge_window.as_ref()?;
        let refused = Some(UseDisposition::Rejected);
        if matches!(charge, ChargeWindow::Spend { .. })
            && self
                .states
                .quest_callbacks
                .records
                .lock()
                .is_ok_and(|records| {
                    records.get(&session).is_some_and(|record| {
                        record.actor == actor && record.key == key && record.cause == cause
                    })
                })
        {
            // A known or uncertain prior receipt reconciles its original debit.
            return None;
        }
        let (
            Some(fence),
            Some((tool, intent)),
            QuestCause::Use(command),
            Ok(position),
            Some(source),
        ) = (
            fence,
            selected,
            cause,
            runtime.read_actor_position(actor),
            &callback.selector.native,
        )
        else {
            return refused;
        };
        if !charge.accepts(appearance)
            || !map.facts.quest_callback_capacity(key, revision, 8)
            || revision > u64::MAX - 3
        {
            return refused;
        }
        let context = QuestWorldContext {
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_revision: format!(
                "sha256:{}",
                hex(&runtime.content_pin().server_artifact_digest())
            ),
            placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
            reset_epoch: self.reset_epoch,
            actor_placement: actor.placement_identity(),
            movement_revision: position.revision(),
            party: Vec::new(),
        };
        let Some(context) = context.with_selected_tool(*tool, intent.clone()) else {
            return refused;
        };
        let Some(reset) = map
            .facts
            .quest_callback_replacement(key, source.appearance)
            .filter(|f| !f.pickupable)
        else {
            return refused;
        };
        let (tool, op, phase) = match charge {
            ChargeWindow::Charge { tool, phases, .. } => {
                let Some(phase) = phases.iter().find(|p| p.from == appearance) else {
                    return refused;
                };
                let Some(burning) = map
                    .facts
                    .quest_callback_replacement(key, phase.burning)
                    .filter(|f| !f.pickupable)
                else {
                    return refused;
                };
                let Some(ready) = map
                    .facts
                    .quest_callback_replacement(key, phase.ready)
                    .filter(|f| !f.pickupable)
                else {
                    return refused;
                };
                let op = if let Some([min, max]) = phase.capacity {
                    let digest = Sha256::new()
                        .chain_update(b"oteryn:world-quest-charge:v1")
                        .chain_update(command.game_session_id().as_bytes())
                        .chain_update(command.command_id().get().to_be_bytes())
                        .chain_update(key.to_be_bytes())
                        .chain_update(revision.to_be_bytes())
                        .finalize();
                    let mut occurrence = [0; 16];
                    occurrence.copy_from_slice(&digest[..16]);
                    let root = GameplayDecisionRoot::from_bytes(
                        runtime.content_pin().server_artifact_digest(),
                    );
                    let Ok(draw) = deterministic_decision_u64(
                        &root,
                        DecisionOccurrenceId::from_bytes(occurrence),
                        "quest.world-charge.capacity",
                        0,
                    ) else {
                        return refused;
                    };
                    let amount = crate::spell::uniform_draw(draw, i64::from(min), i64::from(max));
                    let Ok(amount) = u8::try_from(amount) else {
                        return refused;
                    };
                    ChargeObservation::Open {
                        amount,
                        burn_ms: phase.burn_ms,
                        ready_ms: phase.ready_ms,
                    }
                } else {
                    ChargeObservation::CheckTool
                };
                (tool.as_str(), op, Some((phase, burning, ready)))
            }
            ChargeWindow::Spend { .. } => {
                let Some(tool) = &callback.item_on_target else {
                    return refused;
                };
                (tool.item.as_str(), ChargeObservation::CheckWindow, None)
            }
        };
        let Ok(observed) = self
            .root
            .observe_quest_charge(
                self.character,
                self.holder,
                permit,
                fence,
                command,
                context,
                charge.group().to_owned(),
                tool.to_owned(),
                op,
            )
            .await
        else {
            return refused;
        };
        match (observed, phase) {
            (ChargeState::Allowed, None) => None,
            (ChargeState::Empty, None)
                if map
                    .facts
                    .apply_quest_callback(key, revision, reset, &[], &[], false, None) =>
            {
                Some(UseDisposition::Committed)
            }
            (ChargeState::Allowed, Some((phase, burning, ready))) => {
                let now = self.owner_now().get();
                let Some(next) = now.checked_add(u64::from(phase.burn_ms) * 1_000) else {
                    return refused;
                };
                let Some(reset_at) = next.checked_add(u64::from(phase.ready_ms) * 1_000) else {
                    return refused;
                };
                if !map.facts.apply_quest_callback(
                    key,
                    revision,
                    burning,
                    &[],
                    &[],
                    false,
                    Some(&QuestCallbackProjection::TimedDecayChain {
                        next: (next, ready),
                        reset: (reset_at, reset),
                    }),
                ) {
                    return refused;
                }
                states.owner_wake.notify_one();
                if let ChargeWindow::Charge {
                    cue,
                    announcement,
                    radius,
                    ..
                } = charge
                {
                    map.facts.emit_movement_effect(at, u16::from(*cue));
                    self.states.chat.publish_quest_charge_announcement(
                        runtime,
                        actor,
                        session,
                        at,
                        *radius,
                        announcement,
                    );
                }
                Some(UseDisposition::Committed)
            }
            _ => refused,
        }
    }
}

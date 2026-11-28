//! Gated legacy USE adapter: current direct-backpack custody selects the existing Rune caster.
//! Child modules compose native replay and current Item revision resolution;
//! the spell executor, payment and retained-batch cache remain with their owners.

#[path = "rune_use_bridge/replay.rs"]
mod replay;
#[path = "rune_use_bridge/revision.rs"]
mod revision;
pub(super) use replay::bind_batch;
#[cfg(test)]
#[path = "rune_use_bridge/source_positions_tests.rs"]
mod source_positions_tests;

use super::ComposedFreshAdmission;
use super::actor_spell::NativeCastDispatch;
use super::capabilities::SelectedCapabilities;
use super::connection::{EarnedNotice, FreshAdmissionAuthority, UseCommand, UseOutcome};
use super::item_view::{ItemKey, SessionItemView};
use super::world_spatial::SessionVisibility;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use crate::spell::SpellDefinition;
use oteryn_protocol_oteryn::actor_spell::{
    SpellCastDisposition, SpellCastIntent, SpellTarget, SpellTargetPosition,
};
use oteryn_protocol_oteryn::actor_spell_item_v2::ItemSpellCastIntent;
use oteryn_protocol_oteryn::world_object::{
    CAPABILITY_RUNE_USE_V1, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::num::NonZeroU32;

// Deterministic cancellation points on the real native writer. No alternate
// payment/installation implementation exists in the fixture.
#[cfg(test)]
pub(super) mod test_faults {
    use std::cell::Cell;

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub(in crate::gameplay_transport) enum Point {
        BeforeCommit,
        BeforeCommitThenBusyReplay,
        SessionReadBusy,
        AfterCommit,
        ReplayReadBusy,
        ReplayHeld,
    }
    thread_local! {
        static STOP: Cell<Option<Point>> = const { Cell::new(None) };
        static CONTENTION: Cell<Option<tokio::sync::oneshot::Sender<()>>> = const { Cell::new(None) };
    }
    pub(in crate::gameplay_transport) fn watch_contention() -> tokio::sync::oneshot::Receiver<()> {
        let (send, receive) = tokio::sync::oneshot::channel();
        CONTENTION.set(Some(send));
        receive
    }
    pub(in crate::gameplay_transport) fn observed_contention() {
        if let Some(send) = CONTENTION.take() {
            let _ = send.send(());
        }
    }
    pub(in crate::gameplay_transport) fn arm(point: Point) {
        STOP.set(Some(point));
    }
    pub(in crate::gameplay_transport) fn take(point: Point) -> bool {
        if STOP.get() == Some(point) {
            STOP.set(None);
            true
        } else {
            false
        }
    }

    pub(super) fn replay_busy() -> bool {
        STOP.get() == Some(Point::ReplayHeld) || take(Point::ReplayReadBusy)
    }

    pub(in crate::gameplay_transport) fn roll_back_before_commit() -> bool {
        if take(Point::BeforeCommitThenBusyReplay) {
            arm(Point::ReplayHeld);
            true
        } else {
            take(Point::BeforeCommit)
        }
    }
}

/// The exact original USE and resolved creature survive native pending/replay handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyRuneUse {
    pub(super) original: UseIntent,
    pub(super) creature: Option<EntityRef>,
    pub(super) fence: crate::durability::item_transfer::CurrentCharacterItemFence,
}

impl LegacyRuneUse {
    pub(super) fn binding(
        &self,
    ) -> Result<Vec<u8>, oteryn_protocol_oteryn::world_object::WorldObjectError> {
        // The original was decoded by the same bounded codec at ingress.
        encode_item_use_intent(&self.original)
    }
}

pub(super) fn routes(intent: &UseIntent, selected: &SelectedCapabilities) -> bool {
    selected.contains(CAPABILITY_RUNE_USE_V1)
        && matches!(intent.target, UseTarget::Item(_))
        && intent.use_with_map_item.is_none()
}

pub(super) async fn dispatch<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    command: UseCommand,
    intent: UseIntent,
    view: Option<&SessionItemView>,
    visibility: Option<&SessionVisibility>,
    selected: &SelectedCapabilities,
) -> Option<UseOutcome> {
    let key = match intent.target {
        UseTarget::Item(handle) => {
            view.and_then(|v| v.resolve(handle).filter(|key| v.is_backpack_entry(key)))
        }
        _ => None,
    };
    let visible = intent
        .use_with
        .is_none_or(|entity| visibility.is_some_and(|v| v.shows(entity)));
    authority
        .use_rune(
            actor,
            command,
            intent,
            key,
            visible,
            selected.contains(oteryn_protocol_oteryn::world_object::CAPABILITY_FIELD_RUNES_V1),
        )
        .await
}

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        committed: None,
        earned: EarnedNotice::NoneEarned,
        reward_granted: None,
        gate_message_id: None,
        potion_utterance: None,
        refusal_detail: None,
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum RuneReadReason {
    HolderBusy,
    PassDeadline,
    ReadUnavailable,
    TileRead,
    PositionMismatch,
    GenerationMismatch,
    CorpseCustody,
    PredecessorCustody,
    DuplicateItemPlan,
}

impl RuneReadReason {
    pub(super) fn durability(error: &crate::durability::DurabilityError) -> Self {
        use crate::durability::DurabilityError;
        match error {
            DurabilityError::RootUnavailable => Self::HolderBusy,
            DurabilityError::RootPassDeadlineExceeded
            | DurabilityError::CommitTransactionTimeout => Self::PassDeadline,
            _ => Self::ReadUnavailable,
        }
    }

    pub(super) fn corpse(
        error: &crate::durability::spell_item_transaction::SpellItemError,
    ) -> Self {
        use crate::durability::spell_item_transaction::SpellItemError;
        match error {
            SpellItemError::Rejected(
                "Ground custody belongs to an unreconciled scope generation",
            ) => Self::PredecessorCustody,
            SpellItemError::Rejected("unreconciled opaque or incompatible Ground placement") => {
                Self::PositionMismatch
            }
            SpellItemError::Rejected("duplicate or invalid item identity") => {
                Self::DuplicateItemPlan
            }
            SpellItemError::Rejected(_) => Self::CorpseCustody,
            SpellItemError::Durability(error) => Self::durability(error),
            _ => Self::ReadUnavailable,
        }
    }
}

#[derive(Default)]
pub(super) struct RuneReadEvidence {
    pub(super) reason: Option<RuneReadReason>,
    pub(super) generation: Option<u64>,
    pub(super) holder: Option<&'static std::panic::Location<'static>>,
    pub(super) corpse_item: Option<[u8; 16]>,
    pub(super) corpse_stage: Option<String>,
}

impl RuneReadEvidence {
    pub(super) fn log(&mut self, command_id: u64, stage: &str, reason: RuneReadReason) {
        self.reason = Some(reason);
        let holder = self
            .holder
            .map_or_else(|| "none".into(), |site| site.to_string());
        let generation = self
            .generation
            .map_or_else(|| "none".into(), |value| value.to_string());
        let corpse_item = self.corpse_item.map_or_else(
            || "none".into(),
            |item| {
                item.iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            },
        );
        let corpse_stage = self.corpse_stage.as_deref().unwrap_or("none");
        super::operator_event(&format!(
            "event=rune_use_read_refused stage={stage} command_id={command_id} reason={reason:?} holder={holder} corpse_item_id={corpse_item} corpse_stage={corpse_stage} generation={generation}"
        ));
    }
}

pub(super) fn rune_read_refusal(command_id: u64, stage: &str) -> NativeCastDispatch {
    RuneReadEvidence::default().log(command_id, stage, RuneReadReason::ReadUnavailable);
    NativeCastDispatch::RuneRefused(UseDisposition::Rejected)
}

pub(super) async fn retry_rune_owner(
    budget: &super::AdmissionRetryBudget<'_>,
    command_id: u64,
    stage: &str,
    attempt: &mut u32,
) -> bool {
    *attempt += 1;
    super::operator_event(&format!(
        "event=rune_use_owner_busy stage={stage} command_id={command_id} attempt={attempt}"
    ));
    budget
        .retries_unavailable(super::connection::AdmissionRefusal::Unavailable)
        .await
}

async fn read_rune_owner<T>(
    budget: &super::AdmissionRetryBudget<'_>,
    command_id: u64,
    stage: &str,
    mut read: impl AsyncFnMut() -> Result<T, crate::durability::DurabilityError>,
) -> Result<T, crate::durability::DurabilityError> {
    use crate::durability::DurabilityError;
    let mut attempt = 0;
    loop {
        let result = tokio::time::timeout_at(budget.deadline, read())
            .await
            .unwrap_or(Err(DurabilityError::RootPassDeadlineExceeded));
        if !matches!(
            result,
            Err(DurabilityError::RootUnavailable | DurabilityError::RootPassDeadlineExceeded)
        ) || !retry_rune_owner(budget, command_id, stage, &mut attempt).await
        {
            return result;
        }
    }
}

fn target(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    spell: &SpellDefinition,
    original: UseIntent,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    frame: super::wire_position::WirePositionFrame,
) -> Result<(SpellTarget, LegacyRuneUse), UseDisposition> {
    // docs/reference/spells/utility-runes-2026-10-29.md:10: a creature
    // target must not turn into the movable item underneath that creature.
    if original.use_with.is_some()
        && matches!(
            spell.carrier,
            crate::spell::Carrier::Rune { item: 3178, .. }
        )
    {
        return Err(UseDisposition::TargetIllegal);
    }
    // Preserve `original` for retained payment/replay binding. Only the
    // decoded positional target enters the current physical frame once.
    let position = match original.use_with_position {
        Some(position) => {
            let physical = frame
                .unproject(oteryn_protocol_oteryn::world_spatial::ActorPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                })
                .ok_or(UseDisposition::TargetIllegal)?;
            Some(SpellTargetPosition {
                x: physical.x,
                y: physical.y,
                floor: physical.floor,
            })
        }
        None => None,
    };
    let caster = runtime
        .read_actor_position(actor)
        .map_err(|_| UseDisposition::Rejected)?;
    if position.is_some_and(|p| p.floor != caster.position().floor) {
        return Err(UseDisposition::TargetIllegal);
    }
    let entities = runtime.visible_entities();
    let census = runtime
        .positioned_actor_census()
        .map_err(|_| UseDisposition::Rejected)?;
    let intense_healing = matches!(
        spell.carrier,
        crate::spell::Carrier::Rune { item: 3152, .. }
    );
    let entity = |a: ExactActorRef| {
        entities
            .creatures
            .iter()
            .find(|c| c.actor == a)
            .map(|c| EntityRef {
                identity: a.placement_identity(),
                generation: c.generation,
            })
            .or_else(|| {
                entities
                    .players
                    .iter()
                    .find(|p| p.actor == a)
                    .map(|p| EntityRef {
                        identity: a.placement_identity(),
                        generation: p.generation,
                    })
            })
    };
    let creature = if let Some(wanted) = original.use_with {
        let found = census.iter().find(|(a, p, _)| {
            entity(*a) == Some(wanted) && super::attack::sees(caster.position(), p.position())
        });
        Some(
            entity(found.ok_or(UseDisposition::TargetIllegal)?.0)
                .ok_or(UseDisposition::TargetIllegal)?,
        )
    } else if spell.needs_target
        && let Some(p) = position
    {
        // Native ordinary casts select the lowest actor id on a tile. Select it before
        // legality filtering and retain its exact generation; never fall through to a sibling.
        census
            .iter()
            .filter(|(a, at, _)| {
                let at = at.position();
                entity(*a).is_some()
                    && (at.x, at.y, at.floor) == (p.x, p.y, p.floor)
                    && super::attack::sees(caster.position(), at)
            })
            .min_by_key(|(a, _, _)| a.actor_local_id())
            .and_then(|(a, _, _)| entity(*a))
    } else {
        None
    };
    if intense_healing && creature.is_none() {
        return Err(UseDisposition::NoTarget);
    }
    let mapped = if spell.needs_target && creature.is_some() {
        SpellTarget::AttackTarget
    } else {
        let at = if let Some(creature) = creature {
            census
                .iter()
                .find(|(a, _, _)| entity(*a) == Some(creature))
                .map(|(_, at, _)| at.position())
                .ok_or(UseDisposition::TargetIllegal)?
        } else {
            caster.position()
        };
        let physical_position = position.unwrap_or(SpellTargetPosition {
            x: at.x,
            y: at.y,
            floor: at.floor,
        });
        if spell.needs_target && position.is_some() {
            return Err(UseDisposition::NoTarget);
        }
        SpellTarget::Position(physical_position)
    };
    Ok((
        mapped,
        LegacyRuneUse {
            original,
            creature,
            fence,
        },
    ))
}

pub(super) fn spell_disposition(disposition: SpellCastDisposition) -> UseDisposition {
    match disposition {
        SpellCastDisposition::Cast => UseDisposition::Committed,
        SpellCastDisposition::CoolingDown => UseDisposition::Exhausted,
        SpellCastDisposition::LevelTooLow
        | SpellCastDisposition::MagicLevelTooLow
        | SpellCastDisposition::NotEnoughMana
        | SpellCastDisposition::NotEnoughSoul
        | SpellCastDisposition::NotAvailable => UseDisposition::RequirementNotMet,
        SpellCastDisposition::TargetRequired => UseDisposition::NoTarget,
        SpellCastDisposition::TargetIllegal => UseDisposition::TargetIllegal,
        SpellCastDisposition::Rejected => UseDisposition::Rejected,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_legacy_rune(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        key: Option<ItemKey>,
        visible: bool,
        field_runes: bool,
    ) -> Option<UseOutcome> {
        let mut intent = intent;
        // Source targets retain their wire coordinates for replay and enter the
        // physical frame in target(). Other Bundles retain legacy receipt binding.
        if !self
            .effective_rooms()
            .gate_wire_position()
            .is_some_and(|room| room.pinned_activity_source_room().is_some())
            && let Some(position) = &mut intent.use_with_position
        {
            position.floor = self.runtime_floor(position.floor);
        }
        let store =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone());
        let mut retry = super::AdmissionRetryBudget::for_command(self.root);
        let Ok((current, _)) = read_rune_owner(&retry, command.command_id, "session", async || {
            #[cfg(test)]
            if test_faults::take(test_faults::Point::SessionReadBusy) {
                return Err(crate::durability::DurabilityError::RootUnavailable);
            }
            store.current_session_at(command.game_session_id).await
        })
        .await
        else {
            return self
                .finish_rune_retry(actor, command, &intent, &retry, "session", None)
                .await;
        };
        if current.session_state() != crate::foundation::GameSessionState::Active
            || super::item_fence_of(current) != command.item_fence
            || command.item_fence.is_none()
        {
            return Some(UseOutcome::rejected());
        }
        let mut recovery_attempts = 0;
        let replay = loop {
            match tokio::time::timeout_at(
                retry.deadline,
                self.replay_legacy_rune(actor, command, &intent),
            )
            .await
            {
                Ok(Ok(result)) => break result,
                Ok(Err(
                    crate::durability::DurabilityError::RootUnavailable
                    | crate::durability::DurabilityError::RootPassDeadlineExceeded
                    | crate::durability::DurabilityError::Unavailable,
                )) if retry_rune_owner(
                    &retry,
                    command.command_id,
                    "replay",
                    &mut recovery_attempts,
                )
                .await => {}
                _ => break NativeCastDispatch::Pending,
            }
        };
        match replay {
            NativeCastDispatch::Outcome(cast) => {
                return Some(outcome(spell_disposition(cast.disposition)));
            }
            NativeCastDispatch::RuneRefused(reason) => return Some(outcome(reason)),
            NativeCastDispatch::Pending => {
                return self
                    .finish_rune_retry(actor, command, &intent, &retry, "replay", None)
                    .await;
            }
            NativeCastDispatch::NotApplicable => (),
        }
        // A retained original is resolved before current handles, custody or target reads.
        let retained = {
            let states = self.states.spell_states.lock().await;
            states
                .pending_native
                .iter()
                .find(|p| p.is_for(actor, command.game_session_id))
                .map(|p| (p.command.command_id().get(), p.intent.clone()))
        };
        let (rune, legacy) = if let Some((id, (_, rune, _, legacy))) = retained {
            let (Some(rune), Some(legacy)) = (rune, legacy) else {
                return None;
            };
            if id != command.command_id || legacy.original != intent {
                return None;
            }
            (rune, legacy)
        } else {
            let Some(ItemKey::Instance(instance)) = key else {
                return Some(outcome(UseDisposition::StaleState));
            };
            let Some(fence) = command.item_fence else {
                return Some(UseOutcome::rejected());
            };
            let Some(native) = self.active_generation.and_then(|a| a.native_gameplay()) else {
                return Some(UseOutcome::rejected());
            };
            let backpack = read_rune_owner(&retry, command.command_id, "backpack", async || {
                self.root
                    .read_character_backpack(self.character, fence.character_id)
                    .await
                    .map_err(|error| match error {
                        crate::durability::item_transfer::ItemTransferError::Unavailable(error) => {
                            error
                        }
                        _ => crate::durability::DurabilityError::Unavailable,
                    })
            })
            .await;
            let backpack = match backpack {
                Ok(Some(backpack)) => backpack,
                Ok(None) => return Some(outcome(UseDisposition::StaleState)),
                Err(_) => {
                    let _ = rune_read_refusal(command.command_id, "backpack");
                    return Some(UseOutcome::rejected());
                }
            };
            let entry = backpack
                .entries
                .iter()
                .find(|e| e.item.item_instance_id == instance);
            let plain_use = intent.use_with.is_none()
                && intent.use_with_map_item.is_none()
                && intent.use_with_position.is_none();
            // The original instance can still be named by the connection when its coin stack
            // has already been consumed. Check the receipt before refusing the absent entry.
            if plain_use
                && entry.is_none()
                && let Some(outcome) = self
                    .use_coin(actor, command, ItemKey::Instance(instance), true)
                    .await
            {
                return outcome;
            }
            let Some(entry) = entry else {
                return Some(outcome(UseDisposition::StaleState));
            };
            let policy = native.item_policy(
                &entry.item.definition.production_key,
                &entry.item.definition.revision_ref,
            );
            let spell = policy
                .and_then(|p| p.record().production_binding.external_id.parse().ok())
                .and_then(|id| self.spells.rune(id));
            let Some(spell) = spell.filter(|s| {
                oteryn_gameplay_spell_rules::dispatch_plans::rune_use::direct_rune(s)
                    || matches!(
                        s.carrier,
                        crate::spell::Carrier::Rune {
                            item: 3165 | 3177 | 3178,
                            ..
                        }
                    )
            }) else {
                if plain_use {
                    let item = ItemKey::Instance(instance);
                    if crate::domain::currency::Coin::from_production_key(
                        &entry.item.definition.production_key,
                    )
                    .is_some()
                        && let Some(outcome) = self.use_coin(actor, command, item, false).await
                    {
                        return outcome;
                    }
                    return Some(self.use_backpack_item(actor, command, item).await);
                }
                return Some(outcome(UseDisposition::NothingToUse));
            };
            if !field_runes
                && !matches!(
                    spell.carrier,
                    crate::spell::Carrier::Rune {
                        item: 3152
                            | 3165
                            | 3174
                            | 3177
                            | 3178
                            | 3188
                            | 3189
                            | 3197
                            | 3200
                            | 3203
                            | 17512,
                        ..
                    }
                )
            {
                return Some(UseOutcome::rejected());
            }
            let Some(index) = self
                .spells
                .spells
                .iter()
                .position(|s| s.key == spell.key)
                .and_then(|i| u32::try_from(i).ok()?.checked_add(1))
                .and_then(NonZeroU32::new)
            else {
                let _ = rune_read_refusal(command.command_id, "spell_index");
                return Some(UseOutcome::rejected());
            };
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            let Some(state) = states.get(&runtime, actor, command.game_session_id) else {
                return Some(UseOutcome::rejected());
            };
            if super::item_ref_admission::pinned_index(&runtime, self.active_generation).is_none() {
                return Some(UseOutcome::rejected());
            }
            if let Err(gate) = crate::spell::cast::condition_gate(state, spell, self.owner_now()) {
                super::operator_event(&format!(
                    "event=rune_use_condition_gated gate={}",
                    gate.key()
                ));
                return Some(outcome(UseDisposition::RequirementNotMet));
            }
            if spell.aggressive
                && state
                    .cooldowns
                    .group_ready_at(oteryn_gameplay_spell_rules::cast::STAIRHOP_COOLDOWN_KEY)
                    .is_some_and(|until| self.owner_now() < until)
            {
                return Some(outcome(UseDisposition::Exhausted));
            }
            if !visible {
                return Some(outcome(UseDisposition::TargetIllegal));
            }
            let Some(frame) = self.wire_position_frame(&runtime) else {
                return Some(outcome(UseDisposition::Rejected));
            };
            let (mapped, legacy) = match target(&runtime, actor, spell, intent, fence, frame) {
                Ok(value) => value,
                Err(reason) => return Some(outcome(reason)),
            };
            (
                ItemSpellCastIntent {
                    intent: SpellCastIntent {
                        spell: index,
                        target: mapped,
                        aim_at_target: false,
                    },
                    rune_item_instance: instance,
                    expected_state_revision: 0,
                    target_item: None,
                },
                legacy,
            )
        };
        let corpse_cast = self
            .spells
            .indexed(rune.intent.spell)
            .is_some_and(super::actor_spell::native_combat_cast::corpse_companion_spell);
        if corpse_cast {
            retry.allow_corpse_rune_pass();
        }
        if !self.ensure_source_map_initialized().await {
            return if corpse_cast {
                self.finish_rune_retry(actor, command, &legacy.original, &retry, "source_map", None)
                    .await
            } else {
                None
            };
        }
        let access = self
            .refresh_and_apply_spell_access(actor, command.game_session_id)
            .await;
        let mut native_read = RuneReadEvidence::default();
        let mut result = self
            .cast_rune_item(
                actor,
                command,
                &rune,
                legacy.clone(),
                &access,
                (&retry, &mut native_read),
            )
            .await;
        // Complete the original command while the client is still connected.
        // Replaying first reclaims a parked writer using its exact retained
        // source and commit receipt before a current handle can be considered.
        while corpse_cast && matches!(result, NativeCastDispatch::Pending) {
            if !retry_rune_owner(
                &retry,
                command.command_id,
                "native_cast",
                &mut recovery_attempts,
            )
            .await
            {
                return self
                    .finish_rune_retry(
                        actor,
                        command,
                        &legacy.original,
                        &retry,
                        "native_cast",
                        Some(&mut native_read),
                    )
                    .await;
            }
            result = match tokio::time::timeout_at(
                retry.deadline,
                self.replay_legacy_rune(actor, command, &legacy.original),
            )
            .await
            {
                Ok(Ok(result)) => result,
                _ => NativeCastDispatch::Pending,
            };
            if matches!(result, NativeCastDispatch::NotApplicable) {
                let retained = self
                    .states
                    .spell_states
                    .lock()
                    .await
                    .pending_native
                    .iter()
                    .find(|marker| {
                        marker.is_for(actor, command.game_session_id)
                            && marker.command.command_id().get() == command.command_id
                    })
                    .and_then(|marker| marker.intent.1);
                // A failed read before preparation leaves no marker. Retry
                // the same ingress with its original handle and fence.
                let retry_rune = retained.unwrap_or(rune);
                result = self
                    .cast_rune_item(
                        actor,
                        command,
                        &retry_rune,
                        legacy.clone(),
                        &access,
                        (&retry, &mut native_read),
                    )
                    .await;
            }
        }
        match result {
            NativeCastDispatch::Pending => None,
            NativeCastDispatch::RuneRefused(disposition) => Some(outcome(disposition)),
            NativeCastDispatch::NotApplicable => Some(outcome(UseDisposition::NothingToUse)),
            NativeCastDispatch::Outcome(cast) => {
                if cast.disposition == SpellCastDisposition::Cast {
                    let _ = self.drain_kill_rewards(command.game_session_id).await;
                }
                let disposition = spell_disposition(cast.disposition);
                Some(outcome(disposition))
            }
        }
    }

    async fn finish_rune_retry(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        original: &UseIntent,
        budget: &super::AdmissionRetryBudget<'_>,
        stage: &str,
        evidence: Option<&mut RuneReadEvidence>,
    ) -> Option<UseOutcome> {
        let disposition = self
            .finish_legacy_rune_at_deadline(actor, command, original, budget.deadline)
            .await?;
        if disposition == UseDisposition::Rejected {
            let reason = if budget.check_deadline().is_err() {
                RuneReadReason::PassDeadline
            } else {
                RuneReadReason::ReadUnavailable
            };
            let mut fallback = RuneReadEvidence {
                generation: command
                    .item_fence
                    .map(|fence| fence.scope_ownership_generation.get()),
                holder: self.root.semantic_holder_location(),
                ..RuneReadEvidence::default()
            };
            let evidence = evidence.unwrap_or(&mut fallback);
            evidence.log(command.command_id, stage, evidence.reason.unwrap_or(reason));
        }
        Some(outcome(disposition))
    }
}

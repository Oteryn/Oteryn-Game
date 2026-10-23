//! Gated legacy USE adapter: current direct-backpack custody selects the existing Rune caster.
//! Child modules compose native replay and current Item revision resolution;
//! the spell executor, payment and retained-batch cache remain with their owners.

#[path = "rune_use_bridge/replay.rs"]
mod replay;
#[path = "rune_use_bridge/revision.rs"]
mod revision;
pub(super) use replay::bind_batch;

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
        AfterCommit,
    }
    thread_local! {
        static STOP: Cell<Option<Point>> = const { Cell::new(None) };
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
        .use_rune(actor, command, intent, key, visible)
        .await
}

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        committed: None,
        earned: EarnedNotice::NoneEarned,
        gate_message_id: None,
    }
}

fn target(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    spell: &SpellDefinition,
    original: UseIntent,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
) -> Result<(SpellTarget, LegacyRuneUse), UseDisposition> {
    let caster = runtime
        .read_actor_position(actor)
        .map_err(|_| UseDisposition::Rejected)?;
    if original
        .use_with_position
        .is_some_and(|p| p.floor != caster.position().floor)
    {
        return Err(UseDisposition::TargetIllegal);
    }
    let entities = runtime.visible_entities();
    let census = runtime
        .positioned_actor_census()
        .map_err(|_| UseDisposition::Rejected)?;
    let entity = |a: ExactActorRef| {
        entities
            .creatures
            .iter()
            .find(|c| c.actor == a)
            .map(|c| EntityRef {
                identity: a.placement_identity(),
                generation: c.generation,
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
        && let Some(p) = original.use_with_position
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
        let position = original.use_with_position.unwrap_or(SpellTargetPosition {
            x: at.x,
            y: at.y,
            floor: at.floor,
        });
        if spell.needs_target && original.use_with_position.is_some() {
            return Err(UseDisposition::NoTarget);
        }
        SpellTarget::Position(position)
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
    ) -> Option<UseOutcome> {
        let Ok((current, _)) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(command.game_session_id)
                .await
        else {
            return None;
        };
        if current.session_state() != crate::foundation::GameSessionState::Active
            || super::item_fence_of(current) != command.item_fence
            || command.item_fence.is_none()
        {
            return Some(UseOutcome::rejected());
        }
        match self.replay_legacy_rune(actor, command, &intent).await {
            NativeCastDispatch::Outcome(cast) => {
                return Some(outcome(spell_disposition(cast.disposition)));
            }
            NativeCastDispatch::RuneRefused(reason) => return Some(outcome(reason)),
            NativeCastDispatch::Pending => return None,
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
            let backpack = self
                .root
                .read_character_backpack(self.character, fence.character_id)
                .await
                .ok()??;
            let Some(entry) = backpack
                .entries
                .iter()
                .find(|e| e.item.item_instance_id == instance)
            else {
                return Some(outcome(UseDisposition::StaleState));
            };
            let policy = native.item_policy(
                &entry.item.definition.production_key,
                &entry.item.definition.revision_ref,
            );
            let spell = policy
                .and_then(|p| p.record().production_binding.external_id.parse().ok())
                .and_then(|id| self.spells.rune(id));
            let Some(spell) = spell
                .filter(|s| oteryn_gameplay_spell_rules::dispatch_plans::rune_use::direct_rune(s))
            else {
                if intent.use_with.is_none() && intent.use_with_position.is_none() {
                    return Some(
                        self.use_backpack_item(actor, command, ItemKey::Instance(instance))
                            .await,
                    );
                }
                return Some(outcome(UseDisposition::NothingToUse));
            };
            let index = self
                .spells
                .spells
                .iter()
                .position(|s| s.key == spell.key)
                .and_then(|i| u32::try_from(i).ok()?.checked_add(1))
                .and_then(NonZeroU32::new)?;
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
            if !visible {
                return Some(outcome(UseDisposition::TargetIllegal));
            }
            let (mapped, legacy) = match target(&runtime, actor, spell, intent, fence) {
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
        if !self.ensure_source_map_initialized().await {
            return None;
        }
        let access = self
            .refresh_and_apply_spell_access(actor, command.game_session_id)
            .await;
        match self
            .cast_rune_item(
                actor,
                command.game_session_id,
                command.command_id,
                &rune,
                legacy,
                &access,
            )
            .await
        {
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
}

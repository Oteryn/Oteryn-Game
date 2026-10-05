//! The actual Channel cast joins source facts, locked Item reads, player payment,
//! physical HP and the existing owner timer lane. No input here is wire authority.
//! The caller holds runtime -> player states -> LocalObject locks and the same
//! independently fenced SQL transaction throughout preparation and commit.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::{ChannelSpellStates, commit_owner_batch, stage_player_batch};
use crate::ability::AbilityOccurrence;
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{QualifiedNativeEntryRoom, SpellTileLookupError};
use crate::durability::character_build::{
    BuildChangeRequest, BuildOccurrence, CommittedBuildChange,
};
use crate::durability::spell_item_transaction::{
    SpellItemAuthority, read_spell_tile_in_transaction,
};
use crate::foundation::{
    ChannelRuntimeV1, CharacterId, CommandRef, ExactActorRef, GameSessionId, MovementFacing,
    MovementLocalPosition, MovementPositionSnapshot, RuntimeScopeRefV1,
};
use crate::spell::cast::{PlayerSpellState, prepare_native_owner_cast_with_caster};
use crate::spell::chain::TilePosition;
use crate::spell::combat_batch::{
    CombatBatchReceipt, Error, LiveActorBinding, OwnerCombatBatch, lower_native,
};
use crate::spell::delayed_execution::{
    CastBinding, SavedNativeOwnerEffect, ScheduleRequest, SpellTimerOccurrence, TimerPayload,
    ValidatedTimerInstall,
};
use crate::spell::formula::FormulaInputs;
use crate::spell::magnitude_owner::PreparedMagnitudeOwner;
use crate::spell::mana_training::PreparedPlayerTraining;
use crate::spell::native::{Facts, Plan};
use crate::spell::native_combat::{
    Direction, ElementalStance, NativeCombatFacts, NativeCombatPlan, NativeTargetFact, TargetKind,
};
use crate::spell::owned_cast_facts::OwnedCastFacts;
use crate::spell::world_items_execution::SpellGroundTarget;
use crate::spell::{Execution, OperationalCastFacts, SpellBook, SpellDefinition};
use crate::world_runtime::LocalObjectRuntime;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};
use oteryn_protocol_oteryn::actor_spell_item_v2::ItemSpellCastIntent;
use oteryn_protocol_oteryn::actor_spell_v2::{
    ParameterSpellCastIntent, ParameterSpellCastResult, decode_parameter_spell_cast_result,
    encode_parameter_spell_cast_intent, encode_parameter_spell_cast_result,
};
use oteryn_simulation_determinism::SemanticTimeMicros;
#[path = "native_companion_item_cast.rs"]
mod native_companion_item_cast;

pub(super) fn direct_companion_spell(spell: &SpellDefinition) -> bool {
    native_companion_item_cast::direct_applicable(spell)
}
#[path = "native_world_item_cast.rs"]
mod native_world_item_cast;
#[path = "ordinary_field_items.rs"]
mod ordinary_field_items;
#[path = "rune_item_cast.rs"]
mod rune_item_cast;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use std::collections::{BTreeMap, BTreeSet};

/// Candidate work budget: exceeding it refuses the whole cast, never clips its area/chain.
const MAX_WORLD_TILES: usize = 4096;
const MAX_SIGHT_DISTANCE: u32 = 64;
/// Retained unknown durable outcomes remain obligations; saturation refuses a new cast.
const MAX_PENDING_NATIVE: usize = 256;

#[path = "ordinary_combat.rs"]
pub(in crate::gameplay_transport) mod ordinary_combat;

/// One original normalized attempt on the existing caster owner. It is retained
/// before the first potentially committing SQL await, never reconstructed on retry.
#[derive(Debug)]
pub(crate) struct PendingNativeCast {
    actor: ExactActorRef,
    session: GameSessionId,
    intent: SpellCastIntent,
    rune: Option<ItemSpellCastIntent>,
    parameter: Option<ParameterSpellCastIntent>,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    prepared: PreparedNativeCombatCast,
    owned: OwnedCastFacts,
    request: crate::durability::spell_items_abi::SpellItemTransactionRequest,
}

#[derive(Debug)]
pub(in crate::gameplay_transport) enum NativeCastDispatch {
    NotApplicable,
    Outcome(super::SpellCastOutcome),
    /// The FND owner must keep the original command outstanding, without terminal rejection.
    Pending,
}

impl ChannelSpellStates {
    pub(crate) fn has_pending_native(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.pending_native
            .iter()
            .any(|p| p.actor == actor && p.session == session)
    }
}

#[derive(Debug)]
pub(crate) struct PreparedNativeCombatCast {
    corpse_companion: Option<native_companion_item_cast::PreparedCorpseCompanionState>,
    direct_companion: Option<native_companion_item_cast::PreparedDirectCompanionState>,
    field_policy_revision: Option<String>,
    carried_target: Option<native_world_item_cast::CarriedTargetBinding>,
    item_operations: Vec<crate::durability::spell_items_abi::SpellItemOperation>,
    item_creations: Vec<ordinary_combat::OrdinaryItemCreation>,
    party: Option<super::super::party_spell_owner::SourcePartyWorld>,
    before: PlayerSpellState,
    paid: PlayerSpellState,
    batch: OwnerCombatBatch,
    tile_facts: BTreeMap<TilePosition, TileFlags>,
    roster: Vec<(
        ExactActorRef,
        MovementPositionSnapshot,
        Option<GameSessionId>,
    )>,
    training: PreparedPlayerTraining,
    timers: Option<ValidatedTimerInstall>,
    magnitude: Option<PreparedMagnitudeOwner>,
    facts_binding: crate::spell::owned_cast_facts::CastFactsBinding,
    creatures: Vec<crate::foundation::CompanionSnapshot>,
    players: Vec<(ExactActorRef, GameSessionId, PlayerSpellState)>,
    equipment: crate::durability::character_equipment::EquipmentSnapshot,
    presentation: Option<super::super::spell_presentations::PreparedPresentation>,
    installation: Option<PreparedOwnerInstallation>,
}
#[derive(Debug)]
struct PreparedOwnerInstallation {
    physical: crate::foundation::StagedSpellBatch,
    player: super::PlayerBatchPreflight,
    paid: PlayerSpellState,
}
impl PreparedNativeCombatCast {
    #[allow(
        clippy::expect_used,
        reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
    )]
    fn current_connection_binding(
        &self,
        runtime: &ChannelRuntimeV1,
        owned: &OwnedCastFacts,
        reconnect: Option<
            &crate::durability::admission_journal::spell_reconnect::SpellReconnectTransition,
        >,
    ) -> Result<crate::spell::owned_cast_facts::CastFactsBinding, Error> {
        let mut expected = self.facts_binding.clone();
        if owned.binding().connection_generation != expected.connection_generation {
            let actual = runtime.binding();
            let character = crate::domain::CharacterId::from_bytes(self.facts_binding.character)
                .map_err(|_| Error::InvalidBatch)?;
            if !reconnect.is_some_and(|proof| {
                proof.matches(
                    crate::foundation::ConnectionGeneration::new(expected.connection_generation)
                        .expect("qualified original connection"),
                    crate::foundation::ConnectionGeneration::new(
                        owned.binding().connection_generation,
                    )
                    .expect("qualified current connection"),
                    self.batch.command,
                    character,
                    self.batch.current_lease_generation,
                    RuntimeScopeRefV1::channel(actual.world_id(), actual.channel_id()),
                    actual.scope_generation().get(),
                )
            }) {
                return Err(Error::SnapshotChanged);
            }
            expected.connection_generation = owned.binding().connection_generation;
        }
        Ok(expected)
    }
    /// Invoked by the source writer only on its NEW-write branch, before its first INSERT.
    /// A genuine historical receipt instead completes the already accepted immutable decision.
    #[allow(
        clippy::too_many_arguments,
        reason = "the owner turn binds every independently resolved fact explicitly"
    )]
    fn validate_new_write(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        owned: &OwnedCastFacts,
        original_owned: &OwnedCastFacts,
        spell: &SpellDefinition,
        now: u64,
        reconnect: Option<
            &crate::durability::admission_journal::spell_reconnect::SpellReconnectTransition,
        >,
    ) -> Result<(), Error> {
        owned
            .check_current_access(spell, now)
            .map_err(|_| Error::InvalidBatch)?;
        if owned.binding() != &self.current_connection_binding(runtime, owned, reconnect)?
            || owned.wheel_revision() != original_owned.wheel_revision()
            || states.get(
                runtime,
                self.batch.caster,
                self.batch.command.game_session_id(),
            ) != Some(&self.before)
        {
            return Err(Error::SnapshotChanged);
        }
        runtime.validate_positioned_actor_census(&self.roster)?;
        for creature in &self.creatures {
            runtime.validate_companion_snapshot(creature)?;
        }
        for (actor, session, before) in &self.players {
            if states.get(runtime, *actor, *session) != Some(before) {
                return Err(Error::SnapshotChanged);
            }
        }
        if let Some(magnitude) = &self.magnitude {
            magnitude.validate_current_with_lookup(
                runtime,
                &self.before,
                owned.magnitude().ok_or(Error::InvalidBatch)?,
                &|actor| {
                    self.players
                        .iter()
                        .find(|(a, _, _)| *a == actor)
                        .map(|(_, _, p)| p)
                },
            )?;
        }
        Ok(())
    }
    /// All bounded owner allocations happen before the source cost transaction can COMMIT.
    /// A retained unknown outcome refreshes these proofs before any retry of durable work.
    fn stage_installation(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
    ) -> Result<(), Error> {
        if let Some(companion) = self.direct_companion.as_mut() {
            companion
                .reservation
                .reserve_physical(runtime)
                .map_err(|_| Error::SnapshotChanged)?;
        }
        if let Some(companion) = self.corpse_companion.as_mut() {
            companion
                .spawn
                .reserve_physical(runtime)
                .map_err(|_| Error::SnapshotChanged)?;
        }
        if let Some(timers) = self.timers.as_mut() {
            timers
                .refresh_current(
                    states.spell_timers.as_ref().ok_or(Error::InvalidBatch)?,
                    runtime.owner_fence()?,
                    &self.batch,
                )
                .map_err(|_| Error::SnapshotChanged)?;
        }
        let mut physical = runtime.stage_spell_batch(&self.batch)?;
        let player = stage_player_batch(runtime, states, &self.batch, Some(self.paid.clone()))
            .map_err(|_| Error::PlayerVitalsOwnerRequired)?;
        runtime.reserve_spell_batch(&mut physical)?;
        self.installation = Some(PreparedOwnerInstallation {
            physical,
            player,
            paid: self.paid.clone(),
        });
        if let Some(presentation) = &self.presentation {
            states
                .presentations
                .as_mut()
                .ok_or(Error::InvalidBatch)?
                .hold_prepared_before_sql(runtime, presentation, &self.batch)
                .map_err(|_| Error::SnapshotChanged)?;
        }
        Ok(())
    }
    pub(crate) fn training_request(&self) -> Option<&BuildChangeRequest> {
        self.training.request()
    }
    pub(crate) fn batch(&self) -> &OwnerCombatBatch {
        &self.batch
    }
    /// Call in the same uninterrupted owner turn/SQL transaction as preparation.
    /// Unknown durable training outcomes retain this original preparation; do not reroll.
    #[allow(
        clippy::expect_used,
        reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
    )]
    #[allow(
        clippy::too_many_arguments,
        reason = "the owner turn binds every independently resolved fact explicitly"
    )]
    pub(crate) fn commit(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        owned: &OwnedCastFacts,
        receipt: Option<&CommittedBuildChange>,
        companion_receipt: Option<
            &crate::durability::spell_item_transaction::CommittedCompanionAcquisition,
        >,
        direct_receipt: Option<
            &crate::durability::spell_item_transaction::CommittedDirectCompanionAcquisition,
        >,
        reconnect: Option<
            &crate::durability::admission_journal::spell_reconnect::SpellReconnectTransition,
        >,
    ) -> Result<CombatBatchReceipt, Error> {
        let current = states
            .get(
                runtime,
                self.batch.caster,
                self.batch.command.game_session_id(),
            )
            .ok_or(Error::SnapshotChanged)?;
        let mut expected_binding = self.current_connection_binding(runtime, owned, reconnect)?;
        if let Some(receipt) = receipt {
            if receipt.original_character_revision.get() != expected_binding.character_revision
                || receipt.after != *owned.durable_build()
            {
                return Err(Error::SnapshotChanged);
            }
            expected_binding.character_revision = receipt.committed_character_revision.get();
        }
        expected_binding.equipment_revision = self.equipment.revision;
        let current_equipment = owned.equipment();
        if current != &self.before
            || owned.binding() != &expected_binding
            || current_equipment.character != self.equipment.character
            || current_equipment.content_digest != self.equipment.content_digest
            || current_equipment.revision != self.equipment.revision
            || current_equipment.combat_mode != self.equipment.combat_mode
            || current_equipment.items != self.equipment.items
        {
            return Err(Error::SnapshotChanged);
        }
        for creature in &self.creatures {
            if !self
                .batch
                .effects
                .iter()
                .any(|e| e.target == creature.actor)
            {
                continue;
            }
            if runtime.validate_companion_snapshot(creature).is_err() {
                return Err(Error::SnapshotChanged);
            }
        }
        for (actor, session, before) in &self.players {
            if *actor != self.batch.caster && !self.batch.effects.iter().any(|e| e.target == *actor)
            {
                continue;
            }
            if states.get(runtime, *actor, *session) != Some(before) {
                return Err(Error::SnapshotChanged);
            }
        }
        // Check presence before any physical mutation. Installation itself is infallible.
        if self.timers.is_some() && states.spell_timers.is_none() {
            return Err(Error::InvalidBatch);
        }
        if let Some(proof) = &self.timers {
            proof
                .validate_current(
                    states.spell_timers.as_ref().ok_or(Error::InvalidBatch)?,
                    runtime.owner_fence()?,
                    &self.batch,
                )
                .map_err(|_| Error::SnapshotChanged)?;
        }
        if let Some(proof) = &self.presentation {
            states
                .presentations
                .as_ref()
                .ok_or(Error::InvalidBatch)?
                .validate_prepared(runtime, proof, &self.batch)
                .map_err(|_| Error::SnapshotChanged)?;
        }
        let installation = self.installation.as_mut().ok_or(Error::InvalidBatch)?;
        runtime.validate_staged_spell_batch(&installation.physical)?;
        installation
            .player
            .validate_current(runtime, states)
            .map_err(|_| Error::SnapshotChanged)?;
        if let Some(companion) = self.direct_companion.as_ref() {
            companion
                .reservation
                .validate_current(runtime, &companion.restrictions)
                .map_err(|_| Error::SnapshotChanged)?;
            if !direct_receipt
                .is_some_and(|receipt| receipt.matches_reservation(&companion.reservation))
            {
                return Err(Error::InvalidBatch);
            }
        }
        if let Some(companion) = self.corpse_companion.as_ref() {
            companion
                .spawn
                .validate_current(runtime, &companion.restrictions)
                .map_err(|_| Error::SnapshotChanged)?;
            if !companion_receipt.is_some_and(|receipt| receipt.matches_spawn(&companion.spawn)) {
                return Err(Error::InvalidBatch);
            }
        }
        self.training
            .prepare_install(
                current,
                &mut installation.paid,
                self.batch.anchor.as_ref().ok_or(Error::InvalidAnchor)?,
                receipt,
            )
            .map_err(|_| Error::InvalidAnchor)?;
        let mut installation = self
            .installation
            .take()
            .expect("validated preallocated owner installation");
        installation
            .player
            .rebind_training(runtime, states, installation.paid)
            .map_err(|_| Error::InvalidAnchor)?;
        let result = commit_owner_batch(
            runtime,
            states,
            installation.physical,
            Some(installation.player),
        )?;
        if result.applied {
            if let Some(companion) = self.direct_companion.take() {
                companion
                    .reservation
                    .install_prevalidated(
                        runtime,
                        direct_receipt.expect("prevalidated actual direct COMMIT"),
                    )
                    .expect("same uninterrupted owner turn preserves prevalidated direct slot");
            }
            if let Some(companion) = self.corpse_companion.take() {
                crate::spell::companion_lifecycle::install_reserved_companion(
                    runtime,
                    companion.spawn,
                    &companion.restrictions,
                    companion_receipt.expect("prevalidated actual compound COMMIT"),
                )
                .expect("same uninterrupted owner turn retains prevalidated companion slot");
            }
            if let Some(proof) = self.presentation.take() {
                states
                    .presentations
                    .as_mut()
                    .expect("prevalidated real presentation owner")
                    .install_preflighted(proof, &result);
            }
            if let Some(proof) = self.timers.take() {
                // The same exclusive states borrow retains this exact lane across preflight.
                states
                    .spell_timers
                    .as_mut()
                    .expect("prevalidated actual timer owner")
                    .install_preflighted(proof);
            }
        }
        Ok(result)
    }
}

fn reject<T>() -> Result<T, SpellCastDisposition> {
    Err(SpellCastDisposition::Rejected)
}

fn is_source_self(spell: &SpellDefinition) -> bool {
    !ordinary_combat::has_world_geometry(spell)
        && matches!(spell.carrier, crate::spell::Carrier::Instant { .. })
        && !spell.needs_target
        && !spell.target_or_direction
        && spell.chain.is_none()
        && matches!(
            &spell.execution,
            Execution::Effects(_) | Execution::AbilityVariants(_) | Execution::ActorFocus { .. }
        )
        || matches!(&spell.execution, Execution::NativeProfile(p)
            if matches!(p.spell()["execution"]["native_behavior"]["key"].as_str(),Some("monk_focus"|"companion_haste")))
}

struct PaidOwnerSelf {
    next: PlayerSpellState,
    anchor: crate::foundation::runtime_actor_spell_types::SpellAnchor,
    ability_variant: Option<crate::spell::executable_catalog::DefinitionRef>,
    party_heal: Option<(ExactActorRef, i64)>,
    companion_updates: Vec<(
        crate::foundation::CompanionSnapshot,
        crate::foundation::CompanionState,
    )>,
    companions: Vec<crate::foundation::CompanionSnapshot>,
}
#[allow(clippy::too_many_arguments)]
fn prepare_paid_self(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    owned: &OwnedCastFacts,
    content: &NativeGameplayState,
    before: &PlayerSpellState,
    spell: &SpellDefinition,
    book: &SpellBook,
    operational: &OperationalCastFacts,
    caster: &crate::spell::CasterState,
    party: Option<&super::super::party_spell_owner::SourcePartyWorld>,
    actor_label: &str,
    command: CommandRef,
    occurrence: AbilityOccurrence,
    now: SemanticTimeMicros,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<PaidOwnerSelf, SpellCastDisposition> {
    let focus_parameters = match &spell.execution {
        Execution::ActorFocus { profile } => Some(profile.clone()),
        Execution::NativeProfile(p)
            if p.spell()["execution"]["native_behavior"]["key"] == "monk_focus" =>
        {
            Some(p.spell()["execution"]["native_behavior"]["parameters"].clone())
        }
        _ => None,
    };
    if let Some(parameters) = focus_parameters {
        let world = party.ok_or(SpellCastDisposition::Rejected)?;
        let paid = crate::spell::cast::prepare_focus_owner_cast_with_caster(
            before,
            spell,
            book,
            operational,
            caster,
            crate::spell::cast::CastContext {
                caster: actor_label,
                occurrence,
                owner_scope: "channel:source-native-cast",
                now,
                draw,
            },
        )?;
        let party_heal = if let Some(healing) = &paid.plan.healing {
            let selected = crate::spell::native_actor_states::focus_healing_target(
                &parameters,
                world.caster_facts(),
                world.member_facts(),
            )
            .map_err(|_| SpellCastDisposition::Rejected)?;
            let (actor, _) = world
                .actor_for_id(selected)
                .ok_or(SpellCastDisposition::Rejected)?;
            let amount = healing
                .finish_draw(draw(healing.bounds.minimum, healing.bounds.maximum))
                .map_err(|_| SpellCastDisposition::Rejected)?;
            Some((actor, amount))
        } else {
            None
        };
        return Ok(PaidOwnerSelf {
            next: paid.next,
            anchor: paid.anchor,
            ability_variant: None,
            party_heal,
            companion_updates: Vec::new(),
            companions: Vec::new(),
        });
    }
    if matches!(&spell.execution,Execution::NativeProfile(p) if p.spell()["execution"]["native_behavior"]["key"]=="companion_haste")
    {
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
            content.source_digest(),
        );
        let decision = oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(nonce(
            b"oteryn:native-cast-occurrence:v1",
            owned.binding().actor,
            command.game_session_id(),
            command.command_id().get(),
            &[],
        ));
        // These positive SPEED/ATTRIBUTES profiles do not consume shield capacity or
        // PvE damage re-entry flags; those nominal fields confer no such qualification.
        let facts = crate::ability::condition::ApplicationFacts {
            now: now.get(),
            base_speed: u16::try_from(before.owned_base_speed())
                .map_err(|_| SpellCastDisposition::Rejected)?,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: decision,
        };
        let paid = crate::spell::companion_execution::prepare_haste_owner_cast_with_caster(
            runtime,
            room.movement_cells(),
            before,
            spell,
            operational,
            caster,
            owned.binding().actor,
            command.game_session_id(),
            owned.equipment_speed_bonus(content).ok(),
            &facts,
            draw,
        )?;
        return Ok(PaidOwnerSelf {
            next: paid.next,
            anchor: paid.anchor,
            ability_variant: None,
            party_heal: None,
            companion_updates: paid.companion_updates,
            companions: paid.companions,
        });
    }
    let paid = crate::spell::cast::prepare_source_self_cast_with_caster(
        before,
        spell,
        book,
        operational,
        caster,
        crate::spell::cast::CastContext {
            caster: actor_label,
            owner_scope: "channel-owner",
            occurrence,
            now,
            draw,
        },
    )?;
    Ok(PaidOwnerSelf {
        next: paid.next,
        anchor: paid.anchor,
        ability_variant: paid.ability_variant,
        party_heal: None,
        companion_updates: Vec::new(),
        companions: Vec::new(),
    })
}

/// Ordinary self effects use the same durable source cost, original-attempt ledger,
/// training receipt and whole-player successor as native combat. The pure self helper
/// resolves the actual ConditionStore/health/Harmony on a clone, never the live actor.
#[allow(
    clippy::too_many_arguments,
    reason = "the owner turn binds every independently resolved fact explicitly"
)]
async fn prepare_source_self_from_owners(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    book: &SpellBook,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    command: CommandRef,
    occurrence: AbilityOccurrence,
    training_occurrence: BuildOccurrence,
    now: SemanticTimeMicros,
    rune: Option<&ItemSpellCastIntent>,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
    if !is_source_self(spell)
        || intent.target != SpellTarget::None
        || intent.aim_at_target
        || book.indexed(intent.spell) != Some(spell)
        || content.spell_book().indexed(intent.spell) != Some(spell)
        || !matches!(spell.carrier, crate::spell::Carrier::Instant { .. })
    {
        return reject();
    }
    let b = owned.binding();
    let actual = runtime.binding();
    if b.session != command.game_session_id()
        || authority.command() != command
        || authority.character_id_bytes() != b.character
        || authority.compatible_content_digest() != content.source_digest()
        || authority.runtime_scope()
            != RuntimeScopeRefV1::channel(actual.world_id(), actual.channel_id())
        || authority.scope_generation() != actual.scope_generation().get()
        || b.content_digest != runtime.content_pin().server_artifact_digest()
    {
        return reject();
    }
    let before = states
        .get(runtime, b.actor, b.session)
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    let party = if matches!(
        before.source_party_vocation(),
        crate::spell::Vocation::Monk | crate::spell::Vocation::ExaltedMonk
    ) {
        Some(
            super::super::party_spell_owner::read_source_party_world_in_transaction(
                tx, root, authority, runtime, states, b.actor, b.session,
            )
            .await
            .map_err(|_| SpellCastDisposition::Rejected)?,
        )
    } else {
        None
    };
    let caster = owned
        .caster(
            &before,
            spell,
            content,
            before.owned_harmony_multiplier(spell)?,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let current = runtime
        .read_actor_position(b.actor)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let position = tile(current.position());
    let flags = read_tile(tx, authority, room, runtime, objects, position).await?;
    if !flags.present {
        return reject();
    }
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE source_self_prepare stage=tile_qualified");
    let operational = OperationalCastFacts {
        caster_position: position,
        target_position: None,
        target: None,
        line_of_sight_clear: None,
        direction_available: current.facing().is_some(),
        wheel_unlocked: None,
        in_protection_zone: flags.protection,
        target_tile_solid: None,
        target_tile_creature: None,
    };
    let actor_label = format!("actor:{}", super::hex(&b.actor.placement_identity()));
    let profile = spell
        .authored
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?;
    let mut requests = Vec::new();
    let mut append = |binding: &str| {
        requests.push(super::super::spell_presentations::LocatedCueRequest {
            binding: binding.into(),
            target: super::super::spell_presentations::CueTarget::Actor(b.actor),
        })
    };
    if let Some(p) = &profile.header.presentation {
        for binding in p.cast_cue.iter().chain(p.impact_cue.iter()) {
            append(binding);
        }
    }
    for effect in &profile.dependencies.effects {
        if let Some(p) = &effect.presentation {
            for binding in p
                .impact_asset_binding
                .iter()
                .chain(p.caster_effect_asset_binding.iter())
            {
                append(binding);
            }
            // A projectile to self has no qualified projectile target path.
            if p.projectile_asset_binding.is_some() {
                return reject();
            }
        }
    }
    if let Execution::NativeProfile(p) = &spell.execution {
        if p.spell()["execution"]["native_behavior"]["key"] == "companion_haste"
            && p.spell()["execution"]["native_behavior"]["parameters"]["caster_effect"]
                == "magic_green"
        {
            append("appearance:effect/magic_green");
        }
        if let Some(effect) = p.spell()["execution"]["native_behavior"]["parameters"]
            ["presentation"]["effect_asset_binding"]
            .as_str()
        {
            append(effect);
        }
    }
    let roster = runtime
        .positioned_actor_census()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let attacker = CharacterId::decode(&b.character).map_err(|_| SpellCastDisposition::Rejected)?;
    let formula = content
        .training_formula()
        .ok_or(SpellCastDisposition::Rejected)?;
    let make_batch = |paid: &PaidOwnerSelf| -> Result<OwnerCombatBatch, SpellCastDisposition> {
        let mut effects=paid.companion_updates.iter().enumerate().map(|(ordinal,(expected,next))|
            Ok(crate::foundation::runtime_actor_spell_types::OwnerCombatEffect {target:expected.actor,
                sub_ordinal:u16::try_from(ordinal).map_err(|_|SpellCastDisposition::Rejected)?,
                change:crate::foundation::runtime_actor_spell_types::OwnerCombatChange::CompanionConditions(Box::new(
                    crate::foundation::runtime_actor_spell_types::CompanionConditionUpdate {expected:expected.clone(),next:next.clone()}))
            })).collect::<Result<Vec<_>,SpellCastDisposition>>()?;
        if let Some((actor, magnitude)) = paid.party_heal {
            effects.push(
                crate::foundation::runtime_actor_spell_types::OwnerCombatEffect {
                    target: actor,
                    sub_ordinal: u16::try_from(effects.len())
                        .map_err(|_| SpellCastDisposition::Rejected)?,
                    change: crate::foundation::runtime_actor_spell_types::OwnerCombatChange::Heal {
                        target_atom: crate::spell::combat_execution::actor_atom(actor),
                        magnitude,
                    },
                },
            );
        }
        Ok(OwnerCombatBatch { caster:b.actor,attacker,current_lease_generation:b.lease_generation,command,
            occurrence:occurrence.clone().into(),anchor:Some(paid.anchor.clone()),now_ms:now.get()/1_000,
            effects,deferred:None,binding:serde_json::to_vec(&json!({"intent":source_cast_intent(intent,rune),
                "source_definition":format!("{profile:?}"),"capture_format":"owner-debug-r21",
                "equipment_facts":format!("{:?}",owned.equipment()),"character_revision":b.character_revision,
                "paid_successor":format!("{:?}",paid.next),"selected_variant":format!("{:?}",paid.ability_variant),
                "source_party":party.as_ref().map(|p|format!("{:?}",p.source()))})).map_err(|_|SpellCastDisposition::Rejected)? })
    };
    let preview = prepare_paid_self(
        runtime,
        room,
        owned,
        content,
        &before,
        spell,
        book,
        &operational,
        &caster,
        party.as_ref(),
        &actor_label,
        command,
        occurrence.clone(),
        now,
        &mut |minimum, _| minimum,
    )?;
    // Source reentry protection suppresses outgoing ally healing; self healing
    // remains eligible. This selection preview consumes no live random stream.
    if preview
        .party_heal
        .is_some_and(|(target, _)| target != b.actor)
        && runtime
            .current_player_reentry_protection(b.actor, b.session, now.get())
            .map_err(|_| SpellCastDisposition::Rejected)?
    {
        return reject();
    }
    let mut preview_batch = make_batch(&preview)?;
    let mut preview_requests = requests.clone();
    if let Some((actor, _)) = preview.party_heal {
        // Canary harmonyHeal impact magic-blue is separate from the Focus cast effect.
        preview_requests.push(super::super::spell_presentations::LocatedCueRequest {
            binding: "appearance:effect/magic_blue".into(),
            target: super::super::spell_presentations::CueTarget::Actor(actor),
        });
    }
    states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?
        .prepare_source_definition(
            runtime,
            content,
            intent.spell,
            spell,
            &mut preview_batch,
            preview_requests,
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    runtime
        .stage_spell_batch(&preview_batch)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    stage_player_batch(runtime, states, &preview_batch, Some(preview.next.clone()))
        .map_err(|_| SpellCastDisposition::Rejected)?;
    before
        .prepare_paid_training(
            &preview.next,
            &preview.anchor,
            formula,
            training_occurrence,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE source_self_prepare stage=preview_training_qualified");
    let paid = prepare_paid_self(
        runtime,
        room,
        owned,
        content,
        &before,
        spell,
        book,
        &operational,
        &caster,
        party.as_ref(),
        &actor_label,
        command,
        occurrence.clone(),
        now,
        draw,
    )?;
    let mut selected_cues = BTreeSet::new();
    if let Some(p) = &profile.header.presentation {
        selected_cues.extend(
            p.cast_cue
                .iter()
                .chain(p.impact_cue.iter())
                .map(String::as_str),
        );
    }
    let ability = match &paid.ability_variant {
        Some(reference) => Some(
            profile
                .dependencies
                .abilities
                .iter()
                .find(|a| {
                    reference.family == "Ability"
                        && a.identity.key == reference.key
                        && a.identity.revision == reference.revision
                })
                .ok_or(SpellCastDisposition::Rejected)?,
        ),
        None => profile.ability.as_ref(),
    };
    if matches!(spell.execution, Execution::AbilityVariants(_)) && paid.ability_variant.is_none() {
        return reject();
    }
    if let Some(ability) = ability {
        for reference in &ability.effects {
            let effect = profile
                .dependencies
                .effects
                .iter()
                .find(|e| {
                    reference.family == "Effect"
                        && e.identity.key == reference.key
                        && e.identity.revision == reference.revision
                })
                .ok_or(SpellCastDisposition::Rejected)?;
            if let Some(p) = &effect.presentation {
                selected_cues.extend(
                    p.impact_asset_binding
                        .iter()
                        .chain(p.caster_effect_asset_binding.iter())
                        .map(String::as_str),
                );
            }
        }
    }
    if let Execution::NativeProfile(p) = &spell.execution {
        if p.spell()["execution"]["native_behavior"]["key"] == "companion_haste"
            && p.spell()["execution"]["native_behavior"]["parameters"]["caster_effect"]
                == "magic_green"
        {
            selected_cues.insert("appearance:effect/magic_green");
        }
        if let Some(effect) = p.spell()["execution"]["native_behavior"]["parameters"]
            ["presentation"]["effect_asset_binding"]
            .as_str()
        {
            selected_cues.insert(effect);
        }
    }
    // Keep the originally selected Ability reference. Preflight above covers every possible
    // source cue before RNG, but the committed queue contains only the resolved branch.
    requests.retain(|request| selected_cues.contains(request.binding.as_str()));
    if let Some((actor, _)) = paid.party_heal {
        requests.push(super::super::spell_presentations::LocatedCueRequest {
            binding: "appearance:effect/magic_blue".into(),
            target: super::super::spell_presentations::CueTarget::Actor(actor),
        });
    }
    let mut batch = make_batch(&paid)?;
    let presentation = states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?
        .prepare_source_definition(runtime, content, intent.spell, spell, &mut batch, requests)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let training = before
        .prepare_paid_training(
            &paid.next,
            &paid.anchor,
            formula,
            training_occurrence,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let player_predecessors = party
        .as_ref()
        .map_or_else(Vec::new, |p| p.player_predecessors());
    Ok(PreparedNativeCombatCast {
        corpse_companion: None,
        direct_companion: None,
        field_policy_revision: None,
        carried_target: None,
        item_operations: Vec::new(),
        item_creations: Vec::new(),
        party,
        before,
        paid: paid.next,
        batch,
        tile_facts: BTreeMap::from([(position, flags)]),
        roster,
        training,
        timers: None,
        magnitude: None,
        facts_binding: b.clone(),
        creatures: paid.companions,
        players: player_predecessors,
        equipment: owned.equipment().clone(),
        presentation: Some(presentation),
        installation: None,
    })
}
fn cell(p: TilePosition) -> MovementLocalPosition {
    MovementLocalPosition {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
fn tile(p: MovementLocalPosition) -> TilePosition {
    TilePosition {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
fn offset(p: TilePosition, dx: i32, dy: i32) -> Result<TilePosition, SpellCastDisposition> {
    Ok(TilePosition {
        x: p.x.checked_add(dx).ok_or(SpellCastDisposition::Rejected)?,
        y: p.y.checked_add(dy).ok_or(SpellCastDisposition::Rejected)?,
        floor: p.floor,
    })
}
fn direction(facing: MovementFacing) -> Direction {
    match facing {
        MovementFacing::North => Direction::North,
        MovementFacing::East => Direction::East,
        MovementFacing::South => Direction::South,
        MovementFacing::West => Direction::West,
    }
}
fn delta(d: Direction) -> (i32, i32) {
    match d {
        Direction::North => (0, -1),
        Direction::East => (1, 0),
        Direction::South => (0, 1),
        Direction::West => (-1, 0),
        Direction::NorthWest => (-1, -1),
        Direction::NorthEast => (1, -1),
        Direction::SouthEast => (1, 1),
        Direction::SouthWest => (-1, 1),
    }
}
fn source_intent(intent: &SpellCastIntent) -> Value {
    json!({"index":intent.spell.get(),
    "target":match intent.target {SpellTarget::None=>json!(["none"]),
        SpellTarget::AttackTarget=>json!(["attack"]),SpellTarget::Position(p)=>json!(["position",p.x,p.y,p.floor])},
    "aim":intent.aim_at_target})
}

fn source_cast_intent(intent: &SpellCastIntent, rune: Option<&ItemSpellCastIntent>) -> Value {
    let mut source = source_intent(intent);
    if let Some(rune) = rune {
        source["rune"] = json!({"instance":rune.rune_item_instance,
            "expected_state_revision":rune.expected_state_revision});
        if let Some(target) = rune.target_item {
            source["target_item"] = json!({"instance":target.item_instance,"expected_state_revision":target.expected_state_revision});
        }
    }
    source
}

fn source_parameter_intent(
    intent: &SpellCastIntent,
    parameter: &ParameterSpellCastIntent,
) -> Result<Value, SpellCastDisposition> {
    if parameter.intent != *intent || intent.target != SpellTarget::None || intent.aim_at_target {
        return reject();
    }
    let mut value = source_intent(intent);
    value["parameter_v2_bytes"] = json!(
        encode_parameter_spell_cast_intent(parameter)
            .map_err(|_| SpellCastDisposition::Rejected)?
    );
    Ok(value)
}
fn source_original_intent(
    intent: &SpellCastIntent,
    rune: Option<&ItemSpellCastIntent>,
    parameter: Option<&ParameterSpellCastIntent>,
) -> Result<Value, SpellCastDisposition> {
    match parameter {
        Some(p) if rune.is_none() => source_parameter_intent(intent, p),
        Some(_) => reject(),
        None => Ok(source_cast_intent(intent, rune)),
    }
}
/// Exact retained original input resolves before clock/current-target/cost reads or RNG.
/// A retry whose spell/target/aim changed cannot reuse another command's receipt.
pub(crate) fn replay(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    caster: ExactActorRef,
    character: CharacterId,
    lease: u64,
    command: CommandRef,
    intent: &SpellCastIntent,
) -> Result<Option<CombatBatchReceipt>, Error> {
    replay_with_rune(
        runtime, states, caster, character, lease, command, intent, None, None, &mut None,
    )
}
#[allow(
    clippy::too_many_arguments,
    reason = "the owner turn binds every independently resolved fact explicitly"
)]
fn replay_with_rune(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    caster: ExactActorRef,
    character: CharacterId,
    lease: u64,
    command: CommandRef,
    intent: &SpellCastIntent,
    rune: Option<&ItemSpellCastIntent>,
    parameter: Option<&ParameterSpellCastIntent>,
    parameter_output: &mut Option<ParameterSpellCastResult>,
) -> Result<Option<CombatBatchReceipt>, Error> {
    let Some(batch) = runtime.retained_spell_batch(caster, character, lease, command)? else {
        return Ok(None);
    };
    let saved: Value = serde_json::from_slice(&batch.binding).map_err(|_| Error::InvalidBatch)?;
    if saved["intent"]
        != source_original_intent(intent, rune, parameter).map_err(|_| Error::CommandConflict)?
    {
        return Err(Error::CommandConflict);
    }
    let private_result = if parameter.is_some() {
        let bytes: Vec<u8> = serde_json::from_value(saved["parameter_result"].clone())
            .map_err(|_| Error::InvalidBatch)?;
        Some(decode_parameter_spell_cast_result(&bytes).map_err(|_| Error::InvalidBatch)?)
    } else {
        None
    };
    let staged = runtime.stage_spell_batch(&batch)?;
    if staged.will_apply() {
        return Err(Error::InvalidBatch);
    }
    let receipt = commit_owner_batch(runtime, states, staged, None)?;
    *parameter_output = private_result;
    Ok(Some(receipt))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TileFlags {
    projectile: bool,
    solid: bool,
    floor_change: bool,
    protection: bool,
    present: bool,
}
/// Only this constructor reads the flags: source tile metadata plus locked actual Item rows.
async fn read_tile(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    room: &QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    objects: &LocalObjectRuntime,
    position: TilePosition,
) -> Result<TileFlags, SpellCastDisposition> {
    let cells = room.movement_cells();
    let logical = crate::content::LogicalCell {
        x: position.x,
        y: position.y,
        z: i32::from(position.floor),
    };
    match cells.spell_tiles().lookup(cells.scope(), logical) {
        Err(SpellTileLookupError::Absent) => {
            return Ok(TileFlags {
                projectile: false,
                solid: false,
                floor_change: false,
                protection: false,
                present: false,
            });
        }
        Err(SpellTileLookupError::ScopeMismatch) => return reject(),
        // Mutable source placements require same-transaction initialization receipts;
        // the source index alone intentionally leaves these cells unknown.
        Err(SpellTileLookupError::Unknown) | Ok(_) => {}
    }
    let static_tile = crate::spell::world_execution::qualified_combat_tile_in_transaction(
        tx,
        authority,
        room,
        runtime,
        objects,
        cell(position),
    )
    .await
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let target = SpellGroundTarget::for_native_tile_read(room, runtime, cell(position))
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let actual = read_spell_tile_in_transaction(tx, authority, &target)
        .await
        .map_err(|_| SpellCastDisposition::Rejected)?;
    Ok(TileFlags {
        projectile: static_tile.block_projectile()
            || actual.items.iter().any(|i| i.blocks_projectile),
        solid: static_tile.block_solid() || actual.items.iter().any(|i| i.blocks_movement),
        floor_change: static_tile.floor_change(),
        protection: static_tile.protection_zone(),
        present: static_tile.ground_present(),
    })
}

/// Canary 99902524 map.cpp 845–949: endpoint-exclusive u16 Wu accumulator.
/// The third tuple member is the source last-diagonal-step obstruction exemption.
fn sight_steps(
    mut a: TilePosition,
    mut b: TilePosition,
) -> Result<Vec<(TilePosition, bool)>, SpellCastDisposition> {
    if a.floor != b.floor {
        return reject();
    }
    let dx = a.x.abs_diff(b.x);
    let dy = a.y.abs_diff(b.y);
    if dx.max(dy) > MAX_SIGHT_DISTANCE {
        return reject();
    }
    let mut out = Vec::new();
    if dx.max(dy) <= 1 {
        return Ok(out);
    }
    if dx == 0 || dy == 0 {
        let (sx, sy) = ((b.x - a.x).signum(), (b.y - a.y).signum());
        for _ in 1..dx.max(dy) {
            a = offset(a, sx, sy)?;
            out.push((a, false));
        }
        return Ok(out);
    }
    let y_major = dy > dx;
    if (y_major && a.y > b.y) || (!y_major && a.x > b.x) {
        std::mem::swap(&mut a, &mut b);
    }
    let minor_sign = if y_major {
        (b.x - a.x).signum()
    } else {
        (b.y - a.y).signum()
    };
    let adj =
        (((if y_major { dx } else { dy }) as u64) << 16) / (if y_major { dy } else { dx }) as u64;
    let adj = adj as u16;
    let mut acc = if minor_sign < 0 {
        0u16.wrapping_sub(adj)
    } else {
        0
    };
    for _ in 1..dx.max(dy) {
        let before = acc;
        acc = acc.wrapping_add(adj);
        let minor = if acc <= before { minor_sign } else { 0 };
        let near = a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1;
        a = if y_major {
            offset(a, minor, 1)?
        } else {
            offset(a, 1, minor)?
        };
        out.push((a, near));
    }
    Ok(out)
}

/// Geometry is read from the already closed native recipe, independent of tile permission.
fn area_candidates(
    parameters: &Value,
    origin: TilePosition,
    d: Direction,
) -> Result<BTreeSet<TilePosition>, SpellCastDisposition> {
    let mut out = BTreeSet::new();
    let Some(area) = parameters.get("area") else {
        return Ok(out);
    };
    let directional = area["directional"]
        .as_bool()
        .ok_or(SpellCastDisposition::Rejected)?;
    let diagonal = area["diagonal"].is_array();
    let (matrix, turn) = if !directional {
        (&area["orthogonal"], 0)
    } else {
        match d {
            Direction::NorthWest if diagonal => (&area["diagonal"], 0),
            Direction::NorthEast if diagonal => (&area["diagonal"], 1),
            Direction::SouthEast if diagonal => (&area["diagonal"], 2),
            Direction::SouthWest if diagonal => (&area["diagonal"], 3),
            Direction::North => (&area["orthogonal"], 0),
            Direction::South => (&area["orthogonal"], 2),
            Direction::West | Direction::NorthWest | Direction::SouthWest => {
                (&area["orthogonal"], 3)
            }
            _ => (&area["orthogonal"], 1),
        }
    };
    let rows = matrix.as_array().ok_or(SpellCastDisposition::Rejected)?;
    let mut center = None;
    let mut active = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        for (x, v) in row
            .as_array()
            .ok_or(SpellCastDisposition::Rejected)?
            .iter()
            .enumerate()
        {
            let n = v.as_u64().ok_or(SpellCastDisposition::Rejected)?;
            if n >= 2 && center.replace((x as i32, y as i32)).is_some() {
                return reject();
            }
            if n == 1 || n == 3 {
                active.push((x as i32, y as i32));
            }
        }
    }
    let (cx, cy) = center.ok_or(SpellCastDisposition::Rejected)?;
    for (x, y) in active {
        let (dx, dy) = (x - cx, y - cy);
        let (dx, dy) = match turn {
            0 => (dx, dy),
            1 => (-dy, dx),
            2 => (-dx, -dy),
            _ => (dy, -dx),
        };
        out.insert(offset(origin, dx, dy)?);
    }
    // Read the full side footprint even at stage0. It is not granted to the plan unless unlocked.
    if parameters.get("beam_mastery").is_some() {
        let central = out.clone();
        for p in central {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                out.insert(offset(p, dx, dy)?);
            }
        }
    }
    Ok(out)
}

/// All facts originate in the actual owner, static qualified content and the same SQL Item
/// transaction. Missing Premium/learning/Wheel/elemental/magnitude is refused before RNG.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn prepare_from_owners(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    book: &SpellBook,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    command: CommandRef,
    occurrence: AbilityOccurrence,
    training_occurrence: BuildOccurrence,
    now: SemanticTimeMicros,
    rune: Option<&ItemSpellCastIntent>,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
    let Execution::NativeProfile(profile) = &spell.execution else {
        return reject();
    };
    let recipe = &profile.spell()["execution"]["native_behavior"];
    let key = recipe["key"]
        .as_str()
        .ok_or(SpellCastDisposition::Rejected)?;
    if !matches!(
        key,
        "wheel_combat"
            | "avatar_state"
            | "monster_ai_override"
            | "mass_spirit_mend"
            | "mana_shield_capacity"
    ) || intent.aim_at_target
        || intent.target != SpellTarget::None
        || book.indexed(intent.spell) != Some(spell)
        || content.spell_book().indexed(intent.spell) != Some(spell)
        || !matches!(spell.carrier, crate::spell::Carrier::Instant { .. })
    {
        return reject();
    }
    let parameters = &recipe["parameters"];
    let b = owned.binding();
    let actor = b.actor;
    let session = command.game_session_id();
    let actual = runtime.binding();
    if b.session != session
        || authority.command() != command
        || authority.character_id_bytes() != b.character
        || authority.compatible_content_digest() != content.source_digest()
        || authority.runtime_scope()
            != RuntimeScopeRefV1::channel(actual.world_id(), actual.channel_id())
        || authority.scope_generation() != actual.scope_generation().get()
        || b.content_digest != runtime.content_pin().server_artifact_digest()
    {
        return reject();
    }
    let before_snapshot = states
        .get(runtime, actor, session)
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    let before = &before_snapshot;
    let party = if matches!(
        before.source_party_vocation(),
        crate::spell::Vocation::Monk | crate::spell::Vocation::ExaltedMonk
    ) {
        Some(
            super::super::party_spell_owner::read_source_party_world_in_transaction(
                tx, root, authority, runtime, states, b.actor, b.session,
            )
            .await
            .map_err(|_| SpellCastDisposition::Rejected)?,
        )
    } else {
        None
    };
    let caster = owned
        .caster(
            before,
            spell,
            content,
            before.owned_harmony_multiplier(spell)?,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let training_formula = content
        .training_formula()
        .ok_or(SpellCastDisposition::Rejected)?;
    let stage = match parameters.get("wheel") {
        Some(w) => owned
            .wheel_stage(
                w["perk"].as_str().ok_or(SpellCastDisposition::Rejected)?,
                now.get(),
            )
            .map_err(|_| SpellCastDisposition::Rejected)?,
        None => 0,
    };
    let elemental = if parameters.get("elemental_stance").is_some() {
        owned
            .elemental_stance(now.get())
            .ok_or(SpellCastDisposition::Rejected)?
    } else {
        ElementalStance::None
    }; // unconsumed for these recipes
    let roster = runtime
        .positioned_actor_census()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let creatures = roster
        .iter()
        .filter(|(_, _, s)| s.is_none())
        .map(|(actor, _, _)| {
            runtime
                .companion_snapshot(*actor)
                .map_err(|_| SpellCastDisposition::Rejected)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let player_predecessors: Vec<(ExactActorRef, GameSessionId, PlayerSpellState)> = states
        .actors
        .iter()
        .filter(|(a, s, _)| states.get(runtime, *a, *s).is_some())
        .cloned()
        .collect();
    let current = runtime
        .read_actor_position(actor)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let position = tile(current.position());
    let needs_direction = spell
        .authored
        .as_ref()
        .is_some_and(|p| p.header.targeting.needs_direction)
        || parameters["area"]["directional"] == true;
    let facing = current.facing();
    if needs_direction && facing.is_none() {
        return reject();
    }
    let facing_direction = facing.map(direction).unwrap_or(Direction::North); // unused by symmetric/self recipes
    let origin = match intent.target {
        SpellTarget::Position(p) => TilePosition {
            x: p.x,
            y: p.y,
            floor: p.floor,
        },
        SpellTarget::None if needs_direction => {
            let (dx, dy) = delta(facing_direction);
            offset(position, dx, dy)?
        }
        SpellTarget::None => position,
        SpellTarget::AttackTarget => return reject(),
    };
    if origin.floor != position.floor {
        return reject();
    }
    let sight_origin = if needs_direction && origin != position {
        let (dx, dy) = delta(facing_direction);
        offset(origin, -dx, -dy)?
    } else {
        origin
    };
    let mut candidates = area_candidates(parameters, origin, facing_direction)?;
    candidates.insert(position);
    candidates.insert(origin);
    let mut bindings = Vec::new();
    let mut attackable = BTreeSet::new();
    let mut targets = Vec::new();
    for (target, snapshot, target_session) in &roster {
        let p = tile(snapshot.position());
        if p.floor != position.floor
            || p.x.abs_diff(position.x) > MAX_SIGHT_DISTANCE
            || p.y.abs_diff(position.y) > MAX_SIGHT_DISTANCE
        {
            continue;
        }
        let id = u64::from(target.actor_local_id());
        let (name, kind, reward_boss, preferred_distance, health, maximum_health, atom) =
            if let Some(s) = target_session {
                let state = states
                    .get(runtime, *target, *s)
                    .ok_or(SpellCastDisposition::Rejected)?;
                let v = state.vitals();
                (
                    String::new(),
                    TargetKind::Player,
                    false,
                    0,
                    v.health,
                    v.max_health,
                    crate::spell::combat_execution::actor_atom(*target),
                )
            } else {
                let creature = runtime
                    .companion_snapshot(*target)
                    .map_err(|_| SpellCastDisposition::Rejected)?;
                let policy = &creature.state.policy;
                if policy.flags.attackable {
                    attackable.insert(id);
                }
                let kind = match &creature.state.master {
                    None => TargetKind::MasterlessMonster,
                    Some(_) => TargetKind::PlayerSummon,
                };
                (
                    policy.display_name.clone(),
                    kind,
                    policy.reward_boss.ok_or(SpellCastDisposition::Rejected)?,
                    policy
                        .preferred_distance
                        .ok_or(SpellCastDisposition::Rejected)?,
                    u32::try_from(creature.health).map_err(|_| SpellCastDisposition::Rejected)?,
                    u32::try_from(creature.maximum_health)
                        .map_err(|_| SpellCastDisposition::Rejected)?,
                    runtime
                        .creature_spell_target_atom(*target)
                        .map_err(|_| SpellCastDisposition::Rejected)?,
                )
            };
        bindings.push(LiveActorBinding {
            source_id: id,
            actor: *target,
            target_atom: atom,
        });
        targets.push(NativeTargetFact {
            id,
            position: p,
            kind,
            name,
            reward_boss,
            preferred_distance,
            health,
            maximum_health,
            legal: false,
        });
        candidates.insert(p);
    }
    let caster_id = u64::from(actor.actor_local_id());
    if !bindings.iter().any(|b| b.actor == actor) {
        return reject();
    }
    let mut paths = BTreeMap::new();
    for target in &candidates {
        paths.insert((sight_origin, *target), sight_steps(sight_origin, *target)?);
    }
    if parameters.get("chain").is_some() {
        for a in targets
            .iter()
            .map(|t| t.position)
            .chain(std::iter::once(position))
        {
            for b in targets.iter().map(|t| t.position) {
                if a.x.abs_diff(b.x) <= 7 && a.y.abs_diff(b.y) <= 7 {
                    paths.insert((a, b), sight_steps(a, b)?);
                }
            }
        }
    }
    for steps in paths.values() {
        for (p, _) in steps {
            candidates.insert(*p);
        }
    }
    if candidates.len() > MAX_WORLD_TILES {
        return reject();
    }
    let mut tiles = BTreeMap::new();
    for p in candidates {
        tiles.insert(
            p,
            read_tile(tx, authority, room, runtime, objects, p).await?,
        );
    }
    let caster_tile = tiles.get(&position).ok_or(SpellCastDisposition::Rejected)?;
    if !caster_tile.present {
        return reject();
    }
    let mut clear_sight = BTreeSet::new();
    for (pair, steps) in paths {
        if steps
            .iter()
            .all(|(p, exempt)| *exempt || tiles.get(p).is_some_and(|t| !t.projectile))
        {
            clear_sight.insert(pair);
        }
    }
    for target in &mut targets {
        let t = tiles
            .get(&target.position)
            .ok_or(SpellCastDisposition::Rejected)?;
        // Actual PvE permission only. PvP requires its own independently owned policy.
        target.legal = t.present
            && !t.floor_change
            && !t.solid
            && target.health > 0
            && (!spell.aggressive || (!t.protection && !caster_tile.protection))
            && (!spell.aggressive
                || (target.kind == TargetKind::MasterlessMonster
                    && attackable.contains(&target.id)));
    }
    let target_position = match intent.target {
        SpellTarget::Position(_) => Some(origin),
        _ => None,
    };
    let operational = OperationalCastFacts {
        caster_position: position,
        target_position,
        target: None,
        line_of_sight_clear: target_position
            .map(|p| p == position || clear_sight.contains(&(position, p))),
        direction_available: facing.is_some(),
        wheel_unlocked: parameters.get("wheel").map(|_| stage > 0),
        in_protection_zone: caster_tile.protection,
        target_tile_solid: tiles.get(&origin).map(|t| t.solid),
        target_tile_creature: Some(
            targets
                .iter()
                .any(|t| t.position == origin && t.id != caster_id),
        ),
    };
    let facts = NativeCombatFacts {
        now_ms: now.get() / 1000,
        stage,
        inputs: FormulaInputs {
            level: caster.level,
            magic_level: caster.magic_level,
            base_power: spell.base_power,
            attack_skill: caster.attack_skill,
            attack_value: caster.attack_value,
            attack_factor: caster.attack_factor,
            shielding_skill: caster.shielding_skill,
            shield_defense: caster.shield_defense,
        },
        maximum_mana: caster.max_mana,
        caster: caster_id,
        caster_position: position,
        area_origin: origin,
        area_sight_origin: sight_origin,
        direction: facing_direction,
        explicit_target: None,
        attacked_target: None,
        targets,
        clear_sight,
        floor_change_tiles: tiles
            .iter()
            .filter_map(|(p, t)| t.floor_change.then_some(*p))
            .collect(),
        elemental_stance: elemental,
        incoming_hit: None,
    };
    let players: Vec<_> = player_predecessors
        .iter()
        .map(|(a, _, p)| (*a, p))
        .collect();
    // A lower-bound preview is never committed and consumes no live random stream. It validates
    // common costs, source shape, all magnitude prerequisites and training before the first draw.
    let mut preview = prepare_native_owner_cast_with_caster(
        before,
        spell,
        &operational,
        Facts::Combat(&facts),
        now,
        &mut |minimum, _| minimum,
        &caster,
    )?;
    let Plan::Combat(preview_plan) = &preview.plan else {
        return reject();
    };
    let preview_plan = preview_plan.clone();
    if let NativeCombatPlan::Combat(p) = preview_plan.as_ref()
        && (p.use_weapon_charges || p.weapon_missile || p.hits.iter().any(|h| h.delay_ms > 0))
    {
        return reject();
    }
    let needs_magnitude = matches!(
        preview_plan.as_ref(),
        NativeCombatPlan::Combat(_) | NativeCombatPlan::MassSpiritMend { .. }
    );
    let mut preview_magnitude = if needs_magnitude {
        Some(
            PreparedMagnitudeOwner::qualify(
                runtime,
                before,
                actor,
                session,
                owned.magnitude(),
                &preview_plan,
                &bindings,
                &players,
                facts.now_ms,
            )
            .map_err(|_| SpellCastDisposition::Rejected)?,
        )
    } else {
        None
    };
    let binding = serde_json::to_vec(
        &json!({"intent":source_cast_intent(intent,rune),"spell":profile.spell(),
        "equipment_revision":b.equipment_revision,"character_revision":b.character_revision,
        "source_stage":stage,"capture_format":"owner-debug-r21",
        // Immutable receipts only; these captures grant no owner authority.
        "native_facts":format!("{facts:?}"),
        "magnitude_facts":format!("{:?}",owned.magnitude()),
        "equipment_facts":format!("{:?}",owned.equipment())}),
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let attacker = CharacterId::decode(&b.character).map_err(|_| SpellCastDisposition::Rejected)?;
    let preview_batch = OwnerCombatBatch {
        caster: actor,
        attacker,
        current_lease_generation: b.lease_generation,
        command,
        occurrence: occurrence.clone().into(),
        binding: binding.clone(),
        anchor: Some(preview.anchor.clone()),
        now_ms: facts.now_ms,
        effects: Vec::new(),
        deferred: None,
    };
    let mut preview_lowered = lower_native(
        runtime,
        preview_batch,
        caster_id,
        &preview_plan,
        &bindings,
        0,
        &mut |plan, hit, target| {
            preview_magnitude
                .as_mut()
                .ok_or(Error::InvalidBatch)?
                .finish(plan, hit, target, &mut |minimum, _| minimum)
        },
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    crate::spell::native_cast_commit::finalize_paid_combat_cast(
        before,
        spell,
        book,
        &mut preview,
        &mut preview_lowered,
        now,
    )?;
    if preview_lowered.batch.effects.iter().any(|effect| {
        effect.target != actor
            && matches!(
                effect.change,
                crate::foundation::runtime_actor_spell_types::OwnerCombatChange::Heal { .. }
            )
    }) && runtime
        .current_player_reentry_protection(actor, session, now.get())
        .map_err(|_| SpellCastDisposition::Rejected)?
    {
        return reject();
    }
    let preview_cues =
        presentation_requests(room, runtime, &preview_plan, profile, &bindings, actor)?;
    let presentation_owner = states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?;
    presentation_owner
        .reserve_before_draw(runtime, preview_cues.len())
        .map_err(|_| SpellCastDisposition::Rejected)?;
    presentation_owner
        .prepare_positions(
            runtime,
            content,
            intent.spell,
            profile,
            &mut preview_lowered.batch,
            preview_cues,
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    runtime
        .stage_spell_batch(&preview_lowered.batch)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    stage_player_batch(
        runtime,
        states,
        &preview_lowered.batch,
        Some(preview.next.clone()),
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let preview_requests = timer_requests(&preview_lowered, profile, &occurrence, position, now)?;
    let stamp = if preview_requests.is_empty() {
        None
    } else {
        let stamp = runtime
            .issue_owner_work()
            .map_err(|_| SpellCastDisposition::Rejected)?;
        timer_preflight(
            runtime,
            states,
            &preview_lowered.batch,
            preview_requests,
            stamp,
        )?;
        Some(stamp)
    };
    before
        .prepare_paid_training(
            &preview.next,
            &preview.anchor,
            training_formula,
            training_occurrence,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let mut paid = prepare_native_owner_cast_with_caster(
        before,
        spell,
        &operational,
        Facts::Combat(&facts),
        now,
        draw,
        &caster,
    )?;
    let Plan::Combat(plan) = &paid.plan else {
        return reject();
    };
    let mut magnitude = if needs_magnitude {
        Some(
            PreparedMagnitudeOwner::qualify(
                runtime,
                before,
                actor,
                session,
                owned.magnitude(),
                plan,
                &bindings,
                &players,
                facts.now_ms,
            )
            .map_err(|_| SpellCastDisposition::Rejected)?,
        )
    } else {
        None
    };
    let batch = OwnerCombatBatch {
        caster: actor,
        attacker,
        current_lease_generation: b.lease_generation,
        command,
        occurrence: occurrence.clone().into(),
        binding,
        anchor: Some(paid.anchor.clone()),
        now_ms: facts.now_ms,
        effects: Vec::new(),
        deferred: None,
    };
    let mut lowered = lower_native(
        runtime,
        batch,
        caster_id,
        plan,
        &bindings,
        0,
        &mut |plan, hit, target| {
            magnitude
                .as_mut()
                .ok_or(Error::InvalidBatch)?
                .finish(plan, hit, target, draw)
        },
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    crate::spell::native_cast_commit::finalize_paid_combat_cast(
        before,
        spell,
        book,
        &mut paid,
        &mut lowered,
        now,
    )?;
    let Plan::Combat(final_plan) = &paid.plan else {
        return reject();
    };
    let cues = presentation_requests(room, runtime, final_plan, profile, &bindings, actor)?;
    let presentation = states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?
        .prepare_positions(
            runtime,
            content,
            intent.spell,
            profile,
            &mut lowered.batch,
            cues,
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let training = before
        .prepare_paid_training(
            &paid.next,
            lowered
                .batch
                .anchor
                .as_ref()
                .ok_or(SpellCastDisposition::Rejected)?,
            training_formula,
            training_occurrence,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let requests = timer_requests(&lowered, profile, &occurrence, position, now)?;
    // Physical/timer capacity was also checked by the non-executed source preview before RNG.
    runtime
        .stage_spell_batch(&lowered.batch)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let timers = if requests.is_empty() {
        None
    } else {
        Some(timer_preflight(
            runtime,
            states,
            &lowered.batch,
            requests,
            stamp.ok_or(SpellCastDisposition::Rejected)?,
        )?)
    };
    Ok(PreparedNativeCombatCast {
        corpse_companion: None,
        direct_companion: None,
        field_policy_revision: None,
        carried_target: None,
        item_operations: Vec::new(),
        item_creations: Vec::new(),
        party,
        before: before.clone(),
        paid: paid.next,
        batch: lowered.batch,
        tile_facts: tiles,
        roster,
        training,
        timers,
        magnitude,
        facts_binding: b.clone(),
        creatures,
        players: player_predecessors,
        equipment: owned.equipment().clone(),
        presentation: Some(presentation),
        installation: None,
    })
}

fn presentation_requests(
    room: &QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    plan: &NativeCombatPlan,
    profile: &crate::spell::native::CompiledNativeSpell,
    actors: &[LiveActorBinding],
    caster: ExactActorRef,
) -> Result<Vec<super::super::spell_presentations::LocatedCueRequest>, SpellCastDisposition> {
    use super::super::spell_presentations::{CueTarget, LocatedCueRequest, QualifiedCueTile};
    let mut result = Vec::new();
    let effect =
        profile.spell()["execution"]["native_behavior"]["parameters"]["effect_asset_binding"]
            .as_str()
            .ok_or(SpellCastDisposition::Rejected)?;
    let actor = |id| {
        actors
            .iter()
            .find(|a| a.source_id == id)
            .map(|a| a.actor)
            .ok_or(SpellCastDisposition::Rejected)
    };
    let mut tiles = BTreeSet::new();
    let mut direct = Vec::new();
    match plan {
        NativeCombatPlan::Combat(p) => match &p.area {
            Some(area) => {
                tiles.extend(area.central_tiles.iter().copied());
                tiles.extend(area.side_tiles.iter().copied());
            }
            None => {
                for hit in &p.hits {
                    if hit.delay_ms == 0 {
                        direct.push(actor(hit.target)?);
                    }
                }
            }
        },
        NativeCombatPlan::MassSpiritMend { area, .. } => {
            tiles.extend(area.central_tiles.iter().copied());
        }
        NativeCombatPlan::MonsterAi { overrides, .. } => {
            for target in overrides {
                if target.delay_ms == 0 {
                    direct.push(actor(target.target)?);
                }
            }
        }
        NativeCombatPlan::Avatar(_) | NativeCombatPlan::ManaShield { .. } => direct.push(caster),
    }
    for target in direct {
        result.push(LocatedCueRequest {
            binding: effect.to_owned(),
            target: CueTarget::Actor(target),
        });
    }
    for position in tiles {
        result.push(LocatedCueRequest {
            binding: effect.to_owned(),
            target: CueTarget::Tile(
                QualifiedCueTile::from_owners(room, runtime, cell(position))
                    .map_err(|_| SpellCastDisposition::Rejected)?,
            ),
        });
    }
    if let Some(binding) = profile.spell()["presentation"]["cast_cue"].as_str() {
        result.push(LocatedCueRequest {
            binding: binding.to_owned(),
            target: CueTarget::Actor(caster),
        });
    }
    Ok(result)
}

fn timer_requests(
    lowered: &crate::spell::combat_batch::LoweredNativeCombat,
    profile: &crate::spell::native::CompiledNativeSpell,
    occurrence: &AbilityOccurrence,
    position: TilePosition,
    now: SemanticTimeMicros,
) -> Result<Vec<ScheduleRequest>, SpellCastDisposition> {
    let mut requests = Vec::new();
    for deferred in &lowered.delayed {
        let due = now
            .get()
            .checked_add(
                deferred
                    .delay_ms
                    .checked_mul(1000)
                    .ok_or(SpellCastDisposition::Rejected)?,
            )
            .ok_or(SpellCastDisposition::Rejected)?;
        requests.push(ScheduleRequest {
            occurrence: SpellTimerOccurrence {
                command: lowered.batch.command,
                phase: deferred.effect.sub_ordinal,
            },
            due: crate::foundation::owner_timer::SemanticTimeMicros::from_micros(due),
            payload: TimerPayload::NativeOwnerEffect(SavedNativeOwnerEffect {
                binding: CastBinding {
                    spell: profile.clone(),
                    caster: lowered.batch.caster,
                    attacker: lowered.batch.attacker,
                    command: lowered.batch.command,
                    occurrence: occurrence.clone(),
                    parent_binding: lowered.batch.binding.clone(),
                    cast_at: crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                        now.get(),
                    ),
                    cast_position: position,
                    cast_snapshot: None,
                },
                effect: deferred.effect.clone(),
            }),
        });
    }
    Ok(requests)
}
fn timer_preflight(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    batch: &OwnerCombatBatch,
    requests: Vec<ScheduleRequest>,
    stamp: crate::foundation::RuntimeWorkStamp,
) -> Result<ValidatedTimerInstall, SpellCastDisposition> {
    let owner = states
        .spell_timers
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?;
    let fence = runtime
        .owner_fence()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let reservation = owner
        .schedule_reservation(fence, stamp, requests)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    owner
        .preflight_install(fence, stamp, batch, reservation)
        .map_err(|_| SpellCastDisposition::Rejected)
}

fn nonce(
    tag: &[u8],
    actor: ExactActorRef,
    session: GameSessionId,
    command: u64,
    extra: &[u8],
) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let hash = Sha256::new()
        .chain_update(tag)
        .chain_update(actor.placement_identity())
        .chain_update(session.as_bytes())
        .chain_update(command.to_be_bytes())
        .chain_update(extra)
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hash[..16]);
    bytes[6] = (bytes[6] & 15) | 0x70;
    bytes[8] = (bytes[8] & 63) | 0x80;
    bytes
}
#[allow(clippy::too_many_arguments)]
async fn prepare_named_failure(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    spell: &SpellDefinition,
    parameter: &ParameterSpellCastIntent,
    command: CommandRef,
    occurrence: AbilityOccurrence,
    training_occurrence: BuildOccurrence,
    now: SemanticTimeMicros,
    reason: &str,
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
    let actor = owned.binding().actor;
    let session = command.game_session_id();
    if !super::parameter_cast::named_player_spell(spell)
        || owned.binding().session != session
        || authority.command() != command
        || authority.character_id_bytes() != owned.binding().character
    {
        return reject();
    }
    let before = states
        .get(runtime, actor, session)
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    let caster = owned
        .caster(
            &before,
            spell,
            content,
            before.owned_harmony_multiplier(spell)?,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let paid = crate::spell::cast::prepare_named_player_failure_owner_cast_with_caster(
        &before, spell, now, &caster,
    )?;
    let position = runtime
        .read_actor_position(actor)
        .map_err(|_| SpellCastDisposition::Rejected)?
        .position();
    let origin = TilePosition {
        x: position.x,
        y: position.y,
        floor: position.floor,
    };
    let tile = read_tile(tx, authority, room, runtime, objects, origin).await?;
    let feedback = super::parameter_cast::source_feedback(reason, Some("poff"))?;
    let result = ParameterSpellCastResult {
        disposition: SpellCastDisposition::Rejected,
        feedback,
        editor: None,
    };
    let result_bytes =
        encode_parameter_spell_cast_result(&result).map_err(|_| SpellCastDisposition::Rejected)?;
    let training = before
        .prepare_paid_training(
            &paid.next,
            &paid.anchor,
            content
                .training_formula()
                .ok_or(SpellCastDisposition::NotAvailable)?,
            training_occurrence,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let batch=OwnerCombatBatch{caster:actor,attacker:CharacterId::decode(&owned.binding().character).map_err(|_|SpellCastDisposition::Rejected)?,
        current_lease_generation:owned.binding().lease_generation,command,occurrence:occurrence.into(),anchor:Some(paid.anchor),
        now_ms:now.get()/1000,effects:Vec::new(),deferred:None,
        binding:serde_json::to_vec(&json!({"intent":source_parameter_intent(&parameter.intent,parameter)?,"parameter_result":result_bytes,
            "spell":format!("{spell:?}"),"before":format!("{before:?}"),"paid":format!("{:?}",paid.next),"owned":format!("{owned:?}")})).map_err(|_|SpellCastDisposition::Rejected)?};
    let roster = runtime
        .positioned_actor_census()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    Ok(PreparedNativeCombatCast {
        corpse_companion: None,
        direct_companion: None,
        field_policy_revision: None,
        carried_target: None,
        item_operations: Vec::new(),
        item_creations: Vec::new(),
        party: None,
        before: before.clone(),
        paid: paid.next,
        batch,
        tile_facts: BTreeMap::from([(origin, tile)]),
        roster,
        training,
        timers: None,
        magnitude: None,
        facts_binding: owned.binding().clone(),
        creatures: Vec::new(),
        players: vec![(actor, session, before)],
        equipment: owned.equipment().clone(),
        presentation: None,
        installation: None,
    })
}

fn source_cost_request(
    prepared: &PreparedNativeCombatCast,
    spell: &SpellDefinition,
) -> Result<crate::durability::spell_items_abi::SpellItemTransactionRequest, SpellCastDisposition> {
    use crate::durability::spell_items_abi::{SourceCasterOrigin, SpellItemTransactionRequest};
    let b = prepared.batch();
    let profile = spell
        .authored
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?;
    let cost = crate::spell::companion_lifecycle::familiar_cost_binding(
        &prepared.before,
        &prepared.paid,
        b.anchor.as_ref().ok_or(SpellCastDisposition::Rejected)?,
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let transaction_id = nonce(
        b"oteryn:native-cast-source:v1",
        b.caster,
        b.command.game_session_id(),
        b.command.command_id().get(),
        &[],
    );
    Ok(SpellItemTransactionRequest {
        command: b.command,
        spell: crate::durability::item_mint::TypedDefinitionRef {
            family: "Spell".into(),
            production_key: profile.header.identity.key.clone(),
            revision_ref: profile.header.identity.revision.clone(),
        },
        catalog_digest: prepared.facts_binding.content_digest,
        transaction_id,
        event_id: nonce(
            b"oteryn:native-cast-effects:v1",
            b.caster,
            b.command.game_session_id(),
            b.command.command_id().get(),
            format!("{b:?}").as_bytes(),
        ),
        cost,
        caster_origin: Some(SourceCasterOrigin {
            actor: b.caster,
            character_lease_generation: b.current_lease_generation,
        }),
        operations: prepared.item_operations.clone(),
        direct_companion: prepared.direct_companion.as_ref().map(|companion|
            crate::durability::spell_item_transaction::PreparedDirectCompanionAcquisition::from_reservation(
                transaction_id, &companion.reservation)).transpose().map_err(|_| SpellCastDisposition::Rejected)?,
        companion: prepared.corpse_companion.as_ref().map(|companion| {
            crate::durability::spell_item_transaction::PreparedCompanionAcquisition::from_spawn(
                transaction_id,
                &companion.spawn,
            )
        }),
    })
}

impl super::super::ComposedFreshAdmission<'_, '_, '_> {
    /// Retry the retained original before durable control loss removes its current
    /// source authority. Pending keeps the actor and original command outstanding.
    pub(in crate::gameplay_transport) async fn reconcile_pending_native_for_control_loss<
        A: super::super::spell_access_facts::CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        access: &A,
    ) -> Option<NativeCastDispatch> {
        let original = {
            let states = self.spell_states.lock().await;
            states
                .pending_native
                .iter()
                .find(|p| p.actor == actor && p.session == session)
                .map(|p| {
                    (
                        p.prepared.batch.command.command_id().get(),
                        p.intent,
                        p.rune,
                        p.parameter.clone(),
                    )
                })
        };
        let (command, intent, rune, parameter) = original?;
        let result = self
            .cast_native_combat_inner(
                actor, session, command, &intent, rune, parameter, &mut None, access, true,
            )
            .await;
        if self
            .spell_states
            .lock()
            .await
            .has_pending_native(actor, session)
        {
            Some(NativeCastDispatch::Pending)
        } else {
            Some(result)
        }
    }

    /// A missing registered source owner is passed explicitly by the caller and
    /// remains unknown. No default Premium, learned spell or Wheel grant is added.
    pub(in crate::gameplay_transport) async fn cast_native_combat<
        A: super::super::spell_access_facts::CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: &SpellCastIntent,
        access: &A,
    ) -> NativeCastDispatch {
        self.cast_native_combat_inner(
            actor, session, command_id, intent, None, None, &mut None, access, false,
        )
        .await
    }
    /// Unallocated candidate port; caller supplies an exact original instance/revision,
    /// while the real current Item owner derives custody and source count in SQL.
    pub(in crate::gameplay_transport) async fn cast_rune_item<
        A: super::super::spell_access_facts::CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        rune: &ItemSpellCastIntent,
        access: &A,
    ) -> NativeCastDispatch {
        self.cast_native_combat_inner(
            actor,
            session,
            command_id,
            &rune.intent,
            Some(*rune),
            None,
            &mut None,
            access,
            false,
        )
        .await
    }
    pub(in crate::gameplay_transport) async fn cast_named_player_parameters<
        A: super::super::spell_access_facts::CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        parameter: &ParameterSpellCastIntent,
        access: &A,
    ) -> super::parameter_cast::ParameterCastDispatch {
        let mut output = None;
        let cast = self
            .cast_native_combat_inner(
                actor,
                session,
                command_id,
                &parameter.intent,
                None,
                Some(parameter.clone()),
                &mut output,
                access,
                false,
            )
            .await;
        super::parameter_cast::ParameterCastDispatch {
            cast,
            result: output,
        }
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "the owner turn binds every independently resolved fact explicitly"
    )]
    async fn cast_native_combat_inner<
        A: super::super::spell_access_facts::CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: &SpellCastIntent,
        rune: Option<ItemSpellCastIntent>,
        parameter: Option<ParameterSpellCastIntent>,
        parameter_output: &mut Option<ParameterSpellCastResult>,
        access: &A,
        reconcile_only: bool,
    ) -> NativeCastDispatch {
        use super::super::spell_access_facts::load_owned_cast_facts_in_transaction;
        use crate::durability::character_progression::CurrentCharacterGameplayFence;
        use crate::durability::fresh_admission::FreshAdmissionStore;
        use crate::durability::item_transfer::CurrentCharacterItemFence;
        use crate::durability::{
            DurabilityError, spell_item_transaction as item_tx,
            spell_items_abi::SpellItemTransactionOutcome,
        };
        use crate::foundation::{CommandId, GameSessionState};
        let Some((spell, active_spell)) = self.spells.source_indexed(intent.spell) else {
            return NativeCastDispatch::NotApplicable;
        };
        let native_combat = matches!(&spell.execution, Execution::NativeProfile(p) if matches!(
            p.spell()["execution"]["native_behavior"]["key"].as_str(), Some("wheel_combat" | "avatar_state"
                | "monster_ai_override" | "mass_spirit_mend" | "mana_shield_capacity")));
        if !native_combat
            && !is_source_self(spell)
            && !ordinary_combat::applicable(spell)
            && !native_world_item_cast::applicable(spell)
            && !native_companion_item_cast::applicable(spell)
            && !native_companion_item_cast::direct_applicable(spell)
        {
            return NativeCastDispatch::NotApplicable;
        }
        let rejected = || NativeCastDispatch::Outcome(super::SpellCastOutcome::rejected());
        if parameter.as_ref().is_some_and(|p| {
            p.intent != *intent
                || intent.target != SpellTarget::None
                || intent.aim_at_target
                || rune.is_some()
                || !(super::parameter_cast::named_player_spell(spell)
                    || native_companion_item_cast::direct_applicable(spell))
        }) {
            return rejected();
        }
        if rune.as_ref().is_some_and(|r| {
            oteryn_protocol_oteryn::actor_spell_item_v2::encode_item_spell_cast_intent(r).is_err()
                || (r.target_item.is_some() && !matches!(&spell.execution,Execution::NativeProfile(p) if p.spell()["execution"]["native_behavior"]["key"]=="tile_item_operation" && p.spell()["execution"]["native_behavior"]["parameters"]["operation"]=="mimic_item"))
        }) {return rejected();}
        match (&spell.carrier, rune.as_ref()) {
            (crate::spell::Carrier::Rune { .. }, None) => {
                return NativeCastDispatch::Outcome(super::SpellCastOutcome {
                    disposition: SpellCastDisposition::NotAvailable,
                    vitals: None,
                });
            }
            (crate::spell::Carrier::Instant { .. }, Some(_)) => return rejected(),
            (_, Some(r))
                if r.intent != *intent
                    || r.expected_state_revision == 0
                    || r.rune_item_instance[6] >> 4 != 7
                    || r.rune_item_instance[8] >> 6 != 2 =>
            {
                return rejected();
            }
            _ => {}
        }

        if !active_spell || command_id == 0 {
            return rejected();
        }
        let Some(active) = self.active_generation else {
            return rejected();
        };
        let Some(native) = active.native_gameplay() else {
            return rejected();
        };
        let Some(_room) = self.qualified_room else {
            return rejected();
        };
        let Ok((current, _)) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(session)
            .await
        else {
            return NativeCastDispatch::Pending;
        };
        if current.session_state() != GameSessionState::Active {
            return rejected();
        }
        let Ok(character) =
            crate::domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
        else {
            return rejected();
        };
        let Ok(root_record) = self
            .root
            .read_current_character(self.character, character)
            .await
        else {
            return NativeCastDispatch::Pending;
        };
        let fence = CurrentCharacterGameplayFence {
            character_id: character,
            game_session_id: session,
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: current.current_runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
            expected_character_revision: root_record.revision,
        };
        let Ok(command) = CommandId::new(command_id).map(|id| CommandRef::new(session, id)) else {
            return rejected();
        };
        let mut runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        if runtime.player_control_facts(actor, session).is_err()
            || runtime.owner_fence().is_err()
            || runtime.content_pin().server_artifact_digest() != native.source_digest()
        {
            return rejected();
        }
        let Ok(attacker) = CharacterId::decode(character.as_bytes()) else {
            return rejected();
        };
        match replay_with_rune(
            &mut runtime,
            &mut states,
            actor,
            attacker,
            fence.character_lease_generation,
            command,
            intent,
            rune.as_ref(),
            parameter.as_ref(),
            parameter_output,
        ) {
            Ok(Some(_)) => {
                return NativeCastDispatch::Outcome(super::SpellCastOutcome {
                    disposition: parameter_output
                        .as_ref()
                        .map_or(SpellCastDisposition::Cast, |r| r.disposition),
                    vitals: super::observe_vitals(&runtime, &states, actor, session),
                });
            }
            Ok(None) => {}
            Err(_) => return rejected(),
        }
        let retained_index = states
            .pending_native
            .iter()
            .position(|p| p.actor == actor && p.session == session);
        if let Some(index) = retained_index {
            let pending = &states.pending_native[index];
            if pending.intent != *intent
                || pending.rune != rune
                || pending.parameter != parameter
                || pending.prepared.batch.command != command
            {
                return NativeCastDispatch::Pending;
            }
        } else if reconcile_only
            || states.pending_native.len() >= MAX_PENDING_NATIVE
            || states.pending_native.try_reserve(1).is_err()
        {
            return rejected();
        }
        let retained = retained_index.map(|index| states.pending_native.remove(index));
        let objects = self.door.lock().await;
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            if let Some(retained) = retained {
                states.pending_native.push(retained);
            }
            return NativeCastDispatch::Pending;
        };
        let intent = *intent;
        // SPELL-TARGET-1: the held attack target (ATTACK-0 §4), read last in the lock order.
        let attack_target = self
            .attack
            .lock()
            .await
            .combat_state(actor, session, self.owner_now())
            .target;
        // Prepared lives in the external context before any committing await. Cancellation of
        // the bounded pass therefore leaves the original normalized occurrence recoverable.
        let mut context = (
            self,
            access,
            &mut *runtime,
            &mut *states,
            &*objects,
            retained,
            parameter_output,
        );
        let result=pass.run_with_context(&mut context,move|holder,deadline,ctx|Box::pin(async move{
            let (owner,access,runtime,states,objects,pending,parameter_output)=ctx;
            let active=owner.active_generation.ok_or(DurabilityError::Unavailable)?;
            let content=active.native_gameplay().ok_or(DurabilityError::Unavailable)?;
            let room=owner.qualified_room.ok_or(DurabilityError::Unavailable)?;
            let spell=owner.spells.indexed(intent.spell).ok_or(DurabilityError::Unavailable)?;
            let item_fence=CurrentCharacterItemFence{character_id:fence.character_id,game_session_id:session,
                connection_generation:fence.connection_generation,character_lease_generation:fence.character_lease_generation,
                runtime_scope:fence.runtime_scope,scope_ownership_generation:fence.scope_ownership_generation};
            let mut tx=item_tx::begin_spell_owner_transaction(holder,deadline).await?;
            let authority=item_tx::assert_spell_item_authority_in_transaction(&mut tx,owner.root,owner.character,
                owner.holder,&item_fence,command,content.source_digest()).await.map_err(|_|DurabilityError::Unavailable)?;
            let state=states.get(runtime,actor,session).ok_or(DurabilityError::Unavailable)?;
            let mut owned=load_owned_cast_facts_in_transaction(&mut tx,owner.root,owner.character,owner.holder,&item_fence,
                command,runtime,actor,state,active,*access,owner.owner_now().get()).await.map_err(|_|DurabilityError::Unavailable)?;
            if pending.is_none(){
                if states.apply_current_premium(runtime,actor,session,&owned,owner.owner_now().get())
                    .map_err(|_|DurabilityError::Unavailable)? {
                    let state=states.get(runtime,actor,session).ok_or(DurabilityError::Unavailable)?;
                    owned=load_owned_cast_facts_in_transaction(&mut tx,owner.root,owner.character,owner.holder,&item_fence,
                        command,runtime,actor,state,active,*access,owner.owner_now().get()).await
                        .map_err(|_|DurabilityError::Unavailable)?;
                }
                let now=owner.owner_now();
                let state=states.get(runtime,actor,session).ok_or(DurabilityError::Unavailable)?;
                if matches!(state.source_party_vocation(),crate::spell::Vocation::Monk|crate::spell::Vocation::ExaltedMonk){
                    runtime.assert_actor_spell_unreserved(actor).map_err(|_|DurabilityError::Unavailable)?;
                    let world=super::super::party_spell_owner::read_source_party_world_in_transaction(
                        &mut tx,owner.root,&authority,runtime,states,actor,session).await.map_err(|_|DurabilityError::Unavailable)?;
                    let changed=states.get_mut(runtime,actor,session).ok_or(DurabilityError::Unavailable)?
                        .tick_with_world(now,&world).map_err(|_|DurabilityError::Unavailable)?;
                    if changed {
                        let state=states.get(runtime,actor,session).ok_or(DurabilityError::Unavailable)?;
                        owned=load_owned_cast_facts_in_transaction(&mut tx,owner.root,owner.character,owner.holder,
                            &item_fence,command,runtime,actor,state,active,*access,now.get()).await.map_err(|_|DurabilityError::Unavailable)?;
                    }
                }
                let rune_reservation=match rune.as_ref(){
                    Some(r)=>match rune_item_cast::reserve(&mut tx,&authority,runtime,room,content,spell,r).await {
                        Ok(reserved)=>Some(reserved),
                        Err(_)=>return Ok(NativeCastDispatch::Outcome(super::SpellCastOutcome{
                            disposition:SpellCastDisposition::Rejected,vitals:super::observe_vitals(runtime,states,actor,session)})),
                    },
                    None=>None,
                };
                let bytes=nonce(b"oteryn:native-cast-occurrence:v1",actor,session,command_id,&[]);
                let hex=|v:[u8;16]|v.iter().map(|b|format!("{b:02x}")).collect::<String>();
                let revisions=crate::ability::RevisionSet::new("ruleset:spell-native-r20",&format!("content:{}",
                    content.source_digest().iter().map(|b|format!("{b:02x}")).collect::<String>()),
                    "world:spell-pve-candidate-r21","formula:spell-p2-r20","simulation:v1")
                    .map_err(|_|DurabilityError::Unavailable)?;
                let occurrence=AbilityOccurrence::new(&format!("native-cast:{}",hex(bytes)),revisions)
                    .map_err(|_|DurabilityError::Unavailable)?;
                let decision=oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(bytes);
                let stream=oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(content.source_digest());
                let mut ordinal=0_u64;let mut invalid_draw=false;let mut draw=|minimum,maximum|{
                    match (oteryn_simulation_determinism::deterministic_decision_u64(&stream,decision,"spell.cast.draw",ordinal),ordinal.checked_add(1)) {
                        (Ok(value),Some(next))=>{ordinal=next;crate::spell::uniform_draw(value,minimum,maximum)},
                        _=>{invalid_draw=true;minimum},
                    }
                };
                let training_occurrence=BuildOccurrence::from_bytes(nonce(b"oteryn:native-cast-training:v1",actor,session,command_id,&[]))
                    .map_err(|_|DurabilityError::Unavailable)?;
                let prepared_result=if native_companion_item_cast::direct_applicable(spell) {
                    native_companion_item_cast::prepare_direct(&mut tx,owner.root,&authority,runtime,states,room,objects,content,&owned,
                        owner.spells,spell,&intent,command,occurrence,training_occurrence,now,rune.as_ref(),parameter.as_ref(),&mut draw).await
                } else if let Some(parameter)=parameter.as_ref(){
                    match super::parameter_cast::resolve_named_player_in_transaction(&mut tx,owner.root,&authority,runtime,states,parameter).await {
                        Ok(super::parameter_cast::NamedPlayerResolution::Found(target))=>ordinary_combat::prepare_named(&mut tx,owner.root,&authority,runtime,states,room,objects,content,&owned,
                            owner.spells,spell,&intent,command,occurrence,training_occurrence,now,parameter,&target,&mut draw).await,
                        Ok(super::parameter_cast::NamedPlayerResolution::Absent(reason))=>prepare_named_failure(&mut tx,&authority,runtime,states,room,objects,content,&owned,spell,parameter,command,occurrence,training_occurrence,now,reason).await,
                        Err(disposition)=>Err(disposition),
                    }
                } else if native_companion_item_cast::applicable(spell) {
                    native_companion_item_cast::prepare(&mut tx,owner.root,&authority,runtime,states,room,objects,content,&owned,
                        owner.spells,spell,&intent,command,occurrence,training_occurrence,now,rune.as_ref(),&mut draw).await
                } else if native_world_item_cast::applicable(spell) {
                    native_world_item_cast::prepare(&mut tx,owner.root,&authority,runtime,states,room,objects,content,&owned,
                        owner.spells,spell,&intent,command,occurrence,training_occurrence,now,rune.as_ref(),&mut draw).await
                } else if is_source_self(spell) {
                    // Simple self effects use their existing whole-player owner.
                    // The broad ordinary predicate also includes these Effects;
                    // target/area/chain spells retain the magnitude-owner route.
                    prepare_source_self_from_owners(&mut tx,owner.root,&authority,runtime,states,room,objects,content,&owned,
                        owner.spells,spell,&intent,command,occurrence,training_occurrence,now,rune.as_ref(),&mut draw).await
                } else if ordinary_combat::applicable(spell) {
                    ordinary_combat::prepare(&mut tx,owner.root,&authority,runtime,states,room,objects,content,&owned,
                        owner.spells,spell,&intent,command,occurrence,training_occurrence,now,rune.as_ref(),attack_target,&mut draw).await
                } else {
                    prepare_from_owners(&mut tx,owner.root,&authority,runtime,states,room,objects,content,&owned,
                        owner.spells,spell,&intent,command,occurrence,training_occurrence,now,rune.as_ref(),&mut draw).await
                };
                let mut prepared=match prepared_result{
                    Ok(p)=>p,Err(disposition)=>{
                    #[cfg(test)]
                    eprintln!("SEAM_EVIDENCE native_combat_prepare result={disposition:?} ordinary={}",ordinary_combat::applicable(spell));
                    return Ok(NativeCastDispatch::Outcome(super::SpellCastOutcome{disposition,vitals:super::observe_vitals(runtime,states,actor,session)}))
                }
                };
                if invalid_draw{return Err(DurabilityError::Unavailable);}
                if let Some(reserved)=rune_reservation.as_ref(){
                    item_tx::validate_rune_reservation_in_transaction(&mut tx,&authority,reserved).await
                        .map_err(|_|DurabilityError::Unavailable)?;
                    prepared.equipment=rune_item_cast::equipment_successor(&prepared.equipment,reserved.consumption())
                        .map_err(|_|DurabilityError::Unavailable)?;
                }
                let mut request=source_cost_request(&prepared,spell).map_err(|_|DurabilityError::Unavailable)?;
                if let Some(reserved)=rune_reservation {
                    request.operations.push(crate::durability::spell_items_abi::SpellItemOperation::ConsumeInventory(
                        reserved.consumption().clone()));
                }
                ordinary_field_items::append_creations(&mut tx,&authority,owner.root,runtime,room,objects,content,&mut prepared,&mut request).await
                    .map_err(|_|DurabilityError::Unavailable)?;
                *pending=Some(PendingNativeCast{actor,session,intent,rune,parameter:parameter.clone(),fence,prepared,owned:owned.clone(),request});
            }
            let attempt=pending.as_mut().ok_or(DurabilityError::Unavailable)?;
            // Current actor facts never come from historical cost/training rows.
            if attempt.prepared.before!=*states.get(runtime,actor,session).ok_or(DurabilityError::Unavailable)?{
                return Err(DurabilityError::Unavailable);
            }
            attempt.prepared.stage_installation(runtime,states).map_err(|_|DurabilityError::Unavailable)?;
            let reconnect=crate::durability::admission_journal::spell_reconnect::prove_pending_spell_reconnect(
                &mut tx,&authority,&attempt.fence,&item_fence).await?;
            // Re-read only the original source footprint. A changed/unavailable current tile
            // rejects a NEW write, while a genuine historical receipt keeps its original plan.
            let mut original_tiles_current=true;
            for (position,expected) in &attempt.prepared.tile_facts {
                if !matches!(read_tile(&mut tx,&authority,room,runtime,objects,*position).await,
                    Ok(actual) if actual==*expected) { original_tiles_current=false; }
            }
            let original_party_current=if let Some(party)=attempt.prepared.party.as_ref(){
                match crate::durability::world_party::read_world_party_in_transaction(&mut tx,&authority).await{
                    Ok(current)=>party.matches_current_world(&current)&&party.validate_physical(runtime,states).is_ok(),
                    Err(_)=>false,
                }
            }else{true};
            let field_policy_current=if let Some(expected)=attempt.prepared.field_policy_revision.as_ref(){
                match crate::durability::spell_field_policy::read_world_field_policy_in_transaction(&mut tx,owner.root,authority.runtime_scope(),authority.scope_generation()).await {
                    Ok(Some(current))=>current.bind(&tx).revision()==expected,
                    _=>false,
                }
            }else{true};
            let carried_target_current=native_world_item_cast::carried_target_is_current(&mut tx,&authority,content,attempt.prepared.carried_target.as_ref()).await;
            let companion_current=if let Some(companion)=attempt.prepared.corpse_companion.as_ref(){
                match native_companion_item_cast::read_current_restrictions(&mut tx,owner.root,&authority,runtime,states,&owned,owner.owner_now()).await {
                    Ok(current)=>current==companion.restrictions && companion.spawn.validate_current(runtime,&current).is_ok(),
                    Err(_)=>false,
                }
            }else{true};
            let direct_current=if let Some(companion)=attempt.prepared.direct_companion.as_ref(){
                match native_companion_item_cast::read_current_direct_restrictions(&mut tx,owner.root,&authority,runtime,states,&owned,owner.owner_now(),spell).await {
                    Ok(current)=>current==companion.restrictions && companion.reservation.validate_current(runtime,&current).is_ok(),
                    Err(_)=>false,
                }
            }else{true};
            let mut new_guard_refused=false;
            let items=item_tx::apply_spell_items_in_transaction_guarded(&mut tx,&authority,&attempt.request,||{
                (if original_tiles_current && original_party_current && carried_target_current && field_policy_current && companion_current && direct_current {
                    attempt.prepared.validate_new_write(runtime,states,&owned,&attempt.owned,spell,
                        owner.owner_now().get(),reconnect.as_ref())
                } else { Err(Error::SnapshotChanged) }).map_err(|_|{
                    new_guard_refused=true;
                    item_tx::SpellItemError::Rejected("native new-write source eligibility changed")
                })
            }).await;
            let items=match items {
                Ok(items)=>items,
                Err(_) if new_guard_refused=>{
                    // The writer holds the exact-command lock and found no historical receipt.
                    // Only a successful rollback proves this attempt cannot later COMMIT.
                    tx.rollback().await.map_err(|_|DurabilityError::Unavailable)?;
                    if let Some(companion)=attempt.prepared.direct_companion.as_ref(){
                        companion.reservation.rollback_physical(runtime).map_err(|_|DurabilityError::Unavailable)?;
                    }
                    if let Some(companion)=attempt.prepared.corpse_companion.as_ref(){
                        companion.spawn.rollback_physical(runtime).map_err(|_|DurabilityError::Unavailable)?;
                    }
                    if let Some(installation)=attempt.prepared.installation.as_ref(){
                        runtime.release_definitely_uncommitted_spell_batch(&installation.physical)
                            .map_err(|_|DurabilityError::Unavailable)?;
                    }
                    if let Some(presentation)=attempt.prepared.presentation.as_ref(){
                        states.presentations.as_mut().ok_or(DurabilityError::Unavailable)?
                            .release_definitely_uncommitted(presentation);
                    }
                    *pending=None;
                    return Ok(NativeCastDispatch::Outcome(super::SpellCastOutcome{
                        disposition:SpellCastDisposition::Rejected,
                        vitals:super::observe_vitals(runtime,states,actor,session)}));
                }
                Err(_)=>return Err(DurabilityError::Unavailable),
            };
            let private_result=if let Some(parameter)=attempt.parameter.as_ref(){
                let binding:Value=serde_json::from_slice(&attempt.prepared.batch.binding).map_err(|_|DurabilityError::InvalidStoredState)?;
                let bytes:Vec<u8>=serde_json::from_value(binding["parameter_result"].clone()).map_err(|_|DurabilityError::InvalidStoredState)?;
                let result=decode_parameter_spell_cast_result(&bytes).map_err(|_|DurabilityError::InvalidStoredState)?;
                crate::durability::spell_parameter_result::write_parameter_result_in_transaction(&mut tx,&authority,&attempt.request.spell,parameter,&result,Some(attempt.request.transaction_id)).await.map_err(|_|DurabilityError::Unavailable)?;
                Some(result)
            }else{None};
            let formula=content.training_formula().ok_or(DurabilityError::Unavailable)?;
            let training=if let Some(request)=attempt.prepared.training_request(){
                Some(crate::durability::character_build::prepare_character_build_in_transaction(owner.root,&mut tx,
                    owner.character,owner.holder,attempt.fence,request.clone(),formula).await.map_err(|_|DurabilityError::Unavailable)?)
            }else{None};
            let committed=match items{
                SpellItemTransactionOutcome::Applied(items)=>{
                    let companion=if let Some(companion)=attempt.prepared.corpse_companion.as_ref(){
                        Some(item_tx::record_companion_acquisition_in_transaction(&mut tx,&authority,&attempt.request,&companion.spawn)
                            .await.map_err(|_|DurabilityError::Unavailable)?)
                    }else{None};
                    let prepared=if attempt.prepared.direct_companion.is_some(){
                        if companion.is_some(){return Err(DurabilityError::InvalidStoredState);}
                        let direct=item_tx::record_prepared_direct_companion_acquisition_in_transaction(&mut tx,&authority,&attempt.request)
                            .await.map_err(|_|DurabilityError::Unavailable)?;
                        item_tx::stage_spell_owner_commit_with_direct(&mut tx,&authority,items,direct,deadline).await
                    } else {
                        item_tx::stage_spell_owner_commit(&mut tx,&authority,items,companion,None,deadline).await
                    }.map_err(|_|DurabilityError::Unavailable)?;
                    crate::durability::spell_owner_commit::commit_spell_owner_transaction(tx,prepared).await?
                }
                SpellItemTransactionOutcome::AlreadyCommitted(items)=>{
                    let companion=if attempt.prepared.corpse_companion.is_some(){
                        item_tx::reconcile_committed_companion_acquisition_in_transaction(&mut tx,&authority,attempt.request.transaction_id)
                            .await.map_err(|_|DurabilityError::Unavailable)?
                    }else{None};
                    let committed=if attempt.prepared.direct_companion.is_some(){
                        if companion.is_some(){return Err(DurabilityError::InvalidStoredState);}
                        let direct=item_tx::reconcile_committed_direct_companion_acquisition_in_transaction(&mut tx,&authority,&attempt.request)
                            .await.map_err(|_|DurabilityError::Unavailable)?.ok_or(DurabilityError::InvalidStoredState)?;
                        item_tx::reconcile_spell_owner_commit_with_direct_in_transaction(&mut tx,&authority,items,direct).await
                    } else {
                        item_tx::reconcile_spell_owner_commit_in_transaction(&mut tx,&authority,items,companion).await
                    }.map_err(|_|DurabilityError::Unavailable)?;
                    drop(tx);
                    committed
                }
            };
            let receipt=training.map(|p|p.after_commit(&committed)).transpose().map_err(|_|DurabilityError::InvalidStoredState)?;
            let receipt=receipt.as_ref().map(|r|match r{
                crate::durability::character_build::BuildCommitOutcome::Committed(r)|
                crate::durability::character_build::BuildCommitOutcome::AlreadyCommitted(r)=>r});
            // Re-read current real equipment/account projections after joined training may have
            // advanced the root. The original formula roll is retained, never evaluated again.
            let mut fresh=item_tx::begin_spell_owner_transaction(holder,deadline).await?;
            let fresh_authority=item_tx::assert_spell_item_authority_in_transaction(&mut fresh,owner.root,
                owner.character,owner.holder,&item_fence,command,content.source_digest()).await
                .map_err(|_|DurabilityError::Unavailable)?;
            let fresh_reconnect=crate::durability::admission_journal::spell_reconnect::prove_pending_spell_reconnect(
                &mut fresh,&fresh_authority,&attempt.fence,&item_fence).await?;
            let state=states.get(runtime,actor,session).ok_or(DurabilityError::Unavailable)?;
            let current_owned=load_owned_cast_facts_in_transaction(&mut fresh,owner.root,owner.character,owner.holder,
                &item_fence,command,runtime,actor,state,active,*access,owner.owner_now().get()).await
                .map_err(|_|DurabilityError::Unavailable)?;
            attempt.prepared.commit(runtime,states,&current_owned,receipt,committed.companion(),committed.direct_companion(),fresh_reconnect.as_ref()).map_err(|_|DurabilityError::Unavailable)?;
            // No SQL write follows HP/player/timer installation. Rollback only releases the
            // current read locks; its outcome cannot retract the already observed source COMMIT.
            drop(fresh);
            let disposition=private_result.as_ref().map_or(SpellCastDisposition::Cast,|r|r.disposition);
            **parameter_output=private_result;
            *pending=None;
            Ok(NativeCastDispatch::Outcome(super::SpellCastOutcome{disposition,
                vitals:super::observe_vitals(runtime,states,actor,session)}))
        })).await;
        if let Some(pending) = context.5.take() {
            context.3.pending_native.push(pending);
        }
        match result {
            Ok(result) => result,
            Err(_) => NativeCastDispatch::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    fn p(x: i32, y: i32) -> TilePosition {
        TilePosition { x, y, floor: 7 }
    }
    #[test]
    fn rune_instance_and_revision_are_part_of_original_command_binding() {
        let intent = SpellCastIntent {
            spell: std::num::NonZeroU32::new(1).unwrap(),
            target: SpellTarget::None,
            aim_at_target: false,
        };
        let rune = ItemSpellCastIntent {
            target_item: None,
            intent,
            rune_item_instance: [7; 16],
            expected_state_revision: 1,
        };
        let original = source_cast_intent(&intent, Some(&rune));
        let mut changed = rune;
        changed.expected_state_revision += 1;
        assert_ne!(original, source_cast_intent(&intent, Some(&changed)));
        changed = rune;
        changed.rune_item_instance[0] += 1;
        assert_ne!(original, source_cast_intent(&intent, Some(&changed)));
        assert_ne!(original, source_cast_intent(&intent, None));
        assert_eq!(source_cast_intent(&intent, None), source_intent(&intent));
    }
    #[test]
    fn named_parameter_binding_preserves_original_query_and_separates_rune_and_v1() {
        let intent = SpellCastIntent {
            spell: std::num::NonZeroU32::new(1).unwrap(),
            target: SpellTarget::None,
            aim_at_target: false,
        };
        let p = ParameterSpellCastIntent {
            intent,
            parameter: Some("Al D~".into()),
        };
        let original = source_original_intent(&intent, None, Some(&p)).unwrap();
        let mut other = p.clone();
        other.parameter = Some("Al Dric".into());
        assert_ne!(
            original,
            source_original_intent(&intent, None, Some(&other)).unwrap()
        );
        assert_ne!(
            original,
            source_original_intent(&intent, None, None).unwrap()
        );
        let rune = ItemSpellCastIntent {
            target_item: None,
            intent,
            rune_item_instance: [7; 16],
            expected_state_revision: 1,
        };
        assert!(source_original_intent(&intent, Some(&rune), Some(&p)).is_err());
        let mut forged = p.clone();
        forged.intent.target = SpellTarget::AttackTarget;
        assert!(source_parameter_intent(&forged.intent, &forged).is_err());
        assert!(source_parameter_intent(&intent, &forged).is_err());
    }
    #[test]
    fn source_sight_excludes_endpoints_and_wraps_equal_diagonal() {
        assert!(sight_steps(p(0, 0), p(1, 1)).unwrap().is_empty());
        assert_eq!(
            sight_steps(p(0, 0), p(4, 0)).unwrap(),
            vec![(p(1, 0), false), (p(2, 0), false), (p(3, 0), false)]
        );
        assert_eq!(
            sight_steps(p(0, 0), p(4, 4))
                .unwrap()
                .iter()
                .map(|x| x.0)
                .collect::<Vec<_>>(),
            vec![p(1, 1), p(2, 2), p(3, 3)]
        );
        assert_eq!(
            sight_steps(p(4, 0), p(0, 4))
                .unwrap()
                .iter()
                .map(|x| x.0)
                .collect::<Vec<_>>(),
            vec![p(1, 3), p(2, 2), p(3, 1)]
        );
        assert!(sight_steps(p(0, 0), p(65, 0)).is_err());
    }
    #[test]
    fn source_area_rotation_and_origin_are_not_permission() {
        let recipe = json!({"area":{"orthogonal":[[1],[2]],"diagonal":null,"directional":true}});
        assert_eq!(
            area_candidates(&recipe, p(9, 9), Direction::North).unwrap(),
            BTreeSet::from([p(9, 8)])
        );
        assert_eq!(
            area_candidates(&recipe, p(9, 9), Direction::East).unwrap(),
            BTreeSet::from([p(10, 9)])
        );
    }
}

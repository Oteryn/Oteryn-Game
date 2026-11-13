//! Resolve source USE facts through the existing custody and composed map owners.
use super::*;
use crate::durability::item_use::soul_war_source::{
    QualifiedSoulWarUseTarget, SelectedSoulWarUseTarget, SoulWarUseTargetRead, source_use,
};
use crate::foundation::{
    CommandId, CommandRef, SourceActorReservation, SourceActorReservationProof,
    source_reservation_seal,
};
use crate::gameplay_transport::soul_war_callbacks::cruelty::maw::PreparedMawUse;
use crate::gameplay_transport::soul_war_callbacks::megalomania::sanity::PreparedSanityUse;
use crate::map::view::{self, MapEntryKey};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseIntent, UseTarget};

#[cfg(test)]
#[path = "soul_war_source/test_pause.rs"]
mod test_pause;

struct CurrentSelection<'a> {
    command: UseCommand,
    actor: ExactActorRef,
    source: &'a TypedDefinitionRef,
    selected: SelectedSoulWarUseTarget,
}

#[derive(Debug, Clone)]
struct PreparedSorrowUse {
    before: oteryn_gameplay_encounters::bosses::goshnars_hatred::HatredRoom,
    after: oteryn_gameplay_encounters::bosses::goshnars_hatred::HatredRoom,
    affected: [ExactActorRef; 1],
    at: crate::foundation::MovementLocalPosition,
}
impl SoulWarUseTargetRead for CurrentSelection<'_> {
    fn source_definition(&self, _: &UseIntent) -> Option<TypedDefinitionRef> {
        Some(self.source.clone())
    }
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        rotten_essence::current(runtime, self.actor, self.command)
    }
    fn resolve(&self, _: &UseIntent) -> Option<SelectedSoulWarUseTarget> {
        Some(self.selected.clone())
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// The returned entry is read from the actual fenced backpack tree. The
    /// caller must pass that same entry and proof to the paired Item/Quest owner.
    pub(in crate::gameplay_transport) async fn qualify_soul_war_source_use(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Option<(BackpackEntry, QualifiedSoulWarUseTarget, String, String)> {
        let fence = command.item_fence?;
        let native = self.active_generation?.native_gameplay()?;
        {
            let runtime = self.runtime.lock().await;
            if !rotten_essence::current(&runtime, actor, command)
                // A House actor is a proxy: its map must be resolved by the
                // House owner, never the origin Channel map used below.
                || runtime.house_actor_state(actor, command.game_session_id).is_ok()
            {
                return None;
            }
        }
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .ok()?;
        let mut runtime = self.runtime.lock().await;
        if !rotten_essence::current(&runtime, actor, command) {
            return None;
        }
        let index =
            super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)?;
        let item = match intent.target {
            UseTarget::Item(_) => match source? {
                ItemKey::Instance(id) => id,
                _ => return None,
            },
            UseTarget::ItemByDefinition(reference) => {
                let (key, revision) = index.definition_at(reference)?;
                let definition = TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.into(),
                    revision_ref: revision.into(),
                };
                source_use(&definition)?;
                let mut states = self.states.spell_states.lock().await;
                inventory
                    .first_unreserved(
                        &definition,
                        &rotten_essence::gate(
                            &mut states,
                            &runtime,
                            actor,
                            command.game_session_id,
                        )
                        .unspendable,
                    )
                    .ok()?
            }
            _ => return None,
        };
        let entry = inventory
            .backpack
            .as_ref()
            .into_iter()
            .flat_map(|bag| bag.entries.iter())
            .chain(inventory.nested.iter().map(|(_, entry)| entry))
            .find(|entry| entry.item.item_instance_id == item)?
            .clone();
        source_use(&entry.item.definition)?;
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(item))
        {
            return None;
        }
        let selected = if let Some(wanted) = intent.use_with {
            let visible = runtime.visible_entities();
            let mut creature = visible
                .creatures
                .iter()
                .find(|entry| {
                    entry.actor.placement_identity() == wanted.identity
                        && entry.generation == wanted.generation
                })
                .map(|entry| entry.actor);
            if creature.is_none()
                && source_use(&entry.item.definition)
                    == Some(crate::durability::item_use::soul_war_source::SourceUse::Sorrow)
            {
                // The source resolves Creature(id), including HP0 before its
                // copied death handlers finish. This is no combat eligibility.
                creature = runtime
                    .source_registered_creature_census()
                    .ok()?
                    .into_iter()
                    .find(|(actor, _)| {
                        actor.placement_identity() == wanted.identity
                            && actor.actor_local_generation() == wanted.generation
                    })
                    .map(|(actor, _)| actor);
            }
            let selected = creature.or_else(|| {
                visible
                    .players
                    .iter()
                    .find(|entry| {
                        entry.actor.placement_identity() == wanted.identity
                            && entry.generation == wanted.generation
                    })
                    .map(|entry| entry.actor)
            })?;
            let own = runtime.read_actor_position(actor).ok()?.position();
            let target_position = runtime.read_actor_position(selected).ok()?.position();
            if own.floor != target_position.floor
                || own.x.abs_diff(target_position.x) > 1
                || own.y.abs_diff(target_position.y) > 1
            {
                return None;
            }
            if creature.is_some() {
                SelectedSoulWarUseTarget::Creature(selected)
            } else {
                SelectedSoulWarUseTarget::OtherMapEntry(target_position)
            }
        } else if matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_)))
            && matches!(target, Some(ItemKey::Instance(_)))
        {
            let Some(ItemKey::Instance(target_id)) = target else {
                return None;
            };
            let target_entry = inventory
                .backpack
                .as_ref()
                .into_iter()
                .flat_map(|bag| bag.entries.iter())
                .chain(inventory.nested.iter().map(|(_, entry)| entry))
                .find(|entry| entry.item.item_instance_id == target_id)?;
            SelectedSoulWarUseTarget::Item {
                definition: target_entry.item.definition.clone(),
                position: runtime.read_actor_position(actor).ok()?.position(),
            }
        } else if intent.use_with_map_item.is_some() {
            let at = super::super::bundle_map_position(
                &mut runtime,
                self.effective_movement_cells(),
                actor,
            )?;
            let facts = at.map.facts.pinned();
            let map = super::super::world_map::MapViewSource {
                overlay: &at.map.overlay,
                facts: &facts,
                content_generation: at.content_generation,
                reset_epoch: self.reset_epoch,
            };
            let (position, key) = match &intent.use_with_map_item {
                Some(MapUseTarget::Object(object)) => {
                    let resolved =
                        super::super::world_map::resolve_map_target(&map, object, |_| None::<()>)
                            .ok()?;
                    (resolved.pos, MapEntryKey::Base(resolved.placement_key))
                }
                Some(MapUseTarget::Item(_)) => {
                    match super::super::world_map::resolve_map_handle(&map, target?).ok()? {
                        super::super::world_map::MapHandleTarget::Base {
                            pos,
                            placement_key,
                            ..
                        } => (pos, MapEntryKey::Base(placement_key)),
                        _ => return None,
                    }
                }
                None => return None,
            };
            // Crystal actions.cpp:269-280,688-694: these literal Actions
            // never enable far use; floor and one-cell Chebyshev reach apply.
            if at.at.floor != i16::from(position.floor)
                || at.at.x.abs_diff(i32::from(position.x)) > 1
                || at.at.y.abs_diff(i32::from(position.y)) > 1
            {
                return None;
            }
            let tile = view::compose_tile(&at.map.overlay, &facts, position).ok()??;
            let entry = tile
                .entries
                .iter()
                .find(|entry| entry.source.key() == key)?;
            let position = crate::foundation::MovementLocalPosition {
                x: i32::from(position.x),
                y: i32::from(position.y),
                floor: i16::from(position.floor),
            };
            match entry.facts.definition {
                MapDefinition::Item(reference) => SelectedSoulWarUseTarget::Item {
                    definition: super::super::item_look::typed_definition(index, reference)?,
                    position,
                },
                _ => SelectedSoulWarUseTarget::OtherMapEntry(position),
            }
        } else {
            SelectedSoulWarUseTarget::OtherMapEntry(
                runtime.read_actor_position(actor).ok()?.position(),
            )
        };
        let reader = CurrentSelection {
            command,
            actor,
            source: &entry.item.definition,
            selected,
        };
        let qualified = QualifiedSoulWarUseTarget::from_current_owner(
            &runtime,
            native,
            actor,
            command.game_session_id,
            &entry.item.definition,
            intent,
            &reader,
        )?;
        Some((
            entry,
            qualified,
            inventory.content_revision,
            inventory.ruleset_revision,
        ))
    }
}

#[derive(Debug, Clone)]
pub(super) struct PendingSourceUse {
    actor: ExactActorRef,
    command: UseCommand,
    wire: [u8; 32],
    input: crate::durability::quest_state::soul_war_source::SoulWarSourceUse,
    maw: Option<PreparedMawUse>,
    sanity: Option<PreparedSanityUse>,
    sorrow: Option<PreparedSorrowUse>,
    reservation: Option<SourceActorReservation>,
    phase: SourceUsePhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceUsePhase {
    Preparing,
    WriteIssued,
    Completed(UseDisposition),
}
impl PendingSourceUse {
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        let selected = &self.input.target;
        rotten_essence::current(runtime, self.actor, self.command)
            && runtime.binding().runtime_scope() == selected.runtime_scope()
            && runtime.binding().scope_generation() == selected.scope_generation()
            && pin.activation_sequence() == selected.activation_sequence()
            && pin.server_artifact_digest() == selected.server_artifact_digest()
            && pin.client_artifact_digest() == selected.client_artifact_digest()
            && pin.frame_binding_digest() == selected.frame_binding_digest()
            && runtime.map_revision_digest() == selected.map_revision_digest()
    }
}
impl source_reservation_seal::Sealed for PendingSourceUse {}
impl SourceActorReservationProof for PendingSourceUse {
    fn owner(&self) -> (ExactActorRef, GameSessionId) {
        (self.actor, self.command.game_session_id)
    }
    fn event_identity(&self) -> [u8; 16] {
        self.input.item.event_id
    }
    fn source_binding_digest(&self) -> [u8; 32] {
        self.wire
    }
    fn affected_actors(&self) -> &[ExactActorRef] {
        self.maw
            .as_ref()
            .map(PreparedMawUse::affected)
            .or_else(|| self.sanity.as_ref().map(PreparedSanityUse::affected))
            .or_else(|| {
                self.sorrow
                    .as_ref()
                    .map(|prepared| prepared.affected.as_slice())
            })
            .unwrap_or(&[])
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.current_for(runtime)
    }
}
#[cfg(test)]
fn source_use_diagnostic(
    pending: &PendingSourceUse,
    phase: &'static str,
    outcome: &'static str,
    receipt: &'static str,
) {
    use crate::durability::item_use::soul_war_source::SourceUse;
    let family = match pending.input.target.source_use() {
        SourceUse::MortalEssence => "maw",
        SourceUse::CleansedSanity => "sanity",
        SourceUse::Sorrow => "sorrow",
    };
    let state = match pending.phase {
        SourceUsePhase::Preparing => "preparing",
        SourceUsePhase::WriteIssued => "write_issued",
        SourceUsePhase::Completed(_) => "completed",
    };
    eprintln!(
        "PR3056_SOURCE_USE family={family} phase={phase} state={state} outcome={outcome} receipt={receipt}"
    );
}

#[cfg(test)]
fn source_use_durability_error(error: &crate::durability::DurabilityError) -> &'static str {
    use crate::durability::{DurabilityError as D, SqlxFailureClass as S};
    match error {
        D::RootUnavailable => "root_unavailable",
        D::RootPassDeadlineExceeded => "pass_deadline",
        D::CommitOutcomeUnknown => "commit_unknown",
        D::CommitRejected => "commit_rejected",
        D::InvalidStoredState => "invalid_stored_state",
        D::RootTaskFailed => "root_task_failed",
        D::SessionUseAuthorization(_) => "session_use_authorization",
        D::AdmissionGameSessionLedgerExhausted => "session_ledger_exhausted",
        D::Unavailable => "durability_unavailable",
        D::InvalidConfiguration => "invalid_configuration",
        D::SchemaIncompatible(_) => "schema_incompatible",
        D::Migration(_) => "migration",
        D::Database(S::RetryableStatement) => "database_retryable",
        D::Database(S::Database(Some(_))) => "database_registered",
        D::Database(S::Database(None)) => "database_unregistered",
        D::Database(S::Transport) => "database_transport",
        D::Database(S::Tls) => "database_tls",
        D::Database(S::Protocol) => "database_protocol",
        D::Database(S::Data) => "database_data",
        D::Database(S::Pool) => "database_pool",
        D::Database(S::Configuration) => "database_configuration",
        D::Database(S::Runtime) => "database_runtime",
        D::Database(S::Migration) => "database_migration",
        D::Database(S::Transaction) => "database_transaction",
        D::Database(S::Other) => "database_other",
    }
}

#[cfg(test)]
fn source_use_progression_error(
    error: &crate::durability::character_progression::CharacterProgressionError,
) -> &'static str {
    use crate::durability::DurabilityError as D;
    use crate::durability::character_progression::CharacterProgressionError as E;
    match error {
        E::Unavailable(D::RootUnavailable) => "root_unavailable",
        E::Unavailable(D::RootPassDeadlineExceeded) => "pass_deadline",
        E::Unavailable(D::CommitOutcomeUnknown) => "commit_unknown",
        E::Unavailable(D::Database(_)) => "database",
        E::Unavailable(_) => "durability_other",
        E::InvalidInput => "invalid_input",
        E::AuthorityRejected => "authority_rejected",
        E::MissingProgressionState => "missing_progression",
        E::CharacterRevisionMismatch => "character_revision",
        E::ProgressionContextMismatch => "progression_context",
        E::ConflictingOccurrence => "conflicting_occurrence",
        E::RespawnPending => "respawn_pending",
        E::HeldBlessingsMismatch => "held_blessings",
        E::BuildStateMismatch => "build_state",
        E::StanceStateMismatch => "stance_state",
        E::FamiliarStateMismatch => "familiar_state",
        E::Calculation(_) => "calculation",
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn dispatch_soul_war_source_use(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        use crate::durability::quest_state::soul_war_source::SoulWarSourceUse;
        let wire: [u8; 32] = {
            let mut hash = Sha256::new();
            hash.update(b"oteryn:soul-war-source-use-wire:v1");
            hash.update(
                oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent)
                    .map_err(|_| ())?,
            );
            hash.finalize().into()
        };
        let pending = {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let held = rotten_essence::gate(&mut states, &runtime, actor, command.game_session_id);
            if let Some((id, previous, disposition)) = held.source_last
                && id == command.command_id
            {
                return if previous == wire {
                    Ok(Some(UseOutcome {
                        disposition,
                        ..UseOutcome::rejected()
                    }))
                } else {
                    Err(())
                };
            }
            held.source_pending.as_deref().cloned()
        };
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let pending = if let Some(pending) = pending {
            if pending.actor != actor || pending.command != command || pending.wire != wire {
                return Err(());
            }
            pending
        } else {
            let Some((entry, qualified, content_revision, ruleset_revision)) = self
                .qualify_soul_war_source_use(actor, command, intent, source, target)
                .await
            else {
                return Ok(None);
            };
            if matches!(
                qualified.source_use(),
                crate::durability::item_use::soul_war_source::SourceUse::MortalEssence
                    | crate::durability::item_use::soul_war_source::SourceUse::Sorrow
            ) && !qualified.target_accepted()
            {
                return Ok(Some(UseOutcome {
                    disposition: UseDisposition::NotPossible,
                    ..UseOutcome::rejected()
                }));
            }
            let mut pending = PendingSourceUse {
                actor,
                command,
                wire,
                maw: None,
                sanity: None,
                sorrow: None,
                reservation: None,
                phase: SourceUsePhase::Preparing,
                input: SoulWarSourceUse {
                    target: qualified,
                    item: ItemUseRequest {
                        kind: ItemUseKind::SoulWarSourceUse { wire_binding: wire },
                        command_id: command.command_id,
                        content_revision,
                        ruleset_revision,
                        sim_revision: entry_chest::SIM_REVISION.into(),
                        transaction_id: crate::operator_files::uuid_v7().map_err(|_| ())?,
                        event_id: crate::operator_files::uuid_v7().map_err(|_| ())?,
                        occurred_at_unix_ms: unix_ms().map_err(|_| ())?,
                        server_build_id: SERVER_BUILD_ID.into(),
                        source_item_instance_id: entry.item.item_instance_id,
                        source_definition: entry.item.definition,
                        source_quantity_before: entry.item.quantity,
                        flask: ItemUseFlaskPlan::None,
                    },
                },
            };
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if !rotten_essence::current(&runtime, actor, command)
                || states.is_dead(actor)
                || states.has_pending_spell_commit(actor, command.game_session_id)
            {
                return Err(());
            }
            let mut sorrow_reservation = None;
            if pending.input.target.source_use()
                == crate::durability::item_use::soul_war_source::SourceUse::Sorrow
            {
                let native = self
                    .active_generation
                    .and_then(|generation| generation.native_gameplay())
                    .ok_or(())?;
                let (target, _, at) = pending.input.target.creature_target().ok_or(())?;
                let Ok((before, after)) = states.soul_war_callbacks.prepare_sorrow_use(
                    &runtime,
                    native,
                    &pending.input.target,
                ) else {
                    return Ok(Some(UseOutcome {
                        disposition: UseDisposition::NotPossible,
                        ..UseOutcome::rejected()
                    }));
                };
                pending.sorrow = Some(PreparedSorrowUse {
                    before,
                    after,
                    affected: [target],
                    at,
                });
                sorrow_reservation = Some(
                    runtime
                        .prepare_source_actor_reservation(&pending)
                        .map_err(|_| ())?,
                );
            }
            let held = rotten_essence::gate(&mut states, &runtime, actor, command.game_session_id);
            held.admit(command.command_id, self.owner_ms())
                .map_err(|_| ())?;
            if held
                .unspendable
                .contains(&pending.input.item.source_item_instance_id)
            {
                return Err(());
            }
            // The shared one-write-in-flight gate starts without inventing a
            // gameplay cooldown; the source thirty-second clock is durable.
            held.start_drink(self.owner_ms()).map_err(|_| ())?;
            held.unspendable
                .push(pending.input.item.source_item_instance_id);
            if let Some(mut reservation) = sorrow_reservation {
                runtime
                    .reserve_source_actors(&mut reservation, &pending)
                    .map_err(|_| ())?;
                pending.reservation = Some(reservation);
                states
                    .soul_war_callbacks
                    .retain_sorrow_use(pending.input.item.event_id);
            }
            // Sorrow's exact target, clock successor and source reservation are
            // installed in this same owner turn, before any cancellation point.
            rotten_essence::gate(&mut states, &runtime, actor, command.game_session_id)
                .source_pending = Some(Box::new(pending.clone()));
            pending
        };
        self.settle_soul_war_source_use(&permit, pending)
            .await
            .map(Some)
    }

    /// Continue the original source action before another native owner turn.
    /// This also settles a cancelled client future without requiring a new USE.
    pub(in crate::gameplay_transport) async fn drain_retained_soul_war_uses(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> Result<(), ()> {
        let pending = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            let states = self.states.spell_states.lock().await;
            let pending = states
                .item_uses
                .0
                .iter()
                .filter_map(|(_, _, gate)| gate.source_pending.as_deref().cloned())
                .collect::<Vec<_>>();
            if pending.len() > runtime.actor_slot_capacity() {
                return Err(());
            }
            pending
        };
        for pending in pending {
            self.settle_soul_war_source_use(permit, pending).await?;
        }
        Ok(())
    }

    /// The last disconnected player has no connected native tick. Lifecycle
    /// recovery therefore settles only this actor's original retained USE.
    pub(in crate::gameplay_transport) async fn recover_pending_soul_war_source_use(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let pending = {
            let states = self.states.spell_states.lock().await;
            states
                .item_uses
                .0
                .iter()
                .find(|(a, s, _)| *a == actor && *s == session)
                .and_then(|(_, _, gate)| gate.source_pending.as_deref())
                .cloned()
        };
        let Some(pending) = pending else {
            return true;
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return false;
        };
        self.settle_soul_war_source_use(&permit, pending)
            .await
            .is_ok()
    }

    fn finish_soul_war_source_use(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        pending: &PendingSourceUse,
        disposition: UseDisposition,
    ) -> Result<UseOutcome, ()> {
        let held = rotten_essence::gate(
            states,
            runtime,
            pending.actor,
            pending.command.game_session_id,
        );
        let retained = held.source_pending.as_mut().ok_or(())?;
        if retained.actor != pending.actor
            || retained.command != pending.command
            || retained.wire != pending.wire
            || matches!(retained.phase, SourceUsePhase::Completed(previous) if previous != disposition)
        {
            return Err(());
        }
        // Physical publication has already finished (or no write was issued).
        // Keep that fact if ingress cannot take the original result, so retry
        // never repeats the volatile effect or fabricates a second command.
        retained.phase = SourceUsePhase::Completed(disposition);
        #[cfg(test)]
        source_use_diagnostic(retained, "handoff", "entered", "unobserved");
        let reference = CommandRef::new(
            pending.command.game_session_id,
            CommandId::new(pending.command.command_id).map_err(|_| ())?,
        );
        #[cfg(test)]
        if test_pause::refuse_handoff(reference) {
            source_use_diagnostic(retained, "handoff", "injected_refusal", "unobserved");
            return Err(());
        }
        let handoff = self
            .states
            .command_ingress
            .complete_item_use(reference, disposition);
        #[cfg(test)]
        source_use_diagnostic(
            retained,
            "handoff",
            if handoff.is_some() {
                "published"
            } else {
                "unavailable"
            },
            "unobserved",
        );
        handoff.ok_or(())?;
        // Handoff and marker removal are one synchronous owner turn.
        held.source_pending = None;
        held.source_last = Some((pending.command.command_id, pending.wire, disposition));
        held.unspendable
            .retain(|id| *id != pending.input.item.source_item_instance_id);
        held.settle(pending.command.command_id, disposition, &[]);
        Ok(UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        })
    }

    async fn start_soul_war_source_write(
        &self,
        pending: &mut PendingSourceUse,
    ) -> Result<Option<UseOutcome>, ()> {
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if pending.phase == SourceUsePhase::Preparing && !pending.current_for(&runtime) {
            if pending.reservation.is_some() {
                return Err(());
            }
            return self
                .finish_soul_war_source_use(
                    &runtime,
                    &mut states,
                    pending,
                    UseDisposition::NotPossible,
                )
                .map(Some);
        }
        let held = rotten_essence::gate(
            &mut states,
            &runtime,
            pending.actor,
            pending.command.game_session_id,
        );
        let retained = held.source_pending.as_mut().ok_or(())?;
        if retained.actor != pending.actor
            || retained.command != pending.command
            || retained.wire != pending.wire
            || retained.phase != pending.phase
        {
            return Err(());
        }
        // Retain before the first await that can issue this exact transaction.
        // Cancellation from here is ambiguous until the durable owner answers.
        retained.phase = SourceUsePhase::WriteIssued;
        pending.phase = SourceUsePhase::WriteIssued;
        Ok(None)
    }

    async fn settle_soul_war_source_use(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        pending: PendingSourceUse,
    ) -> Result<UseOutcome, ()> {
        use crate::durability::quest_state::{
            QuestTransitionOutcome, soul_war_source::SoulWarSourceAction,
        };
        let actor = pending.actor;
        let command = pending.command;
        let wire = pending.wire;
        // A waiting client and the autonomous owner can observe the same gate.
        // After acquiring the lane, reuse the retained state or its settled memo.
        let mut pending = {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let gate = rotten_essence::gate(&mut states, &runtime, actor, command.game_session_id);
            if let Some((id, previous, disposition)) = gate.source_last
                && id == command.command_id
            {
                return if previous == wire {
                    Ok(UseOutcome {
                        disposition,
                        ..UseOutcome::rejected()
                    })
                } else {
                    Err(())
                };
            }
            let retained = gate.source_pending.as_deref().ok_or(())?;
            if retained.actor != actor || retained.command != command || retained.wire != wire {
                return Err(());
            }
            let retained = retained.clone();
            #[cfg(test)]
            source_use_diagnostic(&retained, "settle_entry", "selected", "unobserved");
            if let SourceUsePhase::Completed(disposition) = retained.phase {
                return self.finish_soul_war_source_use(
                    &runtime,
                    &mut states,
                    &retained,
                    disposition,
                );
            }
            if retained.phase == SourceUsePhase::Preparing && !retained.current_for(&runtime) {
                #[cfg(test)]
                source_use_diagnostic(&retained, "prewrite_current", "not_current", "unobserved");
                if retained.reservation.is_some() {
                    return Err(());
                }
                return self.finish_soul_war_source_use(
                    &runtime,
                    &mut states,
                    &retained,
                    UseDisposition::NotPossible,
                );
            }
            retained
        };
        if pending.input.target.source_use()
            == crate::durability::item_use::soul_war_source::SourceUse::Sorrow
        {
            return self.settle_sorrow_use(permit, pending).await;
        }
        if pending.input.target.source_use()
            == crate::durability::item_use::soul_war_source::SourceUse::MortalEssence
        {
            if pending.maw.is_none() {
                if pending.phase != SourceUsePhase::Preparing {
                    return Err(());
                }
                let plan = {
                    let runtime = self.runtime.lock().await;
                    if !pending.current_for(&runtime) {
                        return Err(());
                    }
                    self.prepare_source_names(&runtime, &["Goshnar's Cruelty"])
                        .inspect_err(|_| {
                            #[cfg(test)]
                            source_use_diagnostic(
                                &pending,
                                "names_plan",
                                "unavailable",
                                "unobserved",
                            );
                        })?
                };
                #[cfg(test)]
                test_pause::at(test_pause::Point::BeforeNames, command).await;
                let read = self
                    .read_source_names(permit, &plan)
                    .await
                    .inspect_err(|_| {
                        #[cfg(test)]
                        source_use_diagnostic(&pending, "names_read", "unavailable", "unobserved");
                    })?;
                let mut runtime = self.runtime.lock().await;
                let mut states = self.states.spell_states.lock().await;
                if !pending.current_for(&runtime) {
                    return Err(());
                }
                let names = self.seal_source_names(&runtime, read).inspect_err(|_| {
                    #[cfg(test)]
                    source_use_diagnostic(&pending, "names_seal", "not_current", "unobserved");
                })?;
                let Ok(named) = crate::gameplay_transport::source_names::creature_named(
                    &names,
                    "Goshnar's Cruelty",
                ) else {
                    return self.finish_soul_war_source_use(
                        &runtime,
                        &mut states,
                        &pending,
                        UseDisposition::NotPossible,
                    );
                };
                let native = self
                    .active_generation
                    .and_then(|active| active.native_gameplay())
                    .ok_or(())?;
                let Ok(prepared) = states.soul_war_callbacks.maw.prepare(
                    &runtime,
                    native,
                    pending.input.item.event_id,
                    named,
                ) else {
                    return self.finish_soul_war_source_use(
                        &runtime,
                        &mut states,
                        &pending,
                        UseDisposition::NotPossible,
                    );
                };
                pending.maw = Some(prepared);
                let mut reservation = runtime
                    .prepare_source_actor_reservation(&pending)
                    .map_err(|_| ())?;
                runtime
                    .reserve_source_actors(&mut reservation, &pending)
                    .map_err(|_| ())?;
                pending.reservation = Some(reservation);
                // No await after slot reservation until the complete original
                // effect and clock fence are retained beside the Item gate.
                states
                    .soul_war_callbacks
                    .maw
                    .retain(pending.maw.as_ref().ok_or(())?);
                rotten_essence::gate(&mut states, &runtime, actor, command.game_session_id)
                    .source_pending = Some(Box::new(pending.clone()));
            } else {
                let runtime = self.runtime.lock().await;
                runtime
                    .validate_source_actor_reservation(
                        pending.reservation.as_ref().ok_or(())?,
                        &pending,
                    )
                    .map_err(|_| ())?;
            }
        }
        if pending.input.target.source_use()
            == crate::durability::item_use::soul_war_source::SourceUse::CleansedSanity
            && pending.input.target.target_accepted()
        {
            if pending.sanity.is_none() {
                if pending.phase != SourceUsePhase::Preparing {
                    return Err(());
                }
                let plan = {
                    let runtime = self.runtime.lock().await;
                    if !pending.current_for(&runtime) {
                        return Err(());
                    }
                    self.prepare_source_names(&runtime, &["Goshnar's Megalomania"])
                        .inspect_err(|_| {
                            #[cfg(test)]
                            source_use_diagnostic(
                                &pending,
                                "names_plan",
                                "unavailable",
                                "unobserved",
                            );
                        })?
                };
                #[cfg(test)]
                test_pause::at(test_pause::Point::BeforeNames, command).await;
                let read = self
                    .read_source_names(permit, &plan)
                    .await
                    .inspect_err(|_| {
                        #[cfg(test)]
                        source_use_diagnostic(&pending, "names_read", "unavailable", "unobserved");
                    })?;
                let mut runtime = self.runtime.lock().await;
                let mut states = self.states.spell_states.lock().await;
                if !pending.current_for(&runtime) {
                    return Err(());
                }
                let names = self.seal_source_names(&runtime, read).inspect_err(|_| {
                    #[cfg(test)]
                    source_use_diagnostic(&pending, "names_seal", "not_current", "unobserved");
                })?;
                let Ok(named) = crate::gameplay_transport::source_names::creature_named(
                    &names,
                    "Goshnar's Megalomania",
                ) else {
                    return self.finish_soul_war_source_use(
                        &runtime,
                        &mut states,
                        &pending,
                        UseDisposition::NotPossible,
                    );
                };
                let native = self
                    .active_generation
                    .and_then(|active| active.native_gameplay())
                    .ok_or(())?;
                let Ok(prepared) = states.soul_war_callbacks.sanity.prepare(
                    &runtime,
                    native,
                    pending.input.item.event_id,
                    named,
                ) else {
                    return self.finish_soul_war_source_use(
                        &runtime,
                        &mut states,
                        &pending,
                        UseDisposition::NotPossible,
                    );
                };
                pending.sanity = Some(prepared);
                let mut reservation = runtime
                    .prepare_source_actor_reservation(&pending)
                    .map_err(|_| ())?;
                runtime
                    .reserve_source_actors(&mut reservation, &pending)
                    .map_err(|_| ())?;
                pending.reservation = Some(reservation);
                // No await after slot reservation until the complete original
                // effect and clock fence are retained beside the Item gate.
                states
                    .soul_war_callbacks
                    .sanity
                    .retain(pending.sanity.as_ref().ok_or(())?);
                rotten_essence::gate(&mut states, &runtime, actor, command.game_session_id)
                    .source_pending = Some(Box::new(pending.clone()));
            } else {
                let runtime = self.runtime.lock().await;
                runtime
                    .validate_source_actor_reservation(
                        pending.reservation.as_ref().ok_or(())?,
                        &pending,
                    )
                    .map_err(|_| ())?;
            }
        }
        // Every issued source USE, including wrong-target Sanity with no
        // volatile preparation, reconciles its immutable receipt before fresh
        // gameplay authority. The sole writer still fences any missing receipt.
        let original = pending.command.item_fence.ok_or(())?;
        let fence = crate::durability::character_progression::CurrentCharacterGameplayFence {
            character_id: original.character_id,
            game_session_id: original.game_session_id,
            connection_generation: original.connection_generation,
            character_lease_generation: original.character_lease_generation,
            runtime_scope: original.runtime_scope,
            scope_ownership_generation: original.scope_ownership_generation,
            // The existing revision sequencer supplies its canonical cursor.
            expected_character_revision: crate::domain::CharacterRevision::new(1)
                .map_err(|_| ())?,
        };
        let catalogue = self.quest_catalogue.as_ref().ok_or(())?;
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        #[cfg(test)]
        test_pause::at(test_pause::Point::BeforeWrite, command).await;
        if let Some(outcome) = self
            .start_soul_war_source_write(&mut pending)
            .await
            .inspect_err(|_| {
                #[cfg(test)]
                source_use_diagnostic(&pending, "start_write", "unavailable", "unobserved");
            })?
        {
            return Ok(outcome);
        }
        #[cfg(test)]
        source_use_diagnostic(&pending, "quest_writer", "issued", "unobserved");
        let result = slot
            .commit_soul_war_source_action(
                self.root,
                self.character,
                self.holder,
                fence,
                SoulWarSourceAction::Use(pending.input.clone()),
                std::sync::Arc::clone(catalogue),
                None,
            )
            .await;
        #[cfg(test)]
        test_pause::at(test_pause::Point::AfterWrite, command).await;
        drop(slot);
        #[cfg(test)]
        {
            let (outcome, receipt) = match &result {
                Ok(QuestTransitionOutcome::Committed(_)) => ("committed", "present"),
                Ok(QuestTransitionOutcome::AlreadyCommitted(_)) => ("already_committed", "present"),
                Ok(QuestTransitionOutcome::Refused(_)) => ("refused", "unobserved"),
                Ok(QuestTransitionOutcome::ObligationClosed) => ("obligation_closed", "unobserved"),
                Err(error) => (source_use_progression_error(error), "unobserved"),
            };
            source_use_diagnostic(&pending, "quest_writer", outcome, receipt);
        }
        let committed = match &result {
            Ok(
                QuestTransitionOutcome::Committed(receipt)
                | QuestTransitionOutcome::AlreadyCommitted(receipt),
            ) => Some(receipt),
            _ => None,
        };
        let known_transition = committed.is_some();
        let mut disposition = match &result {
            Ok(
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_),
            ) => UseDisposition::Committed,
            Ok(QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed) => {
                UseDisposition::NotPossible
            }
            Err(
                crate::durability::character_progression::CharacterProgressionError::Unavailable(_),
            ) => return Err(()),
            Err(_) => UseDisposition::Rejected,
        };
        if known_transition
            && pending.input.target.source_use()
                == crate::durability::item_use::soul_war_source::SourceUse::CleansedSanity
            && !pending.input.target.target_accepted()
        {
            // Lua stores the cooldown before testing the target, then returns
            // false. A known Quest commit is not an action success or Item burn.
            disposition = UseDisposition::NotPossible;
        }
        {
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if let Some(prepared) = &pending.maw {
                let second = if let Some(receipt) = committed {
                    use oteryn_gameplay_quests::quest::soul_war::source_items as rule;
                    let request = pending.input.request(&fence).map_err(|_| ())?;
                    let [change] = receipt.changes.as_slice() else {
                        return Err(());
                    };
                    if receipt.character_id != fence.character_id
                        || receipt.cause != request.cause
                        || receipt.transition_key != request.transition_key
                        || receipt.quest_key != rule::QUEST
                        || receipt.completes
                        || receipt.experience.is_some()
                        || change.track != rule::MAW
                        || change.after < change.before.checked_add(30).ok_or(())?
                    {
                        return Err(());
                    }
                    Some(u64::try_from(change.after).map_err(|_| ())?)
                } else {
                    None
                };
                let next = states
                    .soul_war_callbacks
                    .maw
                    .completed_clock(prepared, second)?;
                let target = if second.is_some() {
                    prepared.target()
                } else {
                    None
                };
                runtime
                    .complete_soul_war_cruelty_maw(
                        pending.reservation.clone().ok_or(())?,
                        &pending,
                        target,
                    )
                    .map_err(|_| ())?;
                states.soul_war_callbacks.maw.publish(next);
                if known_transition {
                    // Crystal utils_definitions.hpp: DRAWBLOOD=1. The real
                    // target resolver retained this original map position.
                    if let Some(at) = pending.input.target.item_target_position()
                        && let (Ok(x), Ok(y), Ok(floor)) = (
                            u16::try_from(at.x),
                            u16::try_from(at.y),
                            i8::try_from(at.floor),
                        )
                        && let Some(map) = self.bundle_map()
                    {
                        map.facts
                            .emit_movement_effect(crate::map::overlay::TilePos { x, y, floor }, 1);
                    }
                    self.states.server_messages.send_current(
                        command.game_session_id,
                        oteryn_protocol_oteryn::zones_nologout::ServerMessage::Advance(
                            "Use the item again within 15 seconds, or the monster's defense will increase by 2 every 15 seconds.".into(),
                        ),
                    );
                }
                if known_transition && prepared.missing_type() {
                    disposition = UseDisposition::NotPossible;
                }
            }
            if let Some(prepared) = &pending.sanity {
                let second = if let Some(receipt) = committed {
                    use oteryn_gameplay_quests::quest::soul_war::source_items as rule;
                    let request = pending.input.request(&fence).map_err(|_| ())?;
                    let [change] = receipt.changes.as_slice() else {
                        return Err(());
                    };
                    if receipt.character_id != fence.character_id
                        || receipt.cause != request.cause
                        || receipt.transition_key != request.transition_key
                        || receipt.quest_key != rule::QUEST
                        || receipt.completes
                        || receipt.experience.is_some()
                        || change.track != rule::SANITY
                        || change.after < change.before.checked_add(30).ok_or(())?
                    {
                        return Err(());
                    }
                    Some(u64::try_from(change.after).map_err(|_| ())?)
                } else {
                    None
                };
                let next = states
                    .soul_war_callbacks
                    .sanity
                    .completed_clock(prepared, second)?;
                let target = if second.is_some() {
                    prepared.target()
                } else {
                    None
                };
                runtime
                    .complete_soul_war_megalomania_sanity(
                        pending.reservation.clone().ok_or(())?,
                        &pending,
                        target,
                    )
                    .map_err(|_| ())?;
                states.soul_war_callbacks.sanity.publish(next);
                if known_transition {
                    // Crystal utils_definitions.hpp: DRAWBLOOD=1. The real
                    // target resolver retained this original map position.
                    if let Some(at) = pending.input.target.item_target_position()
                        && let (Ok(x), Ok(y), Ok(floor)) = (
                            u16::try_from(at.x),
                            u16::try_from(at.y),
                            i8::try_from(at.floor),
                        )
                        && let Some(map) = self.bundle_map()
                    {
                        map.facts
                            .emit_movement_effect(crate::map::overlay::TilePos { x, y, floor }, 1);
                    }
                    self.states.server_messages.send_current(
                        command.game_session_id,
                        oteryn_protocol_oteryn::zones_nologout::ServerMessage::Advance(
                            "Use the item again within 15 seconds, or the monster's defense will increase every 15 seconds.".into(),
                        ),
                    );
                }
                if known_transition && prepared.missing_type() {
                    disposition = UseDisposition::NotPossible;
                }
            }
            self.finish_soul_war_source_use(&runtime, &mut states, &pending, disposition)?;
        }
        if known_transition {
            Box::pin(self.refresh_quest_session(command.game_session_id, false)).await;
        }
        Ok(UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        })
    }

    async fn settle_sorrow_use(
        &self,
        _permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        mut pending: PendingSourceUse,
    ) -> Result<UseOutcome, ()> {
        use crate::durability::item_use::{ItemUseError, ItemUseOutcome};
        let actor = pending.actor;
        let command = pending.command;
        {
            let runtime = self.runtime.lock().await;
            runtime
                .validate_source_actor_reservation(
                    pending.reservation.as_ref().ok_or(())?,
                    &pending,
                )
                .map_err(|_| ())?;
            let states = self.states.spell_states.lock().await;
            states.soul_war_callbacks.validate_sorrow_use(
                pending.input.item.event_id,
                &pending.sorrow.as_ref().ok_or(())?.before,
            )?;
        }
        #[cfg(test)]
        test_pause::at(test_pause::Point::BeforeWrite, command).await;
        if let Some(outcome) = self
            .start_soul_war_source_write(&mut pending)
            .await
            .inspect_err(|_| {
                #[cfg(test)]
                source_use_diagnostic(&pending, "start_write", "unavailable", "unobserved");
            })?
        {
            return Ok(outcome);
        }
        #[cfg(test)]
        source_use_diagnostic(&pending, "sorrow_writer", "issued", "unobserved");
        let result = self
            .root
            .commit_soul_war_sorrow_use(
                self.character,
                self.holder,
                command.item_fence.ok_or(())?,
                pending.input.item.clone(),
                pending.input.target.clone(),
            )
            .await;
        #[cfg(test)]
        test_pause::at(test_pause::Point::AfterWrite, command).await;
        #[cfg(test)]
        {
            let (outcome, receipt) = match &result {
                Ok(ItemUseOutcome::Written { .. }) => ("written", "present"),
                Ok(ItemUseOutcome::AlreadyCommitted { .. }) => ("already_committed", "present"),
                Err(ItemUseError::Unavailable(error)) => {
                    (source_use_durability_error(error), "unobserved")
                }
                Err(ItemUseError::StaleState) => ("stale_state", "unobserved"),
                Err(ItemUseError::AuthorityRejected) => ("authority_rejected", "unobserved"),
                Err(ItemUseError::Cooldown) => ("cooldown", "unobserved"),
                Err(_) => ("item_refusal", "unobserved"),
            };
            source_use_diagnostic(&pending, "sorrow_writer", outcome, receipt);
        }
        let (committed, disposition) = match result {
            Ok(ItemUseOutcome::Written { .. } | ItemUseOutcome::AlreadyCommitted { .. }) => {
                (true, UseDisposition::Committed)
            }
            Err(ItemUseError::Unavailable(_)) => return Err(()),
            Err(
                ItemUseError::StaleState | ItemUseError::AuthorityRejected | ItemUseError::Cooldown,
            ) => (false, UseDisposition::NotPossible),
            Err(_) => (false, UseDisposition::Rejected),
        };
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let prepared = pending.sorrow.as_ref().ok_or(())?;
        runtime
            .validate_source_actor_reservation(pending.reservation.as_ref().ok_or(())?, &pending)
            .map_err(|_| ())?;
        states
            .soul_war_callbacks
            .validate_sorrow_use(pending.input.item.event_id, &prepared.before)?;
        runtime
            .release_source_actors_after_observed_outcome(
                pending.reservation.clone().ok_or(())?,
                &pending,
            )
            .map_err(|_| ())?;
        states.soul_war_callbacks.publish_sorrow_use(
            pending.input.item.event_id,
            &prepared.before,
            committed.then_some(&prepared.after),
        )?;
        if committed
            && let Some(map) = self.bundle_map()
            && let Some(position) = super::super::soul_war_scene::wire_position(prepared.at)
        {
            let captured_name = self
                .states
                .chat
                .speaker_name(command.game_session_id)
                .and_then(|name| {
                    oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(&name).ok()
                });
            map.facts
                .emit_movement_speech(crate::map::facts::MovementSpeech {
                    speaker: actor,
                    position,
                    talk_type: 36,
                    text: "The flame of hatred is doused!".into(),
                    captured_name,
                });
        }
        self.finish_soul_war_source_use(&runtime, &mut states, &pending, disposition)
    }
}

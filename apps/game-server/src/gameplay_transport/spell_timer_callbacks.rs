//! Delayed source callbacks read fresh real caster owners before the due owner turn.
use super::super::ComposedFreshAdmission;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{GameSessionState, RuntimeScopeRefV1};
use crate::spell::delayed_execution::{Error, FireReport, NativeAiTimerSource, TimerPayload};
use crate::spell::ordinary_timer::OrdinaryCastBinding;
use crate::spell::{Execution, SpellBook};

#[derive(Clone)]
enum DueSource {
    Native(NativeAiTimerSource),
    Ordinary(OrdinaryCastBinding),
}
impl DueSource {
    fn caster(&self) -> crate::foundation::ExactActorRef {
        match self {
            Self::Native(s) => s.caster,
            Self::Ordinary(s) => s.caster,
        }
    }
    fn attacker(&self) -> crate::foundation::CharacterId {
        match self {
            Self::Native(s) => s.attacker,
            Self::Ordinary(s) => s.attacker,
        }
    }
    fn command(&self) -> crate::foundation::CommandRef {
        match self {
            Self::Native(s) => s.command,
            Self::Ordinary(s) => s.command,
        }
    }
    fn active(&self, book: &SpellBook, runtime: &crate::foundation::ChannelRuntimeV1) -> bool {
        match self {
            Self::Native(s) => active_profile(book, s),
            Self::Ordinary(s) => s.active(book, runtime),
        }
    }
    fn same_source(&self, other: &Self) -> bool {
        if self.caster() != other.caster()
            || self.attacker() != other.attacker()
            || self.command() != other.command()
        {
            return false;
        }
        match (self, other) {
            (Self::Native(a), Self::Native(b)) => a.spell.spell() == b.spell.spell(),
            (Self::Ordinary(a), Self::Ordinary(b)) => {
                a.source == b.source
                    && a.content_digest == b.content_digest
                    && a.parent_binding == b.parent_binding
            }
            _ => false,
        }
    }
    fn matches_payload(&self, payload: &TimerPayload) -> bool {
        let binding = payload.binding();
        if self.caster() != binding.caster
            || self.attacker() != binding.attacker
            || self.command() != binding.command
        {
            return false;
        }
        match (self, payload) {
            (Self::Native(source), TimerPayload::NativeOwnerEffect(saved)) => {
                source.spell.spell() == saved.binding.spell.spell()
            }
            (Self::Ordinary(source), TimerPayload::OrdinaryCombat(saved)) => {
                source.source == saved.binding.source
                    && source.content_digest == saved.binding.content_digest
                    && source.parent_binding == saved.binding.parent_binding
            }
            _ => false,
        }
    }
}
struct QualifiedCaster {
    source: DueSource,
    lease: u64,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
}
fn active_profile(book: &SpellBook, source: &NativeAiTimerSource) -> bool {
    let mut index = 1u32;
    while let Some(number) = std::num::NonZeroU32::new(index) {
        let Some((spell, active)) = book.source_indexed(number) else {
            return false;
        };
        if active
            && matches!(&spell.execution,Execution::NativeProfile(profile) if profile.spell()==source.spell.spell())
        {
            return true;
        }
        let Some(next) = index.checked_add(1) else {
            return false;
        };
        index = next;
    }
    false
}
/// Constructed only from this Channel's actual committed timer lane while its
/// physical/state locks are held. No client or saved value can construct SQL authority.
struct DueEvidence<'a> {
    runtime: &'a crate::foundation::ChannelRuntimeV1,
    timer: &'a crate::spell::delayed_execution::SpellTimerOwner,
    saved: &'a crate::spell::ordinary_timer::SavedOrdinaryCombat,
    lease: u64,
    now: crate::foundation::owner_timer::SemanticTimeMicros,
    spell: crate::durability::item_mint::TypedDefinitionRef,
}
impl crate::durability::spell_item_transaction::due_read_seal::Sealed for DueEvidence<'_> {}
impl crate::durability::spell_item_transaction::CommittedSpellDueEvidence for DueEvidence<'_> {
    fn command(&self) -> crate::foundation::CommandRef {
        self.saved.binding.command
    }
    fn caster_placement_identity(&self) -> [u8; 16] {
        self.saved.binding.caster.placement_identity()
    }
    fn spell(&self) -> &crate::durability::item_mint::TypedDefinitionRef {
        &self.spell
    }
    fn content_digest(&self) -> [u8; 32] {
        self.saved.binding.content_digest
    }
    fn verify_current_retained_source(&self) -> Result<(), &'static str> {
        if !self
            .timer
            .contains_due_ordinary_source(self.runtime, self.saved, self.now)
            || !self.saved.current_target(self.runtime)
        {
            return Err("current committed timer source missing");
        }
        match self.runtime.retained_spell_batch(
            self.saved.binding.caster,
            self.saved.binding.attacker,
            self.lease,
            self.saved.binding.command,
        ) {
            Ok(Some(original)) if original.binding == self.saved.binding.parent_binding => Ok(()),
            // The bounded physical history may have evicted this original command.
            // The actual installed lane and independently verified committed SQL
            // cost receipt remain required; this branch never issues authority alone.
            Err(crate::spell::combat_batch::Error::StaleCommand) => Ok(()),
            _ => Err("original physical source history conflicts"),
        }
    }
}
fn validate_numeric_predecessor(
    runtime: &crate::foundation::ChannelRuntimeV1,
    states: &super::ChannelSpellStates,
    saved: &crate::spell::ordinary_timer::SavedOrdinaryCombat,
) -> Result<(), Error> {
    let resolved = saved.resolved.as_ref().ok_or(Error::InvalidPayload)?;
    let state = states
        .get(
            runtime,
            saved.binding.caster,
            saved.binding.command.game_session_id(),
        )
        .ok_or(Error::StaleOwner)?;
    resolved
        .magnitude
        .validate_current_with_lookup(
            runtime,
            state,
            resolved.owned.magnitude().ok_or(Error::InvalidPayload)?,
            &|actor| {
                states
                    .actors
                    .iter()
                    .find(|(exact, session, _)| {
                        *exact == actor && states.get(runtime, *exact, *session).is_some()
                    })
                    .map(|(_, _, state)| state)
            },
        )
        .map_err(|_| Error::StaleOwner)
}
fn apply_due_under_current_owners(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    states: &mut super::ChannelSpellStates,
    book: &SpellBook,
    casters: &[QualifiedCaster],
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) -> Result<FireReport, Error> {
    // Both physical/state locks stayed held across every fenced SQL owner read.
    // Each due callback independently compares the exact active source again.
    let proofs = states
        .spell_timers
        .as_ref()
        .map(|timer| {
            timer.due_ordinary_payloads(
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
            )
        })
        .unwrap_or_default();
    // Stale exact targets are cancelled by the current lane below. Their visual
    // holds have no applied physical receipt and are released in this same turn.
    for saved in proofs.iter().filter(|saved| !saved.current_target(runtime)) {
        if let (Some(resolved), Some(outbox)) = (&saved.resolved, states.presentations.as_mut()) {
            outbox.release_definitely_uncommitted(&resolved.presentation);
        }
    }
    let report = states.drain_spell_timers(
        runtime,
        now,
        |runtime, states, payload, occurrence, due, _| {
            let binding = payload.binding();
            let current = casters
                .iter()
                .find(|c| c.source.matches_payload(payload))
                .ok_or(Error::StaleOwner)?;
            if !current.source.active(book, runtime)
                || runtime
                    .player_control_facts(binding.caster, binding.command.game_session_id())
                    .is_err()
                || states
                    .get(runtime, binding.caster, binding.command.game_session_id())
                    .is_none()
            {
                return Err(Error::StaleOwner);
            }
            let effect = match payload {
                TimerPayload::NativeOwnerEffect(saved) => saved.at_due(due)?,
                TimerPayload::OrdinaryCombat(saved) => {
                    let deadline = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                        saved
                            .binding
                            .cast_at
                            .get()
                            .checked_add(saved.delay_ms.checked_mul(1000).ok_or(Error::Bounds)?)
                            .ok_or(Error::Bounds)?,
                    );
                    saved.validate(occurrence, deadline)?;
                    if due < deadline || !saved.current_target(runtime) {
                        return Err(Error::StaleOwner);
                    }
                    validate_numeric_predecessor(runtime, states, saved)?;
                    let resolved = saved.resolved.as_ref().ok_or(Error::InvalidPayload)?;
                    if resolved.batch.current_lease_generation != current.lease {
                        return Err(Error::StaleOwner);
                    }
                    states
                        .presentations
                        .as_ref()
                        .ok_or(Error::InvalidPayload)?
                        .validate_prepared(runtime, &resolved.presentation, &resolved.batch)
                        .map_err(|_| Error::StaleOwner)?;
                    return Ok(resolved.batch.clone());
                }
                _ => return Err(Error::InvalidPayload),
            };
            Ok(crate::spell::combat_batch::OwnerCombatBatch {
                caster: binding.caster,
                attacker: binding.attacker,
                current_lease_generation: current.lease,
                command: binding.command,
                occurrence: (*binding.occurrence).clone().into(),
                binding: binding.parent_binding.clone(),
                anchor: None,
                now_ms: due.get() / 1000,
                effects: vec![effect],
                deferred: None,
            })
        },
    )?;
    for receipt in &report.receipts {
        if let Some(resolved) = proofs
            .iter()
            .find(|saved| {
                saved.binding.command == receipt.occurrence.command
                    && saved.phase == receipt.occurrence.phase
            })
            .and_then(|saved| saved.resolved.as_ref())
        {
            states
                .presentations
                .as_mut()
                .ok_or(Error::InvalidPayload)?
                .install_preflighted((*resolved.presentation).clone(), &receipt.batch);
        }
    }
    Ok(report)
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// The one real timer lane consumes callbacks exactly once. Native and ordinary
    /// sources remain distinct; no saved timer value provides live lease authority.
    pub(in crate::gameplay_transport) async fn drain_native_ai_timers(
        &self,
    ) -> Result<FireReport, Error> {
        let now = self.owner_now();
        // Refresh external access before taking the Channel owner locks: refresh
        // itself reads those owners and must never recursively acquire the turn.
        let ordinary = {
            let states = self.spell_states.lock().await;
            states
                .spell_timers
                .as_ref()
                .map(|t| {
                    t.due_ordinary_payloads(
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
                    )
                })
                .unwrap_or_default()
        };
        let mut access = Vec::new();
        for saved in &ordinary {
            if !access.iter().any(|(actor, session, _)| {
                *actor == saved.binding.caster
                    && *session == saved.binding.command.game_session_id()
            }) {
                access.push((
                    saved.binding.caster,
                    saved.binding.command.game_session_id(),
                    self.refresh_spell_access(
                        saved.binding.caster,
                        saved.binding.command.game_session_id(),
                    )
                    .await,
                ));
            }
        }
        let mut runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        let Some(timer) = states.spell_timers.as_ref() else {
            return Ok(FireReport::default());
        };
        let at = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get());
        let proposals = timer
            .due_native_ai_sources(at)
            .into_iter()
            .map(DueSource::Native)
            .chain(
                timer
                    .due_ordinary_payloads(at)
                    .into_iter()
                    .map(|saved| DueSource::Ordinary(saved.binding)),
            );
        let mut casters: Vec<QualifiedCaster> = Vec::new();
        let store = FreshAdmissionStore::from_root(self.root.clone());
        for source in proposals {
            if casters
                .iter()
                .any(|caster| caster.source.same_source(&source))
            {
                continue;
            }
            if !source.active(self.spells, &runtime) {
                continue;
            }
            let session = source.command().game_session_id();
            if runtime
                .player_control_facts(source.caster(), session)
                .is_err()
                || states.get(&runtime, source.caster(), session).is_none()
            {
                continue;
            }
            let Ok((current, _)) = store.current_session_at(session).await else {
                continue;
            };
            if !(current.session_state() == GameSessionState::Active
                || matches!(source, DueSource::Ordinary(_))
                    && current.session_state() == GameSessionState::Reconnectable)
                || current.commit().character_id() != source.attacker()
                || current.current_runtime_scope()
                    != RuntimeScopeRefV1::channel(
                        runtime.binding().world_id(),
                        runtime.binding().channel_id(),
                    )
                || current.current_scope_generation() != runtime.binding().scope_generation()
            {
                continue;
            }
            let character = crate::domain::CharacterId::from_bytes(*source.attacker().as_bytes())
                .map_err(|_| Error::StaleOwner)?;
            if self
                .root
                .read_current_character(self.character, character)
                .await
                .is_err()
            {
                continue;
            }
            let lease = current.current_character_lease().generation();
            if lease == 0 {
                continue;
            }
            let fence = crate::durability::item_transfer::CurrentCharacterItemFence {
                character_id: character,
                game_session_id: session,
                connection_generation: current.current_connection_generation(),
                character_lease_generation: lease,
                runtime_scope: current.current_runtime_scope(),
                scope_ownership_generation: current.current_scope_generation(),
            };
            casters.push(QualifiedCaster {
                source,
                lease,
                fence,
            });
        }
        if let Some(timer) = states.spell_timers.as_ref() {
            timer.validate_prepared_due(|payload, batch| {
                // The same fenced lane will cancel this stale exact target before use.
                if !payload.target_current(&runtime) {
                    return Ok(());
                }
                let binding = payload.binding();
                if let TimerPayload::OrdinaryCombat(saved) = payload {
                    validate_numeric_predecessor(&runtime, &states, saved)?;
                    let resolved = saved.resolved.as_ref().ok_or(Error::InvalidPayload)?;
                    states
                        .presentations
                        .as_ref()
                        .ok_or(Error::InvalidPayload)?
                        .validate_prepared(&runtime, &resolved.presentation, &resolved.batch)
                        .map_err(|_| Error::StaleOwner)?;
                }
                let current = casters
                    .iter()
                    .find(|c| c.source.matches_payload(payload))
                    .ok_or(Error::StaleOwner)?;
                if current.lease != batch.current_lease_generation
                    || !current.source.active(self.spells, &runtime)
                    || runtime
                        .player_control_facts(binding.caster, binding.command.game_session_id())
                        .is_err()
                    || states
                        .get(&runtime, binding.caster, binding.command.game_session_id())
                        .is_none()
                {
                    return Err(Error::StaleOwner);
                }
                Ok(())
            })?;
        }
        // Resolve only the first ordinary phase in this turn. Later phases read
        // numerical owners after earlier real combat receipts have been applied.
        let unresolved = states.spell_timers.as_ref().and_then(|timer| {
            timer.due_ordinary_payloads(at).into_iter().find(|saved| {
                saved.current_target(&runtime)
                    && saved.binding.active(self.spells, &runtime)
                    && casters.iter().any(|c| {
                        c.source
                            .matches_payload(&TimerPayload::OrdinaryCombat(saved.clone()))
                    })
            })
        });
        if let Some(saved) = unresolved {
            let occurrence = crate::spell::delayed_execution::SpellTimerOccurrence {
                command: saved.binding.command,
                phase: saved.phase,
            };
            let current = casters
                .iter()
                .find(|c| {
                    c.source
                        .matches_payload(&TimerPayload::OrdinaryCombat(saved.clone()))
                })
                .ok_or(Error::StaleOwner)?;
            let access = access
                .iter()
                .find(|(actor, session, _)| {
                    *actor == saved.binding.caster
                        && *session == saved.binding.command.game_session_id()
                })
                .ok_or(Error::StaleOwner)?
                .2
                .clone();
            let objects = self.door.lock().await;
            let pass = self
                .root
                .try_issue_semantic_pass()
                .map_err(|_| Error::StaleOwner)?;
            let mut accepted = None;
            let mut context = (
                self,
                &mut *runtime,
                &mut *states,
                &*objects,
                access,
                &saved,
                &current.fence,
                &casters,
                &mut accepted,
            );
            pass.run_with_context(&mut context, move |holder, deadline, ctx| {
                Box::pin(async move {
                    use crate::durability::{DurabilityError, spell_item_transaction as item_tx};
                    use sha2::{Digest, Sha256};
                    let (owner, runtime, states, objects, access, saved, fence, casters, accepted) =
                        ctx;
                    let active = owner
                        .active_generation
                        .ok_or(DurabilityError::Unavailable)?;
                    let content = active
                        .native_gameplay()
                        .ok_or(DurabilityError::Unavailable)?;
                    let room = owner.qualified_room.ok_or(DurabilityError::Unavailable)?;
                    if !saved.binding.active(owner.spells, runtime)
                        || !saved.current_target(runtime)
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                    let identity = &saved
                        .binding
                        .source
                        .authored
                        .as_ref()
                        .ok_or(DurabilityError::Unavailable)?
                        .header
                        .identity;
                    let evidence = DueEvidence {
                        runtime,
                        timer: states
                            .spell_timers
                            .as_ref()
                            .ok_or(DurabilityError::Unavailable)?,
                        saved,
                        lease: fence.character_lease_generation,
                        now: crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                            owner.owner_now().get(),
                        ),
                        spell: crate::durability::item_mint::TypedDefinitionRef {
                            family: "Spell".into(),
                            production_key: identity.key.clone(),
                            revision_ref: identity.revision.clone(),
                        },
                    };
                    let due_authority =
                        item_tx::assert_due_spell_item_read_authority_in_transaction(
                            &mut tx,
                            owner.root,
                            owner.character,
                            owner.holder,
                            fence,
                            saved.binding.command,
                            content.source_digest(),
                            &evidence,
                        )
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                    let authority = due_authority.read_authority();
                    let state = states
                        .get(
                            runtime,
                            saved.binding.caster,
                            saved.binding.command.game_session_id(),
                        )
                        .ok_or(DurabilityError::Unavailable)?;
                    let owned =
                        super::super::spell_access_facts::load_owned_cast_facts_in_transaction(
                            &mut tx,
                            owner.root,
                            owner.character,
                            owner.holder,
                            fence,
                            saved.binding.command,
                            runtime,
                            saved.binding.caster,
                            state,
                            active,
                            access.as_ref(),
                            owner.owner_now().get(),
                        )
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                    if saved.resolved.is_none() {
                        let phase_now = owner.owner_now();
                        let requests = super::prepare_ordinary_due_presentations(
                            &mut tx,
                            authority,
                            runtime,
                            room,
                            objects,
                            &saved.binding.source,
                            saved.from,
                            saved.target,
                        )
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                        let index = (1_u32..)
                            .map_while(std::num::NonZeroU32::new)
                            .take_while(|index| content.spell_book().indexed(*index).is_some())
                            .find(|index| {
                                content.spell_book().indexed(*index)
                                    == Some(saved.binding.source.as_ref())
                            })
                            .ok_or(DurabilityError::Unavailable)?;
                        let mut source: serde_json::Value =
                            serde_json::from_slice(&saved.binding.parent_binding)
                                .map_err(|_| DurabilityError::Unavailable)?;
                        let object = source.as_object_mut().ok_or(DurabilityError::Unavailable)?;
                        object.remove("source_presentation_bytes");
                        object.insert("ordinary_due_phase".into(), saved.phase.into());
                        let binding = serde_json::to_vec(&source)
                            .map_err(|_| DurabilityError::Unavailable)?;
                        let mut batch = crate::spell::combat_batch::OwnerCombatBatch {
                            caster: saved.binding.caster,
                            attacker: saved.binding.attacker,
                            current_lease_generation: fence.character_lease_generation,
                            command: saved.binding.command,
                            occurrence: saved.binding.occurrence.clone().into(),
                            binding,
                            anchor: None,
                            now_ms: phase_now.get() / 1000,
                            effects: vec![crate::spell::combat_batch::OwnerCombatEffect {
                                target: saved.target,
                                sub_ordinal: saved.sub_ordinal,
                                change: crate::spell::combat_batch::OwnerCombatChange::Damage {
                                    target_atom: runtime
                                        .creature_spell_target_atom(saved.target)
                                        .map_err(|_| DurabilityError::Unavailable)?,
                                    magnitude: 0,
                                },
                            }],
                            deferred: None,
                        };
                        // Qualify presentation capacity and source before the numerical draw.
                        let mut preview = batch.clone();
                        states
                            .presentations
                            .as_mut()
                            .ok_or(DurabilityError::Unavailable)?
                            .prepare_source_definition(
                                runtime,
                                content,
                                index,
                                &saved.binding.source,
                                &mut preview,
                                requests.clone(),
                            )
                            .map_err(|_| DurabilityError::Unavailable)?;

                        let hash = Sha256::new()
                            .chain_update(b"oteryn:ordinary-chain-due:v1")
                            .chain_update(&saved.binding.parent_binding)
                            .chain_update(saved.phase.to_be_bytes())
                            .finalize();
                        let mut bytes = [0; 16];
                        bytes.copy_from_slice(&hash[..16]);
                        let decision =
                            oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(bytes);
                        let stream =
                            oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
                                content.source_digest(),
                            );
                        let mut ordinal = 0_u64;
                        let mut invalid_draw = false;
                        let mut draw = |minimum, maximum| match (
                            oteryn_simulation_determinism::deterministic_decision_u64(
                                &stream,
                                decision,
                                "spell.ordinary.due.draw",
                                ordinal,
                            ),
                            ordinal.checked_add(1),
                        ) {
                            (Ok(value), Some(next)) => {
                                ordinal = next;
                                crate::spell::uniform_draw(value, minimum, maximum)
                            }
                            _ => {
                                invalid_draw = true;
                                minimum
                            }
                        };
                        let (mut effect, magnitude) = super::prepare_ordinary_due_from_owners(
                            &mut tx,
                            authority,
                            runtime,
                            states,
                            room,
                            objects,
                            content,
                            &owned,
                            &saved.binding.source,
                            saved.target,
                            saved.step,
                            saved.binding.command,
                            &saved.binding.occurrence,
                            phase_now,
                            &mut draw,
                        )
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                        if invalid_draw {
                            return Err(DurabilityError::Unavailable);
                        }
                        effect.sub_ordinal = saved.sub_ordinal;
                        batch.effects = vec![effect];
                        let presentation = states
                            .presentations
                            .as_mut()
                            .ok_or(DurabilityError::Unavailable)?
                            .prepare_source_definition(
                                runtime,
                                content,
                                index,
                                &saved.binding.source,
                                &mut batch,
                                requests,
                            )
                            .map_err(|_| DurabilityError::Unavailable)?;
                        states
                            .presentations
                            .as_mut()
                            .ok_or(DurabilityError::Unavailable)?
                            .hold_prepared_before_sql(runtime, &presentation, &batch)
                            .map_err(|_| DurabilityError::Unavailable)?;
                        let resolved = crate::spell::ordinary_timer::ResolvedOrdinaryCombat {
                            batch,
                            presentation: std::sync::Arc::new(presentation),
                            magnitude: std::sync::Arc::new(magnitude),
                            owned: owned.clone(),
                        };
                        // Retain before the committing await. An ambiguous SQL outcome never
                        // permits a second draw; each retry still qualifies live owners above.
                        states
                            .spell_timers
                            .as_mut()
                            .ok_or(DurabilityError::Unavailable)?
                            .retain_ordinary_numeric(runtime, occurrence, resolved)
                            .map_err(|_| DurabilityError::Unavailable)?;
                    }
                    if let Some(resolved) = &saved.resolved {
                        if resolved.owned.binding() != owned.binding()
                            || resolved.owned.durable_build() != owned.durable_build()
                            || resolved.owned.equipment() != owned.equipment()
                            || resolved.owned.magnitude() != owned.magnitude()
                        {
                            return Err(DurabilityError::Unavailable);
                        }
                        let state = states
                            .get(
                                runtime,
                                saved.binding.caster,
                                saved.binding.command.game_session_id(),
                            )
                            .ok_or(DurabilityError::Unavailable)?;
                        resolved
                            .magnitude
                            .validate_current_with_lookup(
                                runtime,
                                state,
                                owned.magnitude().ok_or(DurabilityError::Unavailable)?,
                                &|actor| {
                                    states
                                        .actors
                                        .iter()
                                        .find(|(exact, session, _)| {
                                            *exact == actor
                                                && states.get(runtime, *exact, *session).is_some()
                                        })
                                        .map(|(_, _, state)| state)
                                },
                            )
                            .map_err(|_| DurabilityError::Unavailable)?;
                    }
                    // Apply physical HP and publish its actual receipt before releasing
                    // the SQL authority locks. No awaited gap can rebase the source proof.
                    **accepted = Some(
                        apply_due_under_current_owners(
                            runtime,
                            states,
                            owner.spells,
                            casters,
                            owner.owner_now(),
                        )
                        .map_err(|_| DurabilityError::Unavailable)?,
                    );
                    tx.rollback()
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                    Ok(())
                })
            })
            .await
            .map_err(|_| Error::StaleOwner)?;
            if let Some(report) = accepted {
                return Ok(report);
            }
        }
        apply_due_under_current_owners(
            &mut runtime,
            &mut states,
            self.spells,
            &casters,
            self.owner_now(),
        )
    }
}

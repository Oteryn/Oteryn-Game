//! Due strikes use the same committed-cost SQL reader and physical combat owner as chains.
use super::*;
use crate::durability::spell_item_transaction as item_tx;
use crate::foundation::ChannelRuntimeV1;
use crate::spell::delayed_execution::SavedStrike;

pub(super) fn validate_numeric(
    runtime: &ChannelRuntimeV1,
    states: &super::super::ChannelSpellStates,
    saved: &SavedStrike,
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
                    .find(|(a, s, _)| *a == actor && states.get(runtime, *a, *s).is_some())
                    .map(|(_, _, s)| s)
            },
        )
        .map_err(|_| Error::StaleOwner)?;
    states
        .presentations
        .as_ref()
        .ok_or(Error::InvalidPayload)?
        .validate_prepared(runtime, &resolved.presentation, &resolved.batch)
        .map_err(|_| Error::StaleOwner)
}

/// Retiring a callback also retires its uncommitted presentation reservation.
pub(super) fn cancel(
    runtime: &ChannelRuntimeV1,
    states: &mut super::super::ChannelSpellStates,
    command: crate::foundation::CommandRef,
) -> Result<(), Error> {
    let Some(timer) = states.spell_timers.as_mut() else {
        return Ok(());
    };
    let mut found = false;
    let presentations: Vec<_> = timer
        .payloads
        .iter()
        .filter_map(|p| match &p.payload {
            TimerPayload::Strike(saved) if saved.binding.command == command => {
                found = true;
                saved.resolved.as_ref().map(|r| r.presentation.clone())
            }
            _ => None,
        })
        .collect();
    if !found {
        return Ok(());
    }
    timer.cancel_strike(runtime, command)?;
    if let Some(outbox) = states.presentations.as_mut() {
        for presentation in presentations {
            outbox.release_definitely_uncommitted(&presentation);
        }
    }
    Ok(())
}

struct StrikeEvidence<'a> {
    runtime: &'a ChannelRuntimeV1,
    timer: &'a crate::spell::delayed_execution::SpellTimerOwner,
    saved: &'a SavedStrike,
    now: crate::foundation::owner_timer::SemanticTimeMicros,
    lease: u64,
    spell: crate::durability::item_mint::TypedDefinitionRef,
}
impl item_tx::due_read_seal::Sealed for StrikeEvidence<'_> {}
impl item_tx::CommittedSpellDueEvidence for StrikeEvidence<'_> {
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
        self.runtime.content_pin().server_artifact_digest()
    }
    fn verify_current_retained_source(&self) -> Result<(), &'static str> {
        if !self
            .timer
            .contains_due_strike(self.runtime, self.saved, self.now)
        {
            return Err("committed strike missing");
        }
        match self.runtime.retained_spell_batch(
            self.saved.binding.caster,
            self.saved.binding.attacker,
            self.lease,
            self.saved.binding.command,
        ) {
            Ok(Some(receipt)) if receipt.binding == self.saved.binding.parent_binding => Ok(()),
            Err(crate::spell::combat_batch::Error::StaleCommand) => Ok(()),
            _ => Err("strike source receipt conflicts"),
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "one locked due turn and its fresh canonical caster fence"
    )]
    pub(super) async fn resolve_strike_due(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        runtime: &mut ChannelRuntimeV1,
        states: &mut super::super::ChannelSpellStates,
        room: &crate::content::QualifiedNativeEntryRoom,
        saved: &SavedStrike,
        fence: &crate::durability::item_transfer::CurrentCharacterItemFence,
        casters: &[QualifiedCaster],
        access: &(impl crate::gameplay_transport::spell_access_facts::CurrentSpellAccessOwner + Sync),
    ) -> Result<FireReport, Error> {
        let objects = self.door.lock().await;
        let pass = self
            .root
            .try_issue_semantic_pass()
            .map_err(|_| Error::StaleOwner)?;
        let mut accepted = None;
        let mut context = (runtime, states, &mut accepted);
        pass.run_with_context(&mut context, |holder, deadline, ctx| Box::pin(async move {
            use crate::durability::DurabilityError;
            let (runtime, states, accepted) = ctx;
            let active = self.active_generation.ok_or(DurabilityError::Unavailable)?;
            let content = active.native_gameplay().ok_or(DurabilityError::Unavailable)?;
            let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
            let profile = saved.binding.spell.spell();
            let evidence = StrikeEvidence { runtime, timer: states.spell_timers.as_ref().ok_or(DurabilityError::Unavailable)?, saved,
                now: crate::foundation::owner_timer::SemanticTimeMicros::from_micros(self.owner_now().get()),
                lease: fence.character_lease_generation,
                spell: crate::durability::item_mint::TypedDefinitionRef { family: "Spell".into(),
                    production_key: profile["identity"]["key"].as_str().ok_or(DurabilityError::Unavailable)?.into(),
                    revision_ref: profile["identity"]["revision"].as_str().ok_or(DurabilityError::Unavailable)?.into() } };
            let authority = item_tx::assert_due_spell_item_read_authority_in_transaction(&mut tx, permit,
                self.root, self.character, self.holder, fence, saved.binding.command, content.source_digest(), &evidence)
                .await.map_err(|_|DurabilityError::Unavailable)?;
            let state = states.get(runtime, saved.binding.caster, saved.binding.command.game_session_id()).ok_or(DurabilityError::Unavailable)?;
            let owned = crate::gameplay_transport::spell_access_facts::load_owned_cast_facts_in_transaction(&mut tx,
                self.root, self.character, self.holder, fence, saved.binding.command, runtime, saved.binding.caster,
                state, active, access, self.owner_now().get()).await.map_err(|_|DurabilityError::Unavailable)?;
            if let Some(resolved) = &saved.resolved {
                if resolved.owned.binding() != owned.binding() || resolved.owned.durable_build() != owned.durable_build()
                    || resolved.owned.equipment() != owned.equipment() || resolved.owned.magnitude() != owned.magnitude() {
                    return Err(DurabilityError::Unavailable);
                }
                validate_numeric(runtime, states, saved).map_err(|_|DurabilityError::Unavailable)?;
            } else {
                use sha2::{Digest, Sha256};
                let hash = Sha256::new().chain_update(b"oteryn:delayed-strike-due:v1")
                    .chain_update(&saved.binding.parent_binding).finalize();
                let mut bytes = [0;16]; bytes.copy_from_slice(&hash[..16]);
                let decision = oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(bytes);
                let stream = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(content.source_digest());
                let mut ordinal = 0_u64;
                let mut invalid = false;
                let mut draw = |min, max| match (oteryn_simulation_determinism::deterministic_decision_u64(&stream, decision, "spell.strike.due.draw", ordinal), ordinal.checked_add(1)) {
                    (Ok(value),Some(next)) => { ordinal = next; crate::spell::uniform_draw(value, min, max) },
                    _ => { invalid = true; min },
                };
                let talisman=crate::durability::task_board::read_talisman_in_item_transaction(&mut tx,*fence).await?;
                let resolved = super::super::native_combat_cast::prepare_strike_due(&mut tx, authority.read_authority(), runtime, states,
                    room, &objects, content, &owned, self.spells, saved, self.owner_now(), &talisman, decision, &mut draw).await.map_err(|_|DurabilityError::Unavailable)?;
                if invalid { return Err(DurabilityError::Unavailable); }
                let Some(resolved) = resolved else {
                    cancel(runtime, states, saved.binding.command).map_err(|_|DurabilityError::Unavailable)?;
                    **accepted = Some(FireReport { dropped: vec![crate::spell::delayed_execution::SpellTimerOccurrence {
                        command: saved.binding.command, phase: 0 }], ..FireReport::default() });
                    tx.rollback().await.map_err(|_|DurabilityError::Unavailable)?;
                    return Ok(());
                };
                states.spell_timers.as_mut().ok_or(DurabilityError::Unavailable)?
                    .retain_strike(runtime, saved, resolved, crate::foundation::owner_timer::SemanticTimeMicros::from_micros(self.owner_now().get()))
                    .map_err(|_|DurabilityError::Unavailable)?;
            }
            **accepted = Some(apply_due_under_current_owners(runtime, states, self.spells, casters, self.owner_now()).map_err(|_|DurabilityError::Unavailable)?);
            tx.rollback().await.map_err(|_|DurabilityError::Unavailable)?;
            Ok(())
        })).await.map_err(|_|Error::StaleOwner)?;
        accepted.ok_or(Error::StaleOwner)
    }
}

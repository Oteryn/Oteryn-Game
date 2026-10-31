use super::*;
use crate::domain::soulpit::{MAX_PARTICIPANTS, SoulpitStack};
use crate::durability::character_progression::{CharacterProgressionError, assert_gameplay_fence};

pub(super) async fn write(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    mut request: SoulpitStepRequest,
) -> Result<SoulpitStepOutcome, SoulpitWriteError> {
    if !is_uuid_v7(&request.handoff_id) || !is_uuid_v7(&request.occurrence)
        || request.survivors.len() > MAX_PARTICIPANTS {
        return Err(SoulpitWriteError::Rule(SoulpitError::InvalidState));
    }
    let RuntimeScopeRefV1::Instance { instance_id, .. } = request.runtime_scope else {
        return Err(SoulpitWriteError::AuthorityRejected);
    };
    request.survivors.sort_unstable_by_key(|fence| *fence.character_id.as_bytes());
    let intent = intent(&request);
    lock_handoff(tx, &request.handoff_id).await?;
    let previous = sqlx::query("SELECT revision,intent,checkpoint,effect FROM game_soulpit_events WHERE handoff_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid")
        .bind(request.handoff_id.as_slice()).bind(request.occurrence.as_slice()).fetch_optional(&mut **tx).await?;
    if let Some(row) = previous {
        if row.try_get::<Value,_>("intent")? != intent { return Err(SoulpitWriteError::ConflictingOccurrence); }
        return Ok(SoulpitStepOutcome {
            revision: u64::try_from(row.try_get::<i64,_>("revision")?).map_err(|_| DurabilityError::InvalidStoredState)?,
            encounter: decode_checkpoint(row.try_get("checkpoint")?)?,
            effect: serde_json::from_value(row.try_get("effect")?).map_err(|_| DurabilityError::InvalidStoredState)?,
            replayed: true,
        });
    }
    if !assert_runtime_scope_fence(tx, request.runtime_scope, request.scope_ownership_generation, node).await? {
        return Err(SoulpitWriteError::AuthorityRejected);
    }
    let row = sqlx::query("SELECT f.revision,f.species_key,f.checkpoint,f.terminal,g.ownership_generation::text FROM game_soulpit_encounters f JOIN game_activity_handoff_groups g USING(handoff_id) WHERE f.handoff_id=encode($1,'hex')::uuid AND g.world_id=encode($2,'hex')::uuid AND g.instance_id=encode($3,'hex')::uuid AND g.state=2 AND g.activity_key=$4 FOR UPDATE OF f")
        .bind(request.handoff_id.as_slice()).bind(request.runtime_scope.world_id().as_bytes().as_slice())
        .bind(instance_id.as_slice()).bind(SOULPIT_ACTIVITY_KEY).fetch_optional(&mut **tx).await?
        .ok_or(SoulpitWriteError::AuthorityRejected)?;
    let generation = row.try_get::<String,_>("ownership_generation")?.parse::<u64>().map_err(|_| DurabilityError::InvalidStoredState)?;
    if (request.action != SoulpitAction::Interrupt && generation != request.scope_ownership_generation.get())
        || generation > request.scope_ownership_generation.get() {
        return Err(SoulpitWriteError::AuthorityRejected);
    }
    let revision = u64::try_from(row.try_get::<i64,_>("revision")?).map_err(|_| DurabilityError::InvalidStoredState)?;
    if revision != request.expected_revision || row.try_get::<bool,_>("terminal")? {
        return Err(SoulpitWriteError::StaleState);
    }
    let species: String = row.try_get("species_key")?;
    let mut encounter = decode_checkpoint(row.try_get("checkpoint")?)?;
    let effect = apply(&mut encounter, &request.action, request.now_micros)?;
    let next_revision = revision.checked_add(1).and_then(|value| i64::try_from(value).ok())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let state = checkpoint(&encounter)?;
    let terminal = encounter.snapshot().phase.is_terminal();
    sqlx::query("INSERT INTO game_soulpit_events(handoff_id,occurrence_id,revision,intent,checkpoint,effect,terminal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7)")
        .bind(request.handoff_id.as_slice()).bind(request.occurrence.as_slice()).bind(next_revision)
        .bind(intent).bind(&state).bind(serde_json::to_value(&effect).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(terminal).execute(&mut **tx).await?;
    if let EncounterEffect::Completed { surviving_slots } = &effect {
        credit_survivors(tx, node, &request, surviving_slots, &species).await?;
    }
    let changed = sqlx::query("UPDATE game_soulpit_encounters SET revision=$2,checkpoint=$3,terminal=$4,last_occurrence=encode($5,'hex')::uuid WHERE handoff_id=encode($1,'hex')::uuid AND revision=$6")
        .bind(request.handoff_id.as_slice()).bind(next_revision).bind(state).bind(terminal).bind(request.occurrence.as_slice())
        .bind(i64::try_from(revision).map_err(|_| DurabilityError::InvalidStoredState)?).execute(&mut **tx).await?.rows_affected();
    if changed != 1 { return Err(SoulpitWriteError::StaleState); }
    Ok(SoulpitStepOutcome { revision: u64::try_from(next_revision).map_err(|_| DurabilityError::InvalidStoredState)?, encounter, effect, replayed: false })
}

fn intent(request: &SoulpitStepRequest) -> Value {
    let participants: Vec<_> = request.survivors.iter().map(|fence| json!({
        "character":hex(fence.character_id.as_bytes()),"session":hex(fence.game_session_id.as_bytes()),
        "connection":fence.connection_generation.get(),"lease":fence.character_lease_generation,
        "revision":fence.expected_character_revision.get(),"scope":scope_intent(fence.runtime_scope),
        "generation":fence.scope_ownership_generation.get(),
    })).collect();
    json!({"version":1,"scope":scope_intent(request.runtime_scope),"generation":request.scope_ownership_generation.get(),
        "expected_revision":request.expected_revision,"now":request.now_micros,
        "action":request.action,"survivors":participants})
}

fn scope_intent(scope: RuntimeScopeRefV1) -> Value {
    match scope {
        RuntimeScopeRefV1::Channel { world_id, channel_id } => json!({"kind":1,"world":hex(world_id.as_bytes()),"channel":hex(channel_id.as_bytes())}),
        RuntimeScopeRefV1::Instance { world_id, instance_id } => json!({"kind":2,"world":hex(world_id.as_bytes()),"instance":hex(&instance_id)}),
    }
}

fn apply(encounter: &mut SoulpitEncounter, action: &SoulpitAction, now: u64) -> Result<EncounterEffect, SoulpitError> {
    match action {
        SoulpitAction::Advance => encounter.advance(now),
        SoulpitAction::Spawned { wave, monster_ids } => {
            encounter.spawned(*wave, *monster_ids, now)?;
            Ok(EncounterEffect::NoChange)
        }
        SoulpitAction::Summoned { wave, monster_id, stack } => {
            encounter.summoned(*wave, *monster_id, SoulpitStack::from_count(*stack)?)?;
            encounter.observe_terminal(now)
        }
        SoulpitAction::Defeated { wave, monster_id } => {
            encounter.defeated(*wave, *monster_id)?;
            encounter.observe_terminal(now)
        }
        // Source global ticks alone clear waves. Death/logout observations still settle
        // independent timeout or last-participant abandonment in this same transaction.
        SoulpitAction::Leave { slot } => { encounter.leave(*slot, now)?; encounter.observe_terminal(now) }
        SoulpitAction::Rejoin { slot } => { encounter.rejoin(*slot, now)?; encounter.observe_terminal(now) }
        // The process monotonic clock may restart at zero. Restart destroys the donor's
        // process-local fight; it must not depend on comparing clocks across processes.
        SoulpitAction::Interrupt => encounter.interrupt(now.max(encounter.snapshot().last_observed_at_micros)),
    }
}

async fn credit_survivors(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    request: &SoulpitStepRequest,
    slots: &[u8],
    species: &str,
) -> Result<(), SoulpitWriteError> {
    if slots.is_empty() || slots.len() != request.survivors.len() {
        return Err(SoulpitWriteError::Rule(SoulpitError::InvalidParticipants));
    }
    let members = sqlx::query("SELECT uuid_send(p.character_id) AS character,uuid_send(p.destination_game_session_id) AS session,s.staging_slot FROM game_soulpit_participants s JOIN game_activity_handoff_participants p USING(handoff_id,character_id) WHERE s.handoff_id=encode($1,'hex')::uuid ORDER BY p.character_id LIMIT 6")
        .bind(request.handoff_id.as_slice()).fetch_all(&mut **tx).await?;
    let mut seen_slots = 0_u8;
    for fence in &request.survivors {
        let member = members.iter().find(|row| row.try_get::<Vec<u8>,_>("character").is_ok_and(|id| id==fence.character_id.as_bytes()))
            .ok_or(SoulpitWriteError::AuthorityRejected)?;
        let slot = u8::try_from(member.try_get::<i16,_>("staging_slot")?).map_err(|_| DurabilityError::InvalidStoredState)?;
        if !slots.contains(&slot) || seen_slots & (1<<slot) != 0
            || member.try_get::<Vec<u8>,_>("session")? != fence.game_session_id.as_bytes()
            || fence.runtime_scope != request.runtime_scope || fence.scope_ownership_generation != request.scope_ownership_generation {
            return Err(SoulpitWriteError::AuthorityRejected);
        }
        seen_slots |= 1<<slot;
        if assert_gameplay_fence(tx, fence, node).await?.is_err() { return Err(SoulpitWriteError::AuthorityRejected); }
        super::super::animus_mastery::complete_in_transaction(tx, fence.character_id, species).await.map_err(|error| match error {
            CharacterProgressionError::Unavailable(error) => SoulpitWriteError::Unavailable(error),
            _ => SoulpitWriteError::AuthorityRejected,
        })?;
        sqlx::query("INSERT INTO game_soulpit_survival_rewards(handoff_id,character_id,species_key,occurrence_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid)")
            .bind(request.handoff_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(species).bind(request.occurrence.as_slice())
            .execute(&mut **tx).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::soulpit::EncounterPhase;

    #[test]
    fn retained_wave_progress_requires_all_seven_and_rejects_foreign_monster() -> Result<(), SoulpitError> {
        let mut encounter = SoulpitEncounter::new(0, &[0, 2])?;
        apply(&mut encounter, &SoulpitAction::Spawned { wave: 1, monster_ids: [1,2,3,4,5,6,7].map(Some) }, 4_000_000)?;
        assert_eq!(apply(&mut encounter, &SoulpitAction::Defeated { wave: 1, monster_id: 99 }, 4_500_000), Err(SoulpitError::InvalidMonster));
        for monster_id in 1..7 {
            assert_eq!(apply(&mut encounter, &SoulpitAction::Defeated { wave: 1, monster_id }, 4_500_000)?, EncounterEffect::NoChange);
        }
        let restored = decode_checkpoint(checkpoint(&encounter).map_err(|_| SoulpitError::InvalidState)?).map_err(|_| SoulpitError::InvalidState)?;
        assert_eq!(restored, encounter);
        assert_eq!(apply(&mut encounter, &SoulpitAction::Defeated { wave: 1, monster_id: 7 }, 4_500_000)?, EncounterEffect::NoChange);
        assert_eq!(encounter.snapshot().wave,1);
        assert_eq!(apply(&mut encounter,&SoulpitAction::Advance,5_000_000)?,EncounterEffect::WaveStarted {wave:2});
        Ok(())
    }

    #[test]
    fn remaining_participant_leave_and_rejoin_cannot_clear_wave_between_global_ticks()->Result<(),SoulpitError> {
        let mut encounter=SoulpitEncounter::new(0,&[0,2])?;
        apply(&mut encounter,&SoulpitAction::Spawned {wave:1,monster_ids:[None;7]},4_000_000)?;
        assert_eq!(apply(&mut encounter,&SoulpitAction::Leave {slot:2},4_600_000)?,EncounterEffect::NoChange);
        assert_eq!(encounter.snapshot().wave,1);
        assert_eq!(apply(&mut encounter,&SoulpitAction::Rejoin {slot:2},4_700_000)?,EncounterEffect::NoChange);
        assert_eq!(encounter.snapshot().wave,1);
        assert_eq!(apply(&mut encounter,&SoulpitAction::Advance,5_000_000)?,EncounterEffect::WaveStarted {wave:2});
        Ok(())
    }

    #[test]
    fn interruption_and_last_departure_never_grant_survival_reward() -> Result<(), SoulpitError> {
        let mut interrupted = SoulpitEncounter::new(0, &[0])?;
        apply(&mut interrupted, &SoulpitAction::Spawned { wave: 1, monster_ids: [1,2,3,4,5,6,7].map(Some) }, 4_000_000)?;
        assert_eq!(apply(&mut interrupted, &SoulpitAction::Interrupt, 0)?, EncounterEffect::Interrupted);
        assert_eq!(interrupted.snapshot().phase, EncounterPhase::Interrupted);
        let mut departed = SoulpitEncounter::new(0, &[0])?;
        assert_eq!(apply(&mut departed, &SoulpitAction::Leave { slot: 0 }, 1)?, EncounterEffect::Abandoned);
        assert!(apply(&mut departed, &SoulpitAction::Rejoin { slot: 0 }, 2).is_err());
        Ok(())
    }
}

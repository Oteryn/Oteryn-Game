//! Shared return provenance is proved before this Soulpit-only lifecycle consequence.
use super::*;
use crate::durability::activity_handoff::{ActivityReturnReason, ActivityReturnRequest};
use sha2::{Digest, Sha256};

/// Called only after the existing return/recovery owner accepted the exact participant,
/// source authority, origin and reason, and before it ends that source session.
/// The surrounding transaction owns both this consequence and the successor session.
pub(crate) async fn write(
    tx: &mut Transaction<'_, Postgres>,
    request: &ActivityReturnRequest,
    recovering: bool,
) -> Result<(), DurabilityError> {
    let row = sqlx::query("SELECT f.revision,f.checkpoint,f.terminal,s.staging_slot FROM game_soulpit_encounters f JOIN game_soulpit_participants s USING(handoff_id) WHERE f.handoff_id=encode($1,'hex')::uuid AND s.character_id=encode($2,'hex')::uuid FOR UPDATE OF f")
        .bind(request.handoff_id.as_slice()).bind(request.source.character_id.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(());
    };
    if row.try_get::<bool, _>("terminal")? {
        return Ok(());
    }
    let revision = u64::try_from(row.try_get::<i64, _>("revision")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut encounter = decode_checkpoint(row.try_get("checkpoint")?)?;
    let snapshot = encounter.snapshot();
    let (action, now) = if recovering || request.reason == ActivityReturnReason::OwnerLost {
        (SoulpitAction::Interrupt, snapshot.last_observed_at_micros)
    } else if request.reason == ActivityReturnReason::Timeout {
        (
            SoulpitAction::Advance,
            snapshot.last_observed_at_micros.max(
                snapshot
                    .started_at_micros
                    .saturating_add(crate::domain::soulpit::TIME_LIMIT_MICROS),
            ),
        )
    } else {
        (
            SoulpitAction::Leave {
                slot: u8::try_from(row.try_get::<i16, _>("staging_slot")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            },
            snapshot.last_observed_at_micros,
        )
    };
    let effect = progress::apply(&mut encounter, &action, now)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    // Return never completes a wave or grants mastery. Completion belongs only to
    // the physical wave owner; timeout, death, logout and recovery remove survivors.
    if matches!(effect, EncounterEffect::Completed { .. }) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let digest = Sha256::new()
        .chain_update(b"oteryn:soulpit:return:v1")
        .chain_update(request.handoff_id)
        .chain_update(request.return_id)
        .finalize();
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&digest[..16]);
    occurrence[6] = (occurrence[6] & 15) | 0x70;
    occurrence[8] = (occurrence[8] & 63) | 0x80;
    let step = SoulpitStepRequest {
        handoff_id: request.handoff_id,
        occurrence,
        runtime_scope: request.source.runtime_scope,
        scope_ownership_generation: request.source.scope_ownership_generation,
        expected_revision: revision,
        now_micros: now,
        action,
        survivors: Vec::new(),
    };
    let next = revision
        .checked_add(1)
        .and_then(|v| i64::try_from(v).ok())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let state = checkpoint(&encounter)?;
    let terminal = encounter.snapshot().phase.is_terminal();
    sqlx::query("INSERT INTO game_soulpit_events(handoff_id,occurrence_id,revision,intent,checkpoint,effect,terminal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7)")
        .bind(request.handoff_id.as_slice()).bind(occurrence.as_slice()).bind(next)
        .bind(progress::intent(&step)).bind(&state)
        .bind(serde_json::to_value(effect).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(terminal).execute(&mut **tx).await?;
    let changed = sqlx::query("UPDATE game_soulpit_encounters SET revision=$2,checkpoint=$3,terminal=$4,last_occurrence=encode($5,'hex')::uuid WHERE handoff_id=encode($1,'hex')::uuid AND revision=$6")
        .bind(request.handoff_id.as_slice()).bind(next).bind(state).bind(terminal)
        .bind(occurrence.as_slice()).bind(i64::try_from(revision).map_err(|_| DurabilityError::InvalidStoredState)?)
        .execute(&mut **tx).await?.rows_affected();
    if changed != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

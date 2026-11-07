//! Descendant of an authenticated shared return, never an alternate scope grant.
use super::*;
use crate::durability::activity_handoff::{ActivityReturnReason, ActivityReturnRequest};

pub(in crate::durability) async fn lock_shared_return(
    tx: &mut Transaction<'_, Postgres>,
    handoff: &[u8; 16],
) -> Result<bool, DurabilityError> {
    let source: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_activity_handoff_groups g JOIN game_soulpit_encounters f USING(handoff_id) WHERE g.handoff_id=encode($1,'hex')::uuid AND g.activity_key=$2)")
        .bind(handoff.as_slice()).bind(SOULPIT_ACTIVITY_KEY).fetch_one(&mut **tx).await?;
    if source {
        lock_handoff(tx, handoff).await?;
    }
    Ok(source)
}

/// The shared owner has already proved current control or its explicit recovery
/// cause, issued the successor and inserted this exact immutable return receipt.
/// Only departure/interrupt is composed here; no spawn, XP or Animus grant exists.
pub(in crate::durability) async fn record_shared_return(
    tx: &mut Transaction<'_, Postgres>,
    request: &ActivityReturnRequest,
) -> Result<(), DurabilityError> {
    let RuntimeScopeRefV1::Instance { instance_id, .. } = request.source.runtime_scope else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let row = sqlx::query("SELECT f.revision,f.checkpoint,f.terminal,s.staging_slot FROM game_soulpit_encounters f JOIN game_activity_handoff_groups g USING(handoff_id) JOIN game_soulpit_participants s USING(handoff_id) JOIN game_activity_runtime_returns r ON r.handoff_id=s.handoff_id AND r.character_id=s.character_id WHERE r.return_id=encode($1,'hex')::uuid AND r.handoff_id=encode($2,'hex')::uuid AND r.character_id=encode($3,'hex')::uuid AND r.source_game_session_id=encode($4,'hex')::uuid AND g.world_id=encode($5,'hex')::uuid AND g.instance_id=encode($6,'hex')::uuid AND g.activity_key=$7 AND g.state=2 AND g.ownership_generation=$8::text::numeric AND r.reason=$9 FOR UPDATE OF f")
        .bind(request.return_id.as_slice()).bind(request.handoff_id.as_slice())
        .bind(request.source.character_id.as_bytes().as_slice()).bind(request.source.game_session_id.as_bytes().as_slice())
        .bind(request.source.runtime_scope.world_id().as_bytes().as_slice()).bind(instance_id.as_slice())
        .bind(SOULPIT_ACTIVITY_KEY).bind(request.source.scope_ownership_generation.get().to_string())
        .bind(request.reason.stored()).fetch_optional(&mut **tx).await?;
    let row = row.ok_or(DurabilityError::InvalidStoredState)?;
    if row.try_get::<bool, _>("terminal")? {
        return Ok(());
    }
    let slot = u8::try_from(row.try_get::<i16, _>("staging_slot")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let revision = row.try_get::<i64, _>("revision")?;
    let next = revision
        .checked_add(1)
        .filter(|value| *value > 0)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut encounter = decode_checkpoint(row.try_get("checkpoint")?)?;
    let snapshot = encounter.snapshot();
    let mut now = snapshot.last_observed_at_micros;
    let action = if request.reason == ActivityReturnReason::OwnerLost {
        SoulpitAction::Interrupt
    } else {
        if request.reason == ActivityReturnReason::Timeout {
            // The authenticated shared Timeout receipt proves this wall-clock
            // deadline. Translate that fact to the retained encounter's clock;
            // never compare process monotonic origins across ownership recovery.
            now = now.max(
                snapshot
                    .started_at_micros
                    .checked_add(crate::domain::soulpit::TIME_LIMIT_MICROS)
                    .ok_or(DurabilityError::InvalidStoredState)?,
            );
        } else if !snapshot.surviving_slots.contains(&slot) {
            return Ok(());
        }
        SoulpitAction::Leave { slot }
    };
    let effect = progress::apply(&mut encounter, &action, now)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let state = checkpoint(&encounter)?;
    let terminal = encounter.snapshot().phase.is_terminal();
    let intent = json!({"version":1,"cause":"shared-runtime-return","return":hex(&request.return_id),"source_session":hex(request.source.game_session_id.as_bytes()),"reason":request.reason.stored(),"expected_revision":revision,"now":now,"action":action});
    sqlx::query("INSERT INTO game_soulpit_events(handoff_id,occurrence_id,revision,intent,checkpoint,effect,terminal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7)")
        .bind(request.handoff_id.as_slice()).bind(request.return_id.as_slice()).bind(next).bind(intent).bind(&state)
        .bind(serde_json::to_value(effect).map_err(|_|DurabilityError::InvalidStoredState)?).bind(terminal).execute(&mut **tx).await?;
    let changed=sqlx::query("UPDATE game_soulpit_encounters SET revision=$2,checkpoint=$3,terminal=$4,last_occurrence=encode($5,'hex')::uuid WHERE handoff_id=encode($1,'hex')::uuid AND revision=$6")
        .bind(request.handoff_id.as_slice()).bind(next).bind(state).bind(terminal).bind(request.return_id.as_slice()).bind(revision).execute(&mut **tx).await?.rows_affected();
    if changed != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

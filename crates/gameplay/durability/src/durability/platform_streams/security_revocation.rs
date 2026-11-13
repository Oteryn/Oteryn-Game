//! Durable revocation projection. Admission's relation fence serializes capture with admission.
use super::*;
use sqlx::Row;

pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    item: &SecurityRevocation,
) -> Result<i16, DurabilityError> {
    if item.native_security_generation <= 0
        || !(1..=4).contains(&item.cause)
        || item.effective_at_ms <= 0
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let inserted = sqlx::query("INSERT INTO game_security_revocations(revocation_id,account_id,native_security_generation,cause,effective_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5) ON CONFLICT DO NOTHING")
        .bind(item.revocation_id.as_slice()).bind(item.account_id.as_slice()).bind(item.native_security_generation).bind(item.cause).bind(item.effective_at_ms).execute(&mut **tx).await?;
    if inserted.rows_affected() == 0 {
        let same: bool = sqlx::query_scalar("SELECT account_id=encode($2,'hex')::uuid AND native_security_generation=$3 AND cause=$4 AND effective_at_ms=$5 FROM game_security_revocations WHERE revocation_id=encode($1,'hex')::uuid")
            .bind(item.revocation_id.as_slice()).bind(item.account_id.as_slice()).bind(item.native_security_generation).bind(item.cause).bind(item.effective_at_ms).fetch_one(&mut **tx).await?;
        return if same {
            Ok(2)
        } else {
            Err(DurabilityError::InvalidStoredState)
        };
    }
    let prior: Option<i64> = sqlx::query_scalar("SELECT native_security_generation FROM game_security_revocation_floors WHERE account_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(item.account_id.as_slice()).fetch_optional(&mut **tx).await?;
    if prior.is_some_and(|generation| generation > item.native_security_generation) {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("INSERT INTO game_security_revocation_floors(account_id,native_security_generation) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(account_id) DO UPDATE SET native_security_generation=GREATEST(game_security_revocation_floors.native_security_generation,EXCLUDED.native_security_generation)")
        .bind(item.account_id.as_slice()).bind(item.native_security_generation).execute(&mut **tx).await?;
    // The canonical partial unique index permits at most one nonterminal session per account.
    let rows = sqlx::query("SELECT s.game_session_id::text,s.admission_security_generation::text AS generation,r.operation_json,(SELECT a.record_json FROM game_durability_reconnect_attempts a WHERE a.game_session_id=s.game_session_id ORDER BY a.control_loss_epoch DESC,a.reconnect_attempt_ref DESC LIMIT 1) AS record_json FROM game_durability_reconnect_sessions s LEFT JOIN game_durability_fresh_admission_receipts r ON r.replay_key=COALESCE(s.initial_fresh_replay_key,s.fresh_replay_key) WHERE s.account_id=encode($1,'hex')::uuid AND s.session_state IN(1,2) ORDER BY s.game_session_id LIMIT 2 FOR UPDATE OF s")
        .bind(item.account_id.as_slice()).fetch_all(&mut **tx).await?;
    if rows.len() > 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    for row in rows {
        let generation = admission_generation(&row)?;
        let revoked =
            item.cause == 3 || u128::from(generation) < item.native_security_generation as u128;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET admission_security_generation=$2::text::numeric,security_revoked=security_revoked OR $3 WHERE game_session_id=$1::text::uuid")
            .bind(row.try_get::<String,_>("game_session_id")?).bind(generation.to_string()).bind(revoked).execute(&mut **tx).await?;
    }
    Ok(1)
}

fn admission_generation(row: &sqlx::postgres::PgRow) -> Result<u64, DurabilityError> {
    Ok(
        if let Some(generation) = row.try_get::<Option<String>, _>("generation")? {
            generation
                .parse::<u64>()
                .map_err(|_| DurabilityError::InvalidStoredState)?
        } else if let Some(encoded) = row.try_get::<Option<String>, _>("operation_json")? {
            super::super::fresh_admission::decode_operation(
                &encoded,
                super::super::MAX_FRESH_OPERATION_BYTES,
            )?
            .authorization
            .signed_security_generation
        } else {
            let encoded: Option<String> = row.try_get("record_json")?;
            let record: serde_json::Value = serde_json::from_str(
                encoded
                    .as_deref()
                    .ok_or(DurabilityError::InvalidStoredState)?,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?;
            record["compatibility"]["account_security_generation"]
                .as_u64()
                .filter(|g| *g > 0)
                .ok_or(DurabilityError::InvalidStoredState)?
        },
    )
}

pub(super) async fn lineage(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: crate::foundation::GameSessionId,
) -> Result<(u64, bool), DurabilityError> {
    let row = sqlx::query("SELECT s.admission_security_generation::text AS generation,game_security_session_revoked(s.game_session_id) AS revoked,r.operation_json,(SELECT a.record_json FROM game_durability_reconnect_attempts a WHERE a.game_session_id=s.game_session_id ORDER BY a.control_loss_epoch DESC,a.reconnect_attempt_ref DESC LIMIT 1) AS record_json FROM game_durability_reconnect_sessions s LEFT JOIN game_durability_fresh_admission_receipts r ON r.replay_key=COALESCE(s.initial_fresh_replay_key,s.fresh_replay_key) WHERE s.game_session_id=encode($1,'hex')::uuid FOR UPDATE OF s")
        .bind(session.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    Ok((admission_generation(&row)?, row.try_get("revoked")?))
}

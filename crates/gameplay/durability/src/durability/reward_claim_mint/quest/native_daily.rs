use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};

fn available(last_claim: Option<i64>, boundary: i64) -> bool {
    last_claim.is_none_or(|last| last < boundary)
}

/// The quest writer already holds the session-generation-fenced Character root.
/// This write rolls back with a refused mint or any other failed quest participant.
pub(super) async fn admit(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    claim: &str,
) -> Result<Option<String>, DurabilityError> {
    // The channel's server-save calendar is TIBIA_SERVER_SAVE_BERLIN_06.
    // Truncating in local time keeps the save at 06:00 across DST changes.
    let (now, boundary, last): (i64, i64, Option<i64>) = sqlx::query_as(
        "WITH clock AS (SELECT game_native_daily_reward_clock() AS now)
         SELECT floor(extract(epoch FROM now)*1000)::bigint,
            floor(extract(epoch FROM ((date_trunc('day',
                (now AT TIME ZONE 'Europe/Berlin')-interval '6 hours')+interval '6 hours')
                AT TIME ZONE 'Europe/Berlin'))*1000)::bigint,
            greatest(
                (SELECT last_claimed_at FROM game_native_daily_reward_claims
                 WHERE character_id=encode($1,'hex')::uuid AND claim_key=$2),
                (SELECT claimed_at FROM game_reward_claims
                 WHERE character_id=encode($1,'hex')::uuid AND claim_family='RewardClaim'
                   AND claim_production_key=$2))
         FROM clock",
    )
    .bind(character.as_bytes().as_slice())
    .bind(claim)
    .fetch_one(&mut **tx)
    .await?;
    if !available(last, boundary) {
        return Ok(None);
    }
    // Preserve the canonical first receipt. Later mints use one identity per save day.
    let mint_key = if last.is_none() {
        claim.to_owned()
    } else {
        let digest = Sha256::new()
            .chain_update(b"oteryn:native-daily-claim:v1\0")
            .chain_update(boundary.to_be_bytes())
            .chain_update(claim.as_bytes())
            .finalize();
        let digest: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        format!("oteryn:reward-claim.daily.{digest}")
    };
    sqlx::query(
        "INSERT INTO game_native_daily_reward_claims(character_id,claim_key,last_claimed_at,mint_key,claimed_xact_id)
         VALUES(encode($1,'hex')::uuid,$2,$3,$4,pg_current_xact_id())
         ON CONFLICT(character_id,claim_key) DO UPDATE
         SET last_claimed_at=EXCLUDED.last_claimed_at,mint_key=EXCLUDED.mint_key,
             claimed_xact_id=EXCLUDED.claimed_xact_id",
    )
    .bind(character.as_bytes().as_slice())
    .bind(claim)
    .bind(now)
    .bind(&mint_key)
    .execute(&mut **tx)
    .await?;
    Ok(Some(mint_key))
}

#[cfg(test)]
mod tests {
    use super::available;

    #[test]
    fn native_daily_claim_reopens_at_the_next_boundary() {
        let last_claim = Some(1001);
        assert!(available(None, 1000));
        assert!(!available(last_claim, 1000));
        assert!(!available(last_claim, 1001));
        assert!(available(last_claim, 1002));
    }
}

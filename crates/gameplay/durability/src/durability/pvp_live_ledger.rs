//! Bounded 16-full / 64-compact victim ledger, under the already locked victim state.
use super::{LivePvpFacts, Result, refused};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Bucket {
    at_us: i64,
    damage: u64,
    assist: bool,
}

pub(super) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    facts: &LivePvpFacts,
    damage: u32,
) -> Result<()> {
    let now: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    sqlx::query("DELETE FROM game_character_pvp_ledger WHERE victim_character_id=encode($1,'hex')::uuid AND last_at<clock_timestamp()-interval '5 minutes'")
        .bind(facts.target.as_slice()).execute(&mut **tx).await?;
    let rows=sqlx::query("SELECT contributor_character_id::text AS id, buckets, compact FROM game_character_pvp_ledger WHERE victim_character_id=encode($1,'hex')::uuid ORDER BY last_at,contributor_character_id LIMIT 81 FOR UPDATE")
        .bind(facts.target.as_slice()).fetch_all(&mut **tx).await?;
    if rows.len() > 80 {
        return Err(refused());
    }
    let target = uuid(facts.attacker);
    let mut buckets = Vec::<Bucket>::new();
    let mut full = Vec::new();
    let mut compact = Vec::new();
    for row in rows {
        let id: String = row.try_get("id")?;
        if row.try_get::<bool, _>("compact")? {
            compact.push(id);
            continue;
        }
        let stored: Vec<Bucket> =
            serde_json::from_value(row.try_get("buckets")?).map_err(|_| refused())?;
        if stored.len() > 31 || stored.iter().any(|b| b.at_us > now) {
            return Err(refused());
        }
        let stored: Vec<_> = stored
            .into_iter()
            .filter(|b| b.at_us >= now - now % 10_000_000 - 300_000_000)
            .collect();
        let total = stored
            .iter()
            .try_fold(0_u64, |n, b| n.checked_add(b.damage))
            .ok_or_else(refused)?;
        if id == target {
            buckets = stored;
        }
        full.push((id, total));
    }
    if compact.contains(&target) {
        // Migration 0081 forbids deleting an unexpired contributor. Keep its Damage
        // identity rather than attempting an impossible immediate full/compact swap.
        sqlx::query("UPDATE game_character_pvp_ledger SET last_at=clock_timestamp(),compact_kind=1 WHERE victim_character_id=encode($1,'hex')::uuid AND contributor_character_id=$2::text::uuid")
            .bind(facts.target.as_slice()).bind(&target).execute(&mut **tx).await?;
        return Ok(());
    }
    let already_full = full.iter().any(|(id, _)| id == &target);
    if !already_full && full.len() >= 16 {
        if compact.len() >= 64 {
            // Basic combat refuses saturation; never evict a live contributor or
            // weaken the database's retention/cap guards to admit another attacker.
            return Err(refused());
        }
        // Stable tie break: rows already ordered by oldest last_at, then CharacterId.
        let least = full
            .iter()
            .enumerate()
            .min_by_key(|(index, (_, damage))| (*damage, *index))
            .map(|(_, v)| (v.0.clone(), if v.1 > 0 { 1_i16 } else { 2_i16 }))
            .ok_or_else(refused)?;
        sqlx::query("UPDATE game_character_pvp_ledger SET compact=true,compact_kind=$3,buckets='[]'::jsonb WHERE victim_character_id=encode($1,'hex')::uuid AND contributor_character_id=$2::text::uuid")
            .bind(facts.target.as_slice()).bind(least.0).bind(least.1).execute(&mut **tx).await?;
    }
    let bucket_at = now - now % 10_000_000;
    if let Some(bucket) = buckets.iter_mut().find(|b| b.at_us == bucket_at) {
        bucket.damage = bucket
            .damage
            .checked_add(u64::from(damage))
            .ok_or_else(refused)?;
    } else {
        buckets.push(Bucket {
            at_us: bucket_at,
            damage: u64::from(damage),
            assist: false,
        });
    }
    if buckets.len() > 31 {
        return Err(refused());
    }
    sqlx::query("INSERT INTO game_character_pvp_ledger(victim_character_id,contributor_character_id,last_at,buckets) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,clock_timestamp(),$3) ON CONFLICT(victim_character_id,contributor_character_id) DO UPDATE SET last_at=EXCLUDED.last_at,buckets=EXCLUDED.buckets,compact=false,compact_kind=NULL")
        .bind(facts.target.as_slice()).bind(facts.attacker.as_slice()).bind(serde_json::to_value(&buckets).map_err(|_|refused())?).execute(&mut **tx).await?;
    Ok(())
}
fn uuid(id: [u8; 16]) -> String {
    let h = id.iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}

//! Minimum PREY-0/D118 durable hunting state consumed by creature XP.
//! Slot absence is PREY-0 §6.5's uninitialized slot, never a synthetic active bonus.
use super::{DurabilityError, character_progression::CurrentCharacterGameplayFence};
use oteryn_gameplay_progression::daily_reward::xp_boost::{
    XpBoostGrant, XpBoostLines, active_xp_boost_lines,
};
use sqlx::{PgConnection, Postgres, Row, Transaction};

#[derive(Debug, Clone, Copy)]
pub struct HuntingXpModifiers {
    pub prey_slot: Option<i16>,
    pub prey_epoch: Option<i64>,
    pub prey_grade: Option<i16>,
    pub prey_percent: i16,
    pub xp_boost_percent: i16,
    pub stamina_minutes: i32,
    /// Level before this kill, for the Global low-level bonus.
    pub level: u32,
}

pub(super) struct HuntingActivity<'a> {
    pub death: &'a [u8],
    pub at_ms: i64,
    pub elapsed_ms: u64,
    pub earns_experience: bool,
}

/// Caller owns the fenced Character root lock before reading its typed hunting children.
pub(super) async fn read_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterGameplayFence,
    creature: &str,
    premium: bool,
    live_stamina: Option<u16>,
    activity: HuntingActivity<'_>,
) -> Result<HuntingXpModifiers, DurabilityError> {
    let stamina: Option<i32> = sqlx::query_scalar(
        "SELECT stamina_minutes FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    // The fenced root lock already serializes level writers; the XP commit locks this row later.
    // A Character without progression state has no XP commit to bonus; level 50 adds nothing.
    let level: Option<i64> = sqlx::query_scalar(
        "SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let stamina = stamina.unwrap_or(2520);
    let stamina = live_stamina.map_or(stamina, |live| stamina.min(i32::from(live)));
    // OFFLINE-0 §3 defines this absent-build-row seed. Reading it never creates a build
    // row or invents a hunting tick; build/stamina writers own that lifecycle.
    if activity.earns_experience {
        let elapsed = i64::try_from(activity.elapsed_ms).map_err(|_| DurabilityError::InvalidStoredState)?;
        sqlx::query(
            "SELECT game_consume_store_hunting_time($1, encode($2,'hex')::uuid,
                encode($3,'hex')::uuid, $4, $5, $6)",
        ).bind(activity.death).bind(fence.character_id.as_bytes().as_slice())
            .bind(fence.game_session_id.as_bytes().as_slice()).bind(activity.at_ms)
            .bind(elapsed.min(activity.at_ms)).bind(stamina).execute(&mut **tx).await?;
    }
    let mut result = HuntingXpModifiers {
        prey_slot: None,
        prey_epoch: None,
        prey_grade: None,
        prey_percent: 0,
        xp_boost_percent: xp_boost_lines(
            tx,
            fence.character_id.as_bytes().as_slice(),
            fence.runtime_scope.world_id().as_bytes().as_slice(),
            Some((activity.at_ms, stamina)),
        )
        .await?
        .percent(),
        stamina_minutes: stamina,
        level: level
            .map(u32::try_from)
            .transpose()
            .map_err(|_| DurabilityError::InvalidStoredState)?
            .unwrap_or(50),
    };
    let rows = sqlx::query(
        "SELECT p.slot,p.slot_epoch,p.grade,p.bonus_type,p.bonus_percent,p.state,\
         p.creature_key,p.creature_list,p.option,p.free_reroll_at,p.hunting_s_left FROM game_character_prey_slots p \
         WHERE p.character_id=encode($1,'hex')::uuid AND p.creature_key=$2 \
         AND p.state='ACTIVE' AND p.bonus_type='XP' AND p.hunting_s_left>0 \
         AND (p.slot=1 OR (p.slot=2 AND $3) OR EXISTS(SELECT 1 FROM game_character_permanent_prey_slots u \
         WHERE u.character_id=p.character_id AND u.target=p.slot)) \
         ORDER BY p.slot LIMIT 3",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(creature)
    .bind(premium)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if let Some(row) = rows.first() {
        let (slot, epoch, bonus) = super::prey::decode_xp_slot(row)?;
        result.prey_slot = Some(slot);
        result.prey_epoch = Some(epoch);
        result.prey_grade = Some(i16::from(bonus.grade));
        result.prey_percent = i16::from(bonus.percent);
    }
    Ok(result)
}

/// Winter Update 2026 (thread 565): the Daily Reward and Store XP boosts are separate
/// lines whose percentages add. Daily and legacy Store grants use the effect clock.
/// Purchased Store boosts use the remaining hunting time from committed kill awards.
pub(super) async fn xp_boost_lines(
    connection: &mut PgConnection,
    character: &[u8],
    world: &[u8],
    hunt: Option<(i64, i32)>,
) -> Result<XpBoostLines, DurabilityError> {
    let rows = sqlx::query(
        "WITH instant AS MATERIALIZED (SELECT public.game_daily_effect_now() AS at) \
         SELECT source,rank,percent,started_ms,expires_ms FROM ( \
           (SELECT 4 AS source,row_number() OVER (ORDER BY c.reset_epoch DESC,l.line DESC) AS rank,l.xp_boost_percent AS percent, \
              ceil(extract(epoch FROM (l.effect_started_at-instant.at))*1000)::bigint AS started_ms, \
              ceil(extract(epoch FROM (l.effect_expires_at-instant.at))*1000)::bigint AS expires_ms \
            FROM public.game_character_daily_reward_claims c \
            JOIN public.game_character_daily_reward_lines l ON l.claim_id=c.transaction_id,instant \
            WHERE c.character_id=encode($1,'hex')::uuid AND c.world_id=encode($2,'hex')::uuid \
              AND l.kind='XP_BOOST' AND l.status='DELIVERED' \
            ORDER BY c.reset_epoch DESC,l.line DESC LIMIT 2) \
           UNION ALL \
           (SELECT 3,row_number() OVER (ORDER BY s.expires_at DESC),s.percent, \
              ceil(extract(epoch FROM (s.starts_at-instant.at))*1000)::bigint, \
              ceil(extract(epoch FROM (s.expires_at-instant.at))*1000)::bigint \
            FROM public.game_character_store_xp_boosts s,instant \
            WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid \
              AND s.starts_at<=instant.at AND s.expires_at>instant.at \
            ORDER BY s.expires_at DESC LIMIT 8) \
           UNION ALL \
           SELECT 3,0::bigint,50::smallint,0::bigint,b.remaining_ms \
           FROM public.game_character_store_hunting_boosts b \
           WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid \
             AND b.remaining_ms>0 AND ($3::bigint IS NULL OR b.started_at_ms<=$3) \
             AND ($4::integer IS NULL OR $4>=840)) lines \
         ORDER BY source,rank",
    )
    .bind(character)
    .bind(world)
    .bind(hunt.map(|(at, _)| at))
    .bind(hunt.map(|(_, stamina)| stamina))
    .fetch_all(connection)
    .await?;
    let (mut daily, mut store) = (Vec::with_capacity(2), Vec::with_capacity(8));
    for row in rows {
        let grant = XpBoostGrant {
            started_ms: row.try_get("started_ms")?,
            expires_ms: row.try_get("expires_ms")?,
            percent: row.try_get("percent")?,
        };
        match row.try_get::<i32, _>("source")? {
            4 => daily.push(grant),
            3 => store.push(grant),
            _ => return Err(DurabilityError::InvalidStoredState),
        }
    }
    Ok(active_xp_boost_lines(&daily, &store, 0))
}

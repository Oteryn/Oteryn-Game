//! W22c Wealth Duplex: one use burns one unit through the item-use writer and starts a one-hour
//! effect row in the same transaction (migration 0582). The kill settle reads the effect from
//! the same server clock; a replayed CommandRef answers from its record and never burns twice.
//! Crystal ff7ede59 `data/libs/systems/concoctions.lua:179-199`, `config.lua.dist:310-311`.
//! W22b: the hour counts only while the Character is online (Crystal 201189f8
//! `config.lua.dist:310-312`, `concoctions.lua:126-152`), measured from its sessions' admission
//! and terminal stamps (migration 0596). The 24-hour cooldown stays on the wall clock
//! (`concoctions.lua:181`).

use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::*;
use crate::domain::CharacterId;

pub mod kooldown_aid;

pub const CONCOCTION_CAUSE: u32 = 527;
pub const WEALTH_DUPLEX_KEY: &str = "oteryn:item.tibia.i36727";
/// The online hour of one activation (Crystal `config.lua.dist:311`,
/// `tibiadromeConcoctionDuration = 60 * 60`).
pub const WEALTH_DUPLEX_ONLINE_MICROS: i64 = 3_600_000_000;

/// Crystal `concoctions.lua:180-185`: refused while the last activation is inside its 24-hour
/// cooldown (`config.lua.dist:310`); nothing is burned.
pub(super) async fn admit(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
) -> Result<()> {
    if request.source_definition.production_key != WEALTH_DUPLEX_KEY {
        return Err(ItemUseError::InvalidInput);
    }
    let cooling: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_concoction_effects \
          WHERE character_id=encode($1,'hex')::uuid AND concoction='wealth_duplex' \
            AND started_at+interval '24 hours'>public.game_daily_effect_now())",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(connection)
    .await?;
    if cooling {
        return Err(ItemUseError::Cooldown);
    }
    Ok(())
}

/// The effect row of the burn `request` just wrote.
pub(super) async fn start(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO game_character_concoction_effects\
         (transaction_id,character_id,concoction,started_at,expires_at) \
         SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,'wealth_duplex',at,at+interval '1 hour' \
           FROM (SELECT public.game_daily_effect_now() AS at) instant",
    )
    .bind(request.transaction_id.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .execute(connection)
    .await?;
    Ok(())
}

impl DurabilityRoot {
    /// Whether `character_id` has a live Wealth Duplex now: its latest activation has spent less
    /// than one hour online. Crystal `concoctions.lua:179-199` resets the time left on each
    /// activation, so only the latest one counts.
    pub async fn wealth_duplex_active(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<bool> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let active: bool = sqlx::query_scalar(
                        "WITH instant AS (SELECT public.game_daily_effect_now() AS at), \
                         latest AS (SELECT e.character_id, e.started_at \
                           FROM game_character_concoction_effects e, instant \
                          WHERE e.character_id=encode($1,'hex')::uuid \
                            AND e.concoction='wealth_duplex' AND e.started_at<=instant.at \
                          ORDER BY e.started_at DESC LIMIT 1) \
                         SELECT COALESCE((SELECT public.game_character_online_us(\
                             latest.character_id, latest.started_at, instant.at) < $2 \
                           FROM latest, instant), false)",
                    )
                    .bind(character_id.as_bytes().as_slice())
                    .bind(WEALTH_DUPLEX_ONLINE_MICROS)
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(active))
                })
            })
            .await?
    }
}

/// One authoritative current effect, grouped by source/percentage for overlapping XP grants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivePotionDuration {
    pub source: u8,
    pub item_id: u32,
    pub percent: u16,
    pub remaining_ms: u64,
}
impl DurabilityRoot {
    /// Current timers from the same semantic clock as effect consumption. Wealth Duplex
    /// spends online microseconds; daily/store XP deadlines spend wall-clock time. The
    /// active set is bounded by one Wealth timer, one store percentage and four daily tiers.
    pub async fn read_active_potion_durations(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &super::super::runtime_scope_assignment::NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        origin_channel: [u8; 16],
    ) -> Result<Vec<ActivePotionDuration>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            super::super::db::lock_admission_relations(&mut tx).await?;
            let current = match fence.runtime_scope {
                crate::foundation::RuntimeScopeRefV1::Instance { .. } => super::super::house_hireling::fence::assert_house_custody_fence(&mut tx, &node, &fence, origin_channel).await?.is_some(),
                _ => super::super::house_hireling::fence::assert_inventory_fence(&mut tx, &node, &fence, origin_channel).await?,
            };
            if !current { return Ok(Err(ItemUseError::AuthorityRejected)); }
            let rows = sqlx::query(
                "WITH instant AS MATERIALIZED (SELECT public.game_daily_effect_now() AS at), \
                 latest AS (SELECT e.character_id,e.started_at FROM game_character_concoction_effects e,instant \
                   WHERE e.character_id=encode($1,'hex')::uuid AND e.concoction='wealth_duplex' \
                     AND e.started_at<=instant.at ORDER BY e.started_at DESC LIMIT 1), \
                 wealth AS (SELECT 1 AS source,36727 AS item_id,0::smallint AS percent, \
                   ($3-public.game_character_online_us(latest.character_id,latest.started_at,instant.at))/1000 AS remaining_ms \
                   FROM latest,instant), \
                 boosts AS (SELECT 3 AS source,percent,expires_at FROM game_character_store_xp_boosts s,instant \
                   WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid \
                     AND s.starts_at<=instant.at AND s.expires_at>instant.at \
                   UNION ALL SELECT 4 AS source,l.xp_boost_percent AS percent,l.effect_expires_at AS expires_at \
                   FROM game_character_daily_reward_claims c JOIN game_character_daily_reward_lines l ON l.claim_id=c.transaction_id,instant \
                   WHERE c.character_id=encode($1,'hex')::uuid AND c.world_id=encode($2,'hex')::uuid \
                     AND l.kind='XP_BOOST' AND l.status='DELIVERED' AND l.xp_boost_percent>0 \
                     AND l.effect_started_at<=instant.at AND l.effect_expires_at>instant.at), \
                 xp AS (SELECT source,0 AS item_id,percent, \
                   ceil(extract(epoch FROM (max(expires_at)-instant.at))*1000)::bigint AS remaining_ms \
                   FROM boosts,instant GROUP BY source,percent,instant.at) \
                 SELECT source,item_id,percent,remaining_ms FROM (SELECT * FROM wealth UNION ALL SELECT * FROM xp) effects \
                   WHERE remaining_ms>0 ORDER BY source,item_id,percent LIMIT 6")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .bind(WEALTH_DUPLEX_ONLINE_MICROS).fetch_all(&mut *tx).await?;
            let mut active = Vec::with_capacity(rows.len());
            for row in rows {
                active.push(ActivePotionDuration {
                    source: u8::try_from(row.try_get::<i32,_>("source")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                    item_id: u32::try_from(row.try_get::<i32,_>("item_id")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                    percent: u16::try_from(row.try_get::<i16,_>("percent")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                    remaining_ms: u64::try_from(row.try_get::<i64,_>("remaining_ms")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                });
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(active))
        })).await?
    }
}

//! W09FX adds the remaining eighteen effects from Crystal 201189f8
//! `data/scripts/systems/concoctions.lua:7-37` to the same canonical activation owner.
//!
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
pub mod stamina_extension;

use oteryn_gameplay_items::concoctions::Concoction;

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
    let kind = Concoction::from_production_key(&request.source_definition.production_key)
        .filter(|kind| !matches!(kind, Concoction::KooldownAid | Concoction::StaminaExtension))
        .ok_or(ItemUseError::InvalidInput)?;
    admit_kind(connection, fence, kind).await
}

pub(super) async fn admit_kind(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    kind: Concoction,
) -> Result<()> {
    let cooling: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_concoction_effects \
          WHERE character_id=encode($1,'hex')::uuid AND concoction=$2 \
            AND started_at+interval '24 hours'>public.game_daily_effect_now())",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(kind.effect_key())
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
    let kind = Concoction::from_production_key(&request.source_definition.production_key)
        .ok_or(ItemUseError::InvalidInput)?;
    sqlx::query(
        "INSERT INTO game_character_concoction_effects\
         (transaction_id,character_id,concoction,started_at,expires_at) \
         SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,at,at+interval '1 hour' \
           FROM (SELECT public.game_daily_effect_now() AS at) instant",
    )
    .bind(request.transaction_id.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(kind.effect_key())
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

/// One authoritative current effect; the Store and Daily Reward XP boosts are one line each.
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
    /// active set is bounded by eighteen concoction timers, one Store line and one Daily Reward line.
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
            let mut active = active_concoctions(&mut tx, fence.character_id).await?;
            // Winter Update 2026: source 3 (Store) and 4 (Daily Reward) are separate lines.
            let lines = super::super::hunting_xp::xp_boost_lines(&mut tx, fence.character_id.as_bytes().as_slice(), fence.runtime_scope.world_id().as_bytes().as_slice()).await?;
            for (source, line) in [(3, lines.store), (4, lines.daily)] {
                if let Some(line) = line {
                    active.push(ActivePotionDuration {
                        source,
                        item_id: 0,
                        percent: u16::try_from(line.percent).map_err(|_| DurabilityError::InvalidStoredState)?,
                        remaining_ms: u64::try_from(line.remaining_ms).map_err(|_| DurabilityError::InvalidStoredState)?,
                    });
                }
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(active))
        })).await?
    }
}

/// Crystal config.lua.dist:310-312, libs/systems/concoctions.lua:126-152:
/// latest activation per kind, one hour of session time; cooldowns stay on wall time.
/// Twenty registered identities bound the result independently of activation history.
pub(in crate::durability) async fn active_concoctions(
    connection: &mut PgConnection,
    character: CharacterId,
) -> std::result::Result<Vec<ActivePotionDuration>, sqlx::Error> {
    let rows = sqlx::query(
        "WITH instant AS MATERIALIZED (SELECT public.game_daily_effect_now() AS at), \
         latest AS (SELECT DISTINCT ON(e.concoction) e.concoction,e.character_id,e.started_at \
           FROM game_character_concoction_effects e,instant \
          WHERE e.character_id=encode($1,'hex')::uuid AND e.started_at<=instant.at \
            AND e.concoction NOT IN ('kooldown_aid','stamina_extension') ORDER BY e.concoction,e.started_at DESC), \
         remaining AS (SELECT concoction,($2-public.game_character_online_us( \
             latest.character_id,latest.started_at,instant.at))/1000 AS ms FROM latest,instant) \
         SELECT concoction,ms FROM remaining WHERE ms>0 LIMIT 20",
    ).bind(character.as_bytes().as_slice()).bind(WEALTH_DUPLEX_ONLINE_MICROS)
     .fetch_all(connection).await?;
    rows.into_iter().map(|row| {
        let key: String = row.try_get("concoction")?;
        let kind = Concoction::from_effect_key(&key)
            .ok_or_else(|| sqlx::Error::Decode("unknown concoction".into()))?;
        Ok(ActivePotionDuration { source: 1, item_id: kind.item_id(), percent: 0,
            remaining_ms: u64::try_from(row.try_get::<i64,_>("ms")?)
                .map_err(|_| sqlx::Error::Decode("concoction duration".into()))? })
    }).collect()
}

//! W22c Wealth Duplex: one use burns one unit through the item-use writer and starts a one-hour
//! effect row in the same transaction (migration 0582). The kill settle reads the effect from
//! the same server clock; a replayed CommandRef answers from its record and never burns twice.
//! Crystal ff7ede59 `data/libs/systems/concoctions.lua:179-199`, `config.lua.dist:310-311`.
//! W22b: the hour counts only while the Character is online (Crystal 201189f8
//! `config.lua.dist:310-312`, `concoctions.lua:126-152`), measured from its sessions' admission
//! and terminal stamps (migration 0596). The 24-hour cooldown stays on the wall clock
//! (`concoctions.lua:181`).

use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
    apply_build_in_transaction,
};
use super::super::character_progression::{CurrentCharacterGameplayFence, assert_session_fence};
use super::super::offline_training_state::{self, OfflineChange};
use super::*;
use oteryn_gameplay_characters::stamina::MAX_STAMINA_MINUTES;
use oteryn_gameplay_items::concoctions::Concoction;
use crate::domain::CharacterId;

pub mod kooldown_aid;

pub const CONCOCTION_CAUSE: u32 = 527;
pub const WEALTH_DUPLEX_KEY: &str = Concoction::WealthDuplex.production_key();
pub const STAMINA_EXTENSION_KEY: &str = Concoction::StaminaExtension.production_key();
/// Crystal 201189f8 data/scripts/systems/concoctions.lua:7-13.
const STAMINA_EXTENSION_MINUTES: u16 = 60;

pub fn stamina_extension_after(before: u16) -> u16 {
    before.saturating_add(STAMINA_EXTENSION_MINUTES).min(MAX_STAMINA_MINUTES as u16)
}
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
    let kind = match request.kind {
        ItemUseKind::Concoction if request.source_definition.production_key == WEALTH_DUPLEX_KEY => "wealth_duplex",
        ItemUseKind::StaminaExtension { .. } => "stamina_extension",
        _ => return Err(ItemUseError::InvalidInput),
    };
    let cooling: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_concoction_effects \
          WHERE character_id=encode($1,'hex')::uuid AND concoction=$2 \
            AND started_at+interval '24 hours'>public.game_daily_effect_now())",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(kind)
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

/// Keep the canonical build chain contiguous. The guarded activation identifies this
/// capped gain by the same transaction ID and proves it shares the item burn's SQL transaction.
pub(super) async fn extend_stamina(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    item: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
    revision: u64,
) -> Result<()> {
    let ItemUseKind::StaminaExtension { wire_binding, minutes_before } = request.kind else {
        return Err(ItemUseError::InvalidInput);
    };
    let character = item.character_id.as_bytes();
    let before = offline_training_state::load(tx, character).await?;
    if before.stamina_minutes != minutes_before {
        return Err(ItemUseError::StaleState);
    }
    let minutes_after = stamina_extension_after(minutes_before);
    if minutes_after > minutes_before {
        let row = sqlx::query("SELECT * FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_slice()).fetch_one(&mut **tx).await?;
        let build = DurableBuildState::stored(&row, "vocation", "")?;
        let fence = CurrentCharacterGameplayFence {
            character_id: item.character_id,
            game_session_id: item.game_session_id,
            connection_generation: item.connection_generation,
            character_lease_generation: item.character_lease_generation,
            runtime_scope: item.runtime_scope,
            scope_ownership_generation: item.scope_ownership_generation,
            expected_character_revision: crate::domain::CharacterRevision::new(revision)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        };
        // Use the canonical stamina-gain writer so later checkpoints, death and logout
        // consume the same build projection. The activation receipt owns the gameplay cause.
        let change = BuildChangeRequest {
            occurrence: BuildOccurrence::from_bytes(request.transaction_id)
                .map_err(|_| ItemUseError::InvalidInput)?,
            cause: BuildCause::RestingStamina,
            after: build.clone(),
            before: build,
            pruned_stance: None,
        };
        let outcome = apply_build_in_transaction(
            tx, node, &fence, change, Sha256::digest(b"stamina-extension-v1").into(),
            request.content_revision.clone(), request.binding().to_vec(),
            Some(OfflineChange {
                before,
                after: offline_training_state::OfflineTrainingState {
                    stamina_minutes: minutes_after,
                    ..before
                },
            }),
        ).await?;
        match outcome {
            Ok(BuildCommitOutcome::Committed(_)) => {}
            Ok(BuildCommitOutcome::AlreadyCommitted(_)) => return Err(ItemUseError::ConflictingCommand),
            Err(CharacterProgressionError::Unavailable(error)) => return Err(error.into()),
            Err(_) => return Err(ItemUseError::StaleState),
        }
    }
    sqlx::query(
        "INSERT INTO game_character_concoction_effects \
         (transaction_id,character_id,concoction,started_at,expires_at,wire_binding,stamina_before,stamina_after) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'stamina_extension',statement_timestamp(),statement_timestamp(),$3,$4,$5)",
    )
    .bind(request.transaction_id.as_slice())
    .bind(character.as_slice())
    .bind(wire_binding.as_slice())
    .bind(i32::from(minutes_before))
    .bind(i32::from(minutes_after))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

impl DurabilityRoot {
    /// Look up an exact carried USE even after the item or the local command memo is gone.
    pub async fn replay_stamina_extension(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command_id: u64,
        wire_binding: [u8; 32],
    ) -> Result<bool> {
        let recovery = authority.record_for(self).map_err(|_| ItemUseError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let stored: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT e.wire_binding FROM game_character_concoction_effects e \
                 JOIN game_item_use_records u USING(transaction_id) \
                 WHERE u.game_session_id=encode($1,'hex')::uuid AND u.command_id=$2::text::numeric \
                   AND e.character_id=encode($3,'hex')::uuid AND e.concoction='stamina_extension'",
            )
            .bind(fence.game_session_id.as_bytes().as_slice())
            .bind(command_id.to_string())
            .bind(fence.character_id.as_bytes().as_slice())
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(stored) = stored {
                lock_admission_relations(&mut tx).await?;
                match assert_session_fence(&mut tx, &fence, &node).await? {
                    Ok(_) => {}
                    Err(CharacterProgressionError::Unavailable(error)) => return Err(error),
                    Err(_) => return Ok(Err(ItemUseError::AuthorityRejected)),
                }
                if stored != wire_binding {
                    return Ok(Err(ItemUseError::ConflictingCommand));
                }
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(true));
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(false))
        })).await?
    }

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
    /// active set is bounded by one Wealth timer, one Store line and one Daily Reward line.
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
                   FROM latest,instant) \
                 SELECT source,item_id,percent,remaining_ms FROM wealth WHERE remaining_ms>0 LIMIT 1")
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

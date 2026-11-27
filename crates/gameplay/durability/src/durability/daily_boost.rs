//! World-owned selection, serialized against server-save activation and fenced by the Channel owner.
use super::character_authority::ReconciledCharacterAuthority;
use super::character_progression::numeric_u64;
use super::db::begin_type2_transaction;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::RuntimeScopeRefV1;
use oteryn_gameplay_content::content::daily_boost::{DailyBoostSelection, select_daily_boost};
use sqlx::Row;

pub struct DailyBoostBinding {
    pub scope: RuntimeScopeRefV1,
    pub generation: u64,
    pub bundle: [u8; 32],
    pub epoch: u64,
}

impl DurabilityRoot {
    /// Load before drawing: restart and concurrent Channels always use the original durable row.
    pub async fn prepare_daily_boost(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        binding: DailyBoostBinding,
        pools: (Vec<String>, Vec<String>),
    ) -> Result<DailyBoostSelection, DurabilityError> {
        let mut context = (authority, node, permit);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let (authority, node, permit) = context;
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::spell_item_transaction::assert_spell_item_scope_in_transaction(
                &mut tx, permit, self, authority, node, binding.scope, binding.generation,
            ).await.map_err(|_| DurabilityError::Unavailable)?;
            // Exclusive pointer lock serializes the first draw across Channels and conflicts
            // with activation. The old epoch cannot create rows during retirement.
            let active = sqlx::query("SELECT a.reset_epoch::text,e.target_digest FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid FOR UPDATE OF a")
                .bind(binding.scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            if !active.as_ref().is_some_and(|row| numeric_u64(row,"reset_epoch").ok() == Some(binding.epoch)
                && row.try_get::<Vec<u8>,_>("target_digest").ok().as_deref() == Some(binding.bundle.as_slice())) {
                return Err(DurabilityError::Unavailable);
            }
            let retiring: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND phase='RETIRING')")
                .bind(binding.scope.world_id().as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if retiring { return Err(DurabilityError::Unavailable); }
            let current = sqlx::query("SELECT creature_key,boss_key FROM game_world_daily_boosts WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
                .bind(binding.scope.world_id().as_bytes().as_slice()).bind(binding.epoch.to_string()).fetch_optional(&mut **tx).await?;
            let selection = if let Some(row) = current {
                DailyBoostSelection { reset_epoch: binding.epoch, creature: row.try_get("creature_key")?, boss: row.try_get("boss_key")? }
            } else {
                // Bound the result by the supplied pools, regardless of the number of resets.
                // Epochs are not days: every selection in the last 30 days is ineligible.
                let eligible = sqlx::query("SELECT ARRAY(SELECT key FROM unnest($3::text[]) AS key WHERE NOT EXISTS(SELECT 1 FROM game_world_daily_boosts h WHERE h.world_id=encode($1,'hex')::uuid AND h.reset_epoch<$2::text::numeric AND h.selected_at>=CURRENT_TIMESTAMP-INTERVAL '30 days' AND h.creature_key=key)) AS creatures, ARRAY(SELECT key FROM unnest($4::text[]) AS key WHERE NOT EXISTS(SELECT 1 FROM game_world_daily_boosts h WHERE h.world_id=encode($1,'hex')::uuid AND h.reset_epoch<$2::text::numeric AND h.selected_at>=CURRENT_TIMESTAMP-INTERVAL '30 days' AND h.boss_key=key)) AS bosses")
                    .bind(binding.scope.world_id().as_bytes().as_slice()).bind(binding.epoch.to_string())
                    .bind(&pools.0).bind(&pools.1).fetch_one(&mut **tx).await?;
                let creatures: Vec<String> = eligible.try_get("creatures")?;
                let bosses: Vec<String> = eligible.try_get("bosses")?;
                let selection = select_daily_boost(binding.scope.world_id().as_bytes(), binding.epoch, creatures, bosses)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                if [&selection.creature, &selection.boss].iter().any(|key| key.is_empty() || key.len() > 255) {
                    return Err(DurabilityError::InvalidStoredState);
                }
                sqlx::query("INSERT INTO game_world_daily_boosts(world_id,reset_epoch,creature_key,boss_key) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4)")
                    .bind(binding.scope.world_id().as_bytes().as_slice()).bind(binding.epoch.to_string()).bind(&selection.creature).bind(&selection.boss).execute(&mut **tx).await?;
                selection
            };
            tx.commit(deadline).await?;
            Ok(selection)
        })).await
    }
}

pub struct WorldLoginInfo {
    pub players_online: u32,
    pub boost: Option<WorldLoginBoost>,
}

pub struct WorldLoginBoost {
    pub creature_key: String,
    pub boss_key: String,
    pub reset_epoch: u64,
    pub effective_from: i64,
    pub effective_until: i64,
}

impl DurabilityRoot {
    /// Public aggregate for the locally served World. It never opens a session
    /// or initializes a boost. The census and active selection share one SQL cut.
    pub async fn read_world_login_info(
        &self,
        world: crate::foundation::WorldId,
        bundle: Option<[u8; 32]>,
        epoch: u64,
    ) -> Result<WorldLoginInfo, DurabilityError> {
        const MAX_ONLINE: i64 =
            oteryn_gameplay_native_admission_source::public_projections::MAX_ONLINE as i64;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
            let row = sqlx::query(
                "WITH requested AS (SELECT encode($1,'hex')::uuid AS world_id),
                census AS (
                    SELECT count(*) AS players_online FROM (
                        SELECT 1 FROM requested w
                        JOIN game_durability_admission_character_guards g USING (world_id)
                        JOIN game_durability_reconnect_sessions s ON s.game_session_id=g.holder_game_session_id
                        JOIN game_character_roots c ON c.character_id=g.character_id AND c.world_id=w.world_id
                        WHERE g.eligible AND s.world_id=w.world_id AND s.character_id=g.character_id
                          AND s.account_id=g.account_id AND c.account_id=g.account_id
                          AND s.character_lease_generation=g.lease_generation
                          AND s.session_state=2 AND s.current_transport_ref IS NOT NULL
                          AND c.lifecycle=1 AND NOT game_account_closure_detached(g.character_id)
                        LIMIT $2
                    ) bounded
                )
                SELECT players_online,b.creature_key,b.boss_key,b.effective_from,b.effective_until
                FROM census CROSS JOIN requested w LEFT JOIN LATERAL (
                    SELECT d.creature_key,d.boss_key,
                        floor(extract(epoch FROM e.activated_at)*1000)::bigint AS effective_from,
                        floor(extract(epoch FROM boundary.effective_until)*1000)::bigint AS effective_until
                    FROM game_world_reset_active a
                    JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
                    JOIN game_world_daily_boosts d USING(world_id,reset_epoch)
                    CROSS JOIN LATERAL (
                        SELECT game_server_save_at(game_server_save_day(e.activated_at,e.legacy_six_am)+1) AS effective_until
                    ) boundary
                    WHERE e.activated_at <= statement_timestamp()
                      AND statement_timestamp() < boundary.effective_until
                      AND a.world_id=w.world_id AND e.target_digest=$3 AND e.reset_epoch=$4::text::numeric
                      AND e.calendar='TIBIA_SERVER_SAVE_BERLIN_06'
                      AND NOT EXISTS (SELECT 1 FROM game_world_reset_jobs j WHERE j.world_id=w.world_id AND j.phase='RETIRING')
                ) b ON true",
            ).bind(world.as_bytes().as_slice()).bind(MAX_ONLINE+1)
                .bind(bundle.as_ref().map(<[u8; 32]>::as_slice)).bind(epoch.to_string())
                .fetch_one(&mut *tx).await?;
            let count: i64 = row.try_get("players_online")?;
            if count > MAX_ONLINE {
                return Err(DurabilityError::Unavailable);
            }
            let players_online = u32::try_from(count).map_err(|_| DurabilityError::InvalidStoredState)?;
            let boost = match row.try_get::<Option<String>, _>("creature_key")? {
                Some(creature_key) => Some(WorldLoginBoost {
                    creature_key,
                    boss_key: row.try_get("boss_key")?,
                    reset_epoch: epoch,
                    effective_from: row.try_get("effective_from")?,
                    effective_until: row.try_get("effective_until")?,
                }),
                None => None,
            };
            Ok(WorldLoginInfo { players_online, boost })
        })).await
    }
}

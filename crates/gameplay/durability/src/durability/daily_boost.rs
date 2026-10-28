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
                let previous = sqlx::query("SELECT creature_key,boss_key FROM game_world_daily_boosts WHERE world_id=encode($1,'hex')::uuid AND reset_epoch<$2::text::numeric ORDER BY reset_epoch DESC LIMIT 1")
                    .bind(binding.scope.world_id().as_bytes().as_slice()).bind(binding.epoch.to_string()).fetch_optional(&mut **tx).await?;
                let (mut creatures, mut bosses) = pools;
                if let Some(row) = previous {
                    let creature: String = row.try_get("creature_key")?;
                    let boss: String = row.try_get("boss_key")?;
                    // Crystal excludes yesterday's race. A single eligible race remains usable
                    // in small content sets instead of creating an impossible draw.
                    if creatures.iter().any(|key| key != &creature) { creatures.retain(|key| key != &creature); }
                    if bosses.iter().any(|key| key != &boss) { bosses.retain(|key| key != &boss); }
                }
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

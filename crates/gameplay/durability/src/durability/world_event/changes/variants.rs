//! The current reset decision, Character fence, and visit bookkeeping share one transaction.
use super::*;
use crate::durability::{
    character_progression::{assert_item_only_fence, assert_runtime_scope_fence},
    runtime_scope_assignment::NodeIncarnationProof,
};
use crate::foundation::{
    CharacterId, CommandRef, CreatureDeathOccurrenceKey, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use oteryn_gameplay_world_change::variants::Position;

pub const SPIDER_BOSS: &str = "oteryn:creature.mamma_longlegs";
// Tibia Fandom archive: Warpath Mini World Change/Spoiler, revision 2022-07-31.
const WARPATH_COOLDOWN_MS: i64 = 24 * 60 * 60 * 1000;
// Tibia Fandom archive: Spider Nest Mini World Change/Spoiler, revision 2021-10-04.
const SPIDER_LIFETIME_KILLS: i64 = 3;

#[derive(Clone, Copy)]
pub struct PassageStep {
    pub epoch: u64,
    pub position: Position,
    pub command: CommandRef,
    pub binding: [u8; 32],
}

#[derive(Clone, Copy, Debug)]
pub struct SpiderDeath {
    epoch: u64,
    occurrence: CreatureDeathOccurrenceKey,
    home: Position,
    principal: CharacterId,
}

impl SpiderDeath {
    pub fn capture(
        runtime: &mut oteryn_gameplay_entities::carrier::ChannelRuntimeV1,
        actor: oteryn_gameplay_entities::carrier::ExactActorRef,
        principal: CharacterId,
        epoch: u64,
    ) -> Option<Self> {
        let ("spider_nest", generation, home) = runtime.world_spawn_cohort(actor)? else {
            return None;
        };
        if generation != epoch {
            return None;
        }
        let owner = runtime.borrow_combat_death();
        if owner.creature_target_identity(actor).ok()? != SPIDER_BOSS.as_bytes() {
            return None;
        }
        let (occurrence, _) = owner.projected_death(actor).ok()?;
        let home = (u16::try_from(home.x).ok()?, u16::try_from(home.y).ok()?, i8::try_from(home.floor).ok()?);
        Some(Self { epoch, occurrence, home, principal })
    }
}

impl DurabilityRoot {
    pub async fn daily_variant_passage(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        step: PassageStep,
    ) -> Result<Option<Position>, CharacterProgressionError> {
        if step.command.game_session_id() != fence.game_session_id
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(refusal) = assert_item_only_fence(&mut tx, &fence, &node).await? {
                return Ok(Err(refusal));
            }
            let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND session_state=2 AND current_transport_ref IS NOT NULL)")
                .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if !active {
                return Ok(Err(CharacterProgressionError::AuthorityRejected));
            }
            let world = fence.runtime_scope.world_id();
            let Some(day) = load(&mut tx, world, step.epoch).await? else { return Ok(Ok(None)); };
            let Some((definition, portal)) = day.variants.passage(step.position) else {
                return Ok(Ok(None));
            };
            // The retained passages are public. Unsupported content fails closed.
            if portal.premium || portal.promoted || portal.min_level != 0 {
                return Err(DurabilityError::InvalidStoredState);
            }
            let admitted = if portal.exit || !matches!(definition.key.as_str(), "warpath" | "spider_nest") {
                true
            } else {
                record_entry(&mut tx, fence, step, &definition.key).await?
            };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(admitted.then_some(portal.destination)))
        })).await?
    }

    pub async fn record_spider_nest_death(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        death: SpiderDeath,
    ) -> Result<(), DurabilityError> {
        if !matches!(scope, RuntimeScopeRefV1::Channel { .. })
            || scope != death.occurrence.runtime_scope()
            || generation != death.occurrence.scope_ownership_generation()
        {
            return Ok(());
        }
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                return Err(DurabilityError::Unavailable);
            }
            let world = scope.world_id();
            let Some(day) = load(&mut tx, world, death.epoch).await? else {
                return Ok(());
            };
            let from_selected_home = day.variants.active().any(|(definition, variant)| {
                definition.key == "spider_nest" && variant.spawns.iter().any(|spawn| {
                    spawn.position == death.home && spawn.creature == SPIDER_BOSS
                })
            });
            if from_selected_home {
                record_spider_death(&mut tx, world, death).await?;
            }
            commit_semantic_transaction(tx, deadline).await
        })).await
    }
}

async fn record_entry(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    step: PassageStep,
    key: &str,
) -> Result<bool, DurabilityError> {
    let world = fence.runtime_scope.world_id();
    let session = fence.game_session_id.as_bytes().as_slice();
    let character = fence.character_id.as_bytes().as_slice();
    let command = step.command.command_id().get().to_string();
    let epoch = step.epoch.to_string();
    let previous: Option<(bool, bool)> = sqlx::query_as("SELECT admitted,character_id=encode($3,'hex')::uuid AND world_id=encode($4,'hex')::uuid AND reset_epoch=$5::text::numeric AND source_hash=$6 FROM game_daily_variant_entries WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(session).bind(&command).bind(character).bind(world.as_bytes().as_slice()).bind(&epoch).bind(step.binding.as_slice())
        .fetch_optional(&mut **tx).await?;
    if let Some((admitted, matches)) = previous {
        return Ok(admitted && matches);
    }

    let admitted = match key {
        "warpath" => {
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&mut **tx).await?;
            let previous: Option<i64> = sqlx::query_scalar("SELECT last_entry_attempt_ms FROM game_daily_variant_warpath WHERE character_id=encode($1,'hex')::uuid")
                .bind(character).fetch_optional(&mut **tx).await?;
            sqlx::query("INSERT INTO game_daily_variant_warpath VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(character_id) DO UPDATE SET last_entry_attempt_ms=greatest(game_daily_variant_warpath.last_entry_attempt_ms,excluded.last_entry_attempt_ms)")
                .bind(character).bind(now).execute(&mut **tx).await?;
            previous.is_none_or(|previous| now.saturating_sub(previous) >= WARPATH_COOLDOWN_MS)
        }
        "spider_nest" => {
            let deaths: i64 = sqlx::query_scalar("SELECT coalesce(sum(deaths),0)::bigint FROM game_daily_variant_spider_progress WHERE character_id=encode($1,'hex')::uuid")
                .bind(character).fetch_one(&mut **tx).await?;
            if deaths >= SPIDER_LIFETIME_KILLS {
                false
            } else {
                sqlx::query("INSERT INTO game_daily_variant_spider_progress(world_id,reset_epoch,character_id,visits) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,1) ON CONFLICT(character_id,world_id,reset_epoch) DO UPDATE SET visits=game_daily_variant_spider_progress.visits+1")
                    .bind(world.as_bytes().as_slice()).bind(&epoch).bind(character).execute(&mut **tx).await?;
                true
            }
        }
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    sqlx::query("INSERT INTO game_daily_variant_entries VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6,$7)")
        .bind(session).bind(command).bind(character).bind(world.as_bytes().as_slice()).bind(epoch).bind(step.binding.as_slice()).bind(admitted).execute(&mut **tx).await?;
    Ok(admitted)
}

async fn record_spider_death(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    death: SpiderDeath,
) -> Result<(), DurabilityError> {
    let character = death.principal.as_bytes().as_slice();
    let world = world.as_bytes().as_slice();
    let epoch = death.epoch.to_string();
    let occurrence = crate::durability::party_kill_xp::encode_death_key(death.occurrence);
    // Identical root lock order to entry and the existing World credit writer.
    let root: Option<i32> = sqlx::query_scalar("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(character).bind(world).fetch_optional(&mut **tx).await?;
    if root.is_none() {
        return Ok(());
    }
    let first = sqlx::query("INSERT INTO game_daily_variant_spider_deaths(world_id,reset_epoch,occurrence,character_id) VALUES(encode($2,'hex')::uuid,$4::text::numeric,$3,encode($1,'hex')::uuid) ON CONFLICT DO NOTHING")
        .bind(character).bind(world).bind(occurrence).bind(&epoch).execute(&mut **tx).await?.rows_affected() == 1;
    if first {
        sqlx::query("INSERT INTO game_daily_variant_spider_progress(world_id,reset_epoch,character_id,deaths) VALUES(encode($2,'hex')::uuid,$3::text::numeric,encode($1,'hex')::uuid,1) ON CONFLICT(character_id,world_id,reset_epoch) DO UPDATE SET deaths=least(game_daily_variant_spider_progress.deaths+1,3)")
            .bind(character).bind(world).bind(epoch).execute(&mut **tx).await?;
    }
    Ok(())
}

pub(in crate::durability) async fn desert_fishing_admitted(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    gathering: &crate::durability::item_craft::GatheringCraft,
) -> Result<bool, DurabilityError> {
    use oteryn_gameplay_items::fishing::{
        DESERT_FISHING_RECOVERY_MS, DESERT_FISHING_SPENT, DESERT_FISHING_WATER, SANDFISH,
    };
    let mutation = &gathering.mutation;
    if mutation.tool_key != oteryn_gameplay_items::fishing::FISHING_ROD
        || !gathering.attempt.consumes_bait
        || !matches!(gathering.attempt.yield_item, None | Some(SANDFISH))
        || gathering.attempt.after != Some(DESERT_FISHING_SPENT)
        || mutation.after != u32::from(DESERT_FISHING_SPENT)
        || mutation.decay != [(u32::from(DESERT_FISHING_WATER), i64::from(DESERT_FISHING_RECOVERY_MS))]
    {
        return Ok(false);
    }
    let Ok(epoch) = u64::try_from(gathering.scope.reset_epoch) else {
        return Ok(false);
    };
    let Some(position) = mutation.placement.get(32..).filter(|_| mutation.placement.len() == 40)
        .and_then(|key| key.try_into().ok()).map(u64::from_be_bytes)
    else {
        return Ok(false);
    };
    if position >> 48 != 0 || position & 255 > 63 {
        return Ok(false);
    }
    let Ok(floor) = i8::try_from((position >> 8) & 255) else {
        return Ok(false);
    };
    let position = ((position >> 32) as u16, (position >> 16) as u16, floor);
    let Some(day) = load(tx, world, epoch).await? else {
        return Ok(false);
    };
    Ok(day.variants.active().any(|(definition, variant)| {
        definition.key == "river_runs_deep" && variant.tiles.iter().any(|tile| {
            tile.position == position && tile.entries.iter().any(|entry| entry.0 == DESERT_FISHING_WATER)
        })
    }))
}

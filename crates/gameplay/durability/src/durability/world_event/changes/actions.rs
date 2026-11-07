//! WCONTRIB: live horse/Noodles effects under the existing World changes owner.
//! Quest actions join its fenced receipt transaction; ticks join the existing raid scheduler.
use super::*;
mod horse;
pub(crate) use horse::{lock_horse_projection,record_horse_deaths};
use crate::durability::character_progression::{CurrentCharacterGameplayFence, assert_gameplay_fence};
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_gameplay_world_change::contribution::{HorseStationState, NoodlesState, SharedChanges};
use serde::{Deserialize, Serialize};

pub const STABLE_HORSES: u16 = 9;
pub const HORSE_ESCAPE: &str = "oteryn:quest-transition/world-action/horse/escape";
pub const HORSE_RETURN: &str = "oteryn:quest-transition/world-action/horse/return";
pub const HORSE_RENT_AVAILABLE: &str = "oteryn:quest-transition/world-action/horse/rent-available";
pub const NOODLES_ESCORT: &str = "oteryn:quest-transition/world-action/noodles/escort";
pub const NOODLES_RETURN: &str = "oteryn:quest-transition/world-action/noodles/return";

pub fn recognizes(key: &str) -> bool {
    matches!(key, HORSE_ESCAPE|HORSE_RETURN|HORSE_RENT_AVAILABLE|NOODLES_ESCORT|NOODLES_RETURN)
}

pub fn horse_projection(key: &str) -> bool {
    key == "raid:horsestation.wild" || stable_projection_count(key).is_some()
}

fn stable_projection_count(key: &str) -> Option<u16> {
    let count=key.strip_prefix("raid:horsestation.stable.")?;
    (count.len()==1).then_some(())?;
    count.parse::<u16>().ok().filter(|count|(1..=STABLE_HORSES).contains(count))
}

pub fn projection_matches(key: &str, state: &HorseStationState, now_ms: i64) -> bool {
    if key == "raid:horsestation.wild" {
        state.raid_active(now_ms)
    } else {
        stable_projection_count(key).is_some_and(|count| !state.raid_active(now_ms) && state.stable_horses==count)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveChanges {
    pub horse: HorseStationState,
    pub noodles: NoodlesState,
    pub escort_character: Option<[u8;16]>,
}

impl LiveChanges {
    fn from_saved(saved: SharedChanges) -> Self {
        let mut horse = saved.horse;
        if horse.stable_horses == 0 && horse.escape_started_ms.is_none() && horse.raid_until_ms.is_none() {
            horse.stable_horses = STABLE_HORSES;
        }
        Self { horse, noodles: saved.noodles, escort_character: None }
    }

    pub fn overlay(self, mut saved: SharedChanges) -> SharedChanges {
        saved.horse = self.horse;
        saved.noodles = self.noodles;
        saved
    }

    fn advance(&mut self, now_ms: i64) {
        let before = self.horse.raid_until_ms;
        self.horse.advance(now_ms, STABLE_HORSES);
        // The donor clears both stables when the raid starts, including a timer start.
        if before.is_none() && self.horse.raid_until_ms.is_some() {
            self.horse.stable_horses = 0;
        }
    }
}

struct Snapshot {
    revision: u64,
    state: LiveChanges,
    updated_at_ms: i64,
}

async fn snapshot(
    tx: &mut Transaction<'_, Postgres>, world: WorldId, epoch: u64,
) -> Result<Option<Snapshot>, DurabilityError> {
    let Some(saved) = super::shared(tx, world, epoch).await? else { return Ok(None); };
    let row: Option<(String, serde_json::Value, i64)> = sqlx::query_as(
        "SELECT revision::text,state,updated_at_ms FROM game_world_change_live \
         WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric",
    ).bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_optional(&mut **tx).await?;
    row.map(|(revision,state,updated_at_ms)| {
        Ok(Snapshot {
            revision: revision.parse().map_err(|_|DurabilityError::InvalidStoredState)?,
            state: serde_json::from_value(state).map_err(|_|DurabilityError::InvalidStoredState)?,
            updated_at_ms,
        })
    }).transpose().map(|stored| stored.or_else(||Some(Snapshot {
        revision: 0, state: LiveChanges::from_saved(saved), updated_at_ms: 0,
    })))
}

/// Historical overlay for the existing fenced server-save transaction only.
/// It cannot authorize current gameplay; immutable save projection remains its caller.
pub(super) async fn for_save(
    tx: &mut Transaction<'_, Postgres>, world: WorldId, epoch: u64,
) -> Result<Option<LiveChanges>, DurabilityError> {
    let state: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT state FROM game_world_change_live WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric",
    ).bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_optional(&mut **tx).await?;
    state.map(|state|serde_json::from_value(state).map_err(|_|DurabilityError::InvalidStoredState)).transpose()
}

/// Read-only current-epoch overlay. Admission/mutation authority belongs to the caller.
pub async fn load(
    tx: &mut Transaction<'_, Postgres>, world: WorldId, epoch: u64,
) -> Result<Option<LiveChanges>, DurabilityError> {
    Ok(snapshot(tx,world,epoch).await?.map(|value|value.state))
}

/// Paid travel re-evaluates the timer in its own fenced fee/arrival transaction.
pub async fn horse_state(
    tx: &mut Transaction<'_, Postgres>, world: WorldId, epoch: u64, now_ms: i64,
) -> Result<Option<HorseStationState>, DurabilityError> {
    Ok(load(tx,world,epoch).await?.map(|mut live| {
        live.advance(now_ms);
        live.horse
    }))
}

async fn lock(
    tx: &mut Transaction<'_, Postgres>, world: WorldId,
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:world-change-live:'||encode($1,'hex'),0))")
        .bind(world.as_bytes().as_slice()).execute(&mut **tx).await?;
    Ok(())
}

enum Proof<'a> {
    Quest { character: &'a [u8;16], session: &'a [u8;16], ordinal: u64, transition: &'a str },
    Tick { node: &'a NodeIncarnationProof, channel: ChannelId, generation: ScopeOwnershipGeneration },
}

/// The caller already holds the existing Quest Character/session/revision fences.
/// The deferred FK makes its exact Quest receipt inseparable from this shared action.
pub(crate) async fn apply_quest(
    tx: &mut Transaction<'_, Postgres>, recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof, fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    context: Option<&WorldActionContext>,
) -> Result<bool, DurabilityError> {
    if !recognizes(&request.transition_key) {
        return Ok(true);
    }
    let (QuestCause::Command(command)|QuestCause::Use(command))=request.cause else {return Ok(false);};
    if command.game_session_id()!=fence.game_session_id
        || assert_gameplay_fence(tx,&fence,node).await?.is_err()
        || !super::custody::current_session(tx,node,fence).await? {
        return Ok(false);
    }
    assert_recovery_fence(tx,recovery).await?;
    let world=fence.runtime_scope.world_id();
    let epoch: Option<String>=sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(epoch)=epoch else {return Ok(false);};
    let epoch=epoch.parse().map_err(|_|DurabilityError::InvalidStoredState)?;
    lock(tx,world).await?;
    let Some(before)=snapshot(tx,world,epoch).await? else {return Ok(false);};
    let now_ms:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
        .fetch_one(&mut **tx).await?;
    if now_ms<before.updated_at_ms {return Err(DurabilityError::InvalidStoredState);}
    let replay: Option<bool>=sqlx::query_scalar("SELECT transition_key=$4 AND world_id=encode($5,'hex')::uuid AND reset_epoch=$6::text::numeric FROM game_world_change_action_receipts WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric")
        .bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string()).bind(&request.transition_key)
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_optional(&mut **tx).await?;
    if let Some(matches)=replay {return Ok(matches);}
    let mut after=before.state.clone();
    after.advance(now_ms);
    match request.transition_key.as_str() {
        HORSE_ESCAPE => {
            if !after.horse.escape(now_ms) {return Ok(false);}
            after.advance(now_ms);
        }
        HORSE_RETURN => {
            if after.horse.stable_horses>=STABLE_HORSES
                || (after.horse.escape_started_ms.is_none() && !after.horse.raid_active(now_ms)) {
                return Ok(false);
            }
            after.horse.stable_horses+=1;
            // Authored conservative return rule: all nine restored horses end the raid.
            if after.horse.stable_horses==STABLE_HORSES {
                after.horse.raid_until_ms=None;
                after.horse.escape_started_ms=None;
            }
        }
        HORSE_RENT_AVAILABLE => {
            // Availability only: the existing rental owner retains money and mount grants.
            if after.horse.raid_active(now_ms) || after.horse.stable_horses==0 {return Ok(false);}
        }
        NOODLES_ESCORT => {
            let last: Option<i64> = sqlx::query_scalar("SELECT updated_at_ms FROM game_world_change_action_receipts WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2 ORDER BY updated_at_ms DESC LIMIT 1")
                .bind(fence.character_id.as_bytes().as_slice()).bind(NOODLES_RETURN).fetch_optional(&mut **tx).await?;
            if last.is_some_and(|last| now_ms.saturating_sub(last) < 14 * 24 * 60 * 60 * 1000) { return Ok(false); }
            if !context.is_some_and(|c| c.valid_target("oteryn:npc.noodles")) { return Ok(false); }
            if !after.noodles.missing || after.escort_character.is_some()
                || !has_collar(tx, fence).await? { return Ok(false); }
            after.escort_character = Some(*fence.character_id.as_bytes());
        }
        NOODLES_RETURN => {
            if !context.is_some_and(|c| c.valid_target("oteryn:npc.king_tibianus")
                && c.target_position.floor == -6
                && c.target_position.x.abs_diff(32311) <= 1
                && c.target_position.y.abs_diff(32172) <= 1) { return Ok(false); }
            if after.escort_character != Some(*fence.character_id.as_bytes())
                || !has_collar(tx, fence).await?
                || !after.noodles.return_to_palace() { return Ok(false); }
            after.escort_character = None;
        }
        _=>return Ok(false),
    }
    if after!=before.state {
        write(tx,world,epoch,&before,&after,now_ms,Proof::Quest {
            character:fence.character_id.as_bytes(), session:command.game_session_id().as_bytes(),
            ordinal:command.command_id().get(), transition:&request.transition_key,
        }).await?;
    }
    Ok(true)
}

/// This joins check_raid_schedules after current node/reset/Channel-generation proof.
/// The same scheduler performs the ordinary raid and stable actor projections.
pub(crate) async fn tick(
    tx: &mut Transaction<'_, Postgres>, world: WorldId, epoch: u64,
    node: &NodeIncarnationProof, channel: ChannelId, generation: ScopeOwnershipGeneration,
    now_ms: i64,
) -> Result<Option<LiveChanges>, DurabilityError> {
    lock(tx,world).await?;
    let Some(before)=snapshot(tx,world,epoch).await? else {return Ok(None);};
    if now_ms<before.updated_at_ms {return Err(DurabilityError::InvalidStoredState);}
    let mut after=before.state.clone();
    after.advance(now_ms);
    if after!=before.state {
        write(tx,world,epoch,&before,&after,now_ms,Proof::Tick {node,channel,generation}).await?;
    }
    Ok(Some(after))
}

async fn write(
    tx: &mut Transaction<'_, Postgres>, world: WorldId, epoch: u64,
    before: &Snapshot, after: &LiveChanges, now_ms: i64, proof: Proof<'_>,
) -> Result<(), DurabilityError> {
    let revision=before.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
    let (character,session,ordinal,transition,node_id,node_revision,channel,generation)=match proof {
        Proof::Quest{character,session,ordinal,transition}=>(
            Some(character.to_vec()),Some(session.to_vec()),Some(ordinal.to_string()),Some(transition),
            None,None,None,None,
        ),
        Proof::Tick{node,channel,generation}=>{
            let fact=node.fact();
            (None,None,None,None,Some(fact.node_id().as_bytes().to_vec()),
                Some(fact.registration_revision().to_string()),Some(channel.as_bytes().to_vec()),Some(generation.get().to_string()))
        }
    };
    let before_state=serde_json::to_value(&before.state).map_err(|_|DurabilityError::InvalidStoredState)?;
    let after_state=serde_json::to_value(after).map_err(|_|DurabilityError::InvalidStoredState)?;
    let binding:[u8;32]=Sha256::digest(serde_json::to_vec(&serde_json::json!({
        "version":1,"world":world.as_bytes(),"epoch":epoch,"revision":revision,
        "character":character,"session":session,"command":ordinal,"transition":transition,
        "node":node_id,"registration":node_revision,"channel":channel,"generation":generation,
        "before":before_state,"after":after_state,"now_ms":now_ms,
    })).map_err(|_|DurabilityError::InvalidStoredState)?).into();
    sqlx::query("INSERT INTO game_world_change_action_receipts(world_id,reset_epoch,revision,operation_id,request_binding,before_state,after_state,updated_at_ms,character_id,game_session_id,cause_ordinal,transition_key,node_id,node_registration_revision,channel_id,scope_generation) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3::text::numeric,$4,$5,$6,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11::text::numeric,$12,encode($13,'hex')::uuid,$14::text::numeric,encode($15,'hex')::uuid,$16::text::numeric)")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(revision.to_string())
        .bind(&binding[..16]).bind(binding.as_slice()).bind(&before_state).bind(&after_state).bind(now_ms)
        .bind(character).bind(session).bind(ordinal).bind(transition).bind(node_id).bind(node_revision)
        .bind(channel).bind(generation).execute(&mut **tx).await?;
    // BEFORE INSERT guards run even for ON CONFLICT UPDATE. Admit the initial
    // revision once, then advance the locked row through the UPDATE-only chain.
    let query = if before.revision == 0 {
        "INSERT INTO game_world_change_live(world_id,reset_epoch,revision,state,updated_at_ms) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3::text::numeric,$4,$5)"
    } else {
        "UPDATE game_world_change_live SET revision=$3::text::numeric,state=$4,updated_at_ms=$5 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND revision=$3::text::numeric-1"
    };
    let changed = sqlx::query(query)
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(revision.to_string())
        .bind(after_state).bind(now_ms).execute(&mut **tx).await?;
    if changed.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
    Ok(())
}

/// Evidence is carried Item custody, never client item ID or dialogue text.
async fn has_collar(tx: &mut Transaction<'_, Postgres>, fence: CurrentCharacterGameplayFence) -> Result<bool, DurabilityError> {
    let item: Option<String> = sqlx::query_scalar("SELECT i.item_instance_id::text FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i14156' AND i.quantity=1 AND c.kind IN ('container','nested','equipment') ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    Ok(item.is_some())
}

/// This gates earning; an already committed pending reward stays deliverable.
pub(crate) async fn collar_available(tx: &mut Transaction<'_, Postgres>, fence: CurrentCharacterGameplayFence) -> Result<bool, DurabilityError> {
    let world = fence.runtime_scope.world_id();
    let epoch: Option<String> = sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(epoch) = epoch else { return Ok(false); };
    lock(tx, world).await?;
    let epoch = epoch.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(load(tx, world, epoch).await?.is_some_and(|live| live.noodles.missing && live.escort_character.is_none()))
}

impl DurabilityRoot {
    pub async fn live_world_changes(&self, world: WorldId, epoch: u64) -> Result<Option<LiveChanges>, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let live = load(&mut tx, world, epoch).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(live)
        })).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldActionContext {
    pub actor: oteryn_gameplay_quests::quest::world_callbacks::Position,
    pub target_npc: String,
    pub target_position: oteryn_gameplay_quests::quest::world_callbacks::Position,
}
impl WorldActionContext {
    fn valid_target(&self, expected: &str) -> bool {
        self.target_npc == expected && self.actor.floor == self.target_position.floor
            && self.actor.x.abs_diff(self.target_position.x) <= 1
            && self.actor.y.abs_diff(self.target_position.y) <= 1
    }
}

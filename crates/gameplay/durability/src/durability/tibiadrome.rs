//! W17 rules: docs/reference/tibia-manual/combat.md:249-284 and
//! docs/reference/content-prep/arena-rules-content-1/admission-evidence/
//! tibiadrome-reference-evidence.json:10-852. Crystal 201189f and Canary
//! 47dfd51 contain no Drome lifecycle. Choices below fill only those missing owners.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use serde::{Deserialize, Serialize};
use sqlx::Row;

pub const ROTATION_SECONDS: u32 = 14 * 24 * 3600;
pub const PREPARATION_SECONDS: u32 = 8;
pub const WAVE_SECONDS: u32 = 120;
/// ARENA1-RL-08: reaching this wave ends the battle before it is spawned.
pub const MAX_WAVE: u32 = 300;
pub fn monster_count(players: usize) -> Option<usize> {
    [4, 8, 11, 14, 17].get(players.checked_sub(1)?).copied()
}
pub fn eligible(level: u32, premium: bool) -> bool {
    level >= 50 && premium
}
pub fn rank_potions(rank: u32) -> u32 {
    match rank {
        1..=5 => 3,
        6..=10 => 2,
        11..=20 => 1,
        _ => 0,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// None is an explicitly unknown legacy baseline; never infer historical levels.
    #[serde(default)]
    pub lifetime_levels: Option<u64>,
    pub rotation: u32,
    pub level: u32,
    pub start: u32,
    pub points: u32,
    pub cooldown_until: u32,
    pub waves: u32,
}
impl Default for Progress {
    fn default() -> Self {
        Self {
            lifetime_levels: Some(0),
            rotation: 0,
            level: 0,
            start: 0,
            points: 0,
            cooldown_until: 0,
            waves: 0,
        }
    }
}
impl Progress {
    pub fn wave(&mut self, cleared_wave: u32) {
        if cleared_wave >= MAX_WAVE {
            return;
        }
        let level = cleared_wave + 1;
        self.reached_level(level);
        self.start = self.start.max(level);
        self.waves = self.waves.saturating_add(1);
    }
    /// Advance the level and preserve an explicitly unknown lifetime baseline.
    pub fn reached_level(&mut self, level: u32) {
        let gained = u64::from(level.saturating_sub(self.level));
        if let Some(lifetime) = &mut self.lifetime_levels {
            *lifetime = lifetime.saturating_add(gained);
        }
        self.level = self.level.max(level);
    }
    pub fn finish(&mut self, now: u32) {
        self.start = self.start.saturating_sub(5);
        self.cooldown_until = now.saturating_add(3600);
    }
    pub fn rotate(&mut self, rotation: u32) -> u32 {
        if rotation <= self.rotation {
            return 0;
        }
        let points = if self.waves > 0 {
            self.level.saturating_mul(10)
        } else {
            0
        };
        self.points = self.points.saturating_add(points);
        self.start = self
            .start
            .saturating_sub(rotation.saturating_sub(self.rotation).saturating_mul(5));
        self.rotation = rotation;
        self.level = 0;
        self.waves = 0;
        points
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub progress: Progress,
    pub session: [u8; 16],
    pub run: [u8; 16],
    pub generation: u64,
    pub wave: u32,
    pub completed: u32,
    pub active: bool,
    #[serde(default)]
    pub preparing: bool,
    pub expires: u32,
}
#[derive(Debug)]
pub struct Read {
    pub potion: Option<(u32, u32, u16)>,
    pub rotation_points_eligible: bool,
    pub state: Record,
    pub now: u32,
    pub rank: u32,
    pub potions: u32,
    pub replay: Option<Vec<u8>>,
}

impl DurabilityRoot {
    pub async fn read_tibiadrome(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        payload: &[u8],
    ) -> Result<Read, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let payload = payload.to_vec();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let now = now(&mut tx).await?;
                    let state = load(&mut tx, fence).await?;
                    let rotation_points_eligible = state.progress.rotation
                        >= now / ROTATION_SECONDS
                        || rotation_eligible(&mut tx, fence.character_id, state.progress.rotation)
                            .await?;
                    let replay = replay(&mut tx, fence, command, &payload).await?;
                    publish(&mut tx, fence.character_id, now / ROTATION_SECONDS).await?;
                    let potion = next_potion(&mut tx, fence).await?;
                    let (rank, potions) = standing(&mut tx, fence, now / ROTATION_SECONDS).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Read {
                        potion,
                        rotation_points_eligible,
                        state,
                        now,
                        rank,
                        potions,
                        replay,
                    }))
                })
            })
            .await?
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "fenced Drome transition and its immutable command receipt"
    )]
    pub async fn commit_tibiadrome(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        payload: Vec<u8>,
        before: Record,
        after: Record,
        disposition: u32,
        cosmetics: Option<crate::content::native_gameplay::NativeCosmeticsView>,
        hazards: Option<oteryn_protocol_oteryn::arena::DromeHazards>,
    ) -> Result<Vec<u8>, CharacterProgressionError> {
        if command.game_session_id() != fence.game_session_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error)=assert_gameplay_fence(&mut tx,&fence,&node).await? {return Ok(Err(error));}
            if let Some(reply)=replay(&mut tx,fence,command,&payload).await? {
                commit_semantic_transaction(tx,deadline).await?; return Ok(Ok(reply));
            }
            if load(&mut tx,fence).await?!=before {return Ok(Err(CharacterProgressionError::ConflictingOccurrence));}
            let now=now(&mut tx).await?;
            let rotation=now/ROTATION_SECONDS;
            let mut earned=before.progress.clone();
            if earned.rotation < rotation && !rotation_eligible(&mut tx, fence.character_id, earned.rotation).await? { earned.waves=0; }
            earned.rotate(rotation);
            let expected_lifetime=earned.lifetime_levels.map(|levels|levels.saturating_add(u64::from(after.progress.level.saturating_sub(earned.level))));
            if after.progress.rotation!=rotation || after.progress.points!=earned.points
                || after.progress.lifetime_levels!=expected_lifetime
                || (after.active && (after.session!=*fence.game_session_id.as_bytes() || after.generation!=fence.scope_ownership_generation.get())) {
                return Err(DurabilityError::InvalidStoredState);
            }
            // Rank snapshots are immutable once their rotation ended. The unique World/rotation
            // advisory lock serializes first publication without locking other Characters' roots.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,973))")
                .bind(format!("{:?}/{rotation}",fence.runtime_scope.world_id())).execute(&mut *tx).await?;
            publish(&mut tx,fence.character_id,rotation).await?;
            if after.progress.level>0 && after.progress.waves>0 {
                sqlx::query("INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience) SELECT r.world_id,$2,p.character_id,$3,p.total_experience FROM game_character_progression_state p JOIN game_character_roots r USING(character_id) WHERE p.character_id=encode($1,'hex')::uuid ON CONFLICT(world_id,rotation,character_id) DO UPDATE SET level=EXCLUDED.level,experience=EXCLUDED.experience WHERE EXCLUDED.level>game_tibiadrome_scores.level")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(i64::from(rotation)).bind(i64::from(after.progress.level)).execute(&mut *tx).await?;
            }
            let mut after=after;
            let mut disposition=disposition;
            if let Ok(oteryn_protocol_oteryn::arena::ArenaAction::Drome(oteryn_protocol_oteryn::arena::DromeAction::Buy(selector @ 17..=18)))=oteryn_protocol_oteryn::arena::decode_match_request(&payload)
                && !after.active && disposition==1 {
                disposition=grant_cosmetic(&mut tx,fence,command,&mut after,selector,cosmetics.as_ref()).await?;
            }
            let (rank,potions)=standing(&mut tx,fence,rotation).await?;
            let reply=oteryn_protocol_oteryn::arena::DromeResult {
                disposition,level:after.progress.level,start:after.progress.start,points:after.progress.points,
                phase:if after.active {if after.preparing {1}else{2}}else{0},wave:after.wave,remaining:if after.active {after.expires.saturating_sub(now)}else{0},
                cooldown:after.progress.cooldown_until.saturating_sub(now),rank,potions,
                rotation_ends:rotation.saturating_add(1).saturating_mul(ROTATION_SECONDS),
            }.encode();
            let reply=if after.active {
                if let Some(hazards)=&hazards {oteryn_protocol_oteryn::arena::append_drome_hazards(reply,hazards).map_err(|_|DurabilityError::InvalidStoredState)?} else {reply}
            } else {reply};
            save(&mut tx,fence,command,&payload,&after,&reply).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(reply))
        })).await?
    }
}

pub(super) async fn now(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<u32, DurabilityError> {
    let seconds: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    u32::try_from(seconds).map_err(|_| DurabilityError::InvalidStoredState)
}
pub(super) async fn load(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
) -> Result<Record, DurabilityError> {
    let body:Option<serde_json::Value>=sqlx::query_scalar("SELECT body FROM game_character_tibiadrome WHERE character_id=encode($1,'hex')::uuid FOR UPDATE").bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    body.map(serde_json::from_value)
        .transpose()
        .map(|v| v.unwrap_or_default())
        .map_err(|_| DurabilityError::InvalidStoredState)
}
pub(super) async fn replay(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    payload: &[u8],
) -> Result<Option<Vec<u8>>, DurabilityError> {
    let row=sqlx::query("SELECT payload,reply FROM game_tibiadrome_receipts WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric")
        .bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    if let Some(row) = row {
        if row.try_get::<Vec<u8>, _>("payload")? != payload {
            return Err(DurabilityError::InvalidStoredState);
        }
        return Ok(Some(row.try_get("reply")?));
    }
    Ok(None)
}
pub(super) async fn save(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    payload: &[u8],
    state: &Record,
    reply: &[u8],
) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_tibiadrome_receipts(character_id,game_session_id,command_id,payload,reply) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(payload).bind(reply).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_character_tibiadrome(character_id,body) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(character_id) DO UPDATE SET body=EXCLUDED.body")
        .bind(fence.character_id.as_bytes().as_slice()).bind(serde_json::to_value(state).map_err(|_|DurabilityError::InvalidStoredState)?).execute(&mut **tx).await?;
    Ok(())
}
// Trading eligibility at the rotation boundary: manual combat.md:263. The
// existing immutable Bazaar transitions (0994_bazaar_authority.sql:45-58) retain
// lifecycle history; Bazaar's decided_at is Unix seconds (bazaar/mod.rs:276).
// A later unlock cannot recover an expired rotation, and a later listing cannot
// forfeit an earlier rotation. Rotation boundaries belong to the new rotation.
async fn rotation_eligible(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: crate::domain::CharacterId,
    rotation: u32,
) -> Result<bool, DurabilityError> {
    sqlx::query_scalar("SELECT game_tibiadrome_lifecycle_at_cut(encode($1,'hex')::uuid,$2)=1")
        .bind(character.as_bytes().as_slice())
        .bind((i64::from(rotation) + 1) * i64::from(ROTATION_SECONDS))
        .fetch_one(&mut **tx)
        .await
        .map_err(Into::into)
}
pub(super) async fn standing(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    rotation: u32,
) -> Result<(u32, u32), DurabilityError> {
    let row=sqlx::query("SELECT COALESCE((SELECT rank FROM (SELECT s.character_id,row_number() OVER(ORDER BY s.level DESC,s.experience,s.character_id) AS rank FROM game_tibiadrome_scores s JOIN game_character_roots r USING(character_id) WHERE s.world_id=encode($2,'hex')::uuid AND s.rotation=$3 AND r.world_id=s.world_id AND r.lifecycle=1 AND NOT game_account_closure_detached(s.character_id)) ranked WHERE character_id=encode($1,'hex')::uuid),0)::bigint AS rank,COALESCE((SELECT sum(potions-claimed) FROM game_tibiadrome_awards WHERE character_id=encode($1,'hex')::uuid),0)::bigint AS potions")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(i64::from(rotation)).fetch_one(&mut **tx).await?;
    Ok((
        u32::try_from(row.try_get::<i64, _>("rank")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        u32::try_from(row.try_get::<i64, _>("potions")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    ))
}
pub(super) async fn publish(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: crate::domain::CharacterId,
    rotation: u32,
) -> Result<(), DurabilityError> {
    let world: Vec<u8>=sqlx::query_scalar("SELECT uuid_send(world_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,973))")
        .bind(format!("drome-rotation:{world:?}"))
        .execute(&mut **tx)
        .await?;
    let pending:Option<i64>=sqlx::query_scalar("SELECT min(rotation) FROM game_tibiadrome_scores WHERE world_id=encode($1,'hex')::uuid AND rotation>$3 AND rotation<$2")
        .bind(world.as_slice()).bind(i64::from(rotation))
        .bind(sqlx::query_scalar::<_,i64>("SELECT COALESCE(max(rotation),-1) FROM game_tibiadrome_rotations WHERE world_id=encode($1,'hex')::uuid").bind(world.as_slice()).fetch_one(&mut **tx).await?)
        .fetch_one(&mut **tx).await?;
    let Some(pending) = pending else {
        return Ok(());
    };
    // oteryn-choice: epoch-aligned fortnight boundaries and UUID final tie-break keep
    // rotation/replay deterministic where the captured sources do not specify an anchor.
    // Manual combat.md:284: twenty ranked winners and exactly one fifth of
    // all remaining participants; a large rotation must not truncate that share.
    // oteryn-choice: a stable exponential-race draw implements weighted sampling
    // without replacement; capacity refusals and repeat reads never reroll a prize.
    sqlx::query("WITH ranked AS (SELECT world_id,rotation,character_id,level,row_number() OVER(ORDER BY level DESC,experience,character_id) AS rank FROM game_tibiadrome_scores s WHERE world_id=encode($1,'hex')::uuid AND rotation=$2 AND game_tibiadrome_lifecycle_at_cut(s.character_id,$3)=1), top AS (SELECT * FROM ranked WHERE rank<=20 ORDER BY rank LIMIT 20), raffle AS (SELECT * FROM ranked WHERE rank>20 ORDER BY -ln((('x'||substr(md5(character_id::text||rotation::text),1,8))::bit(32)::bigint+1)::double precision/4294967297.0)/level,character_id LIMIT (SELECT count(*)/5 FROM ranked WHERE rank>20)) INSERT INTO game_tibiadrome_awards(world_id,rotation,character_id,rank,potions) SELECT world_id,rotation,character_id,rank,CASE WHEN rank<=5 THEN 3 WHEN rank<=10 THEN 2 ELSE 1 END FROM top UNION ALL SELECT world_id,rotation,character_id,rank,1 FROM raffle ON CONFLICT DO NOTHING")
        .bind(world.as_slice()).bind(pending).bind((pending+1)*i64::from(ROTATION_SECONDS)).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_tibiadrome_rotations(world_id,rotation) VALUES(encode($1,'hex')::uuid,$2)").bind(world.as_slice()).bind(pending).execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
mod tests;

// Crystal 201189f data/scripts/actions/items/drome_cube.lua:3-24 admits the twenty
// special potions. oteryn-choice: stable per-award selection prevents capacity rerolls.
pub(super) async fn next_potion(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
) -> Result<Option<(u32, u32, u16)>, DurabilityError> {
    let row=sqlx::query("SELECT rotation,claimed FROM game_tibiadrome_awards WHERE character_id=encode($1,'hex')::uuid AND claimed<potions ORDER BY rotation,world_id LIMIT 1 FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        use sha2::{Digest, Sha256};
        let rotation = u32::try_from(row.try_get::<i64, _>("rotation")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let claimed = u32::try_from(row.try_get::<i32, _>("claimed")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let hash = Sha256::new()
            .chain_update(fence.character_id.as_bytes())
            .chain_update(rotation.to_le_bytes())
            .chain_update(claimed.to_le_bytes())
            .finalize();
        Ok((rotation, claimed, 36723 + u16::from(hash[0] % 20)))
    })
    .transpose()
}

pub struct RunUpdate {
    pub run: [u8; 16],
    pub completed: u32,
    pub level: u32,
    pub ended: bool,
    /// Actual owner wave retained when the terminal runtime state is removed.
    pub wave: u32,
}

impl DurabilityRoot {
    /// Server cadence and actor-end settlement. The active run and completed ordinal make
    /// retries idempotent without borrowing a client's command-id sequence.
    pub async fn settle_tibiadrome(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        update: RunUpdate,
    ) -> Result<(), CharacterProgressionError> {
        let RunUpdate {
            run,
            completed,
            level,
            ended,
            wave,
        } = update;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            if let Err(error)=assert_gameplay_fence(&mut tx,&fence,&node).await?{return Ok(Err(error));}
            let mut state=load(&mut tx,fence).await?;
            if state.active && state.run==run && state.session==*fence.game_session_id.as_bytes() && state.generation==fence.scope_ownership_generation.get(){
                let now=now(&mut tx).await?;
                if state.progress.rotation==now/ROTATION_SECONDS {
                    if completed>state.completed {
                        let count=completed-state.completed;
                        state.progress.reached_level(level);
                        state.progress.start=state.progress.start.saturating_add(count).max(level);
                        state.progress.waves=state.progress.waves.saturating_add(count);
                    }
                    // A newer clear carries its actual physical wave; an older
                    // snapshot cannot rewind a wave already acknowledged here.
                    if completed>state.completed || ended {
                        state.wave=wave;
                    } else if completed==state.completed {
                        state.wave=state.wave.max(wave);
                    }
                    state.completed=state.completed.max(completed);
                    if state.progress.waves>0 {
                        sqlx::query("INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience) SELECT r.world_id,$2,p.character_id,$3,p.total_experience FROM game_character_progression_state p JOIN game_character_roots r USING(character_id) WHERE p.character_id=encode($1,'hex')::uuid ON CONFLICT(world_id,rotation,character_id) DO UPDATE SET level=EXCLUDED.level,experience=EXCLUDED.experience WHERE EXCLUDED.level>game_tibiadrome_scores.level")
                            .bind(fence.character_id.as_bytes().as_slice()).bind(i64::from(state.progress.rotation)).bind(i64::from(state.progress.level)).execute(&mut *tx).await?;
                    }
                }
                if ended || state.progress.rotation!=now/ROTATION_SECONDS {
                    state.wave=wave;state.active=false;state.progress.finish(now);
                }
                if state.progress.rotation < now/ROTATION_SECONDS && !rotation_eligible(&mut tx, fence.character_id, state.progress.rotation).await? { state.progress.waves=0; }
                state.progress.rotate(now/ROTATION_SECONDS);
                sqlx::query("UPDATE game_character_tibiadrome SET body=$2 WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(serde_json::to_value(state).map_err(|_|DurabilityError::InvalidStoredState)?).execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx,deadline).await?;Ok(Ok(()))
        })).await?
    }
}

// Costs: captured tibiadrome-reference-evidence.json:291-817; identities: Crystal
// 201189f data/items/items.xml:46581-46605,58905-58910,62655-62662,
// 68592-68600,68657-68671. The Cube is drome_cube.lua:1.
// Phant and the Lion of War base use the explicit native cosmetic grant owner below.
pub fn shop_reward(selector: u8) -> Option<(u16, u32)> {
    const REWARDS: [(u16, u32); 16] = [
        (36827, 2500),
        (36833, 250),
        (23489, 400),
        (23490, 750),
        (23491, 1500),
        (32944, 2500),
        (36869, 2500),
        (36867, 2500),
        (36868, 2500),
        (36865, 2500),
        (36866, 2500),
        (32945, 2500),
        (23492, 3000),
        (31472, 5000),
        (31470, 5000),
        (23493, 6000),
    ];
    REWARDS.get(usize::from(selector.checked_sub(1)?)).copied()
}
pub fn shop_key(command: CommandRef) -> String {
    let identity = super::arena_match::identity(command, b"drome-shop");
    format!(
        "oteryn:reward.tibiadrome.shop.{}",
        identity
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>()
    )
}

async fn grant_cosmetic(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    state: &mut Record,
    selector: u8,
    cosmetics: Option<&crate::content::native_gameplay::NativeCosmeticsView>,
) -> Result<u32, DurabilityError> {
    let Some(key) = cosmetics.and_then(|c| c.drome_grant(selector)) else {
        return Ok(6);
    };
    let price = if selector == 17 { 8000 } else { 12500 }; // captured reward evidence:787-852.
    if state.progress.points < price {
        return Ok(7);
    }
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,973))")
        .bind(format!("drome:{account}:{key}"))
        .execute(&mut **tx)
        .await?;
    let owned:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_appearance_unlocks WHERE account_id=$1 AND unlock_key=$2)").bind(account).bind(key).fetch_one(&mut **tx).await?;
    if owned {
        return Ok(7);
    }
    let id = super::arena_match::identity(command, b"drome-cosmetic");
    // oteryn-choice: use the existing account-owned NPC appearance grant receipts,
    // matching Oteryn's permanent cosmetic owner rather than a second unlock store.
    sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES(encode($1,'hex')::uuid,$2,$3,'crystal-201189f-tibiadrome-v1',encode($4,'hex')::uuid,'npc',$1)")
        .bind(id.as_slice()).bind(account).bind(key).bind(fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) VALUES($1,$2,encode($3,'hex')::uuid)").bind(account).bind(key).bind(id.as_slice()).execute(&mut **tx).await?;
    state.progress.points -= price;
    Ok(1)
}

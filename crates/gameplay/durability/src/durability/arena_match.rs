//! SV15: persistent Svargrond progress and exact command receipts. Item value remains
//! in the existing fee BURN and RewardClaim MINT owners, never an arena wallet.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::db::{
    begin_semantic_transaction, begin_type2_transaction, commit_semantic_transaction,
    lock_admission_relations,
};
use super::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnRequest, FeeChangeFacts, burn_fee_in_transaction,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_protocol_oteryn::arena::{ArenaDisposition, ArenaResult, ArenaState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
pub mod donor;
mod pit_lease;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchState {
    pub completed: u8,
    pub entry_revision: u64,
    pub rank: u8,
    pub pit: u8,
    pub phase: u8,
    pub deadline_ms: i64,
    pub session: [u8; 16],
    pub scope_generation: u64,
    pub run: [u8; 16],
}
impl MatchState {
    /// M51: project the state committed by pit custody/fee admission, never a
    /// refused next-pit proposal. Source arena_kill.lua:37 preserves a credited
    /// victory even when movements_arena_pit.lua:52-56 refuses the next room.
    pub fn reconcile_arena_refusal(
        &mut self,
        before: &Self,
        disposition: ArenaDisposition,
        committed: &ArenaState,
    ) {
        match disposition {
            ArenaDisposition::Busy => {
                *self = before.clone();
                self.phase = committed.phase;
            }
            ArenaDisposition::InsufficientGold | ArenaDisposition::InventoryFull => {
                *self = before.clone()
            }
            _ => {}
        }
    }

    pub fn wire(&self, now_ms: i64, masks: [u8; 3]) -> ArenaState {
        ArenaState {
            completed_rank: self.completed,
            active_rank: self.rank,
            pit: self.pit,
            phase: self.phase,
            remaining_seconds: if self.phase == 1 || self.phase == 2 {
                ((self.deadline_ms.saturating_sub(now_ms).max(0) + 999) / 1000).min(600) as u32
            } else {
                0
            },
            claimed_masks: masks,
        }
    }
    pub fn idle(&self) -> Self {
        Self {
            completed: self.completed,
            ..Self::default()
        }
    }
}

pub fn identity(command: CommandRef, purpose: &[u8]) -> [u8; 16] {
    let hash = Sha256::new()
        .chain_update(b"oteryn:svargrond-match:v1")
        .chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_be_bytes())
        .chain_update(purpose)
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}

pub struct MatchRead {
    pub pit_owned: bool,
    pub state: MatchState,
    pub masks: [u8; 3],
    pub now_ms: i64,
    pub replay: Option<Vec<u8>>,
    pub dead: bool,
}

pub struct MatchTransition {
    pub command: CommandRef,
    pub payload: Vec<u8>,
    pub before: MatchState,
    pub after: MatchState,
    pub disposition: ArenaDisposition,
    pub victory: Option<crate::foundation::CreatureDeathOccurrenceKey>,
    pub change: Option<FeeChangeFacts>,
    pub achievement: Option<super::account_achievement::AchievementGrantRequest>,
}

impl DurabilityRoot {
    pub async fn read_arena_match(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        payload: &[u8],
    ) -> Result<MatchRead, CharacterProgressionError> {
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
                    let state = load_state(&mut tx, fence).await?;
                    let masks = claimed_masks(&mut tx, fence).await?;
                    let now_ms = now(&mut tx).await?;
                    let replay = replay(&mut tx, fence, command, &payload).await?;
                    let dead: bool = matches!(state.phase,1|2) && sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid AND committed_character_revision>$2::text::numeric)")
                        .bind(fence.character_id.as_bytes().as_slice()).bind(state.entry_revision.to_string()).fetch_one(&mut *tx).await?;
                    let pit_owned = !matches!(state.phase,1|2) || (!dead &&
                        (pit_lease::owned(&mut tx,fence,&state,now_ms).await?
                         || pit_lease::claim(&mut tx,fence,&state,now_ms).await?));
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(MatchRead {
                        pit_owned,
                        state,
                        masks,
                        now_ms,
                        replay,
                        dead,
                    }))
                })
            })
            .await?
    }

    pub async fn commit_arena_match(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: MatchTransition,
    ) -> Result<Vec<u8>, CharacterProgressionError> {
        if request.command.game_session_id() != fence.game_session_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let root=match assert_gameplay_fence(&mut tx,&fence,&node).await? {Ok(root)=>root,Err(error)=>return Ok(Err(error))};
            if let Some(bytes)=replay(&mut tx,fence,request.command,&request.payload).await? {
                tx.commit(deadline).await?;return Ok(Ok(bytes));
            }
            let prior=load_state(&mut tx,fence).await?;
            if prior!=request.before {return Ok(Err(CharacterProgressionError::ConflictingOccurrence));}
            let now_ms=now(&mut tx).await?;
            let mut ready=prior.clone();
            if let Some(death)=request.victory {
                if prior.phase!=1 || death.runtime_scope()!=fence.runtime_scope || death.scope_ownership_generation()!=fence.scope_ownership_generation { return Err(DurabilityError::InvalidStoredState); }
                let death_key=super::party_kill_xp::encode_death_key(death);
                sqlx::query("INSERT INTO game_arena_victories(character_id,run_id,rank,pit,death_key) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5)")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(prior.run.as_slice()).bind(i16::from(prior.rank)).bind(i16::from(prior.pit)).bind(death_key).execute(&mut **tx).await?;
                ready.phase=2;
            }
            if !valid_transition(&prior,&ready,&request.after,request.disposition,&request.payload,fence,request.command,now_ms) {return Err(DurabilityError::InvalidStoredState);}
            let mut after=request.after;
            let mut disposition=request.disposition;
            // Claim shared custody before charging an entry fee or moving to the
            // next pit. Runtime census alone cannot close concurrent admissions.
            if disposition==ArenaDisposition::Applied && matches!(after.phase,1|2)
                && !pit_lease::claim(&mut tx,fence,&after,now_ms).await? {
                after=ready.clone();
                disposition=ArenaDisposition::Busy;
            }
            let entering=disposition==ArenaDisposition::Applied && after.phase==1 && prior.rank==0;
            let mut fee_occurrence=None;
            if entering {
                if after.rank!=prior.completed+1 || after.pit!=1 || after.rank>3 {return Err(DurabilityError::InvalidStoredState);}
                let Some(change)=request.change else {return Err(DurabilityError::InvalidStoredState);};
                let occurrence=identity(request.command,b"fee");
                let fee=FeeBurnRequest{cause:FeeBurnCause::ArenaEntry{occurrence},
                    fee_gold_units:donor::FEES[usize::from(after.rank-1)],
                    transaction_id:identity(request.command,b"fee-tx"),event_id:identity(request.command,b"fee-event"),
                    occurred_at_unix_ms:now_ms,server_build_id:super::character_authority::SERVER_BUILD_ID.into(),change};
                match burn_fee_in_transaction(&mut tx,&fence,&fee).await {
                    Ok(_)=>{fee_occurrence=Some(occurrence);}
                    Err(FeeBurnError::InsufficientFunds)=>{after=prior.clone();disposition=ArenaDisposition::InsufficientGold;}
                    Err(FeeBurnError::ChangeDoesNotFit|FeeBurnError::CapacityExceeded)=>{after=prior.clone();disposition=ArenaDisposition::InventoryFull;}
                    Err(FeeBurnError::Unavailable(error))=>return Err(error),
                    Err(_)=>return Err(DurabilityError::InvalidStoredState),
                }
            }
            if after.completed>prior.completed {
                if prior.pit!=10 || after.completed!=prior.completed+1 || after.completed!=prior.rank || after.phase!=3 {
                    return Err(DurabilityError::InvalidStoredState);
                }
                let grant=request.achievement.ok_or(DurabilityError::InvalidStoredState)?;
                if grant.achievement_key!=donor::ACHIEVEMENTS[usize::from(after.completed-1)]
                    || grant.source.event_id.as_slice()!=prior.run || grant.source.kind!="oteryn:svargrond-arena" {
                    return Err(DurabilityError::InvalidStoredState);
                }
                let granter=super::account_achievement::FencedGrantingCharacter::in_transaction(&mut tx,fence.character_id).await?;
                if super::account_achievement::record_achievement_grant(&mut tx,granter,&grant).await?.is_err() {
                    return Err(DurabilityError::InvalidStoredState);
                }
            }
            let masks=claimed_masks(&mut tx,fence).await?;
            let bytes=oteryn_protocol_oteryn::arena::encode_match_result(&ArenaResult{disposition,state:after.wire(now_ms,masks)})
                .map_err(|_|DurabilityError::InvalidStoredState)?;
            if let Some(occurrence)=fee_occurrence {
                let next=root.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
                sqlx::query("INSERT INTO game_arena_entry_receipts(character_id,game_session_id,command_id,fee_occurrence,rank,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) SELECT character_id,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,$5,character_revision,$6::text::numeric,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(occurrence.as_slice()).bind(i16::from(after.rank)).bind(next.to_string()).execute(&mut **tx).await?;
                for query in ["UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric", "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric"] {
                    if sqlx::query(query).bind(fence.character_id.as_bytes().as_slice()).bind(next.to_string()).bind(root.revision.to_string()).execute(&mut **tx).await?.rows_affected()!=1 {return Err(DurabilityError::InvalidStoredState);}
                }
            }
            if !matches!(after.phase,1|2) { pit_lease::release(&mut tx,fence).await?; }
            save(&mut tx,fence,request.command,&request.payload,&prior,&after,&bytes).await?;
            tx.commit(deadline).await?;Ok(Ok(bytes))
        })).await?
    }
}

pub(super) async fn load_state(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
) -> Result<MatchState, DurabilityError> {
    let body:Option<serde_json::Value>=sqlx::query_scalar("SELECT body FROM game_character_arena_matches WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    body.map(|body| serde_json::from_value(body).map_err(|_| DurabilityError::InvalidStoredState))
        .transpose()
        .map(|body| body.unwrap_or_default())
}
pub(super) async fn now(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<i64, DurabilityError> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?,
    )
}
pub(super) async fn claimed_masks(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
) -> Result<[u8; 3], DurabilityError> {
    let keys:Vec<String>=sqlx::query_scalar("SELECT claim_production_key FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid AND claim_family='RewardClaim' AND claim_production_key LIKE 'oteryn:reward.svargrond-arena.%' LIMIT 13")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    if keys.len() > 12 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut masks = [0; 3];
    for rank in 1..=3 {
        for category in 1..=4 {
            if keys.contains(&donor::claim_key(rank, category)) {
                masks[usize::from(rank - 1)] |= 1 << (category - 1);
            }
        }
    }
    Ok(masks)
}
pub(super) async fn replay(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    payload: &[u8],
) -> Result<Option<Vec<u8>>, DurabilityError> {
    let row=sqlx::query("SELECT payload,reply FROM game_arena_match_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric AND character_id=encode($3,'hex')::uuid")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
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
    before: &MatchState,
    after: &MatchState,
    reply: &[u8],
) -> Result<(), DurabilityError> {
    let before = serde_json::to_value(before).map_err(|_| DurabilityError::InvalidStoredState)?;
    let after = serde_json::to_value(after).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_arena_match_receipts(character_id,game_session_id,command_id,payload,before_state,after_state,reply) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6,$7)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(payload).bind(before).bind(&after).bind(reply).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_character_arena_matches(character_id,body,game_session_id,command_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4::text::numeric) ON CONFLICT(character_id) DO UPDATE SET body=EXCLUDED.body,game_session_id=EXCLUDED.game_session_id,command_id=EXCLUDED.command_id")
        .bind(fence.character_id.as_bytes().as_slice()).bind(after).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).execute(&mut **tx).await?;
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "closed state transition and its current command/fence/time"
)]
fn valid_transition(
    before: &MatchState,
    ready: &MatchState,
    after: &MatchState,
    disposition: ArenaDisposition,
    payload: &[u8],
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    now_ms: i64,
) -> bool {
    use oteryn_protocol_oteryn::arena::{
        ArenaAction as A, ArenaDisposition as D, decode_match_request,
    };
    let Ok(action) = decode_match_request(payload) else {
        return false;
    };
    if after == ready {
        return true;
    }
    if after == &before.idle()
        && before.rank > 0
        && (matches!(disposition, D::TimedOut | D::WrongState) || matches!(action, A::Leave))
    {
        return true;
    }
    if disposition != D::Applied {
        return false;
    }
    let mut expected = ready.clone();
    match action {
        A::Enter if before.rank == 0 && before.completed < 3 => {
            expected = MatchState {
                completed: before.completed,
                entry_revision: fence.expected_character_revision.get().saturating_add(1),
                rank: before.completed + 1,
                pit: 1,
                phase: 1,
                deadline_ms: after.deadline_ms,
                session: *fence.game_session_id.as_bytes(),
                scope_generation: fence.scope_ownership_generation.get(),
                run: identity(command, b"run"),
            };
        }
        A::Advance if ready.phase == 2 && ready.pit == 10 => {
            expected.completed = ready.rank;
            expected.pit = 0;
            expected.phase = 3;
            expected.deadline_ms = 0;
        }
        A::Advance if ready.phase == 2 && ready.pit < 10 => {
            expected.pit += 1;
            expected.phase = 1;
            expected.deadline_ms = after.deadline_ms;
        }
        _ => return false,
    }
    after == &expected
        && (after.phase != 1
            || (now_ms < after.deadline_ms
                && after.deadline_ms <= now_ms + i64::from(donor::PIT_SECONDS) * 1000))
}

#[cfg(test)]
#[path = "arena_match/projection_tests.rs"]
mod projection_tests;

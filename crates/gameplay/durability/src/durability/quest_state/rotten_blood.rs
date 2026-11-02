//! Rotten Blood extends the Quest owner. Native kill consumers share the XP
//! transaction; NPC cleanse uses a fenced Character receipt and database time.
//! Access remains an existing Quest track, so altar items and paid access are
//! consumed by the existing item/wallet callback owners, never by this module.

use oteryn_gameplay_quests::quest::rotten_blood::{
    ACCESS_TRACK, CleanseMode, RottenBloodBoss, RottenBloodRefusal, RottenBloodState,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

use super::super::character_appearance::{QuestOutfitGrant, grant_quest_outfit};
use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, FencedCharacterRoot,
    assert_gameplay_fence, assert_item_only_fence, numeric_u64, state_matches_root,
};
use super::super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::RuntimeScopeRefV1;

type Result<T> = std::result::Result<T, CharacterProgressionError>;
type StoredResult<T> = std::result::Result<T, DurabilityError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RottenBloodCommit {
    pub character_id: CharacterId,
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: Option<CharacterRevision>,
    pub state: RottenBloodState,
    /// Once-only quest completion; appearance itself remains account-owned.
    pub grant_base_outfit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RottenBloodOutcome {
    Committed(RottenBloodCommit),
    AlreadyCommitted(RottenBloodCommit),
    Refused(RottenBloodRefusal),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptResult {
    state: RottenBloodState,
    refusal: Option<RottenBloodRefusal>,
    grant_base_outfit: bool,
}

struct Mutation<'a> {
    occurrence: &'a [u8],
    binding: [u8; 32],
    kind: i16,
    source_death: Option<&'a [u8; 52]>,
    source_terminal: Option<&'a [u8; 32]>,
    party_minimum: Option<u8>,
    dealt_damage: Option<bool>,
    before: RottenBloodState,
    result: ReceiptResult,
}

impl DurabilityRoot {
    pub async fn read_rotten_blood_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<RottenBloodState> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                    Ok(_) => {}
                    Err(error) => return Ok(Err(error)),
                }
                let state = read_state(&mut tx, fence.character_id).await?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(state))
            }))
            .await?
    }

    /// Called only after the NPC owner verifies the current Bloodshade dialogue,
    /// position, selected reply and retained command. No client death/state input.
    pub async fn commit_rotten_blood_cleanse(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command_id: u64,
        mode: CleanseMode,
    ) -> Result<RottenBloodOutcome> {
        let occurrence = cleanse_occurrence(fence, command_id)?;
        let binding = cleanse_binding(fence.character_id, mode);
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                // Replays retain their historical revision but cannot bypass a
                // changed connection, lease, runtime scope or node incarnation.
                let root = match assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await? {
                    Ok(root) => root,
                    Err(error) => return Ok(Err(error)),
                };
                if let Some(stored) = replay(&mut tx, fence.character_id, &occurrence, &binding).await? {
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(stored);
                }
                if root.revision != fence.expected_character_revision.get() {
                    return Ok(Err(CharacterProgressionError::CharacterRevisionMismatch));
                }
                verify_progression(&mut tx, fence.character_id, &root).await?;
                let before = read_state(&mut tx, fence.character_id).await?;
                let now = database_now(&mut tx).await?;
                let (state, refusal) = match before.cleanse(mode, now) {
                    Ok(state) => (state, None),
                    Err(refusal) => (before, Some(refusal)),
                };
                let result = write_transition(&mut tx, fence.character_id, &root, Mutation {
                    occurrence: &occurrence,
                    binding,
                    kind: match mode { CleanseMode::Regular => 3, CleanseMode::All => 4 },
                    source_death: None,
                    source_terminal: None,
                    party_minimum: None,
                    dealt_damage: None,
                    before,
                    result: ReceiptResult { state, refusal, grant_base_outfit: false },
                }).await?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(result))
            }))
            .await?
    }

    /// Read retained terminal data only. An NPC resume must never turn missing
    /// data into a new write after the NPC moved, disappeared or lost its focus.
    pub async fn reconcile_rotten_blood_cleanse(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterGameplayFence,
        command_id: u64,
        mode: CleanseMode,
    ) -> Result<Option<RottenBloodOutcome>> {
        let occurrence = cleanse_occurrence(fence, command_id)?;
        let binding = cleanse_binding(fence.character_id, mode);
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                // The writer locks this root before it can publish a receipt.
                // Synchronize with an unknown in-flight commit without claiming
                // that its original session/lease is still current authority.
                sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                let stored = replay(&mut tx, fence.character_id, &occurrence, &binding).await?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(stored.transpose())
            }))
            .await?
    }
}

/// Production native-death consumer. The party XP owner calls this after all
/// member receipts exist and before committing its transaction, with all native
/// Character sequencer slots held. The bool is derived from the native damage
/// contributors, never XP shares or a client assertion. Non-damaging party
/// participants remain in the snapshot used for the party's minimum taint.
pub async fn apply_rotten_blood_kill_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    death_key: &[u8; 52],
    creature: &str,
    participants: &[(CurrentCharacterGameplayFence, bool)],
) -> Result<()> {
    let Some(boss) = RottenBloodBoss::from_creature_key(creature) else {
        return Ok(());
    };
    if participants.is_empty()
        || participants.len() > 5
        || participants.windows(2).any(|p| p[0].0.character_id >= p[1].0.character_id)
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let source: Option<(String, bool, i16)> = sqlx::query_as(
        "SELECT creature_key,created_xact_id=pg_current_xact_id(),principal_count FROM game_party_kill_xp_receipts WHERE death_key=$1",
    )
    .bind(death_key.as_slice())
    .fetch_optional(&mut **tx)
    .await
    .map_err(DurabilityError::from)?;
    let Some((source_creature, same_transaction, count)) = source else {
        return Err(CharacterProgressionError::AuthorityRejected);
    };
    if source_creature != creature {
        return Err(CharacterProgressionError::ConflictingOccurrence);
    }
    if usize::try_from(count).ok() != Some(participants.len()) {
        return Err(CharacterProgressionError::AuthorityRejected);
    }
    let binding = kill_binding(death_key, boss, participants);
    let occurrence = kill_occurrence(death_key);
    let memberships=sqlx::query("SELECT character_id::text,party_id::text FROM game_party_kill_xp_members WHERE death_key=$1 ORDER BY character_id")
        .bind(death_key.as_slice()).fetch_all(&mut **tx).await.map_err(DurabilityError::from)?;
    if memberships.len()!=participants.len() {
        return Err(CharacterProgressionError::AuthorityRejected);
    }
    let mut groups=std::collections::BTreeMap::new();
    for row in memberships {
        let character=super::super::character_progression::uuid_text(row.try_get("character_id").map_err(DurabilityError::from)?)?;
        let party=row.try_get::<Option<String>,_>("party_id").map_err(DurabilityError::from)?;
        let group=match party {
            Some(party)=>(true,super::super::character_progression::uuid_text(&party)?),
            None=>(false,character),
        };
        groups.insert(character,group);
    }
    let mut locked = Vec::with_capacity(participants.len());
    for &(fence, damage) in participants {
        let RuntimeScopeRefV1::Channel { world_id, channel_id } = fence.runtime_scope else {
            return Err(CharacterProgressionError::AuthorityRejected);
        };
        if death_key[..16] != *world_id.as_bytes()
            || death_key[16..32] != *channel_id.as_bytes()
            || death_key[32..40] != fence.scope_ownership_generation.get().to_be_bytes()
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let root = assert_item_only_fence(tx, &fence.item_fence(), node).await??;
        let stored = replay(tx, fence.character_id, &occurrence, &binding).await?;
        if let Some(outcome) = stored {
            outcome?;
            locked.push((fence.character_id, root, read_state(tx, fence.character_id).await?, damage, true));
            continue;
        }
        // Historical XP data may reconcile an existing Quest receipt but can
        // never acquire new progression authority in a later transaction.
        if !same_transaction {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let member = sqlx::query(
            "SELECT m.reward_occurrence_id IS NOT NULL AS has_xp, \
             x.original_character_revision::text AS original_revision, \
             x.committed_character_revision::text AS committed_revision \
             FROM game_party_kill_xp_members m LEFT JOIN game_character_xp_receipts x \
             ON x.reward_occurrence_id=m.reward_occurrence_id AND x.character_id=m.character_id \
             WHERE m.death_key=$1 AND m.character_id=encode($2,'hex')::uuid",
        )
        .bind(death_key.as_slice())
        .bind(fence.character_id.as_bytes().as_slice())
        .fetch_optional(&mut **tx)
        .await
        .map_err(DurabilityError::from)?
        .ok_or(CharacterProgressionError::AuthorityRejected)?;
        let has_xp = member.try_get::<bool, _>("has_xp").map_err(DurabilityError::from)?;
        let source_revision = if has_xp {
            if numeric_u64(&member, "original_revision")? < fence.expected_character_revision.get() {
                return Err(CharacterProgressionError::CharacterRevisionMismatch);
            }
            numeric_u64(&member, "committed_revision")?
        } else {
            fence.expected_character_revision.get()
        };
        if root.revision < source_revision {
            return Err(CharacterProgressionError::CharacterRevisionMismatch);
        }
        // The existing party owner verified the original fence before creating
        // this same-transaction death. Forge Dust may precede XP, and any
        // source-owned descendants may follow it. They can only advance this
        // locked root. Rebind only that revision and check every
        // live authority fact again; historical XP cannot enter this branch.
        let mut current=fence;
        current.expected_character_revision=CharacterRevision::new(root.revision)
            .map_err(|_|DurabilityError::InvalidStoredState)?;
        let root=assert_gameplay_fence(tx,&current,node).await??;
        verify_progression(tx, fence.character_id, &root).await?;
        locked.push((fence.character_id, root, read_state(tx, fence.character_id).await?, damage, false));
    }
    // One snapshot before any write: earlier recipients cannot raise the
    // minimum used for later recipients of the same death occurrence.
    // A native kill can include several parties. An unrelated party or solo
    // attacker cannot lower another party's minimum; use the immutable native
    // party membership captured by the existing XP owner for this death.
    let mut minima=std::collections::BTreeMap::new();
    for (character,_,state,_,_) in &locked {
        let group=groups.get(character.as_bytes()).ok_or(CharacterProgressionError::AuthorityRejected)?;
        let minimum=minima.entry(*group).or_insert(state.regular_taints());
        *minimum=(*minimum).min(state.regular_taints());
    }
    for (character, root, before, dealt_damage, already) in locked {
        if already {
            continue;
        }
        let group=groups.get(character.as_bytes()).ok_or(CharacterProgressionError::AuthorityRejected)?;
        let minimum=*minima.get(group).ok_or(DurabilityError::InvalidStoredState)?;
        let result = if boss == RottenBloodBoss::Bakragore {
            before.defeat_final(dealt_damage).map(|victory| ReceiptResult {
                state: victory.state, refusal: None, grant_base_outfit: victory.grant_base_outfit,
            })
        } else {
            before.defeat_child(boss, minimum, dealt_damage).map(|state| ReceiptResult {
                state, refusal: None, grant_base_outfit: false,
            })
        }.unwrap_or_else(|refusal| ReceiptResult {
            state: before, refusal: Some(refusal), grant_base_outfit: false,
        });
        write_transition(tx, character, &root, Mutation {
            occurrence: &occurrence,
            binding,
            kind: if boss == RottenBloodBoss::Bakragore { 2 } else { 1 },
            source_death: Some(death_key),
            source_terminal: None,
            party_minimum: Some(minimum),
            dealt_damage: Some(dealt_damage),
            before,
            result,
        }).await?;
    }
    Ok(())
}

/// Called only by the shared terminal-outcome consumer. A native player death
/// or disconnect is not terminal authority. The terminal decision and every
/// Character change commit together; a historical decision only replays.
pub(crate) async fn apply_rotten_blood_failed_final_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    attempt: &[u8; 32],
    removed_flags: u8,
    participants: &[CurrentCharacterGameplayFence],
) -> Result<()> {
    if participants.is_empty() || participants.len()>5
        || participants.windows(2).any(|p| p[0].character_id>=p[1].character_id)
        || removed_flags>15 || removed_flags.count_ones()!=2 {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let mut roster=Vec::with_capacity(16*participants.len());
    for fence in participants { roster.extend_from_slice(fence.character_id.as_bytes()); }
    let mut occurrence=b"rotten-blood/failed-final/v1".to_vec();
    occurrence.extend_from_slice(attempt);
    let binding: [u8;32]=Sha256::new().chain_update(&occurrence).chain_update([removed_flags]).chain_update(&roster).finalize().into();
    let mut pending=Vec::new();
    for &fence in participants {
        if let Some(stored)=replay(tx,fence.character_id,&occurrence,&binding).await? { stored?; }
        else { pending.push(fence); }
    }
    if pending.is_empty() { return Ok(()); }
    // A partially recorded roster is impossible under the shared atomic owner.
    if pending.len()!=participants.len() { return Err(DurabilityError::InvalidStoredState.into()); }
    let decision=sqlx::query("SELECT encounter_kind,removed_flags,participants, \
        world_id::text,scope_kind,scope_id::text,scope_ownership_generation::text, \
        created_xact_id=pg_current_xact_id() AS current_transaction \
        FROM game_endgame_terminal_outcomes WHERE attempt=$1")
        .bind(attempt.as_slice()).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?
        .ok_or(CharacterProgressionError::AuthorityRejected)?;
    if decision.try_get::<i16,_>("encounter_kind").map_err(DurabilityError::from)?!=2
        || decision.try_get::<i16,_>("removed_flags").map_err(DurabilityError::from)?!=i16::from(removed_flags)
        || decision.try_get::<Vec<u8>,_>("participants").map_err(DurabilityError::from)?!=roster
        || !decision.try_get::<bool,_>("current_transaction").map_err(DurabilityError::from)? {
        return Err(CharacterProgressionError::AuthorityRejected);
    }
    let world=super::super::character_progression::uuid_text(decision.try_get("world_id").map_err(DurabilityError::from)?)?;
    let scope=super::super::character_progression::uuid_text(decision.try_get("scope_id").map_err(DurabilityError::from)?)?;
    let scope_kind=decision.try_get::<i16,_>("scope_kind").map_err(DurabilityError::from)?;
    let generation=numeric_u64(&decision,"scope_ownership_generation")?;
    for fence in pending {
        let (kind,id)=match fence.runtime_scope {
            RuntimeScopeRefV1::Channel { channel_id,.. } => (1,*channel_id.as_bytes()),
            RuntimeScopeRefV1::Instance { instance_id,.. } => (2,instance_id),
        };
        if *fence.runtime_scope.world_id().as_bytes()!=world || kind!=scope_kind
            || id!=scope || fence.scope_ownership_generation.get()!=generation {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let root=assert_gameplay_fence(tx,&fence,node).await??;
        verify_progression(tx,fence.character_id,&root).await?;
        let before=read_state(tx,fence.character_id).await?;
        let state=before.fail_final(removed_flags).map_err(|_|CharacterProgressionError::InvalidInput)?;
        write_transition(tx,fence.character_id,&root,Mutation {
            occurrence:&occurrence,binding,kind:5,source_death:None,source_terminal:Some(attempt),
            party_minimum:None,dealt_damage:None,before,
            result:ReceiptResult { state,refusal:None,grant_base_outfit:false },
        }).await?;
    }
    Ok(())
}

async fn read_state(tx: &mut Transaction<'_, Postgres>, character: CharacterId) -> StoredResult<RottenBloodState> {
    let access: i64 = sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks \
         WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)",
    ).bind(character.as_bytes().as_slice()).bind(ACCESS_TRACK).fetch_one(&mut **tx).await?;
    let row = sqlx::query(
        "SELECT regular_taints,child_flags,final_taint,completed,last_cleanse_at \
         FROM game_character_rotten_blood WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    ).bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let access = u8::try_from(access).map_err(|_| DurabilityError::InvalidStoredState)?;
    let Some(row) = row else {
        return RottenBloodState::from_stored(access, 0, 0, false, false, None)
            .map_err(|_| DurabilityError::InvalidStoredState);
    };
    RottenBloodState::from_stored(
        access,
        u8::try_from(row.try_get::<i16, _>("regular_taints")?).map_err(|_| DurabilityError::InvalidStoredState)?,
        u8::try_from(row.try_get::<i16, _>("child_flags")?).map_err(|_| DurabilityError::InvalidStoredState)?,
        row.try_get("final_taint")?,
        row.try_get("completed")?,
        row.try_get::<Option<i64>, _>("last_cleanse_at")?.map(u64::try_from).transpose().map_err(|_| DurabilityError::InvalidStoredState)?,
    ).map_err(|_| DurabilityError::InvalidStoredState)
}

async fn verify_progression(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    root: &FencedCharacterRoot,
) -> StoredResult<()> {
    let state = sqlx::query(
        "SELECT character_revision::text,profile_revision,ruleset_revision,content_revision \
         FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    ).bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
        .ok_or(DurabilityError::InvalidStoredState)?;
    if numeric_u64(&state, "character_revision")? != root.revision || !state_matches_root(&state, root) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns \
         WHERE character_id=encode($1,'hex')::uuid)",
    ).bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if pending {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

async fn database_now(tx: &mut Transaction<'_, Postgres>) -> StoredResult<u64> {
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&mut **tx).await?;
    u64::try_from(now).map_err(|_| DurabilityError::InvalidStoredState)
}

fn cleanse_occurrence(fence: CurrentCharacterGameplayFence, command_id: u64) -> Result<Vec<u8>> {
    if command_id == 0 {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let mut out = b"rotten-blood/cleanse/v1".to_vec();
    out.extend_from_slice(fence.game_session_id.as_bytes());
    out.extend_from_slice(&fence.connection_generation.get().to_be_bytes());
    out.extend_from_slice(&command_id.to_be_bytes());
    Ok(out)
}

fn cleanse_binding(character: CharacterId, mode: CleanseMode) -> [u8; 32] {
    Sha256::new().chain_update(b"rotten-blood/cleanse/v1")
        .chain_update(character.as_bytes())
        .chain_update([match mode { CleanseMode::Regular => 0, CleanseMode::All => 1 }])
        .finalize().into()
}

fn kill_occurrence(death: &[u8; 52]) -> Vec<u8> {
    let mut out = b"rotten-blood/kill/v1".to_vec();
    out.extend_from_slice(death);
    out
}

fn kill_binding(death: &[u8; 52], boss: RottenBloodBoss, participants: &[(CurrentCharacterGameplayFence, bool)]) -> [u8; 32] {
    let mut hash = Sha256::new().chain_update(b"rotten-blood/kill/v1").chain_update(death).chain_update([boss as u8]);
    for (fence, damage) in participants {
        hash.update(fence.character_id.as_bytes());
        hash.update([u8::from(*damage)]);
    }
    hash.finalize().into()
}

async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    occurrence: &[u8],
    binding: &[u8; 32],
) -> StoredResult<Option<Result<RottenBloodOutcome>>> {
    let Some(row) = sqlx::query(
        "SELECT request_binding,result,original_character_revision::text, \
         committed_character_revision::text FROM game_character_rotten_blood_receipts \
         WHERE character_id=encode($1,'hex')::uuid AND occurrence=$2",
    ).bind(character.as_bytes().as_slice()).bind(occurrence).fetch_optional(&mut **tx).await? else {
        return Ok(None);
    };
    if row.try_get::<Vec<u8>, _>("request_binding")?.as_slice() != binding {
        return Ok(Some(Err(CharacterProgressionError::ConflictingOccurrence)));
    }
    let result: ReceiptResult = serde_json::from_value(row.try_get("result")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let original = CharacterRevision::new(numeric_u64(&row, "original_character_revision")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let committed = row.try_get::<Option<String>, _>("committed_character_revision")?
        .map(|value| value.parse::<u64>().ok().and_then(|value| CharacterRevision::new(value).ok())
            .ok_or(DurabilityError::InvalidStoredState)).transpose()?;
    Ok(Some(Ok(if let Some(refusal) = result.refusal {
        RottenBloodOutcome::Refused(refusal)
    } else {
        RottenBloodOutcome::AlreadyCommitted(RottenBloodCommit {
            character_id: character,
            original_character_revision: original,
            committed_character_revision: committed,
            state: result.state,
            grant_base_outfit: result.grant_base_outfit,
        })
    })))
}

async fn write_transition(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    root: &FencedCharacterRoot,
    mutation: Mutation<'_>,
) -> StoredResult<RottenBloodOutcome> {
    let Mutation { occurrence, binding, kind, source_death, source_terminal, party_minimum, dealt_damage, before, result } = mutation;
    let changed = before != result.state;
    if result.refusal.is_some() && changed {
        return Err(DurabilityError::InvalidStoredState);
    }
    let next = root.revision.checked_add(u64::from(changed)).ok_or(DurabilityError::InvalidStoredState)?;
    let before_json = serde_json::to_value(before).map_err(|_| DurabilityError::InvalidStoredState)?;
    let result_json = serde_json::to_value(&result).map_err(|_| DurabilityError::InvalidStoredState)?;
    let inserted = sqlx::query(
        "INSERT INTO game_character_rotten_blood_receipts(character_id,occurrence,request_binding, \
         kind,source_death_key,before_state,result,original_character_revision,committed_character_revision, \
         level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision, \
         content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision, \
         source_party_minimum,source_dealt_damage,source_terminal_attempt) \
         SELECT s.character_id,$2,$3,$4,$5,$6,$7,s.character_revision,$8::text::numeric, \
         s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision, \
         s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,$10,$11,$12 \
         FROM game_character_progression_state s WHERE s.character_id=encode($1,'hex')::uuid \
         AND s.character_revision=$9::text::numeric",
    ).bind(character.as_bytes().as_slice()).bind(occurrence).bind(binding.as_slice()).bind(kind)
        .bind(source_death.map(|key| key.as_slice())).bind(before_json).bind(result_json)
        .bind(changed.then(|| next.to_string())).bind(root.revision.to_string())
        .bind(party_minimum.map(i16::from)).bind(dealt_damage).bind(source_terminal.map(|value|value.as_slice())).execute(&mut **tx).await?;
    if inserted.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if changed {
        for query in [
            "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
            "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
        ] {
            if sqlx::query(query).bind(character.as_bytes().as_slice()).bind(next.to_string())
                .bind(root.revision.to_string()).execute(&mut **tx).await?.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
        let state = result.state;
        sqlx::query(
            "INSERT INTO game_character_rotten_blood(character_id,regular_taints,child_flags,final_taint,completed,last_cleanse_at) \
             VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6) ON CONFLICT(character_id) DO UPDATE SET \
             regular_taints=EXCLUDED.regular_taints,child_flags=EXCLUDED.child_flags,final_taint=EXCLUDED.final_taint, \
             completed=EXCLUDED.completed,last_cleanse_at=EXCLUDED.last_cleanse_at",
        ).bind(character.as_bytes().as_slice()).bind(i16::from(state.regular_taints())).bind(i16::from(state.child_flags()))
            .bind(state.final_taint()).bind(state.completed())
            .bind(state.last_cleanse_at().map(i64::try_from).transpose().map_err(|_| DurabilityError::InvalidStoredState)?)
            .execute(&mut **tx).await?;
    }
    if result.grant_base_outfit {
        grant_quest_outfit(tx, character, occurrence, QuestOutfitGrant::DecayingDefender).await?;
    }
    Ok(if let Some(refusal) = result.refusal {
        RottenBloodOutcome::Refused(refusal)
    } else {
        RottenBloodOutcome::Committed(RottenBloodCommit {
            character_id: character,
            original_character_revision: CharacterRevision::new(root.revision).map_err(|_| DurabilityError::InvalidStoredState)?,
            committed_character_revision: changed.then(|| CharacterRevision::new(next)).transpose().map_err(|_| DurabilityError::InvalidStoredState)?,
            state: result.state,
            grant_base_outfit: result.grant_base_outfit,
        })
    })
}

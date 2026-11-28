//! M51 admission for physical pit reset. Crystal201189f
//! lib/quests/svargrond_arena.lua:193-246; movements_arena_enter.lua:22-30.
//! A successful read is a snapshot, never a retained write grant. Retirement
//! must call the same validation again in its deciding transaction.
use super::*;
use crate::durability::db::commit_semantic_read;

impl DurabilityRoot {
    pub async fn read_arena_pit_reset(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        expected: &MatchState,
    ) -> Result<bool, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let expected = expected.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let accepted = lock_reset_custody(&mut tx, fence, &expected).await?;
                    commit_semantic_read(tx, deadline).await?;
                    Ok(Ok(accepted))
                })
            })
            .await?
    }
}

/// Caller first holds the current Character/node/scope fence and root lock.
/// Keep the exact arena and pit rows locked through physical retirement; this
/// helper never claims, refreshes or releases custody. A detached read is not
/// authority for a later transaction.
pub(in crate::durability) async fn lock_reset_custody(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    expected: &MatchState,
) -> Result<bool, DurabilityError> {
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    let state = load_state(tx, fence).await?;
    if &state != expected
        || state.phase != 1
        || !(1..=3).contains(&state.rank)
        || !(1..=10).contains(&state.pit)
        || state.entry_revision == 0
        || state.session != *fence.game_session_id.as_bytes()
        || state.scope_generation != fence.scope_ownership_generation.get()
    {
        return Ok(false);
    }
    let held: Option<i32> = sqlx::query_scalar("SELECT 1 FROM game_svargrond_pit_leases WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND pit=$3 AND character_id=encode($4,'hex')::uuid AND game_session_id=encode($5,'hex')::uuid AND run_id=encode($6,'hex')::uuid AND scope_generation=$7::text::numeric AND expires_at_ms=$8 FOR UPDATE")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(i16::from(state.pit)).bind(fence.character_id.as_bytes().as_slice())
        .bind(state.session.as_slice()).bind(state.run.as_slice())
        .bind(state.scope_generation.to_string()).bind(state.deadline_ms)
        .fetch_optional(&mut **tx).await?;
    if held.is_none() {
        return Ok(false);
    }
    let dead: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid AND committed_character_revision>$2::text::numeric)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(state.entry_revision.to_string())
        .fetch_one(&mut **tx).await?;
    // Read the deciding clock after acquiring both rows, never before a lock wait.
    Ok(!dead && state.deadline_ms > now(tx).await?)
}

#[cfg(test)]
#[path = "pit_reset_tests.rs"]
mod tests;

use crate::durability::character_progression::assert_item_only_fence;
use crate::durability::spell_items_abi::decode_ground_cell;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::RuntimeScopeRefV1;
use oteryn_gameplay_content::content::{ReferenceItemField, native_gameplay::NativeGameplayState};
use oteryn_gameplay_quests::quest::barbarian_arena::{Pit, ResetTopThing};

/// A movable Ground candidate from the pinned native stack. The item owner
/// still has to find this exact item and definition at the durable top.
pub struct ArenaPitResetItem {
    pub(in crate::durability) identity: [u8; 16],
    pub(in crate::durability) position: [u8; 10],
    pub(in crate::durability) ordinal: i64,
    pub(in crate::durability) key: String,
    pub(in crate::durability) revision: String,
}

impl ArenaPitResetItem {
    pub fn observe(
        native: &NativeGameplayState,
        pit: Pit,
        identity: [u8; 16],
        position: &[u8],
        ordinal: u64,
        definition: std::num::NonZeroU32,
    ) -> Option<Self> {
        let (key, revision) = native.item_index()?.definition_at(definition)?;
        let policy = native.item_policy(key, revision)?;
        let record = policy.record();
        let appearance = record.production_binding.external_id.parse().ok()?;
        let movable = matches!(&record.semantics.physical, ReferenceItemField::Known(physical)
            if physical.movable == ReferenceItemField::Known(true));
        let cell = decode_ground_cell(position).ok()?;
        pit.layout().item_to_reset(
            [cell.x, cell.y, -cell.z.abs()],
            ResetTopThing::Item {
                identity,
                appearance,
                movable,
            },
        )?;
        Some(Self {
            identity,
            position: position.try_into().ok()?,
            ordinal: ordinal.try_into().ok()?,
            key: key.into(),
            revision: revision.into(),
        })
    }

    pub(in crate::durability) fn other_floor(&self) -> [u8; 10] {
        let mut position = self.position;
        let floor = i16::from_be_bytes([position[8], position[9]]);
        position[8..].copy_from_slice(&(-floor).to_be_bytes());
        position
    }
}

pub(in crate::durability) struct ArenaResetVisit {
    pub(in crate::durability) fence: CurrentCharacterGameplayFence,
    pub(in crate::durability) state: MatchState,
    pub(in crate::durability) binding: [u8; 32],
}

pub enum ArenaPitResetOutcome {
    Applied,
    Refused,
    GroundBusy,
}

impl DurabilityRoot {
    pub async fn retire_arena_pit_items(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        state: &MatchState,
        items: Vec<ArenaPitResetItem>,
    ) -> Result<ArenaPitResetOutcome, CharacterProgressionError> {
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Ok(ArenaPitResetOutcome::Refused);
        };
        permit.check_scope(fence.runtime_scope)?;
        let Some(pit) = Pit::new(state.pit) else {
            return Ok(ArenaPitResetOutcome::Refused);
        };
        let mut cells = std::collections::BTreeSet::new();
        for item in &items {
            let at = decode_ground_cell(&item.position)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if !pit.layout().contains_reset([at.x, at.y, -at.z.abs()])
                || !cells.insert((at.x, at.y))
            {
                return Ok(ArenaPitResetOutcome::Refused);
            }
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let state = state.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, permit| {
            Box::pin(async move {
                let mut tx = begin_type2_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if let Err(error) = assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await? {
                    return Ok(Err(error));
                }
                if !lock_reset_custody(&mut tx, fence, &state).await? {
                    return Ok(Ok(ArenaPitResetOutcome::Refused));
                }
                let binding: Option<Vec<u8>> = sqlx::query_scalar(
                    "INSERT INTO game_arena_pit_item_resets(world_id,channel_id,run_id,pit,rank,character_id,
                         game_session_id,scope_generation,expires_at_ms)
                     VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,
                         encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8::text::numeric,$9)
                     ON CONFLICT(binding) DO NOTHING RETURNING binding",
                )
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
                .bind(state.run.as_slice()).bind(i16::from(state.pit)).bind(i16::from(state.rank))
                .bind(fence.character_id.as_bytes().as_slice()).bind(state.session.as_slice())
                .bind(state.scope_generation.to_string()).bind(state.deadline_ms)
                .fetch_optional(&mut **tx).await?;
                if let Some(binding) = binding {
                    let visit = ArenaResetVisit {
                        fence,
                        state,
                        binding: binding.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
                    };
                    if !items.is_empty() && !sqlx::query_scalar::<_, bool>(
                        "SELECT pg_try_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))",
                    )
                    .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
                    .fetch_one(&mut **tx).await? {
                        return Ok(Ok(ArenaPitResetOutcome::GroundBusy));
                    }
                    for item in items {
                        crate::durability::item_decay_retire::arena_reset::retire_top(&mut tx, permit, &node, &visit, item).await?;
                    }
                }
                tx.commit(deadline).await?;
                Ok(Ok(ArenaPitResetOutcome::Applied))
            })
        }).await?
    }
}

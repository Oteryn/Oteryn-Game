//! Personal Stone return attunement and its last position, committed together.
use super::*;
use crate::durability::character_last_position::{
    FinalLastPosition, LastPositionKey, LastPositionWrite, upsert_last_position,
};
use crate::durability::character_progression::assert_item_only_fence;
use crate::durability::db::commit_semantic_read;
use crate::durability::item_transfer::CurrentCharacterItemFence;

pub const STONE_KEY: &str = "oteryn:item.tibia.i16277";
const QUEST: &str = "oteryn:quest.adventurers_guild.stone";
// Crystal 201189f8 data-global/lib/core/storages.lua:1968-1969.
const TRACK: &str = "oteryn:quest-progress/storage/52130";
const PIN: &str = "guild-stone-v1";
const ENTER: &str = "oteryn:quest-transition/adventurers-guild-stone/enter";
const RETURN: &str = "oteryn:quest-transition/adventurers-guild-stone/return";

#[derive(Clone, Copy)]
pub enum GuildStoneTravel {
    Enter { item: [u8; 16], town: u32 },
    Return { town: u32 },
}

async fn origin(
    tx: &mut sqlx::PgConnection,
    character: CharacterId,
) -> StoredResult<Option<i64>> {
    Ok(sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks \
         WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 AND quest_key=$3",
    )
    .bind(character.as_bytes().as_slice())
    .bind(TRACK)
    .bind(QUEST)
    .fetch_optional(tx)
    .await?)
}

impl DurabilityRoot {
    /// A failed read is distinct from a player without Stone attunement.
    pub async fn read_guild_stone_origin(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> StoredResult<Option<u32>> {
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::CommitRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            assert_item_only_fence(&mut tx, &fence, &node).await?
                .map_err(|_| DurabilityError::CommitRejected)?;
            let town = match origin(&mut tx, fence.character_id).await? {
                None | Some(-1) => None,
                Some(value) => Some(u32::try_from(value).ok()
                    .filter(|town| *town > 0).ok_or(DurabilityError::InvalidStoredState)?),
            };
            commit_semantic_read(tx, deadline).await?;
            Ok(town)
        })).await
    }

    /// Re-prove the actual carried source, or consume the exact remembered town,
    /// in the same fenced transaction as the saved position.
    pub async fn commit_guild_stone_travel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        travel: GuildStoneTravel,
        placed: FinalLastPosition,
    ) -> StoredResult<()> {
        if command.game_session_id() != fence.game_session_id
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { world_id, .. } if world_id == placed.record.world_id)
        {
            return Err(DurabilityError::CommitRejected);
        }
        let (cause, key, town, item, value) = match travel {
            GuildStoneTravel::Enter { item, town } => (QuestCause::Use(command), ENTER, town, Some(item), i64::from(town)),
            GuildStoneTravel::Return { town } => (QuestCause::Step(command), RETURN, town, None, -1),
        };
        let request = QuestTransitionRequest { transition_key: key.into(), cause };
        let mut binding = request_binding(fence.character_id, &request);
        let mut digest = Sha256::new();
        digest.update(binding);
        digest.update(town.to_be_bytes());
        digest.update(item.unwrap_or_default());
        digest.update(placed.record.cell.x.to_be_bytes());
        digest.update(placed.record.cell.y.to_be_bytes());
        digest.update(placed.record.cell.floor.to_be_bytes());
        digest.update(placed.record.server_artifact_digest);
        binding[1..].copy_from_slice(&digest.finalize());

        let recovery = authority.record_for(self).map_err(|_| DurabilityError::CommitRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let root = assert_item_only_fence(&mut tx, &fence, &node).await?
                .map_err(|_| DurabilityError::CommitRejected)?;
            let character = fence.character_id;
            // The current Character root already serializes every writer of this receipt.
            if let Some(receipt) = load_receipt(&mut tx, character, &request).await? {
                if receipt.try_get::<Vec<u8>, _>("request_binding")? != binding {
                    return Err(DurabilityError::CommitRejected);
                }
                commit_semantic_read(tx, deadline).await?;
                return Ok(());
            }
            let before = origin(&mut tx, character).await?;
            if let Some(item) = item {
                let inventory = super::super::item_use::read_use_inventory(
                    &mut tx, &fence, root.content_revision, root.ruleset_revision,
                ).await.map_err(|error| match error {
                    super::super::item_use::ItemUseError::Unavailable(error) => error,
                    _ => DurabilityError::CommitRejected,
                })?;
                if !inventory.backpack.iter().flat_map(|bag| &bag.entries)
                    .chain(inventory.nested.iter().map(|(_, entry)| entry))
                    .any(|entry| entry.item.item_instance_id == item
                        && entry.item.definition.production_key == STONE_KEY)
                {
                    return Err(DurabilityError::CommitRejected);
                }
            } else if before != Some(i64::from(town)) {
                return Err(DurabilityError::CommitRejected);
            }
            let state_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_character_quest_states \
                 WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2)",
            ).bind(character.as_bytes().as_slice()).bind(QUEST).fetch_one(&mut *tx).await?;
            if (!state_exists && !capacity::new_state_fits(&mut tx, character).await?)
                || !capacity::tracks_fit(&mut tx, character, &[TRACK], usize::from(before.is_some())).await?
                || upsert_last_position(
                    &mut tx,
                    character.as_bytes(),
                    LastPositionKey {
                        character_lease_generation: fence.character_lease_generation,
                        write_sequence: placed.write_sequence,
                    },
                    &placed.record,
                ).await? != LastPositionWrite::Written
            {
                return Err(DurabilityError::CommitRejected);
            }
            let transition = oteryn_gameplay_quests::quest::QuestTransition {
                key: key.into(),
                quest: QUEST.into(),
                effects: Vec::new(),
                completes: false,
                experience: None,
                world_change: None,
            };
            // The storage ABI is stable across world generations and separate from the
            // visible Adventurers' Guild quest's existing progress definition.
            let hash = Sha256::digest(PIN.as_bytes()).into();
            let changes = [QuestTrackChange { track: TRACK.into(), before: before.unwrap_or(-1), after: value }];
            persist::persist(&mut tx, persist::QuestWrite {
                character, root_revision: root.revision, request: &request, transition: &transition,
                pin: PIN, hash: &hash, changes: &changes, binding: &binding, state_exists,
            }).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(())
        })).await
    }
}

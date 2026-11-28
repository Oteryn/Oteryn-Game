//! Read terminal command replies from the original writer's immutable receipt.
use super::db::{begin_semantic_transaction, commit_semantic_read, lock_admission_relations};
use super::fresh_admission::FreshAdmissionStore;
use super::item_transfer::CurrentCharacterItemFence;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{CommandId, GameSessionState};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReplyReceipt {
    BossDifficulty,
    ContainerSort,
    GuildBank,
    ManagedContainerMove,
    PlayerReport,
    QuickLoot,
    StashContainer,
    TeamFinderLeader,
    TeamFinderMember,
    Vip,
    WorldEvent,
}

pub enum LostCommandReply {
    Stale,
    Absent,
    Stored { rejected: bool, payload: Vec<u8> },
}

impl DurabilityRoot {
    /// The admission relation fence serializes the receipt probe with command COMMIT and
    /// control loss. In particular, an empty read under this fence proves non-commit.
    pub async fn read_lost_command_reply(
        &self,
        fence: CurrentCharacterItemFence,
        command: CommandId,
        receipt: ReplyReceipt,
    ) -> Result<LostCommandReply, DurabilityError> {
        let store = FreshAdmissionStore::from_root(self.clone());
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                lock_admission_relations(&mut tx).await?;
                let current = store.current_session_in_transaction(&mut tx, fence.game_session_id).await?;
                if current.session_state() != GameSessionState::Reconnectable
                    || current.current_connection_generation() != fence.connection_generation
                    || current.commit().character_id().as_bytes() != fence.character_id.as_bytes()
                    || current.current_runtime_scope() != fence.runtime_scope
                    || current.current_character_lease().generation() != fence.character_lease_generation
                    || current.current_scope_generation() != fence.scope_ownership_generation
                {
                    return Ok(LostCommandReply::Stale);
                }

                // The statements are selected by server-owned families. The key binds every
                // receipt's session, original command and character, plus generation where stored.
                let statement = match receipt {
                    ReplyReceipt::BossDifficulty =>
                        "SELECT false, reply FROM game_boss_difficulty_receipts, receipt_key k
                         WHERE game_session_id=k.session_id AND command_id=k.ordinal
                         AND character_id=k.actor_id AND connection_generation=k.generation",
                    ReplyReceipt::ContainerSort =>
                        "SELECT false, ''::bytea FROM game_item_container_sort_receipts, receipt_key k
                         WHERE game_session_id=k.session_id AND command_id=k.ordinal
                         AND character_id=k.actor_id AND world_id=k.world AND connection_generation=k.generation",
                    ReplyReceipt::GuildBank =>
                        "SELECT false, reply FROM game_guild_bank_operations, receipt_key k
                         WHERE game_session_id=k.session_id AND command_id=k.ordinal
                         AND actor=k.actor_id AND world_id=k.world",
                    ReplyReceipt::ManagedContainerMove =>
                        "SELECT false, int4send(moved::integer) FROM game_managed_container_move_receipts, receipt_key k
                         WHERE game_session_id=k.session_id AND command_id=k.ordinal
                         AND character_id=k.actor_id AND world_id=k.world",
                    ReplyReceipt::PlayerReport =>
                        "SELECT false, convert_to(result::text,'UTF8') FROM game_player_report_receipts, receipt_key k
                         WHERE game_session_id=k.session_id AND command_id=k.ordinal
                         AND character_id=k.actor_id",
                    ReplyReceipt::QuickLoot =>
                        "SELECT false, result FROM game_quick_loot_receipts, receipt_key k
                         WHERE game_session_id=k.session_id AND command_id=k.ordinal
                         AND character_id=k.actor_id",
                    ReplyReceipt::StashContainer =>
                        "SELECT false, int4send(quantity) FROM game_stash_container_receipts, receipt_key k
                         WHERE game_session_id=k.session_id AND command_id=k.ordinal
                         AND character_id=k.actor_id AND world_id=k.world",
                    ReplyReceipt::TeamFinderLeader =>
                        "SELECT NOT accepted, ''::bytea FROM game_team_finder_leader_receipts, receipt_key k
                         WHERE game_session_id=k.session_id AND command_id=k.ordinal
                         AND leader_character_id=k.actor_id",
                    ReplyReceipt::TeamFinderMember =>
                        "SELECT NOT accepted, ''::bytea FROM game_team_finder_member_receipts, receipt_key k
                         WHERE game_session_id=k.session_id AND command_id=k.ordinal
                         AND character_id=k.actor_id",
                    ReplyReceipt::Vip =>
                        "SELECT false, convert_to(result::text,'UTF8') FROM (
                            SELECT COALESCE(
                                (SELECT a.result FROM game_account_friend_receipts a
                                 WHERE a.game_session_id=k.session_id AND a.command_id=k.ordinal
                                 AND a.character_id=k.actor_id),
                                (SELECT v.result FROM game_vip_command_receipts v
                                 JOIN game_character_roots r USING (account_id, world_id)
                                 WHERE v.game_session_id=k.session_id AND v.command_id=k.ordinal
                                 AND r.character_id=k.actor_id AND v.character_id=k.actor_id AND v.world_id=k.world)
                            ) AS result FROM receipt_key k
                         ) receipts WHERE result IS NOT NULL",
                    ReplyReceipt::WorldEvent =>
                        "SELECT false, result FROM game_world_event_query_receipts, receipt_key k
                         WHERE game_session_id=k.session_id AND command_id=k.ordinal
                         AND character_id=k.actor_id AND world_id=k.world",
                };
                let sql = sqlx::AssertSqlSafe(format!(
                    "WITH receipt_key AS (
                        SELECT encode($1,'hex')::uuid AS session_id, $2::text::numeric AS ordinal,
                               encode($3,'hex')::uuid AS actor_id, $4::text::numeric AS generation,
                               encode($5,'hex')::uuid AS world
                     ) {statement}"
                ));
                let result: Option<(bool, Vec<u8>)> = sqlx::query_as(sql)
                    .bind(fence.game_session_id.as_bytes().as_slice())
                    .bind(command.get().to_string())
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(fence.connection_generation.get().to_string())
                    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                commit_semantic_read(tx, deadline).await?;
                let Some((rejected, payload)) = result else {
                    return Ok(LostCommandReply::Absent);
                };
                let payload = match receipt {
                    ReplyReceipt::ManagedContainerMove | ReplyReceipt::StashContainer => {
                        let quantity = u32::from_be_bytes(payload.try_into()
                            .map_err(|_| DurabilityError::InvalidStoredState)?);
                        oteryn_protocol_oteryn::container_batch::Reply::Committed(quantity)
                            .encode().to_vec()
                    }
                    ReplyReceipt::PlayerReport => {
                        let result = serde_json::from_slice(&payload)
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                        oteryn_protocol_oteryn::player_report::encode_player_report_reply(
                            &super::player_report::reply(&result)?,
                        ).map_err(|_| DurabilityError::InvalidStoredState)?
                    }
                    ReplyReceipt::Vip => {
                        let result = serde_json::from_slice(&payload)
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                        super::vip::encode_receipt(&result).ok_or(DurabilityError::InvalidStoredState)?
                    }
                    _ => payload,
                };
                Ok(LostCommandReply::Stored { rejected, payload })
            }))
            .await
    }
}

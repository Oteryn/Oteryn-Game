//! S04b World private-channel owner. Only metadata and immutable hashed receipts persist.
//! Every fresh operation re-proves recovery, node, session and negotiated connection custody.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{assert_session_fence, uuid_text};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::character_name::CharacterName;
use crate::domain::premium::PremiumStatus;
use oteryn_protocol_oteryn::chat::private_channel::{
    ChatChannelList, InvitedChannel, PrivateChannelIntent,
};
use oteryn_protocol_oteryn::chat::{ChatDisposition, ChatIntent, ChatIntentResult, ChatRoomSet};
use sha2::{Digest, Sha256};
use sqlx::{Acquire, Postgres, Row, Transaction};

mod mutations;
mod reads;

type Result<T> = std::result::Result<T, DurabilityError>;

pub enum Prepared {
    Replayed(ChatIntentResult),
    Fresh(Box<Write>),
}

/// Server-only coarse routing interest, always checked again under durable dequeue custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JoinedChannel {
    pub channel_id: u16,
    pub epoch: u64,
    pub since_micros: u64,
}

pub struct ChannelView {
    pub list: ChatChannelList,
    pub joined: Vec<JoinedChannel>,
}

/// A proposed encrypted publication; commit validates membership and the epoch again.
pub struct Publication {
    pub channel_id: u16,
    pub epoch: u64,
    pub sender_name: CharacterName,
}

pub struct Write {
    pub account: [u8; 16],
    pub publication: Option<Publication>,
    root: DurabilityRoot,
    recovery: CharacterRecoveryFenceV1,
    node: NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    command: u64,
    digest: [u8; 32],
    request: PrivateChannelIntent,
}

fn digest(request: &PrivateChannelIntent) -> Result<[u8; 32]> {
    let wire = oteryn_protocol_oteryn::chat::encode_chat_intent(&ChatIntent::PrivateChannel(
        request.clone(),
    ))
    .map_err(|_| DurabilityError::Unavailable)?;
    Ok(Sha256::digest(wire).into())
}

fn result(disposition: ChatDisposition) -> ChatIntentResult {
    ChatIntentResult {
        disposition,
        wait_seconds: 0,
    }
}

fn disposition(code: i16) -> Result<ChatDisposition> {
    Ok(match code {
        1 => ChatDisposition::Ok,
        2 => ChatDisposition::Muted,
        3 => ChatDisposition::Exhausted,
        4 => ChatDisposition::LevelTooLow,
        5 => ChatDisposition::NotOnline,
        6 => ChatDisposition::NoVocation,
        7 => ChatDisposition::RoomNotOpen,
        8 => ChatDisposition::ChatUnavailable,
        9 => ChatDisposition::Rejected,
        10 => ChatDisposition::NotAllowed,
        11 => ChatDisposition::NotInvited,
        _ => return Err(DurabilityError::Unavailable),
    })
}

async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    command: u64,
    binding: &[u8; 32],
) -> Result<Option<ChatIntentResult>> {
    let row = sqlx::query("SELECT world_id=encode($3,'hex')::uuid AND character_id=encode($4,'hex')::uuid AND binding=$5 AS matches,disposition,wait_seconds FROM game_private_chat_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(binding.as_slice()).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        if row.try_get::<bool, _>("matches")? {
            Ok(ChatIntentResult {
                disposition: disposition(row.try_get("disposition")?)?,
                wait_seconds: u32::try_from(row.try_get::<i32, _>("wait_seconds")?)
                    .map_err(|_| DurabilityError::Unavailable)?,
            })
        } else {
            Ok(result(ChatDisposition::Rejected))
        }
    })
    .transpose()
}

/// Serialize private-channel changes only, independently for each World. The key is never
/// used for authority; both the row keys and the complete session fence carry the WorldId.
async fn lock_world(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
) -> Result<()> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('private-chat:' || encode($1,'hex'),0))",
    )
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn current(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    negotiated: bool,
) -> Result<Option<[u8; 16]>> {
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    let Ok(account) = assert_session_fence(tx, fence, node).await? else {
        return Ok(None);
    };
    let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND session_state=2 AND current_transport_ref IS NOT NULL AND control_loss_epoch IS NULL)")
        .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if !active {
        return Ok(None);
    }
    if negotiated {
        let selected: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_private_chat_participants WHERE game_session_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid AND connection_generation=$4::text::numeric AND enabled)")
            .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
            .bind(fence.character_id.as_bytes().as_slice()).bind(fence.connection_generation.get().to_string()).fetch_one(&mut **tx).await?;
        if !selected {
            return Ok(None);
        }
    }
    Ok(Some(uuid_text(&account)?))
}

impl DurabilityRoot {
    pub async fn prepare_private_channel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: u64,
        request: PrivateChannelIntent,
    ) -> Result<Prepared> {
        if command == 0 {
            return Ok(Prepared::Replayed(result(ChatDisposition::Rejected)));
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let root = self.clone();
        let node = node.clone();
        let digest = digest(&request)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    if let Some(result) = replay(&mut tx, &fence, command, &digest).await? {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Prepared::Replayed(result));
                    }
                    let Some(account) = current(&mut tx, &recovery, &node, &fence, true).await?
                    else {
                        return Ok(Prepared::Replayed(result(ChatDisposition::Rejected)));
                    };
                    let publication = if let PrivateChannelIntent::Say { channel_id, .. } = &request
                    {
                        reads::publication(&mut tx, &fence, *channel_id).await?
                    } else {
                        None
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Prepared::Fresh(Box::new(Write {
                        account,
                        publication,
                        root,
                        recovery,
                        node,
                        fence,
                        command,
                        digest,
                        request,
                    })))
                })
            })
            .await
    }
}

impl Write {
    /// The encrypted relay NOTIFY and the terminal receipt commit together. No text or
    /// ciphertext is retained; replay returns the receipt and never notifies again.
    pub async fn commit(
        self,
        status: PremiumStatus<'_>,
        tx: Option<&mut Transaction<'_, Postgres>>,
        sealed: Option<(String, String)>,
    ) -> Result<ChatIntentResult> {
        let premium = !matches!(status, PremiumStatus::NotCurrent);
        if let Some(tx) = tx {
            if !matches!(status, PremiumStatus::NotActivated(_)) {
                return Err(DurabilityError::Unavailable);
            }
            let mut savepoint = tx.begin().await?;
            match self
                .execute(&mut savepoint, premium, sealed.as_ref(), None)
                .await
            {
                Ok(reply) => {
                    savepoint.commit().await?;
                    Ok(reply)
                }
                Err(error) => {
                    savepoint.rollback().await?;
                    Err(error)
                }
            }
        } else {
            if matches!(status, PremiumStatus::NotActivated(_)) {
                return Err(DurabilityError::Unavailable);
            }
            self.root
                .try_issue_semantic_pass()?
                .run(move |holder, deadline| {
                    Box::pin(async move {
                        let mut tx = begin_semantic_transaction(holder, deadline).await?;
                        let reply = self
                            .execute(&mut tx, premium, sealed.as_ref(), None)
                            .await?;
                        commit_semantic_transaction(tx, deadline).await?;
                        Ok(reply)
                    })
                })
                .await
        }
    }

    /// A terminal admission/rate/readiness refusal is as replayable as a successful
    /// mutation. It cannot turn into a publication after the original reply was sent.
    pub async fn refuse(self, refusal: ChatIntentResult) -> Result<ChatIntentResult> {
        if refusal.disposition == ChatDisposition::Ok {
            return Err(DurabilityError::Unavailable);
        }
        self.root
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let reply = self.execute(&mut tx, false, None, Some(refusal)).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(reply)
                })
            })
            .await
    }

    async fn execute(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        premium: bool,
        sealed: Option<&(String, String)>,
        refusal: Option<ChatIntentResult>,
    ) -> Result<ChatIntentResult> {
        assert_recovery_fence(tx, &self.recovery).await?;
        if let Some(reply) = replay(tx, &self.fence, self.command, &self.digest).await? {
            return Ok(reply);
        }
        if current(tx, &self.recovery, &self.node, &self.fence, true)
            .await?
            .is_none()
        {
            return Ok(result(ChatDisposition::Rejected));
        }
        lock_world(tx, &self.fence).await?;
        if let Some(reply) = replay(tx, &self.fence, self.command, &self.digest).await? {
            return Ok(reply);
        }
        let reply = if let Some(refusal) = refusal {
            refusal
        } else {
            result(
                mutations::apply(
                    tx,
                    &self.fence,
                    &self.request,
                    premium,
                    self.publication.as_ref(),
                    sealed,
                )
                .await?,
            )
        };
        sqlx::query("INSERT INTO game_private_chat_receipts(game_session_id,command_id,world_id,character_id,connection_generation,binding,disposition,wait_seconds) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,$8)")
            .bind(self.fence.game_session_id.as_bytes().as_slice()).bind(self.command.to_string())
            .bind(self.fence.runtime_scope.world_id().as_bytes().as_slice()).bind(self.fence.character_id.as_bytes().as_slice())
            .bind(self.fence.connection_generation.get().to_string()).bind(self.digest.as_slice()).bind(reply.disposition as i16).bind(i32::try_from(reply.wait_seconds).map_err(|_|DurabilityError::Unavailable)?).execute(&mut **tx).await?;
        Ok(reply)
    }
}

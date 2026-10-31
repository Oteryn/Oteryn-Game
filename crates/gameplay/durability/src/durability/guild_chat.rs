//! Connection-owner acceptance under the Guild reader's current durable custody.
//! The callback is synchronous. No socket I/O or asynchronous consumer work holds SQL locks.
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::fresh_admission::FreshAdmissionStore;
use super::guild::{GuildChatMembership, GuildError, read_current_guild_chat_data};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityRoot, character_authority::ReconciledCharacterAuthority};
use crate::domain::{character_name::CharacterName, guild::GuildId};
use crate::foundation::GameSessionState;

enum RoomOwner {
    Guild(Option<GuildId>),
    Party(Option<u64>),
}

impl DurabilityRoot {
    /// Reuses the Guild owner, retaining its locks until `accept` has taken custody of the
    /// result. Expected connection data is checked against canonical live admission again
    /// inside this transaction; no earlier observation is a grant.
    pub async fn with_guild_chat_custody<T: Send, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        payload_guild: Option<GuildId>,
        accept: F,
    ) -> Result<T, GuildError>
    where
        F: FnOnce(Option<GuildChatMembership>, CharacterName) -> T + Send,
    {
        self.with_chat_custody(
            authority,
            node,
            fence,
            RoomOwner::Guild(payload_guild),
            move |guild, _, name| accept(guild, name),
        )
        .await
    }

    /// The Party owner retains its World membership lock through synchronous egress custody.
    pub async fn with_party_chat_custody<T: Send, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        sent_at_micros: Option<u64>,
        accept: F,
    ) -> Result<T, GuildError>
    where
        F: FnOnce(Option<[u8; 16]>, CharacterName) -> T + Send,
    {
        self.with_chat_custody(
            authority,
            node,
            fence,
            RoomOwner::Party(sent_at_micros),
            move |_, party, name| accept(party, name),
        )
        .await
    }

    async fn with_chat_custody<T: Send, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        room: RoomOwner,
        accept: F,
    ) -> Result<T, GuildError>
    where
        F: FnOnce(Option<GuildChatMembership>, Option<[u8; 16]>, CharacterName) -> T + Send,
    {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        let store = FreshAdmissionStore::from_root(self.clone());
        let mut callback = Some(accept);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut callback, move |holder, deadline, callback| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let (guild, party) = match room {
                        RoomOwner::Guild(payload_guild) => {
                            match read_current_guild_chat_data(
                                &mut tx,
                                &recovery,
                                &node,
                                &fence,
                                payload_guild,
                            )
                            .await
                            {
                                Ok(data) => (data, None),
                                Err(GuildError::Unavailable(error)) => return Err(error),
                                Err(error) => return Ok(Err(error)),
                            }
                        }
                        RoomOwner::Party(sent_at_micros) => {
                            super::character_authority::assert_recovery_fence(&mut tx, &recovery)
                                .await?;
                            super::db::lock_admission_relations(&mut tx).await?;
                            if super::character_progression::assert_session_fence(
                                &mut tx, &fence, &node,
                            )
                            .await?
                            .is_err()
                            {
                                return Ok(Err(GuildError::NotCurrent));
                            }
                            let party = super::world_party::read_chat_membership_in_transaction(
                                &mut tx,
                                &fence,
                                sent_at_micros,
                            )
                            .await?;
                            (None, party)
                        }
                    };
                    let current = store
                        .current_session_in_transaction(&mut tx, fence.game_session_id)
                        .await?;
                    if current.session_state() != GameSessionState::Active
                        || current.current_game_session_id() != fence.game_session_id
                        || current.commit().character_id().as_bytes()
                            != fence.character_id.as_bytes()
                        || current.current_connection_generation() != fence.connection_generation
                        || current.current_character_lease().generation()
                            != fence.character_lease_generation
                        || current.current_runtime_scope() != fence.runtime_scope
                        || current.current_scope_generation() != fence.scope_ownership_generation
                    {
                        return Ok(Err(GuildError::NotCurrent));
                    }
                    let name =
                        super::character_chat_name::read_character_name_for_share_in_transaction(
                            &mut tx,
                            fence.character_id.as_bytes(),
                        )
                        .await?;
                    let Ok(name) = CharacterName::parse(&name) else {
                        return Ok(Err(GuildError::NotCurrent));
                    };
                    let Some(accept) = callback.take() else {
                        return Ok(Err(GuildError::NotCurrent));
                    };
                    // Linearization: the connection owner's synchronous callback reserves its
                    // in-flight slot while exclusion, resume and scope handoff are still locked out.
                    let accepted = accept(guild, party, name);
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(accepted))
                })
            })
            .await?
    }
}

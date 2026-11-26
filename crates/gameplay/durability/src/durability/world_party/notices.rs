use super::*;
use oteryn_protocol_oteryn::party::PartyMembershipChange;

pub struct MembershipNotice {
    pub character: [u8; 16],
    pub change: PartyMembershipChange,
}

/// Queue notices in the mutating transaction. For a join, the character is
/// the new member. For an invitation, the character is the invitee.
pub(super) async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    party: [u8; 16],
    character: [u8; 16],
    change: PartyMembershipChange,
) -> Result<()> {
    sqlx::query("SELECT game_enqueue_party_membership_notice(encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3)")
        .bind(party.as_slice())
        .bind(character.as_slice())
        .bind(change as i16)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

impl super::super::DurabilityRoot {
    pub async fn take_party_membership_notices(
        &self,
        authority: &super::super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &super::super::runtime_scope_assignment::NodeIncarnationProof,
        fence: super::super::item_transfer::CurrentCharacterItemFence,
        house: Option<super::super::house_scope_handoff::HouseId>,
    ) -> std::result::Result<Vec<MembershipNotice>, super::super::DurabilityError> {
        use super::super::DurabilityError;
        use super::super::db::{
            begin_semantic_transaction, commit_semantic_transaction, try_lock_admission_relations,
        };
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    // Most ticks have no work. The indexed probe observes only whether
                    // this exact mailbox could contain anything, and grants no custody.
                    let pending: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_party_membership_notices
                         WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid
                           AND connection_generation=$3::text::numeric(20,0))",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(fence.game_session_id.as_bytes().as_slice())
                    .bind(fence.connection_generation.get().to_string())
                    .fetch_one(&mut *tx).await?;
                    if !pending {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Vec::new());
                    }
                    super::super::character_authority::assert_recovery_fence(&mut tx, &recovery).await?;
                    let current = if let Some(house) = house {
                        try_lock_admission_relations(&mut tx).await?;
                        super::super::consume_transform::house_action_current_in_transaction(
                            &mut tx, &node, fence, &house,
                        ).await?
                    } else {
                        // Keep the exact session, lease, account and runtime guards
                        // stable through the mailbox delete. No reconnect journal
                        // or receipt table participates in that proof.
                        sqlx::query(
                            "LOCK TABLE game_durability_admission_account_guards,
                                        game_durability_admission_character_guards,
                                        game_durability_admission_runtime_guards,
                                        game_durability_reconnect_sessions
                             IN SHARE MODE NOWAIT",
                        )
                        .execute(&mut *tx)
                        .await?;
                        super::super::character_progression::assert_session_fence(
                            &mut tx, &fence, &node,
                        ).await?.is_ok()
                    };
                    if !current {
                        return Ok(Vec::new());
                    }
                    let events: Option<Vec<Vec<u8>>> = sqlx::query_scalar(
                        "DELETE FROM game_party_membership_notices n
                         USING game_durability_reconnect_sessions s,
                               game_durability_admission_character_guards g,
                               game_durability_admission_account_guards a
                         WHERE n.character_id=encode($1,'hex')::uuid AND n.game_session_id=encode($2,'hex')::uuid
                           AND n.connection_generation=$3::text::numeric(20,0)
                           AND s.game_session_id=n.game_session_id AND s.character_id=n.character_id
                           AND s.current_generation=n.connection_generation
                           AND s.session_state=2 AND g.character_id=n.character_id
                           AND g.eligible AND g.lease_generation=s.character_lease_generation
                           AND g.holder_game_session_id=n.game_session_id AND g.world_id=s.world_id
                           AND a.account_id=s.account_id AND a.account_id=g.account_id
                           AND a.holder_game_session_id=n.game_session_id AND a.presence_character_id=n.character_id
                         RETURNING n.events",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(fence.game_session_id.as_bytes().as_slice())
                    .bind(fence.connection_generation.get().to_string())
                    .fetch_optional(&mut *tx).await?;
                    let notices = events.into_iter().flatten().map(|event| {
                        let (&kind, character) = event.split_last().ok_or(DurabilityError::Unavailable)?;
                        let character = character.try_into().map_err(|_| DurabilityError::Unavailable)?;
                        let change = match kind {
                            1 => PartyMembershipChange::Joined,
                            2 => PartyMembershipChange::Left,
                            3 => PartyMembershipChange::Invited,
                            4 => PartyMembershipChange::Disbanded,
                            _ => return Err(DurabilityError::Unavailable),
                        };
                        Ok(MembershipNotice { character, change })
                    }).collect::<std::result::Result<Vec<_>, _>>()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(notices)
                })
            })
            .await
    }
}

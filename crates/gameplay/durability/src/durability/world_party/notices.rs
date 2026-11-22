use super::*;
use oteryn_protocol_oteryn::party::PartyMembershipChange;

pub struct MembershipNotice {
    pub character: [u8; 16],
    pub change: PartyMembershipChange,
}

/// Capture the current members in the mutating transaction. Join calls this
/// after insertion and leave before removal, so both include the changed member.
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
    ) -> std::result::Result<Vec<MembershipNotice>, super::super::DurabilityError> {
        use super::super::DurabilityError;
        use super::super::db::{
            begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
        };
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    super::super::character_authority::assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if super::super::character_progression::assert_session_fence(&mut tx, &fence, &node)
                        .await?.is_err()
                    {
                        return Ok(Vec::new());
                    }
                    let events: Option<Vec<Vec<u8>>> = sqlx::query_scalar(
                        "DELETE FROM game_party_membership_notices n USING game_durability_reconnect_sessions s
                         WHERE n.character_id=encode($1,'hex')::uuid AND n.game_session_id=encode($2,'hex')::uuid
                           AND n.connection_generation=$3::text::numeric(20,0)
                           AND s.game_session_id=n.game_session_id AND s.session_state=2
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

//! Reuse Task Board quest tracks/revision receipts in the Soulpit's open Type2 transaction.
use super::*;
use crate::domain::CharacterRevision;
use crate::durability::DurabilityError;
use crate::durability::{
    character_progression::CurrentCharacterGameplayFence, item_mint_audit::Type2Transaction,
    item_transfer::CurrentCharacterItemFence, soulpit::SoulpitWriteError,
};
use crate::foundation::CommandRef;
use sha2::{Digest, Sha256};
pub(crate) async fn spend_soulseals_in_transaction(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    fight: &SoulsealFight,
    intent: &[u8],
) -> Result<u64, SoulpitWriteError> {
    let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let gameplay = CurrentCharacterGameplayFence {
        character_id: fence.character_id,
        game_session_id: fence.game_session_id,
        connection_generation: fence.connection_generation,
        character_lease_generation: fence.character_lease_generation,
        runtime_scope: fence.runtime_scope,
        scope_ownership_generation: fence.scope_ownership_generation,
        expected_character_revision: CharacterRevision::new(
            revision
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?,
    };
    let before = super::state::load(tx, gameplay, true).await?;
    let mut view = before.clone();
    view.balances.soulseals = view
        .balances
        .soulseals
        .checked_sub(i64::from(fight.cost))
        .filter(|n| *n >= 0)
        .ok_or(SoulpitWriteError::InsufficientSoulseals)?;
    super::state::update_tracks(&mut view);
    let changes = super::state::plan_changes(&before, &mut view)?;
    if !super::state::fits_capacity(tx, gameplay, &before, &changes).await? {
        return Err(SoulpitWriteError::AuthorityRejected);
    }
    let binding: [u8; 32] = Sha256::new()
        .chain_update(b"oteryn:soulseal-entry:v1")
        .chain_update(intent)
        .chain_update(fight.species.as_bytes())
        .chain_update(fight.cost.to_le_bytes())
        .finalize()
        .into();
    let cause = QuestCause::Command(command);
    let transition =
        super::state::advance(tx, gameplay, cause, &binding, &view, changes, 0).await?;
    let remaining = view.balances.soulseals;
    let outcome = TaskBoardCommit {
        view,
        refusal: None,
        transition: Some(transition),
        replayed: false,
    };
    super::state::record_action(
        tx,
        gameplay,
        cause,
        &binding,
        serde_json::json!({"SoulsealFight":{"species":fight.species,"cost":fight.cost}}),
        &outcome,
        None,
    )
    .await?;
    u64::try_from(remaining).map_err(|_| DurabilityError::InvalidStoredState.into())
}

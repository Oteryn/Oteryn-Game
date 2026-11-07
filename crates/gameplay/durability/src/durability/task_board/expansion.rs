//! Trusted Game-side delivery seam. The Store offer/payment is owned elsewhere;
//! only an already authorized immutable delivery reference enters this owner.

use super::super::character_authority::ReconciledCharacterAuthority;
use super::super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};
use super::*;
use crate::foundation::CommandRef;
use sqlx::{Postgres, Row, Transaction};

impl DurabilityRoot {
    /// Grant the permanent entitlement after a trusted delivery owner has accepted
    /// its immutable reference. There is deliberately no matching wire action.
    /// The delivery reference is unique across sessions and cannot be rebound to
    /// another Character; replay returns the exact original successful snapshot.
    #[allow(clippy::too_many_arguments)]
    pub async fn grant_task_board_expansion(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        delivery_reference: &str,
        catalogue: Arc<TaskBoardCatalogue>,
    ) -> Result<TaskBoardCommit, CharacterProgressionError> {
        if delivery_reference.is_empty()
            || delivery_reference.len() > 256
            || !delivery_reference
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._:-/".contains(&byte))
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let mut binding = b"oteryn:taskboard:expansion-delivery:v1:".to_vec();
        binding.extend_from_slice(delivery_reference.as_bytes());
        self.commit_task_board_inner(
            authority,
            node,
            fence,
            TaskBoardCommand {
                cause: QuestCause::Command(command),
                action: TaskBoardAction::GrantExpansion,
                payload_binding: binding,
                fee_change: None,
            },
            catalogue,
            None,
            Some(delivery_reference.to_owned()),
        )
        .await
    }
}

pub(super) async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    delivery: &str,
) -> Result<Option<TaskBoardCommit>, CharacterProgressionError> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('oteryn:taskboard:expansion:'||$1,0))",
    )
    .bind(delivery)
    .execute(&mut **tx)
    .await
    .map_err(DurabilityError::from)?;
    let previous = sqlx::query("SELECT uuid_send(s.character_id) AS character,uuid_send(s.world_id) AS world,r.reply,r.refusal FROM game_task_board_expansion_deliveries s JOIN game_task_board_receipts r USING(character_id,cause_id,cause_ordinal) WHERE s.delivery_reference=$1")
        .bind(delivery).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
    let Some(previous) = previous else {
        return Ok(None);
    };
    if previous
        .try_get::<Vec<u8>, _>("character")
        .map_err(DurabilityError::from)?
        != fence.character_id.as_bytes()
        || previous
            .try_get::<Vec<u8>, _>("world")
            .map_err(DurabilityError::from)?
            != fence.runtime_scope.world_id().as_bytes()
    {
        return Err(CharacterProgressionError::ConflictingOccurrence);
    }
    Ok(Some(TaskBoardCommit {
        view: serde_json::from_value(previous.try_get("reply").map_err(DurabilityError::from)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        refusal: serde_json::from_value(
            previous.try_get("refusal").map_err(DurabilityError::from)?,
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        transition: None,
        replayed: true,
    }))
}

pub(super) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &TaskBoardCommand,
    delivery: &str,
) -> Result<(), DurabilityError> {
    let (_, cause, ordinal) = state::cause_key(request.cause);
    sqlx::query("INSERT INTO game_task_board_expansion_deliveries(delivery_reference,character_id,world_id,cause_id,cause_ordinal) VALUES($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric)")
        .bind(delivery).bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(cause.as_slice()).bind(ordinal.to_string())
        .execute(&mut **tx).await?;
    Ok(())
}

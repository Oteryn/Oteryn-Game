//! Promotion-scroll burns double as the permanent Wheel grants.
use super::*;
use crate::domain::CharacterId;
use crate::durability::db::{begin_semantic_transaction, commit_semantic_read};
use crate::durability::item_mint_audit::Type2Transaction;
use crate::foundation::CommandRef;
use oteryn_gameplay_progression::wheel::WHEEL_LEVEL_OFFSET;

pub(in crate::durability) const PROMOTION_SCROLL_CAUSE: u32 = 3358;

pub fn points(family: &str, key: &str) -> Option<u32> {
    if family != "Item" {
        return None;
    }
    match key {
        "oteryn:item.tibia.i43946" => Some(3),
        "oteryn:item.tibia.i43947" => Some(5),
        "oteryn:item.tibia.i43948" => Some(9),
        "oteryn:item.tibia.i43949" => Some(13),
        "oteryn:item.tibia.i43950" => Some(20),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromotionScrollOutcome {
    Redeemed,
    AlreadyCommitted,
    AlreadyUsed,
    LevelRequired,
}

/// The original server-selected source and write identity, kept with the wire reservation.
#[derive(Clone, Copy)]
pub struct PromotionScrollReceiptKey {
    pub transaction_id: [u8; 16],
    pub source: [u8; 16],
}

impl DurabilityRoot {
    /// A resume probe takes the writer's Character lock before proving either a commit
    /// or absence. It can resolve an old connection's exact receipt after control loss.
    pub async fn read_promotion_scroll_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        key: PromotionScrollReceiptKey,
    ) -> std::result::Result<bool, DurabilityError> {
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if command.game_session_id() != fence.game_session_id {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            sqlx::query("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let exact: Option<bool> = sqlx::query_scalar(
                "SELECT cause_kind=3358 AND shape=1 AND character_id=encode($3,'hex')::uuid \
                    AND world_id=encode($4,'hex')::uuid AND channel_id=encode($5,'hex')::uuid \
                    AND expected_session_generation=$6::text::numeric \
                    AND runtime_scope_ownership_generation=$7::text::numeric \
                    AND transaction_id=encode($8,'hex')::uuid AND source_item_instance_id=encode($9,'hex')::uuid \
                 FROM game_item_use_records WHERE game_session_id=encode($1,'hex')::uuid \
                    AND command_id=$2::text::numeric",
            )
            .bind(command.game_session_id().as_bytes().as_slice())
            .bind(command.command_id().get().to_string())
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(world_id.as_bytes().as_slice())
            .bind(channel_id.as_bytes().as_slice())
            .bind(fence.connection_generation.get().to_string())
            .bind(fence.scope_ownership_generation.get().to_string())
            .bind(key.transaction_id.as_slice())
            .bind(key.source.as_slice())
            .fetch_optional(&mut *tx).await?;
            let committed = match exact {
                Some(true) => true,
                Some(false) => return Err(DurabilityError::InvalidStoredState),
                None => false,
            };
            commit_semantic_read(tx, deadline).await?;
            Ok(committed)
        })).await
    }

    pub async fn redeem_promotion_scroll(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemUseRequest,
    ) -> Result<PromotionScrollOutcome> {
        if request.kind != ItemUseKind::PromotionScroll
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(ItemUseError::InvalidInput);
        }
        request.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match redeem(&mut tx, &fence, &node, &request).await {
                        Ok(
                            outcome @ (PromotionScrollOutcome::Redeemed
                            | PromotionScrollOutcome::AlreadyCommitted),
                        ) => {
                            tx.commit(deadline).await?;
                            Ok(Ok(outcome))
                        }
                        Ok(refusal) => Ok(Ok(refusal)),
                        Err(ItemUseError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

async fn redeem(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterItemFence,
    node: &NodeIncarnationProof,
    request: &ItemUseRequest,
) -> Result<PromotionScrollOutcome> {
    if replay(tx, fence, request).await?.is_some() {
        return Ok(PromotionScrollOutcome::AlreadyCommitted);
    }
    let root = match assert_item_only_fence(tx, fence, node).await? {
        Ok(root)
            if root.content_revision == request.content_revision
                && root.ruleset_revision == request.ruleset_revision =>
        {
            root
        }
        Ok(_) => return Err(ItemUseError::StaleState),
        Err(CharacterProgressionError::Unavailable(error)) => return Err(error.into()),
        Err(_) => return Err(ItemUseError::AuthorityRejected),
    };
    // An exact retry may have appeared while this transaction waited for the Character lock.
    if replay(tx, fence, request).await?.is_some() {
        return Ok(PromotionScrollOutcome::AlreadyCommitted);
    }
    let level: Option<i64> = sqlx::query_scalar(
        "SELECT level FROM game_character_progression_state \
         WHERE character_id=encode($1,'hex')::uuid AND character_revision=$2::text::numeric \
           AND profile_revision=$3 AND ruleset_revision=$4 AND content_revision=$5",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(root.revision.to_string())
    .bind(&root.profile_revision)
    .bind(&root.ruleset_revision)
    .bind(&root.content_revision)
    .fetch_optional(&mut ***tx)
    .await?;
    if level.ok_or(ItemUseError::StaleState)? <= i64::from(WHEEL_LEVEL_OFFSET) {
        return Ok(PromotionScrollOutcome::LevelRequired);
    }

    let already_used: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_use_records WHERE cause_kind=3358 \
          AND character_id=encode($1,'hex')::uuid AND source_definition_production_key=$2)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&request.source_definition.production_key)
    .fetch_one(&mut ***tx)
    .await?;
    if already_used {
        return Ok(PromotionScrollOutcome::AlreadyUsed);
    }
    let tuple = tx.tuple();
    match write(tx, fence, request, tuple).await? {
        ItemUseOutcome::Written { .. } => Ok(PromotionScrollOutcome::Redeemed),
        ItemUseOutcome::AlreadyCommitted { .. } => Ok(PromotionScrollOutcome::AlreadyCommitted),
    }
}

pub(in crate::durability) async fn read_points(
    connection: &mut PgConnection,
    character: CharacterId,
) -> std::result::Result<u32, DurabilityError> {
    // Migration 3358 constrains the keys and stores at most one of each per Character.
    let keys: Vec<String> = sqlx::query_scalar(
        "SELECT source_definition_production_key FROM game_item_use_records \
         WHERE cause_kind=3358 AND character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_all(connection)
    .await?;
    keys.iter()
        .map(|key| points("Item", key).ok_or(DurabilityError::InvalidStoredState))
        .sum()
}

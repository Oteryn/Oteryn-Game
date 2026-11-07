//! W09 Kooldown Aid: one canonical item-use burn and its activation receipt.
//! Crystal 201189f8 `data/scripts/systems/concoctions.lua:12-17` and
//! `data/libs/systems/concoctions.lua:179-205`: instant effect, per-kind 24h cooldown.
//! Receipt reads are historical evidence, never permission to reset current actor state.

use super::super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::*;
use crate::foundation::CommandRef;

pub const KOOLDOWN_AID_KEY: &str = "oteryn:item.tibia.i36723";

impl DurabilityRoot {
    /// Read the exact original intent before resolving a possibly consumed source handle.
    pub async fn read_kooldown_aid_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        binding: [u8; 32],
    ) -> Result<bool> {
        if command.game_session_id() != fence.game_session_id {
            return Err(ItemUseError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = sqlx::query(
                        "SELECT e.wire_binding, uuid_send(u.character_id) AS character, \
                                uuid_send(u.world_id) AS world, uuid_send(u.channel_id) AS channel \
                           FROM game_item_use_records u \
                           LEFT JOIN game_character_concoction_effects e USING(transaction_id) \
                          WHERE u.game_session_id=encode($1,'hex')::uuid \
                            AND u.command_id=$2::text::numeric(20,0) AND u.cause_kind=527 \
                            AND u.source_definition_production_key='oteryn:item.tibia.i36723'",
                    )
                    .bind(command.game_session_id().as_bytes().as_slice())
                    .bind(command.command_id().get().to_string())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let result = match row {
                        None => Ok(false),
                        Some(row) => {
                            let RuntimeScopeRefV1::Channel {
                                world_id,
                                channel_id,
                            } = fence.runtime_scope
                            else {
                                return Ok(Err(ItemUseError::AuthorityRejected));
                            };
                            let wire: Vec<u8> = row.try_get("wire_binding")?;
                            let character: Vec<u8> = row.try_get("character")?;
                            let world: Vec<u8> = row.try_get("world")?;
                            let channel: Vec<u8> = row.try_get("channel")?;
                            if wire.as_slice() != binding
                                || character.as_slice() != fence.character_id.as_bytes()
                                || world.as_slice() != world_id.as_bytes()
                                || channel.as_slice() != channel_id.as_bytes()
                            {
                                Err(ItemUseError::ConflictingCommand)
                            } else {
                                Ok(true)
                            }
                        }
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await?
    }

    /// Compose the activation with the existing canonical burn and audit, never a second
    /// inventory mutation. Unknown commit outcomes retain this exact request at the caller.
    pub async fn commit_kooldown_aid(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemUseRequest,
        binding: [u8; 32],
    ) -> Result<ItemUseOutcome> {
        request.validate()?;
        if request.kind != ItemUseKind::Concoction
            || request.source_definition.family != "Item"
            || request.source_definition.production_key != KOOLDOWN_AID_KEY
            || !matches!(request.flask, ItemUseFlaskPlan::None)
        {
            return Err(ItemUseError::InvalidInput);
        }
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
                    match replay(&mut tx, &fence, &request).await {
                        Ok(Some(outcome)) => {
                            let old: Option<Vec<u8>> = sqlx::query_scalar(
                                "SELECT wire_binding FROM game_character_concoction_effects \
                                  WHERE transaction_id=encode($1,'hex')::uuid \
                                    AND concoction='kooldown_aid'",
                            )
                            .bind(request.transaction_id.as_slice())
                            .fetch_optional(&mut **tx)
                            .await?
                            .flatten();
                            if old.as_deref() != Some(binding.as_slice()) {
                                return Ok(Err(ItemUseError::ConflictingCommand));
                            }
                            tx.commit(deadline).await?;
                            return Ok(Ok(outcome));
                        }
                        Ok(None) => {}
                        Err(ItemUseError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    let root = match assert_item_only_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(CharacterProgressionError::Unavailable(error)) => return Err(error),
                        Err(_) => return Ok(Err(ItemUseError::AuthorityRejected)),
                    };
                    if root.content_revision != request.content_revision
                        || root.ruleset_revision != request.ruleset_revision
                    {
                        return Ok(Err(ItemUseError::StaleState));
                    }
                    let cooling: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM game_character_concoction_effects \
                          WHERE character_id=encode($1,'hex')::uuid AND concoction='kooldown_aid' \
                            AND started_at+interval '24 hours'>public.game_daily_effect_now())",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_one(&mut **tx)
                    .await?;
                    if cooling {
                        return Ok(Err(ItemUseError::Cooldown));
                    }
                    let tuple = tx.tuple();
                    let outcome = match write(&mut tx, &fence, &request, tuple).await {
                        Ok(outcome) => outcome,
                        Err(ItemUseError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    };
                    sqlx::query(
                        "INSERT INTO game_character_concoction_effects \
                         (transaction_id,character_id,concoction,started_at,expires_at,wire_binding) \
                         SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid, \
                                'kooldown_aid',at,at,$3 \
                           FROM (SELECT public.game_daily_effect_now() AS at) instant",
                    )
                    .bind(request.transaction_id.as_slice())
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(binding.as_slice())
                    .execute(&mut **tx)
                    .await?;
                    tx.commit(deadline).await?;
                    Ok(Ok(outcome))
                })
            })
            .await?
    }
}

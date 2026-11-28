//! One fenced transaction converts the same stacks shown by the preview.
use super::*;
use oteryn_protocol_oteryn::{container_batch::Reply, item_view::ItemHandle};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgRow;
use std::num::NonZeroU64;

const MAX_STACKS: i64 = 500;

#[derive(Clone, Copy)]
pub struct Target {
    pub item: [u8; 16],
    pub handle: ItemHandle,
}

#[derive(Clone, Copy)]
pub struct Quote {
    fence: CurrentCharacterItemFence,
    target: Target,
    digest: [u8; 32],
}

#[derive(Clone, Copy)]
pub enum Request {
    Preview(Target),
    Confirm {
        preview: NonZeroU64,
        quote: Option<Quote>,
    },
}

pub struct ResultView {
    pub reply: Reply,
    pub quote: Option<Quote>,
}

struct Stacks {
    rows: Vec<PgRow>,
    quantity: u32,
    digest: [u8; 32],
}

async fn eligible_stacks<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    content: &C,
    native: &NativeGameplayState,
    target: Target,
) -> Result<Option<Stacks>, DurabilityError> {
    let world = fence.runtime_scope.world_id();
    let root = sqlx::query(
        "SELECT i.definition_family,i.definition_production_key AS definition_key,                 i.definition_revision_ref AS definition_revision          FROM game_item_instances i CROSS JOIN LATERAL game_item_loop_owner(i.item_instance_id) owner          WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1            AND i.world_id=encode($2,'hex')::uuid AND owner.owner_character=encode($3,'hex')::uuid",
    )
    .bind(target.item.as_slice())
    .bind(world.as_bytes().as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(root) = root else { return Ok(None) };
    if root.try_get::<String, _>("definition_family")? != "Item"
        || content.loop_facts(&definition(&root)?).and_then(|facts| facts.container_capacity).is_none()
    {
        return Ok(None);
    }
    let rows = sqlx::query(
        "WITH RECURSIVE edges AS NOT MATERIALIZED (            SELECT item_instance_id,parent_item_instance_id,placement_ordinal,false AS nested            FROM game_item_container_entries WHERE world_id=encode($2,'hex')::uuid            UNION ALL            SELECT item_instance_id,parent_item_instance_id,placement_ordinal,true AS nested            FROM game_item_nested_entries WHERE world_id=encode($2,'hex')::uuid          ), tree AS (            SELECT e.*,1 AS depth FROM edges e WHERE parent_item_instance_id=encode($1,'hex')::uuid            UNION ALL SELECT e.*,t.depth+1 FROM edges e JOIN tree t ON e.parent_item_instance_id=t.item_instance_id            WHERE t.depth<9          ) SELECT i.definition_family,i.definition_production_key AS definition_key,              i.definition_revision_ref AS definition_revision,i.quantity,to_jsonb(i) AS before_item,              tree.placement_ordinal::text AS ordinal,i.item_instance_id::text AS item,              uuid_send(tree.parent_item_instance_id) AS parent,tree.nested,tree.depth,              NOT EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)              AND NOT EXISTS(SELECT 1 FROM game_item_tiers WHERE item_instance_id=i.item_instance_id)              AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)              AND NOT EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)              AND NOT EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)              AND NOT EXISTS(SELECT 1 FROM edges WHERE parent_item_instance_id=i.item_instance_id)              AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=i.item_instance_id)              AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules WHERE item_instance_id=i.item_instance_id) AS plain            FROM tree JOIN game_item_instances i USING(item_instance_id)            WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1            ORDER BY i.item_instance_id LIMIT $3",
    )
    .bind(target.item.as_slice())
    .bind(world.as_bytes().as_slice())
    .bind(MAX_STACKS + 1)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > MAX_STACKS as usize {
        return Ok(None);
    }
    let mut stacks = Stacks { rows: Vec::new(), quantity: 0, digest: [0; 32] };
    let mut hash = Sha256::new();
    hash.update(content.loop_content_digest());
    hash.update(target.item);
    for row in rows {
        if row.try_get::<i32, _>("depth")? > 8 {
            return Ok(None);
        }
        if !row.try_get::<bool, _>("plain")? || row.try_get::<String, _>("definition_family")? != "Item" {
            continue;
        }
        let definition = definition(&row)?;
        let quantity = u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if !stowable(native, &definition)
            || !content.loop_facts(&definition).is_some_and(|facts| {
                facts.definition == definition
                    && facts.container_capacity.is_none()
                    && stack_maximum(facts.stack)
                        .is_ok_and(|(max, stackable)| stackable && (1..=max).contains(&quantity))
            })
        {
            continue;
        }
        let item: Value = row.try_get("before_item")?;
        hash.update(item.to_string().as_bytes());
        hash.update(row.try_get::<Vec<u8>, _>("parent")?);
        hash.update(row.try_get::<String, _>("ordinal")?.as_bytes());
        hash.update([u8::from(row.try_get::<bool, _>("nested")?)]);
        stacks.quantity += quantity;
        stacks.rows.push(row);
    }
    stacks.digest = hash.finalize().into();
    Ok((!stacks.rows.is_empty()).then_some(stacks))
}

pub struct Admission<'a> {
    pub permit: &'a super::super::spell_owner_commit::SpellLanePermit,
    pub recovery: &'a ReconciledCharacterAuthority<'a, 'a>,
    pub node: &'a NodeIncarnationProof,
    pub fence: CurrentCharacterItemFence,
    pub command: CommandRef,
    pub reachable: bool,
}

impl DurabilityRoot {
    pub async fn stash_container<C: ItemLoopContent>(
        &self,
        admission: Admission<'_>,
        request: Request,
        native: &NativeGameplayState,
        content: &C,
    ) -> Result<Option<ResultView>, DurabilityError> {
        let Admission { permit, recovery, node, fence, command, reachable } = admission;
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (permit, native, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, ctx| Box::pin(async move {
            let (permit, native, content) = *ctx;
            let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            let command_id = command.command_id().get().to_string();
            let session = fence.game_session_id.as_bytes();
            if command.game_session_id() != fence.game_session_id {
                return Ok(None);
            }
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:stash:' || encode($1,'hex') || ':' || $2,0))")
                .bind(session.as_slice()).bind(&command_id).execute(&mut *tx).await?;
            if let Request::Confirm { preview, .. } = request {
                let receipt = sqlx::query(
                    "SELECT character_id::text,world_id::text,preview_id::text,quantity FROM game_stash_container_receipts                      WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric",
                ).bind(session.as_slice()).bind(&command_id).fetch_optional(&mut *tx).await?;
                if let Some(receipt) = receipt {
                    let same = uuid_text(receipt.try_get("character_id")?)? == *fence.character_id.as_bytes()
                        && uuid_text(receipt.try_get("world_id")?)? == *fence.runtime_scope.world_id().as_bytes()
                        && receipt.try_get::<String, _>("preview_id")? == preview.to_string();
                    let quantity = u32::try_from(receipt.try_get::<i32, _>("quantity")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    super::super::db::commit_semantic_transaction(tx, deadline).await?;
                    return Ok(same.then_some(ResultView { reply: Reply::Committed(quantity), quote: None }));
                }
            }
            let (target, confirmation) = match request {
                Request::Preview(target) => (target, None),
                Request::Confirm { preview, quote: Some(quote) } if quote.fence == fence => (quote.target, Some((preview, quote.digest))),
                Request::Confirm { .. } => return Ok(None),
            };
            if !reachable || content.loop_content_digest() != native.source_digest() {
                return Ok(None);
            }
            permit.check_scope(fence.runtime_scope)?;
            assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), content.loop_content_digest())
                .await.map_err(|_| DurabilityError::Unavailable)?;
            let Some(stacks) = eligible_stacks(&mut tx, fence, content, native, target).await? else {
                return Ok(None);
            };
            let Some((preview, digest)) = confirmation else {
                super::super::db::commit_semantic_transaction(tx, deadline).await?;
                return Ok(Some(ResultView {
                    reply: Reply::Preview(stacks.quantity),
                    quote: Some(Quote { fence, target, digest: stacks.digest }),
                }));
            };
            if digest != stacks.digest {
                return Ok(None);
            }
            let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
            for (index, row) in stacks.rows.iter().enumerate() {
                let request = StashRequest {
                    command,
                    intent: StashIntent { page: 0, action: StashAction::Stow(target.handle) },
                    source: Some(uuid_text(row.try_get("item")?)?),
                    row_open: true,
                    access_refusal: None,
                };
                let planned = plan(&mut tx, fence, &request, native, content, Some(row)).await?;
                if planned.is_err() {
                    tx.rollback().await?;
                    return Ok(None);
                }
                let ordinal = i16::try_from(index + 1).map_err(|_| DurabilityError::InvalidStoredState)?;
                let intent = encode_stash_intent(request.intent).map_err(|_| DurabilityError::InvalidStoredState)?;
                let writer = StashWriter { fence, channel, request: &request, intent: &intent, house_key: None };
                writer.write(&mut tx, planned, 0, ordinal).await?;
                // The next stack may advance this same balance, so prove the current ledger head now.
                sqlx::raw_sql(
                    "SET CONSTRAINTS game_stash_consistent,game_stash_audit_consistent,game_item_instance_change_proven IMMEDIATE;                      SET CONSTRAINTS game_stash_consistent,game_stash_audit_consistent,game_item_instance_change_proven DEFERRED",
                ).execute(&mut *tx).await?;
            }
            sqlx::query(
                "INSERT INTO game_stash_container_receipts(game_session_id,command_id,character_id,world_id,preview_id,stacks,quantity)                  VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6,$7)",
            ).bind(session.as_slice()).bind(&command_id).bind(fence.character_id.as_bytes().as_slice())
                .bind(world.as_bytes().as_slice()).bind(preview.to_string())
                .bind(i16::try_from(stacks.rows.len()).map_err(|_| DurabilityError::InvalidStoredState)?)
                .bind(i32::try_from(stacks.quantity).map_err(|_| DurabilityError::InvalidStoredState)?)
                .execute(&mut *tx).await?;
            super::super::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(Some(ResultView { reply: Reply::Committed(stacks.quantity), quote: None }))
        })).await
    }
}

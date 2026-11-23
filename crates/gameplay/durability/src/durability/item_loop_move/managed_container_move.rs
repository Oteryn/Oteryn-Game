//! Move the confirmed carried stacks through the existing conserved item loop.
use super::*;
use crate::durability::character_quick_loot;
use oteryn_gameplay_items::quick_loot::Recipient;
use oteryn_protocol_oteryn::container_batch::Reply;
use sha2::{Digest, Sha256};
use std::num::NonZeroU64;

#[derive(Clone, Copy)]
pub struct Target {
    pub item: [u8; 16],
    pub recursive: bool,
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

struct Plan {
    moves: Vec<LoopMoveRequest>,
    digest: [u8; 32],
}

async fn plan<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    content: &C,
    target: Target,
) -> LoopResult<Plan> {
    let world = *fence.runtime_scope.world_id().as_bytes();
    if loop_owner(tx, target.item).await?.map(|(character, _)| character) != Some(*fence.character_id.as_bytes()) {
        return refuse(LoopMoveRefusal::Stale);
    }
    let Some((definition, _, _)) = lock_live_item(tx, target.item, world).await? else {
        return refuse(LoopMoveRefusal::Stale);
    };
    if content.loop_facts(&definition).and_then(|facts| facts.container_capacity).is_none() {
        return refuse(LoopMoveRefusal::NotSupported);
    }
    let settings = character_quick_loot::load(tx, fence.character_id.as_bytes()).await?;
    let rows = sqlx::query(
        "WITH RECURSIVE edges AS NOT MATERIALIZED (            SELECT item_instance_id,parent_item_instance_id,placement_ordinal            FROM game_item_container_entries WHERE world_id=encode($2,'hex')::uuid            UNION ALL SELECT item_instance_id,parent_item_instance_id,placement_ordinal            FROM game_item_nested_entries WHERE world_id=encode($2,'hex')::uuid          ), tree AS (            SELECT e.*,1 AS depth FROM edges e WHERE parent_item_instance_id=encode($1,'hex')::uuid            UNION ALL SELECT e.*,t.depth+1 FROM edges e JOIN tree t ON e.parent_item_instance_id=t.item_instance_id            WHERE t.depth<$3          ) SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref,              tree.parent_item_instance_id::text,tree.placement_ordinal::text,tree.depth,              i.quantity,i.state_revision::text            FROM tree JOIN game_item_instances i USING(item_instance_id)            WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1            ORDER BY i.item_instance_id LIMIT $4",
    )
    .bind(target.item.as_slice())
    .bind(world.as_slice())
    .bind(if target.recursive { ITEM_LOOP_DEPTH_MAX + 1 } else { 1 })
    .bind(ITEM_LOOP_TREE_ITEMS_MAX + 1)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > ITEM_LOOP_TREE_ITEMS_MAX as usize {
        return refuse(LoopMoveRefusal::NotSupported);
    }

    let mut moves = Vec::new();
    let mut hash = Sha256::new();
    hash.update(content.loop_content_digest());
    hash.update(target.item);
    hash.update([u8::from(target.recursive)]);
    let mut previous = None;
    for row in rows {
        let item = uuid_text(row.try_get("item_instance_id")?)?;
        if previous.replace(item) == Some(item)
            || row.try_get::<i32, _>("depth")? > ITEM_LOOP_DEPTH_MAX {
            return refuse(LoopMoveRefusal::NotSupported);
        }
        let definition = definition_of(&row)?;
        let Some(facts) = content.loop_facts(&definition) else { continue };
        if facts.container_capacity.is_some() {
            continue;
        }
        let category = quick_loot::preferred_category(content, &definition, |category| {
            settings.category_targets.contains_key(&category)
        });
        let recipient = settings.category_targets.get(&category).copied().unwrap_or(settings.unassigned_target);
        let target_item = match recipient {
            Recipient::MainContainer => continue,
            Recipient::Container(id) => id,
            Recipient::GoldPouch(id) if quick_loot::is_loot_coin(&definition) => id,
            Recipient::GoldPouch(_) => continue,
        };
        let parent = uuid_text(row.try_get("parent_item_instance_id")?)?;
        if parent == target_item {
            continue;
        }
        let destination = quick_loot::recipient(tx, fence.character_id, world, content, recipient).await?;
        hash.update(item);
        hash.update(parent);
        hash.update(target_item);
        hash.update(stored_u64(row.try_get("placement_ordinal")?)?.to_be_bytes());
        hash.update(stored_u64(row.try_get("state_revision")?)?.to_be_bytes());
        hash.update(row.try_get::<i64, _>("quantity")?.to_be_bytes());
        moves.push(LoopMoveRequest { item, count: None, pickup_cell: None, destination });
    }
    if moves.is_empty() {
        return refuse(LoopMoveRefusal::Stale);
    }
    Ok(Plan { moves, digest: hash.finalize().into() })
}

pub struct Admission<'a> {
    pub permit: &'a SpellLanePermit,
    pub recovery: &'a ReconciledCharacterAuthority<'a, 'a>,
    pub node: &'a NodeIncarnationProof,
    pub fence: CurrentCharacterItemFence,
    pub command: CommandRef,
    pub allowed: bool,
}

impl DurabilityRoot {
    pub async fn move_to_managed_containers<C: ItemLoopContent>(
        &self,
        admission: Admission<'_>,
        request: Request,
        content: &C,
    ) -> std::result::Result<Option<ResultView>, DurabilityError> {
        let Admission { permit, recovery, node, fence, command, allowed } = admission;
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (permit, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, ctx| Box::pin(async move {
            let (permit, content) = *ctx;
            let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            if command.game_session_id() != fence.game_session_id {
                return Ok(None);
            }
            lock_loop_command(&mut tx, command).await?;
            let session = fence.game_session_id.as_bytes();
            let command_id = command.command_id().get().to_string();
            let world = fence.runtime_scope.world_id();
            if let Request::Confirm { preview, .. } = request {
                let receipt = sqlx::query(
                    "SELECT character_id::text,world_id::text,preview_id::text,moved FROM game_managed_container_move_receipts                      WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric",
                ).bind(session.as_slice()).bind(&command_id).fetch_optional(&mut *tx).await?;
                if let Some(receipt) = receipt {
                    let same = uuid_text(receipt.try_get("character_id")?)? == *fence.character_id.as_bytes()
                        && uuid_text(receipt.try_get("world_id")?)? == *world.as_bytes()
                        && receipt.try_get::<String, _>("preview_id")? == preview.to_string();
                    let moved = u32::try_from(receipt.try_get::<i16, _>("moved")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    super::super::db::commit_semantic_transaction(tx, deadline).await?;
                    return Ok(same.then_some(ResultView { reply: Reply::Committed(moved), quote: None }));
                }
            }
            let (target, confirmation) = match request {
                Request::Preview(target) => (target, None),
                Request::Confirm { preview, quote: Some(quote) } if quote.fence == fence => (quote.target, Some((preview, quote.digest))),
                Request::Confirm { .. } => return Ok(None),
            };
            if !allowed {
                return Ok(None);
            }
            permit.check_scope(fence.runtime_scope)?;
            let attempt: LoopResult<Option<ResultView>> = async {
                assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), content.loop_content_digest()).await?;
                let plan = plan(&mut tx, fence, content, target).await?;
                let moved = u32::try_from(plan.moves.len()).map_err(|_| DurabilityError::InvalidStoredState)?;
                let Some((preview, digest)) = confirmation else {
                    return Ok(Some(ResultView { reply: Reply::Preview(moved), quote: Some(Quote { fence, target, digest: plan.digest }) }));
                };
                if digest != plan.digest {
                    return refuse(LoopMoveRefusal::Stale);
                }
                for (index, request) in plan.moves.iter().enumerate() {
                    let intent = intent_bytes(fence.character_id, request)?;
                    let ordinal = i16::try_from(index + 1).map_err(|_| DurabilityError::InvalidStoredState)?;
                    apply_loop_move(&mut tx, permit, &record, &node, &fence, command, content, request,
                        &intent, None, Some(MoveBatch::Managed(ordinal)), None, None).await?;
                    // A later stack may merge into the same receiver.
                    sqlx::query("SET CONSTRAINTS game_item_loop_line_proven DEFERRED")
                        .execute(&mut *tx).await?;
                }
                sqlx::query(
                    "INSERT INTO game_managed_container_move_receipts(game_session_id,command_id,character_id,world_id,preview_id,moved)                      VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6)",
                ).bind(session.as_slice()).bind(&command_id).bind(fence.character_id.as_bytes().as_slice())
                    .bind(world.as_bytes().as_slice()).bind(preview.to_string())
                    .bind(i16::try_from(moved).map_err(|_| DurabilityError::InvalidStoredState)?)
                    .execute(&mut *tx).await?;
                Ok(Some(ResultView { reply: Reply::Committed(moved), quote: None }))
            }.await;
            match attempt {
                Ok(result) => {
                    super::super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                }
                Err(error) => {
                    tx.rollback().await?;
                    loop_error(error).map(|_| None)
                }
            }
        })).await
    }
}

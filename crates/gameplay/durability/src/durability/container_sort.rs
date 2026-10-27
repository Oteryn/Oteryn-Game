//! Ordinal-only reorder, owned by item_loop_move. Custody and item state never change.
use super::*;
use oteryn_protocol_oteryn::container_sort::{Request, SortKey};
use std::collections::BTreeMap;

pub trait ContainerSortContent: ItemLoopContent {
    fn sort_name(&self, definition: &TypedDefinitionRef) -> Option<String>;
}

#[derive(Debug)]
struct Entry {
    id: [u8; 16],
    parent: [u8; 16],
    ordinal: u64,
    nested: bool,
    definition: TypedDefinitionRef,
    quantity: u32,
    bag: bool,
    name: String,
    weight: u64,
    expiry: Option<i64>,
}

fn compare(a: &Entry, b: &Entry, request: Request) -> std::cmp::Ordering {
    let key = match request.key {
        SortKey::Name => a.name.cmp(&b.name),
        SortKey::Weight => a.weight.cmp(&b.weight),
        SortKey::StackSize => a.quantity.cmp(&b.quantity),
        // Some expiry precedes None, including a paused timer's remaining time.
        SortKey::Expiry => match (a.expiry, b.expiry) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        },
    };
    b.bag
        .cmp(&a.bag)
        .then(if request.descending {
            key.reverse()
        } else {
            key
        })
        .then_with(|| {
            a.definition
                .production_key
                .cmp(&b.definition.production_key)
        })
        .then_with(|| a.id.cmp(&b.id))
}

fn permutation(entries: &[Entry], target: [u8; 16], request: Request) -> serde_json::Value {
    let mut groups: BTreeMap<[u8; 16], Vec<&Entry>> = BTreeMap::new();
    for entry in entries {
        if request.recursive || entry.parent == target {
            groups.entry(entry.parent).or_default().push(entry);
        }
    }
    let mut result = Vec::new();
    for group in groups.values_mut() {
        let mut ordinals: Vec<_> = group.iter().map(|e| e.ordinal).collect();
        ordinals.sort_unstable_by(|a, b| b.cmp(a)); // Views read newest first.
        group.sort_by(|a, b| compare(a, b, request));
        for (entry, ordinal) in group.iter().zip(ordinals) {
            result.push(json!({
                "item": uuid_string(entry.id), "parent": uuid_string(entry.parent),
                "nested": entry.nested, "before": entry.ordinal.to_string(), "after": ordinal.to_string(),
            }));
        }
    }
    json!(result)
}
fn uuid_string(id: [u8; 16]) -> String {
    let h: String = id.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}

#[allow(clippy::too_many_arguments)]
async fn reorder<C: ContainerSortContent>(
    tx: &mut Transaction<'_, Postgres>,
    record: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    content: &C,
    target: [u8; 16],
    request: Request,
    intent: &[u8],
) -> LoopResult<()> {
    assert_current_equipment_in_transaction(
        tx,
        record,
        node,
        fence,
        Some(command),
        content.loop_content_digest(),
    )
    .await?;
    let (world, _) = scope_of(fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
    let main = load_slot(tx, fence.character_id)
        .await?
        .is_some_and(|s| s.item_instance_id == target);
    if !main
        && !loop_owner(tx, target)
            .await?
            .is_some_and(|(owner, _)| owner == *fence.character_id.as_bytes())
    {
        return refuse(LoopMoveRefusal::Stale);
    }
    let rows = sqlx::query(
        "WITH RECURSIVE edges AS NOT MATERIALIZED ( \
           SELECT item_instance_id,parent_item_instance_id,placement_ordinal,false AS nested FROM game_item_container_entries \
           UNION ALL SELECT item_instance_id,parent_item_instance_id,placement_ordinal,true FROM game_item_nested_entries \
         ), tree AS ( \
           SELECT e.*,1 AS depth FROM edges e WHERE parent_item_instance_id=encode($1,'hex')::uuid \
           UNION ALL SELECT e.*,t.depth+1 FROM edges e JOIN tree t ON e.parent_item_instance_id=t.item_instance_id WHERE t.depth<9 \
         ) SELECT item_instance_id::text,parent_item_instance_id::text,placement_ordinal::text,nested,depth FROM tree ORDER BY item_instance_id LIMIT 501"
    ).bind(target.as_slice()).fetch_all(&mut **tx).await?;
    if rows.len() > ITEM_LOOP_TREE_ITEMS_MAX as usize
        || rows
            .iter()
            .any(|r| r.get::<i32, _>("depth") > ITEM_LOOP_DEPTH_MAX)
    {
        return refuse(LoopMoveRefusal::NotSupported);
    }
    let mut ids = vec![target];
    for row in &rows {
        ids.push(uuid_text(row.try_get("item_instance_id")?)?);
    }
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return refuse(LoopMoveRefusal::NotSupported);
    }
    for id in ids {
        if lock_live_item(tx, id, *world.as_bytes()).await?.is_none() {
            return refuse(LoopMoveRefusal::Stale);
        }
    }
    let Some((definition, _, _)) = lock_live_item(tx, target, *world.as_bytes()).await? else {
        return refuse(LoopMoveRefusal::Stale);
    };
    if content
        .loop_facts(&definition)
        .and_then(|f| f.container_capacity)
        .is_none()
    {
        return refuse(LoopMoveRefusal::NotSupported);
    }
    let now: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM transaction_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        if !request.recursive
            && request.key != SortKey::Weight
            && uuid_text(row.try_get("parent_item_instance_id")?)? != target
        {
            continue;
        }
        let id = uuid_text(row.try_get("item_instance_id")?)?;
        let Some((definition, quantity, _)) = lock_live_item(tx, id, *world.as_bytes()).await?
        else {
            return refuse(LoopMoveRefusal::Stale);
        };
        let facts = content
            .loop_facts(&definition)
            .ok_or(LoopError::Refused(LoopMoveRefusal::Rejected))?;
        // Fail closed only on metadata required by the selected key.
        let name = if request.key == SortKey::Name {
            content
                .sort_name(&definition)
                .ok_or(LoopError::Refused(LoopMoveRefusal::Rejected))?
                .to_lowercase()
        } else {
            String::new()
        };
        let weight = if request.key == SortKey::Weight {
            u64::from(
                content
                    .weight_of(&definition)
                    .map_err(|_| LoopError::Refused(LoopMoveRefusal::WeightUnknown))?,
            ) * u64::from(quantity)
        } else {
            0
        };
        let expiry: Option<i64> = sqlx::query_scalar("SELECT CASE WHEN deadline_at IS NULL THEN remaining_ms ELSE GREATEST(0,deadline_at-$2) END FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id.as_slice()).bind(now).fetch_optional(&mut **tx).await?.flatten();
        entries.push(Entry {
            id,
            parent: uuid_text(row.try_get("parent_item_instance_id")?)?,
            ordinal: stored_u64(row.try_get("placement_ordinal")?)?,
            nested: row.try_get("nested")?,
            definition,
            quantity,
            bag: facts.container_capacity.is_some(),
            name,
            weight,
            expiry,
        });
    }
    if request.key == SortKey::Weight {
        // Add each descendant's own stack weight to every ancestor in this subtree.
        let weights: Vec<_> = entries.iter().map(|e| (e.parent, e.weight)).collect();
        for (mut parent, weight) in weights {
            for _ in 0..ITEM_LOOP_DEPTH_MAX {
                let Some(ancestor) = entries.iter_mut().find(|e| e.id == parent) else {
                    break;
                };
                ancestor.weight = ancestor
                    .weight
                    .checked_add(weight)
                    .ok_or(DurabilityError::InvalidStoredState)?;
                parent = ancestor.parent;
            }
        }
    }
    let permutation = permutation(&entries, target, request);
    sqlx::query("INSERT INTO game_item_container_sort_receipts(game_session_id,command_id,character_id,world_id,connection_generation,lease_generation,scope_generation,content_digest,intent,permutation) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6::text::numeric,$7::text::numeric,$8,$9,$10)")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
        .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(content.loop_content_digest().as_slice()).bind(intent).bind(&permutation).execute(&mut **tx).await?;
    sqlx::query("SET CONSTRAINTS game_item_container_entries_sort_ordinals,game_item_nested_entries_sort_ordinals DEFERRED").execute(&mut **tx).await?;
    for table in ["game_item_container_entries", "game_item_nested_entries"] {
        let nested = table == "game_item_nested_entries";
        let sql = format!(
            "UPDATE {table} e SET placement_ordinal=(x->>'after')::numeric FROM jsonb_array_elements($1::jsonb) x WHERE e.item_instance_id=(x->>'item')::uuid AND (x->>'nested')::boolean=$2 AND e.placement_ordinal<>(x->>'after')::numeric"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(&permutation)
            .bind(nested)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

fn request_intent(target: [u8; 16], request: Request) -> Vec<u8> {
    let mut intent = target.to_vec();
    intent.extend(request.encode());
    intent
}
async fn receipt(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    command: CommandRef,
    target: Option<[u8; 16]>,
    request: Request,
) -> LoopResult<Option<bool>> {
    let row=sqlx::query("SELECT character_id::text,intent FROM game_item_container_sort_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        let intent: Vec<u8> = row.try_get("intent")?;
        Ok(
            uuid_text(row.try_get("character_id")?)? == *character.as_bytes()
                && intent.get(16..) == Some(request.encode().as_slice())
                && target.is_none_or(|id| intent.get(..16) == Some(id.as_slice())),
        )
    })
    .transpose()
}

impl DurabilityRoot {
    /// Reconcile historical success before current trade, actor or Content gates.
    /// The command lock also waits for a previous uncertain attempt to finish.
    pub async fn read_item_loop_container_sort_receipt(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        command: CommandRef,
        target: Option<[u8; 16]>,
        request: Request,
    ) -> std::result::Result<Option<bool>, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::super::db::begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    lock_loop_command(&mut tx, command).await?;
                    let found = receipt(&mut tx, character, command, target, request)
                        .await
                        .map_err(|error| match error {
                            LoopError::Durability(error) => error,
                            LoopError::Database(_) | LoopError::Refused(_) => {
                                DurabilityError::Unavailable
                            }
                        })?;
                    super::super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(found)
                })
            })
            .await
    }
    /// Same item lane, current item fence and semantic transaction as moves. The
    /// caller resolves an open view and excludes physical trade reservations.
    #[allow(clippy::too_many_arguments)]
    pub async fn sort_item_loop_container<C: ContainerSortContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        target: [u8; 16],
        request: Request,
    ) -> std::result::Result<bool, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let fence = *fence;
        let intent = request_intent(target, request);
        let mut context = content;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, content| {
                Box::pin(async move {
                    let mut tx =
                        super::super::db::begin_semantic_transaction(holder, deadline).await?;
                    let attempt: LoopResult<bool> = async {
                        assert_recovery_fence(&mut tx, &record).await?;
                        lock_loop_command(&mut tx, command).await?;
                        if let Some(found) =
                            receipt(&mut tx, fence.character_id, command, Some(target), request)
                                .await?
                        {
                            return Ok(found);
                        }
                        if committed_loop_move(&mut tx, command, fence.character_id, &intent)
                            .await?
                            .is_some()
                        {
                            return Ok(false);
                        }
                        reorder(
                            &mut tx, &record, &node, &fence, command, *content, target, request,
                            &intent,
                        )
                        .await?;
                        Ok(true)
                    }
                    .await;
                    match attempt {
                        Ok(true) => {
                            super::super::db::commit_semantic_transaction(tx, deadline).await?;
                            Ok(true)
                        }
                        Ok(false) => Ok(false),
                        Err(error) => loop_error(error).map(|_| false),
                    }
                })
            })
            .await
    }
}

#[cfg(test)]
#[path = "container_sort_tests.rs"]
mod tests;

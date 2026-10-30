//! Depot search retrieval: one command moves every matching depot or Inbox item, nested
//! entries included, into the main backpack as a batch of the item loop's conserved moves.
//! Canary 52550d5e `src/creatures/players/player.cpp:10813` retrieveAllItemsFromDepotSearch:
//! the items found by `requestDepotSearchItem` (:10763, ContainerIterator, nested included)
//! move until the inventory has no room, and what does not fit stays.
use super::*;
use crate::domain::premium::PremiumGate;

/// The CHECK bound of a child loop receipt's batch ordinal (migration 0227).
const RETRIEVE_ITEMS_MAX: i64 = 1280;

/// The terminal outcome of one retrieval command, also its replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepotSearchRetrieval {
    /// Units moved into the backpack; zero when nothing fit.
    Retrieved(u32),
    /// Nothing of the definition is held in depot or Inbox custody.
    NotFound,
}

/// The owner of the item nested at any depth under a Character's depot or Inbox root, the
/// Store Gold Pouch excluded (its coins are already carried).
pub(super) async fn custody_owner(
    tx: &mut Transaction<'_, Postgres>,
    item: [u8; 16],
) -> LoopResult<Option<[u8; 16]>> {
    let owner: Option<String> =
        sqlx::query_scalar("SELECT game_depot_search_custody_owner(encode($1,'hex')::uuid)::text")
            .bind(item.as_slice())
            .fetch_one(&mut **tx)
            .await?;
    optional_uuid(owner)
}

struct Candidate {
    item: [u8; 16],
    quantity: u32,
    depot: Option<(u16, u8)>,
    path: Vec<[u8; 16]>,
}

impl DurabilityRoot {
    /// Command 182 retrieval from depot and Inbox custody. Replay returns the stored outcome.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_depot_search_retrieval<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        client_id: u32,
        all: bool,
    ) -> std::result::Result<
        std::result::Result<DepotSearchRetrieval, LoopMoveRefusal>,
        DurabilityError,
    > {
        permit.check_scope(fence.runtime_scope)?;
        if client_id == 0 {
            return Ok(Err(LoopMoveRefusal::Rejected));
        }
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut request = client_id.to_be_bytes().to_vec();
        request.push(u8::from(all));
        let key = format!("oteryn:item.tibia.i{client_id}");
        let mut context = (permit, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let (permit, content) = *context;
            let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            lock_loop_command(&mut tx, command).await?;
            if let Some(row) = sqlx::query("SELECT character_id::text,request,found,retrieved FROM game_depot_search_retrieve_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut *tx).await? {
                if uuid_text(row.try_get("character_id")?)? != *fence.character_id.as_bytes() || row.try_get::<Vec<u8>,_>("request")? != request {
                    tx.rollback().await?;
                    return Ok(Err(LoopMoveRefusal::Rejected));
                }
                let retrieved = u32::try_from(row.try_get::<i64,_>("retrieved")?).map_err(|_| DurabilityError::InvalidStoredState)?;
                let found: bool = row.try_get("found")?;
                super::super::db::commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(if found { DepotSearchRetrieval::Retrieved(retrieved) } else { DepotSearchRetrieval::NotFound }));
            }
            let attempt: LoopResult<(bool, u32)> = async {
                assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), content.loop_content_digest()).await?;
                let (world, _) = scope_of(&fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                // Roots in fixed order, then their nested entries; each row carries its ancestry,
                // so an entry inside an already moved match travels with it, never twice.
                let rows = sqlx::query(r#"
                    WITH RECURSIVE roots AS (
                        SELECT d.item_instance_id, 4::smallint kind, d.town_id, d.box_number
                          FROM game_item_character_depot_locations d
                         WHERE d.character_id=encode($1,'hex')::uuid AND d.world_id=encode($2,'hex')::uuid
                        UNION ALL
                        SELECT x.item_instance_id, 6::smallint, NULL::int, NULL::smallint
                          FROM game_item_character_inbox_locations x JOIN game_item_instances p USING(item_instance_id)
                         WHERE x.character_id=encode($1,'hex')::uuid AND x.world_id=encode($2,'hex')::uuid
                           AND p.definition_production_key<>'oteryn:item.tibia.i23721'
                    ), tree AS (
                        SELECT item_instance_id, kind, town_id, box_number, 0 depth, ARRAY[item_instance_id::text] path FROM roots
                        UNION ALL
                        SELECT n.item_instance_id, t.kind, t.town_id, t.box_number, t.depth+1, t.path||n.item_instance_id::text
                          FROM tree t JOIN game_item_nested_entries n ON n.parent_item_instance_id=t.item_instance_id
                         WHERE t.depth<16
                    )
                    SELECT t.item_instance_id::text, t.depth, t.town_id, t.box_number, t.path, i.quantity
                      FROM tree t JOIN game_item_instances i USING(item_instance_id)
                     WHERE i.lifecycle=1 AND i.world_id=encode($2,'hex')::uuid AND i.definition_family='Item'
                       AND i.definition_production_key=$3
                     ORDER BY t.kind, t.town_id, t.box_number, t.path
                     LIMIT $4"#)
                    .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&key).bind(RETRIEVE_ITEMS_MAX)
                    .fetch_all(&mut *tx).await?;
                let mut candidates = Vec::with_capacity(rows.len());
                for row in rows {
                    let town: Option<i32> = row.try_get("town_id")?;
                    let box_number: Option<i16> = row.try_get("box_number")?;
                    let depth: i32 = row.try_get("depth")?;
                    candidates.push(Candidate {
                        item: uuid_text(row.try_get("item_instance_id")?)?,
                        quantity: stored_u32(row.try_get("quantity")?)?,
                        depot: match (depth, town, box_number) {
                            (0, Some(town), Some(box_number)) => Some((
                                u16::try_from(town).map_err(|_| DurabilityError::InvalidStoredState)?,
                                u8::try_from(box_number).map_err(|_| DurabilityError::InvalidStoredState)?,
                            )),
                            _ => None,
                        },
                        path: row.try_get::<Vec<String>,_>("path")?.iter().map(|id| uuid_text(id)).collect::<Result<_,_>>()?,
                    });
                }
                if candidates.is_empty() {
                    return Ok((false, 0));
                }
                let mut moved: Vec<[u8; 16]> = Vec::new();
                let mut retrieved = 0_u32;
                for candidate in candidates {
                    if candidate.path.iter().any(|ancestor| moved.contains(ancestor)) {
                        continue;
                    }
                    let ordinal = i16::try_from(moved.len() + 1).map_err(|_| DurabilityError::InvalidStoredState)?;
                    if !attempt_child(&mut tx, permit, &record, &node, &fence, command, content, &candidate, ordinal).await? {
                        // The backpack is full or the Character overweight: the rest stays.
                        break;
                    }
                    moved.push(candidate.item);
                    retrieved = retrieved.saturating_add(candidate.quantity);
                    if !all {
                        break;
                    }
                }
                Ok((true, retrieved))
            }.await;
            match attempt {
                Ok((found, retrieved)) => {
                    sqlx::query("INSERT INTO game_depot_search_retrieve_receipts(game_session_id,command_id,character_id,request,found,retrieved) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4,$5,$6)")
                        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice())
                        .bind(&request).bind(found).bind(i64::from(retrieved)).execute(&mut *tx).await?;
                    super::super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(if found { DepotSearchRetrieval::Retrieved(retrieved) } else { DepotSearchRetrieval::NotFound }))
                }
                Err(error) => { tx.rollback().await?; loop_error(error).map(Err) }
            }
        })).await
    }
}

/// One child move under a savepoint: a refusal (no room, overweight) rolls back only itself.
#[allow(clippy::too_many_arguments)]
async fn attempt_child<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    permit: &SpellLanePermit,
    record: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    content: &C,
    candidate: &Candidate,
    ordinal: i16,
) -> LoopResult<bool> {
    sqlx::query("SAVEPOINT depot_search_item")
        .execute(&mut **tx)
        .await?;
    let attempt: LoopResult<()> = async {
        let request = LoopMoveRequest {
            item: candidate.item,
            count: None,
            pickup_cell: None,
            destination: LoopDestination::MainBackpack,
        };
        let depot = candidate.depot.map(|(town, box_number)| DepotMove {
            town,
            box_number,
            deposit: false,
            request_digest: [0; 32],
            premium: PremiumGate::CLOSED,
        });
        let intent = serde_json::to_vec(&json!({"character": fence.character_id.as_bytes(), "request": request, "depot": depot, "depot_search": ordinal}))
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        apply_loop_move(
            tx,
            permit,
            record,
            node,
            fence,
            command,
            content,
            &request,
            &intent,
            depot,
            None,
            Some(ordinal),
        )
        .await?;
        // Each child proves its exact state before a later child may top it up.
        sqlx::query("SET CONSTRAINTS game_item_loop_line_proven IMMEDIATE")
            .execute(&mut **tx)
            .await?;
        sqlx::query("SET CONSTRAINTS game_item_loop_line_proven DEFERRED")
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
    .await;
    let accepted = match attempt {
        Ok(()) => true,
        Err(LoopError::Refused(
            LoopMoveRefusal::NoRoom | LoopMoveRefusal::Overweight | LoopMoveRefusal::NoBackpack,
        )) => false,
        Err(error) => return Err(error),
    };
    if !accepted {
        sqlx::query("ROLLBACK TO SAVEPOINT depot_search_item")
            .execute(&mut **tx)
            .await?;
    }
    sqlx::query("RELEASE SAVEPOINT depot_search_item")
        .execute(&mut **tx)
        .await?;
    Ok(accepted)
}

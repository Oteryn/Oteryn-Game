//! Retrieve a real mailed parcel with its descendants still inside, at the capacity boundary.
use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::ItemWeightLookup;
use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
use oteryn_protocol_oteryn::depot_search::{Outcome, Request};
use oteryn_protocol_oteryn::item_view::WorldTilePosition;

fn definition(client_id: u32) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{client_id}"),
        revision_ref: "definition-r1".into(),
    }
}

/// The batch outcome may be retained; a refused child must not change custody or value.
async fn snapshot(pool: &sqlx::PgPool) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object(\
           'items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),\
           'main',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_container_entries e),\
           'nested',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_nested_entries e),\
           'inbox',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_character_inbox_locations e),\
           'inbox_counter',(SELECT jsonb_agg(to_jsonb(c) ORDER BY character_id) FROM game_character_inbox_counters c),\
           'moves',(SELECT count(*) FROM game_item_loop_move_receipts),\
           'lines',(SELECT count(*) FROM game_item_loop_move_lines),\
           'audits',(SELECT count(*) FROM game_item_audit_outbox))",
    )
    .fetch_one(pool)
    .await?)
}

/// Previously acquired filler is fixture input. The attempted retrievals, the one-unit
/// drop that releases capacity, and both replays run through the production owners.
async fn seed_filler(
    pool: &sqlx::PgPool,
    character: [u8; 16],
    ingots: u64,
    coins: u64,
) -> TestResult<[u8; 16]> {
    assert!(coins > 0);
    let mut stacks = Vec::new();
    for (client_id, mut units) in [(9058, ingots), (3031, coins)] {
        while units > 0 {
            let quantity = units.min(100);
            stacks.push((client_id, quantity));
            units -= quantity;
        }
    }
    assert!(
        stacks.len() < 20,
        "reserve a real backpack slot for the parcel"
    );
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let mut coin = None;
    for (ordinal, (client_id, quantity)) in stacks.into_iter().enumerate() {
        let item = id(210 + u8::try_from(ordinal)?);
        sqlx::query(
            "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) \
             SELECT encode($1,'hex')::uuid,world_id,'Item',$3,'definition-r1',$4,1,encode($5,'hex')::uuid \
             FROM game_item_container_slots WHERE character_id=encode($2,'hex')::uuid",
        )
        .bind(item.as_slice())
        .bind(character.as_slice())
        .bind(definition(client_id).production_key)
        .bind(i32::try_from(quantity)?)
        .bind(id(230).as_slice())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) \
             SELECT encode($1,'hex')::uuid,world_id,character_id,item_instance_id,$3,encode($4,'hex')::uuid \
             FROM game_item_container_slots WHERE character_id=encode($2,'hex')::uuid",
        )
        .bind(item.as_slice())
        .bind(character.as_slice())
        .bind(i64::try_from(ordinal + 1)?)
        .bind(id(231).as_slice())
        .execute(&mut *tx)
        .await?;
        if client_id == 3031 {
            coin = Some(item);
        }
    }
    tx.commit().await?;
    coin.ok_or("capacity fixture needs a removable coin".into())
}

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    parcel: [u8; 16],
    label: [u8; 16],
    keg: [u8; 16],
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = admitted.item_fence.ok_or("fence")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let weights = NativeItemWeights(native);
    let weight = |client_id| -> TestResult<u64> {
        Ok(u64::from(
            weights.weight_of(&definition(client_id)).map_err(debug)?,
        ))
    };
    let backpack = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    assert!(backpack.entries.is_empty());
    assert!(
        owner.premium_coordinator.is_none(),
        "fixture has no Wheel bonus"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_equipment_slots")
            .fetch_one(pool)
            .await?,
        0
    );
    let (level, vocation): (i64, String) = sqlx::query_as(
        "SELECT p.level,COALESCE(b.vocation,'none') FROM game_character_progression_state p \
         LEFT JOIN game_character_build_state b USING(character_id) WHERE p.character_id=encode($1,'hex')::uuid",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(pool).await?;
    let capacity = oteryn_gameplay_items::item_capacity::project_global_capacity_centi_oz(
        u32::try_from(level)?,
        &vocation,
    )
    .map_err(debug)?;
    let carried = u64::from(
        weights
            .weight_of(&backpack.backpack.definition)
            .map_err(debug)?,
    ) * u64::from(backpack.backpack.quantity);
    let root = weight(3504)?;
    let subtree = root + weight(3507)? + weight(25904)?;
    let coin_weight = weight(3031)?;
    let ingot_weight = weight(9058)?;
    assert!(coin_weight > 0 && ingot_weight >= coin_weight);
    // Keep one removable coin even when the exact filler is a multiple of an ingot.
    let exact_filler = capacity
        .checked_sub(carried + subtree)
        .ok_or("fixture subtree fits")?;
    let ingots = exact_filler / ingot_weight;
    let coin_remainder = exact_filler % ingot_weight;
    assert_eq!(coin_remainder % coin_weight, 0);
    let coins = coin_remainder / coin_weight + 1;
    let filler = ingots * ingot_weight + coins * coin_weight;
    assert!(
        carried + filler + root <= capacity,
        "the root alone must fit"
    );
    assert_eq!(carried + filler + subtree, capacity + coin_weight);
    let coin = seed_filler(pool, *fence.character_id.as_bytes(), ingots, coins).await?;
    let request = Request::Retrieve {
        client_id: 3504,
        all: true,
    };
    let before = snapshot(pool).await?;
    assert_eq!(before["nested"].as_array().map(Vec::len), Some(2));
    let refused = owner
        .depot_search_command(actor, admitted, 600, request)
        .await;
    assert_eq!((refused.outcome, refused.retrieved), (Outcome::NoRoom, 0));
    assert_eq!(snapshot(pool).await?, before);
    assert_eq!(
        owner
            .depot_search_command(actor, admitted, 600, request)
            .await,
        refused
    );
    assert_eq!(snapshot(pool).await?, before);

    assert_eq!(
        owner
            .move_loop_item(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 601,
                    item_fence: Some(fence),
                },
                LoopMove {
                    item: ItemKey::Instance(coin),
                    count: Some(1),
                    target: LoopTarget::Ground(WorldTilePosition {
                        x: 7,
                        y: 0,
                        floor: -7
                    }),
                }
            )
            .await,
        LoopMoveResult::Committed
    );
    assert_eq!(carried + filler - coin_weight + subtree, capacity);
    let fitting = snapshot(pool).await?;
    // Releasing capacity does not change the already retained failed command.
    assert_eq!(
        owner
            .depot_search_command(actor, admitted, 600, request)
            .await,
        refused
    );
    assert_eq!(snapshot(pool).await?, fitting);
    let retrieved = owner
        .depot_search_command(actor, admitted, 602, request)
        .await;
    assert_eq!(
        (retrieved.outcome, retrieved.retrieved),
        (Outcome::Retrieved, 1)
    );
    let after = snapshot(pool).await?;
    let before_items = fitting["items"]
        .as_array()
        .ok_or("items before retrieval")?;
    let after_items = after["items"].as_array().ok_or("items after retrieval")?;
    assert_eq!(before_items.len(), after_items.len());
    for (before, after) in before_items.iter().zip(after_items) {
        let root = before["definition_production_key"] == "oteryn:item.tibia.i3504";
        let fields = before.as_object().ok_or("item before retrieval")?;
        assert_eq!(
            after.as_object().map(|value| value.len()),
            Some(fields.len())
        );
        for (field, value) in fields {
            match (root, field.as_str()) {
                (true, "state_revision") => assert_eq!(
                    after[field].as_u64(),
                    value.as_u64().and_then(|revision| revision.checked_add(1))
                ),
                (true, "last_transaction_id") => assert_ne!(after[field], *value),
                _ => assert_eq!(after[field], *value),
            }
        }
    }
    assert_eq!(after["nested"], fitting["nested"]);
    for child in [label, keg] {
        let parent: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(parent_item_instance_id) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(child.as_slice()).fetch_one(pool).await?;
        assert_eq!(parent, parcel);
    }
    let backpack = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    assert!(
        backpack
            .entries
            .iter()
            .any(|e| e.item.item_instance_id == parcel)
    );
    let actual_carried =
        backpack
            .entries
            .iter()
            .try_fold(carried, |total, entry| -> TestResult<u64> {
                Ok(total
                    + u64::from(weights.weight_of(&entry.item.definition).map_err(debug)?)
                        * u64::from(entry.item.quantity))
            })?
            + subtree
            - root;
    assert_eq!(
        actual_carried, capacity,
        "the committed entire tree exactly fits"
    );
    let inbox: (i64, String) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id=encode($1,'hex')::uuid),committed::text \
         FROM game_character_inbox_counters WHERE character_id=encode($2,'hex')::uuid",
    ).bind(parcel.as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_one(pool).await?;
    assert_eq!(
        inbox,
        (0, "1".into()),
        "only the letter remains in the Inbox"
    );
    assert_eq!(
        owner
            .depot_search_command(actor, admitted, 602, request)
            .await,
        retrieved
    );
    assert_eq!(snapshot(pool).await?, after);
    let receipts: Vec<(bool, i64)> = sqlx::query_as(
        "SELECT found,retrieved FROM game_depot_search_retrieve_receipts ORDER BY command_id",
    )
    .fetch_all(pool)
    .await?;
    assert_eq!(receipts, [(true, 0), (true, 1)]);
    Ok(())
}

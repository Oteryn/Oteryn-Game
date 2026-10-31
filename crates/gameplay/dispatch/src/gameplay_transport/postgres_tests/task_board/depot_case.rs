//! Deterministic historical custody exercises the admitted stash and depot shapes.
use super::*;
use crate::durability::task_board::TaskBoardItem;
use oteryn_gameplay_quests::quest::task_board::full::{DELIVERY_ITEMS, WeeklyDeliveryTask};

pub(super) async fn availability_snapshot_case(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
    item: &TaskBoardItem,
    required: u32,
) -> TestResult {
    let before = full_board(owner, admitted, 4, 0, "").await?;
    assert_eq!(
        before.weekly.deliveries[0].current, required,
        "carried, stash, flat depot and the depth-nine depot leaf are all counted"
    );
    let extra = historical_stash_counts(harness, item, &[75]).await?;
    let command = admitted.continuity.next_command_id;
    let original = serve_from(owner, admitted, command, &[full_request(4, 0, "")]).await?;
    assert_eq!(original[0].1, CommandStatus::Accepted);
    let available = wire::full::decode_reply(&original[0].2).map_err(debug)?;
    assert_eq!(
        available.weekly.deliveries[0].current,
        required + 75,
        "informative ownership is not capped to the requested delivery quantity"
    );
    remove_historical_stash_counts(harness, item, &extra).await?;
    assert_eq!(
        full_board(owner, admitted, 4, 0, "")
            .await?
            .weekly
            .deliveries[0]
            .current,
        required
    );
    let replay = serve_from(owner, admitted, command, &[full_request(4, 0, "")]).await?;
    assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
    assert_eq!(
        replay[0].2, original[0].2,
        "the earlier reply keeps its availability after ownership changes"
    );

    // The stash owner permits one billion per definition revision. Five valid
    // historical revisions cross the source display width without breaking that
    // bound or modifying the actual delivery cost. Their sum is 2^32 + 17.
    let extra = historical_stash_counts(
        harness,
        item,
        &[
            1_000_000_000,
            1_000_000_000,
            1_000_000_000,
            1_000_000_000,
            294_967_313,
        ],
    )
    .await?;
    let wrapped = full_board(owner, admitted, 4, 0, "").await;
    remove_historical_stash_counts(harness, item, &extra).await?;
    assert_eq!(
        wrapped?.weekly.deliveries[0].current,
        required + 17,
        "Crystal adds uint32 ownership counts modulo 2^32; no intermediate BIGINT overflow"
    );
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: delivery_availability_snapshot_source_width_and_replay"
    );
    Ok(())
}

/// Test-only historical balances preserve the existing per-revision stash bound.
/// Normal runtime queries and delivery still execute with every guard enabled.
async fn historical_stash_counts(
    harness: &Harness,
    item: &TaskBoardItem,
    quantities: &[i64],
) -> TestResult<Vec<(String, String)>> {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let mut inserted = Vec::new();
    for (index, &quantity) in quantities.iter().enumerate() {
        let revision = format!("task-board-availability-historical-{index}");
        let entry:String=sqlx::query_scalar("INSERT INTO game_character_stash_entries(entry_id,transaction_id,character_id,world_id,definition_key,definition_revision,amount,quantity_before,quantity_after) VALUES(game_character_uuid_v7(),game_character_uuid_v7(),encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,0,$5) RETURNING entry_id::text")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(&item.key).bind(&revision).bind(quantity)
            .fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_stash_balances(character_id,world_id,definition_key,definition_revision,quantity,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(&item.key).bind(&revision).bind(quantity).bind(&entry)
            .execute(&mut *tx).await?;
        inserted.push((revision, entry));
    }
    tx.commit().await?;
    Ok(inserted)
}

async fn remove_historical_stash_counts(
    harness: &Harness,
    item: &TaskBoardItem,
    inserted: &[(String, String)],
) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (revision, entry) in inserted {
        assert_eq!(sqlx::query("DELETE FROM game_character_stash_balances WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND definition_key=$3 AND definition_revision=$4 AND last_entry_id=$5::uuid")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(&item.key).bind(revision).bind(entry)
            .execute(&mut *tx).await?.rows_affected(),1);
        assert_eq!(
            sqlx::query("DELETE FROM game_character_stash_entries WHERE entry_id=$1::uuid")
                .bind(entry)
                .execute(&mut *tx)
                .await?
                .rows_affected(),
            1
        );
    }
    tx.commit().await?;
    Ok(())
}

pub(super) async fn fixture_stackable_deliveries(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
) -> TestResult {
    let mut data = fixture_board_data(harness).await?;
    let catalogue = owner.task_board_catalogue().ok_or("task catalogue")?;
    // Select deterministic historical custody only after proving that the real
    // generation admits the entire donor pool. Choosing two convenient native
    // rows must never hide an unfulfillable generated delivery elsewhere.
    assert_eq!(DELIVERY_ITEMS.len(), 436);
    for delivery in DELIVERY_ITEMS {
        assert!(
            catalogue.items.contains_key(&delivery.item_id),
            "donor delivery {} needs its qualified production Item profile and index entry",
            delivery.item_id
        );
    }
    use oteryn_gameplay_content::content::{ReferenceEquipmentSlot, ReferenceItemField};
    let native = owner
        .active_generation
        .ok_or("active generation")?
        .native_gameplay()
        .ok_or("native gameplay")?;
    let talisman = native
        .item_policy_for_source_id(51978)
        .ok_or("production bounty talisman policy")?;
    let talisman = talisman.record();
    assert!(
        matches!(&talisman.semantics.physical,ReferenceItemField::Known(physical)
        if physical.pickupable==ReferenceItemField::Known(true)
            && physical.weight==ReferenceItemField::Known(75)),
        "the pinned source talisman is pickupable and weighs 75 centi-ounces"
    );
    assert!(
        matches!(&talisman.semantics.equipment,ReferenceItemField::Known(equipment)
        if matches!(&equipment.patterns,ReferenceItemField::Known(patterns)
            if patterns.iter().any(|pattern|pattern.primary_slot
                ==ReferenceItemField::Known(ReferenceEquipmentSlot::Extra)))),
        "the pinned moveevent ammo slot must qualify the existing EXTRA/slot-10 owner"
    );
    let talisman = catalogue
        .items
        .get(&51978)
        .ok_or("admitted talisman catalogue entry")?;
    assert!(
        !talisman.stackable && talisman.mint.is_some(),
        "the existing Inbox mint owner must receive qualified nonstackable talisman facts"
    );
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: complete_donor_delivery_pool_and_native_talisman_admission"
    );
    let weekly = data.weekly.as_mut().ok_or("weekly board")?;
    let mut selected = Vec::new();
    for item in DELIVERY_ITEMS {
        if !catalogue
            .items
            .get(&item.item_id)
            .is_some_and(|facts| facts.stackable)
            || weekly
                .deliveries
                .iter()
                .skip(2)
                .any(|task| task.item_id == item.item_id)
        {
            continue;
        }
        let key = &catalogue.items[&item.item_id].key;
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1)")
            .bind(key).fetch_one(&harness.pool).await?;
        if exists {
            continue;
        }
        selected.push(WeeklyDeliveryTask {
            item_id: item.item_id,
            required: item.amount,
            delivered: false,
        });
        if selected.len() == 2 {
            break;
        }
    }
    assert_eq!(
        selected.len(),
        2,
        "two donor entries with admitted stackable custody"
    );
    weekly.deliveries[..2].clone_from_slice(&selected);
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_character_task_boards SET body=$2 WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .bind(serde_json::to_value(&data)?)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn fixture_large_nested_depot(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    item: &TaskBoardItem,
) -> TestResult {
    let native = owner
        .active_generation
        .ok_or("active generation")?
        .native_gameplay()
        .ok_or("native gameplay")?;
    let bag = native
        .item_policy_for_source_id(2854)
        .ok_or("admitted backpack")?;
    let definition = &bag.record().production_definition;
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    // All roots are legal within the existing 15,000-per-town ceiling. Their
    // unrelated definitions must not impose a new 10,000-node delivery cutoff.
    sqlx::query("WITH items AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,$3,1,1,game_character_uuid_v7() FROM generate_series(1,10001) RETURNING item_instance_id) INSERT INTO game_item_character_depot_locations(item_instance_id,world_id,character_id,town_id,box_number,ordinal) SELECT item_instance_id,encode($1,'hex')::uuid,encode($4,'hex')::uuid,2,1,row_number() OVER(ORDER BY item_instance_id) FROM items")
        .bind(id(42).as_slice()).bind(&definition.production_key).bind(&definition.revision_ref).bind(id(41).as_slice()).execute(&mut *tx).await?;
    let mut parent:String=sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_character_depot_locations WHERE character_id=encode($1,'hex')::uuid AND town_id=2 ORDER BY ordinal LIMIT 1")
        .bind(id(41).as_slice()).fetch_one(&mut *tx).await?;
    // Current depot-search traverses sixteen levels. The leaf at depth nine
    // proves delivery follows that owner instead of the old carried depth eight.
    for _ in 1..9 {
        let child:String=sqlx::query_scalar("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,$3,1,1,game_character_uuid_v7()) RETURNING item_instance_id::text")
            .bind(id(42).as_slice()).bind(&definition.production_key).bind(&definition.revision_ref).fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,$3::uuid,1,game_character_uuid_v7())")
            .bind(&child).bind(id(42).as_slice()).bind(&parent).execute(&mut *tx).await?;
        parent = child;
    }
    let (original,quantity):(String,i64)=sqlx::query_as("SELECT i.item_instance_id::text,i.quantity FROM game_item_instances i JOIN game_item_character_depot_locations d USING(item_instance_id) WHERE d.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$2 ORDER BY i.item_instance_id LIMIT 1")
        .bind(id(41).as_slice()).bind(&item.key).fetch_one(&mut *tx).await?;
    let nested = quantity / 2;
    assert!(nested > 0);
    sqlx::query(
        "UPDATE game_item_instances SET quantity=quantity-$2 WHERE item_instance_id=$1::uuid",
    )
    .bind(&original)
    .bind(nested)
    .execute(&mut *tx)
    .await?;
    let leaf:String=sqlx::query_scalar("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,$3,$4,1,game_character_uuid_v7()) RETURNING item_instance_id::text")
        .bind(id(42).as_slice()).bind(&item.key).bind(&item.revision).bind(nested).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,$3::uuid,1,game_character_uuid_v7())")
        .bind(&leaf).bind(id(42).as_slice()).bind(&parent).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_depot_box_counters(character_id,town_id,box_number,next_ordinal,committed) SELECT character_id,town_id,box_number,max(ordinal)+1,count(*) FROM game_item_character_depot_locations WHERE character_id=encode($1,'hex')::uuid AND town_id=2 GROUP BY character_id,town_id,box_number")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

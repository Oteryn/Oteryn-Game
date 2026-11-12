//! Independently seeded historical custody; these fixtures do not call the exporter.
use super::{
    TestResult,
    fixture::{Harness, debug, id},
};
use crate::native_admission_source::journal::{
    ItemRow, ItemsPage, ItemsPayload, PublishGameJournalRecordV1,
};
use sha2::{Digest, Sha256};
use sqlx::Row;
pub(super) async fn seed(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    // Historical fixture restoration only, matching the repository's custody
    // fixtures. Production writes and export reads retain all their real guards.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (n, quantity) in [
        (180, 1),
        (181, 1),
        (182, 1),
        (183, 11),
        (184, 23),
        (185, 41),
    ] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i1000','definition-r1',$3,1,encode($1,'hex')::uuid)").bind(id(n).as_slice()).bind(id(42).as_slice()).bind(quantity as i64).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING").bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,3,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($2,'hex')::uuid)").bind(id(41).as_slice()).bind(id(180).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($2,'hex')::uuid)").bind(id(41).as_slice()).bind(id(181).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,4,encode($1,'hex')::uuid)").bind(id(182).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(181).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,6,encode($1,'hex')::uuid)").bind(id(183).as_slice()).bind(id(42).as_slice()).bind(id(182).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_character_depot_locations(item_instance_id,world_id,character_id,town_id,box_number,ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,16,2,8)").bind(id(184).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_character_inbox_locations(item_instance_id,world_id,character_id,ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,10)").bind(id(185).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
fn uuid(n: u8) -> TestResult<String> {
    Ok(crate::foundation::CharacterId::decode(&id(n))
        .map_err(debug)?
        .as_bytes()
        .iter()
        .enumerate()
        .fold(String::new(), |mut s, (i, b)| {
            if [4, 6, 8, 10].contains(&i) {
                s.push('-');
            }
            s.push_str(&format!("{b:02x}"));
            s
        }))
}
pub(super) fn assert_export(records: &[Vec<u8>]) -> TestResult {
    assert_eq!(records.len(), 3);
    let begin = PublishGameJournalRecordV1::decode(&records[0])?;
    let ItemsPayload::Begin(begin) = begin.payload else {
        return Err("begin".into());
    };
    assert_eq!((begin.page_count, begin.item_count), (1, 6));
    assert_eq!(begin.depots, vec![16]);
    let mut expected = Vec::new();
    for (n, quantity, root, slot, parent, ordinal) in [
        (180, 1, "EQUIPMENT", Some(3), None, "1"),
        (181, 1, "EQUIPMENT", Some(9), None, "1"),
        (182, 1, "EQUIPMENT", Some(9), Some(181), "4"),
        (183, 11, "EQUIPMENT", Some(9), Some(182), "6"),
        (184, 23, "DEPOT", None, None, "8"),
        (185, 41, "INBOX", None, None, "10"),
    ] {
        expected.push(ItemRow {
            item_instance_id: uuid(n)?,
            definition_family: "Item".into(),
            definition_production_key: "oteryn:item.tibia.i1000".into(),
            definition_revision_ref: "definition-r1".into(),
            quantity,
            root: root.into(),
            town_id: (root == "DEPOT").then_some(16),
            depot_box: (root == "DEPOT").then_some(2),
            house_id: None,
            spatial_position_hex: None,
            equipment_slot: slot,
            parent_item_instance_id: parent.map(uuid).transpose()?,
            placement_ordinal: ordinal.into(),
        });
    }
    expected.sort_by(|a, b| a.item_instance_id.cmp(&b.item_instance_id));
    let expected = ItemsPage {
        snapshot_id: begin.snapshot_id,
        page_index: 0,
        items: expected,
    };
    let page = PublishGameJournalRecordV1::decode(&records[1])?;
    assert_eq!(page.payload, ItemsPayload::Page(expected.clone()));
    let digest = Sha256::digest(serde_json::to_vec(&expected)?)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let end = PublishGameJournalRecordV1::decode(&records[2])?;
    let ItemsPayload::End(end) = end.payload else {
        return Err("end".into());
    };
    assert_eq!(end.pages_sha256, digest);
    assert_eq!((end.page_count, end.item_count), (1, 6));
    Ok(())
}
pub(super) async fn runtime_role(root: &crate::durability::DurabilityRoot) -> TestResult {
    let mut connection = root.try_acquire_ready()?;
    sqlx::query("SET ROLE oteryn_game_runtime")
        .execute(&mut *connection)
        .await?;
    let role: String = sqlx::query_scalar("SELECT current_user")
        .fetch_one(&mut *connection)
        .await?;
    assert_eq!(role, "oteryn_game_runtime");
    let returned = connection
        .return_to_pool_observed_until(
            std::time::Instant::now() + std::time::Duration::from_secs(5),
        )
        .await;
    assert!(matches!(
        returned,
        sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
    ));
    Ok(())
}
pub(super) fn world(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_items(
        "oteryn:item.tibia.i1629",
        "oteryn:item.tibia.i1000",
    );
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let tile = bundle
        .sectors
        .first_mut()
        .ok_or("sector")?
        .tiles
        .iter_mut()
        .find(|t| t.x == 7)
        .ok_or("depot tile")?;
    tile.items.last_mut().ok_or("depot placement")?.attrs.depot = Some(16);
    let bytes =
        oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns)
            .map_err(debug)?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 2,
                y: 0,
                floor: -7,
            },
        },
        world,
        channel,
        crate::map::boot::tests::door_items,
    )
    .map_err(debug)
}
pub(super) async fn assert_runtime_helpers(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let broad:bool=sqlx::query_scalar("SELECT has_table_privilege(current_user,'game_item_house_reclaim_provenance','SELECT') OR has_table_privilege(current_user,'game_character_account_projection_epoch','UPDATE') OR has_table_privilege(current_user,'game_character_account_projections','UPDATE')").fetch_one(&mut *tx).await?;
    assert!(!broad, "helpers must not broaden runtime table privileges");
    let r=sqlx::query("SELECT current_user::text AS role,game_journal_projection_epoch() AS epoch,game_journal_account_projection(encode($1,'hex')::uuid) AS projection,game_journal_unknown_house(encode($2,'hex')::uuid) AS house")
        .bind(id(40).as_slice()).bind(id(41).as_slice()).fetch_one(&mut *tx).await?;
    assert_eq!(r.try_get::<String, _>("role")?, "oteryn_game_runtime");
    assert!(!r.try_get::<bool, _>("house")?);
    assert!(r.try_get::<Option<String>, _>("epoch")?.is_some());
    assert!(r.try_get::<Option<String>, _>("projection")?.is_some());
    tx.commit().await?;
    Ok(())
}

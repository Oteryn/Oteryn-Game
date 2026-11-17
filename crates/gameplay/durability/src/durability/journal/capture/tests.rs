use super::*;
#[test]
fn journal_snapshot_pages_hash_exact_golden_payload()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let r=PublishGameJournalRecordV1::decode(br#"{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":"019a0000-0000-7000-8000-000000000003","character_revision":"1","from_projection_revision":"1","to_projection_revision":"1","projection_epoch":"1","family":"ITEMS","recovery_generation":"1","source_revision":"1","record_id":"019a0000-0000-7000-8000-000000000004","occurred_at":"1793836800","coverage_start_at":"1793836800","kind":"ITEMS_BEGIN","payload":{"snapshot_id":"019a0000-0000-7000-8000-000000000005","character_revision":"1","page_count":0,"item_count":0,"depots":[],"houses":[]}}"#)?;
    let stream = ReadGameJournalCursorV1 {
        contract_version: 1,
        operation: "ReadGameJournalCursorV1".into(),
        source_authority: r.source_authority,
        account_id: r.account_id,
        world_id: r.world_id,
        character_id: r.character_id,
        character_revision: r.character_revision,
        from_projection_revision: r.from_projection_revision,
        to_projection_revision: r.to_projection_revision,
        projection_epoch: r.projection_epoch,
        family: r.family,
        recovery_generation: r.recovery_generation,
    };
    let golden=br#"{"snapshot_id":"019a0000-0000-7000-8000-000000000005","page_index":0,"items":[{"item_instance_id":"019a0000-0000-7000-8000-00000000000b","definition_family":"ITEM","definition_production_key":"oteryn:content.item.backpack","definition_revision_ref":"rev-1","quantity":1,"root":"EQUIPMENT","town_id":null,"depot_box":null,"house_id":null,"spatial_position_hex":null,"equipment_slot":3,"parent_item_instance_id":null,"placement_ordinal":"1"}]}"#;
    let page: ItemsPage = serde_json::from_slice(golden)?;
    let result = payloads(
        stream.character_revision.as_deref().ok_or("cut")?,
        &page.snapshot_id,
        page.items.clone(),
        &[],
    )?
    .ok_or("representable page")?;
    assert_eq!(result.len(), 3);
    let ItemsPayload::Page(actual) = &result[1] else {
        return Err("page family".into());
    };
    assert_eq!(serde_json::to_vec(actual)?, golden);
    let ItemsPayload::End(end) = &result[2] else {
        return Err("end family".into());
    };
    assert_eq!(
        end.pages_sha256,
        "3e1f2270e29acdbd0a78d5a9d46a96e18cf5f0c43b21e0b57d47201809bd6954"
    );
    assert_eq!((end.page_count, end.item_count), (1, 1));
    let mut items = page.items;
    items.first_mut().ok_or("row")?.definition_production_key = "x".repeat(65537);
    assert!(
        payloads(
            stream.character_revision.as_deref().ok_or("cut")?,
            &page.snapshot_id,
            items,
            &[]
        )?
        .is_none()
    );
    Ok(())
}

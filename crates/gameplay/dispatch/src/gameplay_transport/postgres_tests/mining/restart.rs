use super::*;
use sha2::{Digest, Sha256};

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
        .return_to_pool_observed_until(std::time::Instant::now() + Duration::from_secs(5))
        .await;
    assert!(matches!(
        returned,
        sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
    ));
    Ok(())
}

pub(super) async fn reload(
    harness: &Harness,
    session: GameSessionId,
    intent: &UseIntent,
) -> TestResult {
    let root = crate::durability::DurabilityRoot::connect_test_runtime(&harness.database.url)?;
    assert!(root.maintain_ready_once().await?);
    runtime_role(&root).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = root.open_character_authority(&seal).await.map_err(debug)?;
    let binding = Sha256::digest(
        oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).map_err(debug)?,
    )
    .into();
    let command = crate::foundation::CommandRef::new(
        session,
        crate::foundation::CommandId::new(5).map_err(debug)?,
    );
    assert!(
        root.read_pick_refining_receipt(&authority, item_fence()?, command, binding)
            .await
            .map_err(debug)?
            .is_some()
    );
    let inventory = root
        .read_item_use_inventory(&authority, &harness.node, item_fence()?)
        .await
        .map_err(debug)?;
    let bag = inventory.backpack.expect("persisted backpack");
    assert_eq!(bag.entries.len(), 2);
    let output = bag
        .entries
        .iter()
        .find(|e| e.item.item_instance_id != id(80))
        .expect("persisted output");
    assert_eq!(output.item.quantity, 1);
    assert!(
        [3028, 3031, 3035, 3043]
            .iter()
            .any(|id| output.item.definition.production_key == format!("oteryn:item.tibia.i{id}"))
    );
    for seed in [100, 110] {
        let entry = inventory
            .nested
            .iter()
            .find(|(_, e)| e.item.item_instance_id == id(seed))
            .expect("persisted pick/stone");
        assert_eq!(entry.1.item.quantity, 1);
    }
    let wire: Vec<u8> = sqlx::query_scalar("SELECT envelope FROM game_item_audit_outbox WHERE transaction_id=(SELECT transaction_id FROM game_pick_refinements)").fetch_one(&harness.pool).await?;
    let (_, audit) =
        crate::durability::item_use_audit::decode_item_use_envelope(&wire).map_err(debug)?;
    assert_eq!(audit.cause.expect("cause").kind, 3);
    assert_eq!(audit.source_before.expect("stone before").quantity, 2);
    assert_eq!(audit.source_after.expect("stone after").quantity, 1);
    assert_eq!(
        audit
            .flask
            .expect("required output")
            .after
            .expect("output after")
            .quantity,
        1
    );
    Ok(())
}

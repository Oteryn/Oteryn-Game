//! M49: an absent caster is not evidence that a durable field is environmental.
use crate::bestiary_postgres_harness as f;
use crate::durability::native_map_items_abi::{
    NativeMapInitializationProof, NativeMapItemPlacement, NativeMapOwnerBinding,
    map_initializer_seal,
};
use crate::durability::spell_owner_commit::SpellLanePermit;

// Persistence fixture only: this supplies a map placement to the real native
// initializer, not a production map qualification or a new current authority.
struct MapFixture {
    binding: NativeMapOwnerBinding,
    placements: Vec<NativeMapItemPlacement>,
}
impl map_initializer_seal::Sealed for MapFixture {}
impl NativeMapInitializationProof for MapFixture {
    fn current_binding(&self) -> Result<NativeMapOwnerBinding, &'static str> {
        Ok(self.binding.clone())
    }
    fn placements(&self) -> &[NativeMapItemPlacement] {
        &self.placements
    }
}

#[test]
fn qn1_environment_field_read_requires_positive_native_map_receipt() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "qn1_environment_field", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "QN1_ENVIRONMENT_FIELD_PG_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "environment field PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    use crate::durability::item_mint::{GroundPlacement, TypedDefinitionRef};
    use crate::durability::native_world_items::SpellGroundTarget;
    use crate::durability::spell_item_transaction::{
        assert_spell_item_scope_in_transaction, read_qualification_scope_tile_in_transaction,
    };
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QN1_ENVIRONMENT_FIELD_PG_READY PostgreSQL {version}");
    let scope = f::scope()?;
    let binding = NativeMapOwnerBinding {
        world: scope.world_id(),
        channel: scope.channel_id().ok_or("Channel fixture")?,
        scope_generation: 1,
        content_digest: [1; 32],
        map_digest: [7; 32],
        frame_digest: [3; 32],
    };
    let ground = GroundPlacement {
        spatial_position: vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 13],
        corpse_ref: b"native-source-map-init-r21".to_vec(),
        map_revision: format!("sha256:{}", "07".repeat(32)),
        content_revision: format!("sha256:{}", "01".repeat(32)),
        native_room_placement_context: vec![3; 32],
    };
    let proof = MapFixture {
        binding,
        placements: vec![NativeMapItemPlacement {
            placement_key: "fixture:energy-field".into(),
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2126".into(),
                revision_ref: "definition-r1".into(),
            },
            quantity: 1,
            ground: ground.clone(),
            source_binding: br#"{"fixture":"native-energy-field"}"#.to_vec(),
            source_attributes: serde_json::json!({}),
            blocks_movement: false,
            blocks_projectile: false,
            immovable_block_solid: false,
            owner_kind: None,
        }],
    };
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let permit = SpellLanePermit::of_fresh_scope(scope)
        .await
        .map_err(f::debug)?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let minted = crate::durability::native_map_items::initialize_in_transaction(
        &mut tx, &permit, &h.root, &recovery, &h.node, &proof,
    )
    .await
    .map_err(f::debug)?;
    assert_eq!(minted.len(), 1);
    tx.commit().await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let authority = assert_spell_item_scope_in_transaction(
        &mut tx, &permit, &h.root, &recovery, &h.node, scope, 1,
    )
    .await
    .map_err(f::debug)?;
    let target = SpellGroundTarget {
        item_profile_source: None,
        spatial_position: ground.spatial_position,
        map_revision: ground.map_revision,
        content_revision: ground.content_revision,
        placement_context: ground.native_room_placement_context,
    };
    let read = read_qualification_scope_tile_in_transaction(&mut tx, &authority, &target)
        .await
        .map_err(f::debug)?;
    assert_eq!(read.items.len(), 1);
    let item = &read.items[0];
    assert_eq!(item.item_instance_id, minted[0].item_instance_id);
    assert!(!item.environment_field_origin);
    assert!(item.field_origin.is_none());
    assert!(item.creature_field_origin.is_none());
    assert!(
        item.native_map_origin,
        "the actual native receipt must survive tile decoding"
    );
    let incompatible = SpellGroundTarget {
        map_revision: format!("sha256:{}", "08".repeat(32)),
        ..target
    };
    assert!(
        read_qualification_scope_tile_in_transaction(&mut tx, &authority, &incompatible)
            .await
            .is_err()
    );
    tx.rollback().await?;
    Ok(())
}

//! WCONTRIB dirty water uses the native fishing wire and craft receipt owner.
use super::TestResult;

#[test]
fn wcontrib_fishing_shimmer_catch_custody_replay_and_personal_cooldown() -> TestResult {
    super::standard_fishing::run_gathering(3483, 12561, Some(12558), Some(12557), true)
}

#[test]
fn wcontrib_fishing_dirty_water_can_consume_bait_without_yield() -> TestResult {
    super::standard_fishing::run_gathering(3483, 12562, Some(12559), None, true)
}

#[test]
fn wcontrib_fishing_spent_dirty_water_never_falls_back_to_ordinary_fish() -> TestResult {
    for target in 12558..=12560 {
        super::standard_fishing::run_gathering(3483, target, Some(target), None, false)?;
    }
    Ok(())
}

pub(super) async fn assert_personal_cooldown(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    pool: &sqlx::PgPool,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    native: &crate::content::native_gameplay::NativeGameplayState,
    placement: &[u8],
    map_revision: String,
) -> TestResult {
    use crate::durability::item_craft::{GatheringCraft, ItemCraftRequest};
    use crate::durability::terrain_tool::{TerrainMutation, TerrainScope};
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_items::fishing::GatheringAttempt;
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
        encode_item_use_intent,
    };

    let last: i64 = sqlx::query_scalar("SELECT last_catch_at FROM game_twisted_water_catches")
        .fetch_one(pool)
        .await?;
    let reset =
        sqlx::query("UPDATE game_twisted_water_catches SET last_catch_at=last_catch_at-72000000")
            .execute(pool)
            .await;
    assert!(reset.as_ref().is_err_and(|error| {
        error
            .as_database_error()
            .is_some_and(|database| database.code().as_deref() == Some("23514"))
    }));
    let source: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3492' AND lifecycle=1")
        .fetch_one(pool).await?;
    let tool: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3483' AND lifecycle=1")
        .fetch_one(pool).await?;
    let source: [u8; 16] = source.try_into().map_err(|_| "source id")?;
    let tool: [u8; 16] = tool.try_into().map_err(|_| "tool id")?;
    for command in 2..=3 {
        if command == 3 {
            // Advance the persisted fixture deadline by exactly the sourced20h;
            // no sleep and no wall-clock manipulation hides a missing cooldown.
            let mut fixture_tx = pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *fixture_tx)
                .await?;
            sqlx::query(
                "UPDATE game_twisted_water_catches SET last_catch_at=last_catch_at-72000000",
            )
            .execute(&mut *fixture_tx)
            .await?;
            fixture_tx.commit().await?;
        }
        sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=0")
            .execute(pool)
            .await?;
        owner.refresh_terrain_tools().await;
        let revision: i64 =
            sqlx::query_scalar("SELECT object_revision FROM game_terrain_tool_transforms")
                .fetch_one(pool)
                .await?;
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("tool handle")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement: placement.to_vec(),
                expected_revision: u64::try_from(revision)?,
            })),
            use_with_position: None,
        };
        let request = ItemCraftRequest {
            command: CommandRef::new(
                fence.game_session_id,
                CommandId::new(command).map_err(super::fixture::debug)?,
            ),
            source,
            tool: Some(tool),
            ingredient: None,
            station: None,
            intent: encode_item_use_intent(&intent).map_err(super::fixture::debug)?,
            gathering: Some(GatheringCraft {
                scope: TerrainScope {
                    map_revision: map_revision.clone(),
                    content_generation: native.source_digest(),
                    reset_epoch: 0,
                },
                mutation: TerrainMutation {
                    tool,
                    tool_key: "oteryn:item.tibia.i3483".into(),
                    tool_revision: "definition-r1".into(),
                    placement: placement.to_vec(),
                    expected_revision: revision,
                    before: 12561,
                    after: 12558,
                    decay: vec![(12561, 1_200_000)],
                },
                attempt: GatheringAttempt {
                    after: Some(12558),
                    yield_item: Some(12557),
                    consumes_bait: true,
                },
            }),
            enchanting: None,
        };
        let permit = owner.spell_lane_permit().await.ok_or("craft permit")?;
        let content = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
            .with_wheel(owner.premium_coordinator);
        assert_eq!(
            owner
                .root
                .craft_item(
                    &permit,
                    owner.character,
                    owner.holder,
                    fence,
                    request.clone(),
                    native,
                    &content,
                    false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        assert_eq!(
            owner
                .root
                .craft_item(
                    &permit,
                    owner.character,
                    owner.holder,
                    fence,
                    request,
                    native,
                    &content,
                    false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        let shimmer: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i12557' AND lifecycle=1")
            .fetch_one(pool).await?;
        assert_eq!(shimmer, if command == 2 { 1 } else { 2 });
        let bones: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3111' AND lifecycle=1")
            .fetch_one(pool).await?;
        assert_eq!(bones, 1);
        let after: i64 = sqlx::query_scalar("SELECT last_catch_at FROM game_twisted_water_catches")
            .fetch_one(pool)
            .await?;
        if command == 2 {
            assert_eq!(after, last);
        } else {
            assert!(after >= last);
        }
    }
    eprintln!(
        "WCONTRIB_FISHING_COOLDOWN_PG_EXECUTED restricted_catch=fishbones elapsed20h=shimmer replay_never_recatches=true"
    );
    Ok(())
}

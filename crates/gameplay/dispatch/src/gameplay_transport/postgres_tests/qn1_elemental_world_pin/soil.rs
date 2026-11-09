//! Real carried soil USE consumes one unit with its terrain CAS and deadline.
use super::super::fixture::{Harness, debug};
use super::super::{ComposedFreshAdmission, TestResult};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{connection::UseCommand, item_view::ItemKey};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
    fence: CurrentCharacterItemFence,
) -> TestResult {
    eprintln!("QN1_ELEMENTAL_SOIL_PG_READY");
    super::super::quest_world_callbacks::input_item(
        owner,
        fence,
        &Offering {
            position: Position {
                x: 33198,
                y: 32103,
                floor: -13,
            },
            item: "oteryn:item.tibia.i941".into(),
            count: 2,
        },
        88,
        true,
    )
    .await?;
    let source: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i941' AND lifecycle=1 LIMIT 1").fetch_one(&h.pool).await?;
    let source: [u8; 16] = source.try_into().map_err(|_| "soil identity")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("index")?;
    let reference = index
        .definition_ref("Item", "oteryn:item.tibia.i941", "definition-r1")
        .ok_or("soil reference")?;
    let map = owner.bundle_map().ok_or("map")?;
    let pos = crate::map::overlay::TilePos {
        x: 33198,
        y: 32102,
        floor: -13,
    };
    let key = crate::map::view::placement_key(pos, 1).ok_or("hole key")?;
    let intent = |key, revision| {
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&u64::to_be_bytes(key));
        UseIntent {
            target: UseTarget::ItemByDefinition(reference),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: revision,
            })),
            use_with_position: None,
        }
    };
    let use_soil = |id, intent| async move {
        owner
            .use_cooking_conversion(
                actor,
                UseCommand {
                    game_session_id: fence.game_session_id,
                    command_id: id,
                    item_fence: Some(fence),
                },
                intent,
                Some(ItemKey::Instance(source)),
                None,
            )
            .await
            .ok_or("soil route")
    };
    let wrong = intent(
        crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: 33199,
                y: 32103,
                floor: -13,
            },
            0,
        )
        .ok_or("altar key")?,
        0,
    );
    assert_ne!(
        use_soil(65000, wrong).await?.disposition,
        UseDisposition::Committed
    );
    let valid = intent(key, 0);
    assert_eq!(
        use_soil(65001, valid.clone()).await?.disposition,
        UseDisposition::Committed,
        "soil USE must charge the source hole through the native item owner"
    );
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(source.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(quantity, 1);
    // The new zero-output debit requires this transaction's matching terrain CAS.
    let mut forged = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *forged)
        .await?;
    let denied = sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,consume_source) SELECT game_character_uuid_v7(),game_character_uuid_v7(),game_session_id,65003,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,consume_source FROM game_item_craft_records WHERE source_item_instance_id=encode($1,'hex')::uuid AND command_id=65001")
        .bind(source.as_slice()).execute(&mut *forged).await;
    let Err(sqlx::Error::Database(error)) = denied else {
        return Err("soil debit without a fresh terrain receipt was not rejected".into());
    };
    assert_eq!(error.code().as_deref(), Some("23514"));
    assert!(
        error
            .message()
            .contains("elemental soil requires a fresh matching terrain receipt")
    );
    forged.rollback().await.map_err(debug)?;
    let state: (i64,Vec<i64>,Vec<i64>,Option<i64>) = sqlx::query_as("SELECT state_item_id,decay_states,decay_delays,decay_at_unix_ms FROM game_terrain_tool_transforms LIMIT 1").fetch_one(&h.pool).await?;
    assert_eq!(
        (state.0, state.1, state.2),
        (7745, vec![7743], vec![120000])
    );
    assert!(state.3.is_some());
    assert_eq!(map.facts.object_revision(key), 1);
    assert_eq!(
        use_soil(65001, valid.clone()).await?.disposition,
        UseDisposition::Committed
    );
    assert_eq!(
        use_soil(65001, intent(key, 1)).await?.disposition,
        UseDisposition::Rejected
    );
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(source.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(quantity, 1, "replay cannot debit the soil twice");
    sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=0")
        .execute(&h.pool)
        .await?;
    owner.refresh_terrain_tools().await;
    let state: i64 =
        sqlx::query_scalar("SELECT state_item_id FROM game_terrain_tool_transforms LIMIT 1")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(state, 7743);
    assert_eq!(map.facts.object_revision(key), 2);
    assert_eq!(
        use_soil(65002, intent(key, 2)).await?.disposition,
        UseDisposition::Committed
    );
    let state: (i64,i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(state, (0, 2));
    assert_eq!(
        use_soil(65002, intent(key, 2)).await?.disposition,
        UseDisposition::Committed
    );
    eprintln!("QN1_ELEMENTAL_SOIL_PG_EXECUTED");
    Ok(())
}

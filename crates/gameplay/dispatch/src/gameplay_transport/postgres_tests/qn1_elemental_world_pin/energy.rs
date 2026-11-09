//! Crystal soils1.lua: soil 945 adds a field above the unchanged 7737 altar.
use super::super::fixture::Harness;
use super::super::{ComposedFreshAdmission, TestResult};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{connection::UseCommand, item_view::ItemKey};
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
    eprintln!("QN1_ELEMENTAL_ENERGY_SOIL_PG_READY");
    super::super::quest_world_callbacks::input_item(
        owner,
        fence,
        &Offering {
            position: Position {
                x: 33093,
                y: 32189,
                floor: -13,
            },
            item: "oteryn:item.tibia.i945".into(),
            count: 2,
        },
        88,
        true,
    )
    .await?;
    let source: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i945' AND lifecycle=1 LIMIT 1").fetch_one(&h.pool).await?;
    let source: [u8; 16] = source.try_into().map_err(|_| "energy soil identity")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("index")?;
    let reference = index
        .definition_ref("Item", "oteryn:item.tibia.i945", "definition-r1")
        .ok_or("energy soil reference")?;
    let map = owner.bundle_map().ok_or("energy map")?;
    let pos = crate::map::overlay::TilePos {
        x: 33094,
        y: 32189,
        floor: -13,
    };
    let altar = crate::map::view::compose_tile(&map.overlay, &map.facts, pos)
        .map_err(super::super::fixture::debug)?
        .ok_or("native altar tile")?
        .entries
        .into_iter()
        .find(|entry| {
            let oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference) =
                entry.facts.definition
            else {
                return false;
            };
            crate::gameplay_transport::item_look::typed_definition(index, reference)
                .is_some_and(|definition| definition.production_key == "oteryn:item.tibia.i7737")
        })
        .ok_or("retained native energy altar")?;
    let crate::map::view::MapEntryKey::Base(key) = altar.source.key() else {
        return Err("native base altar".into());
    };
    let original_definition = altar.facts.definition;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let intent = UseIntent {
        target: UseTarget::ItemByDefinition(reference),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    };
    let use_soil = || async {
        owner
            .use_cooking_conversion(
                actor,
                UseCommand {
                    game_session_id: fence.game_session_id,
                    command_id: 65001,
                    item_fence: Some(fence),
                },
                intent.clone(),
                Some(ItemKey::Instance(source)),
                None,
            )
            .await
            .ok_or("energy soil route")
    };
    assert_eq!(
        use_soil().await?.disposition,
        UseDisposition::Committed,
        "energy soil must create its field through the durable native USE owner"
    );
    assert_eq!(use_soil().await?.disposition, UseDisposition::Committed);
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(source.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(quantity, 1, "replay never consumes a second soil");
    let unchanged = crate::map::view::compose_tile(&map.overlay, &map.facts, pos)
        .map_err(super::super::fixture::debug)?
        .ok_or("unchanged altar tile")?
        .entries
        .into_iter()
        .find(|entry| entry.source.key() == crate::map::view::MapEntryKey::Base(key))
        .ok_or("unchanged altar")?;
    assert_eq!(unchanged.facts.definition, original_definition);
    let fields:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i2126'")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        fields, 1,
        "one real persistent field, including after exact replay"
    );
    let ground = owner
        .root
        .read_item_loop_ground(owner.world_id, owner.channel_id)
        .await
        .map_err(super::super::fixture::debug)?;
    let field = ground
        .iter()
        .find(|entry| entry.item.definition.production_key == "oteryn:item.tibia.i2126")
        .ok_or("the actual energy field must enter the altar Ground readset")?;
    assert_eq!(field.field_collision, Some((false, false)));
    assert!(!field.native_corpse);
    let projection =
        crate::gameplay_transport::loop_move::loop_ground(owner, map.overlay.base(), Some(index))
            .await
            .ok_or("current energy field projection")?;
    assert_eq!(
        projection
            .ground_entry(&field.item.item_instance_id)
            .ok_or("projected field")?
            .0,
        pos
    );
    let permit = owner.spell_lane_permit().await.ok_or("field read lane")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let authority =
        crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
            &mut tx,
            &permit,
            owner.root,
            owner.character,
            owner.holder,
            fence.runtime_scope,
            fence.scope_ownership_generation.get(),
        )
        .await
        .map_err(super::super::fixture::debug)?;
    let read =
        crate::durability::spell_item_transaction::read_qualification_scope_tile_in_transaction(
            &mut tx,
            &authority,
            &crate::durability::native_world_items::SpellGroundTarget {
                spatial_position: field.item.ground.spatial_position.clone(),
                map_revision: field.item.ground.map_revision.clone(),
                content_revision: field.item.ground.content_revision.clone(),
                placement_context: field.item.ground.native_room_placement_context.clone(),
            },
        )
        .await
        .map_err(super::super::fixture::debug)?;
    let visible = read
        .items
        .iter()
        .find(|entry| entry.item_instance_id == field.item.item_instance_id)
        .ok_or("energy field in the exact current tile read")?;
    assert!(!visible.native_map_origin);
    assert!(visible.environment_field_origin);
    assert!(visible.field_origin.is_none());
    assert!(visible.creature_field_origin.is_none());
    assert!(!visible.blocks_movement);
    assert!(!visible.blocks_projectile);
    eprintln!("QN1_ELEMENTAL_ENERGY_FIELD_COUNT_PG_READY");
    let active: i64 = sqlx::query_scalar("SELECT game_live_channel_field_count(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::bytea[])")
        .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice())
        .bind(Vec::<Vec<u8>>::new()).fetch_one(&mut *tx).await?;
    assert_eq!(
        active, 1,
        "the shared field budget must count the actual scripted field"
    );
    let removed: i64 = sqlx::query_scalar("SELECT game_live_channel_field_count(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::bytea[])")
        .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice())
        .bind(vec![field.item.item_instance_id.to_vec()]).fetch_one(&mut *tx).await?;
    assert_eq!(
        removed, 0,
        "a planned removal uses the same actual field identity"
    );
    eprintln!("QN1_ELEMENTAL_ENERGY_FIELD_COUNT_PG_EXECUTED");
    tx.rollback().await?;
    drop(permit);
    super::handoff::exercise(owner, h, field.item.item_instance_id).await?;
    eprintln!("QN1_ELEMENTAL_ENERGY_SOIL_PG_EXECUTED");
    Ok(())
}

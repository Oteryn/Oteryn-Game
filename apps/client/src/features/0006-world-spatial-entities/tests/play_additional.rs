use super::*;

#[test]
fn skinning_decoded_stationary_corpse_updates_native_look_target()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{
        EntityDetail, EntityKind, EntityRef, WorldSpatialEntitiesDelta,
        decode_world_spatial_entities_delta_with_item_handles,
        encode_world_spatial_entities_delta_with_item_handles,
    };
    let reference = EntityRef {
        identity: [9; 16],
        // Existing object codec uses generation zero; only living actors have generations.
        generation: 0,
    };
    let position = ActorPosition {
        x: 101,
        y: 200,
        floor: 0,
    };
    let fresh = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Corpse,
        entity: reference,
        position,
        detail: EntityDetail::Object {
            item_definition_ref: 42,
            quantity: 1,
            item_handle: Some(std::num::NonZeroU64::MIN),
        },
    };
    let own = ActorPosition {
        x: 100,
        y: 200,
        floor: 0,
    };
    let fresh_delta = WorldSpatialEntitiesDelta {
        content_generation: [0; 32],
        actor_position: own,
        enter: vec![fresh],
        update: Vec::new(),
        leave: Vec::new(),
    };
    let fresh_bytes = encode_world_spatial_entities_delta_with_item_handles(&fresh_delta)
        .map_err(|e| format!("{e:?}"))?;
    let decoded = decode_world_spatial_entities_delta_with_item_handles(&fresh_bytes)
        .map_err(|e| format!("{e:?}"))?;
    let (link, mut commands, outbox) = play_channel();
    outbox.world_context_initialize(true, false);
    // The production session stepper publishes its decoded current entity store to PlayLink.
    outbox.entities(decoded.enter);
    let mut play = view()?;
    play.tick(&link).map_err(|e| format!("{e:?}"))?;
    let tile = TileCoord::new(position.x, position.y);
    play.click(tile)?;
    let current = play
        .scene()
        .target()
        .and_then(|target| target.item)
        .ok_or("fresh native corpse target")?;
    assert_eq!(current.definition(), 42);
    assert_eq!(current.quantity(), 1);
    let sprite_count = play.scene().sprites().len();
    let mut spent = fresh;
    spent.detail = EntityDetail::Object {
        item_definition_ref: 43,
        quantity: 1,
        item_handle: Some(std::num::NonZeroU64::MIN),
    };
    let delta = WorldSpatialEntitiesDelta {
        content_generation: [0; 32],
        actor_position: own,
        enter: Vec::new(),
        update: vec![spent],
        leave: Vec::new(),
    };
    let bytes = encode_world_spatial_entities_delta_with_item_handles(&delta)
        .map_err(|e| format!("{e:?}"))?;
    let decoded = decode_world_spatial_entities_delta_with_item_handles(&bytes)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(decoded.update[0].entity, reference);
    assert_eq!(decoded.update[0].position, position);
    outbox.entities(decoded.update);
    play.tick(&link).map_err(|e| format!("{e:?}"))?;
    play.click(tile)?;
    let current = play
        .scene()
        .target()
        .and_then(|target| target.item)
        .ok_or("spent native corpse target")?;
    assert_eq!(current.definition(), 43);
    assert!(
        matches!(current, GroundItem::Entity { reference: actual, handle: Some(handle), .. }
        if actual == reference && handle == std::num::NonZeroU64::MIN)
    );
    assert_eq!(play.scene().sprites().len(), sprite_count);
    let menu = play
        .world_context(tile, &link)
        .ok_or("native corpse Look menu")?;
    assert!(play.world_context_choose(
        &link,
        &menu,
        oteryn_client_runtime::world_interaction::Action::Look
    ));
    let command = commands.try_recv()?;
    assert!(
        matches!(command, crate::combat_input::PlayCommand::WorldContext(
        oteryn_client_runtime::world_interaction::Command {
            target: oteryn_client_runtime::world_interaction::Target::Item(handle),
            action: oteryn_client_runtime::world_interaction::Action::Look, ..
        }) if handle == std::num::NonZeroU64::MIN)
    );
    Ok(())
}

use super::*;

#[test]
fn srx3_source_rune_position_boundary_maps_target_and_keeps_original_use() -> Result<(), String> {
    let scene = crate::gameplay_transport::wire_position::test_scene::source_scene()?;
    assert!(scene.room.pinned_activity_source_room().is_some());
    let spell = scene.book.rune(3191).ok_or("actual native area rune")?;
    assert!(
        !spell.needs_target,
        "fixture uses the existing native area rune"
    );
    let original = UseIntent {
        target: UseTarget::Item(
            oteryn_protocol_oteryn::item_view::ItemHandle::new(1).ok_or("handle")?,
        ),
        use_with: None,
        use_with_position: Some(SpellTargetPosition {
            x: scene.wire.x,
            y: scene.wire.y,
            floor: scene.wire.floor,
        }),
        use_with_map_item: None,
    };
    let bytes = encode_item_use_intent(&original).map_err(|e| format!("{e:?}"))?;
    let (mapped, legacy) = target(
        &scene.runtime,
        scene.actor,
        spell,
        original.clone(),
        scene.fence,
        crate::gameplay_transport::wire_position::WirePositionFrame::from_pin(
            Some(&scene.room),
            scene.room.movement_cells(),
            scene.runtime.content_pin(),
            scene.runtime.map_revision_digest(),
        )
        .ok_or("actual sealed Source frame")?,
    )
    .map_err(|e| format!("actual Source Rune ingress: {e:?}"))?;
    assert_eq!(
        mapped,
        SpellTarget::Position(SpellTargetPosition {
            x: scene.physical.x,
            y: scene.physical.y,
            floor: scene.physical.floor
        })
    );
    assert_eq!(
        legacy.original, original,
        "retained payment/replay binding uses original wire input"
    );
    assert_eq!(legacy.binding().map_err(|e| format!("{e:?}"))?, bytes);
    Ok(())
}

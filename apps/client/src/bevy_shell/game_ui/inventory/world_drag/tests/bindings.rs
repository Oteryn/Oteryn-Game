use super::*;
#[test]
fn ground_prompt_revalidates_removed_definition_and_latest_count_before_confirm() -> TestResult {
    for change in 0..3 {
        let (mut owner, mut ui, mut gesture, viewport, point, mut rx) = fixture(true, false)?;
        start(&mut owner, &mut ui, &mut gesture, viewport, point)?;
        gesture.picking.drop_on(
            0,
            owner.generation(),
            Entity::PLACEHOLDER,
            picking::Destination::Slot(Slot {
                generation: owner.generation(),
                location: Location::Inventory,
                item: None,
                destination: Some(ItemMoveDestination::MainBackpack),
            }),
        );
        step(
            &mut owner,
            &mut ui,
            &mut gesture,
            viewport,
            Vec2::ZERO,
            button(ButtonState::Released),
        )?;
        let (item, destination) = owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .item_movement()
            .prompt
            .ok_or("prompt")?;
        let prompt = Prompt {
            generation: owner.generation(),
            item,
            destination,
        };
        ui.prompt = Some(prompt);
        ui.split_count = 4;
        ui.split_text = Some("4".into());
        let entities = if change == 0 {
            vec![]
        } else {
            vec![WorldSpatialEntity {
                display_name: None,
                entity: EntityRef {
                    identity: [1; 16],
                    generation: 2,
                },
                kind: EntityKind::GroundItem,
                position: ActorPosition {
                    x: 100,
                    y: 100,
                    floor: 0,
                },
                detail: EntityDetail::Object {
                    item_definition_ref: if change == 1 { 43 } else { 42 },
                    quantity: 3,
                    item_handle: Some(7.try_into()?),
                },
            }]
        };
        let play = owner.play.as_mut().ok_or("play")?;
        play.view = play.view.clone().with_entities(entities)?;
        super::super::super::amount::confirm(&owner, &mut ui, prompt);
        assert!(
            rx.try_recv().is_err(),
            "stale draft must not move for change {change}"
        );
        assert!(!ui.split_feedback.is_empty());
        if change == 2 {
            ui.split_text = Some("3".into());
            super::super::super::amount::confirm(&owner, &mut ui, prompt);
            let PlayCommand::MoveItem(request) = rx.try_recv()? else {
                return Err("bounded count missing".into());
            };
            assert_eq!(request.count, Some(3));
            assert_eq!(
                owner
                    .play
                    .as_ref()
                    .ok_or("play")?
                    .view
                    .item_drag_source(TileCoord::new(100, 100))
                    .ok_or("ground still authoritative")?
                    .count,
                3
            );
        }
    }
    Ok(())
}
#[test]
fn viewport_change_or_wrong_floor_cancels_an_active_ground_source() -> TestResult {
    for floor_change in [false, true] {
        let (mut owner, mut ui, mut gesture, viewport, point, mut rx) = fixture(false, false)?;
        start(&mut owner, &mut ui, &mut gesture, viewport, point)?;
        note_world(&mut gesture);
        let changed = if floor_change {
            let play = owner.play.as_mut().ok_or("play")?;
            play.view = PlayView::new(TileCoord::new(100, 100), -1, &[])?;
            viewport
        } else {
            WorldViewport {
                origin: viewport.origin + UVec2::X,
                size: viewport.size,
            }
        };
        before(
            &mut ui,
            &mut gesture,
            0,
            &owner,
            changed,
            Some(point),
            &button(ButtonState::Released),
            true,
        );
        assert!(rx.try_recv().is_err());
        assert!(!gesture.active);
    }
    Ok(())
}

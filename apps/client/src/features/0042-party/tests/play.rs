type Error = Box<dyn std::error::Error + Send + Sync>;

#[test]
fn world_party_target_uses_live_player_kind_floor_and_generation() -> Result<(), Error> {
    use crate::play::PlayView;
    use oteryn_renderer::TileCoord;
    use oteryn_session::{
        ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, WorldSpatialEntity,
    };
    let player = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: [4; 16],
            generation: 8,
        },
        position: ActorPosition {
            x: 2,
            y: 1,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let view = PlayView::new(TileCoord::new(1, 1), 7, &[])?.with_entities(vec![player])?;
    assert_eq!(view.party_target(TileCoord::new(2, 1)), Some(player.entity));
    assert_eq!(view.party_target(TileCoord::new(1, 1)), None);
    for other in [
        WorldSpatialEntity {
            kind: EntityKind::Creature,
            ..player
        },
        WorldSpatialEntity {
            position: ActorPosition {
                floor: 6,
                ..player.position
            },
            ..player
        },
    ] {
        let view = PlayView::new(TileCoord::new(1, 1), 7, &[])?.with_entities(vec![other])?;
        assert_eq!(view.party_target(TileCoord::new(2, 1)), None);
    }
    Ok(())
}

#[test]
fn party_does_not_consume_non_player_context_or_unrelated_batch_events() -> Result<(), Error> {
    use oteryn_input_actions::{
        ButtonState, KeyCode, Modifier, Modifiers, MouseButton, NormalizedInputEvent,
        PointerCoordinate, PointerDelta, PointerMotion, PointerPosition,
    };
    let (link, _commands, _outbox) = crate::play::play_channel();
    let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(1, 1), 7, &[])?;
    let events = [
        NormalizedInputEvent::PointerMoved {
            position: PointerPosition::new(
                PointerCoordinate::new(10)?,
                PointerCoordinate::new(20)?,
            ),
            motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
        },
        NormalizedInputEvent::MouseButton {
            button: MouseButton::SECONDARY,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
        },
        NormalizedInputEvent::Key {
            code: KeyCode::new(19)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control),
            repeat: false,
        },
    ];
    assert!(!link.party_input(
        &events,
        &view,
        &mut oteryn_client_ui::party::PartyInput::default()
    ));
    assert!(oteryn_client_ui::prey::open_key(&events));
    Ok(())
}

#[test]
fn party_member_effects_server_result_reaches_native_panel_and_removes() -> Result<(), Error> {
    use oteryn_client_runtime::party::{CommandStatus, PartyResponse};
    use oteryn_client_ui::party::{Intent, panel_text_at, party};
    let (link, mut commands, outbox) = crate::play::play_channel();
    outbox.party_initialize(true, false);
    let mut state = party::PartyResult::refused(party::PartyDisposition::Ok);
    state.members.push(party::PartyEntry {
        handle: 1,
        name: "Friend".into(),
        leader: false,
        expires_at: 0,
        party_spell_remaining_ms: Some([0, 120_000, 0, 0, 0]),
    });
    for (remaining, expected) in [(120_000, true), (0, false)] {
        link.party_request(Intent::Refresh);
        assert!(commands.try_recv().is_ok());
        state.members[0].party_spell_remaining_ms = Some([0, remaining, 0, 0, 0]);
        let bytes = party::encode_party_result(&state).map_err(|error| format!("{error:?}"))?;
        outbox.party_result(&PartyResponse {
            status: CommandStatus::Accepted,
            operation: bytes.clone(),
            view: Some(bytes),
        });
        assert_eq!(
            panel_text_at(&link.party_panel(), 1_000).contains("Friend | Protect Party: 119s"),
            expected
        );
    }
    Ok(())
}

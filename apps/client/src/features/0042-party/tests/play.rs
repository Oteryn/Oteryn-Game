type Error = Box<dyn std::error::Error + Send + Sync>;

#[test]
fn world_party_target_uses_live_player_kind_floor_and_generation() -> Result<(), Error> {
    use crate::play::PlayView;
    use oteryn_renderer::TileCoord;
    use oteryn_session::{
        ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, WorldSpatialEntity,
    };
    let player = WorldSpatialEntity {
        motion: None,
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
        list: Vec::new(),
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

#[test]
fn party_list_real_play_queue_filters_sorts_and_replaces_nearby_vitals() -> Result<(), Error> {
    use oteryn_client_runtime::party::{CommandStatus, PartyResponse};
    use oteryn_client_ui::party::{Intent, ListAction, ListSort, party};
    let (link, mut commands, outbox) = crate::play::play_channel();
    outbox.party_initialize(true, false);
    let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(1, 1), 7, &[])?;
    let key = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(10)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    assert!(link.party_input(&[key], &view, &mut Default::default()));
    assert!(commands.try_recv().is_ok());
    let mut state = party::PartyResult::refused(party::PartyDisposition::Ok);
    for (handle, name, vocation, distance, hp) in
        [(1, "Zed", 1, 30, Some(40)), (2, "Amy", 2, 31, None)]
    {
        state.members.push(party::PartyEntry {
            handle,
            name: name.into(),
            leader: handle == 1,
            expires_at: 0,
            party_spell_remaining_ms: None,
            list: vec![party::PartyListEntity {
                entity: oteryn_session::EntityRef {
                    identity: [handle as u8; 16],
                    generation: 1,
                },
                name: name.into(),
                vocation,
                summon: false,
                distance,
                health_percent: hp,
                mana_percent: hp,
                skull: hp.map(|_| 0),
                portrait: None,
                portrait_colours: [[0; 3]; 4],
            }],
        });
    }
    use oteryn_client_runtime::appearance::{Binding, Context, Look};
    let entity = state.members[0].list[0].entity;
    state.members[0].list[0].portrait = Some(Binding {
        context: Context {
            content_generation: [1; 32],
            cosmetics_digest: [2; 32],
        },
        appearance_ref: 1,
        entity,
        root_revision: 1,
        condition_revision: 0,
        owner_time_us: 1,
        look: Look::Outfit {
            look_type: 128,
            colours: [0; 4],
            addons: 3,
        },
        selection_revision: None,
        forge_stacks: None,
    });
    state.members[0].list[0].portrait_colours = [[255, 0, 0]; 4];
    let mut summon = state.members[0].list[0].clone();
    summon.portrait = None;
    summon.entity.identity = [3; 16];
    summon.name = "Rat".into();
    summon.vocation = 0;
    summon.summon = true;
    state.members[0].list.push(summon);
    let apply = |state: &party::PartyResult| -> Result<(), Error> {
        let bytes = party::encode_party_result(state).map_err(|e| format!("{e:?}"))?;
        outbox.party_result(&PartyResponse {
            status: CommandStatus::Accepted,
            operation: bytes.clone(),
            view: Some(bytes),
        });
        Ok(())
    };
    apply(&state)?;
    assert_eq!(
        link.party_panel().list_rows()[0]
            .1
            .portrait
            .as_ref()
            .map(|binding| &binding.look),
        state.members[0].list[0]
            .portrait
            .as_ref()
            .map(|binding| &binding.look)
    );
    assert_eq!(
        link.party_panel().list_rows()[0].1.portrait_colours,
        [[255, 0, 0]; 4]
    );
    link.party_list_action(ListAction::Rename("Healers".into()));
    link.party_list_action(ListAction::Sort(ListSort::Name));
    assert_eq!(link.party_panel().list_rows()[0].1.name, "Amy");
    link.party_select_member(0);
    assert_eq!(link.party_panel().selected_member, Some(2));
    link.party_select_member(1);
    assert_eq!(link.party_panel().selected_member, None);
    link.party_list_action(ListAction::ToggleGroup(2));
    assert_eq!(link.party_panel().list_rows().len(), 2);
    link.party_list_action(ListAction::ToggleGroup(6));
    assert_eq!(link.party_panel().list_rows().len(), 1);
    link.party_list_action(ListAction::Sort(ListSort::Health));
    assert_eq!(link.party_panel().list_rows()[0].1.health_percent, Some(40));
    link.party_request(Intent::Refresh);
    assert!(commands.try_recv().is_ok());
    state.members[0].list[0].distance = 31;
    state.members[0].list[0].health_percent = None;
    state.members[0].list[0].mana_percent = None;
    state.members[0].list[0].skull = None;
    apply(&state)?;
    assert_eq!(link.party_panel().list_rows()[0].1.health_percent, None);
    assert_eq!(link.party_panel().list_title, "Healers");
    Ok(())
}

#[test]
fn open_shared_party_refreshes_until_closed() -> Result<(), Error> {
    let (link, mut commands, outbox) = crate::play::play_channel();
    outbox.party_initialize(true, false);
    link.open_party();
    assert!(commands.try_recv().is_ok());
    let bytes = oteryn_client_ui::party::party::encode_party_result(
        &oteryn_client_ui::party::party::PartyResult::refused(oteryn_client_ui::party::party::PartyDisposition::Ok),
    ).map_err(|error| format!("{error:?}"))?;
    outbox.party_result(&oteryn_client_runtime::party::PartyResponse {
        status: oteryn_session::CommandStatus::Accepted,
        operation: bytes.clone(), view: Some(bytes),
    });
    let runtime = oteryn_client_runtime::ClientRuntime::new()?;
    runtime.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(3), commands.recv()).await
    })??.ok_or("no periodic refresh")?;
    link.party_close();
    runtime.block_on(async {
        assert!(tokio::time::timeout(std::time::Duration::from_millis(1100), commands.recv()).await.is_err());
    })?;
    drop(link);
    runtime.block_on(async {
        assert!(tokio::time::timeout(std::time::Duration::from_millis(100), commands.recv()).await?.is_none());
        Ok::<(), Error>(())
    })??;
    Ok(())
}

#[test]
fn party_targets_other_player_on_own_tile_by_identity() -> Result<(), Error> {
    use oteryn_session::{ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, WorldSpatialEntity};
    let own = WorldSpatialEntity {
        motion: None,
        display_name: None, kind: EntityKind::Player,
        entity: EntityRef { identity: [1;16], generation: 1 },
        position: ActorPosition { x: 1, y: 1, floor: 7 },
        detail: EntityDetail::Actor { direction: StepDirection::South, appearance_ref: 1, health_percent: 100 },
    };
    let other = WorldSpatialEntity { entity: EntityRef { identity: [2;16], generation: 4 }, ..own };
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(1,1), 7, &[])?.with_entities(vec![other, own])?;
    view.scene.set_own_identity(own.entity.identity);
    assert_eq!(view.party_target(oteryn_renderer::TileCoord::new(1,1)), Some(other.entity));
    view.entities = vec![own];
    assert_eq!(view.party_target(oteryn_renderer::TileCoord::new(1,1)), None);
    Ok(())
}

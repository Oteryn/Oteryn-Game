use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use oteryn_session::{
    ActorPosition, EntityDetail, EntityDisplayName, EntityKind, EntityRef, StepDirection,
    WorldSpatialEntity,
};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
fn player() -> WorldSpatialEntity {
    WorldSpatialEntity {
        display_name: EntityDisplayName::new("Visible player").ok(),
        entity: EntityRef {
            identity: [4; 16],
            generation: 8,
        },
        kind: EntityKind::Player,
        position: ActorPosition {
            x: 101,
            y: 100,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}
fn app() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let mut host = HostOwner::new(None)?;
    let (link, commands, events) = play_channel();
    link.use_shared_frontend();
    events.bevy_party_selected(true, true, "Leader");
    host.play = Some(super::super::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?.with_entities(vec![player()])?,
        link,
    });
    host.phase = Phase::Playing;
    let mut app = App::new();
    app.insert_non_send(host)
        .insert_resource(Skin {
            buttons: source::ButtonImages {
                up: Handle::default(),
                down: Handle::default(),
            },
            frame: Handle::default(),
            font: Handle::default(),
        })
        .init_resource::<InputFocus>()
        .add_systems(Update, project);
    app.world_mut().spawn((
        Overlay,
        super::super::game_ui::GameModal,
        GlobalZIndex(160),
        Node {
            display: Display::None,
            ..default()
        },
    ));
    Ok((app, commands, events))
}
fn choose(app: &mut App, caption: &str) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Reply, &Children)>()
        .iter(app.world())
        .find(|(_, _, children)| {
            children.iter().any(|child| {
                app.world()
                    .get::<Text>(child)
                    .is_some_and(|text| text.0 == caption)
            })
        })
        .map(|(entity, _, _)| entity)
        .ok_or_else(|| format!("missing choice: {caption}"))?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
#[test]
fn inspect_player_context_requests_equipment_and_opens_window() -> Result {
    let (mut app, mut commands, _) = app()?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bevy_item_details_selected(true);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .request_world_context(TileCoord::new(101, 100));
    app.update();
    choose(&mut app, "Inspect")?;
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("inspection command".into());
    };
    assert_eq!(command.capability, 132);
    assert_eq!(
        command.payload,
        oteryn_client_ui::inspection::encode_request(
            oteryn_client_ui::inspection::Request::Player(player().entity)
        )
        .map_err(|error| format!("request: {error:?}"))?
    );
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .inspection_panel()
            .open
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn inspect_player_context_rejects_changed_generation() -> Result {
    let (mut app, mut commands, _) = app()?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bevy_item_details_selected(true);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .request_world_context(TileCoord::new(101, 100));
    app.update();
    let mut changed = player();
    changed.entity.generation += 1;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .view = PlayView::new(TileCoord::new(100, 100), 7, &[])?.with_entities(vec![changed])?;
    choose(&mut app, "Inspect")?;
    assert!(commands.try_recv().is_err());
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .inspection_panel()
            .open
    );
    Ok(())
}

#[test]
fn party_context_without_look_opens_query_from_actual_choice() -> Result {
    let (mut app, mut commands, _) = app()?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .request_world_context(TileCoord::new(101, 100));
    app.update();
    choose(&mut app, "Party")?;
    let PlayCommand::Party(command) = commands.try_recv()? else {
        return Err("query".into());
    };
    assert_eq!(
        command.kind,
        oteryn_client_runtime::party::party::COMMAND_TYPE_PARTY_QUERY
    );
    assert!(!app.world().non_send::<HostOwner>().dialogs.active());
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .party_is_open()
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
fn load_empty_party(
    app: &mut App,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    events: &crate::play::Outbox,
) -> Result {
    use oteryn_client_runtime::party::{PartyResponse, party};
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .open_party();
    let _ = commands.try_recv()?;
    let view = party::PartyResult {
        disposition: party::PartyDisposition::Ok,
        shared_xp_enabled: false,
        members: vec![],
        invitations: vec![],
    };
    let bytes = party::encode_party_result(&view).map_err(|_| "party view")?;
    events.bevy_party_response(&PartyResponse {
        status: oteryn_session::CommandStatus::Accepted,
        operation: bytes.clone(),
        view: Some(bytes),
    });
    Ok(())
}
#[test]
fn combined_party_choice_closes_own_modal_before_exact_invite_and_rejects_stale_recipient() -> Result
{
    use oteryn_client_runtime::party::party;
    let (mut app, mut commands, events) = app()?;
    load_empty_party(&mut app, &mut commands, &events)?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .request_world_context(TileCoord::new(101, 100));
    app.update();
    let mut changed = player();
    changed.entity.generation += 1;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .view = PlayView::new(TileCoord::new(100, 100), 7, &[])?.with_entities(vec![changed])?;
    choose(&mut app, "Invite to Party")?;
    assert!(commands.try_recv().is_err());
    assert!(!app.world().non_send::<HostOwner>().dialogs.active());
    app.update();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .request_world_context(TileCoord::new(101, 100));
    app.update();
    choose(&mut app, "Invite to Party")?;
    let PlayCommand::Party(command) = commands.try_recv()? else {
        return Err("invite".into());
    };
    assert_eq!(
        party::decode_party_intent(&command.payload).map_err(|_| "intent")?,
        party::PartyIntent::Invite(changed.entity)
    );
    assert!(!app.world().non_send::<HostOwner>().dialogs.active());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn combined_menu_preserves_base_look_and_real_name_and_old_choices_cannot_replace_new_dialog()
-> Result {
    let (mut app, mut commands, _) = app()?;
    let tile = TileCoord::new(101, 100);
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let target = party::context(&owner, tile).ok_or("party context")?;
        let mut menu = Menu::new(
            oteryn_client_runtime::world_interaction::Target::Creature(player().entity),
            true,
            false,
        );
        menu.tile = Some(tile);
        let generation = owner.generation();
        owner.dialogs.open(
            generation,
            Content::Context {
                menu: Some(Box::new(menu)),
                party: Some(Box::new(target)),
                inspection: None,
                report: None,
            },
        );
    }
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Visible player")
    );
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Look")
    );
    let invite = app
        .world_mut()
        .query::<(Entity, &Reply)>()
        .iter(app.world())
        .find(|(_, reply)| matches!(reply.choice, Choice::Invite))
        .map(|(entity, _)| entity)
        .ok_or("invite")?;
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(invite)
            .is_some(),
        "unknown party state must not enable Invite"
    );
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .request_logout_confirmation();
    choose(&mut app, "Party")?;
    assert!(
        app.world().non_send::<HostOwner>().dialogs.active(),
        "stale context reply must preserve logout confirmation"
    );
    assert!(commands.try_recv().is_err());
    app.update();
    choose(&mut app, "Cancel")?;
    assert!(!app.world().non_send::<HostOwner>().dialogs.active());
    Ok(())
}

#[test]
fn rule_violation_context_report_opens_for_captured_player() -> Result {
    let (mut app, mut commands, events) = app()?;
    events.bevy_report_selected(true);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .request_world_context(TileCoord::new(101, 100));
    app.update();
    choose(&mut app, "Report")?;
    let owner = app.world().non_send::<HostOwner>();
    let panel = owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .player_report_panel();
    assert!(panel.open);
    assert_eq!(panel.fields[2], "Visible player");
    assert!(panel.fields[0].is_empty());
    assert!(panel.fields[1].is_empty());
    assert!(commands.try_recv().is_err());
    assert!(!owner.dialogs.active());
    Ok(())
}

#[test]
fn rule_violation_shortcut_respects_text_focus() -> Result {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut app, _, events) = app()?;
    events.bevy_report_selected(true);
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(21)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Alt),
        repeat: false,
    };
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.route_input(std::slice::from_ref(&key), true);
    assert!(
        !owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .player_report_panel()
            .open
    );
    owner.route_input(&[key], false);
    assert!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .player_report_panel()
            .open
    );
    Ok(())
}

#[test]
fn combat_and_follow_context_buttons_use_existing_command_owners() -> Result {
    use oteryn_session::target_move::{TargetMoveIntent, decode_target_move_intent};
    let (mut app, mut commands, events) = app()?;
    events.world_context_initialize(true, false);
    events.target_move_initialize(true);
    let mut combat = oteryn_session::ActorCombatState {
        target: None,
        modes: oteryn_session::FightModes::DEFAULT,
        in_fight: false,
    };
    events.battle_publish(Some(combat), Some(7), &[player()]);
    let open = |app: &mut App| {
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .request_world_context(TileCoord::new(101, 100));
        app.update();
    };
    open(&mut app);
    choose(&mut app, "Attack")?;
    assert_eq!(
        commands.try_recv()?,
        PlayCommand::Combat(crate::combat_input::CombatCommand::Attack(player().entity))
    );
    combat.target = Some(player().entity);
    events.battle_publish(Some(combat), Some(7), &[player()]);
    open(&mut app);
    choose(&mut app, "Stop Attack")?;
    assert_eq!(
        commands.try_recv()?,
        PlayCommand::Combat(crate::combat_input::CombatCommand::Stop)
    );
    open(&mut app);
    choose(&mut app, "Follow")?;
    let PlayCommand::ServerMechanic(follow) = commands.try_recv()? else {
        return Err("follow".into());
    };
    assert_eq!(
        decode_target_move_intent(&follow.payload),
        Ok(TargetMoveIntent::Follow(player().entity))
    );
    events.target_move_result(
        &follow,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![8, 1],
        },
    );
    open(&mut app);
    choose(&mut app, "Stop Follow")?;
    let PlayCommand::ServerMechanic(stop) = commands.try_recv()? else {
        return Err("stop follow".into());
    };
    assert_eq!(
        decode_target_move_intent(&stop.payload),
        Ok(TargetMoveIntent::Stop)
    );
    Ok(())
}

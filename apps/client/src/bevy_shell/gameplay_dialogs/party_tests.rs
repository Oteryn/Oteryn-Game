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
                menu: Some(menu),
                party: Some(Box::new(target)),
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

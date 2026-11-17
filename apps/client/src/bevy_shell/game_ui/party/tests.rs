use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use oteryn_client_runtime::party::{PartyResponse, party};
use oteryn_renderer::TileCoord;
use oteryn_session::CommandStatus;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    HostOwner,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
fn host() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let mut host = HostOwner::new(None)?;
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    host.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    host.phase = crate::bevy_shell::owner::Phase::Playing;
    outbox.bevy_party_selected(true, true, "Leader");
    Ok((host, commands, outbox))
}
fn state() -> party::PartyResult {
    let entry = |handle, name: &str, leader| party::PartyEntry {
        handle,
        name: name.into(),
        leader,
        expires_at: 0,
        list: vec![],
        party_spell_remaining_ms: None,
    };
    party::PartyResult {
        disposition: party::PartyDisposition::Ok,
        shared_xp_enabled: false,
        members: vec![entry(1, "Leader", true), entry(2, "Member", false)],
        invitations: vec![party::PartyEntry {
            expires_at: 4_000_000_000,
            ..entry(3, "Inviter", false)
        }],
    }
}
fn publish(
    outbox: &crate::play::Outbox,
    state: &party::PartyResult,
    disposition: party::PartyDisposition,
) -> Result {
    outbox.bevy_party_response(&PartyResponse {
        status: CommandStatus::Accepted,
        operation: party::encode_party_result(&party::PartyResult::refused(disposition))
            .map_err(|_| "operation")?,
        view: Some(party::encode_party_result(state).map_err(|_| "view")?),
    });
    Ok(())
}
#[test]
fn canonical_party_shortcut_reaches_query_and_preserves_modal_input_fence() -> Result {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut owner, mut commands, _) = host()?;
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(10)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
        repeat: false,
    };
    owner.route_input(std::slice::from_ref(&key), true);
    assert!(commands.try_recv().is_err());
    owner.open_graphics();
    owner.route_input(std::slice::from_ref(&key), false);
    assert!(commands.try_recv().is_err());
    owner.cancel();
    owner.route_input(&[key], false);
    let PlayCommand::Party(command) = commands.try_recv()? else {
        return Err("party query missing".into());
    };
    assert_eq!(command.kind, party::COMMAND_TYPE_PARTY_QUERY);
    assert!(command.payload.is_empty());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn real_server_members_are_presented_in_nonmodal_party_dock() -> Result {
    let (owner, mut commands, outbox) = host()?;
    owner.play.as_ref().ok_or("play")?.link.open_party();
    assert!(matches!(commands.try_recv()?, PlayCommand::Party(_)));
    publish(&outbox, &state(), party::PartyDisposition::Ok)?;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.world_mut().spawn((PartyDock, Node::default()));
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|name| name.as_str() == "party.member")
            .count(),
        2,
        "canonical members need actual source-shaped rows"
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
fn app() -> std::result::Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        crate::play::Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let (owner, mut commands, outbox) = host()?;
    owner.play.as_ref().ok_or("play")?.link.open_party();
    let _ = commands.try_recv()?;
    publish(&outbox, &state(), party::PartyDisposition::Ok)?;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.world_mut().spawn((PartyDock, Node::default()));
    // Command tests advance the clock only when exercising live refresh.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    app.update();
    Ok((app, commands, outbox))
}
fn click(app: &mut App, kind: impl Fn(Kind) -> bool) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| kind(control.kind))
        .map(|(entity, _)| entity)
        .ok_or("control")?;
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity });
    Ok(())
}
fn command(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> std::result::Result<oteryn_client_runtime::party::PartyCommand, Box<dyn std::error::Error>> {
    let PlayCommand::Party(command) = commands.try_recv()? else {
        return Err("party command".into());
    };
    Ok(command)
}
#[test]
fn actual_member_and_invitation_controls_send_exact_handles_without_optimistic_membership() -> Result
{
    for action in [
        Kind::Pass,
        Kind::Kick,
        Kind::Join,
        Kind::Leave,
        Kind::Shared,
    ] {
        let (mut app, mut commands, outbox) = app()?;
        let roles: Vec<_> = app
            .world_mut()
            .query::<&crate::bevy_shell::game_ui::install::RememberScroll>()
            .iter(app.world())
            .map(|key| (key.0, key.1))
            .collect();
        assert_eq!(
            roles
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            roles.len()
        );
        let before = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .party_panel()
            .state;
        if matches!(action, Kind::Pass | Kind::Kick) {
            click(&mut app, |kind| matches!(kind, Kind::Member(2)))?;
            app.update();
        }
        if matches!(action, Kind::Join) {
            click(&mut app, |kind| matches!(kind, Kind::Invitation(3)))?;
            app.update();
        }
        click(&mut app, |kind| {
            matches!(
                (kind, action),
                (Kind::Pass, Kind::Pass)
                    | (Kind::Kick, Kind::Kick)
                    | (Kind::Join, Kind::Join)
                    | (Kind::Leave, Kind::Leave)
                    | (Kind::Shared, Kind::Shared)
            )
        })?;
        let sent = command(&mut commands)?;
        if matches!(action, Kind::Shared) {
            assert_eq!(sent.shared_setting(), Some(true));
        } else {
            assert_eq!(
                party::decode_party_intent(&sent.payload).map_err(|_| "intent")?,
                match action {
                    Kind::Pass => party::PartyIntent::PassLeadership(2),
                    Kind::Kick => party::PartyIntent::Kick(2),
                    Kind::Join => party::PartyIntent::Accept(3),
                    _ => party::PartyIntent::Leave,
                }
            );
        }
        assert_eq!(
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .party_panel()
                .state,
            before
        );
        assert!(commands.try_recv().is_err());
        publish(&outbox, &state(), party::PartyDisposition::NotLeader)?;
        app.update();
        let panel = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .party_panel();
        assert_eq!(panel.state, before);
        assert!(panel.pending.is_none());
        assert!(
            panel
                .feedback
                .is_some_and(|message| message.contains("leader"))
        );
    }
    Ok(())
}
#[test]
fn closing_pending_party_preserves_data_and_late_response_stays_closed() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    click(&mut app, |kind| matches!(kind, Kind::Refresh))?;
    assert_eq!(
        command(&mut commands)?.kind,
        party::COMMAND_TYPE_PARTY_QUERY
    );
    click(&mut app, |kind| matches!(kind, Kind::Close))?;
    app.update();
    let panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .party_panel();
    assert!(!panel.open);
    assert!(panel.pending.is_some());
    assert_eq!(panel.state, Some(state()));
    publish(&outbox, &state(), party::PartyDisposition::Ok)?;
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|name| name.as_str() == "party.sidebar")
            .count(),
        0
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn stale_revision_selection_pending_capability_and_top_modal_never_dispatch() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    click(&mut app, |kind| matches!(kind, Kind::Member(2)))?;
    app.update();
    let mut changed = state();
    changed.members[1].name = "Changed member".into();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .party_request(oteryn_client_ui::party::Intent::Refresh);
    let _ = command(&mut commands)?;
    publish(&outbox, &changed, party::PartyDisposition::Ok)?;
    click(&mut app, |kind| matches!(kind, Kind::Kick))?;
    assert!(commands.try_recv().is_err());
    app.update();
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(180), Node::default()))
        .id();
    click(&mut app, |kind| matches!(kind, Kind::Leave))?;
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    outbox.bevy_party_selected(false, true, "Leader");
    click(&mut app, |kind| matches!(kind, Kind::Leave))?;
    assert!(commands.try_recv().is_err());
    outbox.bevy_party_selected(true, true, "Leader");
    app.world_mut().non_send_mut::<HostOwner>().close();
    click(&mut app, |kind| matches!(kind, Kind::Leave))?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
mod context;

#[test]
fn party_combat_rows_show_wire_vitals_summons_and_reachable_filters() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    let mut result = state();
    let row = |id, name: &str, summon, distance| party::PartyListEntity {
        entity: EntityRef {
            identity: [id; 16],
            generation: 1,
        },
        name: name.into(),
        vocation: 1,
        summon,
        distance,
        health_percent: (distance <= 30).then_some(42),
        mana_percent: (distance <= 30).then_some(65),
        skull: (distance <= 30).then_some(3),
        portrait: None,
        portrait_colours: [[0; 3]; 4],
    };
    result.members[0].list = vec![row(1, "Zed", false, 30), row(3, "Summon", true, 30)];
    result.members[1].list = vec![row(2, "Amy", false, 31)];
    click(&mut app, |kind| matches!(kind, Kind::Refresh))?;
    let _ = command(&mut commands)?;
    publish(&outbox, &result, party::PartyDisposition::Ok)?;
    app.update();
    let count = |app: &mut App, tag: &str| {
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|name| name.as_str() == tag)
            .count()
    };
    assert_eq!(count(&mut app, "party.combat-row"), 3);
    assert_eq!(count(&mut app, "party.health"), 2);
    assert_eq!(count(&mut app, "party.mana"), 2);
    assert_eq!(count(&mut app, "party.skull"), 2);
    let fills: Vec<_> = app
        .world_mut()
        .query::<(&Name, &Node)>()
        .iter(app.world())
        .filter(|(name, _)| name.as_str() == "party.health.fill")
        .map(|(_, node)| node.width)
        .collect();
    assert_eq!(fills, vec![percent(42.); 2]);
    // Drive the actual button observer, rather than changing the retained model directly.
    let button = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find(|(text, _)| text.0 == "Summons")
        .map(|(_, parent)| parent.parent())
        .ok_or("summon filter")?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert_eq!(count(&mut app, "party.combat-row"), 2);
    assert_eq!(count(&mut app, "party.health"), 1);
    click(&mut app, |kind| {
        matches!(kind, Kind::Sort(oteryn_client_ui::party::ListSort::Name))
    })?;
    app.update();
    let parent = app
        .world_mut()
        .query::<(&Name, &ChildOf)>()
        .iter(app.world())
        .find(|(name, _)| name.as_str() == "party.combat-row")
        .map(|(_, parent)| parent.parent())
        .ok_or("combat list")?;
    let order: Vec<_> = app
        .world()
        .get::<Children>(parent)
        .ok_or("rows")?
        .iter()
        .filter_map(|entity| app.world().get::<Control>(entity))
        .filter_map(|control| {
            if let Kind::Member(handle) = control.kind {
                Some(handle)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        order,
        [2, 1],
        "Name sorting must reorder the actual rendered rows"
    );
    click(&mut app, |kind| matches!(kind, Kind::Filter(1)))?;
    app.update();
    assert_eq!(count(&mut app, "party.combat-row"), 0);

    assert!(
        commands.try_recv().is_err(),
        "local filtering must not send membership commands"
    );
    Ok(())
}

#[test]
fn party_portrait_projects_pinned_outfit_artwork_into_bevy_image() -> Result {
    use oteryn_client_runtime::appearance::{Binding, Context, Look};
    let (mut app, mut commands, outbox) = app()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let assets = super::super::outfit::preview::Assets::load(&root)?;
    *app.world_mut().resource_mut::<combat::Artwork>() = combat::Artwork {
        assets: Some(assets),
        attempted: true,
    };
    let entity = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let mut result = state();
    result.members[0].list.push(party::PartyListEntity {
        entity,
        name: "Leader".into(),
        vocation: 1,
        summon: false,
        distance: 0,
        health_percent: Some(100),
        mana_percent: Some(100),
        skull: None,
        portrait: Some(Binding {
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
                addons: 3,
                colours: [0; 4],
            },
            selection_revision: None,
            forge_stacks: None,
        }),
        portrait_colours: [[255, 100, 50]; 4],
    });
    click(&mut app, |kind| matches!(kind, Kind::Refresh))?;
    let _ = command(&mut commands)?;
    publish(&outbox, &result, party::PartyDisposition::Ok)?;
    app.update();
    let portrait = app
        .world_mut()
        .query::<(&Name, &ImageNode)>()
        .iter(app.world())
        .find(|(name, _)| name.as_str() == "party.portrait")
        .map(|(_, image)| image.clone())
        .ok_or("portrait image")?;
    assert_eq!(portrait.rect, Some(Rect::new(32., 32., 96., 96.)));
    let image = app
        .world()
        .resource::<Assets<Image>>()
        .get(&portrait.image)
        .ok_or("image asset")?;
    assert_eq!(image.size(), UVec2::splat(96));
    assert!(
        image
            .data
            .as_ref()
            .ok_or("pixel data")?
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[..3] != [32, 32, 32]),
        "portrait must contain verified outfit pixels"
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn open_party_refreshes_live_vitals_without_overlapping_requests_or_polling_when_closed() -> Result
{
    let (mut app, mut commands, outbox) = app()?;
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs(1),
    ));
    app.world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(std::time::Duration::from_secs(2));
    app.update();
    app.update();
    assert_eq!(
        command(&mut commands)?.kind,
        party::COMMAND_TYPE_PARTY_QUERY
    );
    for _ in 0..3 {
        app.update();
    }
    assert!(
        commands.try_recv().is_err(),
        "a pending query must not overlap"
    );
    publish(&outbox, &state(), party::PartyDisposition::Ok)?;
    // A closed dock must stop the refresh stream immediately.
    click(&mut app, |kind| matches!(kind, Kind::Close))?;
    for _ in 0..3 {
        app.update();
    }
    assert!(commands.try_recv().is_err());
    Ok(())
}

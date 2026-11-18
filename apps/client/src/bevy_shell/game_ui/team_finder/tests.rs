use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::ui::InteractionDisabled;
use oteryn_client_runtime::server_mechanic::{ServerMechanicCommand, ServerMechanicResponse};
use oteryn_client_ui::team_finder::*;
use oteryn_client_ui::team_finder_leader::{LeaderListing, TeamTarget};
pub(super) type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
pub(super) type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture(selected: bool) -> Result<Fixture> {
    fixture_with_layout(selected, false)
}
pub(super) fn fixture_with_layout(selected: bool, layout: bool) -> Result<Fixture> {
    let (link, rx, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_team_selected(selected);
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
    owner.play.as_ref().ok_or("play")?.link.discovery_open(232);
    let mut app = App::new();
    if layout {
        super::super::pointer_layout::install(&mut app, UVec2::new(1200, 900));
        app.add_systems(
            Startup,
            configure_shell_camera.after(crate::bevy_shell::ui::setup),
        );
        app.add_message::<bevy::window::Ime>()
            .add_message::<bevy::window::WindowEvent>()
            .add_message::<bevy::window::WindowFocused>()
            .add_plugins((
                bevy::input::InputPlugin,
                bevy::input_focus::InputFocusPlugin,
                bevy::input_focus::InputDispatchPlugin,
                bevy::input_focus::pointer_focus::PointerFocusPlugin,
                bevy::input_focus::tab_navigation::TabNavigationPlugin,
                bevy::mesh::MeshPlugin,
                bevy::camera::visibility::VisibilityPlugin,
                bevy::picking::PickingPlugin,
                bevy::picking::input::PointerInputPlugin,
                bevy::picking::InteractionPlugin,
                bevy::ui_widgets::ButtonPlugin,
                bevy::ui_widgets::ScrollAreaPlugin,
                bevy::ui_widgets::TextInputPlugin,
                oteryn_client_ui_bevy::GlobalUiPlugin,
            ));
    } else {
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
        ))
        .init_resource::<UiScale>()
        .init_resource::<bevy::text::RemSize>()
        .add_systems(
            PostUpdate,
            bevy::ui::widget::update_editable_text_styles.before(bevy::text::EditableTextSystems),
        );
    }
    app.add_systems(Startup, test_font.after(crate::bevy_shell::ui::setup))
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, rx, outbox))
}
fn configure_shell_camera(
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut Camera), With<IsDefaultUiCamera>>,
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
) {
    for (entity, mut camera) in &mut cameras {
        if camera.computed.target_info.is_some() {
            // The fixture camera is superseded by the production shell camera.
            commands.entity(entity).despawn();
        } else {
            camera.computed.target_info = Some(bevy::camera::RenderTargetInfo {
                physical_size: window.physical_size(),
                scale_factor: window.scale_factor(),
            });
        }
    }
}

fn test_font(mut skins: ResMut<Skins>, mut fonts: ResMut<Assets<Font>>) {
    let font = Font::from_bytes(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/assets/FiraMono-subset.ttf"
        ))
        .to_vec(),
    );
    skins.font = fonts.add(font);
}
pub(super) fn click(app: &mut App, kind: Kind) -> Result {
    let key = app.world().resource::<State>().key;
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, c)| c.key == key && c.kind == kind)
        .map(|(e, _)| e)
        .ok_or("team control absent")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
pub(super) fn command(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<ServerMechanicCommand> {
    match rx.try_recv()? {
        PlayCommand::ServerMechanic(c) => Ok(c),
        _ => Err("wrong command".into()),
    }
}
pub(super) fn reply(outbox: &Outbox, payload: Vec<u8>) {
    outbox.bevy_team_response(&ServerMechanicResponse {
        command_id: 1,
        status: oteryn_session::CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    });
}
pub(super) fn page(status: MemberStatus) -> TeamFinderResult {
    let listing = LeaderListing {
        min_level: 10,
        max_level: 100,
        vocation_mask: 15,
        slots: 4,
        free_slots: 3,
        party_members_join: false,
        timestamp: 0,
        target: TeamTarget::Boss(1),
    };
    TeamFinderResult::Browse {
        listings: vec![
            BrowseListing {
                leader_handle: 7,
                name: "First".into(),
                listing,
                occupied_slots: 1,
                status,
            },
            BrowseListing {
                leader_handle: 9,
                name: "Second".into(),
                listing,
                occupied_slots: 2,
                status,
            },
        ],
        next: Some(9),
    }
}
#[test]
fn team_search_filter_application_cancel_and_paging() -> Result {
    let (mut app, mut rx, outbox) = fixture(true)?;
    click(&mut app, Kind::Browse(false))?;
    assert_eq!(
        decode_query(&command(&mut rx)?.payload).map_err(|error| format!("{error:?}"))?,
        TeamFinderQuery::Browse { after: None }
    );
    reply(
        &outbox,
        encode_result(&page(MemberStatus::None)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    let filter = TeamFilter {
        name: "second".into(),
        level: Some(25),
        ..default()
    };
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if matches!(field, Field::Name) {
            *text = EditableText::new("second");
        } else if matches!(field, Field::Level) {
            *text = EditableText::new("25");
        }
    }
    click(&mut app, Kind::Select(1))?;
    app.update();
    click(&mut app, Kind::Filter(TeamFilter::default()))?;
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains("Second") && t.0.contains("2 free"))
    );
    assert_eq!(
        app.world()
            .resource::<State>()
            .snapshot
            .as_ref()
            .ok_or("snapshot")?
            .1
            .filter,
        filter
    );
    click(&mut app, Kind::Apply(false))?;
    let sent = command(&mut rx)?;
    assert_eq!((sent.capability, sent.command_type), (232, 445));
    assert_eq!(
        decode_member_action(&sent.payload).map_err(|error| format!("{error:?}"))?,
        MemberAction::Request { leader: 9 }
    );
    // The old UI is still present until projection; duplicate activation must not send.
    click(&mut app, Kind::Apply(false))?;
    assert!(rx.try_recv().is_err());
    reply(&outbox, vec![]);
    app.update();
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&page(MemberStatus::Pending)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    click(&mut app, Kind::Apply(true))?;
    assert_eq!(
        decode_member_action(&command(&mut rx)?.payload).map_err(|error| format!("{error:?}"))?,
        MemberAction::Cancel { leader: 9 }
    );
    reply(&outbox, vec![]);
    app.update();
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&page(MemberStatus::None)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    click(&mut app, Kind::Browse(true))?;
    assert_eq!(
        decode_query(&command(&mut rx)?.payload).map_err(|error| format!("{error:?}"))?,
        TeamFinderQuery::Browse { after: Some(9) }
    );
    app.update();
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}
#[test]
fn team_unavailable_session_and_stale_snapshot_send_nothing() -> Result {
    let (mut app, mut rx, _) = fixture(false)?;
    click(&mut app, Kind::Browse(false))?;
    assert!(rx.try_recv().is_err());
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    let (mut app, mut rx, outbox) = fixture(true)?;
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&page(MemberStatus::None)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    click(&mut app, Kind::Select(1))?;
    click(&mut app, Kind::Apply(false))?;
    assert!(rx.try_recv().is_err());
    app.update();
    click(&mut app, Kind::Apply(false))?;
    assert_eq!(
        decode_member_action(&command(&mut rx)?.payload).map_err(|error| format!("{error:?}"))?,
        MemberAction::Request { leader: 9 }
    );
    Ok(())
}
fn listing_texts(app: &mut App, values: [&str; 9]) {
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if let Field::Listing(index) = field
            && let Some(value) = values.get(*index)
        {
            *text = EditableText::new(*value);
        }
    }
}
pub(super) fn shown(app: &mut App, value: &str) -> bool {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| text.0.contains(value))
}
#[test]
fn manage_team_decides_applicants_and_publishes_and_removes_the_listing() -> Result {
    use oteryn_client_ui::team_finder_leader::{decode_upsert, encode_upsert};
    let (mut app, mut rx, outbox) = fixture(true)?;
    outbox.bevy_team_leader_selected(true);
    app.update();
    click(&mut app, Kind::Applicants(false))?;
    let sent = command(&mut rx)?;
    assert_eq!((sent.capability, sent.command_type), (232, 444));
    assert_eq!(
        decode_query(&sent.payload).map_err(|error| format!("{error:?}"))?,
        TeamFinderQuery::Applicants { after: None }
    );
    // The wire requires the leader's listing on any applicants page with members.
    let applicants = TeamFinderResult::Applicants {
        listing: Some(advertised(10)),
        occupied_slots: 1,
        members: vec![Applicant {
            handle: 5,
            name: "Ally".into(),
            level: 50,
            vocation: 1,
            status: MemberStatus::Pending,
        }],
        next: None,
    };
    reply(
        &outbox,
        encode_result(&applicants).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    assert!(shown(&mut app, "Social — Manage Team"));
    assert!(shown(&mut app, "Ally — level 50"));
    click(&mut app, Kind::Decide(3))?;
    let sent = command(&mut rx)?;
    assert_eq!((sent.capability, sent.command_type), (232, 446));
    assert_eq!(
        decode_leader_action(&sent.payload).map_err(|error| format!("{error:?}"))?,
        LeaderAction::Accept { member: 5 }
    );
    reply(&outbox, vec![]);
    app.update();
    // An incomplete listing stays local and says why.
    listing_texts(&mut app, [""; 9]);
    click(&mut app, Kind::Publish(Vec::new()))?;
    assert!(rx.try_recv().is_err());
    app.update();
    assert!(shown(&mut app, tr!("Enter a valid team listing")));
    listing_texts(&mut app, ["10", "100", "15", "4", "3", "1", "2", "7", "12"]);
    click(&mut app, Kind::Publish(Vec::new()))?;
    let sent = command(&mut rx)?;
    assert_eq!((sent.capability, sent.command_type), (200, 390));
    let listing = LeaderListing {
        min_level: 10,
        max_level: 100,
        vocation_mask: 15,
        slots: 4,
        free_slots: 3,
        party_members_join: true,
        timestamp: 0,
        target: TeamTarget::Hunt { kind: 7, area: 12 },
    };
    assert_eq!(
        decode_upsert(&sent.payload).map_err(|error| format!("{error:?}"))?,
        listing
    );
    assert_eq!(
        sent.payload,
        encode_upsert(listing).map_err(|error| format!("{error:?}"))?
    );
    outbox.bevy_team_leader_response(&ServerMechanicResponse {
        command_id: 2,
        status: oteryn_session::CommandStatus::Accepted,
        server_sequence: 2,
        payload: vec![],
    });
    app.update();
    assert!(shown(&mut app, tr!("Advertisement confirmed")));
    assert!(
        app.world_mut()
            .query::<(&Field, &EditableText)>()
            .iter(app.world())
            .any(|(field, text)| matches!(field, Field::Listing(8)) && text.value() == "12")
    );
    click(&mut app, Kind::Withdraw)?;
    let sent = command(&mut rx)?;
    assert_eq!((sent.capability, sent.command_type), (200, 391));
    assert!(sent.payload.is_empty());
    outbox.bevy_team_leader_response(&ServerMechanicResponse {
        command_id: 3,
        status: oteryn_session::CommandStatus::Accepted,
        server_sequence: 3,
        payload: vec![],
    });
    app.update();
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    assert_eq!(link.discovery_panel().leader.confirmed, None);
    click(&mut app, Kind::Browse(false))?;
    assert_eq!(
        decode_query(&command(&mut rx)?.payload).map_err(|error| format!("{error:?}"))?,
        TeamFinderQuery::Browse { after: None }
    );
    Ok(())
}
fn editors(app: &mut App) -> [String; 9] {
    let mut values: [String; 9] = Default::default();
    for (field, text) in app
        .world_mut()
        .query::<(&Field, &EditableText)>()
        .iter(app.world())
    {
        if let Field::Listing(index) = field
            && let Some(value) = values.get_mut(*index)
        {
            *value = text.value().to_string();
        }
    }
    values
}
fn withdraw_enabled(app: &mut App) -> Result<bool> {
    let (entity, _) = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| control.kind == Kind::Withdraw)
        .ok_or("remove control absent")?;
    Ok(app.world().get::<InteractionDisabled>(entity).is_none())
}
fn leader_reply(outbox: &Outbox, status: oteryn_session::CommandStatus) {
    outbox.bevy_team_leader_response(&ServerMechanicResponse {
        command_id: 2,
        status,
        server_sequence: 2,
        payload: vec![],
    });
}
pub(super) fn advertised(min_level: u16) -> LeaderListing {
    LeaderListing {
        min_level,
        max_level: 100,
        vocation_mask: 15,
        slots: 4,
        free_slots: 3,
        party_members_join: false,
        timestamp: 0,
        target: TeamTarget::Boss(1),
    }
}
pub(super) fn applicants_page(listing: Option<LeaderListing>) -> TeamFinderResult {
    let member = |handle: u64, name: &str| Applicant {
        handle,
        name: name.into(),
        level: 50,
        vocation: 1,
        status: MemberStatus::Pending,
    };
    // The wire carries applicants only under an active listing.
    let members = if listing.is_some() {
        vec![member(5, "Ally"), member(6, "Brave")]
    } else {
        Vec::new()
    };
    TeamFinderResult::Applicants {
        listing,
        occupied_slots: u16::from(listing.is_some()),
        members,
        next: None,
    }
}
/// Opens Manage Team through its tab and answers the applicants query with `listing`.
fn manage_team(listing: Option<LeaderListing>) -> Result<Fixture> {
    manage_team_with_layout(listing, false)
}
pub(super) fn manage_team_with_layout(
    listing: Option<LeaderListing>,
    layout: bool,
) -> Result<Fixture> {
    let (mut app, mut rx, outbox) = fixture_with_layout(true, layout)?;
    outbox.bevy_team_leader_selected(true);
    app.update();
    click(&mut app, Kind::Applicants(false))?;
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&applicants_page(listing)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    Ok((app, rx, outbox))
}
#[test]
fn publish_explains_every_listing_the_protocol_refuses() -> Result {
    let (mut app, mut rx, _) = manage_team(None)?;
    assert!(shown(
        &mut app,
        tr!("Vocation mask (1-255, at least one vocation)")
    ));
    assert!(!shown(&mut app, "0 = any"));
    for (values, problem) in [
        (
            ["10", "100", "0", "4", "3", "1", "2", "7", "12"],
            tr!("Choose at least one vocation (mask 1-255)"),
        ),
        (
            ["0", "100", "15", "4", "3", "1", "2", "7", "12"],
            tr!("The minimum level must be at least 1 and not above the maximum level"),
        ),
        (
            ["101", "100", "15", "4", "3", "1", "1", "7", ""],
            tr!("The minimum level must be at least 1 and not above the maximum level"),
        ),
        (
            ["10", "100", "15", "0", "0", "0", "3", "7", ""],
            tr!("The team size must be at least 1"),
        ),
        (
            ["10", "100", "15", "4", "5", "0", "1", "7", ""],
            tr!("Free slots cannot exceed the team size"),
        ),
    ] {
        listing_texts(&mut app, values);
        click(&mut app, Kind::Publish(Vec::new()))?;
        assert!(rx.try_recv().is_err());
        app.update();
        assert!(shown(&mut app, problem), "{problem}");
    }
    Ok(())
}
#[test]
fn manage_team_editors_follow_the_latest_server_listing() -> Result {
    use oteryn_session::CommandStatus::Accepted;
    let (mut app, mut rx, outbox) = manage_team(Some(advertised(10)))?;
    assert_eq!(editors(&mut app)[0], "10");
    listing_texts(&mut app, ["20", "100", "15", "4", "3", "0", "1", "1", ""]);
    click(&mut app, Kind::Publish(Vec::new()))?;
    command(&mut rx)?;
    app.update();
    leader_reply(&outbox, Accepted);
    app.update();
    assert!(shown(&mut app, tr!("Advertisement confirmed")));
    assert_eq!(editors(&mut app)[0], "20");
    click(&mut app, Kind::Withdraw)?;
    command(&mut rx)?;
    app.update();
    leader_reply(&outbox, Accepted);
    app.update();
    assert_eq!(editors(&mut app), <[String; 9]>::default());
    assert!(!withdraw_enabled(&mut app)?);
    listing_texts(&mut app, ["30", "100", "15", "4", "3", "0", "1", "1", ""]);
    click(&mut app, Kind::Publish(Vec::new()))?;
    command(&mut rx)?;
    app.update();
    leader_reply(&outbox, Accepted);
    app.update();
    assert_eq!(editors(&mut app)[0], "30");
    assert!(withdraw_enabled(&mut app)?);
    // The server's applicants page says the listing is gone: the older confirmation must not win.
    click(&mut app, Kind::Applicants(false))?;
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&applicants_page(None)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    assert_eq!(editors(&mut app), <[String; 9]>::default());
    assert!(!withdraw_enabled(&mut app)?);
    Ok(())
}
#[test]
fn manage_team_keeps_the_listing_draft_across_selection_queries_and_rejection() -> Result {
    let (mut app, mut rx, outbox) = manage_team(Some(advertised(10)))?;
    let draft = ["40", "90", "15", "5", "2", "1", "2", "7", "12"];
    let typed = draft.map(String::from);
    listing_texts(&mut app, draft);
    click(&mut app, Kind::Select(1))?;
    app.update();
    assert!(shown(&mut app, "> Brave"));
    assert_eq!(editors(&mut app), typed);
    click(&mut app, Kind::Applicants(false))?;
    command(&mut rx)?;
    app.update();
    assert_eq!(editors(&mut app), typed);
    reply(
        &outbox,
        encode_result(&applicants_page(Some(advertised(10))))
            .map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    assert_eq!(editors(&mut app), typed);
    let invalid = ["40", "90", "x", "5", "2", "1", "2", "7", "12"];
    listing_texts(&mut app, invalid);
    click(&mut app, Kind::Publish(Vec::new()))?;
    assert!(rx.try_recv().is_err());
    app.update();
    assert!(shown(&mut app, tr!("Enter a valid team listing")));
    assert_eq!(editors(&mut app), invalid.map(String::from));
    listing_texts(&mut app, draft);
    click(&mut app, Kind::Publish(Vec::new()))?;
    command(&mut rx)?;
    app.update();
    assert!(shown(&mut app, tr!("Waiting for the server")));
    assert_eq!(editors(&mut app), typed);
    leader_reply(&outbox, oteryn_session::CommandStatus::Rejected);
    app.update();
    assert!(shown(
        &mut app,
        tr!("Advertisement refused or invalid reply")
    ));
    assert_eq!(editors(&mut app), typed);
    Ok(())
}
#[test]
fn manage_team_renders_in_the_selected_language() -> Result {
    use oteryn_client_ui::localization::{Language, translate, with_language};
    let (mut app, mut rx, outbox) = fixture(true)?;
    // Systems run on the test thread, where the language override applies.
    app.edit_schedule(Update, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    outbox.bevy_team_leader_selected(true);
    app.update();
    click(&mut app, Kind::Applicants(false))?;
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&applicants_page(None)).map_err(|error| format!("{error:?}"))?,
    );
    with_language(Language::De, || app.update());
    for source in [
        "Social — Manage Team",
        "Join Team",
        "Players who applied to or were invited to your team. Select one to decide.",
        "Decline",
        "Your team listing",
        "Vocation mask (1-255, at least one vocation)",
        "Team size",
        "Party members join (0 or 1)",
        "Activity (1 boss, 2 hunt, 3 quest)",
        "Boss, hunt or quest id",
        "Hunting area (hunts only)",
        "Minimum level",
        "Publish listing",
        "Remove listing",
        "Close",
    ] {
        let german = translate(Language::De, source);
        assert_ne!(german, source);
        assert!(shown(&mut app, german), "{german}");
        assert!(!shown(&mut app, source), "{source}");
    }
    Ok(())
}

#[test]
fn acknowledgement_preserves_later_edits() -> Result {
    let (mut app, mut rx, outbox) = manage_team(Some(advertised(10)))?;
    listing_texts(&mut app, ["20", "100", "15", "4", "3", "0", "1", "1", ""]);
    click(&mut app, Kind::Publish(Vec::new()))?;
    command(&mut rx)?;
    app.update();
    listing_texts(&mut app, ["21", "100", "15", "4", "3", "0", "1", "1", ""]);
    leader_reply(&outbox, oteryn_session::CommandStatus::Accepted);
    app.update();
    assert_eq!(editors(&mut app)[0], "21");
    Ok(())
}

#[test]
fn publish_fences_pending_paste() -> Result {
    let (mut app, mut rx, _) = manage_team(Some(advertised(10)))?;
    let clipboard = std::sync::Arc::new(std::sync::Mutex::new(None));
    let field = listing_editor(&mut app)?;
    app.world_mut()
        .get_mut::<EditableText>(field)
        .ok_or("minimum level")?
        .queue_edit(bevy::text::TextEdit::SelectAll);
    app.update();
    app.world_mut()
        .get_mut::<EditableText>(field)
        .ok_or("minimum level")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(clipboard.clone()));
    click(&mut app, Kind::Publish(Vec::new()))?;
    assert!(rx.try_recv().is_err());
    app.update();
    assert!(rx.try_recv().is_err());
    *clipboard.lock().map_err(|_| "clipboard lock")? = Some(Ok("32".into()));
    app.update();
    let sent = command(&mut rx)?;
    assert_eq!(
        oteryn_client_ui::team_finder_leader::decode_upsert(&sent.payload)
            .map_err(|e| format!("{e:?}"))?
            .min_level,
        32
    );
    app.update();
    assert!(rx.try_recv().is_err());
    Ok(())
}

pub(super) fn listing_editor(app: &mut App) -> Result<Entity> {
    app.world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| matches!(field, Field::Listing(0)).then_some(entity))
        .ok_or("minimum level".into())
}

#[test]
fn fast_withdrawal_clears_the_submitted_draft() -> Result {
    let (mut app, mut rx, outbox) = manage_team(Some(advertised(10)))?;
    listing_texts(&mut app, ["20", "100", "15", "4", "3", "0", "1", "1", ""]);
    click(&mut app, Kind::Withdraw)?;
    assert_eq!(command(&mut rx)?.command_type, 391);
    leader_reply(&outbox, oteryn_session::CommandStatus::Accepted);
    app.update();
    assert_eq!(editors(&mut app), <[String; 9]>::default());
    assert!(!withdraw_enabled(&mut app)?);
    Ok(())
}

#[test]
fn selection_and_acknowledgement_preserve_queued_editor_input() -> Result {
    let (mut app, mut rx, outbox) = manage_team(Some(advertised(10)))?;
    let field = listing_editor(&mut app)?;
    let mut editor = app
        .world_mut()
        .get_mut::<EditableText>(field)
        .ok_or("minimum level")?;
    editor.queue_edit(bevy::text::TextEdit::SelectAll);
    editor.queue_edit(bevy::text::TextEdit::Insert("20".into()));
    click(&mut app, Kind::Select(1))?;
    app.update();
    let edited = editors(&mut app)[0].clone();
    assert_eq!(edited, "20");
    click(&mut app, Kind::Publish(Vec::new()))?;
    command(&mut rx)?;
    app.update();
    let field = listing_editor(&mut app)?;
    let mut editor = app
        .world_mut()
        .get_mut::<EditableText>(field)
        .ok_or("minimum level")?;
    editor.queue_edit(bevy::text::TextEdit::SelectAll);
    editor.queue_edit(bevy::text::TextEdit::Insert("21".into()));
    leader_reply(&outbox, oteryn_session::CommandStatus::Accepted);
    app.update();
    assert_eq!(editors(&mut app)[0], "21");
    Ok(())
}

#[test]
fn deferred_publish_survives_applicants_reply() -> Result {
    let (mut app, mut rx, outbox) = manage_team(Some(advertised(10)))?;
    click(&mut app, Kind::Applicants(false))?;
    command(&mut rx)?;
    app.update();
    let field = listing_editor(&mut app)?;
    let mut editor = app
        .world_mut()
        .get_mut::<EditableText>(field)
        .ok_or("minimum level")?;
    editor.queue_edit(bevy::text::TextEdit::SelectAll);
    editor.queue_edit(bevy::text::TextEdit::Insert("32".into()));
    click(&mut app, Kind::Publish(Vec::new()))?;
    assert!(rx.try_recv().is_err());
    reply(
        &outbox,
        encode_result(&applicants_page(Some(advertised(10))))
            .map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    assert_eq!(
        oteryn_client_ui::team_finder_leader::decode_upsert(&command(&mut rx)?.payload)
            .map_err(|error| format!("{error:?}"))?
            .min_level,
        32
    );
    Ok(())
}

#[test]
fn selection_preserves_a_search_paste_that_completes_later() -> Result {
    let (mut app, mut rx, outbox) = fixture(true)?;
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&page(MemberStatus::None)).map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    let name = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| matches!(field, Field::Name).then_some(entity))
        .ok_or("name editor")?;
    let clipboard = std::sync::Arc::new(std::sync::Mutex::new(None));
    app.world_mut()
        .get_mut::<EditableText>(name)
        .ok_or("name editor")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(clipboard.clone()));
    click(&mut app, Kind::Select(1))?;
    app.update();
    *clipboard.lock().map_err(|_| "clipboard lock")? = Some(Ok("second".into()));
    app.update();
    click(&mut app, Kind::Filter(TeamFilter::default()))?;
    let panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .discovery_panel()
        .teams;
    assert_eq!(panel.filter.name, "second");
    Ok(())
}

#[test]
fn regression_publish_survives_applicants_reply_before_projection() -> Result {
    listing_action_after_applicants_reply(Kind::Publish(Vec::new()), 390)
}

#[test]
fn regression_withdraw_survives_applicants_reply_before_projection() -> Result {
    listing_action_after_applicants_reply(Kind::Withdraw, 391)
}

fn listing_action_after_applicants_reply(kind: Kind, command_type: u32) -> Result {
    let (mut app, mut rx, outbox) = manage_team(Some(advertised(10)))?;
    click(&mut app, Kind::Applicants(false))?;
    command(&mut rx)?;
    app.update();
    reply(
        &outbox,
        encode_result(&applicants_page(Some(advertised(10))))
            .map_err(|error| format!("{error:?}"))?,
    );
    click(&mut app, kind)?;
    assert_eq!(command(&mut rx)?.command_type, command_type);
    Ok(())
}

#[test]
fn regression_rejected_equivalent_publish_keeps_raw_draft() -> Result {
    let (mut app, mut rx, outbox) = manage_team(Some(advertised(10)))?;
    listing_texts(&mut app, ["010", "100", "15", "4", "3", "0", "1", "1", ""]);
    click(&mut app, Kind::Publish(Vec::new()))?;
    command(&mut rx)?;
    leader_reply(&outbox, oteryn_session::CommandStatus::Rejected);
    app.update();
    assert!(shown(
        &mut app,
        tr!("Advertisement refused or invalid reply")
    ));
    assert_eq!(editors(&mut app)[0], "010");
    Ok(())
}

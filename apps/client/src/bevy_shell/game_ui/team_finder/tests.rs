use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::ui::{InteractionDisabled, UiTransform};
use oteryn_client_runtime::server_mechanic::{ServerMechanicCommand, ServerMechanicResponse};
use oteryn_client_ui::team_finder::*;
use oteryn_client_ui::team_finder_leader::{LeaderListing, TeamTarget};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture(selected: bool) -> Result<Fixture> {
    fixture_with_layout(selected, false)
}
fn fixture_with_layout(selected: bool, layout: bool) -> Result<Fixture> {
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
fn click(app: &mut App, kind: Kind) -> Result {
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
fn command(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<ServerMechanicCommand> {
    match rx.try_recv()? {
        PlayCommand::ServerMechanic(c) => Ok(c),
        _ => Err("wrong command".into()),
    }
}
fn reply(outbox: &Outbox, payload: Vec<u8>) {
    outbox.bevy_team_response(&ServerMechanicResponse {
        command_id: 1,
        status: oteryn_session::CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    });
}
fn page(status: MemberStatus) -> TeamFinderResult {
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
fn shown(app: &mut App, value: &str) -> bool {
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
fn advertised(min_level: u16) -> LeaderListing {
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
fn applicants_page(listing: Option<LeaderListing>) -> TeamFinderResult {
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
fn manage_team_with_layout(listing: Option<LeaderListing>, layout: bool) -> Result<Fixture> {
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

fn listing_editor(app: &mut App) -> Result<Entity> {
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

fn browsing_reply_during_preedit() -> Result<(App, Entity)> {
    use bevy::window::Ime;
    let (mut app, _, outbox) = pending_browse()?;
    let name = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| matches!(field, Field::Name).then_some(entity))
        .ok_or("name editor")?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(name, bevy::input_focus::FocusCause::Pressed);
    complete_browse(&outbox)?;
    app.world_mut().write_message(Ime::Preedit {
        window: Entity::PLACEHOLDER,
        value: "り".into(),
        cursor: None,
    });
    app.update();
    Ok((app, name))
}

fn assert_idle_composition(app: &App, name: Entity) -> Result {
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(name));
    let editor = app.world().get::<EditableText>(name).ok_or("name editor")?;
    assert!(editor.is_composing());
    assert!(editor.pending_edits.is_empty());
    Ok(())
}

#[test]
fn regression_browse_reply_keeps_ime_commit_destination() -> Result {
    use bevy::window::Ime;

    let (mut app, name) = browsing_reply_during_preedit()?;
    assert_idle_composition(&app, name)?;
    assert_browse_projected(&mut app);
    app.update();
    assert_idle_composition(&app, name)?;
    app.world_mut().write_message(Ime::Commit {
        window: Entity::PLACEHOLDER,
        value: "李".into(),
    });
    app.update();
    app.update();
    click(&mut app, Kind::Filter(TeamFilter::default()))?;
    let owner = app.world().non_send::<HostOwner>();
    let panel = owner.play.as_ref().ok_or("play")?.link.discovery_panel();
    assert_eq!(panel.teams.filter.name, "李");
    Ok(())
}

#[test]
fn regression_close_survives_browse_reply_during_ime_composition() -> Result {
    let (mut app, name) = browsing_reply_during_preedit()?;
    app.update();
    assert_idle_composition(&app, name)?;
    let close = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == Kind::Close).then_some(entity))
        .ok_or("close button")?;
    let location = press_button(&mut app, close)?;
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(close));
    for _ in 0..4 {
        app.update();
    }
    release_button(&mut app, location);
    assert!(app.world().resource::<State>().root.is_none());
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    Ok(())
}

#[test]
fn regression_selection_has_pickable_controls_in_the_same_frame() -> Result {
    let (mut app, mut rx, outbox) = fixture_with_layout(true, true)?;
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&page(MemberStatus::None)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    let previous_root = app.world().resource::<State>().root;
    click(&mut app, Kind::Select(1))?;
    app.update();
    assert!(shown(&mut app, "> Second"));
    assert_eq!(app.world().resource::<State>().root, previous_root);
    let (close, node, visibility) = app
        .world_mut()
        .query::<(Entity, &Control, &ComputedNode, &InheritedVisibility)>()
        .iter(app.world())
        .find_map(|(entity, control, node, visibility)| {
            (control.kind == Kind::Close).then_some((entity, node, visibility))
        })
        .ok_or("close button")?;
    assert!(
        node.size().cmpgt(Vec2::ZERO).all(),
        "close size: {:?}",
        node.size()
    );
    assert!(visibility.get(), "retained close button is hidden");

    let location = press_button(&mut app, close)?;
    release_button(&mut app, location);
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

fn press_button(app: &mut App, button: Entity) -> Result<bevy::picking::pointer::Location> {
    use super::super::pointer_layout;
    use bevy::{
        camera::RenderTarget,
        picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
        window::PrimaryWindow,
    };

    let primary = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())?;
    let location = Location {
        target: RenderTarget::default()
            .normalize(Some(primary))
            .ok_or("primary target")?,
        position: pointer_layout::center(app, button)?,
    };
    let computed = app.world().get::<ComputedNode>(button).copied();
    let visible = app.world().get::<InheritedVisibility>(button).copied();
    for action in [
        PointerAction::Move { delta: Vec2::ZERO },
        PointerAction::Press(PointerButton::Primary),
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
    }
    app.update();
    assert!(
        app.world().get::<bevy::ui::Pressed>(button).is_some(),
        "button {button} at {:?}, size {:?}, visibility {:?}, hover {:?}",
        location.position,
        computed.map(|node| node.size()),
        visible,
        app.world()
            .resource::<bevy::picking::hover::HoverMap>()
            .get(&PointerId::Mouse)
    );
    Ok(location)
}

fn release_button(app: &mut App, location: bevy::picking::pointer::Location) {
    use bevy::picking::pointer::{PointerAction, PointerButton, PointerId, PointerInput};

    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location,
        PointerAction::Release(PointerButton::Primary),
    ));
    app.update();
}

fn pending_browse() -> Result<Fixture> {
    let (mut app, mut rx, outbox) = fixture_with_layout(true, true)?;
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    app.update();
    assert!(!shown(&mut app, "First"));
    Ok((app, rx, outbox))
}

fn complete_browse(outbox: &Outbox) -> Result {
    reply(
        outbox,
        encode_result(&page(MemberStatus::None)).map_err(|error| format!("{error:?}"))?,
    );
    Ok(())
}

fn assert_browse_projected(app: &mut App) {
    assert!(shown(app, "> First"), "browse reply remained unprojected");
    assert!(
        app.world()
            .resource::<State>()
            .snapshot
            .as_ref()
            .is_some_and(|(_, panel, _)| panel.pending.is_none())
    );
}

fn close_button(app: &mut App) -> Result<Entity> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == Kind::Close).then_some(entity))
        .ok_or_else(|| "close button".into())
}

fn primary_window(app: &mut App) -> Result<Entity> {
    Ok(app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())?)
}

#[test]
fn regression_pending_browse_projects_during_mouse_press() -> Result {
    let (mut app, _, outbox) = pending_browse()?;
    // Close must remain under the pointer when the user has moved the dialog.
    let root = app.world().resource::<State>().root.ok_or("team modal")?;
    let mut dialogs = app.world_mut().query_filtered::<
        (&ChildOf, &mut UiTransform),
        With<crate::bevy_shell::window_drag::DraggableDialog>,
    >();
    let (_, mut transform) = dialogs
        .iter_mut(app.world_mut())
        .find(|(parent, _)| parent.parent() == root)
        .ok_or("team dialog")?;
    transform.translation = bevy::ui::Val2::px(160., 20.);
    app.update();
    let close = close_button(&mut app)?;
    let location = press_button(&mut app, close)?;
    complete_browse(&outbox)?;
    app.update();
    assert_browse_projected(&mut app);
    release_button(&mut app, location);
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

fn touch(app: &mut App, phase: bevy::input::touch::TouchPhase, position: Vec2) -> Result {
    use bevy::{input::touch::TouchInput, window::WindowEvent};
    let event = TouchInput {
        phase,
        position,
        window: primary_window(app)?,
        force: None,
        id: 1,
    };
    // Winit sends both the unified and typed messages.
    app.world_mut()
        .write_message(WindowEvent::TouchInput(event));
    app.world_mut().write_message(event);
    app.update();
    Ok(())
}

#[test]
fn regression_pending_browse_projects_during_touch_press() -> Result {
    use bevy::input::touch::TouchPhase;
    let (mut app, _, outbox) = pending_browse()?;
    let close = close_button(&mut app)?;
    let position = super::super::pointer_layout::center(&app, close)?;
    touch(&mut app, TouchPhase::Started, position)?;
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_some());
    complete_browse(&outbox)?;
    app.update();
    assert_browse_projected(&mut app);
    touch(&mut app, TouchPhase::Ended, position)?;
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

#[test]
fn regression_pending_browse_projects_after_touch_cancel() -> Result {
    use bevy::{input::touch::TouchPhase, picking::pointer::PointerId};
    let (mut app, _, outbox) = pending_browse()?;
    let close = close_button(&mut app)?;
    let position = super::super::pointer_layout::center(&app, close)?;
    touch(&mut app, TouchPhase::Started, position)?;
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_some());
    touch(&mut app, TouchPhase::Canceled, position)?;
    assert!(
        !app.world_mut()
            .query::<&PointerId>()
            .iter(app.world())
            .any(|id| *id == PointerId::Touch(1))
    );
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_none());
    complete_browse(&outbox)?;
    app.update();
    assert_browse_projected(&mut app);
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_none());
    let images = app
        .world()
        .get::<oteryn_client_ui_bevy::ButtonImages>(close)
        .ok_or("button images")?;
    assert_eq!(
        app.world()
            .get::<ImageNode>(close)
            .ok_or("button image")?
            .image,
        images.up
    );
    Ok(())
}

#[test]
fn regression_tab_switch_keeps_keyboard_edit() -> Result {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    let (mut app, mut rx, outbox) = fixture_with_layout(true, true)?;
    let manage = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "Team Finder: Manage Team").then_some(entity))
        .ok_or("manage tab")?;
    let location = press_button(&mut app, manage)?;
    let window = primary_window(&mut app)?;
    let name = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| (*field == Field::Name).then_some(entity))
        .ok_or("name editor")?;
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Tab,
        logical_key: Key::Tab,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(name));
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyA,
        logical_key: Key::Character("a".into()),
        state: ButtonState::Pressed,
        text: Some("a".into()),
        repeat: false,
        window,
    });
    release_button(&mut app, location);
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&applicants_page(None)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    complete_browse(&outbox)?;
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(name)
            .ok_or("retained name editor")?
            .value(),
        "a"
    );
    Ok(())
}

#[test]
fn regression_pending_browse_projects_with_keyboard_edit() -> Result {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    let (mut app, _, outbox) = pending_browse()?;
    let name = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| matches!(field, Field::Name).then_some(entity))
        .ok_or("name editor")?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(name, bevy::input_focus::FocusCause::Pressed);
    let window = primary_window(&mut app)?;
    complete_browse(&outbox)?;
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyA,
        logical_key: Key::Character("a".into()),
        state: ButtonState::Pressed,
        text: Some("a".into()),
        repeat: false,
        window,
    });
    app.update();
    assert_browse_projected(&mut app);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(name));
    assert_eq!(
        app.world()
            .get::<EditableText>(name)
            .ok_or("name editor")?
            .value()
            .to_string(),
        "a"
    );
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: Key::Escape,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

#[test]
fn regression_pending_browse_projects_after_window_focus_loss() -> Result {
    use bevy::{
        input::keyboard::KeyboardFocusLost,
        window::{WindowEvent, WindowFocused},
    };
    let (mut app, _, outbox) = pending_browse()?;
    let close = close_button(&mut app)?;
    press_button(&mut app, close)?;
    let window = primary_window(&mut app)?;
    app.world_mut()
        .get_mut::<Window>(window)
        .ok_or("window")?
        .focused = false;
    let event = WindowFocused {
        window,
        focused: false,
    };
    app.world_mut()
        .write_message(WindowEvent::WindowFocused(event.clone()));
    app.world_mut().write_message(event);
    app.world_mut().write_message(KeyboardFocusLost);
    app.update();
    complete_browse(&outbox)?;
    app.update();
    assert_browse_projected(&mut app);
    Ok(())
}

#[test]
fn regression_touch_cancel_after_browse_reply_releases_close() -> Result {
    use bevy::input::touch::TouchPhase;

    let (mut app, _, outbox) = pending_browse()?;
    let close = close_button(&mut app)?;
    let position = super::super::pointer_layout::center(&app, close)?;
    touch(&mut app, TouchPhase::Started, position)?;
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_some());
    complete_browse(&outbox)?;
    app.update();
    assert_browse_projected(&mut app);
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_some());
    touch(&mut app, TouchPhase::Canceled, position)?;
    for _ in 0..4 {
        app.update();
    }
    assert!(app.world().resource::<State>().root.is_some());
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_none());
    let images = app
        .world()
        .get::<oteryn_client_ui_bevy::ButtonImages>(close)
        .ok_or("close images")?;
    assert_eq!(
        app.world()
            .get::<ImageNode>(close)
            .ok_or("close image")?
            .image,
        images.up
    );
    Ok(())
}

#[test]
fn regression_publish_press_survives_applicants_refresh() -> Result {
    let listing = Some(advertised(10));
    let (mut app, mut rx, outbox) = manage_team_with_layout(listing, true)?;
    click(&mut app, Kind::Applicants(false))?;
    assert_eq!(command(&mut rx)?.command_type, 444);
    app.update();
    let publish = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| matches!(control.kind, Kind::Publish(_)).then_some(entity))
        .ok_or("Publish listing")?;
    let location = press_button(&mut app, publish)?;
    reply(
        &outbox,
        encode_result(&applicants_page(listing)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    assert!(app.world().get::<InteractionDisabled>(publish).is_none());
    release_button(&mut app, location);
    assert_eq!(command(&mut rx)?.command_type, 390);
    assert!(rx.try_recv().is_err());
    Ok(())
}

#[derive(Resource, Default)]
struct CopySelection(Option<Option<String>>);

// Observe the selection at Bevy's clipboard boundary without a desktop clipboard.
fn record_copy_selection(
    mut copied: ResMut<CopySelection>,
    editors: Query<(&Field, &EditableText)>,
) {
    for (field, text) in &editors {
        if *field == Field::Listing(0) && text.pending_edits.contains(&bevy::text::TextEdit::Copy) {
            copied.0 = Some(text.editor().selected_text().map(str::to_owned));
        }
    }
}

#[test]
fn regression_copy_and_tab_use_selection_before_listing_reply_sync() -> Result {
    use bevy::{
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput},
        },
        text::TextEdit,
    };

    let (mut app, mut rx, outbox) = manage_team_with_layout(Some(advertised(10)), true)?;
    app.init_resource::<CopySelection>().add_systems(
        PostUpdate,
        record_copy_selection.before(bevy::text::EditableTextSystems),
    );
    click(&mut app, Kind::Applicants(false))?;
    command(&mut rx)?;
    app.update();
    let editor = listing_editor(&mut app)?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, bevy::input_focus::FocusCause::Pressed);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("level editor")?
        .queue_edit(TextEdit::SelectAll);
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("level editor")?
            .editor()
            .selected_text(),
        Some("10")
    );
    reply(
        &outbox,
        encode_result(&applicants_page(Some(advertised(20))))
            .map_err(|error| format!("{error:?}"))?,
    );
    let window = primary_window(&mut app)?;
    for (key_code, logical_key) in [(KeyCode::Copy, Key::Copy), (KeyCode::Tab, Key::Tab)] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
    }
    app.update();
    assert_ne!(app.world().resource::<InputFocus>().get(), Some(editor));
    assert_eq!(
        app.world().resource::<CopySelection>().0,
        Some(Some("10".into()))
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("level editor")?
            .value(),
        "20"
    );
    Ok(())
}

#[test]
fn regression_next_page_resets_scroll_after_same_page_refresh() -> Result {
    use bevy::{
        camera::RenderTarget,
        input::{mouse::MouseScrollUnit, touch::TouchPhase},
        picking::pointer::{Location, PointerAction, PointerId, PointerInput},
        ui::ScrollPosition,
        ui_widgets::ScrollArea,
    };

    let (mut app, mut rx, outbox) = pending_browse()?;
    let browse_page = |first| TeamFinderResult::Browse {
        listings: (first..first + 100)
            .map(|leader_handle| BrowseListing {
                leader_handle,
                name: format!("Team {leader_handle}"),
                listing: advertised(10),
                occupied_slots: 1,
                status: MemberStatus::None,
            })
            .collect(),
        next: Some(first + 99),
    };
    let encode = |first| encode_result(&browse_page(first)).map_err(|error| format!("{error:?}"));
    reply(&outbox, encode(1)?);
    app.update();
    let list = app
        .world_mut()
        .query_filtered::<(Entity, &ComputedNode), With<ScrollArea>>()
        .iter(app.world())
        .find_map(|(entity, node)| (node.size().y > 0.).then_some(entity))
        .ok_or("visible team list")?;
    let window = primary_window(&mut app)?;
    let location = Location {
        target: RenderTarget::default()
            .normalize(Some(window))
            .ok_or("window target")?,
        position: super::super::pointer_layout::center(&app, list)?,
    };
    for action in [
        PointerAction::Move { delta: Vec2::ZERO },
        PointerAction::Scroll {
            unit: MouseScrollUnit::Pixel,
            x: 0.,
            y: -500.,
            phase: TouchPhase::Moved,
        },
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
    }
    app.update();
    let offset = |app: &App| app.world().get::<ScrollPosition>(list).map(|value| value.y);
    assert_eq!(offset(&app), Some(500.));
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    app.update();
    reply(&outbox, encode(1)?);
    app.update();
    assert_eq!(offset(&app), Some(500.));
    let next = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == Kind::Browse(true)).then_some(entity))
        .ok_or("Next page")?;
    let location = press_button(&mut app, next)?;
    release_button(&mut app, location);
    let query = command(&mut rx)?;
    assert_eq!(
        decode_query(&query.payload).map_err(|error| format!("{error:?}"))?,
        TeamFinderQuery::Browse { after: Some(100) }
    );
    app.update();
    reply(&outbox, encode(101)?);
    app.update();
    assert!(shown(&mut app, "> Team 101"));
    assert_eq!(offset(&app), Some(0.));
    Ok(())
}

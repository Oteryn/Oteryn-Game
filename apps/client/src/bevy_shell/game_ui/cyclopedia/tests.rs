use super::cyclopedia::{self, Control, Kind, Tab};
use crate::{
    bevy_shell::owner::{ActivePlay, HostOwner, Phase},
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_runtime::{
    cyclopedia_character::{CharacterTab, DeathDetails, GeneralStats, TabExtension, TabView},
    raw_feature::{CommandStatus, GatedCommandResult},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_cyclopedia_initialize(true, true, true);
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::modal::install(&mut app);
    cyclopedia::install(&mut app);
    app.update();
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(24)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    app.update();
    Ok((app, commands, outbox))
}
fn command(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<oteryn_client_runtime::raw_feature::FeatureCommand, Box<dyn std::error::Error>> {
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("expected feature command".into());
    };
    assert_eq!(command.capability, 199);
    Ok(command)
}
fn response(outbox: &Outbox, payload: Vec<u8>) {
    outbox.bevy_cyclopedia_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    });
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("control absent")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn texts(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect()
}
#[test]
fn cyclopedia_tab_requests_data() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    assert_eq!(command(&mut commands)?.payload, [1, 0, 0, 20]);
    let expected_tabs: [(Tab, &[u8]); 12] = [
        (Tab::General, &[1, 0, 0, 20]),
        (Tab::Character(CharacterTab::Offence), &[8, 0, 0, 20]),
        (Tab::Character(CharacterTab::Defence), &[9, 0, 0, 20]),
        (Tab::Character(CharacterTab::Misc), &[10, 0, 0, 20]),
        (Tab::Character(CharacterTab::RecentDeaths), &[2, 0, 0, 20]),
        (Tab::Character(CharacterTab::RecentPvpKills), &[3, 0, 0, 20]),
        (Tab::Character(CharacterTab::Achievements), &[128, 0]),
        (Tab::Items, &[130, 0, 0]),
        (Tab::Character(CharacterTab::Appearances), &[129, 0, 0]),
        (Tab::Character(CharacterTab::StoreSummary), &[6, 0, 0, 20]),
        (Tab::Character(CharacterTab::Inspection), &[132]),
        (Tab::Badges, &[132]),
    ];
    for (tab, expected) in expected_tabs {
        response(&outbox, vec![]); // Refusal/invalid response releases the existing pending owner.
        app.update();
        click(&mut app, Kind::Tab(tab))?;
        let actual = command(&mut commands)?;
        assert_eq!(actual.kind, 388);
        assert_eq!(actual.payload, expected);
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn cyclopedia_renders_general_stats() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    assert!(!texts(&mut app).iter().any(|t| t == "Level: 42"));
    let payload = TabView::GeneralStats(GeneralStats {
        level: 42,
        experience: 987654,
        magic_level: 9,
        skills: [11, 12, 13, 14, 15, 16, 17],
        stamina_minutes: 125,
        offline_training_minutes: 60,
        last_vitals: Some((444, 222)),
    })
    .encode_extended(&TabExtension {
        vocation_id: Some(6),
        deaths: None,
    })
    .map_err(|_| "encode")?;
    response(&outbox, payload);
    app.update();
    let rows = texts(&mut app);
    for expected in [
        "Level: 42",
        "Experience: 987654",
        "Health (last saved): 444",
        "Mana (last saved): 222",
        "Stamina: 2:05",
        "Vocation: Knight",
    ] {
        assert!(
            rows.iter().any(|row| row == expected),
            "missing {expected}: {rows:?}"
        );
    }
    Ok(())
}
#[test]
fn cyclopedia_death_rows() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    response(&outbox, vec![]);
    app.update();
    click(
        &mut app,
        Kind::Tab(Tab::Character(CharacterTab::RecentDeaths)),
    )?;
    assert_eq!(command(&mut commands)?.payload, [2, 0, 0, 20]);
    response(
        &outbox,
        TabView::RecentDeaths(vec![(1700000000123, "Died by a dragon".into())])
            .encode_extended(&TabExtension {
                vocation_id: None,
                deaths: Some(vec![DeathDetails {
                    level: 41,
                    killer: "dragon".into(),
                    most_damage: "dragon".into(),
                    unjustified: false,
                }]),
            })
            .map_err(|_| "encode deaths")?,
    );
    app.update();
    let rows = texts(&mut app);
    assert!(
        rows.iter().any(|row| row.contains("1700000000123")
            && row.contains("Died by a dragon")
            && row.contains("dragon")
            && row.contains("41")),
        "{rows:?}"
    );
    Ok(())
}

#[test]
fn cyclopedia_pending_and_closed_controls_cannot_send() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    click(
        &mut app,
        Kind::Tab(Tab::Character(CharacterTab::RecentDeaths)),
    )?;
    assert!(commands.try_recv().is_err());
    response(&outbox, vec![]);
    app.update();
    click(&mut app, Kind::Close)?;
    assert!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .next()
            .is_none()
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn cyclopedia_renders_item_and_appearance_names_from_replies() -> TestResult {
    use oteryn_client_runtime::cyclopedia_character::{gallery, items};
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    response(&outbox, vec![]);
    app.update();
    click(&mut app, Kind::Tab(Tab::Items))?;
    assert_eq!(command(&mut commands)?.payload, items::request(0));
    response(
        &outbox,
        items::Page {
            offset: 0,
            total: 1,
            rows: vec![items::Row {
                definition: "oteryn:item/sword".into(),
                quantities: [1, 2, 3, 4],
            }],
        }
        .encode()
        .map_err(|_| "encode items")?,
    );
    app.update();
    assert!(
        texts(&mut app)
            .iter()
            .any(|row| row.contains("oteryn:item/sword")
                && row.contains("Inventory: 1")
                && row.contains("Inbox: 4"))
    );
    click(
        &mut app,
        Kind::Tab(Tab::Character(CharacterTab::Appearances)),
    )?;
    assert_eq!(command(&mut commands)?.payload, gallery::request(0));
    response(
        &outbox,
        gallery::Page {
            offset: 0,
            total: 1,
            entries: vec![gallery::Entry {
                kind: gallery::Kind::Mount,
                key: "oteryn:mount/bear".into(),
                look_type: 12,
            }],
        }
        .encode()
        .map_err(|_| "encode gallery")?,
    );
    app.update();
    assert!(
        texts(&mut app)
            .iter()
            .any(|row| row.contains("oteryn:mount/bear") && row.contains("Mount"))
    );
    Ok(())
}

#[test]
fn cyclopedia_collects_achievement_pages() -> TestResult {
    use oteryn_client_runtime::{
        account_achievements::{AccountAchievementRow, AccountAchievementsResult},
        cyclopedia_character::achievements::{AchievementPage, AchievementRequest},
    };
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    response(&outbox, vec![]);
    app.update();
    click(
        &mut app,
        Kind::Tab(Tab::Character(CharacterTab::Achievements)),
    )?;
    assert_eq!(
        command(&mut commands)?.payload,
        AchievementRequest {
            page: 0,
            target: None
        }
        .encode()
    );
    for (page, name) in [(0, "First discovery"), (1, "Second discovery")] {
        response(
            &outbox,
            AchievementPage {
                character: [7; 16],
                name: "Explorer".into(),
                page: AccountAchievementsResult {
                    total_points: 65,
                    fact_count: 65,
                    page,
                    has_more: page == 0,
                    rows: (0..if page == 0 { 64 } else { 1 })
                        .map(|index| AccountAchievementRow {
                            key: format!("oteryn:achievement/discovery_{page}_{index}"),
                            name: name.into(),
                            description: "A confirmed discovery".into(),
                            grade: 1,
                            points: 1,
                            earned_at: 1700000000123,
                            secret: false,
                        })
                        .collect(),
                },
            }
            .encode()
            .map_err(|_| "encode achievements")?,
        );
        app.update();
        if page == 0 {
            assert_eq!(
                command(&mut commands)?.payload,
                AchievementRequest {
                    page: 1,
                    target: None
                }
                .encode()
            );
        }
    }
    let rows = texts(&mut app);
    for name in ["First discovery", "Second discovery"] {
        assert!(rows.iter().any(|row| row.contains(name)), "{rows:?}");
    }
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn cyclopedia_shortcut_route_does_not_open_the_unported_blessings_window() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    response(&outbox, vec![]);
    app.update();
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(69)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Alt)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    let owner = app.world().non_send::<HostOwner>();
    assert!(
        !owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .cyclopedia_character_panel()
            .blessings
            .open
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn cyclopedia_reopened_while_pending_sends_its_tab_once_the_builder_accepts() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    assert_eq!(command(&mut commands)?.payload, [1, 0, 0, 20]);
    // Close and reopen before the reply: the retained builder refuses the first request.
    {
        let owner = app.world().non_send::<HostOwner>();
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .window_toggle("0199-cyclopedia-character-open", &[]);
    }
    app.update();
    let reopen = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(24)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[reopen], false);
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "builder must refuse while pending"
    );
    response(&outbox, vec![]);
    app.update();
    assert_eq!(command(&mut commands)?.payload, [1, 0, 0, 20]);
    assert!(commands.try_recv().is_err());
    Ok(())
}

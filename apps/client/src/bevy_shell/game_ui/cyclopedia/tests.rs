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
    let expected_tabs: [(Tab, &[u8]); 13] = [
        (Tab::General, &[1, 0, 0, 20]),
        (Tab::Character(CharacterTab::Offence), &[8, 0, 0, 20]),
        (Tab::Character(CharacterTab::Defence), &[9, 0, 0, 20]),
        (Tab::Character(CharacterTab::Misc), &[10, 0, 0, 20]),
        (Tab::Character(CharacterTab::RecentDeaths), &[2, 0, 0, 20]),
        (Tab::Character(CharacterTab::RecentPvpKills), &[3, 0, 0, 20]),
        (Tab::Character(CharacterTab::Achievements), &[128, 0]),
        (Tab::Items, &[130, 0, 0]),
        (Tab::Character(CharacterTab::Appearances), &[129, 0, 0]),
        (Tab::Familiars, &[129, 0, 0]),
        (Tab::Character(CharacterTab::StoreSummary), &[6, 0, 0, 20]),
        (Tab::Character(CharacterTab::Inspection), &[132]),
        (Tab::Badges, &[0xf0, 0]),
    ];
    for (tab, expected) in expected_tabs {
        response(&outbox, vec![]); // Refusal/invalid response releases the existing pending owner.
        app.update();
        click(&mut app, Kind::Tab(tab))?;
        let actual = command(&mut commands)?;
        assert_eq!(actual.kind, if tab == Tab::Badges { 387 } else { 388 });
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

#[test]
fn cyclopedia_familiar_page_is_reachable_and_distinguishes_catalogue_from_unlocks() -> TestResult {
    use oteryn_client_runtime::cyclopedia_character::gallery;
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    response(&outbox, vec![]);
    app.update();
    let caption = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find_map(|(entity, text)| (text.0 == "Familiars").then_some(entity))
        .ok_or("Familiars tab absent")?;
    let button = app
        .world()
        .get::<ChildOf>(caption)
        .ok_or("tab parent")?
        .parent();
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert_eq!(command(&mut commands)?.payload, gallery::request(0));
    response(
        &outbox,
        gallery::Page {
            offset: 0,
            total: 2,
            entries: vec![
                gallery::Entry {
                    kind: gallery::Kind::Mount,
                    key: "oteryn:mount/bear".into(),
                    look_type: 12,
                },
                gallery::Entry {
                    kind: gallery::Kind::Familiar,
                    key: "oteryn:familiar/sorcerer".into(),
                    look_type: 13,
                },
            ],
        }
        .encode()
        .map_err(|_| "gallery encode")?,
    );
    app.update();
    let rows = texts(&mut app);
    assert!(
        rows.iter()
            .any(|row| row.contains("oteryn:familiar/sorcerer"))
    );
    assert!(!rows.iter().any(|row| row.contains("oteryn:mount/bear")));
    assert!(rows.iter().any(|row| row.contains("not account unlocks")));
    click(&mut app, Kind::Refresh)?;
    assert_eq!(command(&mut commands)?.payload, gallery::request(0));
    Ok(())
}

#[test]
fn cyclopedia_profile_shows_confirmed_identity_equipment_and_badges_together() -> TestResult {
    use oteryn_client_runtime::cyclopedia_character::{Badge, EquippedItem, profile};
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    response(&outbox, vec![]);
    app.update();
    click(
        &mut app,
        Kind::Tab(Tab::Character(CharacterTab::Inspection)),
    )?;
    assert_eq!(command(&mut commands)?.payload, profile::request(None));
    response(
        &outbox,
        profile::Profile {
            character: [7; 16],
            name: "Explorer".into(),
            badges: vec![Badge::FledgelingHero],
            equipment: vec![EquippedItem {
                slot: 1,
                definition: "oteryn:item/sword".into(),
                quantity: 1,
            }],
        }
        .encode()
        .map_err(|_| "profile encode")?,
    );
    app.update();
    let rows = texts(&mut app);
    assert!(rows.iter().any(|row| row.contains("Explorer")));
    assert!(rows.iter().any(|row| row.contains("oteryn:item/sword")));
    assert!(
        rows.iter()
            .any(|row| row == &format!("Earned badge: {}", Badge::FledgelingHero.name()))
    );
    Ok(())
}

#[test]
fn cyclopedia_badges_tab_queries_visibility_and_toggles_an_earned_badge() -> TestResult {
    use oteryn_client_runtime::cyclopedia_character::{Badge, badge_visibility};
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    response(&outbox, vec![]);
    app.update();
    click(&mut app, Kind::Tab(Tab::Badges))?;
    let query = command(&mut commands)?;
    assert_eq!(query.kind, 387);
    assert_eq!(
        query.payload,
        badge_visibility::Request::Query(None).encode()
    );
    let reply = |hidden: Vec<Badge>| {
        badge_visibility::Reply {
            earned: vec![Badge::FledgelingHero],
            hidden,
        }
        .encode()
        .map_err(|_| "badge reply encode")
    };
    response(&outbox, reply(vec![])?);
    app.update();
    let name = Badge::FledgelingHero.name();
    let rows = texts(&mut app);
    assert!(
        rows.iter()
            .any(|row| row == &format!("{name} — Earned; public")),
        "{rows:?}"
    );
    assert!(
        rows.iter()
            .any(|row| row == &format!("Hide {name} publicly"))
    );
    click(&mut app, Kind::BadgeVisibility(Badge::FledgelingHero.id()))?;
    let toggle = command(&mut commands)?;
    assert_eq!(toggle.kind, 387);
    assert_eq!(
        toggle.payload,
        badge_visibility::Request::Set {
            badge: Badge::FledgelingHero,
            public: false,
        }
        .encode()
    );
    assert!(commands.try_recv().is_err());
    response(&outbox, reply(vec![Badge::FledgelingHero])?);
    app.update();
    let rows = texts(&mut app);
    assert!(
        rows.iter()
            .any(|row| row == &format!("{name} — Earned; hidden publicly")),
        "{rows:?}"
    );
    assert!(
        rows.iter()
            .any(|row| row == &format!("Show {name} publicly"))
    );
    Ok(())
}

#[test]
fn cyclopedia_profile_refusal_removes_previously_confirmed_details() -> TestResult {
    use oteryn_client_runtime::cyclopedia_character::{Badge, profile};
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    response(&outbox, vec![]);
    app.update();
    click(
        &mut app,
        Kind::Tab(Tab::Character(CharacterTab::Inspection)),
    )?;
    command(&mut commands)?;
    response(
        &outbox,
        profile::Profile {
            character: [7; 16],
            name: "Explorer".into(),
            badges: vec![Badge::FledgelingHero],
            equipment: vec![],
        }
        .encode()
        .map_err(|_| "profile encode")?,
    );
    app.update();
    assert!(texts(&mut app).iter().any(|row| row.contains("Explorer")));
    click(&mut app, Kind::Refresh)?;
    command(&mut commands)?;
    outbox.bevy_cyclopedia_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 2,
        status: CommandStatus::Rejected,
        payload: vec![],
    });
    app.update();
    let rows = texts(&mut app);
    assert!(
        !rows
            .iter()
            .any(|row| row.contains("Explorer") || row.contains(Badge::FledgelingHero.name()))
    );
    assert!(
        rows.iter()
            .any(|row| row.contains("refused the profile query"))
    );
    Ok(())
}

#[test]
fn cyclopedia_familiars_browse_shared_pages_and_keep_server_look_ids() -> TestResult {
    use oteryn_client_runtime::cyclopedia_character::gallery;
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    response(&outbox, vec![]);
    app.update();
    click(&mut app, Kind::Tab(Tab::Familiars))?;
    assert_eq!(command(&mut commands)?.payload, gallery::request(0));
    response(
        &outbox,
        gallery::Page {
            offset: 0,
            total: 25,
            entries: (0..24)
                .map(|index| gallery::Entry {
                    kind: gallery::Kind::Outfit,
                    key: format!("outfit/{index}"),
                    look_type: index + 1,
                })
                .collect(),
        }
        .encode()
        .map_err(|_| "first gallery page")?,
    );
    app.update();
    assert!(
        texts(&mut app)
            .iter()
            .any(|row| row.contains("No confirmed familiars on this appearance page"))
    );
    click(&mut app, Kind::Next)?;
    assert_eq!(command(&mut commands)?.payload, gallery::request(24));
    response(
        &outbox,
        gallery::Page {
            offset: 24,
            total: 25,
            entries: vec![gallery::Entry {
                kind: gallery::Kind::Familiar,
                key: "oteryn:familiar/sorcerer".into(),
                look_type: 13,
            }],
        }
        .encode()
        .map_err(|_| "last gallery page")?,
    );
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&super::cyclopedia::artwork::Portrait>()
            .iter(app.world())
            .map(|portrait| portrait.0)
            .collect::<Vec<_>>(),
        [13]
    );
    let next_disabled = app
        .world_mut()
        .query::<(&Control, Option<&bevy::ui::InteractionDisabled>)>()
        .iter(app.world())
        .any(|(control, disabled)| control.kind == Kind::Next && disabled.is_some());
    assert!(next_disabled);
    click(&mut app, Kind::Previous)?;
    assert_eq!(command(&mut commands)?.payload, gallery::request(0));
    Ok(())
}

#[test]
fn srvcat_holdings_search_location_and_pages_use_named_server_results() -> TestResult {
    use oteryn_client_runtime::cyclopedia_character::items::{self, Location, Page, Row, filtered};
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    response(&outbox, Vec::new());
    outbox.bevy_holdings_initialize();
    app.update();
    click(&mut app, Kind::Tab(Tab::Items))?;
    assert_eq!(
        filtered::Query::decode(&command(&mut commands)?.payload)
            .map_err(|e| format!("{e:?}"))?
            .offset,
        0
    );
    let rows = (0..items::PAGE_SIZE)
        .map(|i| Row {
            definition: format!("oteryn:item.a{i:02}"),
            quantities: [1, 0, 2, 0],
        })
        .collect();
    response(
        &outbox,
        filtered::Page {
            holdings: Page {
                offset: 0,
                total: 25,
                rows,
            },
            names: (0..24).map(|i| format!("Item {i}")).collect(),
        }
        .encode()
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    click(&mut app, Kind::Next)?;
    let next =
        filtered::Query::decode(&command(&mut commands)?.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(next.offset, 24);
    let row = Row {
        definition: "oteryn:item.tibia.i3031".into(),
        quantities: [0, 0, 7, 0],
    };
    response(
        &outbox,
        filtered::Page {
            holdings: Page {
                offset: 24,
                total: 25,
                rows: vec![row.clone()],
            },
            names: vec!["gold coin".into()],
        }
        .encode()
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    assert!(
        texts(&mut app)
            .iter()
            .any(|text| text.contains("gold coin | Inventory: 0"))
    );
    let search = app
        .world_mut()
        .query_filtered::<Entity, With<cyclopedia::HoldingsSearch>>()
        .single(app.world())?;
    app.world_mut()
        .get_mut::<bevy::text::EditableText>(search)
        .ok_or("holdings search")?
        .editor_mut()
        .set_text("GoLd");
    click(&mut app, Kind::HoldingsFilter)?;
    let query =
        filtered::Query::decode(&command(&mut commands)?.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (query.offset, query.search.as_str(), query.location),
        (0, "GoLd", None)
    );
    response(
        &outbox,
        filtered::Page {
            holdings: Page {
                offset: 0,
                total: 1,
                rows: vec![row],
            },
            names: vec!["gold coin".into()],
        }
        .encode()
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    click(&mut app, Kind::HoldingsLocation(Some(Location::Stash)))?;
    let query =
        filtered::Query::decode(&command(&mut commands)?.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (query.offset, query.search.as_str(), query.location),
        (0, "GoLd", Some(Location::Stash))
    );
    Ok(())
}

fn named_node(app: &mut App, name: &str) -> Result<(Entity, Node), Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Name, &Node)>()
        .iter(app.world())
        .find_map(|(entity, candidate, node)| {
            (candidate.as_str() == name).then(|| (entity, node.clone()))
        })
        .ok_or_else(|| format!("{name} absent").into())
}
#[test]
fn cyclopedia_window_uses_global_geometry_and_caption() -> TestResult {
    let (mut app, _, _) = fixture()?;
    let (_, window) = named_node(&mut app, "cyclopedia.window")?;
    assert_eq!((window.width, window.height), (px(700.), px(538.)));
    let (tabs, bar) = named_node(&mut app, "cyclopedia.tabs")?;
    assert_eq!(bar.height, px(36.));
    let (_, page) = named_node(&mut app, "cyclopedia.page")?;
    assert_eq!(page.height, px(420.));
    assert!(texts(&mut app).iter().any(|text| text == "Cyclopedia"));
    let active = app
        .world_mut()
        .query_filtered::<(&Node, &ChildOf), With<bevy::ui::Checked>>()
        .iter(app.world())
        .find_map(|(node, parent)| (parent.parent() == tabs).then_some(node.width))
        .ok_or("active tab absent")?;
    assert_eq!(active, px(200.));
    Ok(())
}

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

#[test]
fn holdings_search_can_acquire_focus() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    command(&mut commands)?;
    response(&outbox, Vec::new());
    outbox.bevy_holdings_initialize();
    app.update();
    click(&mut app, Kind::Tab(Tab::Items))?;
    let world = app.world_mut();
    let field = world.query_filtered::<Entity, With<cyclopedia::HoldingsSearch>>().single(world)?;
    crate::bevy_shell::ui::assert_editor_acquires_focus(world, field);
    Ok(())
}

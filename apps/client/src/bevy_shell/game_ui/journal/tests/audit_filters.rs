use super::*;

#[test]
fn castable_toggles_using_retained_filter_despite_stale_editors() -> Result {
    let (mut owner, mut commands, outbox) = host()?;
    outbox.bevy_hud_skills_fixture()?;
    outbox.bevy_journal_spellbook_select();
    open_spells(&mut owner);
    commands.try_recv()?;
    outbox.bevy_journal_spellbook_result(&result(
        Page {
            page: 0,
            total: 0,
            entries: vec![],
        }
        .encode()
        .ok_or("page")?,
    ));
    let key = Key {
        generation: owner.generation(),
        tab: Tab::Spells,
    };
    let mut state = State {
        key: Some(key),
        ..default()
    };
    let values = actions::Values {
        minimum: "0".into(),
        maximum: "0".into(),
        ..default()
    };
    for expected in [30, 0] {
        assert!(actions::perform(
            &mut owner,
            &mut state,
            Action {
                key,
                kind: Kind::Castable
            },
            &values
        ));
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("query".into());
        };
        let query = SpellQuery::decode(&command.payload).ok_or("query decode")?;
        assert_eq!(query.max_level, expected);
        outbox.bevy_journal_spellbook_result(&result(
            Page {
                page: 0,
                total: 0,
                entries: vec![],
            }
            .encode()
            .ok_or("page")?,
        ));
    }
    Ok(())
}

#[test]
fn castable_updates_level_editors_before_later_search() -> Result {
    let (mut owner, mut commands, outbox) = host()?;
    outbox.bevy_hud_skills_fixture()?;
    outbox.bevy_journal_spellbook_select();
    open_spells(&mut owner);
    commands.try_recv()?;
    outbox.bevy_journal_spellbook_result(&result(
        Page {
            page: 0,
            total: 0,
            entries: vec![],
        }
        .encode()
        .ok_or("page")?,
    ));
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    let castable = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| matches!(a.kind, Kind::Castable).then_some(e))
        .ok_or("castable")?;
    app.world_mut().trigger(Activate { entity: castable });
    app.update();
    let maximum = app
        .world_mut()
        .query::<(&Field, &EditableText)>()
        .iter(app.world())
        .find_map(|(f, t)| (*f == Field::Maximum).then(|| t.value().to_string()))
        .ok_or("maximum")?;
    assert_eq!(maximum, "30");
    Ok(())
}

#[test]
fn fallback_filter_controls_and_search_preserve_selections() -> Result {
    let (mut owner, mut commands, outbox) = host()?;
    confirm_elite_knight(&owner, &mut commands, &outbox)?;
    outbox.bevy_journal_catalogue(
        oteryn_session::spell_catalog::encode_catalog(&[]).map_err(|_| "catalogue")?,
    )?;
    open_spells(&mut owner);
    let link = &owner.play.as_ref().ok_or("play")?.link;
    assert!(link.spell_catalog_search("", "", "2", true, "knight"));
    let key = Key {
        generation: owner.generation(),
        tab: Tab::Spells,
    };
    let mut state = State {
        key: Some(key),
        ..default()
    };
    assert!(actions::perform(
        &mut owner,
        &mut state,
        Action {
            key,
            kind: Kind::Search
        },
        &actions::Values {
            search: "mend".into(),
            ..default()
        }
    ));
    let panel = owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .spell_catalog_panel();
    assert_eq!(panel.group, Some(2));
    assert!(panel.learned_only);
    assert_eq!(panel.vocation.as_deref(), Some("knight"));
    for kind in [Kind::Vocation, Kind::Group, Kind::Premium] {
        assert!(actions::perform(
            &mut owner,
            &mut state,
            Action { key, kind },
            &actions::Values::default()
        ));
    }
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Action>()
            .iter(app.world())
            .filter(|action| matches!(action.kind, Kind::Vocation | Kind::Group | Kind::Premium))
            .count(),
        3
    );
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .spell_catalog_search("", "", "", false, "");
    let button = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, action)| matches!(action.kind, Kind::MyVocation).then_some(entity))
        .ok_or("catalogue my vocation")?;
    for expected in [Some("elite_knight"), None] {
        app.world_mut().trigger(Activate { entity: button });
        app.update();
        let catalogue = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_catalog_panel();
        assert_eq!(catalogue.vocation.as_deref(), expected);
    }
    Ok(())
}

#[test]
fn spell_list_my_vocation_uses_confirmed_character() -> Result {
    for cached in [true, false] {
        let (mut owner, mut commands, outbox) = host()?;
        if cached {
            confirm_elite_knight(&owner, &mut commands, &outbox)?;
        } else {
            outbox.bevy_cyclopedia_initialize(true, true, false);
        }
        outbox.bevy_journal_view_other_character();
        assert!(
            owner
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .cyclopedia_character_request(oteryn_client_ui::cyclopedia_character::Intent::Tab)
        );
        commands.try_recv()?;
        outbox.bevy_cyclopedia_result(&result(
            oteryn_client_runtime::cyclopedia_character::TabView::GeneralStats(Default::default())
                .encode_extended(&oteryn_client_runtime::cyclopedia_character::TabExtension {
                    vocation_id: Some(4),
                    deaths: None,
                })
                .map_err(|_| "foreign stats")?,
        ));
        outbox.bevy_journal_spellbook_select();
        open_spells(&mut owner);
        commands.try_recv()?;
        outbox.bevy_journal_spellbook_result(&result(
            Page {
                page: 0,
                total: 0,
                entries: vec![],
            }
            .encode()
            .ok_or("page")?,
        ));
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Font>()
            .init_resource::<InputFocus>()
            .add_message::<bevy::input::keyboard::KeyboardInput>()
            .insert_non_send(owner)
            .add_systems(Startup, crate::bevy_shell::ui::setup);
        install(&mut app);
        app.update();
        let button = app
            .world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| (text.0 == "My vocation").then_some(parent.parent()))
            .ok_or("my vocation control")?;
        app.world_mut().trigger(Activate { entity: button });
        if !cached {
            let PlayCommand::Feature(request) = commands.try_recv()? else {
                return Err("expected self stats request".into());
            };
            assert_eq!(request.kind, 388);
            let query =
                oteryn_client_runtime::cyclopedia_character::decode_tab_request(&request.payload)
                    .map_err(|_| "tab query")?;
            assert_eq!(query.target, None);
            assert_eq!(
                query.tab,
                oteryn_client_runtime::cyclopedia_character::CharacterTab::GeneralStats
            );
            outbox.bevy_cyclopedia_result(&result(
                oteryn_client_runtime::cyclopedia_character::TabView::GeneralStats(
                    Default::default(),
                )
                .encode_extended(&oteryn_client_runtime::cyclopedia_character::TabExtension {
                    vocation_id: Some(7),
                    deaths: None,
                })
                .map_err(|_| "own stats")?,
            ));
            app.update();
            app.world_mut().trigger(Activate { entity: button });
        }
        let profile = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .cyclopedia_character_panel();
        assert_eq!(profile.target, Some([42; 16]));
        assert_eq!(profile.vocation_id, Some(4));
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("spellbook query".into());
        };
        assert_eq!(
            SpellQuery::decode(&command.payload)
                .ok_or("filter")?
                .vocation,
            7
        );
    }
    Ok(())
}

fn confirm_elite_knight(
    owner: &HostOwner,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    outbox: &crate::play::Outbox,
) -> Result {
    use oteryn_client_runtime::cyclopedia_character::{GeneralStats, TabExtension, TabView};
    outbox.bevy_cyclopedia_initialize(true, true, false);
    owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .cyclopedia_character_request(oteryn_client_ui::cyclopedia_character::Intent::Tab);
    commands.try_recv()?;
    outbox.bevy_cyclopedia_result(&result(
        TabView::GeneralStats(GeneralStats::default())
            .encode_extended(&TabExtension {
                vocation_id: Some(7),
                deaths: None,
            })
            .map_err(|_| "stats")?,
    ));
    Ok(())
}

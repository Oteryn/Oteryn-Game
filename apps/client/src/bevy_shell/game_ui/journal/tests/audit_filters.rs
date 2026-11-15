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
    let (mut owner, _, outbox) = host()?;
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
    Ok(())
}

#[test]
fn battle_and_player_health_use_percentage_colours() -> TestResult {
    let (mut app, _commands, outbox) = app()?;
    for (percent, rgb) in [
        (100, [0, 188, 0]),
        (92, [80, 161, 80]),
        (60, [161, 161, 0]),
        (30, [191, 10, 10]),
        (8, [145, 15, 15]),
        (3, [133, 12, 12]),
    ] {
        let mut enemy = row(7);
        if let EntityDetail::Actor { health_percent, .. } = &mut enemy.detail {
            *health_percent = percent;
        }
        outbox.bevy_hud_fixture(
            &[enemy],
            Some(ActorVitals {
                health: u32::from(percent),
                max_health: 100,
                mana: 10,
                max_mana: 20,
                ..default()
            }),
        );
        app.update();
        let expected = Color::srgb_u8(rgb[0], rgb[1], rgb[2]);
        assert_eq!(
            app.world_mut()
                .query::<(&battle::HealthFill, &BackgroundColor)>()
                .iter(app.world())
                .map(|(_, color)| color.0)
                .collect::<Vec<_>>(),
            vec![expected]
        );
        assert_eq!(
            app.world_mut()
                .query::<(&VitalFill, &BackgroundColor)>()
                .iter(app.world())
                .filter(|(fill, _)| !fill.0)
                .map(|(_, color)| color.0)
                .collect::<Vec<_>>(),
            vec![expected]
        );
    }
    Ok(())
}

#[test]
fn compact_status_projects_the_settings_owner() -> TestResult {
    let (mut app, _, _) = app()?;
    for compact in [false, true] {
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences
            .compact_status = compact;
        app.update();
        let owner = app.world().non_send::<HostOwner>();
        let play = owner.play.as_ref().ok_or("play")?;
        let expected = if compact {
            oteryn_client_ui::spell_action_bar::vitals_text(play.link.actor_vitals().as_ref())
        } else {
            let combined = status_line(play, None);
            for part in [
                play.view.hud_status(),
                play.link.item_movement_status().to_owned(),
                play.link.custom_hotkey_status(),
                format!(
                    "Audio {}% (ready)",
                    play.view.preferences().audio.master_volume
                ),
                "Ctrl+arrows: Turn; Ctrl+F: Follow; Esc: Stop".to_owned(),
            ] {
                assert!(
                    combined.contains(&part),
                    "combined status line misses {part}"
                );
            }
            format!("{combined}\n{}", play.view.combat_status())
        };
        assert!(
            app.world_mut()
                .query::<(&InfoText, &Text)>()
                .iter(app.world())
                .any(|(_, text)| text.0 == expected)
        );
    }
    Ok(())
}

#[test]
fn parity_a_battle_chord_and_stacked_row_preserve_exact_target() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerPress},
        pointer::{Location, PointerButton, PointerId},
    };
    for gesture in 0..3 {
        let chord = gesture == 1;
        let (mut app, mut commands, outbox) = app()?;
        crate::play::PlayLink::bevy_items_fixture(&outbox, None, None, false, false);
        let target = row(7);
        let mut stacked = row(8);
        stacked.entity.identity = [3; 16];
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .play
            .as_mut()
            .ok_or("play")?
            .view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?
            .with_entities(vec![target, stacked])?;
        let entity = app
            .world_mut()
            .query::<(Entity, &HudAction)>()
            .iter(app.world())
            .find_map(|(entity, action)| {
                matches!(action.command, Command::Attack(id) if id == target.entity)
                    .then_some(entity)
            })
            .ok_or("row")?;
        let pointer = Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 100,
                    height: 100,
                },
                position: Vec2::ZERO,
            },
        );
        if chord {
            app.init_resource::<ButtonInput<MouseButton>>();
            // InputPlugin may publish both releases before picking dispatches these presses.
            for button in [MouseButton::Left, MouseButton::Right] {
                let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
                mouse.press(button);
                mouse.release(button);
            }
            for button in [PointerButton::Primary, PointerButton::Secondary] {
                app.world_mut().trigger(PointerPress {
                    entity,
                    pointer: pointer.clone(),
                    button,
                    hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
                    count: 1,
                });
            }
        } else if gesture == 0 {
            app.init_resource::<ButtonInput<KeyCode>>();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ShiftLeft);
        }
        for button in if chord {
            vec![PointerButton::Primary, PointerButton::Secondary]
        } else if gesture == 0 {
            vec![PointerButton::Primary]
        } else {
            vec![PointerButton::Secondary]
        } {
            app.world_mut().trigger(PointerClick {
                entity,
                pointer: pointer.clone(),
                button,
                hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
                duration: std::time::Duration::ZERO,
                count: 1,
            });
        }
        if gesture == 2 {
            assert!(app.world().non_send::<HostOwner>().dialogs.active());
        } else {
            assert!(
                matches!(commands.try_recv()?, PlayCommand::WorldContext(command) if command.target == oteryn_client_ui::world_interaction::menu::Target::Creature(target.entity) && command.action == oteryn_client_ui::world_interaction::menu::Action::Look)
            );
        }
        assert!(
            commands.try_recv().is_err(),
            "chord releases must not attack or open a second menu"
        );
    }
    Ok(())
}

#[test]
fn edit_lists_button_opens_the_battle_list_editor_form() -> TestResult {
    let (mut app, _, _) = app()?;
    let menu = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| {
            matches!(action.command, Command::BattleMenu).then_some(entity)
        })
        .ok_or("battle list menu button")?;
    app.world_mut().trigger(Activate { entity: menu });
    app.update();
    let button = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| {
            matches!(action.command, Command::BattleListEditor).then_some(entity)
        })
        .ok_or("edit lists button")?;
    app.world_mut().trigger(Activate { entity: button });
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let form = crate::play::FeatureForm::all()
        .iter()
        .copied()
        .find(|form| form.title(link) == "Battle lists")
        .ok_or("battle list form")?;
    let view = link.feature_form_view(form);
    assert!(view.open);
    assert_eq!(view.fields.first().map(String::as_str), Some("Nearby"));
    Ok(())
}

#[test]
fn the_status_widget_is_a_sidebar_bar_pair_without_prose_or_ids() -> TestResult {
    let (mut app, _, _) = app_with(
        false,
        Some(ActorVitals {
            health: 150,
            max_health: 150,
            mana: 35,
            max_mana: 90,
            ..default()
        }),
    )?;
    app.update();
    let vitals = app.world().resource::<super::super::GameRegions>().vitals;
    let rows: Vec<Entity> = app
        .world()
        .get::<Children>(vitals)
        .ok_or("status rows")?
        .iter()
        .take(2)
        .collect();
    for (row, key) in rows.into_iter().zip(["status-health", "status-mana"]) {
        let parts: Vec<Entity> = app
            .world()
            .get::<Children>(row)
            .ok_or("status row parts")?
            .iter()
            .collect();
        let icon = app.world().get::<super::super::art::ArtSlot>(parts[0]);
        assert_eq!(icon.map(|slot| slot.key.as_str()), Some(key));
        let size = |entity: Entity| {
            app.world()
                .get::<Node>(entity)
                .map(|node| (node.width, node.height))
        };
        assert_eq!(size(parts[0]), Some((px(12.), px(11.))));
        assert_eq!(size(parts[1]), Some((px(94.), px(11.))));
        assert_eq!(
            app.world().get::<Node>(parts[2]).map(|node| node.width),
            Some(px(42.))
        );
    }
    assert_eq!(
        app.world_mut()
            .query::<&InfoText>()
            .iter(app.world())
            .filter(|kind| matches!(kind, InfoText::Status))
            .count(),
        0,
        "the status prose and world ids live only in the developer overlay"
    );
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|value| value.0.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!text.contains("Ctrl+arrows"));
    // The sidebar widget shows the current value only; "cur/max" belongs to the status bar.
    let vitals = app.world().resource::<super::super::GameRegions>().vitals;
    assert!(
        !texts_under(&app, vitals)
            .iter()
            .any(|text| text.contains("150/150"))
    );
    Ok(())
}

fn named(app: &mut App, name: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, found)| (found.as_str() == name).then_some(entity))
        .ok_or_else(|| format!("{name} missing").into())
}
fn texts_under(app: &App, root: Entity) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(text) = app.world().get::<Text>(entity) {
            out.push(text.0.clone());
        }
        stack.extend(app.world().get::<Children>(entity).into_iter().flatten());
    }
    out
}
#[test]
fn top_status_bar_shows_vitals_level_xp_and_conditions_above_the_game_window() -> TestResult {
    let (mut app, mut commands, outbox) = app()?;
    outbox.bevy_hud_skills_fixture()?;
    app.update();
    app.update();
    let bar = app.world().resource::<super::super::StatusBar>().0;
    let top = named(&mut app, "Status bar")?;
    assert_eq!(
        app.world()
            .get::<ChildOf>(top)
            .ok_or("top parent")?
            .parent(),
        bar
    );
    // Line 1: health and mana side by side, each half the width, "cur/max" centred.
    let vitals = named(&mut app, "Status bar vitals")?;
    let halves: Vec<Val> = app
        .world()
        .get::<Children>(vitals)
        .ok_or("vital tracks")?
        .iter()
        .filter_map(|track| app.world().get::<Node>(track).map(|node| node.width))
        .collect();
    assert_eq!(halves, [percent(50.), percent(50.)]);
    let texts = texts_under(&app, vitals);
    assert!(texts.contains(&"42/100".to_owned()), "{texts:?}");
    assert!(texts.contains(&"10/20".to_owned()), "{texts:?}");
    // Line 2: the level, a red XP bar with ticks and the XP Boost button.
    let experience = named(&mut app, "Status bar experience")?;
    assert!(texts_under(&app, experience).contains(&"30".to_owned()));
    let fill = named(&mut app, "Status bar XP fill")?;
    assert_eq!(
        app.world().get::<Node>(fill).ok_or("xp fill")?.width,
        percent(23.45)
    );
    let ticks = app
        .world_mut()
        .query::<&Name>()
        .iter(app.world())
        .filter(|name| name.as_str() == "XP tick")
        .count();
    assert_eq!(ticks, 9);
    // Below: the centred condition row.
    let status = named(&mut app, "Status icons")?;
    let centre = app
        .world()
        .get::<ChildOf>(status)
        .ok_or("status parent")?
        .parent();
    assert_eq!(
        app.world()
            .get::<ChildOf>(centre)
            .ok_or("centre parent")?
            .parent(),
        top
    );
    assert_eq!(
        app.world()
            .get::<Node>(centre)
            .ok_or("centre node")?
            .justify_content,
        JustifyContent::Center
    );
    // XP Boost opens the store.
    while commands.try_recv().is_ok() {}
    let boost = named(&mut app, "status-xp-boost")?;
    app.world_mut().trigger(Activate { entity: boost });
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .store_panel()
            .open
    );
    Ok(())
}

#[test]
fn harmony_stays_visible_when_serene_ends() -> TestResult {
    let (mut app, _, outbox) = app()?;
    for serene in [true, false] {
        outbox.bevy_hud_fixture(&[row(7)], Some(ActorVitals { harmony: 3, serene, ..default() }));
        app.update();
        let world = app.world_mut();
        let node = world.query_filtered::<&Node, With<MonkRow>>().single(world)?;
        assert_eq!(node.display, Display::Flex);
        let icon = world.query::<(&Name, Option<&Visibility>)>().iter(world)
            .find(|(name, _)| name.as_str() == "status-serene")
            .ok_or("Serene indicator")?;
        assert_eq!(icon.1 == Some(&Visibility::Hidden), !serene);
    }
    Ok(())
}

use super::*;
#[test]
fn source_skill_rows_project_actual_levels_progress_and_saved_number_mode()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _commands, outbox) = super::super::tests::app()?;
    outbox.bevy_hud_skills_fixture()?;
    app.update();
    let values = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .filter(|text| text.0 == "20 / 23 / 24")
        .count();
    assert_eq!(values, 8);
    for (_, node) in app.world_mut().query::<(&Fill, &Node)>().iter(app.world()) {
        assert_eq!(node.width, percent(45.));
    }
    {
        let mut host = app.world_mut().non_send_mut::<HostOwner>();
        host.settings.preferences.skills_layout.numbers = SkillsNumbers::Current;
        let HostOwner { settings, play, .. } = &mut *host;
        let play = play.as_mut().ok_or("play")?;
        settings.restore(&mut play.view, [1024, 720])?;
        settings.sync_skills_layout(&play.link);
    }
    app.update();
    let values = app
        .world_mut()
        .query::<(&Value, &Text)>()
        .iter(app.world())
        .map(|(_, text)| text.0.clone())
        .collect::<Vec<_>>();
    assert_eq!(values, vec!["24"; 8]);
    Ok(())
}

#[test]
fn skills_hover_and_visibility_reset_controls_are_reachable()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _, outbox) = super::super::tests::app()?;
    outbox.bevy_hud_skills_fixture()?;
    app.update();
    let find = |app: &mut App, label: &str| -> Result<Entity, Box<dyn std::error::Error>> {
        app.world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find_map(|(entity, name)| (name.as_str() == label).then_some(entity))
            .ok_or_else(|| format!("missing {label}").into())
    };
    let menu = find(&mut app, "Skills controls")?;
    app.world_mut().trigger(Activate { entity: menu });
    app.update();
    let toggle = find(&mut app, "Toggle Magic")?;
    app.world_mut().trigger(Activate { entity: toggle });
    app.update();
    let host = app.world().non_send::<HostOwner>();
    assert!(!host.settings.preferences.skills_layout.order.contains(&0));
    assert_eq!(
        host.play
            .as_ref()
            .map(|p| p.link.skills_widget().rows.len()),
        Some(7)
    );
    app.world_mut().trigger(Activate { entity: toggle });
    app.update();
    let host = app.world().non_send::<HostOwner>();
    assert_eq!(
        host.play
            .as_ref()
            .map(|p| p.link.skills_widget().rows.len()),
        Some(8)
    );
    let progress = find(&mut app, "Show/hide all bars")?;
    app.world_mut().trigger(Activate { entity: progress });
    app.update();
    assert!(
        app.world_mut()
            .query::<(&Track, &Node)>()
            .iter(app.world())
            .all(|(_, node)| node.display == Display::None)
    );
    let reset = find(&mut app, "Reset XP counter")?;
    app.world_mut().trigger(Activate { entity: reset });
    app.update();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .tooltip_delay_ms = 0;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .tooltips_enabled = true;
    let mut window = Window::default();
    window.set_cursor_position(Some(Vec2::new(200., 100.)));
    app.world_mut().spawn((window, bevy::window::PrimaryWindow));
    let row = app
        .world_mut()
        .query::<(Entity, &Slot)>()
        .iter(app.world())
        .find_map(|(entity, slot)| (slot.0 == 0).then_some(entity))
        .ok_or("row")?;
    app.world_mut()
        .entity_mut(row)
        .insert(bevy::picking::hover::Hovered(true));
    app.update();
    let tooltip = find(&mut app, "Skills tooltip")?;
    assert_eq!(
        app.world()
            .get::<Node>(tooltip)
            .ok_or("tooltip node")?
            .display,
        Display::Flex
    );
    assert!(
        app.world()
            .get::<Text>(tooltip)
            .ok_or("tooltip text")?
            .0
            .contains("550 training points to next level")
    );
    app.world_mut().entity_mut(row).insert(Hovered(false));
    let header = app
        .world_mut()
        .query_filtered::<Entity, With<Header>>()
        .single(app.world())?;
    app.world_mut().entity_mut(header).insert(Hovered(true));
    app.update();
    assert!(
        app.world()
            .get::<Text>(tooltip)
            .ok_or("XP tooltip")?
            .0
            .contains("XP/hour: 0")
    );
    Ok(())
}

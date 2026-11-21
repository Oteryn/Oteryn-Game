use super::*;
#[test]
fn wheel_preset_management_is_reachable() -> TestResult {
    let (mut app, _commands, _outbox) = fixture(model())?;
    let labels: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    for caption in [
        "Save named build",
        "Rename selected",
        "Delete selected",
        "Export code",
        "Export URL",
        "Import code / URL",
    ] {
        assert!(
            labels.iter().any(|text| text == caption),
            "missing {caption}"
        );
    }
    Ok(())
}
fn preset_action(app: &mut App, index: usize) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &presets::Action)>()
        .iter(app.world())
        .find_map(|(e, a)| (a.index == index).then_some(e))
        .ok_or("preset action absent")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn preset_field(app: &mut App, index: usize, value: &str) {
    let world = app.world_mut();
    let mut query = world.query::<(&presets::Field, &mut bevy::text::EditableText)>();
    for (field, mut text) in query.iter_mut(world) {
        if field.0 == index {
            text.editor_mut().set_text(value);
        }
    }
}
#[test]
fn wheel_preset_export_import_proposes_points_and_preserves_gems() -> TestResult {
    let mut p = model();
    p.points[14] = 7;
    let code = p.build_code();
    let (mut app, mut commands, outbox) = fixture(p)?;
    preset_action(&mut app, 3)?;
    assert_eq!(panel(&app)?.preset_fields[1], code);
    preset_action(&mut app, 4)?;
    let url = code.replacen("OW1:", "oteryn://wheel/v1/", 1);
    assert_eq!(panel(&app)?.preset_fields[1], url);
    publish(&outbox, model());
    app.update();
    let vessels = panel(&app)?.vessels;
    preset_field(&mut app, 1, &url);
    preset_action(&mut app, 5)?;
    assert_eq!(panel(&app)?.points[14], 7);
    assert_eq!(panel(&app)?.vessels, vessels);
    while let Ok(command) = commands.try_recv() {
        assert!(
            matches!(command, PlayCommand::Feature(_)),
            "import must only query a preview"
        );
    }
    let mut refreshed = panel(&app)?;
    refreshed.diagram.pending = None;
    publish(&outbox, refreshed);
    app.update();
    preset_field(&mut app, 1, "OW1:1,2");
    preset_action(&mut app, 5)?;
    assert_eq!(panel(&app)?.points[14], 7);
    assert_eq!(panel(&app)?.preset_feedback, "Build needs 36 slots");
    Ok(())
}
#[test]
fn wheel_preset_edits_survive_preview_and_stale_actions_are_fenced() -> TestResult {
    let (mut app, _commands, outbox) = fixture(model())?;
    preset_field(&mut app, 0, "New hunting build");
    let mut refreshed = model();
    refreshed.diagram.feedback = "Preview refreshed".into();
    publish(&outbox, refreshed);
    app.update();
    assert!(
        app.world_mut()
            .query::<(&presets::Field, &bevy::text::EditableText)>()
            .iter(app.world())
            .any(|(field, text)| field.0 == 0 && text.value() == "New hunting build")
    );
    let expected = panel(&app)?;
    let mut changed = expected.clone();
    changed.points[14] = 10;
    publish(&outbox, changed);
    let changed = panel(&app)?;
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    assert!(!link.wheel_preset_bound(&expected, 3, [String::new(), String::new()]));
    assert_eq!(link.wheel_panel(), changed);
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    preset_action(&mut app, 3)?;
    assert!(panel(&app)?.preset_fields[1].is_empty());
    Ok(())
}

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

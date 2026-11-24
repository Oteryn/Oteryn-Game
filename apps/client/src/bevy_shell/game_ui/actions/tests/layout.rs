use super::*;

#[test]
fn boundary_arrows_jump_directly_to_first_and_last_visible_slots() -> TestResult {
    let (mut app, _) = fixture()?;
    let capacity = app.world().resource::<BarCapacities>().0[0];
    for (label, expected) in [
        ("Last action button", ROW_SLOTS - capacity),
        ("First action button", 0),
    ] {
        let entity = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find_map(|(e, name)| (name.as_str() == format!("{label} 1")).then_some(e))
            .ok_or("action bar boundary arrow")?;
        app.world_mut().trigger(Activate { entity });
        assert_eq!(
            usize::from(
                app.world()
                    .non_send::<HostOwner>()
                    .play
                    .as_ref()
                    .ok_or("play")?
                    .link
                    .bar_layout()
                    .rows[0]
                    .offset
            ),
            expected
        );
    }
    Ok(())
}

#[test]
fn hiding_assigned_hotkeys_removes_live_slot_corner_text() -> TestResult {
    let (mut app, _) = fixture()?;
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.settings.preferences.action_bar.show_hotkey = false;
    app.update();
    assert!(
        app.world_mut()
            .query::<(&SlotHotkey, &Text)>()
            .iter(app.world())
            .all(|(_, text)| text.0.is_empty())
    );
    Ok(())
}

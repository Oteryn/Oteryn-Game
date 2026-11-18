use super::*;

#[test]
fn disabled_bar_tooltips_hide_object_hint() -> TestResult {
    let (mut app, _) = fixture()?;
    let first = slot(&mut app, 0)?;
    {
        let owner = &mut *app.world_mut().non_send_mut::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        assert!(link.assign_bar_action(
            0,
            crate::play::BarAction::Use {
                definition: 256u32.try_into()?,
                sub_type: None,
                mode: crate::play::ItemUseMode::Use,
            }
        ));
        owner.settings.sync_bar_actions(link);
        owner.settings.preferences.action_bar.tooltips = false;
        owner.settings.sync_local_preferences(link);
    }
    app.world_mut()
        .entity_mut(first)
        .insert(bevy::picking::hover::Hovered(true));
    app.update();
    assert!(
        app.world_mut()
            .query_filtered::<&Text, With<ActionHint>>()
            .single(app.world())?
            .0
            .is_empty()
    );
    Ok(())
}

#[test]
fn action_hint_obeys_global_tooltip_setting_and_delay() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        hover::{HoverMap, update_is_hovered},
        pointer::PointerId,
    };
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;
    let (mut app, _) = fixture()?;
    let first = slot(&mut app, 0)?;
    let hint = app
        .world_mut()
        .query_filtered::<Entity, With<ActionHint>>()
        .single(app.world())?;
    {
        let owner = &mut *app.world_mut().non_send_mut::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        assert!(link.assign_bar_action(
            0,
            crate::play::BarAction::Use {
                definition: 256u32.try_into()?,
                sub_type: None,
                mode: crate::play::ItemUseMode::Use,
            }
        ));
        owner.settings.sync_bar_actions(link);
        let prefs = &mut owner.settings.preferences;
        prefs.tooltips_enabled = false;
        prefs.action_bar.tooltips = true;
        prefs.tooltip_delay_ms = 100;
    }
    let mut hover = HoverMap::default();
    hover
        .entry(PointerId::Mouse)
        .or_default()
        .insert(first, HitData::new(Entity::PLACEHOLDER, 0., None, None));
    app.insert_resource(hover)
        .add_systems(PreUpdate, update_is_hovered);
    for (enabled, ms, visible) in [
        (false, 0, false),
        (true, 0, false),
        (true, 99, false),
        (true, 1, true),
        (false, 0, false),
    ] {
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences
            .tooltips_enabled = enabled;
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            ms,
        )));
        app.update();
        assert_eq!(
            app.world().get::<Text>(hint).ok_or("hint")?.0,
            if visible {
                "Object appearance unavailable."
            } else {
                ""
            }
        );
    }
    Ok(())
}

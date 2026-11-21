#[test]
fn cyclopedia_map_donate_waits_for_the_pending_paste_and_sends_the_pasted_gold()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_map as map;
    use std::sync::{Arc, Mutex};
    let (mut app, mut rx, _outbox) = ready()?;
    let field = app
        .world_mut()
        .query_filtered::<Entity, With<Donation>>()
        .single(app.world())?;
    {
        let mut text = app
            .world_mut()
            .get_mut::<EditableText>(field)
            .ok_or("gold")?;
        *text = EditableText::new("500");
        text.queue_edit(bevy::text::TextEdit::SelectAll);
        text.pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::new(
            Mutex::new(None),
        )));
    }
    press(&mut app, Kind::Explore(Intent::Donate))?;
    assert!(rx.try_recv().is_err(), "Donate spent the unsettled amount");
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new("50"));
    app.update();
    let donate = feature(&mut rx)?;
    assert_eq!(
        (donate.kind, &donate.payload),
        (73, &map::request(4, 50).map_err(|e| format!("{e:?}"))?.1)
    );
    assert!(rx.try_recv().is_err());
    Ok(())
}

#[test]
fn cyclopedia_map_donate_pressed_before_a_map_update_is_sent_after_the_redraw()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_map as map;
    let (mut app, mut rx, _outbox) = ready()?;
    for mut text in app
        .world_mut()
        .query_filtered::<&mut EditableText, With<Donation>>()
        .iter_mut(app.world_mut())
    {
        *text = EditableText::new("500");
    }
    // Settle the constructor's queued cursor move before pressing Donate.
    app.update();
    let entity = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| (a.kind == Kind::Explore(Intent::Donate)).then_some(e))
        .ok_or("missing control")?;
    app.world_mut().trigger(Activate { entity });
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .cyclopedia_map_edit(0, "1".into());
    app.update();
    let donate = feature(&mut rx)?;
    assert_eq!(
        (donate.kind, &donate.payload),
        (73, &map::request(4, 500).map_err(|e| format!("{e:?}"))?.1)
    );
    assert!(rx.try_recv().is_err());
    Ok(())
}

#[test]
fn cyclopedia_map_keeps_the_typed_donation_across_a_window_redraw()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, mut rx, _outbox) = ready()?;
    for mut text in app
        .world_mut()
        .query_filtered::<&mut EditableText, With<Donation>>()
        .iter_mut(app.world_mut())
    {
        *text = EditableText::new("750");
    }
    press(&mut app, Kind::Explore(Intent::Query))?;
    let query = feature(&mut rx)?;
    let expected =
        oteryn_client_runtime::cyclopedia_map::request(0, 0).map_err(|e| format!("{e:?}"))?;
    assert_eq!((query.kind, query.payload), expected);
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .cyclopedia_map_panel()
            .pending,
        "the status request redraws the window"
    );
    assert_eq!(gold(&mut app)?, "750");
    Ok(())
}

#[test]
fn cyclopedia_map_floor_controls_reach_every_region_atlas_floor()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_map::region_atlas;
    let (mut app, mut rx, outbox) = ready()?;
    press(&mut app, Kind::Explore(Intent::RegionAtlas))?;
    let atlas = feature(&mut rx)?;
    let area = |id, parent, name: &str| region_atlas::Area {
        id,
        parent,
        name: name.into(),
    };
    let run = |area, floor| region_atlas::Run {
        area,
        x: 100,
        end: 103,
        y: 100,
        floor,
    };
    reply(
        &outbox,
        &atlas,
        region_atlas::encode(&region_atlas::Page {
            next: None,
            areas: vec![
                area(1, 0, "Harbour"),
                area(2, 1, "Quays"),
                area(3, 1, "Cellars"),
            ],
            runs: vec![run(2, 7), run(3, 6)],
        })
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    assert!(has_text(&mut app, "Harbour > Quays [#2]"));
    assert!(!has_text(&mut app, "Harbour > Cellars [#3]"));
    press(&mut app, Kind::Floor { previous: false })?;
    assert!(has_text(&mut app, "Harbour > Cellars [#3]"));
    assert!(!has_text(&mut app, "Harbour > Quays [#2]"));
    press(&mut app, Kind::Floor { previous: true })?;
    assert!(has_text(&mut app, "Harbour > Quays [#2]"));
    assert!(rx.try_recv().is_err(), "floor selection stays local");
    Ok(())
}

#[test]
fn cyclopedia_map_donate_preserves_pending_edits_when_the_snapshot_changes()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_map as map;
    let (mut app, mut rx, _outbox) = ready()?;
    let field = app
        .world_mut()
        .query_filtered::<Entity, With<Donation>>()
        .single(app.world())?;
    {
        let mut text = app
            .world_mut()
            .get_mut::<EditableText>(field)
            .ok_or("gold")?;
        *text = EditableText::new("75");
        text.queue_edit(bevy::text::TextEdit::Insert("0".into()));
    }
    let button = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| (a.kind == Kind::Explore(Intent::Donate)).then_some(e))
        .ok_or("Donate")?;
    app.world_mut().trigger(Activate { entity: button });
    let serial = app.world().resource::<State>().serial;
    let root = app.world().resource::<State>().root;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .cyclopedia_map_edit(0, "1".into());
    app.update();
    assert_eq!(
        app.world().resource::<State>().serial,
        serial,
        "pending edits defer redraw"
    );
    assert_eq!(app.world().resource::<State>().root, root);
    assert_eq!(gold(&mut app)?, "750");
    assert!(rx.try_recv().is_err());
    app.update();
    assert_ne!(app.world().resource::<State>().serial, serial);
    let donate = feature(&mut rx)?;
    assert_eq!(
        (donate.kind, &donate.payload),
        (73, &map::request(4, 750).map_err(|e| format!("{e:?}"))?.1)
    );
    assert!(rx.try_recv().is_err());
    Ok(())
}

#[test]
fn rejected_gold_draft_does_not_donate() -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, mut rx, _) = ready()?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .cyclopedia_map_edit(0, "100".into());
    app.update();
    app.world_mut()
        .query_filtered::<&mut EditableText, With<Donation>>()
        .single_mut(app.world_mut())?
        .editor_mut()
        .set_text(&"é".repeat(33));
    press(&mut app, Kind::Explore(Intent::Donate))?;
    assert!(rx.try_recv().is_err());
    Ok(())
}

#[test]
fn next_floor_requests_selected_footprint() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_map::geometry;
    let (mut app, mut rx, _) = ready()?;
    let entity = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == "Next floor").then_some(parent.parent()))
        .ok_or("Next floor")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    let request =
        geometry::decode_request(&feature(&mut rx)?.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((request.area, request.floor), (1, 6));
    Ok(())
}

#[test]
fn map_donation_can_acquire_focus() -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _, _) = ready()?;
    let world = app.world_mut();
    let field = world.query_filtered::<Entity, With<Donation>>().single(world)?;
    crate::bevy_shell::ui::assert_editor_acquires_focus(world, field);
    Ok(())
}

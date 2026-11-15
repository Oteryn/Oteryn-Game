use super::*;
use oteryn_client_runtime::highscores::wire;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn panel() -> HighscoresPanel {
    let own = wire::HighscoreRow {
        rank: 51,
        name: "Own Hero".into(),
        vocation: "knight".into(),
        level: 100,
        value: 900,
    };
    let mut panel = HighscoresPanel::default();
    panel.selected = true;
    panel.reply = Some(wire::HighscoresResult {
        world: None,
        own_page: Some(2),
        category: 1,
        snapshot_id: 7,
        computed_at: 60000,
        page: 2,
        has_more: true,
        rows: vec![
            own.clone(),
            wire::HighscoreRow {
                rank: 52,
                name: "Other Hero".into(),
                ..own.clone()
            },
        ],
        own_row: Some(own),
    });
    panel
}
fn query(
    panel: &mut HighscoresPanel,
    kind: Kind,
) -> Result<wire::HighscoresQuery, Box<dyn std::error::Error>> {
    let action = request_action(panel, kind).ok_or("action")?;
    let command = oteryn_client_ui::highscores::request(panel, action).ok_or("command")?;
    wire::decode_highscores_query(&command.payload).map_err(|e| format!("{e:?}").into())
}
#[test]
fn highscores_filter_change_requests_page() -> TestResult {
    for kind in [
        Kind::Category,
        Kind::Vocation,
        Kind::World(Some(
            wire::WorldId::decode(&[1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 1])
                .map_err(|e| format!("{e:?}"))?,
        )),
    ] {
        let mut p = panel();
        let q = query(&mut p, kind)?;
        assert_eq!(q.page, 1);
        assert_eq!(q.snapshot_id, 0);
        match kind {
            Kind::Category => assert_eq!(q.category, 2),
            Kind::Vocation => assert_eq!(q.vocation, 1),
            Kind::World(world) => assert_eq!(q.world, world),
            _ => return Err("filter".into()),
        }
    }
    Ok(())
}
#[test]
fn highscores_rows_and_own_row_highlight() -> TestResult {
    let p = panel();
    let rows: Vec<_> = table_rows(&p, &[]).collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].cells,
        ["51", "Own Hero", "knight", "Current world", "100", "900"]
    );
    assert_eq!(rows[0].color, Color::srgb_u8(0x60, 0xf8, 0x60));
    assert_ne!(rows[0].background, rows[1].background);
    assert_ne!(rows[0].color, rows[1].color);
    assert_eq!(updated_text(&p, 240000), "Last updated 3 minutes ago");
    Ok(())
}
#[test]
fn highscores_paging_bounds() -> TestResult {
    for (kind, page) in [
        (Kind::First, 1),
        (Kind::Previous, 1),
        (Kind::Next, 3),
        (Kind::Last, 3),
        (Kind::Own, 2),
    ] {
        let q = query(&mut panel(), kind)?;
        assert_eq!(q.page, page);
        assert_eq!(q.snapshot_id, 7);
    }
    let mut p = panel();
    let reply = p.reply.as_mut().ok_or("reply")?;
    reply.page = 1;
    reply.has_more = false;
    reply.own_page = None;
    for kind in [
        Kind::First,
        Kind::Previous,
        Kind::Next,
        Kind::Last,
        Kind::Own,
    ] {
        assert!(request_action(&p, kind).is_none());
    }
    p.pending = true;
    assert!(request_action(&p, Kind::Category).is_none());
    Ok(())
}
#[test]
fn highscores_table_projects_bevy_cells() -> TestResult {
    use bevy::ecs::system::RunSystemOnce;
    let mut world = World::new();
    let parent = world.spawn(Node::default()).id();
    let p = panel();
    world
        .run_system_once(move |mut commands: Commands| {
            widgets::table(&mut commands, Handle::default(), parent, &p, &[]);
        })
        .map_err(|e| format!("{e:?}"))?;
    let cells: Vec<_> = world.query::<(&Text, &TextColor)>().iter(&world).collect();
    assert!(
        cells.iter().any(
            |(text, color)| text.0 == "Own Hero" && color.0 == Color::srgb_u8(0x60, 0xf8, 0x60)
        )
    );
    for heading in ["Rank", "Name", "Vocation", "World", "Level", "Points"] {
        assert!(cells.iter().any(|(text, _)| text.0 == heading));
    }
    let stripes: Vec<_> = world
        .query::<(&Children, &BackgroundColor)>()
        .iter(&world)
        .filter(|(children, _)| children.len() == 6)
        .map(|(_, color)| color.0)
        .collect();
    assert_eq!(stripes.len(), 3);
    assert_ne!(stripes[1], stripes[2]);
    Ok(())
}
#[test]
fn highscores_routed_shortcut_is_ctrl_h_without_repeat() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(11)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    };
    assert!(oteryn_client_ui::highscores::open_key(&[event]));
    assert!(!oteryn_client_ui::highscores::open_key(&[
        NormalizedInputEvent::DeviceLost
    ]));
    assert!(!oteryn_client_ui::highscores::open_key(&[
        NormalizedInputEvent::Key {
            code: KeyCode::new(11)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control),
            repeat: true
        }
    ]));
    Ok(())
}
#[test]
fn highscores_first_and_last_follow_snapshot_until_boundary() -> TestResult {
    for (kind, start, end) in [(Kind::First, 4, 1), (Kind::Last, 2, 5)] {
        let mut p = panel();
        p.reply.as_mut().ok_or("reply")?.page = start;
        while let Some(action) = request_action(&p, kind) {
            let command = oteryn_client_ui::highscores::request(&mut p, action).ok_or("command")?;
            let q =
                wire::decode_highscores_query(&command.payload).map_err(|e| format!("{e:?}"))?;
            assert!((1..=5).contains(&q.page));
            assert_eq!(q.snapshot_id, 7);
            assert!(request_action(&p, kind).is_none());
            p.pending = false;
            let reply = p.reply.as_mut().ok_or("reply")?;
            reply.page = q.page;
            reply.has_more = q.page < 5;
        }
        assert_eq!(p.reply.as_ref().ok_or("reply")?.page, end);
    }
    Ok(())
}
#[test]
fn highscores_categories_respect_loyalty_capability() -> TestResult {
    let mut p = panel();
    p.category = 12;
    assert_eq!(query(&mut p, Kind::Category)?.category, 14);
    let mut p = panel();
    p.category = 12;
    p.loyalty_selected = true;
    assert_eq!(query(&mut p, Kind::Category)?.category, 13);
    Ok(())
}
#[test]
fn highscores_saved_f9_shortcut_dispatch_respects_blocked_input() -> TestResult {
    shortcut_dispatch(false)
}
#[test]
fn highscores_rebound_shortcut_dispatch_respects_blocked_input() -> TestResult {
    shortcut_dispatch(true)
}
fn shortcut_dispatch(rebound: bool) -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut owner = HostOwner::new(None)?;
    let (link, _commands, _outbox) = crate::play::play_channel();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    owner.settings.preferences.rebind(
        "0009-highscores-open",
        if rebound { "ALT+H" } else { "CTRL+SHIFT+F9" },
    )?;
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(if rebound { 11 } else { 66 })?,
        state: ButtonState::Pressed,
        modifiers: if rebound {
            Modifiers::one(Modifier::Alt)
        } else {
            Modifiers::one(Modifier::Control).with(Modifier::Shift)
        },
        repeat: false,
    };
    owner.route_input(std::slice::from_ref(&event), true);
    assert_eq!(owner.controls.highscores_generation, None);
    owner.route_input(&[event], false);
    assert_eq!(
        owner.controls.highscores_generation,
        Some(owner.generation())
    );
    Ok(())
}

#[test]
fn highscores_unknown_world_keeps_its_identity() -> TestResult {
    let world =
        wire::WorldId::decode(&[1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 2])
            .map_err(|e| format!("{e:?}"))?;
    let mut p = panel();
    p.reply.as_mut().ok_or("reply")?.world = Some(world);
    assert!(
        table_rows(&p, &[])
            .all(|row| row.cells[3] == oteryn_client_ui::highscores::world_text(world))
    );
    let worlds = [(world, "Named World".into())];
    assert!(table_rows(&p, &worlds).all(|row| row.cells[3] == "Named World"));
    Ok(())
}

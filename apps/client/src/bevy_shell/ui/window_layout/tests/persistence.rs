use super::*;

#[test]
fn exit_after_panel_despawn_saves_resize_anchor() -> TestResult {
    let fixture_dir = crate::play::screenshot::TestDir::new()?;
    let directory = fixture_dir.join("layout");
    let (mut app, path, _) = saved_market(&directory)?;
    stacking::ui_layout(&mut app, UVec2::new(1600, 900));
    install_close(&mut app);
    let screen = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1600, 900).into(),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    let container = app
        .world_mut()
        .spawn(Node {
            width: px(1600.),
            height: px(900.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .id();
    let panel = app
        .world_mut()
        .spawn((window("market", default()), ChildOf(container)))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<ComputedNode>(panel)
            .ok_or("initial layout")?
            .size(),
        Vec2::new(1400., 600.)
    );
    drag_grip(&mut app, panel, IVec2::NEG_X, Vec2::new(46., 0.))?;
    app.update();
    assert_eq!(
        app.world()
            .get::<UiTransform>(panel)
            .ok_or("panel")?
            .translation,
        Val2::px(23., 0.)
    );
    app.world_mut().despawn(panel);
    app.world_mut()
        .write_message(bevy::window::WindowCloseRequested { window: screen });
    app.update();
    let market = WindowLayouts::load(&path)?.0["market"];
    assert_eq!(market.size, Some(Vec2::new(1354., 600.)));
    assert_eq!(market.offset, Vec2::new(23., 0.));
    Ok(())
}

#[test]
fn vertical_grip_preserves_clipped_preferred_width() -> TestResult {
    let fixture_dir = crate::play::screenshot::TestDir::new()?;
    let directory = fixture_dir.join("layout");
    let (mut app, path, _) = saved_market(&directory)?;
    stacking::ui_layout(&mut app, UVec2::new(1024, 900));
    install_close(&mut app);
    let screen = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1024, 900).into(),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    let container = app
        .world_mut()
        .spawn(Node {
            width: px(1024.),
            height: px(900.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .id();
    let panel = app
        .world_mut()
        .spawn((window("market", default()), ChildOf(container)))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<ComputedNode>(panel)
            .ok_or("initial layout")?
            .size(),
        Vec2::new(1024., 600.)
    );
    drag_grip(&mut app, panel, IVec2::Y, Vec2::new(0., 46.))?;
    app.update();
    assert_eq!(
        app.world()
            .get::<UiTransform>(panel)
            .ok_or("panel")?
            .translation,
        Val2::px(0., 23.)
    );
    app.world_mut().despawn(panel);
    app.world_mut()
        .write_message(bevy::window::WindowCloseRequested { window: screen });
    app.update();
    let market = WindowLayouts::load(&path)?.0["market"];
    assert_eq!(market.size, Some(Vec2::new(1400., 646.)));
    assert_eq!(market.offset, Vec2::new(0., 23.));
    Ok(())
}
#[test]
fn moving_prey_preserves_other_clients_market_edit() -> TestResult {
    let fixture_dir = crate::play::screenshot::TestDir::new()?;
    let directory = fixture_dir.join("layout");
    let (mut app, path, _) = saved_market(&directory)?;
    let (mut first, _, _) = saved_market(&directory)?;
    let prey = app.world_mut().spawn(window("prey", default())).id();
    app.update();
    let market = first.world_mut().spawn(window("market", default())).id();
    first.update();
    first
        .world_mut()
        .get_mut::<UiTransform>(market)
        .ok_or("market")?
        .translation = Val2::px(22., 44.);
    first.world_mut().write_message(AppExit::Success);
    first.update();
    let other = WindowLayouts::load(&path)?;
    assert_eq!(other.0["market"].offset, Vec2::new(22., 44.));
    app.world_mut()
        .get_mut::<UiTransform>(prey)
        .ok_or("prey")?
        .translation = Val2::px(12., 34.);
    app.world_mut().write_message(AppExit::Success);
    app.update();
    let saved = WindowLayouts::load(&path)?;
    assert_eq!(saved.0["market"], other.0["market"]);
    assert_eq!(saved.0["prey"].offset, Vec2::new(12., 34.));
    Ok(())
}

#[test]
fn save_failure_retries_on_exit() -> TestResult {
    let fixture_dir = crate::play::screenshot::TestDir::new()?;
    let directory = fixture_dir.join("layout");
    let (mut app, path, _) = saved_market(&directory)?;
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs(1),
    ));
    let panel = app.world_mut().spawn(window("market", default())).id();
    app.update();
    let parent = path.parent().ok_or("directory")?;
    let backup = parent.with_extension("backup");
    std::fs::rename(parent, &backup)?;
    std::fs::write(parent, b"blocked")?;
    app.world_mut()
        .get_mut::<UiTransform>(panel)
        .ok_or("panel")?
        .translation = Val2::px(14., 20.);
    for _ in 0..4 {
        app.update();
    }
    std::fs::remove_file(parent)?;
    std::fs::rename(backup, parent)?;
    app.world_mut().write_message(AppExit::Success);
    app.update();
    assert_eq!(
        WindowLayouts::load(&path)?.0["market"].offset,
        Vec2::new(14., 20.)
    );
    Ok(())
}

#[test]
fn simultaneous_publications_are_complete_documents() -> TestResult {
    let fixture_dir = crate::play::screenshot::TestDir::new()?;
    let directory = fixture_dir.join("layout");
    let (_, path, layouts) = saved_market(&directory)?;
    let barrier = std::sync::Barrier::new(4);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..4)
            .map(|index| {
                let mut layouts = layouts.clone();
                let barrier = &barrier;
                let path = &path;
                scope.spawn(move || -> std::io::Result<()> {
                    layouts
                        .0
                        .get_mut("market")
                        .ok_or_else(|| std::io::Error::other("market"))?
                        .offset
                        .x = index as f32;
                    barrier.wait();
                    for _ in 0..20 {
                        let deadline =
                            std::time::Instant::now() + std::time::Duration::from_secs(1);
                        loop {
                            match layouts.save(path) {
                                Err(error)
                                    if error.kind() == io::ErrorKind::WouldBlock
                                        && std::time::Instant::now() < deadline =>
                                {
                                    std::thread::sleep(std::time::Duration::from_millis(1));
                                }
                                result => {
                                    result?;
                                    break;
                                }
                            }
                        }
                        WindowLayouts::load(path)?;
                    }
                    Ok(())
                })
            })
            .collect();
        for worker in workers {
            worker.join().map_err(|_| "writer panic")??;
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    Ok(())
}

#[test]
fn moving_market_preserves_concurrent_resize() -> TestResult {
    let fixture_dir = crate::play::screenshot::TestDir::new()?;
    let directory = fixture_dir.join("layout");
    let (mut app, path, mut saved) = saved_market(&directory)?;
    let panel = app.world_mut().spawn(window("market", default())).id();
    app.update();
    saved.0.get_mut("market").ok_or("market")?.size = Some(Vec2::new(1300., 500.));
    saved.save(&path)?;
    app.world_mut()
        .get_mut::<UiTransform>(panel)
        .ok_or("panel")?
        .translation = Val2::px(12., 34.);
    app.world_mut().write_message(AppExit::Success);
    app.update();
    let latest = WindowLayouts::load(&path)?.0["market"];
    assert_eq!(latest.offset, Vec2::new(12., 34.));
    assert_eq!(latest.size, saved.0["market"].size);
    Ok(())
}

#[test]
fn moving_prey_recovers_malformed_layout_on_exit() -> TestResult {
    let fixture_dir = crate::play::screenshot::TestDir::new()?;
    let directory = fixture_dir.join("layout");
    let (mut app, path, _) = saved_market(&directory)?;
    std::fs::write(&path, b"{")?;
    let panel = app.world_mut().spawn(window("prey", default())).id();
    app.update();
    app.world_mut()
        .get_mut::<UiTransform>(panel)
        .ok_or("panel")?
        .translation = Val2::px(12., 34.);
    app.world_mut().write_message(AppExit::Success);
    app.update();
    assert_eq!(
        WindowLayouts::load(&path)?.0["prey"].offset,
        Vec2::new(12., 34.)
    );
    Ok(())
}

#[test]
fn held_layout_lock_does_not_block_exit_schedule() -> TestResult {
    let fixture_dir = crate::play::screenshot::TestDir::new()?;
    let directory = fixture_dir.join("layout");
    let (mut app, path, saved) = saved_market(&directory)?;
    let panel = app.world_mut().spawn(window("market", default())).id();
    app.update();
    app.world_mut()
        .get_mut::<UiTransform>(panel)
        .ok_or("panel")?
        .translation = Val2::px(12., 34.);
    let lock = std::fs::File::options()
        .read(true)
        .write(true)
        .open(path.with_extension("json.lock"))?;
    lock.lock()?;
    let (finished, wait) = std::sync::mpsc::channel();
    let guard = std::thread::spawn(move || {
        let result = wait.recv_timeout(std::time::Duration::from_secs(1));
        drop(lock);
        result
    });
    app.world_mut().write_message(AppExit::Success);
    app.update();
    let still_saved = WindowLayouts::load(&path)?;
    finished.send(())?;
    guard.join().map_err(|_| "lock holder panicked")??;
    assert_eq!(still_saved, saved);
    app.world_mut().write_message(AppExit::Success);
    app.update();
    assert_eq!(
        WindowLayouts::load(&path)?.0["market"].offset,
        Vec2::new(12., 34.)
    );
    Ok(())
}

#[test]
fn bottom_grip_preserves_width_under_market_percentage_cap() -> TestResult {
    let fixture_dir = crate::play::screenshot::TestDir::new()?;
    let directory = fixture_dir.join("layout");
    let (mut app, path, _) = saved_market(&directory)?;
    WindowLayouts::default().save(&path)?;
    stacking::ui_layout(&mut app, UVec2::new(1024, 900));
    let screen = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1024, 900).into(),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    let panel = app
        .world_mut()
        .spawn(window(
            "market",
            Node {
                max_width: percent(96.),
                max_height: percent(94.),
                ..default()
            },
        ))
        .id();
    app.update();
    let width = app
        .world()
        .get::<ComputedNode>(panel)
        .ok_or("panel")?
        .size()
        .x;
    assert!((982. ..=984.).contains(&width), "{width}");
    drag_grip(&mut app, panel, IVec2::Y, Vec2::new(0., 66.))?;
    app.world_mut().write_message(AppExit::Success);
    app.update();
    assert_eq!(
        WindowLayouts::load(&path)?.0["market"].size,
        Some(Vec2::new(1000., 700.))
    );
    app.world_mut()
        .get_mut::<Window>(screen)
        .ok_or("screen")?
        .resolution
        .set(1400., 900.);
    app.update();
    assert_eq!(
        app.world().get::<Node>(panel).ok_or("panel")?.width,
        px(1000.)
    );
    Ok(())
}

#[test]
fn concurrent_market_resizes_and_moves_merge_each_axis() -> TestResult {
    let fixture_dir = crate::play::screenshot::TestDir::new()?;
    let directory = fixture_dir.join("layout");
    let (mut first, path, _) = saved_market(&directory)?;
    let (mut second, _, _) = saved_market(&directory)?;
    WindowLayouts::default().save(&path)?;
    for (app, side, distance, offset) in [
        (
            &mut first,
            IVec2::X,
            Vec2::new(100., 0.),
            Vec2::new(12., 0.),
        ),
        (
            &mut second,
            IVec2::Y,
            Vec2::new(0., 66.),
            Vec2::new(0., 34.),
        ),
    ] {
        stacking::ui_layout(app, UVec2::new(1600, 900));
        app.world_mut().spawn((
            Window {
                resolution: (1600, 900).into(),
                ..default()
            },
            PrimaryWindow,
        ));
        let panel = app.world_mut().spawn(window("market", default())).id();
        app.update();
        drag_grip(app, panel, side, distance)?;
        app.update();
        app.world_mut()
            .get_mut::<UiTransform>(panel)
            .ok_or("panel")?
            .translation = Val2::px(offset.x, offset.y);
    }
    for app in [&mut first, &mut second] {
        app.world_mut().write_message(AppExit::Success);
        app.update();
    }
    let market = WindowLayouts::load(&path)?.0["market"];
    assert_eq!(
        (market.size, market.offset),
        (Some(Vec2::new(1100., 700.)), Vec2::new(12., 34.))
    );
    Ok(())
}

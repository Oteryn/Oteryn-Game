//! Untouched real Source USE/barrier snapshots and independent cold wave/return epochs.
use super::*;
use crate::play::capture_test_support::{self as replay, Capture, Command, Epoch};
use oteryn_session::test_support::wire;

fn caps(value: &serde_json::Value) -> Result<Vec<u32>, ScopeError> {
    value["selected_capabilities"]
        .as_array()
        .ok_or("captured selected capabilities")?
        .iter()
        .map(|id| Ok(u32::try_from(id.as_u64().ok_or("captured capability")?)?))
        .collect()
}
fn view(epoch: &Epoch) -> Result<PlayView, ScopeError> {
    let mut view = PlayView::from_join(epoch.session.join_snapshot())?;
    view.set_live_map(
        &epoch
            .session
            .world_map_snapshot()
            .ok_or("actual server Map17")?,
    )?;
    Ok(view)
}
async fn enter(capture: &Capture) -> Result<(), ScopeError> {
    let metadata = capture.value["source_admission"].clone();
    let command = Command::read(&capture.value["entry_command"])?;
    let intent = wire::world_object::decode_use_intent_target(&caps(&metadata)?, &command.payload)
        .map_err(|error| format!("actual source USE payload: {error:?}"))?;
    let wire::world_object::UseTarget::Item(handle) = intent.target else {
        return Err("actual native core USE must use its displayed inventory handle".into());
    };
    let epoch = replay::epoch(
        metadata,
        capture.character,
        capture.frames("initial_frames")?,
        Some(command),
        capture.frames("entry_frames")?,
    )
    .await?;
    let mut view = view(&epoch)?;
    let (source, floor) = replay::position(&capture.value["source_position"])?;
    assert_eq!(view.own(), source);
    assert_eq!(view.floor, floor);
    let (link, commands, outbox) = play_channel();
    let Epoch {
        session,
        trigger,
        peer,
    } = epoch;
    let runner = tokio::spawn(run_session(session, commands, outbox));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !link
            .inventory_panel()
            .rows()
            .iter()
            .any(|row| row.handle == handle)
        {
            view.tick(&link)
                .map_err(|error| format!("actual source inventory: {error:?}"))?;
            tokio::task::yield_now().await;
        }
        Ok::<(), ScopeError>(())
    })
    .await
    .map_err(|_| "actual source inventory handle unavailable")??;
    assert!(link.inventory_select(handle));
    assert!(link.inventory_use_with());
    // Pinned Crystal source-room obelisk; exact placement/revision comes from captured Map17.
    assert!(view.use_with_click(TileCoord::new(32375, 31157), &link));
    view.in_flight = true;
    view.inputs
        .push_back(Input::Arrow(crate::input::StepDir::North));
    let (destination, floor) = replay::position(&capture.value["destination_position"])?;
    render_until(&mut view, &link, destination, "Entered boss instance").await?;
    assert_eq!(view.floor, floor);
    assert!(view.inputs.is_empty());
    assert!(!view.in_flight);
    assert!(link.inventory_panel().selected_item().is_none());
    assert!(view.scene().sprites().len() > 1);
    drop(link);
    runner.await?;
    trigger.send(()).await?;
    peer.await??;
    Ok(())
}
async fn wave(capture: &Capture, stage: &serde_json::Value) -> Result<(), ScopeError> {
    let raw = replay::frames(&stage["frames"])?;
    let (initial, updates) = replay::split_initial(&raw)?;
    let epoch = replay::epoch(
        stage["admission"].clone(),
        capture.character,
        initial,
        None,
        updates,
    )
    .await?;
    let mut view = view(&epoch)?;
    let (link, commands, outbox) = play_channel();
    let Epoch {
        session,
        trigger,
        peer,
    } = epoch;
    let runner = tokio::spawn(run_session(session, commands, outbox));
    // Any post-initial updates remain in this same captured cold epoch.
    trigger.send(()).await?;
    let wave = stage["wave"].as_u64().ok_or("actual wave number")?;
    let label = match wave {
        1 => "[ReducedHealth 1]",
        2 => "[ReducedHealth 5]",
        3 => "[ReducedHealth 15]",
        4 => "[ReducedHealth! 0]",
        _ => return Err("unexpected captured Soulpit wave".into()),
    };
    let (position, floor) = replay::position(&stage["position"])?;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            view.tick(&link)
                .map_err(|error| format!("actual wave render: {error:?}"))?;
            let scene = view.scene();
            if view.own() == position
                && view.floor == floor
                && scene.native_name_fragment_rendered(label)
            {
                assert!(scene.sprites().len() > 1);
                break;
            }
            tokio::task::yield_now().await;
        }
        Ok::<(), ScopeError>(())
    })
    .await
    .map_err(|_| format!("actual wave{wave} native badge not rendered: {label}"))??;
    drop(link);
    runner.await?;
    trigger.send(()).await?;
    peer.await??;
    Ok(())
}
async fn returned(capture: &Capture) -> Result<(), ScopeError> {
    if capture.value["autonomous_return"] != true || !capture.value["return_command"].is_null() {
        return Err("completed Soulpit capture must retain its autonomous return".into());
    }
    let (initial, subsequent) = replay::split_initial(&capture.frames("return_frames")?)?;
    let epoch = replay::epoch(
        capture.value["before_return_admission"].clone(),
        capture.character,
        initial,
        None,
        subsequent,
    )
    .await?;
    let mut view = view(&epoch)?;
    let (link, commands, outbox) = play_channel();
    let Epoch {
        session,
        trigger,
        peer,
    } = epoch;
    let runner = tokio::spawn(run_session(session, commands, outbox));
    trigger.send(()).await?;
    let (destination, floor) = replay::position(&capture.value["return_position"])?;
    render_until(&mut view, &link, destination, "Returned to channel").await?;
    assert_eq!(view.floor, floor);
    assert!(view.scene().sprites().len() > 1);
    drop(link);
    runner.await?;
    trigger.send(()).await?;
    peer.await??;
    Ok(())
}
#[test]
fn srx3_soulpit_actual_server_transcript_native_use_waves_icons_and_return()
-> Result<(), ScopeError> {
    let Some(capture) = Capture::load("SRX3_SOULPIT_CAPTURE_PATH")? else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            oteryn_client_runtime::ClientRuntime::new()?.block_on(async {
                enter(&capture).await?;
                let stages = capture.value["wave_stages"]
                    .as_array()
                    .ok_or("actual captured waves")?;
                assert_eq!(
                    stages.len(),
                    4,
                    "all donor Soulpit wave snapshots must be actual server data"
                );
                for stage in stages {
                    wave(&capture, stage).await?;
                }
                returned(&capture).await?;
                Ok::<(), ScopeError>(())
            })?
        })?
        .join()
        .map_err(|_| "actual Soulpit native replay thread panicked")??;
    eprintln!(
        "SRX3_SOULPIT ACTUAL_SERVER_NATIVE_PLAY_COMPLETED use,waves1-4,icons,autonomous_return duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[path = "actual_capture_exit.rs"]
mod early_exit;

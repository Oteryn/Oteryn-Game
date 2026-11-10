//! Two genuine socket epochs: retained inventory core USE, then native exit USE.
//! No frames, session UUIDs, counters or source identities are rewritten.
use super::*;
use oteryn_client_runtime::world_interaction::Action;

async fn gesture(capture: &Capture, key: &str, leaving: bool) -> Result<(), ScopeError> {
    let stage = &capture.value[key];
    let metadata = stage["admission"].clone();
    let command = Command::read(&stage["command"])?;
    let intent = wire::world_object::decode_use_intent_target(&caps(&metadata)?, &command.payload)
        .map_err(|e| format!("actual {key} USE: {e:?}"))?;
    let epoch = replay::epoch(
        metadata,
        capture.character,
        replay::frames(&stage["initial_frames"])?,
        Some(command),
        replay::frames(&stage["subsequent_frames"])?,
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
    if leaving {
        // This exact donor exit is resolved from the decoded current Map17,
        // including its genuine placement, revision and scoped frame binding.
        let tile = TileCoord::new(32375, 31153);
        let menu = view
            .world_context(tile, &link)
            .ok_or("actual Soulpit exit context absent")?;
        assert!(matches!(
            intent.target,
            wire::world_object::UseTarget::WorldObject(_)
        ));
        assert!(view.world_context_choose(&link, &menu, Action::Use));
    } else {
        let wire::world_object::UseTarget::Item(handle) = intent.target else {
            return Err("actual core entry must retain its inventory handle".into());
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !link
                .inventory_panel()
                .rows()
                .iter()
                .any(|row| row.handle == handle)
            {
                view.tick(&link)
                    .map_err(|e| format!("native core inventory: {e:?}"))?;
                tokio::task::yield_now().await;
            }
            Ok::<(), ScopeError>(())
        })
        .await
        .map_err(|_| "actual core handle unavailable")??;
        assert!(link.inventory_select(handle));
        assert!(link.inventory_use_with());
        assert!(view.use_with_click(TileCoord::new(32375, 31157), &link));
    }
    let (destination, label) = if leaving {
        (TileCoord::new(32373, 31158), tr!("Returned to the channel."))
    } else {
        (TileCoord::new(32373, 31151), tr!("Entered activity instance. Use the exit to return."))
    };
    render_until(&mut view, &link, destination, label).await?;
    assert_eq!(view.floor, -8);
    assert!(view.scene().sprites().len() > 1);
    drop(link);
    runner.await?;
    trigger.send(()).await?;
    peer.await??;
    Ok(())
}
#[test]
fn srx3_soulpit_actual_server_native_core_and_early_exit_gestures() -> Result<(), ScopeError> {
    let Some(capture) = Capture::load_schema(
        "SRX3_SOULPIT_EXIT_CAPTURE_PATH",
        "OTERYN_NATIVE_SOULPIT_EXIT_REPLAY/v1",
    )?
    else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            oteryn_client_runtime::ClientRuntime::new()?.block_on(async {
                gesture(&capture, "entry", false).await?;
                gesture(&capture, "exit", true).await?;
                Ok::<(), ScopeError>(())
            })?
        })?
        .join()
        .map_err(|_| "actual native Soulpit exit replay panicked")??;
    eprintln!(
        "SRX3_SOULPIT ACTUAL_SERVER_NATIVE_EARLY_EXIT_PLAY_COMPLETED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

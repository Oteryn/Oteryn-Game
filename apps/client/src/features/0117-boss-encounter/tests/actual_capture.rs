//! Actual production Primal transcripts: two live handoffs and one independent cold observation.
use super::*;
use crate::play::PlayView;
use crate::play::capture_test_support::{self as replay, Capture, Command, Epoch};
use oteryn_client_ui::boss_encounter::Intent;
use oteryn_session::world_map::MapDefinition;

async fn m25_gesture(capture: &Capture, entering: bool) -> Result<(), Error> {
    let (metadata, command, initial, subsequent, destination) = if entering {
        (
            capture.value["source_admission"].clone(),
            Command::read(&capture.value["entry_command"])?,
            capture.frames("initial_frames")?,
            capture.frames("entry_frames")?,
            "entry_position",
        )
    } else {
        (
            capture.value["exit_admission"].clone(),
            Command::read(&capture.value["exit_command"])?,
            capture.frames("before_exit_frames")?,
            capture.frames("exit_frames")?,
            "returned_position",
        )
    };
    let epoch = replay::epoch(
        metadata,
        capture.character,
        initial,
        Some(command.clone()),
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
    if entering {
        let key = oteryn_client_runtime::boss_encounter::decode_boss_key(&command.payload)
            .map_err(|error| format!("actual Ahau entry command: {error:?}"))?
            .boss_key;
        tokio::time::timeout(Duration::from_secs(10), async {
            while !link.boss_encounter_panel().selected {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        link.boss_encounter_edit(0, key);
        link.boss_encounter_request(Intent::Enter);
    } else {
        use crate::input::StepDir;
        use oteryn_session::test_support::wire::world_spatial::{self, StepDirection};
        let direction = world_spatial::decode_step_intent(&command.payload)
            .map_err(|error| format!("actual Ahau paced STEP: {error:?}"))?;
        let direction = match direction {
            StepDirection::North => StepDir::North,
            StepDirection::East => StepDir::East,
            StepDirection::South => StepDir::South,
            StepDirection::West => StepDir::West,
            StepDirection::NorthEast => StepDir::NorthEast,
            StepDirection::SouthEast => StepDir::SouthEast,
            StepDirection::SouthWest => StepDir::SouthWest,
            StepDirection::NorthWest => StepDir::NorthWest,
        };
        assert_eq!(
            capture.value["exit_trigger_source_position"],
            serde_json::json!([34002, 31706, 10])
        );
        assert_eq!(
            direction.from(view.own()),
            oteryn_renderer::TileCoord::new(34002, 31706)
        );
        view.arrow(direction);
    }
    let (at, floor) = replay::position(&capture.value[destination])?;
    if !entering {
        assert_eq!(at, oteryn_renderer::TileCoord::new(34036, 31717));
    }
    render_until(
        &mut view,
        &link,
        at,
        floor,
        if entering {
            "Entered boss instance"
        } else {
            "Returned to channel"
        },
    )
    .await?;
    assert!(
        view.scene().native_own_rendered(),
        "actual scope handoff must retain the player's native appearance pixels"
    );
    drop(link);
    runner.await?;
    trigger.send(()).await?;
    peer.await??;
    Ok(())
}

#[test]
fn m25_actual_server_ahau_entry_and_step_exit_render_in_native_client() -> Result<(), Error> {
    let Some(capture) = Capture::load("SRX3_M25_CAPTURE_PATH")? else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            oteryn_client_runtime::ClientRuntime::new()?.block_on(async {
                m25_gesture(&capture, true).await?;
                m25_gesture(&capture, false).await
            })?
        })?
        .join()
        .map_err(|_| "actual native Ahau replay panicked")??;
    eprintln!(
        "M25_ACTUAL_SERVER_NATIVE_ENTRY_STEP_EXIT_COMPLETED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn item_reference(capture: &Capture, key: &str) -> Result<u32, Error> {
    capture.value["native_item_refs"][key]
        .as_u64()
        .and_then(|reference| u32::try_from(reference).ok())
        .filter(|reference| *reference != 0)
        .ok_or_else(|| format!("missing capture-time native Item identity {key}").into())
}
fn view(epoch: &Epoch) -> Result<PlayView, Error> {
    let mut view = PlayView::from_join(epoch.session.join_snapshot())?;
    if let Some(entities) = epoch.session.world_entities() {
        view.set_own_identity(*entities.own_identity());
    }
    view.set_live_map(
        &epoch
            .session
            .world_map_snapshot()
            .ok_or("actual server Map17")?,
    )?;
    view.set_presentations(epoch.session.visible_appearances())?;
    let view = view.with_entities(
        epoch
            .session
            .world_entities()
            .map(|entities| entities.others().copied().collect())
            .unwrap_or_default(),
    )?;
    Ok(view)
}
async fn render_until(
    view: &mut PlayView,
    link: &PlayLink,
    at: oteryn_renderer::TileCoord,
    floor: i16,
    text: &str,
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            view.tick(link)
                .map_err(|e| format!("actual Primal view tick: {e:?}"))?;
            if view.own() == at && view.floor == floor && view.action_line == Some(text) {
                break;
            }
            tokio::task::yield_now().await;
        }
        Ok::<(), Error>(())
    })
    .await
    .map_err(|_| format!("actual Primal scope not rendered: {text}"))??;
    assert!(!view.scene().sprites().is_empty());
    assert!(!view.scene().tiles().is_empty());
    Ok(())
}
async fn handoff(capture: &Capture, entry: bool) -> Result<(), Error> {
    let (metadata, command, initial, subsequent) = if entry {
        (
            capture.value["source_admission"].clone(),
            Command::read(&capture.value["entry_command"])?,
            capture.frames("initial_frames")?,
            capture.frames("entry_frames")?,
        )
    } else {
        let (initial, subsequent) = replay::split_initial(&capture.frames("return_frames")?)?;
        (
            capture.value["before_return_admission"].clone(),
            Command::read(&capture.value["return_command"])?,
            initial,
            subsequent,
        )
    };
    let key = oteryn_client_runtime::boss_encounter::decode_boss_key(&command.payload)
        .map_err(|e| format!("actual captured boss key: {e:?}"))?
        .boss_key;
    let epoch = replay::epoch(
        metadata,
        capture.character,
        initial,
        Some(command),
        subsequent,
    )
    .await?;
    let captured_vitals = epoch
        .session
        .actor_vitals()
        .cloned()
        .ok_or("actual ungated ActorVitals domain3")?;
    let mut view = view(&epoch)?;
    if !entry {
        assert!(
            captured_vitals.health < captured_vitals.max_health,
            "real incoming encounter damage must be retained in the return epoch"
        );
        let reference = item_reference(capture, "corpse39529")?;
        let corpse = epoch
            .session
            .world_entities()
            .ok_or("actual terminal corpse domain1")?
            .others()
            .find(|entity| {
                entity.kind == oteryn_session::EntityKind::Corpse
                    && matches!(
                        entity.detail,
                        oteryn_session::EntityDetail::ObjectWithAppearance { item_definition_ref, quantity: 1, item_handle: Some(_), appearance_id }
                            if item_definition_ref == reference && appearance_id.get() == 39529
                    )
            })
            .map(|entity| entity.position)
            .ok_or("actual source corpse39529 missing from domain1 before return")?;
        assert!(
            view.scene().native_item_rendered_at(corpse, reference),
            "actual source corpse must resolve pinned pixels and enter native Scene before return"
        );
    }
    let source_key = if entry {
        "source_position"
    } else {
        "destination_position"
    };
    let (source, source_floor) = replay::position(&capture.value[source_key])?;
    assert_eq!((view.own(), view.floor), (source, source_floor));
    let (link, commands, outbox) = play_channel();
    let Epoch {
        session,
        trigger,
        peer,
    } = epoch;
    let runner = tokio::spawn(run_session(session, commands, outbox));
    tokio::time::timeout(Duration::from_secs(10), async {
        while !link.boss_encounter_panel().selected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|_| "actual native boss panel not initialized")?;
    link.boss_encounter_edit(0, key);
    link.boss_encounter_request(if entry {
        Intent::Enter
    } else {
        Intent::Defeated
    });
    let (at, floor) = replay::position(
        &capture.value[if entry {
            "destination_position"
        } else {
            "return_position"
        }],
    )?;
    let rendered = render_until(
        &mut view,
        &link,
        at,
        floor,
        if entry {
            "Entered boss instance"
        } else {
            "Returned to channel"
        },
    )
    .await;
    if let Err(error) = rendered {
        runner.abort();
        if peer.is_finished() {
            return Err(format!("{error}; actual transcript peer: {:?}", peer.await?).into());
        }
        peer.abort();
        return Err(error);
    }
    assert_eq!(view.actor_vitals(), Some(&captured_vitals));
    assert!(view.hud_status().contains(&format!(
        "{}/{}",
        captured_vitals.health, captured_vitals.max_health
    )));
    drop(link);
    runner.await?;
    trigger.send(()).await?;
    peer.await??;
    Ok(())
}
async fn live_pod(capture: &Capture) -> Result<(), Error> {
    let (initial, updates) = replay::split_initial(&capture.frames("update_frames")?)?;
    let epoch = replay::epoch(
        capture.value["update_admission"].clone(),
        capture.character,
        initial,
        None,
        updates,
    )
    .await?;
    let reference = item_reference(capture, "pod39176")?;
    let definition = MapDefinition::Item(reference.try_into()?);
    let observed_vitals = epoch
        .session
        .actor_vitals()
        .cloned()
        .ok_or("actual ungated ActorVitals domain3 in Pod epoch")?;
    let snapshot = epoch
        .session
        .world_map_snapshot()
        .ok_or("actual Pod Map17")?;
    let position = snapshot
        .tiles
        .iter()
        .find(|tile| tile.items.iter().any(|item| item.definition == definition))
        .map(|tile| tile.position)
        .ok_or("actual source Pod39176 absent from server snapshot")?;
    let mut view = view(&epoch)?;
    let (link, commands, outbox) = play_channel();
    let Epoch {
        session,
        trigger,
        peer,
    } = epoch;
    let runner = tokio::spawn(run_session(session, commands, outbox));
    trigger.send(()).await?;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            view.tick(&link)
                .map_err(|e| format!("actual Pod native view: {e:?}"))?;
            if view.scene().native_item_rendered_at(position, reference)
                && view.actor_vitals() == Some(&observed_vitals)
                && view.hud_status().contains(&format!(
                    "{}/{}",
                    observed_vitals.health, observed_vitals.max_health
                ))
            {
                break;
            }
            tokio::task::yield_now().await;
        }
        Ok::<(), Error>(())
    })
    .await
    .map_err(
        |_| "actual Pod39176 native pixels and initial Session HUD delivery did not both arrive",
    )??;
    assert!(view.scene().sprites().len() > 1);
    assert_eq!(view.actor_vitals(), Some(&observed_vitals));
    assert!(view.hud_status().contains(&format!(
        "{}/{}",
        observed_vitals.health, observed_vitals.max_health
    )));
    drop(link);
    runner.await?;
    trigger.send(()).await?;
    peer.await??;
    Ok(())
}
#[test]
fn srx3_primal_actual_server_transcript_native_entry_pod_and_return() -> Result<(), Error> {
    let Some(capture) = Capture::load("SRX3_PRIMAL_CAPTURE_PATH")? else {
        return Ok(());
    };
    let artifact = replay::hex(&capture.value["native_server_artifact_digest"])?;
    if artifact.len() != 32 || artifact.iter().all(|byte| *byte == 0) {
        return Err(
            "actual native identity metadata requires its captured server artifact digest".into(),
        );
    }
    if std::env::var_os("OTERYN_ASSET_DIR").is_none() {
        return Err("actual Primal pixel play requires cached pinned OTERYN_ASSET_DIR".into());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            oteryn_client_runtime::ClientRuntime::new()?.block_on(async {
                handoff(&capture, true).await?;
                live_pod(&capture).await?;
                handoff(&capture, false).await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "actual Primal native replay panicked")??;
    eprintln!(
        "SRX3_PRIMAL ACTUAL_SERVER_NATIVE_PLAY_COMPLETED entry178,pod39176,return179 separate_epochs duration={:?}",
        started.elapsed()
    );
    Ok(())
}

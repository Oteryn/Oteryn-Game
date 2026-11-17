//! B24: real canonical admission and pinned native mask -> live sound observer.
use super::cyclopedia::support;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::presentation::PresentationEventKind;

#[test]
fn reaudit_ambient_sounds_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let began = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "ambient scenario panicked")??;
    eprintln!("B24 PostgreSQL duration: {:?}", began.elapsed());
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "ambient_b24", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activation_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let world = super::subarea::world(&deps, generation)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        support::seed(&h, world.base().digest()).await?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activation_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (32362, 31009, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = (&cells).into();
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        assert!(owner.subarea_available());
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[6, 288],
            )
            .ok_or("ambient selection")?;
        let (_, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &[]).await?;
        let mut snapshot_seen = false;
        let mut bytes = output.as_slice();
        while bytes.len() >= 4 {
            let length = u32::from_be_bytes(bytes[..4].try_into()?) as usize;
            let envelope =
                crate::foundation::decode_wire_envelope(&bytes[4..4 + length]).map_err(debug)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                        .map_err(debug)?;
                for domain in domains {
                    assert!(![19, 20].contains(&domain.domain_id));
                    if domain.domain_id == 101 {
                        assert!(domain.payload.is_empty());
                        snapshot_seen = true;
                    }
                }
            }
            bytes = &bytes[4 + length..];
        }
        assert!(
            snapshot_seen,
            "the negotiated live connection sends the ambient snapshot"
        );
        let session = deps.admitted.game_session_id;
        let (cursor, initial) = owner
            .observe_sounds(deps.actor, session, u64::MAX)
            .await
            .ok_or("head")?;
        assert!(initial.is_empty());
        let (cursor, events) = owner
            .observe_sounds(deps.actor, session, cursor)
            .await
            .ok_or("ambient")?;
        // The pinned fixture is Nibelor: it must not borrow a city's ambience.
        assert!(matches!(
            events[0].kind,
            PresentationEventKind::AmbientSound { cue_id: 0 }
        ));
        assert!(
            owner
                .observe_sounds(deps.actor, session, cursor)
                .await
                .ok_or("repeat")?
                .1
                .is_empty()
        );
        // Drive the same producer with an admitted donor sound-zone input.
        let cursor = {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            states
                .sounds
                .ambient(&runtime, deps.actor, session, Some("thais-city"), 600);
            let (cursor, events) = states
                .sounds
                .observe(&runtime, deps.actor, session, cursor, true)
                .ok_or("day")?;
            assert!(matches!(
                events[0].kind,
                PresentationEventKind::AmbientSound { cue_id: 25 }
            ));
            cursor
        };
        {
            let mut runtime = owner.runtime.lock().await;
            let at = runtime.read_actor_position(deps.actor).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_cardinal(
                    at,
                    MovementLocalPosition {
                        x: 32361,
                        y: 31009,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
        }
        let (_, events) = owner
            .observe_sounds(deps.actor, session, cursor)
            .await
            .ok_or("exit")?;
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].kind,
            PresentationEventKind::AmbientSound { cue_id: 0 }
        ));
        Ok(())
    })
}

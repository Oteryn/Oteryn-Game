use super::house_transfer_fixtures::{
    RecoverySource, TRANSFER_TEST_HOUSE as HOUSE, initialize_transfer_source,
    prepare_owned_session, prepare_transfer_scene,
};
use super::*;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, ConnectionEnd, FreshAdmissionAttempt, FreshAdmissionAuthority, IDLE_LIVENESS,
    ResumeAttempt, serve_admitted,
};
use crate::gameplay_transport::{StepOutcome, WorldSpatialObservation};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
    encode_client_command,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

const MARKER_ONCE: &str = "house_input_marker_written_once_for_transfer_destination";
const OLD_VIEWS: &str = "house_recovery_fences_old_vitals_and_cooldowns";
const OLD_TRANSPORT: &str = "house_input_old_transport_refused_after_replacement";

#[test]
fn house_input_marker_written_once_for_transfer_destination() -> TestResult {
    scenario(MARKER_ONCE)
}

#[test]
fn house_input_old_transport_refused_after_replacement() -> TestResult {
    scenario(OLD_TRANSPORT)
}

#[test]
fn house_recovery_fences_old_vitals_and_cooldowns() -> TestResult {
    scenario(OLD_VIEWS)
}

fn scenario(name: &'static str) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "house_input_scope", true, false)
                .await?;
            eprintln!("hguard-marker: {name} db ready");
            let result = exercise(&h, name).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness, name: &str) -> TestResult {
    let evidence = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let (controller, cells, deps) = prepare_transfer_scene(h).await?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = controller.active();
    owner.movement_cells = (&cells).into();
    owner.evidence = &evidence.evidence;
    let source =
        initialize_transfer_source(h, &owner, &deps, cells.scope().generation_digest).await?;
    let mut body = vec![8, 1, 18, HOUSE.len() as u8];
    body.extend_from_slice(HOUSE.as_bytes());
    let (destination, _) = owner
        .house_transfer(source, 1, &body)
        .await
        .ok_or("house transfer")?;
    let destination = destination.ok_or("house transfer destination")?;
    assert_ne!(source.game_session_id, destination.game_session_id);
    assert_eq!(marker_count(h, destination.game_session_id).await?, 0);

    if name == OLD_VIEWS {
        return check_replaced_views(h, &owner, destination).await;
    }

    let probe = Probe {
        owner: &owner,
        h,
        steps: Cell::new(0),
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, destination, &probe, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let commands = async {
        read_message(&mut client, MessageType::SnapshotCommit).await?;
        let generation = destination.continuity.connection_generation;
        if name == OLD_TRANSPORT {
            let token = RecoverySource::token(7)?;
            let replacement = owner
                .resume(ResumeAttempt {
                    game_session_id: destination.game_session_id,
                    recovery_material: token.as_bytes(),
                    last_applied_server_sequence: 0,
                    supported_capabilities: destination.continuity.selected_capabilities.as_slice(),
                    transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[8; 16])?,
                })
                .await
                .map_err(debug)?;
            assert_eq!(replacement.game_session_id, destination.game_session_id);
            assert!(replacement.continuity.connection_generation > generation);
            send_step(&mut client, generation, 1).await?;
            let mut trailing = Vec::new();
            client.read_to_end(&mut trailing).await?;
            assert!(trailing.is_empty(), "replaced transport received a reply");
            assert_eq!(probe.steps.get(), 0);
            assert_eq!(marker_count(h, destination.game_session_id).await?, 0);
        } else {
            send_step(&mut client, generation, 1).await?;
            expect_step(&mut client, 1).await?;
            assert_eq!(marker_count(h, destination.game_session_id).await?, 1);

            // The real writer cannot enter while this pass is checked out.
            // A second INSERT ON CONFLICT would defer the command as well.
            let held = h.root.try_issue_semantic_pass()?;
            send_step(&mut client, generation, 2).await?;
            expect_step(&mut client, 2).await?;
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            assert_eq!(probe.steps.get(), 2);
            assert_eq!(marker_count(h, destination.game_session_id).await?, 1);
            client.shutdown().await?;
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, ()) = tokio::time::timeout(Duration::from_secs(15), async {
        tokio::try_join!(async { Ok(serving.await) }, commands)
    })
    .await??;
    assert!(
        matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
        "{end:?}"
    );
    Ok(())
}

struct Probe<'a, A> {
    owner: &'a A,
    h: &'a Harness,
    steps: Cell<usize>,
}

impl<A: FreshAdmissionAuthority> FreshAdmissionAuthority for Probe<'_, A> {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        self.owner.observe(actor).await
    }

    async fn house_scope_ack(
        &self,
        admitted: AdmittedSession,
    ) -> Result<Option<(u64, Vec<u8>)>, ()> {
        self.owner.house_scope_ack(admitted).await
    }

    async fn house_transfer_input(
        &self,
        admitted: AdmittedSession,
    ) -> Result<(), crate::durability::DurabilityError> {
        self.owner.house_transfer_input(admitted).await
    }

    async fn paced_step(
        &self,
        _: ExactActorRef,
        session: GameSessionId,
        _: StepDirection,
    ) -> (StepOutcome, Option<Duration>) {
        assert!(
            matches!(marker_count(self.h, session).await, Ok(1)),
            "dispatch must see the committed first-input receipt"
        );
        self.steps.set(self.steps.get() + 1);
        (
            StepOutcome {
                disposition: StepDisposition::Blocked,
                moved_to: None,
            },
            None,
        )
    }
}

async fn marker_count(h: &Harness, session: GameSessionId) -> TestResult<i64> {
    let mut key = b"house-transfer-input-v1".to_vec();
    key.extend_from_slice(session.as_bytes());
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_durability_admission_lifecycle_receipts WHERE operation_key=$1",
    )
    .bind(key)
    .fetch_one(&h.pool)
    .await?)
}

async fn send_step(client: &mut DuplexStream, generation: u64, id: u64) -> TestResult {
    let frame = encode_client_command(
        generation,
        &ClientCommandValue {
            command_id: id,
            command_type: COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            payload: &encode_step_intent(StepDirection::East),
        },
    )?;
    client.write_u32(frame.len() as u32).await?;
    client.write_all(&frame).await?;
    Ok(())
}

async fn read_message(client: &mut DuplexStream, kind: MessageType) -> TestResult<Vec<u8>> {
    loop {
        let len = client.read_u32().await? as usize;
        let mut frame = vec![0; len];
        client.read_exact(&mut frame).await?;
        if decode_wire_envelope(&frame)?.message_type() == kind {
            return Ok(frame);
        }
    }
}

async fn expect_step(client: &mut DuplexStream, id: u64) -> TestResult {
    let frame = read_message(client, MessageType::CommandResult).await?;
    let result = decode_command_result(decode_wire_envelope(&frame)?.payload())?;
    assert_eq!(result.command_id, id);
    assert_eq!(result.status, CommandStatus::Accepted, "STEP {id}");
    assert_eq!(
        decode_step_result(result.payload),
        Ok(StepDisposition::Blocked)
    );
    Ok(())
}

async fn check_replaced_views(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    destination: AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use oteryn_protocol_oteryn::presentation as wire;
    let offered =
        crate::gameplay_transport::native_cue_offer::offered(owner.offered_capabilities(), true);
    let selected = SelectedCapabilities::select(&offered, &[6, wire::CAPABILITY_PRESENTATION_V1])
        .ok_or("presentation selection")?;
    let mut old = destination;
    old.continuity.selected_capabilities = selected;
    let actor = old.runtime_actor.ok_or("house actor")?;
    let generation = old.continuity.connection_generation;
    let (vitals, feed) = owner.observe_join_vitals(actor, old, true).await;
    assert!(vitals.is_some(), "original House vitals");
    let feed = feed.ok_or("original House cooldown feed")?;

    use oteryn_gameplay_domain::domain::hireling::food::{GourmetBuff, GourmetConditions};
    let saved = GourmetConditions {
        attributes: Some(GourmetBuff {
            item_id: 29409,
            remaining_ms: 60_000,
        }),
        haste: None,
    };
    let session = old.game_session_id;
    let fence = old.item_fence.ok_or("original House fence")?;
    // Fixture only. The service tick still runs the production checkpoint
    // transaction with its normal triggers and authority checks.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_hireling_gourmet_state
        (character_id, last_transaction_id, cooldown_until, attributes_item, attributes_ms, haste_ms)
        VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, now(), 29409, 60000, 0)")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(id(177).as_slice())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let clock = super::srx3_native_source_cast_fixtures::NativeOwnerClock::new(owner);
    let checkpoint_at = (owner.owner_now().get() / 1000 + 1) * 1000;
    clock.set(checkpoint_at);
    {
        let mut states = owner.states.spell_states.lock().await;
        let state = &mut states
            .actors
            .iter_mut()
            .find(|(a, s, _)| *a == actor && *s == session)
            .ok_or("retained House actor")?
            .2;
        crate::spell::actor_conditions::restore_gourmet_food(state, saved, checkpoint_at / 1000)
            .map_err(debug)?;
        crate::gameplay_transport::item_use::gourmet::remember(
            &mut states,
            actor,
            session,
            saved,
            checkpoint_at,
        );
    }

    let token = RecoverySource::token(7)?;
    let mut replacement = owner
        .resume(ResumeAttempt {
            game_session_id: destination.game_session_id,
            recovery_material: token.as_bytes(),
            last_applied_server_sequence: 0,
            supported_capabilities: destination.continuity.selected_capabilities.as_slice(),
            transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[8; 16])?,
        })
        .await
        .map_err(debug)?;
    assert_eq!(replacement.runtime_actor, Some(actor));
    assert_eq!(replacement.continuity.connection_generation, generation + 1);
    let stale_drain = feed.take(generation, None).is_some();
    drop(feed);
    let (vitals, feed) = owner.observe_join_vitals(actor, old, true).await;
    assert_eq!(
        (vitals.is_some(), feed.is_some(), stale_drain),
        (false, false, false),
        "the replaced socket must lose join vitals, registration, and its old mailbox",
    );
    assert!(owner.observe_house_vitals(actor, old).await.is_none());
    clock.set(checkpoint_at + 60_001_000);
    assert!(owner.tick_house_vitals(actor, old).await.is_none());
    let read = || {
        owner
            .root
            .read_gourmet_conditions(owner.character, fence.character_id)
    };
    owner.root.await_ready().await?;
    assert_eq!(
        read().await.map_err(debug)?,
        saved,
        "stale House tick changed storage"
    );
    owner.root.await_ready().await?;
    assert!(owner.tick_house_vitals(actor, replacement).await.is_some());
    owner.root.await_ready().await?;
    assert_eq!(
        read().await.map_err(debug)?,
        GourmetConditions::default(),
        "current House tick checkpoints"
    );
    replacement.continuity.selected_capabilities = selected;
    let (vitals, feed) = owner.observe_join_vitals(actor, replacement, true).await;
    assert!(vitals.is_some());
    assert!(feed.is_some());
    Ok(())
}

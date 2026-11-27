//! The listener's one-bootstrap path uses the real fresh owner and mandatory snapshot reads.
use super::fixture::{Harness, debug, id};
use super::house_transfer_fixtures::RecoverySource;
use super::{Dependencies, TestResult};
use crate::durability::DurabilityRoot;
use crate::foundation::{AuthenticatedTransportRefV1, CharacterId, GameSessionId};
use crate::gameplay_transport::connection::{
    BEFORE_JOIN_PHASE, ConnectionEnd, ConnectionIdentifiers, FreshAdmissionAuthority,
    IDLE_LIVENESS, LogoutReply, admit_frame_for_join, serve_admitted_for_join,
};
use crate::gameplay_transport::tcp_tls::read_frame;
use oteryn_protocol_oteryn::actor_spell_catalog::CAPABILITY_ACTOR_SPELL_CATALOG_V1;
use oteryn_protocol_oteryn::attack::CAPABILITY_ATTACK_V1;
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_protocol_oteryn::pvp_expert::CAPABILITY_PVP_EXPERT_V1;
use oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;
use oteryn_protocol_oteryn::{
    ClientBootstrapValue, FoundationProtocolError, MessageType, decode_protocol_error,
    decode_wire_envelope, encode_client_bootstrap,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt as _;
use tokio::sync::Notify;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Relog,
    Snapshot,
    Exhausted,
    Catalogue,
    PeerField,
    Soulpit,
    InitialRaw,
}

#[test]
fn w13_logout_then_immediate_relog_waits_for_the_holder_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Relog)
}

#[test]
fn w13_initial_join_snapshot_waits_for_the_holder_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Snapshot)
}

#[test]
fn w13_join_refuses_with_typed_holder_cause_at_deadline_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Exhausted)
}

#[test]
fn w13_blocked_live_catalogue_releases_holder_for_join_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Catalogue)
}

#[test]
fn w13_peer_field_turn_waiting_on_runtime_leaves_holder_for_join_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::PeerField)
}

#[test]
fn w13_expert_join_waits_for_soulpit_handoff_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Soulpit)
}

#[test]
fn w13_initial_join_wakes_on_raw_holder_drop_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::InitialRaw)
}

fn run(admin: String, case: Case) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("w13-join-release", move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let mut result = Ok(());
        let lines = oteryn_error_codes::capture(|| {
            result = runtime.block_on(scenario(admin, case));
        });
        result?;
        let phase = match case {
            Case::Soulpit => "expert_fight_modes",
            Case::InitialRaw => "pvp_refresh",
            _ => "spell_catalog",
        };
        if case != Case::PeerField {
            assert!(
                lines
                    .iter()
                    .any(|line| line.contains("event=admission_retry")
                        && line.contains(&format!("phase={phase}"))
                        && line.contains("cause=holder_busy"))
            );
        }
        if matches!(case, Case::Soulpit | Case::InitialRaw) {
            let expected_owner = if case == Case::Soulpit {
                "activity_handoff/soulpit.rs"
            } else {
                "w13_join_release.rs"
            };
            assert!(
                lines
                    .iter()
                    .any(|line| line.contains("event=holder_wait_begin")
                        && line.contains(&format!("phase={phase}"))
                        && line.contains("state=checked_out")
                        && line.contains("owner=")
                        && line.contains(expected_owner)),
                "{lines:?}"
            );
            assert!(
                !lines.iter().any(|line| line.contains("state=untracked")
                    || line.contains("event=holder_wait_timeout")),
                "{lines:?}"
            );
        }
        if case == Case::Relog {
            assert!(
                lines
                    .iter()
                    .any(|line| line.contains("event=admission_retry")
                        && line.contains("phase=fresh_admission")
                        && line.contains("cause=holder_busy"))
            );
        }
        if case == Case::Exhausted {
            assert!(lines.iter().any(|line| line.contains("event=serve_end")
                && line.contains("reason=join_refused")
                && line.contains("phase=spell_catalog")
                && line.contains("cause=holder_busy")
                && line.contains("owner=")
                && line.contains("w13_join_release.rs")));
            assert!(
                lines
                    .iter()
                    .any(|line| line.contains("event=holder_wait_timeout")
                        && line.contains("phase=spell_catalog")
                        && line.contains("cause=holder_busy")
                        && line.contains("state=checked_out")
                        && line.contains("owner=")
                        && line.contains("w13_join_release.rs")
                        && line.contains("held_ms="))
            );
        }
        Ok(())
    })
}

struct Identifiers(u8);

impl ConnectionIdentifiers for Identifiers {
    fn game_session_id(&self) -> Option<GameSessionId> {
        GameSessionId::decode(&id(self.0)).ok()
    }

    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
        AuthenticatedTransportRefV1::decode(&[self.0; 16]).ok()
    }
}

fn bootstrap(nonce: u8, supported: &[u32]) -> TestResult<Vec<u8>> {
    let token = RecoverySource::fresh_token(nonce)?;
    encode_client_bootstrap(&ClientBootstrapValue {
        schema_revision: 1,
        character_id: CharacterId::decode(&id(41)).map_err(debug)?,
        admission_material: token.as_bytes(),
        client_build_id: "w13-join-release",
        supported_capabilities: supported,
    })
    .map_err(debug)
}

async fn scenario(admin: String, case: Case) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "w13_join_release", true, false).await?;
    let result = exercise(&h, case).await;
    let cleanup = h.cleanup().await;
    result?;
    cleanup
}

async fn exercise(h: &Harness, case: Case) -> TestResult {
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
    };

    let source = RecoverySource::start(h).await?;
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    )?;
    let mut readiness = super::fresh_fixture::fresh::Source::new(now)?;
    readiness
        .rows
        .retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    h.root
        .publish_runtime_readiness(
            &h.node,
            &AdmissionAuthorityPublicationV1::prepare(&readiness, now).map_err(debug)?,
        )
        .await
        .map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &source.evidence;

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    if matches!(case, Case::Relog | Case::Snapshot) {
        let frame = bootstrap(91, &[])?;
        let (old, pending) = admit_frame_for_join(&mut server, &frame, &owner, &Identifiers(91))
            .await
            .map_err(debug)?;
        assert!(pending.is_some(), "the old actor is a normal Channel entry");
        // Exercise the terminal release owner and assert its exact successful result.
        assert_eq!(
            owner.logout(old).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        let terminal: bool = sqlx::query_scalar(
            "SELECT session_state=3 FROM game_durability_reconnect_sessions
                WHERE game_session_id=encode($1,'hex')::uuid",
        )
        .bind(old.game_session_id.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert!(terminal);
    }

    let supported = if case == Case::Soulpit {
        vec![
            CAPABILITY_ACTOR_SPELL_CATALOG_V1,
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
            CAPABILITY_ATTACK_V1,
            CAPABILITY_PVP_EXPERT_V1,
        ]
    } else {
        vec![CAPABILITY_ACTOR_SPELL_CATALOG_V1]
    };
    let frame = bootstrap(92, &supported)?;
    let ids = Identifiers(92);
    let (admitted, mut pending) = {
        let mut admission = std::pin::pin!(DurabilityRoot::with_holder_owner(
            admit_frame_for_join(&mut server, &frame, &owner, &ids),
        ));
        if case == Case::Relog {
            let holder = h.root.try_issue_semantic_pass()?;
            assert!(
                tokio::time::timeout(Duration::from_millis(100), &mut admission)
                    .await
                    .is_err()
            );
            holder.run(|_, _| Box::pin(async { Ok(()) })).await?;
        }
        admission.await.map_err(debug)?
    };
    assert!(supported.iter().all(|&capability| {
        admitted
            .continuity
            .selected_capabilities
            .contains(capability)
    }));
    assert!(
        pending.is_some(),
        "normal fresh Channel admission defers acceptance"
    );

    if case == Case::InitialRaw {
        // Exercise the uncached PvP read during the initial ACTOR_VITALS observation.
        owner.states.spell_states.lock().await.pvp_locks.clear();
    }
    let barrier = Arc::new((Notify::new(), Notify::new()));
    let phase = match case {
        Case::Soulpit => "expert_fight_modes",
        Case::InitialRaw => "actor_vitals",
        Case::PeerField => "join_snapshot",
        _ => "spell_catalog",
    };
    let returned = Notify::new();
    let finished = Notify::new();
    let snapshot_committed = Notify::new();
    let serving = async {
        let serve =
            serve_admitted_for_join(&mut server, admitted, &owner, IDLE_LIVENESS, &mut pending);
        let end = DurabilityRoot::with_holder_owner(
            BEFORE_JOIN_PHASE.scope((phase, barrier.clone()), serve),
        )
        .await;
        finished.notify_one();
        end
    };
    let mut catalogue_lock = if case == Case::Catalogue {
        Some(block_catalogue(h, admitted.world_id).await?)
    } else {
        None
    };
    let contending = async {
        use std::future::{Future, poll_fn};
        use std::task::Poll;

        barrier.0.notified().await;
        if case == Case::Soulpit {
            let mut block = h.pool.begin().await?;
            sqlx::query("LOCK TABLE game_soulpit_encounters IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *block)
                .await?;
            let handoff =
                crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone());
            let discover =
                DurabilityRoot::with_background_holder(h.root.with_background_operation(async {
                    let result = handoff
                        .active_soulpits(&authority, &h.node, admitted.world_id, None)
                        .await;
                    assert!(result?.is_empty());
                    // The same scheduler's granted follow-up can take the holder only
                    // after the already-waiting join has had its checkout.
                    assert!(matches!(
                        h.root.try_acquire_ready(),
                        Err(crate::durability::DurabilityError::RootUnavailable)
                    ));
                    Ok::<_, Box<dyn std::error::Error>>(())
                }));
            let mut discover = std::pin::pin!(discover);
            assert!(
                poll_fn(|cx| Poll::Ready(discover.as_mut().poll(cx)))
                    .await
                    .is_pending()
            );
            assert!(
                h.root
                    .checked_out_pass_site()
                    .is_some_and(|site| site.file().ends_with("activity_handoff/soulpit.rs"))
            );
            barrier.1.notify_one();
            let held = async {
                assert!(
                    tokio::time::timeout(Duration::from_millis(350), finished.notified())
                        .await
                        .is_err()
                );
                block.rollback().await?;
                Ok::<_, Box<dyn std::error::Error>>(())
            };
            tokio::try_join!(discover, held)?;
            returned.notify_one();
            return Ok(());
        }
        if case == Case::InitialRaw {
            let holder = h.root.try_acquire_ready()?;
            barrier.1.notify_one();
            assert!(
                tokio::time::timeout(Duration::from_millis(350), finished.notified())
                    .await
                    .is_err()
            );
            let wake = h.root.holder_return_notification();
            drop(holder);
            tokio::time::timeout(Duration::from_secs(2), wake).await?;
            returned.notify_one();
            return Ok(());
        }
        if case == Case::PeerField {
            let peer_dependencies = Dependencies::new()?;
            let peer = peer_dependencies.owner(h, &authority);
            let blocked = peer.runtime.lock().await;
            let mut step = std::pin::pin!(peer.lock_field_step_turn());
            assert!(
                poll_fn(|cx| Poll::Ready(step.as_mut().poll(cx)))
                    .await
                    .is_pending()
            );
            assert!(
                h.root.is_ready(),
                "the blocked peer must not take the holder"
            );
            barrier.1.notify_one();

            // Hold the peer runtime until the actual client observes its snapshot.
            snapshot_committed.notified().await;
            drop(blocked);
            let (runtime, states, pass) = step.await.map_err(debug)?;
            drop((runtime, states));
            pass.run(|_, _| Box::pin(async { Ok(()) })).await?;
            returned.notify_one();
            return Ok::<_, Box<dyn std::error::Error>>(());
        }
        if case == Case::Catalogue {
            use crate::durability::public_projections::house_catalogue::PublicHouseCatalogue;
            let mut publishing = std::pin::pin!(DurabilityRoot::with_holder_owner(
                h.root.sync_public_house_catalogue(
                    &authority,
                    &h.node,
                    PublicHouseCatalogue {
                        world: admitted.world_id,
                        channel: admitted.channel_id,
                        generation: 1,
                        reset_epoch: 0,
                        map_revision: "map-1".into(),
                        houses: None,
                    }
                ),
            ));
            assert!(
                poll_fn(|cx| Poll::Ready(publishing.as_mut().poll(cx)))
                    .await
                    .is_pending()
            );
            let deadline = h
                .root
                .checked_out_pass_deadline()
                .ok_or("catalogue checkout")?;
            assert!(deadline <= tokio::time::Instant::now() + crate::durability::DB_PASS_DEADLINE);
            barrier.1.notify_one();
            assert!(
                publishing.await.is_err(),
                "the SQL lock remains held until after join"
            );
            returned.notify_one();
            return Ok::<_, Box<dyn std::error::Error>>(());
        }
        let holder = h.root.try_issue_semantic_pass()?;
        barrier.1.notify_one();
        if case == Case::Exhausted {
            finished.notified().await;
        } else {
            assert!(
                tokio::time::timeout(
                    crate::gameplay_transport::COMMAND_HOLDER_WINDOW + Duration::from_millis(250),
                    finished.notified(),
                )
                .await
                .is_err(),
                "snapshot refused while the holder was still settling"
            );
        }
        if case == Case::Exhausted {
            drop(holder);
            h.root.await_ready().await?;
        } else {
            holder.run(|_, _| Box::pin(async { Ok(()) })).await?;
        }
        returned.notify_one();
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let receiving = async {
        let first = read_frame(&mut client).await?;
        let envelope = decode_wire_envelope(&first).map_err(debug)?;
        if case == Case::Exhausted {
            assert_eq!(envelope.message_type(), MessageType::ProtocolError);
            assert_eq!(envelope.connection_generation(), 0);
            assert_eq!(
                decode_protocol_error(envelope.payload())
                    .map_err(debug)?
                    .error_code,
                FoundationProtocolError::SessionJoinUnavailable.code()
            );
        } else {
            assert_eq!(envelope.message_type(), MessageType::ServerAccepted);
            loop {
                let frame = read_frame(&mut client).await?;
                match decode_wire_envelope(&frame).map_err(debug)?.message_type() {
                    MessageType::SnapshotCommit => break,
                    MessageType::SnapshotBegin | MessageType::SnapshotChunk => {}
                    kind => return Err(format!("unexpected snapshot frame {kind:?}").into()),
                }
            }
        }
        snapshot_committed.notify_one();
        returned.notified().await;
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, contend, receive) = tokio::time::timeout(Duration::from_secs(22), async {
        tokio::join!(serving, contending, receiving)
    })
    .await?;
    if let Some(tx) = catalogue_lock.take() {
        tx.rollback().await?;
    }
    contend?;
    receive?;
    if case == Case::Exhausted {
        assert!(matches!(
            end,
            ConnectionEnd::AdmittedThenClosed(_, FoundationProtocolError::SessionJoinUnavailable)
        ));
        assert!(pending.is_some());
    } else {
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        assert!(pending.is_none());
    }
    Ok(())
}

async fn block_catalogue(
    h: &Harness,
    world: crate::foundation::WorldId,
) -> TestResult<sqlx::Transaction<'static, sqlx::Postgres>> {
    sqlx::query("INSERT INTO game_world_reset_epochs
        (world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month)
        VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),
            to_char((clock_timestamp() AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM'))
        ON CONFLICT (world_id,reset_epoch) DO NOTHING")
        .bind(world.as_bytes().as_slice())
        .bind([7_u8; 32].as_slice())
        .execute(&h.pool).await?;
    sqlx::query(
        "INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)
        ON CONFLICT (world_id) DO NOTHING",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("LOCK TABLE game_public_house_catalogue_state IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

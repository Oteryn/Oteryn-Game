//! W18: house-podium commands through framed ingress, composed dispatch, and PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::house_transfer_fixtures::{RecoverySource, prepare_owned_session};
use super::{Dependencies, TestResult};
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::house_scope_handoff::{
    HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentRequest,
};
use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelContentPin, CharacterId, CharacterLease, MessageType,
    MovementLocalPosition, decode_wire_envelope,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, ControlLossResult, FirstEntryOutcome, FreshAdmissionAuthority,
    IDLE_LIVENESS, ResumeAttempt, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, StepDirection,
    encode_step_intent,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
    house_podium as wire,
};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";
const POSITION: [i32; 3] = [32310, 32804, 7];
const BOSS: &str = "oteryn:creature.ferumbras";
const RAT: &str = "oteryn:creature.rat";

#[derive(Clone, Copy)]
enum Case {
    Roundtrip { boss: bool },
    FullRenown,
    Refusals,
    MissingCapability,
    Walk,
    CancelWalk,
    Malformed,
    Unavailable,
    GuardBusy(u32),
    GuardBusyResume,
    NoPath,
}

#[test]
fn house_podium_dispatch_query_set_projection_roundtrip_postgres() -> TestResult {
    postgres_case("house_podium_roundtrip", Case::Roundtrip { boss: false })
}

#[test]
fn house_podium_dispatch_vigour_requires_expertise_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    postgres_case("house_podium_vigour", Case::Roundtrip { boss: true })
}

#[test]
fn house_podium_dispatch_full_renown_roundtrip_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    postgres_case("house_podium_renown", Case::FullRenown)
}

#[test]
fn house_podium_dispatch_refusals_leave_display_unchanged_postgres() -> TestResult {
    postgres_case("house_podium_refusals", Case::Refusals)
}

#[test]
fn house_podium_dispatch_without_capability_150_is_rejected_postgres() -> TestResult {
    postgres_case("house_podium_without_capability", Case::MissingCapability)
}

#[test]
fn house_podium_dispatch_walks_before_querying_distant_podium() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    postgres_case("house_podium_walk", Case::Walk)
}

#[test]
fn house_podium_dispatch_manual_move_cancels_pending_query() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    postgres_case("house_podium_cancel_walk", Case::CancelWalk)
}

#[test]
fn house_podium_dispatch_malformed_query_closes_connection() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    postgres_case("house_podium_malformed", Case::Malformed)
}

#[test]
fn house_podium_dispatch_unavailable_target_sends_no_result() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    postgres_case("house_podium_unavailable", Case::Unavailable)
}

#[test]
fn house_podium_dispatch_busy_guard_sends_no_result() -> TestResult {
    postgres_case(
        "house_podium_guard_busy",
        Case::GuardBusy(wire::COMMAND_QUERY),
    )
}

#[test]
fn house_podium_dispatch_busy_guard_sends_no_projection_result() -> TestResult {
    postgres_case(
        "house_podium_projection_guard_busy",
        Case::GuardBusy(wire::COMMAND_PROJECTION),
    )
}

#[test]
fn house_podium_dispatch_busy_guard_allows_same_session_resume() -> TestResult {
    postgres_case("house_podium_guard_resume", Case::GuardBusyResume)
}

#[test]
fn house_podium_dispatch_valid_query_without_path_rejects() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    postgres_case("house_podium_no_path", Case::NoPath)
}

fn postgres_case(name: &'static str, case: Case) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            run(async |admin| {
                let started = std::time::Instant::now();
                let h = Harness::create_with_admission_seed(admin, name, true, false).await?;
                let result = scenario(&h, case).await;
                h.cleanup().await?;
                result?;
                eprintln!(
                    "POSTGRES_CASE_COMPLETED {name} elapsed={:?}",
                    started.elapsed()
                );
                Ok::<_, Box<dyn std::error::Error>>(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "house podium dispatch thread panicked")??;
    Ok(())
}

async fn scenario(h: &Harness, case: Case) -> TestResult {
    super::reset();
    let evidence = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let base = Dependencies::new()?;
    let walks = matches!(case, Case::Walk | Case::CancelWalk | Case::GuardBusyResume);
    let world = if walks {
        super::house_wrap_fixtures::walkable_house_bundle(
            base.admitted.world_id,
            base.admitted.channel_id,
        )?
    } else {
        super::house_wrap_fixtures::wrap_bundle(base.admitted.world_id, base.admitted.channel_id)?
    };
    let mut packet = cosmetics_test_packet();
    packet["authoring"]["document"]["content_revision"] = "content-1".into();
    let full_renown = matches!(case, Case::FullRenown);
    let mount_key = "oteryn:content.mount.proposed";
    if full_renown {
        packet = serde_json::from_str(&serde_json::to_string(&packet)?
            .replace("oteryn:outfit.proposed", "oteryn:content.outfit.proposed")
            .replace("oteryn:mount.proposed", mount_key))?;
        packet["authoring"]["document"]["grant_mappings"] = serde_json::json!([
            {"grant_key":"podium-mount", "target":{"kind":"Mount","mount_key":mount_key},
             "compatible_provenance":["content-1"]}
        ]);
    }
    let outfit_key = packet["authoring"]["document"]["outfits"][0]["key"]
        .as_str()
        .ok_or("cosmetics outfit key")?
        .to_owned();
    let bytes = serde_json::to_vec(&packet)?;
    let (controller, native_pin) =
        activated_with_optional_cosmetics(base.admitted.world_id, None, Some(&bytes));
    let room = if walks {
        native_pin.qualified_room().clone()
    } else {
        base.room
    };
    let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
    let active = controller.active().ok_or("active content")?;
    let pin = ChannelContentPin::from_activation(
        base.admitted.world_id,
        1,
        active.identity().server_artifact_digest(),
        [2; 32],
        [3; 32],
        [4; 32],
        (32309, 32805, 7),
    );
    let deps = Dependencies::with_content(pin, room, base.spells)?;
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = (&cells).into();
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    const OFFERS: &[crate::gameplay_transport::capabilities::OfferedCapability] = &[
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 4,
            requires: &[6],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 6,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 49,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 86,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 150,
            requires: &[49],
        },
        crate::gameplay_transport::capabilities::OfferedCapability { id: 489, requires: &[150] },
        crate::gameplay_transport::capabilities::OfferedCapability { id: 631, requires: &[489] },
    ];
    owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        &[], crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id, deps.admitted.channel_id), 1, "policy-1", OFFERS,
    ).map_err(debug)?;
    if walks {
        super::tutorial_vitals_fixture::publish_native_activation(h, &native_pin).await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
    }
    deps.initialize(&owner).await;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 32309,
                    y: 32805,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    sqlx::query(include_str!(
        "../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants SELECT session_user,encode($1,'hex')::uuid,$2,1")
        .bind(id(42).as_slice()).bind(HOUSE).execute(&h.pool).await?;
    h.root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([82; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Assign {
                house: HouseId::new(deps.admitted.world_id, HOUSE).ok_or("house")?,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(deps.admitted.game_session_id)
        .await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    let selected: &[u32] = if matches!(case, Case::MissingCapability) {
        &[4, 6, 49, 86]
    } else if full_renown {
        &[4, 6, 49, 86, 150, 489, 631]
    } else {
        &[4, 6, 49, 86, 150]
    };
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), selected)
            .ok_or("house podium offer")?;
    let mut body = vec![8, 1, 18, HOUSE.len() as u8];
    body.extend(HOUSE.as_bytes());
    let (inside, _) = owner
        .transfer_house(admitted, 2, &body)
        .await
        .ok_or("house transfer outcome")?;
    let mut admitted = inside.ok_or("house transfer admission")?;

    let boss = matches!(case, Case::Roundtrip { boss: true });
    let (item_key, podium_kind, display_key) = if full_renown {
        ("oteryn:item.tibia.i35973", 1, RAT)
    } else if boss {
        ("oteryn:item.tibia.i38707", 2, BOSS)
    } else {
        ("oteryn:item.tibia.i42367", 3, RAT)
    };
    super::item_look_fixtures::seed_main_backpack(h, &[]).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$4,'definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(id(73).as_slice()).bind(item_key).execute(&mut *tx).await?;
    let position = if walks {
        let actor = admitted.runtime_actor.ok_or("house actor")?;
        let state = owner
            .runtime
            .lock()
            .await
            .house_actor_state(actor, admitted.game_session_id)
            .map_err(debug)?;
        assert_eq!((state.position.x, state.position.y), (32309, 32804));
        for x in [32310_i32, 32311] {
            let intermediate = [x, 32804, 7].map(i32::to_be_bytes).concat();
            sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
                .bind(id(42).as_slice()).bind(HOUSE).bind(intermediate).execute(&mut *tx).await?;
        }
        [32312, 32804, 7]
    } else if matches!(case, Case::NoPath) {
        [POSITION[0], POSITION[1], POSITION[2] + 1]
    } else {
        POSITION
    };
    let position: Vec<u8> = position
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect();
    sqlx::query(
        "INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING",
    )
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(&position)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,1,encode($5,'hex')::uuid)")
        .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(&position).bind(id(74).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
        .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    admitted.continuity.next_command_id = 1;

    if full_renown {
        return renown_roundtrip(&owner, h, admitted, &outfit_key, mount_key).await;
    }
    let set = wire::SetRequest {
        item: id(71),
        expected_revision: 0,
        display_kind: 3,
        direction: 2,
        key: display_key.into(),
    }
    .encode()
    .ok_or("rat set payload")?;
    if matches!(case, Case::MissingCapability) {
        let command = admitted.continuity.next_command_id;
        assert_eq!(
            exchange(&owner, &mut admitted, command, wire::COMMAND_SET, &set).await?,
            Some((CommandStatus::Rejected, Vec::new()))
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_podium_displays")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        return Ok(());
    }

    if matches!(case, Case::Unavailable) {
        // The harness pool connects as the administrative role, so revoking EXECUTE
        // from the runtime role would not make the target unavailable here.
        sqlx::query("ALTER FUNCTION game_house_podium_target_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID) RENAME TO test_unavailable_house_podium_target")
            .execute(&h.pool).await?;
    }
    let item = id(71);
    if matches!(case, Case::GuardBusyResume) {
        let command = admitted.continuity.next_command_id;
        let (status, _) = exchange(
            &owner,
            &mut admitted,
            command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            &encode_step_intent(StepDirection::East),
        )
        .await?
        .ok_or("establishing House step result")?;
        assert_eq!(status, CommandStatus::Accepted);
        let actor = admitted.runtime_actor.ok_or("established House actor")?;
        assert!(
            owner
                .validate_house_resume(actor, admitted.game_session_id)
                .await,
            "the fixture must qualify House placement before the guard outage"
        );
    }
    let payload = if matches!(case, Case::Malformed) {
        &item[..15]
    } else {
        &item[..]
    };
    let command = admitted.continuity.next_command_id;
    let result = conversation(
        &owner,
        &mut admitted,
        command,
        match case {
            Case::GuardBusy(command_type) => command_type,
            _ => wire::COMMAND_QUERY,
        },
        payload,
        matches!(case, Case::CancelWalk),
        matches!(case, Case::GuardBusy(_) | Case::GuardBusyResume).then_some(h),
    )
    .await?;
    match case {
        Case::Malformed => {
            assert!(matches!(
                result.end,
                ConnectionEnd::AdmittedThenClosed(
                    _,
                    oteryn_protocol_oteryn::FoundationProtocolError::MalformedEnvelope
                )
            ));
            assert!(result.protocol_error);
            assert!(result.replies.is_empty());
            return Ok(());
        }
        Case::Unavailable | Case::GuardBusy(_) | Case::GuardBusyResume => {
            assert!(matches!(
                result.end,
                ConnectionEnd::AdmittedThenDisconnected(_)
            ));
            assert!(!result.protocol_error);
            assert!(result.replies.is_empty());
            if matches!(case, Case::GuardBusyResume) {
                let session = admitted.game_session_id;
                let generation = admitted.continuity.connection_generation;
                assert_eq!(
                    owner.lose_control(admitted, Duration::ZERO, None).await,
                    ControlLossResult::Recorded
                );
                let token = RecoverySource::token(94)?;
                let resumed = owner
                    .resume(ResumeAttempt {
                        game_session_id: session,
                        recovery_material: token.as_bytes(),
                        transport: AuthenticatedTransportRefV1::decode(&[94; 16]).map_err(debug)?,
                        last_applied_server_sequence: admitted.continuity.server_sequence,
                        supported_capabilities: selected,
                    })
                    .await
                    .map_err(|error| {
                        let streams = &owner.states.command_ingress;
                        let mut recovered = admitted;
                        let can_resume_commands =
                            streams.recover_trade_continuity(&mut recovered).is_some()
                                && streams.resumable(&recovered);
                        format!("same-session resume {error:?}, command continuity ready: {can_resume_commands}")
                    })?;
                assert_eq!(resumed.game_session_id, session);
                assert_eq!(resumed.continuity.connection_generation, generation + 1);
                assert_eq!(resumed.continuity.next_command_id, command + 1);
            }
            return Ok(());
        }
        Case::NoPath => {
            assert_eq!(
                result.replies,
                [(command, CommandStatus::Rejected, Vec::new())]
            );
            assert_eq!(result.steps, 0);
            return Ok(());
        }
        Case::CancelWalk => {
            assert_eq!(result.steps, 2);
            assert!(result.post_move_probe);
            assert_eq!(result.replies.len(), 2);
            assert_eq!(
                result.replies[0],
                (command, CommandStatus::Rejected, Vec::new())
            );
            assert_eq!(
                (result.replies[1].0, result.replies[1].1),
                (command + 1, CommandStatus::Accepted)
            );
            return Ok(());
        }
        _ => {}
    }
    let [(result_id, status, payload)] = result.replies.as_slice() else {
        return Err(format!("Expected one query result, got {result:?}").into());
    };
    assert_eq!((*result_id, *status), (command, CommandStatus::Accepted));
    assert_eq!(wire::Projection::decode(payload), Some(None));
    if walks {
        assert_eq!(result.steps, 2);
        let actor = admitted.runtime_actor.ok_or("house actor")?;
        let house = owner
            .runtime
            .lock()
            .await
            .house_actor_state(actor, admitted.game_session_id)
            .map_err(debug)?;
        assert_eq!((house.position.x, house.position.y), (32311, 32804));
        return Ok(());
    }
    let threshold = if boss {
        let rewards = active.native_gameplay().ok_or("native podium")?.reward_table();
        let category = &rewards.boss(BOSS).ok_or("Ferumbras Bosstiary entry")?.category;
        let [_, expertise, _] = crate::domain::bosstiary::kill_thresholds(category)
            .ok_or("expertise threshold")?;
        i64::try_from(expertise)?
    } else {
        i64::from(crate::gameplay_transport::charm_progression::bestiary_race_completion_threshold(RAT)
            .ok_or("Rat completion threshold")?)
    };
    if matches!(case, Case::Roundtrip { .. }) {
        seed_kills(h, display_key, threshold - 1, boss).await?;
        let command = admitted.continuity.next_command_id;
        let (status, payload) = exchange(&owner, &mut admitted, command, wire::COMMAND_SET, &set)
            .await?.ok_or("locked display result")?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            wire::SetResult::decode(&payload),
            Some(wire::SetResult::Refused(wire::SetRefusal::NotOwned))
        );
        let displays: i64 = sqlx::query_scalar("SELECT count(*) FROM game_house_podium_displays")
            .fetch_one(&h.pool).await?;
        assert_eq!(displays, 0);
    }
    seed_kills(h, display_key, threshold, boss).await?;
    let command = admitted.continuity.next_command_id;
    let (status, payload) = exchange(&owner, &mut admitted, command, wire::COMMAND_SET, &set)
        .await?
        .ok_or("set command result")?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        wire::SetResult::decode(&payload),
        Some(wire::SetResult::Accepted { revision: 1 })
    );
    let expected = Some(wire::Projection {
        podium_kind,
        display_kind: 3,
        direction: 2,
        revision: 1,
        key: display_key.into(),
    });

    if matches!(case, Case::Roundtrip { .. }) {
        let command = admitted.continuity.next_command_id;
        let result = exchange(
            &owner,
            &mut admitted,
            command,
            wire::COMMAND_PROJECTION,
            &id(71),
        )
        .await?
        .ok_or("projection command result")?;
        assert_eq!(result.0, CommandStatus::Accepted);
        assert_eq!(wire::Projection::decode(&result.1), Some(expected));
        let stored: (String, String) =
            sqlx::query_as("SELECT revision::text,display_key FROM game_house_podium_displays")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(stored, ("1".into(), display_key.into()));
        return Ok(());
    }

    let command = admitted.continuity.next_command_id;
    let (status, payload) = exchange(&owner, &mut admitted, command, wire::COMMAND_SET, &set)
        .await?
        .ok_or("stale set command result")?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        wire::SetResult::decode(&payload),
        Some(wire::SetResult::Refused(wire::SetRefusal::Changed))
    );
    let wrong_display = wire::SetRequest {
        item: id(71),
        expected_revision: 1,
        display_kind: 1,
        direction: 2,
        key: outfit_key,
    }
    .encode()
    .ok_or("outfit set payload")?;
    let command = admitted.continuity.next_command_id;
    let (status, payload) = exchange(
        &owner,
        &mut admitted,
        command,
        wire::COMMAND_SET,
        &wrong_display,
    )
    .await?
    .ok_or("wrong-display command result")?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        wire::SetResult::decode(&payload),
        Some(wire::SetResult::Refused(wire::SetRefusal::WrongDisplay))
    );
    let command = admitted.continuity.next_command_id;
    let (status, payload) = exchange(
        &owner,
        &mut admitted,
        command,
        wire::COMMAND_PROJECTION,
        &id(71),
    )
    .await?
    .ok_or("unchanged projection result")?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(wire::Projection::decode(&payload), Some(expected));
    Ok(())
}

async fn renown_roundtrip(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    mut admitted: AdmittedSession,
    outfit_key: &str,
    mount_key: &str,
) -> TestResult {
    let flags = wire::DisplayFlags::default();
    let mut appearance = wire::RenownAppearance {
        outfit_sex: Some(wire::OutfitSex::Male),
        addons: 1,
        illuminated: true,
        mount_key: Some(mount_key.into()),
    };
    let mut set = wire::SetRequest {
        item: id(71),
        expected_revision: 0,
        display_kind: 1,
        direction: 2,
        key: outfit_key.into(),
    };
    let command = admitted.continuity.next_command_id;
    let locked = set.encode_full(flags, &appearance).ok_or("full podium SET")?;
    let (status, reply) = exchange(owner, &mut admitted, command, wire::COMMAND_SET, &locked)
        .await?.ok_or("unowned mount refusal")?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(wire::SetResult::decode(&reply), Some(wire::SetResult::Refused(wire::SetRefusal::NotOwned)));

    sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'podium-mount','content-1',encode($3,'hex')::uuid,'quest',$1)")
        .bind(id(70).as_slice()).bind(id(40).as_slice()).bind(id(41).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_account_appearance_unlocks VALUES(encode($1,'hex')::uuid,'podium-mount',encode($2,'hex')::uuid)")
        .bind(id(40).as_slice()).bind(id(70).as_slice()).execute(&h.pool).await?;

    for (sex, illuminated, mounted) in [
        (Some(wire::OutfitSex::Male), true, true),
        (Some(wire::OutfitSex::Female), false, false),
        (None, true, false),
    ] {
        appearance.outfit_sex = sex;
        appearance.addons = u8::from(sex == Some(wire::OutfitSex::Male));
        appearance.illuminated = illuminated;
        appearance.mount_key = mounted.then(|| mount_key.to_owned());
        if sex.is_none() {
            set.display_kind = 2;
            set.key = mount_key.into();
        }
        let command = admitted.continuity.next_command_id;
        let payload = set.encode_full(flags, &appearance).ok_or("full podium SET")?;
        let (status, reply) = exchange(owner, &mut admitted, command, wire::COMMAND_SET, &payload)
            .await?.ok_or("full podium SET result")?;
        let revision = set.expected_revision + 1;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(wire::SetResult::decode(&reply), Some(wire::SetResult::Accepted { revision }));
        let projection = wire::Projection {
            podium_kind: 1, display_kind: set.display_kind, direction: set.direction,
            revision, key: set.key.clone(),
        };
        for command_type in [wire::COMMAND_QUERY, wire::COMMAND_PROJECTION, wire::COMMAND_PROJECTION] {
            let command = admitted.continuity.next_command_id;
            let (status, reply) = exchange(owner, &mut admitted, command, command_type, &set.item)
                .await?.ok_or("full podium projection")?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(wire::FullProjection::decode(&reply), Some(Some(wire::FullProjection {
                display: projection.clone(), flags, renown: Some(appearance.clone()),
            })));
        }
        set.expected_revision = revision;
    }
    Ok(())
}

async fn seed_kills(h: &Harness, key: &str, kills: i64, boss: bool) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let sql = if boss {
        "INSERT INTO game_character_bosstiary_progress(character_id,boss_key,kill_count)
         VALUES(encode($1,'hex')::uuid,$2,$3)
         ON CONFLICT (character_id,boss_key) DO UPDATE SET kill_count=EXCLUDED.kill_count"
    } else {
        "INSERT INTO game_character_bestiary_progress(character_id,race_key,kill_count,
             committed_character_revision,last_bestiary_occurrence_id)
         VALUES(encode($1,'hex')::uuid,$2,$3,2,'018f0000-0000-7000-8000-000000000001')
         ON CONFLICT (character_id,race_key) DO UPDATE SET kill_count=EXCLUDED.kill_count"
    };
    sqlx::query(sql)
        .bind(id(41).as_slice())
        .bind(key)
        .bind(kills)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
) -> TestResult<Option<(CommandStatus, Vec<u8>)>> {
    Ok(conversation(
        owner,
        admitted,
        command_id,
        command_type,
        payload,
        false,
        None,
    )
    .await?
    .replies
    .pop()
    .map(|(_, status, payload)| (status, payload)))
}

#[derive(Debug)]
struct Conversation {
    end: ConnectionEnd,
    replies: Vec<(u64, CommandStatus, Vec<u8>)>,
    steps: usize,
    protocol_error: bool,
    post_move_probe: bool,
}

async fn conversation(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
    interrupt: bool,
    guard_fixture: Option<&Harness>,
) -> TestResult<Conversation> {
    let generation = admitted.continuity.connection_generation;
    let frame = encode_client_command(
        generation,
        &ClientCommandValue {
            command_id,
            command_type,
            payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    if guard_fixture.is_none() {
        write_frame(&mut client, &frame).await?;
    }
    let receive = async move {
        let mut guard_fixture = guard_fixture;
        let mut guard_lock = None;
        let mut replies = Vec::new();
        let mut steps = 0;
        let mut protocol_error = false;
        let mut manual_finished = false;
        let mut post_move_probe = false;
        loop {
            let raw = match read_frame(&mut client).await {
                Ok(raw) => raw,
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
            };
            let envelope = decode_wire_envelope(&raw).map_err(debug)?;
            match envelope.message_type() {
                MessageType::SnapshotCommit if guard_fixture.is_some() => {
                    let h = guard_fixture.take().ok_or("guard fixture")?;
                    // The House must finish admission before the command guard
                    // encounters contention on the same durable relation.
                    let mut tx = h.pool.begin().await?;
                    sqlx::query(
                        "LOCK TABLE game_durability_admission_lifecycle_receipts IN ROW SHARE MODE",
                    )
                    .execute(&mut *tx)
                    .await?;
                    guard_lock = Some(tx);
                    write_frame(&mut client, &frame).await?;
                }
                MessageType::LivenessProbe => {
                    let probe = oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload())
                        .map_err(debug)?;
                    let ack = oteryn_protocol_oteryn::encode_liveness_ack(
                        generation,
                        probe,
                        envelope.server_sequence(),
                    )
                    .map_err(debug)?;
                    write_frame(&mut client, &ack).await?;
                    if manual_finished {
                        post_move_probe = true;
                        client.shutdown().await?;
                    }
                }
                MessageType::StateDelta => {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id == STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY {
                        steps += 1;
                        if interrupt && steps == 1 {
                            let manual = encode_client_command(
                                generation,
                                &ClientCommandValue {
                                    command_id: command_id + 1,
                                    command_type: COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                                    payload: &encode_step_intent(StepDirection::East),
                                },
                            )
                            .map_err(debug)?;
                            write_frame(&mut client, &manual).await?;
                        }
                    }
                }
                MessageType::ProtocolError => protocol_error = true,
                MessageType::CommandResult => {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    replies.push((result.command_id, result.status, result.payload.to_vec()));
                    if !interrupt || steps == 0 {
                        client.shutdown().await?;
                    }
                    if interrupt && result.command_id == command_id + 1 {
                        // Keep the socket live through the next ready probe, past the
                        // canceled retry, to detect late movement or a second result.
                        manual_finished = true;
                    }
                }
                _ => {}
            }
        }
        assert!(
            guard_fixture.is_none(),
            "the House must finish its initial snapshot"
        );
        if let Some(tx) = guard_lock {
            tx.rollback().await?;
        }
        Ok::<_, Box<dyn std::error::Error>>((replies, steps, protocol_error, post_move_probe))
    };
    let before = *admitted;
    let serve = async move { serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await };
    let (received, end) = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(receive, serve)
    })
    .await?;
    let (replies, steps, protocol_error, post_move_probe) = received?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &end
    {
        *admitted = *current;
    }
    Ok(Conversation {
        end,
        replies,
        steps,
        protocol_error,
        post_move_probe,
    })
}

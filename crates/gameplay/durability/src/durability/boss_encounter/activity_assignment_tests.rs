//! Native activity allocation qualification, not activity admission qualification.
use crate::bestiary_postgres_harness::id;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
use crate::bestiary_postgres_harness as fixture;
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentError, AssignmentOutcome, AssignmentReceipt, AssignmentRejection,
    AssignmentRequest, AssignmentState, ControlActor, NodeIncarnationProof, OperationKey,
    ReconcileOutcome,
};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
    AdmissionAuthorityPublicationChangeV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
    AdmissionPublicationSourceV1,
};
use crate::foundation::{RuntimeScopeRefV1, WorldId};

fn request(tag: u8, command: AssignmentCommand) -> TestResult<AssignmentRequest> {
    Ok(AssignmentRequest {
        operation_key: OperationKey::from_bytes([tag; 32]),
        actor: ControlActor::new("oteryn_test_admin").map_err(|e| format!("{e:?}"))?,
        command,
    })
}

fn committed(outcome: Result<AssignmentOutcome, AssignmentError>) -> TestResult<AssignmentReceipt> {
    match outcome.map_err(|e| format!("{e:?}"))? {
        AssignmentOutcome::Committed(receipt) => Ok(receipt),
        other => Err(format!("unexpected allocation: {other:?}").into()),
    }
}

async fn readiness(
    harness: &fixture::Harness,
    scope: RuntimeScopeRefV1,
    revision: u64,
    generation: u64,
) -> TestResult<AdmissionAuthorityPublicationV1> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let change = AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Runtime(scope),
        source: AdmissionPublicationSourceV1 {
            authority: "boss-activity-runtime-publisher".into(),
            purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
            source_revision: revision,
            decision_identity: format!("boss-activity-ready-{revision}"),
            source_observed_at: now,
            clock_uncertainty_seconds: 0,
        },
        precondition: if revision == 1 {
            AdmissionPublicationPreconditionV1::Bootstrap {
                restored_publication_high_water: Some(0),
            }
        } else {
            AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: revision - 1,
            }
        },
        publication_revision: revision,
        state: AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation: generation,
            ready: true,
            route_revision: "route-1".into(),
            runtime_observation_revision: "runtime-1".into(),
            protocol_major: 1,
            transport_profile: 1,
            ruleset_revision: "ruleset-1".into(),
            content_revision: "content-1".into(),
            map_revision: "map-1".into(),
            world_policy_revision: "world-policy-1".into(),
            offer_revision: "offer-1".into(),
        },
    };
    fixture::ground_native_readiness_enforcement::prepare(change, now)
}

async fn qualify(h: &fixture::Harness) -> TestResult {
    let world = WorldId::decode(&fixture::id(fixture::WORLD)).map_err(|e| format!("{e:?}"))?;
    let scope = RuntimeScopeRefV1::instance(world, id(88)).map_err(|e| format!("{e:?}"))?;
    let assign = request(
        80,
        AssignmentCommand::Assign {
            scope,
            target: h.node.fact(),
        },
    )?;
    // The Channel grant in this real native fixture must never grant an activity.
    assert!(matches!(
        h.writer.submit(&assign).await,
        Ok(AssignmentOutcome::Rejected(AssignmentRejection::NotGranted))
    ));
    sqlx::query("INSERT INTO game_control_activity_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice()).bind(id(88).as_slice()).execute(&h.pool).await?;
    // A grant for the right instance in the wrong World still refuses.
    let wrong_world = RuntimeScopeRefV1::instance(
        WorldId::decode(&id(89)).map_err(|e| format!("{e:?}"))?,
        id(88),
    )
    .map_err(|e| format!("{e:?}"))?;
    for denied_scope in [
        wrong_world,
        RuntimeScopeRefV1::instance(world, id(89)).map_err(|e| format!("{e:?}"))?,
    ] {
        assert!(matches!(
            h.writer
                .submit(&request(
                    81,
                    AssignmentCommand::Assign {
                        scope: denied_scope,
                        target: h.node.fact(),
                    }
                )?)
                .await,
            Ok(AssignmentOutcome::Rejected(AssignmentRejection::NotGranted))
        ));
    }
    let first = committed(h.writer.submit(&assign).await)?;
    assert_eq!(first.assignment.scope, scope);
    assert_eq!(first.assignment.ownership_generation, 1);
    assert_eq!(first.assignment.source_revision, 2); // fixture Channel used revision 1
    assert_eq!(committed(h.writer.submit(&assign).await)?, first);
    assert_eq!(
        h.writer
            .reconcile(&assign)
            .await
            .map_err(|e| format!("{e:?}"))?,
        ReconcileOutcome::Committed(first.clone())
    );
    assert_eq!(
        h.root
            .read_runtime_scope_assignment(scope)
            .await
            .map_err(|e| format!("{e:?}"))?,
        Some(first.assignment.clone())
    );
    let identity: (i16, bool, bool, Vec<u8>) = sqlx::query_as(
        "SELECT scope_kind,channel_id IS NULL,house_key IS NULL,scope_key FROM game_runtime_scope_assignments WHERE activity_instance_id=encode($1,'hex')::uuid")
        .bind(id(88).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((identity.0, identity.1, identity.2), (3, true, true));
    assert_eq!(identity.3, [&[3][..], world.as_bytes(), &id(88)].concat());
    let mut changed = assign.clone();
    changed.command = AssignmentCommand::Assign {
        scope: wrong_world,
        target: h.node.fact(),
    };
    assert!(matches!(
        h.writer.submit(&changed).await,
        Ok(AssignmentOutcome::Rejected(
            AssignmentRejection::OperationConflict
        ))
    ));
    // Runtime kind 2 resolves to assignment kind 3, with possession required.
    let ready = readiness(h, scope, 1, 1).await?;
    let false_proof = NodeIncarnationProof::new(
        h.node.fact(),
        crate::durability::runtime_scope_assignment::BootstrapSecret::from_bytes([99; 32]),
    );
    assert!(matches!(
        h.root.publish_runtime_readiness(&false_proof, &ready).await,
        Err(AssignmentError::NotCurrentHolder)
    ));
    assert_eq!(
        h.root
            .publish_runtime_readiness(&h.node, &ready)
            .await
            .map_err(|e| format!("{e:?}"))?,
        GuardPublicationDisposition::Applied
    );
    let replace = request(
        82,
        AssignmentCommand::Replace {
            scope,
            predecessor: first.predecessor(),
            target: h.node.fact(),
        },
    )?;
    assert!(matches!(
        h.writer.submit(&replace).await,
        Ok(AssignmentOutcome::Rejected(AssignmentRejection::NotGranted))
    ));
    sqlx::query("INSERT INTO game_control_activity_scope_grants SELECT session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,op FROM unnest(ARRAY[2,3]::smallint[]) op")
        .bind(world.as_bytes().as_slice()).bind(id(88).as_slice()).execute(&h.pool).await?;
    // Publication advanced the guard after the first receipt. The stale CAS refuses.
    assert!(matches!(
        h.writer.submit(&replace).await,
        Ok(AssignmentOutcome::Rejected(
            AssignmentRejection::PredecessorMismatch
        ))
    ));
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(|e| format!("{e:?}"))?
        .ok_or("missing predecessor")?;
    let second = committed(
        h.writer
            .submit(&request(
                83,
                AssignmentCommand::Replace {
                    scope,
                    predecessor,
                    target: h.node.fact(),
                },
            )?)
            .await,
    )?;
    assert_eq!(second.assignment.ownership_generation, 2);
    assert_eq!(second.fenced_publication_revision, Some(2));
    // The instance guard is fenced, while the fixture Channel remains ready.
    let guard: (String, bool) = sqlx::query_as("SELECT ownership_generation::text,ready FROM game_durability_admission_runtime_guards WHERE scope_key=$1")
        .bind([&[2][..], world.as_bytes(), &id(88)].concat()).fetch_one(&h.pool).await?;
    assert_eq!(guard, ("2".into(), false));
    assert_eq!(
        h.root
            .publish_runtime_readiness(&h.node, &readiness(h, scope, 3, 1).await?)
            .await
            .map_err(|e| format!("{e:?}"))?,
        GuardPublicationDisposition::Stale
    );
    assert_eq!(
        h.root
            .publish_runtime_readiness(&h.node, &readiness(h, scope, 3, 2).await?)
            .await
            .map_err(|e| format!("{e:?}"))?,
        GuardPublicationDisposition::Applied
    );
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(|e| format!("{e:?}"))?
        .ok_or("missing predecessor")?;
    let revoke = request(84, AssignmentCommand::Revoke { scope, predecessor })?;
    let third = committed(h.writer.submit(&revoke).await)?;
    assert_eq!(third.assignment.state, AssignmentState::Revoked);
    assert_eq!(third.assignment.ownership_generation, 3);
    assert_eq!(third.fenced_publication_revision, Some(4));
    assert_eq!(committed(h.writer.submit(&revoke).await)?, third);
    assert!(matches!(
        h.root
            .publish_runtime_readiness(&h.node, &readiness(h, scope, 5, 3).await?)
            .await,
        Err(AssignmentError::NotCurrentHolder)
    ));
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT game_runtime_scope_assignment_history_valid()")
            .fetch_one(&h.pool)
            .await?
    );
    // A session cannot obtain a retained activity identity without its handoff,
    // including after that assignment is revoked.
    let error = sqlx::query("UPDATE game_durability_reconnect_sessions SET runtime_scope_kind=2,runtime_scope_channel_id=NULL,runtime_scope_instance_id=encode($1,'hex')::uuid WHERE game_session_id=encode($2,'hex')::uuid")
        .bind(id(88).as_slice()).bind(id(fixture::SESSION).as_slice())
        .execute(&h.pool).await.err().ok_or("bare activity session was admitted")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    assert!(
        error
            .to_string()
            .contains("activity session requires a committed activity handoff")
    );
    // Identity and retained receipt history cannot be mutated, including after revoke.
    for sql in [
        "UPDATE game_runtime_scope_assignments SET activity_instance_id=NULL WHERE scope_kind=3",
        "UPDATE game_runtime_scope_assignments SET scope_kind=2 WHERE scope_kind=3",
        "DELETE FROM game_runtime_scope_assignments WHERE scope_kind=3",
        "DELETE FROM game_runtime_scope_assignment_receipts WHERE source_revision=2",
    ] {
        assert!(
            sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
                .execute(&h.pool)
                .await
                .is_err()
        );
    }
    qualify_collisions(h, world).await?;
    qualify_concurrent_predecessor(h, world).await?;
    Ok(())
}

// Exercise the shared durable session fence through its production cooldown
// consumer. A bare Instance session is representable by FND-04, but neither it
// nor an activity's allocation/readiness is a participant handoff proof.
async fn qualify_gameplay_fence(h: &fixture::Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let channel_fence = fixture::item_fence()?;
    assert!(
        h.root
            .read_boss_cooldown_list(&authority, &h.node, channel_fence, &[])
            .await?
            .is_some()
    );
    let world = channel_fence.runtime_scope.world_id();
    let mut instance_fence = channel_fence;
    instance_fence.runtime_scope =
        RuntimeScopeRefV1::instance(world, id(88)).map_err(fixture::debug)?;
    assert!(
        h.root
            .read_boss_cooldown_list(&authority, &h.node, instance_fence, &[])
            .await?
            .is_none()
    );
    // A real bare session at a different, unallocated Instance cannot borrow
    // the previous Channel's assignment or ready guard. No trigger is disabled.
    // The house schema forbids origin metadata on a bare session; an activity
    // handoff needs a separately qualified schema before that can change.
    sqlx::query("UPDATE game_durability_reconnect_sessions SET runtime_scope_kind=2,runtime_scope_channel_id=NULL,runtime_scope_instance_id=encode($1,'hex')::uuid WHERE game_session_id=encode($2,'hex')::uuid")
        .bind(id(89).as_slice())
        .bind(id(fixture::SESSION).as_slice()).execute(&h.pool).await?;
    instance_fence.runtime_scope =
        RuntimeScopeRefV1::instance(world, id(89)).map_err(fixture::debug)?;
    for fence in [channel_fence, instance_fence] {
        assert!(
            h.root
                .read_boss_cooldown_list(&authority, &h.node, fence, &[])
                .await?
                .is_none()
        );
    }
    sqlx::query("UPDATE game_durability_reconnect_sessions SET runtime_scope_kind=1,runtime_scope_channel_id=encode($1,'hex')::uuid,runtime_scope_instance_id=NULL,origin_channel_id=NULL WHERE game_session_id=encode($2,'hex')::uuid")
        .bind(id(fixture::CHANNEL).as_slice()).bind(id(fixture::SESSION).as_slice())
        .execute(&h.pool).await?;
    // Allocation and attested readiness still cannot admit a Channel session.
    sqlx::query("INSERT INTO game_control_activity_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice()).bind(id(88).as_slice()).execute(&h.pool).await?;
    let scope = RuntimeScopeRefV1::instance(world, id(88)).map_err(fixture::debug)?;
    committed(
        h.writer
            .submit(&request(
                95,
                AssignmentCommand::Assign {
                    scope,
                    target: h.node.fact(),
                },
            )?)
            .await,
    )?;
    assert_eq!(
        h.root
            .publish_runtime_readiness(&h.node, &readiness(h, scope, 1, 1).await?)
            .await
            .map_err(fixture::debug)?,
        GuardPublicationDisposition::Applied
    );
    instance_fence.runtime_scope = scope;
    assert!(
        h.root
            .read_boss_cooldown_list(&authority, &h.node, instance_fence, &[])
            .await?
            .is_none()
    );
    assert!(
        h.root
            .read_boss_cooldown_list(&authority, &h.node, channel_fence, &[])
            .await?
            .is_some()
    );
    qualify_channel_position_consumers(h, &authority, scope).await?;
    Ok(())
}

// The shared session fence's typed Instance support must not turn a Channel
// position into instance-local coordinates or run a Channel respawn placement.
// This qualifies consumer refusal, not an accepted activity session or return.
async fn qualify_channel_position_consumers(
    h: &fixture::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    scope: RuntimeScopeRefV1,
) -> TestResult {
    use crate::durability::character_death::PlayerDeathOccurrence;
    use crate::durability::character_last_position::{
        FinalLastPosition, LastPositionCell, LastPositionRecord, LastPositionWrite, LastVitals,
        PeriodicLastPosition,
    };
    use crate::durability::character_progression::CharacterProgressionError;
    use std::sync::atomic::{AtomicBool, Ordering};

    let channel = fixture::fence(1)?;
    let record = LastPositionRecord {
        world_id: channel.runtime_scope.world_id(),
        cell: LastPositionCell {
            x: 100,
            y: 200,
            floor: 7,
        },
        coordinate_frame_digest: [1; 32],
        map_revision_digest: [2; 32],
        server_artifact_digest: [3; 32],
        vitals: Some(LastVitals::new(100, 50).ok_or("vitals")?),
        rest: Some([10, 20, 30, 40]),
    };
    assert_eq!(
        h.root
            .write_character_last_position(authority, &h.node, channel, 1, record)
            .await?,
        LastPositionWrite::Written
    );
    let mut instance = channel;
    instance.runtime_scope = scope;
    let local = LastPositionRecord {
        cell: LastPositionCell {
            x: 1,
            y: 2,
            floor: 3,
        },
        vitals: Some(LastVitals::new(1, 0).ok_or("instance vitals")?),
        rest: None,
        ..record
    };
    assert!(matches!(
        h.root
            .write_character_last_position(authority, &h.node, instance, 99, local)
            .await,
        Err(CharacterProgressionError::InvalidInput)
    ));
    assert!(matches!(
        h.root
            .read_character_last_position(authority, &h.node, instance)
            .await,
        Err(CharacterProgressionError::InvalidInput)
    ));
    let outcomes = h
        .root
        .write_character_last_positions(
            authority,
            &h.node,
            vec![
                PeriodicLastPosition {
                    fence: instance,
                    write_sequence: 100,
                    record: local,
                },
                PeriodicLastPosition {
                    fence: channel,
                    write_sequence: 2,
                    record,
                },
            ],
        )
        .await?;
    assert_eq!(outcomes.len(), 2);
    assert!(matches!(
        outcomes[0],
        Err(CharacterProgressionError::InvalidInput)
    ));
    assert!(matches!(outcomes[1], Ok(LastPositionWrite::Written)));

    let occurrence = PlayerDeathOccurrence::from_bytes(id(99)).map_err(fixture::debug)?;
    assert!(matches!(
        h.root.pending_respawn(authority, &h.node, instance).await,
        Err(CharacterProgressionError::InvalidInput)
    ));
    assert!(matches!(
        h.root
            .consume_pending_respawn(authority, &h.node, instance, Some(occurrence))
            .await,
        Err(CharacterProgressionError::InvalidInput)
    ));
    let placed = FinalLastPosition {
        record: local,
        write_sequence: 101,
    };
    assert!(matches!(
        h.root
            .consume_pending_respawn_placed(authority, &h.node, instance, Some(occurrence), placed)
            .await,
        Err(CharacterProgressionError::InvalidInput)
    ));
    let called = AtomicBool::new(false);
    assert!(matches!(
        h.root
            .place_and_consume_pending_respawn(authority, &h.node, instance, occurrence, |_| {
                called.store(true, Ordering::SeqCst);
                std::future::ready(true)
            })
            .await,
        Err(CharacterProgressionError::InvalidInput)
    ));
    assert!(matches!(
        h.root
            .place_and_consume_pending_respawn_placed(
                authority,
                &h.node,
                instance,
                occurrence,
                placed,
                |_| {
                    called.store(true, Ordering::SeqCst);
                    std::future::ready(true)
                }
            )
            .await,
        Err(CharacterProgressionError::InvalidInput)
    ));
    assert!(!called.load(Ordering::SeqCst));
    let saved = h
        .root
        .read_character_last_position(authority, &h.node, channel)
        .await?
        .ok_or("saved Channel position missing")?;
    assert_eq!(saved.record, record);
    assert_eq!(
        saved.key.character_lease_generation,
        channel.character_lease_generation
    );
    assert_eq!(saved.key.write_sequence, 2);
    let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(channel.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(revision, "1");
    Ok(())
}

async fn qualify_collisions(h: &fixture::Harness, world: WorldId) -> TestResult {
    use crate::durability::house_scope_handoff::{
        HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome,
        HouseScopeAssignmentRequest,
    };
    for (tag, activity_first) in [(90, true), (91, false)] {
        let house = HouseId::new(world, &format!("oteryn:content.house.boss_collision_{tag}"))
            .ok_or("invalid fixture house")?;
        let instance = house.instance_id();
        let scope = house.runtime_scope()?;
        sqlx::query("INSERT INTO game_control_house_scope_grants VALUES(session_user,encode($1,'hex')::uuid,$2,1)")
            .bind(world.as_bytes().as_slice()).bind(house.house_key()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_activity_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
            .bind(world.as_bytes().as_slice()).bind(instance.as_slice()).execute(&h.pool).await?;
        let house_request = HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([tag; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(|e| format!("{e:?}"))?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Assign {
                house,
                target: h.node.fact(),
            },
        };
        let activity_request = request(
            tag + 2,
            AssignmentCommand::Assign {
                scope,
                target: h.node.fact(),
            },
        )?;
        if activity_first {
            committed(h.writer.submit(&activity_request).await)?;
            assert!(matches!(
                h.root.assign_house_scope(house_request).await,
                Ok(HouseScopeAssignmentOutcome::Rejected(
                    AssignmentRejection::ScopeInUse
                ))
            ));
        } else {
            assert!(matches!(
                h.root.assign_house_scope(house_request).await,
                Ok(HouseScopeAssignmentOutcome::Committed(_))
            ));
            assert!(matches!(
                h.writer.submit(&activity_request).await,
                Ok(AssignmentOutcome::Rejected(AssignmentRejection::ScopeInUse))
            ));
        }
    }
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT game_runtime_scope_assignment_history_valid()")
            .fetch_one(&h.pool)
            .await?
    );
    Ok(())
}

async fn qualify_concurrent_predecessor(h: &fixture::Harness, world: WorldId) -> TestResult {
    let scope = RuntimeScopeRefV1::instance(world, id(94)).map_err(|e| format!("{e:?}"))?;
    sqlx::query("INSERT INTO game_control_activity_scope_grants SELECT session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,op FROM unnest(ARRAY[1,2]::smallint[]) op")
        .bind(world.as_bytes().as_slice()).bind(id(94).as_slice()).execute(&h.pool).await?;
    let first = committed(
        h.writer
            .submit(&request(
                95,
                AssignmentCommand::Assign {
                    scope,
                    target: h.node.fact(),
                },
            )?)
            .await,
    )?;
    let command = AssignmentCommand::Replace {
        scope,
        predecessor: first.predecessor(),
        target: h.node.fact(),
    };
    let left = request(96, command)?;
    let right = request(97, command)?;
    let left_writer = h.writer.clone();
    let left_task = tokio::spawn(async move { left_writer.submit(&left).await });
    let right_result = h.writer.submit(&right).await;
    let left_result = left_task.await?;
    let outcomes = [
        left_result.map_err(|e| format!("{e:?}"))?,
        right_result.map_err(|e| format!("{e:?}"))?,
    ];
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| matches!(o, AssignmentOutcome::Committed(_)))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| matches!(
                o,
                AssignmentOutcome::Rejected(AssignmentRejection::PredecessorMismatch)
            ))
            .count(),
        1
    );
    let current = h
        .root
        .read_runtime_scope_assignment(scope)
        .await
        .map_err(|e| format!("{e:?}"))?
        .ok_or("missing concurrent allocation")?;
    assert_eq!(current.ownership_generation, 2);
    assert_eq!(
        current.source_revision,
        first.assignment.source_revision + 1
    );
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT game_runtime_scope_assignment_history_valid()")
            .fetch_one(&h.pool)
            .await?
    );
    Ok(())
}

#[test]
fn boss_activity_command_rejects_invalid_instance_identity() -> TestResult {
    let world_id = WorldId::decode(&id(88)).map_err(|e| format!("{e:?}"))?;
    let malformed = RuntimeScopeRefV1::Instance {
        world_id,
        instance_id: [0; 16],
    };
    assert!(matches!(
        request(
            98,
            AssignmentCommand::Revoke {
                scope: malformed,
                predecessor: crate::durability::runtime_scope_assignment::AssignmentPredecessor {
                    ownership_generation: 1,
                    source_revision: 1,
                    runtime_guard_publication_revision: None,
                },
            }
        )?
        .encode(),
        Err(AssignmentError::InvalidInput)
    ));
    Ok(())
}

#[test]
fn boss_activity_assignment_postgres() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async move {
        let h = fixture::Harness::create(admin, "boss_activity", false).await?;
        eprintln!("Canonical PostgreSQL 18.6; activity kind 3 native assignment/receipt/readiness qualification");
        let result = qualify(&h).await;
        h.cleanup().await?;
        result
    })
}

#[test]
fn boss_activity_gameplay_fence_postgres() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async move {
        let h = fixture::Harness::create(admin, "boss_activity_fence", false).await?;
        eprintln!("Canonical PostgreSQL 18.6; activity session fence isolation qualification");
        let result = qualify_gameplay_fence(&h).await;
        h.cleanup().await?;
        result
    })
}

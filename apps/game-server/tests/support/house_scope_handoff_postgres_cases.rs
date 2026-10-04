// SCOPE-HANDOFF-1 house scope entry handoff (migration 0074) on the CHARM-2 harness: Character
// 41 on live Channel session 50, held by node 1. Migration 0074's `game_house_access` is a stub
// that admits nobody; HOUSE-1a writes the final body. These cases replace the body in the test
// database from test code with one reading a test-only fixture table `FOR SHARE`
// (HOUSE-RUNTIME-0 §10 test house). Every wrapper provides the same path-loaded crate root.

use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, WORLD, configured_admin, debug, id, register, runtime,
    scope,
};
use crate::durability::DurabilityRoot;
use crate::durability::house_scope_handoff::{
    HouseAbortOutcome, HouseCommitOutcome, HouseEntryCommit, HouseEntryRefusal, HouseEntryRequest,
    HouseHandoffDirection, HouseHandoffError, HouseHandoffState, HouseId, HousePrepareOutcome,
    HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome, HouseScopeAssignmentRequest,
    HouseScopePredecessor, commit_house_entry_in_transaction,
};
use crate::durability::runtime_scope_assignment::{
    AssignmentRejection, ControlActor, OperationKey,
};
use crate::foundation::{CharacterId, GameSessionId, WorldId};
use sqlx::Row;

const HOUSE_KEY: &str = "oteryn:content.house.thais_1";
const HANDOFF: u8 = 60;
const DESTINATION: u8 = 61;

fn house() -> TestResult<HouseId> {
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    Ok(HouseId::new(world, HOUSE_KEY).ok_or("house key")?)
}

fn request(handoff: u8) -> TestResult<HouseEntryRequest> {
    Ok(HouseEntryRequest {
        direction: HouseHandoffDirection::Entry,
        handoff_id: id(handoff),
        character_id: CharacterId::decode(&id(CHARACTER)).map_err(debug)?,
        source_game_session_id: GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        source_connection_generation: 1,
        source_character_lease_generation: 1,
        source_scope: scope()?,
        source_scope_ownership_generation: 1,
        house: house()?,
        house_scope_ownership_generation: 1,
        acl_revision: 1,
        guild_revisions: None,
        reserved_tile: vec![1, 2, 3],
        prepared_at: 100,
    })
}

fn commit(handoff: u8) -> TestResult<HouseEntryCommit> {
    Ok(HouseEntryCommit {
        handoff_id: id(handoff),
        destination_game_session_id: GameSessionId::decode(&id(DESTINATION)).map_err(debug)?,
        committed_at: 200,
    })
}

fn house_request(
    key: u8,
    command: HouseScopeAssignmentCommand,
) -> TestResult<HouseScopeAssignmentRequest> {
    Ok(HouseScopeAssignmentRequest {
        operation_key: OperationKey::from_bytes([key; 32]),
        actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
        actor_role: "oteryn_test_admin".into(),
        command,
    })
}

/// Session-use history for Character 41, house grants, the house assigned to node 1 at
/// generation 1, and the fixture property admitting Character 41 at ACL revision 1. The
/// access function stays the migration stub.
async fn seed(harness: &Harness) -> TestResult {
    sqlx::query(
        "INSERT INTO game_durability_session_use_ledgers VALUES (encode($1,'hex')::uuid,1,true,1,1)",
    )
    .bind(id(CHARACTER).as_slice())
    .execute(&harness.pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_session_use_memberships VALUES \
         (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL)",
    )
    .bind(id(SESSION).as_slice())
    .bind(id(CHARACTER).as_slice())
    .execute(&harness.pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_control_house_scope_grants (control_role, world_id, house_key, operation) \
         SELECT session_user, encode($1,'hex')::uuid, $2, operation \
           FROM unnest(ARRAY[1, 2, 3]::SMALLINT[]) AS operation",
    )
    .bind(id(WORLD).as_slice())
    .bind(HOUSE_KEY)
    .execute(&harness.pool)
    .await?;
    let outcome = harness
        .root
        .assign_house_scope(house_request(
            8,
            HouseScopeAssignmentCommand::Assign {
                house: house()?,
                target: harness.node.fact(),
            },
        )?)
        .await
        .map_err(debug)?;
    let HouseScopeAssignmentOutcome::Committed(assignment) = outcome else {
        return Err(format!("unexpected house assignment: {outcome:?}").into());
    };
    assert_eq!(assignment.ownership_generation, 1);
    sqlx::query(
        "CREATE TABLE test_house_properties (world_id UUID, house_key TEXT, \
           acl_revision NUMERIC(20,0), open BOOLEAN, guest UUID, PRIMARY KEY (world_id, house_key))",
    )
    .execute(&harness.pool)
    .await?;
    sqlx::query("INSERT INTO test_house_properties VALUES (encode($1,'hex')::uuid, $2, 1, true, encode($3,'hex')::uuid)")
        .bind(id(WORLD).as_slice())
        .bind(HOUSE_KEY)
        .bind(id(CHARACTER).as_slice())
        .execute(&harness.pool)
        .await?;
    Ok(())
}

/// The test house: `game_house_access` reads the fixture row `FOR SHARE`, the guest is a GUEST,
/// a closed property is content-fenced. Test code only, never a migration.
async fn install_test_house(harness: &Harness) -> TestResult {
    sqlx::query(
        "CREATE OR REPLACE FUNCTION game_house_access( \
             p_world_id UUID, p_house_key TEXT, p_character_id UUID) \
         RETURNS TABLE (role TEXT, content_fenced BOOLEAN, acl_revision NUMERIC(20,0), \
                        guild_revisions JSONB) \
         LANGUAGE sql AS $$ \
             SELECT CASE WHEN p.guest = p_character_id THEN 'GUEST' END, NOT p.open, \
                    p.acl_revision, NULL::JSONB \
               FROM test_house_properties p \
              WHERE p.world_id = p_world_id AND p.house_key = p_house_key FOR SHARE \
         $$",
    )
    .execute(&harness.pool)
    .await?;
    Ok(())
}

/// (session state, nonterminal session ids of Character 41, handoff rows).
async fn state(harness: &Harness) -> TestResult<(i16, Vec<Vec<u8>>, i64)> {
    let source: i16 = sqlx::query_scalar(
        "SELECT session_state FROM game_durability_reconnect_sessions \
          WHERE game_session_id = encode($1,'hex')::uuid",
    )
    .bind(id(SESSION).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    let live: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(game_session_id) FROM game_durability_reconnect_sessions \
          WHERE character_id = encode($1,'hex')::uuid AND session_state IN (1,2) \
          ORDER BY game_session_id",
    )
    .bind(id(CHARACTER).as_slice())
    .fetch_all(&harness.pool)
    .await?;
    Ok((
        source,
        live,
        harness.count("game_house_scope_handoffs").await?,
    ))
}

fn run(tag: &'static str, case: impl AsyncFnOnce(&Harness) -> TestResult) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag, false).await?;
        seed(&harness).await?;
        install_test_house(&harness).await?;
        case(&harness).await?;
        harness.cleanup().await
    })
}

async fn prepare(harness: &Harness, root: &DurabilityRoot, handoff: u8) -> TestResult {
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = root.open_character_authority(&seal).await.map_err(debug)?;
    let outcome = root
        .prepare_house_entry(&authority, &harness.node, request(handoff)?)
        .await
        .map_err(debug)?;
    let HousePrepareOutcome::Prepared(record) = outcome else {
        return Err(format!("unexpected prepare: {outcome:?}").into());
    };
    assert_eq!(record.state, HouseHandoffState::Prepared);
    Ok(())
}

/// A replacement candidate copying its predecessor's runtime scope, without house columns.
const CANDIDATE: &str = "INSERT INTO game_durability_reconnect_sessions (\
        game_session_id, account_id, character_id, world_id, runtime_scope_kind, \
        runtime_scope_world_id, runtime_scope_channel_id, runtime_scope_instance_id, \
        control_loss_epoch, original_grace_deadline, predecessor_generation, \
        character_lease_generation, scope_ownership_generation, current_generation, \
        attempt_count, session_state) \
     SELECT encode($2,'hex')::uuid, account_id, character_id, world_id, runtime_scope_kind, \
            runtime_scope_world_id, runtime_scope_channel_id, runtime_scope_instance_id, \
            1, 500, current_generation, character_lease_generation, \
            scope_ownership_generation, current_generation, 0, 1 \
       FROM game_durability_reconnect_sessions \
      WHERE game_session_id = encode($1,'hex')::uuid";

/// The terminal replacement path: terminalize the predecessor, write the replacement receipt,
/// then insert the candidate with the runtime scope and no house columns.
async fn replace_session(
    tx: &mut sqlx::PgConnection,
    predecessor: u8,
    candidate: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE game_durability_reconnect_sessions SET session_state = 3 \
          WHERE game_session_id = encode($1,'hex')::uuid",
    )
    .bind(id(predecessor).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_session_replacements \
         SELECT character_id, game_session_id, encode($2,'hex')::uuid, '\\x0101010101010101', \
                current_generation, character_lease_generation, scope_ownership_generation \
           FROM game_durability_reconnect_sessions \
          WHERE game_session_id = encode($1,'hex')::uuid",
    )
    .bind(id(predecessor).as_slice())
    .bind(id(candidate).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(CANDIDATE)
        .bind(id(predecessor).as_slice())
        .bind(id(candidate).as_slice())
        .execute(&mut *tx)
        .await?;
    Ok(())
}

/// Prepares and commits the entry: Character 41 is inside on house session 61.
async fn admit(harness: &Harness) -> TestResult {
    prepare(harness, &harness.root, HANDOFF).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let outcome = harness
        .root
        .commit_house_entry(&authority, &harness.node, commit(HANDOFF)?)
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, HouseCommitOutcome::Committed(_)));
    Ok(())
}

/// Replaces the house scope (generation 1) onto node 2: the house is at generation 2.
async fn replace_house_onto_node_2(harness: &Harness) -> TestResult {
    let other = register(&harness.root, 2).await?;
    let outcome = harness
        .root
        .assign_house_scope(house_request(
            12,
            HouseScopeAssignmentCommand::Replace {
                house: house()?,
                predecessor: HouseScopePredecessor {
                    ownership_generation: 1,
                    source_revision: 2,
                },
                target: other.fact(),
            },
        )?)
        .await
        .map_err(debug)?;
    let HouseScopeAssignmentOutcome::Committed(replaced) = outcome else {
        return Err(format!("unexpected house replace: {outcome:?}").into());
    };
    assert_eq!(replaced.ownership_generation, 2);
    Ok(())
}

fn refused_replacement(error: &sqlx::Error) -> bool {
    error.as_database_error().is_some_and(|error| {
        error.code().as_deref() == Some("23514")
            && error
                .message()
                .contains("a house scope session is not replaced")
    })
}

#[test]
fn crash_after_prepare_recovers_the_source_session() -> TestResult {
    run("hsh_prepare_crash", async |harness| {
        prepare(harness, &harness.root, HANDOFF).await?;
        // The commit body runs, then its backend dies before COMMIT.
        let mut tx = harness.pool.begin().await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *tx)
            .await?;
        let outcome = commit_house_entry_in_transaction(&mut tx, &harness.node, &commit(HANDOFF)?)
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, HouseCommitOutcome::Committed(_)));
        let killed: bool = sqlx::query_scalar("SELECT pg_terminate_backend($1)")
            .bind(pid)
            .fetch_one(&harness.pool)
            .await?;
        assert!(killed);
        assert!(tx.commit().await.is_err());

        let (source, live, handoffs) = state(harness).await?;
        assert_eq!((source, live, handoffs), (1, vec![id(SESSION).to_vec()], 1));
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let report = harness
            .root
            .reconcile_house_entries(&authority, &harness.node, None)
            .await
            .map_err(debug)?;
        assert_eq!(report.aborted, vec![id(HANDOFF)]);
        let (source, live, handoffs) = state(harness).await?;
        assert_eq!((source, live, handoffs), (1, vec![id(SESSION).to_vec()], 0));
        // The aborted handoff is gone; its commit finds nothing.
        let outcome = harness
            .root
            .commit_house_entry(&authority, &harness.node, commit(HANDOFF)?)
            .await
            .map_err(debug)?;
        assert_eq!(outcome, HouseCommitOutcome::Absent);
        Ok(())
    })
}

#[test]
fn crash_after_commit_admits_once_with_a_fresh_session() -> TestResult {
    run("hsh_commit_crash", async |harness| {
        prepare(harness, &harness.root, HANDOFF).await?;
        {
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let outcome = harness
                .root
                .commit_house_entry(&authority, &harness.node, commit(HANDOFF)?)
                .await
                .map_err(debug)?;
            let HouseCommitOutcome::Committed(record) = outcome else {
                return Err(format!("unexpected commit: {outcome:?}").into());
            };
            assert_eq!(record.state, HouseHandoffState::Committed);
        }
        // A restarted root recovers from durable state only.
        let root = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(root.maintain_ready_once().await?);
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let report = root
            .reconcile_house_entries(&authority, &harness.node, None)
            .await
            .map_err(debug)?;
        assert!(report.aborted.is_empty());
        let outcome = root
            .commit_house_entry(&authority, &harness.node, commit(HANDOFF)?)
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, HouseCommitOutcome::Replayed(_)));
        let mut other = commit(HANDOFF)?;
        other.destination_game_session_id = GameSessionId::decode(&id(62)).map_err(debug)?;
        assert!(matches!(
            root.commit_house_entry(&authority, &harness.node, other)
                .await,
            Err(HouseHandoffError::InvalidInput)
        ));
        assert!(matches!(
            root.abort_house_entry(&authority, id(HANDOFF))
                .await
                .map_err(debug)?,
            HouseAbortOutcome::AlreadyCommitted(_)
        ));
        let (source, live, handoffs) = state(harness).await?;
        assert_eq!(
            (source, live, handoffs),
            (3, vec![id(DESTINATION).to_vec()], 1)
        );
        let house_session = sqlx::query(
            "SELECT runtime_scope_kind, runtime_scope_house_key, \
                    uuid_send(origin_channel_id) AS origin, \
                    character_lease_generation::text AS lease, \
                    scope_ownership_generation::text AS scope_generation, \
                    runtime_scope_instance_id = game_house_scope_instance_id(world_id, $2) \
                      AS instance \
               FROM game_durability_reconnect_sessions \
              WHERE game_session_id = encode($1,'hex')::uuid",
        )
        .bind(id(DESTINATION).as_slice())
        .bind(HOUSE_KEY)
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(house_session.try_get::<i16, _>("runtime_scope_kind")?, 2);
        assert_eq!(
            house_session.try_get::<String, _>("runtime_scope_house_key")?,
            HOUSE_KEY
        );
        assert_eq!(
            house_session.try_get::<Vec<u8>, _>("origin")?,
            id(43).to_vec()
        );
        assert_eq!(house_session.try_get::<String, _>("lease")?, "2");
        assert_eq!(house_session.try_get::<String, _>("scope_generation")?, "1");
        assert!(house_session.try_get::<bool, _>("instance")?);
        let membership: String = sqlx::query_scalar(
            "SELECT membership_revision::text FROM game_durability_session_use_memberships \
              WHERE game_session_id = encode($1,'hex')::uuid",
        )
        .bind(id(DESTINATION).as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(membership, "2");
        // A committed handoff is immutable history.
        for statement in [
            "UPDATE game_house_scope_handoffs SET committed_at = 300",
            "DELETE FROM game_house_scope_handoffs",
        ] {
            assert!(sqlx::query(statement).execute(&harness.pool).await.is_err());
        }
        Ok(())
    })
}

#[test]
fn replacing_a_house_session_is_refused() -> TestResult {
    run("hsh_replace_refused", async |harness| {
        admit(harness).await?;
        // A reconnect into a house is a fresh entry handoff, never a terminal replacement.
        let mut tx = harness.pool.begin().await?;
        let error = replace_session(&mut tx, DESTINATION, 63)
            .await
            .err()
            .ok_or("replacement of a house session admitted")?;
        assert!(refused_replacement(&error), "{error:?}");
        tx.rollback().await?;
        let (_, live, _) = state(harness).await?;
        assert_eq!(live, vec![id(DESTINATION).to_vec()]);
        // Without a replacement receipt no session takes the house instance id bare.
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET session_state = 3 \
              WHERE game_session_id = encode($1,'hex')::uuid",
        )
        .bind(id(DESTINATION).as_slice())
        .execute(&harness.pool)
        .await?;
        let error = sqlx::query(CANDIDATE)
            .bind(id(DESTINATION).as_slice())
            .bind(id(64).as_slice())
            .execute(&harness.pool)
            .await
            .err()
            .ok_or("bare house instance session admitted")?;
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514")
        );
        assert!(error.to_string().contains("must name its house"), "{error}");
        Ok(())
    })
}

#[test]
fn a_restart_reconcile_keeps_another_nodes_prepared_handoff() -> TestResult {
    run("hsh_reconcile_scoped", async |harness| {
        // The PREPARED handoff belongs to node 1, which holds its origin Channel.
        prepare(harness, &harness.root, HANDOFF).await?;
        let other = register(&harness.root, 2).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // Node 2 restarts and reconciles: node 1's live handoff survives, for all Characters
        // and for its own Character alike.
        let character = CharacterId::decode(&id(CHARACTER)).map_err(debug)?;
        for filter in [None, Some(character)] {
            let report = harness
                .root
                .reconcile_house_entries(&authority, &other, filter)
                .await
                .map_err(debug)?;
            assert!(report.aborted.is_empty());
            let (source, live, handoffs) = state(harness).await?;
            assert_eq!((source, live, handoffs), (1, vec![id(SESSION).to_vec()], 1));
        }
        // Node 1's own restart reconcile aborts it.
        let report = harness
            .root
            .reconcile_house_entries(&authority, &harness.node, Some(character))
            .await
            .map_err(debug)?;
        assert_eq!(report.aborted, vec![id(HANDOFF)]);
        let (source, live, handoffs) = state(harness).await?;
        assert_eq!((source, live, handoffs), (1, vec![id(SESSION).to_vec()], 0));
        Ok(())
    })
}

#[test]
fn a_house_held_by_another_node_is_busy() -> TestResult {
    run("hsh_cross_node_busy", async |harness| {
        replace_house_onto_node_2(harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut cross = request(HANDOFF)?;
        cross.house_scope_ownership_generation = 2;
        assert_eq!(
            harness
                .root
                .prepare_house_entry(&authority, &harness.node, cross)
                .await
                .map_err(debug)?,
            HousePrepareOutcome::Refused(HouseEntryRefusal::Busy)
        );
        // No handoff row, so no tile reservation.
        let (source, live, handoffs) = state(harness).await?;
        assert_eq!((source, live, handoffs), (1, vec![id(SESSION).to_vec()], 0));
        Ok(())
    })
}

#[test]
fn a_revoked_origin_channel_fails_the_commit() -> TestResult {
    run("hsh_origin_revoked", async |harness| {
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest,
        };
        prepare(harness, &harness.root, HANDOFF).await?;
        let predecessor = harness
            .root
            .read_runtime_scope_predecessor(scope()?)
            .await
            .map_err(debug)?
            .ok_or("origin Channel assignment")?;
        let outcome = harness
            .writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([12; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Revoke {
                    scope: scope()?,
                    predecessor,
                },
            })
            .await
            .map_err(debug)?;
        assert!(
            matches!(outcome, AssignmentOutcome::Committed(_)),
            "{outcome:?}"
        );
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            harness
                .root
                .commit_house_entry(&authority, &harness.node, commit(HANDOFF)?)
                .await,
            Err(HouseHandoffError::AuthorityRejected)
        ));
        // The source session stays live, no house session exists and the handoff stays PREPARED.
        let (source, live, handoffs) = state(harness).await?;
        assert_eq!((source, live, handoffs), (1, vec![id(SESSION).to_vec()], 1));
        Ok(())
    })
}

#[test]
fn a_bare_first_session_of_an_assigned_house_is_refused() -> TestResult {
    run("hsh_bare_first", async |harness| {
        // No house session has ever existed; the house is assigned (seed).
        let error = sqlx::query(
            "INSERT INTO game_durability_reconnect_sessions (\
                game_session_id, account_id, character_id, world_id, runtime_scope_kind, \
                runtime_scope_world_id, runtime_scope_channel_id, runtime_scope_instance_id, \
                control_loss_epoch, original_grace_deadline, predecessor_generation, \
                character_lease_generation, scope_ownership_generation, current_generation, \
                attempt_count, session_state) \
             SELECT encode($2,'hex')::uuid, account_id, character_id, world_id, 2, \
                    runtime_scope_world_id, NULL, \
                    game_house_scope_instance_id(runtime_scope_world_id, $3), \
                    1, 500, current_generation, character_lease_generation, 1, \
                    current_generation, 0, 3 \
               FROM game_durability_reconnect_sessions \
              WHERE game_session_id = encode($1,'hex')::uuid",
        )
        .bind(id(SESSION).as_slice())
        .bind(id(65).as_slice())
        .bind(HOUSE_KEY)
        .execute(&harness.pool)
        .await
        .err()
        .ok_or("bare first house session admitted")?;
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514")
        );
        assert!(error.to_string().contains("must name its house"), "{error}");
        assert_eq!(
            harness_count(
                &harness.pool,
                "SELECT count(*) FROM game_durability_reconnect_sessions WHERE runtime_scope_kind = 2"
            )
            .await?,
            0
        );
        Ok(())
    })
}

#[test]
fn revocation_committed_first_refuses_the_admission() -> TestResult {
    run("hsh_revoke_first", async |harness| {
        prepare(harness, &harness.root, HANDOFF).await?;
        sqlx::query(
            "UPDATE test_house_properties SET guest = NULL, acl_revision = acl_revision + 1",
        )
        .execute(&harness.pool)
        .await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let outcome = harness
            .root
            .commit_house_entry(&authority, &harness.node, commit(HANDOFF)?)
            .await
            .map_err(debug)?;
        assert_eq!(
            outcome,
            HouseCommitOutcome::Refused(HouseEntryRefusal::NoAccess)
        );
        let (source, live, handoffs) = state(harness).await?;
        assert_eq!((source, live, handoffs), (1, vec![id(SESSION).to_vec()], 0));
        Ok(())
    })
}

#[test]
fn revocation_racing_the_commit_finds_the_character_inside() -> TestResult {
    run("hsh_revoke_race", async |harness| {
        prepare(harness, &harness.root, HANDOFF).await?;
        let mut tx = harness.pool.begin().await?;
        let outcome = commit_house_entry_in_transaction(&mut tx, &harness.node, &commit(HANDOFF)?)
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, HouseCommitOutcome::Committed(_)));
        // The commit transaction holds the property row FOR SHARE: the revocation waits.
        let pool = harness.pool.clone();
        let revocation = tokio::spawn(async move {
            let mut revoke = pool.begin().await?;
            sqlx::query(
                "UPDATE test_house_properties SET guest = NULL, acl_revision = acl_revision + 1",
            )
            .execute(&mut *revoke)
            .await?;
            let inside: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions \
                  WHERE runtime_scope_house_key IS NOT NULL AND session_state IN (1,2) \
                    AND character_id = encode($1,'hex')::uuid)",
            )
            .bind(id(CHARACTER).as_slice())
            .fetch_one(&mut *revoke)
            .await?;
            revoke.commit().await?;
            Ok::<bool, sqlx::Error>(inside)
        });
        let mut waiting = false;
        for _ in 0..200 {
            waiting = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE wait_event_type = 'Lock' \
                  AND query LIKE 'UPDATE test_house_properties%')",
            )
            .fetch_one(&harness.pool)
            .await?;
            if waiting {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(waiting, "the revocation must wait for the admission commit");
        tx.commit().await?;
        assert!(revocation.await??);
        let (source, live, handoffs) = state(harness).await?;
        assert_eq!(
            (source, live, handoffs),
            (3, vec![id(DESTINATION).to_vec()], 1)
        );
        Ok(())
    })
}

#[test]
fn entry_refusals_and_fences_hold() -> TestResult {
    run("hsh_refusals", async |harness| {
        let root = &harness.root;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let node = &harness.node;

        // The exit into a Channel scope stays refused before any durable read.
        let mut exit = request(HANDOFF)?;
        exit.direction = HouseHandoffDirection::Exit;
        assert!(matches!(
            root.prepare_house_entry(&authority, node, exit).await,
            Err(HouseHandoffError::ExitNotAdmitted)
        ));
        // A stale source lease generation is not the live session.
        let mut stale = request(HANDOFF)?;
        stale.source_character_lease_generation = 2;
        assert!(matches!(
            root.prepare_house_entry(&authority, node, stale).await,
            Err(HouseHandoffError::AuthorityRejected)
        ));
        // A house scope generation that is not current is a closed house.
        let mut closed = request(HANDOFF)?;
        closed.house_scope_ownership_generation = 2;
        assert_eq!(
            root.prepare_house_entry(&authority, node, closed)
                .await
                .map_err(debug)?,
            HousePrepareOutcome::Refused(HouseEntryRefusal::HouseClosed)
        );
        assert_eq!(harness.count("game_house_scope_handoffs").await?, 0);

        // One open handoff per Character; the same id replays.
        assert!(matches!(
            root.prepare_house_entry(&authority, node, request(HANDOFF)?)
                .await
                .map_err(debug)?,
            HousePrepareOutcome::Prepared(_)
        ));
        assert!(matches!(
            root.prepare_house_entry(&authority, node, request(HANDOFF)?)
                .await
                .map_err(debug)?,
            HousePrepareOutcome::Replayed(_)
        ));
        assert_eq!(
            root.prepare_house_entry(&authority, node, request(63)?)
                .await
                .map_err(debug)?,
            HousePrepareOutcome::Refused(HouseEntryRefusal::Busy)
        );

        // The database admits only entries in the PREPARED state.
        assert!(
            sqlx::query("UPDATE game_house_scope_handoffs SET direction = 2")
                .execute(&harness.pool)
                .await
                .is_err()
        );

        // A house without a grant cannot be assigned.
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        let other = HouseId::new(world, "oteryn:content.house.thais_2").ok_or("house")?;
        assert_eq!(
            root.assign_house_scope(house_request(
                9,
                HouseScopeAssignmentCommand::Assign {
                    house: other,
                    target: node.fact(),
                },
            )?)
            .await
            .map_err(debug)?,
            HouseScopeAssignmentOutcome::Rejected(AssignmentRejection::NotGranted)
        );

        // Revoking the house scope leaves no holder to commit; the entry is aborted.
        let revoked = root
            .assign_house_scope(house_request(
                10,
                HouseScopeAssignmentCommand::Revoke {
                    house: house()?,
                    predecessor: HouseScopePredecessor {
                        ownership_generation: 1,
                        source_revision: 2,
                    },
                },
            )?)
            .await
            .map_err(debug)?;
        let HouseScopeAssignmentOutcome::Committed(revoked) = revoked else {
            return Err(format!("unexpected revoke: {revoked:?}").into());
        };
        assert!(!revoked.assigned);
        // Like the Channel writer, a revoked house scope cannot be revoked again.
        assert_eq!(
            root.assign_house_scope(house_request(
                11,
                HouseScopeAssignmentCommand::Revoke {
                    house: house()?,
                    predecessor: revoked.predecessor(),
                },
            )?)
            .await
            .map_err(debug)?,
            HouseScopeAssignmentOutcome::Rejected(AssignmentRejection::NotAssigned)
        );
        assert!(matches!(
            root.commit_house_entry(&authority, node, commit(HANDOFF)?)
                .await,
            Err(HouseHandoffError::AuthorityRejected)
        ));
        assert_eq!(
            root.abort_house_entry(&authority, id(HANDOFF))
                .await
                .map_err(debug)?,
            HouseAbortOutcome::Aborted
        );
        let (source, live, handoffs) = state(harness).await?;
        assert_eq!((source, live, handoffs), (1, vec![id(SESSION).to_vec()], 0));
        Ok(())
    })
}

#[test]
fn the_access_stub_and_a_changed_guild_revision_refuse_no_access() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "hsh_access_stub", false).await?;
        seed(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // The migration stub returns no row: every house refuses NO_ACCESS.
        prepare(&harness, &harness.root, HANDOFF).await?;
        assert_eq!(
            harness
                .root
                .commit_house_entry(&authority, &harness.node, commit(HANDOFF)?)
                .await
                .map_err(debug)?,
            HouseCommitOutcome::Refused(HouseEntryRefusal::NoAccess)
        );
        let (source, live, handoffs) = state(&harness).await?;
        assert_eq!((source, live, handoffs), (1, vec![id(SESSION).to_vec()], 0));

        // A guild-entry pre-check whose revisions the access function no longer returns.
        install_test_house(&harness).await?;
        let mut guild = request(HANDOFF + 2)?;
        guild.guild_revisions = Some(serde_json::json!({ "guild": 1 }));
        let prepared = harness
            .root
            .prepare_house_entry(&authority, &harness.node, guild.clone())
            .await
            .map_err(debug)?;
        assert!(
            matches!(prepared, HousePrepareOutcome::Prepared(ref record)
                if record.guild_revisions == guild.guild_revisions),
            "{prepared:?}"
        );
        assert!(matches!(
            harness
                .root
                .prepare_house_entry(&authority, &harness.node, guild.clone())
                .await
                .map_err(debug)?,
            HousePrepareOutcome::Replayed(_)
        ));
        let mut changed = guild.clone();
        changed.guild_revisions = Some(serde_json::json!({ "guild": 2 }));
        assert!(matches!(
            harness
                .root
                .prepare_house_entry(&authority, &harness.node, changed)
                .await,
            Err(HouseHandoffError::InvalidInput)
        ));
        assert_eq!(
            harness
                .root
                .commit_house_entry(&authority, &harness.node, commit(HANDOFF + 2)?)
                .await
                .map_err(debug)?,
            HouseCommitOutcome::Refused(HouseEntryRefusal::NoAccess)
        );
        let (source, live, handoffs) = state(&harness).await?;
        assert_eq!((source, live, handoffs), (1, vec![id(SESSION).to_vec()], 0));
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

// A house scope assignment is not a Channel (#1782 review): its receipt (NULL `channel_id`) only
// conflicts with a reused Channel operation key, and a World whose only assigned scope is a house
// has no assigned Channel, so Character bootstrap is refused.
#[test]
fn a_house_scope_is_never_read_as_a_channel() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let database = crate::Database::create(admin, "hsh_not_channel").await?;
        let result = house_scope_is_never_a_channel(&database).await;
        database.cleanup().await?;
        result
    })
}

async fn house_scope_is_never_a_channel(database: &crate::Database) -> TestResult {
    use crate::durability::character_authority::CharacterAuthorityError;
    use crate::durability::runtime_scope_assignment::{
        AssignmentCommand, AssignmentOutcome, AssignmentRequest, AssignmentState,
        RuntimeScopeAssignmentWriter,
    };
    use crate::foundation::{ChannelId, RuntimeScopeRefV1};
    let root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&database.url).await?;
    let retained = crate::fence_parent().join(format!("oteryn-house-scope-{}", database.name));
    let _ = std::fs::remove_dir_all(&retained);
    std::fs::create_dir(&retained)?;
    let recovery = crate::CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .map_err(debug)?;
    {
        let fresh = recovery
            .authorize_fresh_store(crate::id(11), 100)
            .map_err(debug)?;
        root.admit_fresh_character_recovery(&fresh)
            .await
            .map_err(debug)?;
    }
    let fence = recovery.seal_current().map_err(debug)?;
    let authority = root.open_character_authority(&fence).await.map_err(debug)?;
    let node = crate::register(&root, 1, None).await?;
    crate::initialize_s2(&root, &node).await?;
    crate::allow(&root, &node, 70).await?;
    crate::allow(&root, &node, 71).await?;
    assert_eq!(
        crate::configure(&pool, ["profile-1", "ruleset-1", "content-1", "starter-1"]).await?,
        1
    );

    // World 90: Channel 95 and a house, both held by node 1.
    let world = WorldId::decode(&id(90)).map_err(debug)?;
    let channel = RuntimeScopeRefV1::channel(world, ChannelId::decode(&id(95)).map_err(debug)?);
    let house = HouseId::new(world, HOUSE_KEY).ok_or("house key")?;
    sqlx::query(
        "INSERT INTO game_control_scope_grants (control_role, world_id, channel_id, operation) \
         SELECT session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, operation \
           FROM unnest(ARRAY[1, 3]::SMALLINT[]) AS operation",
    )
    .bind(id(90).as_slice())
    .bind(id(95).as_slice())
    .execute(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_control_house_scope_grants (control_role, world_id, house_key, operation) \
         VALUES (session_user, encode($1,'hex')::uuid, $2, 1)",
    )
    .bind(id(90).as_slice())
    .bind(HOUSE_KEY)
    .execute(&pool)
    .await?;
    let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "house-scope-writer")
        .await
        .map_err(debug)?;
    let channel_request = |key: u8, command| -> TestResult<AssignmentRequest> {
        Ok(AssignmentRequest {
            operation_key: OperationKey::from_bytes([key; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command,
        })
    };
    let outcome = writer
        .submit(&channel_request(
            95,
            AssignmentCommand::Assign {
                scope: channel,
                target: node.fact(),
            },
        )?)
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(assigned) = outcome else {
        return Err(format!("unexpected Channel assignment: {outcome:?}").into());
    };
    let outcome = root
        .assign_house_scope(house_request(
            96,
            HouseScopeAssignmentCommand::Assign {
                house,
                target: node.fact(),
            },
        )?)
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, HouseScopeAssignmentOutcome::Committed(_)),
        "{outcome:?}"
    );

    // The house operation key reused for a Channel command is an ordinary conflict, and the
    // writer stays usable.
    let revoke = AssignmentCommand::Revoke {
        scope: channel,
        predecessor: assigned.predecessor(),
    };
    assert_eq!(
        writer
            .submit(&channel_request(96, revoke)?)
            .await
            .map_err(debug)?,
        AssignmentOutcome::Rejected(AssignmentRejection::OperationConflict)
    );

    // With the Channel assigned, bootstrap admits; once only the house is assigned, it refuses.
    root.bootstrap_character(&authority, &node, &crate::intent(61, 71, 10)?)
        .await
        .map_err(debug)?;
    let outcome = writer
        .submit(&channel_request(97, revoke)?)
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, AssignmentOutcome::Committed(ref receipt) if receipt.assignment.state == AssignmentState::Revoked),
        "{outcome:?}"
    );
    assert!(matches!(
        root.bootstrap_character(&authority, &node, &crate::intent(60, 70, 11)?)
            .await,
        Err(CharacterAuthorityError::Rejected)
    ));
    assert_eq!(
        harness_count(&pool, "SELECT count(*) FROM game_character_roots").await?,
        1
    );
    drop(writer);
    drop(authority);
    pool.close().await;
    std::fs::remove_dir_all(retained)?;
    Ok(())
}

async fn harness_count(pool: &sqlx::PgPool, sql: &'static str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(sql).fetch_one(pool).await?)
}

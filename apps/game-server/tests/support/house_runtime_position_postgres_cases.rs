//! Native House position storage and common Channel writer qualification.
#![allow(clippy::expect_used)]

use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, WORLD, configured_admin, debug, fence, id, runtime,
    scope,
};
use crate::durability::character_last_position::{
    FinalLastPosition, LastPositionCell, LastPositionRecord, LastPositionWrite, LastVitals,
    PeriodicLastPosition,
};
use crate::durability::character_position_house::HousePositionRecord;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::house_scope_handoff::{
    HouseCommitOutcome, HouseEntryCommit, HouseEntryRequest, HouseHandoffDirection, HouseId,
    HousePrepareOutcome, HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome,
    HouseScopeAssignmentRequest, HouseScopePredecessor,
};
use crate::durability::runtime_scope_assignment::{
    BootstrapSecret, ControlActor, NodeIncarnationProof, OperationKey,
};
use crate::foundation::admission_authority_publication::*;
use crate::foundation::fnd04_verifier::{
    Fnd04EvidenceScope, FreshAccountSecurityObservationV1, FreshEvidenceProvenanceV1,
    FreshEvidencePurposeV1,
};
use crate::foundation::{
    CharacterId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration, WorldId,
};
use sqlx::{Executor, Row};

const HOUSE: &str = "oteryn:content.house.thais_1";
const DESTINATION: u8 = 61;

/// Keep the owned database available to cleanup even when a case assertion panics.
/// The canonical harness owns only this case's unique database; the shared PG server lives on.
fn run_house_position_case(
    tag: &str,
    initialized: bool,
    case: impl for<'a> FnOnce(
        &'a std::rc::Rc<Harness>,
    )
        -> std::pin::Pin<Box<dyn std::future::Future<Output = TestResult> + 'a>>,
) -> TestResult {
    let Some(admin) = configured_admin() else {
        // D1366: configured_admin reports PRE-ROUTING / NONCANONICAL here.
        // A missing routed database never emits a PostgreSQL qualification marker.
        return Ok(());
    };
    let runtime = runtime()?;
    let harness = std::rc::Rc::new(runtime.block_on(Harness::create(admin, tag, initialized))?);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(case(&harness))
    }));
    let cleanup = match std::rc::Rc::try_unwrap(harness) {
        Ok(harness) => runtime.block_on(harness.cleanup()),
        Err(_) => Err("House-position fixture still borrowed during cleanup".into()),
    };
    match outcome {
        Ok(result) => {
            cleanup?;
            result?;
            println!("QUALIFIED PostgreSQL17.6: {tag}; case completed and own database removed");
            Ok(())
        }
        Err(panic) => {
            if let Err(error) = cleanup {
                eprintln!("House-position cleanup failed after the case panicked: {error}");
            }
            std::panic::resume_unwind(panic)
        }
    }
}

fn house() -> TestResult<HouseId> {
    HouseId::new(WorldId::decode(&id(WORLD)).map_err(debug)?, HOUSE).ok_or_else(|| "House".into())
}
fn house_fence() -> TestResult<CurrentCharacterGameplayFence> {
    Ok(CurrentCharacterGameplayFence {
        game_session_id: GameSessionId::decode(&id(DESTINATION)).map_err(debug)?,
        character_lease_generation: 2,
        runtime_scope: house()?.runtime_scope()?,
        ..fence(1)?
    })
}
fn capture() -> TestResult<HousePositionRecord> {
    Ok(HousePositionRecord {
        position: LastPositionRecord {
            world_id: WorldId::decode(&id(WORLD)).map_err(debug)?,
            cell: LastPositionCell {
                x: 21,
                y: 22,
                floor: 7,
            },
            coordinate_frame_digest: [1; 32],
            map_revision_digest: [2; 32],
            server_artifact_digest: [3; 32],
            vitals: LastVitals::new(73, 19),
        },
        house: Some(house()?),
    })
}
fn uuid(seed: u8) -> String {
    let b = id(seed);
    let h: String = b.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}

/// Negative publication attempt only: no successful House claim/admission proof is asserted.
struct GuardPublicationAttemptFixture(AdmissionAuthorityPublicationChangeV1);
impl crate::foundation::fnd04_verifier::fresh_source_sealed::Sealed
    for GuardPublicationAttemptFixture
{
}
impl AdmissionAuthorityOwningPublisherV1 for GuardPublicationAttemptFixture {
    fn resolve_publication(
        &self,
        _: i64,
    ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
    {
        Ok(vec![self.0.clone()])
    }
}
async fn rejected_claim_publication(
    h: &Harness,
    c: &AdmissionAuthorityPublicationChangeV1,
) -> TestResult {
    let attempt = AdmissionAuthorityPublicationV1::prepare(
        &GuardPublicationAttemptFixture(c.clone()),
        c.source.source_observed_at,
    )
    .map_err(debug)?;
    let result = crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(
        h.root.clone(),
    )
    .publish(&attempt)
    .await?;
    if result
        != crate::durability::admission_authority_guards::GuardPublicationDisposition::Conflict
    {
        return Err("native guard claim publication unexpectedly did not refuse".into());
    }
    Ok(())
}
fn runtime_state() -> AdmissionAuthorityGuardStateV1 {
    AdmissionAuthorityGuardStateV1::Runtime {
        ownership_generation: 1,
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
    }
}
fn guard_states(
    now: i64,
) -> TestResult<
    Vec<(
        AdmissionAuthorityGuardKeyV1,
        AdmissionAuthorityGuardStateV1,
        AdmissionPublicationPurposeV1,
    )>,
> {
    let character = CharacterId::decode(&id(CHARACTER)).map_err(debug)?;
    let session = GameSessionId::decode(&id(DESTINATION)).map_err(debug)?;
    let provenance = FreshEvidenceProvenanceV1 {
        source_authority: "house-position-independent-fixture".into(),
        purpose: FreshEvidencePurposeV1::PlatformSecurity,
        scope: Fnd04EvidenceScope::FreshAdmission,
        source_revision: 1,
        accepted_source_revision: 1,
        decision_identity: "fixture-security-1".into(),
        accepted_decision_identity: "fixture-security-1".into(),
        source_observed_at: now,
        clock_uncertainty_seconds: 0,
        publication_revision: 1,
    };
    Ok(vec![
        (
            AdmissionAuthorityGuardKeyV1::Character(character),
            AdmissionAuthorityGuardStateV1::Character {
                account_id: uuid(40),
                world_id: WorldId::decode(&id(WORLD)).map_err(debug)?,
                eligible: true,
                lease_generation: 2,
                holder: Some(session),
            },
            AdmissionPublicationPurposeV1::CharacterOwnershipAndLease,
        ),
        (
            AdmissionAuthorityGuardKeyV1::Account {
                account_id: uuid(40),
            },
            AdmissionAuthorityGuardStateV1::Account {
                security: FreshAccountSecurityObservationV1 {
                    account_id: uuid(40),
                    minimum_generation: 1,
                    allowed: true,
                    provenance,
                },
                presence: Some((character, session)),
            },
            AdmissionPublicationPurposeV1::AccountSecurityAndPresence,
        ),
        (
            AdmissionAuthorityGuardKeyV1::Runtime(house()?.runtime_scope()?),
            runtime_state(),
            AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
        ),
    ])
}
/// SYNTHETIC INDEPENDENT current guard DATA, not successful publication/claim permission.
/// The real ordinary publisher correctly rejects bootstrap holder/presence claims.
async fn install_guard_fixture(
    h: &Harness,
    c: &AdmissionAuthorityPublicationChangeV1,
) -> TestResult {
    crate::durability::character_position_house::install_synthetic_independent_guard_data(
        &h.root, c,
    )
    .await
    .map_err(debug)?;
    Ok(())
}
async fn replace_guard(
    h: &Harness,
    c: &mut AdmissionAuthorityPublicationChangeV1,
    state: AdmissionAuthorityGuardStateV1,
) -> TestResult {
    let prior = c.state.clone();
    c.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
        expected_publication_revision: c.publication_revision,
    };
    c.publication_revision += 1;
    c.source.source_revision += 1;
    c.source.decision_identity = format!("fixture-{}", c.publication_revision);
    c.state = state;
    if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut c.state {
        // Rebind the unchanged native observation wrapper; source time/revision/decision stay intact.
        security.provenance.publication_revision = c.publication_revision;
    }
    let changes_claim = match (&prior, &c.state) {
        (
            AdmissionAuthorityGuardStateV1::Character { holder: a, .. },
            AdmissionAuthorityGuardStateV1::Character { holder: b, .. },
        ) => a != b,
        (
            AdmissionAuthorityGuardStateV1::Account { presence: a, .. },
            AdmissionAuthorityGuardStateV1::Account { presence: b, .. },
        ) => a != b,
        _ => false,
    };
    if changes_claim {
        rejected_claim_publication(h, c).await?;
    }
    install_guard_fixture(h, c).await
}

/// Actual existing assignment + prepare/commit APIs create House session61/lease2. Native
/// handoff does not rebind admission guards or install a controller; independently installed canonical native
/// fixture guards/history below qualify STORAGE ONLY. The cell/pins are raw capture fixtures, not
/// physical placement facts or permission to enter/exit. No production table/function is installed.
async fn setup(h: &Harness) -> TestResult<Vec<AdmissionAuthorityPublicationChangeV1>> {
    let expected_checksum = sqlx::migrate!("./migrations")
        .iter()
        .find(|migration| migration.version == 108)
        .map(|migration| migration.checksum.to_vec())
        .ok_or("House position requires registered migration 0108")?;
    let installed: Option<(Vec<u8>, bool)> =
        sqlx::query_as("SELECT checksum, success FROM _sqlx_migrations WHERE version = 108")
            .fetch_optional(&h.pool)
            .await?;
    if installed != Some((expected_checksum, true)) {
        return Err("House position requires the exact installed SQLx migration 0108".into());
    }
    sqlx::query(
        "INSERT INTO game_durability_session_use_ledgers VALUES(encode($1,'hex')::uuid,1,true,1,1)",
    )
    .bind(id(CHARACTER).as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_durability_session_use_memberships VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL)").bind(id(SESSION).as_slice()).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants(control_role,world_id,house_key,operation) SELECT session_user,encode($1,'hex')::uuid,$2,operation FROM unnest(ARRAY[1,2,3]::SMALLINT[]) AS operation").bind(id(WORLD).as_slice()).bind(HOUSE).execute(&h.pool).await?;
    let assigned = h
        .root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([8; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Assign {
                house: house()?,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    if !matches!(assigned, HouseScopeAssignmentOutcome::Committed(_)) {
        return Err("fixture native House assignment refused".into());
    }
    // Accepted §10 test house only, the same lock-bearing fixture function as native handoff tests.
    h.pool.execute("CREATE TABLE test_house_properties(world_id UUID,house_key TEXT,acl_revision NUMERIC(20,0),open BOOLEAN,guest UUID,PRIMARY KEY(world_id,house_key))").await?;
    sqlx::query("INSERT INTO test_house_properties VALUES(encode($1,'hex')::uuid,$2,1,true,encode($3,'hex')::uuid)").bind(id(WORLD).as_slice()).bind(HOUSE).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    h.pool.execute("CREATE OR REPLACE FUNCTION game_house_access(p_world_id UUID,p_house_key TEXT,p_character_id UUID) RETURNS TABLE(role TEXT,content_fenced BOOLEAN,acl_revision NUMERIC(20,0),guild_revisions JSONB) LANGUAGE sql AS $$ SELECT CASE WHEN p.guest=p_character_id THEN 'GUEST' END,NOT p.open,p.acl_revision,NULL::JSONB FROM test_house_properties p WHERE p.world_id=p_world_id AND p.house_key=p_house_key FOR SHARE $$").await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let prepared = h
        .root
        .prepare_house_entry(
            &authority,
            &h.node,
            HouseEntryRequest {
                direction: HouseHandoffDirection::Entry,
                handoff_id: id(60),
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
            },
        )
        .await
        .map_err(debug)?;
    if !matches!(prepared, HousePrepareOutcome::Prepared(_)) {
        return Err("fixture native House prepare refused".into());
    }
    let committed = h
        .root
        .commit_house_entry(
            &authority,
            &h.node,
            HouseEntryCommit {
                handoff_id: id(60),
                destination_game_session_id: GameSessionId::decode(&id(DESTINATION))
                    .map_err(debug)?,
                committed_at: 200,
            },
        )
        .await
        .map_err(debug)?;
    if !matches!(committed, HouseCommitOutcome::Committed(_)) {
        return Err("fixture native House commit refused".into());
    }
    // Remove ONLY the old harness's raw fixture projections ('{}', no history), then install
    // separately controlled canonical native fixture rows/history for the destination. No admission/claim-transition/controller claim.
    h.pool.execute("DELETE FROM game_durability_admission_character_guards; DELETE FROM game_durability_admission_account_guards").await?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut guards = Vec::new();
    for (i, (key, state, purpose)) in guard_states(now)?.into_iter().enumerate() {
        let c = AdmissionAuthorityPublicationChangeV1 {
            key,
            state,
            source: AdmissionPublicationSourceV1 {
                authority: format!("house-position-fixture-{i}"),
                purpose,
                source_revision: 1,
                decision_identity: "fixture-1".into(),
                source_observed_at: now,
                clock_uncertainty_seconds: 0,
            },
            precondition: AdmissionPublicationPreconditionV1::Bootstrap {
                restored_publication_high_water: Some(0),
            },
            publication_revision: 1,
        };
        if matches!(
            c.state,
            AdmissionAuthorityGuardStateV1::Character { .. }
                | AdmissionAuthorityGuardStateV1::Account { .. }
        ) {
            rejected_claim_publication(h, &c).await?;
        }
        // Native public Bootstrap/CAS refusal is preserved. This explicit cfg(test) seed
        // installs SYNTHETIC INDEPENDENT guard DATA; no sealed claim predicate is admitted.
        install_guard_fixture(h, &c).await?;
        guards.push(c);
    }
    Ok(guards)
}

async fn write(
    h: &Harness,
    f: CurrentCharacterGameplayFence,
    seq: u64,
    r: HousePositionRecord,
) -> TestResult<LastPositionWrite> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(|e| format!("open native authority before position sequence{seq}: {e:?}"))?;
    h.root
        .write_character_position_house(&authority, &h.node, f, seq, r)
        .await
        .map_err(|e| format!("native position write sequence{seq}: {e:?}").into())
}
async fn read(
    h: &Harness,
) -> TestResult<Option<crate::durability::character_position_house::StoredHousePosition>> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .read_character_position_house(&authority, &h.node, house_fence()?)
        .await
        .map_err(|e| debug(e).into())
}
async fn rejected(
    h: &Harness,
    f: CurrentCharacterGameplayFence,
    r: HousePositionRecord,
) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    assert!(matches!(
        h.root
            .write_character_position_house(&authority, &h.node, f, 100, r)
            .await,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_last_positions")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 0);
    Ok(())
}

#[derive(Clone, Copy)]
enum NativeChannelWriter {
    Single,
    Batch,
}

async fn native_channel_write(
    h: &Harness,
    writer: NativeChannelWriter,
    current: CurrentCharacterGameplayFence,
    write_sequence: u64,
    record: LastPositionRecord,
) -> TestResult<LastPositionWrite> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let outcome = match writer {
        NativeChannelWriter::Single => {
            h.root
                .write_character_last_position(&authority, &h.node, current, write_sequence, record)
                .await
        }
        NativeChannelWriter::Batch => {
            let mut outcomes = h
                .root
                .write_character_last_positions(
                    &authority,
                    &h.node,
                    vec![PeriodicLastPosition {
                        fence: current,
                        write_sequence,
                        record,
                    }],
                )
                .await
                .map_err(debug)?;
            assert_eq!(outcomes.len(), 1);
            outcomes
                .pop()
                .ok_or("native periodic batch omitted its outcome")?
        }
    };
    outcome.map_err(|error| debug(error).into())
}

async fn position_row(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT to_jsonb(p)::text FROM game_character_last_positions p \
         WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

/// A persisted historical House pair is fixture DATA, while the live session remains the
/// independently admitted native Channel fixture. This grants no House entry or exit.
/// The fixture is a real row mutation and must obey the native strict order guard.
async fn mark_historical_house_pair(h: &Harness) -> TestResult<u64> {
    let sequence: String = sqlx::query_scalar(
        "UPDATE game_character_last_positions \
         SET house_world_id = world_id, house_key = $2, write_sequence = write_sequence + 1 \
         WHERE character_id = encode($1,'hex')::uuid RETURNING write_sequence::text",
    )
    .bind(id(CHARACTER).as_slice())
    .bind(HOUSE)
    .fetch_one(&h.pool)
    .await?;
    Ok(sequence.parse()?)
}

async fn assert_channel_position(
    h: &Harness,
    record: LastPositionRecord,
    write_sequence: u64,
) -> TestResult {
    let both_null: bool = sqlx::query_scalar(
        "SELECT house_world_id IS NULL AND house_key IS NULL \
         FROM game_character_last_positions WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert!(
        both_null,
        "the winning native Channel write retained a House binding"
    );
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let stored = h
        .root
        .read_character_last_position(&authority, &h.node, fence(1)?)
        .await
        .map_err(debug)?
        .ok_or("missing native Channel position")?;
    assert_eq!(stored.record, record);
    assert_eq!(stored.key.character_lease_generation, 1);
    assert_eq!(stored.key.write_sequence, write_sequence);
    Ok(())
}

async fn install_position_commit_failure(h: &Harness) -> TestResult {
    h.pool.execute(
        "CREATE FUNCTION test_position_commit_failure() RETURNS trigger LANGUAGE plpgsql AS \
         $$ BEGIN RAISE EXCEPTION 'fixture deferred position failure' USING ERRCODE='23514'; END $$; \
         CREATE CONSTRAINT TRIGGER test_position_commit_failure AFTER INSERT OR UPDATE \
         ON game_character_last_positions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW \
         EXECUTE FUNCTION test_position_commit_failure()",
    ).await?;
    Ok(())
}

async fn remove_position_commit_failure(h: &Harness) -> TestResult {
    h.pool
        .execute(
            "DROP TRIGGER test_position_commit_failure ON game_character_last_positions; \
         DROP FUNCTION test_position_commit_failure()",
        )
        .await?;
    Ok(())
}

async fn native_channel_clear_case(h: &Harness, writer: NativeChannelWriter) -> TestResult {
    let original = capture()?.position;
    assert_eq!(
        native_channel_write(h, writer, fence(1)?, 10, original).await?,
        LastPositionWrite::Written
    );
    let historical_sequence = mark_historical_house_pair(h).await?;
    let next_sequence = historical_sequence
        .checked_add(1)
        .ok_or("fixture sequence exhausted")?;
    let before = position_row(h).await?;
    let mut changed = original;
    changed.cell.x += 5;
    changed.vitals = LastVitals::new(25, 0);

    // Equal and lower keys cannot clear only the House pair or overwrite any other field.
    for sequence in [historical_sequence, historical_sequence - 1] {
        assert_eq!(
            native_channel_write(h, writer, fence(1)?, sequence, changed).await?,
            LastPositionWrite::Superseded
        );
        assert_eq!(position_row(h).await?, before);
    }
    let stale = CurrentCharacterGameplayFence {
        connection_generation: ConnectionGeneration::new(2).map_err(debug)?,
        ..fence(1)?
    };
    assert!(
        native_channel_write(h, writer, stale, next_sequence, changed)
            .await
            .is_err()
    );
    assert_eq!(position_row(h).await?, before);
    let wrong_world = LastPositionRecord {
        world_id: WorldId::decode(&id(90)).map_err(debug)?,
        ..changed
    };
    assert!(
        native_channel_write(h, writer, fence(1)?, next_sequence, wrong_world)
            .await
            .is_err()
    );
    assert_eq!(position_row(h).await?, before);

    // The database rejects at COMMIT, after the real upsert ran. The entire historical
    // record, including the House pair, must survive the rolled-back native transaction.
    install_position_commit_failure(h).await?;
    assert!(
        native_channel_write(h, writer, fence(1)?, next_sequence, changed)
            .await
            .is_err()
    );
    assert_eq!(position_row(h).await?, before);
    remove_position_commit_failure(h).await?;

    // Even an unchanged cell and vitals clear both columns when their key wins.
    assert_eq!(
        native_channel_write(h, writer, fence(1)?, next_sequence, original).await?,
        LastPositionWrite::Written
    );
    assert_channel_position(h, original, next_sequence).await?;
    let next_sequence = mark_historical_house_pair(h)
        .await?
        .checked_add(1)
        .ok_or("fixture sequence exhausted")?;
    assert_eq!(
        native_channel_write(h, writer, fence(1)?, next_sequence, changed).await?,
        LastPositionWrite::Written
    );
    assert_channel_position(h, changed, next_sequence).await?;

    // A historical row of a newer lease cannot be changed by a late capture, even
    // with a greater sequence in the still independently valid older session.
    mark_historical_house_pair(h).await?;
    h.pool.execute("UPDATE game_character_last_positions SET character_lease_generation=2,write_sequence=1").await?;
    let newer_lease = position_row(h).await?;
    assert_eq!(
        native_channel_write(h, writer, fence(1)?, 999, original).await?,
        LastPositionWrite::Superseded
    );
    assert_eq!(position_row(h).await?, newer_lease);
    assert_eq!(h.root_revision().await?, "1");
    Ok(())
}

#[test]
fn house_projection_native_single_channel_write_clears_atomically() -> TestResult {
    run_house_position_case("house_pos_native_single", true, |h| {
        Box::pin(async move { native_channel_clear_case(h, NativeChannelWriter::Single).await })
    })
}

#[test]
fn house_projection_native_batch_channel_write_clears_atomically() -> TestResult {
    run_house_position_case("house_pos_native_batch", true, |h| {
        Box::pin(async move { native_channel_clear_case(h, NativeChannelWriter::Batch).await })
    })
}

#[test]
fn house_projection_orders_full_native_row_and_reloads() -> TestResult {
    run_house_position_case("house_pos_order", true, |h| {
        Box::pin(async move {
            setup(h).await?;
            assert!(read(h).await?.is_none());
            let initial = capture()?;
            assert_eq!(
                write(h, house_fence()?, 10, initial.clone()).await?,
                LastPositionWrite::Written
            );
            let stored = read(h).await?.ok_or("missing")?;
            assert_eq!(stored.house, initial.house);
            assert_eq!(stored.position.record, initial.position);
            assert_eq!(stored.position.key.character_lease_generation, 2);
            assert_eq!(stored.position.key.write_sequence, 10);
            let mut changed = initial.clone();
            changed.position.cell.x += 1;
            changed.position.vitals = LastVitals::new(12, 0);
            assert_eq!(
                write(h, house_fence()?, 10, changed.clone()).await?,
                LastPositionWrite::Superseded
            );
            assert_eq!(
                write(h, house_fence()?, 9, changed.clone()).await?,
                LastPositionWrite::Superseded
            );
            assert_eq!(read(h).await?.ok_or("missing")?, stored);
            assert_eq!(
                write(h, house_fence()?, 11, changed.clone()).await?,
                LastPositionWrite::Written
            );
            assert_eq!(
                read(h).await?.ok_or("missing")?.position.record,
                changed.position
            );
            let mut higher = changed.clone();
            higher.position.cell.x = 99;
            higher.position.vitals = None;
            // The native Root intentionally has ONE ready holder. Two independently warmed
            // actual Roots qualify database serialization; no production capacity/controller claim.
            let other = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
            if !other.maintain_ready_once().await? {
                return Err("second native Root not ready".into());
            }
            let first = std::rc::Rc::clone(h);
            let second = std::rc::Rc::clone(h);
            let f = house_fence()?;
            let later = higher.clone();
            tokio::task::LocalSet::new()
                .run_until(async move {
                    let a = tokio::task::spawn_local(
                        async move { write(&first, f, 12, changed).await },
                    );
                    let b = tokio::task::spawn_local(async move {
                        let seal = second.recovery.seal_current().map_err(debug)?;
                        let authority = other
                            .open_character_authority(&seal)
                            .await
                            .map_err(|e| format!("second native Root authority: {e:?}"))?;
                        let result: TestResult<LastPositionWrite> = other
                            .write_character_position_house(&authority, &second.node, f, 13, later)
                            .await
                            .map_err(|e| {
                                format!("second native Root position write: {e:?}").into()
                            });
                        result
                    });
                    a.await??;
                    b.await??;
                    TestResult::Ok(())
                })
                .await?;
            let reloaded = read(h).await?.ok_or("missing")?;
            assert_eq!(reloaded.position.key.write_sequence, 13);
            assert_eq!(reloaded.position.record, higher.position);
            assert_eq!(h.root_revision().await?, "1");
            TestResult::Ok(())
        })
    })
}

#[test]
fn house_projection_rejects_independently_changed_current_fences() -> TestResult {
    run_house_position_case("house_pos_fences", true, |h| {
        Box::pin(async move {
            let mut guards = setup(h).await?;
            let good = house_fence()?;
            for f in [
                CurrentCharacterGameplayFence {
                    game_session_id: GameSessionId::decode(&id(62)).map_err(debug)?,
                    ..good
                },
                CurrentCharacterGameplayFence {
                    connection_generation: ConnectionGeneration::new(2).map_err(debug)?,
                    ..good
                },
                CurrentCharacterGameplayFence {
                    character_lease_generation: 3,
                    ..good
                },
                CurrentCharacterGameplayFence {
                    scope_ownership_generation: ScopeOwnershipGeneration::new(2).map_err(debug)?,
                    ..good
                },
                CurrentCharacterGameplayFence {
                    runtime_scope: HouseId::new(
                        good.runtime_scope.world_id(),
                        "oteryn:content.house.other",
                    )
                    .ok_or("other")?
                    .runtime_scope()?,
                    ..good
                },
            ] {
                rejected(h, f, capture()?).await?;
            }
            let mut wrong = capture()?;
            wrong.house = HouseId::new(good.runtime_scope.world_id(), "oteryn:content.house.other");
            rejected(h, good, wrong).await?;
            for guard in &mut guards {
                let valid = guard.state.clone();
                let mut invalid = valid.clone();
                match &mut invalid {
                    AdmissionAuthorityGuardStateV1::Character { holder, .. } => *holder = None,
                    AdmissionAuthorityGuardStateV1::Account { presence, .. } => *presence = None,
                    AdmissionAuthorityGuardStateV1::Runtime { ready, .. } => *ready = false,
                    _ => return Err("unexpected guard".into()),
                }
                replace_guard(h, guard, invalid).await?;
                rejected(h, good, capture()?).await?;
                replace_guard(h, guard, valid).await?;
            }

            // Current mirror corruption with the canonical history unchanged is refused, not trusted as authority.
            h.pool
                .execute("UPDATE game_durability_admission_character_guards SET lease_generation=3")
                .await?;
            {
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                assert!(
                    h.root
                        .write_character_position_house(&authority, &h.node, good, 100, capture()?)
                        .await
                        .is_err()
                );
            }
            h.pool
                .execute("UPDATE game_durability_admission_character_guards SET lease_generation=2")
                .await?;
            // Canonical self-consistent projection for a never-published decision: only immutable
            // current publication HISTORY is missing. Use the actual native encoder, no new codec.
            let published = guards[0].clone();
            let mut unbacked = published.clone();
            unbacked.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: published.publication_revision,
            };
            unbacked.publication_revision += 1;
            unbacked.source.source_revision += 1;
            unbacked.source.decision_identity = "never-published-fixture".into();
            for c in [&unbacked, &published] {
                let encoded = crate::durability::admission_authority_guards::encode_guard(
                    c,
                    crate::durability::MAX_ADMISSION_GUARD_BYTES,
                )?;
                sqlx::query("UPDATE game_durability_admission_character_guards SET publication_revision=$1::text::numeric(20,0),source_revision=$2::text::numeric(20,0),decision_identity=$3,change_json=$4").bind(c.publication_revision.to_string()).bind(c.source.source_revision.to_string()).bind(&c.source.decision_identity).bind(encoded).execute(&h.pool).await?;
                if c.publication_revision == unbacked.publication_revision {
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h
                        .root
                        .open_character_authority(&seal)
                        .await
                        .map_err(debug)?;
                    assert!(
                        h.root
                            .write_character_position_house(
                                &authority,
                                &h.node,
                                good,
                                100,
                                capture()?
                            )
                            .await
                            .is_err()
                    );
                    assert_eq!(h.count("game_character_last_positions").await?, 0);
                }
            }
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let wrong_secret =
                NodeIncarnationProof::new(h.node.fact(), BootstrapSecret::from_bytes([99; 32]));
            assert!(
                h.root
                    .write_character_position_house(
                        &authority,
                        &wrong_secret,
                        good,
                        100,
                        capture()?
                    )
                    .await
                    .is_err()
            );
            assert_eq!(
                write(h, good, 100, capture()?).await?,
                LastPositionWrite::Written
            );
            let before: String =
                sqlx::query_scalar("SELECT to_jsonb(p)::text FROM game_character_last_positions p")
                    .fetch_one(&h.pool)
                    .await?;
            let row=sqlx::query("SELECT ownership_generation::text,source_revision::text FROM game_runtime_scope_assignments WHERE scope_key=$1").bind(house()?.scope_key()).fetch_one(&h.pool).await?;
            let predecessor = HouseScopePredecessor {
                ownership_generation: row.try_get::<String, _>("ownership_generation")?.parse()?,
                source_revision: row.try_get::<String, _>("source_revision")?.parse()?,
            };
            let revoked = h
                .root
                .assign_house_scope(HouseScopeAssignmentRequest {
                    operation_key: OperationKey::from_bytes([9; 32]),
                    actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                    actor_role: "oteryn_test_admin".into(),
                    command: HouseScopeAssignmentCommand::Revoke {
                        house: house()?,
                        predecessor,
                    },
                })
                .await
                .map_err(debug)?;
            assert!(matches!(revoked, HouseScopeAssignmentOutcome::Committed(_)));
            // Session/guard/capture fences remain exactly valid at generation1; ONLY the actual assignment changed.
            assert!(matches!(
                h.root
                    .write_character_position_house(&authority, &h.node, good, 101, capture()?)
                    .await,
                Err(CharacterProgressionError::AuthorityRejected)
            ));
            let after: String =
                sqlx::query_scalar("SELECT to_jsonb(p)::text FROM game_character_last_positions p")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(before, after);

            TestResult::Ok(())
        })
    })
}

#[test]
fn house_projection_schema_rollback_and_same_cell_channel_clear() -> TestResult {
    run_house_position_case("house_pos_clear", true, |h| {
        Box::pin(async move {
            let mut guards = setup(h).await?;
            assert_eq!(
                write(h, house_fence()?, 10, capture()?).await?,
                LastPositionWrite::Written
            );
            let before = read(h).await?.ok_or("missing")?;
            for sql in [
                "UPDATE game_character_last_positions SET house_key=NULL,write_sequence=11",
                "UPDATE game_character_last_positions SET house_world_id=NULL,write_sequence=11",
                "UPDATE game_character_last_positions SET house_key='invalid',write_sequence=11",
            ] {
                let e = h
                    .pool
                    .execute(sql)
                    .await
                    .expect_err("invalid pair/key accepted");
                assert_eq!(
                    e.as_database_error().and_then(|e| e.code()).as_deref(),
                    Some("23514")
                );
                assert_eq!(read(h).await?.ok_or("missing")?, before);
            }
            h.pool.execute("CREATE FUNCTION test_position_rollback() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture deferred failure' USING ERRCODE='23514'; END $$; CREATE CONSTRAINT TRIGGER test_position_rollback AFTER INSERT OR UPDATE ON game_character_last_positions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION test_position_rollback()").await?;
            assert!(write(h, house_fence()?, 11, capture()?).await.is_err());
            assert_eq!(read(h).await?.ok_or("missing")?, before);
            h.pool.execute("DROP TRIGGER test_position_rollback ON game_character_last_positions; DROP FUNCTION test_position_rollback()").await?;
            // Fixture installs a CURRENT Channel destination; no reverse-handoff/entrance or
            // controller claim. The storage method must clear at the same coordinates/vitals.
            h.pool.execute("UPDATE game_durability_reconnect_sessions SET runtime_scope_kind=1,runtime_scope_channel_id=origin_channel_id,runtime_scope_instance_id=NULL,runtime_scope_house_key=NULL,origin_channel_id=NULL WHERE game_session_id IN(SELECT game_session_id FROM game_durability_reconnect_sessions WHERE session_state=1)").await?;
            let mut outside = capture()?;
            outside.house = None;
            // The destination fixture has the existing Channel scope generation and native guard.
            let f = CurrentCharacterGameplayFence {
                runtime_scope: scope()?,
                ..house_fence()?
            };
            assert_eq!(
                write(h, f, 11, outside.clone()).await?,
                LastPositionWrite::Written
            );
            let row=sqlx::query("SELECT house_world_id,house_key,tile_x,health,mana FROM game_character_last_positions").fetch_one(&h.pool).await?;
            assert!(
                row.try_get::<Option<sqlx::types::Uuid>, _>("house_world_id")?
                    .is_none()
            );
            assert!(row.try_get::<Option<String>, _>("house_key")?.is_none());
            assert_eq!(row.try_get::<i32, _>("tile_x")?, outside.position.cell.x);
            assert_eq!(row.try_get::<i32, _>("health")?, 73);
            assert_eq!(row.try_get::<i32, _>("mana")?, 19);
            // Destination greater lease prevents a late old House capture from resurrecting it.
            h.pool.execute("UPDATE game_character_last_positions SET character_lease_generation=3,write_sequence=1").await?;
            assert_eq!(
                write(h, f, 99, outside).await?,
                LastPositionWrite::Superseded
            );
            // Keep state restoration separate from the storage claim; guard mutation is not exit authority.
            let valid = guards[2].state.clone();
            replace_guard(h, &mut guards[2], valid).await?;
            TestResult::Ok(())
        })
    })
}

// Native CHAR-POSITION fixture policy, not an appearance/position authority implementation.
use crate::bestiary_postgres_harness::context;
use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
fn death(tag: u8) -> TestResult<CharacterDeathRequest<10>> {
    const GLOBAL: [i64; 11] = [0, 100, 200, 400, 800, 1500, 2600, 4200, 6400, 9300, 13000];
    let mut thresholds = [LevelThreshold {
        level: 1,
        minimum_experience: ExactI64::new(0),
    }; 10];
    for (index, threshold) in thresholds.iter_mut().enumerate() {
        *threshold = LevelThreshold {
            level: u32::try_from(index + 1)?,
            minimum_experience: ExactI64::new(GLOBAL[index]),
        };
    }
    Ok(CharacterDeathRequest {
        occurrence: PlayerDeathOccurrence::from_bytes(id(tag)).map_err(debug)?,
        context: context(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: FiniteProgressionPolicy {
            context: context(),
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds,
            terminal_exclusive_experience: ExactI64::new(GLOBAL[10]),
            death_loss_numerator: 1,
            death_loss_denominator: 1,
            death_loss_rounding: RoundingMode::Floor,
        },
        held_blessings: Vec::new(),
        death_cell: DeathCell {
            world_id: WorldId::decode(&id(WORLD)).map_err(debug)?,
            channel_id: crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            spatial_position: vec![1, 2, 3, 7],
            map_revision: "map-1".into(),
        },
        respawn_position: vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 7],
    })
}

#[derive(Clone, Copy)]
enum NativeRespawnWriter {
    Admission,
    InSession,
}

/// Exercise the actual native durable callers. The in-session callback is only a
/// placement acknowledgement fixture and does not qualify a controller or physical placement.
async fn native_respawn_write(
    h: &Harness,
    writer: NativeRespawnWriter,
    current: CurrentCharacterGameplayFence,
    occurrence: PlayerDeathOccurrence,
    placed: FinalLastPosition,
    acknowledge_placement: bool,
) -> TestResult<bool> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    match writer {
        NativeRespawnWriter::Admission => h
            .root
            .consume_pending_respawn_placed(&authority, &h.node, current, Some(occurrence), placed)
            .await
            .map(|consumed| consumed.is_some())
            .map_err(|error| debug(error).into()),
        NativeRespawnWriter::InSession => h
            .root
            .place_and_consume_pending_respawn_placed(
                &authority,
                &h.node,
                current,
                occurrence,
                placed,
                move |pending| async move { acknowledge_placement && pending.is_some() },
            )
            .await
            .map_err(|error| debug(error).into()),
    }
}

async fn native_respawn_clear_case(h: &Harness, writer: NativeRespawnWriter) -> TestResult {
    sqlx::query(
        "INSERT INTO game_character_progression_state VALUES \
         (encode($1,'hex')::uuid,1,9,6500,'profile-1','ruleset-1','content-1', \
          'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
    )
    .bind(id(CHARACTER).as_slice())
    .execute(&h.pool)
    .await?;
    let original = capture()?.position;
    assert_eq!(
        native_channel_write(h, NativeChannelWriter::Single, fence(1)?, 10, original).await?,
        LastPositionWrite::Written
    );
    let next_sequence = mark_historical_house_pair(h)
        .await?
        .checked_add(1)
        .ok_or("fixture sequence exhausted")?;
    let before = position_row(h).await?;
    let death = death(81)?;
    let occurrence = death.occurrence;
    {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        h.root
            .settle_player_death(
                &CharacterRevisionSequencer::new(),
                &authority,
                &h.node,
                fence(1)?,
                death,
            )
            .await
            .map_err(debug)?;
    }
    assert_eq!(h.root_revision().await?, "2");
    assert_eq!(h.count("game_character_pending_respawns").await?, 1);
    assert_eq!(position_row(h).await?, before);
    let mut respawn = original;
    respawn.cell = LastPositionCell {
        x: 100,
        y: 200,
        floor: 6,
    };
    respawn.vitals = LastVitals::new(100, 50);
    let placed = FinalLastPosition {
        record: respawn,
        write_sequence: next_sequence,
    };

    // A pending death obligation vetoes both real periodic routes without clearing
    // the old House binding, changing its cell, or consuming the obligation.
    for periodic in [NativeChannelWriter::Single, NativeChannelWriter::Batch] {
        assert_eq!(
            native_channel_write(h, periodic, fence(2)?, next_sequence, respawn).await?,
            LastPositionWrite::PendingObligation
        );
        assert_eq!(position_row(h).await?, before);
        assert_eq!(h.count("game_character_pending_respawns").await?, 1);
    }

    let absent_occurrence = PlayerDeathOccurrence::from_bytes(id(82)).map_err(debug)?;
    assert!(!native_respawn_write(h, writer, fence(2)?, absent_occurrence, placed, true).await?);
    assert_eq!(position_row(h).await?, before);
    assert_eq!(h.count("game_character_pending_respawns").await?, 1);
    assert!(
        native_respawn_write(h, writer, fence(1)?, occurrence, placed, true)
            .await
            .is_err()
    );
    assert_eq!(position_row(h).await?, before);
    assert_eq!(h.count("game_character_pending_respawns").await?, 1);
    let wrong_world = FinalLastPosition {
        record: LastPositionRecord {
            world_id: WorldId::decode(&id(90)).map_err(debug)?,
            ..respawn
        },
        ..placed
    };
    assert!(
        native_respawn_write(h, writer, fence(2)?, occurrence, wrong_world, true)
            .await
            .is_err()
    );
    assert_eq!(position_row(h).await?, before);
    assert_eq!(h.count("game_character_pending_respawns").await?, 1);
    if matches!(writer, NativeRespawnWriter::InSession) {
        assert!(!native_respawn_write(h, writer, fence(2)?, occurrence, placed, false).await?);
        assert_eq!(position_row(h).await?, before);
        assert_eq!(h.count("game_character_pending_respawns").await?, 1);
    }

    // The actual caller has deleted pending_respawn and executed the winning
    // position upsert before this deferred failure. COMMIT must roll both back.
    install_position_commit_failure(h).await?;
    assert!(
        native_respawn_write(h, writer, fence(2)?, occurrence, placed, true)
            .await
            .is_err()
    );
    assert_eq!(position_row(h).await?, before);
    assert_eq!(h.count("game_character_pending_respawns").await?, 1);
    remove_position_commit_failure(h).await?;

    assert!(native_respawn_write(h, writer, fence(2)?, occurrence, placed, true).await?);
    assert_eq!(h.count("game_character_pending_respawns").await?, 0);
    assert_channel_position(h, respawn, next_sequence).await?;
    let committed = position_row(h).await?;
    let replay = FinalLastPosition {
        record: original,
        write_sequence: next_sequence
            .checked_add(1)
            .ok_or("fixture sequence exhausted")?,
    };
    assert!(!native_respawn_write(h, writer, fence(2)?, occurrence, replay, true).await?);
    assert_eq!(position_row(h).await?, committed);
    assert_eq!(h.root_revision().await?, "2");
    Ok(())
}

#[test]
fn house_projection_native_admission_respawn_clears_with_consumption() -> TestResult {
    run_house_position_case("house_pos_admission_respawn", false, |h| {
        Box::pin(async move { native_respawn_clear_case(h, NativeRespawnWriter::Admission).await })
    })
}

#[test]
fn house_projection_native_in_session_respawn_clears_with_consumption() -> TestResult {
    run_house_position_case("house_pos_session_respawn", false, |h| {
        Box::pin(async move { native_respawn_clear_case(h, NativeRespawnWriter::InSession).await })
    })
}

#[test]
fn house_projection_old_null_row_pending_respawn_and_retired_node() -> TestResult {
    run_house_position_case("house_pos_pending", false, |h| {
        Box::pin(async move {
            sqlx::query("INSERT INTO game_character_progression_state VALUES(encode($1,'hex')::uuid,1,9,6500,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')").bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let old = capture()?.position;
            assert_eq!(
                h.root
                    .write_character_last_position(&authority, &h.node, fence(1)?, 5, old)
                    .await
                    .map_err(debug)?,
                LastPositionWrite::Written
            );
            h.root
                .settle_player_death(
                    &CharacterRevisionSequencer::new(),
                    &authority,
                    &h.node,
                    fence(1)?,
                    death(81)?,
                )
                .await
                .map_err(debug)?;
            assert_eq!(h.count("game_character_pending_respawns").await?, 1);
            setup(h).await?;
            let before = read(h).await?.ok_or("old row missing")?;
            assert!(before.house.is_none());
            assert_eq!(before.position.record, old);
            assert_eq!(before.position.key.write_sequence, 5);
            assert_eq!(
                write(h, house_fence()?, 100, capture()?).await?,
                LastPositionWrite::PendingObligation
            );
            assert_eq!(read(h).await?.ok_or("missing")?, before);
            h.root
                .revoke_node_registration(h.node.fact())
                .await
                .map_err(debug)?;
            // Actual node registration was revoked; the unaltered expected fence and valid pending row cannot revive it.
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            assert!(matches!(
                h.root
                    .write_character_position_house(
                        &authority,
                        &h.node,
                        house_fence()?,
                        101,
                        capture()?
                    )
                    .await,
                Err(CharacterProgressionError::AuthorityRejected)
            ));
            let key: String = sqlx::query_scalar(
                "SELECT write_sequence::text FROM game_character_last_positions",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(key, "5");
            TestResult::Ok(())
        })
    })
}

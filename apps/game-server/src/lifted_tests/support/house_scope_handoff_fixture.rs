//! Reused accepted §10 House admission fixture; never production authority.
#![cfg(test)]
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, WORLD, debug, id, scope,
};
use crate::durability::DurabilityRoot;
use crate::durability::house_scope_handoff::{
    HouseCommitOutcome, HouseEntryCommit, HouseEntryRequest, HouseHandoffDirection,
    HouseHandoffState, HouseId, HousePrepareOutcome, HouseScopeAssignmentCommand,
    HouseScopeAssignmentOutcome, HouseScopeAssignmentRequest,
};
use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
use crate::foundation::{CharacterId, GameSessionId, WorldId};
pub(crate) const HOUSE_KEY: &str = "oteryn:content.house.thais_1";
pub(crate) const HANDOFF: u8 = 60;
pub(crate) const DESTINATION: u8 = 61;
pub(crate) fn house() -> TestResult<HouseId> {
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    Ok(HouseId::new(world, HOUSE_KEY).ok_or("house key")?)
}

pub(crate) fn request(handoff: u8) -> TestResult<HouseEntryRequest> {
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

pub(crate) fn commit(handoff: u8) -> TestResult<HouseEntryCommit> {
    Ok(HouseEntryCommit {
        handoff_id: id(handoff),
        destination_game_session_id: GameSessionId::decode(&id(DESTINATION)).map_err(debug)?,
        committed_at: 200,
    })
}

pub(crate) fn house_request(
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

pub(crate) async fn seed(harness: &Harness) -> TestResult {
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

pub(crate) async fn install_test_house(harness: &Harness) -> TestResult {
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

pub(crate) async fn prepare(harness: &Harness, root: &DurabilityRoot, handoff: u8) -> TestResult {
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

pub(crate) async fn admit(harness: &Harness) -> TestResult {
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

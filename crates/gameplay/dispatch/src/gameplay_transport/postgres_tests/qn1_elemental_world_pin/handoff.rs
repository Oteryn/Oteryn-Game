//! Real assignment/readiness rollover; immutable creation is never current ownership.
use super::super::fixture::{Harness, debug};
use super::super::{ComposedFreshAdmission, TestResult};
use crate::durability::native_map_items_abi::{
    NativeMapInitializationProof, NativeMapItemPlacement, NativeMapOwnerBinding,
    map_initializer_seal,
};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
    RuntimeScopeAssignmentWriter,
};

// Expected map binding only: the existing initializer independently checks real
// recovery, assignment, Node and readiness. This is not transport readmission.
struct EmptyMap(NativeMapOwnerBinding);
impl map_initializer_seal::Sealed for EmptyMap {}
impl NativeMapInitializationProof for EmptyMap {
    fn current_binding(&self) -> Result<NativeMapOwnerBinding, &'static str> {
        Ok(self.0.clone())
    }
    fn placements(&self) -> &[NativeMapItemPlacement] {
        &[]
    }
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    item: [u8; 16],
) -> TestResult {
    let (scope, pin) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.binding().runtime_scope(),
            runtime.content_pin().clone(),
        )
    };
    let before: (String, String, String) = sqlx::query_as("SELECT i.minted_transaction_id::text,i.state_revision::text,g.runtime_scope_ownership_generation::text FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    let node = crate::bestiary_postgres_harness::register(owner.root, 177).await?;
    // The harness owns one assignment-writer registration for this Root.
    // Grant only the separate replacement operation used by this case.
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice())
        .execute(&h.pool).await?;
    let writer = RuntimeScopeAssignmentWriter::open(owner.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let predecessor = owner
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(debug)?
        .ok_or("current energy scope assignment")?;
    let outcome = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([179; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Replace {
                scope,
                predecessor,
                target: node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(replaced) = outcome else {
        return Err("energy scope replacement refused".into());
    };
    assert_eq!(replaced.assignment.ownership_generation, 2);
    let proof = EmptyMap(NativeMapOwnerBinding {
        world: owner.world_id,
        channel: owner.channel_id,
        scope_generation: replaced.assignment.ownership_generation,
        content_digest: pin.server_artifact_digest(),
        map_digest: pin.map_revision_digest(),
        frame_digest: pin.frame_binding_digest(),
    });
    let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(scope)
        .await
        .map_err(debug)?;
    assert!(
        owner
            .root
            .initialize_native_map_current_owner(&permit, owner.character, &node, &proof)
            .await
            .is_err(),
        "assignment alone cannot replace real readiness"
    );
    publish_readiness(owner, h, &node, scope).await?;
    eprintln!("QN1_ENERGY_FIELD_HANDOFF_PG_READY scope_generation=2");
    assert!(
        owner
            .root
            .initialize_native_map_current_owner(&permit, owner.character, owner.holder, &proof)
            .await
            .is_err(),
        "the original Node cannot adopt for its successor"
    );
    owner
        .root
        .initialize_native_map_current_owner(&permit, owner.character, &node, &proof)
        .await
        .map_err(debug)?;
    owner
        .root
        .initialize_native_map_current_owner(&permit, owner.character, &node, &proof)
        .await
        .map_err(debug)?;
    let after: (String, String, String) = sqlx::query_as("SELECT i.minted_transaction_id::text,i.state_revision::text,g.runtime_scope_ownership_generation::text FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((&after.0, &after.1), (&before.0, &before.1));
    assert_eq!(
        after.2, "2",
        "the actual current map owner must adopt the permanent field"
    );
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i2126' AND lifecycle=1")
        .fetch_one(&h.pool).await?;
    assert_eq!(count, 1, "handoff/retry must never remint a field");
    let rows = owner
        .root
        .read_item_loop_ground(owner.world_id, owner.channel_id)
        .await
        .map_err(debug)?;
    assert!(rows.iter().any(|entry| entry.item.item_instance_id == item
        && entry.field_collision == Some((false, false))));
    eprintln!("QN1_ENERGY_FIELD_HANDOFF_PG_EXECUTED");
    Ok(())
}

async fn publish_readiness(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
    scope: crate::foundation::RuntimeScopeRefV1,
) -> TestResult {
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
        AdmissionAuthorityPublicationChangeV1, AdmissionPublicationPreconditionV1,
        AdmissionPublicationPurposeV1, AdmissionPublicationSourceV1,
    };
    let key = AdmissionAuthorityGuardKeyV1::Runtime(scope);
    let previous = crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(
        owner.root.clone(),
    )
    .load(std::slice::from_ref(&key))
    .await
    .map_err(debug)?
    .pop()
    .flatten()
    .ok_or("runtime readiness predecessor")?;
    assert_eq!(previous.source.authority, "independent-game-fixture");
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let revision = previous
        .publication_revision
        .checked_add(1)
        .ok_or("readiness revision")?;
    // Only CAS counters come from history. Assignment, Node proof and this
    // fixture publisher's new readiness observation are independent.
    let change = AdmissionAuthorityPublicationChangeV1 {
        key,
        source: AdmissionPublicationSourceV1 {
            authority: "independent-game-fixture".into(),
            purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
            source_revision: previous
                .source
                .source_revision
                .checked_add(1)
                .ok_or("readiness source revision")?,
            decision_identity: format!("qn1-energy-runtime-ready-{revision}"),
            source_observed_at: now,
            clock_uncertainty_seconds: 0,
        },
        precondition: AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: previous.publication_revision,
        },
        publication_revision: revision,
        state: AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation: 2,
            ready: true,
            route_revision: "route-1".into(),
            runtime_observation_revision: "runtime-2".into(),
            protocol_major: 1,
            transport_profile: 1,
            ruleset_revision: "ruleset-1".into(),
            content_revision: "content-1".into(),
            map_revision: "map-1".into(),
            world_policy_revision: "world-policy-1".into(),
            offer_revision: "offer-1".into(),
        },
    };
    let publication =
        crate::bestiary_postgres_harness::ground_native_readiness_enforcement::prepare(
            change, now,
        )?;
    let result = owner
        .root
        .publish_runtime_readiness(node, &publication)
        .await
        .map_err(debug)?;
    assert_eq!(
        result,
        crate::durability::admission_authority_guards::GuardPublicationDisposition::Applied
    );
    Ok(())
}

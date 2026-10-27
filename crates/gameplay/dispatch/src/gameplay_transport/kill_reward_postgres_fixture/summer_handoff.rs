//! A new scope owner, using the same assignment/readiness writers as admission.
use super::*;
use crate::foundation::admission_authority_publication::*;

struct Ready(AdmissionAuthorityPublicationChangeV1);
impl crate::foundation::fnd04_verifier::fresh_source_sealed::Sealed for Ready {}
impl AdmissionAuthorityOwningPublisherV1 for Ready {
    fn resolve_publication(
        &self,
        _: i64,
    ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
    {
        Ok(vec![self.0.clone()])
    }
}

pub(super) async fn move_scope(
    root: &crate::durability::DurabilityRoot,
    pool: &sqlx::PgPool,
    world: WorldId,
    channel: ChannelId,
) -> TestResult<crate::durability::runtime_scope_assignment::NodeIncarnationProof> {
    use crate::durability::runtime_scope_assignment::*;
    let scope = crate::foundation::RuntimeScopeRefV1::channel(world, channel);
    let node = super::super::super::fixture::registered_node(root, 2).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(pool).await?;
    let predecessor = root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(debug)?
        .ok_or("live predecessor")?;
    let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "summer-restart-writer")
        .await
        .map_err(debug)?;
    let moved = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([163; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Replace {
                scope,
                predecessor,
                target: node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(moved) = moved else {
        return Err("scope replacement".into());
    };
    assert_eq!(moved.assignment.ownership_generation, 2);
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(pool)
            .await?;
    let row = sqlx::query("SELECT g.publication_revision::text,g.source_authority,g.source_revision::text,g.source_observed_at FROM game_durability_admission_runtime_guards g JOIN game_runtime_scope_assignments a USING(scope_key) WHERE a.world_id=encode($1,'hex')::uuid AND a.channel_id=encode($2,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).fetch_one(pool).await?;
    let previous: u64 = row.try_get::<String, _>("publication_revision")?.parse()?;
    let source_revision: u64 = row.try_get::<String, _>("source_revision")?.parse()?;
    let change = AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Runtime(scope),
        source: AdmissionPublicationSourceV1 {
            authority: row.try_get("source_authority")?,
            purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
            source_revision: source_revision + 1,
            decision_identity: "summer-runtime-ready-2".into(),
            source_observed_at: now.max(row.try_get("source_observed_at")?),
            clock_uncertainty_seconds: 0,
        },
        precondition: AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: previous,
        },
        publication_revision: previous + 1,
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
        AdmissionAuthorityPublicationV1::prepare(&Ready(change), now).map_err(debug)?;
    assert_eq!(
        root.publish_runtime_readiness(&node, &publication)
            .await
            .map_err(debug)?,
        crate::durability::admission_authority_guards::GuardPublicationDisposition::Applied
    );
    Ok(node)
}

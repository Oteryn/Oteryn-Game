//! Shared existing reset fixture operations, extracted once without a second harness.
use super::*;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{ChannelId, WorldId};

pub(crate) use oteryn_gameplay_durability::durability::world_reset::test_support::seed_retiring;

pub(crate) async fn replace(
    harness: &Harness,
    world: WorldId,
    channel: ChannelId,
    node: &NodeIncarnationProof,
) -> TestResult {
    // A real ordinary assignment replacement supplies the fresh generation
    // and its typed closed readiness; the reset never forges either fact.
    use crate::durability::runtime_scope_assignment::{
        AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
        RuntimeScopeAssignmentWriter,
    };
    let scope = crate::foundation::RuntimeScopeRefV1::channel(world, channel);
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2)")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&harness.pool).await?;
    let writer = RuntimeScopeAssignmentWriter::open(harness.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let predecessor = harness
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(debug)?
        .ok_or("predecessor")?;
    assert!(matches!(
        writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([9; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope,
                    predecessor,
                    target: node.fact()
                },
            })
            .await
            .map_err(debug)?,
        AssignmentOutcome::Committed(_)
    ));
    Ok(())
}

pub(crate) fn set_runtime(
    deps: &mut Dependencies,
    world: WorldId,
    channel: ChannelId,
    pin: &crate::content::NativeEntryContentPin,
    generation: u64,
) -> TestResult {
    let room = pin.qualified_room();
    let start = room.entry_start();
    let content = ChannelContentPin::from_activation(
        world,
        generation,
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    deps.runtime = Mutex::new(
        ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            generation,
            generation,
            &format!("runtime-scope-assignment:{generation}"),
            2,
            content,
        )
        .map_err(debug)?,
    );
    Ok(())
}

pub(crate) async fn publish_ready(harness: &Harness, generation: u64) -> TestResult {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::*;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let mut source = fresh_fixture::fresh::Source::new(now)?;
    source
        .rows
        .retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let row = source.rows.first_mut().ok_or("runtime publication")?;
    let previous = AdmissionGuardStore::from_root(harness.root.clone())
        .load(std::slice::from_ref(&row.key))
        .await?;
    if let Some(previous) = previous.first().and_then(Option::as_ref) {
        row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: previous.publication_revision,
        };
        row.publication_revision = previous.publication_revision + 1;
        row.source.source_revision = previous.source.source_revision + 1;
        row.source.decision_identity = format!("reset-readiness:{generation}");
    }
    if let AdmissionAuthorityGuardStateV1::Runtime {
        ownership_generation,
        ..
    } = &mut row.state
    {
        *ownership_generation = generation;
    }
    let publication = AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?;
    harness
        .root
        .publish_runtime_readiness(&harness.node, &publication)
        .await
        .map_err(debug)?;
    Ok(())
}

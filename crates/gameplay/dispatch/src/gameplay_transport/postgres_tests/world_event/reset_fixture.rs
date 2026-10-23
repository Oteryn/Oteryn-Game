//! Existing reset/admission APIs, with a fresh session after the reset drain.
use super::*;
use crate::content::QualifiedNativeEntryRoom;
use crate::durability::admission_authority_guards::AdmissionGuardStore;
use crate::foundation::admission_authority_publication::*;

pub(super) async fn reset_epochs(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    room: &QualifiedNativeEntryRoom,
    world: WorldId,
    channel: ChannelId,
    epochs: u64,
) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4),(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    assert!(
        h.root
            .record_content_activation(
                &crate::durability::content_activation::ContentActivationRequest {
                    world_id: world,
                    channel_id: channel,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest: room.compiled().server_digest(),
                    client_artifact_digest: room.compiled().client_digest(),
                    frame_binding_digest: room.frame_binding().digest(),
                }
            )
            .await?
    );
    assert!(sqlx::query_scalar::<_, bool>("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_10',1)")
        .bind(world.as_bytes().as_slice()).bind(room.compiled().server_digest().as_slice())
        .bind(room.movement_cells().scope().map_revision.as_str()).bind(room.compiled().client_digest().as_slice())
        .bind(room.frame_binding().digest().as_slice()).fetch_one(&h.pool).await?);
    planned_world_reset::reset_fixture::publish_ready(h, 1).await?;
    for previous in 0..epochs {
        assert!(sqlx::query_scalar::<_, bool>("SELECT game_world_reset_publish(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,$3,$5,$6,'[]'::jsonb,1)")
            .bind(world.as_bytes().as_slice()).bind(previous.to_string())
            .bind(room.compiled().server_digest().as_slice()).bind(room.movement_cells().scope().map_revision.as_str())
            .bind(room.compiled().client_digest().as_slice()).bind(room.frame_binding().digest().as_slice())
            .fetch_one(&h.pool).await?);
        let reset = h
            .root
            .retiring_world_reset(world)
            .await?
            .ok_or("retiring reset")?;
        replace(h, world, channel, previous + 2).await?;
        if previous == 0 {
            // A failed event writer must roll epoch and content issuance back too.
            sqlx::query("CREATE FUNCTION reject_event() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected event failure' USING ERRCODE='23514'; END $$")
                .execute(&h.pool).await?;
            sqlx::query("CREATE TRIGGER reject_event BEFORE INSERT ON game_world_event_epochs FOR EACH ROW EXECUTE FUNCTION reject_event()")
                .execute(&h.pool).await?;
            assert!(
                h.root
                    .finish_world_reset(authority, &h.node, &reset, room, room)
                    .await
                    .is_err()
            );
            assert_eq!(h.root.active_world_reset_epoch(world).await?, Some(0));
            assert_eq!(h.root.active_world_event_epoch(world).await?, Some(0));
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_world_event_epochs")
                    .fetch_one(&h.pool)
                    .await?,
                0
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_content_activations")
                    .fetch_one(&h.pool)
                    .await?,
                1
            );
            sqlx::query("DROP TRIGGER reject_event ON game_world_event_epochs")
                .execute(&h.pool)
                .await?;
        }
        for _ in 0..2 {
            assert_eq!(
                h.root
                    .finish_world_reset(authority, &h.node, &reset, room, room)
                    .await
                    .map_err(debug)?,
                Some(previous + 1)
            );
        }
        let (active, announcement): (bool, String) = sqlx::query_as("SELECT active,announcement FROM game_world_event_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
            .bind(world.as_bytes().as_slice()).bind((previous+1).to_string()).fetch_one(&h.pool).await?;
        assert_eq!(active, previous % 2 == 0);
        assert_eq!(announcement.is_empty(), !active);
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_world_event_epochs")
            .fetch_one(&h.pool)
            .await?,
        i64::try_from(epochs)?
    );
    // Runtime can read/insert these owned facts, never rewrite their history.
    let privileges: (bool, bool) = sqlx::query_as("SELECT has_table_privilege('oteryn_game_runtime','game_world_event_epochs','UPDATE'),has_table_privilege('oteryn_game_runtime','game_world_event_query_receipts','DELETE')")
        .fetch_one(&h.pool).await?;
    assert_eq!(privileges, (false, false));
    Ok(())
}

async fn replace(h: &Harness, world: WorldId, channel: ChannelId, generation: u64) -> TestResult {
    use crate::durability::runtime_scope_assignment::*;
    let scope = crate::foundation::RuntimeScopeRefV1::channel(world, channel);
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(debug)?
        .ok_or("predecessor")?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    assert!(matches!(
        writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([u8::try_from(generation + 100)?; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope,
                    predecessor,
                    target: h.node.fact()
                },
            })
            .await
            .map_err(debug)?,
        AssignmentOutcome::Committed(_)
    ));
    Ok(())
}

pub(super) async fn admit(h: &Harness, generation: u64) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut source = fresh_fixture::fresh::Source::new(now)?;
    source.current.scope_ownership_generation = generation;
    let key = AdmissionAuthorityGuardKeyV1::Runtime(crate::foundation::RuntimeScopeRefV1::channel(
        source.current.world_id,
        source.current.channel_id,
    ));
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    let previous = guards
        .load(std::slice::from_ref(&key))
        .await?
        .into_iter()
        .next()
        .flatten()
        .ok_or("closed readiness")?;
    let row = source
        .rows
        .iter_mut()
        .find(|row| row.key == key)
        .ok_or("runtime publication")?;
    row.publication_revision = previous.publication_revision + 1;
    row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
        expected_publication_revision: previous.publication_revision,
    };
    row.source.source_revision = previous.source.source_revision + 1;
    row.source.decision_identity = format!("world-event-readiness:{generation}");
    if let AdmissionAuthorityGuardStateV1::Runtime {
        ownership_generation,
        ..
    } = &mut row.state
    {
        *ownership_generation = generation;
    }
    let mut runtime_source = fresh_fixture::fresh::Source::new(now)?;
    runtime_source.rows = vec![row.clone()];
    let publication =
        AdmissionAuthorityPublicationV1::prepare(&runtime_source, now).map_err(debug)?;
    h.root
        .publish_runtime_readiness(&h.node, &publication)
        .await
        .map_err(debug)?;
    let mut admission_source = fresh_fixture::fresh::Source::new(now)?;
    admission_source.rows.retain(|row| row.key != key);
    let publication =
        AdmissionAuthorityPublicationV1::prepare(&admission_source, now).map_err(debug)?;
    guards.publish(&publication).await.map_err(debug)?;
    let request = source.request()?;
    assert!(matches!(
        FreshAdmissionStore::from_root(h.root.clone())
            .commit(&request)
            .await
            .map_err(debug)?,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    Ok(())
}

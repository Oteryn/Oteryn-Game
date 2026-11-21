//! An admitted Instance keeps its equipment interpretation after origin activation.
use super::TestResult;
use super::fixture::{Harness, debug, run};
use crate::durability::activity_handoff::{ActivityEnterOutcome, ActivityHandoffOwner};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_equipment::{
    EquipmentSnapshot, assert_equipment_authority_in_transaction,
    initialize_equipment_in_transaction, read_current_equipment_in_transaction,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{
    CommandId, CommandRef, ConnectionGeneration, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};

#[test]
fn instance_equipment_retains_content_after_origin_activation_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| -> Result<(), String> {
        run(async |admin| {
            let started = std::time::Instant::now();
            let h = Harness::create_with_admission_seed(admin, "instance_equipment_content", true, false).await?;
            let result = scenario(&h).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("INSTANCE_EQUIPMENT_ORIGIN_ACTIVATION_POSTGRES_CASE_EXECUTED duration={:?} passed=true", started.elapsed());
            Ok(())
        }).map_err(|error| error.to_string())
    })?.join().map_err(|_| "Instance equipment PostgreSQL thread panicked")?.map_err(Into::into)
}

async fn scenario(h: &Harness) -> TestResult {
    let (_evidence, request) = super::activity_runtime_fixtures::prepare_activity_fixture(h).await?;
    let source = request.participants[0].source;
    let digest = request.destination_content.server_artifact_digest;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    // Write through the actual equipment initializer under the current Channel
    // fence before handing that same Character to its real allocated Instance.
    let mut tx = h.pool.begin().await?;
    let equipment = assert_equipment_authority_in_transaction(
        &mut tx,
        &h.root,
        &authority,
        &h.node,
        &source,
        CommandRef::new(source.game_session_id, CommandId::new(2).map_err(debug)?),
        digest,
    )
    .await
    .map_err(debug)?;
    initialize_equipment_in_transaction(&mut tx, &equipment)
        .await
        .map_err(debug)?;
    tx.commit().await?;
    eprintln!("INSTANCE_EQUIPMENT_STAGE equipment_written_before_handoff");
    let receipt = match ActivityHandoffOwner::new(h.root.clone())
        .enter(&authority, &h.node, request.clone())
        .await?
    {
        ActivityEnterOutcome::Entered(receipt) => receipt,
        other => return Err(format!("equipment fixture handoff refused: {other:?}").into()),
    };
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(request.participants[0].destination_game_session_id)
        .await?
        .0;
    let fence = crate::gameplay_transport::item_fence_of(current)
        .ok_or("current equipment Instance fence")?;
    assert_eq!(fence.runtime_scope, receipt.destination_scope);
    assert!(matches!(
        fence.runtime_scope,
        RuntimeScopeRefV1::Instance { .. }
    ));
    let expected = read(h, &authority, &fence, digest).await?;
    eprintln!("INSTANCE_EQUIPMENT_STAGE retained_instance_read_accepted");
    assert_eq!(
        expected.revision, 1,
        "the equipment owner's committed initialization survives handoff"
    );
    assert_eq!(expected.content_digest, digest);
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = source.runtime_scope
    else {
        return Err("equipment origin must be the retained Channel".into());
    };
    let advanced = [0x55_u8; 32];
    assert_ne!(advanced, digest);
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,1,$3,$4,$5)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(advanced.as_slice()).bind([0x44_u8;32].as_slice())
        .bind(request.destination_content.coordinate_frame_digest.as_slice()).execute(&h.pool).await?;
    let latest: Vec<u8> = sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(latest, advanced);
    eprintln!("INSTANCE_EQUIPMENT_STAGE origin_activation_advanced");
    assert_eq!(
        read(h, &authority, &fence, digest).await?,
        expected,
        "origin activation cannot reinterpret retained Instance equipment"
    );
    assert!(
        read(h, &authority, &fence, advanced).await.is_err(),
        "the origin's new digest cannot replace the retained Instance pin"
    );
    let stale_connection = CurrentCharacterItemFence {
        connection_generation: ConnectionGeneration::new(fence.connection_generation.get() + 1)
            .map_err(debug)?,
        ..fence
    };
    assert!(
        read(h, &authority, &stale_connection, digest)
            .await
            .is_err(),
        "stale connection generation remains fenced"
    );
    let stale_scope = CurrentCharacterItemFence {
        scope_ownership_generation: ScopeOwnershipGeneration::new(
            fence.scope_ownership_generation.get() + 1,
        )
        .map_err(debug)?,
        ..fence
    };
    assert!(
        read(h, &authority, &stale_scope, digest).await.is_err(),
        "stale Instance generation remains fenced"
    );
    assert_eq!(read(h, &authority, &fence, digest).await?, expected);
    Ok(())
}

async fn read(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    fence: &CurrentCharacterItemFence,
    digest: [u8; 32],
) -> TestResult<EquipmentSnapshot> {
    let mut tx = h.pool.begin().await?;
    let result =
        read_current_equipment_in_transaction(&mut tx, &h.root, authority, &h.node, fence, digest)
            .await;
    tx.rollback().await?;
    result.map_err(|error| format!("{error:?}").into())
}

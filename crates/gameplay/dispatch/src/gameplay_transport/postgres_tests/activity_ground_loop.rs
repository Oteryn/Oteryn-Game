//! A real admitted Instance uses the existing generic drop/pickup writer.
//! This is deliberately not a death-drop cause: its current loop lines must
//! satisfy the existing 0128/0227 custody proofs with the exact private scope.
use super::TestResult;
use super::fixture::{Harness, debug, id};
use crate::durability::activity_handoff::{
    ActivityHandoffOwner, ActivityReturnOutcome, ActivityReturnReason, ActivityReturnRequest,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_decay_retire::ActivityGroundRetirement;
use crate::durability::item_loop_move::{
    ItemLoopContent, LoopDestination, LoopGround, LoopMoveReceipt, LoopMoveRefusal, LoopMoveRequest,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    ItemDefinitionFacts, ItemStackClass, ItemWeightLookup, ItemWeightRefused,
};
use crate::durability::spell_owner_commit::SpellLane;
use crate::foundation::{
    AuthenticatedTransportRefV1, CommandId, CommandRef, GameSessionId, RuntimeScopeRefV1,
};
use crate::gameplay_transport::item_fence_of;

#[test]
fn activity_ground_loop_postgres_real_drop_pickup_return_isolation_and_retirement() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        eprintln!("ACTIVITY_GROUND_LOOP POSTGRES_CASE_SKIPPED: no configured admin");
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let runtime =
                crate::bestiary_postgres_harness::runtime().map_err(|error| error.to_string())?;
            let h = runtime
                .block_on(Harness::create_with_admission_seed(
                    admin,
                    "activity_ground_loop",
                    true,
                    false,
                ))
                .map_err(|error| error.to_string())?;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.block_on(scenario(&h))
            }));
            let cleanup = runtime
                .block_on(h.cleanup())
                .map_err(|error| error.to_string());
            match result {
                Ok(result) => {
                    result.map_err(|error| error.to_string())?;
                    cleanup
                }
                Err(panic) => std::panic::resume_unwind(panic),
            }
        })?
        .join()
        .map_err(|_| "Instance generic Ground loop test panicked")??;
    Ok(())
}

async fn scenario(h: &Harness) -> TestResult {
    let entered = super::activity_runtime::enter_fixture(h).await?;
    let current = entered.fence;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = current.runtime_scope else {
        return Err("real Instance admission required".into());
    };
    let origin = entered.request.participants[0].source.runtime_scope;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let activity = ActivityHandoffOwner::new(h.root.clone());
    // Only initial fixture inventory is seeded. Every drop, pickup, return and
    // retirement below executes the runtime owner with all SQL triggers enabled.
    super::item_look::seed_main_backpack(h, &[("oteryn:item.tibia.i3031", 5)]).await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(current.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
    let content = Content(entered.request.destination_content.server_artifact_digest);
    let position = entered.request.participants[0].destination_cell;
    let cell = [
        position.x.to_be_bytes().as_slice(),
        position.y.to_be_bytes().as_slice(),
        position.floor.to_be_bytes().as_slice(),
    ]
    .concat();
    let ground = LoopGround {
        spatial_position: cell.clone(),
        map_revision: entered
            .request
            .destination_content
            .map_revision_digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        content_revision: entered.request.content_revision.clone(),
        placement_context: entered
            .request
            .destination_content
            .coordinate_frame_digest
            .to_vec(),
    };
    let drop_request = LoopMoveRequest {
        item: id(71),
        count: None,
        pickup_cell: None,
        destination: LoopDestination::Ground(ground),
    };
    let pickup_request = LoopMoveRequest {
        item: id(71),
        count: None,
        pickup_cell: Some(cell.clone()),
        destination: LoopDestination::MainBackpack,
    };
    let command = |sequence| -> TestResult<CommandRef> {
        Ok(CommandRef::new(
            current.game_session_id,
            CommandId::new(sequence).map_err(debug)?,
        ))
    };
    let lane = SpellLane::for_scope(current.runtime_scope);
    let permit = lane
        .acquire()
        .await
        .map_err(|_| "Instance spell lane unavailable")?;
    let dropped = h
        .root
        .commit_item_loop_move(
            &permit,
            &authority,
            &h.node,
            &current,
            command(11)?,
            &content,
            drop_request.clone(),
        )
        .await?
        .map_err(debug)?;
    assert_eq!(dropped.operation, 4);
    assert!(!dropped.replayed);
    assert_eq!(
        h.root
            .commit_item_loop_move(
                &permit,
                &authority,
                &h.node,
                &current,
                command(11)?,
                &content,
                drop_request.clone()
            )
            .await?
            .map_err(debug)?,
        LoopMoveReceipt {
            replayed: true,
            ..dropped
        }
    );
    let private = h
        .root
        .read_item_loop_ground_for_scope(current.runtime_scope)
        .await?;
    assert_eq!(private.len(), 1);
    assert_eq!(private[0].item.item_instance_id, id(71));
    assert_eq!(private[0].item.runtime_scope, current.runtime_scope);
    assert_eq!(private[0].item.quantity, 5);
    assert_eq!(private[0].item.ground.spatial_position, cell);
    assert!(
        h.root
            .read_item_loop_ground_for_scope(origin)
            .await?
            .is_empty()
    );
    let exact: (i16, Option<Vec<u8>>, Vec<u8>, String) = sqlx::query_as(
        "SELECT scope_kind,uuid_send(channel_id),uuid_send(instance_id),runtime_scope_ownership_generation::text FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        exact,
        (
            2,
            None,
            instance_id.to_vec(),
            current.scope_ownership_generation.get().to_string()
        )
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_player_death_item_records")
            .fetch_one(&h.pool)
            .await?,
        0,
        "the generic loop must not borrow a player-death placement cause"
    );

    let picked = h
        .root
        .commit_item_loop_move(
            &permit,
            &authority,
            &h.node,
            &current,
            command(12)?,
            &content,
            pickup_request.clone(),
        )
        .await?
        .map_err(debug)?;
    assert_eq!(picked.operation, 5);
    assert!(!picked.replayed);
    assert_eq!(
        h.root
            .commit_item_loop_move(
                &permit,
                &authority,
                &h.node,
                &current,
                command(12)?,
                &content,
                pickup_request.clone()
            )
            .await?
            .map_err(debug)?,
        LoopMoveReceipt {
            replayed: true,
            ..picked
        }
    );
    assert!(
        h.root
            .read_item_loop_ground_for_scope(current.runtime_scope)
            .await?
            .is_empty()
    );
    let carried: (Vec<u8>, Vec<u8>, i64) = sqlx::query_as(
        "SELECT uuid_send(e.character_id),uuid_send(e.parent_item_instance_id),i.quantity FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        carried,
        (current.character_id.as_bytes().to_vec(), id(70).to_vec(), 5)
    );
    let redropped = h
        .root
        .commit_item_loop_move(
            &permit,
            &authority,
            &h.node,
            &current,
            command(13)?,
            &content,
            drop_request,
        )
        .await?
        .map_err(debug)?;
    assert_eq!(redropped.operation, 4);
    type GroundLoopReceiptRow = (i16, Option<Vec<u8>>, Vec<u8>, i16, i16, i16);
    let receipts: Vec<GroundLoopReceiptRow> = sqlx::query_as(
        "SELECT r.scope_kind,uuid_send(r.channel_id),uuid_send(r.instance_id),r.operation,l.from_kind,l.to_kind FROM game_item_loop_move_receipts r JOIN game_item_loop_move_lines l USING(transaction_id) WHERE r.game_session_id=encode($1,'hex')::uuid ORDER BY r.command_id,l.line_ordinal")
        .bind(current.game_session_id.as_bytes().as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        receipts,
        vec![
            (2, None, instance_id.to_vec(), 4, 1, 3),
            (2, None, instance_id.to_vec(), 5, 3, 1),
            (2, None, instance_id.to_vec(), 4, 1, 3),
        ],
        "each real transition has one scoped receipt/line; replays add none"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
            .fetch_one(&h.pool)
            .await?,
        2,
        "drop/pickup moves the existing Item and never mints a substitute"
    );
    drop(permit);

    let placement = activity
        .read_return_placement(&authority, &h.node, current)
        .await?
        .ok_or("retained origin pin")?;
    let destination_session = GameSessionId::decode(&id(112)).map_err(debug)?;
    let returning = ActivityReturnRequest {
        return_id: id(111),
        handoff_id: entered.receipt.handoff_id,
        command_id: Some(14),
        source: current,
        destination_game_session_id: destination_session,
        destination_transport: Some(
            AuthenticatedTransportRefV1::decode(&[112; 16]).map_err(debug)?,
        ),
        destination_generation: placement.destination_generation,
        reason: ActivityReturnReason::Exit,
        current_vitals: entered.request.participants[0].source_position.vitals,
        current_rest: entered.request.participants[0].source_position.rest,
    };
    match activity
        .return_participant(&authority, &h.node, returning)
        .await?
    {
        ActivityReturnOutcome::Returned(receipt) => assert_eq!(receipt.destination_scope, origin),
        other => return Err(format!("real return refused: {other:?}").into()),
    }
    let back = item_fence_of(
        FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(destination_session)
            .await?
            .0,
    )
    .ok_or("fresh returned Channel fence")?;
    assert_eq!(back.runtime_scope, origin);
    assert_ne!(
        back.game_session_id,
        entered.request.participants[0].source.game_session_id
    );
    assert_eq!(
        back.character_lease_generation,
        current.character_lease_generation + 1
    );
    let channel_lane = SpellLane::for_scope(back.runtime_scope);
    let channel_permit = channel_lane
        .acquire()
        .await
        .map_err(|_| "Channel spell lane unavailable")?;
    let channel_content = Content(
        entered.request.participants[0]
            .source_position
            .server_artifact_digest,
    );
    let refused = h
        .root
        .commit_item_loop_move(
            &channel_permit,
            &authority,
            &h.node,
            &back,
            CommandRef::new(back.game_session_id, CommandId::new(15).map_err(debug)?),
            &channel_content,
            pickup_request,
        )
        .await?;
    assert_eq!(
        refused,
        Err(LoopMoveRefusal::Stale),
        "a fresh current Channel cannot take an exact private Instance Ground item"
    );
    assert!(
        h.root
            .read_item_loop_ground_for_scope(back.runtime_scope)
            .await?
            .is_empty()
    );
    assert_eq!(
        h.root
            .read_item_loop_ground_for_scope(current.runtime_scope)
            .await?
            .len(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_loop_move_receipts")
            .fetch_one(&h.pool)
            .await?,
        3,
        "refused foreign-scope pickup writes no receipt or custody change"
    );
    drop(channel_permit);

    let proof = activity
        .retirement_ready(
            &authority,
            &h.node,
            current.runtime_scope,
            current.scope_ownership_generation,
        )
        .await?
        .ok_or("all participants returned")?;
    let permit = lane
        .acquire()
        .await
        .map_err(|_| "Instance retirement lane unavailable")?;
    assert!(matches!(
        h.root
            .retire_activity_ground_step(&permit, &authority, &h.node, &proof)
            .await
            .map_err(debug)?,
        ActivityGroundRetirement::Progress(_)
    ));
    assert!(matches!(
        h.root
            .retire_activity_ground_step(&permit, &authority, &h.node, &proof)
            .await
            .map_err(debug)?,
        ActivityGroundRetirement::Complete
    ));
    assert!(
        h.root
            .read_item_loop_ground_for_scope(current.runtime_scope)
            .await?
            .is_empty()
    );
    assert!(
        h.root
            .read_item_loop_ground_for_scope(origin)
            .await?
            .is_empty()
    );
    let live: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(live, 0);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_container_slots WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(70).as_slice()).fetch_one(&h.pool).await?, 1,
        "retirement leaves the returned Character's backpack in its existing custody");
    eprintln!("ACTIVITY_GROUND_LOOP POSTGRES_CASE_COMPLETED");
    Ok(())
}

/// Explicitly authored fixture Item definitions, pinned to the admitted content
/// digest. They are the existing test backpack and a five-coin stack only.
struct Content([u8; 32]);

impl ItemWeightLookup for Content {
    fn generation_digest(&self) -> [u8; 32] {
        self.0
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        self.loop_facts(definition)
            .and_then(|facts| facts.weight)
            .ok_or(ItemWeightRefused::Unknown)
    }
}

impl ItemLoopContent for Content {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.0
    }
    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        if definition.family != "Item" || definition.revision_ref != "definition-r1" {
            return None;
        }
        let (stack, container_capacity, container_slot_equip_pattern) =
            match definition.production_key.as_str() {
                "oteryn:item.tibia.i2854" => (ItemStackClass::NonStackable, Some(20), true),
                "oteryn:item.tibia.i3031" => (
                    ItemStackClass::Stackable {
                        proven_maximum: Some(100),
                    },
                    None,
                    false,
                ),
                _ => return None,
            };
        Some(ItemDefinitionFacts {
            definition: definition.clone(),
            weight: Some(1),
            stack,
            container_capacity,
            container_slot_equip_pattern,
        })
    }
}

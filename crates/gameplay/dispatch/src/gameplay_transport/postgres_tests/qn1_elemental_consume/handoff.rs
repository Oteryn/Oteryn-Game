//! Physical corpse classification after a real assignment-generation rollover.
use super::*;
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
};
use crate::foundation::ScopeOwnershipGeneration;

pub(super) async fn exercise(
    h: &f::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    content: &Content,
    item: [u8; 16],
) -> f::TestResult {
    let scope = f::scope()?;
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id: world,
        channel_id: channel,
    } = scope
    else {
        return Err("Channel fixture".into());
    };
    let before = h
        .root
        .read_item_loop_ground(world, channel)
        .await
        .map_err(f::debug)?;
    assert!(
        before
            .iter()
            .find(|row| row.item.item_instance_id == item)
            .ok_or("minted corpse view")?
            .native_corpse
    );
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(f::debug)?
        .ok_or("current assignment")?;
    let outcome = h
        .writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([149; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(f::debug)?,
            command: AssignmentCommand::Replace {
                scope,
                predecessor,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(f::debug)?;
    let AssignmentOutcome::Committed(replaced) = outcome else {
        return Err(format!("scope replacement {outcome:?}").into());
    };
    assert_eq!(replaced.assignment.ownership_generation, 2);
    crate::bestiary_postgres_harness::publish_readiness_for_scope(
        &h.pool, &h.root, &h.node, 2, scope,
    )
    .await?;
    eprintln!("QN1_ELEMENTAL_CORPSE_HANDOFF_PG_READY scope_generation=2");
    // Prepared current-session projection for this item-reader case, matching the
    // harness's seeded session model. This does not qualify transport readmission.
    // The actual assignment and readiness were independently published above;
    // neither is reconstructed from the old corpse's immutable MINT receipt.
    let updated = sqlx::query("UPDATE game_durability_reconnect_sessions SET scope_ownership_generation=2 WHERE game_session_id=encode($1,'hex')::uuid AND scope_ownership_generation=1")
        .bind(f::id(f::SESSION).as_slice()).execute(&h.pool).await?;
    assert_eq!(updated.rows_affected(), 1);
    let fence = crate::durability::item_transfer::CurrentCharacterItemFence {
        scope_ownership_generation: ScopeOwnershipGeneration::new(2).map_err(f::debug)?,
        ..f::fence()?
    };
    let permit = SpellLanePermit::of_fresh_scope(scope)
        .await
        .map_err(f::debug)?;
    let source = vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 3];
    let destination = vec![0, 0, 0, 3, 0, 0, 0, 2, 0, 3];
    let moved = h
        .root
        .commit_item_loop_move(
            &permit,
            authority,
            &h.node,
            &fence,
            f::command(98)?,
            content,
            LoopMoveRequest {
                item,
                count: None,
                pickup_cell: Some(source),
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position: destination.clone(),
                    map_revision: format!("sha256:{}", "07".repeat(32)),
                    content_revision: format!("sha256:{}", "01".repeat(32)),
                    placement_context: vec![3; 32],
                }),
            },
        )
        .await
        .map_err(f::debug)?
        .map_err(|e| format!("generation-2 Ground drag refused {e:?}"))?;
    assert!(!moved.replayed);
    let generations: (String, String) = sqlx::query_as("SELECT m.death_scope_ownership_generation::text,g.runtime_scope_ownership_generation::text FROM game_item_mint_receipts m JOIN game_item_ground_locations g USING(item_instance_id) WHERE m.item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(generations, ("1".into(), "2".into()));
    let rows = h
        .root
        .read_item_loop_ground(world, channel)
        .await
        .map_err(f::debug)?;
    let corpse = rows
        .iter()
        .find(|row| row.item.item_instance_id == item)
        .ok_or("dragged corpse view")?;
    assert_eq!(corpse.item.ground.spatial_position, destination);
    assert_eq!(corpse.item.runtime_scope_ownership_generation, 2);
    assert!(
        corpse.native_corpse,
        "a fresh owner's Ground drag must retain the historical corpse classification"
    );
    eprintln!("QN1_ELEMENTAL_CORPSE_HANDOFF_PG_EXECUTED");
    Ok(())
}

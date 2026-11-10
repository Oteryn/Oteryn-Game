//! Produce an actual pre-unification receipt for the current recovery owner.
use super::*;
use crate::durability::item_loop_move::{
    AltarArrivalTarget, LoopDestination, LoopGround, LoopMoveReceipt, LoopMoveRequest,
};
use crate::durability::spell_items_abi::QualifiedItemDefinition;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;

pub(super) async fn seed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    command: UseCommand,
    rule: &SummonRule,
    corpse: [u8; 16],
    readset: &[(u64, u64)],
) -> TestResult<LoopMoveReceipt> {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("historical arrival native content")?;
    let fence = command.item_fence.ok_or("historical arrival item fence")?;
    let qualified = QualifiedItemDefinition::from_native_policy(
        native
            .item_policy(rule.corpse, "definition-r1")
            .ok_or("historical corpse policy")?,
    )
    .map_err(debug)?;
    let (cell, map, content, frame): (Vec<u8>, String, String, Vec<u8>) = sqlx::query_as(
        "SELECT spatial_position,map_revision,content_revision,native_room_placement_context \
         FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(corpse.as_slice())
    .fetch_one(&h.pool)
    .await?;
    // This writer's historical ABI used the spell frame. Only historical test
    // data goes through it; new wire arrivals retain native map coordinates.
    let request = LoopMoveRequest {
        item: corpse,
        count: None,
        pickup_cell: Some(cell),
        destination: LoopDestination::Ground(LoopGround {
            spatial_position: [
                i32::from(rule.spawn.x).to_be_bytes().as_slice(),
                i32::from(rule.spawn.y).to_be_bytes().as_slice(),
                (-i16::from(rule.spawn.floor)).to_be_bytes().as_slice(),
            ]
            .concat(),
            map_revision: map,
            content_revision: content,
            placement_context: frame,
        }),
    };
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("historical arrival lane")?;
    let receipt = owner
        .root
        .commit_item_loop_altar_arrival(
            &permit,
            owner.character,
            owner.holder,
            fence,
            CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).map_err(debug)?,
            ),
            &NativeItemWeights(native).with_wheel(owner.premium_coordinator),
            request,
            &qualified,
            AltarArrivalTarget::new(rule.key, readset.to_vec(), owner.reset_epoch.try_into()?),
        )
        .await
        .map_err(debug)?
        .map_err(debug)?;
    assert!(!receipt.moved.replayed);
    assert!(receipt.summon.is_some());
    Ok(receipt.moved)
}

/// A new wire arrival must not compete with an older, still-unrealized receipt.
pub(super) async fn preserve_pending(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    rule: &SummonRule,
    source: &crate::durability::item_mint::GroundPlacement,
) -> TestResult {
    use crate::durability::item_mint::{
        CORPSE_MATERIALIZATION_PURPOSE_KEY, ItemMintCause, ItemMintOutcome, ItemMintRequest,
        TypedDefinitionRef,
    };
    use crate::gameplay_transport::item_move::ItemMoveStep;
    use crate::gameplay_transport::item_view::{ItemViewContinuity, SessionItemView};
    use oteryn_protocol_oteryn::item_view::{
        ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection, WorldTilePosition,
        encode_item_move_intent_for,
    };
    let fence = command.item_fence.ok_or("pending history item fence")?;
    let definition = |family: &str, key: &str| TypedDefinitionRef {
        family: family.into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    };
    let mut candidate = h
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    owner.world_id,
                    owner.channel_id,
                    fence.scope_ownership_generation,
                    103,
                    1,
                    definition("LootTable", "fixture:m49.pending-history"),
                    CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
                    0,
                ),
                item: definition("Item", rule.corpse),
                quantity: 1,
                ground: source.clone(),
                content_revision: source.content_revision.clone(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = candidate.fresh_death_lane_permit().await;
    let ItemMintOutcome::Committed(corpse) = h
        .root
        .commit_corpse_mint(
            &permit,
            owner.character,
            owner.holder,
            &mut candidate,
            *fence.character_id.as_bytes(),
        )
        .await
        .map_err(debug)?
    else {
        return Err("pending history second corpse MINT".into());
    };
    drop(permit);
    let before: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_elemental_altar_receipts), \
         (SELECT count(*) FROM game_elemental_boss_realizations), \
         (SELECT count(*) FROM game_elemental_arrival_receipts)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(before, (1, 0, 0));
    let item = ItemKey::Instance(corpse.item_instance_id);
    let mut view = SessionItemView::resume(ItemViewContinuity::default())
        .with_map_view()
        .with_equip_drop();
    let handle = view
        .map_view(&[item], |table| {
            table
                .handle(&item)
                .ok_or(crate::gameplay_transport::item_view::ItemViewError::Exhausted)
        })
        .map_err(debug)?;
    let selection = ItemMoveSelection {
        equip_drop: true,
        container_tree: false,
        world_interaction: true,
    };
    let payload = encode_item_move_intent_for(
        &ItemMoveIntent {
            source: handle,
            destination: ItemMoveDestination::Ground(WorldTilePosition {
                x: i32::from(rule.spawn.x),
                y: i32::from(rule.spawn.y),
                floor: i16::from(rule.spawn.floor),
            }),
        },
        selection,
    )
    .map_err(debug)?;
    let command = UseCommand {
        command_id: command.command_id + 1,
        ..command
    };
    for _ in 0..2 {
        let result = crate::gameplay_transport::item_move::item_move(
            owner, actor, command, &payload, selection, false, &mut view,
        )
        .await;
        assert!(
            matches!(result, ItemMoveStep::Result(ItemMoveOutcome::Moved, _)),
            "move beside historical pending arrival: {result:?}"
        );
    }
    let state: (i16, i64, i64) = sqlx::query_as(
        "SELECT i.lifecycle,i.quantity, \
         (SELECT count(*) FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id) \
         FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(corpse.item_instance_id.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(state, (1, 1, 1));
    let after: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_elemental_altar_receipts), \
         (SELECT count(*) FROM game_elemental_boss_realizations), \
         (SELECT count(*) FROM game_elemental_arrival_receipts)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(after, before);
    Ok(())
}

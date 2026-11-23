use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

pub(super) const DRAG_SCENARIO: &str = "qengine-rewards-cobra-corpse-drag";

pub(super) const MINT_SCENARIO: &str = "qengine-rewards-ordinary-orc-corpse-decay";

pub(super) async fn exercise_mint(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    super::qengine_kill_stories_fixtures::death_actor(
        owner,
        admitted,
        "oteryn:creature.orc",
        60000,
    )
    .await?;
    let mints: Vec<(String, i64)> = sqlx::query_as(
        "SELECT item_instance_id::text, game_corpse_retirement_deadline(item_instance_id)-materialized_at
         FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION'
         AND draw_ordinal=0 AND loot_table_production_key='oteryn:item.tibia.i5966'",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(mints.len(), 1);
    let source: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/native-gameplay/ordinary-corpse-cohort.json"
    )))?;
    let chain: Vec<(u32, i64)> = serde_json::from_value(source["chains"]["5966"].clone())?;
    assert_eq!(
        mints[0].1,
        chain.iter().map(|(_, duration)| duration).sum::<i64>()
    );
    let stages: Vec<(String, Option<String>, i64)> = sqlx::query_as(
        "SELECT stage.definition_production_key, stage.target_production_key,
                stage.expires_at_unix_ms-mint.materialized_at
         FROM game_spell_field_temporal_schedules stage JOIN game_item_mint_receipts mint USING(item_instance_id)
         WHERE stage.item_instance_id=$1::uuid AND stage.source_transaction_id=mint.transaction_id
           AND stage.source_ordinal=0
         ORDER BY stage.source_stage",
    )
    .bind(&mints[0].0)
    .fetch_all(&h.pool)
    .await?;
    let mut elapsed = 0;
    let expected: Vec<_> = chain
        .iter()
        .enumerate()
        .map(|(index, (stage, duration))| {
            elapsed += duration;
            (
                format!("oteryn:item.tibia.i{stage}"),
                chain
                    .get(index + 1)
                    .map(|(target, _)| format!("oteryn:item.tibia.i{target}")),
                elapsed,
            )
        })
        .collect();
    assert_eq!(stages, expected);
    Ok(())
}

pub(super) async fn exercise_drag(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
    use crate::gameplay_transport::item_view::ItemKey;
    use oteryn_protocol_oteryn::item_view::WorldTilePosition;

    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let death = super::qengine_kill_stories_fixtures::death_actor(
        owner,
        admitted,
        "oteryn:creature.cobra",
        60000,
    )
    .await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let (at, before) = {
        let runtime = owner.runtime.lock().await;
        let at = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let corpse = runtime
            .bound_corpse(death.corpse_identity())
            .ok_or("bound corpse")?;
        (at, corpse)
    };
    assert_ne!(before.position, at);
    let (map_revision, content_revision, placement_context) =
        sqlx::query_as::<_, (String, String, Vec<u8>)>(
            "SELECT map_revision, content_revision, native_room_placement_context
             FROM game_item_ground_locations WHERE item_instance_id=$1",
        )
        .bind(sqlx::types::Uuid::from_bytes(before.item.item_instance_id))
        .fetch_one(&h.pool)
        .await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("item fence")?
        .item_fence();
    assert_eq!(
        crate::gameplay_transport::loop_move::move_loop_item(
            owner,
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 60001,
                item_fence: Some(fence),
            },
            LoopMove {
                slot: None,
                item: ItemKey::Instance(before.item.item_instance_id),
                count: None,
                target: LoopTarget::Ground(WorldTilePosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor.checked_neg().ok_or("map floor")?,
                }),
            },
        )
        .await,
        LoopMoveResult::Committed,
    );
    let after = owner
        .runtime
        .lock()
        .await
        .bound_corpse(before.identity)
        .ok_or("relocated corpse")?;
    assert_eq!(after.position, at);
    assert!(after.revision > before.revision);
    let snapshot = owner
        .root
        .read_scoped_corpse_decay_schedule(owner.character, fence.runtime_scope)
        .await
        .map_err(debug)?;
    let persisted = snapshot
        .corpses
        .iter()
        .find(|corpse| corpse.corpse_item_instance_id == before.item.item_instance_id)
        .and_then(|corpse| corpse.loop_ground_spatial_position.as_deref())
        .ok_or("durable corpse position")?;
    let persisted = crate::durability::spell_items_abi::decode_ground_cell(persisted)?;
    assert_eq!(
        (persisted.x, persisted.y, persisted.z),
        (at.x, at.y, -i32::from(at.floor))
    );
    {
        use crate::durability::spell_item_transaction as item_tx;
        use crate::durability::spell_items_abi::{QualifiedItemDefinition, SpellGroundTarget};
        use crate::foundation::{CommandId, CommandRef};

        let native = owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or("active native policies")?;
        let target = SpellGroundTarget {
            item_profile_source: native.item_profile_source(),
            spatial_position: [
                at.x.to_be_bytes().as_slice(),
                at.y.to_be_bytes().as_slice(),
                at.floor.to_be_bytes().as_slice(),
            ]
            .concat(),
            map_revision,
            content_revision,
            placement_context,
        };
        let permit = owner.spell_lane_permit().await.ok_or("item lane")?;
        let mut tx = h.pool.begin().await?;
        let authority = item_tx::assert_spell_item_authority_in_transaction(
            &mut tx,
            &permit,
            owner.root,
            owner.character,
            owner.holder,
            &fence,
            CommandRef::new(admitted.game_session_id, CommandId::new(60004).map_err(debug)?),
            native.source_digest(),
        )
        .await
        .map_err(debug)?;
        let definition = item_tx::current_top_corpse_definition_in_transaction(
            &mut tx, &authority, &target,
        )
        .await
        .map_err(debug)?;
        let qualified = QualifiedItemDefinition::from_native_policy(
            native
                .item_policy(&definition.production_key, &definition.revision_ref)
                .ok_or("relocated corpse policy")?,
        )
        .map_err(debug)?;
        let corpse = item_tx::prepare_corpse_in_transaction(
            &mut tx, &authority, &target, &qualified,
        )
        .await
        .map_err(debug)?;
        assert_eq!(corpse.item_instance_id, before.item.item_instance_id);
        let cell = corpse.cell()?;
        assert_eq!((cell.x, cell.y, cell.z), (at.x, at.y, i32::from(at.floor)));
        let tile = item_tx::read_spell_tile_in_transaction(&mut tx, &authority, &target)
            .await
            .map_err(debug)?;
        assert_eq!(tile.items[0].item_instance_id, corpse.item_instance_id);
        tx.rollback().await?;
    }
    use oteryn_protocol_oteryn::quick_loot::{QuickLootAction, QuickLootRequest};

    let loot = QuickLootRequest {
        action: QuickLootAction::Click,
        corpse: Some(before.item.item_instance_id),
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    };
    // A live corpse can carry loot; use Command 114 to empty it before pickup.
    owner
        .execute_quick_loot(
            *admitted,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 60002,
                item_fence: Some(fence),
            },
            loot,
        )
        .await
        .ok_or("quick loot response")?;
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_corpse_container_entries WHERE parent_item_instance_id=$1",
    )
    .bind(sqlx::types::Uuid::from_bytes(before.item.item_instance_id))
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(remaining, 0, "corpse must be empty before pickup");
    assert_eq!(
        crate::gameplay_transport::loop_move::move_loop_item(
            owner,
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 60003,
                item_fence: Some(fence),
            },
            LoopMove {
                slot: None,
                item: ItemKey::Instance(before.item.item_instance_id),
                count: None,
                target: LoopTarget::MainBackpack,
            },
        )
        .await,
        LoopMoveResult::Committed,
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .bound_corpse(before.identity)
            .is_none(),
        "a carried corpse cannot remain visible on the Ground"
    );
    // Recreate the persisted / published split left by losing the pickup ACK.
    owner
        .runtime
        .lock()
        .await
        .set_corpse_item_ground_position(before.item.item_instance_id, Some(at))
        .map_err(debug)?;
    assert_eq!(
        crate::gameplay_transport::loop_move::committed_loop_move(
            owner,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 60003,
                item_fence: Some(fence),
            },
        )
        .await,
        Some(crate::gameplay_transport::item_move::LoopMoveReplay::Moved),
    );
    assert!(
        owner.runtime.lock().await.bound_corpse(before.identity).is_none(),
        "the committed pickup must publish current custody before replay ACK"
    );
    Ok(())
}

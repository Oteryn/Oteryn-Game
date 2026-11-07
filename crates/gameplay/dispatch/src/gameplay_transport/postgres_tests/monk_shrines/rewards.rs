use super::*;
use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;

pub(super) async fn fill(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let bag = owner
        .root
        .read_character_backpack(authority, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("bag")?;
    for n in bag.entries.len()..20 {
        super::super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &world_callbacks::Offering {
                item: "oteryn:item.tibia.i39186".into(),
                count: 1,
                position: Position {
                    x: 32453,
                    y: 32292,
                    floor: -7,
                },
            },
            n as u64 + 200,
            true,
        )
        .await?;
    }
    let bag = owner
        .root
        .read_character_backpack(authority, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("bag")?;
    assert_eq!(bag.entries.len(), 20);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn retry(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    world: &crate::map::boot::BundleWorld,
    callback: &world_callbacks::Callback,
    placement: &[u8],
    key: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let pending = owner
        .root
        .pending_monk_reward(authority, fence.character_id, &callback.key)
        .await
        .map_err(debug)?;
    assert!(pending.is_some(), "full bag cannot erase the earned reward");
    let reservations: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_reservations")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        reservations, 0,
        "a refused delivery leaves no session-bound reservation"
    );
    let bag = owner
        .root
        .read_character_backpack(authority, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("bag")?;
    let original_entries = bag.entries.len();
    let filler = bag
        .entries
        .iter()
        .find(|e| {
            e.item.definition.production_key == "oteryn:item.tibia.i39186"
                || e.item.definition.production_key == "oteryn:item.tibia.i4242"
        })
        .ok_or("filler")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let (digest, frame) = {
        let r = owner.runtime.lock().await;
        (
            r.content_pin().server_artifact_digest(),
            r.content_pin().frame_binding_digest(),
        )
    };
    let revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let moved = owner
        .root
        .commit_item_loop_move(
            &permit,
            authority,
            owner.holder,
            &fence.item_fence(),
            CommandRef::new(
                admitted.game_session_id,
                CommandId::new(4000).map_err(debug)?,
            ),
            &NativeItemWeights(native),
            LoopMoveRequest {
                item: filler.item.item_instance_id,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            32453, 32292, -7,
                        ),
                    map_revision: crate::map::overlay::map_revision(
                        owner.bundle_map().ok_or("map")?.overlay.base(),
                    ),
                    content_revision: revision,
                    placement_context: frame.to_vec(),
                }),
            },
        )
        .await
        .map_err(debug)?;
    assert!(moved.is_ok(), "{moved:?}");
    drop(permit);
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let result = wire(
        owner,
        admitted,
        4,
        2,
        oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.to_vec(),
            expected_revision: world.facts().object_revision(key),
        })
        .map_err(debug)?,
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    assert!(
        owner
            .root
            .pending_monk_reward(authority, fence.character_id, &callback.key)
            .await
            .map_err(debug)?
            .is_none()
    );
    let bag = owner
        .root
        .read_character_backpack(authority, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("bag")?;
    assert_eq!(bag.entries.len(), original_entries);
    assert_eq!(
        bag.entries
            .iter()
            .filter(|e| e.item.definition.production_key == "oteryn:item.tibia.i50267")
            .count(),
        1
    );
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 1);
    let quest_receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(&callback.key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(quest_receipts, 1);
    Ok(())
}

pub(super) async fn fill_capacity(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    use crate::durability::item_transfer::ItemWeightLookup;
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let state = owner
        .root
        .read_character_progression(authority, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("progression")?;
    let capacity =
        oteryn_gameplay_items::item_capacity::project_global_capacity_centi_oz(state.level, "monk")
            .map_err(debug)?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let bag = owner
        .root
        .read_character_backpack(authority, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("bag")?;
    let weights = NativeItemWeights(native);
    let carried = u64::from(weights.weight_of(&bag.backpack.definition).map_err(debug)?)
        + bag
            .entries
            .iter()
            .map(|e| {
                weights
                    .weight_of(&e.item.definition)
                    .map(|w| u64::from(w) * u64::from(e.item.quantity))
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(debug)?
            .into_iter()
            .sum::<u64>();
    let position = Position {
        x: 32453,
        y: 32292,
        floor: -7,
    };
    super::super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &world_callbacks::Offering {
            item: "oteryn:item.tibia.i4242".into(),
            count: 1,
            position,
        },
        400,
        true,
    )
    .await?;
    let remainder = capacity
        .checked_sub(carried + 50000 + 400)
        .ok_or("capacity fixture")?;
    assert_eq!(remainder % 10, 0);
    let mut coins = remainder / 10;
    let mut n = 401;
    while coins > 0 {
        let count = coins.min(100) as u16;
        super::super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &world_callbacks::Offering {
                item: "oteryn:item.tibia.i3031".into(),
                count,
                position,
            },
            n,
            true,
        )
        .await?;
        coins -= u64::from(count);
        n += 1;
    }
    let bag = owner
        .root
        .read_character_backpack(authority, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("bag")?;
    assert!(
        bag.entries.len() < 20,
        "weight refusal must have a free backpack slot"
    );
    Ok(())
}

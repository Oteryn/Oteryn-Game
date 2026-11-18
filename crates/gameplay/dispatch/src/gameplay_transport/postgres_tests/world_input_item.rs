//! Shared mint-and-transfer fixture for source items used by World callback tests.
use super::{TestResult, fixture::id};

pub(super) async fn input_item(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    offering: &oteryn_gameplay_quests::quest::world_callbacks::Offering,
    n: u64,
    carried: bool,
) -> TestResult {
    use crate::durability::{
        item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest},
        item_mint::{
            GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
        },
        item_transfer::{ItemTransferDestination, ItemTransferRequest},
    };
    use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: offering.item.clone(),
        revision_ref: "definition-r1".into(),
    };
    let facts = generation_item_facts(native, &definition)
        .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?;
    let map = owner.bundle_map().ok_or("map")?;
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
    let mut candidate = owner
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    owner.world_id,
                    owner.channel_id,
                    fence.scope_ownership_generation,
                    1,
                    n,
                    TypedDefinitionRef {
                        family: "LootTable".into(),
                        production_key: "fixture:quest-callbacks".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    "fixture:quest-callbacks".into(),
                    0,
                ),
                item: definition,
                quantity: u32::from(offering.count),
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            offering.position.x,
                            offering.position.y,
                            offering.position.floor,
                        ),
                    corpse_ref: id(3).to_vec(),
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: revision.clone(),
                    native_room_placement_context: frame.to_vec(),
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let ItemMintOutcome::Committed(item) = owner
        .root
        .commit_item_mint(&permit, owner.character, owner.holder, &mut candidate)
        .await
        .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?
    else {
        return Err("mint".into());
    };
    let command = |offset| -> TestResult<_> {
        Ok(crate::foundation::CommandRef::new(
            fence.game_session_id,
            crate::foundation::CommandId::new(1000 + n * 10 + offset)
                .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?,
        ))
    };
    let mut transfer = owner
        .root
        .freeze_item_transfer(
            owner.character,
            owner.holder,
            fence,
            ItemTransferRequest {
                weight_generation_digest: native.source_digest(),
                command: command(0)?,
                source_item_instance_id: item.item_instance_id,
                destination: ItemTransferDestination::MainBackpack,
                item: facts,
                backpack: Some(
                    generation_item_facts(
                        native,
                        &TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: "oteryn:item.tibia.i2854".into(),
                            revision_ref: "definition-r1".into(),
                        },
                    )
                    .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?,
                ),
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
            &NativeItemWeights(native),
        )
        .await
        .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?;
    owner
        .root
        .commit_item_transfer(
            &permit,
            owner.character,
            owner.holder,
            fence,
            &mut transfer,
            &NativeItemWeights(native),
        )
        .await
        .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?;
    if carried {
        return Ok(());
    }
    let moved = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence,
            command(1)?,
            &NativeItemWeights(native),
            LoopMoveRequest {
                item: item.item_instance_id,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            offering.position.x,
                            offering.position.y,
                            offering.position.floor,
                        ),
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: revision,
                    placement_context: frame.to_vec(),
                }),
            },
        )
        .await
        .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?;
    assert!(moved.is_ok(), "drop: {moved:?}");
    Ok(())
}

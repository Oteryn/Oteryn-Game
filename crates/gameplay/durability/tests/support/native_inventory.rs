//! Shared native inventory setup: all custody changes go through their existing owners.
use crate::{durability, f, foundation, reward_fixture};
use durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use durability::item_transfer::{
    ItemTransferDestination, ItemTransferOutcome, ItemTransferRequest,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_items::item_ref_admission::generation_item_facts;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn mint_to_bag(
    h: &f::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    native: &NativeGameplayState,
    weights: &reward_fixture::Weights<'_>,
    item: u32,
    quantity: u32,
    ordinal: u32,
) -> f::TestResult<[u8; 16]> {
    use foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
    let mut candidate = h
        .root
        .freeze_item_mint(
            authority,
            &h.node,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    WorldId::decode(&f::id(f::WORLD)).map_err(f::debug)?,
                    ChannelId::decode(&f::id(f::CHANNEL)).map_err(f::debug)?,
                    ScopeOwnershipGeneration::new(1).map_err(f::debug)?,
                    1,
                    1,
                    TypedDefinitionRef {
                        family: "LootTable".into(),
                        production_key: "fixture:engitem".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    "fixture:engitem".into(),
                    ordinal,
                ),
                item: reward_fixture::definition(item),
                quantity,
                ground: GroundPlacement {
                    spatial_position: vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 3],
                    corpse_ref: f::id(3).to_vec(),
                    map_revision: "map-1".into(),
                    content_revision: "content-1".into(),
                    native_room_placement_context: f::id(6).to_vec(),
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(f::debug)?;
    let permit = candidate.fresh_death_lane_permit().await;
    let ItemMintOutcome::Committed(minted) = h
        .root
        .commit_item_mint(&permit, authority, &h.node, &mut candidate)
        .await
        .map_err(f::debug)?
    else {
        return Err("ingredient mint did not commit".into());
    };
    let fence = f::item_fence()?;
    let mut transfer = h
        .root
        .freeze_item_transfer(
            authority,
            &h.node,
            fence,
            ItemTransferRequest {
                weight_generation_digest: native.source_digest(),
                command: reward_fixture::command(9300 + u64::from(ordinal))?,
                source_item_instance_id: minted.item_instance_id,
                destination: ItemTransferDestination::MainBackpack,
                item: generation_item_facts(native, &reward_fixture::definition(item))
                    .map_err(f::debug)?,
                backpack: Some(
                    generation_item_facts(native, &reward_fixture::definition(2854))
                        .map_err(f::debug)?,
                ),
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
            weights,
        )
        .await
        .map_err(f::debug)?;
    let lane = durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fence.runtime_scope)
        .await
        .map_err(f::debug)?;
    let outcome = h
        .root
        .commit_item_transfer(&lane, authority, &h.node, fence, &mut transfer, weights)
        .await
        .map_err(f::debug)?;
    assert!(matches!(outcome, ItemTransferOutcome::Committed(_)));
    Ok(minted.item_instance_id)
}

//! Native item weight, command and backpack fixtures shared by PostgreSQL tests.
use crate::bestiary_postgres_harness as f;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::item_transfer::{
    ItemTransferDestination, ItemTransferOutcome, ItemTransferRequest, ItemWeightLookup,
    ItemWeightRefused,
};
use crate::foundation::{
    ChannelId, CommandId, CommandRef, GameSessionId, ScopeOwnershipGeneration, WorldId,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_items::item_ref_admission::generation_item_facts;

pub(crate) struct Weights<'a>(pub(crate) &'a NativeGameplayState);
impl ItemWeightLookup for Weights<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        generation_item_facts(self.0, definition)
            .map_err(|_| ItemWeightRefused::DefinitionMismatch)?
            .weight
            .ok_or(ItemWeightRefused::Unknown)
    }
}
pub(crate) fn definition(item: u32) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{item}"),
        revision_ref: "definition-r1".into(),
    }
}
pub(crate) fn command(n: u64) -> f::TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
        CommandId::new(n).map_err(f::debug)?,
    ))
}

// Shared engine-only native item fixture; no quest data or direct custody SQL.
pub(crate) async fn equip_backpack(
    h: &f::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    native: &NativeGameplayState,
    weights: &Weights<'_>,
) -> f::TestResult {
    let mut minted = h
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
                    0,
                ),
                item: definition(2854),
                quantity: 1,
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
    let permit = minted.fresh_death_lane_permit().await;
    let ItemMintOutcome::Committed(bag) = h
        .root
        .commit_item_mint(&permit, authority, &h.node, &mut minted)
        .await
        .map_err(f::debug)?
    else {
        return Err("bag mint".into());
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
                command: command(9300)?,
                source_item_instance_id: bag.item_instance_id,
                destination: ItemTransferDestination::ContainerSlot,
                item: generation_item_facts(native, &definition(2854)).map_err(f::debug)?,
                backpack: None,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
            weights,
        )
        .await
        .map_err(f::debug)?;
    let lane =
        crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fence.runtime_scope)
            .await
            .map_err(f::debug)?;
    assert!(matches!(
        h.root
            .commit_item_transfer(&lane, authority, &h.node, fence, &mut transfer, weights)
            .await
            .map_err(f::debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    Ok(())
}

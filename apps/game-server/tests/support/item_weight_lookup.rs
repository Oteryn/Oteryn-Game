//! Test-only independent Content catalogue for the explicitly named native PG fixtures.
//! These synthetic weights are centi-oz; production never imports this module.
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemWeightLookup, ItemWeightRefused};

pub const TEST_WEIGHTS: TestWeights = TestWeights;
pub struct TestWeights;
impl ItemWeightLookup for TestWeights {
    fn generation_digest(&self) -> [u8; 32] {
        [0xD1; 32]
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        match (
            definition.family.as_str(),
            definition.production_key.as_str(),
            definition.revision_ref.as_str(),
        ) {
            (
                "Item",
                "fixture:b3.backpack" | "fixture:b3.coin" | "fixture:b3.stone" | "fixture:b3.bag",
                "definition-r1",
            ) => Ok(1),
            (
                "Item",
                "fixture:equip.sword"
                | "fixture:equip.armor"
                | "fixture:equip.heavy-armor"
                | "fixture:equip.bag"
                | "fixture:equip.timed-helmet"
                | "fixture:npc.sword",
                "definition-r1",
            ) => Ok(1),
            (
                "Item",
                "oteryn:item.tibia.i3031" | "oteryn:item.tibia.i3035" | "oteryn:item.tibia.i3043",
                "definition-r1",
            ) => Ok(10),
            ("Item", "oteryn:item.tibia.i3607", "definition-r1") => Ok(400),
            ("Item", "fixture:house.item", "definition-r1") => Ok(1),
            ("Item", "oteryn:item.tibia.i3577", "definition-r1") => Ok(1),
            ("Item", "fixture:ground.root", "definition-r1") => Ok(1),
            ("ItemType", "fixture:corpse-loot", "rev-1") => Ok(1),
            ("Item", "fixture:capacity.exact", "definition-r1") => Ok(155999),
            ("Item", "fixture:capacity.over", "definition-r1") => Ok(156000),
            ("Item", "fixture:capacity.level51", "definition-r1") => Ok(157998),
            _ => Err(ItemWeightRefused::Unknown),
        }
    }
}

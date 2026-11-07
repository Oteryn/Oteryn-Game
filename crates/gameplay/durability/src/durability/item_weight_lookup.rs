//! Test-only independent Content catalogue for the explicitly named native PG fixtures.
//! These synthetic weights are centi-oz; production never imports this module.
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemWeightLookup, ItemWeightRefused};

pub use oteryn_gameplay_items::item_weight_lookup::{FIXTURE_WEIGHTS, GENERATION_DIGEST};

pub const TEST_WEIGHTS: TestWeights = TestWeights;
pub struct TestWeights;
impl ItemWeightLookup for TestWeights {
    fn generation_digest(&self) -> [u8; 32] {
        GENERATION_DIGEST
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        FIXTURE_WEIGHTS.iter().find_map(|&(family, key, revision, weight)| {
            (definition.family == family
                && definition.production_key == key
                && definition.revision_ref == revision).then_some(weight)
        }).ok_or(ItemWeightRefused::Unknown)
    }
}

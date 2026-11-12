//! Source-bound AddItem assemblies of complete physical floor operands.
/// An authored native counterpart for an otherwise absent source action selector.
#[derive(Clone, Copy, Debug)]
pub struct NativeFloorBinding {
    pub action: u16,
    pub x: u16,
    pub y: u16,
    pub floor: i8,
    pub ordinal: u8,
    pub appearance: u16,
}

// Authored counterpart, not a recovered donor registration. Retain the bare B3
// ground beside the source Morguthis endpoint; provenance is in the lane contract.
pub const NATIVE_BINDINGS: &[NativeFloorBinding] = &[NativeFloorBinding {
    action: 40040,
    x: 33162,
    y: 32694,
    floor: -14,
    ordinal: 0,
    appearance: 417,
}];

#[derive(Debug)]
pub struct FloorRecipe {
    pub key: &'static str,
    pub action: u16,
    pub inputs: &'static [(&'static str, u16)],
    pub output: &'static str,
    pub output_count: u16,
    pub effect: Option<u16>,
}

// Crystal 201189f: data-global/scripts/quests/the_ancient_tombs/movements_craft_helmet.lua:1-39.
pub const RECIPES: &[FloorRecipe] = &[FloorRecipe {
    key: "oteryn:floor-assembly.ancient-helmet",
    action: 40040,
    inputs: &[
        ("oteryn:item.tibia.i3222", 1),
        ("oteryn:item.tibia.i3223", 1),
        ("oteryn:item.tibia.i3224", 1),
        ("oteryn:item.tibia.i3225", 1),
        ("oteryn:item.tibia.i3226", 1),
        ("oteryn:item.tibia.i3227", 1),
        ("oteryn:item.tibia.i3228", 1),
    ],
    output: "oteryn:item.tibia.i3229",
    output_count: 1,
    effect: Some(7),
}];

pub fn recipe(action: u16) -> Option<&'static FloorRecipe> {
    RECIPES.iter().find(|recipe| recipe.action == action)
}

#[derive(Debug)]
pub struct FloorTransformRecipe {
    pub key: &'static str,
    pub action: u16,
    pub input: &'static str,
    pub input_count: u16,
    pub target: &'static str,
    pub output: &'static str,
    pub duration_millis: u32,
    pub effect: Option<u16>,
    pub decay_target: &'static str,
}

// Same pinned handler:5-15; items.xml:8079-8087 fixes the original deadline.
pub const TRANSFORMS: &[FloorTransformRecipe] = &[FloorTransformRecipe {
    key: "oteryn:floor-transform.ancient-helmet-ruby",
    action: 40040,
    input: "oteryn:item.tibia.i3030",
    input_count: 1,
    target: "oteryn:item.tibia.i3229",
    output: "oteryn:item.tibia.i3230",
    duration_millis: 1_800_000,
    decay_target: "oteryn:item.tibia.i3229",
    effect: Some(7),
}];

pub fn transform(action: u16, input: &str) -> Option<&'static FloorTransformRecipe> {
    TRANSFORMS
        .iter()
        .find(|recipe| recipe.action == action && recipe.input == input)
}

pub fn actions() -> impl Iterator<Item = u16> {
    RECIPES
        .iter()
        .map(|recipe| recipe.action)
        .chain(TRANSFORMS.iter().map(|recipe| recipe.action))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
}

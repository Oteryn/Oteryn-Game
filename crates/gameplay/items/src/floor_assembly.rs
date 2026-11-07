//! Source-bound AddItem assemblies of complete physical floor operands.
#[derive(Debug)]
pub struct FloorRecipe {
    pub key: &'static str,
    pub action: u16,
    pub inputs: &'static [(&'static str, u16)],
    pub output: &'static str,
    pub output_count: u16,
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
}];

pub fn recipe(action: u16) -> Option<&'static FloorRecipe> {
    RECIPES.iter().find(|recipe| recipe.action == action)
}

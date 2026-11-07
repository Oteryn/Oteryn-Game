//! Crystal201189f data/scripts/actions/items/{blueberry,strawberry,lemon}_cupcake.lua.
//! A floor/House source burns one unit; actor effects are staged by the native owner.
use crate::novelty_items::{NoveltyEffects, NoveltyKind, NoveltyPlan};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;

pub fn source_id(key: &str) -> Option<u32> {
    match key {
        "oteryn:item.tibia.i28484" => Some(28484),
        "oteryn:item.tibia.i28485" => Some(28485),
        "oteryn:item.tibia.i28486" => Some(28486),
        _ => None,
    }
}

pub fn plan(native: &NativeGameplayState, used: &TypedDefinitionRef) -> Option<NoveltyPlan> {
    let id = source_id(&used.production_key)?;
    let policy = native.item_policy(&used.production_key, &used.revision_ref)?;
    if used.family != "Item" || !policy.record().admission.materializable {
        return None;
    }
    let donor = match id {
        28484 => "data/scripts/actions/items/blueberry_cupcake.lua:3-14",
        28485 => "data/scripts/actions/items/strawberry_cupcake.lua:3-14",
        28486 => "data/scripts/actions/items/lemon_cupcake.lua:1-20",
        _ => return None,
    };
    Some(NoveltyPlan {
        recipe: format!("oteryn:seasonal.cupcake.{id}"),
        outputs: Vec::new(),
        consume_source: true,
        effects: serde_json::to_value(NoveltyEffects {
            kind: NoveltyKind::ConsumeEffect,
            source_item_id: id,
            donor: donor.into(),
            ground: None,
            cues: Vec::new(),
            speech: Some("Mmmm.".into()),
            private_speech: false,
            require_head: false,
            require_player_target: false,
            dice_value: None,
            costume: None,
            damage: None,
            achievement: None,
            achievement_progress: Vec::new(),
            timed_output: None,
        })
        .ok()?,
    })
}

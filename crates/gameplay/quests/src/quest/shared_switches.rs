//! Source-keyed shared World switch leases; no invented personal Quest progress.
use super::world_callbacks::{Callback, Selector, Trigger};

#[derive(Clone, Copy, Debug)]
pub struct Recipe {
    pub key: &'static str,
    pub group: &'static str,
    pub action: u16,
    pub lifetime_millis: i64,
    pub source_line: u16,
}
pub const RECIPES: &[Recipe] = &[
    Recipe {
        key: "oteryn:quest-callback/ancient-tombs-thalas-switch",
        group: "oteryn:world-counter/ancient-tombs/thalas",
        action: 12121,
        lifetime_millis: 1_200_000,
        source_line: 2,
    },
    Recipe {
        key: "oteryn:quest-callback/ancient-tombs-diprath-switch",
        group: "oteryn:world-counter/ancient-tombs/diprath",
        action: 12122,
        lifetime_millis: 1_200_000,
        source_line: 10,
    },
    Recipe {
        key: "oteryn:quest-callback/ancient-tombs-ashmunrah-switch",
        group: "oteryn:world-counter/ancient-tombs/ashmunrah",
        action: 12123,
        lifetime_millis: 1_200_000,
        source_line: 18,
    },
];
pub fn recipe(key: &str) -> Option<&'static Recipe> {
    RECIPES.iter().find(|recipe| recipe.key == key)
}
pub(super) fn callbacks() -> impl Iterator<Item = Callback> {
    RECIPES.iter().map(|r| Callback {
        key:r.key.into(),quest:"oteryn:quest.the_ancient_tombs_quest".into(),trigger:Trigger::Use,
        selector:Selector {position:None,unique:None,action:Some(r.action),appearance:Some(2772)},
        lever:Some([2772,2773]),reversible:false,passages:Vec::new(),teleport:None,facing:None,
        effects:Default::default(),storage:Vec::new(),minimum_level:None,experience:None,monk_reward:None,
        offerings:Vec::new(),item_on_target:None,return_on_failure:false,failure_message:None,
        success_message:None,native_rule:None,
        sources:vec![format!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ancient_tombs/actions_active_teleport_switches.lua:{}",r.source_line)],
    })
}

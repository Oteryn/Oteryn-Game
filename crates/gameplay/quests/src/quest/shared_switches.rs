//! Source-keyed shared World switch leases; no invented personal Quest progress.
pub mod native;
use super::world_callbacks::{
    Callback, CallbackEffectSet, MagicEffect, Position, Selector, Storage, Trigger,
};

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
fn activation_callbacks() -> impl Iterator<Item = Callback> {
    RECIPES.iter().map(|r| Callback { required_completed_quests: Vec::new(), admission: None,
world_change: None,
        key:r.key.into(),quest:"oteryn:quest.the_ancient_tombs_quest".into(),trigger:Trigger::Use,actor_kind:Default::default(),
        selector:Selector { native: None,position:None,unique:None,action:Some(r.action),appearance:Some(2772)},
        lever:Some([2772,2773]),reversible:false,opening_only:false,requires_world_admission:false,passages:Vec::new(),teleport:None,facing:None,
        effects:Default::default(),storage:Vec::new(),state_ops: Vec::new(), world_requirements:Vec::new(),minimum_level:None,experience:None,monk_reward:None,
        offerings:Vec::new(),item_on_target:None,return_on_failure:false,failure_message:None,
        success_message:None,native_rule:None,
        sources:vec![format!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ancient_tombs/actions_active_teleport_switches.lua:{}",r.source_line)],
    })
}

/// One source shared-count STEP predicate, with no personal progress substitute.
#[derive(Clone, Copy, Debug)]
pub struct Gate {
    pub quest: &'static str,
    pub key: &'static str,
    pub unique: u16,
    pub counter_recipe: &'static str,
    pub required: u16,
    pub destination: Position,
    pub effect: u16,
    pub source_file: &'static str,
    pub source_line: u16,
    /// A durable, one-way personal completion flag; missing rows never satisfy it.
    pub personal_completion: Option<&'static str>,
    pub progress: Option<Progress>,
}
#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub track: &'static str,
    pub minimum: i64,
    pub maximum: i64,
    pub target: i64,
}
pub const GATES: &[Gate] = &[
    Gate {
        quest: "oteryn:quest.the_ancient_tombs_quest",
        key: "oteryn:quest-callback/ancient-tombs-thalas-switch-gate",
        unique: 50135,
        counter_recipe: "oteryn:quest-callback/ancient-tombs-thalas-switch",
        required: 8,
        destination: Position {
            x: 33367,
            y: 32805,
            floor: -14,
        },
        effect: 11,
        source_file: "movements_enter_thalas_teleport_switches_done.lua",
        source_line: 7,
        personal_completion: None,
        progress: None,
    },
    Gate {
        quest: "oteryn:quest.the_ancient_tombs_quest",
        key: "oteryn:quest-callback/ancient-tombs-diprath-switch-gate",
        unique: 50137,
        counter_recipe: "oteryn:quest-callback/ancient-tombs-diprath-switch",
        required: 7,
        destination: Position {
            x: 33083,
            y: 32568,
            floor: -14,
        },
        effect: 11,
        source_file: "movements_enter_diprath_teleport_switche_done.lua",
        source_line: 8,
        personal_completion: None,
        progress: None,
    },
    Gate {
        quest: "oteryn:quest.the_ancient_tombs_quest",
        key: "oteryn:quest-callback/ancient-tombs-ashmunrah-switch-gate",
        unique: 50138,
        counter_recipe: "oteryn:quest-callback/ancient-tombs-ashmunrah-switch",
        required: 5,
        destination: Position {
            x: 33198,
            y: 32885,
            floor: -11,
        },
        effect: 11,
        source_file: "movements_enter_ashmunrah_teleport_switche_done.lua",
        source_line: 9,
        personal_completion: None,
        progress: None,
    },
    Gate {
        quest: "oteryn:quest.the_ancient_tombs_quest",
        key: "oteryn:quest-callback/ancient-tombs-diprath-room",
        unique: 40083,
        counter_recipe: "oteryn:quest-callback/ancient-tombs-diprath-switch",
        required: 7,
        destination: Position {
            x: 33092,
            y: 32590,
            floor: -15,
        },
        effect: 11,
        source_file: "movements_enter_diprath_room.lua",
        source_line: 11,
        personal_completion: Some(
            "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/diprath_sign8",
        ),
        progress: Some(Progress {
            track: "oteryn:quest-track/ancient-tombs/diprath-treasure",
            minimum: -1,
            maximum: i64::MAX,
            target: 3,
        }),
    },
];
pub fn gate(key: &str) -> Option<&'static Gate> {
    GATES.iter().find(|gate| gate.key == key)
}
pub(super) fn callbacks() -> impl Iterator<Item = Callback> {
    activation_callbacks().chain(GATES.iter().map(|gate| {
        let mut effects=CallbackEffectSet::default();effects.magic=Some(MagicEffect {position:gate.destination,effect:gate.effect});
        Callback { required_completed_quests: Vec::new(), admission: None,
world_change: None,
            key:gate.key.into(),quest:gate.quest.into(),trigger:Trigger::StepIn,actor_kind:Default::default(),
            selector:Selector { native: None,position:None,unique:Some(gate.unique),action:None,appearance:None},
            lever:None,reversible:false,opening_only:false,requires_world_admission:gate.progress.is_some(),passages:Vec::new(),teleport:Some(gate.destination),facing:None,effects,
            storage:gate.progress.into_iter().map(|p| Storage {track:p.track.into(),minimum:p.minimum,maximum:p.maximum,set:p.target,preserve:false,increment:false,at_least:true}).collect(),state_ops: Vec::new(), world_requirements:Vec::new(),minimum_level:None,experience:None,monk_reward:None,
            offerings:Vec::new(),item_on_target:None,return_on_failure:true,failure_message:None,success_message:None,native_rule:None,
            sources:vec![format!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ancient_tombs/{}:{}",gate.source_file,gate.source_line)],
        }
    }))
}

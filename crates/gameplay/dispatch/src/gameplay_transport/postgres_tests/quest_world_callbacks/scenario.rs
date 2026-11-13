// Keep delegated lane futures off the shared scenario stack as the integration grows.
use super::*;

pub(crate) fn scenario(name: &str, trigger: Position, lever_fixture: bool) -> TestResult {
    scenario_case(name, trigger, lever_fixture, None)
}

#[allow(
    dead_code,
    reason = "per-quest stateless PG cases are independently picked after the common harness"
)]
pub(crate) fn stateless_scenario(
    name: &str,
    trigger: Position,
    expected_destination: Position,
) -> TestResult {
    scenario_case(name, trigger, false, Some(expected_destination))
}

fn scenario_case(
    name: &str,
    trigger: Position,
    lever_fixture: bool,
    stateless_destination: Option<Position>,
) -> TestResult {
    let sidec_npc = name.starts_with("sidec-");
    let name = name.strip_prefix("sidec-").unwrap_or(name);
    let engenc_asura = name.starts_with("engenc-asura-native-");
    let carrot = name.starts_with("m07-carrot-");
    let ring_offering = name == "m07-ring-offering";
    let m07_too_hot = name.starts_with("m07/too-hot-column-");
    let m07_repair = name.starts_with("m07/machinery-repair-");
    let m07_machinery = name.starts_with("m07/machinery-");
    let m07_machinery_level49 = m07_machinery && name.ends_with("-level49");
    let m07_realm = name.starts_with("m07/realm-");
    let lion_trial = matches!(
        name,
        "lions-rock-strength-first" | "lions-rock-beauty" | "lions-rock-tears"
    );
    let m07_lion = name.starts_with("m07/lion-");
    let m07_dreamer = name.starts_with("m07/dreamer-lever-");
    let m49 = name.starts_with("m49-");
    let qm03 = name.starts_with("qm03/");
    let qm47 = name == "barbarian-test-mead-native";
    let m44_flower = name.starts_with("m44-lions-flower-0/");
    let m44_amphora = name == "m44-lions-amphora-1";
    let m44_lions = name.starts_with("m44-lions-strength/");
    let m44_children = name == "qn1-children-lever-1/ordinary";
    let m44 =
        name == "uneasy-alliance-crystal" || m44_children || m44_lions || m44_amphora || m44_flower;
    let qm48 = name == "qm48/mammoth";
    let m03_use = name == super::super::qengine_m03_use::NAME;
    let elemental_pin = name.starts_with("qn1-elemental-");
    let qm50 = name == "goblin-merchant-elevator-upper";
    let m03_pits = name == super::super::qengine_m03_pits::NAME;
    let nomads_window = name.starts_with(super::super::qengine_m02_use::NAME);
    let m01b_explore = name.starts_with("m01b/");
    let qready_craft = name == super::super::qready1_crafting::NAME;
    let qready_workbench = super::super::qready1_workbenches::matches(name);
    let small_ruby = name.starts_with("l6-small-ruby");
    let shared_gate = name.starts_with("l6-tomb-gate-");
    let shared_switch = name.starts_with("l6-tomb-switch-");
    let tomb_native = name.starts_with("l6-tomb-native-");
    let carried_step = name.starts_with("l6-tomb-carried-") || tomb_native;
    let mead = name == "l6/waterskin-of-mead";
    let spirit = name == "qengine-rewards-questpc4-spirit";
    let native_tower = name.starts_with("qengine-rewards-questpc4-native-tower-");
    let native_switch = name.starts_with("l6-tomb-switch-native-");
    let native_upper = name.starts_with("qengine-rewards-questpc4-paradox-upper-");
    let native_story = name.starts_with("qengine-rewards-questpc4-paradox-story-");
    let diprath_room = name.starts_with("l6-diprath-room-");
    let native_signs = name.starts_with("l6-native-signs-");
    let native_flames = name.starts_with("l6-native-flames-");
    let native_room = name.starts_with("l6-native-room-");
    let native_altar = name.starts_with("l6-floor-assembly-native");
    let native_gate = name.starts_with("l6-tomb-gate-native-");
    let last_creep = name == "qengine-rewards-m23-last-creep";
    let native_crop = m03_use
        || engenc_asura
        || m49
        || qm03
        || qm47
        || qm50
        || m44
        || m07_too_hot
        || last_creep
        || elemental_pin
        || m07_machinery
        || m07_realm
        || m07_lion
        || lion_trial
        || m07_dreamer
        || qready_workbench
        || qready_craft
        || m03_pits
        || nomads_window
        || qm48
        || m01b_explore
        || small_ruby
        || mead
        || spirit
        || tomb_native
        || native_tower
        || native_switch
        || native_gate
        || native_altar
        || native_room
        || native_flames
        || native_signs
        || diprath_room
        || native_story
        || native_upper;
    let start = if name == "qengine-rewards-smith-story" {
        Position {
            x: 32775,
            y: 32254,
            floor: -8,
        }
    } else if m03_use {
        Position {
            x: 32429,
            y: 32167,
            floor: -8,
        }
    } else if engenc_asura {
        super::super::engenc_asura_dispatch::start(name)?
    } else if m07_too_hot {
        super::super::m07_too_hot_columns::start()
    } else if m07_repair {
        super::super::m07_machinery_repairs::start()
    } else if m07_machinery {
        super::super::m07_machinery_burn::start(name)
    } else if m07_realm {
        super::super::m07_realm_trials::start()
    } else if lion_trial {
        super::super::m07_lion_offerings::start(match name {
            "lions-rock-beauty" => "m07/lion-beauty",
            "lions-rock-tears" => "m07/lion-tears",
            _ => "m07/lion-strength",
        })
    } else if m07_lion {
        super::super::m07_lion_offerings::start(name)
    } else if m07_dreamer {
        Position {
            x: 32762,
            y: 32289,
            floor: -14,
        }
    } else if m44_flower {
        Position {
            x: trigger.x + 1,
            ..trigger
        }
    } else if m44_amphora {
        Position {
            y: trigger.y - 1,
            ..trigger
        }
    } else if m03_pits {
        Position {
            x: 32791,
            y: 32332,
            floor: -10,
        }
    } else if nomads_window {
        Position {
            x: 33217,
            y: 32854,
            floor: -9,
        }
    } else if qready_workbench {
        super::super::qready1_workbenches::start(name)?
    } else if qready_craft {
        Position {
            x: 32011,
            y: 32907,
            floor: -7,
        }
    } else if qm50 {
        Position {
            x: 33051,
            y: 32098,
            floor: -6,
        }
    } else if name.starts_with("qengine-rewards-m39-tomes") {
        super::super::m39_tomes_journey::world::start()?
    } else if last_creep {
        super::super::m23_last_creep::start()
    } else if name == "unnatural-selection-dance-2" {
        let (x, y, floor) = oteryn_gameplay_quests::quest::unnatural_selection::TILES[7];
        Position { x, y, floor }
    } else if m49 {
        super::super::m49_elemental_arrival::start(name)?
    } else if qm03 || qm47 || m44_children {
        Position {
            x: trigger.x + 1,
            ..trigger
        }
    } else if m44_lions {
        Position {
            x: trigger.x - 1,
            ..trigger
        }
    } else if qm48 || m44 {
        Position {
            x: trigger.x,
            y: if m44 { trigger.y + 1 } else { trigger.y - 1 },
            ..trigger
        }
    } else if name == "engitem-native-seal-paint" {
        super::super::engitem_seal_paint::START
    } else if m01b_explore {
        super::super::m01b_explore::start(name, trigger)
    } else if native_upper {
        super::super::l6_paradox_upper::start(name)
    } else if native_story {
        super::super::l6_paradox_story::start()
    } else if diprath_room {
        super::super::l6_diprath_room::start()?
    } else if native_signs {
        super::super::l6_native_signs::start()?
    } else if native_flames {
        super::super::l6_native_flames::start()?
    } else if native_room {
        super::super::l6_native_rooms::start(name)?
    } else if native_altar {
        super::super::l6_floor_native::start()
    } else if native_gate {
        super::super::l6_shared_gate_native::start(name)?
    } else if native_tower {
        super::super::l6_paradox_native::start(name)
    } else if native_switch {
        super::super::l6_shared_switch_native::start(name)?
    } else if tomb_native {
        Position {
            x: trigger.x,
            y: trigger.y - 1,
            ..trigger
        }
    } else if spirit {
        Position {
            x: 33081,
            y: 31159,
            floor: -8,
        }
    } else if mead {
        Position {
            x: trigger.x,
            y: trigger.y + 1,
            ..trigger
        }
    } else {
        Position {
            x: trigger.x - 1,
            ..trigger
        }
    };
    let reward_fluids = name == "l6-reward-fluids";
    let ancient_instruments = name == "l6-ancient-instruments";
    let floor_profiles = name.starts_with("l6-treasure-reward-")
        || matches!(
            name,
            "l6-floor-profiles" | "l6-ruby-profile" | "l6-stag-item-profile"
        );
    let room_raise = name.starts_with("l6-room-raise-");
    let floor_assembly = name.starts_with("l6-floor-assembly");
    let reusable_tool = name.starts_with("l6-reusable-tool");
    let elvenbane = name == "elvenbane-native-goals";
    let monk = name.starts_with("monk-guidance");
    let feaster = matches!(
        name,
        "qengine-rewards-feaster-prefix" | "qengine-rewards-feaster-thaian"
    );
    let qengine_reward = name.starts_with("qengine-rewards-");
    let outfit_world = name.starts_with("l1-") && name.ends_with("-wardrobe");
    let shared_rewards = name == "questpc7-grave"
        || name == "questpc7-ferumbras"
        || name == "questpc7-hero"
        || name == "questpc7-hydra"
        || name == "questpc7-hoist-story"
        || name == "questpc7-tokens"
        || name.starts_with("questpc7-hidden-")
        || name.starts_with("questpc7-outfits-")
        || name == "l1-warmaster-wardrobe"
        || name.starts_with("questpc7-eligibility-")
        || name.starts_with("qengine-rewards-")
        || name == "questpc7-golden"
        || name == "questpc7-friends";
    let classic_npc = shared_rewards
        || name.starts_with("engoutfit-")
        || qengine_reward
        || name.starts_with("npcfix-m38-mistrock")
        || name == "npcfix-m38-minzy"
        || name == "npcfix-m38-rascalio"
        || matches!(
            name,
            "engnpc-m04"
                | "m05-gnomus"
                | "m16-marker"
                | "catalogue-npc"
                | "catalogue-npc-inventory"
                | "catalogue-npc-end"
                | "catalogue-npc-calendar"
                | "catalogue-npc-xp"
                | "nx6-wayfarer"
                | "npc-item-exchanges"
                | "m42-hot-typed"
                | "m42-hot-final"
                | "authored-mystery-intro"
                | "authored-mystery-assembly"
                | "authored-mystery-geometry"
                | "authored-static-constructors"
                | "authored-venore"
                | "authored-venore-supplies"
                | "authored-winterlight"
                | "authored-wrath-opening"
                | "authored-wrath-hunt"
                | "authored-wrath-crystals"
                | "authored-colours-opening"
                | "catalogue-npc-father"
                | "fathers-burden-handins"
                | "catalogue-npc-hero"
                | "catalogue-npc-legacy"
                | "catalogue-npc-scoped"
                | "questpc2-rotten-source-items"
                | "classic-quests-npc"
                | "questpc2-hero-pipes"
                | "questpc2-oldrak-npc"
                | "questpc2-henricus-start"
                | "questpc2-gerimor-assignments"
                | "questpc2-gnomus-ordnance"
                | "questpc2-albinius-four-tomes"
                | "questpc2-albinius-five-tomes"
                | "questpc2-albinius-combined"
                | "questpc2-albinius-portals-exact"
                | "questpc2-albinius-portals-combined"
                | "classic-rottin-disrepair"
                | "threatened-dreams-npc"
                | "conditional-quest-greetings"
        );
    let rotten_purchase = name == "rotten-blood-access-purchase";
    let target_swap = name == "fish-napping-key-target-swap";
    let sewer = name.starts_with("devil-helmet-sewer");
    let failure_landing = name.starts_with("conditional-failure-");
    let draconia_escape = name.starts_with("draconia-escape-");
    let mut callback = if m03_use {
        super::super::qengine_m03_use::callback()?
    } else if engenc_asura {
        super::super::engenc_asura_dispatch::callback(name)?
    } else if m03_pits {
        callback("qm03/pits")?
    } else if nomads_window {
        callback(super::super::qengine_m02_use::NAME)?
    } else if name == "unnatural-selection-dance-native" {
        callback("unnatural-selection-dance-2")?
    } else if name.starts_with("qengine-rewards-m39-bone") {
        callback("bone-pit-collect")?
    } else if last_creep {
        callback("last-creep-native-start")?
    } else if qm03 {
        super::super::qm03_world_admission::callback(name)?
    } else if draconia_escape {
        super::super::draconia_escape::authored_callback()?
    } else if elvenbane {
        callback("elvenbane-basement-entry")?
    } else if native_signs || diprath_room {
        callback("ancient-tombs-diprath-sign-1")?
    } else if native_flames {
        callback("ancient-tombs-morguthis-seven-flame-gate")?
    } else if native_room {
        super::super::l6_native_rooms::callback(name)?
    } else if feaster {
        callback("questpc7-feaster-entry")?
    } else if name == "qengine-rewards-forgotten-story" {
        callback("questpc7-forgotten-energy-entry")?
    } else if name == "qengine-rewards-heart-story" {
        callback("questpc7-heart-entry")?
    } else if name == "qengine-rewards-grimvale-story" {
        callback("questpc7-grimvale-entry")?
    } else if name == "questpc7-grave" {
        callback("questpc7-grave-entry")?
    } else if name == "questpc7-ferumbras" {
        callback("questpc7-ferumbras-entry")?
    } else if name == "questpc7-hero" {
        callback("questpc7-hero-entry")?
    } else if name == "questpc7-hydra" {
        callback("questpc7-hydra-entry")?
    } else if name == "questpc7-hoist-story" {
        callback("questpc7-hoist-entry")?
    } else if name.starts_with("questpc7-hidden-") {
        callback("questpc7-hidden-entry")?
    } else if name == "questpc7-outfits-formal" {
        callback("questpc7-formal-display")?
    } else if outfit_world {
        callback(name)?
    } else if classic_npc {
        callback("behemoth")?
    } else if rotten_purchase {
        callback("rotten-blood/sacrifice")?
    } else if monk {
        callback("monk-guidance")?
    } else if target_swap {
        callback("fish-napping-key")?
    } else {
        callback(if ring_offering {
            "dreamers-challenge-death-ring-reset"
        } else if carrot {
            "dreamers-challenge-carrot-entry"
        } else if m44_flower {
            "m44-lions-flower-0"
        } else if m44_lions || name == "lions-rock-strength-first" {
            "m07/lion-strength"
        } else if name == "lions-rock-beauty" {
            "m07/lion-beauty"
        } else if name == "lions-rock-tears" {
            "m07/lion-tears"
        } else if name == "triangle-tower-wall" {
            "triangle-tower-lever"
        } else if ancient_instruments {
            "ancient-tombs-diprath-sign-1"
        } else if classic_npc || small_ruby || reward_fluids || sewer {
            "behemoth"
        } else if failure_landing {
            "liquid-black-shortcut"
        } else if rotten_purchase {
            "rotten-blood/sacrifice"
        } else if monk {
            "monk-guidance"
        } else if m07_too_hot {
            name.strip_suffix("-level79").unwrap_or(name)
        } else if m07_machinery {
            name.strip_suffix("-level49").unwrap_or(name)
        } else if name == "giant-smithhammer-asymmetric" {
            "giant-smithhammer"
        } else if name == "authored-trader" {
            "authored-trader-package"
        } else if elemental_pin
            || target_swap
            || reusable_tool
            || floor_profiles
            || floor_assembly
            || m49
            || room_raise
            || carried_step
            || shared_switch
            || shared_gate
        {
            "fish-napping-key"
        } else if qm47 {
            "barbarian-test-mead"
        } else {
            name.strip_suffix("-expiry").unwrap_or(name)
        })?
    };
    if failure_landing {
        callback.selector.position = Some(trigger);
        callback.failure_teleport = Some(Position {
            x: 32818,
            y: 31599,
            floor: if name.ends_with("blocked") { -10 } else { -9 },
        });
    }
    if sewer {
        callback.key = format!("fixture:{name}");
        callback.selector.position = Some(trigger);
        callback.selector.unique = Some(if name.ends_with("unrelated") {
            25031
        } else {
            25030
        });
        callback.selector.action = None;
        callback.selector.appearance = Some(100);
        callback.lever = None;
        callback.storage.clear();
        callback.teleport = None;
        callback.offerings.clear();
        callback.passages = vec![oteryn_gameplay_quests::quest::world_callbacks::Passage {
            if_present: false,
            replacement_passable: None,
            position: Position {
                x: 32482,
                y: 32170,
                floor: -14,
            },
            appearance: 435,
            replacement: None,
            create_on_close: None,
            require_state: false,
        }];
    }
    if shared_gate {
        let unique = name.rsplit('-').next().ok_or("gate UID")?.parse()?;
        callback.key = "oteryn:quest-callback/l6-shared-gate-fixture".into();
        callback.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn;
        callback.selector.position = Some(trigger);
        callback.selector.unique = Some(unique);
        callback.selector.action = None;
        callback.selector.appearance = Some(100);
        callback.lever = None;
        callback.storage.clear();
        callback.offerings.clear();
        callback.passages.clear();
        callback.item_on_target = None;
        callback.effects = Default::default();
        callback.teleport = Some(super::super::l6_shared_gates::spec(unique)?.2);
        callback.return_on_failure = true;
    }
    if shared_switch {
        callback.key = "oteryn:quest-callback/l6-shared-switch-fixture".into();
        callback.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::Use;
        callback.selector.position = Some(trigger);
        callback.selector.unique = None;
        callback.selector.action = Some(name.rsplit('-').next().ok_or("switch action")?.parse()?);
        callback.selector.appearance = Some(2772);
        callback.lever = Some([2772, 2773]);
        callback.reversible = false;
        callback.storage.clear();
        callback.offerings.clear();
        callback.passages.clear();
        callback.item_on_target = None;
        callback.teleport = None;
        callback.effects = Default::default();
    }
    if carried_step {
        // Geometry supplies the exact source UID and both floor endpoints only.
        // Production callback selection and physical branch ownership are unchanged.
        let unique = name.rsplit('-').next().ok_or("tomb UID")?.parse::<u16>()?;
        callback.key = "oteryn:quest-callback/l6-carried-step-fixture".into();
        callback.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn;
        callback.selector.position = Some(trigger);
        callback.selector.unique = Some(unique);
        callback.selector.action = None;
        callback.selector.appearance = Some(100);
        callback.item_on_target = None;
        callback.storage.clear();
        callback.passages.clear();
        callback.offerings.clear();
        callback.lever = None;
        callback.effects = Default::default();
        callback.teleport = stateless_destination;
        if tomb_native {
            callback.key = "oteryn:quest-callback/l6-native-step-fixture".into();
        }
    }
    if name.starts_with("qengine-rewards-world-clock")
        || matches!(
            name,
            "qengine-rewards-authored-world-rewards"
                | "qengine-rewards-authored-world-rewards-full"
                | "qengine-rewards-authored-native-admission"
        )
    {
        callback = super::super::authored_world_rewards::placeholder(trigger)?;
    }
    if name.ends_with("-expiry") {
        callback
            .effects
            .portals
            .as_mut()
            .ok_or("portal pair")?
            .lifetime_us = Some(60_000_000);
    }
    if reusable_tool {
        callback = super::super::l6_reusable_tools::fixture(callback);
    }
    if floor_profiles || floor_assembly {
        callback.key = "oteryn:quest-callback/l6-floor-profile-fixture".into();
        callback.selector.unique = None;
        callback.selector.action = if name.ends_with("-unbound") {
            Some(40041)
        } else if floor_assembly {
            Some(40040)
        } else {
            None
        };
        callback.selector.appearance = Some(100);
        callback.selector.position = None;
        callback.item_on_target = None;
        callback.storage.clear();
        callback.teleport = None;
    }
    if ancient_instruments {
        // The fixture supplies source-verified native positions and appearances;
        // all matching and durable effects still come from the production registry.
        callback.key = "oteryn:quest-callback/l6-instrument-fixture".into();
        callback.selector.unique = None;
        callback.selector.position = Some(trigger);
        callback.selector.action = None;
        callback.selector.appearance = Some(3255);
    }
    if target_swap {
        callback
            .item_on_target
            .as_mut()
            .ok_or("item")?
            .target_replacement = Some(7777);
    }
    if lever_fixture {
        callback.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::Use;
        callback.selector.unique = Some(25012);
        callback.selector.appearance = Some(2772);
        callback.lever = Some([2772, 2773]);
    }
    if stateless_destination.is_some() {
        assert_eq!(callback.selector.position, Some(trigger));
    }
    if name.starts_with("qengine-rewards-questpc4-admission-") {
        callback = super::super::l6_world_admission::callback(name)?;
    }
    if name.starts_with("qengine-rewards-questpc4-puzzle-predicate") {
        callback = super::super::l6_world_puzzles::predicate_fixture(
            callback,
            trigger,
            name.ends_with("-refuse"),
        );
    }
    if name.ends_with("-srv7-m02") {
        callback = super::super::srv7_m02_ground_stack::fixture(callback)?;
    }
    if name == "qengine-rewards-questpc4-puzzle-presence" {
        callback = super::super::l6_world_puzzles::presence_fixture(callback, trigger);
    }
    if name == "qengine-rewards-questpc4-puzzle-ladder" {
        callback = super::super::l6_world_puzzles::ladder_fixture(callback, trigger);
    }
    if name.starts_with("qengine-rewards-qworld-phase-") {
        callback = super::super::qworld_phase_steps::geometry(callback, name)?;
    }
    if name == "authored-wrath-hunt" {
        super::super::questpc8_wrath_hunt::require_bindings()?;
    }
    if name == "nx6-wayfarer" {
        super::super::questpc8_wrath_source_dragon::require_bindings()?;
    }
    if name == "authored-wrath-crystals" {
        super::super::questpc8_wrath_crystals::require_bindings()?;
    }
    let world = if name == "authored-wrath-crystals" {
        super::super::questpc8_wrath_crystals::world()?
    } else if name == "nx6-wayfarer" {
        super::super::questpc8_wrath_source_dragon::world()?
    } else if name == "authored-wrath-hunt" {
        super::super::questpc8_wrath_hunt::world()?
    } else if matches!(name, "authored-wrath-opening" | "authored-colours-opening") {
        super::super::questpc8_wrath_opening::world()?
    } else if name == "authored-winterlight" {
        super::super::questpc8_winterlight::world()?
    } else if name == "authored-venore" {
        super::super::questpc8_venore::world()?
    } else if matches!(
        name,
        "authored-mystery-geometry" | "authored-mystery-assembly"
    ) {
        super::super::questpc8_mystery_geometry::world()?
    } else if name == "authored-secret-library" {
        super::super::questpc8_secret_library::world()?
    } else if name == "authored-treasure-island" {
        super::super::questpc8_treasure_island::world()?
    } else if name == "authored-trader" {
        super::super::questpc8_trader::world()?
    } else if name == "authored-anniversary" {
        super::super::questpc8_anniversary::world()?
    } else if name == "authored-magic-sword" {
        super::super::questpc8_magic_sword::world()?
    } else if name == "authored-torch" {
        super::super::questpc8_bridge_routes::torch_world()?
    } else if name == "authored-life-ring-entry" {
        super::super::questpc8_life_ring::world()?
    } else if name == "authored-life-ring-bridge" {
        super::super::questpc8_life_ring_bridge::world()?
    } else if let Some(route) = super::super::questpc8_retrieval_routes::route(name) {
        super::super::questpc8_retrieval_routes::world(route)?
    } else if name == "authored-braindeath" {
        super::super::quest_callbacks::source_world((32546, 31522, -11), 0, 1983, 0)?
    } else if name == "authored-kormarak" {
        super::super::quest_callbacks::source_world_rectangle_with_prefix(
            (32342, 32125, -10),
            &[(32342, 0)],
            4240,
            2,
            4,
            &[1854],
        )?
    } else if name == "authored-smith-cache" {
        super::super::quest_callbacks::source_world_rectangle(
            (32777, 32254, -8),
            &[(32777, 0)],
            2472,
            2,
        )?
    } else if name == "authored-tinder-box" {
        super::super::quest_callbacks::source_world((32085, 31025, -1), 0, 4285, 0)?
    } else if name == "mintwallin-cyclops-scene" {
        super::super::questpc8_native_scenes::world()?
    } else if name.starts_with("qengine-rewards-fibula-completion") {
        super::super::questpc6_fibula_completion::world()?
    } else if name == "qengine-rewards-fibula-native-goals" {
        let mut cells: Vec<_> = oteryn_gameplay_quests::quest::native_kill_goals::load()?
            .iter()
            .flat_map(|g| g.origins.iter())
            .map(|p| {
                i8::try_from(p.floor).map(|floor| Position {
                    x: p.x,
                    y: p.y,
                    floor: -floor,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        cells.push(Position {
            x: 32200,
            y: 32474,
            floor: -10,
        });
        super::world_with_cells(&callback, trigger, &cells)?
    } else if matches!(
        name,
        "qengine-rewards-goblin-stalkers" | "qengine-rewards-goblin-slug"
    ) {
        world_with_objects(
            &callback,
            trigger,
            &super::super::questpc7_goblin::stalker_objects(),
            &[],
        )?
    } else if name == "qengine-rewards-goblin-trunks" {
        world_with_objects(
            &callback,
            trigger,
            &super::super::questpc7_goblin::trunk_objects(),
            &[],
        )?
    } else if name == "qengine-rewards-goblin-pond" {
        world_with_objects(
            &callback,
            trigger,
            &super::super::questpc7_goblin::pond_objects(),
            &[],
        )?
    } else if name == "qengine-rewards-goblin-signs" {
        world_with_objects(
            &callback,
            trigger,
            &super::super::questpc7_goblin::sign_objects(),
            &[],
        )?
    } else if feaster {
        world_with_objects(
            &callback,
            trigger,
            &super::super::questpc7_feaster::objects()?,
            &[],
        )?
    } else if name == "qengine-rewards-forgotten-story" {
        world_with_objects(
            &callback,
            trigger,
            &super::super::questpc7_forgotten_story::objects()?,
            &[],
        )?
    } else if name == "qengine-rewards-heart-story" {
        world_with_objects(
            &callback,
            trigger,
            &super::super::questpc7_heart_story::objects()?,
            &[],
        )?
    } else if name == "qengine-rewards-grimvale-story" {
        world_with_objects(
            &callback,
            trigger,
            &super::super::questpc7_grimvale_story::objects()?,
            &[],
        )?
    } else if name == "questpc7-grave" {
        world_with_objects(
            &callback,
            trigger,
            &super::super::questpc7_grave::objects()?,
            &[],
        )?
    } else if name == "questpc7-ferumbras" {
        world_with_objects(
            &callback,
            trigger,
            &super::super::questpc7_ferumbras::objects(),
            &[],
        )?
    } else if name == "questpc7-hero" {
        world_with_objects(
            &callback,
            trigger,
            &[
                (
                    Position {
                        x: 33567,
                        y: 31950,
                        floor: -14,
                    },
                    20476,
                ),
                (
                    Position {
                        x: 33568,
                        y: 31951,
                        floor: -14,
                    },
                    20545,
                ),
                (
                    Position {
                        x: 33571,
                        y: 31952,
                        floor: -14,
                    },
                    1855,
                ),
            ],
            &[],
        )?
    } else if name == "questpc7-hydra" {
        world_with_objects(
            &callback,
            trigger,
            &[(
                Position {
                    x: 33004,
                    y: 32664,
                    floor: -3,
                },
                1063,
            )],
            &[],
        )?
    } else if name == "questpc7-hoist-story" {
        let mut objects = Vec::new();
        for x in 32153..=32157 {
            for y in 31116..=31125 {
                objects.push((Position { x, y, floor: -9 }, 7066));
            }
        }
        objects.extend([
            (
                Position {
                    x: 32154,
                    y: 31116,
                    floor: -9,
                },
                2472,
            ),
            (
                Position {
                    x: 32156,
                    y: 31125,
                    floor: -9,
                },
                5772,
            ),
            (
                Position {
                    x: 32157,
                    y: 31125,
                    floor: -9,
                },
                5770,
            ),
            (
                Position {
                    x: 32157,
                    y: 31125,
                    floor: -10,
                },
                5770,
            ),
            (
                Position {
                    x: 32156,
                    y: 31125,
                    floor: -10,
                },
                2773,
            ),
        ]);
        world_with_objects(&callback, trigger, &objects, &[2772])?
    } else if name == "questpc7-hidden-story" {
        world_with_objects(
            &callback,
            trigger,
            &[(
                Position {
                    x: 33080,
                    y: 32014,
                    floor: -13,
                },
                2469,
            )],
            &[],
        )?
    } else if name == "questpc7-outfits-formal" {
        // The fixture includes the same admitted entry appearance as the real palace.
        let mut physical = callback.clone();
        physical
            .passages
            .push(oteryn_gameplay_quests::quest::world_callbacks::Passage {
                if_present: false,
                replacement_passable: None,
                create_on_close: None,
                position: Position {
                    x: 32314,
                    y: 32170,
                    floor: -6,
                },
                appearance: 464,
                replacement: None,
                require_state: false,
            });
        world(&physical, trigger)?
    } else {
        world(&callback, trigger)?
    };
    run(async |admin| {
        let h = if shared_rewards
            || qengine_reward
            || outfit_world
            || matches!(
                name,
                "catalogue-npc" | "catalogue-npc-inventory" | "catalogue-npc-scoped"
            )
            || name == "srv7-reliable-ram"
            || name.starts_with("m09-treasure-")
            || name.starts_with("engoutfit-")
            || name == "engitem-targuna-selected-tool"
            || name == "engitem-native-seal-paint"
        {
            Harness::create_quest_reward(
                admin,
                "quest_rewards",
                !name.ends_with("pirate") && !name.contains("-male-"),
            )
            .await
        } else {
            Harness::create_with_admission_seed(
                admin,
                "quest_world_callbacks",
                m07_realm || (m07_machinery && !m07_machinery_level49),
                false,
            )
            .await
        }
        .map_err(|e| format!("callback harness: {e}"))?;
        if name == "qengine-rewards-goblin-signs" {
            let version: String = sqlx::query_scalar("SHOW server_version")
                .fetch_one(&h.pool)
                .await?;
            eprintln!("QUESTPC7_GOBLIN_SIGNS POSTGRES_STARTED version={version}");
        }

        if name == "mintwallin-cyclops-scene" {
            eprintln!(
                "POSTGRES_CASE_STARTED mintwallin-cyclops-scene after PostgreSQL fixture initialization"
            );
        }
        if name.starts_with("authored-") {
            eprintln!("POSTGRES_CASE_STARTED {name} after PostgreSQL fixture initialization");
        }
        if m07_machinery_level49 || m07_too_hot {
            // Seed a separate historical character, as the shared harness does;
            // never bypass the progression owner's typed successor guard.
            sqlx::query(
                "INSERT INTO game_character_progression_state VALUES \
                (encode($1,'hex')::uuid,1,$2,999,'profile-1','ruleset-1','content-1',\
                 'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
            )
            .bind(id(41).as_slice())
            .bind(if m07_too_hot {
                if name.ends_with("-level79") {
                    79_i64
                } else {
                    80_i64
                }
            } else {
                49_i64
            })
            .execute(&h.pool)
            .await?;
        }
        if small_ruby || carried_step || native_tower || name == "triangle-tower-wall" {
            let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
                .fetch_one(&h.pool)
                .await?;
            if native_tower {
                eprintln!("POSTGRES_DATABASE_READY {name} native_scene_pending version={version}");
            } else if carried_step {
                eprintln!("POSTGRES_DATABASE_READY l6_carried_step version={version}");
            } else if name == "triangle-tower-wall" {
                eprintln!("POSTGRES_DATABASE_READY triangle_tower_wall version={version}");
            } else {
                eprintln!("POSTGRES_DATABASE_READY l6_small_ruby_native_pickup version={version}");
            }
        }
        prepare_current_session(&h)
            .await
            .map_err(|e| format!("callback session: {e}"))?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|e| format!("callback authority: {e:?}"))?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let world = if m03_use {
            super::super::qengine_m03_use::world(
                generation
                    .native_gameplay()
                    .ok_or("machine native policies")?,
            )?
        } else if engenc_asura {
            super::super::engenc_asura_dispatch::world(
                generation.native_gameplay().ok_or("native Asura")?,
                name,
            )?
        } else if m03_pits {
            super::super::qengine_m03_pits::world(
                generation.native_gameplay().ok_or("Pits native policies")?,
            )?
        } else if m44_flower {
            super::super::m44_flower_lifecycle::world(
                generation.native_gameplay().ok_or("flower policies")?,
            )?
        } else if m44_amphora {
            super::super::m44_amphora_lifecycle::world(
                generation.native_gameplay().ok_or("amphora policies")?,
            )?
        } else if nomads_window {
            super::super::qengine_m02_use::world(
                generation
                    .native_gameplay()
                    .ok_or("Nomads native policies")?,
            )?
        } else if elemental_pin {
            super::super::qn1_elemental_world_pin::world(
                generation.native_gameplay().ok_or("elemental policies")?,
                name.starts_with("qn1-elemental-energy-"),
            )?
        } else if qready_workbench {
            super::super::qready1_workbenches::world(
                generation
                    .native_gameplay()
                    .ok_or("workbench native policies")?,
                name,
            )?
        } else if qready_craft {
            super::super::qready1_crafting::world(
                generation
                    .native_gameplay()
                    .ok_or("grinder native policies")?,
            )?
        } else if name.starts_with("qengine-rewards-m39-tomes") {
            super::super::m39_tomes_journey::world::build(
                generation
                    .native_gameplay()
                    .ok_or("Tomes native profiles")?,
            )?
        } else if last_creep {
            super::super::m23_last_creep::world(
                generation.native_gameplay().ok_or("Last Creep native")?,
            )?
        } else if m49 {
            super::super::m49_elemental_arrival::world(
                generation.native_gameplay().ok_or("Elemental policies")?,
                name,
            )?
        } else if qm03 {
            super::super::qm03_world_admission::world(
                name,
                generation.native_gameplay().ok_or("M03 native policies")?,
            )?
        } else if qm47 {
            super::super::qm47_mead_dispatch::world(
                generation.native_gameplay().ok_or("mead policies")?,
            )?
        } else if qm50 {
            super::super::qm50_elevator::world(
                generation.native_gameplay().ok_or("elevator policies")?,
            )?
        } else if m44_lions {
            super::super::m44_lions_lifecycle::world(
                generation.native_gameplay().ok_or("lions policies")?,
            )?
        } else if m44_children {
            super::super::m44_children_lifecycle::world(
                generation.native_gameplay().ok_or("children policies")?,
            )?
        } else if m44 {
            super::super::m44_source_lifecycle::world(
                generation.native_gameplay().ok_or("source policies")?,
            )?
        } else if qm48 {
            super::super::qm48_mammoth::world(
                generation.native_gameplay().ok_or("mammoth policies")?,
            )?
        } else if m01b_explore {
            super::super::m01b_explore::world(
                generation
                    .native_gameplay()
                    .ok_or("native explore policies")?,
                name,
            )?
        } else if native_upper {
            super::super::l6_paradox_upper::world(
                generation
                    .native_gameplay()
                    .ok_or("native upper policies")?,
                name,
            )?
        } else if native_story {
            super::super::l6_paradox_story::world(
                generation
                    .native_gameplay()
                    .ok_or("native story policies")?,
            )?
        } else if diprath_room {
            super::super::l6_diprath_room::world(
                generation
                    .native_gameplay()
                    .ok_or("Diprath native policies")?,
            )?
        } else if native_signs {
            super::super::l6_native_signs::world(
                generation.native_gameplay().ok_or("native sign policies")?,
            )?
        } else if native_flames {
            super::super::l6_native_flames::world(
                generation
                    .native_gameplay()
                    .ok_or("native flame policies")?,
            )?
        } else if native_room {
            super::super::l6_native_rooms::world(
                generation.native_gameplay().ok_or("native room policies")?,
                name,
            )?
        } else if native_altar {
            super::super::l6_floor_native::world(
                generation
                    .native_gameplay()
                    .ok_or("native altar policies")?,
            )?
        } else if native_gate {
            super::super::l6_shared_gate_native::world(
                generation.native_gameplay().ok_or("native gate policies")?,
                name,
            )?
        } else if native_tower {
            super::super::l6_paradox_native::world(
                generation
                    .native_gameplay()
                    .ok_or("native tower policies")?,
                name,
            )?
        } else if native_switch {
            super::super::l6_shared_switch_native::world(
                generation
                    .native_gameplay()
                    .ok_or("native switch policies")?,
                name,
            )?
        } else if tomb_native {
            super::super::l6_tomb_native::world(
                generation.native_gameplay().ok_or("native Tomb policies")?,
                callback.selector.unique.ok_or("source UID")?,
            )?
        } else if small_ruby {
            super::super::l6_world_pickups::small_ruby(
                generation
                    .native_gameplay()
                    .ok_or("native pickup policies")?,
            )?
        } else if spirit {
            super::super::l6_spirit::world(
                generation
                    .native_gameplay()
                    .ok_or("native Spirit policies")?,
            )?
        } else if m07_too_hot {
            super::super::m07_too_hot_columns::world(
                generation.native_gameplay().ok_or("Too Hot policies")?,
            )?
        } else if m07_repair {
            super::super::m07_machinery_repairs::world(
                generation
                    .native_gameplay()
                    .ok_or("Machinery repair policies")?,
            )?
        } else if m07_machinery {
            super::super::m07_machinery_burn::world(
                generation.native_gameplay().ok_or("Machinery policies")?,
                name,
            )?
        } else if m07_realm {
            super::super::m07_realm_trials::world(
                generation.native_gameplay().ok_or("Realm policies")?,
            )?
        } else if m07_lion || lion_trial {
            super::super::m07_lion_offerings::world(
                generation.native_gameplay().ok_or("native Lion policies")?,
                callback
                    .key
                    .strip_prefix("oteryn:quest-callback/")
                    .ok_or("Lion callback key")?,
            )?
        } else if m07_dreamer {
            super::super::m07_dreamer_levers::world(
                generation
                    .native_gameplay()
                    .ok_or("native Dreamer policies")?,
            )?
        } else if mead {
            super::super::l6_mead::world(
                generation.native_gameplay().ok_or("native Mead policies")?,
            )?
        } else {
            world
        };
        let world = if elvenbane {
            super::super::source_world_crop::compiled_region(
                crate::map::overlay::TilePos {
                    x: 32584,
                    y: 31638,
                    floor: -8,
                },
                crate::map::overlay::TilePos {
                    x: 32599,
                    y: 31653,
                    floor: -3,
                },
                crate::map::overlay::TilePos {
                    x: trigger.x,
                    y: trigger.y,
                    floor: trigger.floor,
                },
                generation.native_gameplay().ok_or("native")?,
            )?
        } else {
            world
        };
        let world = if name == "qengine-rewards-smith-story" {
            world_with_native_items(
                &callback,
                trigger,
                &super::super::questpc7_smith::terrain(),
                generation.native_gameplay().ok_or("native")?,
                &super::super::questpc7_smith::items(),
            )?
        } else if name == "qengine-rewards-hot-corpses" {
            world_with_objects(
                &callback,
                trigger,
                &super::super::questpc7_hot_corpses::terrain(),
                &[],
            )?
        } else if matches!(
            name,
            "qengine-rewards-goblin-stalkers" | "qengine-rewards-goblin-slug"
        ) {
            world_with_objects(
                &callback,
                trigger,
                &super::super::questpc7_goblin::stalker_objects(),
                &[],
            )?
        } else if name == "qengine-rewards-goblin-trunks" {
            world_with_native_items(
                &callback,
                trigger,
                &super::super::questpc7_goblin::trunk_objects(),
                generation.native_gameplay().ok_or("native")?,
                &super::super::questpc7_goblin::trunk_items(),
            )?
        } else if name == "qengine-rewards-goblin-pond" {
            let p = Position {
                x: 33134,
                y: 32093,
                floor: -10,
            };
            world_with_native_items(
                &callback,
                trigger,
                &super::super::questpc7_goblin::pond_objects(),
                generation.native_gameplay().ok_or("native")?,
                &[(p, 9527)],
            )?
        } else if name == "qengine-rewards-goblin-signs" {
            world_with_native_items(
                &callback,
                trigger,
                &super::super::questpc7_goblin::sign_objects(),
                generation.native_gameplay().ok_or("native")?,
                &super::super::questpc7_goblin::sign_native_items(),
            )?
        } else if feaster {
            world_with_native_items(
                &callback,
                trigger,
                &super::super::questpc7_feaster::objects()?,
                generation.native_gameplay().ok_or("native")?,
                &super::super::questpc7_feaster::native_items()?,
            )?
        } else if name == "qengine-rewards-forgotten-story" {
            world_with_native_items(
                &callback,
                trigger,
                &super::super::questpc7_forgotten_story::objects()?,
                generation.native_gameplay().ok_or("native")?,
                &super::super::questpc7_forgotten_story::native_items(),
            )?
        } else if name == "qengine-rewards-heart-story" {
            world_with_native_items(
                &callback,
                trigger,
                &super::super::questpc7_heart_story::objects()?,
                generation.native_gameplay().ok_or("native")?,
                &super::super::questpc7_heart_story::native_items(),
            )?
        } else if name == "questpc7-hydra" {
            world_with_native_items(
                &callback,
                trigger,
                &[(
                    Position {
                        x: 33004,
                        y: 32664,
                        floor: -3,
                    },
                    1063,
                )],
                generation.native_gameplay().ok_or("native")?,
                &[(
                    Position {
                        x: 33004,
                        y: 32664,
                        floor: -3,
                    },
                    4839,
                )],
            )?
        } else {
            world
        };

        let initial_level: Option<u32> = match name {
            "catalogue-npc-calendar" | "catalogue-npc-scoped" => Some(35),
            "questpc7-eligibility-high" => Some(1000),
            "qengine-rewards-feaster-prefix" | "qengine-rewards-feaster-thaian" => Some(250),
            "questpc7-ferumbras" | "qengine-rewards-heart-story" => Some(150),
            "questpc7-grave" => Some(250),
            "questpc7-outfits-formal" => Some(20),
            "questpc7-outfits-fiend" | "questpc7-outfits-inquisition" => Some(250),
            _ => None,
        };
        if let Some(initial_level) = initial_level {
            // The subject starts at the published native prerequisite level;
            // no subject quest progress is arranged by this fixture.
            let policy = generation
                .native_gameplay()
                .and_then(|n| n.progression())
                .ok_or("native progression")?
                .policy_for("profile-1", "ruleset-1", "content-1")
                .map_err(debug)?;
            let binding =
                crate::gameplay_transport::SessionProgressionBinding::from_content(policy);
            let xp = binding
                .policy
                .thresholds
                .iter()
                .find(|t| t.level == initial_level)
                .ok_or("native initial level threshold")?
                .minimum_experience
                .get();
            let inserted = sqlx::query("INSERT INTO game_character_progression_state VALUES(encode($1,'hex')::uuid,1,$2,$3,'profile-1','ruleset-1','content-1',$4,$5,$6,$7,$8)")
                .bind(id(41).as_slice()).bind(i64::from(initial_level)).bind(xp).bind(&binding.context.simulation).bind(&binding.context.evidence)
                .bind(&binding.context.declaration).bind(&binding.policy_revision).bind(&binding.reward_revision).execute(&h.pool).await?;
            assert_eq!(inserted.rows_affected(), 1);
        }

        let world = if name.starts_with("qengine-rewards-l2-djinn-") {
            super::super::questl2_djinn_world::world()?
        } else if name == "qengine-rewards-l2-white-raven" {
            super::super::questl2_white_raven_world::world(
                generation.native_gameplay().ok_or("White Raven policies")?,
            )?
        } else {
            world
        };
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let frame_binding = if native_crop || elemental_pin {
            world.frame_binding_digest(deps.admitted.world_id)
        } else {
            activated.frame_binding().digest()
        };
        let bundle_room = Some(
            activated
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    frame_binding,
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: i32::from(start.x),
                        y: i32::from(start.y),
                        floor: -i16::from(trigger.floor),
                    },
                )
                .ok_or("qualified native Bundle room")?,
        );
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame_binding,
            world.map_revision_digest(),
            (
                i32::from(start.x),
                i32::from(start.y),
                -i16::from(trigger.floor),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            if m07_realm || name == "qengine-rewards-questpc4-native-tower-third" {
                4_104 // Quest callbacks must tolerate >4096 unrelated live creatures.
            } else if name == "qengine-rewards-qnpcfx-m10-inquisition" {
                7 // One player and the six source NPCs for the full interview story.
            } else if qm50 {
                8
            } else if name.starts_with("qengine-rewards-l12-") {
                128
            } else if name == "qengine-rewards-grimvale-story" {
                // Player, two retained afflicted guardians, two contacts, one active hunt.
                6
            } else if name == "questpc7-friends" {
                // Thirteen selected vendors/priests and one player; charge every actor slot.
                14
            } else if native_story
                || (native_upper && name.ends_with("-complete"))
                || matches!(name, "catalogue-npc-hero" | "catalogue-npc-scoped")
            {
                6
            } else if name == "authored-magic-sword" {
                9
            } else if name == "authored-trader" {
                7
            } else if name == "authored-treasure-island" {
                3
            } else if name == "authored-secret-library" {
                5
            } else if matches!(name, "authored-anniversary" | "authored-venore") {
                12
            } else if name == "authored-torch" {
                3
            } else if name.starts_with("qengine-rewards-l2-djinn-") {
                13
            } else if classic_npc {
                5
            } else if monk {
                4
            } else if callback.effects.encounter.is_some() {
                16
            } else {
                2
            },
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let origin = local(start);
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let premium = if name.starts_with("authored-")
            || name == "nx6-wayfarer"
            || name == "m07-carrot-premium"
            || m03_use
            || engenc_asura
            || m03_pits
            || (m01b_explore && callback.admission.as_ref().is_some_and(|a| a.premium))
            || matches!(
                name,
                "npc-item-exchanges" | "m42-hot-typed" | "m42-hot-final"
            )
            || name.contains("l6-beggar-")
            || name == "npcfix-m38-mistrock"
            || name == "qengine-rewards-m01a-sven"
            || (qm03 && name.ends_with("-premium"))
            || name.contains("l7-knight-")
            || name.contains("l8-")
            || name.contains("l9-")
            || name.contains("l10-documents-")
            || name.contains("l11-")
            || name.contains("l12-")
            || name.starts_with("qengine-rewards-qworld-phase-")
            || spirit
            || name.starts_with("qengine-rewards-questpc4-paradox")
            || name.contains("l13-")
        {
            use crate::gameplay_transport::spell_premium_coordinator::{
                SpellPremiumCoordinator, purchase_fixture,
            };
            let (consumer, clock, activation, _) =
                purchase_fixture::current(super::super::fixture::id(40));
            h.root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
            Some(SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            ))
        } else {
            None
        };
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        if mead || m07_lion || lion_trial || m44_lions || m07_realm || m07_too_hot {
            h.root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
        }
        let mead_premium =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            );
        let eligibility_premium = if feaster
            || name == "catalogue-npc-inventory"
            || matches!(
                name,
                "catalogue-npc-end" | "catalogue-npc-calendar" | "catalogue-npc-scoped"
            )
            || name == "qengine-rewards-forgotten-story"
            || name == "qengine-rewards-heart-story"
            || name == "qengine-rewards-grimvale-story"
            || name == "questpc7-grave"
            || name == "questpc7-ferumbras"
            || name == "questpc7-hero"
            || name == "questpc7-hydra"
            || name == "questpc7-hoist-story"
            || (name.starts_with("questpc7-eligibility-") && !name.ends_with("no-premium"))
            || name.starts_with("questpc7-outfits-")
        {
            use crate::gameplay_transport::spell_premium_coordinator::{
                SpellPremiumCoordinator, purchase_fixture,
            };
            let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
            Some(SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            ))
        } else {
            None
        };
        let hydra_reset_drain =
            crate::gameplay_transport::gameplay_drain::GameplayDrain::at_reset_epoch(1);
        let mut owner = deps.owner(&h, &authority);
        owner.premium_coordinator = premium.as_ref();
        if mead || m07_lion || lion_trial || m44_lions || m07_realm || m07_too_hot {
            owner.premium_coordinator = Some(&mead_premium);
        }
        if let Some(premium) = &eligibility_premium {
            owner.premium_coordinator = Some(premium);
            owner.admit_premium(id(40), deps.admitted.game_session_id);
            assert!(
                owner
                    .session_premium_gate(deps.admitted.game_session_id)
                    .await
                    .premium_current()
            );
        }

        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        if let Some(room) = bundle_room.as_ref() {
            owner.rooms =
                crate::gameplay_transport::SeamRooms::new(native_crop.then_some(room), Some(room));
        }
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        if draconia_escape {
            owner.quest_catalogue =
                Some(super::super::draconia_escape::catalogue(&callback, name)?);
        }
        if engenc_asura {
            owner.quest_catalogue = Some(std::sync::Arc::new(
                super::super::engenc_asura_dispatch::catalogue(name)?,
            ));
        }
        if m44_flower {
            owner.quest_catalogue = Some(std::sync::Arc::new(
                super::super::m44_flower_lifecycle::catalogue(
                    owner.quest_catalogue.as_ref().ok_or("flower catalogue")?,
                    &callback,
                    name.ends_with("/later"),
                )?,
            ));
        } else if m44_amphora {
            owner.quest_catalogue = Some(std::sync::Arc::new(
                super::super::m44_source_lifecycle::catalogue(
                    owner.quest_catalogue.as_ref().ok_or("amphora catalogue")?,
                    &callback,
                )?,
            ));
        } else if m44_lions {
            owner.quest_catalogue = Some(std::sync::Arc::new(
                super::super::m44_lions_lifecycle::catalogue(
                    owner.quest_catalogue.as_ref().ok_or("lions catalogue")?,
                    &callback,
                    name.ends_with("/later"),
                )?,
            ));
        } else if m44 {
            owner.quest_catalogue = Some(std::sync::Arc::new(
                super::super::m44_source_lifecycle::catalogue(
                    owner.quest_catalogue.as_ref().ok_or("source catalogue")?,
                    &callback,
                )?,
            ));
        }
        if qm48 {
            owner.quest_catalogue =
                Some(std::sync::Arc::new(super::super::qm48_mammoth::catalogue(
                    owner.quest_catalogue.as_ref().ok_or("mammoth catalogue")?,
                    &callback,
                )?));
        }
        if mead {
            owner.quest_catalogue = Some(std::sync::Arc::new(super::super::l6_mead::catalogue(
                owner.quest_catalogue.as_ref().ok_or("Mead catalogue")?,
                &callback,
            )?));
        }
        if name.starts_with("qengine-rewards-qworld-phase-") {
            owner.quest_catalogue = Some(std::sync::Arc::new(
                super::super::qworld_phase_steps::catalogue(
                    owner.quest_catalogue.as_ref().ok_or("catalogue")?,
                )?,
            ));
        }
        if name.starts_with("qengine-rewards-questpc4-admission-")
            || name.starts_with("qengine-rewards-questpc4-puzzle-predicate")
            || lever_fixture
            || target_swap
            || failure_landing
            || name.ends_with("-expiry")
            || name == "triangle-tower-lever"
        {
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![callback.clone()])
                    .is_ok()
            );
        }
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        let activation_frame_binding = if elemental_pin {
            activated.frame_binding().digest()
        } else {
            frame_binding
        };
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activation_frame_binding.as_slice()).execute(&h.pool).await?;
        let fresh = super::super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(|e| format!("callback admission equipment: {e:?}"))?;
        let native = generation.native_gameplay().ok_or("native")?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &template,
            )
            .await
            .map_err(|e| format!("callback starter items: {e:?}"))?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        if let Some(initial_level) = initial_level {
            // Initialization uses common cast fixtures; project the actually seeded
            // durable level before evaluating source NPC eligibility.
            owner.states.level_refresh.mark([fence.character_id]);
            Box::pin(owner.refresh_level_up_vitals(deps.actor, admitted.game_session_id)).await;
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert_eq!(
                states
                    .get(&runtime, deps.actor, admitted.game_session_id)
                    .ok_or("seeded live character facts")?
                    .character_facts()
                    .level,
                initial_level,
                "source eligibility uses the actual durable starting level"
            );
        }
        if name == "nx6-wayfarer"
            || qm03
            || m07_too_hot
            || mead
            || (m01b_explore && callback.admission.as_ref().is_some_and(|a| a.premium))
            || m07_lion
            || lion_trial
            || m44_lions
            || m07_realm
            || name == "npcfix-m38-mistrock"
        {
            owner.admit_premium(id(40), admitted.game_session_id);
        }
        owner.admit_quest_session(&admitted).await;
        if m03_use {
            Box::pin(super::super::qengine_m03_use::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if engenc_asura {
            Box::pin(super::super::engenc_asura_dispatch::exercise(
                &mut owner,
                &mut admitted,
                &h,
                name,
            ))
            .await?;
        } else if m03_pits {
            Box::pin(super::super::qengine_m03_pits::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if nomads_window {
            Box::pin(super::super::qengine_m02_use::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &callback,
                name.ends_with("-wrong-order"),
            ))
            .await?;
        } else if elemental_pin {
            Box::pin(super::super::qn1_elemental_world_pin::exercise(
                &owner,
                &h,
                name,
                deps.actor,
                fence.item_fence(),
                admitted,
            ))
            .await?;
        } else if qready_workbench {
            Box::pin(super::super::qready1_workbenches::exercise(
                &owner,
                &mut admitted,
                &h,
                &callback,
                name,
            ))
            .await?;
        } else if qready_craft {
            Box::pin(super::super::qready1_crafting::exercise(
                &owner,
                &mut admitted,
                &h,
                &callback,
            ))
            .await?;
        } else if matches!(
            name,
            "qready1-bewitched-contribution" | "qready1-threatened-raven-herb"
        ) {
            Box::pin(super::super::qready1_world_clocks::exercise(
                &owner,
                &mut admitted,
                &h,
                &callback,
            ))
            .await?;
        } else if qm47 {
            Box::pin(super::super::qm47_mead_dispatch::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
                &callback,
            ))
            .await?;
        } else if name == "barbarian-test-mead" {
            Box::pin(super::super::qm47_mead::exercise(
                &mut owner, &admitted, &h, fence, &callback,
            ))
            .await?;
        } else if qm03 {
            Box::pin(super::super::qm03_world_admission::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &callback,
                name,
            ))
            .await?;
        } else if qm50 {
            Box::pin(super::super::qm50_elevator::exercise(&owner, &mut admitted)).await?;
        } else if m01b_explore {
            Box::pin(super::super::m01b_explore::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &callback,
            ))
            .await?;
        } else if m07_too_hot {
            Box::pin(super::super::m07_too_hot_columns::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
                &callback,
            ))
            .await?;
        } else if m07_repair {
            Box::pin(super::super::m07_machinery_repairs::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
                &callback,
            ))
            .await?;
        } else if m07_machinery {
            Box::pin(super::super::m07_machinery_burn::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
                &callback,
            ))
            .await?;
        } else if m07_realm {
            Box::pin(super::super::m07_realm_trials::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
                &callback,
            ))
            .await?;
        } else if m07_lion {
            Box::pin(super::super::m07_lion_offerings::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
                &callback,
            ))
            .await?;
        } else if m07_dreamer {
            Box::pin(super::super::m07_dreamer_levers::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &callback,
            ))
            .await?;
        } else if draconia_escape {
            super::super::draconia_escape::exercise(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                name,
            )
            .await?;
        } else if failure_landing {
            super::super::conditional_callback_failure::exercise(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                if name.ends_with("blocked") {
                    trigger
                } else {
                    callback.failure_teleport.ok_or("failure landing")?
                },
            )
            .await?;
        } else if sewer {
            super::super::pressure_plate_sewer::exercise(
                &mut owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                trigger,
                name,
            )
            .await?;
        } else if elvenbane {
            super::super::elvenbane_native_goals::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "questpc2-rotten-source-items" {
            super::super::questpc2_rotten_source_items::exercise(&mut owner, &mut admitted, &h)
                .await?;
        } else if mead {
            Box::pin(super::super::l6_mead::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
                &callback,
            ))
            .await?;
        } else if reward_fluids {
            Box::pin(super::super::l6_reward_fluids::exercise(
                &owner, admitted, &h, &authority,
            ))
            .await?;
        } else if small_ruby {
            Box::pin(super::super::l6_world_pickups::exercise(
                &owner,
                &mut admitted,
                &h,
                0,
            ))
            .await?;
            if name.ends_with("-reset") {
                let mut next = super::super::l6_world_pickups::cycle(
                    &owner,
                    admitted,
                    &h,
                    &authority,
                    bundle_room.as_ref().ok_or("native reset room")?,
                )
                .await?;
                drop(owner);
                let reset_world = super::super::l6_world_pickups::small_ruby(
                    generation
                        .native_gameplay()
                        .ok_or("native pickup policies")?,
                )?;
                let reset_cells = reset_world
                    .movement_cells(activated.qualified_room().movement_cells())
                    .map_err(debug)?;
                let reset_frame = reset_world.frame_binding_digest(deps.admitted.world_id);
                let reset_room = activated
                    .qualified_room()
                    .bundle_view(
                        reset_cells.clone(),
                        reset_frame,
                        reset_world.map_revision_digest(),
                        crate::content::NativeEntryStart {
                            x: origin.x,
                            y: origin.y,
                            floor: origin.floor,
                        },
                    )
                    .ok_or("reset Bundle room")?;
                let reset_pin = ChannelContentPin::from_activation(
                    deps.admitted.world_id,
                    2,
                    generation.identity().server_artifact_digest(),
                    generation.identity().client_artifact_digest(),
                    reset_frame,
                    reset_world.map_revision_digest(),
                    (origin.x, origin.y, origin.floor),
                );
                let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                    deps.admitted.world_id,
                    deps.admitted.channel_id,
                    crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                    1,
                    2,
                    2,
                    "runtime-scope-assignment:2",
                    2,
                    reset_pin,
                )
                .map_err(debug)?;
                let reserved = runtime
                    .reserve_fresh_session(next.game_session_id)
                    .map_err(debug)?;
                deps.actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
                runtime
                    .initialize_pinned_test_position(deps.actor, origin)
                    .map_err(debug)?;
                next.runtime_actor = Some(deps.actor);
                deps.admitted = next;
                deps.room = reset_room.clone();
                *deps.runtime.lock().await = runtime;
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(generation);
                owner.movement_cells = (&reset_cells).into();
                owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&reset_room));
                owner.reset_epoch = h
                    .root
                    .active_world_reset_epoch(owner.world_id)
                    .await?
                    .ok_or("active reset epoch")?;
                deps.initialize(&owner).await;
                owner.admit_progression_session(&mut next).await;
                Box::pin(super::super::l6_world_pickups::exercise(
                    &owner, &mut next, &h, 1,
                ))
                .await?;
                super::super::l6_world_pickups::refuse_old_epoch(&owner, next, &h).await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
        } else if let Some(source_id) = super::super::l6_treasure_rewards::source_id(name) {
            Box::pin(super::super::l6_treasure_rewards::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
                source_id,
            ))
            .await?;
        } else if m49 {
            Box::pin(super::super::m49_elemental_arrival::exercise(
                &mut owner,
                &mut admitted,
                &h,
                name,
                deps.actor,
            ))
            .await?;
        } else if m44_flower {
            Box::pin(super::super::m44_flower_lifecycle::exercise(
                &owner,
                &mut admitted,
                &h,
                name.ends_with("/later"),
                name.ends_with("/uncertain"),
            ))
            .await?;
        } else if m44_amphora {
            Box::pin(super::super::m44_amphora_lifecycle::exercise(
                &owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if m44_lions {
            Box::pin(super::super::m44_lions_lifecycle::exercise(
                &owner,
                &mut admitted,
                &h,
                fence,
                &callback,
                name.ends_with("-uncertain"),
                name.ends_with("/later"),
            ))
            .await?;
        } else if m44_children {
            Box::pin(super::super::m44_children_lifecycle::exercise(
                &owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if m44 {
            Box::pin(super::super::m44_source_lifecycle::exercise(
                &owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if qm48 {
            Box::pin(super::super::qm48_mammoth::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "l6-stag-item-profile" {
            Box::pin(super::super::l6_stag_item::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
            ))
            .await?;
        } else if name == "l6-ruby-profile" {
            Box::pin(super::super::l6_ruby_profile::exercise(&owner, &h)).await?;
        } else if floor_profiles {
            Box::pin(super::super::l6_floor_profiles::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
            ))
            .await?;
        } else if diprath_room {
            Box::pin(super::super::l6_diprath_room::exercise(
                &mut owner,
                &mut admitted,
                &h,
                name,
            ))
            .await?;
        } else if native_signs {
            Box::pin(super::super::l6_native_signs::exercise(
                &mut owner,
                &mut admitted,
                &h,
                name,
            ))
            .await?;
        } else if native_flames {
            Box::pin(super::super::l6_native_flames::exercise(
                &mut owner,
                &mut admitted,
                &h,
                name,
            ))
            .await?;
        } else if native_room {
            Box::pin(super::super::l6_native_rooms::exercise(
                &mut owner,
                &mut admitted,
                &h,
                name,
            ))
            .await?;
        } else if room_raise {
            Box::pin(super::super::l6_room_raise::exercise(
                &mut owner,
                &mut admitted,
                &h,
                name,
            ))
            .await?;
        } else if native_altar {
            Box::pin(super::super::l6_floor_native::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
                name,
            ))
            .await?;
        } else if floor_assembly {
            Box::pin(super::super::l6_floor_assembly::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
                name,
            ))
            .await?;
        } else if native_gate {
            Box::pin(super::super::l6_shared_gate_native::exercise(
                &mut owner,
                &mut admitted,
                &h,
                name,
            ))
            .await?;
        } else if shared_gate {
            Box::pin(super::super::l6_shared_gates::exercise(
                &mut owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                trigger,
                name,
            ))
            .await?;
        } else if native_switch {
            Box::pin(super::super::l6_shared_switch_native::exercise(
                &owner,
                &mut admitted,
                &h,
                trigger,
                name,
            ))
            .await?;
        } else if shared_switch && name.contains("-treasure-") {
            Box::pin(super::super::l6_shared_switch_treasure::exercise(
                &mut owner,
                &mut admitted,
                &h,
                trigger,
                name,
                deps.actor,
            ))
            .await?;
        } else if shared_switch && name.contains("-timer-") {
            Box::pin(super::super::l6_shared_switch_timers::exercise(
                &mut owner,
                &mut admitted,
                &h,
                trigger,
                name,
            ))
            .await?;
        } else if shared_switch {
            Box::pin(super::super::l6_shared_switches::exercise(
                &owner,
                &mut admitted,
                &h,
                trigger,
            ))
            .await?;
        } else if carried_step {
            if name.contains("-present-") {
                let recipe = oteryn_gameplay_quests::quest::carried_step::RECIPES
                    .iter()
                    .find(|r| Some(r.unique) == callback.selector.unique)
                    .ok_or("source key recipe")?;
                super::input_item(
                    &owner,
                    fence.item_fence(),
                    &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                        position: trigger,
                        item: recipe.item.into(),
                        count: 1,
                    },
                    71,
                    true,
                )
                .await?;
            }
            Box::pin(super::stateless::exercise(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                stateless_destination.ok_or("source branch destination")?,
            ))
            .await?;
            super::super::l6_carried_step::verify(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                name.contains("-present-"),
                tomb_native,
            )
            .await?;
        } else if ancient_instruments {
            Box::pin(super::super::l6_ancient_tombs::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                trigger,
            ))
            .await?;
        } else if name == "triangle-tower-wall" {
            Box::pin(super::super::triangle_tower::exercise(
                &mut owner,
                &mut admitted,
                &world,
                &callback,
                trigger,
            ))
            .await?;
        } else if name == "m05-gnomus" {
            Box::pin(super::super::m05_gnomus::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if name == "engoutfit-rock-steady" {
            Box::pin(super::super::engoutfit::exercise_rock(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if name.starts_with("engoutfit-source") {
            Box::pin(super::super::engoutfit::exercise_source(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                name.ends_with("-fault"),
            ))
            .await?;
        } else if name == "m16-marker" {
            Box::pin(super::super::m16_personal_map_marker::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "engnpc-m04" {
            Box::pin(super::super::engnpc_m04_completion::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if name == "npc-item-exchanges" {
            Box::pin(super::super::npc_item_exchanges::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if name == "m42-hot-final" {
            Box::pin(super::super::m42_hot_cuisine::exercise_final(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if name == "m42-hot-typed" {
            Box::pin(super::super::m42_hot_cuisine::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if name == "catalogue-npc-xp" {
            Box::pin(super::super::npc_program::exercise_xp(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if name == "catalogue-npc-calendar" {
            Box::pin(super::super::npc_program::exercise_calendar(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "catalogue-npc-end" {
            Box::pin(super::super::npc_program::exercise_end(
                &mut owner,
                &mut admitted,
            ))
            .await?;
        } else if name == "catalogue-npc-inventory" {
            Box::pin(super::super::npc_program_inventory::exercise(
                &mut owner,
                &mut admitted,
                &authority,
                &h,
            ))
            .await?;
        } else if name == "catalogue-npc" {
            Box::pin(super::super::npc_program::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if name == "engitem-native-seal-paint" {
            Box::pin(super::super::engitem_seal_paint::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &callback,
                trigger,
            ))
            .await?;
        } else if name == "engitem-targuna-selected-tool" {
            Box::pin(super::super::srv7_reliable_ram::exercise_selected(
                &mut owner,
                &mut admitted,
                &h,
                &callback,
                trigger,
            ))
            .await?;
        } else if name == "srv7-reliable-ram" {
            Box::pin(super::super::srv7_reliable_ram::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &callback,
                trigger,
            ))
            .await?;
        } else if name == "qengine-rewards-smith-story" {
            super::super::questpc7_smith::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "qengine-rewards-hot-corpses" {
            super::super::questpc7_hot_corpses::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "qengine-rewards-goblin-slug" {
            super::super::questpc7_goblin_slug::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "qengine-rewards-goblin-perfume-items" {
            super::super::questpc7_goblin::exercise_perfume_items(&mut owner, &mut admitted, &h)
                .await?;
        } else if matches!(
            name,
            "qengine-rewards-goblin-stalkers" | "qengine-rewards-goblin-slug"
        ) {
            super::super::questpc7_goblin::exercise_stalkers(&mut owner, &mut admitted, &h).await?;
        } else if name == "qengine-rewards-goblin-trunks" {
            super::super::questpc7_goblin::exercise_trunks(&mut owner, &mut admitted, &h).await?;
        } else if name == "qengine-rewards-goblin-pond" {
            super::super::questpc7_goblin::exercise_pond(&mut owner, &mut admitted, &h).await?;
        } else if name == "qengine-rewards-goblin-signs" {
            super::super::questpc7_goblin::exercise_signs(&mut owner, &mut admitted, &h).await?;
        } else if name == "qengine-rewards-goblin-intro" {
            super::super::questpc7_goblin::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "qengine-rewards-feaster-thaian" {
            super::super::questpc7_feaster_thaian::exercise(&mut owner, &mut admitted, &h).await?;
        } else if feaster {
            super::super::questpc7_feaster::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "qengine-rewards-forgotten-story" {
            super::super::questpc7_forgotten_story::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "qengine-rewards-heart-story" {
            super::super::questpc7_heart_story::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "qengine-rewards-grimvale-story" {
            super::super::questpc7_grimvale_story::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "qengine-rewards-grimvale-contacts" {
            super::super::questpc7_grimvale::contacts(&mut owner, &mut admitted, &h).await?;
        } else if name == "questpc7-grave" {
            super::super::questpc7_grave::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "questpc7-ferumbras" {
            super::super::questpc7_ferumbras::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "questpc7-hero" {
            super::super::questpc7_hero::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "questpc7-hydra" {
            super::super::questpc7_hydra::exercise(&mut owner, &mut admitted, &h).await?;
            // An activated pointer is a boot/reset-consumer precondition, as in
            // world_reset_map_pickup_postgres_tests; no subject quest/custody seed.
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,$6,$3,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-11')")
                .bind(owner.world_id.as_bytes().as_slice()).bind(world.base().digest().as_slice())
                .bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice())
                .bind(activated.frame_binding().digest().as_slice()).bind(crate::map::overlay::map_revision(world.base())).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
                .bind(owner.world_id.as_bytes().as_slice())
                .execute(&h.pool)
                .await?;
            owner.reset_epoch = hydra_reset_drain.epoch();
            super::super::questpc7_hydra::replenishment(&mut owner, &mut admitted, &h).await?;
        } else if name == "questpc7-hoist-story" {
            super::super::questpc7_hoist_story::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "questpc7-tokens" {
            super::super::questpc7_tokens::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name.starts_with("questpc7-hidden-") {
            super::super::questpc7_native_deaths::exercise(&mut owner, &mut admitted, &h).await?;
            if name == "questpc7-hidden-story" {
                super::super::questpc7_native_deaths::reward(&mut owner, &mut admitted, &h).await?;
            }
        } else if name == "questpc7-outfits-formal" {
            super::super::questpc7_formal::exercise(&mut owner, &mut admitted, &h, &callback)
                .await?;
        } else if name.starts_with("questpc7-outfits-") {
            super::super::questpc7_outfit_items::exercise(&mut owner, &mut admitted, &h, name)
                .await?;
        } else if name == "l1-warmaster-wardrobe" {
            super::super::questpc7_world_outfit::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &callback,
                deps.actor,
            )
            .await?;
        } else if name.starts_with("questpc7-eligibility-") {
            super::super::questpc7_eligibility::exercise(&mut owner, &mut admitted, &h, name)
                .await?;
        } else if name == "questpc7-friends" {
            super::super::questpc7_friends::exercise(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if name == "questpc7-golden" {
            super::super::questpc7_golden::exercise(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if outfit_world {
            Box::pin(super::super::qengine_access::exercise(
                &mut owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                trigger,
            ))
            .await?;
        } else if name.starts_with("m09-treasure-") {
            Box::pin(super::super::m09_treasure_rewards::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &callback,
                trigger,
            ))
            .await?;
        } else if qengine_reward || shared_rewards {
            Box::pin(super::super::qengine_rewards::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                name,
            ))
            .await?;
        } else if ring_offering {
            Box::pin(super::super::m07_dreamer_ring_offering::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
            ))
            .await?;
        } else if callback.key == oteryn_gameplay_quests::quest::dreamer_death_ring::RESET {
            Box::pin(super::super::m07_dreamer_ring_reset::exercise(
                &mut owner,
                &mut admitted,
                &callback,
                trigger,
            ))
            .await?;
        } else if carrot {
            Box::pin(super::super::m07_dreamer_carrot::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                origin,
                name == "m07-carrot-premium",
            ))
            .await?;
        } else if lion_trial {
            Box::pin(super::super::m07_trials::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                trigger,
            ))
            .await?;
        } else if callback.key == oteryn_gameplay_quests::quest::dreamer_fire_bug::IGNITE {
            Box::pin(super::super::m07_dreamer_fire_bug::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                trigger,
            ))
            .await?;
        } else if name == "authored-magic-sword" {
            // Keep each full native route's nested connection futures off this shared future.
            Box::pin(super::super::questpc8_magic_sword::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "authored-trader" {
            Box::pin(super::super::questpc8_trader::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "authored-treasure-island" {
            Box::pin(super::super::questpc8_treasure_island::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "authored-secret-library" {
            Box::pin(super::super::questpc8_secret_library::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "authored-anniversary" {
            Box::pin(super::super::questpc8_anniversary::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "authored-torch" {
            Box::pin(super::super::questpc8_bridge_routes::exercise_torch(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "authored-life-ring-entry" {
            Box::pin(super::super::questpc8_life_ring::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "authored-life-ring-bridge" {
            Box::pin(super::super::questpc8_life_ring_bridge::exercise(
                &mut owner,
                &mut admitted,
            ))
            .await?;
        } else if let Some(route) = super::super::questpc8_retrieval_routes::route(name) {
            Box::pin(super::super::questpc8_retrieval_routes::exercise(
                route,
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "authored-braindeath" {
            Box::pin(
                super::super::questpc8_authored_programmes::exercise_braindeath(
                    &mut owner,
                    &mut admitted,
                    &h,
                ),
            )
            .await?;
        } else if name == "authored-kormarak" {
            Box::pin(super::super::questpc8_kormarak::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "authored-smith-cache" {
            Box::pin(super::super::questpc8_smith_cache::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "authored-tinder-box" {
            Box::pin(
                super::super::questpc8_authored_programmes::exercise_tinder_box(
                    &mut owner,
                    &mut admitted,
                    &h,
                ),
            )
            .await?;
        } else if name == "mintwallin-cyclops-scene" {
            Box::pin(super::super::questpc8_native_scenes::exercise(
                &mut owner,
                &mut admitted,
                &world,
            ))
            .await?;
        } else if name == "giant-smithhammer-asymmetric" {
            super::super::quest_l7_asymmetric::exercise(
                &mut owner,
                &mut admitted,
                &callback,
                trigger,
                &world,
            )
            .await?;
        } else if name.starts_with("formogar-hoist-") {
            super::super::questpc7_hoist::exercise(
                &mut owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                trigger,
            )
            .await?;
        } else if outfit_world
            || callback.effects != Default::default()
            || callback.quest == "oteryn:quest.waterfall_quest" && stateless_destination.is_none()
        {
            Box::pin(super::super::qengine_access::exercise(
                &mut owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                trigger,
            ))
            .await?;
        } else if let Some(expected_destination) = stateless_destination {
            Box::pin(super::stateless::exercise(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                expected_destination,
            ))
            .await?;
        } else if name.starts_with("dryad-garden-") {
            Box::pin(super::super::srvwld_dryad_garden::exercise(
                &owner,
                &mut admitted,
                deps.actor,
                &callback,
                trigger,
            ))
            .await?;
        } else if name == "srv6-schrodinger-island" {
            Box::pin(super::super::srv6_island_title::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                &callback,
                origin,
            ))
            .await?;
        } else if name == "authored-wrath-crystals" {
            super::super::questpc8_wrath_crystals::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            )
            .await?;
        } else if name == "authored-wrath-hunt" {
            super::super::questpc8_wrath_hunt::exercise(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if name == "nx6-wayfarer" {
            Box::pin(super::super::nx6_wayfarer::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if name == "npcfix-m38-minzy" {
            Box::pin(super::super::npcfix_m38_minzy::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "npcfix-m38-rascalio" {
            Box::pin(super::super::npcfix_m38_rascalio::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name.starts_with("npcfix-m38-mistrock") {
            Box::pin(super::super::npcfix_m38_mistrock::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                !name.ends_with("-free"),
            ))
            .await?;
        } else if matches!(
            name,
            "catalogue-npc-father" | "catalogue-npc-legacy" | "fathers-burden-handins"
        ) {
            if sidec_npc {
                super::super::npc_program::exercise_father(
                    &mut owner,
                    &mut admitted,
                    &h,
                    &authority,
                    name == "catalogue-npc-legacy",
                    name == "fathers-burden-handins",
                )
                .await?;
            } else {
                super::super::questpc8_npc_program_regressions::exercise_father(
                    &mut owner,
                    &mut admitted,
                    &h,
                    &authority,
                    name == "catalogue-npc-legacy",
                    name == "fathers-burden-handins",
                )
                .await?;
            }
        } else if matches!(name, "catalogue-npc-hero" | "catalogue-npc-scoped") {
            if sidec_npc {
                super::super::npc_program::exercise_scoped(
                    &mut owner,
                    &mut admitted,
                    &h,
                    &authority,
                    name == "catalogue-npc-hero",
                )
                .await?;
            } else {
                super::super::questpc8_npc_program_regressions::exercise_scoped(
                    &mut owner,
                    &mut admitted,
                    &h,
                    &authority,
                    name == "catalogue-npc-hero",
                )
                .await?;
            }
        } else if name == "authored-mystery-assembly" {
            super::super::questpc8_mystery_assembly::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            )
            .await?;
        } else if name == "authored-mystery-geometry" {
            super::super::questpc8_mystery_geometry::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            )
            .await?;
        } else if name == "authored-colours-opening" {
            super::super::questpc8_colours_opening::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "authored-wrath-opening" {
            super::super::questpc8_wrath_opening::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "authored-winterlight" {
            super::super::questpc8_winterlight::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "authored-venore" {
            super::super::questpc8_venore::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "authored-venore-supplies" {
            super::super::questpc8_venore_supplies::exercise_supplies(
                &mut owner,
                &mut admitted,
                &h,
            )
            .await?;
        } else if name == "authored-static-constructors" {
            super::super::questpc8_static_constructors::exercise(&mut owner, &mut admitted, &h)
                .await?;
        } else if name == "authored-mystery-intro" {
            super::super::questpc8_mystery::exercise_intro(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            )
            .await?;
        } else if name == "conditional-quest-greetings" {
            Box::pin(super::super::conditional_quest_greetings::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if name == "threatened-dreams-npc" {
            Box::pin(super::super::threatened_dreams::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if name == "classic-rottin-disrepair" {
            Box::pin(super::super::classic_quests::exercise_rottin(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if name == "questpc2-hero-pipes" {
            super::super::questpc2_hero_rathleton::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            )
            .await?;
        } else if name.starts_with("questpc2-albinius-portals-") {
            super::super::questpc2_albinius_portals::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                name,
            )
            .await?;
        } else if name.starts_with("questpc2-albinius-") {
            super::super::questpc2_albinius::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                name,
            )
            .await?;
        } else if name == "questpc2-gnomus-ordnance" {
            super::super::questpc2_gnomus::exercise(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if name == "questpc2-gerimor-assignments" {
            super::super::questpc2_gerimor::exercise(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if name == "questpc2-henricus-start" {
            super::super::questpc2_henricus::exercise(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if name == "questpc2-oldrak-npc" {
            super::super::questpc2_oldrak::exercise(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if classic_npc {
            Box::pin(super::super::classic_quests::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            ))
            .await?;
        } else if matches!(
            name,
            "outlaw-camp-entrance"
                | "banshees-first-seal-return"
                | "banshees-second-seal"
                | "banshees-final-battle-exit"
        ) {
            Box::pin(super::super::classic_quests::exercise_step(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                &world,
                &callback,
                trigger,
                origin,
            ))
            .await?;
        } else if monk {
            Box::pin(super::super::monk_shrines::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                &world,
                name.ends_with("full-backpack"),
                name.ends_with("capacity"),
            ))
            .await?;
        } else if callback.key == oteryn_gameplay_quests::quest::rotten_blood::SACRIFICE {
            Box::pin(super::rotten_blood::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                trigger,
                rotten_purchase,
            ))
            .await?;
        } else if reusable_tool {
            Box::pin(super::super::l6_reusable_tools::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                if name.ends_with("multitool") {
                    "oteryn:item.tibia.i9598"
                } else {
                    "oteryn:item.tibia.i3304"
                },
            ))
            .await?;
        } else if callback.item_on_target.is_some()
            || name == "outlaw-camp-door"
            || name == "travelling-trader-delivery"
        {
            Box::pin(super::actions::exercise(
                &owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                trigger,
                origin,
            ))
            .await?;
        } else if !lever_fixture && !callback.offerings.is_empty() {
            for (n, altar) in callback.offerings.iter().take(3).enumerate() {
                offering(&owner, fence.item_fence(), altar, u64::try_from(n)? + 1).await?;
            }
            let east = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                oteryn_protocol_oteryn::world_spatial::StepDirection::East,
            );
            wire(&owner, &mut admitted, 1, 1, east.clone()).await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(trigger)
            );
            let burned: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(burned, 0);
            offering(&owner, fence.item_fence(), &callback.offerings[3], 4).await?;
            wire(
                &owner,
                &mut admitted,
                2,
                1,
                oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                    oteryn_protocol_oteryn::world_spatial::StepDirection::West,
                ),
            )
            .await?;
            wire(&owner, &mut admitted, 3, 1, east).await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(callback.teleport.ok_or("teleport")?)
            );
            let burned: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(burned, 4);
            let copy = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(copy.tracks().get(&callback.storage[0].track), Some(&1));
        } else if name == "lions-rock-inner-sanctum-entry" {
            Box::pin(super::lions_rock::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                origin,
            ))
            .await?;
        } else if name == "unnatural-selection-dance-native" {
            Box::pin(super::super::m40_native_dance::exercise(
                &mut owner,
                &mut admitted,
                &h,
            ))
            .await?;
        } else if name == "unnatural-selection-dance-2" {
            Box::pin(super::unnatural_selection::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                trigger,
            ))
            .await?;
        } else if name == "liquid-black-shortcut" {
            // The source shortcut has no text. Exercise the generic authored refusal message
            // without changing that source binding in production content.
            let mut message_callback = callback.clone();
            message_callback.failure_message = Some("Finish the visitor mission first.".into());
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![message_callback])
                    .is_ok()
            );
            owner
                .states
                .chat
                .admit(admitted.game_session_id, "Callback", owner.owner_now());
            let step = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                oteryn_protocol_oteryn::world_spatial::StepDirection::East,
            );
            wire(&owner, &mut admitted, 1, 1, step.clone()).await?;
            assert!(owner.states.chat.drain(admitted.game_session_id).iter().any(|line|
                matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == "Finish the visitor mission first.")));
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                origin
            );
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            for (n, key) in ["npc_1", "action_2", "action_1", "npc_2"]
                .into_iter()
                .enumerate()
            {
                use crate::durability::quest_state::{
                    QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
                };
                let request = QuestTransitionRequest {
                    transition_key: format!(
                        "oteryn:quest-transition/quest/u9_4/liquid_black_quest/visitor/{key}"
                    ),
                    cause: QuestCause::Command(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(100 + u64::try_from(n)?)
                            .map_err(debug)?,
                    )),
                };
                assert!(matches!(
                    slot.commit_quest_transition(
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        request,
                        owner.quest_catalogue.clone().ok_or("catalogue")?
                    )
                    .await
                    .map_err(debug)?,
                    QuestTransitionOutcome::Committed(_)
                ));
            }
            drop(slot);
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            wire(&owner, &mut admitted, 2, 1, step).await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(callback.teleport.ok_or("teleport")?)
            );
            let copy = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(copy.tracks().get(&callback.storage[0].track), Some(&5));
        } else {
            let key =
                oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
                    .ok_or("key")?;
            let target = |revision| {
                let mut placement = world.base().digest().to_vec();
                placement.extend_from_slice(&key.to_be_bytes());
                WorldObjectTarget {
                    placement,
                    expected_revision: revision,
                }
            };
            if lever_fixture {
                for (n, altar) in callback.offerings.iter().take(3).enumerate() {
                    offering(&owner, fence.item_fence(), altar, u64::try_from(n)? + 1).await?;
                }
            }
            let result = wire(
                &owner,
                &mut admitted,
                1,
                2,
                oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0))
                    .map_err(debug)?,
            )
            .await?;
            let expected = if lever_fixture {
                UseDisposition::Rejected
            } else {
                UseDisposition::Committed
            };
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
                expected
            );
            if lever_fixture {
                let untouched: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(untouched, 0);
                let copy = h
                    .root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?;
                assert!(!copy.tracks().contains_key(&callback.storage[0].track));
                offering(&owner, fence.item_fence(), &callback.offerings[3], 4).await?;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let context = {
                    let runtime = deps.runtime.lock().await;
                    crate::durability::quest_world_callback::QuestWorldContext {
                        map_revision: crate::map::overlay::map_revision(world.base()),
                        content_revision: format!(
                            "sha256:{}",
                            runtime
                                .content_pin()
                                .server_artifact_digest()
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<String>()
                        ),
                        placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                        reset_epoch: owner.reset_epoch,
                        actor_placement: deps.actor.placement_identity(),
                        movement_revision: runtime
                            .read_actor_position(deps.actor)
                            .map_err(debug)?
                            .revision(),
                        party: Vec::new(),
                    }
                };
                owner
                    .states
                    .quest_callbacks
                    .interrupt_after_commit
                    .store(true, std::sync::atomic::Ordering::Release);
                let result = wire(
                    &owner,
                    &mut admitted,
                    2,
                    2,
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0))
                        .map_err(debug)?,
                )
                .await;
                assert!(
                    result.is_err(),
                    "an unknown result must close without an acknowledgement"
                );
                assert!(owner.has_pending_quest_callback(admitted.game_session_id));
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    origin
                );
                let burned: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(burned, 4);
                // Another session must not settle or project this disconnected player's
                // pending callback when its own command stream opens.
                let mut other = admitted;
                other.game_session_id =
                    crate::foundation::GameSessionId::decode(&id(51)).map_err(debug)?;
                other.continuity = crate::gameplay_transport::connection::SessionContinuity::FRESH;
                other.item_fence = None;
                {
                    let mut runtime = deps.runtime.lock().await;
                    let reservation = runtime
                        .reserve_fresh_session(other.game_session_id)
                        .map_err(debug)?;
                    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                    runtime
                        .initialize_pinned_test_position(
                            actor,
                            local(Position {
                                x: trigger.x - 2,
                                ..trigger
                            }),
                        )
                        .map_err(debug)?;
                    other.runtime_actor = Some(actor);
                }
                assert!(
                    wire(&owner, &mut other, 1, 999, Vec::new())
                        .await?
                        .is_empty()
                );
                assert!(owner.has_pending_quest_callback(admitted.game_session_id));
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    origin
                );
                // The next real dispatch reconciles the original USE before allowing input.
                let replay = wire(
                    &owner,
                    &mut admitted,
                    2,
                    2,
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0))
                        .map_err(debug)?,
                )
                .await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&replay)
                        .map_err(debug)?,
                    UseDisposition::Committed
                );
                assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
                use crate::durability::quest_state::{
                    QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
                };
                let request = QuestTransitionRequest {
                    transition_key: callback.key.clone(),
                    cause: QuestCause::Use(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(2).map_err(debug)?,
                    )),
                };
                let catalogue = owner.quest_catalogue.clone().ok_or("catalogue")?;
                let permit = owner.spell_lane_permit().await.ok_or("lane")?;
                let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                assert!(matches!(
                    slot.commit_world_callback(
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        request.clone(),
                        catalogue.clone(),
                        &permit,
                        context.clone(),
                        None,
                    )
                    .await
                    .map_err(debug)?,
                    QuestTransitionOutcome::AlreadyCommitted(_)
                ));
                // A generic quest caller cannot bypass the physical altar proof.
                assert!(matches!(slot.commit_quest_transition(owner.root, owner.character, owner.holder,
                    fence, request.clone(), catalogue.clone()).await,
                    Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)));
                let mut stale = fence;
                stale.character_lease_generation += 1;
                let fresh_request = QuestTransitionRequest {
                    cause: QuestCause::Use(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(999).map_err(debug)?,
                    )),
                    ..request
                };
                assert!(matches!(slot.commit_world_callback(owner.root, owner.character, owner.holder,
                    stale, fresh_request, catalogue, &permit, context,
                    None,
                ).await,
                    Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
                drop(slot);
                drop(permit);
                let (proofs, receipts, remaining): (i64, i64, i64) = sqlx::query_as(
                    "SELECT (SELECT count(*) FROM game_quest_callback_consumptions), \
                     (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1), \
                     (SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$2)")
                    .bind(&callback.key).bind(&callback.offerings[0].item).fetch_one(&h.pool).await?;
                assert_eq!((proofs, receipts, remaining), (4, 1, 0));
                let copy = h
                    .root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?;
                assert_eq!(copy.tracks().get(&callback.storage[0].track), Some(&1));
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    local(callback.teleport.ok_or("teleport")?)
                );
            }
            if !lever_fixture {
                for passage in &callback.passages {
                    assert!(world.enterable(crate::map::overlay::TilePos {
                        x: passage.position.x,
                        y: passage.position.y,
                        floor: passage.position.floor
                    }));
                }
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let result = wire(
                    &owner,
                    &mut admitted,
                    2,
                    2,
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&target(1))
                        .map_err(debug)?,
                )
                .await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&result)
                        .map_err(debug)?,
                    if callback.reversible {
                        UseDisposition::Committed
                    } else {
                        UseDisposition::NothingToUse
                    }
                );
                if callback.reversible {
                    for passage in &callback.passages {
                        assert!(!world.enterable(crate::map::overlay::TilePos {
                            x: passage.position.x,
                            y: passage.position.y,
                            floor: passage.position.floor,
                        }));
                    }
                }
                if name == "giant-smithhammer" {
                    use crate::map::view::MapFacts;
                    let passage = &callback.passages[0];
                    let p = crate::map::overlay::TilePos {
                        x: passage.position.x,
                        y: passage.position.y,
                        floor: passage.position.floor,
                    };
                    let rock = crate::map::view::placement_key(p, 1).ok_or("rock key")?;
                    let rock_facts = world
                        .facts()
                        .quest_callback_replacement(rock, 389)
                        .ok_or("rock")?;
                    let lever_facts = |appearance| -> TestResult<_> {
                        let mut facts = world
                            .facts()
                            .quest_callback_replacement(key, 2772)
                            .ok_or("lever")?;
                        facts.appearance_id = appearance;
                        Ok(facts)
                    };
                    // Source lever.lua:7-12: the two discordant states do nothing.
                    // Alter only the remote rock through the existing map owner.
                    assert!(world.facts().apply_quest_callback(
                        rock,
                        0,
                        rock_facts,
                        &[rock],
                        &[],
                        false,
                        None,
                    ));
                    assert!(world.facts().base_hidden(rock));
                    for (command, revision, appearance) in [(3, 2, 2772), (4, 3, 2773)] {
                        if command == 4 {
                            assert!(world.facts().apply_quest_callback(
                                rock,
                                1,
                                rock_facts,
                                &[rock],
                                &[],
                                true,
                                None,
                            ));
                            assert!(world.facts().apply_quest_callback(
                                key,
                                2,
                                lever_facts(2773)?,
                                &[],
                                &[],
                                false,
                                None,
                            ));
                        }
                        owner.clock_origin = owner
                            .clock_origin
                            .checked_sub(std::time::Duration::from_secs(2))
                            .ok_or("clock")?;
                        let result = wire(
                            &owner,
                            &mut admitted,
                            command,
                            2,
                            oteryn_protocol_oteryn::world_object::encode_use_intent(&target(
                                revision,
                            ))
                            .map_err(debug)?,
                        )
                        .await?;
                        assert_eq!(
                            oteryn_protocol_oteryn::world_object::decode_use_result(&result)
                                .map_err(debug)?,
                            // The source returns true even when neither object changes.
                            UseDisposition::Committed,
                            "discordant lever {appearance} must not toggle"
                        );
                        assert_eq!(world.facts().object_revision(key), revision);
                        assert_eq!(world.facts().base_hidden(rock), appearance == 2772);
                    }
                    assert!(world.facts().apply_quest_callback(
                        rock,
                        2,
                        rock_facts,
                        &[rock],
                        &[],
                        false,
                        None,
                    ));
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_secs(2))
                        .ok_or("clock")?;
                    let result = wire(
                        &owner,
                        &mut admitted,
                        5,
                        2,
                        oteryn_protocol_oteryn::world_object::encode_use_intent(&target(3))
                            .map_err(debug)?,
                    )
                    .await?;
                    assert_eq!(
                        oteryn_protocol_oteryn::world_object::decode_use_result(&result)
                            .map_err(debug)?,
                        UseDisposition::Committed
                    );
                    assert_eq!(world.facts().object_revision(key), 4);
                    assert!(!world.facts().base_hidden(rock));
                    assert!(!world.enterable(p));
                }
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

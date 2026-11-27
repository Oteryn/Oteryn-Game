//! Shipped room and reward checks using the retained native map.
use super::TestResult;
use super::fixture::{debug, id};
use super::kill_reward_e2e_fixtures::scenario_thread;
use super::qready1_reward_claims::{claim_case, retained_world};
use super::quest_catalogue_chests_fixtures::CatalogueChest;

#[test]
fn bear_room_shipped_key_and_rewards_are_exact_once_claims() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario_thread::run("bear-room-source-claims", || {
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let activation =
            super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
        let generation = activation.0.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let chest = CatalogueChest {
            claim: "oteryn:reward-claim.quest.key.id4601".into(),
            position: (32150, 32112, -12),
            unique: 5013,
            servable: true,
            appearance: 2472,
            root: "oteryn:item.tibia.i2970".into(),
            quantity: 1,
            contents: vec![],
            texts: vec![],
            key_number: Some(4601),
            achievement: false,
            charges: vec![],
            fluids: vec![],
        };
        let world = retained_world(chest.position, native)?;
        super::quest_container_chests_fixtures::catalogue_scenario_on_retained_map(
            &chest,
            &activation,
            &world,
        )?;

        for (claim, at, reward) in [
            ("kv.bear_chest1", (32146, 32097, -11), 2853),
            ("kv.bear_chest2", (32144, 32096, -11), 3354),
            ("kv.bear_chest3", (32141, 32097, -11), 3358),
        ] {
            claim_case(
                &format!("oteryn:reward-claim.{claim}"),
                at,
                &[],
                2472,
                &format!("oteryn:item.tibia.i{reward}"),
                1,
            )?;
        }
        Ok(())
    })
}

#[test]
fn beggar_shipped_favorite_staff_is_an_exact_once_claim() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario_thread::run("beggar-source-staff", || {
        claim_case(
            "oteryn:reward-claim.kv.simon_the_beggars_favorite_staff",
            (33167, 31600, -15),
            &[],
            2482,
            "oteryn:item.tibia.i6107",
            1,
        )
    })
}

#[test]
fn battle_axe_shipped_chest_is_an_exact_once_claim() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario_thread::run("battle-axe-source-claim", || {
        claim_case(
            "oteryn:reward-claim.kv.battleaxe",
            (32305, 32254, -9),
            &[],
            4285,
            "oteryn:item.tibia.i3266",
            1,
        )
    })
}

#[test]
fn bear_and_battle_axe_source_ground_admits_pick_and_bear_door_admits_key() -> TestResult {
    use crate::map::{door::DoorUse, overlay::TilePos};
    use oteryn_gameplay_world::terrain_tools;
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native")?;
    let catalogue = terrain_tools::crystal_core_bindings();
    for (x, y, floor) in [(32302, 32257, -8), (32149, 32110, -11)] {
        let world = retained_world((x, y, floor), native)?;
        let pos = TilePos { x, y, floor };
        let key = crate::map::view::placement_key(pos, 0).ok_or("pick ground")?;
        let action = world
            .facts()
            .terrain_tool_action(key)
            .ok_or("terrain selector")?;
        assert_eq!(action, Some(101));
        let plan = terrain_tools::dispatch(&catalogue, "oteryn:item.tibia.i3456", 355, action, 0)
            .ok_or("pick must open the route")?;
        assert_eq!(plan.binding.transformed, 394);
        if floor == -11 {
            let door_pos = TilePos {
                x: 32145,
                y: 32100,
                floor,
            };
            let door = crate::map::view::placement_key(door_pos, 1).ok_or("bear door")?;
            assert_eq!(world.facts().action_at(door), Some(4601));
            assert_eq!(
                world.facts().doors().use_key(door, 0, 4603),
                Some(DoorUse::Locked)
            );
            assert_eq!(
                world.facts().doors().use_key(door, 0, 4601),
                Some(DoorUse::Toggled {
                    revision: 1,
                    open: true
                })
            );
        }
    }
    Ok(())
}

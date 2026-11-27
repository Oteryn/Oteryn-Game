use super::{
    TestResult,
    m06_keys_fixtures::source_key_door,
    quest_catalogue_chests_fixtures::{cases, execute_committed},
};

#[test]
fn outlaw_camp_key_3303_unlocks_the_mine_door() -> TestResult {
    source_key_door(32614, 32175, -9, 1631, 3303)
}

#[test]
fn outlaw_camp_key_3304_unlocks_the_treasure_door() -> TestResult {
    source_key_door(32620, 32199, -10, 6251, 3304)
}

#[test]
fn explorer_society_key_3802_unlocks_dwacatra_prison() -> TestResult {
    source_key_door(32602, 31975, -14, 7714, 3802)
}

#[test]
fn outlaw_camp_treasure_use_commits_bag_and_completion_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("outlaw-camp-treasure", || {
        super::quest_world_callbacks_fixtures::scenario(
            "outlaw-camp-treasure",
            super::outlaw_camp_treasure_fixtures::CHEST,
            false,
        )
    })
}

#[test]
fn explorer_society_enrollment_unlocks_dwacatra_through_npc_service() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("explorer-society-enrollment", || {
        super::quest_world_callbacks_fixtures::scenario(
            "explorer-society-enrollment",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

#[test]
fn classic_quest_caches_grant_and_replay_from_committed_sources() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let uids = [
        6013, 6037, 6038, 6069, 6070, 6113, 6153, 6155, 6171, 6174, 6175, 6176, 6177, 6178, 6179,
    ];
    let mut selected: Vec<_> = cases()?
        .into_iter()
        .filter(|chest| chest.servable && uids.contains(&chest.unique))
        .collect();
    selected.sort_by_key(|chest| chest.unique);
    assert_eq!(
        selected
            .iter()
            .map(|chest| chest.unique)
            .collect::<Vec<_>>(),
        uids
    );
    execute_committed(selected)
}

#[test]
fn classic_quest_terrain_tools_well_and_lamp_use_committed_sources() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    use super::classic_quest_parity_fixtures::{ACCESS, ACCESS_PREFIX};
    super::kill_reward_e2e_fixtures::scenario_thread::run("classic-quest-access", || {
        for case in ACCESS {
            super::quest_world_callbacks_fixtures::scenario(
                &format!("{ACCESS_PREFIX}{}", case.name),
                case.source,
                false,
            )?;
        }
        Ok(())
    })
}

#[test]
fn classic_quest_npcs_offer_and_exchange_with_confirmed_inputs() -> TestResult {
    super::qengine_rewards_fixtures::scenario(super::classic_quest_parity_fixtures::NPC_CASE)
}

#[test]
fn explorer_society_ice_delivery_retains_its_selected_pick() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    use super::classic_quest_parity_fixtures::ICE_DELIVERY;
    super::kill_reward_e2e_fixtures::scenario_thread::run("explorer-society-ice-delivery", || {
        for name in ICE_DELIVERY {
            let access = super::classic_quest_parity_fixtures::access(name).ok_or("icicle access")?;
            super::quest_world_callbacks_fixtures::scenario(name, access.source, false)?;
        }
        Ok(())
    })
}

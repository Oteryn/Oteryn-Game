use super::TestResult;
use super::quest_catalogue_chests_fixtures::{cases, execute_committed};

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
fn classic_quest_approaches_open_the_native_floor_routes() -> TestResult {
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

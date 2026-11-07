//! Source levers, portals and every claim placement, through real dispatch.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc5_braindeath_coffin_exact_backpack_contents_replay_and_refusal() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    // The wiki's standalone Braindeath title is the donor's Beregar
    // BPTheUndeathStare chest, not the rest of the Beregar storyline.
    const CLAIM: &str =
        "oteryn:reward-claim.quest.u8_4.the_hidden_city_of_beregar.bpthe_undeath_stare";
    let shard: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/interactions/reward_claims/reward-claim-variants-00100-00104.json"
    ))?;
    let definition = &shard["records"]
        .as_array()
        .ok_or("claim records")?
        .iter()
        .find(|record| record["definition"]["identity"]["key"] == CLAIM)
        .ok_or("Braindeath source claim")?["definition"];
    let placements = definition["native_contents"]["placements"]
        .as_array()
        .ok_or("native placements")?;
    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0]["root"]["key"], "oteryn:item.tibia.i2867");
    assert_eq!(
        placements[0]["contents"],
        serde_json::json!([
            {"count":13,"item":{"family":"Item","key":"oteryn:item.tibia.i3035","revision":"definition-r1"}},
            {"count":1,"item":{"family":"Item","key":"oteryn:item.tibia.i3003","revision":"definition-r1"}},
            {"count":1,"item":{"family":"Item","key":"oteryn:item.tibia.i3457","revision":"definition-r1"}},
            {"count":1,"item":{"family":"Item","key":"oteryn:item.tibia.i268","revision":"definition-r1"}},
            {"count":2,"item":{"family":"Item","key":"oteryn:item.tibia.i239","revision":"definition-r1"}}
        ])
    );
    super::quest_catalogue_chests::exact_claim_cohort(&[CLAIM])?;
    println!(
        "POSTGRES_CASE_COMPLETED questpc5_braindeath_coffin_exact_backpack_contents_replay_and_refusal duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn questpc5_behemoth_lever_and_all_four_rewards() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("behemoth-access", || {
        super::quest_world_callbacks::scenario(
            "behemoth",
            Position {
                x: 33294,
                y: 31676,
                floor: -15,
            },
            false,
        )
    })?;
    super::quest_catalogue_chests::phase_a_cohort(&["behemoth_quest"])?;
    println!(
        "POSTGRES_CASE_COMPLETED questpc5_behemoth_lever_and_all_four_rewards duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn questpc5_alawars_vault_lever_both_portals_and_all_claims() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("alawars-vault-access", || {
        super::quest_world_callbacks::scenario(
            "alawars-vault-lever",
            Position {
                x: 32180,
                y: 31633,
                floor: -8,
            },
            false,
        )?;
        for (name, trigger, destination) in [
            (
                "alawars-vault-entrance",
                Position {
                    x: 32187,
                    y: 31622,
                    floor: -8,
                },
                Position {
                    x: 32107,
                    y: 31567,
                    floor: -9,
                },
            ),
            (
                "alawars-vault-exit",
                Position {
                    x: 32107,
                    y: 31566,
                    floor: -9,
                },
                Position {
                    x: 32189,
                    y: 31625,
                    floor: -4,
                },
            ),
        ] {
            super::quest_world_callbacks::stateless_scenario(name, trigger, destination)?;
        }
        Ok(())
    })?;
    super::quest_catalogue_chests::phase_a_cohort(&["alawar_s_vault_quest"])?;
    println!(
        "POSTGRES_CASE_COMPLETED questpc5_alawars_vault_lever_both_portals_and_all_claims duration={:?}",
        started.elapsed()
    );
    Ok(())
}

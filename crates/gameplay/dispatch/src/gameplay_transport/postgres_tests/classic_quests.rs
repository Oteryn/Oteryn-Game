//! Real NPC command routing and the existing durable QuestState writer.
//! This test is red without the batch-2 adapter: a static keyword reply cannot create a receipt.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;


#[test]
fn classic_quests_npc_commands_commit_exact_start_branches_and_fence_topics() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("CLASSIC_QUESTS_NPC_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("classic-quests-npc", || {
        super::quest_world_callbacks_fixtures::scenario(
            "classic-quests-npc",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED classic_quests_npc_commands_commit_exact_start_branches_and_fence_topics {:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn classic_rottin_disrepair_commits_three_flags_only_after_the_source_offer() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("CLASSIC_ROTTIN_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("classic-rottin", || {
        super::quest_world_callbacks_fixtures::scenario(
            "classic-rottin-disrepair",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

#[test]
fn classic_draconia_real_levers_open_and_restore_the_source_obstacles() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("CLASSIC_DRACONIA_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("classic-draconia", || {
        super::quest_world_callbacks_fixtures::scenario(
            "draconia-lever-30006",
            Position {
                x: 32792,
                y: 31595,
                floor: -7,
            },
            false,
        )?;
        super::quest_world_callbacks_fixtures::scenario(
            "draconia-lever-30035",
            Position {
                x: 32792,
                y: 31579,
                floor: -7,
            },
            false,
        )
    })
}

#[test]
fn classic_world_steps_use_source_teleports_and_commit_seal_flags_atomically() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("CLASSIC_STEPS_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("classic-steps", || {
        for (name, trigger) in [
            (
                "outlaw-camp-entrance",
                Position {
                    x: 32619,
                    y: 32248,
                    floor: -6,
                },
            ),
            (
                "banshees-first-seal-return",
                Position {
                    x: 32266,
                    y: 31885,
                    floor: -12,
                },
            ),
            (
                "banshees-second-seal",
                Position {
                    x: 32171,
                    y: 31853,
                    floor: -15,
                },
            ),
            (
                "banshees-final-battle-exit",
                Position {
                    x: 32219,
                    y: 31913,
                    floor: -15,
                },
            ),
        ] {
            super::quest_world_callbacks_fixtures::scenario(name, trigger, false)?;
        }
        Ok(())
    })
}

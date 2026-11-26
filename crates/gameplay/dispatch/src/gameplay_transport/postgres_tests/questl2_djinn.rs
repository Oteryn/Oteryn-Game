//! Both faction journeys must issue missions, exchange real items and unlock their shops.
use super::TestResult;

fn journey(case: &'static str, required: &[&str]) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h =
            super::fixture::Harness::create_quest_reward(admin, "l2_djinn_binding", true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED {case}-binding server_version={version}");
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let present = required
            .iter()
            .all(|key| callbacks.iter().any(|callback| callback.key.ends_with(key)));
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_BINDING_PROBE {case} duration={:?}",
            started.elapsed()
        );
        assert!(
            present,
            "complete Djinn mission bindings absent: {required:?}"
        );
        Ok(())
    })?;
    // Presence never qualifies a faction: its entire native journey must run.
    super::qengine_rewards_fixtures::scenario(case)
}

#[test]
fn questl2_djinn_efreet_full_player_journey() -> TestResult {
    journey(
        "qengine-rewards-l2-djinn-efreet",
        &[
            "/l2-efreet-enlist",
            "/l2-efreet-lamp-mission",
            "/l2-efreet-finish",
        ],
    )
}

#[test]
fn questl2_djinn_marid_full_player_journey() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let (bytes, _) = super::qc_pinned_world::artifact()?;
    let world = oteryn_world_bundle::bundle::read(&bytes)?;
    for (key, x, y, floor) in [
        ("bo_ques", 33102, 32521, -5),
        ("fa_hradin", 33102, 32540, -5),
        ("gabel", 33101, 32520, -1),
        ("haroun", 33109, 32525, -4),
        ("maryza", 32634, 31889, -9),
        ("melchior", 33142, 32817, -7),
        ("nah_bob", 33103, 32520, -2),
        ("rata_mari", 33037, 32626, -7),
        ("the_orc_king", 32983, 31732, -9),
        // The placement scope admits Umar on clear ground just north of his Crystal anchor.
        ("umar", 33102, 32531, -6),
    ] {
        let key = format!("oteryn:npc.{key}");
        let npc = world
            .npcs
            .npcs
            .iter()
            .find(|npc| npc.key == key)
            .ok_or_else(|| format!("Marid source contact missing: {key}"))?;
        assert_eq!(npc.placements.len(), 1);
        let placement = &npc.placements[0];
        assert_eq!((placement.x, placement.y, placement.floor), (x, y, floor));
    }
    drop(world);
    drop(bytes);
    journey(
        "qengine-rewards-l2-djinn-marid",
        &[
            "/l2-marid-cookbook",
            "/l2-marid-spy-report",
            "/l2-marid-finish",
            "/marid-maryza-cookbook",
            "/djinn-greeting-word",
            "/marid-umar-passage",
        ],
    )
}

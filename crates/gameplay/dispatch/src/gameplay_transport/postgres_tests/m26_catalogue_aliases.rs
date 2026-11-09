//! Real Soul War boss aliases reach source prerequisites before room allocation.
use super::TestResult;
use super::fixture::{Harness, debug, id};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn m26_soul_war_postgres_all_six_source_aliases_reach_real_entry_gates() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        super::reset();
        let h =
            Harness::create_with_admission_seed(admin, "m26_source_aliases", true, false).await?;
        super::prepare_current_session(&h).await?;
        let deps = super::Dependencies::new()?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[wire::CAPABILITY_BOSS_ENCOUNTER_V1],
        )
        .ok_or("boss capability")?;
        let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert!(level < 250, "fixture exercises the source level gate");
        for (index, boss) in [
            "goshnar_s_spite",
            "goshnar_s_malice",
            "goshnar_s_greed",
            "goshnar_s_hatred",
            "goshnar_s_cruelty",
            "goshnar_s_megalomania_purple",
        ]
        .iter()
        .enumerate()
        {
            let key = format!("oteryn:boss.{boss}");
            assert_eq!(
                super::boss_encounter::exchange_kind(
                    &owner,
                    &mut admitted,
                    index as u64 + 1,
                    wire::COMMAND_TYPE_BOSS_LEVER,
                    &wire::encode_boss_key(&key).map_err(debug)?
                )
                .await?,
                (
                    oteryn_protocol_oteryn::CommandStatus::Rejected,
                    wire::encode_boss_activity_reply(&wire::BossActivityReply::Refused(
                        wire::BossActivityRefusal::Level
                    ))
                    .map_err(debug)?
                ),
                "{key}"
            );
        }
        let groups: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            groups, 0,
            "source capture and prerequisite refusal cannot allocate an Instance"
        );
        Ok(())
    });
    eprintln!(
        "M26_SOURCE_ALIASES POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

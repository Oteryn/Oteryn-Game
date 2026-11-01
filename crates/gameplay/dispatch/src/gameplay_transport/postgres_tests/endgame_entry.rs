//! W10/W11 lever ingress reaches real prerequisites without fabricating a W02 admission.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_fence_of;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::boss_encounter::{
    CAPABILITY_BOSS_ENCOUNTER_V1, COMMAND_TYPE_BOSS_COOLDOWNS, COMMAND_TYPE_BOSS_DEFEATED,
    COMMAND_TYPE_BOSS_LEVER, COMMAND_TYPE_BOSS_TIMEOUT, BossActivityRefusal, BossActivityReply,
    encode_boss_activity_reply, encode_boss_key,
};

#[test]
fn endgame_entry_postgres_real_dispatch_cannot_allocate_or_reset_without_w02() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("ENDGAME_ENTRY POSTGRES_CASE_SKIPPED");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("endgame-entry", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "endgame_entry", true, false).await?;
        prepare_current_session(&h).await?;
        let deps = Dependencies::new()?;
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
        admitted.item_fence = item_fence_of(current);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[CAPABILITY_BOSS_ENCOUNTER_V1],
        )
        .ok_or("boss capability missing")?;
        assert_eq!(
            super::boss_encounter::exchange_kind(
                &owner,
                &mut admitted,
                1,
                COMMAND_TYPE_BOSS_COOLDOWNS,
                &[]
            )
            .await?
            .0,
            CommandStatus::Accepted
        );
        let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert!(level < 250, "fixture exercises every endgame entrance's minimum level");
        let before = counters(&h).await?;
        for (index, key) in [
            "oteryn:creature.the_primal_menace",
            "oteryn:creature.goshnar_s_spite",
            "oteryn:creature.goshnar_s_megalomania_green",
            "oteryn:creature.bakragore",
        ]
        .into_iter()
        .enumerate()
        {
            let payload = encode_boss_key(key).map_err(debug)?;
            assert_eq!(
                super::boss_encounter::exchange_kind(
                    &owner,
                    &mut admitted,
                    index as u64 + 2,
                    COMMAND_TYPE_BOSS_LEVER,
                    &payload
                )
                .await?,
                (CommandStatus::Rejected, encode_boss_activity_reply(&BossActivityReply::Refused(
                    BossActivityRefusal::Level,
                )).map_err(debug)?)
            );
            assert_eq!(counters(&h).await?, before);
        }
        // Terminal occurrences are server-owned even when the client's boss key is real.
        let payload = encode_boss_key("oteryn:creature.bakragore").map_err(debug)?;
        for (id, kind) in [
            (6, COMMAND_TYPE_BOSS_DEFEATED),
            (7, COMMAND_TYPE_BOSS_TIMEOUT),
        ] {
            assert_eq!(
                super::boss_encounter::exchange_kind(&owner, &mut admitted, id, kind, &payload)
                    .await?,
                (CommandStatus::Rejected, encode_boss_activity_reply(&BossActivityReply::Refused(
                    BossActivityRefusal::ContentUnavailable,
                )).map_err(debug)?)
            );
        }
        assert_eq!(counters(&h).await?, before);
        println!("ENDGAME_ENTRY POSTGRES_CASE_COMPLETED");
        Ok(())
    })
}

async fn counters(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_activity_handoff_groups), \
                (SELECT count(*) FROM game_character_boss_cooldowns), \
                (SELECT count(*) FROM game_boss_room_reward_receipts)",
    )
    .fetch_one(&h.pool)
    .await?)
}

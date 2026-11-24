use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::captured_boss_levers;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::CommandStatus;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::item_fence_of;
use oteryn_protocol_oteryn::boss_encounter::{
    self as wire, BossActivityRefusal, BossActivityReply,
};
use serde_json::json;

#[test]
fn staged_boss_lever_rejects_drifted_admission_policy_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("staged-boss-admission", || {
        run(async |admin| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "staged_boss_admission", true, false)
                    .await?;
            let _source = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
            super::house_transfer_fixtures::prepare_owned_session(&h).await?;
            let deps = Dependencies::new()?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let owner = deps.owner(&h, &authority);
            let mut admitted = deps.admitted;
            admitted.item_fence = item_fence_of(
                FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id)
                    .await?
                    .0,
            );
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[wire::CAPABILITY_BOSS_ENCOUNTER_V1],
            )
            .ok_or("boss encounter capability")?;
            let captures = captured_boss_levers().map_err(debug)?;
            let cell = |[x, y, floor]: [i32; 3]| json!({ "x": x, "y": y, "floor": floor });
            let mut command_id = 0;
            // The staged encounter scripts specify the final boss cells.
            for (creature, final_spawn) in [
                ("oteryn:creature.magma_bubble", [33654, 32909, 15]),
                ("oteryn:creature.timira_the_many_headed", [33815, 32703, 9]),
            ] {
                let source = captures
                    .iter()
                    .find(|candidate| candidate.creature_key.as_deref() == Some(creature))
                    .ok_or("Crystal lever capture")?;
                let boss = source.boss_key().ok_or("Crystal boss identity")?;
                sqlx::query(
                    "INSERT INTO game_boss_room_catalogue
                     (boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,
                      minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,
                      room_template_key,reward_key,enabled,template_revision,entry_positions,
                      exit_position,boss_position,after_kill_seconds)
                     VALUES($1,'content-1',$2,1,5,250,72000,1200,$3,$4,$1,true,'content-1',$5,$6,$7,60)",
                )
                .bind(&boss)
                .bind(creature)
                .bind(json!(source.lever_positions.iter().copied().map(cell).collect::<Vec<_>>()))
                .bind(boss.replace("oteryn:boss.", "oteryn:room."))
                .bind(json!(source.entry_positions.iter().copied().map(cell).collect::<Vec<_>>()))
                .bind(cell(source.exit_position.ok_or("Crystal exit cell")?))
                .bind(cell(final_spawn))
                .execute(&h.pool)
                .await?;

                for (level, cooldown, expected) in [
                    (249_i64, 72_000_i64, BossActivityRefusal::ContentChanged),
                    (250, 71_999, BossActivityRefusal::ContentChanged),
                    (250, 72_000, BossActivityRefusal::ContentUnavailable),
                ] {
                    sqlx::query(
                        "UPDATE game_boss_room_catalogue SET minimum_level=$2, cooldown_seconds=$3 WHERE boss_key=$1",
                    )
                    .bind(&boss)
                    .bind(level)
                    .bind(cooldown)
                    .execute(&h.pool)
                    .await?;
                    // Also exercise the creature alias through normal command dispatch.
                    command_id += 1;
                    let payload = wire::encode_boss_key(creature).map_err(debug)?;
                    let (status, bytes) = owner
                        .boss_activity_wire(
                            admitted,
                            command_id,
                            wire::COMMAND_TYPE_BOSS_LEVER,
                            &payload,
                        )
                        .await
                        .ok_or("boss command result")?;
                    assert_eq!(status, CommandStatus::Rejected);
                    assert_eq!(
                        wire::decode_boss_activity_reply(&bytes).map_err(debug)?,
                        BossActivityReply::Refused(expected),
                        "{boss}, level {level}, cooldown {cooldown}"
                    );
                }
            }
            let handoffs: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(handoffs, 0);
            Ok(())
        })
    })?;
    eprintln!(
        "M13_STAGED_BOSS_ADMISSION POSTGRES_CASE_COMPLETED {:?}",
        started.elapsed()
    );
    Ok(())
}

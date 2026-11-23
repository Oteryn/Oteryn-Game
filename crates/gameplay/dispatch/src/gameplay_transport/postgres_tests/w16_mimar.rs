//! Exercise the existing wire lever with unqualified room rows.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn w16_mimar_postgres_lever_refuses_unqualified_room_before_admission() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("w16-mimar", scenario);
    eprintln!(
        "W16_MIMAR POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "w16_mimar", true, false).await?;
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
        admitted.item_fence = crate::gameplay_transport::item_fence_of(
            FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0,
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[
                wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                oteryn_protocol_oteryn::boss_difficulty::CAPABILITY_BOSS_DIFFICULTY_V1,
            ],
        )
        .ok_or("boss capabilities")?;

        let start = owner
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        let cell = serde_json::json!({"x":start.x,"y":start.y,"floor":start.floor});
        let slots = serde_json::json!([cell]);
        // Use the existing harness cell. A parseable generic catalogue row still
        // cannot supply Mimar's unqualified source anchors or quest gate.
        for key in [
            "oteryn:boss.mimar_haffar",
            "oteryn:boss.unqualified_mimar_alias",
        ] {
            sqlx::query("INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,exit_position,boss_position,after_kill_seconds) VALUES($1,'content-1','oteryn:creature.mimar_haffar',1,1,1,72000,900,$2,$4,'fixture:reward',true,'fixture-v1',$2,$3,$3,1)")
                .bind(key).bind(&slots).bind(&cell).bind(key.replacen(":boss.", ":room.", 1)).execute(&h.pool).await?;
            let id = admitted.continuity.next_command_id;
            assert_eq!(
                super::boss_encounter_fixtures::exchange_kind(
                    &owner,
                    &mut admitted,
                    id,
                    wire::COMMAND_TYPE_BOSS_LEVER,
                    &wire::encode_boss_key(key).map_err(debug)?,
                )
                .await?,
                (
                    oteryn_protocol_oteryn::CommandStatus::Rejected,
                    wire::encode_boss_activity_reply(&wire::BossActivityReply::Refused(
                        wire::BossActivityRefusal::ContentUnavailable
                    ))
                    .map_err(debug)?,
                ),
                "{key}"
            );
        }
        let writes: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_activity_handoff_groups) + (SELECT count(*) FROM game_character_boss_cooldowns)")
            .fetch_one(&h.pool).await?;
        assert_eq!(writes, 0);
        Ok(())
    })
}

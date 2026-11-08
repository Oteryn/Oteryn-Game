//! Catalogue-policy slice, not a qualification of Bakragore's map or callback closure.
//! Actual fenced catalogue reads retain the source 68h policy and reject the alternate 20h.
use super::TestResult;
use super::fixture::{Harness, debug, id, run};
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

#[test]
fn m26_bakragore_postgres_fenced_catalogue_preserves_source_cooldown() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("m26-bakragore-policy", scenario);
    eprintln!(
        "M26_BAKRAGORE_POLICY POSTGRES_CASE_COMPLETED duration={:?} success={}",
        start.elapsed(),
        result.is_ok()
    );
    result
}

fn scenario() -> TestResult {
    run(async |admin| {
        super::reset();
        let h =
            Harness::create_with_admission_seed(admin, "m26_bakragore_policy", true, false).await?;
        super::prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let session = GameSessionId::decode(&id(50)).map_err(debug)?;
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?
            .0;
        let fence =
            crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
        let cell =
            |x: i32, y: i32, floor: i16| serde_json::json!({"x":x-5854,"y":y-5298,"floor":-floor});
        let waits = (33010..=33014)
            .map(|x| cell(x, 32392, 15))
            .collect::<Vec<_>>();
        let entries = vec![cell(33044, 32407, 15); 5];
        // The fixture exercises the catalogue owner only: it never publishes a
        // qualified room, allocates an Instance, or bypasses a callback closure.
        sqlx::query("INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,exit_position,boss_position,after_kill_seconds) VALUES('oteryn:boss.bakragore','content-1','oteryn:creature.bakragore',1,5,250,244800,1200,$1,'oteryn:room.boss.bakragore','fixture:reward',true,'m26-policy-fixture',$2,$3,$4,60)")
            .bind(serde_json::json!(waits)).bind(serde_json::json!(entries))
            .bind(cell(34106,32052,13)).bind(cell(33044,32394,15)).execute(&h.pool).await?;
        let store = ActivityHandoffOwner::new(h.root.clone());
        let definition = store
            .read_boss_room(&authority, &h.node, fence, "oteryn:boss.bakragore")
            .await?
            .ok_or("fenced source policy")?;
        assert_eq!(definition.policy.cooldown_ms, 244_800_000);
        assert!(
            crate::gameplay_transport::endgame_room_policy::matches_source_cooldown(&definition)
        );
        sqlx::query("UPDATE game_boss_room_catalogue SET cooldown_seconds=72000 WHERE boss_key='oteryn:boss.bakragore'")
            .execute(&h.pool).await?;
        let changed = ActivityHandoffOwner::new(h.root.clone())
            .read_boss_room(&authority, &h.node, fence, "oteryn:boss.bakragore")
            .await?
            .ok_or("cold owner rereads catalogue")?;
        assert_eq!(changed.policy.cooldown_ms, 72_000_000);
        assert!(
            !crate::gameplay_transport::endgame_room_policy::matches_source_cooldown(&changed),
            "a durable 20h catalogue cannot admit the source's canonical 68h Bakragore route"
        );
        // A catalogue alias must not bypass the creature's source policy on
        // the real command route, even before any qualified room is loaded.
        sqlx::query("INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,exit_position,boss_position,after_kill_seconds) SELECT 'oteryn:boss.native_bakragore_policy',content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,'oteryn:room.boss.native_bakragore_policy',reward_key,enabled,template_revision,entry_positions,exit_position,boss_position,after_kill_seconds FROM game_boss_room_catalogue WHERE boss_key='oteryn:boss.bakragore'")
            .execute(&h.pool).await?;
        let deps = super::Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1],
            )
            .ok_or("boss capability")?;
        use oteryn_protocol_oteryn::boss_encounter::{
            BossActivityRefusal, BossActivityReply, COMMAND_TYPE_BOSS_LEVER,
            encode_boss_activity_reply, encode_boss_key,
        };
        assert_eq!(
            super::boss_encounter::exchange_kind(
                &owner,
                &mut admitted,
                1,
                COMMAND_TYPE_BOSS_LEVER,
                &encode_boss_key("oteryn:boss.native_bakragore_policy").map_err(debug)?
            )
            .await?,
            (
                oteryn_protocol_oteryn::CommandStatus::Rejected,
                encode_boss_activity_reply(&BossActivityReply::Refused(
                    BossActivityRefusal::ContentChanged
                ))
                .map_err(debug)?
            )
        );
        let admissions: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            admissions, 0,
            "catalogue qualification grants no admission authority"
        );
        Ok(())
    })
}

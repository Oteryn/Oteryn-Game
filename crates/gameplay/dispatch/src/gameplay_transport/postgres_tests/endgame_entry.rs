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
        // Canonical template keys must reach the same gates as captured creature
        // aliases; otherwise a qualified template could bypass endgame access.
        for (number, key) in [(8, "oteryn:boss.the_primal_menace"), (9, "oteryn:boss.bakragore")] {
            let payload = encode_boss_key(key).map_err(debug)?;
            assert_eq!(
                super::boss_encounter::exchange_kind(&owner, &mut admitted, number,
                    COMMAND_TYPE_BOSS_LEVER, &payload).await?,
                (CommandStatus::Rejected, encode_boss_activity_reply(&BossActivityReply::Refused(
                    BossActivityRefusal::Level,
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

/// First production prerequisite of the W10 chain: the durable catalogue must
/// expose the qualified room before entry can spawn or settle any creature.
#[test]
fn primal_ordeal_postgres_qualified_production_room_is_reachable() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    qualified_catalogue_scenario(true)
}

#[test]
fn qualified_activity_catalogue_postgres_equals_boot_qualified_set() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    qualified_catalogue_scenario(false)
}

fn qualified_catalogue_scenario(require_primal: bool) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("primal-production-room", move || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "primal_production_room", true, false).await?;
            prepare_current_session(&h).await?;
            let deps = Dependencies::new()?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            let source_bundle = serde_json::to_vec(&serde_json::json!({
                "schema": "OTERYN_NATIVE_SOURCE_REGION_BUNDLE/v1",
                "channel": include_str!("../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"),
                "primal": include_str!("../../../../../../tools/content-schema/native-gameplay/crystal-primal-region.json"),
            }))?;
            let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id, Some(&source_bundle),
            );
            let candidates = crate::content::bosses::activity_template::captured_boss_levers().map_err(debug)?;
            let (templates, diagnostics) = controller.qualify_initial_activity_template(pin.qualified_room(), pin.qualified_room(), "content-1", &candidates);
            let publisher = crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone());
            publisher.publish_qualified_boss_rooms(owner.holder, &templates).await?;
            publisher.publish_qualified_boss_rooms(owner.holder, &templates).await?;
            let actual: Vec<String> = sqlx::query_scalar("SELECT boss_key FROM game_boss_room_catalogue ORDER BY boss_key").fetch_all(&h.pool).await?;
            let mut expected: Vec<String> = templates.iter().map(|t| t.boss_key().to_owned()).collect();
            expected.sort();
            assert_eq!(actual, expected, "catalogue contains exactly the qualified set after replay");
            if !require_primal { return Ok(()); }
            eprintln!("Primal boot qualification diagnostics: {diagnostics:?}");
            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id).await?.0;
            let fence = item_fence_of(current).ok_or("current item fence")?;
            let primal_key = candidates.iter().find_map(|candidate| {
                candidate.boss_key().filter(|key| key == "oteryn:boss.the_primal_menace")
            }).ok_or("captured Primal boss identity")?;
            let room = crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone())
                .read_boss_room_with_pin(owner.character, owner.holder, fence, &primal_key, crate::durability::activity_handoff::ActivityContentPin {
                    coordinate_frame_digest: pin.qualified_room().frame_binding().digest(),
                    map_revision_digest: pin.qualified_room().map_revision_digest(),
                    server_artifact_digest: pin.qualified_room().compiled().server_digest(),
                })
                .await?;
            assert!(room.is_some(), "W10 production Primal Ordeal room is missing from the durable catalogue");
            Ok(())
        })
    })
}

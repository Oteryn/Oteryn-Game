//! W10/W11 lever ingress reaches real prerequisites without fabricating a W02 admission.
use super::fixture::{Harness, debug, id, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_fence_of;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::boss_encounter::{
    BossActivityRefusal, BossActivityReply, CAPABILITY_BOSS_ENCOUNTER_V1,
    COMMAND_TYPE_BOSS_COOLDOWNS, COMMAND_TYPE_BOSS_DEFEATED, COMMAND_TYPE_BOSS_LEVER,
    COMMAND_TYPE_BOSS_TIMEOUT, encode_boss_activity_reply, encode_boss_key,
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
    // Preserve the unavailable-W02 terminal counterfactual independently of
    // the real source catalogues required to reach each lever's Level gate.
    terminal_request_scenario()?;
    for (seed, source, key, origin) in [
        (
            "endgame_primal",
            PinnedActivitySourceRoom::Primal,
            "oteryn:creature.the_primal_menace",
            MovementLocalPosition {
                x: 33548,
                y: 32752,
                floor: 14,
            },
        ),
        (
            "endgame_spite",
            PinnedActivitySourceRoom::GoshnarsSpite,
            "oteryn:creature.goshnar_s_spite",
            MovementLocalPosition {
                x: 33774,
                y: 31634,
                floor: 14,
            },
        ),
        (
            "endgame_megalomania",
            PinnedActivitySourceRoom::GoshnarsMegalomania,
            "oteryn:creature.goshnar_s_megalomania_green",
            MovementLocalPosition {
                x: 33676,
                y: 31634,
                floor: 14,
            },
        ),
        (
            "endgame_bakragore",
            PinnedActivitySourceRoom::Bakragore,
            "oteryn:creature.bakragore",
            MovementLocalPosition {
                x: 33010,
                y: 32392,
                floor: 15,
            },
        ),
    ] {
        source_level_scenario(seed, source, key, origin)?;
    }
    println!("ENDGAME_ENTRY POSTGRES_CASE_COMPLETED");
    Ok(())
}

fn source_level_scenario(
    seed: &'static str,
    source: PinnedActivitySourceRoom,
    key: &'static str,
    origin: MovementLocalPosition,
) -> TestResult {
    run(async move |admin| {
        reset();
        let (scene, activation_pin) = SourceScene::new_with_activation(source, origin)?;
        let active = scene
            .controller
            .active()
            .ok_or("actual source generation")?;
        let template = match source {
            PinnedActivitySourceRoom::Primal => {
                active.qualify_primal_activity_template(&scene.room, "content-1")
            }
            PinnedActivitySourceRoom::GoshnarsSpite
            | PinnedActivitySourceRoom::GoshnarsMegalomania => {
                active.qualify_soul_war_activity_template(&scene.room, "content-1", source)
            }
            PinnedActivitySourceRoom::Bakragore => {
                active.qualify_rotten_blood_activity_template(&scene.room, "content-1", source)
            }
            _ => return Err("unexpected endgame fixture source".into()),
        }
        .map_err(debug)?;
        assert!(
            template
                .lever_positions()
                .iter()
                .any(|cell| (cell.x, cell.y, cell.z)
                    == (origin.x, origin.y, i32::from(origin.floor)))
        );
        let deps = scene.dependencies()?;
        let h = Harness::create_with_admission_seed(admin, seed, false, false).await?;
        let evidence = super::house_transfer::RecoverySource::start(&h).await?;
        super::house_transfer::prepare_owned_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.evidence = &evidence.evidence;
        install_scene(&mut owner, &scene, source)?;
        let map_revision = crate::map::overlay::map_revision(
            owner
                .bundle_map()
                .ok_or("actual source Bundle")?
                .overlay
                .base(),
        );
        super::boss_activity::issue_content_and_reset_with_runtime_pin(
            &h,
            &activation_pin,
            &scene.pin,
            &map_revision,
        )
        .await?;
        let mut admitted = deps.admitted;
        let supported = [
            4,
            6,
            7,
            10,
            11,
            15,
            18,
            23,
            30,
            92,
            CAPABILITY_BOSS_ENCOUNTER_V1,
            385,
            oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_NATIVE_OBJECT_APPEARANCE_V1,
        ];
        let (offered, scope, generation) = {
            let runtime = owner.runtime.lock().await;
            (
                crate::gameplay_transport::world_required_capabilities::with_native_map_presentation_offer(
                    crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                    owner.active_generation, owner.effective_rooms().gate_wire_position(), runtime.content_pin(),
                ),
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation().get(),
            )
        };
        owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
            &[11, 18, 30], scope, generation, "policy-1", offered,
        ).map_err(debug)?;
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::connection::fresh_capability_selection(&owner, &supported)
                .await
                .ok_or("actual source capability selection")?;
        owner
            .required_capabilities
            .authenticate_resume_support(&supported, admitted.continuity.selected_capabilities)
            .map_err(debug)?;
        prepare_player(&h, &owner, &mut admitted, 249).await?;
        let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert!(
            level < 250,
            "fixture exercises every endgame entrance's minimum level"
        );
        ActivityHandoffOwner::new(h.root.clone())
            .publish_qualified_boss_rooms(owner.holder, std::slice::from_ref(&template))
            .await?;
        owner
            .activity_runtimes
            .register_template(&template)
            .map_err(|_| "actual source template")?;
        // Allocate through the current scope owner before the route's readiness
        // gate; this does not reserve a participant or fabricate a ready runtime.
        let instance = super::activity_runtime::allocate_ready_instance(&h, 88).await?;
        let before = counters(&h).await?;
        assert_eq!(before, (0, 0, 0));
        let canonical = if matches!(
            source,
            PinnedActivitySourceRoom::Primal | PinnedActivitySourceRoom::Bakragore
        ) {
            Some(template.boss_key())
        } else {
            None
        };
        for selected in std::iter::once(key).chain(canonical) {
            let command = admitted.continuity.next_command_id;
            let payload = encode_boss_key(selected).map_err(debug)?;
            assert_eq!(
                super::boss_encounter::exchange_kind(
                    &owner,
                    &mut admitted,
                    command,
                    COMMAND_TYPE_BOSS_LEVER,
                    &payload,
                )
                .await?,
                (
                    CommandStatus::Rejected,
                    encode_boss_activity_reply(&BossActivityReply::Refused(
                        BossActivityRefusal::Level,
                    ))
                    .map_err(debug)?
                )
            );
            assert_eq!(counters(&h).await?, before);
        }
        let assignment = h
            .root
            .read_runtime_scope_assignment(instance)
            .await
            .map_err(debug)?
            .ok_or("actual allocated source Instance")?;
        let registered = owner
            .activity_runtimes
            .get(instance, assignment.ownership_generation)
            .ok_or("actual source runtime reached before Level")?;
        let definition = ActivityHandoffOwner::new(h.root.clone())
            .read_boss_room(
                owner.character,
                owner.holder,
                admitted.item_fence.ok_or("current source fence")?,
                template.boss_key(),
            )
            .await?
            .ok_or("actual published source definition")?;
        assert!(
            crate::gameplay_transport::native_activity::boss_room_native_ready(
                &*registered.runtime.lock().await,
                &definition,
            ),
            "Level refusal leaves the exact live source boss and ready Instance untouched"
        );
        assert_eq!(counters(&h).await?, before);
        Ok(())
    })
}

fn terminal_request_scenario() -> TestResult {
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
        // Terminal occurrences are server-owned even when the client's boss key is real.
        let payload = encode_boss_key("oteryn:creature.bakragore").map_err(debug)?;
        for (id, kind) in [
            (2, COMMAND_TYPE_BOSS_DEFEATED),
            (3, COMMAND_TYPE_BOSS_TIMEOUT),
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

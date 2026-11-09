//! Lost Wisdom drives real command-7 dispatch and reloads its atomic item/XP receipt.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

#[test]
fn viola_mission_yes_book_handin_is_atomic_and_survives_restart() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("viola-handin", || scenario(false))?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED viola_mission_yes_book_handin_is_atomic_and_survives_restart {:?}",
        started.elapsed()
    );
    Ok(())
}
#[test]
fn viola_mission_yes_nested_book_refuses_full_bag_without_consuming() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("viola-nested", || scenario(true))?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED viola_mission_yes_nested_book_refuses_full_bag_without_consuming duration={:?}",
        started.elapsed()
    );
    Ok(())
}
fn scenario(nested: bool) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "viola_handin", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let world = super::viola_book::source_world()?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (32570, 32507, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 32570,
                    y: 32507,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = (&cells).into();
        owner.active_generation = Some(generation);
        let mut admitted = deps.admitted;
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &template,
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        let npc = install_source(&owner, &admitted).await?;
        let say = super::classic_quests::talk;
        say(&mut owner, &admitted, npc, 900, "hi").await?;
        {
            // Contended spell state must refuse without suspending with the
            // runtime locked or reserving any part of a Viola transaction.
            let states = owner.states.spell_states.lock().await;
            let refused = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                owner.npc_talk(
                    deps.actor,
                    crate::gameplay_transport::connection::UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 9,
                        item_fence: Some(fence.item_fence()),
                    },
                    oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                        npc_actor: npc,
                        text: "mission".into(),
                    },
                ),
            )
            .await
            .map_err(|_| "Viola suspended while holding the runtime lock")?;
            assert_eq!(
                refused.disposition,
                oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
            );
            assert!(!refused.unknown);
            drop(states);
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_viola_receipts")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(receipts, 0, "contended state creates no transaction");
        }
        let mut stale = fence.item_fence();
        stale.character_lease_generation += 1;
        let refused = owner
            .npc_talk(
                deps.actor,
                crate::gameplay_transport::connection::UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 8,
                    item_fence: Some(stale),
                },
                oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                    npc_actor: npc,
                    text: "mission".into(),
                },
            )
            .await;
        assert_eq!(
            refused.disposition,
            oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
        );

        assert!(
            say(&mut owner, &admitted, npc, 10, "yes")
                .await?
                .join(" ")
                .contains("first")
        );
        assert!(
            say(&mut owner, &admitted, npc, 11, "mission")
                .await?
                .join(" ")
                .contains("Say yes")
        );
        assert!(
            say(&mut owner, &admitted, npc, 12, "no")
                .await?
                .join(" ")
                .contains("change")
        );
        assert!(
            say(&mut owner, &admitted, npc, 13, "yes")
                .await?
                .join(" ")
                .contains("first")
        );
        say(&mut owner, &admitted, npc, 14, "mission").await?;
        say(&mut owner, &admitted, npc, 15, "yes").await?;
        assert!(
            say(&mut owner, &admitted, npc, 16, "mission")
                .await?
                .join(" ")
                .contains("Did you find")
        );
        assert!(
            say(&mut owner, &admitted, npc, 17, "yes")
                .await?
                .join(" ")
                .contains("Have you found")
        );
        let before:i64=sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        relocate(&owner, deps.actor, 32613, 32419).await?;
        let target = super::viola_book::target(&world)?;
        assert_eq!(
            super::quest_callbacks::wire_uses(&owner, admitted, &[1], &target).await?,
            [oteryn_protocol_oteryn::world_object::UseDisposition::Committed]
        );
        if nested {
            super::quest_world_callbacks::input_item(
                &owner,
                fence.item_fence(),
                &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                    item: "oteryn:item.tibia.i2854".into(),
                    count: 1,
                    position: oteryn_gameplay_quests::quest::world_callbacks::Position {
                        x: 32613,
                        y: 32419,
                        floor: -7,
                    },
                },
                70,
                true,
            )
            .await?;
            let (book,bag):(Vec<u8>,Vec<u8>)=sqlx::query_as("SELECT uuid_send(i.item_instance_id),uuid_send(c.item_instance_id) FROM game_item_instances i CROSS JOIN game_item_instances c JOIN game_item_container_entries e ON e.item_instance_id=c.item_instance_id WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i51977' AND c.lifecycle=1 AND c.definition_production_key='oteryn:item.tibia.i2854'").fetch_one(&h.pool).await?;
            let permit = owner.spell_lane_permit().await.ok_or("permit")?;
            let moved = h
                .root
                .commit_item_loop_move(
                    &permit,
                    &authority,
                    &h.node,
                    &fence.item_fence(),
                    reference(admitted.game_session_id, 1800)?,
                    &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                    crate::durability::item_loop_move::LoopMoveRequest {
                        item: book.try_into().map_err(|_| "book id")?,
                        count: None,
                        pickup_cell: None,
                        destination: crate::durability::item_loop_move::LoopDestination::Container(
                            bag.try_into().map_err(|_| "bag id")?,
                        ),
                    },
                )
                .await
                .map_err(debug)?;
            assert!(moved.is_ok(), "nested book: {moved:?}");
            drop(permit);
            for index in 0..19 {
                burden(&owner, fence.item_fence(), index).await?;
            }
        }
        relocate(&owner, deps.actor, 32570, 32507).await?;
        say(&mut owner, &admitted, npc, 101, "hi").await?;
        say(&mut owner, &admitted, npc, 102, "mission").await?;
        let completion_id = if nested {
            assert!(
                say(&mut owner, &admitted, npc, 103, "yes")
                    .await?
                    .join(" ")
                    .contains("make room")
            );
            assert_unfinished(&h, before).await?;
            relocate(&owner, deps.actor, 32613, 32419).await?;
            super::viola_book::free_one(&owner, fence.item_fence(), 1900).await?;
            relocate(&owner, deps.actor, 32570, 32507).await?;
            say(&mut owner, &admitted, npc, 104, "hi").await?;
            say(&mut owner, &admitted, npc, 105, "mission").await?;
            106
        } else {
            // Sequence advancement survives the rollback and proves the deferred
            // completion trigger actually ran, rather than an earlier unknown gate.
            sqlx::query("CREATE SEQUENCE fixture_viola_failure_seen")
                .execute(&h.pool)
                .await?;
            sqlx::query(
                "GRANT USAGE, SELECT ON SEQUENCE fixture_viola_failure_seen TO oteryn_game_runtime",
            )
            .execute(&h.pool)
            .await?;
            sqlx::query("CREATE FUNCTION fixture_viola_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.reply=3 THEN PERFORM nextval('fixture_viola_failure_seen'); RAISE EXCEPTION 'fixture atomic handoff rollback' USING ERRCODE='23514'; END IF; RETURN NULL; END $$").execute(&h.pool).await?;
            sqlx::query("CREATE CONSTRAINT TRIGGER fixture_viola_failure AFTER INSERT ON game_viola_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION fixture_viola_failure()").execute(&h.pool).await?;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            let failed = owner
                .npc_talk(
                    deps.actor,
                    crate::gameplay_transport::connection::UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 103,
                        item_fence: Some(fence.item_fence()),
                    },
                    oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                        npc_actor: npc,
                        text: "yes".into(),
                    },
                )
                .await;
            assert!(failed.unknown, "deferred COMMIT failure is unknown");
            assert!(
                sqlx::query_scalar::<_, bool>("SELECT is_called FROM fixture_viola_failure_seen")
                    .fetch_one(&h.pool)
                    .await?,
                "the rollback must reach the deferred completion trigger"
            );
            assert_unfinished(&h, before).await?;
            sqlx::query("DROP TRIGGER fixture_viola_failure ON game_viola_receipts")
                .execute(&h.pool)
                .await?;
            sqlx::query("DROP FUNCTION fixture_viola_failure()")
                .execute(&h.pool)
                .await?;
            sqlx::query("DROP SEQUENCE fixture_viola_failure_seen")
                .execute(&h.pool)
                .await?;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            let reconciled = owner
                .resume_npc_bank(
                    deps.actor,
                    crate::gameplay_transport::connection::UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 103,
                        item_fence: Some(fence.item_fence()),
                    },
                )
                .await
                .ok_or("retained Viola handoff")?;
            assert!(!reconciled.unknown);
            assert_eq!(
                reconciled.disposition,
                oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
            );
            say(&mut owner, &admitted, npc, 104, "mission").await?;
            105
        };
        assert!(
            say(&mut owner, &admitted, npc, completion_id, "yes")
                .await?
                .join(" ")
                .contains("800 experience")
        );
        let after:i64=sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(after - before, 800);
        let counts:(i64,i64)=sqlx::query_as("SELECT count(*) FILTER(WHERE definition_production_key='oteryn:item.tibia.i51977'),count(*) FILTER(WHERE definition_production_key='oteryn:item.tibia.i3572') FROM game_item_instances WHERE lifecycle=1").fetch_one(&h.pool).await?;
        assert_eq!(counts, (0, 1));
        drop(owner);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        let npc = install_source(&owner, &admitted).await?;
        say(&mut owner, &admitted, npc, 2001, "hi").await?;
        assert!(
            say(&mut owner, &admitted, npc, 2002, "mission")
                .await?
                .join(" ")
                .contains("already")
        );
        let reference = crate::foundation::CommandRef::new(
            admitted.game_session_id,
            crate::foundation::CommandId::new(completion_id).map_err(debug)?,
        );
        assert_eq!(
            h.root
                .reconcile_viola(&authority, fence.character_id, reference)
                .await
                .map_err(debug)?,
            Some(crate::durability::tutorial_viola::ViolaReply::Completed)
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_viola_receipts WHERE reply=3")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        assert!(
            say(&mut owner, &admitted, npc, 2003, "spells")
                .await?
                .join(" ")
                .contains("retired")
        );
        Ok(())
    })
}

fn reference(
    session: crate::foundation::GameSessionId,
    id: u64,
) -> TestResult<crate::foundation::CommandRef> {
    Ok(crate::foundation::CommandRef::new(
        session,
        crate::foundation::CommandId::new(id).map_err(debug)?,
    ))
}
async fn relocate(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    x: i32,
    y: i32,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let mut current = runtime.read_actor_position(actor).map_err(debug)?;
    assert_eq!(current.position().floor, 7);
    // The fixture visits two source cells; initialization is admission-only.
    // Keep the actor, active pin and exact position revision through each step.
    while current.position().x != x || current.position().y != y {
        let mut next = current.position();
        if next.x != x {
            next.x += (x - next.x).signum();
        } else {
            next.y += (y - next.y).signum();
        }
        current = runtime
            .borrow_movement_position()
            .commit_cardinal(current, next)
            .map_err(debug)?;
    }
    assert_eq!(current.position(), MovementLocalPosition { x, y, floor: 7 });
    Ok(())
}
async fn install_source(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult<oteryn_protocol_oteryn::world_spatial_entities::EntityRef> {
    use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model_with_native_quests(
            &model,
            &["oteryn:npc.viola"],
        ),
    ));
    owner.states.chat.admit(
        admitted.game_session_id,
        "Lost Wisdom witness",
        owner.owner_now(),
    );
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/world/npc-placements");
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("index.json"))?)?;
    let mut placements = Vec::new();
    for shard in index["shards"].as_array().ok_or("placement shards")? {
        let path = shard.as_str().ok_or("placement shard path")?;
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").join(path),
        )?)?;
        placements.extend(document["records"].as_array().ok_or("placements")?.iter().cloned());
    }
    let matches: Vec<_> = placements.iter().map(|row| &row["declaration"])
        .filter(|row| row["npc"].as_str() == Some("oteryn:npc.viola")).collect();
    assert_eq!(matches.len(), 1, "one authored Viola placement");
    let declaration = matches[0];
    assert_eq!(
        declaration["cell"],
        serde_json::json!({"x":32571,"y":32507,"floor":7})
    );
    assert_eq!(declaration["direction"], "south");
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let mut runtime = owner.runtime.lock().await;
    runtime
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    if !runtime
        .visible_entities()
        .npcs
        .iter()
        .any(|npc| runtime.npc_key(npc.actor) == Some("oteryn:npc.viola"))
    {
        runtime
            .place_npcs(
                &NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: "oteryn:npc.viola".into(),
                        placements: vec![NpcPlacement {
                            x: 32571,
                            y: 32507,
                            floor: -7,
                            direction: MovementFacing::South,
                        }],
                    }],
                },
                |_| true,
                true,
            )
            .map_err(debug)?;
    }
    let npc = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find(|npc| runtime.npc_key(npc.actor) == Some("oteryn:npc.viola"))
        .ok_or("source NPC")?;
    assert_eq!(
        (npc.position.x, npc.position.y, npc.position.floor),
        (32571, 32507, 7)
    );
    Ok(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: npc.actor.placement_identity(),
        generation: npc.generation,
    })
}
async fn assert_unfinished(h: &Harness, before: i64) -> TestResult {
    let (xp,stage,completed,crafts,books,scarves):(i64,i16,i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid),(SELECT stage FROM game_character_viola WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_viola_receipts WHERE reply=3),(SELECT count(*) FROM game_item_craft_records WHERE recipe='oteryn:npc-handoff.viola'),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i51977'),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3572')").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        (xp, stage, completed, crafts, books, scarves),
        (before, 1, 0, 0, 1, 0)
    );
    Ok(())
}
async fn burden(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    index: u64,
) -> TestResult {
    use crate::durability::{
        item_mint::TypedDefinitionRef, reward_claim_mint::RewardClaimMintRequest,
    };
    use crate::gameplay_transport::item_ref_admission::generation_item_facts;
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native")?;
    let definition = |id| TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    };
    let request = RewardClaimMintRequest {
        backpack_outputs: false,
        command: reference(fence.game_session_id, 200 + index)?,
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: format!("fixture:viola-handoff-burden.{index}"),
            revision_ref: "definition-r1".into(),
        },
        source_placement: "fixture:viola-handoff-burden".into(),
        item: generation_item_facts(native, &definition(3031)).map_err(debug)?,
        quantity: 1,
        contents: Vec::new(),
        root_text: None,
        root_fluid_subtype: None,
        root_charges_remaining: None,
        backpack: generation_item_facts(native, &definition(2854)).map_err(debug)?,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    let mut candidate = owner
        .root
        .freeze_reward_claim_mint(owner.character, owner.holder, fence, request)
        .await
        .map_err(debug)?;
    owner
        .root
        .commit_reward_claim_mint(owner.character, owner.holder, fence, &mut candidate)
        .await
        .map_err(debug)?;
    Ok(())
}

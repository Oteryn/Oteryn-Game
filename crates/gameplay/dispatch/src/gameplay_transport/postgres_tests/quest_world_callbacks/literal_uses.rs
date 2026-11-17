//! Literal source USE callbacks must select the clicked object above ground.
use super::*;

#[test]
fn quest_world_callbacks_literal_use_accepts_clicked_nonzero_ordinal() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("literal-callback-use", || {
        for (name, appearance, ordinal, source, destination) in [
            (
                "blue-valley-2",
                49948,
                3,
                (33605, 31468, -7),
                (32456, 32291, -7),
            ),
            (
                "blue-valley-3",
                49948,
                3,
                (33601, 31468, -7),
                (32456, 32291, -7),
            ),
            (
                "blue-valley-4",
                49948,
                1,
                (32456, 32290, -7),
                (33614, 31494, -7),
            ),
            (
                "blue-valley-5",
                50310,
                2,
                (32203, 32303, -6),
                (33614, 31494, -7),
            ),
            (
                "pirates-tail-cheesy-exit",
                1760,
                1,
                (33925, 31355, -7),
                (33799, 31321, -7),
            ),
            (
                "order-of-the-stag-boat-return",
                6391,
                2,
                (33874, 32250, -7),
                (32395, 32511, -7),
            ),
            (
                "order-of-the-stag-crypt-exit",
                52656,
                1,
                (32402, 32472, -5),
                (32402, 32473, -4),
            ),
        ] {
            let callback = callback(name)?;
            assert_eq!(callback.selector.position, Some(position(source)));
            assert_eq!(callback.teleport, Some(position(destination)));
            exercise(callback, appearance, ordinal, None)?;
        }
        Ok(())
    })
}

#[test]
fn quest_world_callbacks_read_only_stag_uses_gate_without_quest_writes() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("read-only-stag-use", || {
        for (name, appearance, ordinal, minimum, source, destination) in [
            (
                "order-of-the-stag-boat-to-ada",
                6391,
                4,
                3,
                (32393, 32513, -7),
                (33875, 32249, -7),
            ),
            (
                "order-of-the-stag-mark-wall",
                52735,
                1,
                11,
                (33834, 32139, -4),
                (33833, 32138, -4),
            ),
        ] {
            let callback = callback(name)?;
            assert_eq!(callback.selector.position, Some(position(source)));
            assert_eq!(callback.teleport, Some(position(destination)));
            assert!(callback.is_read_only());
            assert_eq!(callback.storage[0].minimum, minimum);
            for stage in [None, Some(minimum - 1), Some(minimum), Some(13)] {
                exercise(callback.clone(), appearance, ordinal, stage)?;
            }
        }
        // The production map contains the pre-startup wall too; position-based
        // server dispatch must not depend on the donor's visual transform.
        exercise(callback("order-of-the-stag-crypt-exit")?, 1507, 1, None)?;
        Ok(())
    })
}

fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}

fn exercise(callback: Callback, appearance: u16, ordinal: u8, stage: Option<i64>) -> TestResult {
    let started = std::time::Instant::now();
    let trigger = callback.selector.position.ok_or("position")?;
    let world = world_ordinal(&callback, trigger, appearance, ordinal)?;
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "quest_world_callbacks", false, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
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
            (
                i32::from(trigger.x) - 1,
                i32::from(trigger.y),
                -i16::from(trigger.floor),
            ),
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
        let origin = local(Position {
            x: trigger.x - 1,
            ..trigger
        });
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        {
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![callback.clone()])
                    .is_ok()
            );
        }
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::super::fresh_fixture::fresh::Source::new(0)?.request()?;
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
        let native = generation.native_gameplay().ok_or("native")?;
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
        owner.admit_quest_session(&admitted).await;

        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Traveller", owner.owner_now());
        if let Some(stage) = stage {
            use oteryn_gameplay_quests::quest::{
                QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
                QuestTransition,
            };
            let storage = callback.storage.first().ok_or("guard storage")?;
            let seed = "oteryn:quest-transition/fixture/stag-stage";
            let catalogue = std::sync::Arc::new(
                QuestStateCatalogue::new(
                    "content-1",
                    vec![QuestTrack {
                        key: storage.track.clone(),
                        quest: callback.quest.clone(),
                        initial: -1,
                        min: -1,
                        max: 13,
                    }],
                    vec![QuestTransition {
                        world_change: None,
                        key: seed.into(),
                        quest: callback.quest.clone(),
                        effects: vec![QuestEffect {
                            track: storage.track.clone(),
                            from: QuestComparison::Between(-1, 13),
                            effect: QuestEffectKind::Set(stage),
                        }],
                        completes: false,
                        experience: None,
                    }],
                )
                .map_err(debug)?,
            );
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let seeded = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    crate::durability::quest_state::QuestTransitionRequest {
                        transition_key: seed.into(),
                        cause: crate::durability::quest_state::QuestCause::Command(
                            crate::foundation::CommandRef::new(
                                admitted.game_session_id,
                                crate::foundation::CommandId::new(900).map_err(debug)?,
                            ),
                        ),
                    },
                    catalogue.clone(),
                )
                .await
                .map_err(debug)?;
            assert!(matches!(
                seeded,
                crate::durability::quest_state::QuestTransitionOutcome::Committed(_)
            ));
            drop(slot);
            owner.quest_catalogue = Some(catalogue);
        }
        let key = oteryn_world_bundle::bundle::placement_key(
            trigger.floor,
            trigger.x,
            trigger.y,
            ordinal,
        )
        .ok_or("key")?;
        let mut placement = owner
            .bundle_map()
            .ok_or("map")?
            .overlay
            .base()
            .digest()
            .to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        let before = owner
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        let receipts_before: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                .fetch_one(&h.pool)
                .await?;
        let allowed = callback
            .storage
            .first()
            .is_none_or(|storage| stage.is_some_and(|stage| stage >= storage.minimum));
        let reply = wire(
            &owner,
            &mut admitted,
            1,
            2,
            oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?,
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
            if allowed {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            }
        );
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            if allowed {
                local(callback.teleport.ok_or("destination")?)
            } else {
                origin
            }
        );
        let messages = owner.states.chat.drain(admitted.game_session_id);
        let expected_message = if allowed {
            &callback.success_message
        } else {
            &callback.failure_message
        };
        if let Some(expected_message) = expected_message {
            assert_eq!(messages.iter().filter(|line| matches!(line,
                oteryn_protocol_oteryn::chat::ChatLine::Private {text, ..} if text == expected_message)).count(), 1);
        }
        let after = owner
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(before, after, "literal USE changed durable quest state");
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(receipts, receipts_before);
        eprintln!(
            "QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED {} stage={stage:?} allowed={allowed} tracks_unchanged receipts_unchanged revision_unchanged duration={:?}",
            callback.key,
            started.elapsed()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

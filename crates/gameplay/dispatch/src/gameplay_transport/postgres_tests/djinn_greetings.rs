use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
use oteryn_gameplay_foundation::foundation::condition::{TickFacts, TickKind};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_simulation_determinism::{DecisionOccurrenceId, SemanticTimeMicros};

#[test]
fn baa_leal_soulfire_postgres_npc_talk() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("baa-leal-soulfire", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "baa_leal", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let actor = deps.actor;
            let session = deps.admitted.game_session_id;
            let position = {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(debug)?;
                let mut spells = owner.states.spell_states.lock().await;
                spells
                    .initialize(
                        &runtime,
                        actor,
                        session,
                        crate::gameplay_transport::actor_spell::tests::FACTS,
                        (0, 0),
                        owner.owner_now(),
                    )
                    .ok_or("player state")?;
                runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position()
            };
            let [npc] = super::questl2_fixture::install_at(
                &owner,
                &deps.admitted,
                ["oteryn:npc.baa_leal"],
                [oteryn_gameplay_quests::quest::world_callbacks::Position {
                    x: u16::try_from(position.x + 1)?,
                    y: u16::try_from(position.y)?,
                    floor: -i8::try_from(position.floor)?,
                }],
            )
            .await?;
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "quest fence")?
                .ok_or("quest fence")?
                .item_fence();
            for (command_id, text) in [(1, "DJANNI'HAH"), (2, "hi")] {
                let outcome = owner
                    .npc_talk(
                        actor,
                        UseCommand {
                            command_id,
                            game_session_id: session,
                            item_fence: Some(fence),
                        },
                        NpcTalkIntent {
                            npc_actor: npc,
                            text: text.into(),
                        },
                    )
                    .await;
                assert!(!outcome.unknown);
                assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
                let mut runtime = deps.runtime.lock().await;
                let mut spells = owner.states.spell_states.lock().await;
                let store = spells
                    .owned_source_condition_fixture_snapshot(&runtime, actor, session)
                    .ok_or("player condition store")?;
                let fire = store
                    .instances()
                    .iter()
                    .find(|instance| instance.definition().key() == "npc.baa_leal.soulfire");
                if command_id == 1 {
                    assert!(fire.is_none());
                } else {
                    let fire = fire.ok_or("wrong greeting did not apply Soulfire")?;
                    assert_eq!(
                        fire.definition().values(),
                        ConditionValues::DamageOverTime {
                            element: DotElement::Fire,
                            total_min: 1_500,
                            total_max: 1_500,
                            per_tick: 10,
                            interval_ms: 5_000,
                            delayed: true,
                        },
                    );
                    let mut ticks = store.clone();
                    let start = fire.started_at();
                    let facts = TickFacts {
                        in_protection_zone: false,
                        standing_on_field: None,
                    };
                    assert!(ticks.take_due(start + 4_999_999, facts).is_empty());
                    for tick in 1..=150 {
                        let due = ticks.take_due(start + tick * 5_000_000, facts);
                        assert_eq!(due.len(), 1);
                        assert_eq!(
                            due[0].kind,
                            TickKind::Damage {
                                element: DotElement::Fire,
                                amount: 10,
                                refused: false,
                            }
                        );
                    }
                    assert!(ticks.take_due(start + 755_000_000, facts).is_empty());
                    let receipt = spells
                        .tick_source_player_conditions(
                            &mut runtime,
                            actor,
                            session,
                            DecisionOccurrenceId::from_bytes([1; 16]),
                            facts,
                            SemanticTimeMicros::from_micros(start + 5_000_000),
                        )
                        .ok_or("live first Soulfire tick")?;
                    assert_eq!(receipt.damage, 10);
                }
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn orc_king_guards_postgres_npc_talk() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("orc-king-greeting", || {
        run(async |admin| {
            use super::fixture::{fence, id};
            use crate::foundation::ChannelContentPin;
            use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
            use oteryn_gameplay_quests::quest::world_callbacks::Position;

            let h = Harness::create_with_admission_seed(admin, "orc_king", true, false).await?;
            prepare_current_session(&h).await?;
            let world = fence(1)?.runtime_scope.world_id();
            let (controller, pin) =
                crate::content::native_gameplay::test_support::activated_with_loot_tables(
                    world, None,
                );
            let active = controller.active().ok_or("active content")?;
            let native = active.native_gameplay().ok_or("native gameplay")?;
            let bundle = super::questl2_djinn_world::world()?;
            let cells = bundle
                .movement_cells(pin.qualified_room().movement_cells())
                .map_err(debug)?;
            let deps = Dependencies::with_content_and_capacity(
                ChannelContentPin::from_activation(
                    world,
                    pin.activation_sequence(),
                    pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),
                    bundle.map_revision_digest(),
                    (33294, 31677, 15),
                ),
                pin.qualified_room().clone(),
                native.spell_book().clone(),
                32,
            )?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            owner.movement_cells = (&cells).into();
            owner.quest_catalogue = Some(std::sync::Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            let mut admitted = deps.admitted;
            let session = admitted.game_session_id;
            {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
                native
                    .install_companion_policies(&mut runtime)
                    .map_err(debug)?;
            }
            // The existing Djinn bundle fixture has eight open neighbors here.
            let king = Position {
                x: 33297,
                y: 31676,
                floor: -15,
            };
            let [npc] = super::questl2_fixture::install_at(
                &owner,
                &admitted,
                ["oteryn:npc.the_orc_king"],
                [king],
            )
            .await?;
            deps.initialize(&owner).await;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
                .bind(id(42).as_slice()).bind(id(43).as_slice())
                .bind(active.identity().server_artifact_digest().as_slice())
                .bind(active.identity().client_artifact_digest().as_slice())
                .bind(deps.room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
                .bind(id(41).as_slice()).execute(&h.pool).await?;
            owner.admit_progression_session(&mut admitted).await;
            owner.refresh_quest_session(session, true).await;
            for command_id in [1, 2] {
                let result =
                    super::qengine_rewards::talk(&mut owner, &mut admitted, npc, command_id, "hi")
                        .await?;
                assert_eq!(result, NpcIntentDisposition::Ok);
                let mut runtime = deps.runtime.lock().await;
                let creatures = runtime.visible_entities().creatures;
                if command_id == 1 {
                    let mut guards = std::collections::BTreeMap::new();
                    for creature in creatures {
                        let snapshot = runtime.companion_snapshot(creature.actor).map_err(debug)?;
                        *guards
                            .entry(snapshot.state.policy.definition_key.clone())
                            .or_insert(0) += 1;
                        runtime
                            .remove_encounter_actor(creature.actor)
                            .map_err(debug)?;
                    }
                    assert_eq!(
                        guards,
                        [
                            ("oteryn:creature.orc_leader".into(), 3),
                            ("oteryn:creature.orc_warlord".into(), 2),
                            ("oteryn:creature.slime".into(), 3),
                        ]
                        .into_iter()
                        .collect()
                    );
                } else {
                    assert!(
                        creatures.is_empty(),
                        "a later greeting does not replace the guards"
                    );
                }
            }
            let greeted: i64 = sqlx::query_scalar(
                "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
            )
            .bind("oteryn:quest-progress/quest/u7_4/djinn_war/orc_king_greeting")
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(greeted, 1);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

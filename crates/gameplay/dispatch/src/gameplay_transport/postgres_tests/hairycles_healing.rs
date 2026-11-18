//! Real NPC talk with a typed quest-healer fixture.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const NPC: &str = "oteryn:npc.hairycles";
const TRACK: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/questline";
const QUEST: &str = "oteryn:quest.the_ape_city_quest";

#[test]
fn hairycles_checks_quest_progress_then_heals_before_fire_and_poison() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("hairycles quest healing", move || scenario(admin))
}

fn scenario(admin: String) -> TestResult {
    run(async move |_| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "hairycles_healing", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let deps = Dependencies::with_content(
            ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                pin.qualified_room().map_revision_digest(),
                (2, 0, 7),
            ),
            pin.qualified_room().clone(),
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::QuestStateCatalogue::new(
                "fixture-hairycles-r1",
                vec![oteryn_gameplay_quests::quest::QuestTrack {
                    key: TRACK.into(),
                    quest: QUEST.into(),
                    initial: 0,
                    min: 0,
                    max: 18,
                }],
                Vec::new(),
            )
            .map_err(debug)?,
        ));
        deps.initialize(&owner).await;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model_with_native_quests(
                &model,
                &[NPC],
            )));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "quest fence read")?
            .ok_or("quest fence")?;
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    session,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            let at = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            let placed = runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: NPC.into(),
                            placements: vec![NpcPlacement {
                                x: u16::try_from(at.x + 1)?,
                                y: u16::try_from(at.y)?,
                                floor: -i8::try_from(at.floor)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            EntityRef {
                identity: placed.actors[0].placement_identity(),
                generation: placed.actors[0].actor_local_generation(),
            }
        };
        // Crystal 201189f8 data-global/npc/hairycles.lua:223-239.
        let json = serde_json::json!({
            "schema": "OTERYN_NPC_HEALING/v1",
            "healers": [{
                "npc": NPC,
                "profile": "quest_emergency",
                "quest_emergency": {
                    "track": TRACK,
                    "minimum": 12,
                    "health_floor": 50,
                    "refusal": "You look for food and rest.",
                    "health_effect": "CONST_ME_MAGIC_BLUE",
                    "fire_effect": "CONST_ME_MAGIC_GREEN",
                    "poison_effect": "CONST_ME_MAGIC_RED"
                }
            }]
        });
        owner
            .states
            .npc_conversations
            .fixture_healers
            .set(
                crate::content::npc_catalogue::healing::Catalogue::from_json(&json.to_string())
                    .ok_or("quest healer catalogue")?,
            )
            .map_err(|_| "quest healer fixture already installed")?;
        set_progress(&owner, &h, session, 11).await?;
        super::npc_paid_travel::talk(&owner, &deps, npc, 1, "hi").await?;
        use crate::domain::condition::{ConditionType, ConditionValues, DotElement, StatusKind};
        use oteryn_gameplay_foundation::foundation::condition::{
            ApplicationFacts, ConditionDefinition, DamageSchedule, DamageSegment,
        };
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([91; 32]);
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            crate::gameplay_transport::actor_spell::tests::wound(
                &mut states,
                deps.actor,
                session,
                10,
            );
            let mut definitions = Vec::new();
            for (key, element) in [
                ("s10.fire", DotElement::Fire),
                ("s10.poison", DotElement::Poison),
            ] {
                definitions.push(
                    ConditionDefinition::new_damage_schedule(
                        key,
                        1,
                        element,
                        DamageSchedule::Fixed {
                            delayed: true,
                            segments: vec![DamageSegment {
                                count: 100,
                                interval_ms: 1000,
                                amount: 1,
                            }],
                        },
                    )
                    .ok_or("condition")?,
                );
            }
            definitions.push(
                ConditionDefinition::new(
                    "s10.drunk",
                    1,
                    ConditionValues::TimedStatus {
                        kind: StatusKind::Drunk,
                        duration_ms: 100_000,
                    },
                )
                .ok_or("status")?,
            );
            let facts = ApplicationFacts {
                now: owner.owner_now().get(),
                base_speed: 220,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                    [92; 16],
                ),
            };
            let next = states
                .get(&runtime, deps.actor, session)
                .ok_or("vitals")?
                .stage_owned_test_conditions(&definitions, &facts)
                .ok_or("stage conditions")?;
            assert!(states.commit(&runtime, deps.actor, session, next));
        }
        // Crystal 201189f8 data/npclib/npc.lua:13-20 vetoes a word-prefixed
        // occurrence anywhere, while permitting "heal" at the start of "healing".
        for (command, text, health, fire, poison) in [
            (2, "heal", 10, true, true),
            (3, "unhealthy, please heal", 10, true, true),
            (4, "please HEALing", 50, true, true),
            (5, "heal", 50, false, true),
            (6, "heal", 50, false, false),
            (7, "heal", 50, false, false),
        ] {
            if command == 3 {
                set_progress(&owner, &h, session, 12).await?;
            }
            owner.clock_origin -= std::time::Duration::from_secs(1);
            let outcome = super::npc_paid_travel::talk(&owner, &deps, npc, command, text).await?;
            if command != 3 {
                let lines = outcome.conversation.ok_or("healing response")?.lines;
                if matches!(command, 2 | 7) {
                    assert_eq!(lines, ["You look for food and rest."]);
                } else {
                    assert!(lines.is_empty());
                }
            }
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states.get(&runtime, deps.actor, session).ok_or("vitals")?;
            assert_eq!(state.vitals().health, health, "command={command}");
            let has = |kind| {
                state
                    .owned_conditions()
                    .instances()
                    .iter()
                    .any(|instance| instance.definition().condition_type() == kind)
            };
            assert_eq!(has(ConditionType::DamageOverTime(DotElement::Fire)), fire);
            assert_eq!(
                has(ConditionType::DamageOverTime(DotElement::Poison)),
                poison
            );
            assert!(
                has(ConditionType::Status(StatusKind::Drunk)),
                "unrelated condition must survive"
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn set_progress(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    session: crate::foundation::GameSessionId,
    value: i64,
) -> TestResult {
    assert_eq!(
        sqlx::query(
            "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)              SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots              WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key)              DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision",
        )
        .bind(id(41).as_slice())
        .bind(TRACK)
        .bind(QUEST)
        .bind(value)
        .execute(&h.pool)
        .await?
        .rows_affected(),
        1,
    );
    owner.refresh_quest_session(session, true).await;
    assert_eq!(
        owner
            .states
            .quest_sessions
            .lock()
            .map_err(|_| "quest sessions")?
            .get(&session)
            .and_then(|state| state.copy.as_ref())
            .and_then(|copy| copy.tracks().get(TRACK))
            .copied(),
        Some(value),
    );
    Ok(())
}

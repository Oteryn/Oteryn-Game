//! Curos's mission sequence from TibiaWiki /Spoiler (curid 41957, oldid 891001).
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_quests::quest::loader;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use std::sync::Arc;

const ALLIANCE: &str = "oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/questline";
const DOOR: &str = "oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/quest_door";

fn curos_position() -> TestResult<Position> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/npc-placements/index.json"),
    )?)?;
    let mut positions = Vec::new();
    for shard in index["shards"].as_array().ok_or("NPC placement shards")? {
        let path = shard.as_str().ok_or("NPC placement shard path")?;
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join(path))?)?;
        for record in document["records"]
            .as_array()
            .ok_or("NPC placement records")?
        {
            let placement = &record["declaration"];
            if placement["npc"] == "oteryn:npc.curos" {
                let cell = &placement["cell"];
                assert_eq!(
                    cell,
                    &serde_json::json!({"x": 33174, "y": 31394, "floor": 7})
                );
                positions.push(Position {
                    x: 33174,
                    y: 31394,
                    floor: -7,
                });
            }
        }
    }
    assert_eq!(positions.len(), 1, "exact Curos placement");
    Ok(positions[0])
}

#[test]
fn curos_dispatch_checks_entry_and_advances_each_mission() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx58-curos", scenario);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx58-curos duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "qcx58_curos", true, false).await?;
        let result: TestResult = async {
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let mut owner = deps.owner(&h, &authority);
            let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
            owner.active_generation = Some(controller.active().ok_or("active generation")?);
            let loaded = loader::load_embedded_quest_state("content-1").map_err(debug)?;
            let canonical = loaded.catalogue();
            owner.quest_catalogue = Some(Arc::new(canonical.clone()));
            let at = curos_position()?;
            owner
                .runtime
                .lock()
                .await
                .initialize_pinned_test_position(
                    deps.actor,
                    MovementLocalPosition {
                        x: i32::from(at.x),
                        y: i32::from(at.y) + 1,
                        floor: -i16::from(at.floor),
                    },
                )
                .map_err(debug)?;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            owner.admit_quest_session(&admitted).await;
            let [curos] = super::classic_quests::install_positions(
                &owner,
                &admitted,
                ["oteryn:npc.curos"],
                [at],
            )
            .await?;
            let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
            h.root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
            let premium = SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            );

            super::classic_quests::talk(&mut owner, &admitted, curos, 100, "hi").await?;
            super::qengine_rewards::talk(&mut owner, &mut admitted, curos, 101, "mission").await?;
            assert_eq!(super::m39_bone_journey::stage(&h, ALLIANCE).await?, None);

            // Arrange upstream credit through the canonical durable storage
            // transitions. The mission assertions exercise Curos's dispatcher.
            for (command, transition_key) in [
                (900, "oteryn:quest-transition/quest/u8_54/the_new_frontier/mission06/other_1"),
                (901, "oteryn:quest-transition/quest/u8_54/the_new_frontier/mission06/npc_1"),
                (902, "oteryn:quest-transition/quest/u8_54/an_uneasy_alliance/quest_door/creature_event_1"),
            ] {
                let fence = owner
                    .current_quest_fence(admitted.game_session_id)
                    .await
                    .map_err(|()| "quest fence")?
                    .ok_or("live session")?;
                let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                let outcome = slot
                    .commit_quest_transition(
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        QuestTransitionRequest {
                            transition_key: transition_key.into(),
                            cause: QuestCause::Command(CommandRef::new(
                                admitted.game_session_id,
                                CommandId::new(command).map_err(debug)?,
                            )),
                        },
                        owner.quest_catalogue.clone().ok_or("quest catalogue")?,
                    )
                    .await
                    .map_err(debug)?;
                assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)), "{transition_key}: {outcome:?}");
                drop(slot);
                owner
                    .refresh_quest_session(admitted.game_session_id, false)
                    .await;

                if command == 900 {
                    owner.premium_coordinator = Some(&premium);
                    owner.admit_premium(id(40), admitted.game_session_id);
                    super::qengine_rewards::talk(&mut owner, &mut admitted, curos, 102, "mission")
                        .await?;
                    assert_eq!(super::m39_bone_journey::stage(&h, ALLIANCE).await?, None);
                } else if command == 901 {
                    owner.premium_coordinator = None;
                    super::qengine_rewards::talk(&mut owner, &mut admitted, curos, 103, "mission")
                        .await?;
                    assert_eq!(super::m39_bone_journey::stage(&h, ALLIANCE).await?, None);

                    owner.premium_coordinator = Some(&premium);
                    let lines =
                        super::classic_quests::talk(&mut owner, &admitted, curos, 104, "mission")
                            .await?;
                    assert!(
                        lines
                            .iter()
                            .any(|line| line.contains("Find him in the mountain north-west"))
                    );
                    assert_eq!(super::m39_bone_journey::stage(&h, ALLIANCE).await?, Some(1));
                    assert_eq!(super::m39_bone_journey::stage(&h, DOOR).await?, Some(0));
                    super::qengine_rewards::talk(&mut owner, &mut admitted, curos, 105, "mission")
                        .await?;
                    assert_eq!(super::m39_bone_journey::stage(&h, ALLIANCE).await?, Some(1));
                }
            }

            let lines =
                super::classic_quests::talk(&mut owner, &admitted, curos, 106, "mission").await?;
            assert!(
                lines
                    .iter()
                    .any(|line| line.contains("With the death of their leader"))
            );
            assert_eq!(super::m39_bone_journey::stage(&h, ALLIANCE).await?, Some(2));
            assert_eq!(super::m39_bone_journey::carried(&h, 10217).await?, 0);
            let lines =
                super::classic_quests::talk(&mut owner, &admitted, curos, 107, "mission").await?;
            assert!(lines.iter().any(|line| {
                line.contains("Infiltrate this tower and destroy their scrying device")
            }));
            assert_eq!(super::m39_bone_journey::stage(&h, ALLIANCE).await?, Some(3));
            super::qengine_rewards::talk(&mut owner, &mut admitted, curos, 108, "mission").await?;
            assert_eq!(super::m39_bone_journey::stage(&h, ALLIANCE).await?, Some(3));
            assert_eq!(super::m39_bone_journey::carried(&h, 10217).await?, 0);
            Ok(())
        }
        .await;
        h.cleanup().await?;
        result
    })
}

//! A live Gamel greeting either publishes both Bandits and the durable stage, or neither.
use super::full_world_support::*;
use oteryn_gameplay_quests::quest::loader::{GAMEL_AMBUSH_TRACK, load_embedded_quest_state};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

#[test]
fn secserv_gamel_greeting_atomically_spawns_bandits_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("secserv-gamel-greeting", scenario)
}

fn scenario() -> TestResult {
    world_scenario!(
        "secserv_gamel",
        |h,
         request,
         seal,
         authority,
         world_id,
         native,
         world,
         cells,
         room,
         pin,
         deps,
         actor,
         session,
         owner,
         fence,
         admitted| {
            owner.quest_catalogue = Some(Arc::new(
                load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            let start = [(GAMEL_AMBUSH_TRACK.into(), 1)];
            super::qengine_outfit_stories::seed_quest_prerequisites(
                &h, &owner, &admitted, &start, 0xa8,
            )
            .await?;
            owner.admit_quest_session(&admitted).await;
            let npc_position = Position {
                x: 9,
                y: 1,
                floor: -7,
            };
            let [gamel] = super::questl2_fixture::install_at(
                &owner,
                &admitted,
                ["oteryn:npc.gamel"],
                [npc_position],
            )
            .await?;

            assert_eq!(
                super::qengine_rewards::talk(&mut owner, &mut admitted, gamel, 200, "hi").await?,
                NpcIntentDisposition::Ok
            );
            assert_eq!(
                super::m39_bone_journey::stage(&h, GAMEL_AMBUSH_TRACK).await?,
                Some(1)
            );
            assert_eq!(bandits(&owner).await, 0);
            super::qengine_rewards::talk(&mut owner, &mut admitted, gamel, 201, "bye").await?;

            super::quest_world_callbacks::input_item(
                &owner,
                fence,
                &Offering {
                    item: "oteryn:item.tibia.i402".into(),
                    count: 1,
                    position: Position {
                        x: 10,
                        y: 0,
                        floor: -7,
                    },
                },
                12,
                true,
            )
            .await?;
            let obstruction = {
                let mut runtime = owner.runtime.lock().await;
                runtime
                    .admit_encounter_actor("oteryn:creature.rat", 20, RAT_CELL)
                    .map_err(debug)?
            };
            assert_eq!(
                super::qengine_rewards::talk(&mut owner, &mut admitted, gamel, 202, "hi").await?,
                NpcIntentDisposition::Rejected
            );
            assert_eq!(
                bandits(&owner).await,
                0,
                "one free slot cannot publish half the batch"
            );
            assert_eq!(
                super::m39_bone_journey::stage(&h, GAMEL_AMBUSH_TRACK).await?,
                Some(1)
            );
            owner
                .runtime
                .lock()
                .await
                .remove_encounter_actor(obstruction)
                .map_err(debug)?;
            super::qengine_rewards::talk(&mut owner, &mut admitted, gamel, 203, "bye").await?;
            assert_eq!(
                super::qengine_rewards::talk(&mut owner, &mut admitted, gamel, 204, "hi").await?,
                NpcIntentDisposition::Ok
            );
            assert_eq!(bandits(&owner).await, 2);
            assert_eq!(
                super::m39_bone_journey::stage(&h, GAMEL_AMBUSH_TRACK).await?,
                Some(2)
            );
            assert_eq!(super::m39_bone_journey::carried(&h, 402).await?, 1);
            super::qengine_rewards::talk(&mut owner, &mut admitted, gamel, 205, "bye").await?;
            super::qengine_rewards::talk(&mut owner, &mut admitted, gamel, 206, "hi").await?;
            assert_eq!(
                bandits(&owner).await,
                2,
                "a second greeting cannot replay the ambush"
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }
    )
}

async fn bandits(owner: &ComposedFreshAdmission<'_, '_, '_>) -> usize {
    let runtime = owner.runtime.lock().await;
    runtime
        .visible_entities()
        .creatures
        .iter()
        .filter(|entry| {
            runtime.matches_live_creature_identity(entry.actor, b"oteryn:creature.bandit")
        })
        .count()
}

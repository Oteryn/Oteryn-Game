use super::TestResult;
use super::full_world_support::world_scenario;

const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/questline";
const ONGULF: &str = "oteryn:npc.ongulf";

#[test]
fn nfront1_postgres_ongulf_assigns_persistent_negotiation_keywords() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("PostgreSQL route is empty".into());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("nfront1-ongulf", scenario);
    eprintln!(
        "POSTGRES_CASE_COMPLETED nfront1_ongulf duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn scenario() -> TestResult {
    world_scenario!(
        "nfront1_ongulf",
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
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            super::qengine_outfit_stories::seed_quest_prerequisites(
                &h,
                &owner,
                &admitted,
                &[(QUESTLINE.into(), 13)],
                0xba,
            )
            .await?;
            owner.admit_quest_session(&admitted).await;
            let [npc] = super::classic_quests::install(&owner, &admitted, [ONGULF]).await?;
            super::classic_quests::talk(&mut owner, &admitted, npc, 50, "hi").await?;
            let _ = owner
                .npc_talk(
                    actor,
                    UseCommand {
                        game_session_id: session,
                        command_id: 51,
                        item_fence: Some(fence),
                    },
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: "mission".into(),
                    },
                )
                .await;
            let read = || async {
                sqlx::query_as::<_,(String,i64,String)>(
                    "SELECT track_key,value,committed_character_revision::text FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.the_new_frontier_quest' ORDER BY track_key"
                ).bind(id(41).as_slice()).fetch_all(&h.pool).await
            };
            let first = read().await?;
            let stage = first
                .iter()
                .find(|row| row.0 == QUESTLINE)
                .ok_or("mission stage")?;
            assert_eq!(stage.1, 14);
            for (name, maximum) in [
                ("angus", 2),
                ("humgolf", 2),
                ("leeland", 2),
                ("telas", 2),
                ("wyrdin", 4),
            ] {
                let suffix = format!("/mission05/{name}_keyword");
                let row = first
                    .iter()
                    .find(|row| row.0.ends_with(&suffix))
                    .ok_or_else(|| format!("missing roll for {name}"))?;
                assert!((1..=maximum).contains(&row.1), "{name} roll was {}", row.1);
                assert_eq!(row.2, stage.2, "{name} must commit with the mission");
            }
            let _ = owner
                .npc_talk(
                    actor,
                    UseCommand {
                        game_session_id: session,
                        command_id: 51,
                        item_fence: Some(fence),
                    },
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: "mission".into(),
                    },
                )
                .await;
            assert_eq!(
                read().await?,
                first,
                "the same mission command cannot reroll"
            );
            h.cleanup().await?;
            Ok(())
        }
    )
}

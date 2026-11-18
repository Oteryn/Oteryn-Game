use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, threatened_dreams};

const NPC: &str = "oteryn:npc.maelyrra";
const NIGHTMARE: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/nightmare_intruders";
const ENFEEBLED: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/enfeebled_count";
const FRAZZLEMAWS: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/frazzlemaws_count";
const KROAZUR: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/kroazur_kill";

#[test]
fn nightmare_intruders_requires_combined_kills_and_kroazur_in_live_dialogue() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("nightmare-intruders", move || {
        run(async move |_| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "nightmare_intruders", true, false)
                .await?;
            let result = async {
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let deps = Dependencies::new()?;
                let mut owner = deps.owner(&h, &authority);
                owner.quest_catalogue = Some(std::sync::Arc::new(
                    load_embedded_quest_state("content-1").map_err(debug)?.catalogue().clone()
                ));
                owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
                let mut admitted = deps.admitted;
                owner.admit_progression_session(&mut admitted).await;
                owner.refresh_quest_session(admitted.game_session_id, true).await;
                deps.initialize(&owner).await;
                let [npc] = super::classic_quests::install(&owner, &admitted, [NPC]).await?;
                let talk = super::classic_quests::talk;
                talk(&mut owner, &admitted, npc, 1, "hi").await?;
                talk(&mut owner, &admitted, npc, 2, "mission").await?;
                talk(&mut owner, &admitted, npc, 3, "yes").await?;
                assert_eq!(super::classic_quests::progress(&owner, &admitted, &authority, NIGHTMARE).await?, 1);

                for (command, frazzlemaws, boss, line, stage) in [
                    (4, 49_i64, 1_i64, 105, 1),
                    (6, 50, 0, 105, 1),
                    (8, 50, 1, 98, 2),
                    (10, 50, 1, 102, 2),
                ] {
                    // Stage the three inputs independently of creature combat so this
                    // test isolates the report gate at 199, 200, and missing boss credit.
                    let mut tx = h.pool.begin().await?;
                    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                    for (track, value) in [(ENFEEBLED, 150_i64), (FRAZZLEMAWS, frazzlemaws), (KROAZUR, boss)] {
                        sqlx::query(
                            "INSERT INTO game_character_quest_tracks
                             (character_id, track_key, quest_key, value, committed_character_revision)
                             SELECT character_id, $3, quest_key, $4, committed_character_revision
                             FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2
                             ON CONFLICT (character_id, track_key) DO UPDATE SET value=EXCLUDED.value"
                        ).bind(id(41).as_slice()).bind(NIGHTMARE).bind(track).bind(value).execute(&mut *tx).await?;
                    }
                    tx.commit().await?;
                    owner.refresh_quest_session(admitted.game_session_id, false).await;
                    talk(&mut owner, &admitted, npc, command, "mission").await?;
                    let lines = talk(&mut owner, &admitted, npc, command + 1, "yes").await?;
                    assert_eq!(lines, threatened_dreams::reply(NPC, line).ok_or("source reply")?);
                    assert_eq!(super::classic_quests::progress(&owner, &admitted, &authority, NIGHTMARE).await?, stage);
                }
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        })
    })
}

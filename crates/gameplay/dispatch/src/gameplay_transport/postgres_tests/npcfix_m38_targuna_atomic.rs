//! Backend atomicity with an actual runtime capture at a synthetic fixture landing.
//! This is not the native Anna dialogue/map story or permission to enable its offer.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_home_town::HomeTownRelocation;
use crate::durability::character_tutorial::TutorialProgressOutcome;
use crate::durability::npc_travel::TravelOccurrence;
use oteryn_gameplay_quests::tutorial::{TutorialProgress, TutorialStep};

#[test]
fn npcfix_m38_targuna_arrival_requires_atomic_citizenship_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("npcfix-m38-targuna-atomic", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "m38_targuna_atomic", true, false)
                .await?;
            let result: TestResult = async {
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let deps = Dependencies::new()?;
                let owner = deps.owner(&h, &authority);
                let session = deps.admitted.game_session_id;
                let (placed, destination) = {
                    let mut runtime = deps.runtime.lock().await;
                    runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
                    let start = runtime.read_actor_position(deps.actor).map_err(debug)?.position();
                    let mut destination = start;
                    destination.x = destination.x.checked_add(1).ok_or("destination overflow")?;
                    // The requested cell is unavailable. Reuse the actual native fallback
                    // owner and admit only the already qualified current fixture cell.
                    let snapshot = runtime.place_admitted_respawn(deps.actor,session,Some(destination),|cell|cell==start).map_err(debug)?;
                    assert_eq!(snapshot.position(), start);
                    assert_ne!(snapshot.position(), destination);
                    (owner.placed_record(&runtime, snapshot.position()), destination)
                };
                let fence = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("fence")?;
                let occurrence = TravelOccurrence::from_bytes(crate::gameplay_transport::npc_trade_command::derived(b"m38-targuna", session, 48001)).map_err(debug)?;
                let before = owner.root.character_tutorial_progress(owner.character,owner.holder,fence,None).await.map_err(debug)?;
                let version = match before {
                    TutorialProgressOutcome::Read(Some(row)) => row.version,
                    TutorialProgressOutcome::Read(None) => 0,
                    _ => return Err("tutorial read outcome".into()),
                };
                owner.root.character_tutorial_progress(owner.character,owner.holder,fence,Some((version,TutorialProgress{step:TutorialStep::Island,hunting:true,corruptor:true}))).await.map_err(debug)?;
                // Seed only the cause/obligation. The production reader and both consume owners
                // still prove current recovery, session, lease, node and gameplay fences.
                let mut tx = h.pool.begin().await?;
                sqlx::query("INSERT INTO game_newhaven_travel_receipts(character_id,game_session_id,command_id,occurrence_id,route) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,48001,encode($3,'hex')::uuid,'targuna')")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(session.as_bytes().as_slice()).bind(occurrence.as_bytes().as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_pending_arrivals(character_id,occurrence_id,world_id,tile_x,tile_y,tile_floor) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6)")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(destination.x).bind(destination.y).bind(destination.floor).execute(&mut *tx).await?;
                tx.commit().await?;
                eprintln!("POSTGRES_STARTED NPCFIX_M38_TARGUNA_ATOMIC");
                let ordinary = owner.root.consume_npc_pending_arrival_placed(owner.character,owner.holder,fence,occurrence,placed).await;
                assert!(ordinary.is_err(), "ordinary consume must not omit citizenship: {ordinary:?}");
                for town in [6, 33] {
                    assert!(owner.root.character_home_town(owner.character,owner.holder,fence,33,Some(town),Some(HomeTownRelocation{placed,respawn:None,arrival:Some(occurrence)})).await.is_err(), "wrong town must roll back");
                    super::maintain_ready(owner.root).await?;
                }
                // Fail the last owner write after arrival/position/tutorial work. Everything
                // must roll back, not just the home-town row.
                sqlx::query("CREATE FUNCTION m38_reject_town() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'M38 injected hometown failure'; END $$").execute(&h.pool).await?;
                sqlx::query("CREATE TRIGGER m38_reject_town BEFORE INSERT OR UPDATE ON game_character_home_towns FOR EACH ROW EXECUTE FUNCTION m38_reject_town()").execute(&h.pool).await?;
                assert!(owner.root.character_home_town(owner.character,owner.holder,fence,33,Some(34),Some(HomeTownRelocation{placed,respawn:None,arrival:Some(occurrence)})).await.is_err(), "late writer failure must roll back all owners");
                super::maintain_ready(owner.root).await?;
                sqlx::query("DROP TRIGGER m38_reject_town ON game_character_home_towns").execute(&h.pool).await?;
                let unchanged: (i16,bool,bool) = sqlx::query_as("SELECT step,hunting,corruptor FROM game_character_tutorial").fetch_one(&h.pool).await?;
                assert_eq!(unchanged,(2,true,true),"failed arrival must preserve all tutorial flags");
                let completed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_newhaven_travel_receipts WHERE arrived_at IS NOT NULL").fetch_one(&h.pool).await?;
                assert_eq!(completed,0,"late failure must not complete receipt");
                let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_pending_arrivals").fetch_one(&h.pool).await?;
                assert_eq!(pending,1,"refusals must preserve recovery obligation");
                assert_eq!(owner.root.character_home_town(owner.character,owner.holder,fence,33,Some(34),Some(HomeTownRelocation{placed,respawn:None,arrival:Some(occurrence)})).await.map_err(debug)?,34);
                let state: (i64,i64,i32) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_pending_arrivals),(SELECT count(*) FROM game_newhaven_travel_receipts WHERE arrived_at IS NOT NULL),(SELECT town_id FROM game_character_home_towns LIMIT 1)").fetch_one(&h.pool).await?;
                assert_eq!(state,(0,1,34),"arrival and citizenship commit together");
                let departed: (i16,bool,bool) = sqlx::query_as("SELECT step,hunting,corruptor FROM game_character_tutorial").fetch_one(&h.pool).await?;
                assert_eq!(departed,(3,false,false),"all tutorial flags clear in the arrival transaction");
                let fresh = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("fresh fence")?;
                assert_eq!(owner.root.character_home_town(owner.character,owner.holder,fresh,33,Some(6),None).await.map_err(debug)?,6);
                let replay_fence = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("replay fence")?;
                assert!(owner.root.character_home_town(owner.character,owner.holder,replay_fence,33,Some(34),Some(HomeTownRelocation{placed,respawn:None,arrival:Some(occurrence)})).await.is_err(),"replaying an old arrival must not rewind later citizenship");
                let town: i32 = sqlx::query_scalar("SELECT town_id FROM game_character_home_towns").fetch_one(&h.pool).await?;
                assert_eq!(town,6);

                Ok(())
            }.await;
            h.cleanup().await?;
            result
        })
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38_TARGUNA_ATOMIC duration={:?}",
        started.elapsed()
    );
    Ok(())
}

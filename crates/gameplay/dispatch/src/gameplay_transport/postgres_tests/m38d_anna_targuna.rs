//! Crystal 201189f8 Anna: real NPC dispatch, durable arrival and citizenship.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::ChannelContentPin;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn m38d_anna_targuna_production_dispatch_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let began = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("m38d-anna", || {
        for (level, stored) in [(7_i64, true), (8, true), (21, false)] {
            scenario(level, stored)?;
        }
        Ok(())
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED M38d Anna->Targuna duration={:?}",
        began.elapsed()
    );
    Ok(())
}

fn scenario(level: i64, stored: bool) -> TestResult {
    run(async move |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "m38d_anna", false, false).await?;
        let result: TestResult = async {
            sqlx::query("INSERT INTO game_character_progression_state VALUES (encode($1,'hex')::uuid,1,$2,999,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
                .bind(id(41).as_slice()).bind(level).execute(&h.pool).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let world_id = super::fixture::fence(1)?.runtime_scope.world_id();
            let room = crate::content::qualify_native_entry_room(world_id).map_err(debug)?;
            // Only geometry is synthetic. NPC content, prompts and durable owners are production.
            let initial_x = if stored { 32534 } else { 32535 };
            let start = Position { x: initial_x, y: 32513, floor: -7 };
            let mut geometry = oteryn_gameplay_quests::quest::world_callbacks::load()?.into_iter()
                .find(|c| c.key.ends_with("behemoth")).ok_or("geometry seed")?;
            geometry.teleport = Some(Position { x: 31938, y: 31928, floor: -7 });
            let world = super::quest_world_callbacks::world(&geometry, start)?;
            let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
            let channel_pin = ChannelContentPin::from_activation(world_id, 1,
                room.compiled().server_digest(), room.compiled().client_digest(),
                room.frame_binding().digest(), world.map_revision_digest(), (i32::from(initial_x),32513,7));
            let deps = Dependencies::with_content(channel_pin, room,
                crate::spell::cast::v1_spell_book().map_err(debug)?)?;
            let mut owner = deps.owner(&h, &authority);
            owner.movement_cells = (&cells).into();
            owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
            let mut admitted = deps.admitted;
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
            use oteryn_gameplay_quests::tutorial::{TutorialProgress, TutorialStep};
            if stored { owner.root.character_tutorial_progress(&authority, owner.holder, fence,
                Some((0,TutorialProgress { step: TutorialStep::Island, hunting: true, corruptor: true }))).await.map_err(debug)?; }
            if !stored {
                let mut tx=h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id,pz_block_until,kill_block_until) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,clock_timestamp()+interval '1 minute',clock_timestamp()+interval '1 minute')")
                    .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
            }
            let [npc] = super::classic_quests::install(&owner, &admitted, ["oteryn:npc.anna"]).await?;
            let talk = super::classic_quests::talk;
            assert_eq!(talk(&mut owner,&admitted,npc,48000,"hi").await?,
                ["Hello, my friend. Do you seek {passage} to the island {Targuna}?"]);
            if level == 7 {
                assert_eq!(talk(&mut owner,&admitted,npc,48001,"passage").await?,
                    ["You need to be at least level 8 to travel to Targuna. Keep training!"]);
                assert_eq!(talk(&mut owner,&admitted,npc,48002,"yes").await?,
                    ["You need to be at least level 8 to travel."]);
                assert_eq!(owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position(),
                    crate::foundation::MovementLocalPosition {x:32534,y:32513,floor:7});
            } else {
                assert_eq!(talk(&mut owner,&admitted,npc,48001,"passage").await?,
                    ["I'm offering a ferry service to the island {Targuna}, are you interested?"]);
                assert_eq!(talk(&mut owner,&admitted,npc,48002,"no").await?,
                    ["No problem. Come back when you're ready to explore Targuna!"]);
                talk(&mut owner,&admitted,npc,48003,"yes").await?;
                assert_eq!(owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position().x,i32::from(initial_x));
                talk(&mut owner,&admitted,npc,48004,"passage").await?;
                talk(&mut owner,&admitted,npc,48005,"sure").await?;
                assert_eq!(owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position().x,i32::from(initial_x));
                assert_eq!(talk(&mut owner,&admitted,npc,48006,"yes").await?,
                    ["Very well, I'll take you to Targuna then. The locals there could use a helping hand!"]);
                assert_eq!(owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position(),
                    crate::foundation::MovementLocalPosition {x:31938,y:31928,floor:7});
            }
            let progress: Option<(i16,bool,bool)> = sqlx::query_as("SELECT step,hunting,corruptor FROM game_character_tutorial WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_optional(&h.pool).await?;
            assert_eq!(progress,if !stored {None} else if level == 7 {Some((2,true,true))} else {Some((3,false,false))});
            let town: Option<i32> = sqlx::query_scalar("SELECT town_id FROM game_character_home_towns WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_optional(&h.pool).await?;
            assert_eq!(town, if level == 7 { None } else { Some(34) });
            let routes: Vec<(String,bool)> = sqlx::query_as("SELECT route,arrived_at IS NOT NULL FROM game_newhaven_travel_receipts ORDER BY command_id")
                .fetch_all(&h.pool).await?;
            assert_eq!(routes, if level == 7 {vec![]} else {vec![("targuna".into(),true)]});
            let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_pending_arrivals").fetch_one(&h.pool).await?;
            assert_eq!(pending,0);
            let fees:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns").fetch_one(&h.pool).await?;
            assert_eq!(fees,0);
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}

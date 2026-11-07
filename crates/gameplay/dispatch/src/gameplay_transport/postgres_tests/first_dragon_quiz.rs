//! Gelidrazah quiz advances only through ordered native NPC answers.
use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug, id, run};
use super::threatened_dreams::barriers::progress;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use std::sync::Arc;
const TRACK: &str = "oteryn:quest-progress/quest/u11_02/the_first_dragon/gelidrazah_access";
const NPC: &str = "oteryn:npc.gelidrazah_s_thirst";
const KEY: &str = "oteryn:quest-reward/questpc3/first-dragon-quiz/access";
const GEOMETRY_ITEM: &str = "oteryn:item.tibia.i24939";
// Synthetic walkable geometry places the actor next to the fixture-installed source NPC.
const TILES: [(u16, u16, i8); 3] = [(32600, 31800, -7), (32610, 31800, -7), (32620, 31800, -7)];
fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}
fn north((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y: y - 1, floor }
}
#[test]
fn first_dragon_quiz_requires_ordered_answers_and_durable_access_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("FIRST_DRAGON_QUIZ POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(||{let started=std::time::Instant::now();match std::panic::catch_unwind(scenario){Ok(result)=>{eprintln!("FIRST_DRAGON_QUIZ POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());result.map_err(|e|e.to_string())},Err(panic)=>{eprintln!("FIRST_DRAGON_QUIZ POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());std::panic::resume_unwind(panic)}}})?.join().map_err(|_|"quiz scenario panicked")??;
    Ok(())
}
fn fixture_catalogue() -> TestResult<QuestStateCatalogue> {
    let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let tracks = vec![source.catalogue().track(TRACK).ok_or("quiz track")?.clone()];
    let transitions = source
        .catalogue()
        .transition(KEY)
        .cloned()
        .into_iter()
        .collect();
    QuestStateCatalogue::new("content-1", tracks, transitions).map_err(debug)
}
fn scenario() -> TestResult {
    let catalogue = Arc::new(fixture_catalogue()?);
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin.clone(), "first_dragon_quiz", false, false)
                .await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("FIRST_DRAGON_QUIZ POSTGRES_CASE_STARTED server_version={version}");
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
        let mut geometry = oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .next()
            .ok_or("map prototype")?;
        let first = TILES[0];
        geometry.selector.position = Some(position(first));
        geometry.selector.action = Some(25002);
        geometry.selector.unique = None;
        geometry.selector.appearance = Some(100);
        geometry.lever = None;
        geometry.teleport = None;
        geometry.item_on_target = None;
        geometry.carried_use = None;
        // These fields construct fixture geometry, never grants or executable offerings.
        geometry.passages = vec![];
        geometry.offerings = TILES
            .into_iter()
            .map(north)
            .map(|at| Offering {
                position: at,
                item: GEOMETRY_ITEM.into(),
                count: 1,
            })
            .collect();
        let additional: Vec<_> = TILES[1..]
            .iter()
            .enumerate()
            .map(|(i, at)| {
                let mut extra = geometry.clone();
                extra.selector.position = Some(position(*at));
                extra.selector.action = Some(25003 + u16::try_from(i).unwrap_or(0));
                extra
            })
            .collect();
        let world = super::quest_world_callbacks::world_with_triggers(
            &geometry,
            position(first),
            &additional,
        )?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let start = north(first);
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(start.x),
                i32::from(start.y),
                -i16::from(start.floor),
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
        runtime
            .initialize_pinned_test_position(deps.actor, super::quest_world_callbacks::local(start))
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = &cells;
        owner.quest_catalogue = Some(catalogue);
        let mut admitted = deps.admitted;
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
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
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        super::item_look::seed_main_backpack(&h, &[]).await?;
        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Quest witness", owner.owner_now());
        owner
            .states
            .quest_callbacks
            .fixture_clock_ms
            .store(3_600_000, std::sync::atomic::Ordering::Relaxed);
        let [npc] = install(&owner, &admitted, [NPC]).await?;
        let mut command = 39000;
        let mut access = progress(&owner, &admitted, &authority, TRACK).await?;
        assert_eq!(access, 0);
        talk(&mut owner, &admitted, npc, command, "hi").await?;
        command += 1;
        talk(&mut owner, &admitted, npc, command, "svir").await?;
        command += 1;
        assert_eq!(
            progress(&owner, &admitted, &authority, TRACK).await?,
            0,
            "the final answer before all questions grants no access"
        );
        for answer in ["yes", "yes", "tahmehe", "ishara"] {
            talk(&mut owner, &admitted, npc, command, answer).await?;
            command += 1;
            assert_eq!(
                progress(&owner, &admitted, &authority, TRACK).await?,
                0,
                "only the final ordered answer writes access"
            );
        }
        let reply = talk(&mut owner, &admitted, npc, command, "svir").await?;
        command += 1;
        access = progress(&owner, &admitted, &authority, TRACK).await?;
        assert!(
            reply.iter().any(|s| s.contains("satisfactorily answered")),
            "source final reply"
        );
        assert_eq!(access, 1);
        for word in ["bye", "hi", "yes", "yes", "tahmehe", "ishara", "svir"] {
            talk(&mut owner, &admitted, npc, command, word).await?;
            command += 1;
        }
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        )
        .bind(KEY)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(progress(&owner, &admitted, &authority, TRACK).await?, 1);
        // The native placement admission is part of the source journey, not only fixture installation.
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../"));
        let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join("content/world/npc-placements/index.json"),
        )?)?;
        let mut placed = false;
        for shard in index["shards"].as_array().ok_or("placement shards")? {
            placed |= std::fs::read_to_string(root.join(shard.as_str().ok_or("placement path")?))?
                .contains(NPC);
        }
        eprintln!(
            "FIRST_DRAGON_QUIZ POSTGRES_EVIDENCE access={access} receipts={receipts} placed={placed}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert_eq!(
            receipts, 1,
            "repeating the completed quiz writes no second value receipt"
        );
        assert!(placed, "source NPC must have native placement admission");
        Ok(())
    })
}

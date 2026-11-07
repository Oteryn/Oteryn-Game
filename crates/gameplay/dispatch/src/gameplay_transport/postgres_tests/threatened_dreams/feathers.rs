//! Five source AIDs must grant physical feathers through committed native movement.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use super::barriers::{progress, quantity, relocate};
use super::rakes::seed;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use oteryn_gameplay_world::map::view::MapFacts;
use std::collections::BTreeMap;
use std::sync::Arc;
const QUEST: &str = "oteryn:quest.threatened_dreams_quest";
const ANIMALS: &str = oteryn_gameplay_quests::quest::threatened_dreams::ANIMALS;
const COUNTER: &str =
    "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission01/feathers_count";
const BASE: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission01/";
const PREFIX: &str = "oteryn:quest-transition/questpc3/threatened-feather/";
const FEATHER: &str = "oteryn:item.tibia.i25244";
// Only geometry is synthetic: source dispatch selects AIDs, not positions.
const TILES: [(u16, u16, i8); 5] = [
    (32600, 31800, -7),
    (32610, 31800, -7),
    (32620, 31800, -7),
    (32630, 31800, -7),
    (32640, 31800, -7),
];
fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}
fn north((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y: y - 1, floor }
}
fn prior(value: i64) -> String {
    format!("oteryn:quest-transition/fixture/threatened-feathers/stage{value}")
}
#[test]
fn threatened_five_feather_steps_mint_once_and_fifth_advances_story() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_FEATHERS POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(||{let started=std::time::Instant::now();match std::panic::catch_unwind(scenario){Ok(result)=>{eprintln!("THREATENED_FEATHERS POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());result.map_err(|e|e.to_string())},Err(panic)=>{eprintln!("THREATENED_FEATHERS POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());std::panic::resume_unwind(panic)}}})?.join().map_err(|_|"feathers scenario panicked")??;
    Ok(())
}
fn fixture_catalogue() -> TestResult<QuestStateCatalogue> {
    let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let mut tracks = BTreeMap::new();
    for key in [ANIMALS, COUNTER] {
        tracks.insert(
            key.into(),
            source.catalogue().track(key).ok_or("source track")?.clone(),
        );
    }
    for i in 1..=5 {
        let key = format!("{BASE}feathers{i}");
        tracks.insert(
            key.clone(),
            source
                .catalogue()
                .track(&key)
                .cloned()
                .unwrap_or(QuestTrack {
                    key,
                    quest: QUEST.into(),
                    initial: -1,
                    min: -1,
                    max: 1,
                }),
        );
    }
    let mut transitions = Vec::new();
    for callback in oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .filter(|c| c.key.starts_with(PREFIX))
    {
        transitions.push(
            source
                .catalogue()
                .transition(&callback.key)
                .ok_or("feather transition")?
                .clone(),
        );
    }
    for value in [12, 13] {
        transitions.push(QuestTransition {
            key: prior(value),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: ANIMALS.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(value),
            }],
            completes: false,
            experience: None,
        });
    }
    QuestStateCatalogue::new("content-1", tracks.into_values().collect(), transitions)
        .map_err(|e| format!("{e:?}").into())
}
fn scenario() -> TestResult {
    let catalogue = Arc::new(fixture_catalogue()?);
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin.clone(), "threatened_feathers", false, false)
                .await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("THREATENED_FEATHERS POSTGRES_CASE_STARTED server_version={version}");
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
        let mut geometry = oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .next()
            .ok_or("map prototype")?;
        let first = TILES[0];
        geometry.selector.position = Some(position(first));
        geometry.selector.action = Some(25024);
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
                item: FEATHER.into(),
                count: 1,
            })
            .collect();
        let additional: Vec<_> = TILES[1..]
            .iter()
            .enumerate()
            .map(|(i, at)| {
                let mut extra = geometry.clone();
                extra.selector.position = Some(position(*at));
                extra.selector.action = Some(25025 + u16::try_from(i).unwrap_or(0));
                extra
            })
            .collect();
        let world = super::super::quest_world_callbacks::world_with_triggers(
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
            .initialize_pinned_test_position(
                deps.actor,
                super::super::quest_world_callbacks::local(start),
            )
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
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        super::super::item_look::seed_main_backpack(&h, &[]).await?;
        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Quest witness", owner.owner_now());
        owner
            .states
            .quest_callbacks
            .fixture_clock_ms
            .store(3_600_000, std::sync::atomic::Ordering::Relaxed);
        seed(&owner, &admitted, &prior(12), 35000).await?;
        step(&mut owner, &mut admitted, TILES[0], 1).await?;
        assert_eq!(
            quantity(&h, FEATHER).await?,
            0,
            "wrong mission stage never mints"
        );
        seed(&owner, &admitted, &prior(13), 35001).await?;
        let mut completed = 0;
        let mut command = 2;
        for (i, at) in TILES.into_iter().enumerate() {
            step(&mut owner, &mut admitted, at, command).await?;
            command += 1;
            let counter = progress(&owner, &admitted, &authority, COUNTER).await?;
            let flag = progress(
                &owner,
                &admitted,
                &authority,
                &format!("{BASE}feathers{}", i + 1),
            )
            .await?;
            let expected = i64::try_from(i + 1)?;
            if counter == expected && flag == 1 && quantity(&h, FEATHER).await? == expected {
                completed += 1;
                assert_eq!(
                    progress(&owner, &admitted, &authority, ANIMALS).await?,
                    if i == 4 { 14 } else { 13 },
                    "only the fifth real feather advances story"
                );
                step(&mut owner, &mut admitted, at, command).await?;
                command += 1;
                assert_eq!(
                    quantity(&h, FEATHER).await?,
                    expected,
                    "fresh reentry never duplicates a feather"
                );
                assert_eq!(
                    progress(&owner, &admitted, &authority, COUNTER).await?,
                    expected
                );
            }
        }
        for at in TILES {
            let key = oteryn_world_bundle::bundle::placement_key(at.2, at.0, at.1, 1)
                .ok_or("placement")?;
            assert_eq!(
                owner.bundle_map().ok_or("map")?.facts.object_revision(key),
                0,
                "feather tile stays unchanged"
            );
        }
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE transition_key LIKE 'oteryn:quest-transition/questpc3/threatened-feather/%'").fetch_one(&h.pool).await?;
        let feathers = quantity(&h, FEATHER).await?;
        let counter = progress(&owner, &admitted, &authority, COUNTER).await?;
        let stage = progress(&owner, &admitted, &authority, ANIMALS).await?;
        eprintln!(
            "THREATENED_FEATHERS POSTGRES_EVIDENCE completed={completed} receipts={receipts} feathers={feathers} counter={counter} stage={stage}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert_eq!(
            completed, 5,
            "all five literal movement sources must mint distinct feathers"
        );
        assert_eq!(receipts, 5);
        assert_eq!(feathers, 5);
        assert_eq!(counter, 5);
        assert_eq!(stage, 14);
        Ok(())
    })
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: (u16, u16, i8),
    command: u64,
) -> TestResult {
    relocate(owner, admitted, north(at)).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?
            .item_fence(),
    );
    super::super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(
            oteryn_protocol_oteryn::world_spatial::StepDirection::South,
        ),
    )
    .await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position(),
        super::super::quest_world_callbacks::local(position(at)),
        "literal native movement enters source tile"
    );
    Ok(())
}

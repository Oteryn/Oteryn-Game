//! Three distinct native life-site visits each advance the source secret counter once.
use super::fixture::{Harness, debug, id, run};
use super::threatened_dreams::barriers::{progress, relocate};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use oteryn_gameplay_world::map::view::MapFacts;
use std::collections::BTreeMap;
use std::sync::Arc;
const QUEST: &str = "oteryn:quest.the_first_dragon_quest";
const COUNTER: &str = "oteryn:quest-progress/quest/u11_02/the_first_dragon/secrets_counter";
const BASE: &str = "oteryn:quest-progress/quest/u11_02/the_first_dragon/";
const PREFIX: &str = "oteryn:quest-transition/questpc3/first-dragon-life/";
const FLAGS: [&str; 3] = ["desert_tile", "stone_sculpture_tile", "suntower_tile"];
const GEOMETRY_ITEM: &str = "oteryn:item.tibia.i24939";
// Only geometry is synthetic: source dispatch selects AIDs, not positions.
const TILES: [(u16, u16, i8); 3] = [(32600, 31800, -7), (32610, 31800, -7), (32620, 31800, -7)];
fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}
fn north((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y: y - 1, floor }
}
#[test]
fn first_dragon_three_distinct_life_sites_count_once_without_questline_gate() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("FIRST_DRAGON_LIFE POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(||{let started=std::time::Instant::now();match std::panic::catch_unwind(scenario){Ok(result)=>{eprintln!("FIRST_DRAGON_LIFE POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());result.map_err(|e|e.to_string())},Err(panic)=>{eprintln!("FIRST_DRAGON_LIFE POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());std::panic::resume_unwind(panic)}}})?.join().map_err(|_|"life-site scenario panicked")??;
    Ok(())
}
fn fixture_catalogue() -> TestResult<QuestStateCatalogue> {
    let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let mut tracks = BTreeMap::new();
    for key in [COUNTER] {
        tracks.insert(
            key.into(),
            source.catalogue().track(key).ok_or("source track")?.clone(),
        );
    }
    for flag in FLAGS {
        let key = format!("{BASE}{flag}");
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
                .ok_or("life-site transition")?
                .clone(),
        );
    }
    QuestStateCatalogue::new("content-1", tracks.into_values().collect(), transitions)
        .map_err(|e| format!("{e:?}").into())
}
fn scenario() -> TestResult {
    let catalogue = Arc::new(fixture_catalogue()?);
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin.clone(), "first_dragon_life", false, false)
                .await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("FIRST_DRAGON_LIFE POSTGRES_CASE_STARTED server_version={version}");
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
        let mut completed = 0;
        let mut command = 1;
        assert_eq!(progress(&owner, &admitted, &authority, COUNTER).await?, 0);
        for (i, at) in TILES.into_iter().enumerate() {
            step(&mut owner, &mut admitted, at, command).await?;
            command += 1;
            let expected = i64::try_from(i + 1)?;
            let counter = progress(&owner, &admitted, &authority, COUNTER).await?;
            let flag = progress(
                &owner,
                &admitted,
                &authority,
                &format!("{BASE}{}", FLAGS[i]),
            )
            .await?;
            if counter == expected && flag == 1 {
                completed += 1;
            }
            step(&mut owner, &mut admitted, at, command).await?;
            command += 1;
            assert_eq!(
                progress(&owner, &admitted, &authority, COUNTER).await?,
                counter,
                "reentering an already counted source never increments the counter"
            );
        }
        eprintln!(
            "FIRST_DRAGON_LIFE POSTGRES_PROGRESS completed={completed} counter={}",
            progress(&owner, &admitted, &authority, COUNTER).await?
        );
        for at in TILES {
            let key = oteryn_world_bundle::bundle::placement_key(at.2, at.0, at.1, 1)
                .ok_or("placement")?;
            let map = owner.bundle_map().ok_or("map")?;
            assert_eq!(
                map.facts.object_revision(key),
                1,
                "one successful StepIn advances each source revision once; reentry is refused"
            );
            let pos = crate::map::overlay::TilePos {
                x: at.0,
                y: at.1,
                floor: at.2,
            };
            assert_eq!(
                map.facts
                    .base_entry(pos, 1, 1)
                    .ok_or("life-site facts")?
                    .appearance_id,
                100,
                "life-site appearance stays unchanged"
            );
        }
        let counter = progress(&owner, &admitted, &authority, COUNTER).await?;
        eprintln!("FIRST_DRAGON_LIFE POSTGRES_EVIDENCE completed={completed} counter={counter}");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert_eq!(
            completed, 3,
            "all three literal source AIDs must record distinct life-site visits"
        );
        assert_eq!(counter, 3);
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
    super::quest_world_callbacks::wire(
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
        super::quest_world_callbacks::local(position(at)),
        "literal native movement enters source tile"
    );
    Ok(())
}

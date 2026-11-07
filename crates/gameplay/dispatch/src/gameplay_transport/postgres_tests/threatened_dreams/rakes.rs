//! Only the prior mission and carried rake are fixtures; all five scratches use native USE_WITH.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use super::barriers::{ToolSource, progress, quantity, relocate, source_instance, use_tool};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Passage, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::collections::BTreeMap;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.threatened_dreams_quest";
const BASE: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/";
const STONE: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/grumpy_stone";
const PREFIX: &str = "oteryn:quest-transition/questpc3/threatened-rake/";
const SEED: &str = "oteryn:quest-transition/fixture/threatened-rake-prior-story";
const READY: &str = "oteryn:quest-transition/fixture/threatened-rake-ready";
const RAKE: &str = "oteryn:item.tibia.i3452";
const TARGETS: [(u16, u16, i8); 5] = [
    (32617, 31863, -7),
    (32616, 31864, -7),
    (32615, 31866, -7),
    (32620, 31864, -7),
    (32619, 31865, -7),
];
const WRONG: (u16, u16, i8) = (32625, 31865, -7);
fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}
fn neighbor((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x: x - 1, y, floor }
}

#[test]
fn threatened_five_rake_scratches_are_exact_and_work_all_day() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_RAKES POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(||{
  let started=std::time::Instant::now();
  match std::panic::catch_unwind(scenario){
   Ok(result)=>{eprintln!("THREATENED_RAKES POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());result.map_err(|e|e.to_string())},
   Err(panic)=>{eprintln!("THREATENED_RAKES POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());std::panic::resume_unwind(panic)}
  }
 })?.join().map_err(|_|"rake scenario panicked")??;
    Ok(())
}
fn fixture_catalogue() -> TestResult<QuestStateCatalogue> {
    let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let mut tracks = BTreeMap::new();
    for key in std::iter::once(STONE.to_owned()).chain((1..=5).map(|i| format!("{BASE}stone{i}"))) {
        tracks.insert(
            key.clone(),
            source
                .catalogue()
                .track(&key)
                .ok_or("source track")?
                .clone(),
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
                .ok_or("source rake transition")?
                .clone(),
        );
    }
    for (key, value) in [(SEED, 0), (READY, 1)] {
        transitions.push(QuestTransition {
            key: key.into(),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: STONE.into(),
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
            Harness::create_with_admission_seed(admin.clone(), "threatened_rakes", false, false)
                .await?;
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
        let first = TARGETS[0];
        geometry.selector.position = Some(position(first));
        geometry.selector.action = Some(25012);
        geometry.selector.unique = None;
        geometry.selector.appearance = Some(100);
        geometry.lever = None;
        geometry.teleport = None;
        geometry.item_on_target = None;
        geometry.carried_use = None;
        // These fields construct fixture geometry, never grants or executable offerings.
        geometry.passages = TARGETS
            .into_iter()
            .chain([WRONG])
            .skip(1)
            .map(|target| Passage {
                position: position(target),
                appearance: 100,
                replacement: None,
            })
            .collect();
        geometry.offerings = TARGETS
            .into_iter()
            .chain([WRONG])
            .map(|target| Offering {
                position: neighbor(target),
                item: "oteryn:item.tibia.i25730".into(),
                count: 1,
            })
            .collect();
        let world = super::super::quest_world_callbacks::world(&geometry, position(first))?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let start = neighbor(first);
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
        super::super::item_look::seed_main_backpack(
            &h,
            &[(RAKE, 1), ("oteryn:item.tibia.i25730", 1)],
        )
        .await?;
        seed(&owner, &admitted, SEED, 31000).await?;
        let target_for = |at: (u16, u16, i8)| -> TestResult<WorldObjectTarget> {
            let key = oteryn_world_bundle::bundle::placement_key(at.2, at.0, at.1, 1)
                .ok_or("placement")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            Ok(WorldObjectTarget {
                placement,
                expected_revision: 0,
            })
        };
        let first_flag = format!("{BASE}stone1");
        owner
            .states
            .quest_callbacks
            .fixture_clock_ms
            .store(3_600_000, std::sync::atomic::Ordering::Relaxed);
        use_tool(
            &mut owner,
            &admitted,
            ToolSource::Handle(id(71)),
            target_for(TARGETS[0])?,
            31001,
        )
        .await?;
        assert_ne!(
            progress(&owner, &admitted, &authority, &first_flag).await?,
            1,
            "wrong mission stage cannot scratch"
        );
        assert_eq!(quantity(&h, RAKE).await?, 1);
        seed(&owner, &admitted, READY, 31002).await?;
        use_tool(
            &mut owner,
            &admitted,
            ToolSource::Handle(id(72)),
            target_for(TARGETS[0])?,
            31003,
        )
        .await?;
        assert_ne!(
            progress(&owner, &admitted, &authority, &first_flag).await?,
            1,
            "wrong carried tool cannot scratch"
        );
        relocate(&owner, &admitted, neighbor(WRONG)).await?;
        use_tool(
            &mut owner,
            &admitted,
            ToolSource::Handle(id(71)),
            target_for(WRONG)?,
            31004,
        )
        .await?;
        assert_ne!(
            progress(&owner, &admitted, &authority, &first_flag).await?,
            1,
            "another world position cannot scratch"
        );
        let source = id(71);
        let mut scratches = 0;
        for (i, at) in TARGETS.into_iter().enumerate() {
            relocate(&owner, &admitted, neighbor(at)).await?;
            let minute = [0, 360, 720, 1080, 1439][i];
            owner.states.quest_callbacks.fixture_clock_ms.store(
                3_600_000 + minute * 2_500,
                std::sync::atomic::Ordering::Relaxed,
            );
            let command = 31100 + u64::try_from(i)? * 10;
            let target = target_for(at)?;
            let result = use_tool(
                &mut owner,
                &admitted,
                ToolSource::Handle(source),
                target.clone(),
                command,
            )
            .await?;
            let flag = format!("{BASE}stone{}", i + 1);
            let worked = result == UseDisposition::Committed
                && progress(&owner, &admitted, &authority, &flag).await? == 1;
            if worked {
                scratches += 1;
                assert_eq!(
                    use_tool(
                        &mut owner,
                        &admitted,
                        ToolSource::Handle(source),
                        target.clone(),
                        command
                    )
                    .await?,
                    UseDisposition::Committed,
                    "exact old-instance receipt replay"
                );
                assert_eq!(
                    source_instance(&owner, &admitted, RAKE).await?,
                    source,
                    "successful scratching retains the same rake UUID"
                );
                use_tool(
                    &mut owner,
                    &admitted,
                    ToolSource::Handle(source),
                    target,
                    command + 1,
                )
                .await?;
                assert_eq!(
                    source_instance(&owner, &admitted, RAKE).await?,
                    source,
                    "repeat target cannot replace or consume the retained rake"
                );
            }
            assert_eq!(
                quantity(&h, RAKE).await?,
                1,
                "scratching never consumes the rake"
            );
            let key =
                oteryn_world_bundle::bundle::placement_key(at.2, at.0, at.1, 1).ok_or("target")?;
            assert_eq!(
                owner.bundle_map().ok_or("map")?.facts.object_revision(key),
                0,
                "per-player scratch never changes global target"
            );
        }
        assert_eq!(
            quantity(&h, "oteryn:item.tibia.i25730").await?,
            1,
            "wrong tool remains intact"
        );
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key LIKE 'oteryn:quest-transition/questpc3/threatened-rake/%'").fetch_one(&h.pool).await?;
        eprintln!("THREATENED_RAKES POSTGRES_EVIDENCE scratches={scratches} receipts={receipts}");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert_eq!(
            scratches, 5,
            "five source rake targets must each write its own flag at every sampled hour"
        );
        assert_eq!(receipts, 5);
        Ok(())
    })
}
async fn seed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
    command: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(command).map_err(debug)?,
                )),
            },
            Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

//! Three source beds exchange only the selected presents, with distinct flags and one shared counter.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{Dependencies, TestResult, prepare_current_session};
use super::barriers::{ToolSource, progress, quantity, relocate, source_instance, use_tool};
use super::rakes::seed;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Passage, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::collections::BTreeMap;
use std::sync::Arc;
const QUEST: &str = "oteryn:quest.threatened_dreams_quest";
const BASE: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission04/";
const PREFIX: &str = "oteryn:quest-transition/questpc3/threatened-present/";
const PRESENT: &str = "oteryn:item.tibia.i25302";
const TOOTH: &str = "oteryn:item.tibia.i25303";
const BEDS: [(u16, u16, i8); 3] = [(32391, 32220, -6), (33001, 32060, -5), (32328, 31794, -6)];
const FLAGS: [&str; 3] = ["bed_thais", "bed_venore", "bed_carlin"];
const WRONG: (u16, u16, i8) = (32395, 32220, -6);
fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}
fn neighbor((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x: x - 1, y, floor }
}
fn prior(value: i64) -> String {
    format!("oteryn:quest-transition/fixture/threatened-present/stage{value}")
}
#[test]
fn threatened_three_presents_exchange_selected_stack_once_per_bed() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_PRESENTS POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(||{let started=std::time::Instant::now();match std::panic::catch_unwind(scenario){Ok(result)=>{eprintln!("THREATENED_PRESENTS POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());result.map_err(|e|e.to_string())},Err(panic)=>{eprintln!("THREATENED_PRESENTS POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());std::panic::resume_unwind(panic)}}})?.join().map_err(|_|"presents scenario panicked")??;
    Ok(())
}
fn fixture_catalogue() -> TestResult<QuestStateCatalogue> {
    let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let mut tracks = BTreeMap::new();
    for (name, max) in [
        ("tooth_fairy", 3),
        ("presents_placed", 3),
        ("bed_thais", 1),
        ("bed_venore", 1),
        ("bed_carlin", 1),
    ] {
        let key = format!("{BASE}{name}");
        tracks.insert(
            key.clone(),
            source
                .catalogue()
                .track(&key)
                .cloned()
                .unwrap_or(QuestTrack {
                    key,
                    quest: QUEST.into(),
                    initial: if name.starts_with("bed_") { -1 } else { 0 },
                    min: -1,
                    max,
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
                .ok_or("present transition")?
                .clone(),
        );
    }
    for value in [0, 1] {
        transitions.push(QuestTransition {
world_change: None,
            key: prior(value),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: format!("{BASE}tooth_fairy"),
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
            Harness::create_with_admission_seed(admin.clone(), "threatened_presents", false, false)
                .await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("THREATENED_PRESENTS POSTGRES_CASE_STARTED server_version={version}");
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
        let first = BEDS[0];
        geometry.selector.position = Some(position(first));
        geometry.selector.action = None;
        geometry.selector.unique = None;
        geometry.selector.appearance = Some(100);
        geometry.lever = None;
        geometry.teleport = None;
        geometry.item_on_target = None;
        geometry.carried_use = None;
        // These fields construct fixture geometry, never grants or executable offerings.
        geometry.passages = BEDS[1..]
            .iter()
            .copied()
            .chain([WRONG])
            .map(|at| Passage {
                position: position(at),
                appearance: 100,
                replacement: None,
            })
            .collect();
        geometry.offerings = BEDS
            .into_iter()
            .chain([WRONG])
            .map(neighbor)
            .map(|at| Offering {
                position: at,
                item: PRESENT.into(),
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
        super::super::item_look::seed_main_backpack(&h, &[(PRESENT, 3)]).await?;
        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Quest witness", owner.owner_now());
        owner
            .states
            .quest_callbacks
            .fixture_clock_ms
            .store(3_600_000, std::sync::atomic::Ordering::Relaxed);
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
        seed(&owner, &admitted, &prior(0), 34000).await?;
        let source = source_instance(&owner, &admitted, PRESENT).await?;
        use_tool(
            &mut owner,
            &admitted,
            ToolSource::Handle(source),
            target_for(BEDS[0])?,
            34001,
        )
        .await?;
        assert_eq!(quantity(&h, PRESENT).await?, 3, "wrong stage never charges");
        seed(&owner, &admitted, &prior(1), 34002).await?;
        relocate(&owner, &admitted, neighbor(WRONG)).await?;
        use_tool(
            &mut owner,
            &admitted,
            ToolSource::Handle(source),
            target_for(WRONG)?,
            34003,
        )
        .await?;
        assert_eq!(quantity(&h, PRESENT).await?, 3, "wrong bed never charges");
        let mut completed = 0;
        for (i, at) in BEDS.into_iter().enumerate() {
            relocate(&owner, &admitted, neighbor(at)).await?;
            let target = target_for(at)?;
            let command = 34100 + u64::try_from(i)? * 10;
            let before = quantity(&h, PRESENT).await?;
            if i == 0 {
                owner
                    .states
                    .quest_callbacks
                    .interrupt_after_commit
                    .store(true, std::sync::atomic::Ordering::Release);
            }
            let result = use_tool(
                &mut owner,
                &admitted,
                ToolSource::Handle(source),
                target.clone(),
                command,
            )
            .await?;
            let result = if i == 0 {
                assert_eq!(
                    result,
                    UseDisposition::StaleState,
                    "only the acknowledgement is lost"
                );
                assert_eq!(quantity(&h, PRESENT).await?, before - 1);
                assert_eq!(quantity(&h, TOOTH).await?, 1);
                use_tool(
                    &mut owner,
                    &admitted,
                    ToolSource::Handle(source),
                    target.clone(),
                    command,
                )
                .await?
            } else {
                result
            };
            if result == UseDisposition::Committed
                && quantity(&h, PRESENT).await? == before - 1
                && quantity(&h, TOOTH).await? == i64::try_from(i + 1)?
                && progress(
                    &owner,
                    &admitted,
                    &authority,
                    &format!("{BASE}{}", FLAGS[i]),
                )
                .await?
                    == 1
            {
                completed += 1;
                assert_eq!(
                    progress(
                        &owner,
                        &admitted,
                        &authority,
                        &format!("{BASE}presents_placed")
                    )
                    .await?,
                    i64::try_from(i + 1)?
                );
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
                    "exact receipt replay"
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
                    quantity(&h, PRESENT).await?,
                    before - 1,
                    "same bed fresh command never charges"
                );
                assert_eq!(quantity(&h, TOOTH).await?, i64::try_from(i + 1)?);
            }
        }
        for at in BEDS {
            let key = oteryn_world_bundle::bundle::placement_key(at.2, at.0, at.1, 1)
                .ok_or("placement")?;
            assert_eq!(
                owner.bundle_map().ok_or("map")?.facts.object_revision(key),
                0,
                "bed stays unchanged"
            );
        }
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key LIKE 'oteryn:quest-transition/questpc3/threatened-present/%'").fetch_one(&h.pool).await?;
        let presents = quantity(&h, PRESENT).await?;
        let teeth = quantity(&h, TOOTH).await?;
        let counter = progress(
            &owner,
            &admitted,
            &authority,
            &format!("{BASE}presents_placed"),
        )
        .await?;
        eprintln!(
            "THREATENED_PRESENTS POSTGRES_EVIDENCE completed={completed} receipts={receipts} presents={presents} teeth={teeth} counter={counter}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert_eq!(
            completed, 3,
            "all three literal beds must consume selected presents and grant teeth"
        );
        assert_eq!(receipts, 3);
        assert_eq!(presents, 0);
        assert_eq!(teeth, 3);
        assert_eq!(counter, 3);
        Ok(())
    })
}

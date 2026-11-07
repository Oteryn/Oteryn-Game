//! Two source deliveries consume only their selected instances; existing tent callbacks are regression witnesses.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{Dependencies, TestResult, prepare_current_session};
use super::barriers::{ToolSource, progress, quantity, relocate, use_tool};
use super::rakes::seed;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Passage, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::collections::BTreeMap;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.threatened_dreams_quest";
const ANIMALS: &str = oteryn_gameplay_quests::quest::threatened_dreams::ANIMALS;
const ALIAS: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission01_1";
const PREFIX: &str = "oteryn:quest-transition/questpc3/threatened-burn/";
const BOOK: &str = "oteryn:item.tibia.i25235";
const FUR: &str = "oteryn:item.tibia.i25238";
const PALLETS: [(u16, u16, i8); 2] = [(32650, 31870, -7), (32660, 31870, -7)];
const STONE: (u16, u16, i8) = (32670, 31870, -7);
const WRONG: (u16, u16, i8) = (32680, 31870, -7);
const TENTS: [(u16, u16, i8); 3] = [(32949, 31811, -7), (32950, 31811, -7), (32951, 31811, -7)];
fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}
fn neighbor((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x: x - 1, y, floor }
}
fn north((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y: y - 1, floor }
}
fn prior(value: i64) -> String {
    format!("oteryn:quest-transition/fixture/threatened-delivery/stage{value}")
}
#[test]
fn threatened_book_and_fur_burn_exact_sources_and_preserve_tent_steps() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_DELIVERY POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(||{let started=std::time::Instant::now();match std::panic::catch_unwind(scenario){
 Ok(result)=>{eprintln!("THREATENED_DELIVERY POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());result.map_err(|e|e.to_string())},
 Err(panic)=>{eprintln!("THREATENED_DELIVERY POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());std::panic::resume_unwind(panic)}
 }} )?.join().map_err(|_|"delivery scenario panicked")??;
    Ok(())
}
fn fixture_catalogue() -> TestResult<QuestStateCatalogue> {
    let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let mut tracks = BTreeMap::new();
    for key in [ANIMALS, ALIAS] {
        tracks.insert(
            key.to_owned(),
            source.catalogue().track(key).ok_or("source track")?.clone(),
        );
    }
    let mut transitions = Vec::new();
    for c in oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .filter(|c| {
            c.key.starts_with(PREFIX)
                || c.key
                    .starts_with("oteryn:quest-callback/threatened-dreams-poacher-discovery-")
        })
    {
        transitions.push(
            source
                .catalogue()
                .transition(&c.key)
                .ok_or("source transition")?
                .clone(),
        );
    }
    for value in [0, 2, 6, 8] {
        transitions.push(QuestTransition {
world_change: None,
            key: prior(value),
            quest: QUEST.into(),
            effects: [ANIMALS, ALIAS]
                .into_iter()
                .map(|track| QuestEffect {
                    track: track.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(value),
                })
                .collect(),
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
            Harness::create_with_admission_seed(admin.clone(), "threatened_delivery", false, false)
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
        let first = PALLETS[0];
        geometry.selector.position = Some(position(first));
        geometry.selector.action = Some(25012);
        geometry.selector.unique = None;
        geometry.selector.appearance = Some(12648);
        geometry.lever = None;
        geometry.teleport = None;
        geometry.item_on_target = None;
        geometry.carried_use = None;
        // These fields construct fixture geometry, never grants or executable offerings.
        geometry.passages = [(PALLETS[1], 12649), (STONE, 25237), (WRONG, 100)]
            .into_iter()
            .map(|(at, appearance)| Passage {
                position: position(at),
                appearance,
                replacement: None,
            })
            .collect();
        geometry.offerings = PALLETS
            .into_iter()
            .chain([STONE, WRONG])
            .map(neighbor)
            .chain(TENTS.into_iter().flat_map(|at| [position(at), north(at)]))
            .map(|at| Offering {
                position: at,
                item: BOOK.into(),
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
        super::super::item_look::seed_main_backpack(
            &h,
            &[(BOOK, 1), (BOOK, 1), (BOOK, 1), (FUR, 1), (FUR, 1)],
        )
        .await?;
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
        seed(&owner, &admitted, &prior(0), 32000).await?;
        use_tool(
            &mut owner,
            &admitted,
            ToolSource::Handle(id(71)),
            target_for(PALLETS[0])?,
            32001,
        )
        .await?;
        assert_eq!(
            quantity(&h, BOOK).await?,
            3,
            "wrong stage cannot debit a selected book"
        );
        seed(&owner, &admitted, &prior(2), 32002).await?;
        use_tool(
            &mut owner,
            &admitted,
            ToolSource::Handle(id(74)),
            target_for(PALLETS[0])?,
            32003,
        )
        .await?;
        assert_eq!(
            quantity(&h, FUR).await?,
            2,
            "wrong selected source cannot be charged"
        );
        relocate(&owner, &admitted, neighbor(WRONG)).await?;
        use_tool(
            &mut owner,
            &admitted,
            ToolSource::Handle(id(71)),
            target_for(WRONG)?,
            32004,
        )
        .await?;
        assert_eq!(
            quantity(&h, BOOK).await?,
            3,
            "wrong selected target cannot be charged"
        );
        let mut burns = 0;
        for (i, at) in PALLETS.into_iter().enumerate() {
            seed(&owner, &admitted, &prior(2), 32100 + u64::try_from(i)? * 10).await?;
            relocate(&owner, &admitted, neighbor(at)).await?;
            owner.states.quest_callbacks.fixture_clock_ms.store(
                3_600_000 + u64::try_from(i)? * 720 * 2_500,
                std::sync::atomic::Ordering::Relaxed,
            );
            let command = 32101 + u64::try_from(i)? * 10;
            let source = id(71 + u8::try_from(i)?);
            let target = target_for(at)?;
            let before = quantity(&h, BOOK).await?;
            let result = use_tool(
                &mut owner,
                &admitted,
                ToolSource::Handle(source),
                target.clone(),
                command,
            )
            .await?;
            let worked = result == UseDisposition::Committed
                && progress(&owner, &admitted, &authority, ANIMALS).await? == 3
                && quantity(&h, BOOK).await? == before - 1;
            if worked {
                burns += 1;
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
                    "receipt replay after selected source is gone"
                );
                assert_eq!(
                    use_tool(
                        &mut owner,
                        &admitted,
                        ToolSource::Handle(id(73)),
                        target.clone(),
                        command
                    )
                    .await?,
                    UseDisposition::Rejected,
                    "different source under the original cause conflicts"
                );
                use_tool(
                    &mut owner,
                    &admitted,
                    ToolSource::Handle(id(73)),
                    target,
                    command + 1,
                )
                .await?;
                assert_eq!(
                    quantity(&h, BOOK).await?,
                    before - 1,
                    "fresh repeat cannot consume the reserved book"
                );
            }
        }
        seed(&owner, &admitted, &prior(8), 32200).await?;
        relocate(&owner, &admitted, neighbor(STONE)).await?;
        owner.states.quest_callbacks.fixture_clock_ms.store(
            3_600_000 + 1439 * 2_500,
            std::sync::atomic::Ordering::Relaxed,
        );
        let target = target_for(STONE)?;
        let result = use_tool(
            &mut owner,
            &admitted,
            ToolSource::Handle(id(74)),
            target.clone(),
            32201,
        )
        .await?;
        let fur = result == UseDisposition::Committed
            && progress(&owner, &admitted, &authority, ANIMALS).await? == 9
            && quantity(&h, FUR).await? == 1;
        if fur {
            burns += 1;
            assert_eq!(
                use_tool(
                    &mut owner,
                    &admitted,
                    ToolSource::Handle(id(74)),
                    target.clone(),
                    32201
                )
                .await?,
                UseDisposition::Committed
            );
            use_tool(
                &mut owner,
                &admitted,
                ToolSource::Handle(id(75)),
                target,
                32202,
            )
            .await?;
            assert_eq!(
                quantity(&h, FUR).await?,
                1,
                "repeat cannot debit the spare fur"
            );
        }
        for at in PALLETS.into_iter().chain([STONE]) {
            let key =
                oteryn_world_bundle::bundle::placement_key(at.2, at.0, at.1, 1).ok_or("target")?;
            assert_eq!(
                owner.bundle_map().ok_or("map")?.facts.object_revision(key),
                0,
                "donor targets have no configured decay transition"
            );
        }
        let mut tents = 0;
        for (i, at) in TENTS.into_iter().enumerate() {
            seed(&owner, &admitted, &prior(6), 32300 + u64::try_from(i)? * 10).await?;
            relocate(&owner, &admitted, north(at)).await?;
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
                &owner,
                &mut admitted,
                1 + u64::try_from(i)?,
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
                "literal movement enters the source tent tile"
            );
            if progress(&owner, &admitted, &authority, ANIMALS).await? == 7
                && progress(&owner, &admitted, &authority, ALIAS).await? == 7
            {
                tents += 1;
            }
        }
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key LIKE 'oteryn:quest-transition/questpc3/threatened-burn/%'").fetch_one(&h.pool).await?;
        eprintln!(
            "THREATENED_DELIVERY POSTGRES_EVIDENCE burns={burns} receipts={receipts} tent_steps={tents}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert_eq!(
            tents, 3,
            "all three existing tent step callbacks must remain wired"
        );
        assert_eq!(
            burns, 3,
            "both pallet variants and the weeping stone must consume only selected sources"
        );
        assert_eq!(receipts, 3);
        Ok(())
    })
}

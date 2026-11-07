//! Four distinct trophy burns admit the two literal lava-pool directions.
use super::fixture::{Harness, debug, id, run};
use super::threatened_dreams::barriers::{
    ToolSource, progress, quantity, source_instance, use_tool,
};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_quests::quest::world_callbacks::{Passage, Position};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::collections::BTreeMap;
use std::sync::Arc;
const QUEST: &str = "oteryn:quest.the_first_dragon_quest";
const BASE: &str = "oteryn:quest-progress/quest/u11_02/the_first_dragon/";
const PREFIX: &str = "oteryn:quest-transition/questpc3/first-dragon-lava/";
const ITEMS: [&str; 4] = [
    "oteryn:item.tibia.i24939",
    "oteryn:item.tibia.i24940",
    "oteryn:item.tibia.i24941",
    "oteryn:item.tibia.i24942",
];
const FLAGS: [&str; 4] = ["scale", "tooth", "horn", "bones"];
const OUTER: (u16, u16, i8) = (33047, 32712, -3);
const INNER: (u16, u16, i8) = (31994, 32390, -9);
const INSIDE: (u16, u16, i8) = (31994, 32391, -9);
const OUTSIDE: (u16, u16, i8) = (33047, 32713, -3);
const WRONG: (u16, u16, i8) = (33046, 32711, -3);
fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}
fn neighbor((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x: x - 1, y, floor }
}
#[test]
fn first_dragon_four_distinct_sacrifices_admit_both_lava_directions() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("FIRST_DRAGON_LAVA POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let started = std::time::Instant::now();
            let result = scenario();
            eprintln!(
                "FIRST_DRAGON_LAVA POSTGRES_CASE_COMPLETED duration_ms={} success={}",
                started.elapsed().as_millis(),
                result.is_ok()
            );
            result.map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "dragon lava scenario panicked")??;
    Ok(())
}
fn fixture_catalogue() -> TestResult<QuestStateCatalogue> {
    let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let mut tracks = BTreeMap::new();
    for name in FLAGS.into_iter().chain(["access_cave"]) {
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
                    initial: -1,
                    min: -1,
                    max: 1,
                }),
        );
    }
    let mut transitions = Vec::new();
    for c in oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .filter(|c| c.key.starts_with(PREFIX))
    {
        transitions.push(
            source
                .catalogue()
                .transition(&c.key)
                .ok_or("lava transition")?
                .clone(),
        );
    }
    QuestStateCatalogue::new("content-1", tracks.into_values().collect(), transitions)
        .map_err(|e| debug(e))
}
fn scenario() -> TestResult {
    let catalogue = Arc::new(fixture_catalogue()?);
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin.clone(), "first_dragon_lava", false, false)
                .await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("FIRST_DRAGON_LAVA POSTGRES_CASE_STARTED server_version={version}");
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
        let first = OUTER;
        geometry.selector.position = Some(position(first));
        geometry.selector.action = None;
        geometry.selector.unique = None;
        geometry.selector.appearance = Some(25160);
        geometry.lever = None;
        geometry.teleport = None;
        geometry.item_on_target = None;
        geometry.carried_use = None;
        // These fields construct fixture geometry, never grants or executable offerings.
        geometry.passages = [
            (INNER, 25160),
            (WRONG, 25160),
            (INSIDE, 100),
            (OUTSIDE, 100),
        ]
        .into_iter()
        .map(|(at, appearance)| Passage {
            position: position(at),
            appearance,
            replacement: None,
        })
        .collect();
        geometry.offerings.clear();
        let world = super::quest_world_callbacks::world(&geometry, position(first))?;
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
        admitted.item_fence = Some(fence.item_fence());
        let entries = ITEMS
            .iter()
            .flat_map(|item| [(*item, 1), (*item, 1)])
            .collect::<Vec<_>>();
        super::item_look::seed_main_backpack(&h, &entries).await?;
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
        let plain = |at| {
            oteryn_protocol_oteryn::world_object::encode_use_intent(&target_for(at)?).map_err(debug)
        };
        let initial =
            super::quest_world_callbacks::wire(&owner, &mut admitted, 1, 2, plain(OUTER)?).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&initial).map_err(debug)?,
            UseDisposition::Rejected,
            "unearned lava access must reject"
        );
        let source = source_instance(&owner, &admitted, ITEMS[0]).await?;
        use_tool(
            &mut owner,
            &admitted,
            ToolSource::Handle(source),
            target_for(WRONG)?,
            38000,
        )
        .await?;
        assert_eq!(
            quantity(&h, ITEMS[0]).await?,
            2,
            "wrong target never charges"
        );
        let mut completed = 0;
        for i in 0..4 {
            let source = source_instance(&owner, &admitted, ITEMS[i]).await?;
            let command = 38100 + u64::try_from(i)? * 10;
            let result = use_tool(
                &mut owner,
                &admitted,
                ToolSource::Handle(source),
                target_for(OUTER)?,
                command,
            )
            .await?;
            if result == UseDisposition::Committed
                && quantity(&h, ITEMS[i]).await? == 1
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
                    progress(&owner, &admitted, &authority, &format!("{BASE}access_cave")).await?,
                    i64::try_from(i + 1)?
                );
                assert_eq!(
                    use_tool(
                        &mut owner,
                        &admitted,
                        ToolSource::Handle(source),
                        target_for(OUTER)?,
                        command
                    )
                    .await?,
                    UseDisposition::Committed,
                    "exact consumed-source replay"
                );
                let spare = source_instance(&owner, &admitted, ITEMS[i]).await?;
                use_tool(
                    &mut owner,
                    &admitted,
                    ToolSource::Handle(spare),
                    target_for(OUTER)?,
                    command + 1,
                )
                .await?;
                assert_eq!(
                    quantity(&h, ITEMS[i]).await?,
                    1,
                    "duplicate trophy never charges"
                );
            }
            if i == 2 {
                let reply =
                    super::quest_world_callbacks::wire(&owner, &mut admitted, 2, 2, plain(OUTER)?)
                        .await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&reply)
                        .map_err(debug)?,
                    UseDisposition::Rejected,
                    "three trophies are insufficient"
                );
            }
        }
        let entry =
            super::quest_world_callbacks::wire(&owner, &mut admitted, 3, 2, plain(OUTER)?).await?;
        let entered = oteryn_protocol_oteryn::world_object::decode_use_result(&entry)
            .map_err(debug)?
            == UseDisposition::Committed;
        let mut portals = 0;
        if entered {
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                super::quest_world_callbacks::local(position(INSIDE))
            );
            portals += 1;
            let exit =
                super::quest_world_callbacks::wire(&owner, &mut admitted, 4, 2, plain(INNER)?)
                    .await?;
            if oteryn_protocol_oteryn::world_object::decode_use_result(&exit).map_err(debug)?
                == UseDisposition::Committed
            {
                assert_eq!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    super::quest_world_callbacks::local(position(OUTSIDE))
                );
                portals += 1;
            }
        }
        for at in [OUTER, INNER] {
            let key = oteryn_world_bundle::bundle::placement_key(at.2, at.0, at.1, 1)
                .ok_or("placement")?;
            assert_eq!(
                owner.bundle_map().ok_or("map")?.facts.object_revision(key),
                0,
                "lava source never transforms"
            );
        }
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key LIKE 'oteryn:quest-transition/questpc3/first-dragon-lava/%'").fetch_one(&h.pool).await?;
        let counter =
            progress(&owner, &admitted, &authority, &format!("{BASE}access_cave")).await?;
        eprintln!(
            "FIRST_DRAGON_LAVA POSTGRES_EVIDENCE sacrifices={completed} portals={portals} receipts={receipts} counter={counter}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert_eq!(
            completed, 4,
            "all four source trophy sacrifices must atomically consume and flag"
        );
        assert_eq!(portals, 2, "both literal lava directions must work");
        assert_eq!(receipts, 4);
        assert_eq!(counter, 4);
        Ok(())
    })
}

//! Prior charged tools/story are fixtures; all fifteen repair writes use native USE_WITH.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Passage, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::collections::BTreeMap;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.threatened_dreams_quest";
const BASE: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/";
const NIGHTMARE: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/nightmare_intruders";
const PREFIX: &str = "oteryn:quest-transition/questpc3/threatened-barrier/";
const SEED: &str = "oteryn:quest-transition/fixture/threatened-barrier-prior-story-and-tools";

#[derive(Clone, Copy)]
pub(in crate::gameplay_transport) enum ToolSource {
    Handle([u8; 16]),
    Definition(std::num::NonZeroU32),
}

struct Family {
    name: &'static str,
    charged: u32,
    empty: u32,
    counter: &'static str,
    flag: &'static str,
    day: bool,
    targets: [(u16, u16, i8); 5],
}
const FAMILIES: [Family; 3] = [
    Family {
        name: "moon",
        charged: 25730,
        empty: 25729,
        counter: "charged_moon_mirror",
        flag: "moon_mirror_pos",
        day: false,
        targets: [
            (33386, 32215, -7),
            (33511, 32320, -6),
            (33518, 32193, -7),
            (33549, 32219, -7),
            (33597, 32182, -7),
        ],
    },
    Family {
        name: "star",
        charged: 25732,
        empty: 25731,
        counter: "charged_starlight_vial",
        flag: "starlight_pos",
        day: false,
        targets: [
            (33442, 32201, -7),
            (33458, 32299, -7),
            (33562, 32256, -7),
            (33541, 32244, -7),
            (33529, 32187, -7),
        ],
    },
    Family {
        name: "sun",
        charged: 25734,
        empty: 25733,
        counter: "charged_sun_catcher",
        flag: "sun_catcher_pos",
        day: true,
        targets: [
            (33419, 32242, -7),
            (33484, 32192, -7),
            (33546, 32155, -7),
            (33547, 32206, -7),
            (33568, 32243, -7),
        ],
    },
];
fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}
fn neighbor((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x: x - 1, y, floor }
}

#[test]
fn threatened_fifteen_distinct_barriers_decrement_exact_carried_tools() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_BARRIERS POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        let started = std::time::Instant::now();
        match std::panic::catch_unwind(scenario) {
            Ok(result) => {
                eprintln!("THREATENED_BARRIERS POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());
                result.map_err(|error|error.to_string())
            }
            Err(panic) => {
                eprintln!("THREATENED_BARRIERS POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());
                std::panic::resume_unwind(panic)
            }
        }
    })?.join().map_err(|_|"barrier scenario panicked")??;
    Ok(())
}

fn fixture_catalogue() -> TestResult<QuestStateCatalogue> {
    let original = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let mut tracks = BTreeMap::new();
    tracks.insert(
        NIGHTMARE.to_owned(),
        original
            .catalogue()
            .track(NIGHTMARE)
            .ok_or("nightmare track")?
            .clone(),
    );
    let mut seed = vec![(NIGHTMARE.to_owned(), 7)];
    for family in &FAMILIES {
        let counter = format!("{BASE}{}", family.counter);
        tracks.insert(
            counter.clone(),
            original
                .catalogue()
                .track(&counter)
                .ok_or("counter track")?
                .clone(),
        );
        seed.push((counter, 5));
        for number in 1..=5 {
            let key = format!("{BASE}{}{number:02}", family.flag);
            tracks.insert(
                key.clone(),
                original
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
    }
    let mut transitions = Vec::new();
    for callback in oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .filter(|callback| callback.key.starts_with(PREFIX))
    {
        transitions.push(
            original
                .catalogue()
                .transition(&callback.key)
                .ok_or("barrier transition")?
                .clone(),
        );
    }
    transitions.push(QuestTransition {
        key: SEED.into(),
        quest: QUEST.into(),
        effects: seed
            .into_iter()
            .map(|(track, value)| QuestEffect {
                track,
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(value),
            })
            .collect(),
        completes: false,
        experience: None,
    });
    QuestStateCatalogue::new("content-1", tracks.into_values().collect(), transitions)
        .map_err(|error| format!("{error:?}").into())
}

fn scenario() -> TestResult {
    // Invalid fixture effects fail before capture and are never counted as real PG RED.
    let catalogue = Arc::new(fixture_catalogue()?);
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin.clone(), "threatened_barriers", false, false)
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
        let first = FAMILIES[0].targets[0];
        geometry.selector.position = Some(position(first));
        geometry.selector.action = Some(25012);
        geometry.selector.unique = None;
        geometry.selector.appearance = Some(100);
        geometry.lever = None;
        geometry.teleport = None;
        geometry.item_on_target = None;
        geometry.carried_use = None;
        // These fields construct fixture geometry, never grants or executable offerings.
        geometry.passages = FAMILIES
            .iter()
            .flat_map(|family| family.targets)
            .skip(1)
            .map(|target| Passage {
                position: position(target),
                appearance: 100,
                replacement: None,
            })
            .collect();
        geometry.offerings = FAMILIES
            .iter()
            .flat_map(|family| family.targets)
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
        let keys: Vec<_> = FAMILIES
            .iter()
            .map(|family| format!("oteryn:item.tibia.i{}", family.charged))
            .collect();
        let entries: Vec<_> = keys.iter().map(|key| (key.as_str(), 1)).collect();
        super::super::item_look::seed_main_backpack(&h, &entries).await?;
        seed_prior(&owner, &admitted).await?;
        let mut successes = Vec::new();
        for (family_index, family) in FAMILIES.iter().enumerate() {
            let counter = format!("{BASE}{}", family.counter);
            let instance = source_instance(&owner, &admitted, &keys[family_index]).await?;
            let source = if family.name == "star" {
                let runtime = owner.runtime.lock().await;
                let index = crate::gameplay_transport::item_ref_admission::pinned_index(
                    &runtime,
                    owner.active_generation,
                )
                .ok_or("pinned index")?;
                let key = &keys[family_index];
                ToolSource::Definition(
                    index
                        .definition_ref("Item", key, index.revision(key).ok_or("source revision")?)
                        .ok_or("source index")?,
                )
            } else {
                ToolSource::Handle(instance)
            };
            for (index, target_position) in family.targets.iter().copied().enumerate() {
                relocate(&owner, &admitted, neighbor(target_position)).await?;
                let key = oteryn_world_bundle::bundle::placement_key(
                    target_position.2,
                    target_position.0,
                    target_position.1,
                    1,
                )
                .ok_or("target placement")?;
                let mut placement = world.base().digest().to_vec();
                placement.extend_from_slice(&key.to_be_bytes());
                let target = WorldObjectTarget {
                    placement,
                    expected_revision: 0,
                };
                let command = 29000 + u64::try_from(family_index * 100 + index * 10)?;
                let before = progress(&owner, &admitted, &authority, &counter).await?;
                clock(&owner, !family.day);
                use_tool(&mut owner, &admitted, source, target.clone(), command).await?;
                assert_eq!(
                    progress(&owner, &admitted, &authority, &counter).await?,
                    before,
                    "wrong period cannot spend a charge"
                );
                clock(&owner, family.day);
                let interrupted = family_index == 0 && index == 0;
                if interrupted {
                    owner
                        .states
                        .quest_callbacks
                        .interrupt_after_commit
                        .store(true, std::sync::atomic::Ordering::Release);
                }
                let mut outcome =
                    use_tool(&mut owner, &admitted, source, target.clone(), command + 1).await?;
                if interrupted {
                    assert_eq!(
                        outcome,
                        UseDisposition::StaleState,
                        "lose only the committed target acknowledgement"
                    );
                    assert!(owner.has_pending_quest_callback(admitted.game_session_id));
                    assert!(
                        owner
                            .reconcile_quest_callback_receipts(admitted.game_session_id)
                            .await
                    );
                    assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
                    owner
                        .refresh_quest_session(admitted.game_session_id, false)
                        .await;
                    outcome = use_tool(&mut owner, &admitted, source, target.clone(), command + 1)
                        .await?;
                }
                let flag = format!("{BASE}{}{:02}", family.flag, index + 1);
                let after = progress(&owner, &admitted, &authority, &counter).await?;
                let repaired = outcome == UseDisposition::Committed
                    && after == before - 1
                    && progress(&owner, &admitted, &authority, &flag).await? == 1;
                if repaired {
                    clock(&owner, !family.day);
                    assert_eq!(
                        use_tool(&mut owner, &admitted, source, target.clone(), command + 1)
                            .await?,
                        UseDisposition::Committed,
                        "exact historical target replay ignores the later clock"
                    );
                    clock(&owner, family.day);
                    let next = family.targets[(index + 1) % 5];
                    let next_key =
                        oteryn_world_bundle::bundle::placement_key(next.2, next.0, next.1, 1)
                            .ok_or("other target")?;
                    let mut different = target.clone();
                    let offset = different
                        .placement
                        .len()
                        .checked_sub(8)
                        .ok_or("target encoding")?;
                    different.placement[offset..].copy_from_slice(&next_key.to_be_bytes());
                    assert_eq!(
                        use_tool(&mut owner, &admitted, source, different, command + 1).await?,
                        UseDisposition::Rejected,
                        "another world target under the committed cause conflicts"
                    );
                }
                use_tool(&mut owner, &admitted, source, target, command + 2).await?;
                assert_eq!(
                    progress(&owner, &admitted, &authority, &counter).await?,
                    after,
                    "each distinct target spends at most one charge"
                );
                assert_eq!(
                    owner.bundle_map().ok_or("map")?.facts.object_revision(key),
                    0,
                    "repair never changes the global object revision"
                );
                successes.push(repaired);
            }
            eprintln!(
                "THREATENED_BARRIERS POSTGRES_FAMILY_REACHED family={} counter={} empty={}",
                family.name,
                progress(&owner, &admitted, &authority, &counter).await?,
                quantity(&h, &format!("oteryn:item.tibia.i{}", family.empty)).await?
            );
        }
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key LIKE 'oteryn:quest-transition/questpc3/threatened-barrier/%'").fetch_one(&h.pool).await?;
        let repaired = successes.iter().filter(|value| **value).count();
        eprintln!(
            "THREATENED_BARRIERS POSTGRES_ASSERTION_REACHED repairs={repaired} receipts={receipts}"
        );
        for family in &FAMILIES {
            if repaired == 15 {
                assert_eq!(
                    quantity(&h, &format!("oteryn:item.tibia.i{}", family.empty)).await?,
                    1,
                    "only the terminal charge restores each exact tool"
                );
                assert_eq!(
                    quantity(&h, &format!("oteryn:item.tibia.i{}", family.charged)).await?,
                    0
                );
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert_eq!(
            repaired, 15,
            "all fifteen source targets must commit distinct per-player repairs"
        );
        assert_eq!(receipts, 15);
        Ok(())
    })
}

fn clock(owner: &ComposedFreshAdmission<'_, '_, '_>, day: bool) {
    owner.states.quest_callbacks.fixture_clock_ms.store(
        3_600_000 + if day { 720 * 2_500 } else { 0 },
        std::sync::atomic::Ordering::Relaxed,
    );
}
pub(in crate::gameplay_transport) async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    to: Position,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    if expected.position() == super::super::quest_world_callbacks::local(to) {
        return Ok(());
    }
    let map = owner.bundle_map().ok_or("map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        owner.movement_cells,
        actor,
        admitted.game_session_id,
        expected,
        super::super::quest_world_callbacks::local(to),
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}
pub(in crate::gameplay_transport) async fn use_tool(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    source: ToolSource,
    target: WorldObjectTarget,
    command: u64,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: command,
        item_fence: Some(fence.item_fence()),
    };
    let outcome = match source {
        ToolSource::Handle(instance) => {
            owner
                .use_fluid_map(actor, command, ItemKey::Instance(instance), target)
                .await
        }
        ToolSource::Definition(definition) => {
            owner
                .use_quest_tool_on_map_by_definition(actor, command, definition, target)
                .await
        }
    };
    Ok(outcome.map_or(UseDisposition::Rejected, |outcome| outcome.disposition))
}
pub(in crate::gameplay_transport) async fn source_instance(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
) -> TestResult<[u8; 16]> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence.item_fence())
        .await
        .map_err(debug)?;
    inventory
        .backpack
        .as_ref()
        .ok_or("backpack")?
        .entries
        .iter()
        .find(|entry| entry.item.definition.production_key == key)
        .map(|entry| entry.item.item_instance_id)
        .ok_or_else(|| "charged source".into())
}
pub(in crate::gameplay_transport) async fn progress(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    track: &str,
) -> TestResult<i64> {
    super::super::classic_quests::progress(owner, admitted, authority, track).await
}
pub(in crate::gameplay_transport) async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(key).fetch_one(&h.pool).await?)
}
async fn seed_prior(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
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
                transition_key: SEED.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(28000).map_err(debug)?,
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

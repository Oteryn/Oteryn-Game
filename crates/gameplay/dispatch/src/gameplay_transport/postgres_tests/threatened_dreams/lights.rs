//! Prior story milestones are fixtures; physical hand-ins and rewards use the real NPC owner.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::UseCommand;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use std::collections::BTreeMap;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.threatened_dreams_quest";
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;
const NIGHTMARE: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/nightmare_intruders";
const SEED: &str = "oteryn:quest-transition/fixture/threatened-light-prior-story";
const FINISH: &str = "oteryn:quest-transition/fixture/threatened-light-prior-finish";
#[test]
fn threatened_six_light_transform_branches_are_fenced() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_LIGHTS POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        let started=std::time::Instant::now();
        match std::panic::catch_unwind(scenario) {
            Ok(result) => {
                eprintln!("THREATENED_LIGHTS POSTGRES_CASE_COMPLETED duration_ms={} success={}", started.elapsed().as_millis(), result.is_ok());
                result.map_err(|error|error.to_string())
            }
            Err(panic) => {
                eprintln!("THREATENED_LIGHTS POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());
                std::panic::resume_unwind(panic)
            }
        }
    })?.join().map_err(|_|"inventory scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let cases = [
            (
                "moon",
                25729,
                25730,
                25975,
                33518,
                32209,
                -7,
                false,
                "charged_moon_mirror",
            ),
            (
                "star",
                25731,
                25732,
                25976,
                33526,
                32298,
                -4,
                false,
                "charged_starlight_vial",
            ),
            (
                "sun",
                25733,
                25734,
                25977,
                33508,
                32163,
                -7,
                true,
                "charged_sun_catcher",
            ),
        ];
        let mut successes = Vec::new();
        for (name, source, charged, cosmetic, x, y, floor, day, counter_name) in cases {
            let h = Harness::create_with_admission_seed(
                admin.clone(),
                "threatened_lights",
                false,
                false,
            )
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
            let trigger = Position { x, y, floor };
            let mut map_callback = oteryn_gameplay_quests::quest::world_callbacks::load()
                .map_err(debug)?
                .into_iter()
                .next()
                .ok_or("map callback")?;
            map_callback.selector.position = Some(trigger);
            map_callback.selector.action = Some(25012);
            map_callback.selector.unique = None;
            map_callback.selector.appearance = None;
            map_callback.passages.clear();
            map_callback.offerings.clear();
            map_callback.teleport = None;
            map_callback.lever = None;
            map_callback.item_on_target = None;
            let world = super::super::quest_world_callbacks::world(&map_callback, trigger)?;
            let cells = world
                .movement_cells(activated.qualified_room().movement_cells())
                .map_err(debug)?;
            let pin = ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                activated.frame_binding().digest(),
                world.map_revision_digest(),
                (i32::from(x), i32::from(y), -i16::from(floor)),
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
                    super::super::quest_world_callbacks::local(trigger),
                )
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = &cells;
            let original =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            let counter = format!(
                "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/{counter_name}"
            );
            let initial_counter = original
                .catalogue()
                .track(&counter)
                .ok_or("counter track")?
                .initial;
            let mut tracks = BTreeMap::new();
            for key in [NIGHTMARE, &counter] {
                tracks.insert(
                    key.to_owned(),
                    original
                        .catalogue()
                        .track(key)
                        .ok_or("source track")?
                        .clone(),
                );
            }
            let mut transitions = Vec::new();
            for suffix in ["gather", "cosmetic"] {
                let key =
                    format!("oteryn:quest-transition/questpc3/threatened-light/{name}-{suffix}");
                if let Some(transition) = original.catalogue().transition(&key) {
                    transitions.push(transition.clone());
                }
            }
            for (key, values) in [
                (SEED, vec![(NIGHTMARE, 6)]),
                (FINISH, vec![(NIGHTMARE, 8), (counter.as_str(), 0)]),
            ] {
                transitions.push(QuestTransition {
world_change: None,
                    key: key.into(),
                    quest: QUEST.into(),
                    effects: values
                        .into_iter()
                        .map(|(track, value)| QuestEffect {
                            track: track.into(),
                            from: QuestComparison::Any,
                            effect: QuestEffectKind::Set(value),
                        })
                        .collect(),
                    completes: false,
                    experience: None,
                });
            }
            owner.quest_catalogue = Some(Arc::new(
                QuestStateCatalogue::new("content-1", tracks.into_values().collect(), transitions)
                    .map_err(debug)?,
            ));
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
            let input = format!("oteryn:item.tibia.i{source}");
            let output = format!("oteryn:item.tibia.i{charged}");
            let final_item = format!("oteryn:item.tibia.i{cosmetic}");
            super::super::item_look::seed_main_backpack(&h, &[(&input, 1), (&input, 1)]).await?;
            fixture_step(&owner, &admitted, SEED, 27000).await?;
            let wrong_minute = if day { 0 } else { 720 };
            owner.states.quest_callbacks.fixture_clock_ms.store(
                3_600_000 + wrong_minute * 2_500,
                std::sync::atomic::Ordering::Relaxed,
            );
            use_tool(&mut owner, &admitted, source, 27001).await?;
            assert_eq!(
                quantity(&h, &input).await?,
                2,
                "wrong period cannot transform the source"
            );
            assert_eq!(
                super::super::classic_quests::progress(&owner, &admitted, &authority, &counter)
                    .await?,
                initial_counter
            );
            let minute = if day { 720 } else { 0 };
            owner.states.quest_callbacks.fixture_clock_ms.store(
                3_600_000 + minute * 2_500,
                std::sync::atomic::Ordering::Relaxed,
            );
            if name == "moon" {
                owner
                    .states
                    .quest_callbacks
                    .interrupt_after_commit
                    .store(true, std::sync::atomic::Ordering::Release);
            }
            let first = use_tool(&mut owner, &admitted, source, 27002).await?;
            if name == "moon" {
                assert_eq!(
                    first,
                    UseDisposition::StaleState,
                    "fixture loses only the commit acknowledgement"
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
            } else {
                assert_eq!(first, UseDisposition::Committed);
            }
            owner.states.quest_callbacks.fixture_clock_ms.store(
                3_600_000 + wrong_minute * 2_500,
                std::sync::atomic::Ordering::Relaxed,
            );
            assert_eq!(
                use_tool(&mut owner, &admitted, source, 27002).await?,
                UseDisposition::Committed,
                "receipt replay precedes the new clock fact"
            );
            assert_eq!(
                use_tool(&mut owner, &admitted, charged, 27002).await?,
                UseDisposition::Rejected,
                "a changed source selection conflicts with that receipt"
            );
            owner.states.quest_callbacks.fixture_clock_ms.store(
                3_600_000 + minute * 2_500,
                std::sync::atomic::Ordering::Relaxed,
            );
            let primary = quantity(&h, &output).await? == 1
                && super::super::classic_quests::progress(&owner, &admitted, &authority, &counter)
                    .await?
                    == 5;
            fixture_step(&owner, &admitted, FINISH, 27003).await?;
            let second = use_tool(&mut owner, &admitted, source, 27004).await?;
            let cosmetic_ok = second == UseDisposition::Committed
                && quantity(&h, &final_item).await? == 1
                && super::super::classic_quests::progress(&owner, &admitted, &authority, &counter)
                    .await?
                    == 0;
            let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key LIKE 'oteryn:quest-transition/questpc3/threatened-light/%'").fetch_one(&h.pool).await?;
            eprintln!(
                "THREATENED_LIGHTS POSTGRES_ASSERTION_REACHED tool={name} primary={primary} cosmetic={cosmetic_ok} receipts={receipts}"
            );
            successes.push(primary && cosmetic_ok && receipts == 2);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
        }
        assert_eq!(
            successes,
            [true, true, true],
            "all six physical transform branches must commit"
        );
        Ok(())
    })
}

async fn fixture_step(
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
async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(key).fetch_one(&h.pool).await?)
}
async fn use_tool(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    source: u32,
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
    let source_index = {
        let runtime = owner.runtime.lock().await;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            owner.active_generation,
        )
        .ok_or("pinned item index")?;
        let key = format!("oteryn:item.tibia.i{source}");
        index
            .definition_ref("Item", &key, index.revision(&key).ok_or("source revision")?)
            .ok_or("source index")?
    };
    Ok(owner
        .use_item_definition(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command,
                item_fence: Some(fence.item_fence()),
            },
            source_index,
        )
        .await
        .disposition)
}

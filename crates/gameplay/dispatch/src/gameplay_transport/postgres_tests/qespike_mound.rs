//! Crystal 201189f8 data-global/scripts/quests/spike_tasks/actions_spirit_shovel.lua:1-43.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::{connection::UseCommand, item_view::ItemKey};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};
use serde_json::json;
use std::{num::NonZeroU64, sync::Arc, time::Duration};

pub(super) const NAME: &str = "qespike-mound";
const QUEST: &str = "oteryn:quest.spike_task";
const TRACK: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_upper_mound_main";
const TOOL: &str = "oteryn:item.tibia.i3003";
const SOUL: &str = "oteryn:creature.enraged_soul";
const MOUND: Position = Position {
    x: 32210,
    y: 32540,
    floor: -8,
};

#[test]
fn spike_mound_draw_transforms_restores_spawns_and_debits_only_on_enraged_soul() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, MOUND, false)
    })
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_callbacks::source_world_spatial(
        (MOUND.x - 1, MOUND.y, MOUND.floor),
        &[
            (MOUND.x, MOUND.y, MOUND.floor, 0, 19211),
            (MOUND.x + 1, MOUND.y, MOUND.floor, 0, 19212),
        ],
    )
}

fn bindings() -> TestResult<Bindings> {
    let source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/spike_tasks/actions_spirit_shovel.lua:1-43";
    let stages = [
        ("empty", 1, 49, None, "You've found nothing special."),
        ("zombie", 50, 69, Some("oteryn:creature.zombie"), "You unearthed some not-so-death creature."),
        ("skeleton", 70, 79, Some("oteryn:creature.demon_skeleton"), "You unearthed some not-so-death creature."),
        ("ghost", 80, 89, Some("oteryn:creature.ghost"), "Your crude digging has angered some ancient ghost."),
        ("enraged-soul", 90, 100, Some(SOUL), "You unearthed a spirit's anger!!!"),
    ].map(|(name, minimum, maximum, spawn, message)| {
        let winner = spawn == Some(SOUL);
        let mut draw = json!({"minimum":minimum,"maximum":maximum,"effect":35});
        if let Some(creature) = spawn { draw["spawn"] = json!(creature); }
        if winner { draw["follow_up"] = json!("Report the task to Gnomilly."); }
        json!({"key":name,"completes":false,"trigger":{"kind":"map_use","callback":{
            "key":format!("oteryn:quest-callback/qespike/mound-{name}"),"quest":QUEST,"trigger":"use",
            "selector":{"appearance":19211},"reversible":false,"return_on_failure":false,
            "passages":[],"offerings":[],"requires_world_admission":true,
            "storage":[{"track":TRACK,"minimum":0,"maximum":0,"set":i64::from(winner),"preserve":!winner}],
            "item_on_target":{"item":TOOL,"count":1,"selected_instance":true,"reusable":!winner},
            "effects":{"tool_use_draw":draw,"transform":19212,"reset_after_ms":120000,"transform_requires_current":true},
            "success_message":message,"sources":[source]
        }}})
    });
    Ok(Bindings::parse(json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":"cff7d45a40c4302abea54b2e4860cfeafa4d44229e922216ddd2eebd1bd3e6dd",
        "sources":[source],"tracks":[{"key":TRACK,"initial":0,"minimum":-1,"maximum":1}],
        "stages":stages
    }]}).to_string().as_bytes())?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let generation = owner.active_generation.ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native")?;
    let seed = generation.identity().server_artifact_digest();
    let base = QuestStateCatalogue::empty("content-1").ok_or("quest catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings()?.attach(&base)?));
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let session = admitted.game_session_id;
    let actor = admitted.runtime_actor.ok_or("digger")?;
    owner.refresh_quest_session(session, false).await;
    super::item_look::seed_main_backpack(harness, &[(TOOL, 1)]).await?;
    let map = owner.bundle_map().ok_or("mound map")?;
    let pos = TilePos {
        x: MOUND.x,
        y: MOUND.y,
        floor: MOUND.floor,
    };
    let key = crate::map::view::placement_key(pos, 1).ok_or("mound placement")?;
    let native_mound = *map
        .overlay
        .base()
        .tile(pos.x, pos.y, pos.floor)
        .ok_or("mound tile")?
        .ids()
        .get(1)
        .ok_or("mound item")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let appearance = |owner: &ComposedFreshAdmission<'_, '_, '_>| {
        owner
            .bundle_map()
            .and_then(|m| m.facts.base_entry(pos, 1, native_mound))
            .map(|entry| entry.appearance_id)
    };
    let mut next_command = admitted.continuity.next_command_id;
    for (minimum, maximum) in [(1, 49), (90, 100)] {
        let command_id = (next_command..next_command + 2048)
            .find(|command| {
                (minimum..=maximum).contains(
                    &crate::gameplay_transport::quest_callbacks::tool_use_roll(
                        seed, session, *command, key,
                    ),
                )
            })
            .ok_or("source outcome interval")?;
        next_command = command_id + 1;
        let map = owner.bundle_map().ok_or("mound map")?;
        let intent = UseIntent {
            target: UseTarget::Item(NonZeroU64::MIN),
            use_with: None,
            use_with_position: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement: placement.clone(),
                expected_revision: map.facts.object_revision(key),
            })),
        };
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("quest fence")?;
        let result = owner
            .tame_mount_impl(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: Some(fence.item_fence()),
                },
                intent,
                Some(ItemKey::Instance(id(71))),
                None,
                true,
            )
            .await
            .map_err(debug)?
            .ok_or("map tool USE")?;
        assert_eq!(result.disposition, UseDisposition::Committed);
        assert_eq!(appearance(owner), Some(19212));
        let progress: i64 = sqlx::query_scalar(
            "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
        )
        .bind(TRACK)
        .fetch_one(&harness.pool)
        .await?;
        let debits: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key LIKE 'oteryn:quest-callback/qespike/mound-%'")
            .fetch_one(&harness.pool).await?;
        if maximum == 49 {
            assert_eq!((progress, debits), (0, 0));
            owner.clock_origin -= Duration::from_secs(121);
            let shutdown = oteryn_foundation::CancellationToken::new();
            let (_, restored) = tokio::join!(owner.run_source_owner_cycles(&shutdown), async {
                let result = tokio::time::timeout(Duration::from_secs(3), async {
                    while appearance(owner) != Some(19211) {
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    }
                })
                .await;
                shutdown.cancel();
                result
            });
            restored?;
        } else {
            assert_eq!((progress, debits), (1, 1));
            let runtime = owner.runtime.lock().await;
            let creatures = runtime.positioned_melee_census().map_err(debug)?;
            assert!(creatures.iter().any(|(actor, snapshot, player)| {
                player.is_none()
                    && snapshot.position() == super::quest_world_callbacks::local(MOUND)
                    && runtime
                        .companion_snapshot(*actor)
                        .is_ok_and(|creature| creature.state.policy.definition_key == SOUL)
            }));
        }
    }
    Ok(())
}

//! Pilgrimage of Ashes is the terminal control while Make Believe is held.

use super::classic_quests_fixtures::{install_with_offsets, talk, talk_expecting};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::starter_grant::StarterTemplate;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_quests::quest::authored_bindings::{Bindings, Trigger};
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state_with_bindings;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

const REVISION: &str = "content-1";

#[test]
fn qbind_terminal_pilgrimage_final_report_completes() -> TestResult {
    journey("qbind_terminal_pilgrimage_final_report_completes", false)
}

#[test]
fn qbind_terminal_pilgrimage_replay_no_second_reward_or_receipt() -> TestResult {
    journey(
        "qbind_terminal_pilgrimage_replay_no_second_reward_or_receipt",
        true,
    )
}

fn journey(name: &'static str, replay: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        run(async move |admin| {
            let h = Harness::create_quest_reward(admin, "qbind_terminal", false).await?;
            eprintln!("qbindplan-marker: {name} db ready");
            let result = exercise(&h, replay).await;
            h.cleanup().await?;
            result
        })
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn exercise(h: &Harness, replay: bool) -> TestResult {
    let report: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/quest-authoring/samples/server-completion/qbind-admission.json"
    )))?;
    let bindings = Bindings::parse_documents(&[&report["admitted"][0]["bindings"].to_string()])?;
    let quest = bindings.quests.first().ok_or("control quest")?;
    let callbacks = quest
        .stages
        .iter()
        .map(|stage| match &stage.trigger {
            Trigger::NpcDialogue {
                callback,
                alternatives,
            } if alternatives.is_empty() => Ok(callback),
            _ => Err("expected unambiguous NPC dialogue"),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (terminal, visits) = callbacks.split_last().ok_or("terminal callback")?;
    let reward = terminal.give.as_ref().ok_or("terminal reward")?;
    let track = quest.tracks.first().ok_or("route track")?;

    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, REVISION)?;
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native generation")?;
    let frame = pin.frame_binding().digest();
    let room = pin.qualified_room().clone();
    let start = pin.entry_start();
    let deps = Dependencies::with_content_and_capacity(
        pin.into_channel_pin(),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        visits.len() + 1,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_pinned_test_position(
            deps.actor,
            crate::foundation::MovementLocalPosition {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        )
        .map_err(debug)?;
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    let lowered = load_embedded_quest_state_with_bindings(REVISION, &bindings).map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.quest_catalogue = Some(Arc::new(lowered.catalogue().clone()));
    owner.premium_coordinator = Some(&premium);
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest session")?;
    admitted.item_fence = Some(fence.item_fence());
    let keys: [&str; 7] = visits
        .iter()
        .map(|callback| callback.npc.as_str())
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "seven registered pilgrimage visits")?;
    // This scenario uses the native entry room's unsigned placement origin.
    let offsets = [(1, 0), (2, 0), (0, 1), (1, 1), (2, 1), (0, 2), (1, 2)];
    let actors = install_with_offsets(&owner, &admitted, keys, Some(offsets)).await?;

    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
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
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &NativeItemWeights(native),
            &StarterTemplate {
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "oteryn:item.tibia.i2854".into(),
                    revision_ref: "definition-r1".into(),
                },
                ..StarterTemplate::main()
            },
        )
        .await
        .map_err(debug)?;
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;
    owner.admit_premium(id(40), admitted.game_session_id);
    owner.admit_quest_session(&admitted).await;

    let character = fence.character_id.as_bytes();
    let initial = outcome(h, character, &quest.quest, &track.key, &reward.item).await?;
    for (visit, (callback, npc)) in visits.iter().zip(actors).enumerate() {
        let command = 1100 + u64::try_from(visit)? * 3;
        talk(&mut owner, &admitted, npc, command, "hi").await?;
        assert_eq!(
            talk(
                &mut owner,
                &admitted,
                npc,
                command + 1,
                &callback.keywords[0]
            )
            .await?,
            callback.reply,
        );
        talk(&mut owner, &admitted, npc, command + 2, "bye").await?;
    }
    let before = outcome(h, character, &quest.quest, &track.key, &reward.item).await?;
    assert_eq!(
        before,
        (
            track.maximum - 1,
            None,
            i64::try_from(visits.len())?,
            0,
            initial.4
        )
    );

    let guide = actors[keys
        .iter()
        .position(|key| *key == terminal.npc)
        .ok_or("return NPC")?];
    let report_command = 1201;
    talk(&mut owner, &admitted, guide, report_command - 1, "hi").await?;
    assert_eq!(
        talk(
            &mut owner,
            &admitted,
            guide,
            report_command,
            &terminal.keywords[0]
        )
        .await?,
        terminal.reply,
    );
    let completed = outcome(h, character, &quest.quest, &track.key, &reward.item).await?;
    assert_eq!(completed.0, track.maximum);
    assert!(
        completed.1.is_some(),
        "final report must set durable completion"
    );
    assert_eq!(completed.2, before.2 + 1);
    assert_eq!(completed.3, 1);
    assert_eq!(completed.4, before.4 + i64::from(reward.count));
    if replay {
        // Repeat the same command, then report again with a fresh command ID.
        for command in [report_command, report_command + 1] {
            talk_expecting(
                &mut owner,
                &admitted,
                guide,
                command,
                &terminal.keywords[0],
                NpcIntentDisposition::Rejected,
            )
            .await?;
            assert_eq!(
                outcome(h, character, &quest.quest, &track.key, &reward.item).await?,
                completed
            );
        }
    }
    Ok(())
}

async fn outcome(
    h: &Harness,
    character: &[u8],
    quest: &str,
    track: &str,
    item: &str,
) -> TestResult<(i64, Option<String>, i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT
            COALESCE((SELECT value FROM game_character_quest_tracks
                WHERE quest_key=$1 AND track_key=$2 AND character_id=encode($4,'hex')::uuid),0),
            (SELECT completed_character_revision::text FROM game_character_quest_states
                WHERE quest_key=$1 AND character_id=encode($4,'hex')::uuid),
            (SELECT count(*) FROM game_character_quest_receipts
                WHERE quest_key=$1 AND character_id=encode($4,'hex')::uuid),
            (SELECT count(*) FROM game_character_quest_receipts
                WHERE quest_key=$1 AND character_id=encode($4,'hex')::uuid AND completes),
            COALESCE((SELECT sum(items.quantity)::bigint FROM game_item_instances items
                JOIN game_item_container_entries entries USING (item_instance_id)
                WHERE items.definition_production_key=$3
                AND entries.character_id=encode($4,'hex')::uuid),0)",
    )
    .bind(quest)
    .bind(track)
    .bind(item)
    .bind(character)
    .fetch_one(&h.pool)
    .await?)
}

//! Physical tutorial rewards and XP share the real NPC/Quest transaction.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_beginning_quest";
const BASE: &str = "oteryn:quest-progress/quest/u8_2/the_beginning_quest/";
const LEGS: &str = "oteryn:item.tibia.i7882";
const FISH: &str = "oteryn:item.tibia.i3578";

#[test]
fn questpc3_beginning_physical_rewards_and_xp_are_atomic() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUESTPC3_BEGINNING_REWARDS POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(Scenario::Rewards).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Beginning rewards scenario panicked")??;
    Ok(())
}

#[test]
fn questpc3_beginning_coat_equipment_and_club_are_atomic() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUESTPC3_BEGINNING_EQUIPMENT POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        let started = std::time::Instant::now();
        match std::panic::catch_unwind(|| scenario(Scenario::Equipment)) {
            Ok(result) => {
                eprintln!("QUESTPC3_BEGINNING_EQUIPMENT POSTGRES_CASE_COMPLETED duration_ms={} success={}", started.elapsed().as_millis(), result.is_ok());
                result.map_err(|error| error.to_string())
            }
            Err(panic) => {
                eprintln!("QUESTPC3_BEGINNING_EQUIPMENT POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true", started.elapsed().as_millis());
                std::panic::resume_unwind(panic)
            }
        }
    })?.join().map_err(|_| "Beginning equipment scenario panicked")??;
    Ok(())
}

#[derive(Clone, Copy)]
enum Scenario {
    Rewards,
    Equipment,
    Damage,
    CarlosLessons,
    CarlosFood,
}

#[test]
fn questpc3_beginning_damage_lesson_is_once_and_recovers_lost_ack() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUESTPC3_BEGINNING_DAMAGE POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        let started = std::time::Instant::now();
        match std::panic::catch_unwind(|| scenario(Scenario::Damage)) {
            Ok(result) => {
                eprintln!("QUESTPC3_BEGINNING_DAMAGE POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());
                result.map_err(|error|error.to_string())
            }
            Err(panic) => {
                eprintln!("QUESTPC3_BEGINNING_DAMAGE POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());
                std::panic::resume_unwind(panic)
            }
        }
    })?.join().map_err(|_|"damage lesson panicked")??;
    Ok(())
}

#[test]
fn questpc3_beginning_carlos_five_lessons_use_native_dialogue() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUESTPC3_CARLOS_LESSONS POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let started = std::time::Instant::now();
            let result = std::panic::catch_unwind(|| scenario(Scenario::CarlosLessons));
            eprintln!(
                "QUESTPC3_CARLOS_LESSONS POSTGRES_CASE_COMPLETED duration_ms={} success={}",
                started.elapsed().as_millis(),
                matches!(&result, Ok(Ok(())))
            );
            match result {
                Ok(result) => result.map_err(|error| error.to_string()),
                Err(panic) => std::panic::resume_unwind(panic),
            }
        })?
        .join()
        .map_err(|_| "Carlos lessons panicked")??;
    Ok(())
}

#[test]
fn questpc3_beginning_carlos_accepts_either_food_without_consumption() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUESTPC3_CARLOS_FOOD POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let started = std::time::Instant::now();
            let result = std::panic::catch_unwind(|| scenario(Scenario::CarlosFood));
            eprintln!(
                "QUESTPC3_CARLOS_FOOD POSTGRES_CASE_COMPLETED duration_ms={} success={}",
                started.elapsed().as_millis(),
                matches!(&result, Ok(Ok(())))
            );
            match result {
                Ok(result) => result.map_err(|error| error.to_string()),
                Err(panic) => std::panic::resume_unwind(panic),
            }
        })?
        .join()
        .map_err(|_| "Carlos food panicked")??;
    Ok(())
}

fn scenario(mode: Scenario) -> TestResult {
    let started = std::time::Instant::now();
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "questpc3_beginning_rewards", false, false)
                .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("generation")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content_and_capacity(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            8,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                crate::foundation::MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        let original = loaded.catalogue();
        // Only prior lessons/delivery are fixture milestones. The physical
        // hand-in, fish MINT, XP and their source successors must execute below.
        let tracks: Vec<_> = [
            "santiago_npc_greet_storage",
            "santiago_quest_log",
            "zirella_npc_greet_storage",
            "zirella_quest_log",
            "carlos_npc_greet_storage",
            "carlos_quest_log",
            "carlos_npc_trade_storage",
        ]
        .into_iter()
        .map(|name| {
            let key = format!("{BASE}{name}");
            original.track(&key).cloned().unwrap_or(QuestTrack {
                key,
                quest: QUEST.into(),
                initial: -1,
                min: -1,
                max: 13,
            })
        })
        .collect();
        let mut transitions = oteryn_gameplay_quests::quest::reward_callbacks::load()
            .map_err(debug)?
            .iter()
            .filter(|c| c.quest == QUEST)
            .map(|c| {
                original
                    .transition(&c.key)
                    .cloned()
                    .ok_or("source transition")
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (phase, npc, greet, log) in [
            ("equipment", "santiago", 3, 2),
            ("damage", "santiago", 9, 7),
            ("legs", "santiago", 6, 4),
            ("fish", "santiago", 10, 8),
            ("zirella", "zirella", 7, 7),
        ] {
            transitions.push(QuestTransition {
                key: format!("oteryn:quest-transition/fixture/beginning-reward/{phase}"),
                quest: QUEST.into(),
                effects: [
                    (format!("{npc}_npc_greet_storage"), greet),
                    (format!("{npc}_quest_log"), log),
                ]
                .into_iter()
                .map(|(name, value)| QuestEffect {
                    track: format!("{BASE}{name}"),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(value),
                })
                .collect(),
                completes: false,
                experience: None,
            });
        }
        let fixture_tracks = tracks.clone();
        let fixture_transitions = transitions.clone();
        owner.quest_catalogue = Some(Arc::new(
            QuestStateCatalogue::new("content-1", tracks, transitions).map_err(debug)?,
        ));
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        super::item_look::seed_main_backpack(&h, &[(LEGS, 1), (LEGS, 1)]).await?;
        let leg_instances: Vec<i64> = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1 ORDER BY item_instance_id").bind(LEGS).fetch_all(&h.pool).await?;
        assert_eq!(
            leg_instances,
            vec![1, 1],
            "source legs are distinct nonstackable Items"
        );
        if matches!(mode, Scenario::CarlosLessons | Scenario::CarlosFood) {
            let [carlos] =
                super::classic_quests::install(&owner, &admitted, ["oteryn:npc.carlos"]).await?;
            let items_before: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1")
                    .fetch_one(&h.pool)
                    .await?;
            super::classic_quests::talk(&mut owner, &admitted, carlos, 4600, "hi").await?;
            for stage in 1..=5 {
                let greet = super::classic_quests::progress(
                    &owner,
                    &admitted,
                    &authority,
                    &format!("{BASE}carlos_npc_greet_storage"),
                )
                .await?;
                let log = super::classic_quests::progress(
                    &owner,
                    &admitted,
                    &authority,
                    &format!("{BASE}carlos_quest_log"),
                )
                .await?;
                eprintln!(
                    "QUESTPC3_CARLOS_LESSONS POSTGRES_ASSERTION_REACHED stage={} actual_greet={} actual_log={} duration_ms={}",
                    stage,
                    greet,
                    log,
                    started.elapsed().as_millis()
                );
                assert_eq!(
                    greet, stage,
                    "native Carlos dialogue must author the lesson milestone"
                );
                assert_eq!(log, stage);
                if stage < 5 {
                    super::classic_quests::talk(
                        &mut owner,
                        &admitted,
                        carlos,
                        4600 + u64::try_from(stage)? * 10,
                        "yes",
                    )
                    .await?;
                    if stage == 1 {
                        super::classic_quests::talk(&mut owner, &admitted, carlos, 4611, "bye")
                            .await?;
                        super::classic_quests::talk(&mut owner, &admitted, carlos, 4612, "hi")
                            .await?;
                    }
                }
            }
            super::classic_quests::talk(&mut owner, &admitted, carlos, 4660, "bye").await?;
            super::classic_quests::talk(&mut owner, &admitted, carlos, 4661, "hi").await?;
            assert_eq!(
                super::classic_quests::progress(
                    &owner,
                    &admitted,
                    &authority,
                    &format!("{BASE}carlos_npc_greet_storage")
                )
                .await?,
                5,
                "return greeting cannot repeat a completed lesson"
            );
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-reward/questpc3/beginning-carlos/%'").fetch_one(&h.pool).await?;
            assert_eq!(
                receipts, 5,
                "all five lessons must commit once through the existing quest writer"
            );
            let items_after: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                items_after, items_before,
                "outfit and hunt instructions grant no physical items"
            );
            if matches!(mode, Scenario::CarlosFood) {
                for (index, input) in [None, Some((3578, 115_u8)), Some((3582, 116_u8))]
                    .into_iter()
                    .enumerate()
                {
                    if let Some((ident, instance)) = input {
                        let mut tx = h.pool.begin().await?;
                        // Only fixture input creation uses the established seed path.
                        // Real NPC callbacks execute after this transaction commits.
                        sqlx::query("SET LOCAL session_replication_role=replica")
                            .execute(&mut *tx)
                            .await?;
                        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
                            .bind(id(instance).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{ident}")).bind(id(81).as_slice()).execute(&mut *tx).await?;
                        sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
                            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(70).as_slice()).bind(id(instance).as_slice()).bind((index + 2).to_string()).bind(id(81).as_slice()).execute(&mut *tx).await?;
                        tx.commit().await?;
                    }
                    assert_eq!(
                        quantity(&h, "oteryn:item.tibia.i3577").await?,
                        0,
                        "only the second source alternative is carried"
                    );
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_secs(2))
                        .ok_or("clock")?;
                    let fence = owner
                        .current_quest_fence(admitted.game_session_id)
                        .await
                        .map_err(|()| "fence")?
                        .ok_or("fence")?;
                    let result = owner
                        .npc_talk(
                            admitted.runtime_actor.ok_or("actor")?,
                            UseCommand {
                                game_session_id: admitted.game_session_id,
                                command_id: 4680 + u64::try_from(index)?,
                                item_fence: Some(fence.item_fence()),
                            },
                            NpcTalkIntent {
                                npc_actor: carlos,
                                text: "yes".into(),
                            },
                        )
                        .await;
                    assert!(!result.unknown);
                    let positive = index == 2;
                    let greet = super::classic_quests::progress(
                        &owner,
                        &admitted,
                        &authority,
                        &format!("{BASE}carlos_npc_greet_storage"),
                    )
                    .await?;
                    let log = super::classic_quests::progress(
                        &owner,
                        &admitted,
                        &authority,
                        &format!("{BASE}carlos_quest_log"),
                    )
                    .await?;
                    let trade = super::classic_quests::progress(
                        &owner,
                        &admitted,
                        &authority,
                        &format!("{BASE}carlos_npc_trade_storage"),
                    )
                    .await?;
                    eprintln!(
                        "QUESTPC3_CARLOS_FOOD POSTGRES_ASSERTION_REACHED case={} greet={} log={} trade={} duration_ms={}",
                        index,
                        greet,
                        log,
                        trade,
                        started.elapsed().as_millis()
                    );
                    assert_eq!(
                        greet,
                        if positive { 6 } else { 5 },
                        "ham alone must satisfy the second source food alternative; empty/fish must not advance"
                    );
                    assert_eq!(log, if positive { 6 } else { 5 });
                    if positive {
                        assert_eq!(trade, 1);
                        assert_eq!(result.disposition, NpcIntentDisposition::Ok);
                    } else {
                        assert!(trade < 1);
                    }
                    let items: i64 = sqlx::query_scalar(
                        "SELECT count(*) FROM game_item_instances WHERE lifecycle=1",
                    )
                    .fetch_one(&h.pool)
                    .await?;
                    assert_eq!(
                        items,
                        items_before + i64::try_from(index)?,
                        "read-only food evidence neither mints nor consumes an item"
                    );
                    if index > 0 {
                        assert_eq!(quantity(&h, FISH).await?, 1);
                    }
                    if positive {
                        assert_eq!(quantity(&h, "oteryn:item.tibia.i3582").await?, 1);
                    }
                }
                super::classic_quests::talk(&mut owner, &admitted, carlos, 4685, "yes").await?;
                let receipts: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
                )
                .bind("oteryn:quest-reward/questpc3/beginning-carlos/food-evidence")
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(
                    receipts, 1,
                    "repeating confirmation cannot create another eligibility receipt"
                );
                assert_eq!(quantity(&h, "oteryn:item.tibia.i3582").await?, 1);
            }
            return Ok(());
        }
        let [santiago, zirella] = super::classic_quests::install(
            &owner,
            &admitted,
            ["oteryn:npc.santiago", "oteryn:npc.zirella"],
        )
        .await?;
        if matches!(mode, Scenario::Damage) {
            let actor = admitted.runtime_actor.ok_or("actor")?;
            for (command_id, lost_ack) in [(4400, false), (4500, true)] {
                // Only the earlier lessons are fixture state. The HP loss,
                // successor, receipt and acknowledgement recovery execute below.
                let fence = owner
                    .current_quest_fence(admitted.game_session_id)
                    .await
                    .map_err(|()| "fence")?
                    .ok_or("fence")?;
                let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                let seeded = slot
                    .commit_quest_transition(
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        QuestTransitionRequest {
                            transition_key:
                                "oteryn:quest-transition/fixture/beginning-reward/damage".into(),
                            cause: QuestCause::Command(CommandRef::new(
                                admitted.game_session_id,
                                CommandId::new(command_id).map_err(debug)?,
                            )),
                        },
                        owner.quest_catalogue.clone().ok_or("catalogue")?,
                    )
                    .await
                    .map_err(debug)?;
                assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
                drop(slot);
                owner
                    .refresh_quest_session(admitted.game_session_id, false)
                    .await;
                super::classic_quests::talk(&mut owner, &admitted, santiago, command_id + 1, "hi")
                    .await?;
                let before = owner
                    .observe_actor_vitals(actor, admitted.game_session_id)
                    .await
                    .ok_or("vitals")?
                    .1;
                assert!(before.health > 20, "healthy source lesson fixture");
                let fence = owner
                    .current_quest_fence(admitted.game_session_id)
                    .await
                    .map_err(|()| "fence")?
                    .ok_or("fence")?;
                let command = UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command_id + 2,
                    item_fence: Some(fence.item_fence()),
                };
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                if lost_ack {
                    owner
                        .states
                        .npc_conversations
                        .fixture_interrupt_quest_dialogue_commit(
                            admitted.game_session_id,
                            command_id + 2,
                        )?;
                }
                let result = owner
                    .npc_talk(
                        actor,
                        command,
                        NpcTalkIntent {
                            npc_actor: santiago,
                            text: "yes".into(),
                        },
                    )
                    .await;
                let after = owner
                    .observe_actor_vitals(actor, admitted.game_session_id)
                    .await
                    .ok_or("vitals")?
                    .1;
                eprintln!(
                    "QUESTPC3_BEGINNING_DAMAGE POSTGRES_ASSERTION_REACHED lost_ack={} before_hp={} after_hp={} duration_ms={}",
                    lost_ack,
                    before.health,
                    after.health,
                    started.elapsed().as_millis()
                );
                assert_eq!(
                    after.health,
                    before.health - 20,
                    "source addHealth(-20) must execute through the existing vitals owner"
                );
                assert_eq!(
                    after.mana, before.mana,
                    "unshielded lesson has no mana debit"
                );
                for (name, value) in [
                    ("santiago_npc_greet_storage", 10),
                    ("santiago_quest_log", 8),
                ] {
                    assert_eq!(
                        super::classic_quests::progress(
                            &owner,
                            &admitted,
                            &authority,
                            &format!("{BASE}{name}")
                        )
                        .await?,
                        value
                    );
                }
                if lost_ack {
                    assert!(result.unknown && result.conversation.is_none());
                    assert!(
                        owner
                            .resume_npc_bank(
                                actor,
                                UseCommand {
                                    command_id: command_id + 3,
                                    ..command
                                }
                            )
                            .await
                            .is_none(),
                        "another command cannot settle the retained cause"
                    );
                    let replay = owner
                        .resume_npc_bank(actor, command)
                        .await
                        .ok_or("retained damage reply")?;
                    assert_eq!(replay.disposition, NpcIntentDisposition::Ok);
                    assert!(!replay.unknown);
                    assert!(owner.resume_npc_bank(actor, command).await.is_none());
                } else {
                    assert_eq!(result.disposition, NpcIntentDisposition::Ok);
                    assert!(!result.unknown);
                }
                let settled = owner
                    .observe_actor_vitals(actor, admitted.game_session_id)
                    .await
                    .ok_or("vitals")?
                    .1;
                assert_eq!(
                    settled.health, after.health,
                    "receipt/reply replay cannot apply HP loss again"
                );
                assert_eq!(settled.mana, after.mana);
                let receipts: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
                )
                .bind("oteryn:quest-reward/questpc3/beginning/damage-lesson")
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(receipts, if lost_ack { 2 } else { 1 });
                super::classic_quests::talk(&mut owner, &admitted, santiago, command_id + 4, "bye")
                    .await?;
            }
            return Ok(());
        }
        if matches!(mode, Scenario::Equipment) {
            let armor_slot = oteryn_protocol_oteryn::item_view::durable_slot(
                oteryn_protocol_oteryn::item_view::EquipmentSlot::Armor,
            )
            .ok_or("canonical armor slot")?;
            let clothing =
                oteryn_gameplay_quests::quest::reward_callbacks::load().map_err(debug)?;
            for name in ["coat-check-3", "coat-check-4"] {
                let key = format!("oteryn:quest-reward/questpc3/beginning/{name}");
                let callback = clothing
                    .iter()
                    .find(|callback| callback.key == key)
                    .ok_or("clothing callback")?;
                assert_eq!(
                    callback.equipped_slots,
                    vec![armor_slot],
                    "use canonical durable TORSO slot"
                );
            }
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let prepared = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key:
                            "oteryn:quest-transition/fixture/beginning-reward/equipment".into(),
                        cause: QuestCause::Command(CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(4300).map_err(debug)?,
                        )),
                    },
                    owner.quest_catalogue.clone().ok_or("catalogue")?,
                )
                .await
                .map_err(debug)?;
            assert!(matches!(prepared, QuestTransitionOutcome::Committed(_)));
            drop(slot);
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
            super::classic_quests::talk(&mut owner, &admitted, santiago, 4301, "hi").await?;
            // No armor and no coat cannot advance the source clothing check.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            let refused = owner
                .npc_talk(
                    admitted.runtime_actor.ok_or("actor")?,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 4302,
                        item_fence: Some(fence.item_fence()),
                    },
                    NpcTalkIntent {
                        npc_actor: santiago,
                        text: "yes".into(),
                    },
                )
                .await;
            eprintln!(
                "QUESTPC3_BEGINNING_EQUIPMENT POSTGRES_ASSERTION_REACHED missing_gear_reply={:?} duration_ms={}",
                refused.disposition,
                started.elapsed().as_millis()
            );
            assert_eq!(refused.disposition, NpcIntentDisposition::Rejected);
            assert_eq!(
                super::classic_quests::progress(
                    &owner,
                    &admitted,
                    &authority,
                    &format!("{BASE}santiago_npc_greet_storage")
                )
                .await?,
                3
            );
            eprintln!(
                "QUESTPC3_BEGINNING_EQUIPMENT POSTGRES_ASSERTION_REACHED missing_gear duration_ms={}",
                started.elapsed().as_millis()
            );
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            for (ident, instance, ordinal) in [(3562, 84, 3), (3567, 85, 4)] {
                sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
                    .bind(id(instance).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{ident}")).bind(id(81).as_slice()).execute(&mut *tx).await?;
                if ident == 3562 {
                    sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
                        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(70).as_slice()).bind(id(instance).as_slice()).bind(ordinal.to_string()).bind(id(81).as_slice()).execute(&mut *tx).await?;
                    tx.commit().await?;
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_secs(2))
                        .ok_or("clock")?;
                    let fence = owner
                        .current_quest_fence(admitted.game_session_id)
                        .await
                        .map_err(|()| "fence")?
                        .ok_or("fence")?;
                    let refused = owner
                        .npc_talk(
                            admitted.runtime_actor.ok_or("actor")?,
                            UseCommand {
                                game_session_id: admitted.game_session_id,
                                command_id: 4303,
                                item_fence: Some(fence.item_fence()),
                            },
                            NpcTalkIntent {
                                npc_actor: santiago,
                                text: "yes".into(),
                            },
                        )
                        .await;
                    assert_eq!(
                        refused.disposition,
                        NpcIntentDisposition::Rejected,
                        "carrying the coat alone is insufficient"
                    );
                    assert_eq!(quantity(&h, "oteryn:item.tibia.i3562").await?, 1);
                    assert_eq!(
                        super::classic_quests::progress(
                            &owner,
                            &admitted,
                            &authority,
                            &format!("{BASE}santiago_npc_greet_storage")
                        )
                        .await?,
                        3
                    );
                    tx = h.pool.begin().await?;
                    sqlx::query("SET LOCAL session_replication_role=replica")
                        .execute(&mut *tx)
                        .await?;
                } else {
                    // Source checks ANY armor-slot item, not that the coat itself is worn.
                    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,$5,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
                        .bind(id(41).as_slice()).bind(id(instance).as_slice()).bind(id(42).as_slice()).bind(id(81).as_slice()).bind(i16::from(armor_slot)).execute(&mut *tx).await?;
                }
            }
            tx.commit().await?;
            super::classic_quests::talk(&mut owner, &admitted, santiago, 4304, "bye").await?;
            super::classic_quests::talk(&mut owner, &admitted, santiago, 4305, "hi").await?;
            super::classic_quests::talk(&mut owner, &admitted, santiago, 4306, "yes").await?;
            assert_eq!(
                super::classic_quests::progress(
                    &owner,
                    &admitted,
                    &authority,
                    &format!("{BASE}santiago_npc_greet_storage")
                )
                .await?,
                4
            );
            assert_eq!(quantity(&h, "oteryn:item.tibia.i3562").await?, 1);
            assert_eq!(quantity(&h, "oteryn:item.tibia.i3270").await?, 0);
            // A fresh conversation at stage four repeats only the clothing check.
            super::classic_quests::talk(&mut owner, &admitted, santiago, 4307, "bye").await?;
            super::classic_quests::talk(&mut owner, &admitted, santiago, 4308, "hi").await?;
            super::classic_quests::talk(&mut owner, &admitted, santiago, 4309, "yes").await?;
            assert_eq!(quantity(&h, "oteryn:item.tibia.i3270").await?, 0);
            super::classic_quests::talk(&mut owner, &admitted, santiago, 4310, "action").await?;
            assert_eq!(
                super::classic_quests::progress(
                    &owner,
                    &admitted,
                    &authority,
                    &format!("{BASE}santiago_npc_greet_storage")
                )
                .await?,
                5
            );
            assert_eq!(
                super::classic_quests::progress(
                    &owner,
                    &admitted,
                    &authority,
                    &format!("{BASE}santiago_quest_log")
                )
                .await?,
                4
            );
            assert_eq!(quantity(&h, "oteryn:item.tibia.i3270").await?, 1);
            assert_eq!(quantity(&h, "oteryn:item.tibia.i3562").await?, 1);
            super::classic_quests::talk(&mut owner, &admitted, santiago, 4311, "bye").await?;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            let _reminder = owner
                .npc_talk(
                    admitted.runtime_actor.ok_or("actor")?,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 4312,
                        item_fence: Some(fence.item_fence()),
                    },
                    NpcTalkIntent {
                        npc_actor: santiago,
                        text: "hi".into(),
                    },
                )
                .await;
            assert_eq!(
                quantity(&h, "oteryn:item.tibia.i3270").await?,
                1,
                "source greeting cannot duplicate the weapon"
            );
            return Ok(());
        }
        let mut npc = santiago;
        let initial_fish = quantity(&h, FISH).await?;
        let initial_xp = experience(&h).await?;
        for (phase, speaker, next_greet, next_log, reward, expected_xp, command) in [
            ("legs", "santiago", 7, 5, "legs-handin", 100, 4000),
            ("fish", "santiago", 11, 9, "food-grant", 100, 4100),
            ("zirella", "zirella", 8, 8, "zirella-reward", 150, 4200),
        ] {
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let prepared = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key: format!(
                            "oteryn:quest-transition/fixture/beginning-reward/{phase}"
                        ),
                        cause: QuestCause::Command(CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(command).map_err(debug)?,
                        )),
                    },
                    owner.quest_catalogue.clone().ok_or("catalogue")?,
                )
                .await
                .map_err(debug)?;
            assert!(
                matches!(prepared, QuestTransitionOutcome::Committed(_)),
                "prerequisite: {prepared:?}"
            );
            drop(slot);
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
            if phase == "legs" {
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let fence = owner
                    .current_quest_fence(admitted.game_session_id)
                    .await
                    .map_err(|()| "fence")?
                    .ok_or("fence")?;
                eprintln!(
                    "QUESTPC3_BEGINNING_REWARDS POSTGRES_ASSERTION_REACHED before_shortfall duration_ms={}",
                    started.elapsed().as_millis()
                );
                let refused = owner
                    .npc_talk(
                        admitted.runtime_actor.ok_or("actor")?,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: command + 1,
                            item_fence: Some(fence.item_fence()),
                        },
                        NpcTalkIntent {
                            npc_actor: npc,
                            text: "hi".into(),
                        },
                    )
                    .await;
                assert_eq!(
                    refused.disposition,
                    NpcIntentDisposition::Rejected,
                    "two legs cannot satisfy the source hand-in"
                );
                assert!(!refused.inventory_changed);
                assert_eq!(quantity(&h, LEGS).await?, 2);
                assert_eq!(experience(&h).await?, initial_xp);
                assert_eq!(
                    super::classic_quests::progress(
                        &owner,
                        &admitted,
                        &authority,
                        &format!("{BASE}santiago_npc_greet_storage")
                    )
                    .await?,
                    6
                );
                assert_eq!(
                    super::classic_quests::progress(
                        &owner,
                        &admitted,
                        &authority,
                        &format!("{BASE}santiago_quest_log")
                    )
                    .await?,
                    4
                );
                // Supply the third physical leg as fixture inventory, never by
                // seeding a successful quest receipt, reward or XP obligation.
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
                    .bind(id(80).as_slice()).bind(id(42).as_slice()).bind(LEGS).bind(id(81).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,3,encode($5,'hex')::uuid)")
                    .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(70).as_slice()).bind(id(80).as_slice()).bind(id(81).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                super::classic_quests::talk(&mut owner, &admitted, npc, command + 2, "bye").await?;
                super::classic_quests::talk(&mut owner, &admitted, npc, command + 3, "hi").await?;
                assert_eq!(
                    quantity(&h, LEGS).await?,
                    0,
                    "all three exact physical inputs consumed"
                );
            } else {
                super::classic_quests::talk(&mut owner, &admitted, npc, command + 1, "bye").await?;
                if phase == "zirella" {
                    npc = zirella;
                }
                super::classic_quests::talk(&mut owner, &admitted, npc, command + 2, "hi").await?;
                super::classic_quests::talk(&mut owner, &admitted, npc, command + 3, "yes").await?;
            }
            assert_eq!(experience(&h).await?, initial_xp + expected_xp);
            for (suffix, expected) in [("npc_greet_storage", next_greet), ("quest_log", next_log)] {
                assert_eq!(
                    super::classic_quests::progress(
                        &owner,
                        &admitted,
                        &authority,
                        &format!("{BASE}{speaker}_{suffix}")
                    )
                    .await?,
                    expected
                );
            }
            if phase != "legs" {
                assert_eq!(quantity(&h, FISH).await?, initial_fish + 1);
            }
            // Exact receipt replay through the same value owner cannot repeat
            // the physical debit, native MINT or transferred XP obligation.
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            let native = owner
                .active_generation
                .and_then(|g| g.native_gameplay())
                .ok_or("native")?;
            let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
                .with_wheel(owner.premium_coordinator);
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let replay = slot
                .commit_quest_reward(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key: format!("oteryn:quest-reward/questpc3/beginning/{reward}"),
                        cause: QuestCause::Command(CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(command + 3).map_err(debug)?,
                        )),
                    },
                    owner.quest_catalogue.clone().ok_or("catalogue")?,
                    QuestRewardAdmission {
                        native,
                        weights: &weights,
                    },
                    &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
                )
                .await
                .map_err(debug)?;
            assert!(
                matches!(replay, QuestTransitionOutcome::AlreadyCommitted(_)),
                "exact replay: {replay:?}"
            );
            drop(slot);
            assert_eq!(experience(&h).await?, initial_xp + expected_xp);
            assert_eq!(quantity(&h, LEGS).await?, 0);
            eprintln!(
                "QUESTPC3_BEGINNING_REWARDS POSTGRES_ASSERTION_REACHED phase={phase} duration_ms={}",
                started.elapsed().as_millis()
            );
        }
        // Reject an executable catalogue substituting XP for the source plan.
        let mut substituted = fixture_transitions;
        substituted
            .iter_mut()
            .find(|t| t.key == "oteryn:quest-reward/questpc3/beginning/legs-handin")
            .ok_or("hand-in transition")?
            .experience = Some(101);
        let substituted = Arc::new(
            QuestStateCatalogue::new("content-1", fixture_tracks, substituted).map_err(debug)?,
        );
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
            .with_wheel(owner.premium_coordinator);
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let refused = slot
            .commit_quest_reward(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: "oteryn:quest-reward/questpc3/beginning/legs-handin".into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(5000).map_err(debug)?,
                    )),
                },
                substituted,
                QuestRewardAdmission {
                    native,
                    weights: &weights,
                },
                &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
            )
            .await;
        assert!(matches!(refused, Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)), "substituted XP: {refused:?}");
        drop(slot);
        assert_eq!(experience(&h).await?, initial_xp + 150);
        assert_eq!(quantity(&h, FISH).await?, initial_fish + 1);
        assert_eq!(quantity(&h, LEGS).await?, 0);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE transition_key LIKE 'oteryn:quest-reward/questpc3/beginning/%'").fetch_one(&h.pool).await?;
        assert_eq!(receipts, 3);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })?;
    eprintln!(
        "QUESTPC3_BEGINNING_REWARDS POSTGRES_CASE_COMPLETED duration_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(key).fetch_one(&h.pool).await?)
}

async fn experience(h: &Harness) -> TestResult<u64> {
    let value: String = sqlx::query_scalar("SELECT total_experience::text FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    Ok(value.parse()?)
}

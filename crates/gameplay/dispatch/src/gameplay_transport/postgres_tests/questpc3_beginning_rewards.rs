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
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Beginning rewards scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
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
        let [santiago, zirella] = super::classic_quests::install(
            &owner,
            &admitted,
            ["oteryn:npc.santiago", "oteryn:npc.zirella"],
        )
        .await?;
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

//! Prior story milestones are fixtures; physical hand-ins and rewards use the real NPC owner.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::collections::BTreeMap;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.threatened_dreams_quest";
const ANIMALS: &str = oteryn_gameplay_quests::quest::threatened_dreams::ANIMALS;
const STONE: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/grumpy_stone";
const SEED: &str = "oteryn:quest-transition/fixture/threatened-animals-prior-story";
const SWAN_SEED: &str = "oteryn:quest-transition/fixture/threatened-animals-prior-swan";
const FEATHERS: &str = "oteryn:item.tibia.i25244";
#[test]
fn threatened_animals_three_source_exchanges_are_atomic() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_ANIMALS POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        let started=std::time::Instant::now();
        match std::panic::catch_unwind(scenario) {
            Ok(result) => {
                eprintln!("THREATENED_ANIMALS POSTGRES_CASE_COMPLETED duration_ms={} success={}", started.elapsed().as_millis(), result.is_ok());
                result.map_err(|error|error.to_string())
            }
            Err(panic) => {
                eprintln!("THREATENED_ANIMALS POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());
                std::panic::resume_unwind(panic)
            }
        }
    })?.join().map_err(|_|"inventory scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "threatened_animals", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (controller, pin) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
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
        let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        let mut tracks = BTreeMap::new();
        let stones: Vec<String> = (1..=5)
            .map(|i| {
                format!("oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/stone{i}")
            })
            .collect();
        for key in [ANIMALS, STONE]
            .into_iter()
            .chain(stones.iter().map(String::as_str))
        {
            tracks.insert(
                key.to_owned(),
                source.catalogue().track(key).ok_or("source track")?.clone(),
            );
        }
        let mut transitions = Vec::new();
        for suffix in ["ikassis-talisman", "swan-feathers", "stone-map"] {
            let key = format!("oteryn:quest-reward/questpc3/threatened-animals/{suffix}");
            if let Some(t) = source.catalogue().transition(&key) {
                transitions.push(t.clone());
            }
        }
        // Only preceding milestones and collected feathers are fixtures. Outputs,
        // hand-in consumption and successful receipts are never seeded.
        transitions.push(QuestTransition {
            key: SEED.into(),
            quest: QUEST.into(),
            effects: [(ANIMALS, 4), (STONE, 1)]
                .into_iter()
                .chain(
                    stones
                        .iter()
                        .enumerate()
                        .map(|(i, s)| (s.as_str(), i64::from(i < 4))),
                )
                .map(|(track, value)| QuestEffect {
                    track: track.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(value),
                })
                .collect(),
            completes: false,
            experience: None,
        });
        transitions.push(QuestTransition {
            key: SWAN_SEED.into(),
            quest: QUEST.into(),
            effects: vec![
                QuestEffect {
                    track: ANIMALS.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(14),
                },
                QuestEffect {
                    track: stones[4].clone(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(1),
                },
            ],
            completes: false,
            experience: None,
        });
        owner.quest_catalogue = Some(Arc::new(
            QuestStateCatalogue::new("content-1", tracks.into_values().collect(), transitions)
                .map_err(debug)?,
        ));
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        super::super::item_look::seed_main_backpack(&h, &[(FEATHERS, 4)]).await?;
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
                    transition_key: SEED.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(24000).map_err(debug)?,
                    )),
                },
                Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
        drop(slot);
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        let [ikassis, swan, stone] = super::super::classic_quests::install(
            &owner,
            &admitted,
            [
                "oteryn:npc.ikassis",
                "oteryn:npc.a_swan",
                "oteryn:npc.grumpy_stone",
            ],
        )
        .await?;
        say(&mut owner, &admitted, stone, 25990, "hi").await?;
        say(&mut owner, &admitted, stone, 25991, "mission").await?;
        assert_eq!(
            quantity(&h, "oteryn:item.tibia.i24945").await?,
            0,
            "four scratch flags cannot earn the map"
        );
        say(&mut owner, &admitted, ikassis, 26001, "hi").await?;
        say(&mut owner, &admitted, ikassis, 26002, "yes").await?;
        assert_eq!(
            quantity(&h, "oteryn:item.tibia.i25295").await?,
            0,
            "confirmation requires source topic"
        );
        say(&mut owner, &admitted, ikassis, 26003, "mission").await?;
        say(&mut owner, &admitted, ikassis, 26004, "yes").await?;
        let talisman = quantity(&h, "oteryn:item.tibia.i25295").await? == 1;
        say(&mut owner, &admitted, ikassis, 26005, "yes").await?;
        assert!(quantity(&h, "oteryn:item.tibia.i25295").await? <= 1);
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        assert!(matches!(
            slot.commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: SWAN_SEED.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(26006).map_err(debug)?
                    ))
                },
                Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?)
            )
            .await
            .map_err(debug)?,
            QuestTransitionOutcome::Committed(_)
        ));
        drop(slot);
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        say(&mut owner, &admitted, swan, 26007, "hi").await?;
        say(&mut owner, &admitted, swan, 26008, "mission").await?;
        assert_eq!(
            quantity(&h, FEATHERS).await?,
            4,
            "short hand-in retains feathers"
        );
        assert_eq!(
            super::super::classic_quests::progress(&owner, &admitted, &authority, ANIMALS).await?,
            14
        );
        append_fixture_input(&h, FEATHERS, 90, 7).await?;
        say(&mut owner, &admitted, swan, 26009, "mission").await?;
        let feathers = quantity(&h, FEATHERS).await? == 0
            && super::super::classic_quests::progress(&owner, &admitted, &authority, ANIMALS)
                .await?
                == 15;
        say(&mut owner, &admitted, stone, 26010, "hi").await?;
        say(&mut owner, &admitted, stone, 26011, "mission").await?;
        let map = quantity(&h, "oteryn:item.tibia.i24945").await? == 1;
        say(&mut owner, &admitted, stone, 26012, "mission").await?;
        assert!(quantity(&h, "oteryn:item.tibia.i24945").await? <= 1);
        eprintln!(
            "THREATENED_ANIMALS POSTGRES_EVIDENCE talisman={talisman} feathers={feathers} stone_map={map}"
        );
        assert!(
            talisman && feathers && map,
            "all three source physical exchanges must execute"
        );
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-reward/questpc3/threatened-animals/%'").fetch_one(&h.pool).await?;
        assert_eq!(
            receipts, 3,
            "only three successful physical exchanges produce receipts"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        Ok(())
    })
}

async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(key).fetch_one(&h.pool).await?)
}

async fn append_fixture_input(h: &Harness, key: &str, nonce: u8, ordinal: i32) -> TestResult {
    // Fixture inventory only, after proving real refusal. No quest receipt,
    // exchange success, output or progression is seeded.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
        .bind(id(nonce).as_slice()).bind(id(42).as_slice()).bind(key).bind(id(nonce+1).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(70).as_slice()).bind(id(nonce).as_slice()).bind(ordinal.to_string()).bind(id(nonce+1).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    command: u64,
    text: &str,
) -> TestResult<crate::gameplay_transport::npc_command::NpcTalkOutcome> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    Ok(owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await)
}

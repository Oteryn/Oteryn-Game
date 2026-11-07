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
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::collections::BTreeMap;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.threatened_dreams_quest";
const COUPLE: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission03_1";
const VIAL: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission03/empty_starlight_vial";
const CATCHER: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission03/dark_sun_catcher";
const CLOAK: &str =
    "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission05/swan_feather_cloak";
const SEED: &str = "oteryn:quest-transition/fixture/threatened-prior-story";
const GEM_INPUTS: [&str; 4] = [
    "oteryn:item.tibia.i675",
    "oteryn:item.tibia.i676",
    "oteryn:item.tibia.i677",
    "oteryn:item.tibia.i678",
];
const FEATHER: &str = "oteryn:item.tibia.i26181";

#[test]
fn threatened_inventory_three_exact_physical_exchanges_are_atomic() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_INVENTORY POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        let started=std::time::Instant::now();
        match std::panic::catch_unwind(scenario) {
            Ok(result) => {
                eprintln!("THREATENED_INVENTORY POSTGRES_CASE_COMPLETED duration_ms={} success={}", started.elapsed().as_millis(), result.is_ok());
                result.map_err(|error|error.to_string())
            }
            Err(panic) => {
                eprintln!("THREATENED_INVENTORY POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());
                std::panic::resume_unwind(panic)
            }
        }
    })?.join().map_err(|_|"inventory scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "threatened_inventory", false, false)
            .await?;
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
        for key in [COUPLE, VIAL, CATCHER, CLOAK] {
            let track = source.catalogue().track(key).ok_or("source track")?.clone();
            tracks.insert(key.to_owned(), track);
        }
        let mut transitions = Vec::new();
        for key in [
            "oteryn:quest-transition/threatened-dreams/cloak-offer",
            "oteryn:quest-reward/questpc3/threatened/aurita-vial",
            "oteryn:quest-reward/questpc3/threatened/taegen-catcher",
            "oteryn:quest-reward/questpc3/threatened/valindara-cloak",
        ] {
            if let Some(transition) = source.catalogue().transition(key) {
                for effect in &transition.effects {
                    tracks.entry(effect.track.clone()).or_insert(
                        source
                            .catalogue()
                            .track(&effect.track)
                            .ok_or("source effect track")?
                            .clone(),
                    );
                }
                transitions.push(transition.clone());
            }
        }
        // Only the required preceding story is a fixture. No reward, exchange
        // flag, item consumption or successful receipt is supplied here.
        transitions.push(QuestTransition {
            key: SEED.into(),
            quest: QUEST.into(),
            effects: [(COUPLE, 4), (VIAL, 0), (CATCHER, 0), (CLOAK, 1)]
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
        owner.quest_catalogue = Some(Arc::new(
            QuestStateCatalogue::new("content-1", tracks.into_values().collect(), transitions)
                .map_err(debug)?,
        ));
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        super::super::item_look::seed_main_backpack(
            &h,
            &[
                (GEM_INPUTS[0], 2),
                (GEM_INPUTS[1], 2),
                (GEM_INPUTS[2], 2),
                (GEM_INPUTS[3], 1),
                (FEATHER, 99),
            ],
        )
        .await?;
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
        let [aurita, taegen, valindara] = super::super::classic_quests::install(
            &owner,
            &admitted,
            [
                "oteryn:npc.aurita",
                "oteryn:npc.taegen",
                "oteryn:npc.valindara",
            ],
        )
        .await?;

        assert_eq!(
            say(&mut owner, &admitted, aurita, 24001, "hi")
                .await?
                .disposition,
            NpcIntentDisposition::Ok
        );
        say(&mut owner, &admitted, aurita, 24002, "starlight vial").await?;
        let vial_ok = quantity(&h, "oteryn:item.tibia.i25731").await? == 1
            && super::super::classic_quests::progress(&owner, &admitted, &authority, VIAL).await?
                == 1;
        say(&mut owner, &admitted, aurita, 24003, "starlight vial").await?;
        assert!(
            quantity(&h, "oteryn:item.tibia.i25731").await? <= 1,
            "vial never duplicates"
        );
        say(&mut owner, &admitted, aurita, 24004, "bye").await?;

        say(&mut owner, &admitted, taegen, 24010, "hi").await?;
        say(&mut owner, &admitted, taegen, 24011, "sun catcher").await?;
        say(&mut owner, &admitted, taegen, 24012, "yes").await?;
        for (key, expected) in GEM_INPUTS.into_iter().zip([2, 2, 2, 1]) {
            assert_eq!(
                quantity(&h, key).await?,
                expected,
                "shortfall cannot consume {key}"
            );
        }
        assert_eq!(quantity(&h, "oteryn:item.tibia.i25733").await?, 0);
        assert_eq!(
            super::super::classic_quests::progress(&owner, &admitted, &authority, CATCHER).await?,
            0
        );
        append_fixture_input(&h, GEM_INPUTS[3], 80, 6).await?;
        say(&mut owner, &admitted, taegen, 24013, "sun catcher").await?;
        say(&mut owner, &admitted, taegen, 24014, "yes please").await?;
        for key in GEM_INPUTS {
            assert_eq!(
                quantity(&h, key).await?,
                2,
                "only exact affirmative can charge"
            );
        }
        say(&mut owner, &admitted, taegen, 24015, "sun catcher").await?;
        say(&mut owner, &admitted, taegen, 24016, "yes").await?;
        let mut gems_left = 0;
        for key in GEM_INPUTS {
            gems_left += quantity(&h, key).await?;
        }
        let catcher_ok = quantity(&h, "oteryn:item.tibia.i25733").await? == 1
            && gems_left == 0
            && super::super::classic_quests::progress(&owner, &admitted, &authority, CATCHER)
                .await?
                == 1;
        say(&mut owner, &admitted, taegen, 24017, "yes").await?;
        assert!(quantity(&h, "oteryn:item.tibia.i25733").await? <= 1);
        say(&mut owner, &admitted, taegen, 24018, "bye").await?;

        say(&mut owner, &admitted, valindara, 24020, "hi").await?;
        say(&mut owner, &admitted, valindara, 24021, "cloak").await?;
        say(&mut owner, &admitted, valindara, 24022, "yes").await?;
        assert_eq!(
            quantity(&h, FEATHER).await?,
            99,
            "feather shortfall cannot charge"
        );
        assert_eq!(quantity(&h, "oteryn:item.tibia.i25779").await?, 0);
        append_fixture_input(&h, FEATHER, 82, 7).await?;
        say(&mut owner, &admitted, valindara, 24023, "cloak").await?;
        say(&mut owner, &admitted, valindara, 24024, "yes").await?;
        let cloak_ok = quantity(&h, "oteryn:item.tibia.i25779").await? == 1
            && quantity(&h, FEATHER).await? == 0
            && super::super::classic_quests::progress(&owner, &admitted, &authority, CLOAK).await?
                == 2;
        say(&mut owner, &admitted, valindara, 24025, "yes").await?;
        assert!(quantity(&h, "oteryn:item.tibia.i25779").await? <= 1);
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE transition_key LIKE 'oteryn:quest-reward/questpc3/threatened/%'").fetch_one(&h.pool).await?;
        eprintln!(
            "THREATENED_INVENTORY POSTGRES_ASSERTION_REACHED vial_ok={vial_ok} catcher_ok={catcher_ok} cloak_ok={cloak_ok} receipts={receipts}"
        );
        assert_eq!(
            [vial_ok, catcher_ok, cloak_ok],
            [true, true, true],
            "all three exact physical exchanges must execute"
        );
        assert_eq!(receipts, 3, "repeated words cannot mint or consume twice");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
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

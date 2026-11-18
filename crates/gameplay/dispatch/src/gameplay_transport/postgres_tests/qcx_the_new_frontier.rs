//! The TibiaWiki quest (curid 41754, oldid 1058656) and spoiler (curid 41756, oldid 1099208).
use super::fixture::{Harness, debug, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition,
    loader::load_embedded_quest_state,
};
use oteryn_protocol_oteryn::{npc_service::NpcTalkIntent, world_spatial_entities::EntityRef};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_new_frontier_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/questline";
const NEW_LAND: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission01";
const MORTAL_COMBAT: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission09_1";
const MISSION: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission10_1";
const CARPET: &str =
    "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission10/magic_carpet_door";
const READY: &str = "oteryn:quest-transition/qcx-new-frontier-warmaster-claimed";

#[test]
fn new_horizons_grants_experience_and_carpet_flag_once() -> TestResult {
    scenario(None)
}

#[test]
fn new_land_starts_only_after_ongulfs_project_dialogue() -> TestResult {
    scenario(Some("short"))
}

#[test]
fn new_land_accepts_the_long_project_dialogue() -> TestResult {
    scenario(Some("long"))
}

fn scenario(entry: Option<&'static str>) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("qcx-new-frontier", move || {
        run(async |admin| {
            let started = std::time::Instant::now();
            let h = Harness::create_with_admission_seed(admin, "qcx_new_frontier", false, false)
                .await?;
            npc_journey(&h, entry).await?;
            h.cleanup().await?;
            match entry {
                Some(story) => println!("New Land ({story}): {:?}", started.elapsed()),
                None => println!("New Horizons: {:?}", started.elapsed()),
            }
            Ok(())
        })
    })
}

async fn npc_journey(h: &Harness, entry: Option<&str>) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let room = pin.qualified_room().clone();
    let deps = Dependencies::with_content_and_capacity(
        pin.into_channel_pin(),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        3,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let (consumer, clock, activation, premium) = purchase_fixture::current(super::fixture::id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let entitlement = SpellPremiumCoordinator::new(
        Arc::clone(&consumer),
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    let mut expired = premium.clone();
    expired.effective_until_us = expired.authority_issued_at_us;
    expired.state = crate::durability::premium_fence::EntitlementState::Expired;
    let mut admitted = deps.admitted;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(controller.active().ok_or("content generation")?);
    owner.premium_coordinator = Some(&entitlement);
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    deps.initialize(&owner).await;
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );

    let source = load_embedded_quest_state("content-1").map_err(debug)?;
    // Only the finished arena and claimed wardrobe are arranged.
    // Chrak and Ongulf must commit the production transitions.
    let catalogue = Arc::new(
        source
            .catalogue()
            .with_fixture_transition(QuestTransition {
                world_change: None,
                key: READY.into(),
                quest: QUEST.into(),
                effects: [QUESTLINE, MORTAL_COMBAT]
                    .into_iter()
                    .zip([27, 2])
                    .map(|(track, value)| QuestEffect {
                        track: track.into(),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::Set(value),
                    })
                    .collect(),
                completes: false,
                experience: None,
            })
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_quest_session(&admitted).await;
    let [ongulf, chrak] = super::classic_quests::install(
        &owner,
        &admitted,
        ["oteryn:npc.ongulf", "oteryn:npc.chrak"],
    )
    .await?;
    talk(&mut owner, &admitted, ongulf, 101, "hi").await?;
    talk(&mut owner, &admitted, ongulf, 102, "mission").await?;
    assert_eq!(stage(h, QUESTLINE).await?, None);
    if let Some(story) = entry {
        let offer = talk(&mut owner, &admitted, ongulf, 103, story).await?;
        assert!(
            offer.iter().any(|line| line.contains("missions")),
            "{offer:?}"
        );
        consumer.prove_for_test(expired.clone());
        assert!(
            !owner
                .session_premium_gate(admitted.game_session_id)
                .await
                .premium_current()
        );
        talk(&mut owner, &admitted, ongulf, 104, "mission").await?;
        assert_eq!(
            stage(h, QUESTLINE).await?,
            None,
            "Premium expired after the opening offer"
        );
        consumer.prove_for_test(premium.clone());
        // Re-establish the topic in case a refusal closed the conversation.
        for (id, text) in [
            (105, "bye"),
            (106, "hi"),
            (107, "project"),
            (108, story),
            (109, "mission"),
        ] {
            talk(&mut owner, &admitted, ongulf, id, text).await?;
        }
        assert_eq!(stage(h, QUESTLINE).await?, Some(1));
        assert_eq!(stage(h, NEW_LAND).await?, Some(1));
        return Ok(());
    }
    assert!(
        owner
            .progression_binding(admitted.game_session_id)
            .is_some()
    );
    let before = experience(h).await?;

    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: READY.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(103).map_err(debug)?,
                    )),
                },
                catalogue,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    talk(&mut owner, &admitted, ongulf, 104, "mission").await?;
    assert_eq!(
        experience(h).await?,
        before,
        "report victory to Chrak first"
    );
    talk(&mut owner, &admitted, chrak, 105, "hi").await?;
    consumer.prove_for_test(expired.clone());
    assert!(
        !owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    talk(&mut owner, &admitted, chrak, 106, "mission").await?;
    assert_eq!(
        stage(h, QUESTLINE).await?,
        Some(27),
        "Premium expired before Chrak's report"
    );
    consumer.prove_for_test(premium.clone());
    talk(&mut owner, &admitted, chrak, 107, "mission").await?;
    assert_eq!(stage(h, QUESTLINE).await?, Some(28));
    assert_eq!(stage(h, MORTAL_COMBAT).await?, Some(3));
    assert_eq!(stage(h, MISSION).await?, Some(1));
    talk(&mut owner, &admitted, chrak, 108, "bye").await?;
    talk(&mut owner, &admitted, ongulf, 109, "hi").await?;
    consumer.prove_for_test(expired);
    assert!(
        !owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    talk(&mut owner, &admitted, ongulf, 110, "mission").await?;
    assert_eq!(
        experience(h).await?,
        before,
        "Premium expired before the finale"
    );
    assert_eq!(stage(h, QUESTLINE).await?, Some(28));
    consumer.prove_for_test(premium);
    talk(&mut owner, &admitted, ongulf, 111, "mission").await?;
    assert_eq!(experience(h).await?, before + 8000);
    for (track, expected) in [(QUESTLINE, 29), (MISSION, 2), (CARPET, 1)] {
        assert_eq!(stage(h, track).await?, Some(expected));
    }
    talk(&mut owner, &admitted, ongulf, 112, "mission").await?;
    assert_eq!(experience(h).await?, before + 8000);
    Ok(())
}

async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT total_experience::bigint FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn stage(h: &Harness, track: &str) -> TestResult<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(track)
            .fetch_optional(&h.pool)
            .await?,
    )
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    id: u64,
    text: &str,
) -> TestResult<Vec<String>> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("NPC rate clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert!(!outcome.unknown, "{text}: NPC dispatch outcome unknown");
    Ok(outcome
        .conversation
        .map(|reply| reply.lines)
        .unwrap_or_default())
}

// Wiki quest page (curid 41754, oldid 1058656), reward list.
// Crystal 201189f:data-global/startup/tables/chest.lua:1324-1343 provides the item IDs.
#[test]
fn native_gold_ingot_chest_claims_two_once() -> TestResult {
    chest("gold_ingot", 33055, 9058, 2, &[])
}

#[test]
fn native_piggy_bank_chest_claims_one_once() -> TestResult {
    chest("pig_bank", 33053, 2995, 1, &[])
}

#[test]
fn native_potion_chest_claims_all_three_once() -> TestResult {
    chest("potions", 33057, 2854, 1, &[7439, 7440, 7443])
}

fn chest(name: &'static str, x: u16, item: u16, quantity: u32, contents: &[u16]) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let chest = super::quest_catalogue_chests::CatalogueChest {
        claim: format!("oteryn:reward-claim.quest.u8_54.the_new_frontier.reward.{name}"),
        position: (x, 31029, -7),
        unique: 0,
        servable: true,
        appearance: 2472,
        root: format!("oteryn:item.tibia.i{item}"),
        quantity,
        contents: contents
            .iter()
            .map(|id| (format!("oteryn:item.tibia.i{id}"), 1, 0))
            .collect(),
        texts: vec![],
        key_number: None,
        achievement: false,
        charges: vec![],
        fluids: vec![],
    };
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        let world = crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?;
        let activation =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        super::quest_container_chests::catalogue_scenario(&chest, &activation)
    })?;
    println!("New Frontier {name} chest: {:?}", started.elapsed());
    Ok(())
}

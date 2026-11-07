//! Free quest blessings use current authority and a Quest receipt, never a paid sale.
use super::classic_quests::{install, progress};
use super::fixture::{Harness, debug, id, run};
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
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.pilgrimage_of_ashes";
const STAGE: &str = "oteryn:quest-progress/questpc3/pilgrimage_of_ashes/stage";
const SEED: &str = "oteryn:quest-transition/fixture/pilgrimage-prior-guide";
const NORF: &str = "oteryn:npc.norf";
const GIFT: &str = "oteryn:quest-reward/questpc3/pilgrimage/norf";
const BLESSING: &str = "oteryn:blessing.the_spiritual_shielding";

#[test]
fn quest_l4_pilgrimage_free_regular_blessing_is_fenced_and_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUEST_L4_PILGRIMAGE_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let started = std::time::Instant::now();
            let result = scenario();
            eprintln!(
                "QUEST_L4_PILGRIMAGE POSTGRES_CASE_COMPLETED duration_ms={} success={}",
                started.elapsed().as_millis(),
                result.is_ok()
            );
            result.map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "pilgrimage scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "quest_l4_pilgrimage", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
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
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        let mut transitions: Vec<_> = source
            .catalogue()
            .transition(GIFT)
            .cloned()
            .into_iter()
            .collect();
        // Only the prior guide visit is a fixture. No blessing, gift receipt or
        // completed quest is seeded; those must be produced by the real caller.
        transitions.push(QuestTransition {
            key: SEED.into(),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: STAGE.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(1),
            }],
            completes: false,
            experience: None,
        });
        owner.quest_catalogue = Some(Arc::new(
            QuestStateCatalogue::new(
                "content-1",
                vec![QuestTrack {
                    key: STAGE.into(),
                    quest: QUEST.into(),
                    initial: -1,
                    min: -1,
                    max: 8,
                }],
                transitions,
            )
            .map_err(debug)?,
        ));
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
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
                        CommandId::new(9500).map_err(debug)?,
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
        owner.premium_coordinator = Some(&premium);
        owner.admit_premium(id(40), admitted.game_session_id);
        let [norf] = install(&owner, &admitted, [NORF]).await?;
        let greeted = say(&mut owner, &admitted, norf, 9501, "hi").await?;
        eprintln!(
            "QUEST_L4_PILGRIMAGE POSTGRES_ASSERTION_REACHED greeting={:?}",
            greeted.disposition
        );
        assert_eq!(
            greeted.disposition,
            oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
        );
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let mut stale = current.item_fence();
        stale.character_lease_generation += 1;
        let refused = owner
            .npc_talk(
                admitted.runtime_actor.ok_or("actor")?,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 9505,
                    item_fence: Some(stale),
                },
                NpcTalkIntent {
                    npc_actor: norf,
                    text: "pilgrimage".into(),
                },
            )
            .await;
        assert_eq!(
            refused.disposition,
            oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
        );
        assert_eq!(progress(&owner, &admitted, &authority, STAGE).await?, 1);
        owner.premium_coordinator = None;
        let no_premium = say(&mut owner, &admitted, norf, 9506, "pilgrimage").await?;
        assert_eq!(
            no_premium.disposition,
            oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
        );
        assert!(!no_premium.unknown);
        assert_eq!(
            progress(&owner, &admitted, &authority, STAGE).await?,
            1,
            "missing current Premium must refuse the gift"
        );
        let absent: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_blessings WHERE blessing_key=$1",
        )
        .bind(BLESSING)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(absent, 0);
        owner.premium_coordinator = Some(&premium);
        say(&mut owner, &admitted, norf, 9502, "pilgrimage please").await?;
        assert_eq!(
            progress(&owner, &admitted, &authority, STAGE).await?,
            1,
            "only the exact source keyword may grant"
        );
        let granted = say(&mut owner, &admitted, norf, 9503, "pilgrimage").await?;
        assert!(!granted.unknown);
        assert_eq!(progress(&owner, &admitted, &authority, STAGE).await?, 2);
        let held: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_blessings WHERE blessing_key=$1",
        )
        .bind(BLESSING)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(held, 1, "Norf grants the real regular blessing");
        say(&mut owner, &admitted, norf, 9504, "pilgrimage").await?;
        assert_eq!(progress(&owner, &admitted, &authority, STAGE).await?, 2);
        let paid: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_npc_purchase_receipts WHERE npc_key=$1",
        )
        .bind(NORF)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            paid, 0,
            "a free quest gift cannot create or charge a paid purchase"
        );
        let held: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_blessings WHERE blessing_key=$1",
        )
        .bind(BLESSING)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(held, 1);
        Ok(())
    })
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

//! M38: permission and outstanding fines compose in the existing fenced travel owner.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::npc_travel::{NpcTravelGate, TravelRefusal};

struct NoPremium;
impl NpcTravelGate for NoPremium {
    async fn travel_permitted(
        &self,
        _tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        _account: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(false)
    }
}

#[test]
fn npcfix_m38_travel_permission_and_clear_fine_are_both_required_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("npcfix-m38-travel-guards", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "m38_travel_guards", true, false)
                .await?;
            let result: TestResult = async {
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            eprintln!("POSTGRES_STARTED NPCFIX_M38 travel permission and clear fine");
            let permission = "oteryn:quest-progress/npcfix/guard_permission";
            let fine = "oteryn:quest-progress/npcfix/outstanding_fine";
            let route = oteryn_gameplay_content::content::npc_catalogue::travel::TravelRoute {
                service: "oteryn:service.travel.m38_guard_fixture".into(),
                route: crate::content::ProjectV2TravelRoute {
                    key: "return".into(),
                    destination: crate::content::ProjectV2TravelDestination {
                        coordinate_frame: "global-target-2026-09-27".into(), x: 2, y: 11, floor: 7,
                    }, price: 100, premium: false, min_level: None,
                }, keyword: "return".into(), postman: false, carpet: false, held: false,
                yalahar_gate: false, yalahar_town_gate: None,
                quest_gate: Some(permission.into()), quest_clear_gate: Some(fine.into()), world_change: None,
            };
            use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition};
            use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
            let quest = "oteryn:quest.npcfix_guard_fixture";
            let steps = [(permission, 1), (fine, 100), (fine, 0), (fine, -1)];
            let transitions = steps.iter().enumerate().map(|(index, (track, value))| QuestTransition {
                key: format!("oteryn:quest-transition/npcfix/guard_{index}"), quest: quest.into(),
                effects: vec![QuestEffect { track: (*track).into(), from: QuestComparison::Any, effect: QuestEffectKind::Set(*value) }],
                completes: false, experience: None, world_change: None,
            }).collect();
            let catalogue = std::sync::Arc::new(QuestStateCatalogue::new("content-1", [permission, fine].map(|key| QuestTrack {
                key: key.into(), quest: quest.into(), initial: 0, min: -1, max: 100,
            }).to_vec(), transitions).map_err(debug)?);
            for (index, expected) in [Err(TravelRefusal::UnavailableRoute), Ok(100), Err(TravelRefusal::UnavailableRoute), Ok(100), Err(TravelRefusal::UnavailableRoute)].into_iter().enumerate() {
                let fence = owner.current_quest_fence(deps.admitted.game_session_id).await
                    .map_err(|()| "current fence")?.ok_or("current fence")?;
                if index > 0 {
                    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                    let outcome = slot.commit_quest_transition(owner.root, owner.character, owner.holder, fence,
                        QuestTransitionRequest {
                            transition_key: format!("oteryn:quest-transition/npcfix/guard_{}", index - 1),
                            cause: QuestCause::Command(crate::foundation::CommandRef::new(fence.game_session_id,
                                crate::foundation::CommandId::new(41000 + u64::try_from(index)?).map_err(debug)?)),
                        }, catalogue.clone()).await.map_err(debug)?;
                    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)), "fixture transition: {outcome:?}");
                }
                let current = owner.current_quest_fence(deps.admitted.game_session_id).await
                    .map_err(|()| "current fence")?.ok_or("current fence")?;
                assert_eq!(h.root.quote_npc_travel(&authority, &h.node, current, route.clone(), &NoPremium)
                    .await.map_err(debug)?, expected, "guard state {index}");
            }
            let current = owner.current_quest_fence(deps.admitted.game_session_id).await
                .map_err(|()| "current fence")?.ok_or("current fence")?;
            for track in ["", "oteryn:", "oteryn:bad key"] {
                let mut invalid = route.clone();
                invalid.quest_clear_gate = Some(track.into());
                assert_eq!(h.root.quote_npc_travel(&authority, &h.node, current, invalid, &NoPremium)
                    .await.map_err(debug)?, Err(TravelRefusal::UnavailableRoute));
            }
            let mut legacy = route;
            legacy.quest_clear_gate = None;
            assert_eq!(h.root.quote_npc_travel(&authority, &h.node, current, legacy, &NoPremium)
                .await.map_err(debug)?, Ok(100), "legacy routes retain their existing policy");
            let writes: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_item_fee_burns)+(SELECT count(*) FROM game_character_pending_arrivals)+(SELECT count(*) FROM game_newhaven_travel_receipts)")
                .fetch_one(&h.pool).await?;
            assert_eq!(writes, 0, "a quote must not charge or create an arrival");
            Ok(())
        }.await;
            h.cleanup().await?;
            result
        })
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38_TRAVEL_GUARDS duration={:?}",
        started.elapsed()
    );
    Ok(())
}

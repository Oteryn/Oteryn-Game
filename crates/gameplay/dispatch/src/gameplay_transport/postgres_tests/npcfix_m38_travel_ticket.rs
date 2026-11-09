//! M38: elapsed ticket validity is read from Quest state using database time.
use super::fixture::{Harness, debug, run};
use super::npcfix_m38_travel_guards::NoPremium;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::npc_travel::TravelRefusal;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use oteryn_gameplay_content::content::npc_catalogue::travel::{
    TravelRoute, TravelTicketRequirement,
};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};

#[test]
fn npcfix_m38_travel_ticket_requires_an_unexpired_server_purchase_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("npcfix-m38-ticket", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "m38_travel_ticket", true, false)
                .await?;
            let result: TestResult = async {
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let track = "oteryn:quest-progress/npcfix/travora_ticket_purchased";
            let quest = "oteryn:quest.npcfix_ticket_fixture";
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&h.pool).await?;
            let steps = [QuestEffectKind::Set(now - 86400), QuestEffectKind::Set(i64::MAX), QuestEffectKind::SetNow,
                         QuestEffectKind::Set(now - 23 * 3600), QuestEffectKind::Set(0)];
            let transitions = steps.into_iter().enumerate().map(|(i, effect)| QuestTransition {
                key: format!("oteryn:quest-transition/npcfix/ticket_{i}"), quest: quest.into(),
                effects: vec![QuestEffect { track: track.into(), from: QuestComparison::Any, effect }],
                completes: false, experience: None, world_change: None,
            }).collect();
            let catalogue = std::sync::Arc::new(QuestStateCatalogue::new("content-1", vec![QuestTrack {
                key: track.into(), quest: quest.into(), initial: 0, min: 0, max: i64::MAX,
            }], transitions).map_err(debug)?);
            let route = TravelRoute {
                service: "oteryn:service.travel.m38_ticket_fixture".into(),
                route: crate::content::ProjectV2TravelRoute {
                    key: "travora".into(), destination: crate::content::ProjectV2TravelDestination {
                        coordinate_frame: "global-target-2026-09-27".into(), x: 2, y: 11, floor: 7,
                    }, price: 100, premium: false, min_level: None,
                }, keyword: "travora".into(), postman: false, carpet: false, held: false,
                yalahar_gate: false, yalahar_town_gate: None, quest_gate: None, quest_clear_gate: None,
                ticket: Some(TravelTicketRequirement { track: track.into(), valid_for_seconds: 86400 }), world_change: None,
            };
            eprintln!("POSTGRES_STARTED NPCFIX_M38 elapsed travel ticket");
            for (index, active) in [false, false, false, true, true, false].into_iter().enumerate() {
                let fence = owner.current_quest_fence(deps.admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
                if index > 0 {
                    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                    let outcome = slot.commit_quest_transition(owner.root, owner.character, owner.holder, fence,
                        QuestTransitionRequest {
                            transition_key: format!("oteryn:quest-transition/npcfix/ticket_{}", index - 1),
                            cause: QuestCause::Command(crate::foundation::CommandRef::new(fence.game_session_id,
                                crate::foundation::CommandId::new(42000 + u64::try_from(index)?).map_err(debug)?)),
                        }, catalogue.clone()).await.map_err(debug)?;
                    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)), "ticket grant: {outcome:?}");
                }
                let current = owner.current_quest_fence(deps.admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
                let before: Option<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                    .bind(current.character_id.as_bytes().as_slice()).bind(track).fetch_optional(&h.pool).await?;
                for _ in 0..2 {
                    assert_eq!(h.root.quote_npc_travel(&authority, &h.node, current, route.clone(), &NoPremium).await.map_err(debug)?,
                        if active { Ok(100) } else { Err(TravelRefusal::UnavailableRoute) }, "ticket state {index}");
                }
                if active {
                    for invalid_lifetime in [0, u32::MAX] {
                        let mut invalid = route.clone();
                        invalid.ticket.as_mut().ok_or("ticket requirement")?.valid_for_seconds = invalid_lifetime;
                        assert_eq!(h.root.quote_npc_travel(&authority, &h.node, current, invalid, &NoPremium)
                            .await.map_err(debug)?, Err(TravelRefusal::UnavailableRoute));
                    }
                }
                let after: Option<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                    .bind(current.character_id.as_bytes().as_slice()).bind(track).fetch_optional(&h.pool).await?;
                assert_eq!(before, after, "using a ticket must not extend its purchase time");
            }
            let current = owner.current_quest_fence(deps.admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
            let mut legacy = route;
            legacy.ticket = None;
            assert_eq!(h.root.quote_npc_travel(&authority, &h.node, current, legacy, &NoPremium).await.map_err(debug)?, Ok(100));
            let writes: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_item_fee_burns)+(SELECT count(*) FROM game_character_pending_arrivals)+(SELECT count(*) FROM game_newhaven_travel_receipts)")
                .fetch_one(&h.pool).await?;
            assert_eq!(writes, 0);
            Ok(())
        }.await;
            h.cleanup().await?;
            result
        })
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38_TRAVEL_TICKET duration={:?}",
        started.elapsed()
    );
    Ok(())
}

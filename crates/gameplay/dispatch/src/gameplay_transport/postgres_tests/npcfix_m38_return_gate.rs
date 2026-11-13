//! M38: quote and commit both require the last completed inbound port.
use super::fixture::{Harness, debug, run};
use super::npcfix_m38_travel_guards::NoPremium;
use super::npcfix_m38_travel_return::{commit, complete, request, requirement};
use super::{ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session};
use crate::content::npc_catalogue::travel::TravelRoute;
use crate::durability::npc_travel::TravelRefusal;

const A: &str = "oteryn:service.travel.m38_origin_a/travora";
const B: &str = "oteryn:service.travel.m38_origin_b/travora";

#[test]
fn npcfix_m38_return_origin_is_rechecked_at_quote_and_commit_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("npcfix-m38-return-gate", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "m38_return_gate", true, false).await?;
            let result: TestResult = async {
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let deps = Dependencies::new()?;
                {
                    let mut runtime = deps.runtime.lock().await;
                    runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
                }
                let owner = deps.owner(&h, &authority);
                let session = deps.admitted.game_session_id;
                let mut back = request(&owner, &deps, 47000, "m38_return").await?;
                let legacy = back.route.clone();
                eprintln!("POSTGRES_STARTED NPCFIX_M38 return-origin quote/commit gate");
                quote(&owner, &deps, legacy, Ok(0)).await?;
                back.route.return_origin = Some(requirement(A));
                quote(&owner, &deps, back.route.clone(), Err(TravelRefusal::UnavailableRoute)).await?;

                let inbound_a = request(&owner, &deps, 47001, "m38_origin_a").await?;
                commit(&owner, session, inbound_a.clone()).await?;
                complete(&owner, &deps, inbound_a.arrival.occurrence).await?;
                clear_cooldown(&h).await?;
                quote(&owner, &deps, back.route.clone(), Ok(0)).await?;

                // The quoted origin changes before confirmation: the durable owner must recheck.
                let inbound_b = request(&owner, &deps, 47002, "m38_origin_b").await?;
                commit(&owner, session, inbound_b.clone()).await?;
                complete(&owner, &deps, inbound_b.arrival.occurrence).await?;
                clear_cooldown(&h).await?;
                quote(&owner, &deps, back.route.clone(), Err(TravelRefusal::UnavailableRoute)).await?;
                let fence = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("fence")?;
                let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                let refused = slot.commit_npc_travel(owner.root, owner.character, owner.holder, fence, back.clone(), &NoPremium).await.map_err(debug)?;
                assert!(matches!(refused, Err(TravelRefusal::UnavailableRoute)), "stale origin: {refused:?}");
                drop(slot);
                let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_newhaven_travel_receipts),(SELECT count(*) FROM game_character_pending_arrivals),(SELECT count(*) FROM game_item_fee_burns)").fetch_one(&h.pool).await?;
                assert_eq!(counts, (2, 0, 0), "refusal must not write travel or fees");

                back.route.return_origin = Some(requirement(B));
                quote(&owner, &deps, back.route.clone(), Ok(0)).await?;
                let mut malformed = back.route.clone();
                malformed.return_origin.as_mut().ok_or("return requirement")?.inbound_routes.clear();
                quote(&owner, &deps, malformed, Err(TravelRefusal::UnavailableRoute)).await?;
                commit(&owner, session, back.clone()).await?;
                complete(&owner, &deps, back.arrival.occurrence).await?;

                // A valid replay stays inert; a changed origin/group is a different binding.
                let fence = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("fence")?;
                let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                let replay = slot.commit_npc_travel(owner.root, owner.character, owner.holder, fence, back.clone(), &NoPremium).await.map_err(debug)?;
                assert!(matches!(replay, Ok(None)), "completed replay: {replay:?}");
                let mut downgraded = back.clone();
                downgraded.route.return_origin = None;
                assert!(slot.commit_npc_travel(owner.root, owner.character, owner.holder, fence, downgraded, &NoPremium).await.is_err(), "removing the origin guard must reject replay");
                back.route.return_origin = Some(requirement(A));
                assert!(slot.commit_npc_travel(owner.root, owner.character, owner.holder, fence, back.clone(), &NoPremium).await.is_err(), "changed expected origin must reject replay");
                let mut changed_group = requirement(B);
                changed_group.inbound_routes.push("oteryn:service.travel.m38_origin_c/travora".into());
                back.route.return_origin = Some(changed_group);
                assert!(slot.commit_npc_travel(owner.root, owner.character, owner.holder, fence, back, &NoPremium).await.is_err(), "changed inbound group must reject replay");
                drop(slot);
                let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_newhaven_travel_receipts),(SELECT count(*) FROM game_character_pending_arrivals),(SELECT count(*) FROM game_item_fee_burns)").fetch_one(&h.pool).await?;
                assert_eq!(counts, (3, 0, 0), "replays and reads must not create more travel or fees");
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        })
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38_RETURN_GATE duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn clear_cooldown(h: &Harness) -> TestResult {
    // Established fixture clock adjustment, not a wait or altered production policy.
    sqlx::query("UPDATE game_newhaven_travel_receipts SET arrived_at=arrived_at-interval '4 seconds' WHERE arrived_at IS NOT NULL")
        .execute(&h.pool).await?;
    Ok(())
}

async fn quote(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    route: TravelRoute,
    expected: Result<u64, TravelRefusal>,
) -> TestResult {
    let fence = owner
        .current_quest_fence(deps.admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    assert_eq!(
        owner
            .root
            .quote_npc_travel(owner.character, owner.holder, fence, route, &NoPremium)
            .await
            .map_err(debug)?,
        expected
    );
    Ok(())
}

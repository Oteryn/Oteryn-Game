//! M38: remember completed inbound receipts through the existing fenced travel owner.
use super::npcfix_m38_travel_return_fixtures::{A, B, commit, complete, request, requirement};

use super::fixture::{Harness, debug, run};
use super::npcfix_m38_travel_guards_fixtures::NoPremium;
use super::{ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session};

use crate::foundation::GameSessionId;

#[test]
fn npcfix_m38_travel_return_remembers_only_completed_inbound_ports_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("npcfix-m38-return", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "m38_travel_return", true, false)
                .await?;
            let result: TestResult = async {
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let deps = Dependencies::new()?;
                {
                    let mut runtime=deps.runtime.lock().await;
                    runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
                }
                let owner = deps.owner(&h, &authority);
                let session = deps.admitted.game_session_id;
                eprintln!("POSTGRES_STARTED NPCFIX_M38 completed return-port receipts");
                check(&owner, session, A, false).await?;
                check(&owner, session, B, false).await?;
                let first = request(&owner, &deps, 46001, "m38_origin_a").await?;
                commit(&owner, session, first.clone()).await?;
                check(&owner, session, A, false).await?;
                complete(&owner, &deps, first.arrival.occurrence).await?;
                check(&owner, session, A, true).await?;
                check(&owner, session, B, false).await?;
                // Advance the established travel fixture clock without changing logical receipt order.
                sqlx::query("UPDATE game_newhaven_travel_receipts SET arrived_at=arrived_at-interval '4 seconds' WHERE arrived_at IS NOT NULL")
                    .execute(&h.pool).await?;
                let second = request(&owner, &deps, 46002, "m38_origin_b").await?;
                commit(&owner, session, second.clone()).await?;
                check(&owner, session, A, true).await?;
                check(&owner, session, B, false).await?;
                complete(&owner, &deps, second.arrival.occurrence).await?;
                check(&owner, session, A, false).await?;
                check(&owner, session, B, true).await?;
                // Wall-clock repair or replay of an old arrival must not change the remembered port.
                sqlx::query("UPDATE game_newhaven_travel_receipts SET arrived_at=clock_timestamp()+interval '1 day' WHERE route=$1")
                    .bind(A).execute(&h.pool).await?;
                check(&owner, session, A, false).await?;
                check(&owner, session, B, true).await?;
                let fence=owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("fence")?;
                let mut slot=owner.revision_sequencer.acquire(fence.character_id).await;
                let replay=slot.commit_npc_travel(owner.root,owner.character,owner.holder,fence,first,&NoPremium).await.map_err(debug)?;
                assert!(matches!(replay,Ok(None)),"old travel replay: {replay:?}");
                drop(slot);
                check(&owner, session, B, true).await?;
                let mut stale=owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("fence")?;
                stale.connection_generation=crate::foundation::ConnectionGeneration::new(stale.connection_generation.get().checked_add(1).ok_or("generation overflow")?).map_err(debug)?;
                assert!(owner.root.npc_travel_return_permitted(owner.character,owner.holder,stale,requirement(B)).await.is_err());
                let counts: (i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_newhaven_travel_receipts),(SELECT count(*) FROM game_character_pending_arrivals),(SELECT count(*) FROM game_item_fee_burns)")
                    .fetch_one(&h.pool).await?;
                assert_eq!(counts,(2,0,0),"return eligibility reads must not create travel or fees");
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        })
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38_TRAVEL_RETURN duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    expected: &str,
    allowed: bool,
) -> TestResult {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    assert_eq!(
        owner
            .root
            .npc_travel_return_permitted(
                owner.character,
                owner.holder,
                fence,
                requirement(expected)
            )
            .await
            .map_err(debug)?,
        allowed,
        "return from {expected}"
    );
    Ok(())
}

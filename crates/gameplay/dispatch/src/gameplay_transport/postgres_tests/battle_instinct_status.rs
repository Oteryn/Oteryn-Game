//! C11's actual composed projection reads the current actor and fences a stale lease.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::battle_instinct_status::{
    BattleInstinctStatus, CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
};

#[test]
fn battle_instinct_status_postgres_composed_query_and_stale_lease() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("battle-instinct-status".into())
        .stack_size(64 << 20)
        .spawn(|| {
            run(async |admin| {
                reset();
                let h = Harness::create_with_admission_seed(
                    admin,
                    "battle_instinct_status",
                    true,
                    false,
                )
                .await?;
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let deps = Dependencies::new()?;
                let (controller, _) =
                    crate::content::native_gameplay::test_support::activated_with_loot_tables(
                        deps.admitted.world_id,
                        None,
                    );
                let active = controller.active().ok_or("active native generation")?;
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(active);
                assert!(owner.battle_instinct_status_available());
                deps.initialize(&owner).await;
                let current = owner
                    .current_quest_fence(deps.admitted.game_session_id)
                    .await
                    .map_err(|_| "current fence")?
                    .ok_or("current session")?;
                let fence = current.item_fence();
                deps.runtime
                    .lock()
                    .await
                    .bind_attacker_lease(
                        deps.actor,
                        deps.admitted.game_session_id,
                        crate::foundation::CharacterLease::new(
                            crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                                .map_err(debug)?,
                            fence.character_lease_generation,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
                let mut admitted = deps.admitted;
                admitted.item_fence = Some(fence);
                admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                    &[OfferedCapability {
                        id: CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
                        requires: &[],
                    }],
                    &[CAPABILITY_BATTLE_INSTINCT_STATUS_V1],
                )
                .ok_or("Battle Instinct capability")?;
                let payload = owner
                    .battle_instinct_status_query(admitted, 1)
                    .await
                    .ok_or("real composed reply")?;
                assert_eq!(
                    BattleInstinctStatus::decode(&payload),
                    Some(BattleInstinctStatus::default())
                );
                let mut stale = admitted;
                let mut stale_fence = fence;
                stale_fence.character_lease_generation += 1;
                stale.item_fence = Some(stale_fence);
                assert!(
                    owner.battle_instinct_status_query(stale, 2).await.is_none(),
                    "a stale item lease cannot read the current actor"
                );
                assert_eq!(
                    owner.battle_instinct_status_query(admitted, 3).await,
                    Some(payload)
                );
                eprintln!("BATTLE_INSTINCT_STATUS POSTGRES_CASE_COMPLETED");
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Battle Instinct PostgreSQL worker failed")??;
    Ok(())
}

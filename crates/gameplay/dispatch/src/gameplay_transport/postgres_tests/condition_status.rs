//! GAP-SV7 C08: actual composed session command consumes the native condition owner.
use super::*;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use oteryn_protocol_oteryn::condition_status::{
    CAPABILITY_CONDITION_STATUS_V1, COMMAND_TYPE_CONDITION_STATUS_QUERY, ConditionStatus,
};

#[test]
fn condition_status_postgres_real_dispatch_food_clear_and_current_lease_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "condition status PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "condition_status", false, false).await?;
        eprintln!(
            "POSTGRES_STARTED GAP-SV7 C08 condition_status_postgres_real_dispatch_food_clear_and_current_lease_fences"
        );
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        deps.admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &[OfferedCapability {
                id: CAPABILITY_CONDITION_STATUS_V1,
                requires: &[],
            }],
            &[CAPABILITY_CONDITION_STATUS_V1],
        )
        .ok_or("capability")?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        let fence = admitted.item_fence.ok_or("fence")?;
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let now = owner.owner_now().get();
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.try_lock().map_err(debug)?;
            let state = states
                .get_mut(&runtime, deps.actor, admitted.game_session_id)
                .ok_or("actor")?;
            *state = state.after_food(180, now / 1_000).ok_or("food")?;
        }
        let command_id = admitted.continuity.next_command_id;
        let (status, bytes) = super::cyclopedia::support::exchange(
            &owner,
            &mut admitted,
            command_id,
            COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
        assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
        let projection = ConditionStatus::decode(&bytes).ok_or("decode")?;
        let food = projection
            .conditions
            .iter()
            .find(|condition| condition.icon == 15)
            .ok_or("food status")?;
        assert!((1..=180_000).contains(&food.remaining_ms));
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.try_lock().map_err(debug)?;
            let state = states
                .get_mut(&runtime, deps.actor, admitted.game_session_id)
                .ok_or("actor")?;
            let before = state.owned_conditions().clone();
            let mut next = before.clone();
            next.remove_type(
                oteryn_gameplay_domain::domain::condition::ConditionType::FoodRegeneration,
            );
            state
                .apply_batch_conditions(&before, &next)
                .map_err(debug)?;
        }
        let command_id = admitted.continuity.next_command_id;
        let (status, bytes) = super::cyclopedia::support::exchange(
            &owner,
            &mut admitted,
            command_id,
            COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
        assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
        assert!(
            ConditionStatus::decode(&bytes)
                .ok_or("empty decode")?
                .conditions
                .is_empty()
        );
        let mut stale = admitted;
        stale
            .item_fence
            .as_mut()
            .ok_or("stale fence")?
            .character_lease_generation += 1;
        assert!(owner.condition_status_page(stale).await.is_none());
        stale = admitted;
        stale.game_session_id = crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?;
        assert!(owner.condition_status_page(stale).await.is_none());
        stale = admitted;
        stale.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert!(owner.condition_status_page(stale).await.is_none());
        Ok(())
    })
}

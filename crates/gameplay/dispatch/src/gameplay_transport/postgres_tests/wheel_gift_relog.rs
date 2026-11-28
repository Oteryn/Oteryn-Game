use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::gameplay_transport::connection::{
    AdmittedSession, ControllerBinding, FirstEntryOutcome, LogoutReply,
    route_wheel_gift_status_for_test,
};
use crate::gameplay_transport::item_fence_of;
use crate::gameplay_transport::last_position::AdmissionPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt as _;
use oteryn_protocol_oteryn::{CommandStatus, logout::LogoutResult, wheel_presentation};
use std::time::Duration;

#[test]
fn wheel_gift_status_recovers_saved_cooldown_after_relog() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_configured_admin| {
        scenario(admin).await
    })
}

async fn scenario(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "wheel_gift_relog", false, false).await?;
    let request = prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (generation, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    super::tutorial_vitals_fixture::publish_native_activation(&h, &pin).await?;
    let active = generation.active().ok_or("active content")?;
    super::tutorial_vitals_fixture::native_asset_revisions(&h, active).await?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let room = pin.qualified_room().clone();
    let (channel_pin, _, _, _) = pin.into_channel_parts();
    let deps = Dependencies::with_content(channel_pin, room, native.spell_book().clone())?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    super::tutorial_vitals_fixture::admit(&owner, &deps, &request).await?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let original_session = admitted.game_session_id;
    admitted.item_fence = item_fence_of(store.current_session(original_session).await?);
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_loop_training(&mut admitted).await;
    assert_eq!(admitted.first_entry, FirstEntryOutcome::Positioned);
    let fence = owner
        .current_quest_fence(original_session)
        .await
        .map_err(|()| "position fence unavailable")?
        .ok_or("position fence absent")?;
    assert!(owner.track_restored(
        original_session,
        deps.actor,
        &AdmissionPosition::FirstLogin,
        fence
    ));
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, deps.actor, original_session)
            .ok_or("spell state")?;
        // Simulate a dormant cooldown carried over from a previous allocation.
        assert!(state.restore_wheel_gift(3_600_000_000, owner.owner_now().get()));
    }
    assert_eq!(
        owner.logout_session(admitted).await,
        LogoutReply::Result(LogoutResult::Accepted)
    );
    let saved = h
        .root
        .read_character_wheel_gift(&authority, fence.character_id)
        .await?;
    assert!(saved > 0, "logout saves the populated cooldown");

    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(120))
        .ok_or("advance the Channel clock during logout")?;
    let (session, request) = super::full_world_support::relogin_request(&h).await?;
    let current = store.current_session(session).await?;
    let transport = current.current_transport().ok_or("relog transport")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
        runtime.commit_fresh_session(reserved).map_err(debug)?
    };
    let (first_entry, _, _) =
        Box::pin(owner.initialize_first_entry(&request, session, transport, actor)).await;
    assert!(matches!(
        first_entry,
        FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
    ));
    let readmitted = AdmittedSession {
        game_session_id: session,
        runtime_actor: Some(actor),
        item_fence: item_fence_of(current),
        controller: Some(ControllerBinding {
            transport,
            account_id: id(40),
        }),
        first_entry,
        ..admitted
    };

    // Isolate the Gift load from the independent PvP snapshot's holder retry.
    assert!(owner.refresh_pvp_locks(actor, session).await);
    h.root.maintain_ready_once().await?;
    let busy = h.root.try_issue_semantic_pass()?;
    let _ = owner.observe_join_vitals(actor, readmitted, false).await;
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let state = states
            .get(&runtime, actor, session)
            .ok_or("re-admitted spell state")?;
        assert!(
            !state.wheel_gift_loaded(),
            "join could not read the timer while the holder was busy"
        );
    }
    busy.run(|_, _| Box::pin(async { Ok(()) })).await?;
    let clock = owner.freeze_owner_clock_for_test();
    let (disposition, status) = route_wheel_gift_status_for_test(&owner, &readmitted)
        .await
        .ok_or("Gift status reply")?;
    assert_eq!(
        disposition,
        CommandStatus::Accepted,
        "a current status request must recover the saved timer after a missed join load"
    );
    assert_eq!(status.len(), wheel_presentation::GIFT_OF_LIFE_STATUS_BYTES);
    assert_eq!(status[0], 0, "no allocation makes Gift inactive");
    let remaining = u64::from_le_bytes(status[1..].try_into()?);
    assert_eq!(remaining, 0, "an inactive perk cannot display a timer");
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let state = states.get(&runtime, actor, session).ok_or("re-admitted spell state")?;
        assert_eq!(
            state.wheel_gift_save(owner.owner_now().get(), true).map(|(_, remaining)| remaining),
            Some(saved),
            "offline time leaves the recovered cooldown untouched"
        );
    }
    assert_eq!(
        route_wheel_gift_status_for_test(&owner, &admitted).await,
        Some((CommandStatus::Rejected, Vec::new())),
        "the retired session cannot read the new actor's status"
    );

    drop(clock);
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

//! Remaining mana shield through the framed gameplay command and native combat owner.
use super::cyclopedia_fixtures::support::exchange;
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::player_lethal::PlayerLethalVitals;
use crate::spell::combat_batch::{ManaShieldState, OwnerCombatChange};
use oteryn_protocol_oteryn::CommandStatus;

#[test]
fn mana_shield_remaining_postgres_real_dispatch_after_damage() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "mana shield scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "mana_shield_remaining", true, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        let session = admitted.game_session_id;
        admitted.first_entry = FirstEntryOutcome::Positioned;

        let (status, bytes) = exchange(&owner, &mut admitted, 1, 6372, &[]).await?;
        assert_eq!((status, bytes), (CommandStatus::Rejected, vec![]));
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &[OfferedCapability {
                id: 605,
                requires: &[],
            }],
            &[605],
        )
        .ok_or("live state capability")?;
        let (status, bytes) = exchange(&owner, &mut admitted, 2, 6372, &[]).await?;
        assert_eq!(
            (status, bytes),
            (CommandStatus::Accepted, 0_u32.to_le_bytes().to_vec())
        );

        {
            let mut runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let now = owner.owner_now();
            let state = states
                .get_mut(&runtime, deps.actor, session)
                .ok_or("spell state")?;
            crate::spell::actor_conditions::apply_combat_change(
                state,
                &OwnerCombatChange::ManaShield(ManaShieldState {
                    capacity: 80,
                    expires_ms: now.get() / 1_000 + 60_000,
                }),
                now.get() / 1_000,
            )
            .map_err(debug)?;
            let damage = states
                .apply_typed_attack_damage(
                    &mut runtime,
                    deps.actor,
                    session,
                    20,
                    crate::player_lethal::PlayerHitKind::Fire,
                    "test:mana-shield-remaining",
                    now,
                )
                .ok_or("native incoming fire damage")?;
            assert_eq!(damage.applied, 0);
        }

        let (status, bytes) = exchange(&owner, &mut admitted, 3, 6372, &[]).await?;
        assert_eq!(
            (status, bytes),
            (CommandStatus::Accepted, 60_u32.to_le_bytes().to_vec())
        );
        let (status, bytes) = exchange(&owner, &mut admitted, 4, 6372, &[0]).await?;
        assert_eq!((status, bytes), (CommandStatus::Rejected, vec![]));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

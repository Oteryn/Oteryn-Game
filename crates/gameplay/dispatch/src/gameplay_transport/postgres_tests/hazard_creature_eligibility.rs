use super::fixture::{Harness, debug, fence};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_combat::{creature_auto_attack::AttackFacts, hazard::incoming_damage};
use oteryn_gameplay_content::content::bosses::primal_closure::{
    FUNGOSAURUS_KEY, PRIMAL_MENACE_KEY,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn gnomprona_masterless_primal_creatures_apply_incoming_hazard_damage() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_async(async move |_| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "hazard_eligibility", true, false).await?;
        prepare_current_session(&h).await?;
        let world = fence(1)?.runtime_scope.world_id();
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(|generation| generation.native_gameplay())
            .ok_or("active source creature policies")?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content_and_capacity(
            pin.into_channel_pin(),
            room,
            native.spell_book().clone(),
            8,
        )?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        assert!(owner.admit_hazard(deps.admitted).await);

        {
            let mut runtime = owner.runtime.lock().await;
            let session = deps.admitted.game_session_id;
            let player = deps.actor;
            let mut position = MovementLocalPosition {
                x: 33502,
                y: 32740,
                floor: 13,
            };
            runtime
                .initialize_pinned_test_position(player, position)
                .map_err(debug)?;
            // The source-profile fixture supplies creature policies. Install the projection
            // that a qualified World bundle would supply so this case isolates combat eligibility.
            runtime
                .install_hazard_world_frame(player, session, "oteryn:frame/world-bundle-v3")
                .map_err(debug)?;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;

            for key in [
                "oteryn:creature.gore_horn",
                FUNGOSAURUS_KEY,
                PRIMAL_MENACE_KEY,
                "oteryn:creature.gore_horn_primal",
            ] {
                position.x += 1;
                let policy = runtime.companion_policy(key).map_err(debug)?;
                let monster = runtime
                    .admit_encounter_actor(key, policy.maximum_health, position)
                    .map_err(debug)?;
                runtime
                    .install_creature_policy(monster, key)
                    .map_err(debug)?;
                let hit = AttackFacts {
                    issuer: monster,
                    target: player,
                    session,
                    revision: 1,
                    visible: true,
                    issuer_pz: false,
                    target_pz: false,
                    issuer_protected: false,
                    target_protected: false,
                    defense: 0,
                    armor: 0,
                    player_level: 1,
                    reference_level_party_lowest: None,
                };
                let scaled = incoming_damage(&runtime, &hit, 100, 999_999, 0, false)
                    .map_err(debug)?;
                assert!(!scaled.critical);
                assert!(scaled.amount > 100, "Hazard damage boost for {key}");
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::kill_reward::DrainOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn boss_outfits_ratmiral_grants_rascoohan_once_on_native_death() -> TestResult {
    exercise("oteryn:creature.ratmiral_blackwhiskers", "rascoohan", true)
}

#[test]
fn boss_outfits_lord_retro_grants_formal_dress_once_on_native_death() -> TestResult {
    exercise("oteryn:creature.lord_retro", "formal_dress", true)
}

#[test]
fn boss_outfits_pale_worm_does_not_grant_poltergeist_outside_its_rectangle() -> TestResult {
    exercise("oteryn:creature.the_pale_worm", "poltergeist", false)
}

#[test]
fn boss_outfits_arbaziloth_does_not_grant_fiend_slayer_outside_its_room() -> TestResult {
    exercise("oteryn:creature.arbaziloth", "fiend_slayer", false)
}

fn exercise(creature: &'static str, outfit: &'static str, earns: bool) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(creature, outfit, earns).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "boss outfit PostgreSQL scenario panicked")??;
    Ok(())
}

fn scenario(creature: &str, outfit: &str, earns: bool) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "boss_outfits", true, false).await?;
        prepare_current_session(&h).await?;
        let world = super::fixture::fence(1)?.runtime_scope.world_id();
        let (generation, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world,
                "content-1",
            )?;
        let active = generation.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let room = pin.qualified_room().clone();
        let runtime_pin = pin.into_channel_pin();
        let deps = Dependencies::with_content_and_capacity(
            runtime_pin,
            room,
            native.spell_book().clone(),
            4,
        )?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id).await?.0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
        admitted.item_fence = Some(fence);
        {
            let mut runtime = owner.runtime.lock().await;
            runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
            native.install_companion_policies(&mut runtime).map_err(debug)?;
        }
        owner.admit_progression_session(&admitted).await;
        for command in [1, 2] {
            let target = super::qengine_kill_stories_fixtures::queue_death(
                &owner, &admitted, creature, command,
            ).await?;
            assert_eq!(owner.drain_kill_rewards(admitted.game_session_id).await, DrainOutcome::Drained);
            let grants: Vec<String> = sqlx::query_scalar(
                "SELECT unlock_key FROM game_account_appearance_unlock_receipts WHERE source_kind='quest' ORDER BY unlock_key",
            ).fetch_all(&h.pool).await?;
            let expected = earns.then(|| format!("oteryn:outfit.{outfit}/quest-grant/0"));
            assert_eq!(grants, expected.into_iter().collect::<Vec<_>>());
            super::qengine_kill_stories_fixtures::drain_deaths(&owner, &admitted, &[target]).await?;
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

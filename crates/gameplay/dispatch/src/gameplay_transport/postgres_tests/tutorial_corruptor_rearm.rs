//! Exercise the reported vocationless death/Forcefield path against durable tutorial state.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::ChannelContentPin;
use oteryn_gameplay_quests::tutorial::{
    TutorialEvent, TutorialStep, corruptor_items, tutorial_transition,
};

#[test]
fn tutorial_corruptor_death_forcefield_rearm_has_no_vocationless_kit() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "tutorial rearm scenario panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "tutorial_rearm", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
        let active = controller.active().ok_or("active generation")?;
        let room = pin.qualified_room().clone();
        let channel_pin = ChannelContentPin::from_activation(
            world, pin.activation_sequence(), pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(), pin.frame_binding().digest(),
            room.map_revision_digest(), (32534, 32513, 7),
        );
        let deps = Dependencies::with_content(channel_pin, room, crate::spell::cast::v1_spell_book().map_err(debug)?)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|_| "fence")?.ok_or("fence")?;
        h.root.initialize_familiar_group(&authority, &h.node, fence).await.map_err(debug)?;
        let build = h.root.read_character_build_state(&authority, fence.character_id).await.map_err(debug)?;
        assert_eq!(build.vocation(), "none");
        let event = async |event| owner.tutorial_event(fence, event).await.map_err(|_| "tutorial event");
        let first = event(TutorialEvent::Forcefield).await?.ok_or("first Forcefield")?;
        assert!(first.progress.corruptor);
        event(TutorialEvent::Corruptor).await?.ok_or("consume arm")?;
        let death = event(TutorialEvent::Death).await?.ok_or("vocationless death")?;
        assert_eq!(death.progress.step, TutorialStep::Islet);
        let rearmed = event(TutorialEvent::Forcefield).await?.ok_or("second Forcefield")?;
        assert!(rearmed.progress.corruptor);
        // The exact selection used by prepare_tutorial_corruptor produces no kit.
        assert!(corruptor_items(build.vocation()).is_empty());
        for vocation in ["knight", "paladin", "sorcerer", "druid", "monk"] {
            assert!(!corruptor_items(vocation).is_empty());
            // A character eligible for an actual kit cannot take the reported return path.
            assert!(tutorial_transition(true, Some(first.progress), TutorialEvent::Death, vocation, false).is_none());
            assert!(tutorial_transition(true, Some(first.progress), TutorialEvent::Login, vocation, false).is_none());
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

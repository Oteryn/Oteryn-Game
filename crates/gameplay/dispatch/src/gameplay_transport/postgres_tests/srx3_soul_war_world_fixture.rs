//! Actual published whole-World geometry for Soul War STEP/raid regressions.
use super::srx3_soul_war_world_fixture_fixtures::{source_world};

use super::TestResult;

use crate::foundation::{ChannelContentPin, MovementLocalPosition};

#[test]
fn srx3_published_world_callback_context_requires_exact_current_world_pins() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("srx3-world-callback-proof", || {
        use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Source;
        let started = std::time::Instant::now();
        let scene = source_world(MovementLocalPosition {
            x: 33587,
            y: 32378,
            floor: 12,
        })?;
        let active = scene.controller.active().ok_or("actual active World")?;
        let sources = [
            Source::CallbackProfessorMaxxen,
            Source::CallbackZamulosh,
            Source::CallbackAnomaly,
            Source::CallbackOutburst,
            Source::CallbackTimeGuardian,
            Source::CallbackLadyTenebris,
        ];
        for source in sources {
            let proof = active
                .qualify_callback_source_context(&scene.room, &scene.pin, source)
                .ok_or("published World must qualify the real closed callback geometry")?;
            assert!(proof.matches(&scene.room, &scene.pin));
            assert_eq!(proof.source(), source);
            assert_eq!(
                proof.source_frame().physical_position([33587, 32378, 12]),
                Some((33587, 32378, 12))
            );
        }
        for changed in 0..5 {
            let pin = ChannelContentPin::from_activation(
                scene.pin.world_id(),
                scene.pin.activation_sequence() + u64::from(changed == 0),
                if changed == 1 {
                    [44; 32]
                } else {
                    scene.pin.server_artifact_digest()
                },
                if changed == 2 {
                    [45; 32]
                } else {
                    scene.pin.client_artifact_digest()
                },
                if changed == 3 {
                    [46; 32]
                } else {
                    scene.pin.frame_binding_digest()
                },
                if changed == 4 {
                    [47; 32]
                } else {
                    scene.pin.map_revision_digest()
                },
                (33587, 32378, 12),
            );
            assert!(
                active
                    .qualify_callback_source_context(&scene.room, &pin, Source::CallbackAnomaly)
                    .is_none()
            );
        }
        eprintln!(
            "SRX3_PUBLISHED_WORLD_CALLBACK_CONTEXT_EXECUTED duration={:?}",
            started.elapsed()
        );
        Ok(())
    })
}

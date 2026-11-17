//! Actual source room qualification and fenced durable publication, not a fight fixture.
use super::TestResult;
use super::fixture::{Harness, debug, id};
use crate::content::native_gameplay::NativeGameplayInput;
use crate::durability::activity_handoff::{ActivityContentPin, ActivityHandoffOwner};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::GameSessionId;

#[test]
fn m26_spite_postgres_production_source_room_qualification_and_publication() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        super::reset();
        let h = Harness::create_with_admission_seed(admin, "m26_spite_qualification", true, false)
            .await?;
        super::prepare_current_session(&h).await?;
        let world = super::fixture::fence(1)?.runtime_scope.world_id();
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json");
        let mut input = NativeGameplayInput::from_manifest(&manifest).map_err(debug)?;
        input
            .build_training
            .as_mut()
            .ok_or("production training")?
            .content_revision = "content-1".into();
        let (controller, source_pin) =
            crate::content::native_gameplay::test_support::try_activate_input(world, &input)
                .map_err(debug)?;
        let room = source_pin.qualified_room();
        let candidate = crate::content::bosses::activity_template::captured_boss_levers()
            .map_err(debug)?
            .into_iter()
            .find(|row| row.creature_key.as_deref() == Some("oteryn:creature.goshnar_s_spite"))
            .ok_or("source Spite candidate")?;
        let template = controller
            .qualify_activity_template(room, room, "content-1", &candidate)
            .map_err(debug)?;
        assert_eq!(template.minimum_level(), 250);
        assert_eq!(template.maximum_players(), 5);
        assert_eq!(template.cooldown_seconds(), 72000);
        assert_eq!(template.after_kill_seconds(), 60);
        assert!(template.spite_source_setup().is_some());
        for other in crate::content::bosses::activity_template::captured_boss_levers()
            .map_err(debug)?
            .into_iter()
            .filter(|row| row.path == candidate.path && row.line != candidate.line)
        {
            assert_eq!(
                controller.qualify_activity_template(room, room, "content-1", &other).err(),
                Some(crate::content::bosses::activity_template::ActivityTemplateRefusal::SourceHookUnavailable),
                "unimplemented source room {}", other.line
            );
        }
        let mut stripped = candidate.clone();
        stripped.required_callbacks.clear();
        assert_eq!(
            controller.qualify_activity_template(room, room, "content-1", &stripped).err(),
            Some(crate::content::bosses::activity_template::ActivityTemplateRefusal::InvalidSourceCapture)
        );
        let pin = source_pin.into_channel_pin();
        super::boss_activity::issue_content_and_reset(&h, &pin).await?;
        let store = ActivityHandoffOwner::new(h.root.clone());
        store
            .publish_qualified_boss_rooms(&h.node, std::slice::from_ref(&template))
            .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(GameSessionId::decode(&id(50)).map_err(debug)?)
            .await?
            .0;
        let fence =
            crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
        let origin = ActivityContentPin {
            coordinate_frame_digest: pin.frame_binding_digest(),
            map_revision_digest: pin.map_revision_digest(),
            server_artifact_digest: pin.server_artifact_digest(),
        };
        let persisted = store
            .read_boss_room_with_pin(&authority, &h.node, fence, template.boss_key(), origin)
            .await?
            .ok_or("published source template")?;
        assert_eq!(
            persisted.boss_creature_key,
            "oteryn:creature.goshnar_s_spite"
        );
        assert_eq!(persisted.policy.cooldown_ms, 72_000_000);
        assert_eq!(persisted.template_revision, template.template_revision());
        assert_eq!(persisted.boss_position.x, 33743 - 5854);
        assert_eq!(persisted.boss_position.y, 31632 - 5298);
        assert_eq!(persisted.boss_position.floor, -14);
        // Publication is idempotent and grants neither admission nor a reward.
        store
            .publish_qualified_boss_rooms(&h.node, std::slice::from_ref(&template))
            .await?;
        let admissions: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(admissions, 0);
        Ok(())
    });
    eprintln!(
        "M26_SPITE_QUALIFICATION POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

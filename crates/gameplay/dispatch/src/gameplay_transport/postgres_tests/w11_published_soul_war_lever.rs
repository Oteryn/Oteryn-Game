//! The real Soul War lever resolves its pinned destination from the published World.
use super::fixture::{Harness, debug};
use super::srx3_activity_source_fixture::prepare_player;
use super::{TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use oteryn_gameplay_quests::quest::soul_war::MINIMUM_LEVEL;
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn w11_soul_war_lever_enters_pinned_spite_from_published_world_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        scenario(admin, PinnedActivitySourceRoom::GoshnarsSpite).await
    })
}

#[test]
fn w11_soul_war_published_final_lever_refuses_before_chain_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        scenario(admin, PinnedActivitySourceRoom::GoshnarsMegalomania).await
    })
}

async fn scenario(admin: String, source: PinnedActivitySourceRoom) -> TestResult {
    reset();
    let (creature, expect_entry) = match source {
        PinnedActivitySourceRoom::GoshnarsSpite => ("oteryn:creature.goshnar_s_spite", true),
        PinnedActivitySourceRoom::GoshnarsMegalomania => {
            ("oteryn:creature.goshnar_s_megalomania_purple", false)
        }
        _ => return Err("unexpected W11 source room".into()),
    };
    let candidate = crate::content::bosses::activity_template::captured_boss_levers()
        .map_err(debug)?
        .into_iter()
        .find(|row| row.creature_key.as_deref() == Some(creature))
        .ok_or("captured Soul War lever")?;
    let &[x, y, z] = candidate
        .lever_positions
        .first()
        .ok_or("first Soul War pad")?;
    let (scene, _, _) = super::srx3_soul_war_world_fixture_fixtures::source_world_with_bundle(
        MovementLocalPosition {
            x,
            y,
            floor: i16::try_from(z)?,
        },
    )?;
    let active = scene.controller.active().ok_or("active World generation")?;
    let source_room = active
        .qualify_activity_source_room(&scene.room, source)
        .map_err(debug)?;
    let source_room = crate::gameplay_transport::activity_runtime::source_bundle::qualify(
        &source_room,
        source,
        active,
        scene.pin.world_id(),
        super::fixture::fence(1)?
            .runtime_scope
            .channel_id()
            .ok_or("World Channel")?,
    )
    .map_err(|_| "pinned Soul War room")?;
    assert_ne!(
        source_room.frame_binding().digest(),
        scene.pin.frame_binding_digest()
    );

    let deps = scene.dependencies()?;
    let h =
        Harness::create_with_admission_seed(admin, "w11_published_soul_war", false, false).await?;
    let evidence = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
    super::house_transfer_fixtures::prepare_owned_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&scene.room));
    owner
        .activity_runtimes
        .register_source_room(source, &source_room)
        .map_err(|_| "register pinned Soul War destination")?;
    super::boss_activity_fixtures::issue_content_and_reset(&h, &scene.pin).await?;
    let mut admitted = deps.admitted;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[
            4,
            6,
            7,
            10,
            11,
            15,
            18,
            23,
            92,
            wire::CAPABILITY_BOSS_ENCOUNTER_V1,
        ],
    )
    .ok_or("native boss capabilities")?;
    prepare_player(&h, &owner, &mut admitted, MINIMUM_LEVEL).await?;
    let grant = super::soul_war_fixtures::command(
        &owner,
        admitted.game_session_id,
        1,
        crate::durability::quest_state::soul_war::SoulWarCommand::Start,
    )
    .await?;
    assert_eq!(grant.result, Ok(()));
    admitted.item_fence = Some(
        super::soul_war_fixtures::current(&owner, admitted.game_session_id)
            .await?
            .item_fence(),
    );
    owner
        .activity_runtimes
        .remember(admitted)
        .map_err(|_| "current World connection")?;

    let destination = super::activity_runtime_fixtures::allocate_ready_instance(&h, 218).await?;
    let origin = admitted
        .item_fence
        .ok_or("current World fence")?
        .runtime_scope;
    let (_, result, barrier) =
        super::boss_activity_fixtures::exchange(&owner, &mut admitted, creature, expect_entry)
            .await?;
    let current = admitted.item_fence.ok_or("current lever fence")?;
    if expect_entry {
        assert!(matches!(result, wire::BossActivityReply::Accepted { .. }));
        assert!(barrier.is_some());
        assert_eq!(current.runtime_scope, destination);
    } else {
        assert_eq!(
            result,
            wire::BossActivityReply::Refused(wire::BossActivityRefusal::AuthorityRejected),
            "access is granted but the final chain is incomplete"
        );
        assert!(barrier.is_none());
        assert_eq!(current.runtime_scope, origin);
        let handoff_exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_activity_handoff_groups)")
                .fetch_one(&h.pool)
                .await?;
        assert!(!handoff_exists);
    }
    eprintln!("W11_PUBLISHED_SOUL_WAR POSTGRES_CASE_COMPLETED");
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

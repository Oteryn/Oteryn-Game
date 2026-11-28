use super::classic_quests_fixtures::progress;
use super::fixture::{Harness, debug};
use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_scene_with_appearances};
use super::qengine_outfit_stories_fixtures::seed_quest_prerequisites;
use super::qengine_voucher_stories_fixtures::{mint, quantity, use_native};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const MOUND: Position = Position {
    x: 32067,
    y: 32715,
    floor: -10,
};
pub(super) const START: Position = Position { y: 32714, ..MOUND };

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(
                "../../../../../../../content/world/placements/region-z10-x125-y127.b3"
            ),
            region: (10, 125, 127),
            bounds: (32065, 32713, 32070, 32718),
            start: (START.x, START.y),
            expected: vec![ExpectedTile {
                position: (MOUND.x, MOUND.y),
                ids: vec![12355, 19211],
                counted_entry: None,
            }],
        }],
        &[],
        &[19212],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    const PHASE: &str = "oteryn:quest-progress/spike/release/progress";
    super::questpc6_fibula_route_fixtures::level(owner, session, 25).await?;
    let mut mint_id = 4000;
    mint(owner, session, 19203, 1, &mut mint_id).await?;
    seed_quest_prerequisites(h, owner, session, &[(PHASE.into(), 1)], 120).await?;
    owner
        .refresh_quest_session(session.game_session_id, false)
        .await;
    for (draw, species) in [
        (1, None),
        (50, Some("zombie")),
        (70, Some("demon_skeleton")),
        (80, Some("ghost")),
        (90, Some("enraged_soul")),
    ] {
        owner
            .states
            .quest_callbacks
            .mound_draw
            .store(draw, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(
            super::tibia_tales_fixtures::use_tool_at(owner, session, MOUND, 1, 19203).await?,
            UseDisposition::Committed
        );
        assert_eq!(quantity(h, 19203).await?, i64::from(draw != 90));
        assert_eq!(
            progress(owner, session, authority, PHASE).await?,
            if draw == 90 { 2 } else { 1 }
        );
        owner.clock_origin -= std::time::Duration::from_secs(1);
        owner.drain_quest_encounter_deadlines().await;
        owner.clock_origin -= std::time::Duration::from_secs(121);
        owner.drain_quest_encounter_deadlines().await;
        owner
            .bundle_map()
            .ok_or("mound map")?
            .facts
            .expire_movement_items(owner.owner_now().get());
        if let Some(species) = species {
            let mut runtime = owner.runtime.lock().await;
            let actors = runtime.current_live_creature_census().map_err(debug)?;
            let [actor] = actors.as_slice() else {
                return Err("expected one surviving mound creature".into());
            };
            assert_eq!(
                runtime
                    .companion_snapshot(*actor)
                    .map_err(debug)?
                    .state
                    .policy
                    .definition_key,
                format!("oteryn:creature.{species}")
            );
            runtime.remove_encounter_actor(*actor).map_err(debug)?;
        }
    }
    for (name, item, steps) in [("tracking", 19205, 3), ("temperature", 19206, 1)] {
        mint(owner, session, item, 1, &mut mint_id).await?;
        let track = format!("oteryn:quest-progress/spike/{name}/progress");
        for step in 1..=steps {
            seed_quest_prerequisites(
                h,
                owner,
                session,
                &[(track.clone(), i64::from(step))],
                130 + step + u8::from(item == 19206) * 4,
            )
            .await?;
            owner
                .refresh_quest_session(session.game_session_id, false)
                .await;
            use_native(owner, session, item, UseDisposition::NotPossible).await?;
            assert_eq!(
                progress(owner, session, authority, &track).await?,
                i64::from(step)
            );
            assert_eq!(quantity(h, item).await?, 1);
        }
    }
    Ok(())
}

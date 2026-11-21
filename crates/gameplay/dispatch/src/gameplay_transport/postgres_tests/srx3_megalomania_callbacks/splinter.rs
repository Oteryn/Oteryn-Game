//! Continue the real committed source entry through its native XML owner and
//! the same production monster pass that drains the one original 120s timer.
use super::*;
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_simulation_determinism::SemanticTimeMicros;

const LESSER: &[u8] = b"oteryn:creature.lesser_splinter_of_madness";
const GREATER: &[u8] = b"oteryn:creature.greater_splinter_of_madness";
const MIGHTY: &[u8] = b"oteryn:creature.mighty_splinter_of_madness";

pub(super) async fn actual_xml_growth(view: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    let point = {
        let runtime = view.runtime.lock().await;
        let (point, _, actor) = runtime
            .spawn_point_creatures()
            .into_iter()
            .find(|(_, position, _)| {
                *position
                    == MovementLocalPosition {
                        x: 33712,
                        y: 31635,
                        floor: 14,
                    }
            })
            .ok_or("actual pinned Lesser XML placement")?;
        assert!(
            actor.is_none(),
            "entry registers an empty XML point, never a synthetic initial Lesser"
        );
        point
    };
    // Three source warnings and admission share the actual existing deadline
    // owner. Earlier independent callbacks may consume a pass at the same time.
    let mut admission_at = None;
    for _ in 0..4 {
        let before = view
            .runtime
            .lock()
            .await
            .pending_respawn(point)
            .ok_or("actual pending XML warning/admission")?;
        for _ in 0..8 {
            view.drain_monster_melee_at(SemanticTimeMicros::from_micros(before.due_micros))
                .await;
            if view.runtime.lock().await.pending_respawn(point) != Some(before) {
                break;
            }
        }
        let runtime = view.runtime.lock().await;
        let current = runtime
            .spawn_point_creatures()
            .into_iter()
            .find(|(candidate, _, _)| *candidate == point)
            .ok_or("retained exact XML point")?;
        if current.2.is_some() {
            assert!(runtime.pending_respawn(point).is_none());
            admission_at = Some(before.due_micros);
            break;
        }
        assert_eq!(
            runtime.pending_respawn(point).map(|next| next.due_micros),
            Some(before.due_micros + 1_400_000),
            "each remaining source warning schedules the next exact1.4s stage"
        );
    }
    assert!(
        admission_at.is_some(),
        "the bounded source warning sequence admits its actual Lesser"
    );
    let (actor, due) = {
        let runtime = view.runtime.lock().await;
        let actor = runtime
            .spawn_point_creatures()
            .into_iter()
            .find(|(candidate, _, _)| *candidate == point)
            .and_then(|(_, _, actor)| actor)
            .ok_or("actual admitted Lesser incarnation")?;
        assert!(runtime.matches_live_creature_identity(actor, LESSER));
        let current = runtime.companion_snapshot(actor).map_err(debug)?;
        assert_eq!(
            current.state.created_at_us, admission_at,
            "birth is the actual admission turn, not entry or warning"
        );
        let due = runtime
            .soul_war_splinter_due_us(actor)
            .map_err(debug)?
            .ok_or("actual Lesser source timer")?;
        assert_eq!(
            Some(due),
            admission_at.and_then(|at| at.checked_add(120_000_000))
        );
        (actor, due)
    };
    view.drain_monster_melee_at(SemanticTimeMicros::from_micros(due - 1))
        .await;
    assert!(
        view.runtime
            .lock()
            .await
            .matches_live_creature_identity(actor, LESSER),
        "the original120s deadline is not early"
    );
    for _ in 0..8 {
        view.drain_monster_melee_at(SemanticTimeMicros::from_micros(due))
            .await;
        if view
            .runtime
            .lock()
            .await
            .matches_live_creature_identity(actor, GREATER)
        {
            break;
        }
    }
    {
        let runtime = view.runtime.lock().await;
        assert!(
            runtime.matches_live_creature_identity(actor, GREATER),
            "the actual detached source timer changes the same XML actor"
        );
        let after = runtime.companion_snapshot(actor).map_err(debug)?;
        assert_eq!((after.health, after.maximum_health), (4000, 4000));
        assert_eq!(after.state.created_at_us, admission_at);
        assert_eq!(
            runtime.soul_war_splinter_due_us(actor).map_err(debug)?,
            None
        );
    }
    for _ in 0..8 {
        view.drain_monster_melee_at(SemanticTimeMicros::from_micros(due + 120_000_000))
            .await;
    }
    let runtime = view.runtime.lock().await;
    assert!(
        runtime.matches_live_creature_identity(actor, GREATER),
        "setType does not invoke Greater onSpawn"
    );
    assert!(
        runtime
            .source_registered_creature_census()
            .map_err(debug)?
            .into_iter()
            .all(|(candidate, _)| !runtime.matches_live_creature_identity(candidate, MIGHTY))
    );
    Ok(())
}

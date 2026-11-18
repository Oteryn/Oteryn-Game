//! Crystal 201189f8, dangerous_depth/creaturescripts_{fiery_heart,snail_slime_kill}.lua.
use super::ComposedFreshAdmission;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, RuntimeScopeRefV1,
};

const COUNT: &str = "oteryn:creature.the_count_of_the_core";
const SLIME: &str = "oteryn:creature.snail_slime";
const SPECTATOR_RANGE: u32 = 20;
const SLIME_LIMIT: usize = 3;

const HEART: &str = "oteryn:creature.fiery_heart";
const BLOOD: &str = "oteryn:creature.fiery_blood";
const EMPOWERED_DUKE: &str = "oteryn:creature.the_fire_empowered_duke";
const DUKE: &str = "oteryn:creature.the_duke_of_the_depths";
const HEART_TIMEOUT_US: u64 = 20_000_000;

#[derive(Debug, Default)]
pub(super) struct HeartTimers(Vec<(ExactActorRef, u64)>);

pub(super) fn tick(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    timers: &mut HeartTimers,
    now_us: u64,
) {
    let Ok(census) = runtime.current_live_creature_census() else {
        return;
    };
    tick_hearts(runtime, timers, &census, now_us);
    let Some(count) = census.iter().find_map(|actor| {
        if !matches!(actor.runtime_scope(), RuntimeScopeRefV1::Instance { .. }) {
            return None;
        }
        let snapshot = runtime.companion_snapshot(*actor).ok()?;
        (snapshot.state.policy.definition_key == COUNT
            && snapshot.health > 0
            && snapshot.health <= snapshot.maximum_health / 2)
            .then_some(snapshot.position)
    }) else {
        return;
    };
    let nearby = |cell: MovementLocalPosition| {
        cell.floor == count.floor
            && cell.x.abs_diff(count.x) <= SPECTATOR_RANGE
            && cell.y.abs_diff(count.y) <= SPECTATOR_RANGE
    };
    let slimes = census.iter().filter(|actor| {
        runtime.companion_snapshot(**actor).is_ok_and(|snapshot| {
            snapshot.state.policy.definition_key == SLIME && nearby(snapshot.position)
        })
    });
    if slimes.take(SLIME_LIMIT).count() == SLIME_LIMIT {
        return;
    }

    // The source requests extended placement starting at the Count's occupied cell.
    for (dx, dy) in [
        (0, 0),
        (0, -1),
        (1, 0),
        (0, 1),
        (-1, 0),
        (1, -1),
        (1, 1),
        (-1, 1),
        (-1, -1),
    ] {
        if let (Some(x), Some(y)) = (count.x.checked_add(dx), count.y.checked_add(dy))
            && super::primal_menace::spawn_creature(
                owner,
                runtime,
                SLIME,
                MovementLocalPosition { x, y, ..count },
            )
            .is_some()
        {
            break;
        }
    }
}

fn tick_hearts(
    runtime: &mut ChannelRuntimeV1,
    timers: &mut HeartTimers,
    census: &[ExactActorRef],
    now_us: u64,
) {
    let hearts: Vec<_> = census
        .iter()
        .copied()
        .filter(|actor| matches!(actor.runtime_scope(), RuntimeScopeRefV1::Instance { .. }))
        .filter_map(|actor| {
            let snapshot = runtime.companion_snapshot(actor).ok()?;
            (snapshot.state.policy.definition_key == HEART).then_some(snapshot)
        })
        .collect();
    timers
        .0
        .retain(|(actor, _)| hearts.iter().any(|heart| heart.actor == *actor));
    for heart in hearts {
        let deadline = match timers.0.iter().find(|(actor, _)| *actor == heart.actor) {
            Some((_, deadline)) => *deadline,
            None => {
                timers
                    .0
                    .push((heart.actor, now_us.saturating_add(HEART_TIMEOUT_US)));
                continue;
            }
        };
        if now_us >= deadline
            && let Ok(change) = runtime.prepare_creature_profile_transformation(&heart, BLOOD)
        {
            let _ = runtime.commit_creature_profile_transformation(change);
        }
    }

    for actor in census.iter().copied() {
        if !matches!(actor.runtime_scope(), RuntimeScopeRefV1::Instance { .. }) {
            continue;
        }
        let Ok(duke) = runtime.companion_snapshot(actor) else {
            continue;
        };
        if duke.state.policy.definition_key != EMPOWERED_DUKE {
            continue;
        }
        let has_heart = census.iter().any(|heart| {
            runtime.companion_snapshot(*heart).is_ok_and(|heart| {
                heart.state.policy.definition_key == HEART
                    && heart.position.floor == duke.position.floor
                    && heart.position.x.abs_diff(duke.position.x) <= SPECTATOR_RANGE
                    && heart.position.y.abs_diff(duke.position.y) <= SPECTATOR_RANGE
            })
        });
        if !has_heart
            && let Ok(change) = runtime.prepare_creature_profile_transformation(&duke, DUKE)
        {
            let _ = runtime.commit_creature_profile_transformation(change);
        }
    }
}

pub(super) fn is_slime(key: &str) -> bool {
    key == SLIME
}

pub(super) fn slime_death(
    runtime: &mut ChannelRuntimeV1,
    slime: ExactActorRef,
    center: MovementLocalPosition,
) {
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };

    if !matches!(slime.runtime_scope(), RuntimeScopeRefV1::Instance { .. }) {
        return;
    }
    let digest = runtime.content_pin().server_artifact_digest();
    let root = GameplayDecisionRoot::from_bytes(digest);
    let occurrence = DecisionOccurrenceId::from_bytes(slime.placement_identity());
    let mut heals = Vec::new();
    // Crystal's AREA_CIRCLE2X2 is a five-by-five square without the corners.
    for (ordinal, (dx, dy)) in (-2_i32..=2)
        .flat_map(|dy| (-2_i32..=2).map(move |dx| (dx, dy)))
        .filter(|(dx, dy)| dx.abs() != 2 || dy.abs() != 2)
        .enumerate()
    {
        let (Some(x), Some(y)) = (center.x.checked_add(dx), center.y.checked_add(dy)) else {
            continue;
        };
        let Ok(Some(actor)) =
            runtime.current_top_actor_on_cell(MovementLocalPosition { x, y, ..center })
        else {
            continue;
        };
        if actor == slime {
            continue;
        }
        let Ok(snapshot) = runtime.companion_snapshot(actor) else {
            continue;
        };
        if snapshot.state.policy.definition_key != COUNT || snapshot.health <= 0 {
            continue;
        }
        let (Ok(maximum), Ok(draw)) = (
            u64::try_from(snapshot.maximum_health),
            deterministic_decision_u64(
                &root,
                occurrence,
                "warzone-slime-death-heal",
                ordinal as u64,
            ),
        ) else {
            continue;
        };
        // Source math.random(0, 2000), inclusive.
        let amount = ((u128::from(draw) * 2_001) >> 64) as u64;
        heals.push((actor, COUNT.to_owned(), maximum, amount));
    }
    if !heals.is_empty() {
        let _ = runtime.commit_source_creature_heal_batch(digest, &heals);
    }
}

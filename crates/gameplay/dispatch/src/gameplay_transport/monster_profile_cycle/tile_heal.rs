//! Crystal 201189f8 data-global/scripts/quests/dark_trails/creaturescripts_think_the_ravager_heal.lua:3-13.
use super::*;
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::foundation::ThinkOccurrence;
use crate::map::boot::BundleCollisionIndexExt;
use crate::map::facts::MovementSpeech;
use oteryn_simulation_determinism::{DecisionOccurrenceId, deterministic_decision_u64};
use sha2::{Digest, Sha256};

pub(super) async fn think(
    runtime: &mut ChannelRuntimeV1,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    occurrences: &[ThinkOccurrence],
    processed: &[crate::foundation::ExactActorRef],
) {
    let rules = native
        .creature_profiles()
        .records
        .iter()
        .filter_map(|record| {
            Some((
                record.profile.target.key.as_str(),
                record.tile_self_heal.as_ref()?,
            ))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    if rules.is_empty() {
        return;
    }
    let root = GameplayDecisionRoot::from_bytes(native.source_digest());
    for occurrence in occurrences {
        let actor = occurrence.actor;
        if !processed.contains(&actor) {
            continue;
        }
        let Ok(snapshot) = runtime.companion_snapshot(actor) else {
            continue;
        };
        let key = snapshot.state.policy.definition_key.as_str();
        let Some(rule) = rules.get(key) else {
            continue;
        };
        let (Ok(health), Ok(maximum)) = (
            u64::try_from(snapshot.health),
            u64::try_from(snapshot.maximum_health),
        ) else {
            continue;
        };
        if health == 0
            || u128::from(health) * 1_000_000
                >= u128::from(maximum) * u128::from(rule.below_health_ppm)
        {
            continue;
        }
        let Ok(position) = runtime.read_actor_position(actor) else {
            continue;
        };
        if map
            .source_has_any_item(runtime, position.position(), &[rule.required_item_id])
            .await
            != Some(true)
        {
            continue;
        }

        let digest = Sha256::new()
            .chain_update(actor.placement_identity())
            .chain_update(occurrence.sequence.to_be_bytes())
            .finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&digest[..16]);
        let Ok(roll) = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(id),
            "creature_tile_self_heal",
            0,
        ) else {
            continue;
        };
        let Ok(draw) = u64::try_from(crate::spell::uniform_draw(
            roll,
            i64::from(rule.minimum),
            i64::from(rule.maximum),
        )) else {
            continue;
        };
        if runtime
            .commit_source_creature_heal_batch(
                native.source_digest(),
                &[(actor, key.to_owned(), maximum, draw)],
            )
            .is_ok()
            && let NativeMovementCollisionIndex::Bundle(index) = map.room().movement_cells().index()
            && let Some(speech) = MovementSpeech::capture_monster(runtime, actor, 36, &rule.speech)
        {
            index.map().facts.emit_movement_speech(speech);
        }
    }
}

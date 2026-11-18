use super::ChannelSpellStates;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition, owner_timer::SemanticTimeMicros};
use crate::player_lethal::PlayerHitKind;
use oteryn_gameplay_encounters::bosses::phase_controller::zelos::MagnorBurst;
use oteryn_protocol_oteryn::analyser::AnalyserDamageSource;
use sha2::{Digest, Sha256};

pub(super) fn apply_magnor_burst(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    burst: MagnorBurst,
    now: u64,
) {
    let Ok(census) = runtime.positioned_melee_census() else {
        return;
    };
    let identity = burst.shard.placement_identity();
    let occurrence = format!("magnor-shard:{:x}", u128::from_le_bytes(identity));
    for (target, position, session) in census {
        let Some(session) = session else {
            continue;
        };
        let position = position.position();
        if position.floor != burst.position.floor
            || position.x.abs_diff(burst.position.x) > 2
            || position.y.abs_diff(burst.position.y) > 2
        {
            continue;
        }
        let draw = Sha256::new()
            .chain_update(b"grave-danger-magnor-shard")
            .chain_update(identity)
            .chain_update(target.placement_identity())
            .finalize();
        let damage = 1800 + u32::from(u16::from_le_bytes([draw[0], draw[1]])) % 401;
        let _ = states.apply_typed_attack_damage_from(
            runtime,
            target,
            session,
            damage,
            PlayerHitKind::LifeDrain,
            &occurrence,
            SemanticTimeMicros::from_micros(now),
            AnalyserDamageSource::None,
        );
    }
}


// Crystal 201189f data/libs/functions/creature.lua:1–34 searches up to two
// rings, breaking ties by distance from the creature and then ring order.
pub(super) fn closest_vortex_landing(
    entered: MovementLocalPosition,
    destination: MovementLocalPosition,
    mut free: impl FnMut(MovementLocalPosition) -> bool,
) -> Option<MovementLocalPosition> {
    for radius in 1..=2 {
        let west = (-radius..radius).rev().map(|dy| (-radius, dy));
        let north = (-radius + 1..=radius).map(|dx| (dx, -radius));
        let east = (-radius + 1..=radius).map(|dy| (radius, dy));
        let south = (-radius..radius).rev().map(|dx| (dx, radius));
        let landing = west
            .chain(north)
            .chain(east)
            .chain(south)
            .filter_map(|(dx, dy)| {
                Some(MovementLocalPosition {
                    x: destination.x.checked_add(dx)?,
                    y: destination.y.checked_add(dy)?,
                    ..destination
                })
            })
            .filter(|&at| free(at))
            .min_by_key(|at| at.x.abs_diff(entered.x).max(at.y.abs_diff(entered.y)));
        if landing.is_some() {
            return landing;
        }
    }
    None
}

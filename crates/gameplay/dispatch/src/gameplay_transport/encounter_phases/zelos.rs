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

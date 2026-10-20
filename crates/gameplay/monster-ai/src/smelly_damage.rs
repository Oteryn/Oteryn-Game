//! Narrow actual HP consumer for source range on unambiguous native top-player tiles.
//! Not a whole Encounter or elemental/presentation owner. Caller qualifies Earth mitigation
//! and wall legality through their actual native owners before enabling this partial route.
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, RuntimeScopeRefV1, RuntimeWorkStamp,
    ScopeRuntimeFence,
};
use crate::player_lethal::{PlayerDamageReceipt, PlayerLethalVitals};
use crate::smelly_cheese::{SmellyDuePulse, SmellyPulse};
#[derive(Debug)]
pub enum SmellyDamageError {
    Owner(CarrierError),
    InvalidRangeDraw,
    WrongPulse,
    StaleOwner,
}
/// Called once for a drained500ms pulse under the same Channel-owner work item.
/// `draw` is the actual server RNG; it draws independently for every qualifying player.
/// `native_legal` must be the existing map/target owner (not authoring metadata).
// Keep commit_smelly_unambiguous_damage ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
#[allow(clippy::too_many_arguments)]
pub fn commit_smelly_unambiguous_damage(
    runtime: &mut ChannelRuntimeV1,
    fence: &ScopeRuntimeFence,
    stamp: RuntimeWorkStamp,
    vitals: &mut impl PlayerLethalVitals,
    pulse: SmellyDuePulse,
    mut draw: impl FnMut() -> u32,
    mut native_legal: impl FnMut(ExactActorRef, ExactActorRef) -> bool,
    now: crate::foundation::owner_timer::SemanticTimeMicros,
) -> Result<Vec<PlayerDamageReceipt>, SmellyDamageError> {
    let binding = runtime.binding();
    if !fence.is_current_for_scope(
        RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        binding.scope_generation(),
    ) || !fence.accepts_stamp(stamp)
    {
        return Err(SmellyDamageError::StaleOwner);
    }
    if pulse.kind() != SmellyPulse::Damage500 {
        return Err(SmellyDamageError::WrongPulse);
    }
    let caster = pulse.caster();
    let cast_sequence = pulse.sequence();
    let targets = runtime
        .smelly_unambiguous_targets(caster)
        .map_err(SmellyDamageError::Owner)?;
    // Freeze all independent draws/legality before any HP mutation. An invalid draw applies
    // no partial cast. Actual health replay is owned by PlayerLethalVitals, not this helper.
    let mut accepted = Vec::new();
    for (target, session) in targets {
        if !native_legal(caster, target) {
            continue;
        }
        let magnitude = draw();
        if !(400..=800).contains(&magnitude) {
            return Err(SmellyDamageError::InvalidRangeDraw);
        }
        accepted.push((target, session, magnitude));
    }
    let mut receipts = Vec::new();
    for (target, session, magnitude) in accepted {
        let occurrence = format!(
            "smelly:{}:{}:{}",
            hex(caster.placement_identity()),
            cast_sequence,
            hex(target.placement_identity())
        );
        if let Some(receipt) =
            vitals.apply_attack_damage(runtime, target, session, magnitude, &occurrence, now)
        {
            receipts.push(receipt);
        }
    }
    Ok(receipts)
}
fn hex(bytes: [u8; 16]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

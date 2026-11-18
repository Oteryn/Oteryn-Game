//! Crystal actions_mission09.lua:26-49. Choose one bounded branch before the
//! existing World receipt begins, then reconcile that exact callback key.
use super::*;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use sha2::{Digest, Sha256};

pub(super) fn select<'a>(
    candidates: &[&'a Callback],
    actor: ExactActorRef,
    command: UseCommand,
    placement: u64,
) -> Option<&'a Callback> {
    let first = candidates.first()?;
    let group = &first.effects.random_use.as_ref()?.group;
    if candidates.len() > world_callbacks::MAX_EFFECTS
        || !candidates
            .iter()
            .all(|candidate| first.random_use_sibling(candidate))
    {
        return None;
    }
    let total: u64 = candidates
        .iter()
        .filter_map(|callback| callback.effects.random_use.as_ref())
        .map(|branch| u64::from(branch.weight))
        .sum();
    let digest = Sha256::new()
        .chain_update(b"oteryn:quest-random-use-v1")
        .chain_update(command.game_session_id.as_bytes())
        .chain_update(actor.placement_identity())
        .chain_update(command.command_id.to_be_bytes())
        .chain_update(placement.to_be_bytes())
        .chain_update(group.as_bytes())
        .finalize();
    let mut draw = u64::from_be_bytes(digest[..8].try_into().ok()?).checked_rem(total)?;
    // Stable regardless of authored document ordering.
    let mut ordered = candidates.to_vec();
    ordered.sort_unstable_by(|left, right| left.key.cmp(&right.key));
    for callback in ordered {
        let weight = u64::from(callback.effects.random_use.as_ref()?.weight);
        if draw < weight {
            return Some(callback);
        }
        draw -= weight;
    }
    None
}

pub(super) fn qualify_spawn<'a>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    callback: &'a Callback,
) -> Result<Option<(&'a str, i64)>, ()> {
    let Some(creature) = callback
        .effects
        .random_use
        .as_ref()
        .and_then(|branch| branch.spawn_at_user.as_deref())
    else {
        return Ok(None);
    };
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or(())?;
    native.install_companion_policies(runtime).map_err(|_| ())?;
    let health = runtime
        .companion_policy(creature)
        .map_err(|_| ())?
        .maximum_health;
    Ok(Some((creature, health)))
}

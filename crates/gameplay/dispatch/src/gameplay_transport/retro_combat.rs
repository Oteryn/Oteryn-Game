//! Combat reactions for the first Lord Retro phase.

#[derive(Clone, Copy)]
pub(super) enum Source {
    Distortion,
    BlissfulTimes,
}

#[derive(Clone, Copy)]
pub(super) enum Target {
    HardTimes,
    ParcelCastle,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Reaction {
    Transform,
    Damage(i64),
    None,
}

pub(super) fn reaction(source: Source, target: Target) -> Reaction {
    match (source, target) {
        (_, Target::HardTimes) => Reaction::Transform,
        (Source::BlissfulTimes, Target::ParcelCastle) => Reaction::Damage(500),
        (Source::Distortion, Target::ParcelCastle) => Reaction::None,
    }
}

use crate::ai_think::profile_schedule::ProfileAbilityProposal;
use crate::foundation::{CarrierError, ChannelRuntimeV1, RuntimeScopeRefV1};
use oteryn_gameplay_entities::OwnerDamageCommand;

const HARD_TIMES: &str = "oteryn:creature.hard_times";
const BLISSFUL_TIMES: &str = "oteryn:creature.blissful_times";
const PARCEL_CASTLE: &str = "oteryn:creature.parcel_castle";

/// Consume an encounter spell in the same owner turn as the retained proposal cursor.
pub(super) fn execute(
    runtime: &mut ChannelRuntimeV1,
    proposal: &ProfileAbilityProposal,
) -> Result<bool, CarrierError> {
    let (source, source_key) = match proposal.ability.key.as_str() {
        "oteryn:ability.spell.retro_time_distortion" => {
            (Source::Distortion, "oteryn:creature.time_distortion")
        }
        "oteryn:ability.spell.retro_blissful_blast" => (Source::BlissfulTimes, BLISSFUL_TIMES),
        _ => return Ok(false),
    };
    if !matches!(
        runtime.binding().runtime_scope(),
        RuntimeScopeRefV1::Instance { .. }
    ) || !runtime.matches_live_creature_identity(proposal.issuer, source_key.as_bytes())
    {
        return Ok(true);
    }
    oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_intent::validate(
        runtime, proposal,
    )
    .map_err(|_| CarrierError::PlanConflict)?;
    if proposal.range_tiles == 0 {
        return Err(CarrierError::PlanConflict);
    }

    let centre = runtime.read_actor_position(proposal.issuer)?.position();
    let range = u32::from(proposal.range_tiles);
    let mut transforms = Vec::new();
    let mut castles = Vec::new();
    let mut damage = 0;
    for actor in runtime.current_live_creature_census()? {
        let identity = runtime.current_live_creature_identity(actor)?;
        let target = if identity == HARD_TIMES.as_bytes() {
            Target::HardTimes
        } else if identity == PARCEL_CASTLE.as_bytes() {
            Target::ParcelCastle
        } else {
            continue;
        };
        let at = runtime.read_actor_position(actor)?.position();
        if at.floor != centre.floor || at.x.abs_diff(centre.x).max(at.y.abs_diff(centre.y)) > range
        {
            continue;
        }
        match reaction(source, target) {
            Reaction::Transform => {
                transforms.push(runtime.prepare_creature_profile_transformation(
                    &runtime.companion_snapshot(actor)?,
                    BLISSFUL_TIMES,
                )?)
            }
            Reaction::Damage(amount) => {
                damage = amount;
                castles.push(actor);
            }
            Reaction::None => continue,
        }
    }
    if !castles.is_empty() {
        let occurrence = proposal.occurrence.id().as_str().as_bytes();
        runtime
            .borrow_exact_actor_commit()
            .commit_raw_creature_damage_batch(
                &castles,
                OwnerDamageCommand {
                    target: PARCEL_CASTLE.as_bytes(),
                    occurrence,
                    binding: occurrence,
                    damage,
                },
                |_| Ok(()),
            )?;
    }
    for transformation in transforms {
        runtime.commit_creature_profile_transformation(transformation)?;
    }
    if matches!(source, Source::BlissfulTimes) {
        runtime.remove_encounter_actor(proposal.issuer)?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distortion_transforms_hard_times_without_damaging_castles() {
        assert_eq!(
            reaction(Source::Distortion, Target::HardTimes),
            Reaction::Transform
        );
        assert_eq!(
            reaction(Source::Distortion, Target::ParcelCastle),
            Reaction::None
        );
    }

    #[test]
    fn blissful_blast_propagates_and_deals_fixed_castle_damage() {
        assert_eq!(
            reaction(Source::BlissfulTimes, Target::HardTimes),
            Reaction::Transform
        );
        assert_eq!(
            reaction(Source::BlissfulTimes, Target::ParcelCastle),
            Reaction::Damage(500)
        );
    }
}

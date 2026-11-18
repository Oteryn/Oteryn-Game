//! Crystal 201189f8 data-global/scripts/quests/grimvale/movements_strangely_glowing_mark.lua:1-28.
use oteryn_gameplay_domain::domain::condition::{ConditionValues, SpeedRange};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind,
};
use oteryn_gameplay_quests::quest::world_callbacks::StepBenefit;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

use super::*;
use crate::spell::cast::PlayerSpellState;

/// Reserve the actor-local successor before the PostgreSQL write. Dispatch holds the same
/// Channel locks through publication, including a retry of the retained STEP receipt.
pub(super) fn prepare(
    runtime: &ChannelRuntimeV1,
    current: &PlayerSpellState,
    callback: &Callback,
    benefit: StepBenefit,
    now: u64,
) -> Option<Option<PlayerSpellState>> {
    match benefit {
        StepBenefit::Heal { amount } => {
            let (next, gained) = current.after_health_gain(u64::from(amount))?;
            Some((gained > 0).then_some(next))
        }
        StepBenefit::Speed {
            duration_ms,
            slope_per_thousand,
            intercept,
        } => {
            let slope = i32::from(slope_per_thousand);
            let intercept = i32::from(intercept);
            let definition = ConditionDefinition::new(
                &callback.key,
                1,
                ConditionValues::Speed {
                    paralysis: false,
                    range: SpeedRange {
                        a_min: slope,
                        a_max: slope,
                        b_min: intercept,
                        b_max: intercept,
                    },
                    duration_ms,
                },
            )?;
            let decision_root =
                GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
            let facts = ApplicationFacts {
                now,
                base_speed: u16::try_from(current.owned_base_speed()).ok()?,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &decision_root,
                // Equal endpoints make this effect independent of the draw.
                occurrence: DecisionOccurrenceId::from_bytes([0; 16]),
            };
            let mut conditions = current.owned_conditions().clone();
            conditions
                .apply(&definition, None, ConditionSourceKind::Field, &[], &facts)
                .ok()?;
            if &conditions == current.owned_conditions() {
                return Some(None);
            }
            let mut next = current.clone();
            next.apply_batch_conditions(current.owned_conditions(), &conditions)
                .ok()?;
            next.advance_batch_revision().ok()?;
            Some(Some(next))
        }
    }
}

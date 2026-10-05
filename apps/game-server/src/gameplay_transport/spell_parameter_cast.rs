//! Candidate parameter transport consumer. The connection owner must call this
//! only after the separately registered additive capability was selected in Hello.
//! The nested v1 targeting/header rules and the common cost owner remain unchanged.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::spell_item_transaction::SpellItemAuthority;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementPositionSnapshot};
use crate::spell::native_house_movement::HouseMovementPlan;
use crate::spell::{Execution, SpellBook};
use oteryn_protocol_oteryn::actor_spell::{ActorSpellError, SpellTarget};
use oteryn_protocol_oteryn::actor_spell_v2::{
    ParameterSpellCastIntent, decode_parameter_spell_cast_intent,
};
use sqlx::{Postgres, Transaction};

#[derive(Debug)]
pub(crate) enum Error {
    Unnegotiated,
    Wire(ActorSpellError),
    InvalidCurrentSpell,
    InvalidTarget,
    FindPerson(crate::spell::find_person_execution::Error),
}
/// Constructed by the connection owner from the actual admitted Hello selection;
/// candidate IDs are supplied by that owner's registry amendment, never payload.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ParameterCapability {
    capability: std::num::NonZeroU32,
    command: std::num::NonZeroU32,
}
impl ParameterCapability {
    pub(crate) fn registered(
        capability: std::num::NonZeroU32,
        command: std::num::NonZeroU32,
    ) -> Option<Self> {
        // Existing accepted command kinds remain untouched. Allocation/registry
        // collision checks against all other kinds belong to the connection owner.
        if command.get() <= 4 || [1, 6, 7].contains(&capability.get()) {
            return None;
        }
        Some(Self {
            capability,
            command,
        })
    }
    pub(crate) fn decode(
        self,
        selected: &[u32],
        command_type: u32,
        payload: &[u8],
    ) -> Result<ParameterSpellCastIntent, Error> {
        if command_type != self.command.get()
            || selected.binary_search(&self.capability.get()).is_err()
        {
            return Err(Error::Unnegotiated);
        }
        decode_parameter_spell_cast_intent(payload).map_err(Error::Wire)
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn plan_find_person_parameter_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    admissions: &FreshAdmissionStore,
    runtime: &ChannelRuntimeV1,
    book: &SpellBook,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    intent: &ParameterSpellCastIntent,
) -> Result<HouseMovementPlan, Error> {
    if intent.intent.target != SpellTarget::None || intent.intent.aim_at_target {
        return Err(Error::InvalidTarget);
    }
    let definition = book
        .indexed(intent.intent.spell)
        .ok_or(Error::InvalidCurrentSpell)?;
    let Execution::NativeProfile(profile) = &definition.execution else {
        return Err(Error::InvalidCurrentSpell);
    };
    crate::spell::find_person_execution::plan_find_person_in_transaction(
        tx,
        authority,
        admissions,
        runtime,
        profile,
        actor,
        expected,
        intent.parameter.as_deref().unwrap_or(""),
    )
    .await
    .map_err(Error::FindPerson)
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn candidate_requires_own_command_and_actual_selected_capability() {
        let token = ParameterCapability::registered(
            std::num::NonZeroU32::new(100).unwrap(),
            std::num::NonZeroU32::new(100).unwrap(),
        )
        .unwrap();
        let bytes = [8, 2, 18, 4, 8, 1, 16, 1];
        assert!(matches!(
            token.decode(&[], 100, &bytes),
            Err(Error::Unnegotiated)
        ));
        assert!(matches!(
            token.decode(&[100], 3, &bytes),
            Err(Error::Unnegotiated)
        ));
        assert!(token.decode(&[100], 100, &bytes).is_ok());
        assert!(
            ParameterCapability::registered(
                std::num::NonZeroU32::new(6).unwrap(),
                std::num::NonZeroU32::new(100).unwrap()
            )
            .is_none()
        );
    }
}

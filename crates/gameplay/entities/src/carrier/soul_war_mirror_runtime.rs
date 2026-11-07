//! Atomic native Mirror births under the existing independently current carrier owner.
use super::runtime_actor_companion::native_birth::PreparedNativeCreatureBirth;
use super::soul_war_health::{HealthSourceContext, HealthSourceKind, apparition_vocation};
use super::*;
use oteryn_gameplay_content::content::native_soul_war_callbacks::SoulWarHealthCallbackKind;
use oteryn_gameplay_domain::domain::soul_war_mirror::{self as rule, DirectPlayerReaction};

#[derive(Debug, Clone)]
pub(super) struct PreparedMirrorReaction {
    pub(super) target: ExactActorRef,
    before: Slot,
    pub(super) birth: Option<PreparedNativeCreatureBirth>,
    pub(super) health: i64,
}
impl ChannelActorCarrier {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_source_mirror_reaction(
        &self,
        continuity: &NamespaceContinuityGuard,
        context: PreProductionPositionContext,
        target: ExactActorRef,
        caster: ExactActorRef,
        session: GameSessionId,
        source: Option<&HealthSourceContext>,
        binding: &[u8],
    ) -> Result<Option<PreparedMirrorReaction>, CarrierError> {
        self.validate_current_continuity(continuity)?;
        let index = self.validate_ref(continuity, target.0)?;
        let Slot::CreatureOccupied {
            generation,
            health,
            companion: Some(companion),
            position: Some(at),
            spell_combat,
            ..
        } = &self.slots[index]
        else {
            return Ok(None);
        };
        if *generation != target.0.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        let Some(callback) = spell_combat
            .soul_war_health
            .as_deref()
            .filter(|p| p.kind() == SoulWarHealthCallbackKind::MirrorImage)
        else {
            return Ok(None);
        };
        let source = source.ok_or(CarrierError::PlanConflict)?;
        if !callback.matches(
            source.content,
            &companion.policy.definition_key,
            &companion.policy.definition_revision,
        ) {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let facts = match source.kind {
            HealthSourceKind::Null | HealthSourceKind::NonPlayer => return Ok(None),
            HealthSourceKind::Player(None) => return Err(CarrierError::PlanConflict),
            HealthSourceKind::Player(Some(facts)) => facts,
        };
        if facts.origin != super::soul_war_player_source::SoulWarPlayerSourceOrigin::Combat
            || *health <= 0
            || at.context != context
        {
            return Err(CarrierError::PlanConflict);
        }
        self.check_remove(continuity, target.0, &[])?;
        // Only the pinned masterless item-created source image has this callback.
        // Its fixed source definition has no summons; do not silently retire unrelated children.
        if self.native_summons.has_children(target) || self.native_summons.child(target).is_some() {
            return Err(CarrierError::PlanConflict);
        }
        let catalogue = spell_combat
            .mirror_births
            .as_deref()
            .ok_or(CarrierError::PlanConflict)?;
        let draw =
            super::soul_war_health::source_rule_draw(binding, target.0, b"mirror-direct", 200)?;
        let reaction = rule::direct_player_reaction(
            apparition_vocation(facts.vocation),
            u16::try_from(draw).map_err(|_| CarrierError::InvalidCommitBinding)?,
        )
        .ok_or(CarrierError::InvalidCommitBinding)?;
        let birth = match reaction {
            DirectPlayerReaction::Disappear => None,
            DirectPlayerReaction::ReplaceAtFullHealth(apparition) => {
                let policy = catalogue.policy(source.content, apparition)?;
                match PreparedNativeCreatureBirth::on_data(
                    self,
                    continuity,
                    context,
                    caster,
                    session,
                    policy,
                    None,
                    None,
                    MovementLocalPosition {
                        x: at.position.x,
                        y: at.position.y,
                        floor: at.position.floor,
                    },
                ) {
                    Ok(birth) => Some(birth),
                    // Source create-before-remove: failure still removes the old image.
                    Err(
                        CarrierError::CapacityExceeded | CarrierError::ActorGenerationExhausted,
                    ) => None,
                    Err(error) => return Err(error),
                }
            }
        };
        Ok(Some(PreparedMirrorReaction {
            target,
            before: self.slots[index].clone(),
            birth,
            health: *health,
        }))
    }

    #[allow(
        clippy::expect_used,
        reason = "all fallible slot/policy/removal/entry checks precede the first physical publication under the same exclusive owner"
    )]
    pub(super) fn apply_source_mirror_reaction(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        mut prepared: PreparedMirrorReaction,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let old = self.validate_ref(continuity, prepared.target.0)?;
        if self.slots.get(old) != Some(&prepared.before) {
            return Err(CarrierError::PlanConflict);
        }
        self.check_remove(continuity, prepared.target.0, &[])?;
        let birth = prepared
            .birth
            .as_mut()
            .map(|birth| {
                Ok::<_, CarrierError>((
                    self.validate_ref(continuity, birth.actor().0)?,
                    self.next_tile_entry_order()?,
                ))
            })
            .transpose()?;
        if let Some(child) = prepared.birth.as_mut() {
            child.reserve_on_data(self, continuity)?;
        }
        // Creation and retirement are one uninterrupted publication; no observer
        // can see a new apparition together with the old source image.
        if let (Some(child), Some((index, order))) = (prepared.birth.as_ref(), birth) {
            child.publish_reserved(self, index, order);
        }
        self.remove(continuity, prepared.target.0)
            .expect("held exact Mirror removal preflight");
        Ok(OwnerDamageResult {
            applied: true,
            health_before: prepared.health,
            health_after: prepared.health,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
pub(in crate::carrier) mod tests {
    include!("soul_war_mirror_runtime/tests.rs");
    mod hatred_health_tests {
        include!("soul_war_mirror_runtime/hatred_health_tests.rs");
    }
}

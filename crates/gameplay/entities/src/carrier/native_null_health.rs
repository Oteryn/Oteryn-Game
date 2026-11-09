//! Source Creature:addHealth(nullptr) synchronization. Samples contain measured
//! native HP or the one explicit Outburst-global fallback; no writer accepts an
//! arbitrary desired HP or fabricates attacker credit.
use super::*;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_content::content::native_monster_callbacks::QualifiedNativeCallback;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rule {
    Guardian,
    GuardianDecrease,
    Outburst,
}

#[derive(Debug, Clone)]
pub struct NativeNullHealthSample {
    rule: Rule,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    activation: u64,
    pins: [[u8; 32]; 4],
    health: i64,
}
impl NativeNullHealthSample {
    pub fn health(&self) -> i64 {
        self.health
    }
}
fn rule(source: &QualifiedNativeCallback) -> Result<Rule, CarrierError> {
    let entry = source.entry();
    if entry.ability.revision != "definition-r1" || entry.creature.revision != "definition-r1" {
        return Err(CarrierError::CreatureTargetMismatch);
    }
    match (entry.creature.key.as_str(), entry.ability.key.as_str()) {
        ("oteryn:creature.the_time_guardian", "oteryn:ability.spell.time_guardian") => {
            Ok(Rule::Guardian)
        }
        ("oteryn:creature.the_time_guardian", "oteryn:ability.spell.time_guardiann") => {
            Ok(Rule::GuardianDecrease)
        }
        ("oteryn:creature.charging_outburst", "oteryn:ability.spell.outburst_explode") => {
            Ok(Rule::Outburst)
        }
        _ => Err(CarrierError::CreatureTargetMismatch),
    }
}
impl ChannelRuntimeV1 {
    fn null_health_sample(
        &self,
        native: &NativeGameplayState,
        source: &QualifiedNativeCallback,
        health: i64,
    ) -> Result<NativeNullHealthSample, CarrierError> {
        self.owner_fence()?;
        if native.source_digest() != self.content_pin().server_artifact_digest()
            || source.content() != native.source_digest()
            || health < 0
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        Ok(NativeNullHealthSample {
            rule: rule(source)?,
            scope: self.binding().runtime_scope(),
            generation: self.binding().scope_generation(),
            activation: self.content_pin().activation_sequence(),
            pins: [
                native.source_digest(),
                self.content_pin().client_artifact_digest(),
                self.content_pin().frame_binding_digest(),
                self.content_pin().map_revision_digest(),
            ],
            health,
        })
    }
    /// Capture at the actual source assignment: a current peer in TimeGuardian,
    /// or current Outburst when its script stores the shared global HP.
    pub fn capture_native_null_health_sample(
        &self,
        native: &NativeGameplayState,
        source: &QualifiedNativeCallback,
        actor: ExactActorRef,
    ) -> Result<NativeNullHealthSample, CarrierError> {
        let snapshot = self.companion_snapshot(actor)?;
        let key = snapshot.state.policy.definition_key.as_str();
        let allowed = match rule(source)? {
            Rule::Outburst => key == "oteryn:creature.outburst",
            Rule::Guardian | Rule::GuardianDecrease => matches!(
                key,
                "oteryn:creature.the_time_guardian"
                    | "oteryn:creature.the_blazing_time_guardian"
                    | "oteryn:creature.the_freezing_time_guardian"
            ),
        };
        let actual = native
            .policies()
            .policies()
            .iter()
            .any(|policy| policy.as_ref() == snapshot.state.policy.as_ref());
        if !allowed || !actual || snapshot.health <= 0 {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        self.null_health_sample(native, source, snapshot.health)
    }
    /// outburst_explode.lua:60 explicitly maps a missing/nonpositive global to0.
    /// No arbitrary numeric literal constructor exists.
    pub fn native_null_health_default_sample(
        &self,
        native: &NativeGameplayState,
        source: &QualifiedNativeCallback,
    ) -> Result<NativeNullHealthSample, CarrierError> {
        if rule(source)? != Rule::Outburst {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        self.null_health_sample(native, source, 0)
    }
    /// actions_outburst.lua:23-25 assigns its literal initial global before
    /// creating the boss. This is source DATA, not a health mutation or spawn.
    pub fn native_null_health_entry_sample(
        &self,
        native: &NativeGameplayState,
        source: &QualifiedNativeCallback,
    ) -> Result<NativeNullHealthSample, CarrierError> {
        if rule(source)? != Rule::Outburst
            || !native.policies().policies().iter().any(|policy| {
                policy.definition_key == "oteryn:creature.outburst"
                    && policy.maximum_health == 290_000
            })
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        self.null_health_sample(native, source, 290_000)
    }
    /// Revalidate the entire current target snapshot and immutable sample pin.
    /// Negative deltas use the canonical native lethal/receipt owner with NO
    /// attacker; positive deltas use the existing canonical creature heal owner.
    pub fn commit_native_null_health_sync(
        &mut self,
        native: &NativeGameplayState,
        source: &QualifiedNativeCallback,
        sample: &NativeNullHealthSample,
        target: &runtime_actor_companion::CompanionSnapshot,
        occurrence: &[u8],
        binding: &[u8],
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.commit_native_null_health_sync_inner(
            native, source, sample, target, occurrence, binding, None,
        )
    }
    /// Production source callbacks retain their real owner microsecond clock
    /// at HP publication, independently of later deferred-task execution.
    #[allow(
        clippy::too_many_arguments,
        reason = "source proof, health sample, target snapshot, occurrence binding and owner clock require independent validation"
    )]
    pub fn commit_native_null_health_sync_at(
        &mut self,
        native: &NativeGameplayState,
        source: &QualifiedNativeCallback,
        sample: &NativeNullHealthSample,
        target: &runtime_actor_companion::CompanionSnapshot,
        occurrence: &[u8],
        binding: &[u8],
        now_us: u64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.commit_native_null_health_sync_inner(
            native,
            source,
            sample,
            target,
            occurrence,
            binding,
            Some(now_us),
        )
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "source proof, health sample, target snapshot, occurrence binding and owner clock require independent validation"
    )]
    fn commit_native_null_health_sync_inner(
        &mut self,
        native: &NativeGameplayState,
        source: &QualifiedNativeCallback,
        sample: &NativeNullHealthSample,
        target: &runtime_actor_companion::CompanionSnapshot,
        occurrence: &[u8],
        binding: &[u8],
        now_us: Option<u64>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let current = self.null_health_sample(native, source, sample.health)?;
        if current.rule != sample.rule
            || current.scope != sample.scope
            || current.generation != sample.generation
            || current.activation != sample.activation
            || current.pins != sample.pins
            || self.companion_snapshot(target.actor)? != *target
            || target.health <= 0
            || target.maximum_health <= 0
        {
            return Err(CarrierError::PlanConflict);
        }
        let key = target.state.policy.definition_key.as_str();
        let target_allowed = match sample.rule {
            Rule::Outburst => key == "oteryn:creature.outburst",
            Rule::Guardian | Rule::GuardianDecrease => matches!(
                key,
                "oteryn:creature.the_time_guardian"
                    | "oteryn:creature.the_blazing_time_guardian"
                    | "oteryn:creature.the_freezing_time_guardian"
            ),
        };
        if !target_allowed
            || !native
                .policies()
                .policies()
                .iter()
                .any(|policy| policy.as_ref() == target.state.policy.as_ref())
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let desired = sample.health.min(target.maximum_health);
        let desired = if sample.rule == Rule::GuardianDecrease {
            desired.min(target.health)
        } else {
            desired
        };
        if desired == target.health {
            return Ok(OwnerDamageResult {
                applied: false,
                health_before: target.health,
                health_after: target.health,
            });
        }
        if desired > target.health {
            return self
                .commit_source_creature_heal_batch(
                    native.source_digest(),
                    &[(
                        target.actor,
                        key.to_owned(),
                        target.maximum_health as u64,
                        (desired - target.health) as u64,
                    )],
                )?
                .into_iter()
                .next()
                .ok_or(CarrierError::PlanConflict);
        }
        let mut owner = match now_us {
            Some(now) => self.borrow_exact_actor_commit_at_us(now),
            None => self.borrow_exact_actor_commit(),
        };
        owner.commit_raw_creature_damage_batch(
            &[target.actor],
            OwnerDamageCommand {
                target: key.as_bytes(),
                occurrence,
                binding,
                damage: target.health - desired,
            },
            |results| results.first().copied().ok_or(CarrierError::PlanConflict),
        )
    }
}

#[cfg(test)]
mod tests;

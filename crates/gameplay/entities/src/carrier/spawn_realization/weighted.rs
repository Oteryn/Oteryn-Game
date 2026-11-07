//! One content point chooses an ordinary identity per lifetime; its home and cooldown stay fixed.
use super::*;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivationSpawnAlternative {
    pub creature: String,
    pub weight: u32,
    pub maximum_health: u64,
    pub blockable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivationWeightedSpawnPoint {
    pub source: u16,
    pub point: u16,
    pub choices: Vec<ActivationSpawnAlternative>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WeightedDefinition {
    root: [u8; 32],
    choices: Vec<ActivationSpawnAlternative>,
    total: u32,
}

impl WeightedDefinition {
    fn new(
        root: [u8; 32],
        choices: Vec<ActivationSpawnAlternative>,
        timing: &str,
    ) -> Result<Self, CarrierError> {
        if !(2..=CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX).contains(&choices.len())
            || !choices.windows(2).all(|p| p[0].creature < p[1].creature)
            || !choices.iter().any(|choice| choice.creature == timing)
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let mut total = 0u32;
        for choice in &choices {
            if choice.weight == 0 || activation_health(choice.maximum_health)? <= 0 {
                return Err(CarrierError::InvalidSpawnDefinition);
            }
            creature_target_binding(&choice.creature)?;
            total = total
                .checked_add(choice.weight)
                .ok_or(CarrierError::InvalidSpawnDefinition)?;
        }
        Ok(Self {
            root,
            choices,
            total,
        })
    }

    fn bucket(&self, mut roll: u64) -> Result<&ActivationSpawnAlternative, CarrierError> {
        for choice in &self.choices {
            if roll < u64::from(choice.weight) {
                return Ok(choice);
            }
            roll -= u64::from(choice.weight);
        }
        Err(CarrierError::InvalidSpawnDefinition)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the decision binds complete scope, point and lifetime provenance"
    )]
    fn choose(
        &self,
        scope: RuntimeScopeRefV1,
        generation: u64,
        source: SpawnSourceId,
        ordinal: usize,
        previous: Option<ExactActorRef>,
        cycle: u32,
    ) -> Result<&ActivationSpawnAlternative, CarrierError> {
        let (tag, local) = match scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => (0u8, *channel_id.as_bytes()),
            RuntimeScopeRefV1::Instance { instance_id, .. } => (1u8, instance_id),
        };
        let mut hash = Sha256::new()
            .chain_update(b"oteryn:world.spawn.weighted-point.occurrence:v1\0")
            .chain_update(scope.world_id().as_bytes())
            .chain_update([tag])
            .chain_update(local)
            .chain_update(generation.to_be_bytes())
            .chain_update(source.0.to_be_bytes())
            .chain_update(
                u16::try_from(ordinal)
                    .map_err(|_| CarrierError::InvalidSpawnDefinition)?
                    .to_be_bytes(),
            )
            .chain_update(cycle.to_be_bytes());
        if let Some(actor) = previous {
            hash.update([1]);
            hash.update(actor.placement_identity());
        } else {
            hash.update([0]);
        }
        let digest = hash.finalize();
        let mut occurrence = [0u8; 16];
        occurrence.copy_from_slice(&digest[..16]);
        let root = GameplayDecisionRoot::from_bytes(self.root);
        let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
        let total = u64::from(self.total);
        let threshold = total.wrapping_neg() % total;
        for draw in 0..64 {
            let value = deterministic_decision_u64(
                &root,
                occurrence,
                "world.spawn.weighted-point.v1",
                draw,
            )
            .map_err(|_| CarrierError::InvalidSpawnDefinition)?;
            if value < threshold {
                continue;
            }
            return self.bucket(value % total);
        }
        Err(CarrierError::InvalidSpawnDefinition)
    }
}

impl SpawnPointDefinition {
    #[allow(
        clippy::too_many_arguments,
        reason = "one point's immutable facts and exact current lifetime select its next identity"
    )]
    pub(super) fn selected(
        &self,
        scope: RuntimeScopeRefV1,
        generation: u64,
        source: SpawnSourceId,
        ordinal: usize,
        previous: Option<ExactActorRef>,
        cycle: u32,
    ) -> Result<(&str, i64, bool), CarrierError> {
        if let Some(weighted) = &self.weighted {
            let choice = weighted.choose(scope, generation, source, ordinal, previous, cycle)?;
            Ok((
                &choice.creature,
                activation_health(choice.maximum_health)?,
                choice.blockable,
            ))
        } else {
            Ok((
                &self.creature_target_identity,
                self.creature_initial_health,
                self.blockable,
            ))
        }
    }
}

impl ChannelRuntimeV1 {
    /// The map owner passes one source plan. A weighted slot never adds a placement,
    /// a timer or a live actor beyond the existing point's declared population.
    pub fn begin_activation_spawn_plan(
        &mut self,
        sources: Vec<ActivationSpawnSourceFacts>,
        weighted: Vec<ActivationWeightedSpawnPoint>,
    ) -> Result<usize, CarrierError> {
        self.owner_fence()?;
        let mut definitions = sources
            .into_iter()
            .map(SpawnDefinition::from_activation_source)
            .collect::<Result<Vec<_>, _>>()?;
        let points = spawn_points_within_ceilings(&definitions)?;
        if weighted.len() > points {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let mut choices = 0usize;
        for selection in weighted {
            choices = choices
                .checked_add(selection.choices.len())
                .ok_or(CarrierError::CapacityArithmeticOverflow)?;
            if choices > CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX {
                return Err(CarrierError::CapacityExceeded);
            }
            let point = definitions
                .get_mut(usize::from(selection.source))
                .and_then(|source| source.points.get_mut(usize::from(selection.point)))
                .ok_or(CarrierError::InvalidSpawnDefinition)?;
            if point.weighted.is_some() {
                return Err(CarrierError::InvalidSpawnDefinition);
            }
            point.weighted = Some(WeightedDefinition::new(
                self.content_pin().server_artifact_digest(),
                selection.choices,
                &point.creature_target_identity,
            )?);
        }
        self.begin_spawn_definitions(definitions)
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "exact integer source-weight boundary assertions"
)]
mod tests {
    use super::*;
    #[test]
    fn srv7_weighted_spawn_source25_25_uses_exact24_25_boundary() {
        let choices = vec![
            ActivationSpawnAlternative {
                creature: "oteryn:creature.silverfrost_sentinel".into(),
                weight: 25,
                maximum_health: 100,
                blockable: true,
            },
            ActivationSpawnAlternative {
                creature: "oteryn:creature.true_frost_flower_asura".into(),
                weight: 25,
                maximum_health: 200,
                blockable: true,
            },
        ];
        let definition =
            WeightedDefinition::new([1; 32], choices, "oteryn:creature.true_frost_flower_asura")
                .expect("source weighted point");
        for roll in [0, 24] {
            assert!(
                definition
                    .bucket(roll)
                    .expect("first source range")
                    .creature
                    .ends_with("sentinel")
            );
        }
        for roll in [25, 49] {
            assert!(
                definition
                    .bucket(roll)
                    .expect("second source range")
                    .creature
                    .ends_with("asura")
            );
        }
        assert!(definition.bucket(50).is_err());
    }
}

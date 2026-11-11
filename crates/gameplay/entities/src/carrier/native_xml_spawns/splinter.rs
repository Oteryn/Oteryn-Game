//! The actual XML birth owns Lesser's one 120s callback. setType(true) does
//! not run Greater's onSpawn or replace a previously queued physical death.
use super::*;
use crate::carrier::native_source_death::{SourceCreatureDeathNotice, SourceCreatureDeathSeal};
use oteryn_gameplay_content::content::bosses::activity_template::PinnedActivitySourceRoom;

const LESSER: &str = "oteryn:creature.lesser_splinter_of_madness";
const GROWTH_US: u64 = 120_000_000;

/// DATA copied only from a real canonical lethal receipt at the closed XML
/// restoration. It never creates another receipt or makes a live death legal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::carrier) struct RestoredSplinterDeath {
    notice: SourceCreatureDeathNotice,
    projection: RuntimeCorpseProjection,
    world: WorldId,
    activation: u64,
    pins: [[u8; 32]; 4],
}
impl RestoredSplinterDeath {
    pub(in crate::carrier) fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        runtime.owner_fence().is_ok()
            && self.notice.actor().runtime_scope() == runtime.binding().runtime_scope()
            && self.notice.actor().scope_generation() == runtime.binding().scope_generation()
            && self.world == pin.world_id()
            && self.activation == pin.activation_sequence()
            && self.pins
                == [
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    pin.frame_binding_digest(),
                    pin.map_revision_digest(),
                ]
    }
    pub(in crate::carrier) fn matches_notice(&self, notice: &SourceCreatureDeathNotice) -> bool {
        self.notice == *notice
    }
    pub(in crate::carrier) fn matches_begin(&self, begin: &SourceCreatureDeathSeal) -> bool {
        self.notice.actor() == begin.actor() && self.notice.death_key() == begin.death_key()
    }
    pub(in crate::carrier) fn projection(
        &self,
        actor: ExactActorRef,
    ) -> Option<&RuntimeCorpseProjection> {
        (self.notice.actor() == actor && self.projection.occurrence.actor == actor)
            .then_some(&self.projection)
    }
}

impl ChannelRuntimeV1 {
    pub(in crate::carrier) fn stamp_soul_war_xml_birth(
        &mut self,
        point: spawn::SpawnPoint,
        actor: ExactActorRef,
        now_us: u64,
    ) -> Result<(), CarrierError> {
        if !self.current_xml_spawn(point)? {
            return Ok(());
        }
        let source = self
            .carrier
            .spawns
            .get(usize::from(point.source))
            .ok_or(CarrierError::InvalidSpawnDefinition)?;
        if source
            .definition
            .xml_source
            .as_ref()
            .map(QualifiedAmbientXmlSpawns::role)
            != Some(PinnedActivitySourceRoom::GoshnarsMegalomania)
        {
            return Ok(());
        }
        if source
            .cells
            .get(usize::from(point.ordinal))
            .and_then(|cell| cell.live)
            != Some(actor)
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            companion: Some(state),
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::InvalidSpawnDefinition);
        };
        if state.policy.definition_key == LESSER {
            if state.created_at_us.is_some() || state.master.is_some() {
                return Err(CarrierError::PlanConflict);
            }
            // A delayed owner pass uses its actual admission time, never the
            // activation, warning, scheduled due or first Think timestamp.
            state.created_at_us = Some(now_us);
        }
        Ok(())
    }

    /// The one source callback is derived from the real XML incarnation. None
    /// means another type/finished task; an unqualified Lesser has no fake epoch.
    pub fn soul_war_splinter_due_us(
        &self,
        actor: ExactActorRef,
    ) -> Result<Option<u64>, CarrierError> {
        self.owner_fence()?;
        let current = self.companion_snapshot_including_dead(actor)?;
        if current.state.policy.definition_key != LESSER
            || !self.source_creature_is_registered(actor)?
        {
            return Ok(None);
        }
        if current.state.master.is_some()
            || !matches!(
                self.binding().runtime_scope(),
                RuntimeScopeRefV1::Instance { .. }
            )
        {
            return Err(CarrierError::WrongScope);
        }
        let (source, ordinal) = self
            .carrier
            .locate_spawn_cell(actor)
            .ok_or(CarrierError::InvalidSpawnDefinition)?;
        let point = spawn::SpawnPoint {
            source: source.0,
            ordinal: u8::try_from(ordinal).map_err(|_| CarrierError::InvalidSpawnDefinition)?,
        };
        if !self.current_xml_spawn(point)?
            || self
                .carrier
                .spawns
                .get(usize::from(source.0))
                .and_then(|source| source.definition.xml_source.as_ref())
                .map(QualifiedAmbientXmlSpawns::role)
                != Some(PinnedActivitySourceRoom::GoshnarsMegalomania)
            || self.companion_policy(LESSER)?.as_ref() != current.state.policy.as_ref()
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        current
            .state
            .created_at_us
            .ok_or(CarrierError::InvalidSpawnDefinition)?
            .checked_add(GROWTH_US)
            .map(Some)
            .ok_or(CarrierError::CapacityArithmeticOverflow)
    }

    pub(in crate::carrier) fn prepare_restored_splinter_death(
        &self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        due_us: u64,
    ) -> Result<Option<Arc<RestoredSplinterDeath>>, CarrierError> {
        if self.soul_war_splinter_due_us(expected.actor)? != Some(due_us) {
            return Err(CarrierError::PlanConflict);
        }
        if expected.health > 0 {
            return Ok(None);
        }
        let index = self
            .carrier
            .validate_ref(&self.continuity, expected.actor.0)?;
        let Slot::CreatureOccupied { spell_combat, .. } = &self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        if spell_combat.source_death_begin.is_some()
            || spell_combat.source_death_completed
            || spell_combat.restored_splinter_death.is_some()
        {
            return Err(CarrierError::PlanConflict);
        }
        let notice = self
            .carrier
            .native_deaths
            .iter()
            .find(|notice| notice.actor() == expected.actor)
            .filter(|notice| {
                notice
                    .enqueued_at_us()
                    .is_some_and(|queued| due_us <= queued)
            })
            .ok_or(CarrierError::CommittedLethalUnavailable)?;
        let original = self
            .carrier
            .committed_lethal_receipt_inner(&self.continuity, expected.actor.0)?;
        if original.statistics_death_key()? != notice.death_key() {
            return Err(CarrierError::CorpseReceiptMismatch);
        }
        let pin = self.content_pin();
        Ok(Some(Arc::new(RestoredSplinterDeath {
            notice: notice.clone(),
            projection: original.projection,
            world: pin.world_id(),
            activation: pin.activation_sequence(),
            pins: [
                pin.server_artifact_digest(),
                pin.client_artifact_digest(),
                pin.frame_binding_digest(),
                pin.map_revision_digest(),
            ],
        })))
    }
}

//! A source-map STEP remains an actual movement-owner operation. A private
//! qualified movement proof is distinct from a spell's relocation authority.
use super::*;

pub mod source_step_seal {
    pub trait Sealed {}
}
pub trait SourceStepCommitProof: source_step_seal::Sealed {
    fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError>;
    fn parts(
        &self,
    ) -> (
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
        MovementFacing,
    );
}
impl ChannelRuntimeV1 {
    pub fn commit_source_step<P: SourceStepCommitProof>(
        &mut self,
        proof: P,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        proof.validate_current(self)?;
        let (actor, expected, next, facing) = proof.parts();
        if actor.0 != expected.0.actor_ref {
            return Err(
                oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionSnapshotMismatch,
            );
        }
        // Obtain the exact slot before committing. No fallible lookup follows
        // the physical mutation; the uninterrupted owner turn owns this slot.
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let mut committed = self.carrier.compare_commit_position(
            &self.continuity,
            expected.0,
            expected.0.version.context,
            LocalPosition {
                x: next.x,
                y: next.y,
                floor: next.floor,
            },
        )?;
        // Source floor changes preserve the actual input direction rather than
        // deriving a new facing from the final stair/hole coordinate offset.
        committed.version.facing = Some(facing);
        match &mut self.carrier.slots[index] {
            Slot::Occupied { position, .. } | Slot::CreatureOccupied { position, .. } => {
                *position = Some(committed.version);
            }
            _ => unreachable!("position commit proved occupied exact slot"),
        }
        Ok(MovementPositionSnapshot(committed))
    }
}

/// One exact actor-position successor in a bounded source scene.
#[derive(Clone, Copy)]
pub struct SourceSceneMove {
    pub actor: ExactActorRef,
    pub expected: MovementPositionSnapshot,
    pub destination: MovementLocalPosition,
}

/// World owns eligibility, landing and the declared scene cells. The carrier
/// independently rechecks exact identity, current position and movement authority.
pub trait SourceSceneRelocationProof: source_step_seal::Sealed {
    fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError>;
    fn moves(&self) -> &[SourceSceneMove];
}

impl ChannelRuntimeV1 {
    /// Commit at most32 source moves and one all-or-nothing scene projection.
    /// Existing movement and summon lifecycle checks remain authoritative. The
    /// private rollback snapshot is discarded before this owner turn is released.
    /// A rejected actor or projection leaves every carrier-owned store unchanged.
    pub fn commit_source_scene_relocations<P, F>(
        &mut self,
        proof: P,
        project: F,
    ) -> Result<Vec<MovementPositionSnapshot>, CarrierError>
    where
        P: SourceSceneRelocationProof,
        F: FnOnce() -> bool,
    {
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.owner_fence()?;
        proof.validate_current(self)?;
        let moves = proof.moves();
        if moves.len() > 32
            || moves.iter().enumerate().any(|(index, change)| {
                moves[..index]
                    .iter()
                    .any(|before| before.actor == change.actor)
            })
        {
            return Err(CarrierError::PlanConflict);
        }
        // Same stores as compare_commit_position's summon-removal rollback.
        // No second position authority is published or retained.
        let before = (
            self.carrier.slots.clone(),
            self.carrier.free_head,
            self.carrier.occupied.clone(),
            self.carrier.native_summons.clone(),
            self.carrier.native_summon_deaths,
            self.carrier.native_summon_death_cursor,
            self.carrier.attackers.clone(),
        );
        let outcome = (|| {
            let mut result = Vec::with_capacity(moves.len());
            for change in moves {
                if change.actor.0 != change.expected.0.actor_ref {
                    return Err(CarrierError::PositionSnapshotMismatch);
                }
                let index = self
                    .carrier
                    .validate_ref(&self.continuity, change.actor.0)?;
                if self
                    .carrier
                    .read_movement_position(&self.continuity, change.actor.0)?
                    != change.expected
                {
                    return Err(CarrierError::PositionSnapshotMismatch);
                }
                if let Slot::NpcOccupied {
                    position,
                    hireling: None,
                    ..
                } = &mut self.carrier.slots[index]
                {
                    let revision = position
                        .revision
                        .checked_add(1)
                        .ok_or(CarrierError::PositionRevisionExhausted)?;
                    *position = VersionedPosition {
                        position: LocalPosition {
                            x: change.destination.x,
                            y: change.destination.y,
                            floor: change.destination.floor,
                        },
                        revision,
                        last_translation_revision: revision,
                        ..*position
                    };
                    result.push(MovementPositionSnapshot(PositionSnapshot {
                        actor_ref: change.actor.0,
                        version: *position,
                    }));
                    continue;
                }
                let mut committed = self.carrier.compare_commit_position(
                    &self.continuity,
                    change.expected.0,
                    change.expected.0.version.context,
                    LocalPosition {
                        x: change.destination.x,
                        y: change.destination.y,
                        floor: change.destination.floor,
                    },
                )?;
                // Forced source transport preserves the actual facing.
                committed.version.facing = change.expected.facing();
                match &mut self.carrier.slots[index] {
                    Slot::Occupied { position, .. } | Slot::CreatureOccupied { position, .. } => {
                        *position = Some(committed.version);
                    }
                    _ => return Err(CarrierError::PositionSnapshotMismatch),
                }
                result.push(MovementPositionSnapshot(committed));
            }
            if !project() {
                return Err(CarrierError::PlanConflict);
            }
            Ok(result)
        })();
        if outcome.is_err() {
            let (slots, free, occupied, summons, deaths, cursor, attackers) = before;
            self.carrier.slots = slots;
            self.carrier.free_head = free;
            self.carrier.occupied = occupied;
            self.carrier.native_summons = summons;
            self.carrier.native_summon_deaths = deaths;
            self.carrier.native_summon_death_cursor = cursor;
            self.carrier.attackers = attackers;
        }
        outcome
    }
}

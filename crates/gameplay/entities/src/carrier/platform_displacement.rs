//! M50: forced platform evacuation includes creatures and every stacked player.
use super::*;

/// Exact before/after facts for the dispatch owner's movement consequences.
#[derive(Clone, Copy, Debug)]
pub struct PlatformDisplacement {
    pub actor: ExactActorRef,
    pub session: Option<GameSessionId>,
    pub before: MovementPositionSnapshot,
    pub after: MovementPositionSnapshot,
}

impl ChannelRuntimeV1 {
    /// The caller holds the Channel owner and proves each destination against its
    /// current map. The complete native census, rather than a top-creature lookup,
    /// defines the occupants. `project` must publish the map batch atomically or
    /// return false without changing it. A refusal restores all actor positions.
    pub fn commit_quest_platform_displacement(
        &mut self,
        from: MovementLocalPosition,
        to: MovementLocalPosition,
        mut admits: impl FnMut(ExactActorRef, Option<GameSessionId>, MovementLocalPosition) -> bool,
        project: impl FnOnce() -> bool,
    ) -> Result<Vec<PlatformDisplacement>, CarrierError> {
        self.owner_fence()?;
        if from == to {
            return Err(CarrierError::PlanConflict);
        }
        let mut occupants = self.positioned_melee_census()?;
        occupants.retain(|(_, position, _)| position.position() == from);
        if occupants
            .iter()
            .any(|(actor, _, session)| !admits(*actor, *session, to))
        {
            return Err(CarrierError::PlanConflict);
        }
        let mut moved = Vec::new();
        moved
            .try_reserve_exact(occupants.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        let before = self.carrier.clone();
        let result = (|| {
            for (actor, expected, session) in occupants {
                let after = self.carrier.compare_commit_position(
                    &self.continuity,
                    expected.0,
                    expected.0.version.context,
                    LocalPosition {
                        x: to.x,
                        y: to.y,
                        floor: to.floor,
                    },
                )?;
                moved.push(PlatformDisplacement {
                    actor,
                    session,
                    before: expected,
                    after: MovementPositionSnapshot(after),
                });
            }
            if !project() {
                return Err(CarrierError::PlanConflict);
            }
            Ok(moved)
        })();
        if result.is_err() {
            self.carrier = before;
        }
        result
    }
}
//! M50: forced platform evacuation includes creatures and every stacked player.
use super::*;

/// Exact before/after facts for the dispatch owner's movement consequences.
#[derive(Clone, Copy, Debug)]
pub struct PlatformDisplacement {
    pub actor: ExactActorRef,
    pub session: Option<GameSessionId>,
    pub before: MovementPositionSnapshot,
    pub after: MovementPositionSnapshot,
}

impl ChannelRuntimeV1 {
    /// Carry only the source rider, with the sealed native teleport proof. Other
    /// occupants stay put, as in the pinned source's on-platform use branch.
    pub fn commit_quest_platform_rider<P: crate::SourceStepCommitProof>(
        &mut self,
        proof: P,
        project: impl FnOnce() -> bool,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        proof.validate_current(self)?;
        let before = self.carrier.clone();
        let result = (|| {
            let landed = self.commit_source_step(proof)?;
            if !project() {
                return Err(CarrierError::PlanConflict);
            }
            Ok(landed)
        })();
        if result.is_err() {
            self.carrier = before;
        }
        result
    }

    /// The caller holds the Channel owner and proves each destination against its
    /// current map. The complete native census, rather than a top-creature lookup,
    /// defines the occupants. `project` must publish the map batch atomically or
    /// return false without changing it. A refusal restores all actor positions.
    pub fn commit_quest_platform_displacement(
        &mut self,
        from: MovementLocalPosition,
        to: MovementLocalPosition,
        mut admits: impl FnMut(ExactActorRef, Option<GameSessionId>, MovementLocalPosition) -> bool,
        project: impl FnOnce() -> bool,
    ) -> Result<Vec<PlatformDisplacement>, CarrierError> {
        self.owner_fence()?;
        if from == to {
            return Err(CarrierError::PlanConflict);
        }
        let mut occupants = self.positioned_melee_census()?;
        occupants.retain(|(_, position, _)| position.position() == from);
        if occupants
            .iter()
            .any(|(actor, _, session)| !admits(*actor, *session, to))
        {
            return Err(CarrierError::PlanConflict);
        }
        let mut moved = Vec::new();
        moved
            .try_reserve_exact(occupants.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        let before = self.carrier.clone();
        let result = (|| {
            for (actor, expected, session) in occupants {
                let after = self.carrier.compare_commit_position(
                    &self.continuity,
                    expected.0,
                    expected.0.version.context,
                    LocalPosition {
                        x: to.x,
                        y: to.y,
                        floor: to.floor,
                    },
                )?;
                moved.push(PlatformDisplacement {
                    actor,
                    session,
                    before: expected,
                    after: MovementPositionSnapshot(after),
                });
            }
            if !project() {
                return Err(CarrierError::PlanConflict);
            }
            Ok(moved)
        })();
        if result.is_err() {
            self.carrier = before;
        }
        result
    }
}

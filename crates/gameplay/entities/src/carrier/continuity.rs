use super::*;

impl PreProductionContinuityGrant {
    /// Test builds only: a grant for a qualification fixture. No production grant API.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn fixture(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
    ) -> Self {
        Self {
            world_id,
            channel_id,
            scope_generation,
        }
    }

    /// Test builds only: the World this grant names.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn world_id(&self) -> WorldId {
        self.world_id
    }

    /// Test builds only: the Channel this grant names.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    /// Test builds only: the scope ownership generation this grant names.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn scope_generation(&self) -> ScopeOwnershipGeneration {
        self.scope_generation
    }
}

impl NamespaceContinuityGuard {
    /// Test builds only: the World of this guard's namespace.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn world_id(&self) -> WorldId {
        self.world_id
    }

    /// Test builds only: the Channel of this guard's namespace.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    /// Test builds only: the current scope ownership generation.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn current_generation(&self) -> ScopeOwnershipGeneration {
        self.current_generation
    }

    test_support_pub! {
        fn from_pre_production_grant(grant: PreProductionContinuityGrant) -> Self {
            Self {
                world_id: grant.world_id,
                channel_id: grant.channel_id,
                current_generation: grant.scope_generation,
                current_generation_claimed: false,
                active: true,
            }
        }
    }

    test_support_pub! {
        fn advance(&mut self, grant: PreProductionContinuityGrant) -> Result<(), CarrierError> {
            if grant.world_id != self.world_id || grant.channel_id != self.channel_id {
                return Err(CarrierError::WrongScope);
            }
            if grant.scope_generation.get() <= self.current_generation.get() {
                return Err(CarrierError::ContinuityGenerationNotNewer);
            }

            self.current_generation = grant.scope_generation;
            self.current_generation_claimed = false;
            self.active = true;
            Ok(())
        }
    }

    pub(super) fn claim_current_generation(&mut self) -> Result<(), CarrierError> {
        self.ensure_current_generation_unclaimed()?;
        self.current_generation_claimed = true;
        Ok(())
    }

    pub(super) fn ensure_current_generation_unclaimed(&self) -> Result<(), CarrierError> {
        if !self.active {
            return Err(CarrierError::WrongScope);
        }
        if self.current_generation_claimed {
            return Err(CarrierError::NamespaceAlreadyClaimed);
        }
        Ok(())
    }
}

impl ChannelRuntimeAssignmentBinding {
    pub const fn world_id(self) -> WorldId {
        self.world_id
    }

    pub const fn channel_id(self) -> ChannelId {
        self.channel_id
    }

    pub const fn scope_generation(self) -> ScopeOwnershipGeneration {
        self.scope_generation
    }

    /// The committed assignment decision revision this runtime was composed from.
    pub const fn source_revision(self) -> u64 {
        self.source_revision
    }

    #[allow(clippy::too_many_arguments)]
    pub fn matches_committed_assignment(
        self,
        world_id: WorldId,
        channel_id: ChannelId,
        node_id: NodeId,
        node_registration_revision: u64,
        ownership_generation: u64,
        source_revision: u64,
        decision_identity: &str,
    ) -> bool {
        let decision_revision = decision_identity
            .strip_prefix("runtime-scope-assignment:")
            .and_then(|value| value.parse::<u64>().ok());
        self.world_id == world_id
            && self.channel_id == channel_id
            && self.node_id == node_id
            && self.node_registration_revision == node_registration_revision
            && self.scope_generation.get() == ownership_generation
            && self.source_revision == source_revision
            && decision_revision == Some(source_revision)
    }
}

impl oteryn_gameplay_foundation::foundation::owner_cycle_grant_seal::Sealed
    for CommittedAssignmentOwnerCycle
{
}

impl oteryn_gameplay_foundation::foundation::OwnerCycleGrant for CommittedAssignmentOwnerCycle {
    fn owner_scope(&self) -> oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1 {
        oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
            self.world_id,
            self.channel_id,
        )
    }

    fn owner_generation(&self) -> ScopeOwnershipGeneration {
        self.scope_generation
    }
}

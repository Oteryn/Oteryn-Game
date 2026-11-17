//! Historical source-scene fixtures call the canonical #3036 owner, never a second writer.
use super::super::ComposedFreshAdmission;
impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    pub(in crate::gameplay_transport) async fn drive_shared_soulpits(
        &self,
        now: u64,
        global: bool,
    ) -> Result<Option<u64>, ()> {
        super::handoff_owner::NativeSoulpitHandoff
            .drive_soulpit(self, now, global)
            .await
    }
    pub(in crate::gameplay_transport) async fn drive_one_soulpit(
        &self,
        registered: &super::super::activity_runtime::RegisteredRuntime<'a>,
        now: u64,
        global: bool,
    ) -> Result<Option<u64>, super::runtime::SoulpitRuntimeError> {
        use super::runtime::{SoulpitActivityBinding, SoulpitRuntimeError};
        let owner =
            crate::durability::activity_handoff::ActivityHandoffOwner::new(self.root.clone());
        let world = registered.scope.world_id();
        let bindings = owner
            .active_soulpits(self.character, self.holder, world, None)
            .await
            .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
        let Some(binding) = bindings.into_iter().find(|binding| {
            binding.scope == registered.scope && binding.generation.get() == registered.generation
        }) else {
            return Ok(None);
        };
        let members = owner
            .soulpit_members(self.character, self.holder, binding)
            .await
            .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
        super::owner_runtime::drive_registered(
            self,
            registered,
            SoulpitActivityBinding {
                handoff_id: binding.handoff_id,
                scope: binding.scope,
                generation: binding.generation,
            },
            &members,
            now,
            global,
        )
        .await
    }
}

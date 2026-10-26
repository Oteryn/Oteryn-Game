use super::*;

/// Immutable source-qualified admission description. It grants no scope or placement authority.
/// The source adapter binds exact child revision; current physical owner reads its active policy.
#[derive(Debug, Clone)]
pub struct NativeEncounterSpawnSpec {
    parent: Box<str>,
    child: Box<str>,
    child_revision: Box<str>,
    content: [u8; 32],
}

impl NativeEncounterSpawnSpec {
    pub fn qualified(
        parent: &str,
        child: &str,
        child_revision: &str,
        content: [u8; 32],
    ) -> Result<Self, CarrierError> {
        if content == [0; 32] {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        Ok(Self {
            parent: copy_native_summon_key(parent)?,
            child: copy_native_summon_key(child)?,
            child_revision: copy_native_summon_key(child_revision)?,
            content,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterSpawnError {
    StaleOwner,
    MissingMap,
    RefusedMap,
    Native(CarrierError),
}

impl ChannelRuntimeV1 {
    /// Preflight a masterless taming subject under the installed current physical owner.
    /// The caller holds the Channel exclusively through its durable attempt and removal.
    pub fn check_mount_taming_subject(
        &self,
        actor: ExactActorRef,
        key: &str,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if !self.matches_live_creature_identity(actor, key.as_bytes())
            || self.native_summon_master(actor).is_some()
            || self
                .companion_snapshot(actor)
                .is_ok_and(|s| s.state.master.is_some())
            || self.carrier.native_summons.has_children(actor)
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        self.assert_actor_spell_unreserved(actor)
    }

    /// Administrative removal through the existing carrier: no death, corpse, loot or XP.
    pub fn remove_mount_taming_subject(
        &mut self,
        actor: ExactActorRef,
        key: &str,
    ) -> Result<(), CarrierError> {
        self.check_mount_taming_subject(actor, key)?;
        self.carrier.remove(&self.continuity, actor.0).map(|_| ())
    }

    /// Immediate masterless source spawn, through the same physical admission/position/policy
    /// owner as native creatures. No static respawn source, master, intrinsic link or reward override.
    /// Current map proof is independently resolved under this same exclusive owner turn.
    pub fn admit_native_encounter_masterless(
        &mut self,
        current: &oteryn_gameplay_foundation::foundation::ScopeRuntimeFence,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
        parent: ExactActorRef,
        spec: &NativeEncounterSpawnSpec,
        at: MovementLocalPosition,
        map_admits: impl FnOnce(&ChannelRuntimeV1, MovementLocalPosition) -> Option<bool>,
    ) -> Result<ExactActorRef, EncounterSpawnError> {
        use EncounterSpawnError as E;
        let b = self.binding();
        if !current.is_current_for_scope(
            oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
                b.world_id(),
                b.channel_id(),
            ),
            b.scope_generation(),
        ) || !current.accepts_stamp(stamp)
        {
            return Err(E::StaleOwner);
        }
        if spec.content != self.content.server_artifact_digest()
            || !self.matches_live_creature_identity(parent, spec.parent.as_bytes())
        {
            return Err(E::Native(CarrierError::CreatureTargetMismatch));
        }
        // This reads the active loader-installed qualified table. A matching display name
        // cannot substitute for the exact child definition or revision.
        let policy = self.companion_policy(&spec.child).map_err(E::Native)?;
        if policy.definition_key.as_str() != spec.child.as_ref()
            || policy.definition_revision.as_str() != spec.child_revision.as_ref()
        {
            return Err(E::Native(CarrierError::CreatureTargetMismatch));
        }
        match map_admits(self, at) {
            None => return Err(E::MissingMap),
            Some(false) => return Err(E::RefusedMap),
            Some(true) => {}
        }
        // `CREATUREAI0-RL-04` (Codex 4242954089), as for native summons: it replaces the D57
        // envelope of 64 live creatures.
        if self.carrier.occupied.len() >= CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX
            || !self.native_summon_cell_free(at)
        {
            return Err(E::Native(CarrierError::CapacityExceeded));
        }
        let context = self.pinned_position_context();
        self.carrier
            .validate_position_context(context)
            .map_err(E::Native)?;
        // Existing carrier compare-commit behavior: either all physical fields are installed,
        // or every modified slot/free-list field is restored before returning a rejection.
        let slots_before = self.carrier.slots.clone();
        let free_before = self.carrier.free_head;
        let occupied_before = self.carrier.occupied.clone();
        let result = (|| {
            let actor = self.carrier.admit_creature(
                &self.continuity,
                ActorState(0),
                &spec.child,
                policy.maximum_health,
            )?;
            self.carrier.initialize_position(
                &self.continuity,
                actor,
                context,
                LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            )?;
            let actor = ExactActorRef(actor);
            self.install_creature_policy(actor, &spec.child)?;
            Ok(actor)
        })();
        if result.is_err() {
            self.carrier.slots = slots_before;
            self.carrier.free_head = free_before;
            self.carrier.occupied = occupied_before;
        }
        result.map_err(E::Native)
    }
    /// Source callback administrative removal: no lethal mint, corpse, loot or XP handoff.
    /// Exact occupied generation + source policy are current; immutable source pins are not grants.
    pub fn remove_native_encounter_subject(
        &mut self,
        current: &oteryn_gameplay_foundation::foundation::ScopeRuntimeFence,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
        actor: ExactActorRef,
        expected_key: &str,
        expected_content: [u8; 32],
    ) -> Result<(), CarrierError> {
        let b = self.binding();
        if !current.is_current_for_scope(
            oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
                b.world_id(),
                b.channel_id(),
            ),
            b.scope_generation(),
        ) || !current.accepts_stamp(stamp)
        {
            return Err(CarrierError::WrongScope);
        }
        if self.content_pin().server_artifact_digest() != expected_content
            || !self.matches_live_creature_identity(actor, expected_key.as_bytes())
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let index = self
            .carrier
            .validate_summon_actor(&self.continuity, actor)?;
        self.carrier.assert_slot_spell_unreserved(index)?;
        // Existing removal cascades intrinsic children. Preflight every fallible current-slot
        // check before entering it; no mutation can occur between checks in this owner turn.
        let mut count = 0;
        for link in self.carrier.native_summons.children(actor) {
            count += 1;
            if count > 16 {
                return Err(CarrierError::CapacityExceeded);
            }
            let child = self
                .carrier
                .validate_summon_actor(&self.continuity, link.child)?;
            self.carrier.assert_slot_spell_unreserved(child)?;
            if self.carrier.native_summons.has_children(link.child) {
                return Err(CarrierError::CreatureTargetMismatch);
            }
        }
        self.carrier.remove(&self.continuity, actor.0).map(|_| ())
    }
}

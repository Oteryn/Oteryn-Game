//! Existing physical summon owner, with entry points for its installed current fence.
use super::*;
impl ChannelRuntimeV1 {
    pub fn admit_native_summon(
        &mut self,
        current: &oteryn_gameplay_foundation::foundation::ScopeRuntimeFence,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
        parent: ExactActorRef,
        spec: &NativeSummonAdmissionSpec,
        at: MovementLocalPosition,
    ) -> Result<ExactActorRef, CarrierError> {
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
        self.retire_native_summons(current, stamp)?;
        self.admit_native_summon_checked(parent, spec, at)
    }
    pub fn admit_current_native_summon(
        &mut self,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
        parent: ExactActorRef,
        spec: &NativeSummonAdmissionSpec,
        at: MovementLocalPosition,
    ) -> Result<ExactActorRef, CarrierError> {
        if !self.owner_fence()?.accepts_stamp(stamp) {
            return Err(CarrierError::WrongScope);
        }
        self.retire_current_native_summons(stamp)?;
        self.admit_native_summon_checked(parent, spec, at)
    }
    fn admit_native_summon_checked(
        &mut self,
        parent: ExactActorRef,
        spec: &NativeSummonAdmissionSpec,
        at: MovementLocalPosition,
    ) -> Result<ExactActorRef, CarrierError> {
        if spec.content() != self.content.server_artifact_digest()
            || !self.matches_live_creature_identity(parent, spec.parent_key().as_bytes())
            || self.native_summon_master(parent).is_some()
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        // The bounded pass above may not have reached this parent's links (Codex 4243039217):
        // retire its own dead children, at most 16, so its links stay its live children and so
        // at most 16 with the one admitted below.
        let mut dead = [None; 16];
        let mut count = 0;
        for link in self.carrier.native_summons.children(parent) {
            if count == 16 {
                return Err(CarrierError::CapacityExceeded);
            }
            let index = self
                .carrier
                .validate_summon_actor(&self.continuity, link.child)?;
            if matches!(
                &self.carrier.slots[index],
                Slot::CreatureOccupied { health: 0, .. }
            ) {
                dead[count] = Some(link.child);
                count += 1;
            }
        }
        if count > 0 {
            self.carrier
                .retire_dead_summons(&self.continuity, dead.iter().flatten().copied())?;
        }
        if self.native_summon_count(parent, None) >= spec.total()
            || self.native_summon_count(parent, Some(spec.child_key())) >= spec.count()
        {
            return Err(CarrierError::CapacityExceeded);
        }
        // `CREATUREAI0-RL-04` (Codex 4242954089), the bound `admit_creature` enforces, checked
        // before anything is written; it replaces the D57 envelope of 64 live creatures. Every
        // link names a distinct child that holds a slot until its removal drops the link, so
        // the links are bounded by the same value.
        if self.carrier.occupied.len() >= CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX
            || self.carrier.native_summons.len() >= CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX
            || !self.native_summon_cell_free(at)
        {
            return Err(CarrierError::CapacityExceeded);
        }
        let target = copy_bounded_binding(spec.child_key().as_bytes())?;
        let origin = spec.origin.try_owned()?;
        let context = self.pinned_position_context();
        self.carrier.validate_position_context(context)?;
        let slots = self.carrier.slots.clone();
        let free = self.carrier.free_head;
        let occupied = self.carrier.occupied.clone();
        let actor = self.carrier.admit_creature(
            &self.continuity,
            ActorState(0),
            spec.child_key(),
            spec.health(),
        )?;
        if let Err(e) = self.carrier.initialize_position(
            &self.continuity,
            actor,
            context,
            LocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        ) {
            self.carrier.slots = slots;
            self.carrier.free_head = free;
            self.carrier.occupied = occupied;
            return Err(e);
        }
        let child = ExactActorRef(actor);
        if let Err(e) = self.carrier.native_summons.insert(NativeSummonLink {
            parent,
            child,
            target,
            origin,
            death: None,
        }) {
            self.carrier.slots = slots;
            self.carrier.free_head = free;
            self.carrier.occupied = occupied;
            return Err(e);
        }
        Ok(child)
    }
    pub fn retire_native_summons(
        &mut self,
        current: &oteryn_gameplay_foundation::foundation::ScopeRuntimeFence,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
    ) -> Result<usize, CarrierError> {
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
        self.retire_native_summons_checked()
    }
    pub fn retire_current_native_summons(
        &mut self,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
    ) -> Result<usize, CarrierError> {
        if !self.owner_fence()?.accepts_stamp(stamp) {
            return Err(CarrierError::WrongScope);
        }
        self.retire_native_summons_checked()
    }
    fn retire_native_summons_checked(&mut self) -> Result<usize, CarrierError> {
        // A bounded pass (`SPAWNCEIL-RL-02`, Codex 4243039217): at most 256 links from where the
        // previous call stopped, wrapping, so any number of dead children is retired over later
        // calls without one call costing every link. It never refuses: the old fixed array of 64
        // refused the whole retirement, and with it every summon admission, once a 65th child was
        // dead (Codex 4242954089), and an admission retires its own parent's dead children.
        let (window, cursor) = self
            .carrier
            .native_summons
            .retirement_window(NATIVE_SUMMON_RETIREMENT_LINKS_PER_CALL_MAX)?;
        let mut dead = Vec::new();
        dead.try_reserve(window.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for child in window {
            let index = self
                .carrier
                .validate_summon_actor(&self.continuity, child)?;
            if matches!(
                &self.carrier.slots[index],
                Slot::CreatureOccupied { health: 0, .. }
            ) {
                dead.push(child);
            }
        }
        self.carrier.native_summons.retire_cursor = cursor;
        let count = dead.len();
        if count > 0 {
            self.carrier.retire_dead_summons(&self.continuity, dead)?;
        }
        Ok(count)
    }
    pub fn retire_native_summon_registration_failure(
        &mut self,
        current: &oteryn_gameplay_foundation::foundation::ScopeRuntimeFence,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
        actor: ExactActorRef,
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
        self.retire_native_summon_registration_checked(actor)
    }
    pub fn retire_current_native_summon_registration_failure(
        &mut self,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        if !self.owner_fence()?.accepts_stamp(stamp) {
            return Err(CarrierError::WrongScope);
        }
        self.retire_native_summon_registration_checked(actor)
    }
    fn retire_native_summon_registration_checked(
        &mut self,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        if self.native_summon_role(actor)?.is_none() {
            return Err(CarrierError::NotCreature);
        }
        let index = self
            .carrier
            .validate_summon_actor(&self.continuity, actor)?;
        if matches!(
            &self.carrier.slots[index],
            Slot::CreatureOccupied { health: 0, .. }
        ) {
            self.carrier
                .project_native_summon_death_inner(&self.continuity, actor)?;
        }
        self.carrier.remove(&self.continuity, actor.0)?;
        Ok(())
    }
}

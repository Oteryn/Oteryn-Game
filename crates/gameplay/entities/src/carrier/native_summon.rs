use super::*;
use std::collections::BTreeMap;

// MONSTER-SUMMON-1 intrinsic ephemeral relation, owned by this physical Channel carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeSummonLink {
    pub(super) parent: ExactActorRef,
    pub(super) child: ExactActorRef,
    pub(super) target: Box<[u8]>,
    pub(super) origin: NativeSummonOrigin,
    pub(super) death: Option<NativeSummonDeath>,
}

/// A slot and its generation: the part of an [`ExactActorRef`] that tells this carrier's actors
/// apart. A lookup by it still compares the whole reference.
type NativeSummonActorKey = (u32, u64);

fn native_summon_actor_key(actor: ExactActorRef) -> NativeSummonActorKey {
    (actor.0.actor_local_id.0, actor.0.actor_local_generation.0)
}

/// SPAWN-CEILINGS-1 (Codex 4243039217): the native summon links, indexed so that finding,
/// replacing or dropping a child's link and listing a parent's children cost the logarithm of the
/// links, never a pass over all of them. Links keep their admission order (the order of the
/// former list), so every per-parent and retirement pass visits them in the same order as before.
///
/// Equality compares the links in admission order, like the former list: not the ordinals nor
/// `retire_cursor`, which only order the links and schedule the bounded retirement work.
#[derive(Debug, Clone, Default)]
pub(super) struct NativeSummonLinks {
    /// Every link by its admission ordinal.
    links: BTreeMap<u64, NativeSummonLink>,
    /// The admission ordinal of each child's link.
    by_child: BTreeMap<NativeSummonActorKey, u64>,
    /// The parent and admission ordinal of every link.
    by_parent: BTreeSet<(NativeSummonActorKey, u64)>,
    /// The admission ordinal of the next link.
    next: u64,
    /// The admission ordinal the next bounded retirement pass starts at
    /// (`NATIVE_SUMMON_RETIREMENT_LINKS_PER_CALL_MAX`).
    pub(super) retire_cursor: u64,
}

impl PartialEq for NativeSummonLinks {
    fn eq(&self, other: &Self) -> bool {
        // The links in admission order, as the former list compared; `by_child` and `by_parent`
        // are derived from `links`, and the ordinals only order them.
        self.links.values().eq(other.links.values())
    }
}

impl Eq for NativeSummonLinks {}

impl NativeSummonLinks {
    pub(super) fn len(&self) -> usize {
        self.links.len()
    }
    #[cfg(any(test, feature = "test-support"))]
    pub(super) fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
    fn ordinal_of_child(&self, child: ExactActorRef) -> Option<u64> {
        let ordinal = *self.by_child.get(&native_summon_actor_key(child))?;
        self.links
            .get(&ordinal)
            .is_some_and(|link| link.child == child)
            .then_some(ordinal)
    }
    /// The link whose child is exactly `child`.
    pub(super) fn child(&self, child: ExactActorRef) -> Option<&NativeSummonLink> {
        self.links.get(&self.ordinal_of_child(child)?)
    }
    fn child_mut(&mut self, child: ExactActorRef) -> Option<&mut NativeSummonLink> {
        let ordinal = self.ordinal_of_child(child)?;
        self.links.get_mut(&ordinal)
    }
    /// The links whose parent is exactly `parent`, in admission order.
    pub(super) fn children(
        &self,
        parent: ExactActorRef,
    ) -> impl Iterator<Item = &NativeSummonLink> + '_ {
        let key = native_summon_actor_key(parent);
        self.by_parent
            .range((key, 0)..=(key, u64::MAX))
            .filter_map(|(_, ordinal)| self.links.get(ordinal))
            .filter(move |link| link.parent == parent)
    }
    pub(super) fn has_children(&self, parent: ExactActorRef) -> bool {
        self.children(parent).next().is_some()
    }
    /// The links whose parent or child is `actor`, in admission order.
    fn touching(&self, actor: ActorRef) -> Vec<&NativeSummonLink> {
        let exact = ExactActorRef(actor);
        let mut found = self
            .ordinal_of_child(exact)
            .into_iter()
            .chain(
                self.by_parent
                    .range(
                        (native_summon_actor_key(exact), 0)
                            ..=(native_summon_actor_key(exact), u64::MAX),
                    )
                    .map(|(_, ordinal)| *ordinal),
            )
            .collect::<Vec<_>>();
        found.sort_unstable();
        found.dedup();
        found
            .into_iter()
            .filter_map(|ordinal| self.links.get(&ordinal))
            .filter(|link| link.parent.0 == actor || link.child.0 == actor)
            .collect()
    }
    pub(super) fn insert(&mut self, link: NativeSummonLink) -> Result<(), CarrierError> {
        let child = native_summon_actor_key(link.child);
        if self.by_child.contains_key(&child) {
            return Err(CarrierError::PlanConflict);
        }
        let ordinal = self.next;
        self.next = ordinal
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        self.by_child.insert(child, ordinal);
        self.by_parent
            .insert((native_summon_actor_key(link.parent), ordinal));
        self.links.insert(ordinal, link);
        Ok(())
    }
    /// Drops the link whose child is exactly `child`.
    pub(super) fn remove_child(&mut self, child: ExactActorRef) -> Option<NativeSummonLink> {
        let ordinal = self.ordinal_of_child(child)?;
        let link = self.links.remove(&ordinal)?;
        self.by_child.remove(&native_summon_actor_key(child));
        self.by_parent
            .remove(&(native_summon_actor_key(link.parent), ordinal));
        Some(link)
    }
    /// The children of at most `budget` links from the retirement cursor on, wrapping once, and
    /// the cursor after them.
    pub(super) fn retirement_window(
        &self,
        budget: usize,
    ) -> Result<(Vec<ExactActorRef>, u64), CarrierError> {
        let mut window = Vec::new();
        window
            .try_reserve(budget.min(self.links.len()))
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut cursor = self.retire_cursor;
        for (ordinal, link) in self
            .links
            .range(self.retire_cursor..)
            .chain(self.links.range(..self.retire_cursor))
            .take(budget)
        {
            window.push(link.child);
            cursor = ordinal.saturating_add(1);
        }
        Ok((window, cursor))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSummonDeath {
    pub child: ExactActorRef,
    pub death: CreatureDeathOccurrenceKey,
    pub position: MovementLocalPosition,
}

impl ChannelRuntimeV1 {
    pub fn native_summon_role(
        &self,
        actor: ExactActorRef,
    ) -> Result<Option<ExactActorRef>, CarrierError> {
        self.carrier
            .validate_summon_actor(&self.continuity, actor)?;
        Ok(self.carrier.native_summons.child(actor).map(|l| l.parent))
    }
    pub fn native_summon_master(&self, actor: ExactActorRef) -> Option<ExactActorRef> {
        self.native_summon_role(actor).ok().flatten()
    }
    pub fn native_summon_count(&self, parent: ExactActorRef, target: Option<&str>) -> u32 {
        self.carrier
            .native_summons
            .children(parent)
            .filter(|l| {
                self.contains_live_creature(l.child)
                    && target.is_none_or(|t| l.target.as_ref() == t.as_bytes())
            })
            .count() as u32
    }
    pub fn native_summon_cell_free(&self, at: MovementLocalPosition) -> bool {
        !self.carrier.cell_occupied(
            self.pinned_position_context(),
            LocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        )
    }

    /// Summon death has its own sealed native key/position, never an ordinary corpse/reward.
    pub fn project_native_summon_death(
        &mut self,
        actor: ExactActorRef,
    ) -> Result<NativeSummonDeath, CarrierError> {
        self.carrier
            .project_native_summon_death_inner(&self.continuity, actor)
    }
}

impl ChannelActorCarrier {
    pub(super) fn validate_summon_actor(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor: ExactActorRef,
    ) -> Result<usize, CarrierError> {
        let index = self.validate_ref(continuity, actor.0)?;
        match &self.slots[index] {
            Slot::CreatureOccupied { generation, .. }
                if *generation == actor.0.actor_local_generation.0 =>
            {
                Ok(index)
            }
            Slot::Occupied { generation, .. }
                if *generation == actor.0.actor_local_generation.0 =>
            {
                Err(CarrierError::NotCreature)
            }
            _ => Err(CarrierError::StaleActorGeneration),
        }
    }

    /// Seals the death of each dead child of `dead` and removes it, as
    /// [`Self::project_native_summon_death_inner`] then [`Self::remove`] each, but drops the
    /// vacated slots from the occupied index and the attacker leases in one pass each (Codex
    /// 4243039217). On a failure the children removed so far stay removed, as before.
    pub(super) fn retire_dead_summons(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        dead: impl IntoIterator<Item = ExactActorRef>,
    ) -> Result<(), CarrierError> {
        let dead = dead.into_iter();
        let mut vacated = Vec::new();
        vacated
            .try_reserve(dead.size_hint().0)
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut result = Ok(());
        for child in dead {
            let removed = self
                .project_native_summon_death_inner(continuity, child)
                .and_then(|_| self.remove_keeping_indexes(continuity, child.0));
            match removed {
                Ok((_, index)) => {
                    if vacated.try_reserve(1).is_err() {
                        self.unindex_slot(index);
                    } else {
                        vacated.push(index);
                    }
                }
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
        vacated.sort_unstable();
        if !vacated.is_empty() {
            self.occupied.retain(|index| {
                !usize::try_from(*index).is_ok_and(|index| vacated.binary_search(&index).is_ok())
            });
            self.attackers
                .retain(|entry| vacated.binary_search(&entry.index).is_err());
        }
        result
    }

    pub(super) fn is_native_summon(&self, actor: ExactActorRef) -> bool {
        self.native_summons.child(actor).is_some()
            || self
                .native_summon_deaths
                .iter()
                .flatten()
                .any(|d| d.child == actor)
    }
    pub(super) fn project_native_summon_death_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ExactActorRef,
    ) -> Result<NativeSummonDeath, CarrierError> {
        self.validate_ref(continuity, actor.0)?;
        if let Some(death) = self
            .native_summon_deaths
            .iter()
            .flatten()
            .find(|d| d.child == actor)
        {
            return Ok(*death);
        }
        self.validate_summon_actor(continuity, actor)?;
        if let Some(death) = self
            .native_summons
            .child(actor)
            .ok_or(CarrierError::NotCreature)?
            .death
        {
            return Ok(death);
        }
        let receipt = self.committed_lethal_receipt_inner(continuity, actor.0)?;
        self.validate_lethal_receipt(continuity, &receipt)?;
        let p = receipt.projection.position.position;
        let death = NativeSummonDeath {
            child: actor,
            death: receipt.projection.occurrence.death_key(),
            position: MovementLocalPosition {
                x: p.x,
                y: p.y,
                floor: p.floor,
            },
        };
        self.native_summons
            .child_mut(actor)
            .ok_or(CarrierError::NotCreature)?
            .death = Some(death);
        Ok(death)
    }
    /// Under the same exclusive physical owner turn as ordinary parent projection. A failure
    /// restores child HP/receipts before returning; no corpse/XP or durable workflow is created.
    pub(super) fn cascade_native_summon_deaths(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        parent: ExactActorRef,
    ) -> Result<(), CarrierError> {
        let mut children = [None; 16];
        let mut count = 0;
        for link in self.native_summons.children(parent) {
            if count == 16 {
                return Err(CarrierError::CapacityExceeded);
            }
            children[count] = Some(link.child);
            count += 1;
        }
        if count == 0 {
            return Ok(());
        }
        let before = self.slots.clone();
        let mut deaths = Vec::new();
        deaths
            .try_reserve(count)
            .map_err(|_| CarrierError::AllocationFailed)?;
        let result = (|| {
            for child in children.into_iter().flatten() {
                let index = match self.validate_summon_actor(continuity, child) {
                    Ok(i) => i,
                    Err(CarrierError::StaleActorGeneration) => continue,
                    Err(e) => return Err(e),
                };
                let Slot::CreatureOccupied {
                    health,
                    target_identity,
                    ..
                } = &self.slots[index]
                else {
                    return Err(CarrierError::NotCreature);
                };
                if *health > 0 {
                    let health = *health;
                    let target = copy_bounded_binding(target_identity)?;
                    let occurrence = format!(
                        "monster-master-death:{}:{}",
                        parent.0.actor_local_id.0, parent.0.actor_local_generation.0
                    );
                    let mut binding = occurrence.as_bytes().to_vec();
                    binding.extend_from_slice(b"\0MONSTER-SUMMON-1-master-lethal");
                    self.commit_creature_damage_inner(
                        continuity,
                        child.0,
                        OwnerDamageCommand {
                            target: &target,
                            occurrence: occurrence.as_bytes(),
                            binding: &binding,
                            damage: health,
                        },
                        None,
                        false,
                    )?;
                }
                let receipt = self.committed_lethal_receipt_inner(continuity, child.0)?;
                self.validate_lethal_receipt(continuity, &receipt)?;
                let p = receipt.projection.position.position;
                deaths.push(NativeSummonDeath {
                    child,
                    death: receipt.projection.occurrence.death_key(),
                    position: MovementLocalPosition {
                        x: p.x,
                        y: p.y,
                        floor: p.floor,
                    },
                });
            }
            Ok(())
        })();
        if let Err(e) = result {
            self.slots = before;
            return Err(e);
        }
        for death in deaths {
            if let Some(link) = self.native_summons.child_mut(death.child) {
                link.death = Some(death)
            }
        }
        Ok(())
    }
}

/// Bounded immutable physical admission input. Root content qualifier supplies source proof;
/// Foundation has no dependency on the project's loader or monster module.
#[derive(Debug, Clone)]
pub struct NativeSummonAdmissionSpec {
    parent: Box<str>,
    child: Box<str>,
    health: i64,
    index: usize,
    pub(super) origin: NativeSummonOrigin,
    total: u32,
    count: u32,
    content: [u8; 32],
}

pub(super) fn copy_native_summon_key(value: &str) -> Result<Box<str>, CarrierError> {
    if value.is_empty() || value.len() > MAX_OWNER_COMMIT_BINDING_BYTES {
        return Err(CarrierError::InvalidCreatureTarget);
    }
    let mut owned = String::new();
    owned
        .try_reserve_exact(value.len())
        .map_err(|_| CarrierError::CommitAllocationFailed)?;
    owned.push_str(value);
    Ok(owned.into_boxed_str())
}

impl NativeSummonAdmissionSpec {
    pub fn qualified(
        parent: &str,
        child: &str,
        health: i64,
        index: usize,
        total: u32,
        count: u32,
        content: [u8; 32],
    ) -> Result<Self, CarrierError> {
        if parent.is_empty()
            || child.is_empty()
            || health <= 0
            || index >= 8
            || total == 0
            || total > 16
            || count == 0
            || count > 16
        {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        Ok(Self {
            parent: copy_native_summon_key(parent)?,
            child: copy_native_summon_key(child)?,
            health,
            index,
            origin: NativeSummonOrigin::Behavior { index },
            total,
            count,
            content,
        })
    }
    pub fn parent_key(&self) -> &str {
        &self.parent
    }
    pub fn child_key(&self) -> &str {
        &self.child
    }
    pub fn health(&self) -> i64 {
        self.health
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn total(&self) -> u32 {
        self.total
    }
    pub fn count(&self) -> u32 {
        self.count
    }
    pub fn content(&self) -> [u8; 32] {
        self.content
    }
}

impl ChannelActorCarrier {
    pub(super) fn summons_separated_by_move(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorRef,
        next: LocalPosition,
    ) -> Result<[Option<ExactActorRef>; 64], CarrierError> {
        let mut removed = [None; 64];
        let mut count = 0;
        for link in self.native_summons.touching(actor) {
            let child = self.validate_summon_actor(continuity, link.child)?;
            let parent = match self.validate_summon_actor(continuity, link.parent) {
                Ok(index) => index,
                Err(CarrierError::StaleActorGeneration) => {
                    removed[count] = Some(link.child);
                    count += 1;
                    continue;
                }
                Err(e) => return Err(e),
            };
            let position = |index: usize| match &self.slots[index] {
                Slot::CreatureOccupied {
                    position: Some(p), ..
                } => Ok(p.position),
                _ => Err(CarrierError::PositionUnavailable),
            };
            let child_at = if link.child.0 == actor {
                next
            } else {
                position(child)?
            };
            let parent_at = if link.parent.0 == actor {
                next
            } else {
                position(parent)?
            };
            if (i64::from(child_at.x) - i64::from(parent_at.x)).abs() > 30
                || (i64::from(child_at.y) - i64::from(parent_at.y)).abs() > 30
                || (i32::from(child_at.floor) - i32::from(parent_at.floor)).abs() > 2
            {
                if count == 64 {
                    return Err(CarrierError::CapacityExceeded);
                }
                removed[count] = Some(link.child);
                count += 1;
            }
        }
        Ok(removed)
    }
    pub(super) fn remove_separated_summons(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        removed: [Option<ExactActorRef>; 64],
    ) -> Result<(), CarrierError> {
        for child in removed.into_iter().flatten() {
            let index = self.validate_summon_actor(continuity, child)?;
            if matches!(&self.slots[index], Slot::CreatureOccupied { health: 0, .. }) {
                self.project_native_summon_death_inner(continuity, child)?;
            }
            self.remove(continuity, child.0)?;
        }
        Ok(())
    }
}

/// Private source lifecycle receipt: a removed moving child has no published stale position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSummonMovementOutcome {
    Moved(MovementPositionSnapshot),
    Removed(ExactActorRef),
}

impl ChannelRuntimeV1 {
    pub fn commit_native_summon_cardinal(
        &mut self,
        current: &oteryn_gameplay_foundation::foundation::ScopeRuntimeFence,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
        actor: ExactActorRef,
        expected: MovementPositionSnapshot,
        next: MovementLocalPosition,
    ) -> Result<NativeSummonMovementOutcome, CarrierError> {
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
        let parent = self
            .native_summon_role(actor)?
            .ok_or(CarrierError::NotCreature)?;
        if !self.contains_live_creature(actor) || !self.contains_live_creature(parent) {
            return Err(CarrierError::CreatureNotActionable);
        }
        if expected.0.actor_ref != actor.0
            || self.read_actor_position(actor)? != expected
            || expected.context() != self.pinned_movement_context()
        {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        let at = expected.position();
        let dx = (i64::from(next.x) - i64::from(at.x)).abs();
        let dy = (i64::from(next.y) - i64::from(at.y)).abs();
        if next.floor != at.floor || dx + dy != 1 {
            return Err(CarrierError::MovementNonCardinal);
        }
        if !self.native_summon_cell_free(next) {
            return Err(CarrierError::CapacityExceeded);
        }
        let committed = self.carrier.compare_commit_position_with_summon_lifecycle(
            &self.continuity,
            expected.0,
            expected.0.version.context,
            LocalPosition {
                x: next.x,
                y: next.y,
                floor: next.floor,
            },
            true,
        )?;
        if self.native_summon_role(actor).is_err() {
            Ok(NativeSummonMovementOutcome::Removed(actor))
        } else {
            Ok(NativeSummonMovementOutcome::Moved(
                MovementPositionSnapshot(committed),
            ))
        }
    }
}

/// Private intrinsic-link provenance; it cannot authorize a current owner mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeSummonOrigin {
    Behavior {
        index: usize,
    },
    SourceCallback {
        ability: Box<str>,
        revision: Box<str>,
        occurrence: Box<str>,
    },
}

impl NativeSummonOrigin {
    pub(super) fn try_owned(&self) -> Result<Self, CarrierError> {
        match self {
            Self::Behavior { index } => Ok(Self::Behavior { index: *index }),
            Self::SourceCallback {
                ability,
                revision,
                occurrence,
            } => Ok(Self::SourceCallback {
                ability: copy_native_summon_key(ability)?,
                revision: copy_native_summon_key(revision)?,
                occurrence: copy_native_summon_key(occurrence)?,
            }),
        }
    }
}

impl NativeSummonAdmissionSpec {
    #[allow(
        clippy::too_many_arguments,
        reason = "Owner ABI keeps independently qualified source, current fence, exact actor and commit facts explicit"
    )]
    pub fn qualified_callback(
        parent: &str,
        child: &str,
        health: i64,
        ability: &str,
        revision: &str,
        occurrence: &str,
        total: u32,
        count: u32,
        content: [u8; 32],
    ) -> Result<Self, CarrierError> {
        let mut spec = Self::qualified(parent, child, health, 0, total, count, content)?;
        spec.origin = NativeSummonOrigin::SourceCallback {
            ability: copy_native_summon_key(ability)?,
            revision: copy_native_summon_key(revision)?,
            occurrence: copy_native_summon_key(occurrence)?,
        };
        Ok(spec)
    }
}

impl ChannelRuntimeV1 {
    pub fn native_summon_origin(
        &self,
        actor: ExactActorRef,
    ) -> Result<Option<NativeSummonOrigin>, CarrierError> {
        self.carrier
            .validate_summon_actor(&self.continuity, actor)?;
        Ok(self
            .carrier
            .native_summons
            .child(actor)
            .map(|l| l.origin.clone()))
    }
}

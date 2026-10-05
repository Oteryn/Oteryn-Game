//! Existing MONSTER-SUMMON-1 proposal -> exact native physical owner. No factory/grant defaults.
use crate::ai_think::ThinkOccurrence;
use crate::ai_think::profile_schedule::{
    MonsterSummonFacts, ObservedSummonCount, ProfileSummonProposal,
};
use crate::content::{
    ProjectReferenceRecord, ProjectV2AuthoringProfileData as Data, ProjectV2DefinitionRef as Ref,
    ProjectV2Draft, ProjectV2Family, ProjectV2SourceIdentityDisposition,
};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, MovementLocalPosition,
    NativeSummonAdmissionSpec, RuntimeScopeRefV1, RuntimeWorkStamp, ScopeRuntimeFence,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SummonError {
    InvalidSource,
    UndefinedTarget,
    StaleOwner,
    StaleParent,
    StaleTarget,
    MissingMap,
    NoFreeTile,
    Capacity,
    ReplayConflict,
    Superseded,
    Carrier(CarrierError),
}
#[derive(Debug, Clone)]
pub(crate) struct NativeSummonSpec {
    parent: Ref,
    child: Ref,
    health: i64,
    index: usize,
    total: u32,
    count: u32,
    content: [u8; 32],
    admission: NativeSummonAdmissionSpec,
}
impl NativeSummonSpec {
    pub(crate) fn parent_key(&self) -> &str {
        &self.parent.key
    }
    pub(crate) fn child_key(&self) -> &str {
        &self.child.key
    }
    pub(crate) fn health(&self) -> i64 {
        self.health
    }
    pub(crate) fn index(&self) -> usize {
        self.index
    }
    pub(crate) fn total(&self) -> u32 {
        self.total
    }
    pub(crate) fn count(&self) -> u32 {
        self.count
    }
    pub(crate) fn content(&self) -> [u8; 32] {
        self.content
    }
}
// Immutable source specs only; live facts and native admission remain independently owned.
#[derive(Clone)]
pub(crate) struct NativeSummonCatalog {
    parent: Ref,
    entries: Vec<Result<NativeSummonSpec, SummonError>>,
    targets: Vec<Ref>,
    content: [u8; 32],
}
fn member(draft: &ProjectV2Draft, r: &Ref) -> bool {
    r.family==ProjectV2Family::Creature && draft.core.records.iter().filter(|record|matches!(record,ProjectReferenceRecord::Creature{identity,..}if identity.family=="Creature"&&identity.key==r.key&&identity.revision==r.revision)).count()==1
 &&draft.state.source_identity_bindings.iter().any(|b|b.target==*r&&b.disposition==ProjectV2SourceIdentityDisposition::Exact&&draft.state.sources.iter().any(|s|s.key==b.source_key&&s.revision==b.source_revision))
}
impl NativeSummonCatalog {
    pub(crate) fn from_project(
        runtime: &ChannelRuntimeV1,
        parent: &Ref,
        draft: &ProjectV2Draft,
        loader_digest: [u8; 32],
    ) -> Result<Self, SummonError> {
        if !native_draft_world_matches(runtime, draft)
            || loader_digest != runtime.content_pin().server_artifact_digest()
            || !member(draft, parent)
        {
            return Err(SummonError::InvalidSource);
        }
        let record = draft
            .core
            .records
            .iter()
            .find_map(|r| match r {
                ProjectReferenceRecord::Creature {
                    identity, behavior, ..
                } if identity.key == parent.key && identity.revision == parent.revision => {
                    Some(behavior)
                }
                _ => None,
            })
            .ok_or(SummonError::InvalidSource)?;
        let behavior = draft
            .state
            .authoring_profiles
            .iter()
            .find_map(|p| match &p.data {
                Data::Behavior(b)
                    if p.target.key == record.key && p.target.revision == record.revision =>
                {
                    Some(b)
                }
                _ => None,
            })
            .ok_or(SummonError::InvalidSource)?;
        let summons = behavior
            .summons
            .as_ref()
            .ok_or(SummonError::InvalidSource)?;
        if summons.entries.len() > 8 || summons.max_summons > 16 || summons.max_summons == 0 {
            return Err(SummonError::InvalidSource);
        }
        let entries = summons
            .entries
            .iter()
            .enumerate()
            .map(|(index, e)| {
                if !member(draft, &e.creature) {
                    return Err(SummonError::UndefinedTarget);
                }
                if e.interval_ms == 0 || e.chance_ppm > 1_000_000 || e.count == 0 || e.count > 16 {
                    return Err(SummonError::InvalidSource);
                }
                let profiles = draft
                    .state
                    .authoring_profiles
                    .iter()
                    .filter(|p| p.target == e.creature)
                    .collect::<Vec<_>>();
                let [p] = profiles.as_slice() else {
                    return Err(SummonError::InvalidSource);
                };
                let Data::Creature(child) = &p.data else {
                    return Err(SummonError::InvalidSource);
                };
                let health = child
                    .health
                    .and_then(|h| i64::try_from(h).ok())
                    .filter(|h| *h > 0)
                    .ok_or(SummonError::InvalidSource)?;
                Ok(NativeSummonSpec {
                    parent: parent.clone(),
                    child: e.creature.clone(),
                    health,
                    index,
                    total: summons.max_summons,
                    count: e.count,
                    content: loader_digest,
                    admission: NativeSummonAdmissionSpec::qualified(
                        &parent.key,
                        &e.creature.key,
                        health,
                        index,
                        summons.max_summons,
                        e.count,
                        loader_digest,
                    )
                    .map_err(SummonError::Carrier)?,
                })
            })
            .collect();
        Ok(Self {
            parent: parent.clone(),
            entries,
            targets: summons.entries.iter().map(|e| e.creature.clone()).collect(),
            content: loader_digest,
        })
    }
    pub(crate) fn facts(
        &self,
        runtime: &ChannelRuntimeV1,
        occurrence: ThinkOccurrence,
        target_with_path: Option<ExactActorRef>,
    ) -> MonsterSummonFacts {
        MonsterSummonFacts {
            occurrence,
            is_summon: current_source_creature_role(runtime, occurrence.actor)
                .map(|role| !role.is_ordinary())
                .unwrap_or(true),
            target_with_path,
            total_count: runtime.native_summon_count(occurrence.actor, None),
            entry_counts: self
                .targets
                .iter()
                .map(|creature| ObservedSummonCount {
                    creature: creature.clone(),
                    count: runtime.native_summon_count(occurrence.actor, Some(&creature.key)),
                })
                .collect(),
        }
    }
}
/// All facts come from the actual current map/path owner under the same Channel turn.
/// Unknown terrain, floor-change, PZ, teleport, sight/path or admission never means allow.
pub(crate) trait SummonLocationPolicy {
    fn current_target_reachable(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        target: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
    fn current_cell_admits(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        child: &Ref,
        position: MovementLocalPosition,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SummonReceipt {
    pub(crate) child: ExactActorRef,
    pub(crate) newly_created: bool,
    pub(crate) position: MovementLocalPosition,
}
struct SummonMemo {
    parent: ExactActorRef,
    index: usize,
    sequence: u64,
    binding: [u8; 32],
    result: Result<SummonReceipt, SummonError>,
}
#[derive(Default)]
pub(crate) struct NativeSummonOwner {
    memos: Vec<SummonMemo>,
}
impl NativeSummonOwner {
    pub(crate) fn execute(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        catalog: &NativeSummonCatalog,
        proposal: &ProfileSummonProposal,
        policy: &mut dyn SummonLocationPolicy,
    ) -> Result<SummonReceipt, SummonError> {
        let b = runtime.binding();
        if !fence.is_current_for_scope(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !fence.accepts_stamp(stamp)
        {
            return Err(SummonError::StaleOwner);
        }
        if catalog.content != runtime.content_pin().server_artifact_digest() {
            return Err(SummonError::InvalidSource);
        }
        if !runtime.matches_live_creature_identity(
            proposal.occurrence.actor,
            catalog.parent.key.as_bytes(),
        ) || !current_source_creature_role(runtime, proposal.occurrence.actor)
            .map(CurrentSourceCreatureRole::is_ordinary)
            .unwrap_or(false)
        {
            return Err(SummonError::StaleParent);
        }
        if runtime
            .read_actor_position(proposal.target_with_path)
            .is_err()
        {
            return Err(SummonError::StaleTarget);
        }
        let source = catalog
            .entries
            .get(proposal.entry_index)
            .ok_or(SummonError::InvalidSource)?
            .as_ref()
            .map_err(|e| *e)?;
        if proposal.creature != source.child
            || proposal.maximum_total != source.total
            || proposal.maximum_of_creature != source.count
        {
            return Err(SummonError::InvalidSource);
        }
        let binding = Sha256::digest(format!("{:?}:{proposal:?}", source).as_bytes()).into();
        self.memos
            .retain(|m| runtime.contains_live_creature(m.parent));
        let index = self
            .memos
            .iter()
            .position(|m| m.parent == proposal.occurrence.actor && m.index == source.index);
        if let Some(i) = index {
            let m = &self.memos[i];
            if proposal.occurrence.sequence < m.sequence {
                return Err(SummonError::Superseded);
            }
            if proposal.occurrence.sequence == m.sequence {
                return if binding == m.binding {
                    m.result.clone().map(|r| SummonReceipt {
                        newly_created: false,
                        ..r
                    })
                } else {
                    Err(SummonError::ReplayConflict)
                };
            }
        }
        if index.is_none() {
            if self.memos.len() >= 64 * 8 {
                return Err(SummonError::Capacity);
            }
            self.memos
                .try_reserve(1)
                .map_err(|_| SummonError::Capacity)?;
        }
        let result = self.place(runtime, fence, stamp, source, proposal, policy);
        let memo = SummonMemo {
            parent: proposal.occurrence.actor,
            index: source.index,
            sequence: proposal.occurrence.sequence,
            binding,
            result: result.clone(),
        };
        match index {
            Some(i) => self.memos[i] = memo,
            None => self.memos.push(memo),
        }
        result
    }
    fn place(
        &self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        source: &NativeSummonSpec,
        proposal: &ProfileSummonProposal,
        policy: &mut dyn SummonLocationPolicy,
    ) -> Result<SummonReceipt, SummonError> {
        if policy.current_target_reachable(
            runtime,
            proposal.occurrence.actor,
            proposal.target_with_path,
            stamp,
        ) != Some(true)
        {
            return Err(SummonError::MissingMap);
        }
        let center = runtime
            .read_actor_position(proposal.occurrence.actor)
            .map_err(SummonError::Carrier)?
            .position();
        let mut free = Vec::new();
        for (dx, dy) in [
            (0, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            let position = MovementLocalPosition {
                x: center.x.checked_add(dx).ok_or(SummonError::InvalidSource)?,
                y: center.y.checked_add(dy).ok_or(SummonError::InvalidSource)?,
                floor: center.floor,
            };
            let allowed = policy
                .current_cell_admits(
                    runtime,
                    proposal.occurrence.actor,
                    &source.child,
                    position,
                    stamp,
                )
                .ok_or(SummonError::MissingMap)?;
            if allowed && runtime.native_summon_cell_free(position) {
                if dx == 0 && dy == 0 {
                    free.clear();
                    free.push(position);
                    break;
                }
                free.push(position)
            }
        }
        if free.is_empty() {
            return Err(SummonError::NoFreeTile);
        }
        let digest = Sha256::digest(format!("summon-place:{proposal:?}").as_bytes());
        let mut id = [0; 16];
        id.copy_from_slice(&digest[..16]);
        let root = GameplayDecisionRoot::from_bytes(source.content);
        let draw = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(id),
            "SUMMON_PLACE",
            source.index as u64,
        )
        .map_err(|_| SummonError::InvalidSource)?;
        let position = free[(draw % (free.len() as u64)) as usize];
        let child = runtime
            .admit_native_summon(
                fence,
                stamp,
                proposal.occurrence.actor,
                &source.admission,
                position,
            )
            .map_err(SummonError::Carrier)?;
        Ok(SummonReceipt {
            child,
            newly_created: true,
            position,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId, crystal_timer_fixture};
    fn reference(key: &str) -> Ref {
        Ref {
            family: ProjectV2Family::Creature,
            key: key.into(),
            revision: "definition-r1".into(),
        }
    }
    pub(super) fn setup() -> (
        ChannelRuntimeV1,
        ScopeRuntimeFence,
        ExactActorRef,
        ExactActorRef,
        NativeSummonCatalog,
        ProfileSummonProposal,
    ) {
        let mut id = [0; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = 1;
        setup_world(
            WorldId::decode(&id)
                .expect("monster_summon.rs:tests:445: qualified fixture operation must succeed"),
        )
    }
    pub(super) fn setup_world(
        world: WorldId,
    ) -> (
        ChannelRuntimeV1,
        ScopeRuntimeFence,
        ExactActorRef,
        ExactActorRef,
        NativeSummonCatalog,
        ProfileSummonProposal,
    ) {
        let id = |t: u8| {
            let mut b = [0; 16];
            b[6] = 0x70;
            b[8] = 0x80;
            b[15] = t;
            b
        };
        // Explicit local physical fixture; source qualification has a separate captured native test.
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            (100, 100, 7),
        );
        let mut r = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2))
                .expect("monster_summon.rs:tests:476: qualified fixture operation must succeed"),
            NodeId::decode(&id(3))
                .expect("monster_summon.rs:tests:477: qualified fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            128,
            pin,
        )
        .expect("monster_summon.rs:tests:485: qualified fixture operation must succeed");
        let parent = reference("oteryn:creature.orc_shaman");
        let child = reference("oteryn:creature.snake");
        let a = r
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &parent.key,
                115,
            )
            .expect("monster_summon.rs:tests:498: qualified fixture operation must succeed");
        let target = r
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 105,
                    y: 100,
                    floor: 7,
                },
                "oteryn:creature.rat",
                20,
            )
            .expect("monster_summon.rs:tests:509: qualified fixture operation must succeed");
        let b = r.binding();
        let (f, _) = crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("monster_summon.rs:tests:515: qualified fixture operation must succeed");
        let spec = NativeSummonSpec {
            parent: parent.clone(),
            child: child.clone(),
            health: 15,
            index: 0,
            total: 2,
            count: 2,
            content: [1; 32],
            admission: NativeSummonAdmissionSpec::qualified(
                &parent.key,
                &child.key,
                15,
                0,
                2,
                2,
                [1; 32],
            )
            .expect("monster_summon.rs:tests:533: qualified fixture operation must succeed"),
        };
        let c = NativeSummonCatalog {
            parent,
            entries: vec![Ok(spec)],
            targets: vec![child.clone()],
            content: [1; 32],
        };
        let p = ProfileSummonProposal {
            occurrence: ThinkOccurrence {
                actor: a,
                sequence: 0,
            },
            entry_index: 0,
            revisions: crate::ability::RevisionSet::new(
                "rules-r1",
                "content-r1",
                "policy-r1",
                "definition-r1",
                "sim-r1",
            )
            .expect("monster_summon.rs:tests:554: qualified fixture operation must succeed"),
            creature: child,
            target_with_path: target,
            maximum_total: 2,
            maximum_of_creature: 2,
        };
        (r, f, a, target, c, p)
    }
    pub(super) struct Map {
        pub(super) known: bool,
        pub(super) allow: bool,
    }
    impl SummonLocationPolicy for Map {
        fn current_target_reachable(
            &mut self,
            r: &ChannelRuntimeV1,
            p: ExactActorRef,
            t: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            r.read_actor_position(p).ok()?;
            r.read_actor_position(t).ok()?;
            self.known.then_some(self.allow)
        }
        fn current_cell_admits(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: &Ref,
            _: MovementLocalPosition,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            self.known.then_some(self.allow)
        }
    }
    pub(super) fn stamp(r: &ChannelRuntimeV1, f: &mut ScopeRuntimeFence) -> RuntimeWorkStamp {
        let o = f
            .accept_input(r.binding().scope_generation())
            .expect("monster_summon.rs:tests:590: qualified fixture operation must succeed");
        f.stamp(o)
    }
    #[test]
    fn summon_replay_interleave_source_caps_and_current_generation() {
        let (mut r, mut f, a, _, c, mut p) = setup();
        let s = stamp(&r, &mut f);
        let mut owner = NativeSummonOwner::default();
        let mut map = Map {
            known: true,
            allow: true,
        };
        let one = owner
            .execute(&mut r, &f, s, &c, &p, &mut map)
            .expect("monster_summon.rs:tests:602: qualified fixture operation must succeed");
        assert!(one.newly_created);
        assert_eq!(r.native_summon_role(one.child), Ok(Some(a)));
        assert_ne!(
            one.position,
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7
            }
        );
        assert!(
            !owner
                .execute(&mut r, &f, s, &c, &p, &mut map)
                .expect("monster_summon.rs:tests:616: qualified fixture operation must succeed")
                .newly_created
        );
        assert_eq!(r.native_summon_count(a, None), 1);
        p.occurrence.sequence = 1;
        let two = owner
            .execute(&mut r, &f, s, &c, &p, &mut map)
            .expect("monster_summon.rs:tests:621: qualified fixture operation must succeed");
        assert_ne!(one.child, two.child);
        assert_eq!(r.native_summon_count(a, None), 2);
        p.occurrence.sequence = 2;
        assert_eq!(
            owner.execute(&mut r, &f, s, &c, &p, &mut map),
            Err(SummonError::Carrier(CarrierError::CapacityExceeded))
        );
        r.remove_test_actor(one.child)
            .expect("monster_summon.rs:tests:629: qualified fixture operation must succeed");
        assert_eq!(
            r.native_summon_role(one.child),
            Err(CarrierError::StaleActorGeneration)
        );
        p.occurrence.sequence = 3;
        let three = owner
            .execute(&mut r, &f, s, &c, &p, &mut map)
            .expect("monster_summon.rs:tests:635: qualified fixture operation must succeed");
        assert_ne!(one.child, three.child);
        assert_eq!(
            r.native_summon_role(one.child),
            Err(CarrierError::StaleActorGeneration)
        );
    }
    #[test]
    fn unknown_map_is_not_buffered_and_superseded_authority_never_spawns() {
        let (mut r, mut f, a, _, c, mut p) = setup();
        let s = stamp(&r, &mut f);
        let mut owner = NativeSummonOwner::default();
        let mut map = Map {
            known: false,
            allow: true,
        };
        assert_eq!(
            owner.execute(&mut r, &f, s, &c, &p, &mut map),
            Err(SummonError::MissingMap)
        );
        assert_eq!(r.native_summon_count(a, None), 0);
        map.known = true;
        assert_eq!(
            owner.execute(&mut r, &f, s, &c, &p, &mut map),
            Err(SummonError::MissingMap)
        );
        p.occurrence.sequence = 1;
        f.apply_external_grant(
            crate::foundation::ScopeOwnershipGeneration::new(2)
                .expect("monster_summon.rs:tests:662: qualified fixture operation must succeed"),
        )
        .expect("monster_summon.rs:tests:663: qualified fixture operation must succeed");
        assert_eq!(
            owner.execute(&mut r, &f, s, &c, &p, &mut map),
            Err(SummonError::StaleOwner)
        );
        assert_eq!(r.native_summon_count(a, None), 0);
    }
    #[test]
    fn parent_despawn_removes_children_without_death_and_parent_lethal_seals_each_child() {
        let (mut r, mut f, a, _, c, p) = setup();
        let s = stamp(&r, &mut f);
        let mut owner = NativeSummonOwner::default();
        let mut map = Map {
            known: true,
            allow: true,
        };
        let child = owner
            .execute(&mut r, &f, s, &c, &p, &mut map)
            .expect("monster_summon.rs:tests:681: qualified fixture operation must succeed")
            .child;
        assert_eq!(
            r.project_native_summon_death(child),
            Err(CarrierError::CommittedLethalUnavailable)
        );
        r.remove_test_actor(a)
            .expect("monster_summon.rs:tests:687: qualified fixture operation must succeed");
        assert!(!r.contains_live_creature(child));
        assert_eq!(
            r.project_native_summon_death(child),
            Err(CarrierError::StaleActorGeneration)
        );
        let (mut r, mut f, a, _, c, p) = setup();
        let s = stamp(&r, &mut f);
        let child = NativeSummonOwner::default()
            .execute(&mut r, &f, s, &c, &p, &mut map)
            .expect("monster_summon.rs:tests:697: qualified fixture operation must succeed")
            .child;
        r.commit_monster_lab_damage(a, b"parent-lethal", 115)
            .expect("monster_summon.rs:tests:700: qualified fixture operation must succeed");
        let mut d = r.borrow_combat_death();
        let receipt = d
            .committed_lethal_receipt(a)
            .expect("monster_summon.rs:tests:702: qualified fixture operation must succeed");
        d.project_committed_lethal(receipt)
            .expect("monster_summon.rs:tests:703: qualified fixture operation must succeed");
        assert!(!r.contains_live_creature(child));
        assert_eq!(r.native_summon_role(child), Ok(Some(a)));
        let death = r
            .project_native_summon_death(child)
            .expect("monster_summon.rs:tests:706: qualified fixture operation must succeed");
        assert_eq!(
            r.project_native_summon_death(child)
                .expect("monster_summon.rs:tests:707: qualified fixture operation must succeed"),
            death
        );
        let mut d = r.borrow_combat_death();
        assert_eq!(
            d.projected_death(child),
            Err(CarrierError::SummonHasNoRewards)
        );
        let receipt = d
            .committed_lethal_receipt(child)
            .expect("monster_summon.rs:tests:713: qualified fixture operation must succeed");
        assert_eq!(
            d.project_committed_lethal(receipt)
                .expect_err("monster_summon.rs:tests:715: expected guarded fixture refusal"),
            CarrierError::SummonHasNoRewards
        );
    }
    #[test]
    fn undefined_target_never_becomes_an_actor() {
        let (mut r, mut f, a, _, mut c, p) = setup();
        c.entries[0] = Err(SummonError::UndefinedTarget);
        let s = stamp(&r, &mut f);
        assert_eq!(
            NativeSummonOwner::default().execute(
                &mut r,
                &f,
                s,
                &c,
                &p,
                &mut Map {
                    known: true,
                    allow: true
                }
            ),
            Err(SummonError::UndefinedTarget)
        );
        assert_eq!(r.native_summon_count(a, None), 0);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod retained_native_source_test {
    use super::*;
    use crate::content::{ProjectEvidenceLimits, ProjectFilesystemLimits, capture_world_project};
    #[test]
    #[ignore = "requires retained frozen 11-document source project; run explicitly locally"]
    fn captured_native_catalog_qualifies_all_authored_summoners_and_spawns_source_child() {
        let retained_native_capture_path = std::env::var_os("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT")
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
            .expect("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT must explicitly name the current final native eleven-document capture");
        let path = retained_native_capture_path.as_path();
        let limits = ProjectEvidenceLimits {
            max_documents: 11,
            max_document_bytes: 96_000_000,
            max_total_bytes: 160_000_000,
            max_json_depth: 24,
            max_decoded_fields: 2_120_000,
            max_string_bytes: 43_000_000,
            max_locator_bytes: 160,
            max_locator_segments: 8,
            max_reference_records: 70000,
            max_import_records: 16,
            max_reimport_states: 404,
        };
        let project = capture_world_project(
            path.parent().expect("monster_summon.rs:retained_native_source_test:768: qualified fixture operation must succeed"),
            path.file_name().expect("monster_summon.rs:retained_native_source_test:769: qualified fixture operation must succeed"),
            ProjectFilesystemLimits {
                project: limits,
                max_entries_per_directory_scan: 32,
                max_total_directory_entries_scanned: 201,
            },
        )
        .expect("monster_summon.rs:retained_native_source_test:774: qualified fixture operation must succeed");
        let draft = project.migrate_to_v2();
        // Source membership is real captured data; this independently declared local physical pin
        // is not claimed to be a compiled Creature artifact (current Item-only compiler #162).
        let (mut runtime, mut fence, parent, _, _, mut proposal) =
            tests::setup_world(project.lower_reference_source().expect("monster_summon.rs:retained_native_source_test:781: qualified fixture operation must succeed").world_id);
        let mut count = 0;
        let mut entries = 0;
        for record in &draft.core.records {
            if let ProjectReferenceRecord::Creature {
                identity, behavior, ..
            } = record
            {
                let has_summons = draft.state.authoring_profiles.iter().any(|p| {
                    p.target.key == behavior.key
                        && p.target.revision == behavior.revision
                        && matches!(&p.data,Data::Behavior(b)if b.summons.is_some())
                });
                if has_summons {
                    let key = Ref {
                        family: ProjectV2Family::Creature,
                        key: identity.key.clone(),
                        revision: identity.revision.clone(),
                    };
                    let c =
                        NativeSummonCatalog::from_project(&runtime, &key, &draft, [1; 32]).expect("monster_summon.rs:retained_native_source_test:801: qualified fixture operation must succeed");
                    assert!(
                        c.entries.iter().all(Result::is_ok),
                        "source undefined child must remain explicit"
                    );
                    count += 1;
                    entries += c.entries.len();
                }
            }
        }
        // Exact final sealed source cohort:199 authored summoners/257 entry instances.
        // The separate aggregate child union is178 (159 ordinary +5 defense-only +14 callback-only).
        assert_eq!(count, 199);
        assert_eq!(entries, 257);
        let c = NativeSummonCatalog::from_project(&runtime, &reference(), &draft, [1; 32]).expect("monster_summon.rs:retained_native_source_test:815: qualified fixture operation must succeed");
        let source = c.entries[0].as_ref().expect("monster_summon.rs:retained_native_source_test:816: qualified fixture operation must succeed");
        proposal.creature = source.child.clone();
        proposal.maximum_total = source.total;
        proposal.maximum_of_creature = source.count;
        let s = tests::stamp(&runtime, &mut fence);
        let child = NativeSummonOwner::default()
            .execute(
                &mut runtime,
                &fence,
                s,
                &c,
                &proposal,
                &mut tests::Map {
                    known: true,
                    allow: true,
                },
            )
            .expect("monster_summon.rs:retained_native_source_test:832: qualified fixture operation must succeed")
            .child;
        assert_eq!(runtime.native_summon_role(child), Ok(Some(parent)));
        assert!(runtime.matches_live_creature_identity(child, source.child.key.as_bytes()));
        assert_eq!(runtime.crystal_router_fixture_health(child), source.health);
    }
    fn reference() -> Ref {
        Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.orc_shaman".into(),
            revision: "definition-r1".into(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod summon_retirement_regression {
    use super::*;
    #[test]
    fn seventeen_actual_child_deaths_never_block_parent_lethal_or_cleanup() {
        let (mut r, mut f, parent, _, catalog, mut p) = tests::setup();
        let s = tests::stamp(&r, &mut f);
        let mut map = tests::Map {
            known: true,
            allow: true,
        };
        let mut owner = NativeSummonOwner::default();
        let mut last = None;
        for sequence in 0..17 {
            p.occurrence.sequence = sequence;
            let child = owner
                .execute(&mut r, &f, s, &catalog, &p, &mut map)
                .expect("monster_summon.rs:summon_retirement_regression:865: qualified fixture operation must succeed")
                .child;
            r.commit_monster_lab_damage(child, b"real-child-lethal", 15)
                .expect("monster_summon.rs:summon_retirement_regression:868: qualified fixture operation must succeed");
            let death = r.project_native_summon_death(child).expect("monster_summon.rs:summon_retirement_regression:869: qualified fixture operation must succeed");
            assert_eq!(r.retire_native_summons(&f, s).expect("monster_summon.rs:summon_retirement_regression:870: qualified fixture operation must succeed"), 1);
            assert_eq!(r.project_native_summon_death(child).expect("monster_summon.rs:summon_retirement_regression:871: qualified fixture operation must succeed"), death);
            assert_eq!(
                r.native_summon_role(child),
                Err(CarrierError::StaleActorGeneration)
            );
            assert_eq!(
                r.borrow_combat_death().projected_death(child),
                Err(CarrierError::SummonHasNoRewards)
            );
            last = Some(death);
        }
        assert_eq!(r.native_summon_count(parent, None), 0);
        p.occurrence.sequence = 17;
        let live = owner
            .execute(&mut r, &f, s, &catalog, &p, &mut map)
            .expect("monster_summon.rs:summon_retirement_regression:886: qualified fixture operation must succeed")
            .child;
        r.commit_monster_lab_damage(parent, b"master-after-seventeen", 115)
            .expect("monster_summon.rs:summon_retirement_regression:889: qualified fixture operation must succeed");
        let mut d = r.borrow_combat_death();
        let receipt = d.committed_lethal_receipt(parent).expect("monster_summon.rs:summon_retirement_regression:891: qualified fixture operation must succeed");
        d.project_committed_lethal(receipt).expect("monster_summon.rs:summon_retirement_regression:892: qualified fixture operation must succeed");
        assert!(!r.contains_live_creature(live));
        assert_eq!(
            r.project_native_summon_death(last.expect("monster_summon.rs:summon_retirement_regression:895: qualified fixture operation must succeed").child).expect("monster_summon.rs:summon_retirement_regression:895: qualified fixture operation must succeed"),
            last.expect("monster_summon.rs:summon_retirement_regression:896: qualified fixture operation must succeed")
        );
        r.remove_test_actor(parent).expect("monster_summon.rs:summon_retirement_regression:898: qualified fixture operation must succeed");
        assert_eq!(
            r.native_summon_role(live),
            Err(CarrierError::StaleActorGeneration)
        );
    }
    #[test]
    fn eighty_source_children_reuse_physical_slots_without_exhausting_dead_links() {
        let (mut r, mut f, parent, _, catalog, mut p) = tests::setup();
        let s = tests::stamp(&r, &mut f);
        let mut map = tests::Map {
            known: true,
            allow: true,
        };
        let mut owner = NativeSummonOwner::default();
        for sequence in 0..80 {
            p.occurrence.sequence = sequence;
            let child = owner
                .execute(&mut r, &f, s, &catalog, &p, &mut map)
                .expect("monster_summon.rs:summon_retirement_regression:917: qualified fixture operation must succeed")
                .child;
            r.commit_monster_lab_damage(child, b"native-child-lethal", 15)
                .expect("monster_summon.rs:summon_retirement_regression:920: qualified fixture operation must succeed");
            assert_eq!(r.retire_native_summons(&f, s).expect("monster_summon.rs:summon_retirement_regression:921: qualified fixture operation must succeed"), 1);
            assert_eq!(r.retire_native_summons(&f, s).expect("monster_summon.rs:summon_retirement_regression:922: qualified fixture operation must succeed"), 0);
            assert_eq!(r.native_summon_count(parent, None), 0);
        }
        p.occurrence.sequence = 80;
        assert!(
            owner
                .execute(&mut r, &f, s, &catalog, &p, &mut map)
                .expect("monster_summon.rs:summon_retirement_regression:929: qualified fixture operation must succeed")
                .newly_created
        );
        r.remove_test_actor(parent).expect("monster_summon.rs:summon_retirement_regression:932: qualified fixture operation must succeed");
    }
}

pub(crate) fn native_draft_world_matches(
    runtime: &ChannelRuntimeV1,
    draft: &ProjectV2Draft,
) -> bool {
    let expected = runtime
        .binding()
        .world_id()
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    draft.core.world_id == expected
}
#[cfg(test)]
pub(crate) fn summon_execution_fixture_for_world(
    world: crate::foundation::WorldId,
) -> (
    ChannelRuntimeV1,
    ScopeRuntimeFence,
    ExactActorRef,
    ExactActorRef,
    NativeSummonCatalog,
    ProfileSummonProposal,
) {
    tests::setup_world(world)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod native_summon_distance_tests {
    use super::*;
    #[test]
    fn committed_parent_cardinal_moves_remove_separated_live_children_without_death() {
        let (mut r, mut f, parent, _, catalog, p) = tests::setup();
        let stamp = tests::stamp(&r, &mut f);
        let child = NativeSummonOwner::default()
            .execute(
                &mut r,
                &f,
                stamp,
                &catalog,
                &p,
                &mut tests::Map {
                    known: true,
                    allow: true,
                },
            )
            .expect("monster_summon.rs:native_summon_distance_tests:982: qualified fixture operation must succeed")
            .child;
        // Physical position primitive fixture; terrain/path production admission is separately
        // exercised by the follow consumer, never asserted by this direct owner component test.
        for _ in 0..32 {
            let at = r.read_actor_position(parent).expect("monster_summon.rs:native_summon_distance_tests:987: qualified fixture operation must succeed");
            let mut next = at.position();
            next.x += 1;
            let after = r
                .borrow_movement_position()
                .commit_cardinal(at, next)
                .expect("monster_summon.rs:native_summon_distance_tests:993: qualified fixture operation must succeed");
            assert_eq!(r.read_actor_position(parent).expect("monster_summon.rs:native_summon_distance_tests:994: qualified fixture operation must succeed"), after);
        }
        assert!(r.contains_live_creature(parent));
        assert_eq!(
            r.native_summon_role(child),
            Err(CarrierError::StaleActorGeneration)
        );
        assert_eq!(
            r.project_native_summon_death(child),
            Err(CarrierError::StaleActorGeneration)
        );
        assert_eq!(r.native_summon_count(parent, None), 0);
    }
    #[test]
    fn outward_child_self_removal_boundary_refuses_without_publishing_stale_position() {
        let (mut r, mut f, parent, _, catalog, p) = tests::setup();
        let stamp = tests::stamp(&r, &mut f);
        let child = NativeSummonOwner::default()
            .execute(
                &mut r,
                &f,
                stamp,
                &catalog,
                &p,
                &mut tests::Map {
                    known: true,
                    allow: true,
                },
            )
            .expect("monster_summon.rs:native_summon_distance_tests:1023: qualified fixture operation must succeed")
            .child;
        while r.read_actor_position(child).expect("monster_summon.rs:native_summon_distance_tests:1025: qualified fixture operation must succeed").position().x < 130 {
            let at = r.read_actor_position(child).expect("monster_summon.rs:native_summon_distance_tests:1026: qualified fixture operation must succeed");
            let mut next = at.position();
            next.x += 1;
            r.borrow_movement_position()
                .commit_cardinal(at, next)
                .expect("monster_summon.rs:native_summon_distance_tests:1031: qualified fixture operation must succeed");
        }
        let before = r.read_actor_position(child).expect("monster_summon.rs:native_summon_distance_tests:1033: qualified fixture operation must succeed");
        let mut next = before.position();
        next.x += 1;
        assert_eq!(
            r.borrow_movement_position().commit_cardinal(before, next),
            Err(CarrierError::MovementCreatureUnavailable)
        );
        assert_eq!(r.read_actor_position(child).expect("monster_summon.rs:native_summon_distance_tests:1040: qualified fixture operation must succeed"), before);
        assert_eq!(r.native_summon_role(child), Ok(Some(parent)));
    }
}

// Source-authored defense summons reuse the actual intrinsic physical admission/lifecycle.
#[derive(Debug, Clone)]
pub(crate) struct DefenseSummonSource {
    pub(crate) index: usize,
    ability: Ref,
    spec: NativeSummonSpec,
    fixed: bool,
    requested: u32,
    threshold: u32,
    offset: u16,
}
impl DefenseSummonSource {
    pub(crate) fn qualify(
        runtime: &ChannelRuntimeV1,
        parent: &Ref,
        draft: &ProjectV2Draft,
        index: usize,
        content: [u8; 32],
    ) -> Result<Self, SummonError> {
        use crate::content::{
            ProjectV2AbilityEffect, ProjectV2InlineEffectOperation as Op, ProjectV2SummonCountMode,
        };
        if !native_draft_world_matches(runtime, draft)
            || content != runtime.content_pin().server_artifact_digest()
            || !member(draft, parent)
        {
            return Err(SummonError::InvalidSource);
        }
        let b = draft
            .core
            .records
            .iter()
            .find_map(|r| match r {
                ProjectReferenceRecord::Creature {
                    identity, behavior, ..
                } if identity.key == parent.key && identity.revision == parent.revision => {
                    Some(behavior)
                }
                _ => None,
            })
            .ok_or(SummonError::InvalidSource)?;
        let Some(Data::Behavior(behavior)) = draft
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target.key == b.key && p.target.revision == b.revision)
            .map(|p| &p.data)
        else {
            return Err(SummonError::InvalidSource);
        };
        // Current qualified13 parents have no ordinary summons. Future mixed caps require qualification.
        if behavior.summons.is_some() {
            return Err(SummonError::InvalidSource);
        }
        let entry = behavior
            .defenses
            .get(index)
            .ok_or(SummonError::InvalidSource)?;
        let Some(Data::Ability(a)) = draft
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target == entry.ability)
            .map(|p| &p.data)
        else {
            return Err(SummonError::InvalidSource);
        };
        let effects = &a
            .details
            .as_ref()
            .ok_or(SummonError::InvalidSource)?
            .effects;
        let mut summon = None;
        for effect in effects {
            match effect {
                ProjectV2AbilityEffect::Inline(e) => match &e.operation {
                    Op::SummonCreature { .. } => {
                        if summon.replace(e).is_some() {
                            return Err(SummonError::InvalidSource);
                        }
                    }
                    Op::PresentationOnly => {}
                    _ => return Err(SummonError::InvalidSource),
                },
                _ => return Err(SummonError::InvalidSource),
            }
        }
        let e = summon.ok_or(SummonError::InvalidSource)?;
        let Op::SummonCreature {
            creatures,
            count_mode,
            count,
            only_below_summons,
            owned,
            max_offset_tiles,
        } = &e.operation
        else {
            return Err(SummonError::InvalidSource);
        };
        let [child] = creatures.as_slice() else {
            return Err(SummonError::InvalidSource);
        };
        if !*owned
            || *count == 0
            || *count > 16
            || *only_below_summons == 0
            || *only_below_summons > 16
            || *max_offset_tiles > 3
            || !member(draft, child)
        {
            return Err(SummonError::InvalidSource);
        }
        let Some(Data::Creature(c)) = draft
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target == *child)
            .map(|p| &p.data)
        else {
            return Err(SummonError::UndefinedTarget);
        };
        let health = c
            .health
            .and_then(|v| i64::try_from(v).ok())
            .filter(|v| *v > 0)
            .ok_or(SummonError::InvalidSource)?;
        let fixed = *count_mode == ProjectV2SummonCountMode::Fixed;
        if !fixed && *count != *only_below_summons {
            return Err(SummonError::InvalidSource);
        }
        let total = if fixed {
            (*only_below_summons - 1).saturating_add(*count).min(16)
        } else {
            *count
        };
        let admission = NativeSummonAdmissionSpec::qualified(
            &parent.key,
            &child.key,
            health,
            0,
            total,
            total,
            content,
        )
        .map_err(SummonError::Carrier)?;
        Ok(Self {
            index,
            ability: entry.ability.clone(),
            spec: NativeSummonSpec {
                parent: parent.clone(),
                child: child.clone(),
                health,
                index: 0,
                total,
                count: total,
                content,
                admission,
            },
            fixed,
            requested: *count,
            threshold: *only_below_summons,
            offset: *max_offset_tiles,
        })
    }
    pub(crate) fn child(&self) -> &Ref {
        &self.spec.child
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DefenseSummonBatch {
    pub(crate) requested: u32,
    pub(crate) children: Vec<SummonReceipt>,
    pub(crate) refused: Option<SummonError>,
    pub(crate) cap_omitted: u32,
    pub(crate) project_cap16_not_global: bool,
    pub(crate) project_current_free_cell_selection: bool,
}
struct DefenseSummonMemo {
    parent: ExactActorRef,
    ability: Ref,
    sequence: u64,
    binding: [u8; 32],
    batch: DefenseSummonBatch,
}
#[derive(Default)]
pub(crate) struct DefenseSummonOwner {
    memos: Vec<DefenseSummonMemo>,
}
impl DefenseSummonOwner {
    // Keep execute ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        source: &DefenseSummonSource,
        think: ThinkOccurrence,
        proposal: &crate::ai_think::profile_schedule::ProfileAbilityProposal,
        policy: &mut dyn SummonLocationPolicy,
    ) -> Result<DefenseSummonBatch, SummonError> {
        let b = runtime.binding();
        if !fence.is_current_for_scope(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !fence.accepts_stamp(stamp)
        {
            return Err(SummonError::StaleOwner);
        }
        if source.spec.content != runtime.content_pin().server_artifact_digest()
            || proposal.ability != source.ability
            || proposal.entry_index != source.index
            || proposal.list != crate::ai_think::profile_schedule::ScheduleList::Defence
            || proposal.issuer != think.actor
            || proposal.target != think.actor
        {
            return Err(SummonError::InvalidSource);
        }
        let expected = format!(
            "ai-profile:{}:{}:defence:{}",
            think
                .actor
                .placement_identity()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            think.sequence,
            source.index
        );
        if proposal.occurrence.id().as_str() != expected {
            return Err(SummonError::InvalidSource);
        }
        if !runtime.matches_live_creature_identity(think.actor, source.spec.parent.key.as_bytes())
            || !current_source_creature_role(runtime, think.actor)
                .map(CurrentSourceCreatureRole::is_ordinary)
                .unwrap_or(false)
        {
            return Err(SummonError::StaleParent);
        }
        self.memos
            .retain(|m| runtime.contains_live_creature(m.parent));
        let binding: [u8; 32] =
            Sha256::digest(format!("{source:?}:{proposal:?}").as_bytes()).into();
        let at = self
            .memos
            .iter()
            .position(|m| m.parent == think.actor && m.ability == source.ability);
        if let Some(i) = at {
            let m = &self.memos[i];
            if think.sequence < m.sequence {
                return Err(SummonError::Superseded);
            }
            if think.sequence == m.sequence {
                if binding != m.binding {
                    return Err(SummonError::ReplayConflict);
                }
                let mut batch = m.batch.clone();
                for c in &mut batch.children {
                    c.newly_created = false
                }
                return Ok(batch);
            }
        }
        if at.is_none() {
            if self.memos.len() >= 64 * 8 {
                return Err(SummonError::Capacity);
            }
            self.memos
                .try_reserve(1)
                .map_err(|_| SummonError::Capacity)?;
        }
        let current = runtime.native_summon_count(think.actor, None);
        let requested = if current >= source.threshold {
            0
        } else if source.fixed {
            source.requested
        } else {
            source.requested.saturating_sub(current)
        };
        let permitted = requested.min(source.spec.total.saturating_sub(current));
        let mut batch = DefenseSummonBatch {
            requested,
            children: Vec::new(),
            refused: None,
            cap_omitted: requested - permitted,
            project_cap16_not_global: source.fixed && source.threshold - 1 + source.requested > 16,
            project_current_free_cell_selection: true,
        };
        batch
            .children
            .try_reserve(permitted as usize)
            .map_err(|_| SummonError::Capacity)?;
        // One accepted source schedule proposal, no additional chance roll or timer.
        for ordinal in 0..permitted {
            match Self::place(
                runtime, fence, stamp, source, think, proposal, ordinal, policy,
            ) {
                Ok(c) => batch.children.push(c),
                Err(e) => {
                    batch.refused = Some(e);
                    break;
                }
            }
        }
        let memo = DefenseSummonMemo {
            parent: think.actor,
            ability: source.ability.clone(),
            sequence: think.sequence,
            binding,
            batch: batch.clone(),
        };
        if let Some(i) = at {
            self.memos[i] = memo
        } else {
            self.memos.push(memo)
        }
        Ok(batch)
    }
    // Keep place ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn place(
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        source: &DefenseSummonSource,
        think: ThinkOccurrence,
        proposal: &crate::ai_think::profile_schedule::ProfileAbilityProposal,
        ordinal: u32,
        policy: &mut dyn SummonLocationPolicy,
    ) -> Result<SummonReceipt, SummonError> {
        let center = runtime
            .read_actor_position(think.actor)
            .map_err(SummonError::Carrier)?
            .position();
        let radius = i32::from(source.offset.max(1));
        let mut free = Vec::new();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let p = MovementLocalPosition {
                    x: center.x.checked_add(dx).ok_or(SummonError::InvalidSource)?,
                    y: center.y.checked_add(dy).ok_or(SummonError::InvalidSource)?,
                    floor: center.floor,
                };
                let allows = policy
                    .current_cell_admits(runtime, think.actor, &source.spec.child, p, stamp)
                    .ok_or(SummonError::MissingMap)?;
                if allows && runtime.native_summon_cell_free(p) {
                    free.push(p)
                }
            }
        }
        if free.is_empty() {
            return Err(SummonError::NoFreeTile);
        }
        let hash = Sha256::digest(format!("defense-summon:{proposal:?}:{ordinal}").as_bytes());
        let mut id = [0; 16];
        id.copy_from_slice(&hash[..16]);
        let draw = deterministic_decision_u64(
            &GameplayDecisionRoot::from_bytes(source.spec.content),
            DecisionOccurrenceId::from_bytes(id),
            "DEFENSE_SUMMON_PLACE",
            ordinal as u64,
        )
        .map_err(|_| SummonError::InvalidSource)?;
        let position = free[(draw % free.len() as u64) as usize];
        let child = runtime
            .admit_native_summon(fence, stamp, think.actor, &source.spec.admission, position)
            .map_err(SummonError::Carrier)?;
        Ok(SummonReceipt {
            child,
            newly_created: true,
            position,
        })
    }
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod defense_summon_tests {
    use super::*;
    fn proposal(
        actor: ExactActorRef,
        ability: Ref,
        index: usize,
        sequence: u64,
    ) -> crate::ai_think::profile_schedule::ProfileAbilityProposal {
        let atom = format!(
            "actor:{}",
            actor
                .placement_identity()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        crate::ai_think::profile_schedule::ProfileAbilityProposal {
            issuer: actor,
            target: actor,
            ability,
            list: crate::ai_think::profile_schedule::ScheduleList::Defence,
            entry_index: index,
            magnitude: None,
            range_tiles: 0,
            occurrence: crate::ability::AbilityOccurrence::new(
                &format!(
                    "ai-profile:{}:{sequence}:defence:{index}",
                    actor
                        .placement_identity()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                ),
                crate::ability::RevisionSet::new("r", "c", "w", "f", "s").expect("monster_summon.rs:defense_summon_tests:1454: qualified fixture operation must succeed"),
            )
            .expect("monster_summon.rs:defense_summon_tests:1456: qualified fixture operation must succeed"),
            intent: crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom]).expect("monster_summon.rs:defense_summon_tests:1457: qualified fixture operation must succeed"),
        }
    }
    fn source(
        c: &NativeSummonCatalog,
        fixed: bool,
        count: u32,
        threshold: u32,
    ) -> DefenseSummonSource {
        let mut spec = c.entries[0].as_ref().expect("monster_summon.rs:defense_summon_tests:1466: qualified fixture operation must succeed").clone();
        spec.total = if fixed {
            (threshold - 1 + count).min(16)
        } else {
            count
        };
        spec.count = spec.total;
        spec.admission = NativeSummonAdmissionSpec::qualified(
            &spec.parent.key,
            &spec.child.key,
            spec.health,
            0,
            spec.total,
            spec.total,
            spec.content,
        )
        .expect(
            "monster_summon.rs:defense_summon_tests:1482: qualified fixture operation must succeed",
        );
        DefenseSummonSource {
            index: 0,
            ability: Ref {
                family: ProjectV2Family::Ability,
                key: "oteryn:ability.fixture.defense_summon".into(),
                revision: "definition-r1".into(),
            },
            spec,
            fixed,
            requested: count,
            threshold,
            offset: 3,
        }
    }
    pub(super) struct Map {
        pub(super) calls: usize,
        pub(super) missing_after: Option<usize>,
    }
    impl SummonLocationPolicy for Map {
        fn current_target_reachable(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            None
        }
        fn current_cell_admits(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: &Ref,
            _: MovementLocalPosition,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            self.calls += 1;
            if self.missing_after.is_some_and(|n| self.calls > n) {
                None
            } else {
                Some(true)
            }
        }
    }
    #[test]
    fn partial_defense_batch_keeps_first_physical_child_and_exact_replay() {
        let (mut r, mut f, parent, _, c, p) = tests::setup();
        let stamp = tests::stamp(&r, &mut f);
        let src = source(&c, false, 4, 4);
        let prepared = proposal(parent, src.ability.clone(), 0, p.occurrence.sequence);
        let mut owner = DefenseSummonOwner::default();
        let mut map = Map {
            calls: 0,
            missing_after: Some(49),
        };
        let first = owner
            .execute(&mut r, &f, stamp, &src, p.occurrence, &prepared, &mut map)
            .expect("monster_summon.rs:defense_summon_tests:1540: qualified fixture operation must succeed");
        assert_eq!(first.requested, 4);
        assert_eq!(first.children.len(), 1);
        assert_eq!(first.refused, Some(SummonError::MissingMap));
        assert_eq!(
            r.native_summon_role(first.children[0].child),
            Ok(Some(parent))
        );
        map.missing_after = None;
        let retry = owner
            .execute(&mut r, &f, stamp, &src, p.occurrence, &prepared, &mut map)
            .expect("monster_summon.rs:defense_summon_tests:1551: qualified fixture operation must succeed");
        assert_eq!(retry.children.len(), 1);
        assert!(!retry.children[0].newly_created);
        assert_eq!(r.native_summon_count(parent, None), 1);
        r.remove_test_actor(parent).expect(
            "monster_summon.rs:defense_summon_tests:1555: qualified fixture operation must succeed",
        );
        assert!(!r.contains_live_creature(first.children[0].child));
    }
    #[test]
    fn source_fixed_ten_honestly_reports_cap16_partial_at_nine_and_refuses_stale_fence() {
        let (mut r, mut f, parent, _, c, mut p) = tests::setup();
        let stamp = tests::stamp(&r, &mut f);
        let src = source(&c, true, 10, 10);
        let mut ordinary = NativeSummonOwner::default();
        let mut initial = c;
        for e in &mut initial.entries {
            let spec = e.as_mut().expect("monster_summon.rs:defense_summon_tests:1566: qualified fixture operation must succeed");
            spec.total = 16;
            spec.count = 16;
            spec.admission = NativeSummonAdmissionSpec::qualified(
                &spec.parent.key,
                &spec.child.key,
                spec.health,
                0,
                16,
                16,
                spec.content,
            )
            .expect("monster_summon.rs:defense_summon_tests:1578: qualified fixture operation must succeed");
        }
        p.maximum_total = 16;
        p.maximum_of_creature = 16;
        // Real existing native admission, spaced physical positions; no parallel child tracker.
        for ordinal in 0..9 {
            let pos = MovementLocalPosition {
                x: 90 + ordinal,
                y: 90,
                floor: 7,
            };
            r.admit_native_summon(
                &f,
                stamp,
                parent,
                &initial.entries[0].as_ref().expect("monster_summon.rs:defense_summon_tests:1593: qualified fixture operation must succeed").admission,
                pos,
            )
            .expect("monster_summon.rs:defense_summon_tests:1596: qualified fixture operation must succeed");
        }
        let prepared = proposal(parent, src.ability.clone(), 0, p.occurrence.sequence);
        let mut owner = DefenseSummonOwner::default();
        let batch = owner
            .execute(
                &mut r,
                &f,
                stamp,
                &src,
                p.occurrence,
                &prepared,
                &mut Map {
                    calls: 0,
                    missing_after: None,
                },
            )
            .expect("monster_summon.rs:defense_summon_tests:1613: qualified fixture operation must succeed");
        assert_eq!(batch.requested, 10);
        assert_eq!(batch.children.len(), 7);
        assert_eq!(batch.cap_omitted, 3);
        assert!(batch.project_cap16_not_global);
        assert_eq!(r.native_summon_count(parent, None), 16);
        f.apply_external_grant(crate::foundation::ScopeOwnershipGeneration::new(2).expect(
            "monster_summon.rs:defense_summon_tests:1619: qualified fixture operation must succeed",
        ))
        .expect(
            "monster_summon.rs:defense_summon_tests:1620: qualified fixture operation must succeed",
        );
        assert_eq!(
            owner.execute(
                &mut r,
                &f,
                stamp,
                &src,
                p.occurrence,
                &prepared,
                &mut Map {
                    calls: 0,
                    missing_after: None
                }
            ),
            Err(SummonError::StaleOwner)
        );
        let _ = &mut ordinary;
    }
    #[test]
    fn unknown_current_map_never_creates_buffered_defense_summons() {
        let (mut r, mut f, parent, _, c, p) = tests::setup();
        let stamp = tests::stamp(&r, &mut f);
        let src = source(&c, false, 2, 2);
        let prepared = proposal(parent, src.ability.clone(), 0, p.occurrence.sequence);
        let mut owner = DefenseSummonOwner::default();
        let batch = owner
            .execute(
                &mut r,
                &f,
                stamp,
                &src,
                p.occurrence,
                &prepared,
                &mut Map {
                    calls: 0,
                    missing_after: Some(0),
                },
            )
            .expect("monster_summon.rs:defense_summon_tests:1658: qualified fixture operation must succeed");
        assert_eq!(batch.children.len(), 0);
        assert_eq!(batch.refused, Some(SummonError::MissingMap));
        assert_eq!(
            owner
                .execute(
                    &mut r,
                    &f,
                    stamp,
                    &src,
                    p.occurrence,
                    &prepared,
                    &mut Map {
                        calls: 0,
                        missing_after: None
                    }
                )
                .expect("monster_summon.rs:defense_summon_tests:1675: qualified fixture operation must succeed"),
            batch
        );
        assert_eq!(r.native_summon_count(parent, None), 0);
    }
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod captured_defense_summon_test {
    use super::*;
    use crate::content::*;
    #[test]
    #[ignore = "requires retained captured11file native source; explicit local source/physical proof"]
    fn captured_all13_defense_instances_qualify_and_actual_source_chance_spawns_current_child() {
        let retained_native_capture_path = std::env::var_os("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT")
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
            .expect("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT must explicitly name the current final native eleven-document capture");
        let path = retained_native_capture_path.as_path();
        let limits = ProjectEvidenceLimits {
            max_documents: 11,
            max_document_bytes: 96_000_000,
            max_total_bytes: 160_000_000,
            max_json_depth: 24,
            max_decoded_fields: 2_120_000,
            max_string_bytes: 43_000_000,
            max_locator_bytes: 160,
            max_locator_segments: 8,
            max_reference_records: 70000,
            max_import_records: 16,
            max_reimport_states: 404,
        };
        let project = capture_world_project(
            path.parent().expect("monster_summon.rs:captured_defense_summon_test:1707: qualified fixture operation must succeed"),
            path.file_name().expect("monster_summon.rs:captured_defense_summon_test:1708: qualified fixture operation must succeed"),
            ProjectFilesystemLimits {
                project: limits,
                max_entries_per_directory_scan: 32,
                max_total_directory_entries_scanned: 201,
            },
        )
        .expect("monster_summon.rs:captured_defense_summon_test:1713: qualified fixture operation must succeed");
        let draft = project.migrate_to_v2();
        let (mut r, mut f, old, _, _, _) =
            tests::setup_world(project.lower_reference_source().expect("monster_summon.rs:captured_defense_summon_test:1718: qualified fixture operation must succeed").world_id);
        let mut count = 0;
        for record in &draft.core.records {
            if let ProjectReferenceRecord::Creature {
                identity, behavior, ..
            } = record
            {
                let parent = Ref {
                    family: ProjectV2Family::Creature,
                    key: identity.key.clone(),
                    revision: identity.revision.clone(),
                };
                if let Some(Data::Behavior(b)) = draft
                    .state
                    .authoring_profiles
                    .iter()
                    .find(|p| {
                        p.target.key == behavior.key && p.target.revision == behavior.revision
                    })
                    .map(|p| &p.data)
                {
                    for (i, entry) in b.defenses.iter().enumerate() {
                        let is_summon=draft.state.authoring_profiles.iter().find(|p|p.target==entry.ability).is_some_and(|p|matches!(&p.data,Data::Ability(a)if a.details.as_ref().is_some_and(|d|d.effects.iter().any(|e|matches!(e,ProjectV2AbilityEffect::Inline(e)if matches!(e.operation,ProjectV2InlineEffectOperation::SummonCreature{..}))))));
                        if is_summon {
                            DefenseSummonSource::qualify(&r, &parent, &draft, i, [1; 32]).expect("monster_summon.rs:captured_defense_summon_test:1742: qualified fixture operation must succeed");
                            count += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(count, 13);
        r.remove_test_actor(old).expect("monster_summon.rs:captured_defense_summon_test:1750: qualified fixture operation must succeed");
        let creature = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.white_pale".into(),
            revision: "definition-r1".into(),
        };
        let parent = r
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &creature.key,
                100,
            )
            .expect("monster_summon.rs:captured_defense_summon_test:1765: qualified fixture operation must succeed");
        let source = DefenseSummonSource::qualify(&r, &creature, &draft, 0, [1; 32]).expect("monster_summon.rs:captured_defense_summon_test:1767: qualified fixture operation must succeed");
        let record = draft
            .core
            .records
            .iter()
            .find_map(|r| match r {
                ProjectReferenceRecord::Creature {
                    identity, behavior, ..
                } if identity.key == creature.key => Some(behavior),
                _ => None,
            })
            .expect("monster_summon.rs:captured_defense_summon_test:1777: qualified fixture operation must succeed");
        let behavior = match &draft
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target.key == record.key)
            .expect("monster_summon.rs:captured_defense_summon_test:1783: qualified fixture operation must succeed")
            .data { Data::Behavior(behavior) => Some(behavior), _ => None }.expect("actual source Creature must reference Behavior authoring");
        let abilities = draft
            .state
            .authoring_profiles
            .iter()
            .filter_map(|p| match &p.data {
                Data::Ability(a) => Some((p.target.clone(), a.clone())),
                _ => None,
            })
            .collect();
        let revisions = crate::ability::RevisionSet::new("r", "c", "w", "f", "s").expect("monster_summon.rs:captured_defense_summon_test:1798: qualified fixture operation must succeed");
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let mut schedule = crate::ai_think::profile_schedule::ProfileScheduleState::new(parent);
        let mut actual = None;
        for sequence in 0..1000 {
            let think = ThinkOccurrence {
                actor: parent,
                sequence,
            };
            let plan = schedule
                .prepare(think, behavior, &abilities, None, &revisions, &root)
                .expect("monster_summon.rs:captured_defense_summon_test:1808: qualified fixture operation must succeed");
            if let Some(proposal) = plan
                .proposals
                .into_iter()
                .find(|p| p.ability == source.ability)
            {
                actual = Some((think, proposal));
                break;
            }
        }
        let (think, proposal) = actual.expect("actual source chance prepares defense");
        let stamp = tests::stamp(&r, &mut f);
        let batch = DefenseSummonOwner::default()
            .execute(
                &mut r,
                &f,
                stamp,
                &source,
                think,
                &proposal,
                &mut defense_summon_tests::Map {
                    calls: 0,
                    missing_after: None,
                },
            )
            .expect("monster_summon.rs:captured_defense_summon_test:1833: qualified fixture operation must succeed");
        assert_eq!(batch.requested, 2);
        assert_eq!(batch.children.len(), 2);
        for child in batch.children {
            assert_eq!(r.native_summon_role(child.child), Ok(Some(parent)));
            assert!(r.matches_live_creature_identity(child.child, source.child().key.as_bytes()));
            assert_eq!(
                r.crystal_router_fixture_health(child.child),
                source.spec.health
            );
        }
    }
}

/// Physical native roles remain owned by Foundation. This reconciles the source
/// monster-parent relationship with95's independently typed player companion role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurrentSourceCreatureRole {
    Ordinary,
    IntrinsicMonsterSummon(ExactActorRef),
    PlayerCompanion(ExactActorRef, crate::foundation::GameSessionId),
    Familiar,
}
impl CurrentSourceCreatureRole {
    pub(crate) fn intrinsic_master(self) -> Option<ExactActorRef> {
        match self {
            Self::IntrinsicMonsterSummon(master) => Some(master),
            _ => None,
        }
    }
    pub(crate) fn is_ordinary(self) -> bool {
        matches!(self, Self::Ordinary)
    }
}
pub(crate) fn current_source_creature_role(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
) -> Result<CurrentSourceCreatureRole, CarrierError> {
    // Exact slot identity/generation is independently validated by native owner.
    let intrinsic = runtime.native_summon_role(actor)?;
    match runtime.companion_snapshot_including_dead(actor) {
        Ok(snapshot) => {
            let player_owned = snapshot.state.master;
            if intrinsic.is_some() && (player_owned.is_some() || snapshot.state.policy.is_familiar)
            {
                return Err(CarrierError::PlanConflict);
            }
            if snapshot.state.policy.is_familiar {
                return Ok(CurrentSourceCreatureRole::Familiar);
            }
            if let Some(master) = player_owned {
                return Ok(CurrentSourceCreatureRole::PlayerCompanion(
                    master.actor,
                    master.session,
                ));
            }
            Ok(intrinsic
                .map(CurrentSourceCreatureRole::IntrinsicMonsterSummon)
                .unwrap_or(CurrentSourceCreatureRole::Ordinary))
        }
        // Legacy qualified monster-parent admission lacks a main player-companion payload.
        // This fallback is allowed only AFTER the exact current native role proved the slot.
        Err(CarrierError::NotCreature) => Ok(intrinsic
            .map(CurrentSourceCreatureRole::IntrinsicMonsterSummon)
            .unwrap_or(CurrentSourceCreatureRole::Ordinary)),
        Err(error) => Err(error),
    }
}

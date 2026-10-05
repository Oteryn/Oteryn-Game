//! Conditions consume the sole native actor-slot store. No transport condition collection.
//! Runtime lock precedes the existing ChannelSpellStates HP lock at every composition call.
use super::*;
use crate::foundation::{
    ActorConditionPlan, ActorConditionTransition, ApplicationFacts, AttributeModifiers,
    ConditionDefinition, ConditionOwnerError, ConditionSource, ConditionType, RuntimeScopeRefV1,
    RuntimeWorkStamp, ScopeRuntimeFence, StatusKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeConditionError {
    Owner(ConditionOwnerError),
    StaleOwner,
    StaleActor,
    ContentChanged,
    CombatRefused,
}
/// Independently current target/tile Combat facts from the existing map/Combat owner.
/// Missing policy refuses before preparing a native condition; no permissive default.
pub(crate) trait NativeConditionAdmission {
    fn current_allowed(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        source: ConditionSource,
    ) -> Option<bool>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pin {
    world: crate::foundation::WorldId,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    movement: crate::foundation::MovementPositionContext,
}
impl Pin {
    fn current(runtime: &ChannelRuntimeV1) -> Self {
        let p = runtime.content_pin();
        Self {
            world: p.world_id(),
            activation: p.activation_sequence(),
            server: p.server_artifact_digest(),
            client: p.client_artifact_digest(),
            frame: p.frame_binding_digest(),
            movement: runtime.pinned_movement_context(),
        }
    }
}
fn current_owner(
    runtime: &ChannelRuntimeV1,
    fence: &ScopeRuntimeFence,
    stamp: RuntimeWorkStamp,
) -> bool {
    let binding = runtime.binding();
    fence.is_current_for_scope(
        RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        binding.scope_generation(),
    ) && fence.accepts_stamp(stamp)
}
/// Immutable source-qualified value projection, not actor or content authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeConditionApplication {
    plan: ActorConditionPlan,
    pin: Pin,
    stamp: RuntimeWorkStamp,
    target: ExactActorRef,
    session: Option<GameSessionId>,
    source: ConditionSource,
}
impl NativeConditionApplication {
    pub(crate) fn native_plan(&self) -> &ActorConditionPlan {
        &self.plan
    }
}
fn creature_current(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> bool {
    runtime.contains_live_creature(actor)
        && runtime
            .read_actor_position(actor)
            .is_ok_and(|p| p.context() == runtime.pinned_movement_context())
}
impl ChannelSpellStates {
    fn condition_player_current(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        !self.is_dead(actor)
            && self
                .get(runtime, actor, session)
                .is_some_and(|s| s.vitals().health > 0)
            && runtime
                .player_control_facts(actor, session)
                .is_ok_and(|f| f.control_loss.is_none())
    }
    fn condition_actor_current(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: Option<GameSessionId>,
    ) -> bool {
        match session {
            Some(session) => self.condition_player_current(runtime, actor, session),
            None => creature_current(runtime, actor),
        }
    }
    /// Qualified source definitions are resolved by the existing loaded-catalog consumer.
    /// This adapter does not grant a raw network payload a definition or Combat authority.
    // Keep prepare_native_conditions ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_native_conditions(
        &self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        target: ExactActorRef,
        session: Option<GameSessionId>,
        source: ConditionSource,
        definitions: &[ConditionDefinition],
        immunities: &[ConditionType],
        facts: ApplicationFacts<'_>,
        policy: &mut impl NativeConditionAdmission,
    ) -> Result<NativeConditionApplication, NativeConditionError> {
        if session.is_some() {
            return Err(NativeConditionError::CombatRefused);
        }
        if !current_owner(runtime, fence, stamp) {
            return Err(NativeConditionError::StaleOwner);
        }
        if !self.condition_actor_current(runtime, target, session)
            || !self.condition_actor_current(runtime, source.actor, source.session)
        {
            return Err(NativeConditionError::StaleActor);
        }
        if policy.current_allowed(runtime, target, source) != Some(true) {
            return Err(NativeConditionError::CombatRefused);
        }
        let plan = runtime
            .prepare_actor_condition(
                target,
                session,
                ActorConditionTransition::ApplyBatch {
                    definitions,
                    source,
                    immunities,
                    facts,
                },
                SemanticTimeMicros::from_micros(facts.now),
            )
            .map_err(NativeConditionError::Owner)?;
        Ok(NativeConditionApplication {
            plan,
            pin: Pin::current(runtime),
            stamp,
            target,
            session,
            source,
        })
    }
    /// A source-qualified concrete cure, staged beside a spell HP/mana projection.
    // Keep prepare_native_type_cure ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_native_type_cure(
        &self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        target: ExactActorRef,
        session: Option<GameSessionId>,
        source: ConditionSource,
        kind: ConditionType,
        facts: ApplicationFacts<'_>,
        policy: &mut impl NativeConditionAdmission,
    ) -> Result<NativeConditionApplication, NativeConditionError> {
        if session.is_some() {
            return Err(NativeConditionError::CombatRefused);
        }
        if !current_owner(runtime, fence, stamp) {
            return Err(NativeConditionError::StaleOwner);
        }
        if !self.condition_actor_current(runtime, target, session)
            || !self.condition_actor_current(runtime, source.actor, source.session)
        {
            return Err(NativeConditionError::StaleActor);
        }
        if policy.current_allowed(runtime, target, source) != Some(true) {
            return Err(NativeConditionError::CombatRefused);
        }
        let plan = runtime
            .prepare_actor_condition(
                target,
                session,
                ActorConditionTransition::RemoveType {
                    kind,
                    source,
                    facts,
                },
                SemanticTimeMicros::from_micros(facts.now),
            )
            .map_err(NativeConditionError::Owner)?;
        Ok(NativeConditionApplication {
            plan,
            pin: Pin::current(runtime),
            stamp,
            target,
            session,
            source,
        })
    }
    /// Reusable current facts check for the atomic HP/conditions consumer. Native slot plan
    /// preimage validation and infallible joint publication remain that consumer's job.
    pub(crate) fn validate_native_conditions(
        &self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        application: &NativeConditionApplication,
        policy: &mut impl NativeConditionAdmission,
    ) -> Result<(), NativeConditionError> {
        if stamp != application.stamp || !current_owner(runtime, fence, stamp) {
            return Err(NativeConditionError::StaleOwner);
        }
        if Pin::current(runtime) != application.pin {
            return Err(NativeConditionError::ContentChanged);
        }
        if !self.condition_actor_current(runtime, application.target, application.session)
            || !self.condition_actor_current(
                runtime,
                application.source.actor,
                application.source.session,
            )
        {
            return Err(NativeConditionError::StaleActor);
        }
        if policy.current_allowed(runtime, application.target, application.source) != Some(true) {
            return Err(NativeConditionError::CombatRefused);
        }
        Ok(())
    }
    /// Conditions-only publication. A combined damage/conditions effect uses the native atomic
    /// HP bridge with `native_plan`, never HP followed by this independently fallible method.
    pub(crate) fn commit_native_conditions(
        &self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        application: &NativeConditionApplication,
        policy: &mut impl NativeConditionAdmission,
        now: SemanticTimeMicros,
    ) -> Result<bool, NativeConditionError> {
        self.validate_native_conditions(runtime, fence, stamp, application, policy)?;
        runtime
            .commit_actor_condition(&application.plan, now)
            .map_err(NativeConditionError::Owner)
    }
    pub(crate) fn native_condition_movement_allowed(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> bool {
        if !self.condition_player_current(runtime, target, session) {
            return false;
        }
        let Some(state) = self.get(runtime, target, session) else {
            return false;
        };
        let store = state.owned_conditions();
        store.accepts_time(now.get())
            && !store.has_status(crate::ability::condition::StatusKind::Rooted, now.get())
            && !store.has_status(crate::ability::condition::StatusKind::Feared, now.get())
    }
    pub(crate) fn native_effective_attributes(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<AttributeModifiers> {
        if !self.condition_player_current(runtime, target, session) {
            return None;
        }
        let store = self.get(runtime, target, session)?.owned_conditions();
        if !store.accepts_time(now.get()) {
            return None;
        }
        let values = store.effective_attributes(now.get());
        let map = |value: Option<crate::ability::condition::AttributeModifier>| {
            value.map(|v| match v {
                crate::ability::condition::AttributeModifier::Add(n) => {
                    crate::foundation::AttributeModifier::Add(n)
                }
                crate::ability::condition::AttributeModifier::PercentOfBase(n) => {
                    crate::foundation::AttributeModifier::PercentOfBase(n)
                }
            })
        };
        Some(AttributeModifiers {
            melee: map(values.melee),
            distance: map(values.distance),
            magic_points: map(values.magic_points),
        })
    }
    pub(crate) fn native_actor_invisible(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: Option<GameSessionId>,
        now: SemanticTimeMicros,
    ) -> Option<bool> {
        if !self.condition_actor_current(runtime, actor, session) {
            return None;
        }
        match session {
            Some(session) => self
                .get(runtime, actor, session)?
                .owned_invisible_at(now.get())
                .ok(),
            None => {
                runtime.actor_active_speed_delta(actor, None, now).ok()?;
                let store = runtime.actor_conditions(actor, None).ok()?;
                Some(
                    store.invisible_at(now.get())
                        || store.has_status(StatusKind::Invisible, now.get()),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::foundation::{ConditionSourceKind, ConditionValues};
    #[test]
    fn native_self_use_creature_requires_exact_self_target() {
        let (mut runtime, player, session) = super::super::tests::runtime_with_player(0x7b);
        let creature = runtime
            .admit_test_creature(crate::foundation::MovementLocalPosition {
                x: 0,
                y: 0,
                floor: 0,
            })
            .unwrap();
        let root = GameplayDecisionRoot::from_bytes([10; 32]);
        let defs = [ConditionDefinition::new(
            "source.self.invisible",
            1,
            ConditionValues::TimedStatus {
                kind: StatusKind::Invisible,
                duration_ms: 1000,
            },
        )
        .unwrap()];
        let source = ConditionSource {
            actor: creature,
            session: None,
            kind: ConditionSourceKind::SelfUse,
        };
        let mut facts = ApplicationFacts {
            now: 0,
            base_speed: 110,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: false,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([98; 16]),
        };
        let plan = runtime
            .prepare_actor_condition(
                creature,
                None,
                ActorConditionTransition::ApplyBatch {
                    definitions: &defs,
                    source,
                    immunities: &[],
                    facts,
                },
                SemanticTimeMicros::from_micros(0),
            )
            .unwrap();
        runtime
            .commit_actor_condition(&plan, SemanticTimeMicros::from_micros(0))
            .unwrap();
        assert!(
            runtime
                .actor_conditions(creature, None)
                .unwrap()
                .has_status(StatusKind::Invisible, 0)
        );
        facts.target_is_player = true;
        facts.occurrence = DecisionOccurrenceId::from_bytes([99; 16]);
        assert_eq!(
            runtime.prepare_actor_condition(
                player,
                Some(session),
                ActorConditionTransition::ApplyBatch {
                    definitions: &defs,
                    source,
                    immunities: &[],
                    facts
                },
                SemanticTimeMicros::from_micros(0)
            ),
            Err(ConditionOwnerError::FactsMismatch)
        );
        assert!(
            runtime
                .actor_conditions(player, Some(session))
                .unwrap()
                .instances()
                .is_empty()
        );
    }
}

impl ChannelSpellStates {
    // Keep commit_source_area_heal_and_paralysis_removal ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn commit_source_area_heal_and_paralysis_removal(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        content: [u8; 32],
        heal: &[(ExactActorRef, String, u64, u64)],
        cure: &[ExactActorRef],
        now: u64,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> Result<Vec<crate::foundation::OwnerDamageResult>, crate::foundation::CarrierError> {
        if !current_owner(runtime, fence, stamp) {
            return Err(crate::foundation::CarrierError::WrongScope);
        }
        runtime.commit_source_creature_heal_and_cure(content, heal, cure, now)
    }
    // Keep commit_self_heal_and_paralysis_removal ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn commit_self_heal_and_paralysis_removal(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        ledger: &mut crate::foundation::CreatureSelfHealLedger,
        registration: &crate::foundation::CreatureSelfHealRegistration,
        actor: ExactActorRef,
        sequence: u64,
        draw: i64,
        now: u64,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> Result<crate::foundation::OwnerDamageResult, crate::foundation::CarrierError> {
        if !current_owner(runtime, fence, stamp) {
            return Err(crate::foundation::CarrierError::WrongScope);
        }
        runtime.commit_creature_self_heal_and_cure(ledger, registration, actor, sequence, draw, now)
    }
}

/// One latest source receipt per current player, bounded by native player admission.
/// No instance/store is held here: this is source input identity and an HP/no-HP receipt.
pub(super) struct NativeSecondaryMemo {
    pub(super) actor: ExactActorRef,
    pub(super) session: GameSessionId,
    pub(super) occurrence: String,
    binding: [u8; 32],
    root: GameplayDecisionRoot,
    receipt: Option<crate::player_lethal::PlayerDamageReceipt>,
}
impl std::fmt::Debug for NativeSecondaryMemo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeSecondaryMemo")
            .field("actor", &self.actor)
            .field("receipt", &self.receipt)
            .finish_non_exhaustive()
    }
}
impl ChannelSpellStates {
    // Keep native_composite_conditions ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn native_composite_conditions(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        occurrence: &str,
        source: ExactActorRef,
        definitions: &[ConditionDefinition],
        facts: &ApplicationFacts<'_>,
        immunities: &[ConditionType],
        magnitude: Option<u32>,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> Option<Option<crate::player_lethal::PlayerDamageReceipt>> {
        use crate::player_lethal::{PlayerDamageReceipt, PlayerLethalReceipt};
        if !current_owner(runtime, fence, stamp)
            || !creature_current(runtime, source)
            || definitions.is_empty()
            || definitions.len() > 16
            || !facts.target_is_player
        {
            return None;
        }
        let text = Self::source_occurrence_text(occurrence)?;
        // Source definitions/Combat admission facts come from this same owning source cast;
        // the existing source caller has already validated current loaded catalog and tile.
        let binding: [u8; 32] = Sha256::digest(
            format!(
                "{:?}:{:?}:{:?}:{:?}:{:?}:{:?}",
                Pin::current(runtime),
                source,
                definitions,
                (
                    facts.now,
                    facts.base_speed,
                    facts.mana_shield_capacity,
                    facts.target_reentry_protected,
                    facts.source_reentry_protected,
                    facts.target_is_player,
                    facts.occurrence
                ),
                immunities,
                magnitude
            )
            .as_bytes(),
        )
        .into();
        self.source_secondary_memos
            .retain(|m| runtime.player_control_facts(m.actor, m.session).is_ok());
        let old = self
            .source_secondary_memos
            .iter()
            .position(|m| m.actor == target && m.session == session);
        if let Some(i) = old {
            let m = &self.source_secondary_memos[i];
            if m.occurrence == occurrence {
                return (m.binding == binding && m.root == *facts.decision_root)
                    .then_some(m.receipt);
            }
        }
        if !self.condition_player_current(runtime, target, session)
            || self.has_pending_spell_commit(target, session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
        {
            return None;
        }
        // A primary-only receipt must never acquire secondary effects on retry.
        if self
            .source_damage
            .iter()
            .any(|m| m.actor == target && m.session == session && m.occurrence == occurrence)
        {
            return None;
        }
        if old.is_none() {
            self.source_secondary_memos.try_reserve(1).ok()?;
        }
        let damage_text = magnitude.map(|_| text.clone());
        let damage_old = self
            .source_damage
            .iter()
            .position(|m| m.actor == target && m.session == session);
        if magnitude.is_some() && damage_old.is_none() {
            self.source_damage.try_reserve(1).ok()?;
        }
        let before = self.get(runtime, target, session)?;
        let mut next = before.clone();
        let mut primary = None;
        if let Some(amount) = magnitude {
            let (successor, damage) =
                crate::spell::actor_conditions::stage_creature_hit(&next, amount, facts.now)
                    .ok()?;
            next = successor;
            primary = Some(damage);
        }
        if next.vitals().health > 0 {
            let label = format!("creature:{}", super::hex(&source.placement_identity()));
            next = next.stage_source_player_conditions(label, definitions, immunities, facts)?;
        }
        let changed = next != *before;
        let revision = if changed {
            before.revision().checked_add(1)?
        } else {
            before.revision()
        };
        next.source_set_batch_revision(revision);
        let death = if next.vitals().health == 0 {
            let cell = runtime.read_actor_position(target).ok()?.position();
            self.deaths.try_reserve(1).ok()?;
            Some(PlayerDeath {
                occurrence: (self.mint_death)()?,
                cell,
            })
        } else {
            None
        };
        let receipt = primary.map(|damage| PlayerDamageReceipt {
            applied: damage.applied,
            health_after: damage.health_after,
            vitals_revision: revision,
            death: death.map(|d| {
                PlayerLethalReceipt::from_native_commit(
                    runtime,
                    target,
                    session,
                    revision,
                    *d.occurrence.as_bytes(),
                    d.cell,
                )
            }),
        });
        if changed && !self.commit(runtime, target, session, next) {
            return None;
        }
        if let Some(death) = death {
            self.deaths.push((target, session, death));
        }
        if let (Some(magnitude), Some(receipt), Some(damage_text)) =
            (magnitude, receipt, damage_text)
        {
            let memo = NativeSourceMemo {
                actor: target,
                session,
                occurrence: damage_text,
                magnitude,
                receipt,
            };
            if let Some(i) = damage_old {
                self.source_damage[i] = memo
            } else {
                self.source_damage.push(memo)
            }
        }
        let memo = NativeSecondaryMemo {
            actor: target,
            session,
            occurrence: text,
            binding,
            root: facts.decision_root.clone(),
            receipt,
        };
        if let Some(i) = old {
            self.source_secondary_memos[i] = memo
        } else {
            self.source_secondary_memos.push(memo)
        }
        Some(receipt)
    }
    // Keep native_remove_attack_invisibility ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn native_remove_attack_invisibility(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        source: ExactActorRef,
        now: u64,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> bool {
        if !current_owner(runtime, fence, stamp)
            || !creature_current(runtime, source)
            || !self.condition_player_current(runtime, target, session)
            || self.has_pending_spell_commit(target, session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
        {
            return false;
        }
        // Exact accepted work stamp plus target/source identifies this already admitted
        // secondary-only owner action. No fake HP hit or fabricated ApplicationFacts.
        let Some(before) = self.get(runtime, target, session) else {
            return false;
        };
        let Some(next) = before.stage_source_remove_invisibility(now) else {
            return false;
        };
        if next.revision() == before.revision() {
            return true;
        }
        self.commit(runtime, target, session, next)
    }
}

#[cfg(test)]
mod native_composite_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::foundation::{ConditionValues, ScopeOwnershipGeneration};
    use crate::player_lethal::PlayerLethalVitals;
    fn ready(
        tag: u8,
    ) -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        GameSessionId,
        ExactActorRef,
        ScopeRuntimeFence,
        RuntimeWorkStamp,
    ) {
        let (mut r, a, g) = super::super::tests::runtime_with_player(tag);
        r.initialize_first_entry_position(a).unwrap();
        let origin = r.read_actor_position(a).unwrap().position();
        let caster = r
            .admit_source_pinned_lab_creature(
                crate::foundation::MovementLocalPosition {
                    x: origin.x + 1,
                    y: origin.y,
                    floor: origin.floor,
                },
                "test:creature",
                20,
            )
            .unwrap();
        assert_eq!(
            r.read_actor_position(a).unwrap().context(),
            r.pinned_movement_context()
        );
        assert_eq!(
            r.read_actor_position(caster).unwrap().context(),
            r.pinned_movement_context()
        );
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &r,
                a,
                g,
                super::super::tests::FACTS,
                (0, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .unwrap();
        let b = r.binding();
        let (fence, stamp) = crate::foundation::crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .unwrap();
        (r, states, a, g, caster, fence, stamp)
    }
    fn facts(root: &GameplayDecisionRoot) -> ApplicationFacts<'_> {
        ApplicationFacts {
            now: 0,
            base_speed: 110,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: root,
            occurrence: DecisionOccurrenceId::from_bytes([102; 16]),
        }
    }
    fn definitions() -> [ConditionDefinition; 1] {
        [ConditionDefinition::new(
            "native.source.invisible",
            1,
            ConditionValues::TimedStatus {
                kind: StatusKind::Invisible,
                duration_ms: 1000,
            },
        )
        .unwrap()]
    }
    #[test]
    fn composite_failed_native_mint_keeps_hp_and_slot_then_native_lethal_clears_it() {
        let (mut r, mut s, a, g, c, fence, stamp) = ready(0x7c);
        let root = GameplayDecisionRoot::from_bytes([11; 32]);
        let defs = definitions();
        let before = r.actor_conditions(a, Some(g)).unwrap().clone();
        let hp = s.get(&r, a, g).cloned();
        let mint = s.mint_death;
        s.mint_death = || None;
        assert_eq!(
            s.apply_composite_attack_damage(
                &mut r,
                a,
                g,
                185,
                "composite:0",
                c,
                &defs,
                &facts(&root),
                &[],
                &fence,
                stamp
            ),
            None
        );
        assert_eq!(r.actor_conditions(a, Some(g)).unwrap(), &before);
        assert_eq!(s.get(&r, a, g).cloned(), hp);
        assert!(s.source_secondary_memos.is_empty());
        s.mint_death = mint;
        let receipt = s
            .apply_composite_attack_damage(
                &mut r,
                a,
                g,
                185,
                "composite:0",
                c,
                &defs,
                &facts(&root),
                &[],
                &fence,
                stamp,
            )
            .unwrap();
        assert_eq!(receipt.health_after, 0);
        assert!(receipt.death.is_some());
        assert_eq!(s.deaths.len(), 1);
        assert!(
            r.actor_conditions(a, Some(g))
                .unwrap()
                .instances()
                .is_empty()
        );
        assert_eq!(
            s.apply_composite_attack_damage(
                &mut r,
                a,
                g,
                185,
                "composite:0",
                c,
                &defs,
                &facts(&root),
                &[],
                &fence,
                stamp
            ),
            Some(receipt)
        );
        assert_eq!(s.deaths.len(), 1);
    }
    #[test]
    fn secondary_only_has_no_hp_receipt_and_current_external_fence_blocks_reentry() {
        let (mut r, mut s, a, g, c, mut fence, stamp) = ready(0x7d);
        let root = GameplayDecisionRoot::from_bytes([12; 32]);
        let defs = definitions();
        let hp = s.get(&r, a, g).cloned();
        assert!(s.apply_attack_conditions(
            &mut r,
            a,
            g,
            "secondary:0",
            c,
            &defs,
            &facts(&root),
            &[],
            &fence,
            stamp
        ));
        assert_eq!(
            s.get(&r, a, g).map(|v| v.vitals()),
            hp.as_ref().map(|v| v.vitals())
        );
        assert!(s.source_damage.is_empty());
        assert_eq!(s.get(&r, a, g).unwrap().owned_invisible_at(0), Ok(true));
        assert!(s.apply_attack_conditions(
            &mut r,
            a,
            g,
            "secondary:0",
            c,
            &defs,
            &facts(&root),
            &[],
            &fence,
            stamp
        ));
        assert!(
            s.apply_attack_damage(
                &mut r,
                a,
                g,
                1,
                "secondary:0",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
            )
            .is_none()
        );
        let before = s.get(&r, a, g).cloned();
        fence
            .apply_external_grant(ScopeOwnershipGeneration::new(2).unwrap())
            .unwrap();
        assert!(!s.apply_attack_conditions(
            &mut r,
            a,
            g,
            "secondary:1",
            c,
            &defs,
            &facts(&root),
            &[],
            &fence,
            stamp
        ));
        assert!(
            s.apply_composite_attack_damage(
                &mut r,
                a,
                g,
                1,
                "empty:stale",
                c,
                &[],
                &facts(&root),
                &[],
                &fence,
                stamp
            )
            .is_none()
        );
        assert_eq!(s.get(&r, a, g).cloned(), before);
    }
}

// Test fixture: use the same owned PlayerSpellState CAS as actual player condition writes.
// Explicit known self-use provenance; no production admission or authority is added.
#[cfg(test)]
impl ChannelSpellStates {
    pub(crate) fn install_owned_player_self_use_test_condition(
        &mut self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        target: ExactActorRef,
        session: GameSessionId,
        definition: &crate::ability::condition::ConditionDefinition,
        facts: &crate::ability::condition::ApplicationFacts<'_>,
    ) -> bool {
        if !current_owner(runtime, fence, stamp)
            || !self.condition_player_current(runtime, target, session)
            || !facts.target_is_player
        {
            return false;
        }
        let Some(before) = self.get(runtime, target, session) else {
            return false;
        };
        let Some(revision) = before.revision().checked_add(1) else {
            return false;
        };
        let mut conditions = before.owned_conditions().clone();
        if conditions
            .apply(
                definition,
                Some(format!(
                    "player:{}",
                    super::hex(&target.placement_identity())
                )),
                crate::ability::condition::ConditionSourceKind::SelfUse,
                &[],
                facts,
            )
            .is_err()
        {
            return false;
        }
        let mut next = before.clone();
        if next
            .apply_batch_conditions(before.owned_conditions(), &conditions)
            .is_err()
        {
            return false;
        }
        next.source_set_batch_revision(revision);
        self.commit(runtime, target, session, next)
    }
}

#[path = "appearance_atomic.rs"]
mod appearance_atomic;

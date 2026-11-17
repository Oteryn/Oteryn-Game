//! Crystal teleport_step_event.lua:97-161: two current-target percentage hits.
use super::*;
use crate::foundation::MovementPositionSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RottenMassiveKind {
    Energy,
    Agony,
}
impl RottenMassiveKind {
    pub fn element(self) -> &'static str {
        match self {
            Self::Energy => "energy",
            Self::Agony => "agony",
        }
    }
    pub fn effect(self) -> u16 {
        // Crystal201189f utils_definitions.hpp:93,215.
        match self {
            Self::Energy => 38,
            Self::Agony => 249,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RottenMassiveSource {
    actor: ExactActorRef,
    key: &'static str,
    revision: String,
    display: String,
    portal: [u8; 16],
    due_us: u64,
    phase: u8,
    kind: RottenMassiveKind,
    stamp: RuntimeWorkStamp,
    pins: ([u8; 32], [u8; 32], [u8; 32], [u8; 32], u64),
}
impl RottenMassiveSource {
    /// The retained physical portal STEP owner supplies its actual Item and due
    /// clock. This is not an Ability, an invented command or a second timer owner.
    pub fn from_current_callback(
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        portal: [u8; 16],
        due_us: u64,
        phase: u8,
        actor: ExactActorRef,
    ) -> Result<Self, AttackError> {
        if !matches!(
            runtime.binding().runtime_scope(),
            crate::foundation::RuntimeScopeRefV1::Instance { .. }
        ) || !matches!(phase, 1 | 2)
            || oteryn_gameplay_domain::domain::ItemInstanceId::from_bytes(portal).is_err()
            || !native
                .qualifies_rotten_blood_callbacks(runtime.content_pin().server_artifact_digest())
            || runtime.source_creature_is_registered(actor).ok() != Some(true)
        {
            return Err(AttackError::InvalidPlan);
        }
        let snapshot = runtime
            .companion_snapshot_including_dead(actor)
            .map_err(|_| AttackError::StaleIssuer)?;
        if !native.creature_profiles().records.iter().any(|record| {
            record.profile.target.key == snapshot.state.policy.definition_key
                && record.profile.target.revision == snapshot.state.policy.definition_revision
        }) || !native
            .policies()
            .policies()
            .iter()
            .any(|policy| policy == &snapshot.state.policy)
        {
            return Err(AttackError::InvalidPlan);
        }
        let (key, y, kind) = match snapshot.state.policy.definition_key.as_str() {
            "oteryn:creature.vemiath" => {
                ("oteryn:creature.vemiath", 32335, RottenMassiveKind::Energy)
            }
            "oteryn:creature.chagorz" => {
                ("oteryn:creature.chagorz", 32366, RottenMassiveKind::Energy)
            }
            "oteryn:creature.bakragore" => {
                ("oteryn:creature.bakragore", 32397, RottenMassiveKind::Agony)
            }
            _ => return Err(AttackError::InvalidPlan),
        };
        let at = runtime
            .read_actor_position(actor)
            .map_err(|_| AttackError::StaleIssuer)?
            .position();
        if snapshot.state.master.is_some()
            || runtime.native_summon_master(actor).is_some()
            || at.floor != 15
            || at.x.abs_diff(33043) > 15
            || at.y.abs_diff(y) > 15
        {
            return Err(AttackError::InvalidPlan);
        }
        let pin = runtime.content_pin();
        let pins = (
            pin.server_artifact_digest(),
            pin.client_artifact_digest(),
            pin.frame_binding_digest(),
            runtime.map_revision_digest(),
            pin.activation_sequence(),
        );
        let revision = snapshot.state.policy.definition_revision.clone();
        let display = snapshot.state.policy.display_name.clone();
        let stamp = runtime
            .issue_owner_work()
            .map_err(|_| AttackError::StaleIssuer)?;
        Ok(Self {
            actor,
            key,
            revision,
            display,
            portal,
            due_us,
            phase,
            kind,
            stamp,
            pins,
        })
    }
    pub fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub fn kind(&self) -> RottenMassiveKind {
        self.kind
    }
    pub fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        runtime
            .owner_fence()
            .is_ok_and(|owner| owner.accepts_stamp(self.stamp))
            && runtime.source_creature_is_registered(self.actor).ok() == Some(true)
            && self.pins
                == (
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    pin.frame_binding_digest(),
                    runtime.map_revision_digest(),
                    pin.activation_sequence(),
                )
            && runtime
                .companion_snapshot_including_dead(self.actor)
                .is_ok_and(|snapshot| {
                    snapshot.state.policy.definition_key == self.key
                        && snapshot.state.policy.definition_revision == self.revision
                        && snapshot.state.policy.display_name == self.display
                        && snapshot.state.master.is_none()
                })
            && runtime.native_summon_master(self.actor).is_none()
    }
    fn occurrence(&self, target: ExactActorRef) -> String {
        format!(
            "native:rotten-massive:{}:{}:{}:{}:{}",
            hex(&self.portal),
            self.phase,
            self.due_us,
            hex(&self.actor.placement_identity()),
            hex(&target.placement_identity())
        )
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "the source event retains actual current target, vitals, map and owner-clock facts"
    )]
    pub fn prepare_player(
        &self,
        runtime: &ChannelRuntimeV1,
        reader: &mut dyn SpellWorldReader,
        vitals: &dyn PlayerLethalVitals,
        target: ExactActorRef,
        session: GameSessionId,
        stamp: RuntimeWorkStamp,
        now: SemanticTimeMicros,
    ) -> Result<RottenMassiveDamagePlan, AttackError> {
        if !self.current(runtime)
            || !runtime
                .owner_fence()
                .is_ok_and(|owner| owner.accepts_stamp(stamp))
            || now.get() < self.due_us
        {
            return Err(AttackError::StaleIssuer);
        }
        let from = runtime
            .read_actor_position(self.actor)
            .map_err(|_| AttackError::StaleIssuer)?;
        let to = runtime
            .read_actor_position(target)
            .map_err(|_| AttackError::StaleTarget)?;
        if from.context() != to.context() {
            return Err(AttackError::StaleTarget);
        }
        let (health, maximum, revision) = vitals
            .source_player_health_including_dead(runtime, target, session)
            .ok_or(AttackError::MissingVitals)?;
        if maximum == 0 || health > maximum || revision == 0 {
            return Err(AttackError::InvalidPlan);
        }
        let percent = if self.phase == 1 { 15_u64 } else { 25_u64 };
        let requested = u32::try_from((u64::from(maximum) * percent).div_ceil(100))
            .map_err(|_| AttackError::NumericOverflow)?;
        let occurrence = self.occurrence(target);
        let mut plan = RottenMassiveDamagePlan {
            source: self.clone(),
            target,
            session,
            stamp,
            from,
            to,
            revision,
            maximum,
            requested,
            amount: 0,
            reflected: 0,
            reflects_equipment: false,
            visual: None,
            critical: false,
            occurrence,
        };
        let element = Some(self.kind.element());
        let facts = reader
            .combat(runtime, self.actor, target, session, element, stamp)
            .ok_or(AttackError::MissingCombatFacts)?;
        let actual = facts.attack;
        if actual.issuer != self.actor
            || actual.target != target
            || actual.session != session
            || actual.revision != revision
            || facts.multiplier_ppm > 10_000_000
        {
            return Err(AttackError::InvalidPlan);
        }
        if actual.issuer_pz
            || actual.target_pz
            || actual.issuer_protected
            || actual.target_protected
        {
            return Ok(plan);
        }
        plan.visual = Some(self.kind.effect());
        // Combat::doCombatHealth emits its effect before the HP function. A
        // registered HP0 Player still receives that effect and the continuation.
        if health == 0 {
            return Ok(plan);
        }
        let hash = Sha256::new()
            .chain_update(b"oteryn:rotten-massive:v1")
            .chain_update(plan.occurrence.as_bytes())
            .finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&hash[..16]);
        let draw = (deterministic_decision_u64(
            &GameplayDecisionRoot::from_bytes(self.pins.0),
            DecisionOccurrenceId::from_bytes(id),
            "hazard_critical",
            0,
        )
        .map_err(|_| AttackError::InvalidPlan)?
            % 1_000_000) as u32;
        let hazard = match self.kind {
            RottenMassiveKind::Energy => {
                // These exact Rotten bosses use the literal percentage as the
                // component. They have no Soul War third-taint damage hook.
                let hit = prepare_native_creature_player_hit(
                    runtime, &facts, requested, 0, false, element, draw, false, now,
                )?;
                plan.reflects_equipment = true;
                if hit.blocked {
                    return Ok(plan);
                }
                // combatBlockHit reflects before combatChangeHealth applies Hazard.
                plan.reflected = facts
                    .mitigate_damage(requested)
                    .ok_or(AttackError::NumericOverflow)?;
                hit.hazard
            }
            RottenMassiveKind::Agony => {
                // game.cpp:7222 skips block/equipment/reflection, not the later
                // Hazard call (8093) or ordinary mana shield (8142).
                oteryn_gameplay_combat::hazard::incoming_damage(
                    runtime,
                    &facts.attack,
                    requested,
                    draw,
                    now.get(),
                    false,
                )
                .map_err(|_| AttackError::NumericOverflow)?
            }
        };
        plan.amount = hazard.amount;
        plan.critical = hazard.critical;
        Ok(plan)
    }
}

/// A current, source-qualified hit consumed synchronously by the existing HP owner.
pub struct RottenMassiveDamagePlan {
    source: RottenMassiveSource,
    target: ExactActorRef,
    session: GameSessionId,
    stamp: RuntimeWorkStamp,
    from: MovementPositionSnapshot,
    to: MovementPositionSnapshot,
    revision: u64,
    maximum: u32,
    requested: u32,
    amount: u32,
    reflected: u32,
    reflects_equipment: bool,
    visual: Option<u16>,
    critical: bool,
    occurrence: String,
}
impl RottenMassiveDamagePlan {
    pub fn validate(&self, runtime: &ChannelRuntimeV1, vitals: &dyn PlayerLethalVitals) -> bool {
        self.source.current(runtime)
            && runtime
                .owner_fence()
                .is_ok_and(|owner| owner.accepts_stamp(self.stamp))
            && runtime.read_actor_position(self.source.actor).ok() == Some(self.from)
            && runtime.read_actor_position(self.target).ok() == Some(self.to)
            && vitals
                .source_player_health_including_dead(runtime, self.target, self.session)
                .is_some_and(|(_, maximum, revision)| {
                    maximum == self.maximum && revision == self.revision
                })
    }
    pub fn source(&self) -> ExactActorRef {
        self.source.actor
    }
    pub fn target(&self) -> (ExactActorRef, GameSessionId) {
        (self.target, self.session)
    }
    pub fn kind(&self) -> RottenMassiveKind {
        self.source.kind
    }
    pub fn source_identity(&self) -> (&str, &str) {
        (self.source.key, &self.source.display)
    }
    pub fn requested(&self) -> u32 {
        self.requested
    }
    pub fn amount(&self) -> u32 {
        self.amount
    }
    pub fn reflection_amount(&self) -> u32 {
        self.reflected
    }
    pub fn reflects_equipment(&self) -> bool {
        self.reflects_equipment
    }
    pub fn visual_effect(&self) -> Option<u16> {
        self.visual
    }
    pub fn critical(&self) -> bool {
        self.critical
    }
    pub fn occurrence(&self) -> &str {
        &self.occurrence
    }
    pub fn source_sequence(&self) -> u64 {
        self.source.due_us
    }
}

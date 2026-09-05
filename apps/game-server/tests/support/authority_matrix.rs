//! Test-only independent source and executable authority coverage registry.
//! No live value is read from the prepared/persisted record. `bind` passes that
//! record only as the expected identity required by the production API.
#![allow(dead_code)] // Shared by focused tests and the separately run PG target.
use oteryn_game_server::foundation::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fmt::Debug;

pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub const ACCOUNT: &str = "123e4567-e89b-12d3-a456-426614174000";
pub const OTHER_ACCOUNT: &str = "123e4567-e89b-12d3-a456-426614174001";
pub fn checked<T, E: Debug>(result: Result<T, E>) -> TestResult<T> {
    result.map_err(|error| std::io::Error::other(format!("{error:?}")).into())
}
pub fn uuid(raw: u64) -> [u8; 16] {
    let mut value = [0; 16];
    value[8..].copy_from_slice(&raw.to_be_bytes());
    value[6] = 0x70;
    value[8] = (value[8] & 0x3f) | 0x80;
    value
}
pub fn session(raw: u64) -> TestResult<GameSessionId> {
    checked(GameSessionId::decode(&uuid(raw)))
}
pub fn character(raw: u64) -> TestResult<CharacterId> {
    checked(CharacterId::decode(&uuid(raw)))
}
pub fn world(raw: u64) -> TestResult<WorldId> {
    checked(WorldId::decode(&uuid(raw)))
}
pub fn channel(raw: u64) -> TestResult<ChannelId> {
    checked(ChannelId::decode(&uuid(raw)))
}
pub fn transport(raw: u8) -> TestResult<AuthenticatedTransportRefV1> {
    checked(AuthenticatedTransportRefV1::decode(&[raw; 16]))
}

#[derive(Clone, Copy, Debug)]
pub struct Seed {
    pub session: u64,
    pub character: u64,
    pub generation: u64,
    pub epoch: u64,
    pub proof_nonce: u8,
    pub attempt: u64,
    pub transport: u8,
    pub now: i64,
}
impl Seed {
    pub const fn fixed() -> Self {
        Self {
            session: 20,
            character: 11,
            generation: 7,
            epoch: 3,
            proof_nonce: 0x55,
            attempt: 1,
            transport: 0x71,
            now: 100,
        }
    }
}

/// Immutable PREPARE evidence is deliberately constructed separately from LiveSource.
pub fn prepared_record(seed: Seed) -> TestResult<ReconnectDurabilityRecordV1> {
    checked(ReconnectDurabilityRecordV1::new(
        checked(ReconnectIdentityV1::new(
            session(seed.session)?,
            checked(ReconnectAttemptRef::new(seed.attempt))?,
            ACCOUNT,
            character(seed.character)?,
            world(12)?,
            RuntimeScopeRefV1::channel(world(12)?, channel(13)?),
        ))?,
        checked(ReconnectConnectionFenceV1::new(
            checked(ConnectionGeneration::new(seed.generation))?,
            checked(ConnectionGeneration::new(seed.generation + 1))?,
            transport(seed.transport)?,
        ))?,
        checked(ReconnectAuthorityFenceV1::new(
            9,
            checked(ScopeOwnershipGeneration::new(10))?,
        ))?,
        checked(ReconnectContinuityV1::new(
            checked(ControlLossEpochRefV1::new(seed.epoch))?,
            seed.now + 120,
            seed.now + 115,
            ProtectionEntitlementV1::unused(),
        ))?,
        ReconnectProofV1::ReauthenticatedRecovery {
            recovery_grant_nonce: [seed.proof_nonce; 32],
        },
        checked(Fnd02ReconciliationFenceV1::new(
            checked(CommandId::new(3))?,
            vec![PendingCommandReconciliationV1::new(
                checked(CommandId::new(1))?,
                PendingCommandDispositionV1::PendingOriginal,
            )],
            41,
            vec![checked(StateDomainRevisionV1::new(1, 4))?],
        ))?,
        checked(ReconnectCompatibilityEvidenceV1::new(
            1,
            1,
            "rules:1",
            "content:2",
            "map:3",
            "world:4",
            12,
            checked(AuthorityEvidenceFenceV1::new(
                "platform-security",
                "reconnect",
                "account",
                "sec:17",
                "decision:sec:17",
                seed.now,
            ))?,
            checked(AuthorityEvidenceFenceV1::new(
                "proof-trust",
                "reconnect",
                "recovery-key",
                "trust:21",
                "decision:trust:21",
                seed.now,
            ))?,
            Some(seed.now + 110),
        ))?,
    ))
}

/// Simulated independent resolver response. The constructor accepts scenario IDs
/// and clock input, never a record. JSON also supports later process-restart tests.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveSource(pub Value);
impl LiveSource {
    pub fn read(seed: Seed) -> Self {
        let mut first = json!({
            "account":ACCOUNT,"presence_character":seed.character,"eligible_character":seed.character,"eligible_world":12,
            "candidate_present":true,"candidate_session":seed.session,"attempt":seed.attempt,
            "candidate_generation":seed.generation+1,"candidate_transport":seed.transport,"candidate_deadline":seed.now+115,
            "runtime_channel":13,"predecessor_session":10,"predecessor_generation":seed.generation,"lease_character":seed.character,"lease_generation":9,"scope_generation":10,
            "epoch":seed.epoch,"grace_deadline":seed.now+120,"proof_nonce":seed.proof_nonce,"fast_proof":null,
            "next_command":3,"pending_terminal":false,"pending_present":true,"pending_id":1,"pending_extra":false,
            "last_sequence":41,"domain_revision":4,"domain_present":true,"domain_id":1,"domain_extra":false,
        });
        let second = json!({
            "protocol_major":1,"transport_profile":1,"rules":"rules:1","content":"content:2","map":"map:3","world_policy":"world:4",
            "security_generation":12,"platform_authority":"platform-security","platform_purpose":"reconnect","platform_scope":"account",
            "platform_revision":"sec:17","platform_decision":"decision:sec:17","platform_observed":seed.now,
            "trust_authority":"proof-trust","trust_purpose":"reconnect","trust_scope":"recovery-key",
            "trust_revision":"trust:21","trust_decision":"decision:trust:21","trust_observed":seed.now,
            "credential_expiration":seed.now+110,"predecessor_state":"terminal","live_state":"reconnectable","controller_present":false,"observed_at":seed.now+2,"authorization_at":seed.now+2
        });
        if let (Some(a), Some(b)) = (first.as_object_mut(), second.as_object()) {
            a.extend(b.clone());
        }
        Self(first)
    }
    pub fn number(&self, key: &str) -> TestResult<u64> {
        self.0[key]
            .as_u64()
            .ok_or_else(|| format!("invalid unsigned source {key}").into())
    }
    pub fn time(&self, key: &str) -> TestResult<i64> {
        self.0[key]
            .as_i64()
            .ok_or_else(|| format!("invalid time source {key}").into())
    }
    pub fn text(&self, key: &str) -> TestResult<&str> {
        self.0[key]
            .as_str()
            .ok_or_else(|| format!("invalid text source {key}").into())
    }
    pub fn flag(&self, key: &str) -> TestResult<bool> {
        self.0[key]
            .as_bool()
            .ok_or_else(|| format!("invalid flag source {key}").into())
    }
    fn presence(&self) -> TestResult<Option<AccountPresenceClaimV1>> {
        if self.0["account"].is_null() {
            return Ok(None);
        }
        Ok(Some(checked(AccountPresenceClaimV1::new(
            self.text("account")?,
            character(self.number("presence_character")?)?,
        ))?))
    }
    fn eligibility(&self) -> TestResult<Option<CharacterWorldEligibilityClaimV1>> {
        if self.0["eligible_character"].is_null() {
            return Ok(None);
        }
        Ok(Some(CharacterWorldEligibilityClaimV1::new(
            character(self.number("eligible_character")?)?,
            world(self.number("eligible_world")?)?,
        )))
    }
    fn evidence(&self, prefix: &str) -> TestResult<AuthorityEvidenceFenceV1> {
        checked(AuthorityEvidenceFenceV1::new(
            self.text(&format!("{prefix}_authority"))?,
            self.text(&format!("{prefix}_purpose"))?,
            self.text(&format!("{prefix}_scope"))?,
            self.text(&format!("{prefix}_revision"))?,
            self.text(&format!("{prefix}_decision"))?,
            self.time(&format!("{prefix}_observed"))?,
        ))
    }
    pub fn bind(
        &self,
        expected: &ReconnectDurabilityRecordV1,
    ) -> TestResult<ReconnectCurrentAuthorityV1> {
        let candidate = if self.flag("candidate_present")? {
            Some(checked(ReconnectCandidateBindingV1::new(
                session(self.number("candidate_session")?)?,
                checked(ReconnectAttemptRef::new(self.number("attempt")?))?,
                checked(ConnectionGeneration::new(
                    self.number("candidate_generation")?,
                ))?,
                transport(self.number("candidate_transport")?.try_into()?)?,
                self.time("candidate_deadline")?,
            ))?)
        } else {
            None
        };
        let proof = if self.0["fast_proof"].is_null() {
            ReconnectProofV1::ReauthenticatedRecovery {
                recovery_grant_nonce: [self.number("proof_nonce")?.try_into()?; 32],
            }
        } else {
            ReconnectProofV1::FastReconnect {
                reconnect_proof_generation: self.number("fast_proof")?,
            }
        };
        let mut pending = if self.flag("pending_present")? {
            vec![PendingCommandReconciliationV1::new(
                checked(CommandId::new(self.number("pending_id")?))?,
                if self.flag("pending_terminal")? {
                    PendingCommandDispositionV1::TerminalOutcomeRetained
                } else {
                    PendingCommandDispositionV1::PendingOriginal
                },
            )]
        } else {
            vec![]
        };
        if self.flag("pending_extra")? {
            pending.push(PendingCommandReconciliationV1::new(
                checked(CommandId::new(2))?,
                PendingCommandDispositionV1::PendingOriginal,
            ));
        }
        let mut domains = if self.flag("domain_present")? {
            vec![checked(StateDomainRevisionV1::new(
                self.number("domain_id")?.try_into()?,
                self.number("domain_revision")?,
            ))?]
        } else {
            vec![]
        };
        if self.flag("domain_extra")? {
            domains.push(checked(StateDomainRevisionV1::new(2, 4))?);
        }
        let fnd02 = checked(Fnd02ReconciliationFenceV1::new(
            checked(CommandId::new(self.number("next_command")?))?,
            pending,
            self.number("last_sequence")?,
            domains,
        ))?;
        let compatibility = checked(ReconnectCompatibilityEvidenceV1::new(
            self.number("protocol_major")?.try_into()?,
            self.number("transport_profile")?.try_into()?,
            self.text("rules")?,
            self.text("content")?,
            self.text("map")?,
            self.text("world_policy")?,
            self.number("security_generation")?,
            self.evidence("platform")?,
            self.evidence("trust")?,
            self.0["credential_expiration"].as_i64(),
        ))?;
        checked(ReconnectCurrentAuthorityV1::from_current_facts(
            expected,
            self.presence()?,
            self.eligibility()?,
            candidate,
            RuntimeScopeRefV1::channel(world(12)?, channel(self.number("runtime_channel")?)?),
            checked(ConnectionGeneration::new(
                self.number("predecessor_generation")?,
            ))?,
            checked(ReconnectAuthorityFenceV1::new(
                self.number("lease_generation")?,
                checked(ScopeOwnershipGeneration::new(
                    self.number("scope_generation")?,
                ))?,
            ))?,
            checked(ControlLossEpochRefV1::new(self.number("epoch")?))?,
            self.time("grace_deadline")?,
            proof,
            fnd02,
            compatibility,
            match self.text("live_state")? {
                "reconnectable" => GameSessionState::Reconnectable,
                "terminal" => GameSessionState::Terminal,
                "active" => GameSessionState::Active,
                _ => return Err("unknown live state".into()),
            },
            self.flag("controller_present")?,
            self.time("observed_at")?,
        ))
    }
    pub fn authorize_replacement(
        &self,
        expected: &ReconnectDurabilityRecordV1,
    ) -> TestResult<TerminalGameSessionReplacementAuthorizationV1> {
        // The initial receipt and independent live state intentionally have different generations.
        let facts = checked(FreshAdmissionFacts::new(
            [0x44; 32],
            character(11)?,
            world(12)?,
            channel(13)?,
            9,
            9,
        ))?;
        let commit = checked(FreshAdmissionCommit::from_facts(
            session(10)?,
            facts,
            transport(0x70)?,
        ))?;
        let snapshot = checked(GameSessionAuthoritySnapshot::from_current_facts(
            commit,
            if self.text("predecessor_state")? == "terminal" {
                GameSessionState::Terminal
            } else {
                GameSessionState::Reconnectable
            },
            checked(ConnectionGeneration::new(
                self.number("predecessor_generation")?,
            ))?,
            if self.flag("controller_present")? {
                Some(transport(0x70)?)
            } else {
                None
            },
            checked(CharacterLease::new(
                character(self.number("lease_character")?)?,
                self.number("lease_generation")?,
            ))?,
            self.eligibility()?,
            RuntimeScopeRefV1::channel(world(12)?, channel(self.number("runtime_channel")?)?),
            checked(ScopeOwnershipGeneration::new(
                self.number("scope_generation")?,
            ))?,
        ))?;
        let snapshot = if self.0["epoch"].is_null() || self.0["grace_deadline"].is_null() {
            snapshot
        } else {
            checked(snapshot.with_control_loss_continuity(
                checked(ControlLossEpochRefV1::new(self.number("epoch")?))?,
                self.time("grace_deadline")?,
            ))?
        };
        checked(
            TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
                ACCOUNT,
                self.presence()?.as_ref(),
                session(self.number("predecessor_session")?)?,
                session(self.number("candidate_session")?)?,
                snapshot,
                expected,
            ),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvariantClass {
    IdentityBinding,
    CurrentAuthority,
    TemporalProvenance,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MutationOperator {
    Missing,
    Different,
    Older,
    Newer,
    Present,
    Nonterminal,
    BeforeEvidence,
    AfterAuthorization,
}
use InvariantClass::{CurrentAuthority, IdentityBinding, TemporalProvenance};
use MutationOperator::{
    AfterAuthorization, BeforeEvidence, Different, Missing, Newer, Nonterminal, Older, Present,
};

macro_rules! invariants {
    ($($name:ident => ($field:literal,$class:ident,$prepare:literal,[$($op:ident),+])),+ $(,)?) => {
        #[derive(Clone,Copy,Debug,Eq,PartialEq,Ord,PartialOrd)]
        pub enum AuthorityInvariant { $($name),+ }
        impl AuthorityInvariant {
            pub const ALL: &'static [Self] = &[$(Self::$name),+];
            pub const fn field(self)-> &'static str { match self {$(Self::$name=>$field),+} }
            pub const fn class(self)->InvariantClass {match self {$(Self::$name=>$class),+}}
            pub const fn prepare(self)->bool {match self {$(Self::$name=>$prepare),+}}
            pub const fn operators(self)-> &'static [MutationOperator] { match self {$(Self::$name=>&[$($op),+]),+} }
        }
    }
}
invariants! {
    AccountPresence => ("account",IdentityBinding,true,[Missing,Different]),
    PresenceCharacter => ("presence_character",IdentityBinding,true,[Different]),
    CharacterEligibility => ("eligible_character",IdentityBinding,true,[Missing,Different]),
    WorldEligibility => ("eligible_world",IdentityBinding,true,[Different]),
    CandidatePresence => ("candidate_present",CurrentAuthority,false,[Missing]),
    CandidateSession => ("candidate_session",IdentityBinding,true,[Different]),
    Attempt => ("attempt",IdentityBinding,false,[Different]),
    CandidateGeneration => ("candidate_generation",IdentityBinding,false,[Older,Newer]),
    CandidateTransport => ("candidate_transport",IdentityBinding,false,[Different]),
    CandidateDeadline => ("candidate_deadline",TemporalProvenance,false,[Older,Newer]),
    RuntimeScope => ("runtime_channel",IdentityBinding,true,[Different]),
    PredecessorSession => ("predecessor_session",IdentityBinding,true,[Different]),
    PredecessorGeneration => ("predecessor_generation",CurrentAuthority,true,[Older,Newer]),
    LeaseCharacter => ("lease_character",IdentityBinding,true,[Different]),
    LeaseGeneration => ("lease_generation",CurrentAuthority,true,[Older,Newer]),
    ScopeGeneration => ("scope_generation",CurrentAuthority,true,[Older,Newer]),
    ContinuityEpoch => ("epoch",IdentityBinding,true,[Older,Newer]),
    GraceDeadline => ("grace_deadline",TemporalProvenance,true,[Older,Newer]),
    RecoveryProof => ("proof_nonce",IdentityBinding,false,[Different]),
    ProofKind => ("fast_proof",IdentityBinding,false,[Present]),
    NextCommand => ("next_command",IdentityBinding,false,[Older,Newer]),
    PendingExtraMember => ("pending_extra",IdentityBinding,false,[Present]),
    DomainExtraMember => ("domain_extra",IdentityBinding,false,[Present]),
    PendingMembership => ("pending_present",IdentityBinding,false,[Missing]),
    PendingIdentifier => ("pending_id",IdentityBinding,false,[Different]),
    DomainMembership => ("domain_present",IdentityBinding,false,[Missing]),
    DomainIdentifier => ("domain_id",IdentityBinding,false,[Different]),
    PendingCommand => ("pending_terminal",IdentityBinding,false,[Different]),
    ServerSequence => ("last_sequence",IdentityBinding,false,[Older,Newer]),
    DomainRevision => ("domain_revision",IdentityBinding,false,[Older,Newer]),
    ProtocolMajor => ("protocol_major",IdentityBinding,false,[Different]),
    TransportProfile => ("transport_profile",IdentityBinding,false,[Different]),
    Ruleset => ("rules",IdentityBinding,false,[Different]),
    Content => ("content",IdentityBinding,false,[Different]),
    Map => ("map",IdentityBinding,false,[Different]),
    WorldPolicy => ("world_policy",IdentityBinding,false,[Different]),
    SecurityGeneration => ("security_generation",CurrentAuthority,false,[Older,Newer]),
    PlatformAuthority => ("platform_authority",TemporalProvenance,false,[Different]),
    PlatformPurpose => ("platform_purpose",TemporalProvenance,false,[Different]),
    PlatformScope => ("platform_scope",TemporalProvenance,false,[Different]),
    PlatformRevision => ("platform_revision",TemporalProvenance,false,[Different]),
    PlatformDecision => ("platform_decision",TemporalProvenance,false,[Different]),
    PlatformObservation => ("platform_observed",TemporalProvenance,false,[Older,Newer]),
    TrustAuthority => ("trust_authority",TemporalProvenance,false,[Different]),
    TrustPurpose => ("trust_purpose",TemporalProvenance,false,[Different]),
    TrustScope => ("trust_scope",TemporalProvenance,false,[Different]),
    TrustRevision => ("trust_revision",TemporalProvenance,false,[Different]),
    TrustDecision => ("trust_decision",TemporalProvenance,false,[Different]),
    TrustObservation => ("trust_observed",TemporalProvenance,false,[Older,Newer]),
    CredentialExpiration => ("credential_expiration",TemporalProvenance,false,[Missing,Older,Newer]),
    SessionState => ("live_state",CurrentAuthority,false,[Nonterminal]),
    PredecessorState => ("predecessor_state",CurrentAuthority,true,[Nonterminal]),
    ControllerExclusion => ("controller_present",CurrentAuthority,true,[Present]),
    ObservationWindow => ("observed_at",TemporalProvenance,false,[BeforeEvidence,AfterAuthorization]),
    CommitClock => ("authorization_at",TemporalProvenance,false,[BeforeEvidence,AfterAuthorization])
}

macro_rules! boundaries {
    ($($name:ident => $label:literal),+ $(,)?) => {
        #[derive(Clone,Copy,Debug,Eq,PartialEq,Ord,PartialOrd)]
        pub enum ConsumerBoundary {$($name),+}
        impl ConsumerBoundary {
            pub const ALL: &'static [Self]=&[$(Self::$name),+];
            pub const fn label(self)-> &'static str {match self {$(Self::$name=>$label),+}}
        }
    }
}
boundaries! { TerminalPrepare=>"terminal-prepare",CommitV1=>"commit-v1",CommitV2=>"commit-v2",ReconcileV1=>"reconcile-v1",ReconcileV2=>"reconcile-v2" }
impl ConsumerBoundary {
    pub fn not_applicable(self, invariant: AuthorityInvariant) -> Option<&'static str> {
        use AuthorityInvariant::{
            CommitClock, LeaseCharacter, PredecessorSession, PredecessorState,
        };
        match self {
            Self::TerminalPrepare if !invariant.prepare() => Some(
                "PREPARE replacement authorization does not consume reconnect candidate/security/proof/time facts; COMMIT revalidates them",
            ),
            Self::TerminalPrepare => None,
            Self::ReconcileV1 | Self::ReconcileV2 if invariant == CommitClock => Some(
                "reconciliation accepts an independently observed current fact time, not a separate COMMIT clock",
            ),
            Self::CommitV1 | Self::CommitV2 | Self::ReconcileV1 | Self::ReconcileV2 => {
                match invariant {
                    PredecessorSession | LeaseCharacter | PredecessorState => Some(
                        "predecessor receipt identity is consumed at replacement PREPARE; reconnect API carries candidate identity and scalar lease fence",
                    ),
                    _ => None,
                }
            }
        }
    }
}

pub fn mutated(
    source: &LiveSource,
    invariant: AuthorityInvariant,
    operator: MutationOperator,
    seed: Seed,
) -> TestResult<LiveSource> {
    let field = invariant.field();
    let mut changed = source.clone();
    let old = &source.0[field];
    changed.0[field] = match operator {
        Missing if old.is_boolean() => json!(false),
        Missing => Value::Null,
        Present if old.is_boolean() => json!(true),
        Present => json!(17),
        Nonterminal => json!("active"),
        BeforeEvidence => json!(seed.now - 1),
        AfterAuthorization => json!(seed.now + 6),
        Older => json!(source.time(field)? - 1),
        Newer => json!(source.time(field)? + 1),
        Different if field == "account" => json!(OTHER_ACCOUNT),
        Different if old.is_boolean() => json!(!source.flag(field)?),
        Different if old.is_string() => json!(format!("{}:other", source.text(field)?)),
        Different => json!(source.number(field)? + 1),
    };
    let original = source.0.as_object().ok_or("source object")?;
    let changed_fields: Vec<_> = original
        .keys()
        .filter(|key| source.0[*key] != changed.0[*key])
        .map(String::as_str)
        .collect();
    assert_eq!(
        changed_fields,
        vec![field],
        "one-invariant rule: {invariant:?}/{operator:?}"
    );
    Ok(changed)
}

pub fn prepared_v1(record: &ReconnectDurabilityRecordV1) -> TestResult<ReconnectDurabilityFlowV1> {
    let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
    checked(
        flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &request,
            ReconnectPrepareDispositionV1::Prepared,
        )),
    )?;
    Ok(flow)
}
pub fn v2_budget(seed: Seed) -> TestResult<ReconnectAttemptBudgetV1> {
    let mut budget =
        ReconnectAttemptBudgetV1::new(checked(ControlLossEpochRefV1::new(seed.epoch))?);
    checked(budget.reserve(
        checked(ReconnectAttemptRef::new(seed.attempt))?,
        transport(seed.transport)?,
    ))?;
    Ok(budget)
}
pub fn prepared_v2(
    record: &ReconnectDurabilityRecordV1,
    seed: Seed,
) -> TestResult<ReconnectDurabilityFlowV2> {
    let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
    checked(flow.accept_prepare_completion(
        ReconnectPrepareCompletionV2::for_request(
            &request,
            ReconnectPrepareDispositionV2::Prepared,
        ),
        &mut v2_budget(seed)?,
    ))?;
    Ok(flow)
}
pub fn reconcile_v1(
    record: &ReconnectDurabilityRecordV1,
    current: ReconnectCurrentAuthorityV1,
    snapshot: ReconnectDurableReconciliationSnapshotV1,
) -> TestResult<ReconnectProjectionDecisionV1> {
    let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
    checked(
        flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &request,
            ReconnectPrepareDispositionV1::Ambiguous,
        )),
    )?;
    let result = flow.accept_reconciliation(snapshot, current);
    if result.is_err() {
        assert_eq!(
            flow.phase(),
            ReconnectDurabilityPhaseV1::ReconciliationRequired
        );
    }
    if let Ok(decision) = &result {
        assert_eq!(
            flow.phase(),
            match decision {
                ReconnectProjectionDecisionV1::InstallController { .. } =>
                    ReconnectDurabilityPhaseV1::Completed,
                ReconnectProjectionDecisionV1::Terminal => ReconnectDurabilityPhaseV1::Terminal,
                ReconnectProjectionDecisionV1::AwaitFinalRevalidation =>
                    ReconnectDurabilityPhaseV1::AwaitFinalRevalidation,
            }
        );
    }
    checked(result)
}
pub fn reconcile_v2(
    record: &ReconnectDurabilityRecordV1,
    current: ReconnectCurrentAuthorityV1,
    snapshot: ReconnectDurableReconciliationSnapshotV2,
    seed: Seed,
) -> TestResult<ReconnectProjectionDecisionV2> {
    let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
    let mut budget = v2_budget(seed)?;
    checked(flow.accept_prepare_completion(
        ReconnectPrepareCompletionV2::for_request(
            &request,
            ReconnectPrepareDispositionV2::Ambiguous,
        ),
        &mut budget,
    ))?;
    let result = flow.accept_reconciliation(snapshot, current, &mut budget);
    if result.is_err() {
        assert_eq!(
            flow.phase(),
            ReconnectDurabilityPhaseV1::ReconciliationRequired
        );
    }
    if let Ok(decision) = &result {
        assert_eq!(
            flow.phase(),
            match decision {
                ReconnectProjectionDecisionV2::InstallController { .. } =>
                    ReconnectDurabilityPhaseV1::Completed,
                ReconnectProjectionDecisionV2::Terminal { .. } =>
                    ReconnectDurabilityPhaseV1::Terminal,
                ReconnectProjectionDecisionV2::AwaitFinalRevalidation =>
                    ReconnectDurabilityPhaseV1::AwaitFinalRevalidation,
            }
        );
    }
    checked(result)
}

pub fn exercise(
    boundary: ConsumerBoundary,
    record: &ReconnectDurabilityRecordV1,
    source: &LiveSource,
    seed: Seed,
) -> TestResult<bool> {
    match boundary {
        ConsumerBoundary::TerminalPrepare => Ok(source.authorize_replacement(record).is_ok()),
        ConsumerBoundary::CommitV1 => {
            let mut flow = prepared_v1(record)?;
            let result =
                flow.authorize_commit(source.bind(record)?, source.time("authorization_at")?);
            assert_eq!(
                flow.phase(),
                if result.is_ok() {
                    ReconnectDurabilityPhaseV1::PendingCommit
                } else {
                    ReconnectDurabilityPhaseV1::Terminal
                }
            );
            Ok(result.is_ok())
        }
        ConsumerBoundary::CommitV2 => {
            let mut flow = prepared_v2(record, seed)?;
            let result =
                flow.authorize_commit(source.bind(record)?, source.time("authorization_at")?);
            assert_eq!(
                flow.phase(),
                if result.is_ok() {
                    ReconnectDurabilityPhaseV1::PendingCommit
                } else {
                    ReconnectDurabilityPhaseV1::Terminal
                }
            );
            Ok(result.is_ok())
        }
        ConsumerBoundary::ReconcileV1 => Ok(matches!(
            reconcile_v1(
                record,
                source.bind(record)?,
                ReconnectDurableReconciliationSnapshotV1::committed(record.clone())
            ),
            Ok(ReconnectProjectionDecisionV1::InstallController { .. })
        )),
        ConsumerBoundary::ReconcileV2 => Ok(matches!(
            reconcile_v2(
                record,
                source.bind(record)?,
                ReconnectDurableReconciliationSnapshotV2::new(
                    record.clone(),
                    ReconnectDurableOutcomeV2::Committed {
                        current_generation: checked(ConnectionGeneration::new(8))?,
                        current_transport_ref: transport(seed.transport)?
                    }
                ),
                seed
            ),
            Ok(ReconnectProjectionDecisionV2::InstallController { .. })
        )),
    }
}

pub fn run_matrix() -> TestResult<Vec<String>> {
    let seed = Seed::fixed();
    let record = prepared_record(seed)?;
    let source = LiveSource::read(seed);
    let mut executed = BTreeSet::new();
    let mut required = BTreeSet::new();
    for &boundary in ConsumerBoundary::ALL {
        assert!(
            exercise(boundary, &record, &source, seed)?,
            "positive control {boundary:?}"
        );
        for &invariant in AuthorityInvariant::ALL {
            if let Some(reason) = boundary.not_applicable(invariant) {
                assert!(!reason.is_empty());
                continue;
            }
            for &operator in invariant.operators() {
                required.insert((boundary, invariant, operator));
            }
        }
    }
    for &(boundary, invariant, operator) in &required {
        let live = mutated(&source, invariant, operator, seed)?;
        assert!(
            !exercise(boundary, &record, &live, seed)?,
            "authority granted: {boundary:?}/{invariant:?}/{operator:?} ({:?})",
            invariant.class()
        );
        assert!(
            executed.insert((boundary, invariant, operator)),
            "duplicate case"
        );
    }
    assert_eq!(executed, required, "registry completeness");
    // Isolated missing continuity is unrepresentable through the public snapshot
    // builder: it sets epoch and deadline together. Do not count two absent facts
    // as one mutated invariant. Nonoptional typed provenance/lease absence and
    // standalone grace expiry (dominated by prepared expiry) are also N/A.
    for reason in [
        "isolated missing continuity: public builder sets epoch and deadline atomically",
        "missing typed provenance/lease/scope: nonoptional constructor inputs",
        "standalone grace expiration: valid prepared deadline is no later than grace",
    ] {
        assert!(!reason.is_empty());
    }
    // Boundary equality is accepted; crossing the five-second evidence window is
    // independently tested through ObservationWindow and CommitClock operators.
    for boundary in [ConsumerBoundary::CommitV1, ConsumerBoundary::CommitV2] {
        let mut at_deadline = source.clone();
        at_deadline.0["authorization_at"] = json!(seed.now + 5);
        assert!(exercise(boundary, &record, &at_deadline, seed)?);
        at_deadline.0["authorization_at"] = json!(seed.now + 2);
        at_deadline.0["observed_at"] = json!(seed.now + 5);
        assert!(exercise(boundary, &record, &at_deadline, seed)?);
    }
    // Changing immutable PREPARE evidence must not alter an independently read live source.
    let other = prepared_record(Seed {
        session: 99,
        ..seed
    })?;
    assert!(
        prepared_v1(&other)?
            .authorize_commit(source.bind(&other)?, seed.now + 2)
            .is_err()
    );
    assert_eq!(source, LiveSource::read(seed));
    println!(
        "authority registry: {} isolated negative cases, {} positive boundaries",
        executed.len(),
        ConsumerBoundary::ALL.len()
    );
    Ok(executed
        .into_iter()
        .map(|(b, i, o)| format!("{}/{i:?}/{o:?}", b.label()))
        .collect())
}

pub fn run_loaded_matrix(
    seed: Seed,
    record: &ReconnectDurabilityRecordV1,
    source: &LiveSource,
    v1: ReconnectDurableReconciliationSnapshotV1,
    v2: ReconnectDurableReconciliationSnapshotV2,
) -> TestResult<()> {
    assert!(matches!(
        reconcile_v1(record, source.bind(record)?, v1.clone())?,
        ReconnectProjectionDecisionV1::InstallController { .. }
    ));
    assert!(matches!(
        reconcile_v2(record, source.bind(record)?, v2.clone(), seed)?,
        ReconnectProjectionDecisionV2::InstallController { .. }
    ));
    let mut count = 0;
    for &invariant in AuthorityInvariant::ALL {
        if ConsumerBoundary::ReconcileV1
            .not_applicable(invariant)
            .is_some()
        {
            continue;
        }
        for &operator in invariant.operators() {
            let live = mutated(source, invariant, operator, seed)?;
            assert!(
                reconcile_v1(record, live.bind(record)?, v1.clone()).is_err(),
                "PG reload V1 {invariant:?}/{operator:?}"
            );
            assert!(
                reconcile_v2(record, live.bind(record)?, v2.clone(), seed).is_err(),
                "PG reload V2 {invariant:?}/{operator:?}"
            );
            count += 2;
        }
    }
    println!(
        "PostgreSQL reload authority matrix: {count} isolated negative cases, both positive InstallController controls"
    );
    Ok(())
}

pub fn registered_cases(boundary: ConsumerBoundary) -> Vec<(AuthorityInvariant, MutationOperator)> {
    AuthorityInvariant::ALL
        .iter()
        .copied()
        .filter(|&invariant| boundary.not_applicable(invariant).is_none())
        .flat_map(|invariant| {
            invariant
                .operators()
                .iter()
                .map(move |&operator| (invariant, operator))
        })
        .collect()
}

/// The durable replay result is supplied by the caller (real PG in the E2E).
/// Replay itself grants no authority: final COMMIT must reread independent facts.
pub fn run_retry_matrix(
    seed: Seed,
    record: &ReconnectDurabilityRecordV1,
    source: &LiveSource,
    replacement: Option<TerminalGameSessionReplacementAuthorizationV1>,
    v1: Option<ReconnectPrepareDispositionV1>,
    v2: ReconnectPrepareDispositionV2,
) -> TestResult<Vec<String>> {
    assert_eq!(v2, ReconnectPrepareDispositionV2::ExistingPrepared);
    if let Some(disposition) = v1 {
        assert_eq!(disposition, ReconnectPrepareDispositionV1::ExistingPrepared);
    }
    let mut executed = Vec::new();
    for boundary in [ConsumerBoundary::CommitV1, ConsumerBoundary::CommitV2] {
        if boundary == ConsumerBoundary::CommitV1 && v1.is_none() {
            assert!(
                replacement.is_some(),
                "V1 N/A only for signed replacement recovery"
            );
            continue;
        }
        let exercise_retry = |live: &LiveSource| -> TestResult<bool> {
            let accepted = if boundary == ConsumerBoundary::CommitV1 {
                let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
                assert_eq!(
                    checked(flow.accept_prepare_completion(
                        ReconnectPrepareCompletionV1::for_request(
                            &request,
                            ReconnectPrepareDispositionV1::Unavailable
                        )
                    ))?,
                    ReconnectPrepareActionV1::RetrySameRequest(request.clone())
                );
                assert_eq!(
                    checked(flow.accept_prepare_completion(
                        ReconnectPrepareCompletionV1::for_request(
                            &request,
                            v1.ok_or("missing V1 disposition")?
                        )
                    ))?,
                    ReconnectPrepareActionV1::AwaitFinalRevalidation
                );
                let result =
                    flow.authorize_commit(live.bind(record)?, live.time("authorization_at")?);
                assert_eq!(
                    flow.phase(),
                    if result.is_ok() {
                        ReconnectDurabilityPhaseV1::PendingCommit
                    } else {
                        ReconnectDurabilityPhaseV1::Terminal
                    }
                );
                result.is_ok()
            } else {
                let (mut flow, request) =
                    ReconnectDurabilityFlowV2::begin(record.clone(), replacement.clone());
                let mut budget = v2_budget(seed)?;
                assert_eq!(
                    checked(flow.accept_prepare_completion(
                        ReconnectPrepareCompletionV2::for_request(
                            &request,
                            ReconnectPrepareDispositionV2::Unavailable
                        ),
                        &mut budget
                    ))?,
                    ReconnectPrepareActionV2::RetrySameRequest(request.clone())
                );
                assert_eq!(
                    checked(flow.accept_prepare_completion(
                        ReconnectPrepareCompletionV2::for_request(&request, v2),
                        &mut budget
                    ))?,
                    ReconnectPrepareActionV2::AwaitFinalRevalidation
                );
                assert_eq!(
                    budget.distinct_attempts(),
                    1,
                    "retry consumed another attempt"
                );
                let result =
                    flow.authorize_commit(live.bind(record)?, live.time("authorization_at")?);
                assert_eq!(
                    flow.phase(),
                    if result.is_ok() {
                        ReconnectDurabilityPhaseV1::PendingCommit
                    } else {
                        ReconnectDurabilityPhaseV1::Terminal
                    }
                );
                result.is_ok()
            };
            Ok(accepted)
        };
        assert!(exercise_retry(source)?, "retry positive {boundary:?}");
        for (invariant, operator) in registered_cases(boundary) {
            let live = mutated(source, invariant, operator, seed)?;
            assert!(
                !exercise_retry(&live)?,
                "retry granted {boundary:?}/{invariant:?}/{operator:?}"
            );
            executed.push(format!("{}/{invariant:?}/{operator:?}", boundary.label()));
        }
    }
    println!(
        "retry authority matrix: {} isolated revalidation cases",
        executed.len()
    );
    Ok(executed)
}

/// Historical terminal evidence retains its exact meaning despite changed live
/// facts. Live equality is explicitly N/A, but controller installation is forbidden.
pub fn run_terminal_matrix(
    seed: Seed,
    record: &ReconnectDurabilityRecordV1,
    source: &LiveSource,
    v1: ReconnectDurableReconciliationSnapshotV1,
    v2: ReconnectDurableReconciliationSnapshotV2,
    disposition: ReconnectDurableTerminalDispositionV1,
) -> TestResult<usize> {
    assert_eq!(
        v2.outcome(),
        ReconnectDurableOutcomeV2::Terminal { disposition }
    );
    let project = |live: &LiveSource| -> TestResult<()> {
        assert_eq!(
            reconcile_v1(record, live.bind(record)?, v1.clone())?,
            ReconnectProjectionDecisionV1::Terminal
        );
        assert_eq!(
            reconcile_v2(record, live.bind(record)?, v2.clone(), seed)?,
            ReconnectProjectionDecisionV2::Terminal { disposition }
        );
        Ok(())
    };
    project(source)?;
    let mut count = 0;
    for &invariant in AuthorityInvariant::ALL {
        for &operator in invariant.operators() {
            project(&mutated(source, invariant, operator, seed)?)?;
            count += 2;
        }
    }
    println!(
        "historical terminal {disposition:?}: {count} changed-source projections, exact reasons/no controller"
    );
    Ok(count)
}

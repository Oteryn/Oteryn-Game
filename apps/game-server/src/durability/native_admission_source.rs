//! Durable non-rollback state for the authenticated native-admission source.

use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::runtime_scope_assignment::{
    NodeIncarnationProof, NodeRegistrationFact, prove_current_incarnation,
};
use super::{DurabilityError, DurabilityRoot};
use sqlx::{Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, DurabilityError>;

const SOURCE_AUTHORITY_BYTES: usize = 128;
const DESCRIPTOR_BYTES: usize = 4096;
const PENDING_CHECKPOINT_BYTES: usize = 16_384;
const JSON_STRING_BYTES: usize = 256;
const SIGNING_KEY_ID_BYTES: usize = 64;
const HTTP_BODY_BYTES: usize = 8192;
const FRESH_ISSUER: &str = "urn:oteryn:platform:game-admission";
const FRESH_PROFILE: &str = "oteryn-pre-admission-v1";
const RECOVERY_ISSUER: &str = "urn:oteryn:platform:game-recovery";
const RECOVERY_PROFILE: &str = "oteryn-reauth-recovery-v1";
const RECOVERY_KEY_PURPOSE: &str = "existing_actor_recovery";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSourceOperation {
    ReadAccountSecurityV1,
    ReadFreshSigningTrustV1,
    ReadRecoveryAccountSecurityV2,
    ReadRecoverySigningTrustV2,
}

impl NativeSourceOperation {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "ReadAccountSecurityV1" => Ok(Self::ReadAccountSecurityV1),
            "ReadFreshSigningTrustV1" => Ok(Self::ReadFreshSigningTrustV1),
            "ReadRecoveryAccountSecurityV2" => Ok(Self::ReadRecoveryAccountSecurityV2),
            "ReadRecoverySigningTrustV2" => Ok(Self::ReadRecoverySigningTrustV2),
            _ => Err(DurabilityError::Unavailable),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::ReadAccountSecurityV1 => "ReadAccountSecurityV1",
            Self::ReadFreshSigningTrustV1 => "ReadFreshSigningTrustV1",
            Self::ReadRecoveryAccountSecurityV2 => "ReadRecoveryAccountSecurityV2",
            Self::ReadRecoverySigningTrustV2 => "ReadRecoverySigningTrustV2",
        }
    }

    fn is_account(self) -> bool {
        matches!(
            self,
            Self::ReadAccountSecurityV1 | Self::ReadRecoveryAccountSecurityV2
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeSourceSubjectKind {
    AccountSecurity {
        account_id: String,
    },
    SigningTrust {
        issuer: String,
        profile: String,
        key_purpose: String,
        key_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSourceSubject {
    kind: NativeSourceSubjectKind,
}

impl NativeSourceSubject {
    pub fn account_security(account_id: impl Into<String>) -> Result<Self> {
        let account_id = account_id.into();
        if !canonical_uuid_v7(&account_id) {
            return Err(DurabilityError::Unavailable);
        }
        Ok(Self {
            kind: NativeSourceSubjectKind::AccountSecurity { account_id },
        })
    }

    pub fn signing_trust(
        issuer: impl Into<String>,
        profile: impl Into<String>,
        key_purpose: impl Into<String>,
        key_id: impl Into<String>,
    ) -> Result<Self> {
        let subject = Self {
            kind: NativeSourceSubjectKind::SigningTrust {
                issuer: issuer.into(),
                profile: profile.into(),
                key_purpose: key_purpose.into(),
                key_id: key_id.into(),
            },
        };
        if !subject.valid_strings() {
            return Err(DurabilityError::Unavailable);
        }
        Ok(subject)
    }

    fn valid_strings(&self) -> bool {
        match &self.kind {
            NativeSourceSubjectKind::AccountSecurity { account_id } => {
                canonical_uuid_v7(account_id)
            }
            NativeSourceSubjectKind::SigningTrust {
                issuer,
                profile,
                key_purpose,
                key_id,
            } => {
                [issuer, profile, key_purpose]
                    .iter()
                    .all(|value| valid_bounded_text(value, JSON_STRING_BYTES))
                    && valid_signing_key_id(key_id)
            }
        }
    }

    fn valid_for(&self, operation: NativeSourceOperation) -> bool {
        match (&self.kind, operation) {
            (NativeSourceSubjectKind::AccountSecurity { .. }, op) => op.is_account(),
            (
                NativeSourceSubjectKind::SigningTrust {
                    issuer,
                    profile,
                    key_purpose,
                    ..
                },
                NativeSourceOperation::ReadFreshSigningTrustV1,
            ) => issuer == FRESH_ISSUER && profile == FRESH_PROFILE && !key_purpose.is_empty(),
            (
                NativeSourceSubjectKind::SigningTrust {
                    issuer,
                    profile,
                    key_purpose,
                    ..
                },
                NativeSourceOperation::ReadRecoverySigningTrustV2,
            ) => {
                issuer == RECOVERY_ISSUER
                    && profile == RECOVERY_PROFILE
                    && key_purpose == RECOVERY_KEY_PURPOSE
            }
            _ => false,
        }
    }

    fn floor_key(&self) -> String {
        match &self.kind {
            NativeSourceSubjectKind::AccountSecurity { account_id } => {
                format!("account:{account_id}")
            }
            NativeSourceSubjectKind::SigningTrust {
                issuer,
                profile,
                key_purpose,
                ..
            } => length_bound_tuple("trust", &[issuer, profile, key_purpose]),
        }
    }

    fn observation_key(&self) -> String {
        match &self.kind {
            NativeSourceSubjectKind::AccountSecurity { account_id } => {
                format!("account:{account_id}")
            }
            NativeSourceSubjectKind::SigningTrust {
                issuer,
                profile,
                key_purpose,
                key_id,
            } => length_bound_tuple("trust-observation", &[issuer, profile, key_purpose, key_id]),
        }
    }

    fn signing_key_id(&self) -> Option<&str> {
        match &self.kind {
            NativeSourceSubjectKind::AccountSecurity { .. } => None,
            NativeSourceSubjectKind::SigningTrust { key_id, .. } => Some(key_id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshStoreProvenance {
    pub namespace: String,
    pub authorization: String,
    pub source_authority: String,
    pub initialized_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorRegistration {
    pub revision: u64,
    pub facts: Vec<u8>,
    pub installed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceObservation {
    pub source_authority: String,
    pub operation: NativeSourceOperation,
    pub subject: NativeSourceSubject,
    pub source_revision: u64,
    pub decision_identity: String,
    pub observed_at: i64,
    pub semantic_facts: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPublication {
    pub slot_id: i16,
    pub operation_binding: Vec<u8>,
    pub checkpointed_at: i64,
}

fn valid_bounded_text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max
}

fn valid_source_authority(value: &str) -> bool {
    valid_bounded_text(value, SOURCE_AUTHORITY_BYTES)
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
}

fn valid_signing_key_id(value: &str) -> bool {
    valid_bounded_text(value, SIGNING_KEY_ID_BYTES)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn canonical_uuid_v7(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && bytes[8] == b'-'
        && bytes[13] == b'-'
        && bytes[18] == b'-'
        && bytes[23] == b'-'
        && bytes[14] == b'7'
        && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
        && bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 8 | 13 | 18 | 23)
                || byte.is_ascii_digit()
                || matches!(byte, b'a'..=b'f')
        })
        && value != "00000000-0000-7000-8000-000000000000"
}

fn length_bound_tuple(prefix: &str, values: &[&String]) -> String {
    let mut result = String::from(prefix);
    for value in values {
        result.push(':');
        result.push_str(&value.len().to_string());
        result.push(':');
        result.push_str(value);
    }
    result
}

fn valid_descriptor(value: &DescriptorRegistration) -> bool {
    value.revision > 0
        && !value.facts.is_empty()
        && value.facts.len() <= DESCRIPTOR_BYTES
        && value.installed_at >= 0
}

fn valid_observation(value: &SourceObservation) -> bool {
    valid_source_authority(&value.source_authority)
        && value.subject.valid_strings()
        && value.subject.valid_for(value.operation)
        && value.source_revision > 0
        && valid_bounded_text(&value.decision_identity, JSON_STRING_BYTES)
        && value.observed_at >= 0
        && !value.semantic_facts.is_empty()
        && value.semantic_facts.len() <= HTTP_BODY_BYTES
}

/// Database-visible custody fence shared by every native-source mutation and
/// reconciliation read. The caller must prove possession of the exact current
/// #415 incarnation (share-locked until commit, so a concurrent revoke or
/// supersession serializes with this transaction) and be the durable custody
/// holder. Process-local state, a retained fact or persisted source state alone
/// never establishes custody.
pub(super) async fn fence_custody(
    tx: &mut Transaction<'_, Postgres>,
    custody: &NodeIncarnationProof,
) -> Result<()> {
    if !prove_current_incarnation(tx, custody).await? {
        return Err(DurabilityError::Unavailable);
    }
    let custody = custody.fact();
    let holder = sqlx::query(
        "SELECT uuid_send(custody_node_id), custody_registration_revision::text \
         FROM game_durability_native_source_registration WHERE registration_id=1 FOR UPDATE",
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(DurabilityError::Unavailable)?;
    let node: Vec<u8> = holder
        .try_get(0)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let revision: String = holder
        .try_get(1)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if node.as_slice() != custody.node_id().as_bytes().as_slice()
        || revision != custody.registration_revision().to_string()
    {
        return Err(DurabilityError::Unavailable);
    }
    Ok(())
}

impl DurabilityRoot {
    pub async fn initialize_native_admission_source(
        &self,
        custody: &NodeIncarnationProof,
        provenance: FreshStoreProvenance,
        descriptor: DescriptorRegistration,
    ) -> Result<()> {
        let custody = custody.clone();
        if !valid_bounded_text(&provenance.namespace, JSON_STRING_BYTES)
            || !valid_bounded_text(&provenance.authorization, JSON_STRING_BYTES)
            || !valid_source_authority(&provenance.source_authority)
            || provenance.initialized_at < 0
            || !valid_descriptor(&descriptor)
        {
            return Err(DurabilityError::Unavailable);
        }
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            if !prove_current_incarnation(&mut tx, &custody).await? { return Err(DurabilityError::Unavailable); }
            // The runtime cannot fabricate or select the initial source and
            // trust descriptor: an exact control-plane issuance must exist.
            let issued: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_native_source_descriptor_issuances WHERE descriptor_revision=$1::text::numeric(20,0) AND descriptor_facts=$2 AND installed_at=$3 AND source_authority=$4 AND bootstrap_namespace=$5 AND bootstrap_provenance=$6 AND initialized_at=$7)")
                .bind(descriptor.revision.to_string()).bind(&descriptor.facts).bind(descriptor.installed_at).bind(&provenance.source_authority).bind(&provenance.namespace).bind(&provenance.authorization).bind(provenance.initialized_at).fetch_one(&mut *tx).await?;
            if !issued { return Err(DurabilityError::Unavailable); }
            let custody = custody.fact();
            let inserted = sqlx::query("INSERT INTO game_durability_native_source_registration (registration_id,bootstrap_namespace,bootstrap_provenance,source_authority,descriptor_revision,descriptor_facts,initialized_at,custody_node_id,custody_registration_revision) VALUES (1,$1,$2,$3,$4::text::numeric(20,0),$5,$6,encode($7,'hex')::uuid,$8::text::numeric(20,0)) ON CONFLICT DO NOTHING")
                .bind(&provenance.namespace).bind(&provenance.authorization).bind(&provenance.source_authority).bind(descriptor.revision.to_string()).bind(&descriptor.facts).bind(provenance.initialized_at).bind(custody.node_id().as_bytes().as_slice()).bind(custody.registration_revision().to_string()).execute(&mut *tx).await?.rows_affected();
            if inserted == 0 { return Err(DurabilityError::Unavailable); }
            sqlx::query("INSERT INTO game_durability_native_source_descriptor_history (registration_id,descriptor_revision,descriptor_facts,installed_at) VALUES (1,$1::text::numeric(20,0),$2,$3)")
                .bind(descriptor.revision.to_string()).bind(&descriptor.facts).bind(descriptor.installed_at).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_durability_native_source_publication_slots (registration_id,slot_id) VALUES (1,1),(1,2)").execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    /// Explicitly move custody to the caller's current incarnation. Allowed only
    /// when the prior holder is no longer a current registration (revoked or
    /// superseded); a live holder keeps exclusive custody.
    pub async fn claim_native_admission_source_custody(
        &self,
        custody: &NodeIncarnationProof,
    ) -> Result<()> {
        let custody = custody.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            if !prove_current_incarnation(&mut tx, &custody).await? { return Err(DurabilityError::Unavailable); }
            let custody = custody.fact();
            let row = sqlx::query("SELECT uuid_send(custody_node_id), custody_registration_revision::text FROM game_durability_native_source_registration WHERE registration_id=1 FOR UPDATE")
                .fetch_optional(&mut *tx).await?.ok_or(DurabilityError::Unavailable)?;
            let node: Vec<u8> = row.try_get(0).map_err(|_| DurabilityError::InvalidStoredState)?;
            let revision: u64 = row.try_get::<String,_>(1).map_err(|_| DurabilityError::InvalidStoredState)?.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
            let prior = NodeRegistrationFact::new(
                oteryn_game_server::foundation::NodeId::decode(&node).map_err(|_| DurabilityError::InvalidStoredState)?,
                revision,
            );
            if prior == custody { return commit_semantic_transaction(tx, deadline).await; }
            // A live prior holder keeps exclusive custody (control-plane currentness check).
            let prior_current: bool = sqlx::query_scalar("SELECT game_node_lock_current_registration(encode($1,'hex')::uuid, $2::text::numeric(20,0))")
                .bind(prior.node_id().as_bytes().as_slice()).bind(prior.registration_revision().to_string()).fetch_one(&mut *tx).await?;
            if prior_current { return Err(DurabilityError::Unavailable); }
            sqlx::query("UPDATE game_durability_native_source_registration SET custody_node_id=encode($1,'hex')::uuid, custody_registration_revision=$2::text::numeric(20,0) WHERE registration_id=1")
                .bind(custody.node_id().as_bytes().as_slice()).bind(custody.registration_revision().to_string()).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    pub async fn register_native_admission_descriptor(
        &self,
        custody: &NodeIncarnationProof,
        descriptor: DescriptorRegistration,
    ) -> Result<()> {
        let custody = custody.clone();
        if !valid_descriptor(&descriptor) {
            return Err(DurabilityError::Unavailable);
        }
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            fence_custody(&mut tx, &custody).await?;
            let row = sqlx::query("SELECT r.descriptor_revision::text,r.descriptor_facts,h.installed_at FROM game_durability_native_source_registration r JOIN game_durability_native_source_descriptor_history h USING (registration_id,descriptor_revision) WHERE r.registration_id=1 FOR UPDATE OF r").fetch_optional(&mut *tx).await?.ok_or(DurabilityError::Unavailable)?;
            let current: u64 = row.try_get::<String,_>(0).map_err(|_| DurabilityError::InvalidStoredState)?.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
            let facts: Vec<u8> = row.try_get(1).map_err(|_| DurabilityError::InvalidStoredState)?;
            let installed_at: i64 = row.try_get(2).map_err(|_| DurabilityError::InvalidStoredState)?;
            if descriptor.revision < current || (descriptor.revision == current && (descriptor.facts != facts || descriptor.installed_at != installed_at)) { return Err(DurabilityError::Unavailable); }
            // Only the latest control-plane issuance, under the store's fixed
            // source authority, may be registered or confirmed.
            let latest: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM (SELECT * FROM game_native_source_descriptor_issuances ORDER BY descriptor_revision DESC LIMIT 1) i JOIN game_durability_native_source_registration r ON r.registration_id=1 AND r.source_authority=i.source_authority WHERE i.descriptor_revision=$1::text::numeric(20,0) AND i.descriptor_facts=$2 AND i.installed_at=$3)")
                .bind(descriptor.revision.to_string()).bind(&descriptor.facts).bind(descriptor.installed_at).fetch_one(&mut *tx).await?;
            if !latest { return Err(DurabilityError::Unavailable); }
            if descriptor.revision == current { return commit_semantic_transaction(tx, deadline).await; }
            sqlx::query("INSERT INTO game_durability_native_source_descriptor_history (registration_id,descriptor_revision,descriptor_facts,installed_at) VALUES (1,$1::text::numeric(20,0),$2,$3)").bind(descriptor.revision.to_string()).bind(&descriptor.facts).bind(descriptor.installed_at).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_durability_native_source_registration SET descriptor_revision=$1::text::numeric(20,0),descriptor_facts=$2 WHERE registration_id=1").bind(descriptor.revision.to_string()).bind(&descriptor.facts).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    /// Control-plane: durably record one Platform producer descriptor issuance
    /// (OPS-NODE-BOOT-01 D2). `provenance` is present only on the issuance
    /// that authorizes a fresh S2 store. An exact replay succeeds; a
    /// conflicting, stale or authority-changing issuance is `Ok(false)`.
    pub async fn record_native_source_descriptor_issuance(
        &self,
        source_authority: &str,
        descriptor: DescriptorRegistration,
        provenance: Option<FreshStoreProvenance>,
    ) -> Result<bool> {
        if !valid_source_authority(source_authority)
            || !valid_descriptor(&descriptor)
            || provenance.as_ref().is_some_and(|provenance| {
                provenance.source_authority != source_authority
                    || !valid_bounded_text(&provenance.namespace, JSON_STRING_BYTES)
                    || !valid_bounded_text(&provenance.authorization, JSON_STRING_BYTES)
                    || provenance.initialized_at < 0
            })
        {
            return Err(DurabilityError::Unavailable);
        }
        let source_authority = source_authority.to_owned();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let recorded = sqlx::query("SELECT game_native_source_record_issuance($1::text::numeric(20,0),$2,$3,$4,$5,$6,$7)")
                .bind(descriptor.revision.to_string()).bind(&descriptor.facts).bind(descriptor.installed_at).bind(&source_authority)
                .bind(provenance.as_ref().map(|p| p.namespace.as_str())).bind(provenance.as_ref().map(|p| p.authorization.as_str())).bind(provenance.as_ref().map(|p| p.initialized_at))
                .execute(&mut *tx).await;
            match recorded {
                Ok(_) => {}
                Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("OTN03") => return Ok(false),
                Err(error) => return Err(error.into()),
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(true)
        })).await
    }

    /// Read-only: the stored S2 bootstrap provenance and current descriptor,
    /// or `None` for an uninitialized store. Used to compare a supplied
    /// fresh-store authorization after an ambiguous initialization.
    pub async fn read_native_admission_source_registration(
        &self,
    ) -> Result<Option<(FreshStoreProvenance, DescriptorRegistration)>> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row = sqlx::query("SELECT r.bootstrap_namespace,r.bootstrap_provenance,r.source_authority,r.initialized_at,r.descriptor_revision::text,r.descriptor_facts,h.installed_at FROM game_durability_native_source_registration r JOIN game_durability_native_source_descriptor_history h USING (registration_id,descriptor_revision) WHERE r.registration_id=1")
                .fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            let Some(row) = row else { return Ok(None) };
            let invalid = |_| DurabilityError::InvalidStoredState;
            Ok(Some((
                FreshStoreProvenance {
                    namespace: row.try_get(0).map_err(invalid)?,
                    authorization: row.try_get(1).map_err(invalid)?,
                    source_authority: row.try_get(2).map_err(invalid)?,
                    initialized_at: row.try_get(3).map_err(invalid)?,
                },
                DescriptorRegistration {
                    revision: row.try_get::<String, _>(4).map_err(invalid)?.parse().map_err(|_| DurabilityError::InvalidStoredState)?,
                    facts: row.try_get(5).map_err(invalid)?,
                    installed_at: row.try_get(6).map_err(invalid)?,
                },
            )))
        })).await
    }

    pub async fn accept_native_source_observation(
        &self,
        custody: &NodeIncarnationProof,
        observation: SourceObservation,
    ) -> Result<()> {
        let custody = custody.clone();
        if !valid_observation(&observation) {
            return Err(DurabilityError::Unavailable);
        }
        let operation = observation.operation.as_str();
        let floor_subject = observation.subject.floor_key();
        let observation_subject = observation.subject.observation_key();
        let signing_key_id = observation.subject.signing_key_id().map(str::to_owned);
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            fence_custody(&mut tx, &custody).await?;
            let registered = sqlx::query_scalar::<_, String>("SELECT source_authority FROM game_durability_native_source_registration WHERE registration_id=1 FOR UPDATE")
                .fetch_optional(&mut *tx).await?.ok_or(DurabilityError::Unavailable)?;
            if registered != observation.source_authority { return Err(DurabilityError::Unavailable); }
            let row = sqlx::query("SELECT source_revision::text,operation,observation_subject,decision_identity,observed_at,semantic_facts FROM game_durability_native_source_floors WHERE registration_id=1 AND source_authority=$1 AND floor_subject=$2 FOR UPDATE")
                .bind(&observation.source_authority).bind(&floor_subject).fetch_optional(&mut *tx).await?;
            if let Some(row) = row {
                let revision: u64 = row.try_get::<String,_>(0).map_err(|_| DurabilityError::InvalidStoredState)?.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
                let exact = row.try_get::<String,_>(1).ok().as_deref() == Some(operation)
                    && row.try_get::<String,_>(2).ok().as_deref() == Some(observation_subject.as_str())
                    && row.try_get::<String,_>(3).ok().as_deref() == Some(observation.decision_identity.as_str())
                    && row.try_get::<i64,_>(4).ok() == Some(observation.observed_at)
                    && row.try_get::<Vec<u8>,_>(5).ok().as_deref() == Some(observation.semantic_facts.as_slice());
                if observation.source_revision < revision || (observation.source_revision == revision && !exact) { return Err(DurabilityError::Unavailable); }
                if observation.source_revision == revision { return commit_semantic_transaction(tx, deadline).await; }
                sqlx::query("INSERT INTO game_durability_native_source_observation_history (registration_id,source_authority,operation,floor_subject,observation_subject,signing_key_id,source_revision,decision_identity,observed_at,semantic_facts) VALUES (1,$1,$2,$3,$4,$5,$6::text::numeric(20,0),$7,$8,$9)").bind(&observation.source_authority).bind(operation).bind(&floor_subject).bind(&observation_subject).bind(signing_key_id.as_deref()).bind(observation.source_revision.to_string()).bind(&observation.decision_identity).bind(observation.observed_at).bind(&observation.semantic_facts).execute(&mut *tx).await?;
                sqlx::query("UPDATE game_durability_native_source_floors SET operation=$3,observation_subject=$4,signing_key_id=$5,source_revision=$6::text::numeric(20,0),decision_identity=$7,observed_at=$8,semantic_facts=$9 WHERE registration_id=1 AND source_authority=$1 AND floor_subject=$2").bind(&observation.source_authority).bind(&floor_subject).bind(operation).bind(&observation_subject).bind(signing_key_id.as_deref()).bind(observation.source_revision.to_string()).bind(&observation.decision_identity).bind(observation.observed_at).bind(&observation.semantic_facts).execute(&mut *tx).await?;
            } else {
                sqlx::query("INSERT INTO game_durability_native_source_observation_history (registration_id,source_authority,operation,floor_subject,observation_subject,signing_key_id,source_revision,decision_identity,observed_at,semantic_facts) VALUES (1,$1,$2,$3,$4,$5,$6::text::numeric(20,0),$7,$8,$9)").bind(&observation.source_authority).bind(operation).bind(&floor_subject).bind(&observation_subject).bind(signing_key_id.as_deref()).bind(observation.source_revision.to_string()).bind(&observation.decision_identity).bind(observation.observed_at).bind(&observation.semantic_facts).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_durability_native_source_floors (registration_id,source_authority,operation,floor_subject,observation_subject,signing_key_id,source_revision,decision_identity,observed_at,semantic_facts) VALUES (1,$1,$2,$3,$4,$5,$6::text::numeric(20,0),$7,$8,$9)").bind(&observation.source_authority).bind(operation).bind(&floor_subject).bind(&observation_subject).bind(signing_key_id.as_deref()).bind(observation.source_revision.to_string()).bind(&observation.decision_identity).bind(observation.observed_at).bind(&observation.semantic_facts).execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    pub async fn checkpoint_native_source_publication(
        &self,
        custody: &NodeIncarnationProof,
        operation_binding: Vec<u8>,
        checkpointed_at: i64,
    ) -> Result<i16> {
        let custody = custody.clone();
        if operation_binding.is_empty()
            || operation_binding.len() > PENDING_CHECKPOINT_BYTES
            || checkpointed_at < 0
        {
            return Err(DurabilityError::Unavailable);
        }
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            fence_custody(&mut tx, &custody).await?;
            if let Some(slot) = sqlx::query_scalar::<_,i16>("SELECT slot_id FROM game_durability_native_source_publication_slots WHERE registration_id=1 AND operation_binding=$1").bind(&operation_binding).fetch_optional(&mut *tx).await? { commit_semantic_transaction(tx, deadline).await?; return Ok(slot); }
            let slot = sqlx::query_scalar::<_,i16>("SELECT slot_id FROM game_durability_native_source_publication_slots WHERE registration_id=1 AND operation_binding IS NULL ORDER BY slot_id FOR UPDATE SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await?.ok_or(DurabilityError::Unavailable)?;
            sqlx::query("UPDATE game_durability_native_source_publication_slots SET operation_binding=$1,checkpointed_at=$2 WHERE registration_id=1 AND slot_id=$3 AND operation_binding IS NULL").bind(&operation_binding).bind(checkpointed_at).bind(slot).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?; Ok(slot)
        })).await
    }

    pub async fn clear_native_source_publication(
        &self,
        custody: &NodeIncarnationProof,
        slot_id: i16,
        operation_binding: Vec<u8>,
    ) -> Result<()> {
        let custody = custody.clone();
        if !matches!(slot_id, 1 | 2)
            || operation_binding.is_empty()
            || operation_binding.len() > PENDING_CHECKPOINT_BYTES
        {
            return Err(DurabilityError::Unavailable);
        }
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            fence_custody(&mut tx, &custody).await?;
            let changed = sqlx::query("UPDATE game_durability_native_source_publication_slots SET operation_binding=NULL,checkpointed_at=NULL WHERE registration_id=1 AND slot_id=$1 AND operation_binding=$2").bind(slot_id).bind(operation_binding).execute(&mut *tx).await?.rows_affected();
            if changed != 1 { return Err(DurabilityError::Unavailable); }
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    pub async fn pending_native_source_publications(
        &self,
        custody: &NodeIncarnationProof,
    ) -> Result<Vec<PendingPublication>> {
        let custody = custody.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            fence_custody(&mut tx, &custody).await?;
            let rows = sqlx::query("SELECT slot_id,operation_binding,checkpointed_at FROM game_durability_native_source_publication_slots WHERE registration_id=1 AND operation_binding IS NOT NULL ORDER BY slot_id").fetch_all(&mut *tx).await?;
            let publications: Vec<PendingPublication> = rows.into_iter().map(|row| Ok(PendingPublication { slot_id: row.try_get(0).map_err(|_| DurabilityError::InvalidStoredState)?, operation_binding: row.try_get(1).map_err(|_| DurabilityError::InvalidStoredState)?, checkpointed_at: row.try_get(2).map_err(|_| DurabilityError::InvalidStoredState)? })).collect::<Result<_>>()?;
            let aggregate = publications.iter().try_fold(0usize, |total, publication| total.checked_add(publication.operation_binding.len())).ok_or(DurabilityError::InvalidStoredState)?;
            if publications.len() > 2 || aggregate > 2 * PENDING_CHECKPOINT_BYTES || publications.iter().any(|publication| publication.operation_binding.is_empty() || publication.operation_binding.len() > PENDING_CHECKPOINT_BYTES) {
                return Err(DurabilityError::InvalidStoredState);
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(publications)
        })).await
    }
}

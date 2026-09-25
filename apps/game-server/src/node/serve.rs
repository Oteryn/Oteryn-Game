//! `oteryn-game-server serve --config <path>`: the boot sequence of one
//! serving GameNode (OPS-NODE-BOOT-01 D3).
//!
//! The node holds only the runtime database credential. It registers exactly
//! one incarnation, establishes S2 custody, opens Character authority, waits
//! for the operator's assignment, binds, publishes readiness and then serves
//! gameplay, the Character bootstrap control socket and audit expiry until a
//! signal or the end of any loop. Events are single stderr lines without
//! secrets (D6).

use super::config::NodeConfig;
use super::descriptor_facts::{MAX_PEM_BYTES, certificates, descriptor_facts};
use super::operator_files::{
    LaunchAuthorizationFile, S2AuthorizationFile, hex, hex_bytes, uuid_v7,
};
use super::secure_file::{FileClass, effective_uid};
use super::{StartupError, connect_root, read_file, uuid_text};
use crate::character_recovery_fence::CharacterRecoveryStore;
use crate::durability::DurabilityRoot;
use crate::durability::admission_authority_guards::{
    AdmissionGuardStore, GuardPublicationDisposition,
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::native_admission_source::{DescriptorRegistration, FreshStoreProvenance};
use crate::durability::runtime_scope_assignment::{
    AssignmentError, AssignmentState, BootstrapSecret, LaunchBinding, NodeIncarnationProof,
    RegistrationError,
};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
    AdmissionAuthorityOwningPublisherV1, AdmissionAuthorityPublicationChangeV1,
    AdmissionAuthorityPublicationErrorV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
    AdmissionPublicationSourceV1,
};
use crate::foundation::{ChannelId, NodeId, RuntimeScopeRefV1, WorldId};
use crate::gameplay_transport::FreshEvidenceSource;
use crate::native_admission_source::descriptor::ProducerDescriptor;
use crate::native_admission_source::{CHARACTER_BOOTSTRAP_INTENT_ISSUER, TransientCapacity};
use crate::{GameplayListenerConfig, GameplaySeamOwners, serve_gameplay};
use oteryn_foundation::CancellationToken;
use rustls::pki_types::PrivateKeyDer;
use rustls::pki_types::pem::PemObject;
use std::future::{Future, poll_fn};
use std::path::Path;
use std::pin::pin;
use std::task::Poll;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UnixListener, UnixStream};

/// Registered protocol facts fixed by the build (D5).
const PROTOCOL_MAJOR: u64 = 1;
const TRANSPORT_PROFILE: u64 = 1;
/// Accepted clock uncertainty bound for readiness `source_observed_at` (D3).
const CLOCK_WAIT_BOUND_SECONDS: i64 = 5;
const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(10 * 60);
const MAINTENANCE_POLL: Duration = Duration::from_secs(1);
const RETRY_BACKOFF_MAX: Duration = Duration::from_secs(5);
const ASSIGNMENT_POLL: Duration = Duration::from_secs(1);
const PENDING_RECONCILE_ATTEMPTS: u32 = 5;
const SHUTDOWN_BUDGET: Duration = Duration::from_secs(10);
const DRAIN_MARGIN: Duration = Duration::from_secs(1);
const AUDIT_INTERVAL: Duration = Duration::from_secs(60 * 60);
const AUDIT_BATCH: u16 = 64;
const AUDIT_RUN_CAP: u32 = 64;
const CONTROL_REQUEST_BYTES: usize = 64;
const CONTROL_DEADLINE: Duration = Duration::from_secs(30);
const MAX_GAMEPLAY_KEY_BYTES: usize = 16 * 1024;

/// A boot failure; every variant exits non-zero with a distinct code.
#[derive(Debug)]
pub enum BootError {
    Startup(StartupError),
    Registration,
    SourceCustody(&'static str),
    CharacterAuthority,
    AssignmentWait,
    Bind(&'static str),
    Readiness(&'static str),
    ClockSkew,
    Serve,
}

impl BootError {
    /// Closed process exit codes.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Startup(StartupError::Database(_)) => 11,
            Self::Startup(_) => 10,
            Self::Registration => 12,
            Self::SourceCustody(_) => 13,
            Self::CharacterAuthority => 14,
            Self::AssignmentWait => 15,
            Self::Bind(_) => 16,
            Self::Readiness(_) | Self::ClockSkew => 17,
            Self::Serve => 18,
        }
    }
}

impl std::fmt::Display for BootError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Startup(error) => write!(formatter, "{error}"),
            Self::Registration => formatter.write_str("registration rejected"),
            Self::SourceCustody(stage) => write!(formatter, "S2 custody failed at {stage}"),
            Self::CharacterAuthority => formatter.write_str("Character authority unavailable"),
            Self::AssignmentWait => formatter.write_str("no operator assignment within the wait"),
            Self::Bind(what) => write!(formatter, "bind failed: {what}"),
            Self::Readiness(stage) => write!(formatter, "readiness failed at {stage}"),
            Self::ClockSkew => formatter.write_str("clock skew beyond the uncertainty bound"),
            Self::Serve => formatter.write_str("gameplay listener configuration rejected"),
        }
    }
}

impl From<StartupError> for BootError {
    fn from(error: StartupError) -> Self {
        Self::Startup(error)
    }
}

/// One structured stderr event line (D6). Only non-secret values are passed.
fn event(line: &str) {
    eprintln!("oteryn-game-server {line}");
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or(0)
}

fn invalid(key: &'static str) -> BootError {
    BootError::Startup(StartupError::Invalid { key })
}

/// Everything read before any socket is bound (D1).
struct Material {
    config: NodeConfig,
    gameplay_chain: Vec<rustls::pki_types::CertificateDer<'static>>,
    gameplay_key: PrivateKeyDer<'static>,
    evidence: FreshEvidenceSource,
    intents: ProducerDescriptor,
    descriptor: DescriptorRegistration,
    launch: LaunchAuthorizationFile,
    s2: Option<S2AuthorizationFile>,
    world: WorldId,
    channel: ChannelId,
}

fn private_key(key: &'static str, pem: &[u8]) -> Result<PrivateKeyDer<'static>, BootError> {
    PrivateKeyDer::from_pem_slice(pem).map_err(|_| invalid(key))
}

fn load(config_path: &Path) -> Result<Material, BootError> {
    let owner = effective_uid();
    let document = read_file(
        "--config",
        config_path,
        FileClass::Trusted,
        owner,
        super::config::MAX_CONFIG_BYTES,
    )?;
    let config = NodeConfig::parse(&document).map_err(|error| invalid(error.key))?;
    let secret = |key: &'static str, path: &Path, max: usize| {
        read_file(key, path, FileClass::Secret, owner, max)
    };
    let gameplay_chain = certificates(&secret(
        "listener.certificate_chain_file",
        &config.listener.certificate_chain_file,
        MAX_PEM_BYTES,
    )?)
    .map_err(|_| invalid("listener.certificate_chain_file"))?;
    let gameplay_key = private_key(
        "listener.private_key_file",
        &secret(
            "listener.private_key_file",
            &config.listener.private_key_file,
            MAX_GAMEPLAY_KEY_BYTES,
        )?,
    )?;
    // The pair must form the served TLS configuration (D3 step 1), so a
    // mismatched chain and key fail before registration or readiness.
    crate::gameplay_transport::validate_gameplay_tls(&gameplay_chain, &gameplay_key)
        .map_err(|_| invalid("listener.private_key_file"))?;
    let platform = &config.platform;
    let roots = certificates(&secret(
        "platform.trust_roots_file",
        &platform.trust_roots_file,
        MAX_PEM_BYTES,
    )?)
    .map_err(|_| invalid("platform.trust_roots_file"))?;
    let client = certificates(&secret(
        "platform.client_certificate_file",
        &platform.client_certificate_file,
        MAX_PEM_BYTES,
    )?)
    .map_err(|_| invalid("platform.client_certificate_file"))?;
    let client_key = private_key(
        "platform.client_key_file",
        &secret(
            "platform.client_key_file",
            &platform.client_key_file,
            MAX_GAMEPLAY_KEY_BYTES,
        )?,
    )?;
    let endpoint = (platform.endpoint.ip().to_string(), platform.endpoint.port());
    // One channel, two authenticated namespaces: the S1 evidence authority
    // and the compiled Character intent issuer (D1).
    let evidence = ProducerDescriptor::new(
        platform.source_authority.clone(),
        endpoint.clone(),
        platform.peer_name.clone(),
        platform.peer_name.clone(),
        roots.clone(),
        client.clone(),
        client_key.clone_key(),
    )
    .map_err(|_| invalid("platform"))?;
    let intents = ProducerDescriptor::new(
        CHARACTER_BOOTSTRAP_INTENT_ISSUER.into(),
        endpoint,
        platform.peer_name.clone(),
        platform.peer_name.clone(),
        roots.clone(),
        client.clone(),
        client_key,
    )
    .map_err(|_| invalid("platform"))?;
    let descriptor = DescriptorRegistration {
        revision: platform.descriptor_revision,
        facts: descriptor_facts(
            &platform.source_authority,
            platform.endpoint,
            &platform.peer_name,
            &roots,
            &client,
        ),
        installed_at: platform.installed_at,
    };
    let launch = LaunchAuthorizationFile::decode(&secret(
        "launch.authorization_file",
        &config.launch.authorization_file,
        super::operator_files::MAX_OPERATOR_FILE_BYTES,
    )?)
    .map_err(|_| invalid("launch.authorization_file"))?;
    let s2 = match &config.launch.s2_authorization_file {
        Some(path) => Some(
            S2AuthorizationFile::decode(&secret(
                "launch.s2_authorization_file",
                path,
                super::operator_files::MAX_OPERATOR_FILE_BYTES,
            )?)
            .map_err(|_| invalid("launch.s2_authorization_file"))?,
        ),
        None => None,
    };
    let world = WorldId::decode(
        &super::uuid_bytes(&config.scope.world_id).ok_or_else(|| invalid("scope.world_id"))?,
    )
    .map_err(|_| invalid("scope.world_id"))?;
    let channel = ChannelId::decode(
        &super::uuid_bytes(&config.scope.channel_id).ok_or_else(|| invalid("scope.channel_id"))?,
    )
    .map_err(|_| invalid("scope.channel_id"))?;
    Ok(Material {
        config,
        gameplay_chain,
        gameplay_key,
        evidence: FreshEvidenceSource::new(evidence),
        intents,
        descriptor,
        launch,
        s2,
        world,
        channel,
    })
}

/// Output of `primary`, or `None` once `stop` completes first.
async fn first<T>(primary: impl Future<Output = T>, stop: impl Future<Output = ()>) -> Option<T> {
    let mut primary = pin!(primary);
    let mut stop = pin!(stop);
    poll_fn(|context| {
        if let Poll::Ready(value) = primary.as_mut().poll(context) {
            return Poll::Ready(Some(value));
        }
        stop.as_mut().poll(context).map(|()| None)
    })
    .await
}

async fn backoff(attempt: &mut u32) {
    let delay = Duration::from_millis(200_u64.saturating_mul(1 << (*attempt).min(5)));
    *attempt = attempt.saturating_add(1);
    tokio::time::sleep(delay.min(RETRY_BACKOFF_MAX)).await;
}

/// Re-establish the durability holder after any reported demand, and at a
/// fixed interval well inside the 30-minute holder lifetime (D3 step 2).
async fn maintain(root: DurabilityRoot, stop: CancellationToken) {
    let mut since = tokio::time::Instant::now();
    while !stop.is_cancelled() {
        if first(tokio::time::sleep(MAINTENANCE_POLL), stop.cancelled())
            .await
            .is_none()
        {
            return;
        }
        if since.elapsed() >= MAINTENANCE_INTERVAL {
            root.request_ready();
            since = tokio::time::Instant::now();
        }
        if root.has_ready_demand() && root.maintain_ready_once().await.is_err() {
            root.request_ready();
        }
    }
}

/// D3 step 3: register exactly one `NodeId`, replaying the identical request
/// through any ambiguous outcome until it is definite.
async fn register(
    root: &DurabilityRoot,
    launch: &LaunchAuthorizationFile,
) -> Result<NodeIncarnationProof, BootError> {
    let secret = BootstrapSecret::from_bytes(
        hex_bytes::<32>(&launch.secret).map_err(|_| invalid("launch.authorization_file"))?,
    );
    let binding = LaunchBinding::new(&launch.launch_binding)
        .map_err(|_| invalid("launch.authorization_file"))?;
    let node_bytes = uuid_v7().map_err(|_| BootError::Registration)?;
    let node = NodeId::decode(&node_bytes).map_err(|_| BootError::Registration)?;
    event(&format!(
        "event=registering node_id={}",
        uuid_text(&node_bytes)
    ));
    let mut attempt = 0;
    loop {
        match root
            .register_node_incarnation(&secret, &binding, node)
            .await
        {
            Ok(proof) => {
                event(&format!(
                    "event=registered node_id={} registration_revision={}",
                    uuid_text(&node_bytes),
                    proof.fact().registration_revision()
                ));
                return Ok(proof);
            }
            Err(RegistrationError::Rejected | RegistrationError::NotCurrent) => {
                return Err(BootError::Registration);
            }
            Err(RegistrationError::Unavailable(_)) => {
                root.request_ready();
                backoff(&mut attempt).await;
            }
        }
    }
}

fn s2_parts(
    file: &S2AuthorizationFile,
) -> Result<(FreshStoreProvenance, DescriptorRegistration), BootError> {
    let facts_hex = &file.descriptor_facts;
    if !facts_hex.len().is_multiple_of(2) || facts_hex.len() > 8192 {
        return Err(invalid("launch.s2_authorization_file"));
    }
    let facts = (0..facts_hex.len() / 2)
        .map(|index| hex_bytes::<1>(&facts_hex[index * 2..index * 2 + 2]).map(|byte| byte[0]))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|_| invalid("launch.s2_authorization_file"))?;
    Ok((
        FreshStoreProvenance {
            namespace: file.namespace.clone(),
            authorization: file.authorization.clone(),
            source_authority: file.source_authority.clone(),
            initialized_at: file.initialized_at,
        },
        DescriptorRegistration {
            revision: file.descriptor_revision,
            facts,
            installed_at: file.installed_at,
        },
    ))
}

/// Retry an S2 operation through transient unavailability, a bounded number
/// of times. Definite rejections surface as errors.
async fn with_retries<T, F, Fut>(root: &DurabilityRoot, mut operation: F) -> Option<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, crate::durability::DurabilityError>>,
{
    let mut attempt = 0;
    for _ in 0..PENDING_RECONCILE_ATTEMPTS {
        match operation().await {
            Ok(value) => return Some(value),
            Err(crate::durability::DurabilityError::RootUnavailable) => {
                root.request_ready();
                let _ = root.maintain_ready_once().await;
                backoff(&mut attempt).await;
            }
            Err(_) => return None,
        }
    }
    None
}

/// D3 step 4: establish S2 custody, confirm the recorded descriptor and
/// reconcile retained publication slots.
async fn establish_custody(
    root: &DurabilityRoot,
    proof: &NodeIncarnationProof,
    material: &Material,
    evidence: &FreshEvidenceSource,
) -> Result<(), BootError> {
    let stored = with_retries(root, || root.read_native_admission_source_registration())
        .await
        .ok_or(BootError::SourceCustody("read"))?;
    match (stored, &material.s2) {
        (None, None) => {
            return Err(BootError::SourceCustody(
                "fresh-store authorization missing",
            ));
        }
        (None, Some(file)) => {
            let (provenance, descriptor) = s2_parts(file)?;
            let initialized = root
                .initialize_native_admission_source(proof, provenance.clone(), descriptor.clone())
                .await;
            if initialized.is_err() {
                // An initialization whose response was lost: compare exactly.
                let after = with_retries(root, || root.read_native_admission_source_registration())
                    .await
                    .flatten()
                    .ok_or(BootError::SourceCustody("initialize"))?;
                if after != (provenance, descriptor) {
                    return Err(BootError::SourceCustody("initialize"));
                }
            }
        }
        (Some((stored, _)), Some(file)) => {
            let (provenance, _) = s2_parts(file)?;
            if stored != provenance {
                return Err(BootError::SourceCustody(
                    "fresh-store authorization differs",
                ));
            }
        }
        (Some(_), None) => {}
    }
    root.claim_native_admission_source_custody(proof)
        .await
        .map_err(|_| BootError::SourceCustody("custody claim"))?;
    root.register_native_admission_descriptor(proof, material.descriptor.clone())
        .await
        .map_err(|_| BootError::SourceCustody("descriptor"))?;
    for attempt in 0..PENDING_RECONCILE_ATTEMPTS {
        match evidence.reconcile_retained(root, proof).await {
            Ok(0) => {
                event("event=s2_custody state=established retained_slots=0");
                return Ok(());
            }
            Ok(_) | Err(_) => {
                let mut step = attempt;
                backoff(&mut step).await;
            }
        }
    }
    Err(BootError::SourceCustody("retained publication slots"))
}

/// D3 step 6: the operator assigns exactly this incarnation within the wait.
async fn await_assignment(
    root: &DurabilityRoot,
    proof: &NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    wait: Duration,
) -> Result<u64, BootError> {
    let fact = proof.fact();
    event(&format!(
        "event=awaiting_assignment node_id={} registration_revision={}",
        uuid_text(fact.node_id().as_bytes()),
        fact.registration_revision()
    ));
    let deadline = tokio::time::Instant::now() + wait;
    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(assignment)) = root.read_runtime_scope_assignment(scope).await
            && assignment.state == AssignmentState::Assigned
            && assignment.holder == Some(fact)
        {
            event(&format!(
                "event=assignment_received ownership_generation={}",
                assignment.ownership_generation
            ));
            return Ok(assignment.ownership_generation);
        }
        tokio::time::sleep(ASSIGNMENT_POLL).await;
    }
    Err(BootError::AssignmentWait)
}

/// D3 step 7: the control-socket directory is the service user's, mode 0700,
/// not writable by others. Only a stale socket of the service user is removed.
fn prepare_control_socket(path: &Path) -> Result<(), BootError> {
    let owner = effective_uid();
    let parent = path
        .parent()
        .ok_or(BootError::Bind("control socket path"))?;
    let directory = rustix::fs::openat(
        rustix::fs::CWD,
        parent,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(|_| BootError::Bind("control socket directory"))?;
    let stat =
        rustix::fs::fstat(&directory).map_err(|_| BootError::Bind("control socket directory"))?;
    if stat.st_uid != owner || stat.st_mode & 0o077 != 0 {
        return Err(BootError::Bind(
            "control socket directory ownership or mode",
        ));
    }
    let name = path
        .file_name()
        .ok_or(BootError::Bind("control socket path"))?;
    match rustix::fs::statat(&directory, name, rustix::fs::AtFlags::SYMLINK_NOFOLLOW) {
        Err(rustix::io::Errno::NOENT) => Ok(()),
        Ok(existing)
            if rustix::fs::FileType::from_raw_mode(existing.st_mode)
                == rustix::fs::FileType::Socket
                && existing.st_uid == owner =>
        {
            rustix::fs::unlinkat(&directory, name, rustix::fs::AtFlags::empty())
                .map_err(|_| BootError::Bind("stale control socket"))
        }
        _ => Err(BootError::Bind("foreign file at the control socket path")),
    }
}

/// The node's own readiness publication (D3 step 8).
struct RuntimeReadiness(AdmissionAuthorityPublicationChangeV1);
impl crate::foundation::fnd04_verifier::fresh_source_sealed::Sealed for RuntimeReadiness {}
impl AdmissionAuthorityOwningPublisherV1 for RuntimeReadiness {
    fn resolve_publication(
        &self,
        _now: i64,
    ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
    {
        Ok(vec![self.0.clone()])
    }
}

struct Readiness<'a> {
    root: &'a DurabilityRoot,
    proof: &'a NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    generation: u64,
    config: &'a NodeConfig,
}

impl Readiness<'_> {
    /// Prepare the exact successor of the current Runtime guard chain.
    async fn prepare(
        &self,
        ready: bool,
        budget: Option<tokio::time::Instant>,
    ) -> Result<AdmissionAuthorityPublicationV1, BootError> {
        let key = AdmissionAuthorityGuardKeyV1::Runtime(self.scope);
        let guards = AdmissionGuardStore::from_root(self.root.clone());
        // History without a current guard is invalid stored state (D3).
        let current = guards
            .load(std::slice::from_ref(&key))
            .await
            .map_err(|_| BootError::Readiness("guard chain"))?
            .pop()
            .flatten();
        let (precondition, publication_revision, source_revision, previous_observed) =
            match &current {
                None => (
                    AdmissionPublicationPreconditionV1::Bootstrap {
                        restored_publication_high_water: Some(0),
                    },
                    1,
                    1,
                    None,
                ),
                Some(current) => (
                    AdmissionPublicationPreconditionV1::CompareAndSet {
                        expected_publication_revision: current.publication_revision,
                    },
                    current
                        .publication_revision
                        .checked_add(1)
                        .ok_or(BootError::Readiness("publication revision"))?,
                    current
                        .source
                        .source_revision
                        .checked_add(1)
                        .ok_or(BootError::Readiness("source revision"))?,
                    Some(current.source.source_observed_at),
                ),
            };
        // Never decreasing and never future-dated: wait out a small lead.
        let mut now = unix_now();
        if let Some(previous) = previous_observed
            && previous > now
        {
            if previous - now > CLOCK_WAIT_BOUND_SECONDS {
                return Err(BootError::ClockSkew);
            }
            let wait = Duration::from_secs(u64::try_from(previous - now + 1).unwrap_or(1));
            if budget.is_some_and(|deadline| tokio::time::Instant::now() + wait > deadline) {
                return Err(BootError::Readiness("clock wait exceeds the budget"));
            }
            tokio::time::sleep(wait).await;
            now = unix_now();
        }
        let fact = self.proof.fact();
        let readiness = &self.config.readiness;
        let change = AdmissionAuthorityPublicationChangeV1 {
            key,
            source: AdmissionPublicationSourceV1 {
                authority: readiness.source_authority.clone(),
                purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
                source_revision,
                decision_identity: format!(
                    "runtime-readiness:{}:{}:{}:{}",
                    hex(fact.node_id().as_bytes()),
                    self.generation,
                    source_revision,
                    ready
                ),
                source_observed_at: now,
                clock_uncertainty_seconds: 0,
            },
            precondition,
            publication_revision,
            state: AdmissionAuthorityGuardStateV1::Runtime {
                ownership_generation: self.generation,
                ready,
                route_revision: readiness.route_revision.clone(),
                runtime_observation_revision: readiness.runtime_observation_revision.clone(),
                protocol_major: PROTOCOL_MAJOR,
                transport_profile: TRANSPORT_PROFILE,
                ruleset_revision: readiness.ruleset_revision.clone(),
                content_revision: readiness.content_revision.clone(),
                map_revision: readiness.map_revision.clone(),
                world_policy_revision: readiness.world_policy_revision.clone(),
                offer_revision: readiness.offer_revision.clone(),
            },
        };
        AdmissionAuthorityPublicationV1::prepare(&RuntimeReadiness(change), now)
            .map_err(|_| BootError::Readiness("prepare"))
    }

    /// Publish; a stale or conflicting CAS rereads once. An ambiguous outcome
    /// replays the exact prepared publication until it is definite (or the
    /// shutdown budget ends).
    async fn publish(
        &self,
        ready: bool,
        budget: Option<tokio::time::Instant>,
    ) -> Result<(), BootError> {
        for _ in 0..2 {
            let publication = self.prepare(ready, budget).await?;
            let mut attempt = 0;
            loop {
                if budget.is_some_and(|deadline| tokio::time::Instant::now() >= deadline) {
                    return Err(BootError::Readiness("shutdown budget"));
                }
                match self
                    .root
                    .publish_runtime_readiness(self.proof, &publication)
                    .await
                {
                    Ok(
                        GuardPublicationDisposition::Applied
                        | GuardPublicationDisposition::Existing,
                    ) => {
                        event(&format!(
                            "event=readiness ready={ready} ownership_generation={}",
                            self.generation
                        ));
                        return Ok(());
                    }
                    Ok(
                        GuardPublicationDisposition::Stale | GuardPublicationDisposition::Conflict,
                    ) => break,
                    Err(
                        AssignmentError::NotCurrentHolder
                        | AssignmentError::InvalidInput
                        | AssignmentError::Unsupported,
                    ) => {
                        return Err(BootError::Readiness("not the current holder"));
                    }
                    Err(_) => {
                        self.root.request_ready();
                        backoff(&mut attempt).await;
                    }
                }
            }
        }
        Err(BootError::Readiness("stale or conflicting publication"))
    }
}

/// The closed control-socket result (D2).
async fn bootstrap_one(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    evidence: &FreshEvidenceSource,
    intents: &ProducerDescriptor,
    capacity: &TransientCapacity,
    operation_id: [u8; 16],
) -> &'static str {
    // A committed result is answered without any external read.
    match root
        .reconcile_character_bootstrap(authority, operation_id)
        .await
    {
        Ok(Some(_)) => return "committed",
        Ok(None) => {}
        Err(_) => return "unavailable",
    }
    let intent = {
        let Ok(mut permit) = capacity.try_queue() else {
            return "unavailable";
        };
        if permit.try_activate().is_err() {
            return "unavailable";
        }
        match crate::character_bootstrap_intent::read_authenticated_intent(
            intents,
            operation_id,
            &mut permit,
        )
        .await
        {
            Ok(intent) => intent,
            Err(_) => return "unavailable",
        }
    };
    let account = uuid_text(intent.account_id().as_bytes());
    // Current account security is fetched into S2 through the D4 lane; an
    // authenticated denial is accepted there and then rejects the bootstrap.
    if evidence
        .refresh_account(root, proof, &account)
        .await
        .is_err()
    {
        return "unavailable";
    }
    match root.bootstrap_character(authority, proof, &intent).await {
        Ok(_) => "committed",
        Err(
            crate::durability::character_authority::CharacterAuthorityError::Rejected
            | crate::durability::character_authority::CharacterAuthorityError::Conflict,
        ) => "rejected",
        Err(_) => "unavailable",
    }
}

fn parse_operation_id(request: &[u8]) -> Option<[u8; 16]> {
    let text = std::str::from_utf8(request).ok()?.trim_end_matches('\n');
    let bytes = super::uuid_bytes(text)?;
    (bytes[6] >> 4 == 7 && bytes[8] & 0xc0 == 0x80).then_some(bytes)
}

async fn handle_control(stream: &mut UnixStream) -> Option<[u8; 16]> {
    let mut request = Vec::with_capacity(CONTROL_REQUEST_BYTES);
    let mut limited = (&mut *stream).take(CONTROL_REQUEST_BYTES as u64 + 1);
    limited.read_to_end(&mut request).await.ok()?;
    if request.len() > CONTROL_REQUEST_BYTES {
        return None;
    }
    parse_operation_id(&request)
}

/// One request at a time, bounded size and deadline; only uid 0 is served.
#[allow(clippy::too_many_arguments)]
async fn control_loop(
    listener: &UnixListener,
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    evidence: &FreshEvidenceSource,
    intents: &ProducerDescriptor,
    stop: &CancellationToken,
) {
    let capacity = TransientCapacity::new();
    while let Some(accepted) = first(listener.accept(), stop.cancelled()).await {
        let Ok((mut stream, _)) = accepted else {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };
        // The peer is checked before any byte is read.
        if !stream
            .peer_cred()
            .is_ok_and(|credentials| credentials.uid() == 0)
        {
            continue;
        }
        let served = tokio::time::timeout(CONTROL_DEADLINE, async {
            let answer = match handle_control(&mut stream).await {
                Some(operation) => {
                    bootstrap_one(
                        root, authority, proof, evidence, intents, &capacity, operation,
                    )
                    .await
                }
                None => "rejected",
            };
            event(&format!("event=character_bootstrap result={answer}"));
            let _ = stream.write_all(format!("{answer}\n").as_bytes()).await;
            let _ = stream.shutdown().await;
        })
        .await;
        if served.is_err() {
            event("event=character_bootstrap result=unavailable reason=deadline");
        }
    }
}

/// Ordinary Character audit expiry at start and then at a fixed interval
/// (D3 step 9). A failed run is logged and retried at the next interval.
async fn audit_expiry_loop(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    stop: &CancellationToken,
) {
    loop {
        let mut deleted = 0_u64;
        let mut failed = false;
        for _ in 0..AUDIT_RUN_CAP {
            match root.expire_character_audit(authority, AUDIT_BATCH).await {
                Ok(count) => {
                    deleted = deleted.saturating_add(count);
                    if count < u64::from(AUDIT_BATCH) {
                        break;
                    }
                }
                Err(_) => {
                    failed = true;
                    break;
                }
            }
        }
        event(&format!(
            "event=audit_expiry deleted={deleted} result={}",
            if failed { "failed" } else { "ok" }
        ));
        if first(tokio::time::sleep(AUDIT_INTERVAL), stop.cancelled())
            .await
            .is_none()
        {
            return;
        }
    }
}

/// Resolves on SIGTERM or SIGINT.
async fn termination() {
    use tokio::signal::unix::{SignalKind, signal};
    let (Ok(mut terminate), Ok(mut interrupt)) = (
        signal(SignalKind::terminate()),
        signal(SignalKind::interrupt()),
    ) else {
        // Without signal delivery the node cannot shut down gracefully.
        std::future::pending::<()>().await;
        return;
    };
    let mut terminate = pin!(terminate.recv());
    let mut interrupt = pin!(interrupt.recv());
    poll_fn(|context| {
        if terminate.as_mut().poll(context).is_ready()
            || interrupt.as_mut().poll(context).is_ready()
        {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
}

/// Run the serving node until shutdown.
pub async fn run(config_path: &Path) -> Result<(), BootError> {
    // D3 step 1: everything is read and checked before a socket is bound.
    let material = load(config_path)?;
    let config = &material.config;
    event(&format!(
        "event=configuration_accepted world_id={} channel_id={}",
        config.scope.world_id, config.scope.channel_id
    ));
    let signalled = CancellationToken::new();
    let watcher = {
        let signalled = signalled.clone();
        tokio::spawn(async move {
            termination().await;
            signalled.cancel();
        })
    };
    // D3 step 2.
    let root = connect_root(&config.database, effective_uid()).await?;
    let stop_maintenance = CancellationToken::new();
    let maintenance = tokio::spawn(maintain(root.clone(), stop_maintenance.clone()));
    let result = boot_and_serve(&root, &material, &signalled).await;
    stop_maintenance.cancel();
    let _ = maintenance.await;
    watcher.abort();
    result
}

/// The drain after readiness is withdrawn: long enough for an in-flight
/// admission (bounded by the entry deadline) or Character bootstrap (bounded
/// by the control deadline) to reach its own outcome.
fn drain_budget(entry_deadline: Duration) -> Duration {
    CONTROL_DEADLINE.max(entry_deadline) + DRAIN_MARGIN
}

/// A signal before readiness was ever published: nothing to withdraw.
fn stopped_before_ready() {
    event("event=shutdown reason=\"signal before ready\"");
    event("event=shutdown state=complete");
}

async fn boot_and_serve(
    root: &DurabilityRoot,
    material: &Material,
    signalled: &CancellationToken,
) -> Result<(), BootError> {
    let config = &material.config;
    let scope = RuntimeScopeRefV1::channel(material.world, material.channel);
    let Some(proof) = first(register(root, &material.launch), signalled.cancelled()).await else {
        stopped_before_ready();
        return Ok(());
    };
    let proof = proof?;
    let evidence = &material.evidence;
    establish_custody(root, &proof, material, evidence).await?;
    // D3 step 5.
    let store = CharacterRecoveryStore::open(
        &config.character.fence_directory,
        config.character.authority_scope_id.clone(),
        config.character.issuer_identity.clone(),
    )
    .map_err(|_| BootError::CharacterAuthority)?;
    let seal = store
        .seal_current()
        .map_err(|_| BootError::CharacterAuthority)?;
    let authority = root
        .open_character_authority(&seal)
        .await
        .map_err(|_| BootError::CharacterAuthority)?;
    event("event=character_authority state=open");
    let Some(generation) = first(
        await_assignment(
            root,
            &proof,
            scope,
            Duration::from_millis(config.scope.assignment_wait_ms),
        ),
        signalled.cancelled(),
    )
    .await
    else {
        stopped_before_ready();
        return Ok(());
    };
    let generation = generation?;
    if signalled.is_cancelled() {
        stopped_before_ready();
        return Ok(());
    }
    // D3 step 7: both sockets are bound before readiness.
    let listener = TcpListener::bind(config.listener.address)
        .await
        .map_err(|_| BootError::Bind("gameplay listener"))?;
    prepare_control_socket(&config.control.socket_path)?;
    let control = UnixListener::bind(&config.control.socket_path)
        .map_err(|_| BootError::Bind("control socket"))?;
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            &config.control.socket_path,
            std::fs::Permissions::from_mode(0o600),
        )
        .map_err(|_| BootError::Bind("control socket mode"))?;
    }
    event(&format!(
        "event=listener_bound address={}",
        config.listener.address
    ));
    let readiness = Readiness {
        root,
        proof: &proof,
        scope,
        generation,
        config,
    };
    // A signal while binding: never publish ready after shutdown was requested.
    if signalled.is_cancelled() {
        let _ = std::fs::remove_file(&config.control.socket_path);
        stopped_before_ready();
        return Ok(());
    }
    readiness.publish(true, None).await?;
    // D3 step 9: three loops under one shutdown token.
    let shutdown = CancellationToken::new();
    let listener_config = GameplayListenerConfig {
        certificates: material.gameplay_chain.clone(),
        private_key: material.gameplay_key.clone_key(),
        max_connections: usize::try_from(config.listener.max_connections)
            .map_err(|_| BootError::Serve)?,
        max_handshake_units: usize::try_from(config.listener.max_handshake_units)
            .map_err(|_| BootError::Serve)?,
        entry_deadline: Duration::from_millis(config.listener.entry_deadline_ms),
    };
    let owners = GameplaySeamOwners {
        root,
        character: &authority,
        holder: &proof,
        evidence,
        world_id: material.world,
        channel_id: material.channel,
    };
    let loops_stop = CancellationToken::new();
    let mut gameplay = pin!(serve_gameplay(
        &listener,
        listener_config,
        owners,
        &shutdown
    ));
    let mut control_task = pin!(control_loop(
        &control,
        root,
        &authority,
        &proof,
        evidence,
        &material.intents,
        &shutdown,
    ));
    let mut expiry = pin!(audit_expiry_loop(root, &authority, &loops_stop));
    let mut signal = pin!(signalled.cancelled());
    let mut serve_error = None;
    let reason = poll_fn(|context| {
        if signal.as_mut().poll(context).is_ready() {
            return Poll::Ready("signal");
        }
        if let Poll::Ready(result) = gameplay.as_mut().poll(context) {
            serve_error = result.err();
            return Poll::Ready("gameplay loop ended");
        }
        if control_task.as_mut().poll(context).is_ready() {
            return Poll::Ready("control loop ended");
        }
        if expiry.as_mut().poll(context).is_ready() {
            return Poll::Ready("audit expiry loop ended");
        }
        Poll::Pending
    })
    .await;
    event(&format!("event=shutdown reason=\"{reason}\""));
    // D3 step 10: withdraw readiness first, within the shutdown budget.
    let budget = tokio::time::Instant::now() + SHUTDOWN_BUDGET;
    if readiness.publish(false, Some(budget)).await.is_err() {
        event("event=readiness ready=false result=failed");
    }
    shutdown.cancel();
    loops_stop.cancel();
    // In-flight admissions and an in-flight bootstrap complete within their
    // own deadlines, which bound the drain.
    let _ = first(
        async {
            let _ = gameplay.as_mut().await;
            control_task.as_mut().await;
            expiry.as_mut().await;
        },
        tokio::time::sleep(drain_budget(Duration::from_millis(
            config.listener.entry_deadline_ms,
        ))),
    )
    .await;
    let _ = std::fs::remove_file(&config.control.socket_path);
    event("event=shutdown state=complete");
    if serve_error.is_some() {
        return Err(BootError::Serve);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn operation_ids_must_be_single_canonical_uuid_v7_lines() {
        assert!(parse_operation_id(b"01890f4c-3b2a-7c01-8d11-9a321b7c0004\n").is_some());
        assert!(parse_operation_id(b"01890f4c-3b2a-4c01-8d11-9a321b7c0004").is_none());
        assert!(parse_operation_id(b"bootstrap everything").is_none());
        assert!(parse_operation_id(b"01890F4C-3B2A-7C01-8D11-9A321B7C0004").is_none());
    }

    #[test]
    fn gameplay_tls_requires_a_matching_chain_and_key() {
        let issue = || {
            rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).expect("certificate")
        };
        let (first, second) = (issue(), issue());
        let chain = vec![first.cert.der().clone()];
        let key = |pair: &rcgen::CertifiedKey<rcgen::KeyPair>| {
            PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(
                pair.signing_key.serialize_der(),
            ))
        };
        let (own, foreign) = (key(&first), key(&second));
        assert!(crate::gameplay_transport::validate_gameplay_tls(&chain, &own).is_ok());
        assert!(crate::gameplay_transport::validate_gameplay_tls(&chain, &foreign).is_err());
    }

    #[test]
    fn drain_outlasts_every_in_flight_deadline() {
        assert!(drain_budget(Duration::from_secs(1)) > CONTROL_DEADLINE);
        let longest = Duration::from_millis(super::super::config::MAX_ENTRY_DEADLINE_MS);
        assert!(drain_budget(longest) > longest);
    }

    #[test]
    fn exit_codes_are_distinct_per_class() {
        let codes = [
            BootError::Startup(StartupError::Invalid { key: "x" }).exit_code(),
            BootError::Startup(StartupError::Database("x")).exit_code(),
            BootError::Registration.exit_code(),
            BootError::SourceCustody("x").exit_code(),
            BootError::CharacterAuthority.exit_code(),
            BootError::AssignmentWait.exit_code(),
            BootError::Bind("x").exit_code(),
            BootError::Readiness("x").exit_code(),
            BootError::Serve.exit_code(),
        ];
        let unique: std::collections::BTreeSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len());
    }
}

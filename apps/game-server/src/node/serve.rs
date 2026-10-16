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
use crate::content::{
    ContentActivationController, NativeEntryActivationError, NativeEntryActivationIssuance,
    NativeEntryContentPin, NodeBootQuiescence, activate_native_entry_room,
};
use crate::durability::DurabilityRoot;
use crate::durability::admission_authority_guards::{
    AdmissionGuardStore, GuardPublicationDisposition,
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_command::{Classified, classify_intent};
use crate::durability::native_admission_source::{DescriptorRegistration, FreshStoreProvenance};
use crate::durability::runtime_scope_assignment::{
    AssignmentError, AssignmentState, BootstrapSecret, LaunchBinding, NodeIncarnationProof,
    RegistrationError, RuntimeScopeAssignment,
};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
    AdmissionAuthorityOwningPublisherV1, AdmissionAuthorityPublicationChangeV1,
    AdmissionAuthorityPublicationErrorV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
    AdmissionPublicationSourceV1,
};
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, NodeId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
};
use crate::gameplay_transport::FreshEvidenceSource;
use crate::native_admission_source::account_characters::{
    self, EpochFence, EpochFenceFile, MtlsSink, ProjectionDescriptor, Publisher,
};
use crate::native_admission_source::character_commands::{
    CommandDescriptor, Consumer, Decider, Decision, Input, MtlsSource,
};
use crate::native_admission_source::descriptor::ProducerDescriptor;
use crate::native_admission_source::public_projections::{
    self, MtlsPublicSink, PublicProjectionDescriptor,
};
use crate::native_admission_source::runtime_status::{RuntimeStatusDescriptor, SystemClock};
use crate::native_admission_source::{CHARACTER_BOOTSTRAP_INTENT_ISSUER, TransientCapacity};
use crate::{GameplayListenerConfig, GameplaySeamOwners, serve_gameplay};
use oteryn_foundation::CancellationToken;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::future::{Future, poll_fn};
use std::path::Path;
use std::pin::pin;
use std::task::Poll;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UnixListener, UnixStream};
use tokio::sync::Mutex;

mod bundle_npcs;

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
/// The share of [`SHUTDOWN_BUDGET`] the shutdown last-position write may take (CHAR-POSITION-1).
const LAST_POSITION_FLUSH_BUDGET: Duration = Duration::from_secs(3);
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
    ContentActivation(&'static str),
    /// MAP-CUTOVER-1a: the configured world bundle failed a boot check.
    WorldBundle(crate::map::boot::BootRefusal),
}

oteryn_error_codes::error_kinds! {
    /// The registered code of each [`BootError`] variant.
    pub enum BootErrorKind {
        ConfigInvalid = (2001, "CONFIG_INVALID", InvalidInput, Terminal),
        DatabaseUnavailable = (2002, "BOOT_DATABASE_UNAVAILABLE", DependencyUnavailable, Retryable),
        RegistrationRejected = (2003, "BOOT_REGISTRATION_REJECTED", Conflict, Terminal),
        SourceCustodyFailed = (2004, "BOOT_SOURCE_CUSTODY_FAILED", InternalUnavailable, Terminal),
        CharacterAuthorityUnavailable = (2005, "BOOT_CHARACTER_AUTHORITY_UNAVAILABLE", DependencyUnavailable, Retryable),
        AssignmentWaitTimeout = (2006, "BOOT_ASSIGNMENT_WAIT_TIMEOUT", Timeout, Retryable),
        BindFailed = (2007, "BOOT_BIND_FAILED", DependencyUnavailable, Retryable),
        ReadinessFailed = (2008, "BOOT_READINESS_FAILED", DependencyUnavailable, Retryable),
        ClockSkew = (2009, "BOOT_CLOCK_SKEW", InternalUnavailable, Retryable),
        ServeConfigRejected = (2010, "BOOT_SERVE_CONFIG_REJECTED", InvalidInput, Terminal),
        ContentActivationRefused = (2011, "BOOT_CONTENT_ACTIVATION_REFUSED", Conflict, Terminal),
        WorldBundleRefused = (2012, "BOOT_WORLD_BUNDLE_REFUSED", InvalidInput, Terminal),
    }
}

oteryn_error_codes::error_kinds! {
    /// Node diagnostics codes that no [`BootError`] carries.
    pub enum LogKind {
        SpecInvalid = (2014, "LOG_SPEC_INVALID", InvalidInput, Terminal),
    }
}

impl BootError {
    /// The registered kind of this failure; the match has no wildcard arm.
    #[must_use]
    pub const fn kind(&self) -> BootErrorKind {
        match self {
            Self::Startup(StartupError::Database(_)) => BootErrorKind::DatabaseUnavailable,
            Self::Startup(_) => BootErrorKind::ConfigInvalid,
            Self::Registration => BootErrorKind::RegistrationRejected,
            Self::SourceCustody(_) => BootErrorKind::SourceCustodyFailed,
            Self::CharacterAuthority => BootErrorKind::CharacterAuthorityUnavailable,
            Self::AssignmentWait => BootErrorKind::AssignmentWaitTimeout,
            Self::Bind(_) => BootErrorKind::BindFailed,
            Self::Readiness(_) => BootErrorKind::ReadinessFailed,
            Self::ClockSkew => BootErrorKind::ClockSkew,
            Self::Serve => BootErrorKind::ServeConfigRejected,
            Self::ContentActivation(_) => BootErrorKind::ContentActivationRefused,
            Self::WorldBundle(_) => BootErrorKind::WorldBundleRefused,
        }
    }

    /// The registered code of this failure.
    #[must_use]
    pub const fn code(&self) -> oteryn_error_codes::ErrorCode {
        self.kind().code()
    }

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
            Self::ContentActivation(_) => 19,
            Self::WorldBundle(_) => 20,
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
            Self::ContentActivation(stage) => {
                write!(
                    formatter,
                    "native entry Content activation refused at {stage}"
                )
            }
            Self::WorldBundle(refusal) => write!(formatter, "world bundle refused: {refusal}"),
        }
    }
}

impl From<StartupError> for BootError {
    fn from(error: StartupError) -> Self {
        Self::Startup(error)
    }
}

static BOOT_TRACE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
const PROCESS: &str = "oteryn-game-server";

/// Write one leveled diagnostic line of `process` to stderr unless `OTERYN_LOG` drops it
/// (ERR-NODE-1). Warn, error and coded lines are always written.
pub fn emit(process: &str, line: &oteryn_error_codes::Line<'_>) {
    if oteryn_error_codes::filter().allows(line.level, line.module, line.code.is_some()) {
        eprintln!("{}", line.render(process, oteryn_error_codes::unix_ms()));
    }
}

/// Start of a game-server process, before anything else: mint the boot trace, install the
/// panic hook, read `OTERYN_LOG` once and write the `process_start` line. A malformed
/// `OTERYN_LOG` writes its coded line and returns the error; the caller exits.
pub fn begin_process(
    process: &'static str,
    version: &str,
    log_spec_code: oteryn_error_codes::ErrorCode,
) -> Result<String, oteryn_error_codes::LogSpecError> {
    use oteryn_error_codes::{Level, Line};
    let trace = uuid_v7().ok().map(|bytes| uuid_text(&bytes));
    let build = oteryn_error_codes::build_id(version);
    oteryn_error_codes::install_panic_hook(process, build.clone(), trace.clone());
    if let Some(trace) = &trace {
        let _ = BOOT_TRACE.set(trace.clone());
    }
    // A non-UTF-8 value is lossy-decoded: U+FFFD is in no spec, so it takes the malformed path.
    let spec = std::env::var_os("OTERYN_LOG").map(|value| value.to_string_lossy().into_owned());
    let filtered = oteryn_error_codes::init_filter(spec.as_deref());
    let start = Line::new(Level::Info, "process", "process_start").build(&build);
    // The first line of every process carries the build whatever the filter says.
    eprintln!(
        "{}",
        trace
            .as_deref()
            .map_or(start.clone(), |trace| start.clone().trace(trace))
            .render(process, oteryn_error_codes::unix_ms())
    );
    if let Err(error) = filtered {
        let detail = error.to_string();
        let mut line = Line::new(Level::Error, "process", "log_spec_invalid")
            .code(log_spec_code)
            .detail(&detail);
        if let Some(trace) = &trace {
            line = line.trace(trace);
        }
        emit(process, &line);
        return Err(error);
    }
    Ok(trace.unwrap_or_default())
}

/// The failure line of a one-shot tool: a registered SQLSTATE on the error's source chain gives
/// that code, any other error gives `default`, with the error text in `detail`.
pub fn tool_failure_line(
    process: &str,
    default: oteryn_error_codes::ErrorCode,
    error: &(dyn std::error::Error + 'static),
    trace: Option<&str>,
    ts_ms: u64,
) -> String {
    use oteryn_error_codes::{Level, Line};
    let code = crate::durability::sqlstate_codes::SqlstateKind::of_error(error)
        .map_or(default, |kind| kind.code());
    let detail = error.to_string();
    let mut line = Line::new(Level::Error, "tool", "failed")
        .code(code)
        .detail(&detail);
    if let Some(trace) = trace {
        line = line.trace(trace);
    }
    line.render(process, ts_ms)
}

/// Run a one-shot game-server tool: panic hook, `process_start`, `--version`, then `run`. A
/// failure writes exactly one coded line and exits 1.
pub fn run_tool(
    process: &'static str,
    version: &str,
    failure: oteryn_error_codes::ErrorCode,
    run: impl FnOnce() -> Result<(), Box<dyn std::error::Error>>,
) -> std::process::ExitCode {
    let Ok(trace) = begin_process(process, version, LogKind::SpecInvalid.code()) else {
        return std::process::ExitCode::from(2);
    };
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("{process} {}", oteryn_error_codes::build_id(version));
        return std::process::ExitCode::SUCCESS;
    }
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "{}",
                tool_failure_line(
                    process,
                    failure,
                    error.as_ref(),
                    Some(trace.as_str()).filter(|trace| !trace.is_empty()),
                    oteryn_error_codes::unix_ms(),
                )
            );
            std::process::ExitCode::from(1)
        }
    }
}

/// The `boot_failed` line: the code and the free-text reason in `detail`.
pub fn boot_failed_line(
    process: &str,
    error: &BootError,
    trace: Option<&str>,
    ts_ms: u64,
) -> String {
    use oteryn_error_codes::{Level, Line};
    let detail = error.to_string();
    let mut line = Line::new(Level::Error, "node", "boot_failed")
        .code(error.code())
        .detail(&detail);
    if let Some(trace) = trace {
        line = line.trace(trace);
    }
    line.render(process, ts_ms)
}

/// Write the `boot_failed` line for `error` (always written: it is coded).
pub fn report_boot_failure(error: &BootError) {
    eprintln!(
        "{}",
        boot_failed_line(
            PROCESS,
            error,
            BOOT_TRACE.get().map(String::as_str),
            oteryn_error_codes::unix_ms()
        )
    );
}

/// One structured stderr event line (D6). Only non-secret values are passed: the
/// `event=<name>` token becomes the `event` field and the rest is `detail`.
fn event(line: &str) {
    event_traced(line, BOOT_TRACE.get().map(String::as_str));
}

fn event_traced(line: &str, trace: Option<&str>) {
    let (level, module, event, detail) = split_event(line);
    let mut out = oteryn_error_codes::Line::new(level, module, event);
    if let Some(trace) = trace {
        out = out.trace(trace);
    }
    if !detail.is_empty() {
        out = out.detail(detail);
    }
    emit(PROCESS, &out);
}

/// `event=<name> rest` as level, module, event and detail.
fn split_event(line: &str) -> (oteryn_error_codes::Level, &'static str, &str, &str) {
    let (name, rest) = match line.strip_prefix("event=") {
        Some(tail) => tail.split_once(' ').unwrap_or((tail, "")),
        None => ("unnamed", line),
    };
    (oteryn_error_codes::Level::Info, "node", name, rest)
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or(0)
}

/// PREM-WIRE-1 production composition (PREMIUM-ACTIVATION-0 §1.1-§1.2): the one Premium fence,
/// the system trusted clock, the durable activation latch and no activation.
fn production_premium_reads(
    root: &DurabilityRoot,
) -> crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator {
    use crate::gameplay_transport::spell_premium_coordinator::{
        SpellPremiumCoordinator, SystemTrustedClock,
    };
    SpellPremiumCoordinator::new(
        std::sync::Arc::default(),
        Some(std::sync::Arc::new(SystemTrustedClock)),
        None,
        Some(root.clone()),
    )
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
    /// Runtime-status identity and declared epoch; `None` disables reporting.
    runtime_status: Option<(std::sync::Arc<RuntimeStatusDescriptor>, u64)>,
    /// `ListCharactersForAccount` publisher: its identity, source authority
    /// and epoch fence; `None` disables it. Taken once by the serving loop.
    account_characters:
        std::sync::Mutex<Option<(ProjectionDescriptor, String, std::path::PathBuf)>>,
    /// Character-command consumer identity and source authority; `None`
    /// (the default) disables it. Taken once by the serving loop.
    character_commands: std::sync::Mutex<Option<(CommandDescriptor, String)>>,
    /// Public-projection publisher: its identity, source authority and epoch
    /// fence `F_pub`; `None` (absent or not enabled) disables it. Taken once
    /// by the serving loop.
    public_projections:
        std::sync::Mutex<Option<(PublicProjectionDescriptor, String, std::path::PathBuf)>>,
    descriptor: DescriptorRegistration,
    launch: LaunchAuthorizationFile,
    s2: Option<S2AuthorizationFile>,
    world: WorldId,
    channel: ChannelId,
}

fn private_key(key: &'static str, pem: &[u8]) -> Result<PrivateKeyDer<'static>, BootError> {
    PrivateKeyDer::from_pem_slice(pem).map_err(|_| invalid(key))
}

/// The projection identity is its own, never the evidence or status one (§3); the
/// runtime-status chain is compared only when `[platform.runtime_status]` is configured.
fn projection_descriptor(
    endpoint: (String, u16),
    peer_name: String,
    roots: Vec<CertificateDer<'static>>,
    (chain, key): (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>),
    evidence: &[CertificateDer<'static>],
    status: Option<&[CertificateDer<'static>]>,
) -> Result<ProjectionDescriptor, BootError> {
    let others: Vec<&[CertificateDer<'static>]> = std::iter::once(evidence).chain(status).collect();
    ProjectionDescriptor::new(endpoint, peer_name, roots, chain, key, &others)
        .map_err(|_| invalid("platform.account_characters"))
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
    let runtime_status = match &platform.runtime_status {
        None => None,
        Some(status) => {
            let chain = certificates(&secret(
                "platform.runtime_status.client_certificate_file",
                &status.client_certificate_file,
                MAX_PEM_BYTES,
            )?)
            .map_err(|_| invalid("platform.runtime_status.client_certificate_file"))?;
            let key = private_key(
                "platform.runtime_status.client_key_file",
                &secret(
                    "platform.runtime_status.client_key_file",
                    &status.client_key_file,
                    MAX_GAMEPLAY_KEY_BYTES,
                )?,
            )?;
            // Its own identity, never the evidence one (contract §3).
            let descriptor = RuntimeStatusDescriptor::new(
                endpoint.clone(),
                platform.peer_name.clone(),
                roots.clone(),
                chain,
                key,
                &[&client],
            )
            .map_err(|_| invalid("platform.runtime_status"))?;
            Some((std::sync::Arc::new(descriptor), status.assignment_epoch))
        }
    };
    let account_characters = match &platform.account_characters {
        None => None,
        Some(projection) => {
            let chain = certificates(&secret(
                "platform.account_characters.client_certificate_file",
                &projection.client_certificate_file,
                MAX_PEM_BYTES,
            )?)
            .map_err(|_| invalid("platform.account_characters.client_certificate_file"))?;
            let key = private_key(
                "platform.account_characters.client_key_file",
                &secret(
                    "platform.account_characters.client_key_file",
                    &projection.client_key_file,
                    MAX_GAMEPLAY_KEY_BYTES,
                )?,
            )?;
            let status_chain = match &platform.runtime_status {
                None => None,
                Some(status) => Some(
                    certificates(&secret(
                        "platform.runtime_status.client_certificate_file",
                        &status.client_certificate_file,
                        MAX_PEM_BYTES,
                    )?)
                    .map_err(|_| invalid("platform.runtime_status.client_certificate_file"))?,
                ),
            };
            let descriptor = projection_descriptor(
                endpoint.clone(),
                platform.peer_name.clone(),
                roots.clone(),
                (chain, key),
                &client,
                status_chain.as_deref(),
            )?;
            Some((
                descriptor,
                projection.source_authority.clone(),
                projection.epoch_fence_file.clone(),
            ))
        }
    };
    let character_commands = match &platform.character_commands {
        None => None,
        Some(commands) => {
            let chain = |key: &'static str, path: &Path| {
                certificates(&secret(key, path, MAX_PEM_BYTES)?).map_err(|_| invalid(key))
            };
            let own = chain(
                "platform.character_commands.client_certificate_file",
                &commands.client_certificate_file,
            )?;
            let key = private_key(
                "platform.character_commands.client_key_file",
                &secret(
                    "platform.character_commands.client_key_file",
                    &commands.client_key_file,
                    MAX_GAMEPLAY_KEY_BYTES,
                )?,
            )?;
            let mut others = vec![client.clone()];
            if let Some(status) = &platform.runtime_status {
                others.push(chain(
                    "platform.runtime_status.client_certificate_file",
                    &status.client_certificate_file,
                )?);
            }
            if let Some(projection) = &platform.account_characters {
                others.push(chain(
                    "platform.account_characters.client_certificate_file",
                    &projection.client_certificate_file,
                )?);
            }
            let others: Vec<&[CertificateDer<'static>]> =
                others.iter().map(Vec::as_slice).collect();
            let descriptor = CommandDescriptor::new(
                endpoint.clone(),
                platform.peer_name.clone(),
                roots.clone(),
                own,
                key,
                &others,
            )
            .map_err(|_| invalid("platform.character_commands"))?;
            Some((descriptor, commands.source_authority.clone()))
        }
    };
    let public_projections = match &platform.public_projections {
        Some(public) if public.enabled => {
            let chain = |key: &'static str, path: &Path| {
                certificates(&secret(key, path, MAX_PEM_BYTES)?).map_err(|_| invalid(key))
            };
            let own = chain(
                "platform.public_projections.client_certificate_file",
                &public.client_certificate_file,
            )?;
            let key = private_key(
                "platform.public_projections.client_key_file",
                &secret(
                    "platform.public_projections.client_key_file",
                    &public.client_key_file,
                    MAX_GAMEPLAY_KEY_BYTES,
                )?,
            )?;
            // Its own identity, never one of the other purposes' (contract §3).
            let mut others = vec![client.clone()];
            if let Some(status) = &platform.runtime_status {
                others.push(chain(
                    "platform.runtime_status.client_certificate_file",
                    &status.client_certificate_file,
                )?);
            }
            if let Some(projection) = &platform.account_characters {
                others.push(chain(
                    "platform.account_characters.client_certificate_file",
                    &projection.client_certificate_file,
                )?);
            }
            if let Some(commands) = &platform.character_commands {
                others.push(chain(
                    "platform.character_commands.client_certificate_file",
                    &commands.client_certificate_file,
                )?);
            }
            let others: Vec<&[CertificateDer<'static>]> =
                others.iter().map(Vec::as_slice).collect();
            let descriptor = PublicProjectionDescriptor::new(
                endpoint.clone(),
                platform.peer_name.clone(),
                roots.clone(),
                own,
                key,
                &others,
            )
            .map_err(|_| invalid("platform.public_projections"))?;
            Some((
                descriptor,
                public.source_authority.clone(),
                public.epoch_fence_file.clone(),
            ))
        }
        _ => None,
    };
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
        runtime_status,
        account_characters: std::sync::Mutex::new(account_characters),
        character_commands: std::sync::Mutex::new(character_commands),
        public_projections: std::sync::Mutex::new(public_projections),
        descriptor,
        launch,
        s2,
        world,
        channel,
    })
}

/// Why an operator resync did not complete.
#[derive(Debug)]
pub enum ResyncError {
    /// F is missing, malformed or unreadable; nothing was changed.
    FenceInvalid,
    /// The raised epoch would not be above F; rolled back.
    NotAboveFence {
        epoch: u64,
        fence: u64,
    },
    /// The function's epoch differs from the one predicted under the lock;
    /// rolled back.
    Unexpected,
    /// The raise committed but F could not be persisted; the publisher
    /// persists F itself before sending anything of the new epoch.
    FenceUnwritten {
        epoch: u64,
    },
    Durability(crate::durability::DurabilityError),
}

impl std::fmt::Display for ResyncError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FenceInvalid => formatter.write_str("epoch fence missing or malformed"),
            Self::NotAboveFence { epoch, fence } => {
                write!(formatter, "raised epoch {epoch} is not above fence {fence}")
            }
            Self::Unexpected => formatter.write_str("resync returned an unexpected epoch"),
            Self::FenceUnwritten { epoch } => {
                write!(
                    formatter,
                    "epoch {epoch} committed; epoch fence not written"
                )
            }
            Self::Durability(error) => write!(formatter, "{error}"),
        }
    }
}

const RESYNC_TIMEOUTS: &str = "SELECT set_config('transaction_timeout', $1, true), \
     set_config('statement_timeout', $1, true), set_config('lock_timeout', $1, true)";
const RESYNC_LOCK: &str = "SELECT projection_epoch, \
     floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint AS now_ms \
     FROM game_character_account_projection_epoch FOR UPDATE";
const PUBLIC_RESYNC_LOCK: &str = "SELECT projection_epoch, \
     floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint AS now_ms \
     FROM game_public_projection_epoch FOR UPDATE";

/// `oteryn-game-ops projection resync`: re-enqueue every account in one
/// transaction (migration 0024). With `raise_epoch` the epoch row is locked
/// first and the raise is refused unless the new epoch is above F; F is
/// persisted after the commit. Returns the epoch.
pub async fn resync(
    root: &DurabilityRoot,
    raise_epoch: bool,
    fence: &mut impl EpochFence,
) -> Result<u64, ResyncError> {
    resync_projection(
        root,
        raise_epoch,
        fence,
        RESYNC_LOCK,
        ("SELECT game_character_account_projection_resync($1)", None),
    )
    .await
}

/// `oteryn-game-ops public-projection resync` (migration 0091): re-enqueue
/// one family (`all`, `highscores`, `character_profile` or `world_online`)
/// in one transaction. An epoch raise covers every family and follows the
/// same fence rules against `F_pub`. Returns the epoch.
pub async fn public_resync(
    root: &DurabilityRoot,
    family: &str,
    raise_epoch: bool,
    fence: &mut impl EpochFence,
) -> Result<u64, ResyncError> {
    resync_projection(
        root,
        raise_epoch,
        fence,
        PUBLIC_RESYNC_LOCK,
        (
            "SELECT game_public_projection_resync($1, $2)",
            Some(family.to_owned()),
        ),
    )
    .await
}

/// One resync transaction: `lock` reads the epoch row `FOR UPDATE`; `call`
/// is the resync function, bound to the family (when it takes one) and the
/// raise choice.
async fn resync_projection(
    root: &DurabilityRoot,
    raise_epoch: bool,
    fence: &mut impl EpochFence,
    lock: &'static str,
    (call, family): (&'static str, Option<String>),
) -> Result<u64, ResyncError> {
    use crate::durability::DurabilityError;
    use sqlx::Row;
    let floor = if raise_epoch {
        Some(
            fence
                .read()
                .map_err(|account_characters::FenceUnusable| ResyncError::FenceInvalid)?,
        )
    } else {
        None
    };
    let outcome = root
        .try_issue_semantic_pass()
        .map_err(ResyncError::Durability)?
        .run(move |holder, deadline| {
            Box::pin(async move {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                let millis = u64::try_from(remaining.as_millis().max(1))
                    .map_err(|_| DurabilityError::RootPassDeadlineExceeded)?;
                let mut tx = sqlx::Connection::begin(&mut **holder).await?;
                sqlx::query(RESYNC_TIMEOUTS)
                    .bind(format!("{millis}ms"))
                    .execute(&mut *tx)
                    .await?;
                let mut expected = None;
                if let Some(fence) = floor {
                    let row = sqlx::query(lock).fetch_one(&mut *tx).await?;
                    let current: i64 = row.try_get("projection_epoch")?;
                    let now_ms: i64 = row.try_get("now_ms")?;
                    match account_characters::raised_epoch(current, now_ms, fence) {
                        Some(epoch) => expected = Some(epoch),
                        None => {
                            tx.rollback().await?;
                            let epoch = current
                                .saturating_add(1)
                                .max(now_ms)
                                .try_into()
                                .unwrap_or(0);
                            return Ok(Err(ResyncError::NotAboveFence { epoch, fence }));
                        }
                    }
                }
                let mut query = sqlx::query_scalar(call);
                if let Some(family) = family {
                    query = query.bind(family);
                }
                let epoch: i64 = query.bind(floor.is_some()).fetch_one(&mut *tx).await?;
                let epoch = u64::try_from(epoch).ok();
                if epoch.is_none() || expected.is_some_and(|e| Some(e) != epoch) {
                    tx.rollback().await?;
                    return Ok(Err(ResyncError::Unexpected));
                }
                tx.commit()
                    .await
                    .map_err(DurabilityError::from_commit_error)?;
                Ok(epoch.ok_or(ResyncError::Unexpected))
            })
        })
        .await
        .map_err(ResyncError::Durability)?;
    let epoch = outcome?;
    if floor.is_some() {
        fence
            .persist(epoch)
            .map_err(|account_characters::FenceUnusable| ResyncError::FenceUnwritten { epoch })?;
    }
    Ok(epoch)
}

const PRIVILEGES: &str = "SELECT has_table_privilege('game_character_roots', 'SELECT') \
     AND has_table_privilege('game_character_account_projections', 'SELECT') \
     AND has_table_privilege('game_character_account_projection_epoch', 'SELECT') \
     AND has_table_privilege('game_character_account_projection_outbox', 'SELECT') \
     AND has_table_privilege('game_character_account_projection_outbox', 'DELETE')";

/// Whether the node's database role can read Character ownership and
/// acknowledge the outbox, or `None` when no holder or database answer was
/// available (a busy or re-establishing holder is not a refusal).
pub async fn store_readable(root: &DurabilityRoot) -> Option<bool> {
    let pass = root.try_issue_semantic_pass().ok()?;
    pass.run(|holder, _| {
        Box::pin(async move {
            Ok(sqlx::query_scalar::<_, bool>(PRIVILEGES)
                .fetch_one(&mut **holder)
                .await?)
        })
    })
    .await
    .ok()
}

/// Repeat `probe` with bounded backoff while it has no answer; only a
/// definite answer ends the wait.
async fn await_answer(mut probe: impl AsyncFnMut() -> Option<bool>) -> bool {
    let mut attempt = 0;
    loop {
        if let Some(readable) = probe().await {
            return readable;
        }
        backoff(&mut attempt).await;
    }
}

/// The `ListCharactersForAccount` publisher until `stop`, under the
/// reconciled Character authority. It starts only when the node's database
/// role can read Character ownership; otherwise the refusal is logged and
/// Platform keeps refusing issuance for lack of a fresh watermark.
async fn account_characters_loop(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    projection: Option<(ProjectionDescriptor, String, std::path::PathBuf)>,
    stop: &CancellationToken,
) {
    let Some((descriptor, source_authority, fence)) = projection else {
        return;
    };
    match first(
        await_answer(async || store_readable(root).await),
        stop.cancelled(),
    )
    .await
    {
        None => return,
        Some(false) => {
            event("event=account_characters_projection state=refused reason=privileges");
            return;
        }
        Some(true) => {}
    }
    event("event=account_characters_projection state=started");
    let publisher = Publisher::new(
        SystemClock::default(),
        crate::durability::account_characters_projection::AccountCharactersStore {
            root,
            authority,
        },
        MtlsSink::new(descriptor),
        EpochFenceFile::new(fence),
        source_authority,
        crate::durability::account_characters_projection::MAX_CHARACTER_TRANSACTION,
    );
    first(publisher.run(), stop.cancelled()).await;
}

const PUBLIC_PRIVILEGES: &str = "SELECT has_table_privilege('game_character_roots', 'SELECT') \
     AND has_table_privilege('game_character_progression_state', 'SELECT') \
     AND has_table_privilege('game_character_build_state', 'SELECT') \
     AND has_table_privilege('game_durability_reconnect_sessions', 'SELECT') \
     AND has_table_privilege('game_public_projection_epoch', 'SELECT') \
     AND has_table_privilege('game_public_character_profiles', 'SELECT') \
     AND has_table_privilege('game_public_highscores', 'SELECT') \
     AND has_table_privilege('game_public_character_profile_outbox', 'SELECT') \
     AND has_table_privilege('game_public_character_profile_outbox', 'DELETE') \
     AND has_table_privilege('game_public_highscore_outbox', 'SELECT') \
     AND has_table_privilege('game_public_highscore_outbox', 'DELETE') \
     AND has_table_privilege('game_public_world_online', 'SELECT') \
     AND has_table_privilege('game_public_world_online', 'INSERT') \
     AND has_table_privilege('game_public_world_online', 'UPDATE')";

/// Whether the node's database role can read the public sources and
/// acknowledge the public outboxes; `None` while no answer was available.
pub async fn public_store_readable(root: &DurabilityRoot) -> Option<bool> {
    let pass = root.try_issue_semantic_pass().ok()?;
    pass.run(|holder, _| {
        Box::pin(async move {
            Ok(sqlx::query_scalar::<_, bool>(PUBLIC_PRIVILEGES)
                .fetch_one(&mut **holder)
                .await?)
        })
    })
    .await
    .ok()
}

/// The public-projection publisher until `stop`, under the reconciled
/// Character authority. Disabled unless configured and enabled; refused (and
/// logged) when the node's role lacks the privileges, so Platform keeps the
/// families stale and unserved.
async fn public_projections_loop(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    public: Option<(PublicProjectionDescriptor, String, std::path::PathBuf)>,
    stop: &CancellationToken,
) {
    let Some((descriptor, source_authority, fence)) = public else {
        return;
    };
    match first(
        await_answer(async || public_store_readable(root).await),
        stop.cancelled(),
    )
    .await
    {
        None => return,
        Some(false) => {
            event("event=public_projections state=refused reason=privileges");
            return;
        }
        Some(true) => {}
    }
    event("event=public_projections state=started");
    let publisher = public_projections::Publisher::new(
        SystemClock::default(),
        crate::durability::public_projections::PublicProjectionsStore { root, authority },
        MtlsPublicSink::new(descriptor),
        EpochFenceFile::new(fence),
        source_authority,
        crate::durability::account_characters_projection::MAX_CHARACTER_TRANSACTION,
    );
    first(publisher.run(), stop.cancelled()).await;
}

/// Both futures to completion on this task.
async fn both(a: impl Future<Output = ()>, b: impl Future<Output = ()>) {
    let mut a = pin!(a);
    let mut b = pin!(b);
    let (mut a_done, mut b_done) = (false, false);
    poll_fn(|context| {
        if !a_done {
            a_done = a.as_mut().poll(context).is_ready();
        }
        if !b_done {
            b_done = b.as_mut().poll(context).is_ready();
        }
        if a_done && b_done {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
}

/// The Character Authority side of the command consumer: it refreshes the
/// account's security evidence of a decoded create, then decides under the
/// reconciled Character authority (contract §6).
struct CommandDecider<'a, 'r, 'c> {
    root: &'a DurabilityRoot,
    authority: &'a ReconciledCharacterAuthority<'r, 'c>,
    proof: &'a NodeIncarnationProof,
    evidence: &'a FreshEvidenceSource,
    source_authority: String,
}

impl Decider for CommandDecider<'_, '_, '_> {
    async fn decide(&mut self, command: &str, operation_id: &str, input: Input<'_>) -> Decision {
        let mut held = None;
        if let Input::Intent(raw) = input
            && let Classified::Create(intent) = classify_intent(raw, command, operation_id)
        {
            // A failed refresh leaves the evidence stale; the decision then
            // reads it as unknown and stays transient until the intent expires.
            // The reason is logged as fixed tokens only (no account or bytes).
            let account = uuid_text(&intent.account_id);
            let started = std::time::Instant::now();
            match self
                .evidence
                .refresh_account_held(self.root, self.proof, &account)
                .await
            {
                Ok(evidence) => {
                    event(&format!(
                        "event=character_commands operation=refresh command={command} result=accepted elapsed_ms={}",
                        started.elapsed().as_millis()
                    ));
                    held = Some(evidence);
                }
                Err(unavailable) => event(&format!(
                    "event=character_commands operation=refresh command={command} result=unavailable {unavailable} elapsed_ms={}",
                    started.elapsed().as_millis()
                )),
            }
        }
        // The decision is the next pass after the acceptance; the accepted
        // slot is cleared only once it has read the evidence.
        let decision = self
            .root
            .decide_character_command(
                self.authority,
                self.proof,
                &self.source_authority,
                command,
                operation_id,
                input,
            )
            .await;
        if let Some(evidence) = held {
            let _ = evidence.release().await;
        }
        decision
    }
}

/// The character-command consumer until `stop`; configured only by an explicit
/// `[platform.character_commands]` section, so disabled by default.
async fn character_commands_loop(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    evidence: &FreshEvidenceSource,
    commands: Option<(CommandDescriptor, String)>,
    stop: &CancellationToken,
) {
    let Some((descriptor, source_authority)) = commands else {
        return;
    };
    event("event=character_commands state=started");
    let decider = CommandDecider {
        root,
        authority,
        proof,
        evidence,
        source_authority: source_authority.clone(),
    };
    let consumer = Consumer::new(
        SystemClock::default(),
        MtlsSource::new(descriptor),
        decider,
        source_authority,
        event,
    );
    first(consumer.run(), stop.cancelled()).await;
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
    backoff_within(attempt, None).await;
}

/// The exponential retry delay for `attempt`, never longer than `remaining`.
fn retry_delay(attempt: u32, remaining: Option<Duration>) -> Duration {
    let delay =
        Duration::from_millis(200_u64.saturating_mul(1 << attempt.min(5))).min(RETRY_BACKOFF_MAX);
    remaining.map_or(delay, |remaining| delay.min(remaining))
}

/// Back off without sleeping past `budget`, so a budgeted retry loop
/// returns by its deadline.
async fn backoff_within(attempt: &mut u32, budget: Option<tokio::time::Instant>) {
    let remaining =
        budget.map(|deadline| deadline.saturating_duration_since(tokio::time::Instant::now()));
    let delay = retry_delay(*attempt, remaining);
    *attempt = attempt.saturating_add(1);
    tokio::time::sleep(delay).await;
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

/// Whether a boot durability step that failed with `error` may run again
/// (NODE-BOOT-DEADLINE-1): only when no definite answer came back. The holder was not
/// ready (no transaction ran), the pass deadline expired, or the commit outcome is unknown.
/// Any database answer, refusal or stored-state failure is definite and ends the boot.
fn boot_step_retryable(error: &crate::durability::DurabilityError) -> bool {
    use crate::durability::DurabilityError;
    matches!(
        error,
        DurabilityError::RootUnavailable
            | DurabilityError::RootPassDeadlineExceeded
            | DurabilityError::CommitOutcomeUnknown
    )
}

/// [`boot_step_retryable`] for the Character authority open.
fn character_authority_retryable(
    error: &crate::durability::character_authority::CharacterAuthorityError,
) -> bool {
    matches!(
        error,
        crate::durability::character_authority::CharacterAuthorityError::Unavailable(inner)
            if boot_step_retryable(inner)
    )
}

/// Run one boot durability step under the boot pass deadline, retrying it a bounded
/// number of times while `retryable` holds; the last error is returned.
///
/// Only idempotent steps are wrapped: the S2 registration read and the Character
/// authority open are read-only, the custody claim writes nothing when custody is
/// already this incarnation's, and the descriptor registration writes nothing when the
/// same descriptor is already current. Each re-run first re-reads the stored row under
/// its lock, so an attempt that committed although its outcome was unknown (or its
/// deadline expired) is confirmed by that read, never applied twice.
async fn with_retries<T, E, F, Fut>(
    root: &DurabilityRoot,
    step: &str,
    deadline: Duration,
    retryable: impl Fn(&E) -> bool,
    mut operation: F,
) -> Result<T, E>
where
    E: std::fmt::Debug,
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut attempt = 0;
    let mut runs = 0;
    loop {
        let result = {
            let _boot = root.boot_pass_deadline(deadline);
            operation().await
        };
        let error = match result {
            Ok(value) => return Ok(value),
            Err(error) => error,
        };
        runs += 1;
        if runs >= PENDING_RECONCILE_ATTEMPTS || !retryable(&error) {
            boot_step_failed(step, &error);
            return Err(error);
        }
        event(&boot_step_retry_line(step, runs, &error));
        root.request_ready();
        let _ = root.maintain_ready_once().await;
        backoff(&mut attempt).await;
    }
}

/// The class of a boot step's error, from the durability error's own form: no SQL text,
/// parameter or credential is ever part of it.
fn boot_step_detail(step: &str, error: &impl std::fmt::Debug) -> String {
    format!("step=\"{step}\" error={error:?}")
}

fn boot_step_retry_line(step: &str, runs: u32, error: &impl std::fmt::Debug) -> String {
    format!(
        "event=boot_step_retry run={runs} {}",
        boot_step_detail(step, error)
    )
}

/// A warn line (always written) naming the failed boot step and its error class, written
/// before `boot_failed`, whose detail carries only the stage.
fn boot_step_failed(step: &str, error: &impl std::fmt::Debug) {
    let detail = boot_step_detail(step, error);
    let mut line =
        oteryn_error_codes::Line::new(oteryn_error_codes::Level::Warn, "node", "boot_step_failed")
            .detail(&detail);
    if let Some(trace) = BOOT_TRACE.get() {
        line = line.trace(trace);
    }
    emit(PROCESS, &line);
}

/// D3 step 4: establish S2 custody, confirm the recorded descriptor and
/// reconcile retained publication slots.
async fn establish_custody(
    root: &DurabilityRoot,
    proof: &NodeIncarnationProof,
    material: &Material,
    evidence: &FreshEvidenceSource,
) -> Result<(), BootError> {
    let deadline = material.config.boot_pass_deadline();
    let stored = with_retries(root, "read", deadline, boot_step_retryable, || {
        root.read_native_admission_source_registration()
    })
    .await
    .map_err(|_| BootError::SourceCustody("read"))?;
    match (stored, &material.s2) {
        (None, None) => {
            return Err(BootError::SourceCustody(
                "fresh-store authorization missing",
            ));
        }
        (None, Some(file)) => {
            let (provenance, descriptor) = s2_parts(file)?;
            let initialized = {
                let _boot = root.boot_pass_deadline(deadline);
                root.initialize_native_admission_source(
                    proof,
                    provenance.clone(),
                    descriptor.clone(),
                )
                .await
            };
            if let Err(error) = initialized {
                // An initialization whose response was lost: compare exactly.
                let after = with_retries(root, "initialize", deadline, boot_step_retryable, || {
                    root.read_native_admission_source_registration()
                })
                .await
                .ok()
                .flatten();
                if after != Some((provenance, descriptor)) {
                    boot_step_failed("initialize", &error);
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
    with_retries(root, "custody claim", deadline, boot_step_retryable, || {
        root.claim_native_admission_source_custody(proof)
    })
    .await
    .map_err(|_| BootError::SourceCustody("custody claim"))?;
    with_retries(root, "descriptor", deadline, boot_step_retryable, || {
        root.register_native_admission_descriptor(proof, material.descriptor.clone())
    })
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

/// CHEST-QUEST-BIND-1 (ARCH-QUEST-WIRING-PACKETS-1 §0.1): a chest placement's quest transition must
/// be in the loaded quest catalogue. A key it lacks would record an obligation no session refresh
/// can apply, so boot refuses it instead.
pub(crate) fn check_chest_quest_transitions(
    content: &crate::content::CanonicalReferencePlayableContent,
    catalogue: &crate::durability::quest_state::quest::QuestStateCatalogue,
) -> Result<(), BootError> {
    let unbound = content
        .definitions
        .iter()
        .filter_map(|definition| match &definition.kind {
            crate::content::ReferenceDefinitionKind::RewardClaim(claim) => Some(claim),
            _ => None,
        })
        .flat_map(|claim| &claim.placements)
        .filter_map(|entry| entry.quest_transition.as_deref())
        .any(|key| catalogue.transition(key).is_none());
    if unbound {
        return Err(BootError::ContentActivation(
            "reward claim quest transition not in the quest catalogue",
        ));
    }
    Ok(())
}

/// QUEST-CAT-BOOT-1 (ARCH-QUEST-WIRING-PACKETS-1 §1.1): the quest state catalogue `load`s for
/// `content_revision` or refuses readiness; an empty or partial catalogue is no fallback.
/// Boot passes the node's declared served revision, `readiness.content_revision` (D634; §1.2's
/// `REVISIONS[0]` is no valid quest or Character revision). Transitions with a `Computed` effect
/// load and refuse `NOT_SUPPORTED` (§1.3).
pub(crate) fn load_quest_catalogue(
    load: impl FnOnce(
        &str,
    ) -> Result<
        crate::durability::quest_state::quest::loader::LoweredQuestState,
        crate::durability::quest_state::quest::loader::QuestLoadError,
    >,
    content_revision: &str,
) -> Result<
    (
        std::sync::Arc<crate::durability::quest_state::quest::QuestStateCatalogue>,
        String,
    ),
    BootError,
> {
    let lowered = load(content_revision)
        .map_err(|_| BootError::ContentActivation("quest state catalogue"))?;
    let counts = lowered.counts();
    let line = format!(
        "event=quest_catalogue state=loaded content_revision={content_revision} quests={} transitions={} not_supported={} not_supported_explicit={} not_supported_inexact={}",
        counts.quests,
        counts.transitions,
        counts.not_supported,
        counts.explicit_computed,
        counts.inexact,
    );
    Ok((std::sync::Arc::new(lowered.catalogue().clone()), line))
}

/// #935 activation issuer: before the Channel runtime, listener or control socket exists, activate
/// the scope's current control-plane issuance of the committed native entry room. A missing,
/// stale or mismatching issuance refuses readiness; nothing is guessed or reset.
async fn activate_content(
    root: &DurabilityRoot,
    world: WorldId,
    channel: ChannelId,
) -> Result<(ContentActivationController, NativeEntryContentPin), BootError> {
    let record = root
        .read_current_content_activation(world, channel)
        .await
        .map_err(|_| BootError::ContentActivation("issuance read"))?
        .ok_or(BootError::ContentActivation("no issuance for this scope"))?;
    let issuance = NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: record.activation_sequence,
        server_artifact_digest: record.server_artifact_digest,
        client_artifact_digest: record.client_artifact_digest,
        frame_binding_digest: record.frame_binding_digest,
    };
    let mut controller = ContentActivationController::new();
    let gameplay = std::env::var_os("OTERYN_NATIVE_GAMEPLAY_MANIFEST")
        .map(|path| {
            crate::content::native_gameplay::NativeGameplayInput::from_manifest(
                std::path::Path::new(&path),
            )
        })
        .transpose()
        .map_err(|_| BootError::ContentActivation("native gameplay manifest"))?;
    let quiescence = NodeBootQuiescence::before_channel_runtime();
    let pin = match &gameplay {
        Some(input) => crate::content::activate_native_entry_room_with_gameplay(
            &mut controller,
            &quiescence,
            world,
            &issuance,
            input,
        ),
        None => activate_native_entry_room(&mut controller, &quiescence, world, &issuance),
    }
    .map_err(|error| {
        BootError::ContentActivation(match error {
            NativeEntryActivationError::Qualification(_) => "qualification",
            NativeEntryActivationError::Activation(_) => "activation",
            NativeEntryActivationError::WorldMismatch => "world",
            NativeEntryActivationError::DigestMismatch => "digest",
            NativeEntryActivationError::FrameBindingMismatch => "frame binding",
        })
    })?;
    event(&format!(
        "event=content_activated activation_sequence={} server_digest={} client_digest={} frame_binding={}",
        pin.activation_sequence(),
        hex(&pin.identity().server_artifact_digest()),
        hex(&pin.identity().client_artifact_digest()),
        hex(&pin.frame_binding().digest()),
    ));
    Ok((controller, pin))
}

/// D3 step 6: the operator assigns exactly this incarnation within the wait.
async fn await_assignment(
    root: &DurabilityRoot,
    proof: &NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    wait: Duration,
) -> Result<RuntimeScopeAssignment, BootError> {
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
            return Ok(assignment);
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
        // History without a current guard is invalid stored state (D3). A
        // durability holder that is re-establishing is retried within the
        // shutdown budget, or a bounded number of times at boot.
        let mut attempt = 0;
        let current = loop {
            match guards.load(std::slice::from_ref(&key)).await {
                Ok(mut rows) => break rows.pop().flatten(),
                Err(error) => {
                    let exhausted = match budget {
                        Some(deadline) => tokio::time::Instant::now() >= deadline,
                        None => attempt >= PENDING_RECONCILE_ATTEMPTS,
                    };
                    if exhausted {
                        event(&format!("event=readiness_guard_chain error={error:?}"));
                        return Err(BootError::Readiness("guard chain"));
                    }
                    self.root.request_ready();
                    backoff_within(&mut attempt, budget).await;
                }
            }
        };
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
    ) -> Result<AdmissionAuthorityPublicationV1, BootError> {
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
                        return Ok(publication);
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
                        backoff_within(&mut attempt, budget).await;
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
            | crate::durability::character_authority::CharacterAuthorityError::Conflict
            | crate::durability::character_authority::CharacterAuthorityError::NameUnavailable,
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
        // One trace per accepted connection, minted here and never read from the peer.
        let connection = uuid_v7().ok().map(|bytes| uuid_text(&bytes));
        let connection = connection.as_deref();
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
            event_traced(
                &format!("event=character_bootstrap result={answer}"),
                connection,
            );
            let _ = stream.write_all(format!("{answer}\n").as_bytes()).await;
            let _ = stream.shutdown().await;
        })
        .await;
        if served.is_err() {
            event_traced(
                "event=character_bootstrap result=unavailable reason=deadline",
                connection,
            );
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
    run_with_npc_data_project(config_path, None).await
}

/// Optionally retain a pinned NPC authoring catalogue for the process lifetime. With a
/// `[world_bundle]` World it also decides which of the bundle's NPC placements become NPC
/// actors at Channel start (NPC-LOAD-1, NPC-PLACE-1 §7, [`bundle_npcs`]); NPC services and
/// talk stay inactive.
pub async fn run_with_npc_data_project(
    config_path: &Path,
    npc_data: Option<(&Path, &str)>,
) -> Result<(), BootError> {
    // D3 step 1: everything is read and checked before a socket is bound.
    let material = load(config_path)?;
    let npc_catalogue = npc_data
        .map(|(root, digest)| crate::content::load_data_only_npc_catalogue(root, digest))
        .transpose()
        .map_err(|_| BootError::Readiness("NPC data import"))?;
    if let Some(catalogue) = &npc_catalogue {
        event(&format!(
            "event=npc_data_imported mode=data_only npcs={} dialogues={} services={} profiles={} source_tree_sha256={} spawned_actors=0 activated_services=0",
            catalogue.npc_count(),
            catalogue.dialogue_count(),
            catalogue.service_count(),
            catalogue.profile_count(),
            catalogue.source_tree_digest(),
        ));
    }
    let config = &material.config;
    event(&format!(
        "event=configuration_accepted world_id={} channel_id={}",
        config.scope.world_id, config.scope.channel_id
    ));
    // MAP-CUTOVER-1a: a bundle World is checked before any durable or fixture step.
    let mut npc_frame = None;
    let bundle = world_bundle_gate(config, |path| {
        let data = read_file(
            "world_bundle.path",
            path,
            FileClass::Trusted,
            effective_uid(),
            oteryn_world_bundle::bundle::READ_CAPS.file_bytes,
        )?;
        npc_frame = Some(bundle_npcs::read(&data)?);
        Ok(data)
    })?;
    let npc_boot = bundle_npcs::prepare(
        npc_frame,
        npc_catalogue
            .as_ref()
            .zip(npc_data.map(|(project_root, _)| project_root)),
        config
            .world_bundle
            .as_ref()
            .is_some_and(|bundle| bundle.production),
    )?;
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
    let result = boot_and_serve(&root, &material, bundle, npc_boot, &signalled).await;
    stop_maintenance.cancel();
    let _ = maintenance.await;
    watcher.abort();
    drop(npc_catalogue);
    result
}

/// A signal before readiness was ever published: nothing to withdraw.
fn stopped_before_ready() {
    event("event=shutdown reason=\"signal before ready\"");
    event("event=shutdown state=complete");
}

/// MAP-CUTOVER-1a: without `[world_bundle]` the node serves the fixture entry room. With it,
/// the bundle's pins are checked (`map::boot::check`) before durability, registration or
/// fixture activation; the Channel boots from it once the served item definitions are active
/// ([`boot_bundle_world`], MAP-CUTOVER-1b).
fn world_bundle_gate(
    config: &NodeConfig,
    read: impl FnOnce(&Path) -> Result<Vec<u8>, BootError>,
) -> Result<Option<crate::map::boot::CheckedBundle>, BootError> {
    let Some(bundle) = &config.world_bundle else {
        return Ok(None);
    };
    let pins = crate::map::boot::BootPins {
        bundle: crate::map::BundlePins {
            digest: bundle
                .digest_bytes()
                .ok_or_else(|| invalid("world_bundle.digest"))?,
            project_format_version: bundle.project_format_version.clone(),
            world_schema_version: bundle.world_schema_version.clone(),
            content_revision: bundle.content_revision.clone(),
            production: bundle.production,
        },
        // Readiness carries the bare digest; the bundle check compares the internal `sha256:` form.
        map_revision: crate::map::overlay::internal_map_revision(&config.readiness.map_revision),
        start: crate::map::overlay::TilePos {
            x: bundle.start_x,
            y: bundle.start_y,
            floor: bundle.start_floor,
        },
    };
    let data = read(&bundle.path)?;
    crate::map::boot::check(data, pins)
        .map(Some)
        .map_err(BootError::WorldBundle)
}

/// MAP-CUTOVER-1b: boots the checked bundle's World against the active generation's Item
/// definitions (§1.6): a palette Item key's revision and reference are the generation's pinned
/// Item definition index entry for that key (MAP-ITEM-REF-1), and its solidity,
/// `blocks_projectile` and pickup eligibility are the generation's Item profile for that key and
/// revision; a fact the generation does not state blocks and is not pickupable. A generation
/// without an Item key set names no Item, so a bundle with any Item fails closed at boot.
fn boot_bundle_items(
    checked: crate::map::boot::CheckedBundle,
    world: WorldId,
    channel: ChannelId,
    gameplay: Option<&crate::content::native_gameplay::NativeGameplayState>,
) -> Result<crate::map::boot::BundleWorld, BootError> {
    use crate::content::{ReferenceItemField, ReferenceItemType};
    let index = gameplay.and_then(|state| state.item_index());
    let item = |key: &str| {
        let index = index?;
        let revision = index.revision(key)?;
        let reference = index.definition_ref("Item", key, revision)?;
        let profile = gameplay
            .and_then(|state| state.item_policy(key, revision))
            .map(|policy| policy.record());
        let pickupable = profile.is_some_and(|profile| {
            matches!(
                &profile.semantics.physical,
                ReferenceItemField::Known(physical)
                    if physical.pickupable == ReferenceItemField::Known(true)
            )
        });
        // MAP-DOOR-1: doorhood is the content profile's `item_type`, never an appearance id.
        let door = profile.is_some_and(|profile| {
            matches!(
                &profile.semantics.classification,
                ReferenceItemField::Known(classification)
                    if classification.item_type == ReferenceItemField::Known(ReferenceItemType::Door)
            )
        });
        let attributes = profile.map(|profile| &profile.attributes);
        Some(crate::map::facts::ItemDefinition {
            reference,
            solid: attributes.and_then(|attributes| attributes.blocks_movement),
            blocks_projectile: attributes
                .and_then(|attributes| attributes.blocks_projectile)
                .unwrap_or(true),
            pickupable,
            door,
        })
    };
    checked
        .boot(world, channel, item)
        .map_err(BootError::WorldBundle)
}

/// MAP-DOOR-1: [`boot_bundle_items`] plus the preprod rule that the World places a usable door.
/// A door is usable only when the generation's Item profile states its open item as walkable
/// (`blocks_movement` false and `blocks_projectile` false); a profile that does not describe
/// the open item leaves the door sealed, so the node refuses readiness instead of serving a
/// doorway that opens in appearance only.
fn boot_bundle_world(
    checked: crate::map::boot::CheckedBundle,
    world: WorldId,
    channel: ChannelId,
    gameplay: Option<&crate::content::native_gameplay::NativeGameplayState>,
) -> Result<crate::map::boot::BundleWorld, BootError> {
    let booted = boot_bundle_items(checked, world, channel, gameplay)?;
    // MAP-DOOR-1: a bundle without a usable door is not a playable preprod World.
    let doors = booted.facts().doors();
    if doors.is_empty() {
        return Err(BootError::WorldBundle(
            crate::map::boot::BootRefusal::NoDoors,
        ));
    }
    event(&format!(
        "event=world_bundle state=booted map_revision={} doors={} doors_sealed={}",
        booted.map_revision(),
        doors.len(),
        doors.sealed()
    ));
    Ok(booted)
}

/// MAP-FLOOR-1: activates the floor changes of the booted bundle World, which its movement
/// cells serve to every step (`map::floor`). The boot already refused a floor change whose
/// destinations never end; a served bundle must also place at least one, so a bundle whose
/// stairs, ladders and holes all failed to resolve against the content catalogue refuses
/// readiness instead of silently serving a flat map. Returns the number of floor-change tiles.
fn activate_floor_changes(world: &crate::map::boot::BundleWorld) -> Result<usize, BootError> {
    let count = world.floors().count();
    if count == 0 {
        return Err(BootError::WorldBundle(
            crate::map::boot::BootRefusal::NoFloorChanges,
        ));
    }
    event(&format!(
        "event=world_bundle_floors state=active floor_changes={count}"
    ));
    Ok(count)
}

/// The Channel content pin of a bundle World: the activated generation's pin with the bundle's
/// frame binding and map-revision identities and its start (native floor `-z`) as the
/// first-entry start.
fn bundle_channel_pin(
    pin: &crate::foundation::ChannelContentPin,
    world: &crate::map::boot::BundleWorld,
) -> Result<crate::foundation::ChannelContentPin, BootError> {
    let start = world.start();
    let floor = start
        .floor
        .checked_neg()
        .ok_or(BootError::ContentActivation("world bundle start floor"))?;
    Ok(crate::foundation::ChannelContentPin::from_activation(
        pin.world_id(),
        pin.activation_sequence(),
        pin.server_artifact_digest(),
        pin.client_artifact_digest(),
        world.frame_binding_digest(pin.world_id()),
        world.map_revision_digest(),
        (i32::from(start.x), i32::from(start.y), i16::from(floor)),
    ))
}

/// BUNDLE-COMBAT-ENABLE-1: the combat room of a bundle World, the qualified room's bundle view
/// over the Channel's bundle `cells` with the bundle's frame binding, map revision and start,
/// exactly as [`bundle_channel_pin`] puts them on the Channel pin. `None` (combat stays closed,
/// the World still serves) when the view does not bind.
fn bundle_combat_room(
    room: &crate::content::QualifiedNativeEntryRoom,
    world: &crate::map::boot::BundleWorld,
    world_id: WorldId,
    cells: &crate::content::NativeEntryMovementCells,
) -> Option<crate::content::QualifiedNativeEntryRoom> {
    let start = world.start();
    let view = start.floor.checked_neg().and_then(|floor| {
        room.bundle_view(
            cells.clone(),
            world.frame_binding_digest(world_id),
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: i32::from(start.x),
                y: i32::from(start.y),
                floor: i16::from(floor),
            },
        )
    });
    event(&format!(
        "event=bundle_combat state={}",
        if view.is_some() {
            "enabled"
        } else {
            "disabled"
        }
    ));
    view
}

/// SPAWN-1a (CREATURE-AI-0 §6.2): the fixture World's spawn source is realized at Channel
/// activation, before the listener binds. A bundle World has no fixture spawn source.
fn realize_fixture_spawns(
    runtime: &mut ChannelRuntimeV1,
    spawn_source: Option<crate::content::NativeEntrySpawnSource>,
    bundle_world: bool,
) -> Result<usize, BootError> {
    let sources = spawn_source
        .filter(|_| !bundle_world)
        .map(|source| source.activation_facts())
        .into_iter()
        .collect();
    runtime
        .realize_activation_spawns(sources)
        .map_err(|_| BootError::Readiness("creature spawn realization"))
}

/// The Channel owner window an activation realizes at most one spawn window in
/// (`CREATUREAI0-RL-14`; the carrier's `spawn::OWNER_WINDOW_MICROS`).
const SPAWN_ACTIVATION_WINDOW_MICROS: u64 = 50_000;

/// SPAWN-1b (CREATURE-AI-0 §6.1, §6.2): a bundle World realizes the spawn family of its bundle
/// at Channel activation, before the listener binds, each bundle source as one runtime source.
/// Each creature's initial health is its served definition's maximum health; a creature the
/// active content does not serve refuses readiness. The served policy does not carry the
/// creature profile's `blocked_by_nearby_players` yet, so every point keeps the SPAWN-1a
/// default (not blockable).
///
/// The activation is begun once (every registered ceiling checked, nothing admitted) and then
/// realized one `CREATUREAI0-RL-14` window (4,096 points) per 50 ms owner window, in canonical
/// order, on a boot-local monotonic owner clock: the loop waits for the next window boundary
/// between windows. Any refusal refuses readiness with nothing realized. Returns the creatures
/// realized and the windows used, or `None` when shutdown was requested meanwhile.
pub(crate) async fn realize_bundle_spawns(
    runtime: &mut ChannelRuntimeV1,
    world: &crate::map::boot::BundleWorld,
    signalled: &CancellationToken,
) -> Result<Option<(usize, u64)>, BootError> {
    let refuse = || BootError::Readiness("creature spawn realization");
    let sources = world
        .activation_spawns(|key| {
            let policy = runtime.companion_policy(key).ok()?;
            Some((u64::try_from(policy.maximum_health).ok()?, false))
        })
        .ok_or_else(refuse)?;
    realize_activation_paced(runtime, sources, signalled).await
}

/// Realizes `sources` as one activation paced at most one `CREATUREAI0-RL-14` window per
/// 50 ms owner window (see [`realize_bundle_spawns`]).
async fn realize_activation_paced(
    runtime: &mut ChannelRuntimeV1,
    sources: Vec<crate::map::boot::BundleSpawnFacts>,
    signalled: &CancellationToken,
) -> Result<Option<(usize, u64)>, BootError> {
    let refuse = || BootError::Readiness("creature spawn realization");
    runtime
        .begin_activation_spawn_sources(sources)
        .map_err(|_| refuse())?;
    let origin = tokio::time::Instant::now();
    let mut windows = 0u64;
    loop {
        let now = u64::try_from(origin.elapsed().as_micros()).unwrap_or(u64::MAX);
        let progress = runtime
            .realize_activation_spawn_window(now)
            .map_err(|_| refuse())?;
        if progress.realized > 0 {
            windows += 1;
        }
        if progress.remaining == 0 {
            return Ok(Some((progress.realized_total, windows)));
        }
        let next = (now / SPAWN_ACTIVATION_WINDOW_MICROS)
            .saturating_add(1)
            .saturating_mul(SPAWN_ACTIVATION_WINDOW_MICROS);
        let wake = origin + Duration::from_micros(next);
        if first(tokio::time::sleep_until(wake), signalled.cancelled())
            .await
            .is_none()
        {
            return Ok(None);
        }
    }
}

async fn boot_and_serve(
    root: &DurabilityRoot,
    material: &Material,
    bundle: Option<crate::map::boot::CheckedBundle>,
    npc_boot: Option<bundle_npcs::NpcBoot>,
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
    let seal = &seal;
    let authority = with_retries(
        root,
        "character authority open",
        config.boot_pass_deadline(),
        character_authority_retryable,
        || root.open_character_authority(seal),
    )
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
    let assignment = generation?;
    let generation = assignment.ownership_generation;
    let fact = proof.fact();
    // The controller holds the active generation for the whole serve lifetime; the Channel
    // runtime pins exactly that generation.
    let (active_content, content) =
        activate_content(root, material.world, material.channel).await?;
    let qualified_room = content.qualified_room().clone();
    let (channel_pin, movement_cells, door_content, spawn_source) = content.into_channel_parts();
    // Spell cast §3 (SPELL-D1): the V1 spell book is loaded with the Content activation, before
    // the Channel runtime; a book that does not load refuses readiness.
    let gameplay = active_content
        .active()
        .ok_or(BootError::ContentActivation("active generation"))?
        .native_gameplay();
    // MAP-CUTOVER-1b: a bundle World's Channel moves over the bundle from its configured start,
    // carries its map view in the movement cells and has no entry room: no door, chest, spell
    // or field qualification of the fixture room applies to it (§1.2).
    let bundle = bundle
        .map(|checked| boot_bundle_world(checked, material.world, material.channel, gameplay))
        .transpose()?;
    if let Some(world) = &bundle {
        activate_floor_changes(world)?;
    }
    let (channel_pin, movement_cells) = match &bundle {
        Some(world) => (
            bundle_channel_pin(&channel_pin, world)?,
            world
                .movement_cells(&movement_cells)
                .map_err(|_| BootError::ContentActivation("world bundle movement cells"))?,
        ),
        None => (channel_pin, movement_cells),
    };
    // BUNDLE-COMBAT-ENABLE-1: player auto-attack and native combat casts read the qualified
    // room; a bundle World serves them the room's bundle view, whose identities are the bundle
    // pin's. Every other `qualified_room` consumer stays closed for a bundle World (§1.2).
    let combat_room = match &bundle {
        Some(world) => bundle_combat_room(&qualified_room, world, material.world, &movement_cells),
        None => Some(qualified_room.clone()),
    };
    let spells = match gameplay {
        Some(native) => native.spell_book().clone(),
        None => crate::spell::cast::v1_spell_book()
            .map_err(|_| BootError::ContentActivation("spell book"))?,
    };
    // Data-only Charm import: retain the complete typed catalogue for this serve lifetime.
    // A malformed source refuses readiness; importing it does not qualify a generation,
    // advertise Charm commands or activate effects whose gameplay consumers are unfinished.
    let imported_charms = crate::content::charm_source::CanonicalCharmCatalogue::embedded()
        .map_err(|_| BootError::ContentActivation("charm source catalogue"))?;
    let (charm_index, charm_shard) = imported_charms.source_bytes();
    event(&format!(
        "event=charm_catalogue state=imported definitions={} source_bytes={} source_sha256={}",
        imported_charms.catalogue().definitions().count(),
        charm_index.len() + charm_shard.len(),
        hex(&imported_charms.source_digest()),
    ));
    // Data import only: retain the qualified reference catalogue for this boot. Loading
    // it does not admit Wheel stages, effects, paid Atelier operations or Item definitions.
    let _wheel_gem_data = crate::wheel_gem_data::WheelGemData::embedded()
        .map_err(|_| BootError::ContentActivation("Wheel/Gem reference data"))?;
    event("event=wheel_gem_data state=loaded runtime_admitted=false");
    // ACHIEVEMENT: the Achievement catalogue loads with the Content activation as well; a
    // malformed catalogue, or activated Content whose RewardClaim names an achievement the
    // catalogue lacks (contract §3.3), refuses readiness. The gameplay seam takes it for the
    // `ACCOUNT_ACHIEVEMENTS_QUERY` display read and the chest USE dispatch (C2).
    let achievements = crate::achievement_catalogue::AchievementCatalogue::embedded()
        .map_err(|_| BootError::ContentActivation("achievement catalogue"))?;
    if !achievements
        .unbound_reward_claim_achievements(&door_content)
        .is_empty()
    {
        return Err(BootError::ContentActivation(
            "reward claim achievement not in the catalogue",
        ));
    }
    let (quest_catalogue, quest_event) = load_quest_catalogue(
        crate::durability::quest_state::quest::loader::load_embedded_quest_state,
        &config.readiness.content_revision,
    )?;
    event(&quest_event);
    // #162 5868482467 (M2b): bind the entry room's one door `LocalObjectRuntime` once, at
    // Channel activation, from this exact activated content — never from a value a later
    // `USE_INTENT` is validating against it. `scope`/`generation` are this same activation's
    // own committed scope and ownership generation (bound above, before the Channel runtime).
    let door_scope_generation = ScopeOwnershipGeneration::new(assignment.ownership_generation)
        .map_err(|_| BootError::Readiness("native entry door scope generation"))?;
    let door_content_generation =
        crate::world_runtime::ReferenceContentGeneration::from_content(&door_content)
            .map_err(|_| BootError::Readiness("native entry door content generation"))?;
    let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_activation(
        scope,
        door_scope_generation,
        door_content_generation,
    );
    let door = Mutex::new(
        crate::world_runtime::bind_native_entry_door(
            &door_content,
            &door_fence,
            scope,
            door_scope_generation,
        )
        .map_err(|_| BootError::Readiness("native entry door runtime binding"))?,
    );
    // C2 (#162 5914960502 Q2a): inject the entry room's one reward chest, its RewardClaim and
    // the reward and backpack Item definitions into a clone of this same activated content,
    // like the door above. The compiled Content and its digests stay unchanged. Its claim's
    // achievement (none today) must be in the catalogue, as for the activated content.
    let chest = match bundle {
        Some(_) => door_content.clone(),
        None => crate::interaction_chest_use::with_entry_chest(&door_content)
            .map_err(|_| BootError::ContentActivation("native entry chest content"))?,
    };
    if !achievements
        .unbound_reward_claim_achievements(&chest)
        .is_empty()
    {
        return Err(BootError::ContentActivation(
            "reward claim achievement not in the catalogue",
        ));
    }
    check_chest_quest_transitions(&chest, &quest_catalogue)?;
    let mut channel_runtime = ChannelRuntimeV1::from_committed_assignment(
        material.world,
        material.channel,
        fact.node_id(),
        fact.registration_revision(),
        assignment.ownership_generation,
        assignment.source_revision,
        &assignment.decision_identity,
        usize::try_from(config.scope.preproduction_actor_capacity)
            .map_err(|_| BootError::Readiness("channel runtime capacity"))?,
        channel_pin,
    )
    .map_err(|_| BootError::Readiness("channel runtime composition"))?;
    if let Some(native) = gameplay {
        native
            .install_companion_policies(&mut channel_runtime)
            .map_err(|_| BootError::ContentActivation("active creature policies"))?;
    }
    // NPC-LOAD-1: the bundle's NPC actors take their cells at Channel start, before any creature.
    bundle_npcs::place(&mut channel_runtime, npc_boot.as_ref())?;
    let (spawned_monsters, spawn_windows) = match &bundle {
        Some(world) => match realize_bundle_spawns(&mut channel_runtime, world, signalled).await? {
            Some(realized) => realized,
            None => {
                stopped_before_ready();
                return Ok(());
            }
        },
        None => {
            let realized = realize_fixture_spawns(&mut channel_runtime, spawn_source, false)?;
            (realized, u64::from(realized > 0))
        }
    };
    event(&format!(
        "event=creature_spawns state=realized spawned_monsters={spawned_monsters} windows={spawn_windows}"
    ));
    let runtime = Mutex::new(channel_runtime);
    event(&format!(
        "event=channel_runtime state=bootstrapped ownership_generation={} source_revision={} capacity={}",
        assignment.ownership_generation,
        assignment.source_revision,
        config.scope.preproduction_actor_capacity
    ));
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
    let committed = readiness.publish(true, None).await?;
    // Contract §8.1: report the committed publication; never gates serving.
    let reporter = material.runtime_status.as_ref().map(|(descriptor, epoch)| {
        super::runtime_status::start_platform(
            descriptor.clone(),
            super::runtime_status::Reported {
                world_id: config.scope.world_id.clone(),
                channel_id: config.scope.channel_id.clone(),
                node_id: uuid_text(fact.node_id().as_bytes()),
                epoch: *epoch,
            },
            root.clone(),
            scope,
            fact,
            generation,
            &committed,
        )
    });
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
    let last_position_flush = crate::gameplay_transport::LastPositionFlush::default();
    // PREM-WIRE-1: the one Premium fence every Premium gate reads, the system trusted clock
    // (PREMIUM-ACTIVATION-0 §1.1) and the durable activation latch. No activation is configured
    // (§1.2): setting one needs separate owner authority, so Premium is not delivered.
    let premium_reads = production_premium_reads(root);
    let owners = GameplaySeamOwners {
        root,
        character: &authority,
        holder: &proof,
        evidence,
        world_id: material.world,
        channel_id: material.channel,
        runtime: &runtime,
        movement_cells: &movement_cells,
        door: &door,
        chest: &chest,
        spells: &spells,
        active_generation: active_content.active(),
        premium_coordinator: Some(&premium_reads),
        rooms: crate::gameplay_transport::SeamRooms::new(
            bundle.is_none().then_some(&qualified_room),
            combat_room.as_ref(),
        ),
        achievements: &achievements,
        imported_charms: &imported_charms,
        npc_talk: bundle_npcs::talk_book(npc_boot.as_ref()),
        quest_catalogue: &quest_catalogue,
        last_position_flush: Some(&last_position_flush),
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
    // Never gates serving; its end is not a shutdown reason. The public
    // projections run beside it, disabled unless configured and enabled.
    let mut projection = pin!(both(
        account_characters_loop(
            root,
            &authority,
            material
                .account_characters
                .lock()
                .ok()
                .and_then(|mut projection| projection.take()),
            &loops_stop
        ),
        public_projections_loop(
            root,
            &authority,
            material
                .public_projections
                .lock()
                .ok()
                .and_then(|mut public| public.take()),
            &loops_stop
        ),
    ));
    let mut projection_done = false;
    // Never gates serving either; disabled unless configured.
    let mut commands = pin!(character_commands_loop(
        root,
        &authority,
        &proof,
        evidence,
        material
            .character_commands
            .lock()
            .ok()
            .and_then(|mut commands| commands.take()),
        &loops_stop
    ));
    let mut commands_done = false;
    let mut signal = pin!(signalled.cancelled());
    let mut serve_error = None;
    // A loop that ended is never polled again.
    let (mut gameplay_done, mut control_done, mut expiry_done) = (false, false, false);
    let reason = poll_fn(|context| {
        if signal.as_mut().poll(context).is_ready() {
            return Poll::Ready("signal");
        }
        if let Poll::Ready(result) = gameplay.as_mut().poll(context) {
            serve_error = result.err();
            gameplay_done = true;
            return Poll::Ready("gameplay loop ended");
        }
        if control_task.as_mut().poll(context).is_ready() {
            control_done = true;
            return Poll::Ready("control loop ended");
        }
        if expiry.as_mut().poll(context).is_ready() {
            expiry_done = true;
            return Poll::Ready("audit expiry loop ended");
        }
        if !projection_done && projection.as_mut().poll(context).is_ready() {
            projection_done = true;
        }
        if !commands_done && commands.as_mut().poll(context).is_ready() {
            commands_done = true;
        }
        Poll::Pending
    })
    .await;
    event(&format!("event=shutdown reason=\"{reason}\""));
    if let Some(reporter) = &reporter {
        reporter.stop_heartbeats();
    }
    // D3 step 10: withdraw readiness first, within the shutdown budget. The
    // loops keep being driven meanwhile, so work already inside a durability
    // pass finishes and releases the holder the withdrawal needs.
    let budget = tokio::time::Instant::now() + SHUTDOWN_BUDGET;
    // CHAR-POSITION-0 §3.2: the tracked last positions are written while the sessions are still
    // fenced in, so before the withdrawal, within its own share of the budget.
    if !gameplay_done {
        last_position_flush.request();
        let mut flushed = pin!(first(
            last_position_flush.done(),
            tokio::time::sleep(LAST_POSITION_FLUSH_BUDGET),
        ));
        poll_fn(|context| {
            if flushed.as_mut().poll(context).is_ready() {
                return Poll::Ready(());
            }
            if let Poll::Ready(result) = gameplay.as_mut().poll(context) {
                serve_error = result.err();
                gameplay_done = true;
                return Poll::Ready(());
            }
            if !control_done && control_task.as_mut().poll(context).is_ready() {
                control_done = true;
            }
            if !expiry_done && expiry.as_mut().poll(context).is_ready() {
                expiry_done = true;
            }
            if !projection_done && projection.as_mut().poll(context).is_ready() {
                projection_done = true;
            }
            if !commands_done && commands.as_mut().poll(context).is_ready() {
                commands_done = true;
            }
            Poll::Pending
        })
        .await;
    }
    let mut withdrawal = pin!(readiness.publish(false, Some(budget)));
    let withdrawn = poll_fn(|context| {
        if let Poll::Ready(result) = withdrawal.as_mut().poll(context) {
            return Poll::Ready(result);
        }
        if !gameplay_done && let Poll::Ready(result) = gameplay.as_mut().poll(context) {
            serve_error = serve_error.take().or(result.err());
            gameplay_done = true;
        }
        if !control_done && control_task.as_mut().poll(context).is_ready() {
            control_done = true;
        }
        if !expiry_done && expiry.as_mut().poll(context).is_ready() {
            expiry_done = true;
        }
        if !projection_done && projection.as_mut().poll(context).is_ready() {
            projection_done = true;
        }
        if !commands_done && commands.as_mut().poll(context).is_ready() {
            commands_done = true;
        }
        Poll::Pending
    })
    .await;
    if let Some(reporter) = &reporter {
        reporter.withdrawn(withdrawn.as_ref().ok());
    }
    if let Err(error) = withdrawn {
        event(&format!(
            "event=readiness ready=false result=failed reason=\"{error}\""
        ));
    }
    shutdown.cancel();
    loops_stop.cancel();
    // In-flight admissions and an in-flight bootstrap are never cancelled
    // midway: each runs to its own outcome under the bounds of its durability
    // passes and Platform exchanges. The supervisor's stop timeout is the
    // outer bound; a kill after it is recovered like any crash.
    if !gameplay_done {
        let _ = gameplay.as_mut().await;
    }
    if !control_done {
        control_task.as_mut().await;
    }
    if !expiry_done {
        expiry.as_mut().await;
    }
    if !projection_done {
        projection.as_mut().await;
    }
    if !commands_done {
        commands.as_mut().await;
    }
    if let Some(reporter) = reporter {
        reporter
            .finish(budget.saturating_duration_since(tokio::time::Instant::now()))
            .await;
    }
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
    fn a_busy_holder_delays_the_privilege_answer_without_refusing() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            // No idle holder at startup: no answer, which is not a privilege refusal.
            let root = DurabilityRoot::connect_test_runtime("postgres://node@127.0.0.1:1/game")
                .expect("lazy root");
            assert_eq!(store_readable(&root).await, None);
            let mut answers = [None, None, Some(true)].into_iter();
            let mut calls = 0;
            let readable = await_answer(async || {
                calls += 1;
                answers.next().flatten()
            })
            .await;
            assert!(readable);
            assert_eq!(calls, 3);
            let mut refused = [None, Some(false)].into_iter();
            assert!(!await_answer(async || refused.next().flatten()).await);
        });
    }

    #[test]
    fn production_premium_reads_configure_no_activation_and_read_the_latch() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            let root = DurabilityRoot::connect_test_runtime("postgres://node@127.0.0.1:1/game")
                .expect("lazy root");
            let reads = production_premium_reads(&root);
            // PREMIUM-ACTIVATION-0 §1.2: no activation record in production, so Premium is
            // never delivered and every account reads Free, without a latch write.
            assert!(reads.activation().is_none());
            assert!(reads.reads_latch());
            assert!(reads.has_clock());
            assert_eq!(
                reads.premium_gate([1; 16]).await,
                crate::domain::premium::PremiumGate::CLOSED
            );
        });
    }

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
    fn account_characters_boots_without_a_runtime_status_section() {
        let issue = || {
            let pair =
                rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).expect("leaf");
            let key = PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(
                pair.signing_key.serialize_der(),
            ));
            (vec![pair.cert.der().clone()], key)
        };
        let (projection, evidence, status) = (issue(), issue(), issue());
        let roots = evidence.0.clone();
        let build = |identity: &(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>),
                     status: Option<&[CertificateDer<'static>]>| {
            projection_descriptor(
                ("127.0.0.1".to_owned(), 8443),
                "localhost".to_owned(),
                roots.clone(),
                (identity.0.clone(), identity.1.clone_key()),
                &evidence.0,
                status,
            )
        };
        // `[platform.account_characters]` without `[platform.runtime_status]` boots.
        assert!(build(&projection, None).is_ok());
        assert!(build(&projection, Some(&status.0)).is_ok());
        // A shared identity is still refused, whichever purpose it is shared with.
        assert!(matches!(
            build(&evidence, None),
            Err(BootError::Startup(StartupError::Invalid {
                key: "platform.account_characters"
            }))
        ));
        assert!(build(&status, Some(&status.0)).is_err());
    }

    #[test]
    fn retry_delay_never_exceeds_the_remaining_budget() {
        // Unbudgeted retries grow exponentially up to the cap.
        assert_eq!(retry_delay(0, None), Duration::from_millis(200));
        assert_eq!(retry_delay(3, None), Duration::from_millis(1600));
        assert_eq!(retry_delay(9, None), RETRY_BACKOFF_MAX);
        // A budgeted retry sleeps at most the time left before the deadline.
        assert_eq!(
            retry_delay(9, Some(Duration::from_millis(700))),
            Duration::from_millis(700)
        );
        assert_eq!(retry_delay(9, Some(Duration::ZERO)), Duration::ZERO);
        assert_eq!(
            retry_delay(0, Some(Duration::from_secs(3))),
            Duration::from_millis(200)
        );
    }

    #[test]
    fn boot_steps_retry_only_root_contention_and_deadline_outcomes() {
        use crate::durability::{
            DurabilityError, MigrationFailureClass, SchemaCompatibility, SqlxFailureClass,
        };
        // The single connection was busy or retired, or the pass ran out of time: the
        // wrapped steps are idempotent re-reads, so the next run confirms the stored state.
        for retryable in [
            DurabilityError::RootUnavailable,
            DurabilityError::RootPassDeadlineExceeded,
            DurabilityError::CommitOutcomeUnknown,
        ] {
            assert!(boot_step_retryable(&retryable), "{retryable:?}");
        }
        // Every other class is a stored-state, schema or configuration verdict: fail closed.
        for terminal in [
            DurabilityError::AdmissionGameSessionLedgerExhausted,
            DurabilityError::Unavailable,
            DurabilityError::Database(SqlxFailureClass::Database(None)),
            DurabilityError::Database(SqlxFailureClass::Transport),
            DurabilityError::Database(SqlxFailureClass::Pool),
            DurabilityError::Migration(MigrationFailureClass::Dirty),
            DurabilityError::SchemaIncompatible(SchemaCompatibility::Incompatible),
            DurabilityError::InvalidConfiguration,
            DurabilityError::CommitRejected,
            DurabilityError::RootTaskFailed,
            DurabilityError::InvalidStoredState,
        ] {
            assert!(!boot_step_retryable(&terminal), "{terminal:?}");
        }
    }

    #[test]
    fn character_authority_open_retries_only_a_retryable_root_cause() {
        use crate::durability::DurabilityError;
        use crate::durability::character_authority::CharacterAuthorityError;
        assert!(character_authority_retryable(
            &CharacterAuthorityError::Unavailable(DurabilityError::RootPassDeadlineExceeded)
        ));
        assert!(character_authority_retryable(
            &CharacterAuthorityError::Unavailable(DurabilityError::RootUnavailable)
        ));
        for terminal in [
            CharacterAuthorityError::Rejected,
            CharacterAuthorityError::Conflict,
            CharacterAuthorityError::NameUnavailable,
            CharacterAuthorityError::Unavailable(DurabilityError::Unavailable),
            CharacterAuthorityError::Unavailable(DurabilityError::InvalidStoredState),
        ] {
            assert!(!character_authority_retryable(&terminal), "{terminal:?}");
        }
    }

    #[test]
    fn boot_step_lines_name_the_step_and_only_the_error_class() {
        use crate::durability::{DurabilityError, SqlxFailureClass};
        assert_eq!(
            boot_step_detail("descriptor", &DurabilityError::RootPassDeadlineExceeded),
            "step=\"descriptor\" error=RootPassDeadlineExceeded"
        );
        assert_eq!(
            boot_step_detail(
                "custody claim",
                &DurabilityError::Database(SqlxFailureClass::Database(None))
            ),
            "step=\"custody claim\" error=Database(Database(None))"
        );
        let line = boot_step_retry_line("descriptor", 2, &DurabilityError::RootUnavailable);
        let (level, module, event, detail) = split_event(&line);
        assert_eq!(level, oteryn_error_codes::Level::Info);
        assert_eq!(module, "node");
        assert_eq!(event, "boot_step_retry");
        assert_eq!(detail, "run=2 step=\"descriptor\" error=RootUnavailable");
    }

    #[test]
    fn budgeted_backoff_returns_by_the_deadline() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            // Unclamped, attempt 9 would sleep the full 5 s cap.
            let started = tokio::time::Instant::now();
            let deadline = started + Duration::from_millis(300);
            let mut attempt = 9;
            backoff_within(&mut attempt, Some(deadline)).await;
            let elapsed = started.elapsed();
            assert!(elapsed >= Duration::from_millis(300), "{elapsed:?}");
            assert!(elapsed < Duration::from_secs(1), "{elapsed:?}");
            assert_eq!(attempt, 10);
            // Past the deadline it does not sleep at all.
            let late = tokio::time::Instant::now();
            backoff_within(&mut attempt, Some(deadline)).await;
            assert!(late.elapsed() < Duration::from_millis(100));
        });
    }

    #[test]
    fn map_cutover_a_fixture_config_skips_and_a_bundle_config_is_checked_before_durability() {
        use super::super::config::tests::{NODE, WORLD_BUNDLE};
        let gate = |document: &str, bytes: &[u8]| {
            let config = NodeConfig::parse(document.as_bytes()).expect("config");
            world_bundle_gate(&config, |_| Ok(bytes.to_vec()))
        };

        // No `[world_bundle]`: the fixture entry room goes on to serve; nothing is read.
        let config = NodeConfig::parse(NODE.as_bytes()).expect("fixture config");
        let fixture = world_bundle_gate(&config, |_| Err(invalid("world_bundle.path")));
        assert!(matches!(fixture, Ok(None)));

        let (bytes, pins) = crate::map::boot::tests::bundle();
        let digest: String = pins.digest.iter().map(|b| format!("{b:02x}")).collect();
        let configured = format!("{NODE}{WORLD_BUNDLE}")
            .replace(
                "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
                &digest,
            )
            .replace("start_x = 100", "start_x = 2")
            .replace("start_y = 200", "start_y = 0");
        let bundle = configured.replace(
            "map_revision = \"map-1\"",
            &format!("map_revision = \"{digest}\""),
        );
        assert!(matches!(gate(&bundle, &bytes), Ok(Some(_))));
        // Each check that needs no item definition refuses before any durable step.
        assert!(matches!(
            gate(&configured, &bytes),
            Err(BootError::WorldBundle(
                crate::map::boot::BootRefusal::MapRevision
            ))
        ));
        assert!(matches!(
            gate(&bundle.replace("start_x = 2", "start_x = 4"), &bytes),
            Err(BootError::WorldBundle(
                crate::map::boot::BootRefusal::StartNotEnterable
            ))
        ));
        assert!(matches!(
            gate(&bundle.replace("\"rev-1\"", "\"rev-2\""), &bytes),
            Err(BootError::WorldBundle(crate::map::boot::BootRefusal::Load(
                crate::map::LoadError::ContentRevision
            )))
        ));
        let config = NodeConfig::parse(bundle.as_bytes()).expect("bundle config");
        let unreadable = world_bundle_gate(&config, |_| Err(invalid("world_bundle.path")));
        assert!(matches!(
            unreadable,
            Err(BootError::Startup(StartupError::Invalid {
                key: "world_bundle.path"
            }))
        ));
    }

    /// #1916: a bundle whose palette names Items boots on the real path, from the active
    /// generation's Item key set and profiles, not from the entry room's content, and its
    /// Channel pin carries the bundle's identities.
    #[test]
    fn boot_realizes_both_d116_rats_on_the_den_cells_before_readiness() {
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let node =
            NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
        let room = crate::content::qualify_native_entry_room(world).expect("entry room");
        let runtime = || {
            ChannelRuntimeV1::from_committed_assignment(
                world,
                channel,
                node,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                8,
                crate::foundation::ChannelContentPin::test(world),
            )
            .expect("runtime")
        };
        let mut booted = runtime();
        assert_eq!(
            realize_fixture_spawns(&mut booted, room.spawn_source().cloned(), false).ok(),
            Some(2)
        );
        let rats = booted
            .spawn_point_creatures()
            .into_iter()
            .map(|(_, cell, live)| ((cell.x, cell.y, cell.floor), live.is_some()))
            .collect::<Vec<_>>();
        assert_eq!(rats, vec![((2, 0, 0), true), ((2, -1, 0), true)]);
        // A bundle World realizes no fixture spawn.
        let mut bundle = runtime();
        assert_eq!(
            realize_fixture_spawns(&mut bundle, room.spawn_source().cloned(), true).ok(),
            Some(0)
        );
        assert!(bundle.spawn_point_creatures().is_empty());
    }

    /// SPAWN-1b (Codex P1 on #1981, `CREATUREAI0-RL-14`): an activation above one window is
    /// realized one window of at most 4,096 points per 50 ms owner window, not in one turn; a
    /// shutdown requested meanwhile stops it before readiness.
    #[test]
    fn spawn1b_activation_is_paced_one_window_per_owner_window() {
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let node =
            NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
        let runtime = || {
            ChannelRuntimeV1::from_committed_assignment(
                world,
                channel,
                node,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                9_000,
                crate::foundation::ChannelContentPin::test(world),
            )
            .expect("runtime")
        };
        // 131 sources of 63 points: 8,253 points in windows of 4,096, 4,096 and 61.
        let sources = || -> Vec<crate::map::boot::BundleSpawnFacts> {
            (0..131)
                .map(|_| {
                    let points = (0..63)
                        .map(|x| {
                            (
                                (x, 0, 7),
                                60_000,
                                "oteryn:creature/rat".to_string(),
                                20,
                                false,
                            )
                        })
                        .collect::<Vec<_>>();
                    (points, 5_000)
                })
                .collect::<Vec<_>>()
        };
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("runtime");
        executor.block_on(async {
            let mut paced = runtime();
            let started = std::time::Instant::now();
            let realized =
                realize_activation_paced(&mut paced, sources(), &CancellationToken::new()).await;
            assert_eq!(realized.ok(), Some(Some((8_253, 3))));
            assert!(started.elapsed() >= Duration::from_millis(100));
            assert_eq!(paced.spawn_point_creatures().len(), 8_253);

            let stopped = CancellationToken::new();
            stopped.cancel();
            let mut cancelled = runtime();
            assert_eq!(
                realize_activation_paced(&mut cancelled, sources(), &stopped)
                    .await
                    .ok(),
                Some(None)
            );
        });
    }

    /// SPAWN-1b (Codex P1 4243711197 on #1981): a family of the pinned pre-production bundle's
    /// shape, 51,078 sources with 82,673 active points of which 31,595 sources mix two
    /// creatures and delays, is 51,078 runtime sources: within `CREATUREAI0-RL-02` (65,536),
    /// where one runtime source per creature/delay run would be 82,673 and refuse. It realizes
    /// paced, in 21 owner windows.
    #[test]
    fn spawn1b_bundle_scale_family_stays_one_runtime_source_per_bundle_source() {
        use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};
        const SOURCES: usize = 51_078;
        const POINTS: usize = 82_673;
        let point = |creature, x, respawn_ms| Point {
            creature,
            x,
            y: 5,
            direction: Direction::South,
            respawn_ms,
        };
        let creature = |key: &str| Creature {
            key: key.into(),
            period: Period::All,
        };
        let table = Table {
            creatures: vec![
                creature("oteryn:creature/bat"),
                creature("oteryn:creature/rat"),
            ],
            sources: (0..SOURCES)
                .map(|index| {
                    let x = u16::try_from(index % 1_024).expect("small");
                    let mut points = vec![point(1, x, 60_000)];
                    if index < POINTS - SOURCES {
                        points.push(point(0, x, 30_000));
                    }
                    Source {
                        key: format!("oteryn:spawn/{index:05}"),
                        floor: -7,
                        x,
                        y: 5,
                        points,
                    }
                })
                .collect(),
        };
        assert_eq!(table.point_count(), POINTS);
        let sources = crate::map::boot::table_activation_spawns(&table, |key| {
            Some((if key.ends_with("rat") { 20 } else { 35 }, false))
        })
        .expect("served creatures");
        drop(table);
        assert_eq!(sources.len(), SOURCES);
        assert_eq!(
            sources.iter().map(|source| source.0.len()).sum::<usize>(),
            POINTS
        );

        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let node =
            NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            POINTS + 1_000,
            crate::foundation::ChannelContentPin::test(world),
        )
        .expect("runtime");
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("runtime");
        let realized = executor.block_on(realize_activation_paced(
            &mut runtime,
            sources,
            &CancellationToken::new(),
        ));
        assert_eq!(realized.ok(), Some(Some((POINTS, 21))));
        assert_eq!(runtime.spawn_point_creatures().len(), POINTS);
    }

    #[test]
    fn map_cutover_b_a_bundle_with_items_boots_from_the_active_generation() {
        const KEYS: &[u8] =
            include_bytes!("../../../../tools/content-schema/native-gameplay/item-keys.json");
        // MAP-DOOR-1: the solid entry is the closed door appearance 1629, whose open item 1630
        // the generation also serves.
        let (boxed, coin) = ("oteryn:item.tibia.i1629", "oteryn:item.tibia.i1000");
        let probe = crate::content::native_gameplay::tests::activated_with_item_keys(Some(KEYS));
        let probe_index = probe
            .active()
            .expect("active")
            .native_gameplay()
            .and_then(|state| state.item_index())
            .expect("index");
        let compact_id = |key: &str| {
            let revision = probe_index.revision(key).expect("revision");
            probe_index
                .definition_ref("Item", key, revision)
                .expect("reference")
                .get()
                - 1
        };
        let (boxed_id, coin_id) = (compact_id(boxed), compact_id(coin));
        let (bytes, pins) =
            crate::map::boot::tests::bundle_with_item_ids((boxed, boxed_id), (coin, coin_id));
        let mut revision = String::from("sha256:");
        for byte in pins.digest {
            revision.push_str(&format!("{byte:02x}"));
        }
        let boot_pins = crate::map::boot::BootPins {
            bundle: pins,
            map_revision: revision,
            start: crate::map::overlay::TilePos {
                x: 2,
                y: 0,
                floor: -7,
            },
        };
        let checked = || crate::map::boot::check(bytes.clone(), boot_pins.clone()).expect("check");
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let with = crate::content::native_gameplay::tests::activated_with_item_keys(Some(KEYS));
        let gameplay = with.active().expect("active").native_gameplay();
        let index = gameplay
            .and_then(|state| state.item_index())
            .expect("index");
        let booted = boot_bundle_items(checked(), world, channel, gameplay).expect("booted");
        // The active profile does not classify 1629 as a Door (`item_type`), so no door is
        // placed whatever its appearance id, and a preprod node refuses to serve this bundle
        // rather than open a doorway in name only.
        assert_eq!(booted.facts().doors().len(), 0);
        assert_eq!(booted.facts().doors().sealed(), 0);
        assert!(matches!(
            boot_bundle_world(checked(), world, channel, gameplay),
            Err(BootError::WorldBundle(
                crate::map::boot::BootRefusal::NoDoors
            ))
        ));
        for (key, x, compact) in [(boxed, 6, boxed_id), (coin, 7, coin_id)] {
            use crate::map::view::MapFacts;
            let reference = index
                .definition_ref("Item", key, index.revision(key).expect("revision"))
                .expect("reference");
            let at = crate::map::overlay::TilePos { x, y: 0, floor: -7 };
            let entry = booted
                .facts()
                .base_entry(at, 1, compact)
                .expect("item facts");
            assert_eq!(
                entry.definition,
                oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference)
            );
        }
        // #1916: the Channel pin of the booted World carries the bundle's map-revision and frame
        // identities, never the entry room's, and starts at the bundle start (legacy `z` 7).
        let entry = crate::foundation::ChannelContentPin::test(world);
        let pin = bundle_channel_pin(&entry, &booted).expect("pin");
        let map_revision: [u8; 32] =
            <sha2::Sha256 as sha2::Digest>::digest(boot_pins.map_revision.as_bytes()).into();
        assert_eq!(booted.map_revision_digest(), map_revision);
        assert_eq!(
            pin,
            crate::foundation::ChannelContentPin::from_activation(
                world,
                entry.activation_sequence(),
                entry.server_artifact_digest(),
                entry.client_artifact_digest(),
                booted.frame_binding_digest(world),
                map_revision,
                (2, 0, 7),
            )
        );
        assert_ne!(pin.map_revision_digest(), entry.map_revision_digest());
        assert_ne!(pin.frame_binding_digest(), entry.frame_binding_digest());
        let other = WorldId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3])
            .expect("other world");
        assert_ne!(
            booted.frame_binding_digest(other),
            booted.frame_binding_digest(world)
        );
        // MAP-DOOR-1: the same bundle with a plain solid box in place of the door places no door,
        // and a preprod node refuses to serve it.
        let (plain, plain_pins) = crate::map::boot::tests::bundle_with_items(
            "oteryn:item.tibia.i100",
            "oteryn:item.tibia.i1000",
        );
        let plain_revision: String = plain_pins
            .digest
            .iter()
            .fold(String::from("sha256:"), |text, byte| {
                format!("{text}{byte:02x}")
            });
        let plain_checked = crate::map::boot::check(
            plain,
            crate::map::boot::BootPins {
                bundle: plain_pins,
                map_revision: plain_revision,
                start: boot_pins.start,
            },
        )
        .expect("check");
        assert!(matches!(
            boot_bundle_world(plain_checked, world, channel, gameplay),
            Err(BootError::WorldBundle(
                crate::map::boot::BootRefusal::NoDoors
            ))
        ));
        // A generation without an Item key set names no Item: the same bundle fails closed.
        let without = crate::content::native_gameplay::tests::activated_with_item_keys(None);
        let gameplay = without.active().expect("active").native_gameplay();
        assert!(matches!(
            boot_bundle_world(checked(), world, channel, gameplay),
            Err(BootError::WorldBundle(_))
        ));
    }

    #[test]
    fn map_floor_activation_counts_floor_changes_and_refuses_a_flat_bundle() {
        use crate::map::boot::tests::{bundle, floor_bundle, floor_items, floor_pins, items};
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let (bytes, pins) = floor_bundle();
        let floors = crate::map::boot::boot(&bytes, &floor_pins(pins), world, channel, floor_items)
            .expect("floor bundle");
        assert_eq!(activate_floor_changes(&floors).expect("activated"), 2);

        // A bundle none of whose entries is a catalogue floor change refuses readiness.
        let (bytes, pins) = bundle();
        let mut revision = String::from("sha256:");
        for byte in pins.digest {
            revision.push_str(&format!("{byte:02x}"));
        }
        let flat = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                bundle: pins,
                map_revision: revision,
                start: crate::map::overlay::TilePos {
                    x: 2,
                    y: 0,
                    floor: -7,
                },
            },
            world,
            channel,
            items,
        )
        .expect("flat bundle");
        assert!(matches!(
            activate_floor_changes(&flat),
            Err(BootError::WorldBundle(
                crate::map::boot::BootRefusal::NoFloorChanges
            ))
        ));
        // Bad floor-change data refuses the boot before it can be activated.
        let (bytes, pins) = crate::map::boot::tests::floor_bundle_with(true);
        assert!(matches!(
            crate::map::boot::boot(&bytes, &floor_pins(pins), world, channel, floor_items),
            Err(crate::map::boot::BootRefusal::FloorChain(_))
        ));
    }

    #[test]
    fn boot_error_codes_match_the_registry() {
        use crate::native_admission_source::runtime_status::NotDeliveredKind;
        let mut codes: Vec<_> = BootErrorKind::ALL.iter().map(|kind| kind.code()).collect();
        codes.extend(LogKind::ALL.iter().map(|kind| kind.code()));
        codes.extend(NotDeliveredKind::ALL.iter().map(|kind| kind.code()));
        crate::durability::sqlstate_codes::tests::assert_in_registry(&codes);
    }

    #[test]
    fn not_delivered_classes_map_to_5001_through_5007() {
        use crate::native_admission_source::runtime_status::NotDelivered as N;
        let classes = [
            N::InvalidReport,
            N::Malformed,
            N::Unauthenticated,
            N::Conflict,
            N::RateLimited,
            N::InvalidResponse,
            N::Unavailable,
        ];
        let numbers: Vec<u32> = classes.iter().map(|class| class.code().number).collect();
        assert_eq!(numbers, (5001..=5007).collect::<Vec<_>>());
    }

    #[test]
    fn boot_failed_parses_in_order_keeps_its_exit_status_and_has_no_player_fields() {
        let cases = [
            (
                BootError::Startup(StartupError::Invalid { key: "x" }),
                10,
                2001,
            ),
            (BootError::Startup(StartupError::Database("x")), 11, 2002),
            (BootError::Registration, 12, 2003),
            (BootError::SourceCustody("x"), 13, 2004),
            (BootError::CharacterAuthority, 14, 2005),
            (BootError::AssignmentWait, 15, 2006),
            (BootError::Bind("x"), 16, 2007),
            (BootError::Readiness("x"), 17, 2008),
            (BootError::ClockSkew, 17, 2009),
            (BootError::Serve, 18, 2010),
            (BootError::ContentActivation("x"), 19, 2011),
            (
                BootError::WorldBundle(crate::map::boot::BootRefusal::MapRevision),
                20,
                2012,
            ),
        ];
        for (error, exit, number) in cases {
            assert_eq!(error.exit_code(), exit);
            let text = boot_failed_line(
                PROCESS,
                &error,
                Some("0192e0a8-0000-7000-8000-000000000001"),
                9,
            );
            let parsed = oteryn_error_codes::parse_line(&text).expect("parses in field order");
            assert_eq!(parsed.event, "boot_failed");
            assert_eq!(parsed.code, Some(number));
            assert_eq!(parsed.detail.as_deref(), Some(error.to_string().as_str()));
            assert!(!text.contains("character="));
        }
        // A reason that forges fields stays inside `detail`.
        let hostile = BootError::SourceCustody("x\" code=E9999 name=FAKE\nsecond");
        let text = boot_failed_line(PROCESS, &hostile, None, 1);
        assert_eq!(text.lines().count(), 1);
        assert_eq!(
            oteryn_error_codes::parse_line(&text).expect("parses").code,
            Some(2004)
        );
    }

    #[test]
    fn a_connection_scoped_line_has_the_trace_and_no_character_field() {
        let (level, module, event, detail) =
            split_event("event=character_bootstrap result=rejected");
        let trace = "0192e0a8-0000-7000-8000-000000000002";
        let text = oteryn_error_codes::Line::new(level, module, event)
            .trace(trace)
            .detail(detail)
            .render(PROCESS, 3);
        let parsed = oteryn_error_codes::parse_line(&text).expect("parses");
        assert_eq!(parsed.trace.as_deref(), Some(trace));
        assert!(!text.contains("character="));
        assert!(!text.contains("character_id"));
    }

    #[test]
    fn tool_failures_carry_their_own_code_and_a_sqlstate_code_wins() {
        use crate::durability::sqlstate_codes::ToolKind;
        let plain = std::io::Error::other("db url missing");
        let text = tool_failure_line(
            "oteryn-game-migrate",
            ToolKind::MigrationFailed.code(),
            &plain,
            None,
            4,
        );
        let parsed = oteryn_error_codes::parse_line(&text).expect("parses");
        assert_eq!(parsed.code, Some(3010));
        assert_eq!(parsed.detail.as_deref(), Some("db url missing"));
        let text = tool_failure_line(
            "oteryn-game-import-proficiencies",
            ToolKind::ProficiencyImportFailed.code(),
            &plain,
            None,
            4,
        );
        assert_eq!(
            oteryn_error_codes::parse_line(&text).expect("parses").code,
            Some(3011)
        );
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
            BootError::ContentActivation("x").exit_code(),
            BootError::WorldBundle(crate::map::boot::BootRefusal::MapRevision).exit_code(),
        ];
        let unique: std::collections::BTreeSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len());
    }
}

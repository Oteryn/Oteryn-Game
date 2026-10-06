//! `oteryn-game-ops`: control-plane actions for one GameNode deployment
//! (OPS-NODE-BOOT-01 D2).
//!
//! The tool runs only as root and never as the node's service user. It holds
//! only the control-plane database credential, never registers a GameNode
//! incarnation, and writes the complete request for every database or fence
//! mutation durably under its validated state directory before submitting it.

use oteryn_game_server::character_recovery_fence::CharacterRecoveryStore;
use oteryn_game_server::content::{
    qualify_native_entry_room, qualify_native_entry_room_from_gameplay_manifest,
};
use oteryn_game_server::durability::DurabilityRoot;
use oteryn_game_server::durability::content_activation::ContentActivationRequest;
use oteryn_game_server::durability::native_admission_source::{
    DescriptorRegistration, FreshStoreProvenance,
};
use oteryn_game_server::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentPredecessor, AssignmentRequest,
    AssignmentState, BootstrapSecret, ControlActor, LaunchBinding, NodeRegistrationFact,
    OperationKey, ReconcileOutcome, RegistrationError, RuntimeScopeAssignmentWriter,
};
use oteryn_game_server::foundation::{ChannelId, NodeId, RuntimeScopeRefV1, WorldId};
use oteryn_game_server::native_admission_source::scope_assignment::{
    self, Assignment, Delivery, NotDelivered, ReportConfig, RetryPolicy, Revocation,
    ScopeAssignmentDescriptor,
};
use oteryn_game_server::node::config::{NodeConfig, OpsConfig};
use oteryn_game_server::node::descriptor_facts::{MAX_PEM_BYTES, certificates, descriptor_facts};
use oteryn_game_server::node::operator_files::{
    AssignmentRequestFile, CharacterRecoveryRequestFile, ContentActivationRequestFile,
    LaunchAuthorizationFile, MAX_OPERATOR_FILE_BYTES, S2AuthorizationFile, hex, hex_bytes,
    random_bytes, uuid_v7,
};
use oteryn_game_server::node::secure_file::{self, FileClass};
use oteryn_game_server::node::{StartupError, connect_root, read_file, uuid_bytes, uuid_text};
use rustix::fd::OwnedFd;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

/// Stable writer registration of this tool (D2).
const WRITER: &str = "oteryn-game-ops";
const FILE_VERSION: u8 = 1;

/// Closed exit codes.
#[derive(Debug)]
enum Failure {
    Usage(&'static str),
    Input(String),
    Identity(&'static str),
    Unavailable(String),
    Rejected(String),
    /// The outcome is unknown; the retained request must be reconciled.
    Ambiguous(String),
}

oteryn_error_codes::error_kinds! {
    /// The registered code of each [`Failure`] and of a malformed `OTERYN_LOG`.
    enum FailureKind {
        Usage = (6001, "OPS_USAGE", InvalidInput, Terminal),
        InputInvalid = (6002, "OPS_INPUT_INVALID", InvalidInput, Terminal),
        IdentityMismatch = (6003, "OPS_IDENTITY_MISMATCH", AuthenticationFailed, SecurityTerminal),
        Unavailable = (6004, "OPS_UNAVAILABLE", DependencyUnavailable, Retryable),
        Rejected = (6005, "OPS_REJECTED", Conflict, Terminal),
        OutcomeAmbiguous = (6006, "OPS_OUTCOME_AMBIGUOUS", InternalUnavailable, Terminal),
        LogSpecInvalid = (6007, "OPS_LOG_SPEC_INVALID", InvalidInput, Terminal),
    }
}

impl Failure {
    /// The registered kind; the match has no wildcard arm.
    fn kind(&self) -> FailureKind {
        match self {
            Self::Usage(_) => FailureKind::Usage,
            Self::Input(_) => FailureKind::InputInvalid,
            Self::Identity(_) => FailureKind::IdentityMismatch,
            Self::Unavailable(_) => FailureKind::Unavailable,
            Self::Rejected(_) => FailureKind::Rejected,
            Self::Ambiguous(_) => FailureKind::OutcomeAmbiguous,
        }
    }

    fn code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::Input(_) => 3,
            Self::Identity(_) => 4,
            Self::Unavailable(_) => 5,
            Self::Rejected(_) => 6,
            Self::Ambiguous(_) => 7,
        }
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (class, detail): (&str, &str) = match self {
            Self::Usage(detail) | Self::Identity(detail) => ("usage", detail),
            Self::Input(detail) => ("input", detail),
            Self::Unavailable(detail) => ("unavailable", detail),
            Self::Rejected(detail) => ("rejected", detail),
            Self::Ambiguous(detail) => ("ambiguous", detail),
        };
        let class = if matches!(self, Self::Identity(_)) {
            "identity"
        } else {
            class
        };
        write!(formatter, "{class}: {detail}")
    }
}

impl From<StartupError> for Failure {
    fn from(error: StartupError) -> Self {
        match error {
            StartupError::Database(_) => Self::Unavailable(error.to_string()),
            _ => Self::Input(error.to_string()),
        }
    }
}

type Outcome = Result<(), Failure>;

struct Arguments {
    words: Vec<String>,
    options: BTreeMap<String, String>,
}

impl Arguments {
    fn parse(raw: Vec<String>) -> Result<Self, Failure> {
        let mut words = Vec::new();
        let mut options = BTreeMap::new();
        let mut raw = raw.into_iter();
        while let Some(argument) = raw.next() {
            if let Some(name) = argument.strip_prefix("--") {
                let value = raw.next().ok_or(Failure::Usage("option without value"))?;
                if options.insert(name.to_owned(), value).is_some() {
                    return Err(Failure::Usage("duplicate option"));
                }
            } else {
                words.push(argument);
            }
        }
        Ok(Self { words, options })
    }

    fn take(&mut self, name: &'static str) -> Result<String, Failure> {
        self.options
            .remove(name)
            .ok_or(Failure::Usage("missing required option"))
    }

    fn take_optional(&mut self, name: &str) -> Option<String> {
        self.options.remove(name)
    }

    fn finish(&self) -> Outcome {
        if self.options.is_empty() {
            Ok(())
        } else {
            Err(Failure::Usage("unknown option"))
        }
    }
}

struct Operator {
    config: OpsConfig,
    state: OwnedFd,
    root: DurabilityRoot,
}

fn event(line: &str) {
    println!("{line}");
}

fn decode_uuid(label: &'static str, text: &str) -> Result<[u8; 16], Failure> {
    uuid_bytes(text).ok_or_else(|| Failure::Input(format!("invalid {label}")))
}

fn parse_u64(label: &'static str, text: &str) -> Result<u64, Failure> {
    text.parse()
        .map_err(|_| Failure::Input(format!("invalid {label}")))
}

fn now_seconds() -> Result<i64, Failure> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Failure::Unavailable("clock".into()))?;
    i64::try_from(elapsed.as_secs()).map_err(|_| Failure::Unavailable("clock".into()))
}

impl Operator {
    fn read_state(&self, name: &str) -> Result<Vec<u8>, Failure> {
        self.read_state_optional(name)?
            .ok_or_else(|| Failure::Input(format!("retained file {name}: missing")))
    }

    /// A retained file, or `None` when it does not exist. Node-read files
    /// belong to the service user and all others to root; the owner is taken
    /// from the opened descriptor.
    fn read_state_optional(&self, name: &str) -> Result<Option<Vec<u8>>, Failure> {
        match secure_file::read_in_owned_by_any(
            &self.state,
            name,
            FileClass::Secret,
            &[0, self.config.operator.service_uid],
            MAX_OPERATOR_FILE_BYTES,
        ) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(secure_file::FileError::Missing) => Ok(None),
            Err(error) => Err(Failure::Input(format!("retained file {name}: {error:?}"))),
        }
    }

    fn create_state(&self, name: &str, hand_to: Option<u32>, content: &[u8]) -> Outcome {
        secure_file::create_exclusive(&self.state, name, hand_to, content)
            .map_err(|error| Failure::Input(format!("file {name}: {error:?}")))
    }

    fn actor(&self) -> Result<ControlActor, Failure> {
        // The recorded actor is the authenticated control login (D2).
        ControlActor::new(&self.config.database.username)
            .map_err(|_| Failure::Input("database.username is not a control actor".into()))
    }
}

async fn authorization(operator: &Operator, mut arguments: Arguments) -> Outcome {
    let action = arguments.words.get(1).cloned().unwrap_or_default();
    let name = arguments.take("file")?;
    match action.as_str() {
        "issue" => {
            let binding = arguments.take("binding")?;
            let supersedes = arguments.take_optional("supersedes");
            arguments.finish()?;
            LaunchBinding::new(&binding).map_err(|_| Failure::Input("invalid binding".into()))?;
            if let Some(node) = &supersedes {
                decode_uuid("supersedes", node)?;
            }
            let file = LaunchAuthorizationFile {
                version: FILE_VERSION,
                secret: hex(
                    &random_bytes::<32>().map_err(|_| Failure::Unavailable("randomness".into()))?
                ),
                launch_binding: binding,
                supersedes,
            };
            let bytes = file
                .encode()
                .map_err(|_| Failure::Input("launch file".into()))?;
            // Durable first, handed to the service user, then the mutation.
            operator.create_state(&name, Some(operator.config.operator.service_uid), &bytes)?;
            issue_launch(operator, &file).await
        }
        "reconcile" => {
            arguments.finish()?;
            let file = LaunchAuthorizationFile::decode(&operator.read_state(&name)?)
                .map_err(|_| Failure::Input("launch file".into()))?;
            issue_launch(operator, &file).await
        }
        "revoke" => {
            arguments.finish()?;
            let file = LaunchAuthorizationFile::decode(&operator.read_state(&name)?)
                .map_err(|_| Failure::Input("launch file".into()))?;
            let secret = BootstrapSecret::from_bytes(
                hex_bytes::<32>(&file.secret).map_err(|_| Failure::Input("launch file".into()))?,
            );
            match operator
                .root
                .revoke_node_bootstrap_authorization(&secret)
                .await
            {
                Ok(()) => {}
                Err(RegistrationError::Rejected) => {
                    return Err(Failure::Rejected(
                        "authorization is consumed or unknown".into(),
                    ));
                }
                Err(error) => return Err(Failure::Ambiguous(format!("revocation: {error:?}"))),
            }
            secure_file::remove_in(&operator.state, &name)
                .map_err(|error| Failure::Unavailable(format!("remove {name}: {error:?}")))?;
            event("authorization=revoked file=removed");
            Ok(())
        }
        _ => Err(Failure::Usage("authorization issue|reconcile|revoke")),
    }
}

async fn issue_launch(operator: &Operator, file: &LaunchAuthorizationFile) -> Outcome {
    let secret = BootstrapSecret::from_bytes(
        hex_bytes::<32>(&file.secret).map_err(|_| Failure::Input("launch file".into()))?,
    );
    let binding = LaunchBinding::new(&file.launch_binding)
        .map_err(|_| Failure::Input("launch file".into()))?;
    let supersedes = match &file.supersedes {
        Some(node) => Some(
            NodeId::decode(&decode_uuid("supersedes", node)?)
                .map_err(|_| Failure::Input("supersedes".into()))?,
        ),
        None => None,
    };
    match operator
        .root
        .issue_node_bootstrap_authorization(&secret, &binding, supersedes)
        .await
    {
        Ok(()) => {
            event(&format!(
                "authorization=issued launch_binding={}",
                file.launch_binding
            ));
            Ok(())
        }
        Err(RegistrationError::Rejected) => Err(Failure::Rejected(
            "authorization conflicts with a recorded one, is revoked or names an unknown NodeId"
                .into(),
        )),
        Err(error) => Err(Failure::Ambiguous(format!(
            "issuance outcome unknown ({error:?}); run `authorization reconcile`"
        ))),
    }
}

async fn registration(operator: &Operator, mut arguments: Arguments) -> Outcome {
    if arguments.words.get(1).map(String::as_str) != Some("revoke") {
        return Err(Failure::Usage("registration revoke"));
    }
    let node = decode_uuid("node-id", &arguments.take("node-id")?)?;
    let revision = parse_u64("revision", &arguments.take("revision")?)?;
    arguments.finish()?;
    let fact = NodeRegistrationFact::new(
        NodeId::decode(&node).map_err(|_| Failure::Input("node-id".into()))?,
        revision,
    );
    match operator.root.revoke_node_registration(fact).await {
        Ok(()) => {
            event("registration=revoked");
            Ok(())
        }
        Err(RegistrationError::Rejected) => Err(Failure::Rejected("unknown registration".into())),
        Err(error) => Err(Failure::Ambiguous(format!("revocation: {error:?}"))),
    }
}

/// The descriptor facts derived from the node configuration's producer.
fn node_descriptor(
    operator: &Operator,
    path: &str,
) -> Result<(NodeConfig, DescriptorRegistration), Failure> {
    let service = operator.config.operator.service_uid;
    let document = read_file(
        "--node-config",
        &PathBuf::from(path),
        FileClass::Trusted,
        service,
        oteryn_game_server::node::config::MAX_CONFIG_BYTES,
    )?;
    let node = NodeConfig::parse(&document).map_err(|error| Failure::Input(error.to_string()))?;
    let platform = &node.platform;
    let roots = read_file(
        "platform.trust_roots_file",
        &platform.trust_roots_file,
        FileClass::Secret,
        service,
        MAX_PEM_BYTES,
    )?;
    let client = read_file(
        "platform.client_certificate_file",
        &platform.client_certificate_file,
        FileClass::Secret,
        service,
        MAX_PEM_BYTES,
    )?;
    let roots =
        certificates(&roots).map_err(|_| Failure::Input("platform.trust_roots_file".into()))?;
    let client = certificates(&client)
        .map_err(|_| Failure::Input("platform.client_certificate_file".into()))?;
    let facts = descriptor_facts(
        &platform.source_authority,
        platform.endpoint,
        &platform.peer_name,
        &roots,
        &client,
    );
    let registration = DescriptorRegistration {
        revision: platform.descriptor_revision,
        facts,
        installed_at: platform.installed_at,
    };
    Ok((node, registration))
}

async fn record_issuance(
    operator: &Operator,
    source_authority: &str,
    descriptor: DescriptorRegistration,
    provenance: Option<FreshStoreProvenance>,
) -> Outcome {
    let revision = descriptor.revision;
    match operator
        .root
        .record_native_source_descriptor_issuance(source_authority, descriptor, provenance)
        .await
    {
        Ok(true) => {
            event(&format!("descriptor_issuance=recorded revision={revision}"));
            Ok(())
        }
        Ok(false) => Err(Failure::Rejected(
            "issuance conflicts, is stale, changes the source authority or precedes fresh-store initialization".into(),
        )),
        Err(error) => Err(Failure::Ambiguous(format!(
            "issuance outcome unknown ({error:?}); re-run with the same inputs"
        ))),
    }
}

async fn s2(operator: &Operator, mut arguments: Arguments) -> Outcome {
    match arguments.words.get(1).map(String::as_str) {
        Some("issue") => {
            let node_config = arguments.take("node-config")?;
            let name = arguments.take("file")?;
            let namespace = arguments.take("namespace")?;
            let authorization = arguments.take("authorization")?;
            arguments.finish()?;
            let (node, descriptor) = node_descriptor(operator, &node_config)?;
            let file = S2AuthorizationFile {
                version: FILE_VERSION,
                namespace,
                authorization,
                source_authority: node.platform.source_authority.clone(),
                initialized_at: now_seconds()?,
                descriptor_revision: descriptor.revision,
                installed_at: descriptor.installed_at,
                descriptor_facts: hex(&descriptor.facts),
            };
            let bytes = file
                .encode()
                .map_err(|_| Failure::Input("S2 file".into()))?;
            operator.create_state(&name, Some(operator.config.operator.service_uid), &bytes)?;
            record_s2_file(operator, &file).await
        }
        Some("reconcile") => {
            let name = arguments.take("file")?;
            arguments.finish()?;
            let file = S2AuthorizationFile::decode(&operator.read_state(&name)?)
                .map_err(|_| Failure::Input("S2 file".into()))?;
            record_s2_file(operator, &file).await
        }
        Some("revision") => {
            // A later descriptor revision: the node configuration's producer,
            // with no fresh-store provenance.
            let node_config = arguments.take("node-config")?;
            arguments.finish()?;
            let (node, descriptor) = node_descriptor(operator, &node_config)?;
            record_issuance(operator, &node.platform.source_authority, descriptor, None).await
        }
        _ => Err(Failure::Usage("s2 issue|reconcile|revision")),
    }
}

async fn record_s2_file(operator: &Operator, file: &S2AuthorizationFile) -> Outcome {
    let facts = decode_facts(&file.descriptor_facts)?;
    record_issuance(
        operator,
        &file.source_authority,
        DescriptorRegistration {
            revision: file.descriptor_revision,
            facts,
            installed_at: file.installed_at,
        },
        Some(FreshStoreProvenance {
            namespace: file.namespace.clone(),
            authorization: file.authorization.clone(),
            source_authority: file.source_authority.clone(),
            initialized_at: file.initialized_at,
        }),
    )
    .await
}

fn decode_facts(text: &str) -> Result<Vec<u8>, Failure> {
    if !text.len().is_multiple_of(2) || text.len() > 8192 {
        return Err(Failure::Input("descriptor facts".into()));
    }
    (0..text.len() / 2)
        .map(|index| {
            hex_bytes::<1>(&text[index * 2..index * 2 + 2])
                .map(|byte| byte[0])
                .map_err(|_| Failure::Input("descriptor facts".into()))
        })
        .collect()
}

async fn character(operator: &Operator, mut arguments: Arguments) -> Outcome {
    match arguments.words.get(1).map(String::as_str) {
        Some("fresh-store") => {
            let name = arguments.take("request")?;
            arguments.finish()?;
            // A retained request is reused exactly; otherwise a new one is
            // written durably before the fence advances.
            let request = match operator.read_state_optional(&name)? {
                Some(bytes) => CharacterRecoveryRequestFile::decode(&bytes)
                    .map_err(|_| Failure::Input("recovery request".into()))?,
                None => {
                    let request = CharacterRecoveryRequestFile {
                        version: FILE_VERSION,
                        recovery_event_id: hex(
                            &uuid_v7().map_err(|_| Failure::Unavailable("randomness".into()))?
                        ),
                        issued_at: u64::try_from(now_seconds()?)
                            .map_err(|_| Failure::Unavailable("clock".into()))?,
                    };
                    let bytes = request
                        .encode()
                        .map_err(|_| Failure::Input("recovery request".into()))?;
                    operator.create_state(&name, None, &bytes)?;
                    request
                }
            };
            let event_id = hex_bytes::<16>(&request.recovery_event_id)
                .map_err(|_| Failure::Input("recovery request".into()))?;
            let fence = &operator.config.character;
            let store = CharacterRecoveryStore::open_owned_by(
                &fence.fence_directory,
                fence.authority_scope_id.clone(),
                fence.issuer_identity.clone(),
                operator.config.operator.service_uid,
            )
            .map_err(|error| Failure::Input(format!("character.fence_directory: {error:?}")))?;
            let transition = store
                .authorize_fresh_store(event_id, request.issued_at)
                .map_err(|error| Failure::Rejected(format!("fresh fence: {error:?}")))?;
            match operator
                .root
                .admit_fresh_character_recovery(&transition)
                .await
            {
                Ok(()) => {
                    event("character_recovery=admitted generation=1");
                    Ok(())
                }
                Err(error) => Err(Failure::Ambiguous(format!(
                    "fresh admission: {error:?}; re-run with the same request"
                ))),
            }
        }
        Some("interpretation") => {
            let profile = arguments.take("profile")?;
            let ruleset = arguments.take("ruleset")?;
            let content = arguments.take("content")?;
            let starter = arguments.take("starter")?;
            arguments.finish()?;
            match operator
                .root
                .configure_character_interpretation(&profile, &ruleset, &content, &starter)
                .await
            {
                Ok(revision) => {
                    event(&format!(
                        "character_interpretation=configured revision={revision}"
                    ));
                    Ok(())
                }
                Err(error) => Err(Failure::Rejected(format!(
                    "interpretation refused until generation 1 is admitted ({error:?})"
                ))),
            }
        }
        Some("bootstrap") => {
            // The operator supplies only the intent's operation id; the node
            // reads the intent and current account security itself (D2).
            let socket = arguments.take("socket")?;
            let operation = arguments.take("operation-id")?;
            arguments.finish()?;
            decode_uuid("operation-id", &operation)?;
            let answer = control_request(&socket, &operation).await?;
            event(&format!("character_bootstrap={answer}"));
            match answer.as_str() {
                "committed" => Ok(()),
                "rejected" => Err(Failure::Rejected("bootstrap rejected".into())),
                _ => Err(Failure::Ambiguous(
                    "bootstrap unavailable; retry with the same operation id".into(),
                )),
            }
        }
        _ => Err(Failure::Usage(
            "character fresh-store|interpretation|bootstrap",
        )),
    }
}

/// One bounded request over the node's local control socket.
async fn control_request(socket: &str, operation: &str) -> Result<String, Failure> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let exchange = async {
        let mut stream = tokio::net::UnixStream::connect(socket).await.ok()?;
        stream
            .write_all(format!("{operation}\n").as_bytes())
            .await
            .ok()?;
        stream.shutdown().await.ok()?;
        let mut answer = Vec::new();
        (&mut stream).take(64).read_to_end(&mut answer).await.ok()?;
        String::from_utf8(answer).ok()
    };
    match tokio::time::timeout(std::time::Duration::from_secs(60), exchange).await {
        Ok(Some(answer)) => Ok(answer.trim_end().to_owned()),
        _ => Err(Failure::Ambiguous(
            "control socket exchange failed; retry with the same operation id".into(),
        )),
    }
}

fn scope_of(world: &str, channel: &str) -> Result<RuntimeScopeRefV1, Failure> {
    Ok(RuntimeScopeRefV1::channel(
        WorldId::decode(&decode_uuid("world", world)?)
            .map_err(|_| Failure::Input("world".into()))?,
        ChannelId::decode(&decode_uuid("channel", channel)?)
            .map_err(|_| Failure::Input("channel".into()))?,
    ))
}

fn request_of(file: &AssignmentRequestFile) -> Result<AssignmentRequest, Failure> {
    let invalid = || Failure::Input("assignment request".into());
    let scope = scope_of(&file.world_id, &file.channel_id)?;
    let predecessor = || -> Result<AssignmentPredecessor, Failure> {
        Ok(AssignmentPredecessor {
            ownership_generation: file.predecessor_generation.ok_or_else(invalid)?,
            source_revision: file.predecessor_source_revision.ok_or_else(invalid)?,
            runtime_guard_publication_revision: file.predecessor_guard_publication_revision,
        })
    };
    let target = || -> Result<NodeRegistrationFact, Failure> {
        let node = decode_uuid(
            "target",
            file.target_node_id.as_deref().ok_or_else(invalid)?,
        )?;
        Ok(NodeRegistrationFact::new(
            NodeId::decode(&node).map_err(|_| invalid())?,
            file.target_registration_revision.ok_or_else(invalid)?,
        ))
    };
    let command = match file.command.as_str() {
        "assign" => AssignmentCommand::Assign {
            scope,
            target: target()?,
        },
        "replace" => AssignmentCommand::Replace {
            scope,
            predecessor: predecessor()?,
            target: target()?,
        },
        "revoke" => AssignmentCommand::Revoke {
            scope,
            predecessor: predecessor()?,
        },
        _ => return Err(invalid()),
    };
    Ok(AssignmentRequest {
        operation_key: OperationKey::from_bytes(
            hex_bytes::<32>(&file.operation_key).map_err(|_| invalid())?,
        ),
        actor: ControlActor::new(&file.actor).map_err(|_| invalid())?,
        command,
    })
}

/// The ownership-authority reporting channel (`--report-config`, runtime-status
/// contract §5). Without it, reporting is disabled and assignment behaves as
/// before (§15).
struct Reporter {
    config: ReportConfig,
    descriptor: ScopeAssignmentDescriptor,
}

fn reporter(
    operator: &Operator,
    path: Option<String>,
    node_config: Option<String>,
) -> Result<Option<Reporter>, Failure> {
    let Some(path) = path else {
        return match node_config {
            None => Ok(None),
            Some(_) => Err(Failure::Usage("--node-config requires --report-config")),
        };
    };
    let node_config =
        node_config.ok_or(Failure::Usage("--report-config requires --node-config"))?;
    let invalid = |key: &str| Failure::Input(format!("--report-config {key}"));
    let document = read_file(
        "--report-config",
        std::path::Path::new(&path),
        FileClass::Trusted,
        0,
        scope_assignment::CONFIG_BYTES,
    )?;
    let config = ReportConfig::parse(&document).map_err(|_| invalid("document"))?;
    let pem = |key: &'static str, path: &std::path::Path, class| {
        read_file(key, path, class, 0, MAX_PEM_BYTES)
    };
    let roots = certificates(&pem(
        "trust_roots_file",
        &config.trust_roots_file,
        FileClass::Trusted,
    )?)
    .map_err(|_| invalid("trust_roots_file"))?;
    let chain = certificates(&pem(
        "client_certificate_file",
        &config.client_certificate_file,
        FileClass::Trusted,
    )?)
    .map_err(|_| invalid("client_certificate_file"))?;
    let key = PrivateKeyDer::from_pem_slice(&pem(
        "client_key_file",
        &config.client_key_file,
        FileClass::Secret,
    )?)
    .map_err(|_| invalid("client_key_file"))?;
    // Every producer identity of the node configuration is always compared,
    // so omitting one fails closed (contract §3).
    let service = operator.config.operator.service_uid;
    let node = NodeConfig::parse(&read_file(
        "--node-config",
        &PathBuf::from(node_config),
        FileClass::Trusted,
        service,
        oteryn_game_server::node::config::MAX_CONFIG_BYTES,
    )?)
    .map_err(|error| Failure::Input(error.to_string()))?;
    let status = node.platform.runtime_status.as_ref().ok_or_else(|| {
        Failure::Input("--node-config platform.runtime_status is required for reporting".into())
    })?;
    // The node's Platform channel and epoch (§3), so both reports reach the
    // same Platform and can match.
    let node_roots = certificates(&read_file(
        "platform.trust_roots_file",
        &node.platform.trust_roots_file,
        FileClass::Trusted,
        service,
        MAX_PEM_BYTES,
    )?)
    .map_err(|_| Failure::Input("platform.trust_roots_file".into()))?;
    let set = |certificates: &[CertificateDer<'static>]| {
        certificates
            .iter()
            .map(|certificate| certificate.as_ref().to_vec())
            .collect::<std::collections::BTreeSet<_>>()
    };
    if !config.matches_node_channel(
        node.platform.endpoint,
        &node.platform.peer_name,
        status.assignment_epoch,
    ) || set(&roots) != set(&node_roots)
    {
        return Err(Failure::Input(
            "--report-config endpoint, peer_name, trust roots or assignment_epoch differ \
             from --node-config platform and platform.runtime_status"
                .into(),
        ));
    }
    let producer = |key: &'static str, path: &std::path::Path| {
        certificates(&read_file(
            key,
            path,
            FileClass::Secret,
            service,
            MAX_PEM_BYTES,
        )?)
        .map_err(|_| Failure::Input(key.into()))
    };
    let mut others = vec![
        producer(
            "platform.client_certificate_file",
            &node.platform.client_certificate_file,
        )?,
        producer(
            "platform.runtime_status.client_certificate_file",
            &status.client_certificate_file,
        )?,
    ];
    // Every configured node host's certificate is required (§3), and each
    // must be the certificate of the identity it is configured for (§5).
    for (identity, path) in &config.node_certificate_files {
        let chain = certificates(&pem("node_certificate_files", path, FileClass::Trusted)?)
            .map_err(|_| invalid("node_certificate_files"))?;
        if !chain
            .first()
            .is_some_and(|leaf| scope_assignment::certificate_has_node_identity(leaf, identity))
        {
            return Err(Failure::Input(
                "node_certificate_files: a certificate subject differs from its node identity"
                    .into(),
            ));
        }
        others.push(chain);
    }
    for path in &config.other_producer_certificate_files {
        others.push(
            certificates(&pem(
                "other_producer_certificate_files",
                path,
                FileClass::Trusted,
            )?)
            .map_err(|_| invalid("other_producer_certificate_files"))?,
        );
    }
    let others: Vec<&[CertificateDer<'static>]> = others.iter().map(Vec::as_slice).collect();
    // Its own identity, never another producer's (contract §3).
    let descriptor = ScopeAssignmentDescriptor::new(
        (config.endpoint.ip().to_string(), config.endpoint.port()),
        config.peer_name.clone(),
        roots,
        chain,
        key,
        &others,
    )
    .map_err(|_| invalid("client identity"))?;
    Ok(Some(Reporter { config, descriptor }))
}

/// Retained state files, so the identity binding is independent of the
/// database.
trait StateFiles {
    fn read_optional(&self, name: &str) -> Result<Option<Vec<u8>>, Failure>;
    fn create(&self, name: &str, content: &[u8]) -> Outcome;
}

impl StateFiles for Operator {
    fn read_optional(&self, name: &str) -> Result<Option<Vec<u8>>, Failure> {
        self.read_state_optional(name)
    }

    fn create(&self, name: &str, content: &[u8]) -> Outcome {
        self.create_state(name, None, content)
    }
}

/// Retained node identity of one registered holder, so a re-send derives the
/// same body from the durable row.
fn identity_file(holder: &NodeRegistrationFact) -> String {
    format!(
        "scope-report-identity-{}-{}",
        hex(holder.node_id().as_bytes()),
        holder.registration_revision()
    )
}

/// The identity offered with one operation, until that operation commits.
fn staged_identity_file(operation_key: &str) -> String {
    format!("scope-report-identity-staged-{operation_key}")
}

fn read_identity(files: &impl StateFiles, name: &str) -> Result<Option<String>, Failure> {
    files
        .read_optional(name)?
        .map(|bytes| {
            String::from_utf8(bytes).map_err(|_| Failure::Input(format!("retained file {name}")))
        })
        .transpose()
}

fn identity_differs() -> Failure {
    Failure::Rejected("node identity differs from the identity bound to this registration".into())
}

/// The identity bound to `holder`, binding `offered` on first use. A
/// different identity for an already bound holder rejects.
fn bind_identity(
    files: &impl StateFiles,
    holder: &NodeRegistrationFact,
    offered: Option<String>,
) -> Result<Option<String>, Failure> {
    let name = identity_file(holder);
    match (read_identity(files, &name)?, offered) {
        (Some(retained), Some(offered)) if retained != offered => Err(identity_differs()),
        (Some(retained), _) => Ok(Some(retained)),
        (None, Some(offered)) => {
            files.create(&name, offered.as_bytes())?;
            Ok(Some(offered))
        }
        (None, None) => Ok(None),
    }
}

/// Stages `offered` with the operation before anything is written. It becomes
/// the holder's binding only when that operation commits (`promote_identity`),
/// so a rejected or failed operation binds nothing.
fn stage_identity(
    files: &impl StateFiles,
    operation_key: &str,
    holder: &NodeRegistrationFact,
    offered: &str,
) -> Outcome {
    if read_identity(files, &identity_file(holder))?.is_some_and(|retained| retained != offered) {
        return Err(identity_differs());
    }
    files.create(&staged_identity_file(operation_key), offered.as_bytes())
}

/// Binds the identity staged with a committed operation to its holder.
fn promote_identity(
    files: &impl StateFiles,
    operation_key: &str,
    holder: &NodeRegistrationFact,
) -> Outcome {
    if let Some(staged) = read_identity(files, &staged_identity_file(operation_key))? {
        bind_identity(files, holder, Some(staged))?;
    }
    Ok(())
}

/// The target holder of an assign or replace request.
fn target_holder(file: &AssignmentRequestFile) -> Result<Option<NodeRegistrationFact>, Failure> {
    let (Some(node), Some(revision)) = (&file.target_node_id, file.target_registration_revision)
    else {
        return Ok(None);
    };
    Ok(Some(NodeRegistrationFact::new(
        NodeId::decode(&decode_uuid("node-id", node)?)
            .map_err(|_| Failure::Input("node-id".into()))?,
        revision,
    )))
}

/// Reports a committed assignment, replacement or revocation.
async fn report_committed(
    operator: &Operator,
    reporter: Option<&Reporter>,
    file: &AssignmentRequestFile,
) -> Outcome {
    match reporter {
        None => Ok(()),
        Some(reporter) if file.command == "revoke" => {
            report(operator, reporter, &file.world_id, &file.channel_id, None).await
        }
        Some(reporter) => {
            if let Some(holder) = target_holder(file)? {
                promote_identity(operator, &file.operation_key, &holder)?;
            }
            report(operator, reporter, &file.world_id, &file.channel_id, None).await
        }
    }
}

/// Canonical scope text and the scope itself.
fn canonical_scope(
    world: &str,
    channel: &str,
) -> Result<(String, String, RuntimeScopeRefV1), Failure> {
    let scope = scope_of(world, channel)?;
    Ok((
        uuid_text(&decode_uuid("world", world)?),
        uuid_text(&decode_uuid("channel", channel)?),
        scope,
    ))
}

/// `ReportScopeAssignmentV1` of the current durable assignment, or
/// `ReportScopeRevocationV1` of a revoked scope (§16.1). The body is derived
/// only from the durable row, the bound node identity (an assignment only) and
/// the declared epoch, so a re-send is byte-identical. A failed report leaves the Game
/// assignment authoritative; the epoch is never raised here (§1.1, U16).
async fn report(
    operator: &Operator,
    reporter: &Reporter,
    world: &str,
    channel: &str,
    offered: Option<String>,
) -> Outcome {
    let (world_id, channel_id, scope) = canonical_scope(world, channel)?;
    let row = operator
        .root
        .read_runtime_scope_assignment(scope)
        .await
        .map_err(|error| Failure::Unavailable(format!("assignment: {error:?}")))?
        .ok_or_else(|| Failure::Rejected("scope has no assignment".into()))?;
    let node_identity = match (row.state, row.holder) {
        (AssignmentState::Assigned, Some(holder)) => {
            if let Some(offered) = &offered
                && !reporter.config.allows(&world_id, &channel_id, offered)
            {
                return Err(Failure::Rejected(
                    "node identity is not configured for the scope".into(),
                ));
            }
            bind_identity(operator, &holder, offered)?.ok_or(Failure::Usage(
                "no node identity is bound to the assigned node; pass --node-identity",
            ))?
        }
        (AssignmentState::Revoked, None) => {
            if offered.is_some() {
                return Err(Failure::Usage(
                    "a revocation carries no node identity (contract §16.1)",
                ));
            }
            let revocation = Revocation {
                assignment_epoch: reporter.config.assignment_epoch,
                world_id,
                channel_id,
                ownership_generation: row.ownership_generation,
                revoked_at: row.decided_at,
            };
            let started = std::time::Instant::now();
            let outcome = scope_assignment::report_revocation(
                &reporter.descriptor,
                &revocation,
                RetryPolicy::default(),
            )
            .await;
            event(&format!(
                "report=ReportScopeRevocationV1 world_id={} channel_id={} result={} attempts={} elapsed_ms={}",
                revocation.world_id,
                revocation.channel_id,
                result_class(outcome.result),
                outcome.attempts,
                started.elapsed().as_millis()
            ));
            return revocation_outcome(outcome.result, world, channel);
        }
        _ => return Err(Failure::Input("scope assignment row".into())),
    };
    if !reporter
        .config
        .allows(&world_id, &channel_id, &node_identity)
    {
        return Err(Failure::Rejected(
            "node identity is not configured for the scope".into(),
        ));
    }
    let assignment = Assignment {
        assignment_epoch: reporter.config.assignment_epoch,
        world_id,
        channel_id,
        ownership_generation: row.ownership_generation,
        node_identity,
        assigned_at: row.decided_at,
    };
    let started = std::time::Instant::now();
    let outcome =
        scope_assignment::report(&reporter.descriptor, &assignment, RetryPolicy::default()).await;
    let class = result_class(outcome.result);
    event(&format!(
        "report=ReportScopeAssignmentV1 world_id={} channel_id={} result={class} attempts={} elapsed_ms={}",
        assignment.world_id,
        assignment.channel_id,
        outcome.attempts,
        started.elapsed().as_millis()
    ));
    match outcome.result {
        Ok(_) => Ok(()),
        Err(not_delivered) if not_delivered.definite() => Err(Failure::Rejected(format!(
            "Platform refused the assignment report ({class}); the Game assignment stands"
        ))),
        Err(_) => Err(Failure::Ambiguous(format!(
            "assignment report not delivered ({class}); the Game assignment stands; \
             run `assignment report --world {world} --channel {channel}`"
        ))),
    }
}

fn result_class(result: Result<Delivery, NotDelivered>) -> &'static str {
    match result {
        Ok(delivery) => delivery.class(),
        Err(not_delivered) => not_delivered.class(),
    }
}

/// Exit outcome of a revocation report. Every failure, definite or not, names
/// the re-send command: the revocation stands and is reported again only by
/// `assignment report` (§16.1).
fn revocation_outcome(
    result: Result<Delivery, NotDelivered>,
    world: &str,
    channel: &str,
) -> Outcome {
    let class = result_class(result);
    let resend = format!("run `assignment report --world {world} --channel {channel}`");
    match result {
        Ok(_) => Ok(()),
        Err(not_delivered) if not_delivered.definite() => Err(Failure::Rejected(format!(
            "Platform refused the revocation report ({class}); the Game revocation stands; \
             {resend} once Platform accepts it"
        ))),
        Err(_) => Err(Failure::Ambiguous(format!(
            "revocation report not delivered ({class}); the Game revocation stands; {resend}"
        ))),
    }
}

async fn assignment(operator: &Operator, mut arguments: Arguments) -> Outcome {
    let action = arguments.words.get(1).cloned().unwrap_or_default();
    let reporter = reporter(
        operator,
        arguments.take_optional("report-config"),
        arguments.take_optional("node-config"),
    )?;
    let node_identity = arguments.take_optional("node-identity");
    if node_identity.is_some() && reporter.is_none() {
        return Err(Failure::Usage("--node-identity requires --report-config"));
    }
    if action == "report" {
        let world = arguments.take("world")?;
        let channel = arguments.take("channel")?;
        arguments.finish()?;
        let reporter =
            reporter.ok_or(Failure::Usage("assignment report requires --report-config"))?;
        return report(operator, &reporter, &world, &channel, node_identity).await;
    }
    let name = arguments.take("request")?;
    let writer = RuntimeScopeAssignmentWriter::open(operator.root.clone(), WRITER)
        .await
        .map_err(|error| Failure::Unavailable(format!("assignment writer: {error:?}")))?;
    if action == "reconcile" {
        if node_identity.is_some() {
            return Err(Failure::Usage(
                "--node-identity is not accepted by reconcile",
            ));
        }
        arguments.finish()?;
        let file = AssignmentRequestFile::decode(&operator.read_state(&name)?)
            .map_err(|_| Failure::Input("assignment request".into()))?;
        let request = request_of(&file)?;
        return match writer.reconcile(&request).await {
            Ok(ReconcileOutcome::Committed(receipt)) => {
                event(&format!(
                    "assignment=committed ownership_generation={} source_revision={}",
                    receipt.assignment.ownership_generation, receipt.assignment.source_revision
                ));
                report_committed(operator, reporter.as_ref(), &file).await
            }
            Ok(ReconcileOutcome::Absent) => {
                event("assignment=absent");
                Ok(())
            }
            Ok(ReconcileOutcome::Conflict) => Err(Failure::Rejected(
                "operation key committed another command".into(),
            )),
            Err(error) => Err(Failure::Ambiguous(format!("reconcile: {error:?}"))),
        };
    }
    // Any occupied writer slot must be reconciled before new work (D2).
    if let Some(key) = writer.unreconciled() {
        return Err(Failure::Ambiguous(format!(
            "operation {} is unreconciled; run `assignment reconcile` with its request file",
            hex(key.as_bytes())
        )));
    }
    let world = arguments.take("world")?;
    let channel = arguments.take("channel")?;
    let scope = scope_of(&world, &channel)?;
    let (target_node_id, target_registration_revision) = match action.as_str() {
        "assign" | "replace" => {
            let node = arguments.take("node-id")?;
            decode_uuid("node-id", &node)?;
            (
                Some(node),
                Some(parse_u64("revision", &arguments.take("revision")?)?),
            )
        }
        "revoke" => (None, None),
        _ => {
            return Err(Failure::Usage(
                "assignment assign|replace|revoke|reconcile|report",
            ));
        }
    };
    arguments.finish()?;
    // The reported node identity is validated before anything is written and
    // staged with the operation; it is bound to the target only on commit (§5).
    let staged = if let (Some(reporter), Some(_)) = (&reporter, &target_node_id) {
        let identity = node_identity.ok_or(Failure::Usage(
            "--report-config requires --node-identity for assign and replace",
        ))?;
        let (world_id, channel_id, _) = canonical_scope(&world, &channel)?;
        if !reporter.config.allows(&world_id, &channel_id, &identity) {
            return Err(Failure::Rejected(
                "node identity is not configured for the scope".into(),
            ));
        }
        Some(identity)
    } else if node_identity.is_some() && action == "revoke" {
        return Err(Failure::Usage(
            "--node-identity is refused for revoke: a revocation carries no node identity \
             (contract §16.1)",
        ));
    } else if node_identity.is_some() {
        return Err(Failure::Usage(
            "--node-identity is accepted by assign and replace",
        ));
    } else {
        None
    };
    let predecessor = if action == "assign" {
        None
    } else {
        Some(
            operator
                .root
                .read_runtime_scope_predecessor(scope)
                .await
                .map_err(|error| Failure::Unavailable(format!("predecessor: {error:?}")))?
                .ok_or_else(|| Failure::Rejected("scope has no assignment".into()))?,
        )
    };
    let file = AssignmentRequestFile {
        version: FILE_VERSION,
        operation_key: hex(
            &random_bytes::<32>().map_err(|_| Failure::Unavailable("randomness".into()))?
        ),
        actor: {
            operator.actor()?;
            operator.config.database.username.clone()
        },
        command: action,
        world_id: world,
        channel_id: channel,
        predecessor_generation: predecessor.map(|p| p.ownership_generation),
        predecessor_source_revision: predecessor.map(|p| p.source_revision),
        predecessor_guard_publication_revision: predecessor
            .and_then(|p| p.runtime_guard_publication_revision),
        target_node_id,
        target_registration_revision,
    };
    let request = request_of(&file)?;
    let bytes = file
        .encode()
        .map_err(|_| Failure::Input("assignment request".into()))?;
    if let (Some(identity), Some(holder)) = (&staged, target_holder(&file)?) {
        stage_identity(operator, &file.operation_key, &holder, identity)?;
    }
    operator.create_state(&name, None, &bytes)?;
    match writer.submit(&request).await {
        Ok(AssignmentOutcome::Committed(receipt)) => {
            event(&format!(
                "assignment=committed ownership_generation={} source_revision={}",
                receipt.assignment.ownership_generation, receipt.assignment.source_revision
            ));
            report_committed(operator, reporter.as_ref(), &file).await
        }
        Ok(AssignmentOutcome::Rejected(rejection)) => Err(Failure::Rejected(format!(
            "assignment rejected: {rejection:?}"
        ))),
        Err(error) => Err(Failure::Ambiguous(format!(
            "assignment outcome unknown ({error:?}); run `assignment reconcile --request {name}`"
        ))),
    }
}

/// #935 activation issuer: `content activate --world <uuid> --channel <uuid> --sequence <n>
/// --previous empty|<n> --request <file>`. The digests and frame binding are computed from the
/// selected native room qualified for the World (`OTERYN_NATIVE_GAMEPLAY_MANIFEST`, if set).
/// An existing request file is replayed
/// exactly (its recorded issuance succeeds again; anything else rejects).
async fn content(operator: &Operator, mut arguments: Arguments) -> Outcome {
    if arguments.words.get(1).map(String::as_str) != Some("activate") {
        return Err(Failure::Usage("content activate"));
    }
    let name = arguments.take("request")?;
    let file = match operator.read_state_optional(&name)? {
        Some(bytes) => {
            arguments.finish()?;
            ContentActivationRequestFile::decode(&bytes)
                .map_err(|_| Failure::Input("content activation request".into()))?
        }
        None => {
            let world_text = arguments.take("world")?;
            let channel_text = arguments.take("channel")?;
            let sequence = parse_u64("sequence", &arguments.take("sequence")?)?;
            let previous = match arguments.take("previous")?.as_str() {
                "empty" => None,
                text => Some(parse_u64("previous", text)?),
            };
            arguments.finish()?;
            let world = WorldId::decode(&decode_uuid("world", &world_text)?)
                .map_err(|_| Failure::Input("world".into()))?;
            ChannelId::decode(&decode_uuid("channel", &channel_text)?)
                .map_err(|_| Failure::Input("channel".into()))?;
            if world_text == channel_text {
                return Err(Failure::Input("world and channel must differ".into()));
            }
            let room = match std::env::var_os("OTERYN_NATIVE_GAMEPLAY_MANIFEST") {
                Some(path) => qualify_native_entry_room_from_gameplay_manifest(
                    world,
                    std::path::Path::new(&path),
                ),
                None => qualify_native_entry_room(world),
            }
            .map_err(|error| Failure::Input(format!("native entry room: {error}")))?;
            let file = ContentActivationRequestFile {
                version: FILE_VERSION,
                world_id: world_text,
                channel_id: channel_text,
                activation_sequence: sequence,
                previous_sequence: previous,
                server_artifact_digest: hex(&room.compiled().server_digest()),
                client_artifact_digest: hex(&room.compiled().client_digest()),
                frame_binding_digest: hex(&room.frame_binding().digest()),
            };
            let bytes = file
                .encode()
                .map_err(|_| Failure::Input("content activation request".into()))?;
            operator.create_state(&name, None, &bytes)?;
            file
        }
    };
    let invalid = || Failure::Input("content activation request".into());
    let request = ContentActivationRequest {
        world_id: WorldId::decode(&decode_uuid("world", &file.world_id)?).map_err(|_| invalid())?,
        channel_id: ChannelId::decode(&decode_uuid("channel", &file.channel_id)?)
            .map_err(|_| invalid())?,
        activation_sequence: file.activation_sequence,
        previous_sequence: file.previous_sequence,
        server_artifact_digest: hex_bytes::<32>(&file.server_artifact_digest)
            .map_err(|_| invalid())?,
        client_artifact_digest: hex_bytes::<32>(&file.client_artifact_digest)
            .map_err(|_| invalid())?,
        frame_binding_digest: hex_bytes::<32>(&file.frame_binding_digest).map_err(|_| invalid())?,
    };
    match operator.root.record_content_activation(&request).await {
        Ok(true) => {
            event(&format!(
                "content_activation=recorded activation_sequence={} server_digest={} client_digest={} frame_binding={}",
                file.activation_sequence,
                file.server_artifact_digest,
                file.client_artifact_digest,
                file.frame_binding_digest
            ));
            Ok(())
        }
        Ok(false) => Err(Failure::Rejected(
            "content activation is not granted for this scope, stale, not newer or conflicts with the recorded sequence".into(),
        )),
        Err(error) => Err(Failure::Ambiguous(format!(
            "content activation outcome unknown ({error:?}); re-run with the same --request {name}"
        ))),
    }
}

const PROJECTION_USAGE: &str = "projection resync --raise-epoch true|false: re-enqueue every \
account for ListCharactersForAccount and print the projection epoch. `true` raises the epoch \
above the epoch fence and then persists the fence; it uses the [projection] operator \
credential of the ops config. After a Character store restore: stop the node (its publisher), \
restore, run `projection resync --raise-epoch true`, then start the node.";

/// `projection resync` (contract §5): prints only the epoch.
async fn projection(operator: &Operator, mut arguments: Arguments) -> Outcome {
    use oteryn_game_server::native_admission_source::account_characters::EpochFenceFile;
    use oteryn_game_server::node::serve::{ResyncError, resync};
    if arguments.words.get(1).map(String::as_str) != Some("resync") || arguments.words.len() != 2 {
        return Err(Failure::Usage(PROJECTION_USAGE));
    }
    let raise = match arguments.take("raise-epoch")?.as_str() {
        "true" => true,
        "false" => false,
        _ => return Err(Failure::Usage(PROJECTION_USAGE)),
    };
    arguments.finish()?;
    let config = operator.config.projection.as_ref().ok_or(Failure::Usage(
        "projection resync requires the [projection] section",
    ))?;
    let root = connect_root(&config.database, secure_file::effective_uid()).await?;
    let mut fence = EpochFenceFile(config.epoch_fence_file.clone());
    match resync(&root, raise, &mut fence).await {
        Ok(epoch) => {
            println!("{epoch}");
            Ok(())
        }
        Err(error @ ResyncError::FenceInvalid) => Err(Failure::Input(error.to_string())),
        Err(
            error @ ResyncError::Durability(
                oteryn_game_server::durability::DurabilityError::CommitOutcomeUnknown,
            ),
        ) => Err(Failure::Ambiguous(error.to_string())),
        Err(error @ ResyncError::Durability(_)) => Err(Failure::Unavailable(error.to_string())),
        Err(
            error @ (ResyncError::NotAboveFence { .. }
            | ResyncError::Unexpected
            | ResyncError::FenceUnwritten { .. }),
        ) => Err(Failure::Rejected(error.to_string())),
    }
}

async fn run(raw: Vec<String>) -> Outcome {
    let mut arguments = Arguments::parse(raw)?;
    let config_path = PathBuf::from(arguments.take("config")?);
    // The one supported operator identity is root, never the service user.
    let invoking = secure_file::effective_uid();
    if invoking != 0 {
        return Err(Failure::Identity("oteryn-game-ops runs only as root"));
    }
    let document = read_file(
        "--config",
        &config_path,
        FileClass::Trusted,
        invoking,
        oteryn_game_server::node::config::MAX_CONFIG_BYTES,
    )?;
    let config = OpsConfig::parse(&document).map_err(|error| Failure::Input(error.to_string()))?;
    if config.operator.service_uid == invoking {
        return Err(Failure::Identity(
            "oteryn-game-ops refuses to run as the service user",
        ));
    }
    let state = secure_file::open_state_directory(&config.operator.state_directory, invoking)
        .map_err(|error| Failure::Input(format!("operator.state_directory: {error:?}")))?;
    let root = connect_root(&config.database, invoking).await?;
    let operator = Operator {
        config,
        state,
        root,
    };
    match arguments.words.first().map(String::as_str) {
        Some("authorization") => authorization(&operator, arguments).await,
        Some("registration") => registration(&operator, arguments).await,
        Some("s2") => s2(&operator, arguments).await,
        Some("character") => character(&operator, arguments).await,
        Some("assignment") => assignment(&operator, arguments).await,
        Some("content") => content(&operator, arguments).await,
        Some("projection") => projection(&operator, arguments).await,
        _ => Err(Failure::Usage(
            "oteryn-game-ops --config <path> authorization|registration|s2|character|assignment|content|projection ...",
        )),
    }
}

fn main() -> ExitCode {
    const PROCESS: &str = "oteryn-game-ops";
    let version = env!("CARGO_PKG_VERSION");
    if oteryn_game_server::node::serve::begin_process(
        PROCESS,
        version,
        FailureKind::LogSpecInvalid.code(),
    )
    .is_err()
    {
        return ExitCode::from(2);
    }
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.first().map(String::as_str) == Some("--version") {
        println!("{PROCESS} {}", oteryn_error_codes::build_id(version));
        return ExitCode::SUCCESS;
    }
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => return ExitCode::from(5),
    };
    match runtime.block_on(run(raw)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            let detail = failure.to_string();
            let line =
                oteryn_error_codes::Line::new(oteryn_error_codes::Level::Error, "ops", "failed")
                    .code(failure.kind().code())
                    .detail(&detail);
            oteryn_game_server::node::serve::emit(PROCESS, &line);
            ExitCode::from(failure.code())
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    #[allow(clippy::expect_used)]
    fn ops_codes_match_the_registry() {
        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json"
        ))
        .expect("game registry");
        for kind in FailureKind::ALL {
            let code = kind.code();
            let entry = registry["codes"]
                .as_array()
                .expect("codes")
                .iter()
                .find(|entry| entry["code"].as_u64() == Some(u64::from(code.number)))
                .expect("registered");
            assert_eq!(entry["name"], code.name);
            assert_eq!(entry["category"], code.category().as_str());
            assert_eq!(entry["progression"], code.progression().as_str());
        }
    }

    #[test]
    fn failures_keep_their_exit_status_and_carry_their_own_code() {
        let cases = [
            (Failure::Usage("x"), 2, 6001),
            (Failure::Input(String::new()), 3, 6002),
            (Failure::Identity("x"), 4, 6003),
            (Failure::Unavailable(String::new()), 5, 6004),
            (Failure::Rejected(String::new()), 6, 6005),
            (Failure::Ambiguous(String::new()), 7, 6006),
        ];
        for (failure, exit, number) in cases {
            assert_eq!(failure.code(), exit);
            assert_eq!(failure.kind().code().number, number);
        }
    }
    use std::cell::RefCell;

    #[derive(Default)]
    struct Files(RefCell<BTreeMap<String, Vec<u8>>>);

    impl StateFiles for Files {
        fn read_optional(&self, name: &str) -> Result<Option<Vec<u8>>, Failure> {
            Ok(self.0.borrow().get(name).cloned())
        }

        fn create(&self, name: &str, content: &[u8]) -> Outcome {
            match self.0.borrow_mut().entry(name.to_owned()) {
                std::collections::btree_map::Entry::Occupied(_) => {
                    Err(Failure::Input(format!("file {name}: exists")))
                }
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(content.to_vec());
                    Ok(())
                }
            }
        }
    }

    fn holder() -> NodeRegistrationFact {
        NodeRegistrationFact::new(
            NodeId::decode(
                &decode_uuid("node-id", "0190a8f2-7c3e-7b4a-8d2f-3e4a5b6c7d8e").unwrap(),
            )
            .unwrap(),
            4,
        )
    }

    #[test]
    fn an_uncommitted_operation_binds_no_identity() {
        let files = Files::default();
        stage_identity(&files, "aa", &holder(), "node-a").unwrap();
        assert_eq!(
            read_identity(&files, &identity_file(&holder())).unwrap(),
            None
        );
        // A corrected identity for the same holder is accepted afterwards.
        stage_identity(&files, "bb", &holder(), "node-b").unwrap();
        promote_identity(&files, "bb", &holder()).unwrap();
        assert_eq!(
            read_identity(&files, &identity_file(&holder())).unwrap(),
            Some("node-b".into())
        );
    }

    #[test]
    fn a_committed_operation_binds_its_staged_identity_once() {
        let files = Files::default();
        stage_identity(&files, "aa", &holder(), "node-a").unwrap();
        promote_identity(&files, "aa", &holder()).unwrap();
        // Promotion is idempotent for a re-reconciled commit.
        promote_identity(&files, "aa", &holder()).unwrap();
        assert_eq!(
            bind_identity(&files, &holder(), None).unwrap(),
            Some("node-a".into())
        );
        assert!(matches!(
            stage_identity(&files, "bb", &holder(), "node-b"),
            Err(Failure::Rejected(_))
        ));
        stage_identity(&files, "cc", &holder(), "node-a").unwrap();
        promote_identity(&files, "cc", &holder()).unwrap();
        // Nothing staged (a revoke, or reporting disabled) binds nothing.
        promote_identity(&files, "dd", &holder()).unwrap();
    }

    #[test]
    fn revocation_failures_exit_non_zero_naming_the_resend() {
        let world = "01934f10-7c02-7001-805b-3b1122334401";
        let channel = "01934f10-7c03-7001-805b-3b1122334401";
        assert!(revocation_outcome(Ok(Delivery::Accepted), world, channel).is_ok());
        assert!(revocation_outcome(Ok(Delivery::Superseded), world, channel).is_ok());
        for not_delivered in [
            NotDelivered::UnexpectedStatus,
            NotDelivered::Conflict,
            NotDelivered::Unauthenticated,
            NotDelivered::Unavailable,
            NotDelivered::RateLimited,
            NotDelivered::InvalidResponse,
        ] {
            let failure = revocation_outcome(Err(not_delivered), world, channel).unwrap_err();
            assert_ne!(failure.code(), 0);
            let text = failure.to_string();
            assert!(text.contains("`assignment report --world"), "{text}");
            assert!(text.contains(not_delivered.class()), "{text}");
        }
        // A 404 from a Platform without the endpoint is a definite stop.
        assert!(matches!(
            revocation_outcome(Err(NotDelivered::UnexpectedStatus), world, channel),
            Err(Failure::Rejected(_))
        ));
    }
}

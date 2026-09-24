//! `oteryn-game-ops`: control-plane actions for one GameNode deployment
//! (OPS-NODE-BOOT-01 D2).
//!
//! The tool runs only as root and never as the node's service user. It holds
//! only the control-plane database credential, never registers a GameNode
//! incarnation, and writes the complete request for every database or fence
//! mutation durably under its validated state directory before submitting it.

use oteryn_game_server::character_recovery_fence::CharacterRecoveryStore;
use oteryn_game_server::durability::DurabilityRoot;
use oteryn_game_server::durability::native_admission_source::{
    DescriptorRegistration, FreshStoreProvenance,
};
use oteryn_game_server::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentPredecessor, AssignmentRequest,
    BootstrapSecret, ControlActor, LaunchBinding, NodeRegistrationFact, OperationKey,
    ReconcileOutcome, RegistrationError, RuntimeScopeAssignmentWriter,
};
use oteryn_game_server::foundation::{ChannelId, NodeId, RuntimeScopeRefV1, WorldId};
use oteryn_game_server::node::config::{NodeConfig, OpsConfig};
use oteryn_game_server::node::descriptor_facts::{MAX_PEM_BYTES, certificates, descriptor_facts};
use oteryn_game_server::node::operator_files::{
    AssignmentRequestFile, CharacterRecoveryRequestFile, LaunchAuthorizationFile,
    MAX_OPERATOR_FILE_BYTES, S2AuthorizationFile, hex, hex_bytes, random_bytes, uuid_v7,
};
use oteryn_game_server::node::secure_file::{self, FileClass};
use oteryn_game_server::node::{StartupError, connect_root, read_file, uuid_bytes};
use rustix::fd::OwnedFd;
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

impl Failure {
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
        secure_file::read_in(
            &self.state,
            name,
            FileClass::Secret,
            self.owner_of(name)?,
            MAX_OPERATOR_FILE_BYTES,
        )
        .map_err(|error| Failure::Input(format!("retained file {name}: {error:?}")))
    }

    /// Node-read files belong to the service user; all others to root.
    fn owner_of(&self, name: &str) -> Result<u32, Failure> {
        let stat = rustix::fs::statat(&self.state, name, rustix::fs::AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|_| Failure::Input(format!("retained file {name}: missing")))?;
        if stat.st_uid == 0 || stat.st_uid == self.config.operator.service_uid {
            Ok(stat.st_uid)
        } else {
            Err(Failure::Input(format!("retained file {name}: Owner")))
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
            let request = match operator.owner_of(&name) {
                Ok(_) => CharacterRecoveryRequestFile::decode(&operator.read_state(&name)?)
                    .map_err(|_| Failure::Input("recovery request".into()))?,
                Err(_) => {
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
        _ => Err(Failure::Usage("character fresh-store|interpretation")),
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

async fn assignment(operator: &Operator, mut arguments: Arguments) -> Outcome {
    let action = arguments.words.get(1).cloned().unwrap_or_default();
    let name = arguments.take("request")?;
    let writer = RuntimeScopeAssignmentWriter::open(operator.root.clone(), WRITER)
        .await
        .map_err(|error| Failure::Unavailable(format!("assignment writer: {error:?}")))?;
    if action == "reconcile" {
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
                Ok(())
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
        _ => return Err(Failure::Usage("assignment assign|replace|revoke|reconcile")),
    };
    arguments.finish()?;
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
    operator.create_state(&name, None, &bytes)?;
    match writer.submit(&request).await {
        Ok(AssignmentOutcome::Committed(receipt)) => {
            event(&format!(
                "assignment=committed ownership_generation={} source_revision={}",
                receipt.assignment.ownership_generation, receipt.assignment.source_revision
            ));
            Ok(())
        }
        Ok(AssignmentOutcome::Rejected(rejection)) => Err(Failure::Rejected(format!(
            "assignment rejected: {rejection:?}"
        ))),
        Err(error) => Err(Failure::Ambiguous(format!(
            "assignment outcome unknown ({error:?}); run `assignment reconcile --request {name}`"
        ))),
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
        _ => Err(Failure::Usage(
            "oteryn-game-ops --config <path> authorization|registration|s2|character|assignment ...",
        )),
    }
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
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
            eprintln!("oteryn-game-ops: {failure}");
            ExitCode::from(failure.code())
        }
    }
}

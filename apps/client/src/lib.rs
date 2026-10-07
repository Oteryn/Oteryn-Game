//! Production pre-native client composition.
//! Terminal evidence is revalidated after the MPL-2.0 and Linux lint corrections.

pub mod cyclopedia;
pub mod input;
pub mod play;
pub mod scene;
pub mod spell;
#[cfg(windows)]
pub mod win_mutex;

use oteryn_client_runtime::{ClientRuntime, RuntimeError};
use oteryn_foundation::ProcessGeneration;
use oteryn_identity::{IdentityError, PkceMaterial};
use oteryn_input_actions::{ActionId, InputError};
use oteryn_input_platform::InputPlatformAdapter;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_platform_client::{PlatformClientConfig, PlatformClientError};
use oteryn_platform_contracts::GameplayAvailability;
use oteryn_renderer::SurfaceState;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

pub use native_entry::{AdmittedSession, NativeLoginConfig, NativeLoginSettingError};

/// The release this executable belongs to (CLIENT-INSTALLER-0 §2):
/// `<client_version>+<channel>.<route>.g<game_commit[0..12]>`, from `OTERYN_RELEASE_ID` at build
/// time. A build without it is a local developer build, `<client_version>+dev.local.g000000000000`.
pub const RELEASE_ID: &str = match option_env!("OTERYN_RELEASE_ID") {
    Some(release_id) => release_id,
    None => concat!(env!("CARGO_PKG_VERSION"), "+dev.local.g000000000000"),
};
const _: () = assert!(
    is_valid_release_id(RELEASE_ID),
    "OTERYN_RELEASE_ID must be <client_version>+<channel>.<route>.g<12 lowercase hex>, at most 50 bytes"
);

/// `client_build` reported to Platform and the Gateway: `oteryn-client/<release_id>`.
pub const CLIENT_BUILD: &str = {
    const PREFIX: &[u8] = b"oteryn-client/";
    const BYTES: [u8; PREFIX.len() + RELEASE_ID.len()] = {
        let mut bytes = [0; PREFIX.len() + RELEASE_ID.len()];
        let mut index = 0;
        while index < bytes.len() {
            bytes[index] = if index < PREFIX.len() {
                PREFIX[index]
            } else {
                RELEASE_ID.as_bytes()[index - PREFIX.len()]
            };
            index += 1;
        }
        bytes
    };
    match std::str::from_utf8(&BYTES) {
        Ok(client_build) => client_build,
        Err(_) => "oteryn-client/invalid",
    }
};

/// Whether `value` is a `<release_id>` of this client version: `<client_version>+<channel>.<route>.g<12 hex>`
/// with channel `dev`, `preproduction` or `stable`, route `ci` or `rel` (`local` only for the
/// all-zero commit of a developer build), lowercase hex, no `~`, at most 50 bytes.
#[must_use]
pub const fn is_valid_release_id(value: &str) -> bool {
    const CHANNELS: [&[u8]; 3] = [b"dev", b"preproduction", b"stable"];
    const ROUTES: [&[u8]; 3] = [b"ci", b"rel", b"local"];
    let bytes = value.as_bytes();
    let version = env!("CARGO_PKG_VERSION").as_bytes();
    if bytes.len() > 50 || !starts_with_at(bytes, 0, version) {
        return false;
    }
    let mut at = version.len();
    if !starts_with_at(bytes, at, b"+") {
        return false;
    }
    at += 1;
    let Some(channel_end) = matching_word(bytes, at, &CHANNELS) else {
        return false;
    };
    at = channel_end;
    if !starts_with_at(bytes, at, b".") {
        return false;
    }
    let Some(route_end) = matching_word(bytes, at + 1, &ROUTES) else {
        return false;
    };
    let local = route_end - (at + 1) == 5;
    at = route_end;
    if !starts_with_at(bytes, at, b".g") || bytes.len() != at + 14 {
        return false;
    }
    at += 2;
    while at < bytes.len() {
        let digit = bytes[at];
        if !(digit.is_ascii_digit() || (digit >= b'a' && digit <= b'f')) || (local && digit != b'0')
        {
            return false;
        }
        at += 1;
    }
    true
}

const fn starts_with_at(bytes: &[u8], at: usize, prefix: &[u8]) -> bool {
    if at + prefix.len() > bytes.len() {
        return false;
    }
    let mut index = 0;
    while index < prefix.len() {
        if bytes[at + index] != prefix[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// The end of the word of `words` that `bytes` holds at `at`, followed by a `.`.
const fn matching_word(bytes: &[u8], at: usize, words: &[&[u8]]) -> Option<usize> {
    let mut index = 0;
    while index < words.len() {
        let end = at + words[index].len();
        if starts_with_at(bytes, at, words[index]) && starts_with_at(bytes, end, b".") {
            return Some(end);
        }
        index += 1;
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayEntryError {
    /// No native login is configured: the pre-native client never requests a route or credential.
    NativeProtocolUnavailable,
    /// The configured native login settings are unusable (URL, character id or dev root).
    InvalidConfiguration,
    /// The Platform directory does not list the configured World; nothing was requested.
    WorldUnavailable,
    /// Login or admission ended with this FND-04A public class.
    Rejected(PublicClass),
}

impl Display for GameplayEntryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NativeProtocolUnavailable => "native gameplay protocol is not available",
            Self::InvalidConfiguration => "native login configuration is invalid",
            Self::WorldUnavailable => "the selected World is not available",
            Self::Rejected(PublicClass::RetryLogin) => "login failed; please try again",
            Self::Rejected(PublicClass::AuthenticationRequired) => "please sign in again",
            Self::Rejected(PublicClass::TemporarilyUnavailable) => {
                "the game is temporarily unavailable; please try again later"
            }
            Self::Rejected(PublicClass::SessionUnavailable) => "the game session is unavailable",
            Self::Rejected(PublicClass::ClientUpdateRequired) => "please update the client",
            Self::Rejected(PublicClass::CharacterAlreadyActive) => {
                "this character is already in game"
            }
        })
    }
}

impl std::error::Error for GameplayEntryError {}

pub struct ClientBootstrap {
    runtime: ClientRuntime,
    renderer_state: SurfaceState,
    input_adapter: InputPlatformAdapter,
    native_login: Option<NativeLoginConfig>,
}

impl ClientBootstrap {
    pub fn new() -> Result<Self, RuntimeError> {
        Ok(Self {
            runtime: ClientRuntime::new()?,
            renderer_state: SurfaceState::new(ProcessGeneration::new(1)),
            input_adapter: InputPlatformAdapter::new(),
            native_login: None,
        })
    }

    /// Enables native gameplay entry (ADR-0011 order: availability, credential, connect).
    #[must_use]
    pub fn with_native_login(mut self, config: NativeLoginConfig) -> Self {
        self.native_login = Some(config);
        self
    }

    #[must_use]
    pub const fn availability(&self) -> GameplayAvailability {
        GameplayAvailability::PreNativeProtocol
    }

    #[must_use]
    pub const fn renderer_state(&self) -> &SurfaceState {
        &self.renderer_state
    }

    #[must_use]
    pub const fn input_adapter(&self) -> &InputPlatformAdapter {
        &self.input_adapter
    }

    pub fn platform_config(base_url: &str) -> Result<PlatformClientConfig, PlatformClientError> {
        PlatformClientConfig::new(base_url)
    }

    pub fn pkce_from_entropy(entropy: &[u8]) -> Result<PkceMaterial, IdentityError> {
        PkceMaterial::from_entropy(entropy)
    }

    pub fn validate_action_id(value: &str) -> Result<ActionId, InputError> {
        ActionId::new(value.to_owned())
    }

    /// Runs native login to an admitted session, or returns the public class that stopped it.
    /// Without native login configured it fails before any route or credential request.
    pub fn request_gameplay_entry(&self) -> Result<AdmittedSession, GameplayEntryError> {
        let Some(config) = &self.native_login else {
            return Err(GameplayEntryError::NativeProtocolUnavailable);
        };
        self.runtime
            .block_on(native_entry::enter(config, self.runtime.cancellation()))
            .map_err(|_error| GameplayEntryError::Rejected(PublicClass::TemporarilyUnavailable))?
    }

    /// Starts the admitted session's task on the client runtime, whose reactor its stream is
    /// bound to, and returns the placeholder view with the shell's link to the task.
    pub fn start_play(
        &self,
        admitted: AdmittedSession,
    ) -> Result<(play::PlayView, play::PlayLink), GameplayEntryError> {
        let unavailable = || GameplayEntryError::Rejected(PublicClass::SessionUnavailable);
        let view = admitted.play_view().map_err(|_error| unavailable())?;
        let (link, commands, events) = play::play_channel();
        self.runtime
            .spawn(play::run_session(admitted.into_session(), commands, events))
            .map_err(|_error| unavailable())?;
        Ok((view, link))
    }

    pub fn shutdown(self) {
        self.runtime.shutdown(Duration::from_millis(250));
    }
}

#[must_use]
pub const fn pre_native_status() -> &'static str {
    GameplayAvailability::PreNativeProtocol.player_message()
}

/// Native login to an admitted session (N4-1): directory, OAuth + PKCE in the system browser,
/// native ticket, Gateway login, TLS connect, admission. Tokens, tickets and grants stay in memory,
/// are never logged and are dropped after use.
mod native_entry {
    use super::{CLIENT_BUILD, GameplayEntryError};
    use oteryn_foundation::{Cancellable, CancellationToken, cancellable};
    use oteryn_identity::{IdentityFlow, LoopbackRedirect, PkceMaterial, StateNonce};
    use oteryn_platform_client::native_login::{
        AttemptRef, GatewayLogin, NativeLoginClient, NativeLoginError, NativeLoginGrant,
        NativeTicket, PublicClass, RetryPacing, parse_uuid_v7, secure_random,
    };
    use oteryn_platform_client::{PlatformClient, PlatformClientConfig};
    use oteryn_session::{
        Admission, AdmissionPublicClass, CLIENT_SUPPORTED_CAPABILITIES, CharacterId, Session,
        SessionError, admission_refusal_class,
    };
    use oteryn_session_tcp::{TcpConnect, TcpTlsStream, connect, load_root_certificate};
    use rustls::pki_types::CertificateDer;
    use std::fmt::{self, Debug, Display, Formatter};
    use std::net::{IpAddr, SocketAddr};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// FND-02 `ClientBootstrap.schema_revision` this client sends.
    const SCHEMA_REVISION: u32 = 1;
    const DEFAULT_CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);
    /// Bounds the endpoint name lookup, the TCP connect, the TLS handshake and each admission read.
    const CONNECT_DEADLINE: Duration = Duration::from_secs(10);
    // N8 (packet §1.2) admission refusal codes the client retries.
    const GRANT_NOT_YET_VALID: u32 = 1103;
    const ATTEMPT_RECONCILIATION_REQUIRED: u32 = 1106;
    const GRANT_SECURITY_EVIDENCE_STALE: u32 = 1108;
    const CAPACITY_EXCEEDED: u32 = 1115;
    /// Retries of the same grant on 1103/1108, and Gateway re-logins on 1106.
    const MAX_GRANT_RETRIES: u32 = 3;

    pub type BrowserOpener = Arc<dyn Fn(&str) -> bool + Send + Sync>;

    /// A required native login setting that is absent or empty.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NativeLoginSettingError(pub &'static str);

    impl Display for NativeLoginSettingError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            write!(formatter, "native login setting {} is missing", self.0)
        }
    }

    impl std::error::Error for NativeLoginSettingError {}

    /// Native login settings (character mode 33a: the character comes from configuration).
    #[derive(Clone)]
    pub struct NativeLoginConfig {
        pub platform_url: String,
        pub gateway_url: String,
        pub oauth_client_id: String,
        /// The directory `world_ref` that must be listed before any credential is requested.
        pub world_ref: String,
        /// Canonical lowercase UUIDv7.
        pub character_id: String,
        /// The only trust root the game endpoint's certificate may chain to (PEM or DER).
        pub dev_root: PathBuf,
        pub client_build: String,
        pub callback_timeout: Duration,
        /// Backoff unit of every retry; one second in production.
        pub retry_unit: Duration,
        pub open_browser: BrowserOpener,
    }

    impl Debug for NativeLoginConfig {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter
                .debug_struct("NativeLoginConfig")
                .field("platform_url", &self.platform_url)
                .field("gateway_url", &self.gateway_url)
                .field("world_ref", &self.world_ref)
                .field("dev_root", &self.dev_root)
                .finish_non_exhaustive()
        }
    }

    impl NativeLoginConfig {
        /// Reads `OTERYN_PLATFORM_URL`, `OTERYN_GATEWAY_URL`, `OTERYN_OAUTH_CLIENT_ID`,
        /// `OTERYN_WORLD`, `OTERYN_CHARACTER_ID` and `OTERYN_DEV_ROOT`. Native login is off
        /// (`Ok(None)`) while `OTERYN_PLATFORM_URL` is unset.
        pub fn from_env(
            lookup: impl Fn(&str) -> Option<String>,
        ) -> Result<Option<Self>, NativeLoginSettingError> {
            let setting = |name: &'static str| {
                lookup(name)
                    .filter(|value| !value.is_empty())
                    .ok_or(NativeLoginSettingError(name))
            };
            let Ok(platform_url) = setting("OTERYN_PLATFORM_URL") else {
                return Ok(None);
            };
            Ok(Some(Self {
                platform_url,
                gateway_url: setting("OTERYN_GATEWAY_URL")?,
                oauth_client_id: setting("OTERYN_OAUTH_CLIENT_ID")?,
                world_ref: setting("OTERYN_WORLD")?,
                character_id: setting("OTERYN_CHARACTER_ID")?,
                dev_root: PathBuf::from(setting("OTERYN_DEV_ROOT")?),
                client_build: CLIENT_BUILD.to_owned(),
                callback_timeout: DEFAULT_CALLBACK_TIMEOUT,
                retry_unit: Duration::from_secs(1),
                open_browser: Arc::new(open_system_browser),
            }))
        }
    }

    /// Opens `url` in the system browser without waiting for it.
    pub fn open_system_browser(url: &str) -> bool {
        #[cfg(windows)]
        let mut command = {
            let mut command = std::process::Command::new("rundll32");
            command.args(["url.dll,FileProtocolHandler", url]);
            command
        };
        #[cfg(not(windows))]
        let mut command = {
            let mut command = std::process::Command::new("xdg-open");
            command.arg(url);
            command
        };
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map(|mut child| {
                std::thread::spawn(move || child.wait());
            })
            .is_ok()
    }

    /// An admitted game session on the routed World and Channel.
    #[derive(Debug)]
    pub struct AdmittedSession {
        session: Session<TcpTlsStream>,
        world_id: String,
        channel_id: String,
    }

    impl AdmittedSession {
        #[must_use]
        pub fn world_id(&self) -> &str {
            &self.world_id
        }

        #[must_use]
        pub fn channel_id(&self) -> &str {
            &self.channel_id
        }

        /// The placeholder view of the join snapshot.
        pub fn play_view(&self) -> Result<crate::play::PlayView, oteryn_renderer::BatchError> {
            crate::play::PlayView::from_join(self.session.join_snapshot())
        }

        #[must_use]
        pub fn into_session(self) -> Session<TcpTlsStream> {
            self.session
        }
    }

    const fn rejected(class: PublicClass) -> GameplayEntryError {
        GameplayEntryError::Rejected(class)
    }

    /// Any failure of the browser sign-in itself: the player starts the login again.
    fn retry_login<E>(_error: E) -> GameplayEntryError {
        rejected(PublicClass::RetryLogin)
    }

    fn login_rejected(error: NativeLoginError) -> GameplayEntryError {
        rejected(error.public_class())
    }

    const fn admission_class(class: AdmissionPublicClass) -> PublicClass {
        match class {
            AdmissionPublicClass::RetryLogin => PublicClass::RetryLogin,
            AdmissionPublicClass::AuthenticationRequired => PublicClass::AuthenticationRequired,
            AdmissionPublicClass::TemporarilyUnavailable => PublicClass::TemporarilyUnavailable,
            AdmissionPublicClass::SessionUnavailable => PublicClass::SessionUnavailable,
            AdmissionPublicClass::ClientUpdateRequired => PublicClass::ClientUpdateRequired,
            AdmissionPublicClass::CharacterAlreadyActive => PublicClass::CharacterAlreadyActive,
        }
    }

    pub(crate) async fn enter(
        config: &NativeLoginConfig,
        cancellation: CancellationToken,
    ) -> Result<AdmittedSession, GameplayEntryError> {
        let platform = PlatformClientConfig::new(&config.platform_url)
            .map_err(|_error| GameplayEntryError::InvalidConfiguration)?;
        let gateway = PlatformClientConfig::new(&config.gateway_url)
            .map_err(|_error| GameplayEntryError::InvalidConfiguration)?;
        let character_id = parse_uuid_v7(&config.character_id)
            .and_then(|bytes| CharacterId::decode(&bytes).ok())
            .ok_or(GameplayEntryError::InvalidConfiguration)?;

        // 1. Availability before any credential.
        let directory = PlatformClient::new(platform.clone())
            .map_err(|_error| GameplayEntryError::InvalidConfiguration)?
            .fetch_directory(cancellation.clone())
            .await
            .map_err(|_error| rejected(PublicClass::TemporarilyUnavailable))?;
        if !directory
            .worlds
            .iter()
            .any(|world| world.world_ref.as_str() == config.world_ref)
        {
            return Err(GameplayEntryError::WorldUnavailable);
        }
        let root = load_root_certificate(&config.dev_root)
            .map_err(|_error| GameplayEntryError::InvalidConfiguration)?;

        // 2-3. OAuth code + PKCE in the system browser, then the native ticket.
        let client = NativeLoginClient::new(&platform, &gateway, &config.oauth_client_id)
            .map_err(|_error| GameplayEntryError::InvalidConfiguration)?
            .with_pacing(RetryPacing {
                unit: config.retry_unit,
                ..RetryPacing::CONTRACT
            });
        let ticket = authorize(&client, config, cancellation.clone()).await?;

        // 4. Gateway login: one attempt_ref for this whole attempt.
        let attempt_ref = AttemptRef::generate()
            .map_err(|_error| rejected(PublicClass::TemporarilyUnavailable))?;
        let login = GatewayLogin {
            attempt_ref: &attempt_ref,
            character_id: &config.character_id,
            client_build: &config.client_build,
        };
        let mut grant = client
            .gateway_login(&ticket, &login, cancellation.clone())
            .await
            .map_err(login_rejected)?;

        // 5-6. Connect and admit, retrying only the N8 RETRYABLE rows.
        let mut grant_retries = 0;
        let mut reconciliations = 0;
        let mut capacity_retried = false;
        loop {
            let code = match admit(&grant, &root, character_id, config).await {
                Ok(session) => {
                    return Ok(AdmittedSession {
                        session,
                        world_id: grant.world_id.clone(),
                        channel_id: grant.channel_id.clone(),
                    });
                }
                Err(Some(code)) => code,
                Err(None) => return Err(rejected(PublicClass::TemporarilyUnavailable)),
            };
            let class = admission_refusal_class(code).map_or(
                PublicClass::TemporarilyUnavailable,
                |(_progression, class)| admission_class(class),
            );
            let delay = match code {
                GRANT_NOT_YET_VALID | GRANT_SECURITY_EVIDENCE_STALE
                    if grant_retries < MAX_GRANT_RETRIES =>
                {
                    grant_retries += 1;
                    config.retry_unit * (1 << (grant_retries - 1))
                }
                CAPACITY_EXCEEDED if !capacity_retried => {
                    capacity_retried = true;
                    config.retry_unit * 2
                }
                ATTEMPT_RECONCILIATION_REQUIRED if reconciliations < MAX_GRANT_RETRIES => {
                    // The same attempt_ref and ticket: Platform returns the identical grant.
                    reconciliations += 1;
                    grant = client
                        .gateway_login(&ticket, &login, cancellation.clone())
                        .await
                        .map_err(login_rejected)?;
                    continue;
                }
                _ => return Err(rejected(class)),
            };
            if Instant::now() + delay >= grant.usable_until() {
                return Err(rejected(class));
            }
            match cancellable(cancellation.clone(), tokio::time::sleep(delay)).await {
                Cancellable::Completed(()) => {}
                Cancellable::Cancelled => return Err(rejected(class)),
            }
        }
    }

    async fn authorize(
        client: &NativeLoginClient,
        config: &NativeLoginConfig,
        cancellation: CancellationToken,
    ) -> Result<NativeTicket, GameplayEntryError> {
        let mut entropy = [0_u8; 32];
        secure_random(&mut entropy).map_err(retry_login)?;
        let pkce = PkceMaterial::from_entropy(&entropy).map_err(retry_login)?;
        let mut nonce = [0_u8; 16];
        secure_random(&mut nonce).map_err(retry_login)?;
        let state = StateNonce::new(
            nonce
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        )
        .map_err(retry_login)?;
        let redirect = LoopbackRedirect::bind().await.map_err(retry_login)?;
        let redirect_uri = redirect.redirect_uri();
        let url = client
            .authorization_url(&redirect_uri, state.as_str(), pkce.challenge())
            .map_err(|_error| GameplayEntryError::InvalidConfiguration)?;
        if !(config.open_browser)(url.as_str()) {
            return Err(rejected(PublicClass::RetryLogin));
        }
        let flow = IdentityFlow::new(state, config.callback_timeout);
        let code = redirect
            .await_code(&flow, cancellation.clone())
            .await
            .map_err(retry_login)?;
        let token = client
            .exchange_code(
                code.expose().expose(),
                pkce.verifier().expose(),
                &redirect_uri,
                cancellation.clone(),
            )
            .await
            .map_err(login_rejected)?;
        client
            .issue_native_ticket(token, cancellation)
            .await
            .map_err(login_rejected)
    }

    /// One connect and admission. `Err(Some(code))` is an admission refusal; `Err(None)` is any
    /// other failure (lookup, connect, TLS, I/O, close, timeout), which is never retried.
    async fn admit(
        grant: &NativeLoginGrant,
        root: &CertificateDer<'static>,
        character_id: CharacterId,
        config: &NativeLoginConfig,
    ) -> Result<Session<TcpTlsStream>, Option<u32>> {
        let addresses = resolve(&grant.endpoint.host, grant.endpoint.port).await;
        let stream = connect_any(&addresses, &grant.endpoint.tls_server_name, root)
            .await
            .ok_or(None)?;
        Session::admit(
            stream,
            Admission {
                schema_revision: SCHEMA_REVISION,
                character_id,
                admission_material: grant.grant_bytes(),
                client_build_id: &config.client_build,
                supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
                deadline: CONNECT_DEADLINE,
            },
        )
        .await
        .map_err(|error| match error {
            SessionError::AdmissionRefused { code } => Some(code),
            _ => None,
        })
    }

    /// Every address of the routed endpoint, in resolver order; empty when the lookup fails.
    async fn resolve(host: &str, port: u16) -> Vec<SocketAddr> {
        if let Ok(ip) = host.parse::<IpAddr>() {
            return vec![SocketAddr::new(ip, port)];
        }
        match tokio::time::timeout(CONNECT_DEADLINE, tokio::net::lookup_host((host, port))).await {
            Ok(Ok(addresses)) => addresses.collect(),
            _ => Vec::new(),
        }
    }

    /// The first address that completes TCP connect and the TLS handshake; later addresses are
    /// tried when an earlier one fails.
    async fn connect_any(
        addresses: &[SocketAddr],
        server_name: &str,
        root: &CertificateDer<'static>,
    ) -> Option<TcpTlsStream> {
        for &address in addresses {
            if let Ok(stream) = connect(TcpConnect {
                address,
                server_name,
                root_certificate: root,
                deadline: CONNECT_DEADLINE,
            })
            .await
            {
                return Some(stream);
            }
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::ClientBootstrap;
        use oteryn_client_runtime::ClientRuntime;
        use rustls::pki_types::PrivatePkcs8KeyDer;
        use std::io::{Read as _, Write as _};
        use std::sync::Mutex;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;
        use tokio_rustls::TlsAcceptor;

        type TestResult = Result<(), Box<dyn std::error::Error>>;

        const WORLD_REF: &str = "w-alpha";
        const CHARACTER: &str = "01890a5d-ac96-774b-bcce-b302099a8058";
        const WORLD: &str = "01890a5d-ac96-774b-bcce-b302099a8059";
        const CHANNEL: &str = "01890a5d-ac96-774b-bcce-b302099a805a";
        const JWS: &str = "eyJhbGciOiJFZERTQSJ9.eyJzdWIiOiJ4In0.c2lnbmF0dXJl";

        /// A local Platform + Gateway (one response per connection) and a TLS game endpoint
        /// that answers every admission with the next scripted refusal code.
        struct Stack {
            servers: ClientRuntime,
            http_url: String,
            requests: Arc<Mutex<Vec<String>>>,
            admissions: Arc<AtomicUsize>,
            root_path: PathBuf,
            game_port: u16,
        }

        impl Stack {
            fn start(refusals: Vec<u32>) -> Result<Self, Box<dyn std::error::Error>> {
                let servers = ClientRuntime::new()?;
                let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
                let root = generated.cert.der().clone();
                static STACKS: AtomicUsize = AtomicUsize::new(0);
                let root_path = std::env::temp_dir().join(format!(
                    "oteryn-n4-root-{}-{}.der",
                    std::process::id(),
                    STACKS.fetch_add(1, Ordering::SeqCst)
                ));
                std::fs::write(&root_path, root.as_ref())?;
                let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
                let mut tls = rustls::ServerConfig::builder_with_provider(Arc::new(
                    rustls::crypto::aws_lc_rs::default_provider(),
                ))
                .with_protocol_versions(&[&rustls::version::TLS13])?
                .with_no_client_auth()
                .with_single_cert(vec![root], key.into())?;
                tls.alpn_protocols = vec![b"oteryn-game/1".to_vec()];
                let acceptor = TlsAcceptor::from(Arc::new(tls));
                let game = servers.block_on(TcpListener::bind("127.0.0.1:0"))??;
                let game_port = game.local_addr()?.port();
                let admissions = Arc::new(AtomicUsize::new(0));
                let counter = Arc::clone(&admissions);
                servers.spawn(async move {
                    while let Ok((tcp, _peer)) = game.accept().await {
                        let index = counter.fetch_add(1, Ordering::SeqCst);
                        let code = refusals
                            .get(index)
                            .or(refusals.last())
                            .copied()
                            .unwrap_or(1100);
                        let Ok(mut stream) = acceptor.accept(tcp).await else {
                            continue;
                        };
                        let _ = oteryn_session::read_frame(&mut stream).await;
                        let mut frame = vec![0x08, 0x0e, 0x22, 0x05, 0x08];
                        frame.extend([(code & 0x7f) as u8 | 0x80, (code >> 7) as u8]);
                        frame.extend([0x10, 0x04]);
                        let _ = oteryn_session::write_frame(&mut stream, &frame).await;
                        let _ = stream.shutdown().await;
                    }
                })?;
                let http = servers.block_on(TcpListener::bind("127.0.0.1:0"))??;
                let http_url = format!("http://127.0.0.1:{}/", http.local_addr()?.port());
                let requests = Arc::new(Mutex::new(Vec::new()));
                let log = Arc::clone(&requests);
                servers.spawn(async move {
                    while let Ok((stream, _peer)) = http.accept().await {
                        serve_http(stream, game_port, &log).await;
                    }
                })?;
                Ok(Self {
                    servers,
                    http_url,
                    requests,
                    admissions,
                    root_path,
                    game_port,
                })
            }

            fn config(&self) -> NativeLoginConfig {
                NativeLoginConfig {
                    platform_url: self.http_url.clone(),
                    gateway_url: self.http_url.clone(),
                    oauth_client_id: "oteryn-native".to_owned(),
                    world_ref: WORLD_REF.to_owned(),
                    character_id: CHARACTER.to_owned(),
                    dev_root: self.root_path.clone(),
                    client_build: "oteryn-client/test".to_owned(),
                    callback_timeout: Duration::from_secs(10),
                    retry_unit: Duration::from_millis(10),
                    open_browser: Arc::new(complete_browser_sign_in),
                }
            }

            fn requests(&self) -> Vec<String> {
                self.requests
                    .lock()
                    .map(|requests| requests.clone())
                    .unwrap_or_default()
            }

            fn entry(
                &self,
                config: NativeLoginConfig,
            ) -> Result<GameplayEntryError, Box<dyn std::error::Error>> {
                let client = ClientBootstrap::new()?.with_native_login(config);
                let result = client.request_gameplay_entry();
                client.shutdown();
                match result {
                    Ok(_admitted) => Err("refusal-only server admitted".into()),
                    Err(error) => Ok(error),
                }
            }

            fn finish(self) {
                let _ = std::fs::remove_file(&self.root_path);
                self.servers.shutdown(Duration::from_millis(100));
            }
        }

        #[test]
        fn an_unreachable_first_address_falls_through_to_the_next() -> TestResult {
            let stack = Stack::start(vec![1100])?;
            let root = CertificateDer::from(std::fs::read(&stack.root_path)?);
            let dead = std::net::TcpListener::bind("127.0.0.1:0")?.local_addr()?;
            let live = SocketAddr::from(([127, 0, 0, 1], stack.game_port));
            let connected = stack.servers.block_on(async {
                (
                    connect_any(&[dead], "localhost", &root).await.is_some(),
                    connect_any(&[dead, live], "localhost", &root)
                        .await
                        .is_some(),
                )
            })?;
            stack.finish();
            assert_eq!(connected, (false, true));
            Ok(())
        }

        /// The browser: signs in at once and follows the redirect with the exact `state`.
        fn complete_browser_sign_in(url: &str) -> bool {
            let query_value = |name: &str| {
                url.split(['?', '&'])
                    .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
                    .map(|value| value.replace("%3A", ":").replace("%2F", "/"))
            };
            let (Some(redirect), Some(state)) = (query_value("redirect_uri"), query_value("state"))
            else {
                return false;
            };
            let Some(authority) = redirect
                .strip_prefix("http://")
                .and_then(|rest| rest.strip_suffix("/callback"))
                .map(str::to_owned)
            else {
                return false;
            };
            std::thread::spawn(move || {
                if let Ok(mut stream) = std::net::TcpStream::connect(&authority) {
                    let _ = write!(
                        stream,
                        "GET /callback?code=code-1&state={state} HTTP/1.1\r\nhost: {authority}\r\n\r\n"
                    );
                    let _ = stream.read_to_end(&mut Vec::new());
                }
            });
            true
        }

        async fn serve_http(
            mut stream: tokio::net::TcpStream,
            game_port: u16,
            log: &Mutex<Vec<String>>,
        ) {
            let mut request = Vec::new();
            let mut chunk = [0_u8; 4096];
            let (head, body) = loop {
                let Ok(read) = stream.read(&mut chunk).await else {
                    return;
                };
                if read == 0 {
                    return;
                }
                request.extend_from_slice(&chunk[..read]);
                let text = String::from_utf8_lossy(&request).into_owned();
                if let Some((head, body)) = text.split_once("\r\n\r\n") {
                    let length = head
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if body.len() >= length {
                        break (head.to_owned(), body.to_owned());
                    }
                }
            };
            let line = head.lines().next().unwrap_or_default();
            let target = line.split(' ').take(2).collect::<Vec<_>>().join(" ");
            let response = match target.as_str() {
                "GET /v1/client/directory" => format!(
                    r#"{{"epoch":1,"worlds":[{{"id":"{WORLD_REF}","name":"Alpha","channels":[],"characters":[]}}]}}"#
                ),
                "POST /oauth/token" => r#"{"token_type":"Bearer","access_token":"access-1","expires_in":300,"scope":"game:ticket"}"#.to_owned(),
                "POST /v1/game-auth/tickets" => {
                    r#"{"protocol_version":1,"ticket":"ticket-1","expires_in":60}"#.to_owned()
                }
                "POST /v1/login" => {
                    let attempt_ref = body
                        .split_once(r#""attempt_ref":""#)
                        .and_then(|(_, rest)| rest.get(..36))
                        .unwrap_or_default();
                    format!(
                        r#"{{"protocol_version":2,"attempt_ref":"{attempt_ref}","world_id":"{WORLD}","channel_id":"{CHANNEL}","endpoint":{{"host":"127.0.0.1","port":{game_port},"tls_server_name":"localhost","alpn":"oteryn-game/1","protocol_major":1,"transport_profile":1}},"grant":{{"profile":"oteryn-pre-admission-v1","token":"{JWS}","valid_for_seconds":30}}}}"#
                    )
                }
                _ => String::new(),
            };
            if let Ok(mut requests) = log.lock() {
                requests.push(format!("{target} {body}"));
            }
            let status = if response.is_empty() {
                "404 Not Found"
            } else {
                "200 OK"
            };
            let _ = stream
                .write_all(
                    format!(
                        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}",
                        response.len()
                    )
                    .as_bytes(),
                )
                .await;
            let _ = stream.shutdown().await;
        }

        fn count(requests: &[String], target: &str) -> usize {
            requests
                .iter()
                .filter(|request| request.starts_with(target))
                .count()
        }

        #[test]
        fn absent_world_stops_before_any_credential_request() -> TestResult {
            let stack = Stack::start(vec![1100])?;
            let mut config = stack.config();
            config.world_ref = "w-other".to_owned();
            assert_eq!(stack.entry(config)?, GameplayEntryError::WorldUnavailable);
            let requests = stack.requests();
            assert_eq!(requests.len(), 1);
            assert_eq!(count(&requests, "GET /v1/client/directory"), 1);
            assert_eq!(stack.admissions.load(Ordering::SeqCst), 0);
            stack.finish();
            Ok(())
        }

        #[test]
        fn terminal_refusal_is_its_public_class_without_retry() -> TestResult {
            // 1105 ADMISSION_GRANT_REPLAYED: SECURITY_TERMINAL, SESSION_UNAVAILABLE.
            let stack = Stack::start(vec![1105])?;
            assert_eq!(
                stack.entry(stack.config())?,
                GameplayEntryError::Rejected(PublicClass::SessionUnavailable)
            );
            let requests = stack.requests();
            for target in [
                "GET /v1/client/directory",
                "POST /oauth/token",
                "POST /v1/game-auth/tickets",
                "POST /v1/login",
            ] {
                assert_eq!(count(&requests, target), 1, "{target}");
            }
            let token = requests
                .iter()
                .find(|request| request.starts_with("POST /oauth/token"))
                .cloned()
                .unwrap_or_default();
            assert!(token.contains("code=code-1"));
            assert!(token.contains("code_verifier="));
            assert_eq!(stack.admissions.load(Ordering::SeqCst), 1);
            stack.finish();
            Ok(())
        }

        #[test]
        fn not_yet_valid_retries_the_same_grant_three_times() -> TestResult {
            let stack = Stack::start(vec![1103])?;
            assert_eq!(
                stack.entry(stack.config())?,
                GameplayEntryError::Rejected(PublicClass::TemporarilyUnavailable)
            );
            assert_eq!(stack.admissions.load(Ordering::SeqCst), 4);
            assert_eq!(count(&stack.requests(), "POST /v1/login"), 1);
            stack.finish();
            Ok(())
        }

        #[test]
        fn capacity_exceeded_retries_once() -> TestResult {
            let stack = Stack::start(vec![1115])?;
            assert_eq!(
                stack.entry(stack.config())?,
                GameplayEntryError::Rejected(PublicClass::TemporarilyUnavailable)
            );
            assert_eq!(stack.admissions.load(Ordering::SeqCst), 2);
            stack.finish();
            Ok(())
        }

        #[test]
        fn reconciliation_repeats_the_gateway_login_with_the_same_attempt() -> TestResult {
            let stack = Stack::start(vec![1106, 1105])?;
            assert_eq!(
                stack.entry(stack.config())?,
                GameplayEntryError::Rejected(PublicClass::SessionUnavailable)
            );
            let logins = stack
                .requests()
                .into_iter()
                .filter(|request| request.starts_with("POST /v1/login"))
                .collect::<Vec<_>>();
            assert_eq!(logins.len(), 2);
            assert_eq!(logins.first(), logins.get(1));
            assert_eq!(count(&stack.requests(), "POST /v1/game-auth/tickets"), 1);
            assert_eq!(stack.admissions.load(Ordering::SeqCst), 2);
            stack.finish();
            Ok(())
        }

        #[test]
        fn configuration_is_read_only_when_the_platform_is_set() -> TestResult {
            assert!(NativeLoginConfig::from_env(|_name| None)?.is_none());
            let partial = NativeLoginConfig::from_env(|name| {
                (name == "OTERYN_PLATFORM_URL").then(|| "https://platform.test/".to_owned())
            });
            assert_eq!(
                partial.err(),
                Some(NativeLoginSettingError("OTERYN_GATEWAY_URL"))
            );
            let config = NativeLoginConfig::from_env(|name| Some(format!("{name}-value")))?
                .ok_or("configured")?;
            assert_eq!(config.character_id, "OTERYN_CHARACTER_ID-value");
            assert!(!format!("{config:?}").contains("OTERYN_OAUTH_CLIENT_ID"));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_id_and_client_build_are_compiled_in() {
        assert!(is_valid_release_id(RELEASE_ID));
        assert_eq!(CLIENT_BUILD, format!("oteryn-client/{RELEASE_ID}"));
        assert!(CLIENT_BUILD.len() <= 64);
    }

    #[test]
    fn release_id_format_is_enforced() {
        let version = env!("CARGO_PKG_VERSION");
        for valid in [
            "+dev.ci.g1a2b3c4d5e6f",
            "+preproduction.rel.g0123456789ab",
            "+stable.ci.gabcdef012345",
            "+dev.local.g000000000000",
        ] {
            assert!(is_valid_release_id(&format!("{version}{valid}")), "{valid}");
        }
        for invalid in [
            "+dev.ci.g1a2b3c4d5e6",
            "+dev.ci.g1a2b3c4d5e6f0",
            "+dev.ci.g1A2B3C4D5E6F",
            "+dev.ci.g1a2b3c4d5e6~",
            "+beta.ci.g1a2b3c4d5e6f",
            "+dev.nightly.g1a2b3c4d5e6f",
            "+dev.local.g1a2b3c4d5e6f",
            "+dev.ci.1a2b3c4d5e6f",
            "dev.ci.g1a2b3c4d5e6f",
            "+dev.cig1a2b3c4d5e6f",
        ] {
            assert!(
                !is_valid_release_id(&format!("{version}{invalid}")),
                "{invalid}"
            );
        }
        assert!(!is_valid_release_id("9.9.9+dev.ci.g1a2b3c4d5e6f"));
        assert!(!is_valid_release_id(""));
    }

    #[test]
    fn gameplay_entry_fails_before_any_route_or_credential() -> Result<(), RuntimeError> {
        let client = ClientBootstrap::new()?;
        assert!(matches!(
            client.request_gameplay_entry(),
            Err(GameplayEntryError::NativeProtocolUnavailable)
        ));
        assert!(!client.availability().is_available());
        client.shutdown();
        Ok(())
    }

    #[test]
    fn migrated_input_contract_remains_available() -> Result<(), InputError> {
        assert_eq!(
            ClientBootstrap::validate_action_id("client.menu")?.as_str(),
            "client.menu"
        );
        Ok(())
    }
}

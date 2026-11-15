//! Native login composition: Platform credentials, runtime retry and TCP transport adapters.
#[cfg(test)]
#[path = "features/local-account-services/tests/entry.rs"]
mod account_services_tests;
mod admitted;
mod authorize;
mod challenge;
mod coin_shop_entry;
mod config;
pub(crate) use coin_shop_entry::open as open_coin_shop;
#[cfg(test)]
#[path = "native_entry/tests/live_bulletin.rs"]
mod live_bulletin_tests;
#[cfg(test)]
#[path = "native_entry/tests/queue_progress.rs"]
mod queue_progress_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use crate::GameplayEntryError;
pub use admitted::AdmittedSession;
pub use config::{NativeLoginConfig, NativeLoginSettingError};
use oteryn_client_runtime::connection::{AdmissionAttempt, AdmissionWait};
use oteryn_foundation::CancellationToken;
use oteryn_identity::{IdentityFlow, LoopbackRedirect, PkceMaterial, StateNonce};
use oteryn_platform_client::PlatformClientConfig;
use oteryn_platform_client::gameplay_entry::login_client;
use oteryn_platform_client::native_login::{
    AttemptRef, GatewayLogin, NativeLoginClient, NativeLoginError, NativeLoginGrant, NativeTicket,
    PublicClass, parse_uuid_v7, secure_random,
};
use oteryn_session::{Admission, AdmissionPublicClass, CharacterId};
use oteryn_session_tcp::connection::CONNECT_DEADLINE;
use oteryn_session_tcp::load_root_certificate;
use rustls::pki_types::CertificateDer;
use std::time::Instant;

/// FND-02 `ClientBootstrap.schema_revision` this client sends.
const SCHEMA_REVISION: u32 = 1;

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
    enter_observed(config, cancellation, None).await
}

pub(crate) async fn enter_observed(
    config: &NativeLoginConfig,
    cancellation: CancellationToken,
    progress: Option<tokio::sync::watch::Sender<Option<AdmissionWait>>>,
) -> Result<AdmittedSession, GameplayEntryError> {
    let platform = PlatformClientConfig::new(&config.platform_url)
        .map_err(|_error| GameplayEntryError::InvalidConfiguration)?;
    let gateway = PlatformClientConfig::new(&config.gateway_url)
        .map_err(|_error| GameplayEntryError::InvalidConfiguration)?;
    let root = load_root_certificate(&config.dev_root)
        .map_err(|_error| GameplayEntryError::InvalidConfiguration)?;

    // OAuth code + PKCE, owner character selection, then the consuming ticket.
    let client = login_client(
        &platform,
        &gateway,
        &config.oauth_client_id,
        config.retry_unit,
    )?;
    let (ticket, selected_character, selected_channel, character_name, training_requested) =
        authorize::authorize(&client, config, cancellation.clone()).await?;
    let character_id = parse_uuid_v7(&selected_character)
        .and_then(|bytes| CharacterId::decode(&bytes).ok())
        .ok_or(GameplayEntryError::InvalidConfiguration)?;

    // 4. Gateway login: one attempt_ref for this whole attempt.
    let attempt_ref =
        AttemptRef::generate().map_err(|_error| rejected(PublicClass::TemporarilyUnavailable))?;
    let login = GatewayLogin {
        attempt_ref: &attempt_ref,
        character_id: &selected_character,
        channel_id: selected_channel.as_deref(),
        client_build: &config.client_build,
    };
    let grant = client
        .gateway_login(&ticket, &login, cancellation.clone())
        .await
        .map_err(login_rejected)?;

    let mut attempt = NativeAttempt {
        queue: None,
        progress,
        client,
        ticket,
        login,
        grant,
        root,
        character_id,
        character_name,
        training_requested,
        config,
    };
    oteryn_client_runtime::connection::enter(&mut attempt, config.retry_unit, cancellation).await
}

struct NativeAttempt<'a> {
    queue: Option<oteryn_session::AdmissionQueue>,
    progress: Option<tokio::sync::watch::Sender<Option<AdmissionWait>>>,
    client: NativeLoginClient,
    ticket: NativeTicket,
    login: GatewayLogin<'a>,
    grant: NativeLoginGrant,
    root: CertificateDer<'static>,
    character_id: CharacterId,
    character_name: String,
    training_requested: bool,
    config: &'a NativeLoginConfig,
}

impl AdmissionAttempt for NativeAttempt<'_> {
    type Output = AdmittedSession;
    type Error = GameplayEntryError;

    fn queue(&self) -> Option<oteryn_session::AdmissionQueue> {
        self.queue
    }

    fn waiting(&mut self, wait: AdmissionWait) {
        if let Some(progress) = &self.progress {
            progress.send_replace(Some(wait));
        }
    }

    async fn admit(&mut self) -> Result<Self::Output, Option<u32>> {
        let capabilities = crate::play::feature_0179_hotkey_equip::view::supported_capabilities(
            *self.character_id.as_bytes(),
            self.config.preference_path.as_deref(),
        );
        self.queue = None;
        let session = oteryn_session_tcp::connection::admit_detailed_optimized(
            &self.grant.endpoint.host,
            self.grant.endpoint.port,
            &self.grant.endpoint.tls_server_name,
            Admission {
                appearance_items: None,
                schema_revision: SCHEMA_REVISION,
                character_id: self.character_id,
                admission_material: self.grant.grant_bytes(),
                client_build_id: &self.config.client_build,
                supported_capabilities: &capabilities,
                deadline: CONNECT_DEADLINE,
            },
            &self.root,
            self.config
                .preference_path
                .clone()
                .and_then(|path| crate::play::PreferenceStore::load(path).ok())
                .is_some_and(|store| store.preferences().optimise_connection),
        )
        .await
        .map_err(|refusal| {
            refusal.map(|refusal| {
                self.queue = refusal.queue;
                refusal.code
            })
        })?;
        Ok(AdmittedSession {
            character_id: self.login.character_id.to_owned(),
            preference_character_id: self.character_id,
            character_name: self.character_name.clone(),
            training_requested: self.training_requested,
            session,
            world_id: self.grant.world_id.clone(),
            channel_id: self.grant.channel_id.clone(),
        })
    }

    async fn reconcile(&mut self, cancellation: CancellationToken) -> Result<(), Self::Error> {
        self.grant = self
            .client
            .gateway_login(&self.ticket, &self.login, cancellation)
            .await
            .map_err(login_rejected)?;
        Ok(())
    }

    fn usable_until(&self) -> Instant {
        self.grant.usable_until()
    }

    fn rejected(class: AdmissionPublicClass) -> Self::Error {
        rejected(admission_class(class))
    }
}

#[cfg(test)]
#[path = "features/local-coin-shop/tests/entry.rs"]
mod coin_shop_tests;

#[cfg(test)]
#[path = "features/local-premium/entry_tests.rs"]
mod premium_tests;

#[cfg(test)]
#[path = "native_entry/training_tests.rs"]
mod training_tests;

#[cfg(test)]
#[path = "native_entry/training_batch_tests.rs"]
mod training_batch_tests;

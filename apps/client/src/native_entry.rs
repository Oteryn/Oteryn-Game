//! Native login composition: Platform credentials, runtime retry and TCP transport adapters.
mod admitted;
mod authorize;
mod config;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use crate::GameplayEntryError;
pub use admitted::AdmittedSession;
pub use config::{NativeLoginConfig, NativeLoginSettingError};
use oteryn_client_runtime::connection::AdmissionAttempt;
use oteryn_foundation::CancellationToken;
use oteryn_identity::{IdentityFlow, LoopbackRedirect, PkceMaterial, StateNonce};
use oteryn_platform_client::PlatformClientConfig;
use oteryn_platform_client::gameplay_entry::{check_world, login_client};
use oteryn_platform_client::native_login::{
    AttemptRef, GatewayLogin, NativeLoginClient, NativeLoginError, NativeLoginGrant, NativeTicket,
    PublicClass, parse_uuid_v7, secure_random,
};
use oteryn_session::{Admission, AdmissionPublicClass, CLIENT_SUPPORTED_CAPABILITIES, CharacterId};
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
    let platform = PlatformClientConfig::new(&config.platform_url)
        .map_err(|_error| GameplayEntryError::InvalidConfiguration)?;
    let gateway = PlatformClientConfig::new(&config.gateway_url)
        .map_err(|_error| GameplayEntryError::InvalidConfiguration)?;
    let character_id = parse_uuid_v7(&config.character_id)
        .and_then(|bytes| CharacterId::decode(&bytes).ok())
        .ok_or(GameplayEntryError::InvalidConfiguration)?;

    // 1. Availability before any credential.
    check_world(&platform, &config.world_ref, cancellation.clone()).await?;
    let root = load_root_certificate(&config.dev_root)
        .map_err(|_error| GameplayEntryError::InvalidConfiguration)?;

    // 2-3. OAuth code + PKCE in the system browser, then the native ticket.
    let client = login_client(
        &platform,
        &gateway,
        &config.oauth_client_id,
        config.retry_unit,
    )?;
    let ticket = authorize::authorize(&client, config, cancellation.clone()).await?;

    // 4. Gateway login: one attempt_ref for this whole attempt.
    let attempt_ref =
        AttemptRef::generate().map_err(|_error| rejected(PublicClass::TemporarilyUnavailable))?;
    let login = GatewayLogin {
        attempt_ref: &attempt_ref,
        character_id: &config.character_id,
        client_build: &config.client_build,
    };
    let grant = client
        .gateway_login(&ticket, &login, cancellation.clone())
        .await
        .map_err(login_rejected)?;

    let mut attempt = NativeAttempt {
        client,
        ticket,
        login,
        grant,
        root,
        character_id,
        config,
    };
    oteryn_client_runtime::connection::enter(&mut attempt, config.retry_unit, cancellation).await
}

struct NativeAttempt<'a> {
    client: NativeLoginClient,
    ticket: NativeTicket,
    login: GatewayLogin<'a>,
    grant: NativeLoginGrant,
    root: CertificateDer<'static>,
    character_id: CharacterId,
    config: &'a NativeLoginConfig,
}

impl AdmissionAttempt for NativeAttempt<'_> {
    type Output = AdmittedSession;
    type Error = GameplayEntryError;

    async fn admit(&mut self) -> Result<Self::Output, Option<u32>> {
        let session = oteryn_session_tcp::connection::admit(
            &self.grant.endpoint.host,
            self.grant.endpoint.port,
            &self.grant.endpoint.tls_server_name,
            Admission {
                appearance_items: None,
                schema_revision: SCHEMA_REVISION,
                character_id: self.character_id,
                admission_material: self.grant.grant_bytes(),
                client_build_id: &self.config.client_build,
                supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
                deadline: CONNECT_DEADLINE,
            },
            &self.root,
        )
        .await?;
        Ok(AdmittedSession {
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

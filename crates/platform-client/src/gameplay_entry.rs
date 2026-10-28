//! Public native entry errors and availability-before-credentials facade.

use crate::native_login::{NativeLoginClient, PublicClass, RetryPacing};
use crate::{PlatformClient, PlatformClientConfig};
use oteryn_foundation::CancellationToken;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

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

/// Checks the selected World before the composition root requests any credential.
pub async fn check_world(
    platform: &PlatformClientConfig,
    world_ref: &str,
    cancellation: CancellationToken,
) -> Result<(), GameplayEntryError> {
    let directory = PlatformClient::new(platform.clone())
        .map_err(|_error| GameplayEntryError::InvalidConfiguration)?
        .fetch_directory(cancellation)
        .await
        .map_err(|_error| GameplayEntryError::Rejected(PublicClass::TemporarilyUnavailable))?;
    if !directory
        .worlds
        .iter()
        .any(|world| world.world_ref.as_str() == world_ref)
    {
        return Err(GameplayEntryError::WorldUnavailable);
    }
    Ok(())
}

/// Builds the native admission facade with the configured retry pacing.
pub fn login_client(
    platform: &PlatformClientConfig,
    gateway: &PlatformClientConfig,
    oauth_client_id: &str,
    retry_unit: Duration,
) -> Result<NativeLoginClient, GameplayEntryError> {
    NativeLoginClient::new(platform, gateway, oauth_client_id)
        .map_err(|_error| GameplayEntryError::InvalidConfiguration)
        .map(|client| {
            client.with_pacing(RetryPacing {
                unit: retry_unit,
                ..RetryPacing::CONTRACT
            })
        })
}

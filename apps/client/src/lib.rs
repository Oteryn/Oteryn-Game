//! Production pre-native client composition.
//! Terminal evidence is revalidated after the MPL-2.0 and Linux lint corrections.

use oteryn_client_runtime::{ClientRuntime, RuntimeError};
use oteryn_foundation::ProcessGeneration;
use oteryn_identity::{IdentityError, PkceMaterial};
use oteryn_input_actions::{ActionId, InputError};
use oteryn_input_platform::InputPlatformAdapter;
use oteryn_platform_client::{PlatformClientConfig, PlatformClientError};
use oteryn_platform_contracts::GameplayAvailability;
use oteryn_renderer::SurfaceState;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayEntryError {
    NativeProtocolUnavailable,
}

impl Display for GameplayEntryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("native gameplay protocol is not available")
    }
}

impl std::error::Error for GameplayEntryError {}

pub struct ClientBootstrap {
    runtime: ClientRuntime,
    renderer_state: SurfaceState,
    input_adapter: InputPlatformAdapter,
}

impl ClientBootstrap {
    pub fn new() -> Result<Self, RuntimeError> {
        Ok(Self {
            runtime: ClientRuntime::new()?,
            renderer_state: SurfaceState::new(ProcessGeneration::new(1)),
            input_adapter: InputPlatformAdapter::new(),
        })
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

    pub fn request_gameplay_entry(&self) -> Result<(), GameplayEntryError> {
        Err(GameplayEntryError::NativeProtocolUnavailable)
    }

    pub fn shutdown(self) {
        self.runtime.shutdown(Duration::from_millis(250));
    }
}

#[must_use]
pub const fn pre_native_status() -> &'static str {
    GameplayAvailability::PreNativeProtocol.player_message()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gameplay_entry_fails_before_any_route_or_credential() -> Result<(), RuntimeError> {
        let client = ClientBootstrap::new()?;
        assert_eq!(
            client.request_gameplay_entry(),
            Err(GameplayEntryError::NativeProtocolUnavailable)
        );
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

// Nonshipping #283 hosted benchmark stimulus: client source surface.

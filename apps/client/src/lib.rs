//! Native client process and component composition.
pub mod combat_input;
pub mod cyclopedia;
pub mod input;
pub mod map;
pub mod play;
pub mod scene;
pub mod spell;
#[cfg(windows)]
pub mod win_mutex;
#[deprecated(note = "use map")]
pub mod world;

mod build_info;
mod native_entry;
#[cfg(test)]
mod tests;

use oteryn_client_runtime::{ClientRuntime, RuntimeError};
use oteryn_foundation::ProcessGeneration;
use oteryn_identity::{IdentityError, PkceMaterial};
use oteryn_input_actions::{ActionId, InputError};
use oteryn_input_platform::InputPlatformAdapter;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_platform_client::{PlatformClientConfig, PlatformClientError};
use oteryn_platform_contracts::GameplayAvailability;
use oteryn_renderer::SurfaceState;
use std::time::Duration;

pub use native_entry::{AdmittedSession, NativeLoginConfig, NativeLoginSettingError};

pub use build_info::{CLIENT_BUILD, RELEASE_ID, is_contract_release_id, is_valid_release_id};
pub use oteryn_platform_client::gameplay_entry::GameplayEntryError;

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

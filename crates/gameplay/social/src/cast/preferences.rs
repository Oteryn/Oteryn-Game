//! Platform owns durable account preferences and password verifiers. Game
//! validates the current policy before starting or changing a leased cast.
use super::{Error, Settings, access::CastAccessClient};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct WirePreferences {
    delay_seconds: u16,
    viewer_cap: u8,
    chat_enabled: bool,
    password_required: bool,
    access_revision: u64,
    description: String,
}
pub struct Preferences {
    pub settings: Settings,
    pub description: String,
}
impl Preferences {
    pub fn validate(&self) -> Result<(), Error> {
        self.settings.validate()?;
        if self.description.len() > 256 || self.description.chars().any(char::is_control) {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
pub trait PreferencesStore: Send + Sync {
    fn load(&self, account: [u8; 16]) -> impl Future<Output = Result<Preferences, Error>> + Send;
}
impl PreferencesStore for CastAccessClient {
    async fn load(&self, account: [u8; 16]) -> Result<Preferences, Error> {
        let bytes = self.exchange(serde_json::json!({"operation":"preferences", "owner":super::access::uuid(account)})).await?;
        let wire: WirePreferences =
            serde_json::from_slice(&bytes).map_err(|_| Error::Unavailable)?;
        let value = Preferences {
            settings: Settings {
                delay_seconds: wire.delay_seconds,
                viewer_cap: wire.viewer_cap,
                chat_enabled: wire.chat_enabled,
                password_required: wire.password_required,
                access_revision: wire.access_revision,
            },
            description: wire.description,
        };
        value.validate()?;
        Ok(value)
    }
}
pub async fn authorize(
    store: &impl PreferencesStore,
    account: [u8; 16],
    requested: Settings,
) -> Result<Preferences, Error> {
    let value = store.load(account).await?;
    value.validate()?;
    // Chat enablement is a stored account preference, absent from the control wire.
    if value.settings
        != (Settings {
            chat_enabled: value.settings.chat_enabled,
            ..requested
        })
    {
        return Err(Error::Conflict);
    }
    Ok(value)
}

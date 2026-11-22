//! C01: local connection settings and the native login form.
use crate::{GameplayEntryError, NativeLoginConfig};
use oteryn_platform_client::PlatformClientConfig;
use oteryn_platform_client::native_login::{PublicClass, parse_uuid_v7};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

mod persistence;
mod refusal;
pub use refusal::error_text;
#[cfg(test)]
mod tests;

pub const FIELDS: [(&str, &str); 6] = [
    ("Platform URL", "OTERYN_PLATFORM_URL"),
    ("Gateway URL", "OTERYN_GATEWAY_URL"),
    ("OAuth client ID", "OTERYN_OAUTH_CLIENT_ID"),
    ("World reference (optional)", "OTERYN_WORLD"),
    (
        "Character UUIDv7 (optional preselect)",
        "OTERYN_CHARACTER_ID",
    ),
    ("Trust certificate path (PEM / DER)", "OTERYN_DEV_ROOT"),
];

/// Only public configuration is serializable. OAuth material never enters this type.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginSettings {
    pub platform_url: String,
    pub gateway_url: String,
    pub oauth_client_id: String,
    pub world_ref: String,
    pub character_id: String,
    pub dev_root: String,
}

impl LoginSettings {
    pub fn from_values(values: [String; 6]) -> Self {
        let [
            platform_url,
            gateway_url,
            oauth_client_id,
            world_ref,
            character_id,
            dev_root,
        ] = values.map(|value| value.trim().to_owned());
        Self {
            platform_url,
            gateway_url,
            oauth_client_id,
            world_ref,
            character_id,
            dev_root,
        }
    }

    pub fn values(&self) -> [&str; 6] {
        [
            &self.platform_url,
            &self.gateway_url,
            &self.oauth_client_id,
            &self.world_ref,
            &self.character_id,
            &self.dev_root,
        ]
    }

    /// Explicit environment values seed/override saved settings, including on first run.
    pub fn with_env(self, lookup: impl Fn(&str) -> Option<String>) -> Self {
        Self::from_values(std::array::from_fn(|index| {
            lookup(FIELDS[index].1).unwrap_or_else(|| self.values()[index].to_owned())
        }))
    }

    pub fn validate(&self) -> Result<(), String> {
        // Gameplay trust is validated after the Gateway grant.
        for (index, value) in self.values().iter().take(5).enumerate() {
            if (value.trim().is_empty() && ![3, 4].contains(&index))
                || value.contains('\0')
                || value.len() > 4096
            {
                return Err(lfmt!(
                    "Enter a valid {}.",
                    oteryn_client_ui::localization::tr(FIELDS[index].0)
                ));
            }
        }
        for (label, value) in [
            (tr!("Platform"), &self.platform_url),
            (tr!("Gateway"), &self.gateway_url),
        ] {
            // URLs are public settings, never a place for credentials or tokens.
            if value.contains(['@', '?', '#'])
                || value.chars().any(char::is_whitespace)
                || PlatformClientConfig::new(value).is_err()
            {
                return Err(lfmt!(
                    "{label} URL: use HTTPS (HTTP only on loopback), without credentials, query or fragment.",
                    label = label
                ));
            }
        }
        if !self.character_id.is_empty() && parse_uuid_v7(&self.character_id).is_none() {
            return Err(tr!("Character: enter a canonical lowercase UUIDv7.").into());
        }
        Ok(())
    }

    pub fn config(&self) -> Result<NativeLoginConfig, String> {
        self.validate()?;
        NativeLoginConfig::from_env(|name| {
            FIELDS
                .iter()
                .position(|field| field.1 == name)
                .map(|index| self.values()[index].to_owned())
                .or_else(|| {
                    (name == "OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED")
                        .then(|| std::env::var(name).ok())
                        .flatten()
                })
        })
        .map_err(|error| error.to_string())?
        .ok_or_else(|| tr!("Enter the Platform URL.").into())
    }
}

pub fn settings_path() -> Result<PathBuf, String> {
    crate::play::PreferenceStore::configuration_path("native-login.json")
        .ok_or_else(|| tr!("Cannot locate the user configuration directory.").into())
}

enum LoginAlert {
    Status(&'static str),
    Information(&'static str, String),
}

enum SignInGate {
    UpdateRequired,
    Maintenance(Instant),
}

pub struct LoginForm {
    pub settings: LoginSettings,
    connected: bool,
    pending: bool,
    retry: bool,
    gate: Option<SignInGate>,
    message: String,
    /// Title of the message box that presents `message`, until the player dismisses it.
    alert: Option<LoginAlert>,
}

impl LoginForm {
    pub fn new(settings: LoginSettings) -> Self {
        Self { settings, connected: false, pending: false, retry: false, gate: None, message: tr!("Enter connection settings, then choose Save and sign in. Sign-in opens your browser.").into(), alert: None }
    }
    /// Action shown after a failed attempt starts a fresh browser admission.
    pub fn submit_caption(&self) -> &'static str {
        if self.retry {
            tr!("Retry sign-in")
        } else {
            tr!("Save and sign in")
        }
    }
    pub fn can_submit(&self) -> bool {
        !self.pending && !self.connected && !self.gate_blocks_sign_in()
    }
    fn gate_blocks_sign_in(&self) -> bool {
        match self.gate {
            Some(SignInGate::UpdateRequired) => true,
            Some(SignInGate::Maintenance(retry_at)) => Instant::now() < retry_at,
            None => false,
        }
    }
    pub fn offline(&mut self) {
        self.connected = false;
    }
    pub fn refresh_gate_message(&mut self, now: Instant) {
        if let Some(SignInGate::Maintenance(retry_at)) = self.gate {
            self.message = refusal::maintenance_text(retry_at, now);
            if now >= retry_at {
                self.gate = None;
            }
        }
    }
    #[cfg(bevy_shell)]
    pub(crate) fn show_status(&self) -> bool {
        self.pending || self.gate_blocks_sign_in()
    }
    pub fn pending(&self) -> bool {
        self.pending
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    /// Progress while admission runs stays in the form; anything else opens the message box.
    pub fn feedback(&mut self, message: String) {
        if self.gate_blocks_sign_in() {
            return;
        }
        if !self.pending {
            self.alert = Some(LoginAlert::Status("Login"));
        }
        self.message = message;
    }
    /// A message box with its own short title (About, Survey, Login Error).
    pub fn inform(&mut self, title: &'static str, message: String) {
        self.alert = Some(LoginAlert::Information(title, message));
    }
    /// The open message box: its title and text.
    pub fn alert(&self) -> Option<(&'static str, &str)> {
        match self.alert.as_ref()? {
            LoginAlert::Status(title) => Some((title, &self.message)),
            LoginAlert::Information(title, message) => Some((title, message)),
        }
    }
    pub fn dismiss_alert(&mut self) {
        self.alert = None;
    }
    pub fn submit(&mut self) -> Result<NativeLoginConfig, String> {
        self.refresh_gate_message(Instant::now());
        if self.connected {
            return Err("Already connected. Log out before signing in again.".into());
        }
        if self.pending {
            return Err(tr!(
                "Sign-in is already running. Complete browser sign-in or wait for it to time out."
            )
            .into());
        }
        if self.gate_blocks_sign_in() {
            return Err(self.message.clone());
        }
        self.gate = None;
        let config = self.settings.config().inspect_err(|error| {
            self.message = error.clone();
            self.alert = Some(LoginAlert::Status("Login Error"));
        })?;
        self.pending = true;
        self.alert = None;
        self.message = tr!("Signing in: complete sign-in in your browser (up to five minutes). Please wait for gateway admission.").into();
        Ok(config)
    }
    pub fn finish(&mut self, result: Result<(), GameplayEntryError>) {
        self.pending = false;
        self.connected = result.is_ok();
        self.retry = result.is_err();
        self.alert = result.is_err().then_some(LoginAlert::Status("Login Error"));
        self.gate = match &result {
            Err(
                GameplayEntryError::ClientUpdateRequired { .. }
                | GameplayEntryError::Rejected(PublicClass::ClientUpdateRequired),
            ) => Some(SignInGate::UpdateRequired),
            Err(GameplayEntryError::Maintenance { retry_at }) => {
                Some(SignInGate::Maintenance(*retry_at))
            }
            _ => None,
        };
        self.message = result.map_or_else(error_text, |()| tr!("Connected.").into());
        self.refresh_gate_message(Instant::now());
    }
}

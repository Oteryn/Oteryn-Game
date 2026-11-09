//! C01: local connection settings and the native login form.
use crate::{GameplayEntryError, NativeLoginConfig};
use oteryn_platform_client::PlatformClientConfig;
use oteryn_platform_client::native_login::{PublicClass, parse_uuid_v7};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[cfg(windows)]
mod native;
#[cfg(windows)]
pub use native::LoginWindow;

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
        for (index, value) in self.values().iter().enumerate() {
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
        oteryn_session_tcp::load_root_certificate(Path::new(&self.dev_root)).map_err(|_| {
            tr!("Certificate: choose a readable PEM or DER trust certificate.").to_owned()
        })?;
        Ok(())
    }

    pub fn config(&self) -> Result<NativeLoginConfig, String> {
        self.validate()?;
        NativeLoginConfig::from_env(|name| {
            FIELDS
                .iter()
                .position(|field| field.1 == name)
                .map(|index| self.values()[index].to_owned())
        })
        .map_err(|error| error.to_string())?
        .ok_or_else(|| tr!("Enter the Platform URL.").into())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
                tr!("Saved login settings are invalid. Correct the fields and save again.").into()
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(_) => {
                Err(tr!("Cannot read saved login settings. Check the file permissions.").into())
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let write = || -> Result<(), Box<dyn std::error::Error>> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let temporary = path.with_extension("json.tmp");
            std::fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
            std::fs::rename(temporary, path)?;
            Ok(())
        };
        write().map_err(|_| tr!("Cannot save login settings. Check the configuration directory permissions and try again.").into())
    }
}

pub fn settings_path() -> Result<PathBuf, String> {
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    base.map(|base| base.join("Oteryn").join("native-login.json"))
        .ok_or_else(|| tr!("Cannot locate the user configuration directory.").into())
}

pub struct LoginForm {
    pub settings: LoginSettings,
    pending: bool,
    retry: bool,
    message: String,
}

impl LoginForm {
    pub fn new(settings: LoginSettings) -> Self {
        Self { settings, pending: false, retry: false, message: tr!("Enter connection settings, then choose Save and sign in. Sign-in opens your browser.").into() }
    }
    /// Action shown after a failed attempt starts a fresh browser admission.
    pub fn submit_caption(&self) -> &'static str {
        if self.retry {
            tr!("Retry sign-in")
        } else {
            tr!("Save and sign in")
        }
    }
    pub fn pending(&self) -> bool {
        self.pending
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn feedback(&mut self, message: String) {
        self.message = message;
    }
    pub fn submit(&mut self) -> Result<NativeLoginConfig, String> {
        if self.pending {
            return Err(tr!(
                "Sign-in is already running. Complete browser sign-in or wait for it to time out."
            )
            .into());
        }
        let config = self
            .settings
            .config()
            .inspect_err(|error| self.message = error.clone())?;
        self.pending = true;
        self.message = tr!("Signing in: complete sign-in in your browser (up to five minutes). Please wait for gateway admission.").into();
        Ok(config)
    }
    pub fn finish(&mut self, result: Result<(), GameplayEntryError>) {
        self.pending = false;
        self.retry = result.is_err();
        self.message = result.map_or_else(
            |error| error_text(error).to_owned(),
            |()| tr!("Connected.").into(),
        );
    }
}

pub fn error_text(error: GameplayEntryError) -> &'static str {
    match error {
        GameplayEntryError::NativeProtocolUnavailable => {
            tr!("Enter connection settings and choose Save and sign in.")
        }
        GameplayEntryError::InvalidConfiguration => {
            tr!(
                "Check Platform and Gateway URLs, OAuth client ID, optional character preselect and the trust certificate path, then sign in again."
            )
        }
        GameplayEntryError::WorldUnavailable => {
            tr!(
                "World is absent from the Platform directory. Check the World reference and Platform URL, then sign in again."
            )
        }
        GameplayEntryError::Rejected(
            PublicClass::RetryLogin | PublicClass::AuthenticationRequired,
        ) => {
            tr!(
                "Browser sign-in failed or expired. Check the OAuth client ID and complete a fresh browser sign-in within five minutes."
            )
        }
        GameplayEntryError::Rejected(PublicClass::TemporarilyUnavailable) => {
            tr!(
                "Connection unavailable. Check your network and the Platform/Gateway URLs; if correct, try again later."
            )
        }
        GameplayEntryError::Rejected(PublicClass::SessionUnavailable) => {
            tr!(
                "Game connection ended or could not be established. Check your network and trust certificate; sign in again. Contact the operator if this repeats."
            )
        }
        GameplayEntryError::Rejected(PublicClass::ClientUpdateRequired) => {
            tr!("Install the current Oteryn client, then sign in again.")
        }
        GameplayEntryError::Rejected(PublicClass::CharacterAlreadyActive) => {
            tr!(
                "Character is already active. Log out its other session or wait for it to end, then sign in again."
            )
        }
    }
}

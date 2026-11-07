//! Validated, atomic replacement of the native client's local preferences.
use super::ReferencePreferences;
use std::{
    io,
    path::{Path, PathBuf},
};

pub struct PreferenceStore {
    path: PathBuf,
    preferences: ReferencePreferences,
}

impl PreferenceStore {
    pub fn defaults(path: PathBuf) -> Self {
        Self {
            path,
            preferences: ReferencePreferences::default(),
        }
    }

    pub fn load(path: PathBuf) -> io::Result<Self> {
        let preferences = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                ReferencePreferences::default()
            }
            Err(error) => return Err(error),
        };
        preferences.validate().map_err(io::Error::other)?;
        Ok(Self { path, preferences })
    }

    pub fn preferences(&self) -> &ReferencePreferences {
        &self.preferences
    }

    pub fn apply(&mut self, preferences: ReferencePreferences) -> io::Result<()> {
        preferences.validate().map_err(io::Error::other)?;
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(&preferences).map_err(io::Error::other)?;
        let temporary = self.path.with_extension("json.tmp");
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
        }
        if let Err(error) = std::fs::rename(&temporary, &self.path) {
            let _ = std::fs::remove_file(&temporary);
            return Err(error);
        }
        self.preferences = preferences;
        Ok(())
    }

    pub fn path() -> Option<PathBuf> {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|root| root.join(Path::new("Oteryn/client-preferences.json")))
    }
}

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
    /// Read a portable settings file with the same validation as local storage.
    pub fn read_transfer(path: &Path) -> io::Result<ReferencePreferences> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take((1 << 20) + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 1 << 20 {
            return Err(io::Error::other(tr!("Settings file exceeds 1 MiB")));
        }
        let preferences: ReferencePreferences =
            serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        preferences.validate().map_err(io::Error::other)?;
        Ok(preferences)
    }

    /// Exports never replace existing files, including the active preferences.
    pub fn export_to(preferences: &ReferencePreferences, path: &Path) -> io::Result<()> {
        use std::io::Write;
        preferences.validate().map_err(io::Error::other)?;
        let bytes = serde_json::to_vec_pretty(preferences).map_err(io::Error::other)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
            let _ = std::fs::remove_file(path);
            return Err(error);
        }
        Ok(())
    }

    pub fn backup_path(&self) -> PathBuf {
        self.path.with_extension("backup.json")
    }

    pub fn replace_with_backup(&mut self, preferences: ReferencePreferences) -> io::Result<()> {
        preferences.validate().map_err(io::Error::other)?;
        let mut backup = Self::defaults(self.backup_path());
        backup.apply(self.preferences.clone())?;
        self.apply(preferences)
    }

    pub fn defaults(path: PathBuf) -> Self {
        Self {
            path,
            preferences: ReferencePreferences::default(),
        }
    }

    pub fn load(path: PathBuf) -> io::Result<Self> {
        let preferences = match Self::read_transfer(&path) {
            Ok(preferences) => preferences,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                ReferencePreferences::default()
            }
            Err(error) => return Err(error),
        };
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

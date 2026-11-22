use super::LoginSettings;
use std::path::Path;

impl LoginSettings {
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
        let lock = crate::atomic_file::lock(path).map_err(|_| Self::save_error())?;
        self.save_with(path, lock, |_| self.clone()).map(|_| ())
    }

    #[cfg(bevy_shell)]
    pub(crate) fn save_edits(
        &self,
        path: &Path,
        before: &Self,
        lock: std::fs::File,
    ) -> Result<Self, String> {
        self.save_with(path, lock, |active| {
            Self::from_values(std::array::from_fn(|index| {
                let edited = self.values()[index];
                if edited == before.values()[index] {
                    active.values()[index].to_owned()
                } else {
                    edited.to_owned()
                }
            }))
        })
    }

    pub(crate) fn save_error() -> String {
        tr!("Cannot save login settings. Check the configuration directory permissions and try again.").to_owned()
    }

    fn save_with(
        &self,
        path: &Path,
        _lock: std::fs::File,
        edit: impl FnOnce(Self) -> Self,
    ) -> Result<Self, String> {
        self.validate()?;
        let error = Self::save_error;
        let settings = match std::fs::read(path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(active) => edit(active),
                Err(_) => {
                    crate::atomic_file::preserve_malformed(path).map_err(|_| error())?;
                    self.clone()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => self.clone(),
            Err(_) => return Err(error()),
        };
        settings.validate()?;
        let bytes = serde_json::to_vec_pretty(&settings).map_err(|_| error())?;
        crate::atomic_file::replace(path, &bytes, true).map_err(|_| error())?;
        Ok(settings)
    }
}

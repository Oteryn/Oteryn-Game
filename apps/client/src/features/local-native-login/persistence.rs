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
        self.save_fields(path, [true; 6])
    }

    pub(crate) fn save_fields(&self, path: &Path, edited: [bool; 6]) -> Result<(), String> {
        let _lock = crate::atomic_file::lock(path).map_err(|_| Self::save_error())?;
        self.validate()?;
        let error = Self::save_error;
        let settings = match std::fs::read(path) {
            Ok(bytes) => match serde_json::from_slice::<Self>(&bytes) {
                Ok(active) => {
                    let submitted = self.values();
                    let disk = active.values();
                    Self::from_stored_values(std::array::from_fn(|i| {
                        if edited[i] { submitted[i] } else { disk[i] }.to_owned()
                    }))
                }
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
        crate::atomic_file::replace(path, &bytes, true).map_err(|_| Self::save_error())
    }

    pub(crate) fn save_error() -> String {
        tr!("Cannot save login settings. Check the configuration directory permissions and try again.").to_owned()
    }
}

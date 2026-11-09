//! Screenshot destination and scope; frame pixels always come from this client's renderer.
use crate::screenshots::{CaptureController, Frame, Rect};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScreenshotPreferences {
    /// Empty uses the client's normal screenshots directory.
    pub directory: String,
    pub game_only: bool,
}
impl ScreenshotPreferences {
    pub fn valid(&self) -> bool {
        self.directory.len() <= 1024
            && !self.directory.chars().any(char::is_control)
            && (self.directory.is_empty() || Path::new(&self.directory).is_absolute())
    }
    pub fn save_frame(
        &self,
        capture: &mut CaptureController,
        fallback: &Path,
        frame: Frame<'_>,
    ) -> io::Result<PathBuf> {
        if !self.valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Choose an absolute screenshot folder",
            ));
        }
        let directory = if self.directory.is_empty() {
            fallback
        } else {
            Path::new(&self.directory)
        };
        std::fs::create_dir_all(directory)?;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let path = directory.join(format!(
            "oteryn-{stamp}-{}.bmp",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let crop = if self.game_only {
            frame.game
        } else {
            Rect {
                x: 0,
                y: 0,
                width: frame.width,
                height: frame.height,
            }
        };
        capture.save(&path, frame.width, frame.height, frame.bgra, crop)?;
        Ok(path)
    }
}

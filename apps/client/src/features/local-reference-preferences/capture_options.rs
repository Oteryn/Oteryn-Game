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
    pub folder: String,
    pub backlog_frames: u8,
    pub on_skill: bool,
    pub game_only: bool,
    pub backlog_seconds: u8,
    pub on_level: bool,
    pub on_death: bool,
    pub on_achievement: bool,
}
impl ScreenshotPreferences {
    pub fn reconciled(mut self, previous: &Self) -> Self {
        self.harmonize(previous);
        self
    }
    pub fn effective(&self) -> Self {
        let mut result = self.clone();
        if !result.directory.is_empty() {
            result.folder.clone_from(&result.directory);
        } else {
            result.directory.clone_from(&result.folder);
        }
        if result.backlog_frames == 0 {
            result.backlog_frames = result.backlog_seconds;
        }
        result
    }
    pub(super) fn harmonize(&mut self, previous: &Self) {
        if self.directory != previous.directory {
            self.folder.clone_from(&self.directory);
        } else if self.folder != previous.folder {
            self.directory.clone_from(&self.folder);
        }
        if self.backlog_seconds != previous.backlog_seconds {
            self.backlog_frames = 0;
        } else if self.backlog_frames != previous.backlog_frames {
            self.backlog_seconds = 0;
        }
    }
    pub fn valid(&self) -> bool {
        let effective = self.effective();
        effective.backlog_seconds <= 10
            && effective.backlog_frames <= 10
            && effective.directory.len() <= 1024
            && !effective.directory.chars().any(char::is_control)
            && effective.folder.len() <= 1024
            && !effective.folder.chars().any(char::is_control)
            && (effective.directory.is_empty() || Path::new(&effective.directory).is_absolute())
    }
    pub fn apply_policy(&self, capture: &mut CaptureController) {
        capture.policy.level = self.on_level;
        capture.policy.death = self.on_death;
        capture.policy.achievement = self.on_achievement;
    }
    /// Uses the same configured or platform screenshot destination as F12.
    pub fn save_frame(
        &self,
        capture: &mut CaptureController,
        _fallback: &Path,
        frame: Frame<'_>,
    ) -> io::Result<PathBuf> {
        if !self.valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Choose an absolute screenshot folder",
            ));
        }
        let effective = self.effective();
        let directory = super::screenshot::screenshot_directory(Path::new(&effective.directory))?;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
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
        super::screenshot::retry_capture_name(|| {
            let path = directory.join(format!(
                "oteryn-{stamp}-{}.bmp",
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            capture.save(&path, frame.width, frame.height, frame.bgra, crop)?;
            Ok(path)
        })
    }
}

#[cfg(test)]
mod audit_tests {
    use super::*;
    #[test]
    fn screenshot_alias_validation_rejects_relative_effective_folders() {
        let preferences = ScreenshotPreferences {
            folder: "relative-captures".into(),
            ..Default::default()
        };
        assert!(!preferences.valid());
        let absolute = ScreenshotPreferences {
            directory: std::env::temp_dir().to_string_lossy().into_owned(),
            ..preferences
        };
        assert!(absolute.valid());
    }
    #[test]
    fn screenshot_alias_frame_writer_rejects_relative_folders()
    -> Result<(), Box<dyn std::error::Error>> {
        let cwd = std::env::current_dir()?;
        let fixture = crate::play::screenshot::TestDir::new_in(&cwd)?;
        let folder = fixture
            .join("captures")
            .strip_prefix(&cwd)?
            .to_string_lossy()
            .into_owned();
        let preferences = ScreenshotPreferences {
            folder: folder.clone(),
            ..Default::default()
        };
        let frame = Frame {
            width: 1,
            height: 1,
            bgra: &[0, 0, 0, 255],
            game: Rect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
        };
        let result =
            preferences.save_frame(&mut CaptureController::default(), fixture.path(), frame);
        assert!(result.is_err());
        assert!(!Path::new(&folder).exists());
        Ok(())
    }
}

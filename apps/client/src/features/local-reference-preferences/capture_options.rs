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
        self.backlog_seconds <= 10
            && self.backlog_frames <= 10
            && self.directory.len() <= 1024
            && !self.directory.chars().any(char::is_control)
            && self.folder.len() <= 1024
            && !self.folder.chars().any(char::is_control)
            && (self.directory.is_empty() || Path::new(&self.directory).is_absolute())
    }
    pub fn apply_policy(&self, capture: &mut CaptureController) {
        capture.policy.level = self.on_level;
        capture.policy.death = self.on_death;
        capture.policy.achievement = self.on_achievement;
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
        let effective = self.effective();
        let directory = if effective.directory.is_empty() {
            fallback
        } else {
            Path::new(&effective.directory)
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

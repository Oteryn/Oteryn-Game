//! Screenshot requests share the settings caller on every platform; native capture supplies pixels.
use super::SettingsState;
#[path = "screenshot_backlog.rs"]
mod backlog;
pub(super) use backlog::ScreenshotRuntime;
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{self, BufWriter, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScreenshotPreferences {
    pub folder: String,
    pub game_only: bool,
    pub backlog_frames: u8,
    pub on_death: bool,
    pub on_level: bool,
    pub on_skill: bool,
}

impl Default for ScreenshotPreferences {
    fn default() -> Self {
        Self {
            folder: std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join("Oteryn/screenshots")
                .to_string_lossy()
                .into_owned(),
            game_only: true,
            backlog_frames: 0,
            on_death: false,
            on_level: false,
            on_skill: false,
        }
    }
}

pub struct Pixels {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}
impl Pixels {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> io::Result<Self> {
        if width == 0
            || height == 0
            || width > 4096
            || height > 4096
            || rgba.len() != width as usize * height as usize * 4
        {
            return Err(io::Error::other("Invalid screenshot dimensions"));
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    pub fn save(&self, folder: &str) -> io::Result<PathBuf> {
        if folder.trim().is_empty() {
            return Err(io::Error::other("Choose a screenshot folder"));
        }
        std::fs::create_dir_all(folder)?;
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let path = PathBuf::from(folder).join(format!(
            "oteryn-{stamp}-{}-{}.bmp",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = BufWriter::new(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?,
        );
        let result = self
            .write_bmp(&mut file)
            .and_then(|()| file.flush())
            .and_then(|()| file.get_ref().sync_all());
        if let Err(error) = result {
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
        Ok(path)
    }

    fn write_bmp(&self, file: &mut impl Write) -> io::Result<()> {
        let stride = (self.width * 3).div_ceil(4) * 4;
        let size = stride * self.height;
        file.write_all(b"BM")?;
        file.write_all(&(54 + size).to_le_bytes())?;
        file.write_all(&[0; 4])?;
        file.write_all(&54_u32.to_le_bytes())?;
        file.write_all(&40_u32.to_le_bytes())?;
        file.write_all(&self.width.to_le_bytes())?;
        file.write_all(&self.height.to_le_bytes())?;
        file.write_all(&1_u16.to_le_bytes())?;
        file.write_all(&24_u16.to_le_bytes())?;
        file.write_all(&0_u32.to_le_bytes())?;
        file.write_all(&size.to_le_bytes())?;
        file.write_all(&[0; 16])?;
        for row in self.rgba.chunks_exact(self.width as usize * 4).rev() {
            for pixel in row.chunks_exact(4) {
                file.write_all(&[pixel[2], pixel[1], pixel[0]])?;
            }
            file.write_all(&[0; 3][..(stride - self.width * 3) as usize])?;
        }
        Ok(())
    }
}

pub trait CaptureSource {
    fn capture(&mut self, game_only: bool) -> io::Result<Pixels>;
}

impl SettingsState {
    pub fn request_screenshot(&mut self) {
        self.screenshot_requested = true;
    }

    pub fn take_screenshot(
        &mut self,
        source: &mut impl CaptureSource,
    ) -> io::Result<Option<PathBuf>> {
        if !std::mem::take(&mut self.screenshot_requested) {
            return Ok(None);
        }
        source
            .capture(self.preferences.screenshots.game_only)?
            .save(&self.preferences.screenshots.folder)
            .map(Some)
    }
}

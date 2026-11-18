//! Screenshot requests share the settings caller on every platform; native capture supplies pixels.
use super::SettingsState;
#[path = "screenshot_backlog.rs"]
mod backlog;
pub(super) use backlog::ScreenshotRuntime;
use std::{
    fs::OpenOptions,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub use super::ScreenshotPreferences;

pub struct Pixels {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}
impl Pixels {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> io::Result<Self> {
        if width == 0
            || height == 0
            || width > 16384
            || height > 16384
            || u64::from(width) * u64::from(height) > 32 * 1024 * 1024
            || rgba.len() != width as usize * height as usize * 4
        {
            return Err(io::Error::other(tr!("Invalid screenshot dimensions")));
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    pub fn save(&self, folder: &str) -> io::Result<PathBuf> {
        self.save_to(&screenshot_directory(folder)?)
    }

    fn save_to(&self, folder: &Path) -> io::Result<PathBuf> {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let path = folder.join(format!(
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
            for pixel in row.as_chunks::<4>().0 {
                file.write_all(&[pixel[2], pixel[1], pixel[0]])?;
            }
            file.write_all(&[0; 3][..(stride - self.width * 3) as usize])?;
        }
        Ok(())
    }
}

fn screenshot_directory(folder: &str) -> io::Result<PathBuf> {
    let folder = if folder.is_empty() {
        let local = std::env::var_os("LOCALAPPDATA");
        #[cfg(unix)]
        if local.is_none() {
            return private_capture_directory();
        }
        local.map_or_else(std::env::temp_dir, PathBuf::from).join("Oteryn/screenshots")
    } else if folder.trim().is_empty() {
        return Err(io::Error::other(tr!("Choose a screenshot folder")));
    } else {
        PathBuf::from(folder)
    };

    if !folder.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Choose an absolute screenshot folder",
        ));
    }
    std::fs::create_dir_all(&folder)?;
    Ok(folder)
}

/// Create the fallback with private permissions from its first moment on a shared temp volume.
#[cfg(unix)]
fn private_capture_directory() -> io::Result<PathBuf> {
    use std::{os::unix::fs::DirBuilderExt, time::{SystemTime, UNIX_EPOCH}};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let folder = std::env::temp_dir().join(format!(
        "oteryn-screenshots-{}-{stamp}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed),
    ));
    std::fs::DirBuilder::new().mode(0o700).create(&folder)?;
    Ok(folder)
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
        let preferences = self.preferences.screenshots.effective();
        source
            .capture(preferences.game_only)?
            .save(&preferences.directory)
            .map(Some)
    }
}

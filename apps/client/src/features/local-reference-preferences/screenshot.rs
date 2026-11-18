//! Screenshot requests share the settings caller on every platform; native capture supplies pixels.
use super::{PreferenceStore, SettingsState};
#[path = "screenshot_backlog.rs"]
mod backlog;
pub(super) use backlog::ScreenshotRuntime;
#[cfg(bevy_shell)]
pub(crate) use backlog::{CaptureAction, ScreenshotBatch};
use std::{
    fs::{DirBuilder, File},
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
        self.save_to(&screenshot_directory(Path::new(folder))?)
    }

    fn save_to(&self, folder: &Path) -> io::Result<PathBuf> {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let (path, file) = retry_capture_name(|| {
            let path = folder.join(format!(
                "oteryn-{stamp}-{}-{}.bmp",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            let file = create_capture_file(&path)?;
            Ok((path, file))
        })?;
        let mut file = BufWriter::new(file);
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

pub(super) fn screenshot_directory(folder: &Path) -> io::Result<PathBuf> {
    if !folder.as_os_str().is_empty() {
        if !folder.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Choose an absolute screenshot folder",
            ));
        }
        return Ok(folder.to_path_buf());
    }
    if let Some(path) = PreferenceStore::path().filter(|path| path.is_absolute()) {
        return Ok(path.with_file_name("screenshots"));
    }
    #[cfg(unix)]
    {
        private_capture_directory()
    }
    #[cfg(not(unix))]
    {
        Ok(local_app_data()
            .unwrap_or_else(std::env::temp_dir)
            .join("Oteryn/screenshots"))
    }
}

#[cfg(not(unix))]
fn local_app_data() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

pub(crate) fn retry_capture_name<T>(mut attempt: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    const MAX_ATTEMPTS: usize = 32;
    for _ in 1..MAX_ATTEMPTS {
        match attempt() {
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            result => return result,
        }
    }
    attempt()
}

/// Open a new screenshot with private permissions before writing any pixels.
pub(crate) fn create_capture_file(path: &Path) -> io::Result<File> {
    let mut directory = DirBuilder::new();
    directory.recursive(true);
    let mut options = File::options();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
        directory.mode(0o700);
        options.mode(0o600);
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    if let Some(parent) = parent {
        directory.create(parent).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                io::Error::new(io::ErrorKind::NotADirectory, error)
            } else {
                error
            }
        })?;
    }
    options.open(path)
}

/// Keep the fallback private and reuse it for all captures in this process.
#[cfg(unix)]
fn private_capture_directory() -> io::Result<PathBuf> {
    use std::{
        os::unix::fs::DirBuilderExt,
        sync::Mutex,
        time::{SystemTime, UNIX_EPOCH},
    };
    static DIRECTORY: Mutex<Option<PathBuf>> = Mutex::new(None);
    let mut directory = DIRECTORY
        .lock()
        .map_err(|_| io::Error::other("Screenshot directory lock poisoned"))?;
    if let Some(path) = directory.as_ref() {
        return Ok(path.clone());
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let folder =
        std::env::temp_dir().join(format!("oteryn-screenshots-{}-{stamp}", std::process::id()));
    std::fs::DirBuilder::new().mode(0o700).create(&folder)?;
    *directory = Some(folder.clone());
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

#[cfg(test)]
#[path = "screenshot_fixture.rs"]
mod fixture;
#[cfg(test)]
pub(crate) use fixture::{TestDir, assert_one_test_passed, with_relative_capture_path};
#[cfg(all(test, bevy_shell, unix))]
pub(crate) use fixture::one_test_passed;

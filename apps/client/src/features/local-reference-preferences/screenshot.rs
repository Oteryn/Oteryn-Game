//! Screenshot requests share the settings caller on every platform; native capture supplies pixels.
use super::{PreferenceStore, SettingsState};
#[path = "screenshot_backlog.rs"]
mod backlog;
pub(super) use backlog::ScreenshotRuntime;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
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
        let path = folder.join(format!(
            "oteryn-{stamp}-{}-{}.bmp",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = BufWriter::new(create_capture_file(&path)?);
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

fn local_app_data() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
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
        directory.create(parent)?;
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    {
        let folder = parent.unwrap_or_else(|| Path::new("."));
        if let Ok(entry) = folder.metadata()
            && entry.mode() & 0o011 != 0
            && let Ok(created) = file.metadata()
        {
            restrict_legacy_bmps(folder, created.uid());
        }
    }
    Ok(file)
}

#[cfg(unix)]
fn restrict_legacy_bmps(folder: &Path, owner: u32) {
    use rustix::fs::{Mode, OFlags, open};
    use std::{io::Read, os::unix::fs::PermissionsExt};

    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(stem) = name.to_str()
            .and_then(|name| name.strip_prefix("oteryn-"))
            .and_then(|name| name.strip_suffix(".bmp"))
        else {
            continue;
        };
        // Both retained BMP writers used this prefix and two or three numeric fields.
        let mut fields = stem.split('-');
        if !(2..=3).contains(&fields.clone().count())
            || !fields.all(|field| {
                !field.is_empty() && field.bytes().all(|byte| byte.is_ascii_digit())
            })
        {
            continue;
        }
        let Ok(before) = entry.metadata() else {
            continue;
        };
        if !before.is_file() || before.uid() != owner || before.nlink() != 1 || before.mode() & 0o077 == 0 {
            continue;
        }
        // Check the exact open inode before changing permissions in a shared directory.
        let Ok(fd) = open(entry.path(), OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC, Mode::empty()) else {
            continue;
        };
        let mut file = File::from(fd);
        let Ok(opened) = file.metadata() else {
            continue;
        };
        let mut signature = [0; 2];
        if opened.is_file()
            && opened.uid() == owner
            && opened.nlink() == 1
            && (opened.dev(), opened.ino()) == (before.dev(), before.ino())
            && file.read_exact(&mut signature).is_ok()
            && &signature == b"BM"
        {
            let _ = file.set_permissions(std::fs::Permissions::from_mode(opened.mode() & !0o077));
        }
    }
}

/// Keep the fallback private and reuse it for all captures in this process.
#[cfg(unix)]
fn private_capture_directory() -> io::Result<PathBuf> {
    use std::{
        os::unix::fs::{DirBuilderExt, PermissionsExt},
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
    // Legacy cleanup is best effort. The new directory already protects fresh captures.
    if let Some(legacy) = local_app_data().map(|root| root.join("Oteryn/screenshots"))
        && let Ok(entry) = legacy.symlink_metadata()
        && entry.is_dir()
        && folder.metadata().is_ok_and(|created| entry.uid() == created.uid())
        && let Ok(opened) = File::open(legacy.join("."))
        && let Ok(actual) = opened.metadata()
        && (actual.dev(), actual.ino()) == (entry.dev(), entry.ino())
    {
        let mode = actual.mode();
        if mode & 0o1022 != 0
            || (mode & 0o077 != 0
                && opened.set_permissions(std::fs::Permissions::from_mode(mode & !0o077)).is_err())
        {
            restrict_legacy_bmps(&legacy, entry.uid());
        }
    }
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

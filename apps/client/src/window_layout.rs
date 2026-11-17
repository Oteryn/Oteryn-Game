//! Local miniwindow geometry, independent of server state and general settings writes.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io, path::Path};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowLayout(BTreeMap<String, [i32; 4]>);

impl WindowLayout {
    pub fn load(path: &Path) -> io::Result<Self> {
        use io::Read;
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take((1 << 20) + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 1 << 20 {
            return Err(io::Error::other(tr!("Window layout exceeds 1 MiB")));
        }
        let layout: Self = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        let mut checked = Self::default();
        for (id, bounds) in layout.0 {
            checked.record(&id, bounds)?;
        }
        Ok(checked)
    }

    #[must_use]
    pub fn bounds(&self, id: &str) -> Option<[i32; 4]> {
        self.0.get(id).copied()
    }

    pub fn record(&mut self, id: &str, [x, y, width, height]: [i32; 4]) -> io::Result<()> {
        if id.is_empty()
            || id.len() > 128
            || id.contains('\0')
            || !(-32768..=32768).contains(&x)
            || !(-32768..=32768).contains(&y)
            || !(200..=4096).contains(&width)
            || !(100..=4096).contains(&height)
            || (self.0.len() >= 128 && !self.0.contains_key(id))
        {
            return Err(io::Error::other(tr!("Invalid window geometry")));
        }
        self.0.insert(id.to_owned(), [x, y, width, height]);
        Ok(())
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("layout.tmp");
        std::fs::write(
            &temporary,
            serde_json::to_vec(self).map_err(io::Error::other)?,
        )?;
        std::fs::rename(temporary, path)
    }
}

#[cfg(windows)]
fn path() -> Option<std::path::PathBuf> {
    crate::play::PreferenceStore::path()
        .map(|path| path.with_file_name("client-window-layout.json"))
}

#[cfg(windows)]
pub(crate) fn restore(id: &str, fallback: [i32; 4]) -> [i32; 4] {
    path()
        .and_then(|path| WindowLayout::load(&path).ok())
        .and_then(|layout| layout.bounds(id))
        .unwrap_or(fallback)
}

#[cfg(windows)]
pub(crate) fn remember(id: &str, bounds: [i32; 4]) {
    // Each native window has its own thread. Serialize read/merge/write to preserve siblings.
    static WRITER: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = WRITER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(path) = path() {
        let result = WindowLayout::load(&path).and_then(|mut layout| {
            layout.record(id, bounds)?;
            layout.save(&path)
        });
        if let Err(error) = result {
            leprintln!("Could not save window layout: {error}", error = error);
        }
    }
}

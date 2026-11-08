//! Portable exploration is bound to the same world and content generation as the active cache.
use super::*;
use std::{
    io::{BufWriter, Write},
    path::Path,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Portable {
    world: String,
    binding: Binding,
    saved: Saved,
}
impl Saved {
    pub(super) fn write_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("tmp");
        let result = (|| {
            let mut file = BufWriter::new(std::fs::File::create(&temporary)?);
            serde_json::to_writer(&mut file, self).map_err(io::Error::other)?;
            file.flush()?;
            file.get_ref().sync_all()?;
            drop(file);
            std::fs::rename(&temporary, path)
        })();
        if result.is_err() && temporary.is_file() {
            let _ = std::fs::remove_file(temporary);
        }
        result
    }
}

impl Minimap {
    pub fn export_to(&self, path: &Path) -> io::Result<()> {
        let world = self
            .world
            .clone()
            .ok_or_else(|| io::Error::other("Join a world before exporting its minimap"))?;
        let binding = self
            .binding
            .ok_or_else(|| io::Error::other("Wait for the server map before exporting"))?;
        let saved = Saved {
            version: 1,
            cells: self
                .cells
                .iter()
                .map(|(&(floor, x, y), &color)| [i32::from(floor), x, y, i32::from(color)])
                .collect(),
            markers: self.markers.clone(),
        };
        saved.validate()?;
        let mut file = BufWriter::new(
            std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)?,
        );
        let result = serde_json::to_writer(
            &mut file,
            &Portable {
                world,
                binding,
                saved,
            },
        )
        .map_err(io::Error::other)
        .and_then(|()| file.flush())
        .and_then(|()| file.get_ref().sync_all());
        if result.is_err() {
            drop(file);
            let _ = std::fs::remove_file(path);
        }
        result
    }

    pub fn import_from(&mut self, path: &Path) -> io::Result<()> {
        let active = self.path().ok_or_else(|| {
            io::Error::other("Join a world with minimap storage before importing")
        })?;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(io::Error::other("Minimap file is too large"));
        }
        let portable: Portable = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        if self.world.as_ref() != Some(&portable.world) || self.binding != Some(portable.binding) {
            return Err(io::Error::other(
                "This minimap belongs to a different world or content generation",
            ));
        }
        portable.saved.validate()?;
        portable.saved.write_to(&active)?;
        self.cells = portable
            .saved
            .cells
            .into_iter()
            .filter(|cell| cell[3] == 55)
            .map(|[f, x, y, c]| ((f as i16, x, y), c as u8))
            .collect();
        self.markers = portable.saved.markers;
        self.dirty = false;
        self.saved_at = Instant::now();
        Ok(())
    }
}

impl Saved {
    pub(super) fn validate(&self) -> io::Result<()> {
        if self.version != 1
            || self.cells.len() > MAX_CELLS
            || self.markers.len() > MAX_MARKERS
            || self.cells.iter().any(|[f, x, y, c]| {
                !(-15..=0).contains(f)
                    || !(0..=65535).contains(x)
                    || !(0..=65535).contains(y)
                    || ![55, 86].contains(c)
            })
            || self.markers.iter().any(|m| {
                !(0..=65535).contains(&m.x)
                    || !(0..=65535).contains(&m.y)
                    || !(-15..=0).contains(&m.floor)
                    || m.color > 3
                    || m.label.trim().is_empty()
                    || m.label.len() > 128
                    || m.label.chars().any(char::is_control)
            })
        {
            return Err(io::Error::other("Invalid minimap file"));
        }
        Ok(())
    }
}

//! Persistent local layout for native miniwindows, independent of server state.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io, path::Path};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dock {
    Left,
    Right,
    Floating,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pane {
    pub dock: Dock,
    pub bounds: [i32; 4],
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Layout {
    order: Vec<String>,
    panes: BTreeMap<String, Pane>,
}
impl Layout {
    pub fn open(&mut self, id: &str, bounds: [i32; 4]) {
        if !self.panes.contains_key(id) && self.panes.len() < 128 {
            self.order.push(id.into());
            self.panes.insert(
                id.into(),
                Pane {
                    dock: Dock::Floating,
                    bounds,
                },
            );
        }
    }
    pub fn dock(&mut self, id: &str, dock: Dock) {
        if let Some(pane) = self.panes.get_mut(id) {
            pane.dock = dock;
        }
    }
    pub fn dragged(&mut self, id: &str, bounds: [i32; 4], root: [i32; 2]) {
        let dock = if bounds[0] <= 24 {
            Dock::Left
        } else if bounds[0] + bounds[2] >= root[0] - 24 {
            Dock::Right
        } else {
            Dock::Floating
        };
        if let Some(pane) = self.panes.get_mut(id) {
            pane.bounds = [
                bounds[0].clamp(0, 4096),
                bounds[1].clamp(0, 4096),
                bounds[2].clamp(200, 4096),
                bounds[3].clamp(100, 4096),
            ];
            pane.dock = dock;
        }
        if dock != Dock::Floating {
            self.order.retain(|entry| entry != id);
            let index = self
                .order
                .iter()
                .position(|entry| {
                    self.panes
                        .get(entry)
                        .is_some_and(|pane| pane.dock == dock && pane.bounds[1] > bounds[1])
                })
                .unwrap_or(self.order.len());
            self.order.insert(index, id.into());
        }
    }
    pub fn rect(&self, id: &str, root: [i32; 2], visible: &[String]) -> Option<[i32; 4]> {
        let pane = self.panes.get(id)?;
        let [rw, rh] = root.map(|size| size.max(1));
        let width = pane.bounds[2].min(rw).max(1);
        let height = pane.bounds[3].min(rh).max(1);
        let y = self
            .order
            .iter()
            .take_while(|entry| entry.as_str() != id)
            .filter(|entry| visible.contains(entry))
            .filter_map(|entry| self.panes.get(entry))
            .filter(|other| other.dock == pane.dock)
            .fold(0_i32, |sum, other| {
                sum.saturating_add(other.bounds[3]).saturating_add(4)
            });
        let (x, y) = match pane.dock {
            Dock::Left => (0, y),
            Dock::Right => (rw - width, y),
            Dock::Floating => (pane.bounds[0], pane.bounds[1]),
        };
        Some([
            x.clamp(0, rw - width),
            y.clamp(0, rh - height),
            width,
            height,
        ])
    }
    pub fn load(path: &Path) -> io::Result<Self> {
        use io::Read;
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e),
        };
        let mut bytes = Vec::new();
        file.take(1_048_577).read_to_end(&mut bytes)?;
        if bytes.len() > 1_048_576 {
            return Err(io::Error::other("Layout too large"));
        }
        let value: Self = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        if value.panes.len() > 128
            || value.order.len() != value.panes.len()
            || value
                .order
                .iter()
                .enumerate()
                .any(|(i, id)| value.order[..i].contains(id) || !value.panes.contains_key(id))
            || value.panes.iter().any(|(id, pane)| {
                id.is_empty()
                    || id.len() > 256
                    || pane.bounds[0..2].iter().any(|v| !(0..=4096).contains(v))
                    || !(200..=4096).contains(&pane.bounds[2])
                    || !(100..=4096).contains(&pane.bounds[3])
            })
        {
            return Err(io::Error::other("Invalid layout"));
        }
        Ok(value)
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("tmp");
        std::fs::write(&temp, serde_json::to_vec(self).map_err(io::Error::other)?)?;
        std::fs::rename(temp, path)
    }
}
#[cfg(windows)]
#[path = "miniwindow/native.rs"]
pub mod native;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn o033_miniwindow_drag_resize_dock_order_and_restart() -> io::Result<()> {
        let mut layout = Layout::default();
        layout.open("Inventory", [100, 100, 300, 200]);
        layout.open("Battle", [200, 200, 300, 200]);
        layout.dragged("Inventory", [0, 210, 300, 200], [1200, 900]);
        layout.dragged("Battle", [0, 0, 300, 150], [1200, 900]);
        let visible = vec!["Inventory".into(), "Battle".into()];
        assert_eq!(
            layout.rect("Battle", [1200, 900], &visible),
            Some([0, 0, 300, 150])
        );
        assert_eq!(
            layout.rect("Inventory", [1200, 900], &visible),
            Some([0, 154, 300, 200])
        );
        let path = std::env::temp_dir().join(format!("o033-layout-{}.json", std::process::id()));
        layout.save(&path)?;
        let mut layout = Layout::load(&path)?;
        assert_eq!(
            layout.rect("Inventory", [1200, 900], &["Inventory".into()]),
            Some([0, 0, 300, 200])
        );
        layout.dock("Inventory", Dock::Right);
        assert_eq!(
            layout.rect("Inventory", [1200, 900], &visible),
            Some([900, 0, 300, 200])
        );
        std::fs::remove_file(path)
    }
}

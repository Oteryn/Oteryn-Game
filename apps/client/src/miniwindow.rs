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
        if pane.dock == Dock::Floating {
            return Some([
                pane.bounds[0].clamp(0, rw - width),
                pane.bounds[1].clamp(0, rh - height),
                width,
                height,
            ]);
        }
        // Fit the visible column together. Collapse trailing panes when even one
        // pixel each cannot fit; returning None would leave native windows in place.
        let column: Vec<_> = self
            .order
            .iter()
            .filter(|entry| visible.contains(entry))
            .filter_map(|entry| self.panes.get(entry).map(|other| (entry, other)))
            .filter(|(_, other)| other.dock == pane.dock)
            .take(usize::try_from(rh).ok()?)
            .collect();
        let count = i32::try_from(column.len()).ok()?;
        if count == 0 {
            return None;
        }
        let gap = 4.min((rh - count) / (count - 1).max(1));
        let budget = rh - gap * (count - 1);
        let weight: i128 = column
            .iter()
            .map(|(_, other)| i128::from(other.bounds[3].clamp(1, rh) - 1))
            .sum();
        let extra = i128::from(budget - count);
        let mut prefix = 0;
        let mut y = 0_i32;
        for (entry, other) in column {
            let requested = other.bounds[3].clamp(1, rh);
            let next = prefix + i128::from(requested - 1);
            let height = if weight > extra {
                // Reserve one pixel each, then apportion the remaining height.
                1 + i32::try_from(next * extra / weight - prefix * extra / weight).ok()?
            } else {
                requested
            };
            if entry == id {
                let x = if pane.dock == Dock::Left {
                    0
                } else {
                    rw - width
                };
                return Some([x, y, width, height]);
            }
            prefix = next;
            y = y.saturating_add(height).saturating_add(gap);
        }
        if visible.iter().any(|entry| entry == id) {
            let x = if pane.dock == Dock::Left {
                0
            } else {
                rw - width
            };
            Some([x, rh, width, 0])
        } else {
            None
        }
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
    fn dock_fitting_handles_tiny_and_extreme_columns_and_hidden_panes() -> io::Result<()> {
        let mut layout = Layout::default();
        let visible: Vec<String> = (0..128).map(|i| i.to_string()).collect();
        for id in &visible {
            layout.open(id, [0, 0, 300, i32::MAX]);
            layout.dock(id, Dock::Right);
        }
        for height in [1, 2, 100, 300, i32::MAX] {
            let mut bottom = 0;
            let mut shown = 0;
            for id in &visible {
                let rect = layout
                    .rect(id, [1200, height], &visible)
                    .ok_or_else(|| io::Error::other("missing native placement"))?;
                assert!(rect[1] >= bottom);
                if rect[3] == 0 {
                    assert_eq!(rect, [900, height, 300, 0]);
                } else {
                    assert!(rect[3] > 0);
                    shown += 1;
                }
                bottom = rect[1] + rect[3];
                assert!(bottom <= height);
            }
            assert_eq!(shown, height.min(128));
        }
        let rect = layout
            .rect("0", [1200, 300], &visible[..1])
            .ok_or_else(|| io::Error::other("visible pane"))?;
        assert_eq!(rect, [900, 0, 300, 300]);
        Ok(())
    }
    #[test]
    fn overflowing_dock_columns_fit_without_overlap_after_resize() -> io::Result<()> {
        for dock in [Dock::Left, Dock::Right] {
            let mut layout = Layout::default();
            let visible: Vec<String> = vec!["Inventory".into(), "Battle".into()];
            for id in &visible {
                layout.open(id, [0, 0, 300, 200]);
                layout.dock(id, dock);
            }
            for height in [900, 300, 100] {
                let first = layout
                    .rect("Inventory", [1200, height], &visible)
                    .ok_or_else(|| io::Error::other("first pane"))?;
                let second = layout
                    .rect("Battle", [1200, height], &visible)
                    .ok_or_else(|| io::Error::other("second pane"))?;
                assert!(first[1] + first[3] <= second[1]);
                assert!(second[1] + second[3] <= height);
                assert!(first[3] > 0 && second[3] > 0);
                assert_eq!(first[0], if dock == Dock::Left { 0 } else { 900 });
            }
        }
        Ok(())
    }
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

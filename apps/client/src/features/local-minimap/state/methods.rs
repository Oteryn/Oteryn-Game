impl Minimap {
    pub fn bind_world(&mut self, world: &str) {
        if !(1..=36).contains(&world.len())
            || !world.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-')
        {
            self.feedback = "Invalid minimap world binding; local saving is disabled".into();
            self.world = None;
        } else if self.world.as_deref() != Some(world) {
            if let Err(error) = self.save() {
                self.feedback = error.to_string();
            }
            self.world = Some(world.into());
            self.binding = None;
            self.cells.clear();
            self.markers.clear();
        }
    }
    fn path(&self) -> Option<PathBuf> {
        let (generation, bundle, epoch) = self.binding?;
        let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        Some(self.root.as_ref()?.join(self.world.as_ref()?).join(format!(
            "{}-{}-{epoch}.json",
            hex(&generation),
            hex(&bundle)
        )))
    }
    pub fn observe(&mut self, map: &WorldMapViewSnapshot) {
        let binding = (
            map.header.content_generation,
            map.header.bundle_digest,
            map.header.reset_epoch,
        );
        if self.binding != Some(binding) {
            if let Err(error) = self.save() {
                self.feedback = error.to_string();
            }
            self.cells.clear();
            self.markers.clear();
            self.dirty = false;
            self.binding = Some(binding);
            if let Err(error) = self.load() {
                self.feedback = format!("Could not load minimap: {error}");
            }
        }
        // The snapshot is authoritative only within its visible window. Retain distant
        // exploration, but remove cleared cells and item-only tiles from remembered ground.
        use oteryn_session::world_map::{
            VIEW_HEIGHT, VIEW_LEFT, VIEW_TOP, VIEW_WIDTH, floors_in_view, in_window,
        };
        let origin = map.header.origin;
        let observed: BTreeSet<_> = map
            .tiles
            .iter()
            .filter(|tile| {
                tile.ground_speed > 0
                    && tile.position.floor <= map.header.first_visible_floor
                    && in_window(origin, tile.position)
            })
            .map(|tile| (tile.position.floor, tile.position.x, tile.position.y))
            .collect();
        if let Some(floors) = floors_in_view(origin.floor) {
            for floor in floors.filter(|f| *f <= map.header.first_visible_floor) {
                let shift = i64::from(floor) - i64::from(origin.floor);
                for y in 0..VIEW_HEIGHT {
                    for x in 0..VIEW_WIDTH {
                        let (Ok(x), Ok(y)) = (
                            i32::try_from(
                                i64::from(origin.x) - i64::from(VIEW_LEFT) + shift + i64::from(x),
                            ),
                            i32::try_from(
                                i64::from(origin.y) - i64::from(VIEW_TOP) + shift + i64::from(y),
                            ),
                        ) else {
                            continue;
                        };
                        let key = (floor, x, y);
                        if !observed.contains(&key) && self.cells.remove(&key).is_some() {
                            self.dirty = true;
                        }
                    }
                }
            }
        }
        for key in observed {
            if self.cells.len() >= MAX_CELLS && !self.cells.contains_key(&key) {
                self.feedback =
                    "Local exploration limit reached; existing exploration is retained".into();
                continue;
            }
            if self.cells.insert(key, 55) != Some(55) {
                self.dirty = true;
            }
        }
        if self.saved_at.elapsed() >= Duration::from_secs(5)
            && let Err(error) = self.save()
        {
            self.feedback = format!("Could not save minimap: {error}");
        }
    }
    pub fn follow(&mut self, x: i32, y: i32, floor: i16) {
        self.own = (x, y, floor);
        if self.tracking {
            self.x = x;
            self.y = y;
            self.floor = floor;
        }
    }
    // Invoked by native Windows controls; the headless tests exercise the same state.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn pan(&mut self, dx: i32, dy: i32) {
        self.tracking = false;
        self.x = self.x.saturating_add(dx.saturating_mul(self.zoom));
        self.y = self.y.saturating_add(dy.saturating_mul(self.zoom));
    }
    pub fn position(&self, x: i32, y: i32) -> Option<(i32, i32, i16)> {
        Some((
            self.x.checked_add((x - 15).checked_mul(self.zoom)?)?,
            self.y.checked_add((y - 12).checked_mul(self.zoom)?)?,
            self.floor,
        ))
    }
    // Invoked by native Windows controls; the headless tests exercise the same state.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn edit_marker(
        &mut self,
        x: i32,
        y: i32,
        floor: i16,
        label: String,
        color: u8,
    ) -> io::Result<()> {
        if !(0..=65_535).contains(&x)
            || !(0..=65_535).contains(&y)
            || !(-15..=0).contains(&floor)
            || color > 3
            || label.trim().is_empty()
            || label.len() > 128
            || label.chars().any(char::is_control)
        {
            return Err(io::Error::other(
                "Use map coordinates 0-65535, floor -15 to 0 and a label of 1-128 bytes",
            ));
        }
        if let Some(marker) = self
            .markers
            .iter_mut()
            .find(|m| (m.x, m.y, m.floor) == (x, y, floor))
        {
            marker.label = label;
            marker.color = color;
        } else {
            if self.markers.len() >= MAX_MARKERS {
                return Err(io::Error::other("The minimap supports up to 512 markers"));
            }
            self.markers.push(Marker {
                x,
                y,
                floor,
                label,
                color,
            });
        }
        self.dirty = true;
        self.save()
    }
    // Invoked by native Windows controls; the headless tests exercise the same state.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn remove_marker(&mut self, x: i32, y: i32, floor: i16) -> io::Result<()> {
        self.markers
            .retain(|m| (m.x, m.y, m.floor) != (x, y, floor));
        self.dirty = true;
        self.save()
    }
    pub fn save(&mut self) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let Some(path) = self.path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let saved = Saved {
            version: 1,
            cells: self
                .cells
                .iter()
                .map(|(&(f, x, y), &c)| [i32::from(f), x, y, i32::from(c)])
                .collect(),
            markers: self.markers.clone(),
        };
        let temporary = path.with_extension("tmp");
        let mut file = std::fs::File::create(&temporary)?;
        serde_json::to_writer(&mut file, &saved).map_err(io::Error::other)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(temporary, path)?;
        self.dirty = false;
        self.saved_at = Instant::now();
        Ok(())
    }
    fn load(&mut self) -> io::Result<()> {
        let Some(path) = self.path() else {
            return Ok(());
        };
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        if file.metadata()?.len() > MAX_BYTES {
            return Err(io::Error::other("Minimap file is too large"));
        }
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(io::Error::other("Minimap file is too large"));
        }
        let saved: Saved = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        if saved.version != 1
            || saved.cells.len() > MAX_CELLS
            || saved.markers.len() > MAX_MARKERS
            || saved.cells.iter().any(|[f, x, y, c]| {
                !(-15..=0).contains(f)
                    || !(0..=65535).contains(x)
                    || !(0..=65535).contains(y)
                    || ![55, 86].contains(c)
            })
            || saved.markers.iter().any(|m| {
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
        // Older caches recorded gray item-only cells. They are not persistent terrain.
        self.dirty |= saved.cells.iter().any(|cell| cell[3] == 86);
        self.cells = saved
            .cells
            .into_iter()
            .filter(|cell| cell[3] == 55)
            .map(|[f, x, y, c]| ((f as i16, x, y), c as u8))
            .collect();
        self.markers = saved.markers;
        Ok(())
    }
}

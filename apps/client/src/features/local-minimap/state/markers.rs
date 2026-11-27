use super::*;

impl Minimap {
    // Invoked by native Windows controls; the headless tests exercise the same state.
    #[allow(dead_code)]
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
            || !(0..=15).contains(&floor)
            || color > 3
            || !valid_marker_description(&label)
        {
            return Err(io::Error::other(tr!(
                "Use map coordinates 0-65535, floor 0 to 15 and a label of up to 128 bytes"
            )));
        }
        self.change_marker(
            (x, y, floor),
            Some(Marker {
                x,
                y,
                floor,
                label,
                color,
            }),
            false,
        )?;
        Ok(())
    }
    pub(in crate::play) fn request_marker_edit(&mut self, at: (i32, i32, i16)) {
        match self.check_marker_target(at, self.path().as_deref()) {
            Ok(()) => self.pending_edit = Some(at),
            Err(error) => self.feedback = error.to_string(),
        }
    }

    /// Inserts a default mark for cache tests that exercise the state directly.
    #[cfg(test)]
    pub fn create_mark_at(&mut self, own: (i32, i32, i16)) -> io::Result<bool> {
        let (x, y, floor) = own;
        if !cache_position(i32::from(floor), x, y) {
            return Err(io::Error::other(tr!(
                "Use map coordinates 0-65535, floor 0 to 15 and a label of up to 128 bytes"
            )));
        }
        self.change_marker(
            own,
            Some(Marker {
                x,
                y,
                floor,
                label: tr!("Mark").into(),
                color: 0,
            }),
            true,
        )
    }
    // Invoked by native Windows controls; the headless tests exercise the same state.
    #[allow(dead_code)]
    pub fn remove_marker(&mut self, x: i32, y: i32, floor: i16) -> io::Result<()> {
        self.change_marker((x, y, floor), None, false)?;
        Ok(())
    }

    fn check_marker_target(
        &self,
        position: (i32, i32, i16),
        path: Option<&Path>,
    ) -> io::Result<()> {
        if let Some(path) = path {
            self.require_cache_owner(path)?;
        } else if self.world.is_some() {
            return Err(io::Error::other("Minimap storage is unavailable"));
        }
        if self
            .quest_markers
            .iter()
            .any(|m| (m.x, m.y, m.floor) == position)
        {
            return Err(io::Error::other("Quest markers are supplied by the server"));
        }
        Ok(())
    }

    fn change_marker(
        &mut self,
        position: (i32, i32, i16),
        replacement: Option<Marker>,
        create_only: bool,
    ) -> io::Result<bool> {
        let path = self.path();
        self.check_marker_target(position, path.as_deref())?;
        let at_position = |marker: &Marker| (marker.x, marker.y, marker.floor) == position;
        let index = self.markers.iter().position(at_position);
        let current = index.map(|index| &self.markers[index]);
        if (create_only && current.is_some()) || current == replacement.as_ref() {
            #[cfg(bevy_shell)]
            if self.background_save {
                return Ok(false);
            }
            self.save()?;
            return Ok(false);
        }
        let mut markers = self.markers.clone();
        match (index, replacement) {
            (Some(index), Some(marker)) => markers[index] = marker,
            (Some(_), None) => markers.retain(|marker| !at_position(marker)),
            (None, Some(marker)) => {
                if markers.len() >= MAX_MARKERS {
                    return Err(io::Error::other(tr!(
                        "The minimap supports up to 512 markers"
                    )));
                }
                markers.push(marker);
            }
            (None, None) => return Ok(false),
        }
        if let Some(path) = path {
            #[cfg(bevy_shell)]
            if self.background_save {
                if self.load_failed {
                    return Err(io::Error::other(tr!("Invalid minimap file")));
                }
                self.markers = markers;
                self.cells_dirty = true;
                self.marker_save_queued = true;
                self.start_marker_save();
                return Ok(true);
            }
            self.write_cache(&path, markers)?;
        } else {
            self.markers = markers;
        }
        Ok(true)
    }
}

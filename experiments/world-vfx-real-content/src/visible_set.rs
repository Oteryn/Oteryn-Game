use crate::prepared_cache::{
    PAGE_BYTES, PreparedCacheStats, PreparedPage, SLOT_SIZE, SpriteLocator,
};
use crate::scene::QualificationBundle;
use std::collections::BTreeSet;

pub const CELL_BYTES: usize = SLOT_SIZE * SLOT_SIZE * 4;
const MAX_VISIBLE_SPRITES: usize = 8_192;
const MAX_VISIBLE_BYTES: usize = MAX_VISIBLE_SPRITES * CELL_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleSpriteEntry {
    pub sprite_source_id: u32,
    pub dense_index: u32,
    pub source_width: u32,
    pub source_height: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VisibleSpriteBuildStats {
    pub sprite_count: usize,
    pub source_page_count: usize,
    pub rgba_bytes: usize,
    pub page_requests: u64,
    pub page_hits: u64,
    pub page_loads: u64,
    pub page_evictions: u64,
    pub bytes_read: u64,
}

#[derive(Debug, Clone)]
pub struct VisibleSpriteSet {
    entries: Vec<VisibleSpriteEntry>,
    rgba_cells: Vec<u8>,
    stats: VisibleSpriteBuildStats,
}

impl VisibleSpriteSet {
    pub fn build(bundle: &mut QualificationBundle) -> Result<Self, String> {
        let mut required = BTreeSet::new();
        required.extend(bundle.scene().unique_sprite_source_ids().iter().copied());
        required.extend(bundle.programs().outfit().sprite_source_ids.iter().copied());
        required.extend(bundle.programs().effect().sprite_source_ids.iter().copied());
        required.extend(
            bundle
                .programs()
                .missile()
                .sprite_source_ids
                .iter()
                .copied(),
        );
        if required.is_empty() || required.len() > MAX_VISIBLE_SPRITES {
            return Err(format!(
                "visible sprite working set must be in 1..={MAX_VISIBLE_SPRITES}, got {}",
                required.len()
            ));
        }
        if required.contains(&0) {
            return Err("visible sprite working set contains sprite_source_id 0".to_owned());
        }

        let byte_len = required
            .len()
            .checked_mul(CELL_BYTES)
            .ok_or_else(|| "visible sprite byte length overflow".to_owned())?;
        if byte_len > MAX_VISIBLE_BYTES {
            return Err(format!(
                "visible sprite working set exceeds {MAX_VISIBLE_BYTES} bytes: {byte_len}"
            ));
        }

        let before = bundle.cache().stats().clone();
        let mut entries = Vec::with_capacity(required.len());
        let mut rgba_cells = Vec::with_capacity(byte_len);
        let mut source_pages = BTreeSet::new();

        for (dense_index, sprite_source_id) in required.into_iter().enumerate() {
            let (locator, page) = bundle.cache_mut().page_for_sprite(sprite_source_id)?;
            source_pages.insert(locator.page_id);
            append_cell(&mut rgba_cells, &page, locator)?;
            entries.push(VisibleSpriteEntry {
                sprite_source_id,
                dense_index: u32::try_from(dense_index)
                    .map_err(|_| "visible sprite dense index exceeds u32".to_owned())?,
                source_width: locator.source_width,
                source_height: locator.source_height,
            });
        }

        if rgba_cells.len() != byte_len {
            return Err(format!(
                "visible sprite payload length mismatch: expected {byte_len}, got {}",
                rgba_cells.len()
            ));
        }
        let after = bundle.cache().stats().clone();
        let stats = diff_stats(&before, &after, entries.len(), source_pages.len(), byte_len);

        Ok(Self {
            entries,
            rgba_cells,
            stats,
        })
    }

    pub fn entries(&self) -> &[VisibleSpriteEntry] {
        &self.entries
    }

    pub fn rgba_cells(&self) -> &[u8] {
        &self.rgba_cells
    }

    pub fn stats(&self) -> &VisibleSpriteBuildStats {
        &self.stats
    }

    pub fn entry(&self, sprite_source_id: u32) -> Option<VisibleSpriteEntry> {
        let index = self
            .entries
            .binary_search_by_key(&sprite_source_id, |entry| entry.sprite_source_id)
            .ok()?;
        self.entries.get(index).copied()
    }

    pub fn cell_rgba(&self, sprite_source_id: u32) -> Option<&[u8]> {
        let entry = self.entry(sprite_source_id)?;
        let start = usize::try_from(entry.dense_index)
            .ok()?
            .checked_mul(CELL_BYTES)?;
        let end = start.checked_add(CELL_BYTES)?;
        self.rgba_cells.get(start..end)
    }
}

fn append_cell(
    target: &mut Vec<u8>,
    page: &PreparedPage,
    locator: SpriteLocator,
) -> Result<(), String> {
    if page.rgba.len() != PAGE_BYTES {
        return Err(format!(
            "prepared page {} payload length changed: expected {PAGE_BYTES}, got {}",
            page.page_id,
            page.rgba.len()
        ));
    }
    if locator.page_id != page.page_id {
        return Err(format!(
            "sprite {} locator page {} does not match loaded page {}",
            locator.sprite_source_id, locator.page_id, page.page_id
        ));
    }
    let layer = usize::try_from(locator.layer)
        .map_err(|_| format!("sprite {} layer exceeds usize", locator.sprite_source_id))?;
    let start = layer
        .checked_mul(CELL_BYTES)
        .ok_or_else(|| format!("sprite {} layer offset overflow", locator.sprite_source_id))?;
    let end = start
        .checked_add(CELL_BYTES)
        .ok_or_else(|| format!("sprite {} layer end overflow", locator.sprite_source_id))?;
    let cell = page.rgba.get(start..end).ok_or_else(|| {
        format!(
            "sprite {} layer {} is outside prepared page {}",
            locator.sprite_source_id, locator.layer, page.page_id
        )
    })?;
    target.extend_from_slice(cell);
    Ok(())
}

fn diff_stats(
    before: &PreparedCacheStats,
    after: &PreparedCacheStats,
    sprite_count: usize,
    source_page_count: usize,
    rgba_bytes: usize,
) -> VisibleSpriteBuildStats {
    VisibleSpriteBuildStats {
        sprite_count,
        source_page_count,
        rgba_bytes,
        page_requests: after.page_requests.saturating_sub(before.page_requests),
        page_hits: after.page_hits.saturating_sub(before.page_hits),
        page_loads: after.page_loads.saturating_sub(before.page_loads),
        page_evictions: after.page_evictions.saturating_sub(before.page_evictions),
        bytes_read: after.bytes_read.saturating_sub(before.bytes_read),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn extracts_exact_64x64_carrier_cell() -> Result<(), String> {
        let mut bytes = vec![0_u8; PAGE_BYTES];
        let layer = 7_usize;
        let start = layer * CELL_BYTES;
        bytes[start..start + CELL_BYTES].fill(23);
        let page = PreparedPage {
            page_id: 4,
            rgba: Arc::from(bytes),
        };
        let locator = SpriteLocator {
            sprite_source_id: 900,
            page_id: 4,
            layer: u32::try_from(layer).map_err(|error| error.to_string())?,
            source_width: 32,
            source_height: 64,
        };
        let mut target = Vec::new();
        append_cell(&mut target, &page, locator)?;
        assert_eq!(target.len(), CELL_BYTES);
        assert!(target.iter().all(|value| *value == 23));
        Ok(())
    }

    #[test]
    fn rejects_locator_page_mismatch() {
        let page = PreparedPage {
            page_id: 2,
            rgba: Arc::from(vec![0_u8; PAGE_BYTES]),
        };
        let locator = SpriteLocator {
            sprite_source_id: 5,
            page_id: 3,
            layer: 0,
            source_width: 32,
            source_height: 32,
        };
        let mut target = Vec::new();
        assert!(append_cell(&mut target, &page, locator).is_err());
    }
}

use crate::play::PlayView;
use oteryn_renderer::TileCoord;
use oteryn_session::WorldObjectOverlayEntry;

/// Byte length of a bundle placement key (`OTERYN_WORLD_BUNDLE_FORMAT_V1.md` §7), and of the key
/// prefixed by the 32-byte bundle digest.
const KEY_BYTES: usize = 8;
const DIGEST_AND_KEY_BYTES: usize = 32 + KEY_BYTES;

/// Semantic placements the server emits today, with the cell each names. The native entry
/// room's door cell (`native_entry.rs` `DOOR_CELL`); a placeholder until map loading carries
/// cell coordinates to the client.
const SEMANTIC_PLACEMENTS: [(&[u8], i32, i32, i16); 1] = [(b"oteryn:cell/entry-door", 1, -1, 0)];

/// The tile and floor a placement names: a known semantic cell key, or the last eight bytes as
/// the big-endian key `x << 32 | y << 16 | (-floor) << 8 | ordinal`. Other shapes name no tile
/// and draw no marker.
fn placement_tile(placement: &[u8]) -> Option<(TileCoord, i16)> {
    if let Some(&(_, x, y, floor)) = SEMANTIC_PLACEMENTS
        .iter()
        .find(|(name, ..)| *name == placement)
    {
        return Some((TileCoord::new(x, y), floor));
    }
    if placement.len() != KEY_BYTES && placement.len() != DIGEST_AND_KEY_BYTES {
        return None;
    }
    let key = u64::from_be_bytes(placement[placement.len() - KEY_BYTES..].try_into().ok()?);
    let x = i32::try_from(key >> 32).ok()?;
    let y = i32::from(u16::try_from((key >> 16) & 0xffff).ok()?);
    let floor = -i16::from(u8::try_from((key >> 8) & 0xff).ok()?);
    Some((TileCoord::new(x, y), floor))
}

impl PlayView {
    #[must_use]
    pub fn marker_count(&self) -> usize {
        self.drawn_markers().count()
    }

    pub(in crate::play) fn drawn_markers(&self) -> impl Iterator<Item = TileCoord> + '_ {
        self.markers
            .values()
            .filter(|(_, floor)| *floor == self.floor)
            .map(|(tile, _)| *tile)
    }

    pub(in crate::play) fn set_marker(&mut self, entry: &WorldObjectOverlayEntry) {
        match placement_tile(&entry.placement) {
            Some(placed) => {
                self.markers.insert(entry.placement.clone(), placed);
            }
            None => {
                self.markers.remove(&entry.placement);
            }
        }
    }
}

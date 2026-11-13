//! Native counterparts may share one exactly identical quest-owned counter.
use super::QuestTrack;

pub(super) fn insert(tracks: &mut Vec<QuestTrack>, track: QuestTrack) -> Result<(), String> {
    if let Some(existing) = tracks.iter().find(|existing| existing.key == track.key) {
        if existing != &track {
            return Err(format!(
                "conflicting authored track declaration {}",
                track.key
            ));
        }
    } else {
        tracks.push(track);
    }
    Ok(())
}

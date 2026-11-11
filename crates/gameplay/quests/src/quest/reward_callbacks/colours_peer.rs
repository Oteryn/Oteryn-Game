//! M43 adds only the peer step; the frozen opening owner remains its dependency.
use super::Document;

pub(super) const KEY: &str = "oteryn:quest-reward/chosen/colours/pair-yellow";
pub(super) const DATA: &str =
    include_str!("../../../../../../content/quests/npc-callbacks/m43-colours-peer.json");

pub(super) fn append_after_opening(document: &mut Document) -> Result<(), String> {
    if !document.callbacks.iter().any(|callback| {
        callback.key == "oteryn:quest-reward/chosen/colours/mark-blue"
            && callback.quest == "oteryn:quest.authored.the_colours_of_magic"
            && callback.carried_item.as_deref() == Some("oteryn:item.tibia.i12208")
    }) {
        return Ok(());
    }
    let mut authored: Document = serde_json::from_str(DATA).map_err(|e| e.to_string())?;
    if authored.schema != 1 || !authored.tracks.is_empty() || authored.callbacks.len() != 1 {
        return Err("M43 peer callback scope".into());
    }
    let callback = authored.callbacks.pop().ok_or("M43 peer callback")?;
    if callback.key != KEY {
        return Err("M43 peer callback identity".into());
    }
    if let Some(existing) = document.callbacks.iter().find(|row| row.key == KEY) {
        return if existing == &callback {
            Ok(())
        } else {
            Err("conflicting M43 peer callback".into())
        };
    }
    document.callbacks.push(callback);
    Ok(())
}

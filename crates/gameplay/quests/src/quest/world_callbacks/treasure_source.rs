//! Two reviewed wiki-backed caches; no claim that their allocation is donor-exact.
use super::{Callback, Position, Trigger};

pub(super) fn admitted(c: &Callback) -> bool {
    let (label, x, y, item, count) = match c.key.as_str() {
        "oteryn:quest-callback/m09-treasure-telescope" => {
            ("telescope", 32144, 32968, "oteryn:item.tibia.i9616", 1)
        }
        "oteryn:quest-callback/m09-treasure-ingots" => {
            ("ingots", 32139, 32978, "oteryn:item.tibia.i9058", 3)
        }
        _ => return false,
    };
    c.quest == "oteryn:quest.treasure_hunt_quest"
        && c.trigger == Trigger::Use
        && c.selector.position == Some(Position { x, y, floor: -13 })
        && c.selector.appearance == Some(2478)
        && c.selector.unique.is_none()
        && c.selector.action.is_none()
        && c.sources.iter().any(|s| s == "https://www.tibiawiki.com.br/index.php?title=Treasure_Hunt_Quest&oldid=431833#Recompensa")
        && c.effects.item_reward.as_ref().is_some_and(|r|
            r.item == item && r.count == count && r.additional.is_empty() && r.contents.is_empty())
        && c.storage.len() == 1
        && c.storage[0].track == format!("oteryn:quest-progress/m09/treasure/{label}")
        && c.storage[0].minimum == -1 && c.storage[0].maximum == 0 && c.storage[0].set == 1
        && !c.storage[0].preserve && !c.storage[0].increment && !c.storage[0].at_least
}

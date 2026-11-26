//! Source anchors of Arito's entrance.
use super::Position;

pub const SCIMITAR: &str = "oteryn:item.tibia.i3307";
pub const PLACE_SCIMITAR: &str = "oteryn:quest-callback/nomads-land-place-scimitar";
pub const OPEN_ENTRANCE: &str = "oteryn:quest-callback/nomads-land-open-entrance";

pub const CRACK: Position = Position {
    x: 33205,
    y: 32537,
    floor: -6,
};
pub const WATER: Position = Position {
    x: 33207,
    y: 32537,
    floor: -6,
};
pub const ENTRANCE: Position = Position {
    x: 33206,
    y: 32536,
    floor: -6,
};
pub const WATER_SUBTYPE: i16 = 1;

#[cfg(test)]
#[test]
fn arito_referral_is_followed_by_the_exact_native_ritual() -> Result<(), String> {
    let rows = super::load()?;
    let place = rows
        .iter()
        .find(|row| row.key == PLACE_SCIMITAR)
        .ok_or(PLACE_SCIMITAR)?;
    let open = rows
        .iter()
        .find(|row| row.key == OPEN_ENTRANCE)
        .ok_or(OPEN_ENTRANCE)?;
    let [offering] = place.offerings.as_slice() else {
        return Err("one scimitar".into());
    };
    assert_eq!(
        (offering.position, offering.item.as_str(), offering.count),
        (CRACK, SCIMITAR, 1)
    );
    assert_eq!(
        (place.effects.transform, open.effects.transform),
        (Some(5858), Some(7181))
    );
    assert_eq!(open.selector.position, Some(ENTRANCE));
    let [gate] = open.world_requirements.as_slice() else {
        return Err("placed scimitar".into());
    };
    assert_eq!((gate.position, gate.appearance), (CRACK, 5858));
    for callback in [place, open] {
        assert_eq!(callback.effects.reset_after_ms, Some(120_000));
        let [stage, proof] = callback.storage.as_slice() else {
            return Err("Arito referral stage".into());
        };
        assert!(stage.track.ends_with("/aritos_task") && stage.preserve);
        assert_eq!((stage.minimum, stage.maximum), (1, 2));
        assert!(proof.at_least && proof.set == 1);
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn revised_quest_operations_accept_their_saved_progress() -> Result<(), String> {
    use crate::quest::{loader, npc_rules};

    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let previous = [
        (
            "oteryn:quest.tibia_tales",
            b"08ddd97de427afb52741eacdef7781de95f438f5a86a88915d484b447e885db7",
        ),
        (
            "oteryn:quest.authored.nomads_land_quest",
            b"852db8606f6acf8a6d12e55c775a2f62a2eeac0594c6457af10ebb0f6b2e2325",
        ),
        (
            "oteryn:quest.the_explorer_society_quest",
            b"f51489994ea4f58e5ed3bd18de0a2120cb2852b8005a63eaee08ff86b4128fb1",
        ),
        (
            "oteryn:quest.golden_outfits_quest",
            b"70331b045a34a984b4554df6b9ddc28c69e98137c5db5f12c6be18dbb95c83dc",
        ),
    ];
    for (quest, old) in previous {
        let pin = crate::quest::published_quest_pins::hash(old);
        let active = catalogue.definition_hash(quest).ok_or(quest)?;
        assert!(
            pin == active || npc_rules::legacy_defaults(catalogue, quest, &pin).is_some(),
            "{quest}"
        );
    }
    Ok(())
}

use super::*;

#[test]
fn siflind_paint_grant_is_registered_at_the_source_confirmation() -> Result<(), String> {
    let callback = load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-reward/ice-islands/siflind-paint")
        .ok_or("missing Siflind paint grant")?;
    assert_eq!(callback.npc, "oteryn:npc.siflind");
    assert_eq!(callback.keywords, ["yes"]);
    assert_eq!((callback.topic, callback.next_topic), (4, 0));
    assert_eq!(callback.storage.len(), 2);
    let questline = &callback.storage[0];
    assert_eq!(questline.track, crate::quest::ice_islands_seals::QUESTLINE);
    assert_eq!(
        (questline.minimum, questline.maximum, questline.set),
        (7, 7, 8)
    );
    let mission = &callback.storage[1];
    assert_eq!(mission.track, crate::quest::ice_islands_seals::MISSION04);
    assert_eq!((mission.minimum, mission.maximum, mission.set), (0, 2, 1));
    let item = callback.give.ok_or("paint vial")?;
    assert_eq!(item.item, crate::quest::ice_islands_seals::PAINT);
    assert_eq!(item.count, 1);
    assert!(!callback.completes);
    Ok(())
}

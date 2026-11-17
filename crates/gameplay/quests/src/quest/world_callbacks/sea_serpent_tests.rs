#[test]
fn sea_serpent_telescope_is_registered_at_the_lookout() -> Result<(), String> {
    let callbacks = super::load()?;
    assert!(callbacks.iter().any(|callback| {
        callback.key == "oteryn:quest-callback/sea-serpent-telescope"
            && callback.trigger == super::Trigger::Use
            && callback.selector.native.as_ref().is_some_and(|selector| {
                selector.position.x == 31942
                    && selector.position.y == 31045
                    && selector.position.floor == -2
                    && selector.ordinal == 2
                    && selector.appearance == 938
            })
    }));
    Ok(())
}

#[test]
fn sea_serpent_telescope_lowers_against_imported_hunt_tracks() -> Result<(), String> {
    use crate::quest::{loader, sea_serpent};
    let state = loader::load_embedded_quest_state("sea-serpent-test")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = state.catalogue();
    let transition = catalogue
        .transition(sea_serpent::KEY)
        .ok_or("telescope transition")?;
    assert_eq!(transition.quest, sea_serpent::QUEST);
    for key in sea_serpent::TRACKS {
        assert_eq!(
            catalogue.track(key).map(|track| track.quest.as_str()),
            Some(sea_serpent::QUEST)
        );
    }
    Ok(())
}

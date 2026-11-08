//! M33: identical STEP selectors may be disambiguated only by exclusive personal phases.
use super::*;

#[test]
fn m33_disjoint_step_phases_share_one_native_selector() -> Result<(), String> {
    let mut first = load()?
        .into_iter()
        .find(|c| c.key.ends_with("/behemoth"))
        .ok_or("template")?;
    first.trigger = Trigger::StepIn;
    first.selector.unique = None;
    first.selector.action = None;
    first.selector.position = Some(Position {
        x: 32627,
        y: 32076,
        floor: -7,
    });
    first.selector.appearance = Some(231);
    first.storage = vec![Storage {
        track: "oteryn:quest-progress/m33/phase".into(),
        minimum: 0,
        maximum: 0,
        set: 0,
        preserve: true,
        increment: false,
        at_least: false,
    }];
    let mut second = first.clone();
    second.key.push_str("-later-phase");
    second.storage[0].minimum = 5;
    second.storage[0].maximum = 5;
    validate_registry(&[first.clone(), second.clone()])?;
    second.storage[0].minimum = 0;
    assert!(
        validate_registry(&[first, second]).is_err(),
        "overlapping phases remain ambiguous"
    );
    Ok(())
}

#[test]
fn m33_authored_feaster_phases_fit_the_derived_programme_track() -> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    let track = loaded
        .catalogue()
        .track("oteryn:quest-progress/questpc7/feaster/programme")
        .ok_or("programme")?;
    assert!(track.min <= 0 && track.max >= 5);
    Ok(())
}

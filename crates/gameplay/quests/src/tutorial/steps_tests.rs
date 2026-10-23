use super::*;

#[test]
fn tutorial_non_newhaven_start_is_inert_for_every_event() {
    for event in [
        TutorialEvent::Login,
        TutorialEvent::Forcefield,
        TutorialEvent::Hunting,
        TutorialEvent::Corruptor,
        TutorialEvent::Death,
        TutorialEvent::Departure,
    ] {
        assert_eq!(tutorial_transition(false, None, event, "none", false), None);
    }
}

#[test]
fn tutorial_login_places_only_new_or_departed_vocationless_non_access_characters()
-> Result<(), &'static str> {
    for (vocation, access) in [("knight", false), ("none", true)] {
        assert_eq!(
            tutorial_transition(true, None, TutorialEvent::Login, vocation, access),
            None
        );
    }
    let first = tutorial_transition(true, None, TutorialEvent::Login, "none", false)
        .ok_or("first login places the islet")?;
    assert!(first.place_islet);
    assert_eq!(first.progress.step, TutorialStep::Islet);
    assert_eq!(
        tutorial_transition(
            true,
            Some(first.progress),
            TutorialEvent::Login,
            "none",
            false
        ),
        None
    );
    Ok(())
}

#[test]
fn tutorial_hints_loot_death_and_departure_follow_the_seven_steps() -> Result<(), &'static str> {
    let island = tutorial_transition(true, None, TutorialEvent::Forcefield, "none", false)
        .ok_or("forcefield arms tracks")?;
    assert_eq!(island.hint, Some(2));
    assert_eq!(
        tutorial_transition(
            true,
            Some(island.progress),
            TutorialEvent::Hunting,
            "none",
            false
        ),
        None
    );
    for (vocation, hint) in [
        ("knight", 3),
        ("royal_paladin", 4),
        ("sorcerer", 5),
        ("elder_druid", 6),
        ("monk", 11),
    ] {
        let hunt = tutorial_transition(
            true,
            Some(island.progress),
            TutorialEvent::Hunting,
            vocation,
            false,
        )
        .ok_or("base vocation hint")?;
        assert_eq!(hunt.hint, Some(hint));
        assert!(!hunt.progress.hunting);
        assert_eq!(
            tutorial_transition(
                true,
                Some(hunt.progress),
                TutorialEvent::Hunting,
                vocation,
                false
            ),
            None
        );
    }
    let loot = tutorial_transition(
        true,
        Some(island.progress),
        TutorialEvent::Corruptor,
        "knight",
        false,
    )
    .ok_or("Corruptor grants once")?;
    assert!(loot.grant_corruptor_loot);
    assert_eq!(
        tutorial_transition(
            true,
            Some(loot.progress),
            TutorialEvent::Corruptor,
            "knight",
            false
        ),
        None
    );
    let death = tutorial_transition(
        true,
        Some(island.progress),
        TutorialEvent::Death,
        "none",
        false,
    )
    .ok_or("owner selected islet respawn")?;
    assert!(death.place_islet);
    assert_eq!(death.progress.step, TutorialStep::Islet);
    let departure = tutorial_transition(
        true,
        Some(island.progress),
        TutorialEvent::Departure,
        "none",
        false,
    )
    .ok_or("departure clears all tracks")?;
    assert_eq!(
        departure.progress,
        TutorialProgress {
            step: TutorialStep::Departed,
            hunting: false,
            corruptor: false
        }
    );
    assert_eq!(
        tutorial_transition(
            true,
            Some(departure.progress),
            TutorialEvent::Death,
            "none",
            false
        ),
        None
    );
    Ok(())
}

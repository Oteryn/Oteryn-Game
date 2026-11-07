use oteryn_gameplay_world_change::timed::{EventKind, EventState, Input, MINUTE, Phase};

#[test]
fn wevents_insect_bucket_full_and_restored_interactions_are_noops() -> Result<(), &'static str> {
    let mut s = EventState::for_kind(EventKind::InsectoidInvasion, 0, 24 * 60 * MINUTE)?;
    for _ in 0..3 {
        assert!(s.advance(EventKind::InsectoidInvasion, 1, Input::Fill(0), 0)?);
    }
    assert!(!s.advance(EventKind::InsectoidInvasion, 1, Input::Fill(0), 0)?);
    for member in 1..=3 {
        assert!(s.advance(
            EventKind::InsectoidInvasion,
            2,
            Input::Extinguish(member),
            0
        )?);
    }
    assert_eq!(s.phase, Phase::Victory);
    assert!(!s.advance(EventKind::InsectoidInvasion, 3, Input::Fill(0), 0)?);
    assert!(!s.advance(EventKind::InsectoidInvasion, 3, Input::Extinguish(1), 0)?);
    Ok(())
}

#[test]
fn wevents_insect_population_uses_synchronized_ninety_second_slots() -> Result<(), &'static str> {
    let mut s = EventState::for_kind(EventKind::InsectoidInvasion, 0, 24 * 60 * MINUTE)?;
    assert!(s.advance(EventKind::InsectoidInvasion, 0, Input::Tick, 0)?);
    assert_eq!(s.wave, 1);
    assert!(!s.advance(EventKind::InsectoidInvasion, 89_999, Input::Tick, 0)?);
    assert!(s.advance(EventKind::InsectoidInvasion, 90_000, Input::Tick, 0)?);
    assert_eq!(s.wave, 2);
    assert!(!s.advance(EventKind::InsectoidInvasion, 90_000, Input::Tick, u64::MAX)?);
    Ok(())
}

#[test]
fn wevents_insect_reserves_only_exact_source_memberships() {
    use oteryn_gameplay_world_change::owner::reserved_static_spawn;
    for source in [
        "oteryn:spawn.x32210_y32056_z7",
        "oteryn:spawn.x32210_y32063_z7",
    ] {
        assert!(reserved_static_spawn(
            source,
            "oteryn:creature.insectoid_scout"
        ));
        assert!(!reserved_static_spawn(source, "oteryn:creature.rat"));
    }
    assert!(!reserved_static_spawn(
        "oteryn:spawn.unrelated",
        "oteryn:creature.insectoid_scout"
    ));
}

#[test]
fn wevents_insect_reset_restores_empty_bucket_and_all_fires() -> Result<(), &'static str> {
    let mut old = EventState::for_kind(EventKind::InsectoidInvasion, 0, 24 * 60 * MINUTE)?;
    old.advance(EventKind::InsectoidInvasion, 1, Input::Fill(0), 0)?;
    old.advance(EventKind::InsectoidInvasion, 2, Input::Extinguish(1), 0)?;
    let fresh = EventState::for_kind(
        EventKind::InsectoidInvasion,
        24 * 60 * MINUTE,
        48 * 60 * MINUTE,
    )?;
    assert_eq!(fresh.phase, Phase::Active);
    assert_eq!(fresh.lifecycle.objects[&0].units(), 0);
    for member in 1..=3 {
        assert!(fresh.lifecycle.objects[&member].lit());
    }
    assert!(!old.lifecycle.objects[&1].lit());
    Ok(())
}

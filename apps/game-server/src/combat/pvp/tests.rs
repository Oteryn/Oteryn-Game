use super::*;
use oteryn_simulation_determinism::SemanticTimeMicros as T;

fn t(ms: u64) -> T {
    T::from_micros(ms * 1_000)
}

fn actor() -> PvpActor {
    PvpActor {
        level: 20,
        in_protection_zone: false,
        on_no_pvp_tile: Some(false),
        post_login_immune: false,
        black_skull: false,
        secure_mode: false,
        expert_mode: ExpertMode::RedFist,
    }
}

fn target() -> PvpTarget {
    PvpTarget {
        level: 20,
        in_protection_zone: false,
        on_no_pvp_tile: Some(false),
        marked: false,
    }
}

fn relation() -> PvpRelation {
    PvpRelation {
        war_between: false,
        same_party_or_guild: Some(false),
        friendly_fire: false,
        target_aggressor_of_actor: false,
        target_aggressor_of_actor_group: false,
    }
}

fn check(world: WorldPvpType, a: PvpActor, b: PvpTarget, r: PvpRelation) -> Result<(), PvpRefusal> {
    check_pvp_legality(world, &a, &b, &r)
}

#[test]
fn open_world_allows_plain_attack_and_halves_damage() {
    assert_eq!(
        check(WorldPvpType::Open, actor(), target(), relation()),
        Ok(())
    );
    assert_eq!(pvp_damage(101, false), 50);
    assert_eq!(pvp_damage(101, true), 101);
}

#[test]
fn optional_world_needs_a_war_hardcore_does_not() {
    let r = relation();
    assert_eq!(
        check(WorldPvpType::Optional, actor(), target(), r),
        Err(PvpRefusal::WorldType)
    );
    let war = PvpRelation {
        war_between: true,
        ..r
    };
    assert_eq!(
        check(WorldPvpType::Optional, actor(), target(), war),
        Ok(())
    );
    assert_eq!(check(WorldPvpType::Hardcore, actor(), target(), r), Ok(()));
}

#[test]
fn level_zone_and_no_pvp_tile_refuse_in_order() {
    let low = PvpActor {
        level: 7,
        ..actor()
    };
    assert_eq!(
        check(WorldPvpType::Open, low, target(), relation()),
        Err(PvpRefusal::Level)
    );
    let low_t = PvpTarget {
        level: 7,
        ..target()
    };
    assert_eq!(
        check(WorldPvpType::Open, actor(), low_t, relation()),
        Err(PvpRefusal::Level)
    );
    let pz = PvpTarget {
        in_protection_zone: true,
        on_no_pvp_tile: Some(true),
        ..target()
    };
    assert_eq!(
        check(WorldPvpType::Open, actor(), pz, relation()),
        Err(PvpRefusal::ProtectionZone)
    );
    let np = PvpActor {
        on_no_pvp_tile: Some(true),
        ..actor()
    };
    for world in [WorldPvpType::Open, WorldPvpType::Hardcore] {
        assert_eq!(
            check(world, np, target(), relation()),
            Err(PvpRefusal::NoPvpTile)
        );
    }
    let unknown = PvpTarget {
        on_no_pvp_tile: None,
        ..target()
    };
    assert_eq!(
        check(WorldPvpType::Open, actor(), unknown, relation()),
        Err(PvpRefusal::NoPvpTile)
    );
}

#[test]
fn post_login_immunity_blocks_aggression_but_not_answering() {
    let fresh = PvpActor {
        post_login_immune: true,
        ..actor()
    };
    assert_eq!(
        check(WorldPvpType::Open, fresh, target(), relation()),
        Err(PvpRefusal::PostLoginImmunity)
    );
    let answer = PvpRelation {
        target_aggressor_of_actor: true,
        ..relation()
    };
    assert_eq!(check(WorldPvpType::Open, fresh, target(), answer), Ok(()));
}

#[test]
fn party_members_are_protected_except_friendly_fire_and_unconfirmed_refuses() {
    let member = PvpRelation {
        same_party_or_guild: Some(true),
        ..relation()
    };
    assert_eq!(
        check(WorldPvpType::Open, actor(), target(), member),
        Err(PvpRefusal::PartyMember)
    );
    let ff = PvpRelation {
        friendly_fire: true,
        ..member
    };
    assert_eq!(check(WorldPvpType::Open, actor(), target(), ff), Ok(()));
    let stale = PvpRelation {
        same_party_or_guild: None,
        ..relation()
    };
    assert_eq!(
        check(WorldPvpType::Open, actor(), target(), stale),
        Err(PvpRefusal::PartyState)
    );
}

#[test]
fn secure_mode_and_black_skull_need_a_marked_target() {
    let secure = PvpActor {
        secure_mode: true,
        ..actor()
    };
    assert_eq!(
        check(WorldPvpType::Open, secure, target(), relation()),
        Err(PvpRefusal::SecureMode)
    );
    let marked = PvpTarget {
        marked: true,
        ..target()
    };
    assert_eq!(
        check(WorldPvpType::Open, secure, marked, relation()),
        Ok(())
    );
    // Hardcore ignores secure mode.
    assert_eq!(
        check(WorldPvpType::Hardcore, secure, target(), relation()),
        Ok(())
    );
    let black = PvpActor {
        black_skull: true,
        ..actor()
    };
    assert_eq!(
        check(WorldPvpType::Open, black, target(), relation()),
        Err(PvpRefusal::BlackSkull)
    );
    assert_eq!(check(WorldPvpType::Open, black, marked, relation()), Ok(()));
}

#[test]
fn expert_mode_restricts_whom_an_actor_may_start_against() {
    let with = |mode| PvpActor {
        expert_mode: mode,
        ..actor()
    };
    let rel = relation();
    let aggr = PvpRelation {
        target_aggressor_of_actor: true,
        ..rel
    };
    let group = PvpRelation {
        target_aggressor_of_actor_group: true,
        ..rel
    };
    let marked = PvpTarget {
        marked: true,
        ..target()
    };
    let w = WorldPvpType::Open;
    assert_eq!(
        check(w, with(ExpertMode::Dove), target(), rel),
        Err(PvpRefusal::ExpertMode)
    );
    assert_eq!(check(w, with(ExpertMode::Dove), target(), aggr), Ok(()));
    assert_eq!(
        check(w, with(ExpertMode::Dove), target(), group),
        Err(PvpRefusal::ExpertMode)
    );
    assert_eq!(
        check(w, with(ExpertMode::WhiteHand), target(), group),
        Ok(())
    );
    assert_eq!(
        check(w, with(ExpertMode::WhiteHand), marked, rel),
        Err(PvpRefusal::ExpertMode)
    );
    assert_eq!(check(w, with(ExpertMode::YellowHand), marked, rel), Ok(()));
    assert_eq!(check(w, with(ExpertMode::RedFist), target(), rel), Ok(()));
    // Hardcore is locked to Red Fist: a stored Dove restricts nothing.
    assert_eq!(
        check(
            WorldPvpType::Hardcore,
            with(ExpertMode::Dove),
            target(),
            rel
        ),
        Ok(())
    );
    // A skulled party member stays protected by rule 5 even for Yellow Hand.
    let member = PvpRelation {
        same_party_or_guild: Some(true),
        ..rel
    };
    assert_eq!(
        check(w, with(ExpertMode::YellowHand), marked, member),
        Err(PvpRefusal::PartyMember)
    );
}

#[test]
fn combat_lock_blocks_logout_pz_entry_and_landing_and_never_shortens() {
    let mut lock = CombatLock::default();
    assert!(!lock.blocks_logout(t(0), None));
    assert!(lock.landing_allowed(t(0), true));

    lock.on_pvp_hit(t(0));
    assert!(lock.blocks_logout(t(59_999), None));
    assert!(!lock.blocks_logout(t(60_000), None));
    // A pure logout block does not bar PZ entry.
    assert!(lock.landing_allowed(t(1_000), true));

    lock.on_aggression(t(1_000));
    assert!(lock.blocks_pz_entry(t(60_999)));
    assert!(!lock.landing_allowed(t(2_000), true));
    assert!(lock.landing_allowed(t(2_000), false));
    assert!(!lock.blocks_pz_entry(t(61_000)));
    assert!(lock.blocks_voluntary_switch(t(30_000), None));

    // A refresh extends; a restore with an older deadline never shortens.
    lock.on_aggression(t(30_000));
    lock.restore(None, Some(t(10_000)), None);
    assert!(lock.blocks_pz_entry(t(89_999)));
    assert!(!lock.blocks_pz_entry(t(90_000)));

    lock.on_kill(t(1_000_000));
    assert!(lock.blocks_logout(t(999_999), None));
    assert!(!lock.landing_allowed(t(999_999), true));
    assert!(!lock.blocks_logout(t(1_000_000), None));
    // The PvE in-fight deadline is honoured too.
    assert!(lock.blocks_logout(t(1_000_000), Some(t(1_000_001))));
}

#[test]
fn restore_after_readmission_applies_stored_deadlines() {
    let mut lock = CombatLock::default();
    lock.restore(Some(t(5_000)), Some(t(6_000)), Some(t(7_000)));
    assert!(lock.blocks_pz_entry(t(5_999)));
    assert!(lock.blocks_logout(t(6_999), None));
    assert!(!lock.blocks_logout(t(7_000), None));
}

#[test]
fn aggression_relations_track_first_and_retaliation_and_cap() {
    let mut rel = AggressionRelations::<u32>::default();
    assert_eq!(rel.record(1, false, t(0)), AggressionOutcome::Started);
    assert!(rel.has_aggression_toward(1, t(59_999)));
    assert!(rel.was_first_against(1, t(0)));
    assert_eq!(rel.record(1, false, t(10)), AggressionOutcome::Refreshed);
    assert!(!rel.has_aggression_toward(1, t(60_010)));
    // Answering an aggressor is retaliation: no new PZ block for the caller to apply.
    assert_eq!(rel.record(2, true, t(0)), AggressionOutcome::Retaliation);
    assert!(!rel.was_first_against(2, t(0)));

    let mut full = AggressionRelations::<u32>::default();
    for id in 0..64 {
        assert_eq!(full.record(id, false, t(0)), AggressionOutcome::Started);
    }
    assert_eq!(full.record(99, false, t(1)), AggressionOutcome::Refused);
    // Once the relations expire, the oldest expired goes first.
    assert_eq!(
        full.record(99, false, t(60_000)),
        AggressionOutcome::Started
    );
}

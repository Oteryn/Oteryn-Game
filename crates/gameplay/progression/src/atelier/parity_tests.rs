use super::*;

type Error = Box<dyn std::error::Error>;

fn state() -> Result<AtelierState, Error> {
    let data = WheelGemData::embedded()?;
    Ok(AtelierState {
        vocation: "knight".into(),
        gold: 100_000_000,
        unrevealed: [GemQuality::Lesser, GemQuality::Regular, GemQuality::Greater]
            .into_iter()
            .map(|quality| (quality, 1))
            .collect(),
        fragments: [(FragmentKind::Basic, 50), (FragmentKind::Supreme, 50)]
            .into_iter()
            .collect(),
        gems: [1, 2]
            .into_iter()
            .map(|id| {
                (
                    id,
                    Gem {
                        id,
                        quality: GemQuality::Lesser,
                        domain: WheelDomain::Green,
                        mods: vec![31],
                        locked: false,
                        vessel: false,
                        ruleset_revision: data.revision().into(),
                    },
                )
            })
            .collect(),
        ..Default::default()
    })
}

fn refused(state: &AtelierState, action: AtelierAction, reason: AtelierRefusal) {
    let mut candidate = state.clone();
    assert_eq!(candidate.commit(state.revision, action), Err(reason));
    assert_eq!(&candidate, state, "refusal must not change any state");
}

fn reveal(quality: GemQuality) -> AtelierAction {
    AtelierAction::Reveal {
        id: 100,
        quality,
        domain: WheelDomain::Red,
        mods: match quality {
            GemQuality::Lesser => vec![31],
            GemQuality::Regular => vec![31, 0],
            GemQuality::Greater => vec![31, 0, 5],
        },
    }
}

#[test]
fn reveal_quality_prices_shapes_and_resource_refusals_are_atomic() -> Result<(), Error> {
    // Crystal player_wheel.cpp:1218-1250; configmanager.cpp:435-437.
    for (quality, fee, mods) in [
        (GemQuality::Lesser, 125_000, vec![31]),
        (GemQuality::Regular, 1_000_000, vec![31, 0]),
        (GemQuality::Greater, 6_000_000, vec![31, 0, 5]),
    ] {
        let mut before = state()?;
        before.gold = fee;
        let mut next = before.clone();
        let outcome = next
            .commit(0, reveal(quality))
            .map_err(|error| format!("reveal refused: {error:?}"))?;
        assert_eq!(outcome.revision, 1);
        assert_eq!(next.gold, 0);
        assert_eq!(next.unrevealed.get(&quality), Some(&0));
        let gem = next.gems.get(&100).ok_or("revealed gem missing")?;
        assert_eq!(gem.quality, quality);
        assert_eq!(gem.domain, WheelDomain::Red);
        assert_eq!(gem.mods, mods);
        assert!(!gem.locked && !gem.vessel);

        before.gold = fee - 1;
        refused(&before, reveal(quality), AtelierRefusal::InsufficientGold);
        before.gold = fee;
        before.unrevealed.remove(&quality);
        refused(&before, reveal(quality), AtelierRefusal::NoGem);
        before.unrevealed.insert(quality, 0);
        refused(&before, reveal(quality), AtelierRefusal::NoGem);
    }
    Ok(())
}

#[test]
fn reveal_rejects_duplicate_identity_wrong_shape_and_incompatible_modifiers() -> Result<(), Error> {
    // Crystal player_wheel.cpp:1243-1250,4124-4129 restricts positions and repeats.
    let state = state()?;
    for (id, quality, mods) in [
        (1, GemQuality::Lesser, vec![31]),
        (100, GemQuality::Lesser, vec![]),
        (100, GemQuality::Regular, vec![31]),
        (100, GemQuality::Regular, vec![3, 3]),
        (100, GemQuality::Regular, vec![31, 31]),
        (100, GemQuality::Greater, vec![31, 0, 42]),
        (100, GemQuality::Lesser, vec![u16::MAX]),
    ] {
        refused(
            &state,
            AtelierAction::Reveal {
                id,
                quality,
                domain: WheelDomain::Green,
                mods,
            },
            AtelierRefusal::InvalidDraw,
        );
    }
    Ok(())
}

#[test]
fn revealed_gem_limit_and_vocation_validation_refuse_without_spending() -> Result<(), Error> {
    // The accepted native WHEEL-GEM-0A cap is shared with the protocol reply bound.
    let mut before = state()?;
    let template = before.gems.get(&1).ok_or("gem")?.clone();
    before.gems = (1..=225)
        .map(|id| {
            (
                id,
                Gem {
                    id,
                    ..template.clone()
                },
            )
        })
        .collect();
    refused(&before, reveal(GemQuality::Lesser), AtelierRefusal::Full);
    before.gems.clear();
    before.vocation.clear();
    refused(
        &before,
        reveal(GemQuality::Lesser),
        AtelierRefusal::InvalidDraw,
    );
    before.vocation = "elite_knight".into();
    before
        .commit(0, reveal(GemQuality::Lesser))
        .map_err(|error| format!("promoted vocation refused: {error:?}"))?;
    assert!(before.gems.contains_key(&100));
    Ok(())
}

#[test]
fn shared_modifier_grade_costs_cap_and_promotion_points_follow_crystal() -> Result<(), Error> {
    // Crystal player_wheel.cpp:1514-1597; grades are shared per modifier.
    for (kind, mod_id, costs) in [
        (FragmentKind::Basic, 31, [2_000_000, 5_000_000, 30_000_000]),
        (
            FragmentKind::Supreme,
            5,
            [5_000_000, 12_000_000, 75_000_000],
        ),
    ] {
        let mut state = state()?;
        for (index, (fee, fragments)) in costs.into_iter().zip([5, 15, 30]).enumerate() {
            let before = state.clone();
            let outcome = state
                .commit(state.revision, AtelierAction::GradeUp { kind, mod_id })
                .map_err(|error| format!("grade refused: {error:?}"))?;
            assert_eq!(state.gold, before.gold - fee);
            assert_eq!(
                state.fragments.get(&kind).copied(),
                before.fragments.get(&kind).map(|held| held - fragments)
            );
            assert_eq!(state.grades.get(&(kind, mod_id)), Some(&(index as u8 + 1)));
            assert_eq!(outcome.promotion_points, u32::from(index == 2));
            assert_eq!(state.gems, before.gems);
        }
        refused(
            &state,
            AtelierAction::GradeUp { kind, mod_id },
            AtelierRefusal::MaxGrade,
        );
    }
    Ok(())
}

#[test]
fn modifier_grade_refusals_preserve_gold_fragments_revision_and_gems() -> Result<(), Error> {
    let mut before = state()?;
    let action = AtelierAction::GradeUp {
        kind: FragmentKind::Basic,
        mod_id: 31,
    };
    before.gold = 1_999_999;
    refused(&before, action.clone(), AtelierRefusal::InsufficientGold);
    before.gold = 2_000_000;
    before.fragments.insert(FragmentKind::Basic, 4);
    refused(
        &before,
        action.clone(),
        AtelierRefusal::InsufficientFragments,
    );
    before.fragments.remove(&FragmentKind::Basic);
    refused(
        &before,
        action.clone(),
        AtelierRefusal::InsufficientFragments,
    );
    before.fragments.insert(FragmentKind::Basic, 5);
    before.grades.insert((FragmentKind::Basic, 31), 1);
    before.grade_ruleset_revision = Some("old-ruleset".into());
    refused(&before, action, AtelierRefusal::RulesetMigrationPending);
    before.grades.clear();
    for (kind, mod_id) in [(FragmentKind::Basic, u16::MAX), (FragmentKind::Supreme, 42)] {
        refused(
            &before,
            AtelierAction::GradeUp { kind, mod_id },
            AtelierRefusal::InvalidDraw,
        );
    }
    Ok(())
}

#[test]
fn rotation_prices_and_clockwise_domains_match_crystal() -> Result<(), Error> {
    // Crystal tools.cpp:2110-2125; configmanager.cpp:438-440.
    for (quality, fee) in [
        (GemQuality::Lesser, 125_000),
        (GemQuality::Regular, 250_000),
        (GemQuality::Greater, 500_000),
    ] {
        let mut state = state()?;
        state.gems.get_mut(&1).ok_or("gem")?.quality = quality;
        for domain in [
            WheelDomain::Red,
            WheelDomain::Purple,
            WheelDomain::Blue,
            WheelDomain::Green,
        ] {
            let gold = state.gold;
            state
                .commit(state.revision, AtelierAction::SwitchDomain { id: 1 })
                .map_err(|error| format!("rotation refused: {error:?}"))?;
            assert_eq!(state.gems.get(&1).ok_or("gem")?.domain, domain);
            assert_eq!(state.gold, gold - fee);
        }
        state.gold = fee - 1;
        refused(
            &state,
            AtelierAction::SwitchDomain { id: 1 },
            AtelierRefusal::InsufficientGold,
        );
    }
    Ok(())
}

#[test]
fn native_vessel_and_lock_guards_remain_atomic() -> Result<(), Error> {
    // Locked protection: Crystal player_wheel.cpp:1299-1302,1359-1362.
    // In-vessel/last-domain refusals: accepted WHEEL-GEM-0 sections7,15 R2.
    let mut state = state()?;
    state
        .commit(0, AtelierAction::Socket { id: 1 })
        .map_err(|error| format!("socket refused: {error:?}"))?;
    refused(
        &state,
        AtelierAction::Socket { id: 1 },
        AtelierRefusal::AlreadySocketed,
    );
    refused(
        &state,
        AtelierAction::Socket { id: 2 },
        AtelierRefusal::VesselOccupied,
    );
    refused(
        &state,
        AtelierAction::SwitchDomain { id: 1 },
        AtelierRefusal::InVessel,
    );
    refused(
        &state,
        AtelierAction::Dismantle {
            id: 1,
            yield_amount: 1,
        },
        AtelierRefusal::InVessel,
    );
    state
        .commit(1, AtelierAction::Remove { id: 1 })
        .map_err(|error| format!("remove refused: {error:?}"))?;
    refused(
        &state,
        AtelierAction::Remove { id: 1 },
        AtelierRefusal::NotSocketed,
    );
    state
        .commit(
            2,
            AtelierAction::Lock {
                id: 1,
                locked: true,
            },
        )
        .map_err(|error| format!("lock refused: {error:?}"))?;
    refused(
        &state,
        AtelierAction::SwitchDomain { id: 1 },
        AtelierRefusal::Locked,
    );
    refused(
        &state,
        AtelierAction::Dismantle {
            id: 1,
            yield_amount: 1,
        },
        AtelierRefusal::Locked,
    );
    state
        .commit(
            3,
            AtelierAction::Lock {
                id: 1,
                locked: false,
            },
        )
        .map_err(|error| format!("unlock refused: {error:?}"))?;
    state.gems.remove(&2);
    refused(
        &state,
        AtelierAction::Dismantle {
            id: 1,
            yield_amount: 1,
        },
        AtelierRefusal::LastDomainGem,
    );
    Ok(())
}

#[test]
fn missing_or_non_current_gems_and_stale_revision_never_mutate() -> Result<(), Error> {
    let before = state()?;
    let mut next = before.clone();
    assert_eq!(
        next.commit(
            1,
            AtelierAction::Lock {
                id: 1,
                locked: true
            }
        ),
        Err(AtelierRefusal::StaleRevision)
    );
    assert_eq!(next, before);
    for action in [
        AtelierAction::Socket { id: 100 },
        AtelierAction::Remove { id: 100 },
        AtelierAction::SwitchDomain { id: 100 },
        AtelierAction::Lock {
            id: 100,
            locked: true,
        },
        AtelierAction::Dismantle {
            id: 100,
            yield_amount: 1,
        },
    ] {
        refused(&before, action, AtelierRefusal::NoGem);
    }
    next.gems.get_mut(&1).ok_or("gem")?.ruleset_revision = "old-ruleset".into();
    for action in [
        AtelierAction::Socket { id: 1 },
        AtelierAction::Remove { id: 1 },
        AtelierAction::SwitchDomain { id: 1 },
        AtelierAction::Lock {
            id: 1,
            locked: true,
        },
        AtelierAction::Dismantle {
            id: 1,
            yield_amount: 1,
        },
    ] {
        refused(&next, action, AtelierRefusal::RulesetMigrationPending);
    }
    Ok(())
}

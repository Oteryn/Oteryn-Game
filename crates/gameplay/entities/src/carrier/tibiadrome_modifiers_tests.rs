use super::*;
use DromeModifier::*;
type Error = Box<dyn std::error::Error + Send + Sync>;
fn actor(value: u32) -> Result<ExactActorRef, Error> {
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    Ok(ExactActorRef(ActorRef::from_scoped_owner_parts(
        RuntimeScopeRefV1::channel(WorldId::decode(&id)?, ChannelId::decode(&id)?),
        ScopeOwnershipGeneration::new(1)?,
        ActorLocalId(value),
        ActorLocalGeneration(1),
    )))
}
fn at(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 6 }
}
fn state(first: DromeModifier) -> Result<DromeModifierState, Error> {
    DromeModifierState::new(
        [
            first,
            if first == Somersault {
                BadRoots
            } else {
                Somersault
            },
        ],
        [1; 16],
        1,
        (0..20)
            .flat_map(|x| (0..20).map(move |y| at(x, y)))
            .collect(),
        0,
    )
    .map_err(|error| format!("{error:?}").into())
}
#[test]
fn srx3_drome_modifier_escalation_is_strictly_below_quarter() -> Result<(), Error> {
    let state = state(ThatEscalatedQuickly)?;
    assert_eq!(state.wave_bonus(25, 100), 0);
    assert_eq!(state.wave_bonus(24, 100), 5);
    Ok(())
}
#[test]
fn srx3_drome_modifier_death_centres_last_hitter_and_corpse_separately() -> Result<(), Error> {
    let players = [
        DromeParticipant {
            actor: actor(1)?,
            position: at(1, 1),
        },
        DromeParticipant {
            actor: actor(2)?,
            position: at(9, 9),
        },
    ];
    assert_eq!(
        state(GoingDownWithMe)?.death(at(9, 9), Some(players[0].actor), &players),
        vec![DromeModifierEffect::CurrentHealthDamage {
            target: players[0].actor,
            percent: 40
        }]
    );
    assert_eq!(
        state(ExplodingCorpses)?.death(at(9, 9), Some(players[0].actor), &players),
        vec![DromeModifierEffect::CurrentHealthDamage {
            target: players[1].actor,
            percent: 25
        }]
    );
    Ok(())
}
#[test]
fn srx3_drome_modifier_somersault_fifteen_percent_and_furthest_target() -> Result<(), Error> {
    let source = actor(3)?;
    let players = [
        DromeParticipant {
            actor: actor(1)?,
            position: at(2, 2),
        },
        DromeParticipant {
            actor: actor(2)?,
            position: at(9, 9),
        },
    ];
    let state = state(Somersault)?;
    assert!(
        state
            .melee(source, at(0, 0), 15, &players, |_| true)
            .is_empty()
    );
    let effects = state.melee(source, at(0, 0), 14, &players, |_| true);
    assert!(
        matches!(effects.as_slice(), [DromeModifierEffect::Teleport { actor, destination }] if *actor == source && destination.x.abs_diff(9)<=1 && destination.y.abs_diff(9)<=1 && *destination != at(9,9))
    );
    Ok(())
}
#[test]
fn srx3_drome_modifier_three_tile_hazards_mark_100_then_resolve_after_three_seconds()
-> Result<(), Error> {
    for modifier in [TheFloorIsLava, BeamMeUp, TankedUp] {
        let mut state = state(modifier)?;
        assert!(state.tick(0, 14_999_999, &[], |_| true).is_empty());
        let effects = state.tick(0, 15_000_000, &[], |_| true);
        assert!(
            matches!(effects.as_slice(), [DromeModifierEffect::Mark { kind, positions, resolves_at }] if *kind == modifier && positions.len()==100 && *resolves_at==18_000_000)
        );
        let mark = match &effects[0] {
            DromeModifierEffect::Mark { positions, .. } => positions[0],
            _ => return Err("missing mark".into()),
        };
        let players = [DromeParticipant {
            actor: actor(1)?,
            position: mark,
        }];
        assert!(state.tick(0, 17_999_999, &players, |_| true).is_empty());
        let effects = state.tick(0, 18_000_000, &players, |_| true);
        assert_eq!(effects.len(), 1);
        match modifier {
            TheFloorIsLava => assert_eq!(
                effects[0],
                DromeModifierEffect::CurrentHealthDamage {
                    target: players[0].actor,
                    percent: 60
                }
            ),
            BeamMeUp => assert!(
                matches!(effects[0], DromeModifierEffect::Teleport { actor, .. } if actor == players[0].actor)
            ),
            TankedUp => assert!(
                matches!(effects[0], DromeModifierEffect::Status { target, duration_ms:10000, incoming_percent:110, .. } if target==players[0].actor)
            ),
            _ => unreachable!(),
        }
    }
    Ok(())
}
#[test]
fn srx3_drome_modifier_seed_step_cancels_and_expiry_fears_or_roots() -> Result<(), Error> {
    for modifier in [SownSorrow, BadRoots] {
        let mut state = state(modifier)?;
        let effects = state.tick(0, 20_000_000, &[], |_| true);
        let seed = match effects.as_slice() {
            [
                DromeModifierEffect::Seed {
                    position,
                    resolves_at: 26_000_000,
                    ..
                },
            ] => *position,
            _ => return Err("source seed missing".into()),
        };
        let player = DromeParticipant {
            actor: actor(1)?,
            position: seed,
        };
        assert!(state.tick(0, 25_999_999, &[player], |_| true).is_empty());
        assert!(state.tick(0, 26_000_000, &[player], |_| true).is_empty());
        let mut state = super::tests::state(modifier)?;
        state.tick(0, 20_000_000, &[], |_| true);
        let away = DromeParticipant {
            position: at(100, 100),
            ..player
        };
        let effects = state.tick(0, 26_000_000, &[away], |_| true);
        assert!(
            matches!(effects.as_slice(), [DromeModifierEffect::Status { target, duration_ms, incoming_percent, .. }] if *target==player.actor && (*duration_ms,*incoming_percent)==if modifier==SownSorrow {(2000,115)} else {(3000,125)})
        );
    }
    Ok(())
}

#[test]
fn srx3_drome_modifier_rotation_selects_two_distinct_from_all_nine() {
    let mut seen = std::collections::HashSet::new();
    for rotation in 0..100 {
        let pair = drome_rotation_modifiers(rotation);
        assert_ne!(pair[0], pair[1]);
        assert_eq!(pair, drome_rotation_modifiers(rotation));
        seen.extend(pair);
    }
    assert_eq!(seen.len(), 9);
}

#[test]
fn srx3_drome_familiar_death_blast_centres_actual_hitter_and_hits_its_hp() -> Result<(), Error> {
    let familiar = DromeParticipant {
        actor: actor(3)?,
        position: at(1, 1),
    };
    let nearby = DromeParticipant {
        actor: actor(1)?,
        position: at(1, 2),
    };
    let master = DromeParticipant {
        actor: actor(2)?,
        position: at(9, 9),
    };
    let effects = state(GoingDownWithMe)?.death_at(at(9, 9), Some(familiar), &[nearby, master]);
    assert_eq!(effects.len(), 2);
    assert!(effects.contains(&DromeModifierEffect::CurrentHealthDamage {
        target: familiar.actor,
        percent: 40
    }));
    assert!(effects.contains(&DromeModifierEffect::CurrentHealthDamage {
        target: nearby.actor,
        percent: 40
    }));
    assert!(
        !effects.contains(&DromeModifierEffect::CurrentHealthDamage {
            target: master.actor,
            percent: 40
        })
    );
    Ok(())
}

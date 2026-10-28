//! Native dispatch boundary for Gem Atelier/Fragment Workshop commands.

pub use oteryn_gameplay_progression::atelier::{
    AtelierAction, AtelierCommit, AtelierRefusal, AtelierState,
};

/// In-memory action boundary. Production admission, fenced item/fee writes, durable receipts
/// and wire routing must be composed before enabling Atelier commands for players.
pub fn dispatch_atelier(
    state: &mut AtelierState,
    expected_revision: u64,
    action: AtelierAction,
) -> Result<AtelierCommit, AtelierRefusal> {
    state.commit(expected_revision, action)
}

/// Server commands on the admitted Character owner. Wire allocation remains a separate task.
pub enum GemBuildCommand<'a> {
    Atelier(crate::durability::character_atelier::AtelierRequest<'a>),
    Wheel(crate::durability::character_wheel::WheelChangeRequest),
}
#[derive(Debug)]
pub enum GemBuildOutcome {
    Atelier(crate::durability::character_atelier::AtelierOutcome),
    Wheel(Box<crate::durability::character_wheel::WheelCommitOutcome>),
}
impl super::ComposedFreshAdmission<'_, '_, '_> {
    /// Runtime Gem Atelier entry point.
    ///
    /// All player-facing Atelier and vessel commands must enter here rather than calling a
    /// durability writer directly. The entry resolves the current session fence, serializes the
    /// Character revision, and refreshes both projections only after the atomic writer returns.
    pub async fn execute_gem_atelier(
        &self,
        session: crate::foundation::GameSessionId,
        command: GemBuildCommand<'_>,
    ) -> Result<GemBuildOutcome, crate::durability::character_progression::CharacterProgressionError>
    {
        self.commit_gem_build(session, command).await
    }

    async fn commit_gem_build(
        &self,
        session: crate::foundation::GameSessionId,
        command: GemBuildCommand<'_>,
    ) -> Result<GemBuildOutcome, crate::durability::character_progression::CharacterProgressionError>
    {
        use crate::durability::character_progression::CharacterProgressionError as Error;
        let ruleset = self.wheel_ruleset.as_ref().ok_or(Error::InvalidInput)?;
        let gate = self.premium_coordinator.ok_or(Error::InvalidInput)?;
        let fence = self
            .current_quest_fence(session)
            .await
            .map_err(|_| Error::AuthorityRejected)?
            .ok_or(Error::AuthorityRejected)?;
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        // A failed or unknown commit must not leave a usable stale actor projection.
        self.states
            .wheel_sessions
            .lock()
            .map_err(|_| Error::InvalidInput)?
            .insert(session, None);
        self.states
            .atelier_sessions
            .lock()
            .map_err(|_| Error::InvalidInput)?
            .insert(session, None);
        let outcome = match command {
            GemBuildCommand::Atelier(request) => GemBuildOutcome::Atelier(
                slot.atelier(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    request,
                    std::sync::Arc::clone(ruleset),
                    gate,
                )
                .await?,
            ),
            GemBuildCommand::Wheel(request) => GemBuildOutcome::Wheel(Box::new(
                slot.commit_wheel(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    request,
                    std::sync::Arc::clone(ruleset),
                    crate::durability::character_wheel::WheelChangeFacts { at_temple: false },
                    gate,
                )
                .await?,
            )),
        };
        // Reload current committed rows, including on receipt replay after a lost acknowledgement.
        let allocation = self
            .root
            .read_character_wheel(
                self.character,
                fence.character_id,
                std::sync::Arc::clone(ruleset),
            )
            .await?;
        let atelier = self
            .root
            .read_character_atelier(self.character, fence.character_id)
            .await?;
        self.states
            .atelier_sessions
            .lock()
            .map_err(|_| Error::InvalidInput)?
            .insert(session, atelier);
        self.states
            .wheel_sessions
            .lock()
            .map_err(|_| Error::InvalidInput)?
            .insert(session, Some(allocation));
        Ok(outcome)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test fixtures")]
mod tests {
    use super::*;
    use oteryn_gameplay_progression::{
        atelier::{FragmentKind, GemQuality},
        wheel::WheelDomain,
    };

    fn funded_state() -> AtelierState {
        AtelierState {
            vocation: "elite_knight".to_owned(),
            gold: 100_000_000,
            ..AtelierState::default()
        }
    }

    fn apply(state: &mut AtelierState, action: AtelierAction) -> AtelierCommit {
        dispatch_atelier(state, state.revision, action).unwrap()
    }

    fn reveal(state: &mut AtelierState, id: u128, quality: GemQuality, mods: Vec<u16>) {
        apply(state, AtelierAction::Drop { quality });
        apply(
            state,
            AtelierAction::Reveal {
                id,
                quality,
                domain: WheelDomain::Green,
                mods,
            },
        );
    }

    fn refused(state: &mut AtelierState, action: AtelierAction, expected: AtelierRefusal) {
        let before = state.clone();
        assert_eq!(
            dispatch_atelier(state, state.revision, action),
            Err(expected)
        );
        assert_eq!(
            *state, before,
            "refusals must preserve every balance and revision"
        );
    }

    #[test]
    fn atelier_reveal_checks_vocation_positions_compatibility_and_exact_fees() {
        let mut state = funded_state();
        for (id, quality, mods, fee) in [
            (1, GemQuality::Lesser, vec![3], 125_000),
            (2, GemQuality::Regular, vec![3, 1], 1_000_000),
            (3, GemQuality::Greater, vec![3, 1, 0], 6_000_000),
        ] {
            let gold = state.gold;
            reveal(&mut state, id, quality, mods);
            assert_eq!(state.gold, gold - fee);
            assert_eq!(state.unrevealed[&quality], 0);
        }
        apply(
            &mut state,
            AtelierAction::Drop {
                quality: GemQuality::Greater,
            },
        );
        for mods in [
            vec![0, 1, 0],
            vec![3, 3, 0],
            vec![3, 30, 0],
            vec![3, 1, 93],
            vec![3],
        ] {
            refused(
                &mut state,
                AtelierAction::Reveal {
                    id: 4,
                    quality: GemQuality::Greater,
                    domain: WheelDomain::Green,
                    mods,
                },
                AtelierRefusal::InvalidDraw,
            );
        }
        state.gold = 5_999_999;
        refused(
            &mut state,
            AtelierAction::Reveal {
                id: 4,
                quality: GemQuality::Greater,
                domain: WheelDomain::Green,
                mods: vec![3, 1, 0],
            },
            AtelierRefusal::InsufficientGold,
        );
    }

    #[test]
    fn atelier_one_socket_per_domain_and_remove_releases_the_vessel() {
        let mut state = funded_state();
        reveal(&mut state, 1, GemQuality::Lesser, vec![3]);
        reveal(&mut state, 2, GemQuality::Lesser, vec![3]);
        apply(&mut state, AtelierAction::Socket { id: 1 });
        refused(
            &mut state,
            AtelierAction::Socket { id: 2 },
            AtelierRefusal::VesselOccupied,
        );
        refused(
            &mut state,
            AtelierAction::SwitchDomain { id: 1 },
            AtelierRefusal::InVessel,
        );
        apply(&mut state, AtelierAction::Remove { id: 1 });
        apply(&mut state, AtelierAction::Socket { id: 2 });
        assert!(!state.gems[&1].vessel);
        assert!(state.gems[&2].vessel);
    }

    #[test]
    fn atelier_switch_lock_dismantle_ranges_and_last_domain_guards() {
        let mut state = funded_state();
        reveal(&mut state, 1, GemQuality::Regular, vec![3, 1]);
        refused(
            &mut state,
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 2,
            },
            AtelierRefusal::LastDomainGem,
        );
        apply(
            &mut state,
            AtelierAction::Lock {
                id: 1,
                locked: true,
            },
        );
        refused(
            &mut state,
            AtelierAction::SwitchDomain { id: 1 },
            AtelierRefusal::Locked,
        );
        refused(
            &mut state,
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 2,
            },
            AtelierRefusal::Locked,
        );
        apply(
            &mut state,
            AtelierAction::Lock {
                id: 1,
                locked: false,
            },
        );
        let before = state.gold;
        for domain in [
            WheelDomain::Red,
            WheelDomain::Purple,
            WheelDomain::Blue,
            WheelDomain::Green,
        ] {
            apply(&mut state, AtelierAction::SwitchDomain { id: 1 });
            assert_eq!(state.gems[&1].domain, domain);
        }
        assert_eq!(state.gold, before - 4 * 250_000);
        reveal(&mut state, 2, GemQuality::Lesser, vec![3]);
        for yield_amount in [1, 6] {
            refused(
                &mut state,
                AtelierAction::Dismantle {
                    id: 1,
                    yield_amount,
                },
                AtelierRefusal::InvalidDraw,
            );
        }
        apply(
            &mut state,
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 5,
            },
        );
        assert_eq!(state.fragments[&FragmentKind::Basic], 5);
        assert!(!state.gems.contains_key(&1));
    }

    #[test]
    fn atelier_grade_catalogue_costs_revisions_and_json_reload() {
        let mut state = funded_state();
        reveal(&mut state, u128::MAX, GemQuality::Regular, vec![3, 1]);
        for (kind, mod_id) in [(FragmentKind::Basic, 32), (FragmentKind::Supreme, 93)] {
            refused(
                &mut state,
                AtelierAction::GradeUp { kind, mod_id },
                AtelierRefusal::InvalidDraw,
            );
        }
        refused(
            &mut state,
            AtelierAction::GradeUp {
                kind: FragmentKind::Basic,
                mod_id: 3,
            },
            AtelierRefusal::InsufficientFragments,
        );
        state.fragments.insert(FragmentKind::Basic, 50);
        for (gold, fragments, points) in
            [(2_000_000, 5, 0), (5_000_000, 15, 0), (30_000_000, 30, 1)]
        {
            let before = state.clone();
            let outcome = apply(
                &mut state,
                AtelierAction::GradeUp {
                    kind: FragmentKind::Basic,
                    mod_id: 3,
                },
            );
            assert_eq!(state.gold, before.gold - gold);
            assert_eq!(
                state.fragments[&FragmentKind::Basic],
                before.fragments[&FragmentKind::Basic] - fragments
            );
            assert_eq!(outcome.promotion_points, points);
            let stored = serde_json::to_value(&state).unwrap();
            assert_eq!(
                serde_json::from_value::<AtelierState>(stored).unwrap(),
                state
            );
        }
        refused(
            &mut state,
            AtelierAction::GradeUp {
                kind: FragmentKind::Basic,
                mod_id: 3,
            },
            AtelierRefusal::MaxGrade,
        );
        state.grade_ruleset_revision = Some("old-ruleset".to_owned());
        refused(
            &mut state,
            AtelierAction::GradeUp {
                kind: FragmentKind::Basic,
                mod_id: 3,
            },
            AtelierRefusal::RulesetMigrationPending,
        );
        assert_eq!(
            apply(
                &mut state,
                AtelierAction::Drop {
                    quality: GemQuality::Lesser
                }
            )
            .promotion_points,
            0
        );
    }

    #[test]
    fn atelier_overflow_stale_revision_and_noncurrent_gems_preserve_state() {
        let mut state = funded_state();
        reveal(&mut state, 1, GemQuality::Lesser, vec![3]);
        let before = state.clone();
        assert_eq!(
            dispatch_atelier(&mut state, 0, AtelierAction::Socket { id: 1 }),
            Err(AtelierRefusal::StaleRevision)
        );
        assert_eq!(state, before);
        state.gems.get_mut(&1).unwrap().ruleset_revision = "old-ruleset".to_owned();
        for action in [
            AtelierAction::Socket { id: 1 },
            AtelierAction::Lock {
                id: 1,
                locked: true,
            },
            AtelierAction::Remove { id: 1 },
            AtelierAction::SwitchDomain { id: 1 },
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 1,
            },
        ] {
            refused(&mut state, action, AtelierRefusal::RulesetMigrationPending);
        }
        state.revision = u64::MAX;
        refused(
            &mut state,
            AtelierAction::Drop {
                quality: GemQuality::Lesser,
            },
            AtelierRefusal::InvalidDraw,
        );
        state.revision = 0;
        state.unrevealed.insert(GemQuality::Lesser, u32::MAX);
        refused(
            &mut state,
            AtelierAction::Drop {
                quality: GemQuality::Lesser,
            },
            AtelierRefusal::InvalidDraw,
        );
    }

    #[test]
    fn atelier_storage_cap_fragment_overflow_and_invalid_grade_json() {
        let mut state = funded_state();
        reveal(&mut state, 1, GemQuality::Lesser, vec![3]);
        let template = state.gems[&1].clone();
        for id in 2..=225 {
            let mut gem = template.clone();
            gem.id = id;
            state.gems.insert(id, gem);
        }
        apply(
            &mut state,
            AtelierAction::Drop {
                quality: GemQuality::Lesser,
            },
        );
        refused(
            &mut state,
            AtelierAction::Reveal {
                id: 226,
                quality: GemQuality::Lesser,
                domain: WheelDomain::Green,
                mods: vec![3],
            },
            AtelierRefusal::Full,
        );
        state.fragments.insert(FragmentKind::Basic, u32::MAX);
        refused(
            &mut state,
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 1,
            },
            AtelierRefusal::InvalidDraw,
        );
        let mut json = serde_json::to_value(&state).unwrap();
        json["grades"] = serde_json::json!([[["Basic", 3], 4]]);
        assert!(serde_json::from_value::<AtelierState>(json.clone()).is_err());
        json["grades"] = serde_json::json!([[["Basic", 3], 1], [["Basic", 3], 2]]);
        assert!(serde_json::from_value::<AtelierState>(json).is_err());
    }

    #[test]
    fn real_dispatch_reveals_sockets_dismantles_and_grades_without_partial_refusals() {
        let mut state = AtelierState {
            gold: 100_000_000,
            vocation: "elite_knight".to_owned(),
            ..AtelierState::default()
        };
        dispatch_atelier(
            &mut state,
            0,
            AtelierAction::Drop {
                quality: GemQuality::Greater,
            },
        )
        .unwrap();
        dispatch_atelier(
            &mut state,
            1,
            AtelierAction::Reveal {
                id: 1,
                quality: GemQuality::Greater,
                domain: WheelDomain::Green,
                mods: vec![3, 1, 0],
            },
        )
        .unwrap();
        dispatch_atelier(&mut state, 2, AtelierAction::Socket { id: 1 }).unwrap();
        let before = state.clone();
        assert_eq!(
            dispatch_atelier(
                &mut state,
                3,
                AtelierAction::Dismantle {
                    id: 1,
                    yield_amount: 3
                }
            ),
            Err(AtelierRefusal::InVessel)
        );
        assert_eq!(state, before);
        dispatch_atelier(&mut state, 3, AtelierAction::Remove { id: 1 }).unwrap();
        dispatch_atelier(
            &mut state,
            4,
            AtelierAction::Drop {
                quality: GemQuality::Lesser,
            },
        )
        .unwrap();
        dispatch_atelier(
            &mut state,
            5,
            AtelierAction::Reveal {
                id: 2,
                quality: GemQuality::Lesser,
                domain: WheelDomain::Green,
                mods: vec![3],
            },
        )
        .unwrap();
        dispatch_atelier(
            &mut state,
            6,
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 3,
            },
        )
        .unwrap();
        state.fragments.insert(FragmentKind::Basic, 50);
        for expected in 7..10 {
            dispatch_atelier(
                &mut state,
                expected,
                AtelierAction::GradeUp {
                    kind: FragmentKind::Basic,
                    mod_id: 0,
                },
            )
            .unwrap();
        }
        assert_eq!(state.grades[&(FragmentKind::Basic, 0)], 3);
    }
}

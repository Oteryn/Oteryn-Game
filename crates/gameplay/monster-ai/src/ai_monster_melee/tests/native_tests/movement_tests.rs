//! Actual position-owner commits wake extra melee without a full Think or a second hit.
use super::*;

impl NativeFixture {
    fn step(
        &mut self,
        actor: ExactActorRef,
        destination: MovementLocalPosition,
    ) -> Result<(), String> {
        let before = self
            .runtime
            .read_actor_position(actor)
            .map_err(|error| format!("{error:?}"))?;
        self.runtime
            .borrow_movement_position()
            .commit_cardinal(before, destination)
            .map_err(|error| format!("{error:?}"))?;
        Ok(())
    }

    fn retry_movement(&mut self, now_us: u64) -> Result<Vec<ExactActorRef>, String> {
        let position = self
            .runtime
            .read_actor_position(self.target)
            .map_err(|error| format!("{error:?}"))?
            .position();
        let players = [TargetCandidate {
            actor: self.target,
            position,
            eligible: true,
            invisible: false,
            health: u64::from(self.vitals.health),
            damage: 0,
        }];
        Ok(self
            .owner
            .creature_ai
            .prepare_movement_melee_retries(&self.runtime, &players, now_us))
    }

    fn due_miss(&mut self) -> Result<(), String> {
        self.step(self.target, position(2))?;
        self.think(0, true)?;
        assert_eq!(
            self.swing(0, 1, 0)?,
            Dispatch::Decision(ThinkOutcome::ChaseStep(crate::movement::CardinalStep::East,))
        );
        assert_eq!((self.vitals.health, self.vitals.revision), (100, 0));
        assert!(
            !self
                .owner
                .creature_ai
                .has_movement_melee_retry(&self.runtime)
        );
        Ok(())
    }
}

#[test]
fn target_or_issuer_translation_retries_pending_extra_before_next_think() -> Result<(), String> {
    for move_issuer in [false, true] {
        let mut fixture = NativeFixture::new(1000, 1, 8)?;
        fixture.due_miss()?;
        let due = fixture.owner.creature_ai.due(fixture.issuer);
        let before = fixture
            .owner
            .creature_ai
            .native_melee_selection(fixture.issuer, 0)
            .ok_or("native selection before movement")?;
        let moved = if move_issuer {
            fixture.issuer
        } else {
            fixture.target
        };
        fixture.step(moved, position(1))?;
        assert!(
            fixture
                .owner
                .creature_ai
                .has_movement_melee_retry(&fixture.runtime)
        );
        assert_eq!(fixture.retry_movement(100_000)?, [fixture.issuer]);
        let selected = fixture
            .owner
            .creature_ai
            .native_melee_selection(fixture.issuer, 0)
            .ok_or("native movement selection")?;
        assert_eq!(selected.occurrence.sequence, before.occurrence.sequence + 1);
        assert_eq!(selected.target, Some(fixture.target));
        assert_eq!(fixture.owner.creature_ai.due(fixture.issuer), due);
        let dispatch = fixture.swing(0, 987, 100_000)?;
        assert!(matches!(dispatch, Dispatch::Bite(Ok(_))));
        assert_eq!((fixture.vitals.health, fixture.vitals.revision), (92, 1));
        assert!(
            !fixture
                .owner
                .creature_ai
                .has_movement_melee_retry(&fixture.runtime)
        );
        assert!(fixture.retry_movement(200_000)?.is_empty());
        assert_eq!(
            fixture
                .owner
                .creature_ai
                .native_melee_selection(fixture.issuer, 0),
            Some(selected)
        );
        assert_eq!(fixture.swing(0, 988, 200_000)?, dispatch);
        assert_eq!((fixture.vitals.health, fixture.vitals.revision), (92, 1));
        assert_eq!(fixture.owner.creature_ai.due(fixture.issuer), due);
    }
    Ok(())
}

#[test]
fn out_and_back_translation_is_observed_once_but_unrelated_motion_is_not() -> Result<(), String> {
    let mut fixture = NativeFixture::new(1000, 1, 8)?;
    fixture.due_miss()?;
    let unrelated = fixture
        .runtime
        .admit_test_creature(position(5))
        .map_err(|error| format!("{error:?}"))?;
    fixture.step(unrelated, position(6))?;
    assert!(
        !fixture
            .owner
            .creature_ai
            .has_movement_melee_retry(&fixture.runtime)
    );
    assert!(fixture.retry_movement(50_000)?.is_empty());
    let before = fixture
        .owner
        .creature_ai
        .native_melee_selection(fixture.issuer, 0)
        .ok_or("initial selection")?;
    fixture.step(fixture.target, position(3))?;
    fixture.step(fixture.target, position(2))?;
    assert!(
        fixture
            .owner
            .creature_ai
            .has_movement_melee_retry(&fixture.runtime)
    );
    assert!(
        fixture.retry_movement(100_000)?.is_empty(),
        "still out of melee reach"
    );
    let after = fixture
        .owner
        .creature_ai
        .native_melee_selection(fixture.issuer, 0)
        .ok_or("one consumed movement event")?;
    assert_eq!(after.occurrence.sequence, before.occurrence.sequence + 1);
    assert_eq!(after.target, None);
    assert!(
        !fixture
            .owner
            .creature_ai
            .has_movement_melee_retry(&fixture.runtime)
    );
    assert!(
        fixture.retry_movement(900_000)?.is_empty(),
        "poll time cannot retry the same movement"
    );
    assert_eq!(
        fixture
            .owner
            .creature_ai
            .native_melee_selection(fixture.issuer, 0),
        Some(after)
    );
    assert_eq!((fixture.vitals.health, fixture.vitals.revision), (100, 0));
    Ok(())
}

#[test]
fn movement_does_not_grant_extra_before_the_source_interval_becomes_due() -> Result<(), String> {
    let mut fixture = NativeFixture::new(4000, 1, 8)?;
    fixture.step(fixture.target, position(2))?;
    fixture.think(0, true)?;
    let before = fixture
        .owner
        .creature_ai
        .native_melee_selection(fixture.issuer, 0);
    fixture.step(fixture.target, position(1))?;
    assert!(
        !fixture
            .owner
            .creature_ai
            .has_movement_melee_retry(&fixture.runtime)
    );
    assert!(fixture.retry_movement(100_000)?.is_empty());
    assert_eq!(
        fixture
            .owner
            .creature_ai
            .native_melee_selection(fixture.issuer, 0),
        before
    );
    assert_eq!(
        fixture.swing(0, 999, 100_000)?,
        Dispatch::Decision(ThinkOutcome::Idle)
    );
    assert_eq!(fixture.vitals.health, 100);
    Ok(())
}

#[test]
fn turning_either_actor_in_place_never_wakes_a_pending_extra_swing() -> Result<(), String> {
    use crate::foundation::MovementFacing;
    let mut fixture = NativeFixture::new(1000, 1, 8)?;
    fixture.due_miss()?;
    let selection = fixture
        .owner
        .creature_ai
        .native_melee_selection(fixture.issuer, 0);
    for actor in [fixture.issuer, fixture.target] {
        for facing in [
            MovementFacing::East,
            MovementFacing::West,
            MovementFacing::North,
        ] {
            let before = fixture
                .runtime
                .read_actor_position(actor)
                .map_err(|error| format!("{error:?}"))?;
            let after = fixture
                .runtime
                .borrow_movement_position()
                .commit_turn(before, facing)
                .map_err(|error| format!("{error:?}"))?;
            assert_ne!(after.revision(), before.revision());
            assert_eq!(
                after.last_translation_revision(),
                before.last_translation_revision()
            );
            assert!(
                !fixture
                    .owner
                    .creature_ai
                    .has_movement_melee_retry(&fixture.runtime)
            );
            assert!(fixture.retry_movement(100_000)?.is_empty());
        }
    }
    assert_eq!(
        fixture
            .owner
            .creature_ai
            .native_melee_selection(fixture.issuer, 0),
        selection
    );
    assert_eq!((fixture.vitals.health, fixture.vitals.revision), (100, 0));
    Ok(())
}

#[test]
fn a_foreign_position_context_on_either_actor_cannot_wake_native_melee() -> Result<(), String> {
    for (issuer_pinned, target_pinned) in [(false, true), (true, false), (false, false)] {
        let mut fixture =
            NativeFixture::with_position_context(1000, 1, 8, issuer_pinned, target_pinned)?;
        fixture.due_miss()?;
        let before = fixture
            .owner
            .creature_ai
            .native_melee_selection(fixture.issuer, 0);
        fixture.step(fixture.target, position(1))?;
        assert!(
            !fixture
                .owner
                .creature_ai
                .has_movement_melee_retry(&fixture.runtime)
        );
        assert!(fixture.retry_movement(100_000)?.is_empty());
        assert_eq!(
            fixture
                .owner
                .creature_ai
                .native_melee_selection(fixture.issuer, 0),
            before
        );
        assert_eq!((fixture.vitals.health, fixture.vitals.revision), (100, 0));
    }
    Ok(())
}

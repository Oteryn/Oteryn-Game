use super::*;
use crate::bestiary_postgres_harness::{debug, fence, id};
use crate::content::qualify_native_entry_room;
use crate::foundation::{CommandId, ScopeOwnershipGeneration, WorldId, crystal_timer_fixture};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn setup()
-> Result<(TravelHoldSlot, TravelCapture, ScopeRuntimeFence, Instant), Box<dyn std::error::Error>> {
    let gameplay = fence(1)?;
    let capture = TravelCapture {
        session: crate::durability::item_transfer::CurrentCharacterItemFence {
            character_id: gameplay.character_id,
            game_session_id: gameplay.game_session_id,
            connection_generation: gameplay.connection_generation,
            character_lease_generation: gameplay.character_lease_generation,
            runtime_scope: gameplay.runtime_scope,
            scope_ownership_generation: gameplay.scope_ownership_generation,
        },
        content: ReferenceContentGeneration::from_content(
            qualify_native_entry_room(WorldId::decode(&id(42)).map_err(debug)?)
                .map_err(debug)?
                .door(),
        )
        .map_err(debug)?,
        position_revision: 7,
    };
    let (owner, work) = crystal_timer_fixture(
        capture.session.runtime_scope,
        capture.session.scope_ownership_generation,
    )
    .map_err(debug)?;
    let frozen = FrozenTravel {
        command: CommandRef::new(
            capture.session.game_session_id,
            CommandId::new(1).map_err(debug)?,
        ),
        npc: "captain-test".into(),
        route: "test-route".into(),
        fare: 10,
        arrival: PendingArrival {
            occurrence: TravelOccurrence::from_bytes(id(70)).map_err(debug)?,
            world_id: WorldId::decode(&id(42)).map_err(debug)?,
            destination: crate::durability::character_last_position::LastPositionCell {
                x: 10,
                y: 20,
                floor: 7,
            },
        },
    };
    let now = Instant::now();
    let mut slot = TravelHoldSlot::default();
    slot.start(frozen, capture.clone(), work, &owner, now)
        .map_err(debug)?;
    Ok((slot, capture, owner, now))
}

fn occurrence() -> Result<TravelOccurrence, String> {
    TravelOccurrence::from_bytes(id(70)).map_err(debug)
}

#[test]
fn npc_travel_ambiguity_keeps_hold_and_fee_replay_never_relocates_twice() -> TestResult {
    let (mut slot, capture, owner, now) = setup()?;
    assert!(
        !slot
            .ambiguity_alert_due(now + Duration::from_millis(1999))
            .map_err(debug)?
    );
    assert!(
        slot.ambiguity_alert_due(now + Duration::from_millis(2000))
            .map_err(debug)?
    );
    assert_eq!(
        slot.fee_outcome(occurrence()?, FeeOutcome::Ambiguous),
        Ok(TravelStep::Wait)
    );
    assert!(slot.is_held());
    assert_eq!(
        slot.placement_decision(&capture, &owner, true, false),
        TravelStep::Wait
    );
    assert_eq!(
        slot.fee_outcome(occurrence()?, FeeOutcome::Committed),
        Ok(TravelStep::PlacementPending)
    );
    assert_eq!(
        slot.fee_outcome(occurrence()?, FeeOutcome::Committed),
        Ok(TravelStep::Wait)
    );
    assert_eq!(
        slot.placement_decision(&capture, &owner, true, false),
        TravelStep::PlacementPending
    );
    slot.placement_recorded(occurrence()?, now + Duration::from_secs(1))
        .map_err(debug)?;
    assert_eq!(
        slot.placement_decision(&capture, &owner, true, false),
        TravelStep::Wait
    );
    assert!(
        slot.ambiguity_alert_due(now + Duration::from_secs(3))
            .map_err(debug)?
    );
    assert_eq!(
        slot.consume_outcome(occurrence()?, ConsumeReplay::RetryConsume),
        Ok(TravelStep::RetryConsume)
    );
    assert!(slot.is_held());
    assert_eq!(
        slot.consume_outcome(occurrence()?, ConsumeReplay::Complete),
        Ok(TravelStep::Complete)
    );
    assert!(!slot.is_held());
    Ok(())
}

#[test]
fn npc_travel_one_hold_freezes_binding_and_old_occurrence_cannot_finish_it() -> TestResult {
    let (mut slot, capture, owner, now) = setup()?;
    let frozen = slot.frozen().ok_or("missing frozen travel")?.clone();
    let (_, work) = crystal_timer_fixture(
        capture.session.runtime_scope,
        capture.session.scope_ownership_generation,
    )
    .map_err(debug)?;
    let mut different = frozen.clone();
    different.fare += 1;
    assert_eq!(
        slot.start(different, capture, work, &owner, now),
        Err(HoldError::Busy)
    );
    assert_eq!(slot.frozen(), Some(&frozen));
    assert_eq!(
        slot.fee_outcome(
            TravelOccurrence::from_bytes(id(71)).map_err(debug)?,
            FeeOutcome::Aborted
        ),
        Err(HoldError::StaleCompletion)
    );
    assert!(slot.is_held());
    assert_eq!(
        slot.fee_outcome(occurrence()?, FeeOutcome::Aborted),
        Ok(TravelStep::Complete)
    );
    assert!(!slot.is_held());
    let (_, capture, owner, _) = setup()?;
    let (_, work) = crystal_timer_fixture(
        capture.session.runtime_scope,
        capture.session.scope_ownership_generation,
    )
    .map_err(debug)?;
    assert_eq!(
        slot.start(frozen.clone(), capture.clone(), work, &owner, now),
        Err(HoldError::StaleCompletion)
    );
    let mut next = frozen;
    next.arrival.occurrence = TravelOccurrence::from_bytes(id(72)).map_err(debug)?;
    next.command = CommandRef::new(
        capture.session.game_session_id,
        CommandId::new(2).map_err(debug)?,
    );
    slot.start(next, capture, work, &owner, now)
        .map_err(debug)?;
    assert_eq!(
        slot.fee_outcome(occurrence()?, FeeOutcome::Committed),
        Err(HoldError::StaleCompletion)
    );
    assert!(slot.is_held());
    Ok(())
}

#[test]
fn npc_travel_independent_current_owner_session_position_and_content_are_required() -> TestResult {
    let (mut slot, capture, mut owner, _) = setup()?;
    slot.fee_outcome(occurrence()?, FeeOutcome::Committed)
        .map_err(debug)?;
    let mut changes = Vec::new();
    let mut changed = capture.clone();
    changed.position_revision += 1;
    changes.push(changed);
    let mut changed = capture.clone();
    changed.session.character_lease_generation += 1;
    changes.push(changed);
    let mut changed = capture.clone();
    changed.session.character_id = crate::domain::CharacterId::from_bytes(id(99)).map_err(debug)?;
    changes.push(changed);
    let mut changed = capture.clone();
    changed.session.runtime_scope = RuntimeScopeRefV1::channel(
        WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(44)).map_err(debug)?,
    );
    changes.push(changed);
    let mut changed = capture.clone();
    changed.session.connection_generation =
        crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
    changes.push(changed);
    let mut changed = capture.clone();
    let mut content = qualify_native_entry_room(WorldId::decode(&id(42)).map_err(debug)?)
        .map_err(debug)?
        .door()
        .clone();
    content.content_lock.revision_digest_token =
        crate::content::ProductionAtom::new("test lock", "changed-lock").map_err(debug)?;
    changed.content = ReferenceContentGeneration::from_content(&content).map_err(debug)?;
    changes.push(changed);
    for changed in changes {
        assert_eq!(
            slot.placement_decision(&changed, &owner, true, false),
            TravelStep::LeaveForAdmission
        );
    }
    assert_eq!(
        slot.placement_decision(&capture, &owner, false, false),
        TravelStep::LeaveForAdmission
    );
    assert_eq!(
        slot.placement_decision(&capture, &owner, true, true),
        TravelStep::LeaveForAdmission
    );
    let (other_owner, _) = crystal_timer_fixture(
        RuntimeScopeRefV1::channel(
            WorldId::decode(&id(42)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(44)).map_err(debug)?,
        ),
        capture.session.scope_ownership_generation,
    )
    .map_err(debug)?;
    assert_eq!(
        slot.placement_decision(&capture, &other_owner, true, false),
        TravelStep::LeaveForAdmission
    );
    owner
        .apply_external_grant(ScopeOwnershipGeneration::new(2).map_err(debug)?)
        .map_err(debug)?;
    assert_eq!(
        slot.placement_decision(&capture, &owner, true, false),
        TravelStep::LeaveForAdmission
    );
    assert!(slot.is_held());
    Ok(())
}

#[test]
fn npc_travel_death_cancels_every_pending_phase_and_makes_late_results_inert() -> TestResult {
    for phase in 0..3 {
        let (mut slot, capture, owner, now) = setup()?;
        if phase > 0 {
            slot.fee_outcome(occurrence()?, FeeOutcome::Committed)
                .map_err(debug)?;
        }
        if phase > 1 {
            slot.placement_recorded(occurrence()?, now).map_err(debug)?;
        }
        slot.cancel_for_death();
        assert!(!slot.is_held());
        assert_eq!(
            slot.fee_outcome(occurrence()?, FeeOutcome::Committed),
            Ok(TravelStep::Complete)
        );
        assert_eq!(
            slot.consume_outcome(occurrence()?, ConsumeReplay::RetryConsume),
            Ok(TravelStep::Complete)
        );
        assert_eq!(
            slot.placement_decision(&capture, &owner, true, false),
            TravelStep::Wait
        );
        assert_eq!(
            slot.placement_recorded(occurrence()?, now),
            Err(HoldError::InvalidPhase)
        );
    }
    Ok(())
}

#[test]
fn npc_travel_invalid_phase_future_time_and_unbound_owner_fail_closed() -> TestResult {
    assert!(TravelOccurrence::from_bytes([0; 16]).is_err());
    let (mut slot, capture, owner, now) = setup()?;
    assert_eq!(
        slot.placement_recorded(occurrence()?, now),
        Err(HoldError::InvalidPhase)
    );
    assert_eq!(
        slot.consume_outcome(occurrence()?, ConsumeReplay::Complete),
        Err(HoldError::InvalidPhase)
    );
    assert_eq!(
        slot.ambiguity_alert_due(now - Duration::from_secs(1)),
        Err(HoldError::NonMonotonicTime)
    );
    let mut frozen = slot.frozen().ok_or("frozen")?.clone();
    frozen.command = CommandRef::new(
        crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?,
        CommandId::new(1).map_err(debug)?,
    );
    let (_, work) = crystal_timer_fixture(
        capture.session.runtime_scope,
        capture.session.scope_ownership_generation,
    )
    .map_err(debug)?;
    let mut empty = TravelHoldSlot::default();
    assert_eq!(
        empty.start(frozen, capture, work, &owner, now),
        Err(HoldError::InvalidBinding)
    );
    assert!(!empty.is_held());
    Ok(())
}

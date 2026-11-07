use oteryn_gameplay_world_change::binding::Binding;
use oteryn_gameplay_world_change::timed::{EventKind, EventState, HOUR, Input, Phase};

#[test]
fn stolen_gold_recovery_requires_the_native_bag_debit() {
    let binding = Binding {
        event: EventKind::BankRobbery,
        action: Input::Recover,
        minimum_level: 0,
        maximum_level: None,
    };
    assert!(binding.valid());
    assert_eq!(binding.cost(), Some((13429, 1)));
}

#[test]
fn recovery_reopens_the_bank_once_and_a_new_day_starts_dormant() -> Result<(), &'static str> {
    let mut state = EventState::for_kind(EventKind::BankRobbery, 0, 24 * HOUR)?;
    assert!(!state.advance(EventKind::BankRobbery, HOUR - 1, Input::Tick, 0)?);
    assert!(state.advance(EventKind::BankRobbery, HOUR, Input::Tick, 0)?);
    assert_eq!(state.phase, Phase::Active);
    assert!(state.advance(EventKind::BankRobbery, HOUR + 1, Input::Recover, 0)?);
    assert_eq!(state.phase, Phase::Victory);
    assert!(!state.advance(EventKind::BankRobbery, HOUR + 2, Input::Recover, 0)?);
    let next = EventState::for_kind(EventKind::BankRobbery, 24 * HOUR, 48 * HOUR)?;
    assert_eq!(next.phase, Phase::Dormant);
    assert_eq!(next.lifecycle.objects.len(), 1);
    Ok(())
}

use super::*;
use crate::content::newhaven_travel::NewhavenRoute;

#[test]
fn m38_travel_prompt_expires_at_the_deadline_including_a_saturated_clock() {
    for start in [100, u64::MAX - 10] {
        let mut prompt = TravelPrompt::new(NewhavenRoute::Anna, (1, 2, 3), start, false);
        let deadline = start.saturating_add(60_000);
        assert_eq!(
            prompt.confirm(&(1, 2, 3), deadline),
            TravelConfirmation::Refused
        );
    }
}

#[test]
fn m38_travel_prompt_retains_selected_route_and_cannot_depart_twice() {
    let mut prompt = TravelPrompt::new(NewhavenRoute::Seagull, (1, 2, 3), 100, true);
    assert_eq!(
        prompt.confirm(&(1, 2, 3), 200),
        TravelConfirmation::RepeatConfirmation
    );
    assert_eq!(
        prompt.confirm(&(1, 2, 3), 300),
        TravelConfirmation::Ready(NewhavenRoute::Seagull)
    );
    assert_eq!(prompt.confirm(&(1, 2, 3), 301), TravelConfirmation::Refused);
    let mut prompt = TravelPrompt::new(NewhavenRoute::Anna, (1, 2, 3), 100, false);
    assert_eq!(
        prompt.confirm(&(1, 2, 3), 200),
        TravelConfirmation::Ready(NewhavenRoute::Anna)
    );
}

#[test]
fn m38_travel_prompt_second_confirmation_does_not_renew_the_window() {
    let mut prompt = TravelPrompt::new(NewhavenRoute::Seagull, (1, 2, 3), 100, true);
    assert_eq!(
        prompt.confirm(&(1, 2, 3), 60_099),
        TravelConfirmation::RepeatConfirmation
    );
    assert_eq!(
        prompt.confirm(&(1, 2, 3), 60_100),
        TravelConfirmation::Refused
    );
}

#[test]
fn m38_travel_prompt_retires_after_npc_conversation_or_catalogue_drift() {
    for context in [(9, 2, 3), (1, 9, 3), (1, 2, 9)] {
        let mut prompt = TravelPrompt::new(NewhavenRoute::Anna, (1, 2, 3), 100, false);
        assert_eq!(prompt.confirm(&context, 200), TravelConfirmation::Refused);
        assert_eq!(prompt.confirm(&(1, 2, 3), 201), TravelConfirmation::Refused);
    }
}

#[test]
fn m38_travel_prompt_ordinary_messages_do_not_consume_the_selected_departure() {
    let mut prompt = TravelPrompt::new(NewhavenRoute::Anna, (1, 2, 3), 100, false);
    for now in [101, 102, 60_099] {
        assert!(prompt.is_current(&(1, 2, 3), now));
    }
    assert!(!prompt.is_current(&(1, 2, 9), 60_099));
    assert_eq!(
        prompt.confirm(&(1, 2, 3), 60_099),
        TravelConfirmation::Ready(NewhavenRoute::Anna)
    );
    assert!(!prompt.is_current(&(1, 2, 3), 60_099));
}

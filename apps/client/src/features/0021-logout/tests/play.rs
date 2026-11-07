use super::*;

#[test]
fn the_in_fight_line_rounds_the_wait_up_to_whole_seconds() {
    let in_fight = |retry_after_ms| Some(LogoutResult::InFight { retry_after_ms });
    assert_eq!(
        logout_refusal_text(in_fight(1)),
        "You may not logout during a fight (1 s)"
    );
    assert_eq!(
        logout_refusal_text(in_fight(60_000)),
        "You may not logout during a fight (60 s)"
    );
    assert_eq!(
        logout_refusal_text(Some(LogoutResult::Busy)),
        "You cannot logout now, try again"
    );
}

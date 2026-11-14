use super::*;
use crate::play::{mailbox::lock, play_channel};
use oteryn_client_ui::boss_difficulty::Intent;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn boss_difficulty_owner_rejections_are_atomic() -> TestResult {
    let (link, mut rx, outbox) = play_channel();
    let expected = BossDifficultyPanel {
        selected: true,
        open: true,
        boss_key: "oteryn:dragon".into(),
        context: Some([7; 16]),
        difficulty: 2,
        ..Default::default()
    };
    outbox.bevy_boss_difficulty_panel(expected.clone());
    for (intent, boss, difficulty, confirmed) in [
        (Intent::Start, "oteryn:dragon", 2, false),
        (Intent::Cancel, "oteryn:dragon", 2, false),
        (Intent::Preview, "oteryn:other", 2, false),
        (Intent::Preview, "oteryn:dragon", 29, false),
        (Intent::Open, "", 2, false),
    ] {
        assert!(
            !link.boss_difficulty_dispatch_bound(&expected, intent, boss, difficulty, confirmed)
        );
        assert_eq!(link.boss_difficulty_panel(), expected);
        assert!(rx.try_recv().is_err());
    }
    let fixture = link.bevy_server_modal_fixture();
    let before = link.boss_difficulty_panel();
    assert!(!link.boss_difficulty_dispatch_bound(
        &expected,
        Intent::Preview,
        "oteryn:dragon",
        2,
        false
    ));
    assert_eq!(link.boss_difficulty_panel(), before);
    assert!(rx.try_recv().is_err());
    drop(fixture);
    lock(&link.mailbox).boss_difficulty.open = false;
    assert!(!link.boss_difficulty_dispatch_bound(
        &expected,
        Intent::Preview,
        "oteryn:dragon",
        2,
        false
    ));
    assert!(!link.boss_difficulty_panel().open);
    Ok(())
}

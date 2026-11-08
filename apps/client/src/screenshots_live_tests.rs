use crate::play::play_channel;
use crate::screenshots::CaptureController;
use oteryn_session::{ActorVitals, AppliedDelta, SessionEvent};

#[test]
fn live_death_events_reach_capture_caller_even_when_respawn_is_coalesced()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _, outbox) = play_channel();
    let mut capture = CaptureController::default();
    capture.policy.death = true;
    capture.observe(link.screenshot_facts());
    let vitals = |health| {
        SessionEvent::ActorVitals(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: ActorVitals {
                health,
                max_health: 150,
                ..Default::default()
            },
        })
    };
    outbox.push(vec![vitals(0), vitals(150)]);
    assert_eq!(link.screenshot_facts().deaths, 1);
    capture.observe(link.screenshot_facts());
    assert!(capture.pending());
    capture.cancel();
    outbox.push(vec![vitals(150)]);
    capture.observe(link.screenshot_facts());
    assert!(!capture.pending());
    Ok(())
}

#[test]
fn server_skill_reply_and_achievement_delta_reach_capture_caller()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::achievement_notices::{
        AchievementEarned, AchievementWatermark, decode_achievement_earned,
        encode_achievement_earned,
    };
    use oteryn_session::{CommandStatus, GatedCommandResult};
    let (link, _, outbox) = play_channel();
    outbox.skill_display_initialize(true);
    let mut capture = CaptureController::default();
    capture.policy.level = true;
    let result = |level| -> Result<GatedCommandResult, Box<dyn std::error::Error>> {
        let full = oteryn_session::skill_display::SkillDisplay {
            level,
            ..Default::default()
        };
        Ok(GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: full.encode().ok_or("skill encode")?,
        })
    };
    outbox.skill_progress_result(&result(8)?);
    capture.observe(link.screenshot_facts());
    assert!(!capture.pending());
    outbox.skill_progress_result(&result(9)?);
    capture.observe(link.screenshot_facts());
    assert!(capture.pending());
    capture.cancel();
    capture.policy.achievement = true;
    crate::play::mailbox::lock(&link.mailbox)
        .achievement_notices
        .selected = true;
    for count in 1..=2 {
        let value = AchievementEarned {
            key: format!("oteryn:achievement/capture{count}"),
            name: format!("Capture {count}"),
            watermark: AchievementWatermark {
                fact_count: count,
                total_points: count,
            },
        };
        let bytes = encode_achievement_earned(&value).map_err(|e| format!("{e:?}"))?;
        let decoded = decode_achievement_earned(&bytes).map_err(|e| format!("{e:?}"))?;
        outbox.push(vec![SessionEvent::AchievementEarned(AppliedDelta {
            server_sequence: count.into(),
            base_revision: (count - 1).into(),
            new_revision: count.into(),
            value: decoded,
        })]);
        capture.observe(link.screenshot_facts());
        assert_eq!(capture.pending(), count == 2);
    }
    Ok(())
}

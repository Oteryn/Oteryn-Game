use crate::combat_input::PlayCommand;
use crate::play::{mailbox::lock, play_channel};
use oteryn_session::skill_display::SkillDisplay;
use oteryn_session::skill_progress::{SkillProgress, SkillProgressSnapshot};
use oteryn_session::{CommandStatus, GatedCommandResult};
use std::time::{Duration, Instant};

#[test]
fn shared_skill_snapshots_refresh_with_bounded_common_polling()
-> Result<(), Box<dyn std::error::Error>> {
    for complete in [false, true] {
        let (link, mut commands, outbox) = play_channel();
        outbox.skill_progress_initialize(true);
        outbox.skill_display_initialize(complete);
        let mut display = SkillDisplay {
            trained: SkillProgressSnapshot {
                skills: [SkillProgress {
                    level: 20,
                    progress: 450,
                    required: 1000,
                    percent: 4500,
                }; 8],
            },
            level: 30,
            experience: 100_000,
            level_percent: 2345,
            loyalty_bonus_percent: 10,
            loyalty_levels: [23; 8],
            current_levels: Some([24; 8]),
        };
        let result = |display: &SkillDisplay| -> Result<GatedCommandResult, &'static str> {
            Ok(GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: if complete {
                    display.encode()
                } else {
                    display.trained.encode()
                }
                .ok_or("encode skills")?,
            })
        };
        outbox.skill_progress_result(&result(&display)?);
        let expire = || -> Result<(), &'static str> {
            lock(&outbox.0).skill_progress_last_request = Some(
                Instant::now()
                    .checked_sub(Duration::from_secs(6))
                    .ok_or("clock")?,
            );
            Ok(())
        };
        expire()?;
        assert_eq!(link.skill_progress_panel().snapshot, Some(display.trained));
        assert!(
            commands.try_recv().is_err(),
            "inactive native panel does not poll"
        );
        link.use_shared_frontend();
        assert_eq!(link.skill_progress_panel().snapshot, Some(display.trained));
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            unreachable!()
        };
        assert_eq!(
            (command.capability, command.kind),
            if complete { (370, 640) } else { (279, 495) }
        );
        for _ in 0..10 {
            link.skill_progress_panel();
            link.skills_widget();
        }
        assert!(commands.try_recv().is_err(), "one request in flight");
        display.trained.skills[0].level = 21;
        display.trained.skills[0].progress = 50;
        display.trained.skills[0].percent = 500;
        display.level = 31;
        display.experience = 110_000;
        outbox.skill_progress_result(&result(&display)?);
        let panel = link.skill_progress_panel();
        assert_eq!(panel.snapshot, Some(display.trained));
        if complete {
            assert_eq!(panel.complete, Some(display));
        }
        assert_eq!(link.skills_widget().rows[0].trained, 21);
        assert_eq!(link.skills_widget().rows[0].progress, Some(500));
        assert!(
            commands.try_recv().is_err(),
            "receipt starts the polling interval"
        );
        expire()?;
        link.skills_widget();
        assert!(
            matches!(commands.try_recv()?, PlayCommand::Feature(_)),
            "widget shares the refresh path"
        );
        outbox.skill_progress_result(&GatedCommandResult {
            status: CommandStatus::Rejected,
            ..result(&display)?
        });
        link.skill_progress_panel();
        assert!(
            commands.try_recv().is_err(),
            "refusal also starts the polling interval"
        );
        expire()?;
        lock(&outbox.0).skill_progress.selected = false;
        link.skill_progress_panel();
        link.skills_widget();
        assert!(
            commands.try_recv().is_err(),
            "unselected feature never polls"
        );
    }
    Ok(())
}

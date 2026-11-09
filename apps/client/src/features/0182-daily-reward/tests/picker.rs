use super::*;
use crate::{combat_input::PlayCommand, play::mailbox::lock};
use daily_reward::picker::{Claim, Offer, Pick};
#[test]
fn item_picker_real_caller_validates_catalogue_and_sends_exact_selection() -> Result<(), Error> {
    let (link, mut incoming, outbox) = play_channel();
    outbox.daily_reward_initialize(true);
    lock(&link.mailbox).daily_reward.wall = Some(daily_reward::DailyRewardWall {
        lane_day: 1,
        streak: 0,
        jokers: 0,
        claimed: false,
        claim_enabled: true,
    });
    link.daily_reward_picker_request(None);
    assert!(
        matches!(incoming.try_recv(),Ok(PlayCommand::Feature(c)) if c.kind==daily_reward::picker::QUERY && c.capability==182)
    );
    let mut offer = Offer {
        epoch: 2,
        day: 1,
        premium: false,
        maximum: 5,
        items: [0; 32],
    };
    offer.items[0] = 266;
    offer.items[1] = 268;
    outbox.daily_reward_picker_result(
        daily_reward::picker::QUERY,
        &oteryn_session::GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: offer.encode()?,
        },
    );
    link.daily_reward_picker_request(Some("266:6"));
    assert!(incoming.try_recv().is_err());
    link.daily_reward_picker_request(Some("237:1"));
    assert!(incoming.try_recv().is_err());
    link.daily_reward_picker_request(Some("268:2,266:3"));
    let Ok(PlayCommand::Feature(c)) = incoming.try_recv() else {
        return Err("pick claim".into());
    };
    assert_eq!((c.capability, c.kind), (182, daily_reward::picker::CLAIM));
    assert_eq!(
        Claim::decode(&c.payload)?,
        Claim {
            epoch: 2,
            day: 1,
            picks: vec![
                Pick {
                    item: 266,
                    count: 3
                },
                Pick {
                    item: 268,
                    count: 2
                }
            ]
        }
    );
    assert!(!link.daily_reward_panel().wall.ok_or("wall")?.claimed);
    outbox.daily_reward_picker_result(
        daily_reward::picker::CLAIM,
        &oteryn_session::GatedCommandResult {
            command_id: 2,
            status: CommandStatus::Accepted,
            server_sequence: 2,
            payload: daily_reward::DailyRewardResult {
                disposition: 1,
                transaction_id: Some([0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]),
            }
            .encode()
            .map_err(|_| "result")?,
        },
    );
    assert!(link.daily_reward_panel().wall.ok_or("wall")?.claimed);
    Ok(())
}

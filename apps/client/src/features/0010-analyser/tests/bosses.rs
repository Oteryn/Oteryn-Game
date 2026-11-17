use super::*;
use oteryn_client_runtime::{
    boss_encounter as encounter,
    boss_rewards::{
        BossRewardsCommand, BossRewardsReply, BossRewardsResponse, boss_rewards as wire,
    },
};
#[test]
fn boss_tracker_controls_use_existing_protocol_owners_and_feed_observed_history()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        combat_input::PlayCommand,
        play::{feature_0010_analyser::input::submit, mailbox::lock},
    };
    use oteryn_client_ui::analyser::Tool;
    let (link, mut incoming, outbox) = play_channel();
    outbox.analyser_initialize(true);
    outbox.boss_encounter_initialize(true);
    outbox.boss_rewards_initialize(true);
    let tools = Tool::ALL.len();
    {
        let mut mailbox = lock(&link.mailbox);
        submit(&mut mailbox, &link.commands, tools - 1, vec![]);
        submit(&mut mailbox, &link.commands, tools + 13, vec![]);
    }
    let PlayCommand::Feature(command) = incoming.try_recv()? else {
        return Err("cooldown query".into());
    };
    assert_eq!(command, encounter::query_taints("").ok_or("query")?);
    {
        let mut mailbox = lock(&link.mailbox);
        submit(
            &mut mailbox,
            &link.commands,
            tools + 17,
            vec![String::new(), "💣".repeat(8)],
        );
        assert!(mailbox.analyser.feedback.is_some());
    }
    assert!(incoming.try_recv().is_err());
    let death = "09".repeat(16);
    {
        let mut mailbox = lock(&link.mailbox);
        submit(
            &mut mailbox,
            &link.commands,
            tools + 17,
            vec![String::new(), death.clone()],
        );
    }
    assert!(
        matches!(incoming.try_recv()?,PlayCommand::BossRewards(BossRewardsCommand::Browse(v)) if v.death==[9;16]&&v.after_ordinal==0)
    );
    let reply = wire::BrowseReply {
        status: wire::BossRewardStatus::Ok,
        death: [9; 16],
        expires_at_unix_ms: 10000,
        rows: vec![wire::RewardRow {
            ordinal: 3,
            definition_reference: 700,
            quantity: 4,
        }],
        more: true,
    };
    let bytes = wire::encode_browse_reply(&reply).map_err(|_| "encode reward")?;
    let decoded = wire::decode_browse_reply(&bytes).map_err(|_| "decode reward")?;
    outbox.boss_rewards_result(&BossRewardsResponse {
        status: oteryn_session::CommandStatus::Accepted,
        reply: Some(BossRewardsReply::Browse(decoded)),
    });
    assert_eq!(link.analyser_panel().boss_drops.len(), 1);
    assert!(link.analyser_text().contains("item 700 × 4"));
    {
        let mut mailbox = lock(&link.mailbox);
        submit(
            &mut mailbox,
            &link.commands,
            tools + 17,
            vec![String::new(), death],
        );
    }
    assert!(
        matches!(incoming.try_recv()?,PlayCommand::BossRewards(BossRewardsCommand::Browse(v)) if v.after_ordinal==3)
    );
    outbox.logged_out();
    assert!(link.analyser_panel().boss_drops.is_empty());
    Ok(())
}

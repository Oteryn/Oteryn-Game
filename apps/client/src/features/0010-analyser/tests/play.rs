use crate::play::play_channel;
use oteryn_session::{AppliedDelta, SessionEvent, analyser_transport::AnalyserFact};
#[test]
fn analyser_push_reaches_closed_panel_and_end_clears_totals() {
    let (link, _, outbox) = play_channel();
    outbox.analyser_initialize(true);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 8,
        base_revision: 1,
        new_revision: 2,
        value: vec![AnalyserFact::Kill { race: None }],
    })]);
    assert_eq!(link.analyser_panel().kills, 1);
    assert!(!link.analyser_panel().open);
    outbox.logged_out();
    assert_eq!(link.analyser_panel().kills, 0);
    assert!(!link.analyser_panel().selected);
}

#[test]
fn analyser_item_query_and_reply_use_the_production_command_mailbox()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{
        CommandStatus,
        analyser_catalogue::{ItemInfo, encode_reply},
        analyser_transport::AnalyserItemCount,
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.analyser_initialize(true);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 8,
        base_revision: 1,
        new_revision: 2,
        value: vec![AnalyserFact::KillLoot {
            race: None,
            corpse_items: vec![AnalyserItemCount {
                item_type: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU16::MIN,
            }],
            gold: 1,
        }],
    })]);
    link.analyser_resolve_items();
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!((command.capability, command.kind), (10, 731));
    assert_eq!(
        oteryn_session::analyser_catalogue::decode_request(&command.payload)?,
        vec![1]
    );
    outbox.analyser_catalogue_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 9,
        payload: encode_reply(&[ItemInfo {
            item_type: 1,
            name: "Gold piece".into(),
            coin_worth: 1,
        }])?,
    });
    assert_eq!(
        link.analyser_panel()
            .item_info
            .get(&1)
            .map(|info| info.coin_worth),
        Some(1)
    );
    assert!(link.analyser_panel().catalogue_pending.is_empty());
    Ok(())
}

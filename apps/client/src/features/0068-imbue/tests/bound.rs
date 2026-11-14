use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{mailbox::lock, play_channel},
};
use oteryn_client_ui::imbue::{ImbueResponse, imbuement as wire};
use std::num::{NonZeroU32, NonZeroU64};
fn setup() -> (
    PlayLink,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
) {
    let (link, commands, outbox) = play_channel();
    {
        let mut m = lock(&outbox.0);
        m.imbue = ImbuePanel {
            selected: true,
            open: true,
            item: Some(NonZeroU64::MIN),
            window: Some(wire::ImbuementWindow {
                slot_count: 2,
                applied: vec![],
                offers: vec![wire::ImbuementOffer {
                    family: 1,
                    tier: 1,
                    gold: 431,
                    requires_premium: false,
                }],
                clear_gold: 28,
            }),
            ..Default::default()
        };
        m.depot
            .items
            .inventory
            .entries
            .push(oteryn_client_runtime::depot::ItemEntry {
                handle: NonZeroU64::MIN,
                item_definition_ref: NonZeroU32::MIN,
                count: NonZeroU32::MIN,
                sub_type: 0,
            });
    }
    (link, commands, outbox)
}
#[test]
fn imbue_apply_and_clear_are_item_slot_bound() -> Result<(), String> {
    let (link, mut commands, outbox) = setup();
    let empty = link.imbue_snapshot();
    assert!(!link.imbue_dispatch_bound(&empty, ImbueControl::Clear));
    assert_eq!(link.imbue_snapshot(), empty);
    assert!(commands.try_recv().is_err());
    assert!(link.imbue_dispatch_bound(&link.imbue_snapshot(), ImbueControl::Slot(1)));
    let expected = link.imbue_snapshot();
    assert!(link.imbue_dispatch_bound(&expected, ImbueControl::Apply));
    assert!(!link.imbue_dispatch_bound(&expected, ImbueControl::Apply));
    let Ok(PlayCommand::Imbue(command)) = commands.try_recv() else {
        return Err("missing apply".into());
    };
    assert_eq!(command.0.item, NonZeroU64::MIN);
    assert_eq!(
        command.0.action,
        wire::ImbuementAction::Apply {
            slot: 1,
            imbuement: 0
        }
    );
    assert_eq!(link.imbue_panel().window, expected.panel.window);
    let mut window = expected.panel.window.ok_or("window")?;
    window.applied.push(wire::AppliedImbuement {
        slot: 1,
        family: 1,
        tier: 1,
        remaining_ms: 90000,
    });
    outbox.imbue_result(&ImbueResponse {
        status: oteryn_client_runtime::imbue::CommandStatus::Accepted,
        reply: Some(wire::ImbuementReply {
            result: wire::ImbuementResult::Ok,
            window: Some(window.clone()),
        }),
        success: vec![],
    });
    assert_eq!(link.imbue_panel().window, Some(window.clone()));
    assert!(link.imbue_dispatch_bound(&link.imbue_snapshot(), ImbueControl::Clear));
    let Ok(PlayCommand::Imbue(command)) = commands.try_recv() else {
        return Err("missing clear".into());
    };
    assert_eq!(command.0.item, NonZeroU64::MIN);
    assert_eq!(command.0.action, wire::ImbuementAction::Clear { slot: 1 });
    assert_eq!(link.imbue_panel().window, Some(window));
    Ok(())
}
#[test]
fn imbue_actions_rejected_when_session_replaced_or_item_changed() {
    for action in [
        ImbueControl::Apply,
        ImbueControl::Clear,
        ImbueControl::Refresh,
        ImbueControl::Details,
        ImbueControl::Slot(1),
        ImbueControl::Offer(0),
        ImbueControl::Item(NonZeroU64::MIN),
        ImbueControl::Close,
        ImbueControl::TrackerRefresh,
        ImbueControl::CloseTracker,
    ] {
        let (old, _, _) = setup();
        let (replacement, mut replacement_commands, _) = setup();
        let before = replacement.imbue_snapshot();
        assert!(!replacement.imbue_dispatch_bound(&old.imbue_snapshot(), action.clone()));
        assert_eq!(replacement.imbue_snapshot(), before);
        assert!(replacement_commands.try_recv().is_err());
        for reason in 0..4 {
            let (link, mut commands, outbox) = setup();
            let expected = link.imbue_snapshot();
            {
                let mut m = lock(&outbox.0);
                match reason {
                    0 => m.imbue_shutdown = true,
                    1 => m.depot.items.inventory.entries.clear(),
                    2 => m.imbue.open = false,
                    _ => m.imbue.select_item(NonZeroU64::MIN.saturating_add(1)),
                }
            }
            let before = link.imbue_snapshot();
            assert!(!link.imbue_dispatch_bound(&expected, action.clone()));
            assert_eq!(link.imbue_snapshot(), before);
            assert!(commands.try_recv().is_err());
        }
    }
}

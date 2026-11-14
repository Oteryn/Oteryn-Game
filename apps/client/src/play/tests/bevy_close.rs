use super::*;
use crate::play::mailbox::lock;

#[test]
fn bevy_closing_unavailable_pending_views_preserves_operations_and_rejects_other_sessions()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    let (other, _commands, _outbox) = play_channel();
    macro_rules! close {
        ($($field:ident).+, $snapshot:expr, $close:expr, $other:expr) => {{
            let stale = $snapshot;
            let mut expected = {
                let mut m = lock(&outbox.0);
                m.$($field).+.open = true;
                m.$($field).+.selected = false;
                m.$($field).+.pending = true;
                m.$($field).+.clone()
            };
            assert!(!($other)(&stale));
            assert!(($close)(&stale));
            expected.open = false;
            assert_eq!(lock(&outbox.0).$($field).+, expected);
        }};
    }
    close!(
        depot,
        link.depot_binding(7),
        |b| link.depot_dispatch_bound(b, 7, None),
        |b| other.depot_dispatch_bound(b, 7, None)
    );
    close!(
        stash,
        link.stash_binding(7),
        |b| link.stash_dispatch_bound(
            b,
            7,
            crate::play::feature_0062_stash::view::StashControl::Close
        ),
        |b| other.stash_dispatch_bound(
            b,
            7,
            crate::play::feature_0062_stash::view::StashControl::Close
        )
    );
    close!(
        depot_search,
        link.depot_search_binding(7),
        |b| link.depot_search_dispatch_bound(b, 7, None),
        |b| other.depot_search_dispatch_bound(b, 7, None)
    );
    close!(
        properties.charm_progression,
        link.bestiary_binding(),
        |b| link.bestiary_dispatch_bound(
            b,
            crate::play::feature_0067_bestiary_charm_progression::view::BestiaryControl::Close,
            false
        ),
        |b| other.bestiary_dispatch_bound(
            b,
            crate::play::feature_0067_bestiary_charm_progression::view::BestiaryControl::Close,
            false
        )
    );
    close!(
        imbue,
        link.imbue_snapshot(),
        |b| link.imbue_dispatch_bound(
            b,
            crate::play::feature_0068_imbue::view::ImbueControl::Close
        ),
        |b| other.imbue_dispatch_bound(
            b,
            crate::play::feature_0068_imbue::view::ImbueControl::Close
        )
    );
    close!(
        store,
        link.store_snapshot(),
        |b| link.store_dispatch_bound(b, crate::play::feature_0154_store::view::StoreAction::Close),
        |b| other
            .store_dispatch_bound(b, crate::play::feature_0154_store::view::StoreAction::Close)
    );
    close!(
        properties.appearance,
        link.appearance_snapshot(),
        |b| link.appearance_dispatch_bound(
            b,
            crate::play::feature_0025_appearance::view::AppearanceControl::Close
        ),
        |b| other.appearance_dispatch_bound(
            b,
            crate::play::feature_0025_appearance::view::AppearanceControl::Close
        )
    );
    // The same retained pending views can be reopened without resubmitting a request.
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    for (code, modifiers) in [
        (15, Modifiers::one(Modifier::Control).with(Modifier::Shift)),
        (22, Modifiers::one(Modifier::Control)),
        (4, Modifiers::one(Modifier::Control).with(Modifier::Shift)),
    ] {
        let code = KeyCode::new(code)?;
        let events = [NormalizedInputEvent::Key {
            code,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        }];
        assert!(
            link.bevy_depot_input(&events)
                || link.bevy_stash_input(&events)
                || link.bevy_depot_search_input(&events)
        );
    }
    assert!(link.depot_panel().open && link.depot_panel().pending);
    assert!(link.stash_panel().open && link.stash_panel().pending);
    assert!(link.depot_search_panel().open && link.depot_search_panel().pending);
    lock(&outbox.0).imbue.tracker_open = true;
    assert!(link.imbue_dispatch_bound(
        &link.imbue_snapshot(),
        crate::play::feature_0068_imbue::view::ImbueControl::CloseTracker
    ));
    assert!(link.imbue_panel().pending);
    assert!(!link.imbue_panel().tracker_open);
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn bevy_wheel_close_retains_pending_bytes_and_retry_state() {
    let (link, mut commands, outbox) = play_channel();
    let stale = link.wheel_panel();
    let mut expected = {
        let mut m = lock(&outbox.0);
        m.wheel.open = true;
        m.wheel.presets_open = true;
        m.wheel.selected = false;
        m.wheel.pending = Some(oteryn_client_ui::wheel_allocation::WheelCommand {
            kind: 1,
            payload: vec![7, 8, 9],
            mutation: true,
        });
        m.wheel.clone()
    };
    assert!(link.wheel_dispatch_bound(
        &stale,
        crate::play::feature_0148_wheel_allocation::view::WheelControl::Close
    ));
    expected.open = false;
    expected.presets_open = false;
    assert_eq!(link.wheel_panel(), expected);
    assert!(commands.try_recv().is_err());
}

#[test]
fn bevy_trade_close_is_local_while_unavailable_and_pending() {
    let (link, mut commands, outbox) = play_channel();
    let stale = link.player_trade_panel();
    let mut expected = {
        let mut m = lock(&outbox.0);
        m.player_trade.open = true;
        m.player_trade.show_requested = true;
        m.player_trade.pending = true;
        m.player_trade.selected = false;
        m.player_trade.clone()
    };
    assert!(link.player_trade_dispatch_bound(
        &stale,
        crate::play::feature_0060_player_trade::view::TradeAction::Close
    ));
    expected.open = false;
    expected.show_requested = false;
    assert_eq!(link.player_trade_panel(), expected);
    assert!(commands.try_recv().is_err());
}

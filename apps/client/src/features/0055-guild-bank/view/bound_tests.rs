use super::*;
use crate::play::play_channel;
fn panel() -> Panel {
    Panel {
        selected: true,
        open: true,
        fields: ["01010101010170018001010101010101".into(), "25".into()],
        summary: "Guild bank: 500".into(),
        ..Default::default()
    }
}
fn request(intent: Intent) -> GuildBankControl {
    GuildBankControl::Request {
        intent,
        fields: panel().fields,
    }
}
#[test]
fn guild_bank_bound_owner_checks_amount_confirmation_pending_and_stale_session() {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_guild_bank_panel(panel());
    let expected = link.guild_bank_window_view();
    for amount in ["0", "-1", "not an amount", "1000000000000"] {
        assert!(!link.guild_bank_window_dispatch(
            &expected,
            GuildBankControl::Request {
                intent: Intent::Deposit,
                fields: [panel().fields[0].clone(), amount.into()]
            },
            true
        ));
        assert_eq!(link.guild_bank_panel(), expected.panel);
    }
    assert!(!link.guild_bank_window_dispatch(&expected, request(Intent::Deposit), false));
    let (other, mut other_commands, other_outbox) = play_channel();
    other.use_shared_frontend();
    other_outbox.bevy_guild_bank_panel(panel());
    assert!(!other.guild_bank_window_dispatch(&expected, request(Intent::Deposit), true));
    assert_eq!(other.guild_bank_panel(), expected.panel);
    assert!(other_commands.try_recv().is_err());
    assert!(link.guild_bank_window_dispatch(&expected, request(Intent::Deposit), true));
    assert!(commands.try_recv().is_ok());
    let pending = link.guild_bank_window_view();
    assert!(!link.guild_bank_window_dispatch(&pending, request(Intent::Withdraw), true));
    assert_eq!(link.guild_bank_panel(), pending.panel);
    assert!(commands.try_recv().is_err());
}
#[test]
fn guild_bank_bound_owner_rejects_server_modal_closed_channel_and_session_end() {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_guild_bank_panel(panel());
    let expected = link.guild_bank_window_view();
    let _modal = link.bevy_server_modal_fixture();
    for intent in [
        Intent::Refresh,
        Intent::Deposit,
        Intent::Withdraw,
        Intent::Claim,
        Intent::Claims,
        Intent::Older,
        Intent::MoreClaims,
    ] {
        assert!(!link.guild_bank_window_dispatch(&expected, request(intent), true));
        assert_eq!(link.guild_bank_panel(), expected.panel);
    }
    assert!(!link.guild_bank_window_dispatch(&expected, GuildBankControl::Close, true));
    assert!(commands.try_recv().is_err());
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_guild_bank_panel(panel());
    let expected = link.guild_bank_window_view();
    drop(commands);
    assert!(!link.guild_bank_window_dispatch(&expected, request(Intent::Deposit), true));
    assert_eq!(link.guild_bank_panel(), expected.panel);
    outbox.bevy_guild_end();
    let ended = link.guild_bank_panel();
    assert!(!link.guild_bank_window_dispatch(&expected, request(Intent::Withdraw), true));
    assert!(!link.guild_bank_window_open());
    assert_eq!(link.guild_bank_panel(), ended);
}

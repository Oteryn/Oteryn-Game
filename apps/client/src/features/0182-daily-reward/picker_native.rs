//! Native item catalogue and validated selection form; all rewards stay server-owned.
use crate::play::PlayLink;
use crate::play::feature_0115_mail::view::native::{Form, open};
pub(super) fn open_picker(link: &PlayLink) {
    open(
        link,
        Form {
            title: "Daily reward — pick items",
            labels: &["Selections (item:count, item:count)"],
            buttons: &["Refresh catalogue", "Claim selected items"],
            snapshot: |m| {
                (
                    m.daily_reward.picker_open,
                    m.daily_reward.pending,
                    vec![String::new()],
                    oteryn_client_ui::daily_reward::picker_text(&m.daily_reward),
                )
            },
            set_open: |m, value| m.daily_reward.picker_open = value,
            submit: |m, commands, action, values| {
                super::super::input::enqueue_picker(
                    m,
                    commands,
                    if action == 0 {
                        None
                    } else {
                        Some(values.first().map_or("", String::as_str))
                    },
                );
            },
        },
    );
}

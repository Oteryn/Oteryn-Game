//! Reuse the admitted boss owners; analyser controls only request read-only projections.
use crate::{combat_input::PlayCommand, play::mailbox::Mailbox};
use tokio::sync::mpsc::UnboundedSender;
pub(super) fn action(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    action: usize,
    death: Option<&str>,
) {
    use oteryn_client_ui::boss_encounter::Intent;
    mailbox.analyser.feedback = None;
    if action < 4 {
        let intent = match action {
            0 => Intent::Refresh,
            1 => Intent::Next,
            2 => Intent::Rewards,
            _ => Intent::RewardsNext,
        };
        crate::play::feature_0117_boss_encounter::input::enqueue(mailbox, commands, intent);
        return;
    }
    if !mailbox.boss_rewards.selected || mailbox.boss_rewards.pending {
        mailbox.analyser.feedback = Some(tr!(
            "Boss rewards unavailable or already waiting for the server"
        ));
        return;
    }
    let Some(death) = death.and_then(parse_death) else {
        mailbox.analyser.feedback = Some(tr!(
            "Enter a 32-digit hexadecimal death id from the drop list"
        ));
        return;
    };
    let cursor = mailbox
        .boss_rewards
        .browse
        .as_ref()
        .filter(|page| page.death == death && page.more)
        .and_then(|page| page.rows.last())
        .map_or(0, |row| row.ordinal);
    mailbox.boss_rewards.death = death;
    mailbox.boss_rewards.after_ordinal = cursor;
    mailbox.boss_rewards.browse = None;
    crate::play::feature_0224_boss_rewards::input::enqueue(
        mailbox,
        commands,
        oteryn_client_ui::boss_rewards::Intent::Browse,
    );
}
fn parse_death(value: &str) -> Option<[u8; 16]> {
    let value = value.trim();
    if value.len() != 32 {
        return None;
    }
    let mut death = [0; 16];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        death[index] = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    (death != [0; 16]).then_some(death)
}

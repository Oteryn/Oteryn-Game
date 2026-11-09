//! Spell detail projection shares the action bar's server-confirmed cooldown state.
use crate::play::mailbox::Mailbox;
use std::{num::NonZeroU32, time::Instant};

pub(super) fn text(mailbox: &Mailbox, row: usize) -> String {
    text_at(mailbox, row, Instant::now())
}

fn text_at(mailbox: &Mailbox, row: usize, now: Instant) -> String {
    let state = &mailbox.spellbook_details;
    let mut text = state.text(row);
    let Some(spell) = state.page.as_ref().and_then(|page| page.entries.get(row)) else {
        return text;
    };
    let panel = &mailbox.spell_bar;
    let Some(index) = NonZeroU32::new(spell.spell.cast_index) else {
        return text;
    };
    if panel.bar.cooldowns.is_none() {
        text.push_str("\r\nCooldown remaining: unavailable (no server snapshot).");
        return text;
    }
    let elapsed = panel.cooldown_received_at.map_or(0, |received| {
        u64::try_from(now.saturating_duration_since(received).as_millis()).unwrap_or(u64::MAX)
    });
    let remaining = panel
        .bar
        .remaining_ms(index, &spell.spell.cooldown_groups)
        .saturating_sub(elapsed);
    use std::fmt::Write;
    let _ = write!(
        text,
        "\r\nCooldown remaining: {remaining} ms (estimated from server snapshot; server validates casts)."
    );
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::{mailbox::lock, play_channel};
    use oteryn_client_runtime::spell_catalog::spellbook_details::{DetailedSpell, Page};
    use oteryn_session::{
        ActorCooldown, ActorCooldownKind, ActorCooldowns, AppliedDelta, CommandStatus,
        GatedCommandResult, SessionEvent, spell_catalog::SpellCatalogEntry,
    };
    use std::time::Duration;

    #[test]
    fn spellbook_details_project_live_spell_and_group_cooldowns_without_claiming_cast_authority()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, mut commands, outbox) = play_channel();
        outbox.spellbook_details_initialize(true);
        assert!(link.spellbook_open());
        let _ = commands.try_recv()?;
        outbox.spellbook_details_result(&GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: Page {
                page: 0,
                total: 1,
                entries: vec![DetailedSpell {
                    spell: SpellCatalogEntry {
                        cast_index: 71,
                        name: "Mending".into(),
                        words: "mend".into(),
                        level: 20,
                        mana: 15,
                        mana_percent: false,
                        soul: 0,
                        learned: true,
                        available: true,
                        cooldown_groups: vec![2],
                        rune_item: None,
                    },
                    vocations: 1,
                    premium: false,
                    magic_level: 0,
                }],
            }
            .encode()
            .ok_or("page")?,
        });
        assert!(text(&lock(&link.mailbox), 0).contains("no server snapshot"));
        outbox.push(vec![SessionEvent::ActorCooldowns(AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: ActorCooldowns {
                server_now_ms: 100,
                rtt_ms: None,
                entries: vec![
                    ActorCooldown {
                        kind: ActorCooldownKind::Spell,
                        id: 71,
                        expires_at_ms: 600,
                    },
                    ActorCooldown {
                        kind: ActorCooldownKind::Group,
                        id: 2,
                        expires_at_ms: 900,
                    },
                    ActorCooldown {
                        kind: ActorCooldownKind::Group,
                        id: 3,
                        expires_at_ms: 5000,
                    },
                ],
            },
        })]);
        let mailbox = lock(&link.mailbox);
        let received = mailbox
            .spell_bar
            .cooldown_received_at
            .ok_or("snapshot time")?;
        assert!(text_at(&mailbox, 0, received).contains("Cooldown remaining: 800 ms"));
        let projected = text_at(&mailbox, 0, received + Duration::from_millis(250));
        assert!(projected.contains("Cooldown remaining: 550 ms"));
        assert!(projected.contains("server validates casts"));
        assert!(
            text_at(&mailbox, 0, received + Duration::from_secs(1))
                .contains("Cooldown remaining: 0 ms")
        );
        assert!(!text_at(&mailbox, 1, received).contains("Cooldown remaining:"));
        assert!(
            commands.try_recv().is_err(),
            "rendering must not issue a cast"
        );
        Ok(())
    }
}

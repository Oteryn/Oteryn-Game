use super::*;
use oteryn_session::ActorVitals;

#[test]
fn clxa_soul_and_monk_vitals_reach_native_hud() -> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    let mut view = view()?;
    for (vitals, expected) in [
        (
            ActorVitals {
                soul: 100,
                harmony: 5,
                serene: true,
                ..Default::default()
            },
            ["Soul: 100", "Harmony: 5/5", "Serene: active"],
        ),
        (
            ActorVitals::default(),
            ["Soul: 0", "Harmony: 0/5", "Serene: inactive"],
        ),
    ] {
        outbox.vitals(Some(vitals));
        view.tick(&link).map_err(|_| "tick")?;
        for text in expected {
            assert!(view.hud_status().contains(text), "{}", view.hud_status());
        }
    }
    Ok(())
}

#[test]
fn vitals_reach_the_play_projection_and_a_new_connection_starts_empty() -> Result<(), BatchError> {
    let (link, _commands, outbox) = play_channel();
    let mut view = view()?;
    assert!(view.actor_vitals().is_none());
    let initial = ActorVitals {
        health: 150,
        max_health: 150,
        mana: 55,
        max_mana: 60,
        ..ActorVitals::default()
    };
    outbox.vitals(Some(initial));
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(view.actor_vitals(), Some(&initial));
    let updated = ActorVitals {
        health: 90,
        mana: 20,
        ..initial
    };
    outbox.push(vec![SessionEvent::ActorVitals(AppliedDelta {
        server_sequence: 42,
        base_revision: 1,
        new_revision: 2,
        value: updated,
    })]);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(view.actor_vitals(), Some(&updated));
    let (fresh, _, _) = play_channel();
    assert!(fresh.actor_vitals().is_none());
    Ok(())
}

#[test]
fn undrained_vitals_keep_only_the_latest_value() -> Result<(), BatchError> {
    let (link, _commands, outbox) = play_channel();
    for health in 0..5_000 {
        outbox.push(vec![SessionEvent::ActorVitals(AppliedDelta {
            server_sequence: u64::from(health) + 1,
            base_revision: u64::from(health),
            new_revision: u64::from(health) + 1,
            value: ActorVitals {
                health,
                max_health: 5_000,
                ..ActorVitals::default()
            },
        })]);
    }
    assert!(link.poll().is_none());
    let mut view = view()?;
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(view.actor_vitals().map(|v| v.health), Some(4_999));
    Ok(())
}

#[test]
fn configured_slot_dispatches_server_index_and_invalid_edit_is_atomic()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, _) = play_channel();
    let mut view = view()?;
    assert!(link.spell_bar_bind(&["71".into(), "".into(), "3".into(), "4".into()]));
    assert!(!link.spell_bar_bind(&["12".into(), "bad".into(), "3".into(), "4".into()]));
    view.cast(std::num::NonZeroU32::new(1).ok_or("slot")?);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get() == 71)
    );
    view.cast(std::num::NonZeroU32::new(2).ok_or("slot")?);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn native_bar_cast_uses_binding_and_obeys_death_gate() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, _) = play_channel();
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    crate::play::feature_local_spell_action_bar::input::submit(
        &mut mailbox,
        &link.commands,
        0,
        &["71".into(), "".into(), "3".into(), "4".into()],
    );
    crate::play::feature_local_spell_action_bar::input::submit(
        &mut mailbox,
        &link.commands,
        1,
        &[],
    );
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get() == 71)
    );
    mailbox.apply_death_vitals(ActorVitals {
        health: 0,
        max_health: 100,
        ..Default::default()
    });
    crate::play::feature_local_spell_action_bar::input::submit(
        &mut mailbox,
        &link.commands,
        1,
        &[],
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn nx7_battle_instinct_reply_reaches_hud_beside_harmony() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, mut commands, outbox) = play_channel();
    let mut view = view()?;
    outbox.battle_instinct_status_initialize(true);
    let status = oteryn_client_ui::battle_instinct_status::BattleInstinctStatus {
        active: true,
        melee: 4,
        shielding: 24,
    };
    outbox.battle_instinct_status_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: status.encode().ok_or("encode")?,
    });
    outbox.vitals(Some(ActorVitals {
        harmony: 5,
        serene: true,
        ..Default::default()
    }));
    view.tick(&link).map_err(|_| "tick")?;
    assert!(view.hud_status().contains("Harmony: 5/5"));
    assert!(
        view.hud_status()
            .contains("Battle Instinct: active (+4 melee, +24 shielding)"),
        "{}",
        view.hud_status()
    );
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Feature(command) if (command.capability, command.kind) == (278, 494))
    );
    view.tick(&link).map_err(|_| "tick")?;
    assert!(
        commands.try_recv().is_err(),
        "HUD refresh is bounded to two seconds"
    );
    outbox.battle_instinct_status_result(&oteryn_session::GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: oteryn_session::CommandStatus::Accepted,
        payload: oteryn_client_ui::battle_instinct_status::BattleInstinctStatus {
            active: true,
            melee: 0,
            shielding: 0,
        }
        .encode()
        .ok_or("encode")?,
    });
    view.tick(&link).map_err(|_| "tick")?;
    assert!(view.hud_status().contains("Battle Instinct: eligible"));
    let (fresh, _, _) = play_channel();
    view.tick(&fresh).map_err(|_| "tick")?;
    assert!(!view.hud_status().contains("Battle Instinct"));
    Ok(())
}

#[path = "directional.rs"]
mod directional;

#[test]
fn full_spell_row_dispatches_last_slot_and_projects_server_cooldown()
-> Result<(), Box<dyn std::error::Error>> {
    use std::num::NonZeroU32;
    let (link, mut commands, outbox) = play_channel();
    let mut fields = vec![String::new(); 12];
    fields[11] = "71".into();
    assert!(link.spell_bar_bind(&fields));
    let mut view = view()?;
    view.cast(NonZeroU32::new(12).ok_or("slot")?);
    view.tick(&link).map_err(|_| "tick")?;
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get()==71)
    );
    outbox.push(vec![SessionEvent::ActorCooldowns(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: oteryn_session::ActorCooldowns {
            server_now_ms: 100,
            rtt_ms: None,
            entries: vec![oteryn_session::ActorCooldown {
                kind: oteryn_session::ActorCooldownKind::Spell,
                id: 71,
                expires_at_ms: 600,
            }],
        },
    })]);
    let mailbox = crate::play::mailbox::lock(&link.mailbox);
    let text = oteryn_client_ui::spell_action_bar::panel_text(
        &mailbox.spell_bar,
        &mailbox.spell_catalog.catalog,
    );
    assert!(text.contains("12: 71"));
    assert!(text.contains("500 ms"));
    assert!(
        mailbox
            .spell_bar
            .bar
            .spell(NonZeroU32::new(13).ok_or("slot")?)
            .is_none()
    );
    Ok(())
}

#[test]
fn mixed_bar_actions_use_real_commands_and_persist_separately_from_spell_indices()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::feature_local_spell_action_bar::view::{BarAction, PassiveAbility};
    use std::num::NonZeroU32;
    let (link, mut commands, outbox) = play_channel();
    outbox.hotkey_equip_initialize(true);
    outbox.hotkey_equip_items(vec![256]);
    assert!(link.assign_bar_action(
        10,
        BarAction::Equip {
            definition: NonZeroU32::new(256).ok_or("item")?
        }
    ));
    crate::play::feature_local_spell_action_bar::input::submit(
        &mut crate::play::mailbox::lock(&link.mailbox),
        &link.commands,
        11,
        &[],
    );
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("equip command".into());
    };
    assert_eq!(
        (command.capability, command.command_type, command.payload),
        (179, 348, vec![0, 0, 1, 0])
    );
    assert!(link.assign_bar_action(
        9,
        BarAction::Text {
            text: "Hello travellers".into()
        }
    ));
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.chat.selected = true;
        mailbox.chat.text = "Draft stays here".into();
    }
    link.cast_spell_slot(NonZeroU32::new(10).ok_or("slot")?);
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::ChatPublic(oteryn_client_runtime::chat::ChatIntent::Say { text, .. }) if text=="Hello travellers")
    );
    assert_eq!(
        crate::play::mailbox::lock(&link.mailbox).chat.text,
        "Draft stays here"
    );
    assert!(link.assign_bar_action(
        8,
        BarAction::Passive {
            ability: PassiveAbility::BattleInstinct
        }
    ));
    link.cast_spell_slot(NonZeroU32::new(9).ok_or("slot")?);
    assert!(
        commands.try_recv().is_err(),
        "passive assignments must never cast"
    );
    let path = std::env::temp_dir().join(format!("mixed-bars-{}.json", std::process::id()));
    let mut settings = crate::play::SettingsState::new(Some(path.clone()));
    settings.sync_bar_actions(&link);
    let saved = crate::play::SettingsState::new(Some(path.clone()));
    assert_eq!(
        saved.preferences.bar_actions,
        settings.preferences.bar_actions
    );
    assert!(!link.assign_bar_action(
        12,
        BarAction::Text {
            text: "out of bounds".into()
        }
    ));
    assert!(!link.assign_bar_action(0, BarAction::Text { text: "\0".into() }));
    std::fs::remove_file(path)?;
    Ok(())
}

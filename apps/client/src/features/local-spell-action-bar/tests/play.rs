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

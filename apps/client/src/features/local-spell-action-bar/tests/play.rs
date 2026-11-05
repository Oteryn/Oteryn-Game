use super::*;
use oteryn_session::ActorVitals;

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

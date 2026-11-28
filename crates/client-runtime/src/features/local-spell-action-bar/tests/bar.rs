use super::*;
#[test]
fn bindings_keep_cast_indices_and_project_spell_and_group_cooldowns() -> Result<(), &'static str> {
    let mut bar = ActionBar::default();
    let slot = NonZeroU32::new(1).ok_or("slot")?;
    let spell = NonZeroU32::new(71).ok_or("spell")?;
    assert!(bar.bind(0, Some(spell)));
    assert_eq!(bar.spell(slot), Some(spell));
    assert!(bar.bind(11, Some(spell)));
    assert!(!bar.bind(ACTION_BAR_SLOTS, Some(spell)));
    assert!(!bar.bind(0, NonZeroU32::new(1 << 28)));
    use oteryn_session::{ActorCooldownKind, ActorCooldowns};
    let state = ActorCooldowns {
        server_now_ms: 100,
        rtt_ms: None,
        entries: vec![
            oteryn_session::ActorCooldown {
                kind: ActorCooldownKind::Spell,
                id: 71,
                expires_at_ms: 120,
            },
            oteryn_session::ActorCooldown {
                kind: ActorCooldownKind::Group,
                id: 2,
                expires_at_ms: 150,
            },
        ],
    };
    assert_eq!(state.entries[0].kind, ActorCooldownKind::Spell);
    let received_at = Instant::now();
    bar.apply_cooldowns(&ActorCooldownUpdate {
        value: state,
        received_at,
        snapshot: true,
    });
    assert_eq!(bar.remaining_ms_at(spell, &[], received_at), 20);
    assert_eq!(bar.remaining_ms_at(spell, &[2], received_at), 50);
    assert!(bar.bind(0, None));
    assert_eq!(bar.spell(slot), None);
    Ok(())
}

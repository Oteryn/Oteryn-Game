#[test]
fn cooldown_deltas_use_decode_time_and_preserve_other_keys() {
    use oteryn_client_runtime::spell::{
        ActorCooldown, ActorCooldownKind, ActorCooldownUpdate, ActorCooldowns,
    };
    use std::time::{Duration, Instant};

    let mut panel = super::ActionBarPanel::default();
    let decoded = Instant::now();
    let spell = std::num::NonZeroU32::MIN;
    let mut update = ActorCooldownUpdate {
        received_at: decoded,
        snapshot: true,
        value: ActorCooldowns {
            server_now_ms: 100,
            rtt_ms: Some(50),
            entries: vec![ActorCooldown {
                kind: ActorCooldownKind::Group,
                id: 2,
                expires_at_ms: 1100,
            }],
        },
    };
    panel.apply_cooldowns(&update);
    assert_eq!(panel.remaining_ms_at(spell, &[2], decoded), 1_000);
    update.snapshot = false;
    update.received_at = decoded + Duration::from_millis(200);
    update.value.server_now_ms = 0;
    update.value.entries = vec![ActorCooldown {
        kind: ActorCooldownKind::Spell,
        id: spell.get(),
        expires_at_ms: 500,
    }];
    panel.apply_cooldowns(&update);
    let later = decoded + Duration::from_millis(500);
    assert_eq!(panel.remaining_ms_at(spell, &[], later), 200);
    assert_eq!(panel.remaining_ms_at(spell, &[2], later), 500);
    update.value.entries[0].expires_at_ms = 0;
    panel.apply_cooldowns(&update);
    assert_eq!(panel.remaining_ms_at(spell, &[], later), 0);
    assert_eq!(panel.remaining_ms_at(spell, &[2], later), 500);
}

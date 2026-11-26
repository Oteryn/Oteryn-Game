//! Replay the C04 capture through the same cooldown reducer used by the native action bar.

use super::*;
use oteryn_session::{
    ActorCooldown, ActorCooldownKind, ActorCooldowns, SpellCastDisposition, decode_actor_cooldowns,
};
use std::{
    num::NonZeroU32,
    time::{Duration, Instant},
};

#[test]
fn replays_c04_server_cooldowns_and_refusals() -> Result<(), Box<dyn std::error::Error>> {
    let (snapshot, payload, expected) = if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR")
    {
        let path = std::path::Path::new(&path);
        (
            std::fs::read(path.join("c04-snapshot.bin"))?,
            std::fs::read(path.join("c04-cooldowns.bin"))?,
            std::fs::read_to_string(path.join("c04-expected.txt"))?,
        )
    } else {
        let state = ActorCooldowns {
            server_now_ms: 1_000,
            rtt_ms: None,
            entries: vec![
                ActorCooldown {
                    kind: ActorCooldownKind::Spell,
                    id: 71,
                    expires_at_ms: 2_000,
                },
                ActorCooldown {
                    kind: ActorCooldownKind::Group,
                    id: 2,
                    expires_at_ms: 3_000,
                },
                ActorCooldown {
                    kind: ActorCooldownKind::Group,
                    id: 9,
                    expires_at_ms: 4_000,
                },
            ],
        };
        (
            vec![0x08, 0xe8, 0x07],
            encode_fixture(&state),
            "spell=71 groups=2,9 remaining_ms=3000 pz=Rejected cast=Cast\n".into(),
        )
    };
    let fields = expected
        .split_whitespace()
        .map(|field| field.split_once('=').ok_or("invalid C04 expected field"))
        .collect::<Result<Vec<_>, _>>()?;
    let value = |name: &str| {
        fields
            .iter()
            .find_map(|(key, value)| (*key == name).then_some(*value))
            .ok_or("missing C04 expected field")
    };
    let spell = NonZeroU32::new(value("spell")?.parse()?).ok_or("zero C04 spell")?;
    let groups = value("groups")?
        .split(',')
        .map(str::parse)
        .collect::<Result<Vec<u32>, _>>()?;
    let remaining: u64 = value("remaining_ms")?.parse()?;

    let initial = decode_actor_cooldowns(&snapshot).map_err(|error| format!("{error:?}"))?;
    let mut bar = ActionBar::default();
    let received_at = Instant::now();
    bar.apply_cooldowns(&ActorCooldownUpdate {
        value: initial,
        received_at,
        snapshot: true,
    });
    bar.apply_cooldowns(&ActorCooldownUpdate {
        value: decode_actor_cooldowns(&payload).map_err(|error| format!("{error:?}"))?,
        received_at,
        snapshot: false,
    });
    assert_eq!(bar.remaining_ms_at(spell, &groups, received_at), remaining);
    let unrelated = ActorCooldowns {
        server_now_ms: 0,
        rtt_ms: None,
        entries: vec![ActorCooldown {
            kind: ActorCooldownKind::Spell,
            id: if spell.get() == 1 { 2 } else { 1 },
            expires_at_ms: 500,
        }],
    };
    let later = received_at + Duration::from_millis(250);
    bar.apply_cooldowns(&ActorCooldownUpdate {
        value: unrelated,
        received_at: later,
        snapshot: false,
    });
    assert_eq!(
        bar.remaining_ms_at(spell, &groups, later),
        remaining.saturating_sub(250),
    );
    assert_eq!(
        feedback_text(SpellCastDisposition::Rejected),
        "Spells unavailable"
    );
    assert_eq!(feedback_text(SpellCastDisposition::Cast), "Spell cast");
    assert_eq!(value("pz")?, "Rejected");
    assert_eq!(value("cast")?, "Cast");

    let before = bar.remaining_ms_at(spell, &groups, received_at);
    assert!(decode_actor_cooldowns(&[0xff]).is_err());
    assert_eq!(bar.remaining_ms_at(spell, &groups, received_at), before);
    Ok(())
}

fn encode_fixture(state: &ActorCooldowns) -> Vec<u8> {
    fn field(bytes: &mut Vec<u8>, tag: u8, mut value: u64) {
        bytes.push(tag);
        while value >= 0x80 {
            bytes.push((value as u8 & 0x7f) | 0x80);
            value >>= 7;
        }
        bytes.push(value as u8);
    }
    let mut bytes = Vec::new();
    field(&mut bytes, 0x08, state.server_now_ms);
    for entry in &state.entries {
        let mut encoded = Vec::new();
        field(
            &mut encoded,
            0x08,
            match entry.kind {
                ActorCooldownKind::Spell => 1,
                ActorCooldownKind::Group => 2,
            },
        );
        field(&mut encoded, 0x10, u64::from(entry.id));
        field(&mut encoded, 0x18, entry.expires_at_ms);
        bytes.push(0x22);
        field(&mut bytes, encoded.len() as u8, 0);
        bytes.pop();
        bytes.extend(encoded);
    }
    bytes
}

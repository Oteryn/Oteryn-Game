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
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        return replay(
            &std::fs::read(path.join("c04-snapshot.bin"))?,
            &std::fs::read(path.join("c04-cooldowns.bin"))?,
            &std::fs::read_to_string(path.join("c04-expected.txt"))?,
        );
    }
    let initial = encode_fixture(&ActorCooldowns {
        server_now_ms: 1000,
        rtt_ms: None,
        entries: vec![ActorCooldown {
            kind: ActorCooldownKind::Spell,
            id: 1,
            expires_at_ms: 2000,
        }],
    });
    for snapshot in [false, true] {
        replay(
            &initial,
            &encode_fixture(&fixture(snapshot)),
            &expected(snapshot),
        )?;
    }
    Ok(())
}

#[test]
fn c04_replay_rejects_each_missing_or_invalid_cast_cooldown()
-> Result<(), Box<dyn std::error::Error>> {
    let initial = encode_fixture(&fixture(true));
    for snapshot in [false, true] {
        let sample = fixture(snapshot);
        let mut empty = sample.clone();
        empty.entries.clear();
        assert!(replay(&initial, &encode_fixture(&empty), &expected(snapshot)).is_err());
        for index in 0..sample.entries.len() {
            let mut missing = sample.clone();
            missing.entries.remove(index);
            assert!(replay(&initial, &encode_fixture(&missing), &expected(snapshot)).is_err());

            for remaining_ms in [0, 1] {
                let mut invalid = sample.clone();
                invalid.entries[index].expires_at_ms = invalid.server_now_ms + remaining_ms;
                assert!(replay(&initial, &encode_fixture(&invalid), &expected(snapshot)).is_err());
            }
        }
    }
    Ok(())
}

fn fixture(snapshot: bool) -> ActorCooldowns {
    let server_now_ms = if snapshot { 1000 } else { 0 };
    ActorCooldowns {
        server_now_ms,
        rtt_ms: None,
        entries: vec![
            ActorCooldown {
                kind: ActorCooldownKind::Spell,
                id: 71,
                expires_at_ms: server_now_ms + 1000,
            },
            ActorCooldown {
                kind: ActorCooldownKind::Group,
                id: 2,
                expires_at_ms: server_now_ms + 2000,
            },
            ActorCooldown {
                kind: ActorCooldownKind::Group,
                id: 9,
                expires_at_ms: server_now_ms + 3000,
            },
        ],
    }
}

fn expected(snapshot: bool) -> String {
    format!(
        "spell=71:1000 groups=2:2000,9:3000 snapshot={snapshot} elapsed_ms=0 pz=Rejected cast=Cast"
    )
}

fn replay(
    snapshot: &[u8],
    payload: &[u8],
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let parse_cooldown = |field: &str| -> Result<(u32, u64), Box<dyn std::error::Error>> {
        let (id, duration) = field.split_once(':').ok_or("C04 expected id:duration")?;
        Ok((id.parse()?, duration.parse()?))
    };
    let (spell_id, spell_ms) = parse_cooldown(value("spell")?)?;
    let spell = NonZeroU32::new(spell_id).ok_or("zero C04 spell")?;
    let groups = value("groups")?
        .split(',')
        .map(parse_cooldown)
        .collect::<Result<Vec<_>, _>>()?;
    let group_ids = groups.iter().map(|&(id, _)| id).collect::<Vec<_>>();
    let complete: bool = value("snapshot")?.parse()?;
    let elapsed_ms: u64 = value("elapsed_ms")?.parse()?;
    let update = decode_actor_cooldowns(payload).map_err(|error| format!("{error:?}"))?;
    let refs = std::iter::once((ActorCooldownKind::Spell, spell_id, spell_ms)).chain(
        groups
            .iter()
            .map(|&(id, ms)| (ActorCooldownKind::Group, id, ms)),
    );
    let mut timers = Vec::new();
    for (kind, id, duration_ms) in refs {
        let remaining = update
            .entries
            .iter()
            .find(|entry| entry.kind == kind && entry.id == id)
            .map_or(0, |entry| {
                entry.expires_at_ms.saturating_sub(update.server_now_ms)
            });
        let bounds = duration_ms.saturating_sub(elapsed_ms).max(1)
            ..=duration_ms.saturating_add(u64::from(complete));
        if !bounds.contains(&remaining) {
            return Err(format!(
                "C04 cast cooldown {kind:?}/{id}: remaining_ms={remaining}, authored_ms={duration_ms}"
            ).into());
        }
        timers.push((kind == ActorCooldownKind::Group, id, remaining));
    }
    let remaining = timers
        .iter()
        .map(|&(_, _, ms)| ms)
        .max()
        .ok_or("no C04 timers")?;

    let initial = decode_actor_cooldowns(snapshot).map_err(|error| format!("{error:?}"))?;
    let mut bar = ActionBar::default();
    let received_at = Instant::now();
    bar.apply_cooldowns(&ActorCooldownUpdate {
        value: initial.clone(),
        received_at,
        snapshot: true,
    });
    let untouched = initial.entries.iter().filter(|initial| {
        !update
            .entries
            .iter()
            .any(|entry| entry.kind == initial.kind && entry.id == initial.id)
    });
    bar.apply_cooldowns(&ActorCooldownUpdate {
        value: update.clone(),
        received_at,
        snapshot: complete,
    });
    for entry in untouched {
        let remaining = if complete {
            0
        } else {
            entry.expires_at_ms.saturating_sub(initial.server_now_ms)
        };
        assert_eq!(
            bar.cooldown_at(
                entry.kind == ActorCooldownKind::Group,
                entry.id,
                received_at
            )
            .0,
            remaining,
        );
    }
    for &(group, id, ms) in &timers {
        assert_eq!(bar.cooldown_at(group, id, received_at).0, ms);
    }
    assert_eq!(
        bar.remaining_ms_at(spell, &group_ids, received_at),
        remaining
    );
    const ELAPSED_MS: u64 = 250;
    let mut unrelated = if complete {
        let mut complete = update;
        complete.server_now_ms += ELAPSED_MS;
        complete
            .entries
            .retain(|entry| entry.expires_at_ms > complete.server_now_ms);
        complete
    } else {
        ActorCooldowns {
            server_now_ms: 0,
            rtt_ms: None,
            entries: Vec::new(),
        }
    };
    let unrelated_id = if spell.get() == 1 { 2 } else { 1 };
    unrelated
        .entries
        .retain(|entry| entry.kind != ActorCooldownKind::Spell || entry.id != unrelated_id);
    unrelated.entries.push(ActorCooldown {
        kind: ActorCooldownKind::Spell,
        id: unrelated_id,
        expires_at_ms: unrelated.server_now_ms + 500,
    });
    let later = received_at + Duration::from_millis(ELAPSED_MS);
    bar.apply_cooldowns(&ActorCooldownUpdate {
        value: unrelated,
        received_at: later,
        snapshot: complete,
    });
    for (group, id, ms) in timers {
        assert_eq!(
            bar.cooldown_at(group, id, later).0,
            ms.saturating_sub(ELAPSED_MS)
        );
    }
    assert_eq!(
        bar.remaining_ms_at(spell, &group_ids, later),
        remaining.saturating_sub(ELAPSED_MS),
    );
    assert_eq!(
        feedback_text(SpellCastDisposition::Rejected),
        "Spells unavailable"
    );
    assert_eq!(feedback_text(SpellCastDisposition::Cast), "Spell cast");
    assert_eq!(value("pz")?, "Rejected");
    assert_eq!(value("cast")?, "Cast");

    let before = bar.remaining_ms_at(spell, &group_ids, received_at);
    assert!(decode_actor_cooldowns(&[0xff]).is_err());
    assert_eq!(bar.remaining_ms_at(spell, &group_ids, received_at), before);
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

//! Replays C14 payloads through the client runtime consumers: `SkillDisplay::decode` (re-exported
//! from `client-runtime/src/lib.rs`) consumes level, experience, and trained-skill replies, while
//! `replacement` and `CatalogState::replace` in
//! `features/0287-actor-spell-catalog/{protocol,state}.rs` consume automatic-learning snapshots.

use crate::{skill_display, spell_catalog};
use oteryn_session::spell_catalog as wire_catalog;
use oteryn_session::{AppliedDelta, GatedSnapshot, SessionEvent};

type Fixture = (Vec<u8>, Vec<u8>, String);

#[test]
fn e2e_c14_replays_progression_and_automatic_learning() -> Result<(), Box<dyn std::error::Error>> {
    let capture = std::env::var_os("E2E_ALL_CAPTURE_DIR");
    let (display_payload, snapshot, expected) = if let Some(path) = &capture {
        let path = std::path::Path::new(path);
        let meta = std::fs::read_to_string(path.join("c14-spell-catalog-meta.txt"))?;
        let fields: Vec<_> = meta.split_whitespace().collect();
        let [session, sequence, domain, kind, revision] = fields.as_slice() else {
            return Err("captured C14 catalog metadata was malformed".into());
        };
        if session.len() != 36 || sequence.parse::<u64>()? == 0 {
            return Err("captured C14 session or skill reply sequence was malformed".into());
        }
        (
            std::fs::read(path.join("c14-skill-display.bin"))?,
            GatedSnapshot {
                domain_id: domain.parse()?,
                snapshot_type: kind.parse()?,
                revision: revision.parse()?,
                payload: std::fs::read(path.join("c14-spell-catalog.bin"))?,
            },
            std::fs::read_to_string(path.join("c14-expected.txt"))?,
        )
    } else {
        let (display, payload, expected) = fixture()?;
        (
            display,
            GatedSnapshot {
                domain_id: wire_catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG,
                snapshot_type: 1,
                revision: 1,
                payload,
            },
            expected,
        )
    };

    let display = skill_display::SkillDisplay::decode(&display_payload)
        .ok_or("captured C14 skill display was malformed")?;
    let catalog = if capture.is_some() {
        wire_catalog::decode_snapshot(&snapshot)?
    } else {
        let event = SessionEvent::GatedSnapshotReplaced(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: snapshot.clone(),
        });
        spell_catalog::replacement(&event).ok_or("catalog replacement event")??
    };
    let mut state = spell_catalog::CatalogState::default();
    state.replace(catalog);
    assert_eq!(visible(&display, state.entries()), expected);

    let before = state.entries().to_vec();
    let mut invalid = snapshot;
    invalid.payload = vec![0xff];
    assert!(wire_catalog::decode_snapshot(&invalid).is_err());
    if capture.is_none() {
        let event = SessionEvent::GatedSnapshotReplaced(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: invalid,
        });
        assert!(
            spell_catalog::replacement(&event)
                .ok_or("invalid replacement event")?
                .is_err()
        );
    }
    assert_eq!(state.entries(), before);
    if capture.is_some() && (display.level <= 1 || display.experience == 0) {
        return Err("C14 live replay received baseline frames without progression".into());
    }
    Ok(())
}

fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let display = skill_display::SkillDisplay {
        trained: crate::skill_progress::SkillProgressSnapshot {
            skills: std::array::from_fn(|index| crate::skill_progress::SkillProgress {
                level: 10 + u16::try_from(index).unwrap_or_default(),
                progress: u64::try_from(index).unwrap_or_default(),
                required: 100,
                percent: u16::try_from(index).unwrap_or_default() * 100,
            }),
        },
        level: 9,
        experience: 5_000,
        level_percent: 2_500,
        loyalty_bonus_percent: 0,
        loyalty_levels: std::array::from_fn(|index| 10 + u16::try_from(index).unwrap_or_default()),
        current_levels: Some(std::array::from_fn(|index| {
            10 + u32::try_from(index).unwrap_or_default()
        })),
    };
    let entries = vec![spell_catalog::SpellCatalogEntry {
        cast_index: 7,
        name: "Light Healing".into(),
        words: "exura".into(),
        level: 9,
        mana: 20,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![1],
        rune_item: None,
    }];
    let display_payload = display.encode().ok_or("display encode")?;
    let catalog_payload = wire_catalog::encode_catalog(&entries)
        .map_err(|error| format!("catalog encode: {error:?}"))?;
    let expected = visible(&display, &entries);
    Ok((display_payload, catalog_payload, expected))
}

fn visible(
    display: &skill_display::SkillDisplay,
    entries: &[spell_catalog::SpellCatalogEntry],
) -> String {
    let skills = display
        .trained
        .skills
        .iter()
        .map(|skill| {
            format!(
                "{}:{}/{}:{}",
                skill.level, skill.progress, skill.required, skill.percent
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let learned = entries
        .iter()
        .filter(|entry| entry.learned)
        .map(|entry| entry.cast_index.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "level={} experience={} level_percent={} skills={} learned={}\n",
        display.level, display.experience, display.level_percent, skills, learned
    )
}

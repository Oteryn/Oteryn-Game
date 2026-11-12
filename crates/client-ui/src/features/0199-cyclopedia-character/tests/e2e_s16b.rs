//! SCENARIO_MATRIX S16b replay coverage: the changed-vocation statistics a fresh admission
//! re-queries. The owning client seams are `0199-cyclopedia-character/protocol.rs`
//! (`request` + `apply_result` for the GeneralStats tab) and `0279-skill-progress`
//! (`request` + `SkillProgressPanel::finish`). The expected numbers are the server-observed
//! values the capture phase wrote to `s16b-expected.txt`, never the client's own rendering.

use super::general_stats_selection as selection;

use super::{CyclopediaCharacterPanel, Intent, apply_result, panel_text, request};
use crate::skill_progress::{self, SkillProgress, SkillProgressPanel, SkillProgressSnapshot};
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{CharacterTab, GeneralStats, TabView},
    raw_feature::GatedCommandResult,
};
use std::collections::BTreeMap;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Inline fixture: a level-8 knight with distinct per-field values.
const FIXTURE_EXPECTED: &str = "xp=4200\nlevel=8\nmagic=3\nfist=11\nclub=12\nsword=13\naxe=14\ndistance=15\nshielding=16\nfishing=17\nstamina=2520\noffline=90\nknown=1\nhealth=185\nmana=90\nskill0=3,5,40,1250\nskill1=11,1,10,1000\nskill2=12,2,10,2000\nskill3=13,3,10,3000\nskill4=14,4,10,4000\nskill5=15,5,10,5000\nskill6=16,6,10,6000\nskill7=17,7,10,7000\n";

fn fixture() -> TestResult<(Vec<u8>, Vec<u8>, String)> {
    let values = FIXTURE_EXPECTED
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect::<BTreeMap<_, _>>();
    let number = |key: &str| -> TestResult<u64> {
        Ok(values.get(key).ok_or("fixture key absent")?.parse()?)
    };
    let stats = TabView::GeneralStats(GeneralStats {
        experience: number("xp")?,
        level: u32::try_from(number("level")?)?,
        magic_level: u16::try_from(number("magic")?)?,
        skills: std::array::from_fn(|i| {
            [
                "fist",
                "club",
                "sword",
                "axe",
                "distance",
                "shielding",
                "fishing",
            ]
            .get(i)
            .and_then(|key| values.get(*key))
            .and_then(|value| value.parse().ok())
            .unwrap_or(0)
        }),
        stamina_minutes: u16::try_from(number("stamina")?)?,
        offline_training_minutes: u16::try_from(number("offline")?)?,
        last_vitals: Some((
            u32::try_from(number("health")?)?,
            u32::try_from(number("mana")?)?,
        )),
    })
    .encode()
    .map_err(|_| "general stats encode")?;
    let mut skills = [SkillProgress::default(); 8];
    for (index, skill) in skills.iter_mut().enumerate() {
        let parts: Vec<u64> = values
            .get(&format!("skill{index}"))
            .ok_or("fixture skill absent")?
            .split(',')
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        let [level, progress, required, percent] = parts.as_slice() else {
            return Err("fixture skill fields".into());
        };
        *skill = SkillProgress {
            level: u16::try_from(*level)?,
            progress: *progress,
            required: *required,
            percent: u16::try_from(*percent)?,
        };
    }
    let snapshot = SkillProgressSnapshot { skills }
        .encode()
        .ok_or("skill snapshot encode")?;
    Ok((stats, snapshot, FIXTURE_EXPECTED.to_owned()))
}

fn reply(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    }
}

#[test]
fn e2e_s16b_changed_vocation_statistics_replay() -> TestResult {
    let (stats, skills, expected, selected) =
        if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let directory = std::path::Path::new(&directory);
            (
                std::fs::read(directory.join("s16b-stats.bin"))?,
                std::fs::read(directory.join("s16b-skills.bin"))?,
                std::fs::read_to_string(directory.join("s16b-expected.txt"))?,
                selection::read(directory, "s16b-stats.bin")?,
            )
        } else {
            let (stats, skills, expected) = fixture()?;
            (stats, skills, expected, selection::legacy())
        };
    let value = |key: &str| -> TestResult<&str> {
        expected
            .lines()
            .find_map(|line| {
                line.strip_prefix(key)
                    .and_then(|rest| rest.strip_prefix('='))
            })
            .ok_or_else(|| format!("s16b expected lacks {key}").into())
    };

    // GeneralStats tab through the cyclopedia request/apply path.
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::GeneralStats,
        extensions: selected.extensions,
        ..Default::default()
    };
    request(&mut panel, Intent::Tab).ok_or("general stats request")?;
    assert!(apply_result(&mut panel, &reply(stats.clone())));
    selection::check(&panel, selected);
    let minutes: u16 = value("stamina")?.parse()?;
    let mut lines = vec![
        "GeneralStats".to_owned(),
        format!(
            "Experience: {}\nLevel: {}\nMagic level: {}",
            value("xp")?,
            value("level")?,
            value("magic")?
        ),
    ];
    for (label, key) in [
        ("Fist", "fist"),
        ("Club", "club"),
        ("Sword", "sword"),
        ("Axe", "axe"),
        ("Distance", "distance"),
        ("Shielding", "shielding"),
        ("Fishing", "fishing"),
    ] {
        lines.push(format!("{label}: {}", value(key)?));
    }
    lines.push(format!(
        "Stamina: {}:{:02} (hours:minutes)\nOffline training: {} minutes",
        minutes / 60,
        minutes % 60,
        value("offline")?
    ));
    lines.push(format!(
        "Health: {} (last saved)\nMana: {} (last saved)",
        value("health")?,
        value("mana")?
    ));
    if let Some(id) = selected.vocation {
        let title = crate::cyclopedia_character_vocation::vocation_title(id)
            .ok_or("invalid captured vocation")?;
        lines.push(format!("Vocation: {title}"));
    }
    assert_eq!(value("known")?, "1");
    assert_eq!(panel.result.as_deref(), Some(lines.join("\n").as_str()));
    assert!(panel_text(&panel).ends_with(&lines.join("\n")));

    // A refused query and a truncated payload leave the shown statistics unchanged.
    let shown = panel.result.clone();
    request(&mut panel, Intent::Tab).ok_or("second request")?;
    assert!(!apply_result(
        &mut panel,
        &GatedCommandResult {
            status: CommandStatus::Rejected,
            ..reply(Vec::new())
        }
    ));
    assert_eq!(
        panel.feedback,
        Some("The server refused the character query")
    );
    assert_eq!(panel.result, shown);
    request(&mut panel, Intent::Tab).ok_or("third request")?;
    assert!(!apply_result(&mut panel, &reply(stats[..41].to_vec())));
    assert_eq!(panel.result, shown);

    // SkillProgress through the skill-progress request/finish path.
    let mut trained = SkillProgressPanel {
        selected: true,
        ..Default::default()
    };
    skill_progress::request(&mut trained).ok_or("skill progress request")?;
    assert!(trained.finish(true, &skills));
    let text = skill_progress::panel_text(&trained);
    for (index, label) in [
        "Magic",
        "Fist",
        "Club",
        "Sword",
        "Axe",
        "Distance",
        "Shielding",
        "Fishing",
    ]
    .iter()
    .enumerate()
    {
        let parts: Vec<u64> = value(&format!("skill{index}"))?
            .split(',')
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        let [level, progress, required, percent] = parts.as_slice() else {
            return Err("expected skill fields".into());
        };
        let filled = usize::try_from(percent / 1_000)?;
        let line = format!(
            "{label}: {level} [{}{}] {}.{:02}% ({progress}/{required})\r\n",
            "#".repeat(filled),
            "-".repeat(10 - filled),
            percent / 100,
            percent % 100
        );
        assert!(text.contains(&line), "skill panel lacks {line:?}");
    }
    let snapshot = trained.snapshot;
    skill_progress::request(&mut trained).ok_or("second skill request")?;
    assert!(!trained.finish(true, &[0]));
    assert_eq!(trained.snapshot, snapshot);
    Ok(())
}

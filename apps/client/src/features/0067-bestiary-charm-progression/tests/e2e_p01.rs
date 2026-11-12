//! P01 replay: server CHARM_PROGRESSION replies applied by the production client reducer.
//!
//! The consumer is `apply` in `apps/client/src/features/0067-bestiary-charm-progression/protocol.rs`;
//! the request is built by the production `command`, so the reducer sees the exact kind the app
//! sends. The replies are the QUERY before a hunt, a refused ASSIGN of a locked charm, and the
//! QUERY after one credited kill.
use super::*;
use oteryn_client_runtime::raw_feature::CommandStatus;
use wire::{
    bestiary::BestiaryRaceProgress,
    charm::{CharmKind, CharmState, CharmView},
};

struct Replies {
    before: Vec<u8>,
    refused: Vec<u8>,
    after: Vec<u8>,
    race: u32,
    kills_before: u32,
    kills_after: u32,
    points_before: u32,
    charm: u32,
}

fn view(kills: u32, points: u32) -> Result<wire::CharmProgressionReply, Box<dyn std::error::Error>> {
    let nz = |value: u32| NonZeroU32::new(value).ok_or("zero");
    Ok(wire::CharmProgressionReply {
        outcome: wire::CharmProgressionOutcome::Query,
        fee_gold: 0,
        bestiary: vec![BestiaryRaceProgress {
            race: nz(3)?,
            kill_count: kills,
            kill_thresholds: [10, 100, 500],
        }],
        charms: CharmView {
            charms: vec![CharmState {
                charm: nz(1)?,
                kind: CharmKind::Major,
                unlocked_stage: 0,
                assigned_race: None,
                next_stage_cost: 5,
                effect_active: false,
            }],
            charm_points_available: points,
            minor_charm_echoes_available: 0,
            assignment_slot_limit: Some(nz(2)?),
        },
    })
}

fn replies() -> Result<Replies, Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        let expected = std::fs::read_to_string(path.join("p01-expected.txt"))?;
        let value = |key: &str| -> Result<u32, Box<dyn std::error::Error>> {
            Ok(expected
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{key}=")))
                .ok_or("expected capture line absent")?
                .parse()?)
        };
        return Ok(Replies {
            before: std::fs::read(path.join("p01-before.bin"))?,
            refused: std::fs::read(path.join("p01-refused.bin"))?,
            after: std::fs::read(path.join("p01-after.bin"))?,
            race: value("race")?,
            kills_before: value("kills_before")?,
            kills_after: value("kills_after")?,
            points_before: value("points_before")?,
            charm: value("locked_charm")?,
        });
    }
    let refused = wire::CharmProgressionReply {
        outcome: wire::CharmProgressionOutcome::Assign(wire::charm::CharmAssignDisposition::CharmLocked),
        ..view(4, 7)?
    };
    Ok(Replies {
        before: wire::encode_charm_progression_reply(&view(4, 7)?)
            .map_err(|error| format!("{error:?}"))?,
        refused: wire::encode_charm_progression_reply(&refused)
            .map_err(|error| format!("{error:?}"))?,
        after: wire::encode_charm_progression_reply(&view(5, 7)?)
            .map_err(|error| format!("{error:?}"))?,
        race: 3,
        kills_before: 4,
        kills_after: 5,
        points_before: 7,
        charm: 1,
    })
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

fn pending(panel: &Panel, intent: Intent) -> Result<(Panel, FeatureCommand), String> {
    let command = command(panel, intent)?;
    let mut next = panel.clone();
    next.selected = true;
    assert!(next.begin(command.kind));
    Ok((next, command))
}

fn race_line(panel: &Panel, race: u32) -> Result<String, String> {
    panel
        .summary
        .lines()
        .find(|line| line.starts_with(&format!("Race {race}: ")))
        .map(str::to_owned)
        .ok_or_else(|| format!("no Race {race} line in {:?}", panel.summary))
}

#[test]
fn e2e_p01_replays_query_refusal_and_kill_delta() -> Result<(), Box<dyn std::error::Error>> {
    let replies = replies()?;
    let before = wire::decode_charm_progression_reply(&replies.before)
        .map_err(|error| format!("{error:?}"))?;
    let entry = before
        .bestiary
        .iter()
        .find(|entry| entry.race.get() == replies.race)
        .ok_or("race absent from QUERY")?;
    assert_eq!(entry.kill_count, replies.kills_before);
    let thresholds = entry.kill_thresholds;
    let line = |kills: u32| {
        let stage = thresholds.iter().filter(|t| kills >= **t).count();
        let next = thresholds.iter().find(|t| kills < **t);
        format!(
            "Race {}: {kills} kills, stage {stage}, thresholds {thresholds:?}, next {next:?}",
            replies.race
        )
    };

    let mut panel = Panel::default();
    let (mut loading, query) = pending(&panel, Intent::Query)?;
    assert_eq!(
        (query.capability, query.kind),
        (
            wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
            wire::COMMAND_TYPE_CHARM_PROGRESSION
        )
    );
    apply(&mut loading, &query, &result(CommandStatus::Accepted, &replies.before));
    assert!(!loading.pending);
    assert_eq!(loading.feedback, "Completed: Query; fee: 0 gold");
    assert_eq!(race_line(&loading, replies.race)?, line(replies.kills_before));
    assert!(loading.summary.starts_with(&format!(
        "Charm points: {}; ",
        replies.points_before
    )));
    panel = loading;

    // A refused ASSIGN reports the refusal and leaves the charm locked.
    panel.fields = [replies.charm.to_string(), replies.race.to_string()];
    let (mut refusing, assign) = pending(&panel, Intent::Assign)?;
    apply(&mut refusing, &assign, &result(CommandStatus::Accepted, &replies.refused));
    assert_eq!(
        refusing.feedback,
        "Server refused: Assign(CharmLocked); fee: 0 gold"
    );
    let reply = refusing.reply.as_ref().ok_or("refused reply not stored")?;
    let state = reply
        .charms
        .charms
        .iter()
        .find(|state| state.charm.get() == replies.charm)
        .ok_or("locked charm absent")?;
    assert_eq!((state.unlocked_stage, state.assigned_race), (0, None));
    assert_eq!(race_line(&refusing, replies.race)?, line(replies.kills_before));

    // The QUERY after the credited kill shows exactly one more kill.
    let (mut hunted, query) = pending(&refusing, Intent::Query)?;
    apply(&mut hunted, &query, &result(CommandStatus::Accepted, &replies.after));
    assert_eq!(hunted.feedback, "Completed: Query; fee: 0 gold");
    assert_eq!(race_line(&hunted, replies.race)?, line(replies.kills_after));
    assert_eq!(replies.kills_after, replies.kills_before + 1);

    // An undecodable payload and a rejected status change neither summary nor reply.
    for (status, payload, message) in [
        (CommandStatus::Accepted, vec![0xff, 0x00], "Invalid charm reply"),
        (
            CommandStatus::Rejected,
            replies.before.clone(),
            "The server refused the charm request",
        ),
    ] {
        let (mut bad, query) = pending(&hunted, Intent::Query)?;
        apply(&mut bad, &query, &result(status, &payload));
        assert!(!bad.pending);
        assert_eq!(bad.feedback, message);
        assert_eq!(bad.summary, hunted.summary);
        assert_eq!(bad.reply, hunted.reply);
    }
    Ok(())
}

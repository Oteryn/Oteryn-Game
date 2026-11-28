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
    pre_mastery: Option<GatedCommandResult>,
    refusal_readback: Option<GatedCommandResult>,
    initial: Option<GatedCommandResult>,
    before: GatedCommandResult,
    refused: GatedCommandResult,
    after: GatedCommandResult,
    durable: Option<GatedCommandResult>,
    race: u32,
    kills_before: u32,
    kills_after: u32,
    points_before: u32,
    charm: u32,
    refusal_race: u32,
}

fn view(
    kills: u32,
    points: u32,
) -> Result<wire::CharmProgressionReply, Box<dyn std::error::Error>> {
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

type Error = Box<dyn std::error::Error>;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    command_id: u64,
    game_session_id: String,
    payload: Vec<u8>,
    server_sequence: u64,
    status: u32,
}

fn captured_reply(
    path: &std::path::Path,
    session: &str,
    previous: &mut (u64, u64),
    expected: &str,
) -> Result<GatedCommandResult, Error> {
    let frame: Envelope = serde_json::from_slice(&std::fs::read(path)?)?;
    let parts: Vec<_> = session.split('-').collect();
    let uuid_v7 = parts.len() == 5
        && parts.iter().zip([8, 4, 4, 4, 12]).all(|(part, width)| {
            part.len() == width && part.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        && parts[2].starts_with('7')
        && matches!(
            parts[3].as_bytes()[0].to_ascii_lowercase(),
            b'8' | b'9' | b'a' | b'b'
        );
    if !uuid_v7 || frame.game_session_id != session {
        return Err(format!("{}: unexpected game session", path.display()).into());
    }
    let (command_id, sequence) = expected
        .split_once(':')
        .ok_or("expected command ID and sequence")?;
    let position = (frame.command_id, frame.server_sequence);
    if position != (command_id.parse::<u64>()?, sequence.parse::<u64>()?) {
        return Err(format!(
            "{}: command ID or sequence differs from live capture",
            path.display()
        )
        .into());
    }
    if frame.command_id <= previous.0 || frame.server_sequence <= previous.1 {
        return Err(format!("{}: command ID or sequence did not advance", path.display()).into());
    }
    if frame.status != CommandStatus::Accepted as u32 || frame.payload.is_empty() {
        return Err(format!(
            "{}: expected an accepted reply with payload",
            path.display()
        )
        .into());
    }
    *previous = position;
    Ok(GatedCommandResult {
        command_id: frame.command_id,
        status: CommandStatus::Accepted,
        server_sequence: frame.server_sequence,
        payload: frame.payload,
    })
}

fn replies() -> Result<Replies, Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        let expected = std::fs::read_to_string(path.join("p01-expected.txt"))?;
        let field = |key: &str| -> Result<&str, Error> {
            expected
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{key}=")))
                .ok_or_else(|| format!("expected capture line {key} absent").into())
        };
        let session = field("session")?;
        let durable_session = field("durable_session")?;
        if session == durable_session {
            return Err("relog capture reused the original session".into());
        }
        let mut previous = (0, 0);
        let mut original =
            |name| captured_reply(&path.join(name), session, &mut previous, field(name)?);
        let durable = "p01-durable.json";
        return Ok(Replies {
            pre_mastery: Some(original("p01-pre-mastery.json")?),
            initial: Some(original("p01-initial.json")?),
            before: original("p01-before.json")?,
            refused: original("p01-refused.json")?,
            refusal_readback: Some(original("p01-refusal-readback.json")?),
            after: original("p01-after.json")?,
            durable: Some(captured_reply(
                &path.join(durable),
                durable_session,
                &mut (0, 0),
                field(durable)?,
            )?),
            race: field("race")?.parse()?,
            kills_before: field("kills_before")?.parse()?,
            kills_after: field("kills_after")?.parse()?,
            points_before: field("points_before")?.parse()?,
            charm: field("locked_charm")?.parse()?,
            refusal_race: field("refusal_race")?.parse()?,
        });
    }
    let refused = wire::CharmProgressionReply {
        outcome: wire::CharmProgressionOutcome::Assign(
            wire::charm::CharmAssignDisposition::CharmLocked,
        ),
        ..view(4, 7)?
    };
    let fixture_reply = |reply: &wire::CharmProgressionReply| -> Result<_, Error> {
        let payload =
            wire::encode_charm_progression_reply(reply).map_err(|error| format!("{error:?}"))?;
        Ok(result(CommandStatus::Accepted, &payload))
    };
    Ok(Replies {
        pre_mastery: None,
        refusal_readback: None,
        initial: None,
        before: fixture_reply(&view(4, 7)?)?,
        refused: fixture_reply(&refused)?,
        after: fixture_reply(&view(5, 7)?)?,
        durable: None,
        race: 3,
        kills_before: 4,
        kills_after: 5,
        points_before: 7,
        charm: 1,
        refusal_race: 3,
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
    let before = wire::decode_charm_progression_reply(&replies.before.payload)
        .map_err(|error| format!("{error:?}"))?;
    let entry = before
        .bestiary
        .iter()
        .find(|entry| entry.race.get() == replies.race)
        .ok_or("race absent from QUERY")?;
    assert_eq!(entry.kill_count, replies.kills_before);
    let thresholds = if replies.initial.is_some() {
        [10, 100, 250]
    } else {
        [10, 100, 500]
    };
    assert_eq!(entry.kill_thresholds, thresholds);
    let line = |kills: u32| {
        let stage = thresholds.iter().filter(|t| kills >= **t).count();
        let next = thresholds.iter().find(|t| kills < **t);
        format!(
            "Race {}: {kills} kills, stage {stage}, thresholds {thresholds:?}, next {next:?}",
            replies.race
        )
    };

    let mut panel = Panel::default();
    if let Some(initial) = &replies.initial {
        let pre_mastery = replies
            .pre_mastery
            .as_ref()
            .ok_or("pre-mastery capture missing")?;
        let (mut seeded, query) = pending(&panel, Intent::Query)?;
        apply(&mut seeded, &query, pre_mastery);
        let mut expected_mastery = seeded.reply.ok_or("pre-mastery Query missing")?;
        assert_eq!(expected_mastery.charms.charm_points_available, 0);
        let [troll] = expected_mastery.bestiary.as_mut_slice() else {
            return Err("expected only the incomplete Troll entry".into());
        };
        assert_eq!(
            (troll.race.get(), troll.kill_count, troll.kill_thresholds),
            (replies.refusal_race, 249, [10, 100, 250])
        );
        troll.kill_count = 250;
        expected_mastery.charms.charm_points_available = 5;
        assert_eq!(replies.points_before, 5);
        // The canonical key-ordered charm catalogue places Enflame at wire index 9.
        assert_eq!(replies.charm, 9);
        let (mut admitted, query) = pending(&panel, Intent::Query)?;
        apply(&mut admitted, &query, initial);
        assert_eq!(admitted.feedback, "Completed: Query; fee: 0 gold");
        let initial = admitted.reply.as_ref().ok_or("initial Query missing")?;
        assert_eq!(initial, &expected_mastery);
        assert!(
            initial
                .bestiary
                .iter()
                .all(|entry| { entry.race.get() != replies.race || entry.kill_count == 0 })
        );
        assert_eq!((replies.kills_before, replies.kills_after), (1, 2));
        let completed = initial
            .bestiary
            .iter()
            .find(|entry| entry.race.get() == replies.refusal_race && entry.is_complete())
            .ok_or("completed refusal race is absent from initial Query")?;
        assert_ne!(completed.race.get(), replies.race);
        let mut expected = initial.clone();
        expected.bestiary.push(BestiaryRaceProgress {
            race: NonZeroU32::new(replies.race).ok_or("zero Rat race")?,
            kill_count: 1,
            kill_thresholds: thresholds,
        });
        expected.bestiary.sort_by_key(|entry| entry.race);
        assert_eq!(before, expected);
        panel = admitted;
    }
    let (mut loading, query) = pending(&panel, Intent::Query)?;
    assert_eq!(
        (query.capability, query.kind),
        (
            wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
            wire::COMMAND_TYPE_CHARM_PROGRESSION
        )
    );
    apply(&mut loading, &query, &replies.before);
    assert!(!loading.pending);
    assert_eq!(loading.feedback, "Completed: Query; fee: 0 gold");
    assert_eq!(
        race_line(&loading, replies.race)?,
        line(replies.kills_before)
    );
    assert!(
        loading
            .summary
            .starts_with(&format!("Charm points: {}; ", replies.points_before))
    );
    panel = loading;

    // A refused ASSIGN reports the refusal and leaves the charm locked.
    panel.fields = [replies.charm.to_string(), replies.refusal_race.to_string()];
    let (mut refusing, assign) = pending(&panel, Intent::Assign)?;
    assert_eq!(
        (assign.capability, assign.kind),
        (
            wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
            wire::COMMAND_TYPE_CHARM_PROGRESSION
        )
    );
    assert_eq!(
        wire::decode_charm_progression_request(&assign.payload)
            .map_err(|error| format!("{error:?}"))?,
        wire::CharmProgressionAction::Assign {
            charm: NonZeroU32::new(replies.charm).ok_or("zero charm")?,
            race: NonZeroU32::new(replies.refusal_race).ok_or("zero refusal race")?,
        }
    );
    apply(&mut refusing, &assign, &replies.refused);
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
    if replies.initial.is_some() {
        assert_eq!(
            (state.kind, state.next_stage_cost, state.effect_active),
            (CharmKind::Major, 400, true)
        );
    }
    assert_eq!(
        race_line(&refusing, replies.race)?,
        line(replies.kills_before)
    );

    if let Some(readback) = &replies.refusal_readback {
        let (mut independent, query) = pending(&refusing, Intent::Query)?;
        apply(&mut independent, &query, readback);
        assert_eq!(independent.feedback, "Completed: Query; fee: 0 gold");
        assert_eq!(independent.reply.as_ref(), Some(&before));
    }

    // The QUERY after the credited kill shows exactly one more kill.
    let (mut hunted, query) = pending(&refusing, Intent::Query)?;
    apply(&mut hunted, &query, &replies.after);
    assert_eq!(hunted.feedback, "Completed: Query; fee: 0 gold");
    assert_eq!(race_line(&hunted, replies.race)?, line(replies.kills_after));
    assert_eq!(replies.kills_after, replies.kills_before + 1);

    assert_eq!(reply.charms, before.charms);
    assert_eq!(reply.bestiary, before.bestiary);
    let mut expected = before;
    expected
        .bestiary
        .iter_mut()
        .find(|entry| entry.race.get() == replies.race)
        .ok_or("Rat absent before second hunt")?
        .kill_count += 1;
    assert_eq!(hunted.reply.as_ref(), Some(&expected));
    if let Some(durable) = &replies.durable {
        let (mut rejoined, query) = pending(&Panel::default(), Intent::Query)?;
        apply(&mut rejoined, &query, durable);
        assert_eq!(rejoined.feedback, "Completed: Query; fee: 0 gold");
        assert_eq!(
            race_line(&rejoined, replies.race)?,
            line(replies.kills_after)
        );
        assert_eq!(rejoined.reply.as_ref(), Some(&expected));
    }

    // An undecodable payload and a rejected status change neither summary nor reply.
    for (status, payload, message) in [
        (
            CommandStatus::Accepted,
            vec![0xff, 0x00],
            "Invalid charm reply",
        ),
        (
            CommandStatus::Rejected,
            replies.before.payload.clone(),
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

#[test]
#[ignore = "requires fresh P01.full live captures"]
fn e2e_p01_full_replays_native_charm_journey() -> Result<(), Error> {
    use wire::CharmProgressionOutcome::{Assign, Query, Unlock};
    use wire::charm::{CharmAssignDisposition, CharmUnlockDisposition};

    let directory = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("P01.full requires fresh captures")?;
    let path = std::path::PathBuf::from(directory);
    let manifest = std::fs::read_to_string(path.join("p01-full-expected.txt"))?;
    let field = |key: &str| -> Result<&str, Error> {
        manifest
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .ok_or_else(|| format!("P01.full capture field {key} absent").into())
    };
    let index = |key: &str| -> Result<NonZeroU32, Error> {
        NonZeroU32::new(field(key)?.parse()?).ok_or_else(|| format!("zero {key}").into())
    };
    let session = field("session")?;
    let durable_session = field("durable_session")?;
    assert_ne!(session, durable_session);
    let rat = index("rat")?;
    let troll = index("troll")?;
    let incomplete = index("incomplete")?;
    let charm = index("charm")?;
    assert_eq!(charm.get(), 9, "canonical Enflame wire index");
    let mut previous = (0, 0);
    let mut panel = Panel::default();
    let mut expected = None;
    for (suffix, intent, selected, outcome) in [
        ("initial", Intent::Query, rat, Query),
        ("mastery", Intent::Query, rat, Query),
        (
            "locked",
            Intent::Assign,
            rat,
            Assign(CharmAssignDisposition::CharmLocked),
        ),
        ("locked-readback", Intent::Query, rat, Query),
        (
            "unlocked",
            Intent::Unlock,
            rat,
            Unlock(CharmUnlockDisposition::Unlocked),
        ),
        (
            "incomplete",
            Intent::Assign,
            incomplete,
            Assign(CharmAssignDisposition::RaceStageTooLow),
        ),
        ("incomplete-readback", Intent::Query, incomplete, Query),
        (
            "assigned",
            Intent::Assign,
            troll,
            Assign(CharmAssignDisposition::Assigned),
        ),
        ("assigned-readback", Intent::Query, troll, Query),
        ("after-hook", Intent::Query, troll, Query),
        ("durable", Intent::Query, troll, Query),
    ] {
        let filename = format!("p01-full-{suffix}.json");
        let capture_session = if suffix == "durable" {
            previous = (0, 0);
            panel = Panel::default();
            durable_session
        } else {
            session
        };
        let original = captured_reply(
            &path.join(&filename),
            capture_session,
            &mut previous,
            field(&filename)?,
        )?;
        panel.fields[0] = charm.to_string();
        panel.fields[1] = selected.to_string();
        let (mut target, command) = pending(&panel, intent)?;
        apply(&mut target, &command, &original);
        let reply = target
            .reply
            .as_ref()
            .ok_or("client did not apply the original reply")?;
        assert_eq!(reply.outcome, outcome);
        assert_eq!(reply.fee_gold, 0);
        if suffix == "initial" {
            assert_eq!(reply.charms.charm_points_available, 400);
            assert!(
                reply
                    .bestiary
                    .iter()
                    .any(|entry| entry.race == rat && entry.kill_count == 249
                        && entry.kill_thresholds == [10, 100, 250] && !entry.is_complete())
            );
            assert!(
                reply
                    .bestiary
                    .iter()
                    .any(|entry| entry.race == troll && entry.is_complete())
            );
            assert!(reply.bestiary.iter().any(|entry| entry.race == incomplete
                && entry.kill_count == 1
                && !entry.is_complete()));
            assert!(reply.charms.charms.iter().any(|entry| entry.charm == charm
                && entry.unlocked_stage == 0
                && entry.next_stage_cost == 400
                && entry.assigned_race.is_none()));
            expected = Some(reply.clone());
        }
        let state = expected.as_mut().ok_or("initial native reply absent")?;
        state.outcome = outcome;
        match suffix {
            "mastery" => {
                assert!(reply.bestiary.iter().any(|entry| entry.race == rat
                    && entry.kill_count == 250 && entry.kill_thresholds == [10, 100, 250]
                    && entry.is_complete()));
                state.charms.charm_points_available = 405;
                state
                    .bestiary
                    .iter_mut()
                    .find(|entry| entry.race == rat)
                    .ok_or("Rat entry absent")?
                    .kill_count = 250;
            }
            "unlocked" => {
                state.charms.charm_points_available = 5;
                state.charms.minor_charm_echoes_available = state
                    .charms
                    .minor_charm_echoes_available
                    .checked_add(50)
                    .ok_or("minor charm echo balance overflow")?;
                let entry = state
                    .charms
                    .charms
                    .iter_mut()
                    .find(|entry| entry.charm == charm)
                    .ok_or("Enflame entry absent")?;
                entry.unlocked_stage = 1;
                entry.next_stage_cost = 600;
            }
            "assigned" => {
                let entry = state
                    .charms
                    .charms
                    .iter_mut()
                    .find(|entry| entry.charm == charm)
                    .ok_or("Enflame entry absent")?;
                entry.assigned_race = Some(troll);
                entry.effect_active = true;
            }
            _ => {}
        }
        assert_eq!(reply, state, "unexpected state after {suffix}");
        panel = target;
    }
    Ok(())
}

//! S04c replay owner: `Session::chat` (`session_commands.rs`) decodes the CHAT command result into
//! `ChatOutcome`, and `Session::dispatch_delta` (`domain_dispatch.rs`) applies the CHAT room-set
//! and line deltas to `Session::chat_log`. Neither retains the raw bytes, so the capture phase
//! records the server-observed outcome of every step in `s04c-expected.txt` (`step status
//! disposition wait speaker`) and this replay encodes each with the production chat encoders
//! before it crosses the real session path. The asserts are on what the session then exposes:
//! the outcome, the open-room set and the exact retained lines.
use super::*;
use crate::tests::{BoxError, block_on};

const FIXTURE: &str = "open-world\tAccepted\tOk\t0\t-\nopen-advertising\tAccepted\tOk\t0\t-\nline-world\tAccepted\tOk\t0\tAda\nline-advertising\tAccepted\tOk\t0\tAda\nexhausted-advertising\tAccepted\tExhausted\t120\t-\nclose-world\tAccepted\tOk\t0\t-\nclosed-world\tAccepted\tRoomNotOpen\t0\t-\nreopen-world\tAccepted\tOk\t0\t-\nmuted-world\tAccepted\tMuted\t45\t-\n";

struct Observed {
    status: CommandStatus,
    disposition: ChatDisposition,
    wait: u32,
    speaker: String,
}

fn parse(expected: &str) -> Result<std::collections::BTreeMap<String, Observed>, BoxError> {
    let mut steps = std::collections::BTreeMap::new();
    for line in expected.lines() {
        let [step, status, disposition, wait, speaker] = line.split('\t').collect::<Vec<_>>()[..]
        else {
            return Err(format!("s04c expected line malformed: {line:?}").into());
        };
        steps.insert(
            step.to_owned(),
            Observed {
                status: match status {
                    "Accepted" => CommandStatus::Accepted,
                    "Rejected" => CommandStatus::Rejected,
                    other => return Err(format!("s04c unknown status {other}").into()),
                },
                disposition: match disposition {
                    "Ok" => ChatDisposition::Ok,
                    "Exhausted" => ChatDisposition::Exhausted,
                    "RoomNotOpen" => ChatDisposition::RoomNotOpen,
                    "Muted" => ChatDisposition::Muted,
                    other => return Err(format!("s04c unknown disposition {other}").into()),
                },
                wait: wait.parse()?,
                speaker: speaker.to_owned(),
            },
        );
    }
    Ok(steps)
}

fn step<'a>(
    steps: &'a std::collections::BTreeMap<String, Observed>,
    name: &str,
) -> Result<&'a Observed, BoxError> {
    steps
        .get(name)
        .ok_or_else(|| format!("s04c expected lacks step {name}").into())
}

fn room_line_for(room: ChatRoom, speaker: &str, text: &str) -> ChatLine {
    ChatLine::Room {
        room,
        speaker_name: speaker.to_owned(),
        text: text.to_owned(),
    }
}

#[test]
fn e2e_s04c_public_channels_and_mute_replay() -> Result<(), BoxError> {
    block_on(async {
        let expected = match std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            Some(dir) => {
                std::fs::read_to_string(std::path::Path::new(&dir).join("s04c-expected.txt"))?
            }
            None => FIXTURE.to_owned(),
        };
        let steps = parse(&expected)?;
        let (world, ad) = ("S04c world", "S04c advertising");
        let world_line =
            room_line_for(ChatRoom::World, &step(&steps, "line-world")?.speaker, world);
        let ad_line = room_line_for(
            ChatRoom::Advertising,
            &step(&steps, "line-advertising")?.speaker,
            ad,
        );
        let both = room_set(&[ChatRoom::World, ChatRoom::Advertising]);
        let only_ad = room_set(&[ChatRoom::Advertising]);
        // One entry per player action, in the order the session sends them: the intent, the
        // observed step name and the pushed delta (if any) the server follows its result with.
        let rooms_world = room_set(&[ChatRoom::World]);
        let say = |room, text: &str| ChatIntent::Room {
            room,
            text: text.into(),
        };
        type Push = Option<(u8, ChatRoomSet, Option<ChatLine>)>;
        let plan: Vec<(&str, ChatIntent, Push)> = vec![
            (
                "open-world",
                ChatIntent::OpenRoom(ChatRoom::World),
                Some((0, rooms_world, None)),
            ),
            (
                "line-world",
                say(ChatRoom::World, world),
                Some((1, rooms_world, Some(world_line.clone()))),
            ),
            (
                "open-advertising",
                ChatIntent::OpenRoom(ChatRoom::Advertising),
                Some((0, both, None)),
            ),
            (
                "line-advertising",
                say(ChatRoom::Advertising, ad),
                Some((1, both, Some(ad_line.clone()))),
            ),
            (
                "exhausted-advertising",
                say(ChatRoom::Advertising, "again"),
                None,
            ),
            (
                "close-world",
                ChatIntent::CloseRoom(ChatRoom::World),
                Some((0, only_ad, None)),
            ),
            ("closed-world", say(ChatRoom::World, "closed"), None),
            (
                "reopen-world",
                ChatIntent::OpenRoom(ChatRoom::World),
                Some((0, both, None)),
            ),
            ("muted-world", say(ChatRoom::World, "muted"), None),
        ];
        let mut script = Vec::new();
        let (mut sequence, mut id, mut revision) = (41_u64, 7_u64, 3_u64);
        for (name, _, push) in &plan {
            let observed = step(&steps, name)?;
            script.push(Step::ReadCommand);
            script.push(chat_result(
                sequence,
                id,
                observed.status,
                observed.disposition,
                observed.wait,
            )?);
            sequence += 1;
            id += 1;
            match push {
                Some((0, rooms, _)) => {
                    script.push(chat_rooms_push(sequence, revision, revision + 1, *rooms)?);
                }
                Some((_, _, Some(line))) => {
                    script.push(chat_line_push(sequence, revision, revision + 1, line)?);
                }
                _ => continue,
            }
            sequence += 1;
            revision += 1;
        }
        let (client, peer) = chat_peer(vec![chat_domain(3, ChatRoomSet::default())], script);
        let mut session = Session::admit(client, chat_admission()?).await?;
        let mut lines_after = Vec::new();
        for (name, intent, _) in &plan {
            let observed = step(&steps, name)?;
            let reply = session.chat(intent).await?;
            session.service_liveness(Duration::from_millis(50)).await?;
            session.take_events();
            assert_eq!(
                (reply.status, reply.disposition, reply.wait_seconds),
                (observed.status, observed.disposition, observed.wait),
                "{name}"
            );
            lines_after.push((
                *name,
                session
                    .chat_log()
                    .map(|log| (log.rooms(), log.lines().cloned().collect::<Vec<_>>()))
                    .ok_or("capability 7 selected")?,
            ));
        }
        let state = |name: &str| -> Result<&(ChatRoomSet, Vec<ChatLine>), BoxError> {
            lines_after
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, state)| state)
                .ok_or_else(|| "s04c state snapshot".into())
        };
        assert_eq!(
            state("open-world")?,
            &(room_set(&[ChatRoom::World]), vec![])
        );
        assert_eq!(
            state("line-world")?,
            &(room_set(&[ChatRoom::World]), vec![world_line.clone()])
        );
        assert_eq!(
            state("line-advertising")?,
            &(both, vec![world_line.clone(), ad_line.clone()])
        );
        // Exhausted: no delta, the retained state equals the state before it.
        assert_eq!(state("exhausted-advertising")?, state("line-advertising")?);
        assert_eq!(state("close-world")?.0, only_ad);
        // RoomNotOpen: refused and unchanged.
        assert_eq!(state("closed-world")?, state("close-world")?);
        assert_eq!(state("reopen-world")?.0, both);
        // Muted: refused and unchanged.
        assert_eq!(state("muted-world")?, state("reopen-world")?);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

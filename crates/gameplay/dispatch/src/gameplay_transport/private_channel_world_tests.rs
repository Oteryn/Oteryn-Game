use super::*;
use crate::foundation::{ChannelId, ExactActorRef, WorldId};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{FirstEntryOutcome, SessionContinuity};

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn id(tag: u8) -> [u8; 16] {
    let mut value = [tag; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}
fn admitted(generation: u64) -> Result<AdmittedSession, Box<dyn std::error::Error>> {
    let world_id = WorldId::decode(&id(1))?;
    let channel_id = ChannelId::decode(&id(2))?;
    Ok(AdmittedSession {
        game_session_id: GameSessionId::decode(&id(3))?,
        world_id,
        channel_id,
        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        item_fence: None,
        continuity: SessionContinuity {
            connection_generation: generation,
            selected_capabilities: SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[6, 7, 42, 80],
            )
            .ok_or("capabilities")?,
            ..SessionContinuity::FRESH
        },
    })
}
fn interest() -> JoinedChannel {
    JoinedChannel {
        channel_id: 100,
        epoch: 7,
        since_micros: 30,
    }
}

#[test]
fn private_channel_older_admission_completion_never_replaces_new_connection() -> TestResult {
    let state = PrivateChannelRuntime::default();
    let old = admitted(1)?;
    let current = admitted(2)?;
    let first = state.begin(old).ok_or("first reservation")?;
    let replacement = state.begin(current).ok_or("replacement reservation")?;
    assert!(state.finish(current.game_session_id, replacement, Some(vec![interest()])));
    assert!(!state.finish(old.game_session_id, first, Some(Vec::new())));
    assert!(state.matches(&current));
    assert!(!state.matches(&old));
    assert!(
        state.begin(old).is_none(),
        "older generation cannot invalidate current binding"
    );
    assert!(state.accepts_relay(current.game_session_id, 100, 7, 30));
    state.remember(&old, Vec::new());
    assert!(
        state.accepts_relay(current.game_session_id, 100, 7, 30),
        "stale list completion cannot replace current interests"
    );
    Ok(())
}

#[test]
fn private_channel_forget_and_same_generation_replacement_invalidate_pending_reads() -> TestResult {
    let state = PrivateChannelRuntime::default();
    let admitted = admitted(1)?;
    let first = state.begin(admitted).ok_or("reservation")?;
    state.forget(admitted.game_session_id);
    assert!(!state.finish(admitted.game_session_id, first, Some(vec![interest()])));
    let second = state.begin(admitted).ok_or("second reservation")?;
    let third = state.begin(admitted).ok_or("third reservation")?;
    assert!(!state.finish(admitted.game_session_id, second, Some(vec![interest()])));
    assert!(state.finish(admitted.game_session_id, third, Some(vec![interest()])));
    assert!(state.matches(&admitted));
    Ok(())
}

#[test]
fn private_channel_queue_interest_requires_current_join_epoch_and_timestamp() -> TestResult {
    let state = PrivateChannelRuntime::default();
    let admitted = admitted(1)?;
    let reservation = state.begin(admitted).ok_or("reservation")?;
    assert!(
        !state.accepts_relay(admitted.game_session_id, 100, 7, 30),
        "pending admission queues no text"
    );
    assert!(state.finish(
        admitted.game_session_id,
        reservation,
        Some(vec![interest()])
    ));
    assert!(state.accepts_relay(admitted.game_session_id, 100, 7, 30));
    assert!(!state.accepts_relay(admitted.game_session_id, 100, 7, 29));
    assert!(!state.accepts_relay(admitted.game_session_id, 101, 7, 30));
    assert!(!state.accepts_relay(admitted.game_session_id, 100, 8, 30));
    assert!(!state.accepts_relay(GameSessionId::decode(&id(4))?, 100, 7, 30));
    let mut legacy = admitted;
    legacy.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[6, 7, 42])
            .ok_or("legacy capabilities")?;
    assert!(!state.matches(&legacy));
    let list = ChatChannelList::default();
    assert!(
        state
            .list(
                &admitted,
                ChannelView {
                    list: list.clone(),
                    joined: vec![interest()]
                }
            )
            .is_some()
    );
    assert!(
        state
            .list(
                &admitted,
                ChannelView {
                    list,
                    joined: Vec::new()
                }
            )
            .is_none(),
        "wire list need not change when membership does"
    );
    assert!(
        !state.accepts_relay(admitted.game_session_id, 100, 7, 30),
        "joined interests still refresh when visible list is unchanged"
    );
    Ok(())
}

#[test]
fn private_channel_legacy_sender_entry_never_creates_a_parallel_local_room() -> TestResult {
    let chat = crate::gameplay_transport::chat_intent::ChatRuntime::default();
    let session = admitted(1)?.game_session_id;
    let intents = [
        PrivateChannelIntent::Create,
        PrivateChannelIntent::Invite {
            name: "Bryn".into(),
        },
        PrivateChannelIntent::Exclude {
            name: "Bryn".into(),
        },
        PrivateChannelIntent::Join { channel_id: 100 },
        PrivateChannelIntent::Leave { channel_id: 100 },
        PrivateChannelIntent::Say {
            channel_id: 100,
            text: "hello".into(),
        },
        PrivateChannelIntent::List,
    ];
    for intent in intents {
        assert_eq!(
            chat.pre_gate(
                session,
                &oteryn_protocol_oteryn::chat::ChatIntent::PrivateChannel(intent),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1)
            ),
            Some(result(ChatDisposition::Rejected))
        );
    }
    Ok(())
}

#[test]
fn private_channel_member_reads_show_the_extension_and_notify_after_the_baseline() -> TestResult {
    use oteryn_protocol_oteryn::chat::line_extended::ChannelNoticeKind;
    use oteryn_protocol_oteryn::chat::private_channel::InvitedChannel;
    let state = PrivateChannelRuntime::default();
    let current = admitted(1)?;
    let viewer = *current.game_session_id.as_bytes();
    let reservation = state.begin(current).ok_or("reservation")?;
    assert!(state.finish(current.game_session_id, reservation, Some(vec![interest()])));
    let list = ChatChannelList {
        own_channel: None,
        invited: vec![InvitedChannel {
            channel_id: 100,
            owner_name: "Bryn".into(),
        }],
        ..ChatChannelList::default()
    };
    let row = |session: [u8; 16], name: &str, joined: bool| ChannelMember {
        channel_id: 100,
        epoch: 7,
        session,
        joined,
        name: name.into(),
    };
    let read = |members: Vec<ChannelMember>, active: Vec<[u8; 16]>| MemberView {
        view: ChannelView {
            list: list.clone(),
            joined: vec![interest()],
        },
        members,
        active,
    };
    let (bryn, cora) = (id(5), id(6));
    // The first read is the baseline: the list with both fields, no notice.
    let (shown, notices) = state
        .list_extended(
            &current,
            read(
                vec![row(viewer, "Aldric", true), row(bryn, "Bryn", true)],
                Vec::new(),
            ),
            true,
            true,
        )
        .ok_or("current binding")?;
    let (_, extension) = shown.ok_or("first list")?;
    assert_eq!(extension.joined, vec![100]);
    assert_eq!(extension.participants[0].names, vec!["Aldric", "Bryn"]);
    assert!(notices.is_empty());
    assert_eq!(state.probe(&current), vec![bryn]);
    // Cora joins: a notice and a changed extension; the legacy list is unchanged.
    let (shown, notices) = state
        .list_extended(
            &current,
            read(
                vec![
                    row(viewer, "Aldric", true),
                    row(bryn, "Bryn", true),
                    row(cora, "Cora", true),
                ],
                vec![bryn],
            ),
            true,
            true,
        )
        .ok_or("current binding")?;
    assert!(shown.is_some(), "a new participant changes field 5");
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].kind, ChannelNoticeKind::Joined);
    assert_eq!(notices[0].name, "Cora");
    // Cora is excluded (row gone, session active), Bryn leaves (row kept, not joined).
    let (_, notices) = state
        .list_extended(
            &current,
            read(
                vec![row(viewer, "Aldric", true), row(bryn, "Bryn", false)],
                vec![bryn, cora],
            ),
            true,
            true,
        )
        .ok_or("current binding")?;
    assert_eq!(
        notices
            .iter()
            .map(|notice| (notice.kind, notice.name.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (ChannelNoticeKind::Left, "Bryn"),
            (ChannelNoticeKind::Excluded, "Cora"),
        ]
    );
    // An unchanged read sends neither a list nor a notice; 494 alone sends no notice.
    let unchanged = state
        .list_extended(
            &current,
            read(
                vec![row(viewer, "Aldric", true), row(bryn, "Bryn", false)],
                vec![bryn],
            ),
            true,
            true,
        )
        .ok_or("current binding")?;
    assert_eq!(unchanged, (None, Vec::new()));
    let (shown, notices) = state
        .list_extended(
            &current,
            read(vec![row(viewer, "Aldric", true)], Vec::new()),
            true,
            false,
        )
        .ok_or("current binding")?;
    assert!(notices.is_empty());
    let (_, extension) = shown.ok_or("field 5 withdrawn")?;
    assert!(extension.participants.is_empty());
    // A stale connection reads nothing.
    assert!(
        state
            .list_extended(&admitted(2)?, read(Vec::new(), Vec::new()), true, true)
            .is_none()
    );
    Ok(())
}

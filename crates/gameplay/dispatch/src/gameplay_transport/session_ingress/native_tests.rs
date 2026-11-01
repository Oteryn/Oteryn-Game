//! Cancellation and completion at the existing stream owner's real boundary.
use super::*;
use oteryn_protocol_oteryn::world_object::{UseDisposition, encode_use_result};
use oteryn_protocol_oteryn::{
    CommandStatus, decode_command_result, decode_wire_envelope, encode_command_result,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
struct Fixture {
    owner: SessionCommands,
    stream: Option<SessionCommandStream>,
    state: Arc<Mutex<SessionIngress>>,
    command: crate::foundation::CommandRef,
}
impl Fixture {
    fn new() -> TestResult<Self> {
        let session =
            GameSessionId::decode(&[1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16])
                .map_err(|error| format!("{error:?}"))?;
        let command = crate::foundation::CommandRef::new(
            session,
            CommandId::new(1).map_err(|error| format!("{error:?}"))?,
        );
        let state = Arc::new(Mutex::new(SessionIngress {
            active: true,
            ..SessionIngress::default()
        }));
        let owner = SessionCommands(Mutex::new(HashMap::from([(session, Arc::clone(&state))])));
        let mut stream = SessionCommandStream {
            state: Some(Arc::clone(&state)),
            next: 1,
            selected: SelectedCapabilities::NONE,
        };
        assert!(matches!(
            stream
                .begin(1, 7, 4, false)
                .map_err(|error| format!("{error:?}"))?,
            CommandAction::Execute
        ));
        Ok(Self {
            owner,
            stream: Some(stream),
            state,
            command,
        })
    }
}

#[test]
fn native_active_verdict_survives_cancellation_and_reconciles_exactly_once() -> TestResult {
    let mut f = Fixture::new()?;
    f.owner
        .complete_item_use(f.command, UseDisposition::Committed)
        .ok_or("retain proven native result")?;
    {
        let state = f.state.lock().map_err(|_| "state lock")?;
        assert_eq!(state.ingress.outstanding(), 1);
        assert!(
            state
                .pending_delivery
                .is_some_and(|pending| pending.proven_native.is_some())
        );
        assert!(
            state.retained.is_none(),
            "the active connection still owns terminal publication"
        );
        assert_eq!(state.result_sequence_floor, 0);
    }
    drop(f.stream.take());
    let mut state = f.state.lock().map_err(|_| "state lock")?;
    state
        .complete_native_if_disconnected()
        .ok_or("disconnected completion")?;
    assert_eq!(state.ingress.outstanding(), 0);
    let first = state.retained.clone().ok_or("retained exact result")?;
    let envelope = decode_wire_envelope(&first).map_err(|error| format!("{error:?}"))?;
    let result = decode_command_result(envelope.payload()).map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        (
            envelope.connection_generation(),
            envelope.server_sequence(),
            result.command_id,
            result.status
        ),
        (7, 5, 1, CommandStatus::Accepted)
    );
    assert_eq!(
        result.payload,
        encode_use_result(UseDisposition::Committed).map_err(|error| format!("{error:?}"))?
    );
    assert_eq!(state.recovered_continuity(1, 4), Some((2, 5)));
    assert_eq!(state.recovered_continuity(2, 5), Some((2, 5)));
    for (next, sequence) in [(0, 4), (3, 5), (1, 3)] {
        assert_eq!(
            state.recovered_continuity(next, sequence),
            None,
            "unrelated continuity {next}/{sequence}"
        );
    }
    state
        .complete_native_if_disconnected()
        .ok_or("idempotent completion")?;
    assert_eq!(state.retained.as_ref(), Some(&first));
    assert_eq!(state.ingress.retained_count(), 1);
    Ok(())
}

#[test]
fn native_unknown_retains_reservation_without_terminal_truth_or_counter_advance() -> TestResult {
    let mut f = Fixture::new()?;
    drop(f.stream.take());
    assert!(f.owner.pending_reconciliation(f.command));
    let mut state = f.state.lock().map_err(|_| "state lock")?;
    state
        .complete_native_if_disconnected()
        .ok_or("no fabricated completion")?;
    assert_eq!(state.ingress.outstanding(), 1);
    assert!(state.retained.is_none());
    assert_eq!(state.recovered_continuity(1, 4), Some((1, 4)));
    Ok(())
}

#[test]
fn native_active_publication_uses_the_original_connection_terminalization() -> TestResult {
    let mut f = Fixture::new()?;
    f.owner
        .complete_item_use(f.command, UseDisposition::Rejected)
        .ok_or("retain refusal")?;
    let payload =
        encode_use_result(UseDisposition::Rejected).map_err(|error| format!("{error:?}"))?;
    let frame = encode_command_result(7, 5, 1, CommandStatus::Rejected, &payload)
        .map_err(|error| format!("{error:?}"))?;
    f.stream
        .as_mut()
        .ok_or("active stream")?
        .terminalize(1, &frame)
        .ok_or("normal terminalization")?;
    drop(f.stream.take());
    let mut state = f.state.lock().map_err(|_| "state lock")?;
    state
        .complete_native_if_disconnected()
        .ok_or("no second terminalization")?;
    assert!(state.pending_delivery.is_none());
    assert!(state.reconciled_sequence.is_none());
    assert_eq!(state.retained.as_ref(), Some(&frame));
    assert_eq!(state.ingress.retained_count(), 1);
    Ok(())
}

#[test]
fn native_legacy_summon_completion_retains_accepted_boolean_codec() -> TestResult {
    for active in [false, true] {
        let mut f = Fixture::new()?;
        drop(f.stream.take());
        f.owner
            .complete_hireling_summon(f.command, active)
            .ok_or("legacy native result")?;
        let state = f.state.lock().map_err(|_| "state lock")?;
        let frame = state.retained.as_ref().ok_or("legacy retained result")?;
        let envelope = decode_wire_envelope(frame).map_err(|error| format!("{error:?}"))?;
        let result =
            decode_command_result(envelope.payload()).map_err(|error| format!("{error:?}"))?;
        assert_eq!(result.status, CommandStatus::Accepted);
        assert_eq!(result.payload, &[u8::from(active)]);
    }
    Ok(())
}

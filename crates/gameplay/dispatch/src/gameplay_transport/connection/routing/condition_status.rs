use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    if oteryn_protocol_oteryn::condition_status::decode_query(command.payload).is_none() {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    }
    match authority
        .condition_status_query(*admitted, command.command_id)
        .await
    {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, ChannelRuntimeV1, CommandStatus, WorldId};
    use crate::gameplay_transport::actor_spell::{
        ChannelSpellStates,
        tests::{FACTS, gate_player, runtime_with_player},
    };
    use crate::gameplay_transport::{
        AdmissionRefusal, FirstEntryOutcome, FreshAdmissionAttempt, SessionContinuity,
    };
    use oteryn_protocol_oteryn::condition_status::{
        COMMAND_TYPE_CONDITION_STATUS_QUERY, ConditionStatus,
    };
    use oteryn_simulation_determinism::SemanticTimeMicros;
    struct Owner {
        runtime: ChannelRuntimeV1,
        states: ChannelSpellStates,
        now: std::cell::Cell<u64>,
    }
    impl FreshAdmissionAuthority for Owner {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }
        async fn condition_status_query(
            &self,
            admitted: AdmittedSession,
            _command_id: u64,
        ) -> Option<Vec<u8>> {
            crate::gameplay_transport::condition_status::snapshot(
                &self.runtime,
                &self.states,
                admitted.runtime_actor?,
                admitted.game_session_id,
                self.now.get(),
            )?
            .encode()
        }
    }
    fn uuid(tag: u8) -> [u8; 16] {
        [
            0x01, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag,
        ]
    }
    #[test]
    fn condition_status_route_reads_live_owner_and_clears_expired_icons() -> Result<(), String> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                let executor = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?;
                executor.block_on(async {
                    let (runtime, actor, session) = runtime_with_player(0x7b);
                    let mut states = ChannelSpellStates::default();
                    states
                        .initialize(
                            &runtime,
                            actor,
                            session,
                            FACTS,
                            (0, 0),
                            SemanticTimeMicros::from_micros(0),
                        )
                        .ok_or("initialize")?;
                    gate_player(&mut states, actor, session, "muted");
                    let owner = Owner {
                        runtime,
                        states,
                        now: std::cell::Cell::new(1_000_000),
                    };
                    let admitted = AdmittedSession {
                        game_session_id: session,
                        world_id: WorldId::decode(&uuid(0x60)).map_err(|e| format!("{e:?}"))?,
                        channel_id: ChannelId::decode(&uuid(0x61)).map_err(|e| format!("{e:?}"))?,
                        runtime_actor: Some(actor),
                        first_entry: FirstEntryOutcome::Positioned,
                        controller: None,
                        continuity: SessionContinuity::FRESH,
                        item_fence: None,
                    };
                    let command = |payload| oteryn_protocol_oteryn::ClientCommandView {
                        command_id: 1,
                        command_type: COMMAND_TYPE_CONDITION_STATUS_QUERY,
                        payload,
                    };
                    let Dispatch::Reply(CommandStatus::Accepted, bytes) =
                        route(&owner, &admitted, command(&[])).await
                    else {
                        return Err("route refused owner".into());
                    };
                    let status = ConditionStatus::decode(&bytes).ok_or("decode")?;
                    assert_eq!(status.conditions.len(), 1);
                    assert_eq!(status.conditions[0].remaining_ms, 9_000);
                    assert_eq!(status.conditions[0].icon, 24);
                    owner.now.set(10_000_000);
                    let Dispatch::Reply(CommandStatus::Accepted, bytes) =
                        route(&owner, &admitted, command(&[])).await
                    else {
                        return Err("expiry route refused".into());
                    };
                    assert_eq!(
                        ConditionStatus::decode(&bytes)
                            .ok_or("expired decode")?
                            .conditions,
                        []
                    );
                    assert!(matches!(
                        route(&owner, &admitted, command(&[1])).await,
                        Dispatch::Reply(CommandStatus::Rejected, _)
                    ));
                    let mut stale = admitted;
                    stale.game_session_id = crate::foundation::GameSessionId::decode(&uuid(0x7c))
                        .map_err(|e| format!("{e:?}"))?;
                    assert!(matches!(
                        route(&owner, &stale, command(&[])).await,
                        Dispatch::Reply(CommandStatus::Rejected, _)
                    ));
                    Ok(())
                })
            })
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| "route thread panicked".to_owned())?
    }
}

use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    let Some(definitions) =
        oteryn_protocol_oteryn::action_bar_cooldowns::decode_query(command.payload)
    else {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    };
    match authority
        .action_bar_cooldowns_query(*admitted, command.command_id, definitions)
        .await
    {
        Some(bytes) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, ChannelRuntimeV1, CommandStatus, WorldId};
    use crate::gameplay_transport::actor_spell::{
        ChannelSpellStates,
        tests::{FACTS, runtime_with_player},
    };
    use crate::gameplay_transport::{
        AdmissionRefusal, FirstEntryOutcome, FreshAdmissionAttempt, SessionContinuity,
    };
    use oteryn_protocol_oteryn::action_bar_cooldowns::{self as wire, ItemCooldown};
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
        async fn action_bar_cooldowns_query(
            &self,
            admitted: AdmittedSession,
            _command: u64,
            definitions: Vec<u32>,
        ) -> Option<Vec<u8>> {
            let mut items = Vec::new();
            for definition in definitions {
                let remaining_ms = self.states.item_uses.cooldown_remaining(
                    &self.runtime,
                    admitted.runtime_actor?,
                    admitted.game_session_id,
                    crate::gameplay_transport::item_use::CooldownClass::Potion,
                    self.now.get(),
                )?;
                items.push(ItemCooldown {
                    definition,
                    remaining_ms,
                });
            }
            wire::encode_reply(&items)
        }
    }
    fn uuid(tag: u8) -> [u8; 16] {
        [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
    }
    #[test]
    fn action_bar_item_cooldown_route_reads_existing_gate_then_expiry_and_rejects_stale_session()
    -> Result<(), String> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?
                    .block_on(async {
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
                        states
                            .item_uses
                            .start_test_potion(&runtime, actor, session, 0)
                            .ok_or("potion prepare")?;
                        let owner = Owner {
                            runtime,
                            states,
                            now: std::cell::Cell::new(500),
                        };
                        let admitted = AdmittedSession {
                            game_session_id: session,
                            world_id: WorldId::decode(&uuid(0x60)).map_err(|e| format!("{e:?}"))?,
                            channel_id: ChannelId::decode(&uuid(0x61))
                                .map_err(|e| format!("{e:?}"))?,
                            runtime_actor: Some(actor),
                            first_entry: FirstEntryOutcome::Positioned,
                            controller: None,
                            continuity: SessionContinuity::FRESH,
                            item_fence: None,
                        };
                        let payload = wire::encode_query(&[256]).ok_or("query")?;
                        let command = |payload| oteryn_protocol_oteryn::ClientCommandView {
                            command_id: 1,
                            command_type: wire::COMMAND_TYPE_ACTION_BAR_COOLDOWNS_QUERY,
                            payload,
                        };
                        let Dispatch::Reply(CommandStatus::Accepted, bytes) =
                            route(&owner, &admitted, command(&payload)).await
                        else {
                            return Err("route refused live item cooldown".into());
                        };
                        assert_eq!(
                            wire::decode_reply(&bytes),
                            Some(vec![ItemCooldown {
                                definition: 256,
                                remaining_ms: 500
                            }])
                        );
                        owner.now.set(1000);
                        let Dispatch::Reply(CommandStatus::Accepted, bytes) =
                            route(&owner, &admitted, command(&payload)).await
                        else {
                            return Err("expiry refused".into());
                        };
                        assert_eq!(
                            wire::decode_reply(&bytes),
                            Some(vec![ItemCooldown {
                                definition: 256,
                                remaining_ms: 0
                            }])
                        );
                        assert!(matches!(
                            route(&owner, &admitted, command(&[1])).await,
                            Dispatch::Reply(CommandStatus::Rejected, _)
                        ));
                        let mut stale = admitted;
                        stale.game_session_id =
                            crate::foundation::GameSessionId::decode(&uuid(0x7c))
                                .map_err(|e| format!("{e:?}"))?;
                        assert!(matches!(
                            route(&owner, &stale, command(&payload)).await,
                            Dispatch::Reply(CommandStatus::Rejected, _)
                        ));
                        Ok(())
                    })
            })
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| "cooldown route thread panicked".to_owned())?
    }
}

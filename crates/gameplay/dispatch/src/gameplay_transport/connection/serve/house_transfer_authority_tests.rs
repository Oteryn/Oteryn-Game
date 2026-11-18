use super::*;
use std::cell::Cell;
use std::time::Duration;

struct Authority {
    staff: bool,
    checks: Cell<usize>,
}

impl FreshAdmissionAuthority for Authority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn house_scope_ack(&self, _: AdmittedSession) -> Result<Option<(u64, Vec<u8>)>, ()> {
        Ok(Some((1, vec![1])))
    }

    async fn staff_refresh(&self, _: AdmittedSession) -> bool {
        if self.staff {
            self.checks.set(self.checks.get() + 1);
            return false;
        }
        true
    }

    async fn account_closure_check(&self, _: AdmittedSession) -> bool {
        if !self.staff {
            self.checks.set(self.checks.get() + 1);
            return true;
        }
        false
    }
}

#[test]
fn house_idle_rechecks_staff_and_account_closure() -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .start_paused(true)
                .build()
                .map_err(|e| e.to_string())?
                .block_on(async {
                    for staff in [false, true] {
                        let authority = Authority { staff, checks: Cell::new(0) };
                        let id = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
                        let world_id = WorldId::decode(&id).map_err(|e| format!("{e:?}"))?;
                        let channel_id = ChannelId::decode(&id).map_err(|e| format!("{e:?}"))?;
                        let admitted = AdmittedSession {
                            game_session_id: GameSessionId::decode(&id).map_err(|e| format!("{e:?}"))?,
                            world_id, channel_id,
                            runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
                            first_entry: FirstEntryOutcome::Positioned,
                            controller: None, item_fence: None, continuity: SessionContinuity::FRESH,
                        };
                        if staff {
                            let mut admitted=admitted;
                            let actor=admitted.runtime_actor.ok_or("House actor")?;
                            let (mut state,_)=house_hireling::State::prepare(&authority,actor,&mut admitted).await.map_err(|_|"prepare")?;
                            let ids=[500,502,511];
                            let offers=ids.map(|id| crate::gameplay_transport::capabilities::OfferedCapability{id,requires:&[]});
                            admitted.continuity.selected_capabilities=SelectedCapabilities::select(&offers,&ids).ok_or("House capabilities")?;
                            for command_type in [6241,6242,6246] {
                                let command=oteryn_protocol_oteryn::ClientCommandView {command_type,command_id:1,payload:&[]};
                                assert!(matches!(state.route(&authority,&mut admitted,command).await,Some(Dispatch::Reply(_, _))),"House must route command {command_type} to its owner");
                            }
                        }
                        let (mut server, _client) = tokio::io::duplex(1 << 16);
                        let mut messages = None;
                        let mut integrity = integrity::Integrity::default();
                        let result = tokio::time::timeout(Duration::from_secs(6), serve(
                            &mut server, admitted, &authority, &authority, &mut messages,
                            IDLE_LIVENESS, &mut integrity, false,
                        )).await;
                        assert!(result.is_ok(), "House failed to recheck authority: staff={staff}");
                        assert!(matches!(result, Ok(Err(end)) if matches!(*end, ConnectionEnd::AdmittedThenDisconnected(_))));
                        assert_eq!(authority.checks.get(), 1);
                    }
                    Ok(())
                })
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "House authority test panicked".to_owned())?
}

use super::*;
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::{AdmissionRefusal, FirstEntryOutcome, FreshAdmissionAttempt, SessionContinuity},
};
use oteryn_protocol_oteryn::party_skull::PvpStatus;
struct Authority(std::cell::Cell<usize>);
impl FreshAdmissionAuthority for Authority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn observe_pvp_status(&self, _: AdmittedSession) -> Option<PvpStatus> {
        self.0.set(self.0.get() + 1);
        Some(PvpStatus {
            points_milli: [500, 1500, 2500],
            red_limits_milli: [3000, 5000, 10000],
            black_limits_milli: [6000, 10000, 20000],
            skull: 3,
            remaining_ms: 899999,
            situations: vec![],
        })
    }
}
#[test]
fn own_pvp_route_preserves_legacy_query_and_gates_current_status()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            tokio::runtime::Builder::new_current_thread()
                .build()?
                .block_on(async {
                    let (runtime, actor, session) =
                        crate::gameplay_transport::actor_spell::tests::runtime_with_player(31);
                    let mut admitted = AdmittedSession {
                        game_session_id: session,
                        world_id: runtime.binding().world_id(),
                        channel_id: runtime.binding().channel_id().ok_or("channel")?,
                        runtime_actor: Some(actor),
                        first_entry: FirstEntryOutcome::Positioned,
                        controller: None,
                        continuity: SessionContinuity::FRESH,
                        item_fence: None,
                    };
                    let authority = Authority(std::cell::Cell::new(0));
                    assert!(matches!(
                        route(&authority, admitted, &[8, 1], None).await,
                        Dispatch::Unregistered
                    ));
                    assert_eq!(authority.0.get(), 0);
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        &[OfferedCapability {
                            id: 134,
                            requires: &[],
                        }],
                        &[134],
                    )
                    .ok_or("selection")?;
                    for payload in [&[8, 0][..], &[8, 2], &[8, 1, 8, 1], &[1], &[]] {
                        assert!(matches!(
                            route(&authority, admitted, payload, None).await,
                            Dispatch::Unregistered
                        ));
                    }
                    assert_eq!(authority.0.get(), 0);
                    let Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes) =
                        route(&authority, admitted, &[8, 1], None).await
                    else {
                        return Err("own PvP reply missing".into());
                    };
                    let status = PvpStatus::decode(&bytes).map_err(|e| format!("{e:?}"))?;
                    assert_eq!(status.points_milli, [500, 1500, 2500]);
                    assert_eq!(status.skull, 3);
                    assert_eq!(status.remaining_ms, 899999);
                    assert_eq!(authority.0.get(), 1);
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "PvP route thread")?
}

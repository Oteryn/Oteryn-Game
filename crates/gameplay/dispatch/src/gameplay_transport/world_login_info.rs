//! Public read-only login probe for the World served by this endpoint.
use super::{ComposedFreshAdmission, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::daily_boost::login as wire;
use std::time::Duration;
use tokio::io::AsyncWrite;

const PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// Returns true for a standalone probe. The listener closes it without handing
/// it to login, reconnect or the gameplay command stream.
pub(super) async fn route(
    stream: &mut (impl AsyncWrite + Unpin),
    frame: &[u8],
    authority: &impl FreshAdmissionAuthority,
) -> bool {
    let Some(world_id) = wire::decode_request(frame) else {
        return false;
    };
    let _ = tokio::time::timeout(PROBE_TIMEOUT, async {
        let info = authority.world_login_info(world_id).await;
        if let Some(frame) = wire::encode_reply(&wire::Reply { world_id, info }) {
            let _ = super::tcp_tls::write_frame(stream, &frame).await;
        }
    })
    .await;
    true
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn read_world_login_information(
        &self,
        id: [u8; 16],
    ) -> Option<wire::WorldInfo> {
        let world = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().ok()?;
            let world = runtime.binding().world_id();
            if world.as_bytes() != &id {
                return None;
            }
            world
        };
        let bundle = self.bundle_map().map(|map| map.overlay.base().digest());
        let info = self
            .root
            .read_world_login_info(world, bundle, self.reset_epoch)
            .await
            .ok()?;
        let boost = info.boost.and_then(|boost| {
            if boost.effective_from >= boost.effective_until {
                return None;
            }
            let native = self.active_generation?.native_gameplay()?;
            let race = |key: String| {
                let policy = native
                    .policies()
                    .policies()
                    .iter()
                    .find(|policy| policy.definition_key == key)?;
                let artwork = if let Some(look_type) = policy.outfit_appearance() {
                    wire::Artwork::Outfit { look_type }
                } else {
                    let look_type = policy.object_look_type.filter(|look| *look > 0)?;
                    wire::Artwork::Item { look_type }
                };
                Some(wire::Race {
                    key,
                    name: policy.display_name.clone(),
                    artwork,
                })
            };
            Some(wire::Boost {
                reset_epoch: boost.reset_epoch,
                effective_from: boost.effective_from,
                effective_until: boost.effective_until,
                creature: race(boost.creature_key)?,
                boss: race(boost.boss_key)?,
            })
        });
        Some(wire::WorldInfo {
            players_online: info.players_online,
            boost,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay_transport::connection::{
        AdmissionRefusal, AdmittedSession, FreshAdmissionAttempt,
    };
    struct WorldSource([u8; 16]);

    impl FreshAdmissionAuthority for WorldSource {
        async fn admit(
            &self,
            _: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Unavailable)
        }

        async fn world_login_info(&self, world: [u8; 16]) -> Option<wire::WorldInfo> {
            (world == self.0).then_some(wire::WorldInfo {
                players_online: 17,
                boost: None,
            })
        }
    }

    #[tokio::test]
    async fn world_login_info_route_only_handles_the_standalone_probe()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = WorldSource([7; 16]);
        let (mut server, mut client) = tokio::io::duplex(4096);
        let admitted_command = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: 1,
                command_type: wire::COMMAND_TYPE,
                payload: &source.0,
            },
        )
        .map_err(|err| format!("{err:?}"))?;
        assert!(!route(&mut server, &admitted_command, &source).await);

        let request = wire::encode_request(source.0);
        assert!(route(&mut server, &request, &source).await);
        let bytes = super::super::tcp_tls::read_frame(&mut client).await?;
        let reply = wire::decode_reply(&bytes).ok_or("probe reply")?;
        assert_eq!(reply.world_id, source.0);
        assert_eq!(reply.info.ok_or("selected World")?.players_online, 17);
        Ok(())
    }
}

//! Execute the unchanged qualification pair phase over two scripted TLS connections.
//! This checks its adapter/read ordering; it does not replace the real-owner WP5 topology.
use oteryn_dev_client::{DevClientSession, JoinRequest, connect_session};
use oteryn_protocol_oteryn::{
    self as wire, item_view, logout, player_trade as trade, world_object, world_spatial,
    world_spatial_entities as entities,
};
use oteryn_session::{read_frame, write_frame};
use rustls::pki_types::PrivatePkcs8KeyDer;
use std::{sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use tokio_rustls::{TlsAcceptor, server::TlsStream};
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type PeerError = Box<dyn std::error::Error + Send + Sync>;
struct SeamClients<'a> {
    partner: Mutex<Option<DevClientSession>>,
    evidence: &'a str,
}
fn evidence(message: &str) {
    println!("{message}");
}
mod client_world {
    use super::*;
    pub(super) async fn connect_character(
        clients: &SeamClients<'_>,
        marker: u8,
        index: usize,
    ) -> TestResult<DevClientSession> {
        assert_eq!((marker, index), (0x74, 0));
        assert_eq!(clients.evidence, "scripted TLS");
        clients
            .partner
            .lock()
            .await
            .take()
            .ok_or_else(|| "partner already connected".into())
    }
    /// Mirrors `client_phases::world::logout_retrying_busy`; the scripted peer never answers Busy.
    pub(super) async fn logout_retrying_busy(
        client: &mut DevClientSession,
    ) -> TestResult<(Option<logout::LogoutResult>, String, usize, bool)> {
        let outcome = client
            .session_mut()
            .logout()
            .await
            .map_err(|error| format!("logout attempt 1 failed: {error}"))?;
        Ok((outcome.result, format!("{outcome:?}"), 1, false))
    }
}
#[path = "../../../apps/game-server/src/qualification/client_phases/player_trade.rs"]
mod phase;
const CAPS: &[u32] = &[4, 6, 21, 60];
fn uuid(marker: u8) -> [u8; 16] {
    [
        1, 147, 79, 16, 124, 0, 112, 1, 128, 91, 59, 17, 34, 51, 68, marker,
    ]
}
fn reference(side: u8) -> entities::EntityRef {
    entities::EntityRef {
        identity: uuid(10 + side),
        generation: 1,
    }
}
fn actor(side: u8) -> entities::WorldSpatialEntity {
    entities::WorldSpatialEntity {
        display_name: None,
        kind: entities::EntityKind::Player,
        entity: reference(side),
        position: world_spatial::ActorPosition {
            x: i32::from(side),
            y: 0,
            floor: 0,
        },
        detail: entities::EntityDetail::Actor {
            direction: world_spatial::StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}
fn encoded<T, E: std::fmt::Debug>(result: Result<T, E>) -> Result<T, PeerError> {
    result.map_err(|e| format!("{e:?}").into())
}
fn inventory(side: u8, exchanged: bool) -> Result<item_view::CharacterInventory, PeerError> {
    let item = item_view::ItemEntry {
        handle: (70 + u64::from(side) + if exchanged { 10 } else { 0 }).try_into()?,
        item_definition_ref: (123 + u32::from(if exchanged { 1 - side } else { side }))
            .try_into()?,
        count: 1.try_into()?,
        sub_type: 0,
    };
    Ok(item_view::CharacterInventory {
        main_backpack: Some(item_view::ItemEntry {
            handle: (90 + u64::from(side)).try_into()?,
            item_definition_ref: 999.try_into()?,
            ..item
        }),
        entries: vec![item],
        equipment: vec![],
    })
}
struct Peer {
    stream: TlsStream<TcpStream>,
    sequence: u64,
    revision: u64,
}
impl Peer {
    async fn join(
        listener: &TcpListener,
        acceptor: &TlsAcceptor,
        side: u8,
    ) -> Result<Self, PeerError> {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        read_frame(&mut stream).await?;
        write_frame(
            &mut stream,
            &wire::encode_server_accepted(&wire::ServerAcceptedValue {
                game_session_id: wire::GameSessionId::decode(&uuid(1 + side))?,
                world_id: wire::WorldId::decode(&uuid(3))?,
                channel_id: wire::ChannelId::decode(&uuid(4))?,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 7,
                schema_revision: 1,
                selected_capabilities: CAPS,
            })?,
        )
        .await?;
        let spatial = encoded(
            entities::encode_world_spatial_entities_snapshot_with_item_handles(
                &entities::WorldSpatialEntitiesSnapshot {
                    content_generation: [1; 32],
                    actor_position: actor(side).position,
                    own_identity: reference(side).identity,
                    entities: vec![actor(0), actor(1)],
                },
            ),
        )?;
        let overlay = encoded(world_object::encode_world_object_overlay_snapshot(&[]))?;
        let inventory = encoded(item_view::encode_character_inventory(&inventory(
            side, false,
        )?))?;
        let state = encoded(trade::encode_state(&Default::default()))?;
        let domains = [
            (1, 2, spatial),
            (2, 1, overlay),
            (9, 1, inventory),
            (11, 1, vec![]),
            (26, 1, state),
        ];
        let domains: Vec<_> = domains
            .iter()
            .map(|(id, kind, payload)| wire::DomainSnapshot {
                domain_id: *id,
                revision: 1,
                snapshot_type: *kind,
                payload,
            })
            .collect();
        for frame in wire::encode_single_chunk_snapshot(1, 1, 40, &domains)? {
            write_frame(&mut stream, &frame).await?;
        }
        Ok(Self {
            stream,
            sequence: 40,
            revision: 1,
        })
    }
    async fn intent(
        &mut self,
        expected: trade::Intent,
        result: trade::ResultCode,
    ) -> Result<(), PeerError> {
        let bytes = read_frame(&mut self.stream).await?;
        let envelope = wire::decode_wire_envelope(&bytes)?;
        let command = envelope.client_command(1)?;
        assert_eq!(command.command_type, 78);
        let actual = encoded(trade::decode_intent(command.payload))?;
        if actual != expected {
            return Err(
                format!("qualification intent mismatch: {actual:?} != {expected:?}").into(),
            );
        }
        self.sequence += 1;
        write_frame(
            &mut self.stream,
            &wire::encode_command_result(
                1,
                self.sequence,
                command.command_id,
                wire::CommandStatus::Accepted,
                &result.payload(),
            )?,
        )
        .await?;
        Ok(())
    }
    async fn state(&mut self, side: u8, phase: u8, accepted: [bool; 2]) -> Result<(), PeerError> {
        let state = if phase == 4 {
            trade::State {
                reason: 1,
                message: "Trade completed.".into(),
                ..Default::default()
            }
        } else {
            trade::State {
                partner: Some(reference(1 - side)),
                trade_id: (phase == 2).then_some([3; 16]),
                own: (side == 0 || phase == 2).then_some(trade::Offer {
                    definition: 123 + u32::from(side),
                    count: 1,
                    sub_type: 0,
                }),
                counter: (phase == 2).then_some(trade::Offer {
                    definition: 123 + u32::from(1 - side),
                    count: 1,
                    sub_type: 0,
                }),
                phase,
                reason: 0,
                accepted,
                ..Default::default()
            }
        };
        self.push(&state).await
    }
    async fn push(&mut self, state: &trade::State) -> Result<(), PeerError> {
        let payload = encoded(trade::encode_state(state))?;
        self.sequence += 1;
        write_frame(
            &mut self.stream,
            &wire::encode_state_delta(
                1,
                self.sequence,
                26,
                self.revision,
                self.revision + 1,
                1,
                &payload,
            )?,
        )
        .await?;
        self.revision += 1;
        Ok(())
    }
    async fn inventory(&mut self, side: u8) -> Result<(), PeerError> {
        let payload = encoded(item_view::encode_character_inventory(&inventory(
            side, true,
        )?))?;
        self.sequence += 1;
        write_frame(
            &mut self.stream,
            &wire::encode_state_delta(1, self.sequence, 9, 1, 2, 1, &payload)?,
        )
        .await?;
        Ok(())
    }
}
async fn peer(listener: TcpListener, acceptor: TlsAcceptor) -> Result<(), PeerError> {
    let mut own = Peer::join(&listener, &acceptor, 0).await?;
    let mut other = Peer::join(&listener, &acceptor, 1).await?;
    // The qualification's single-client cancel must return without a state delta.
    own.intent(trade::Intent::Cancel, trade::ResultCode::Cancelled)
        .await?;
    // Replayed acceptance has no delta; cancellation of an open trade does.
    own.state(0, 2, [true, false]).await?;
    own.intent(
        trade::Intent::Accept { trade_id: [3; 16] },
        trade::ResultCode::Accepted,
    )
    .await?;
    own.intent(trade::Intent::Cancel, trade::ResultCode::Cancelled)
        .await?;
    own.push(&Default::default()).await?;

    own.intent(
        trade::Intent::Offer {
            partner: reference(1),
            item: 70,
        },
        trade::ResultCode::Offered,
    )
    .await?;
    own.state(0, 1, [false; 2]).await?;
    other.state(1, 1, [false; 2]).await?;
    other
        .intent(
            trade::Intent::Offer {
                partner: reference(0),
                item: 71,
            },
            trade::ResultCode::Offered,
        )
        .await?;
    other.state(1, 2, [false; 2]).await?;
    own.state(0, 2, [false; 2]).await?;
    own.intent(
        trade::Intent::Accept { trade_id: [3; 16] },
        trade::ResultCode::Accepted,
    )
    .await?;
    own.state(0, 2, [true, false]).await?;
    other.state(1, 2, [false, true]).await?;
    other
        .intent(
            trade::Intent::Accept { trade_id: [3; 16] },
            trade::ResultCode::Completed,
        )
        .await?;
    other.state(1, 4, [false; 2]).await?;
    own.state(0, 4, [false; 2]).await?;
    own.inventory(0).await?;
    other.inventory(1).await?;
    let bytes = read_frame(&mut other.stream).await?;
    let envelope = wire::decode_wire_envelope(&bytes)?;
    let command = envelope.client_command(1)?;
    assert_eq!(command.command_type, logout::COMMAND_TYPE_LOGOUT_INTENT);
    other.sequence += 1;
    write_frame(
        &mut other.stream,
        &wire::encode_command_result(
            1,
            other.sequence,
            command.command_id,
            wire::CommandStatus::Accepted,
            &encoded(logout::encode_logout_result(logout::LogoutResult::Accepted))?,
        )?,
    )
    .await?;
    Ok(())
}
#[test]
fn qualification_player_trade_pair_phase_consumes_each_command_delta() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?
                .block_on(async {
                    let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
                    let certificate = generated.cert.der().clone();
                    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
                    let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
                        rustls::crypto::aws_lc_rs::default_provider(),
                    ))
                    .with_protocol_versions(&[&rustls::version::TLS13])?
                    .with_no_client_auth()
                    .with_single_cert(vec![certificate.clone()], key.into())?;
                    config.alpn_protocols = vec![b"oteryn-game/1".to_vec()];
                    let listener = TcpListener::bind("127.0.0.1:0").await?;
                    let address = listener.local_addr()?;
                    let task = tokio::spawn(peer(listener, TlsAcceptor::from(Arc::new(config))));
                    let request = |character_id| JoinRequest {
                        address,
                        server_name: "localhost",
                        root_certificate: &certificate,
                        schema_revision: 1,
                        character_id,
                        admission_material: b"scripted qualification",
                        client_build_id: "c21-pair",
                        deadline: Duration::from_secs(3),
                    };
                    let mut client =
                        connect_session(request(wire::CharacterId::decode(&uuid(10))?)).await?;
                    let partner =
                        connect_session(request(wire::CharacterId::decode(&uuid(11))?)).await?;
                    let clients = SeamClients {
                        partner: Mutex::new(Some(partner)),
                        evidence: "scripted TLS",
                    };
                    let result = tokio::time::timeout(Duration::from_secs(10), async {
                        phase::player_trade(&mut client).await?;
                        client.session_mut().read_gated_delta(26, 1).await?;
                        use oteryn_dev_client::client_runtime::player_trade as runtime;
                        let command = runtime::PlayerTradeCommand::new(trade::Intent::Accept {
                            trade_id: [3; 16],
                        })
                        .ok_or("repeat accept command")?;
                        let repeated = runtime::send(client.session_mut(), &command).await?;
                        assert_eq!(repeated.result, trade::ResultCode::Accepted);
                        assert!(repeated.state.accepted[0]);
                        let command = runtime::PlayerTradeCommand::new(trade::Intent::Cancel)
                            .ok_or("cancel command")?;
                        let cancelled = runtime::send(client.session_mut(), &command).await?;
                        assert_eq!(cancelled.state.phase, 4);
                        phase::pair(&mut client, &clients).await
                    })
                    .await?;
                    drop(client);
                    result?;
                    task.await?.map_err(|e| e.to_string())?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "qualification pair thread panicked")?
        .map_err(Into::into)
}

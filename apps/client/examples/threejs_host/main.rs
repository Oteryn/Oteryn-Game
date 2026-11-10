//! Development-only Three.js presentation of the real native client session (#3099).
//! No browser transport profile, credential endpoint or alternate gameplay codec.
mod presentation;

use oteryn_client::input::StepDir;
use oteryn_client::play::{PlayView, play_channel, run_session};
use oteryn_protocol_oteryn::CharacterId;
use oteryn_renderer::TileCoord;
use oteryn_session::{Admission, CLIENT_SUPPORTED_CAPABILITIES, Session};
use oteryn_session_tcp::{TcpConnect, connect};
use rustls::pki_types::CertificateDer;
use std::error::Error;
use std::io::{Read, stdin};
use std::net::SocketAddr;
use std::path::Path;
use std::process::ExitCode;
use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

fn main() -> ExitCode {
    // The native session has the same stack requirement as qualification clients.
    let result = std::thread::Builder::new()
        .name("threejs-host".into())
        .stack_size(64 << 20)
        .spawn(run)
        .map_err(|_| "host thread unavailable")
        .and_then(|thread| thread.join().map_err(|_| "host thread ended"))
        .and_then(|result| result.map_err(|_| "native session unavailable"));
    if result.is_err() {
        // Never print admission material, configuration contents or session error Debug values.
        eprintln!("Three.js native host unavailable; check the local qualification configuration.");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [address, server_name, root_file, grant_file, character] = args.as_slice() else {
        return Err("expected address, TLS name, root DER, grant file, character hex".into());
    };
    let address: SocketAddr = address.parse().map_err(|_| "invalid address")?;
    // Deliberately cannot become a production endpoint/relay by supplying another address.
    if !address.ip().is_loopback() {
        return Err("development host requires a loopback Game endpoint".into());
    }
    let character_id = CharacterId::decode(&character_bytes(character)?)?;
    let root = CertificateDer::from(read_capped(Path::new(root_file), 64 * 1024)?);
    let grant = read_capped(Path::new(grant_file), 64 * 1024)?;
    if grant.trim_ascii().is_empty() {
        return Err("empty admission material".into());
    }
    let input = input_reader()?;
    let output = presentation::writer()?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let deadline = Duration::from_secs(20);
            let stream = connect(TcpConnect {
                address,
                server_name,
                root_certificate: &root,
                deadline,
            })
            .await?;
            let session = Session::admit(
                stream,
                Admission {
                    appearance_items: None,
                    schema_revision: 1,
                    character_id,
                    admission_material: grant.trim_ascii(),
                    client_build_id: oteryn_client::CLIENT_BUILD,
                    supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
                    deadline,
                },
            )
            .await?;
            // No B3/offline map substitution when a live map has not been admitted.
            let map = session.world_map_snapshot().ok_or("live map unavailable")?;
            let own = session.world_spatial().actor_position;
            let mut view = PlayView::new(
                TileCoord::new(own.x, own.y),
                own.floor,
                session.world_object_overlay(),
            )?;
            view.set_live_map(&map)?;
            let (link, commands, outbox) = play_channel();
            // Cancelling either future drops the native session. No detached networking task.
            tokio::select! {
                () = run_session(session, commands, outbox) => Err("session ended".into()),
                result = async {
                    loop {
                        match input.try_recv() {
                            Ok(byte) => view.arrow(direction(byte)?),
                            Err(TryRecvError::Empty) => {},
                            Err(TryRecvError::Disconnected) => return Ok(()),
                        }
                        view.tick(&link).map_err(|_| "play view ended")?;
                        match output.try_reserve() {
                            Ok(permit) => permit.send(presentation::Frame::from_scene(view.scene())),
                            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {},
                            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                return Err("presentation pipe closed".into());
                            },
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                } => result,
            }
        })
}

fn read_capped(path: &Path, cap: u64) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path).map_err(|_| "configuration unavailable")?;
    let mut bytes = Vec::new();
    file.take(cap + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > cap {
        return Err("configuration exceeds bound".into());
    }
    Ok(bytes)
}

fn character_bytes(text: &str) -> Result<[u8; 16]> {
    if text.len() != 32 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("character must be 32 hexadecimal digits".into());
    }
    let mut bytes = [0; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)?;
    }
    Ok(bytes)
}

fn direction(byte: u8) -> Result<StepDir> {
    match byte {
        b'N' => Ok(StepDir::North),
        b'E' => Ok(StepDir::East),
        b'S' => Ok(StepDir::South),
        b'W' => Ok(StepDir::West),
        _ => Err("invalid presentation input".into()),
    }
}

fn input_reader() -> Result<Receiver<u8>> {
    let (sender, receiver) = sync_channel(1);
    let _reader = std::thread::Builder::new()
        .name("threejs-input".into())
        .spawn(move || {
            let mut input = stdin().lock();
            let mut byte = [0];
            while input.read_exact(&mut byte).is_ok() {
                if direction(byte[0]).is_err() || sender.send(byte[0]).is_err() {
                    break;
                }
            }
        })?;
    Ok(receiver)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_and_character_parse_fail_closed() -> Result<()> {
        assert!(direction(b'N').is_ok());
        assert!(direction(b'X').is_err());
        assert!(character_bytes("not-an-id").is_err());
        assert_eq!(character_bytes("00000000000000000000000000000001")?[15], 1);
        Ok(())
    }
}

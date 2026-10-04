//! Live mode entry: argument/env parsing, line-command -> input-event translation, and the
//! terminal loop around [`LiveController`]. Everything except `run` is pure.
//!
//! The grant is a secret: it is only ever read from a file or the environment, never a flag.

use super::controller::LiveController;
use super::input::LiveInput;
use super::model::{DOOR_TILE, RenderModel, Viewport, render_text, tile_centre_pixel};
use oteryn_dev_client::{JoinRequest, connect_session};
use oteryn_input_actions::{
    ButtonState, InputError, KeyCode, Modifiers, MouseButton, NormalizedInputEvent,
    PointerCoordinate, PointerDelta, PointerMotion, PointerPosition,
};
use oteryn_protocol_oteryn::CharacterId;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

const DEFAULT_SURFACE: (u32, u32) = (640, 384);
const IDLE_SLICE: Duration = Duration::from_millis(200);
const JOIN_DEADLINE: Duration = Duration::from_secs(30);
const CLIENT_BUILD_ID: &str = "oteryn-synthetic-client-harness-live";

pub const USAGE: &str = "\
live mode (dev/qualification only):
  synthetic-client-harness --live --addr HOST:PORT --ca CA.pem|der --character-id UUID \\
      (--grant-file FILE | env OTERYN_LIVE_GRANT) [--server-name NAME] [--surface WxH]
  env fallbacks: OTERYN_LIVE_ADDR OTERYN_LIVE_CA OTERYN_LIVE_CHARACTER_ID
                 OTERYN_LIVE_SERVER_NAME OTERYN_LIVE_GRANT_FILE OTERYN_LIVE_GRANT
input lines: up down left right (or w a s d) | use | click PX PY | quit";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantSource {
    File(PathBuf),
    Inline(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveArgs {
    pub address: SocketAddr,
    pub server_name: String,
    pub ca_path: PathBuf,
    pub grant: GrantSource,
    pub character_id: String,
    pub surface: (u32, u32),
}

/// Parses the arguments after `--live`, with `env` as the fallback for every value.
///
/// # Errors
///
/// A message naming the missing or malformed value.
pub fn parse_args(
    args: &[String],
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<LiveArgs, String> {
    let mut address = None;
    let mut server_name = None;
    let mut ca = None;
    let mut grant_file = None;
    let mut character_id = None;
    let mut surface = None;
    let mut rest = args.iter().filter(|arg| arg.as_str() != "--live");
    while let Some(flag) = rest.next() {
        let slot = match flag.as_str() {
            "--addr" => &mut address,
            "--server-name" => &mut server_name,
            "--ca" => &mut ca,
            "--grant-file" => &mut grant_file,
            "--character-id" => &mut character_id,
            "--surface" => &mut surface,
            other => return Err(format!("unknown live argument `{other}`")),
        };
        *slot = Some(
            rest.next()
                .ok_or_else(|| format!("`{flag}` needs a value"))?
                .clone(),
        );
    }
    let need = |value: Option<String>, flag: &str, var: &str| {
        value
            .or_else(|| env(var))
            .ok_or_else(|| format!("missing {flag} (or env {var})"))
    };
    let address_text = need(address, "--addr", "OTERYN_LIVE_ADDR")?;
    let address = address_text
        .parse()
        .map_err(|_| format!("invalid address `{address_text}`"))?;
    let grant = match grant_file.or_else(|| env("OTERYN_LIVE_GRANT_FILE")) {
        Some(path) => GrantSource::File(PathBuf::from(path)),
        None => GrantSource::Inline(env("OTERYN_LIVE_GRANT").ok_or_else(|| {
            "missing grant: --grant-file (or env OTERYN_LIVE_GRANT_FILE / OTERYN_LIVE_GRANT)"
                .to_owned()
        })?),
    };
    Ok(LiveArgs {
        address,
        server_name: server_name
            .or_else(|| env("OTERYN_LIVE_SERVER_NAME"))
            .unwrap_or_else(|| "localhost".to_owned()),
        ca_path: PathBuf::from(need(ca, "--ca", "OTERYN_LIVE_CA")?),
        grant,
        character_id: need(character_id, "--character-id", "OTERYN_LIVE_CHARACTER_ID")?,
        surface: surface.map_or(Ok(DEFAULT_SURFACE), |text| parse_surface(&text))?,
    })
}

fn parse_surface(text: &str) -> Result<(u32, u32), String> {
    let invalid = || format!("invalid surface `{text}` (want WIDTHxHEIGHT)");
    let (width, height) = text.split_once('x').ok_or_else(invalid)?;
    Ok((
        width.parse().map_err(|_| invalid())?,
        height.parse().map_err(|_| invalid())?,
    ))
}

/// A canonical or bare-hex UUID -> `CharacterId` (which itself demands a v7 UUID).
///
/// # Errors
///
/// A message when the text is not 32 hex digits or not a UUID v7.
pub fn parse_character_id(text: &str) -> Result<CharacterId, String> {
    let digits: Vec<u8> = text.bytes().filter(|byte| *byte != b'-').collect();
    if digits.len() != 32 {
        return Err(format!("character id `{text}` is not a UUID"));
    }
    let mut bytes = [0_u8; 16];
    for (slot, pair) in bytes.iter_mut().zip(digits.chunks(2)) {
        let pair = std::str::from_utf8(pair).map_err(|_| "character id is not ASCII".to_owned())?;
        *slot = u8::from_str_radix(pair, 16)
            .map_err(|_| format!("character id `{text}` is not hexadecimal"))?;
    }
    CharacterId::decode(&bytes).map_err(|error| format!("character id `{text}`: {error:?}"))
}

/// A PEM or DER certificate file -> the single trust root.
///
/// # Errors
///
/// A message when the file is unreadable or holds no certificate.
pub fn load_root_certificate(path: &std::path::Path) -> Result<CertificateDer<'static>, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    if bytes.starts_with(b"-----BEGIN") {
        CertificateDer::from_pem_slice(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))
    } else {
        Ok(CertificateDer::from(bytes))
    }
}

/// One line typed into the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCommand {
    Key(KeyCode),
    /// Click the door tile.
    UseDoor,
    Click(i32, i32),
    Quit,
}

#[must_use]
pub fn parse_line(line: &str) -> Option<LineCommand> {
    let mut words = line.split_whitespace();
    let command = match words.next()? {
        "up" | "w" => LineCommand::Key(KeyCode::ARROW_UP),
        "down" | "s" => LineCommand::Key(KeyCode::ARROW_DOWN),
        "left" | "a" => LineCommand::Key(KeyCode::ARROW_LEFT),
        "right" | "d" => LineCommand::Key(KeyCode::ARROW_RIGHT),
        "use" => LineCommand::UseDoor,
        "click" => LineCommand::Click(words.next()?.parse().ok()?, words.next()?.parse().ok()?),
        "quit" | "q" => LineCommand::Quit,
        _ => return None,
    };
    words.next().is_none().then_some(command)
}

/// The normalized input events a line stands for (a key press and release, or a pointer move and
/// a primary click). `UseDoor` clicks the door tile's centre; off the grid it yields nothing.
///
/// # Errors
///
/// The input crate's error for an out-of-range pointer coordinate.
pub fn events_for(
    command: LineCommand,
    view: Viewport,
    model: &RenderModel,
) -> Result<Vec<NormalizedInputEvent>, InputError> {
    let key = |code, state| NormalizedInputEvent::Key {
        code,
        state,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    let click = |px: i32, py: i32| -> Result<Vec<NormalizedInputEvent>, InputError> {
        let button = |state| NormalizedInputEvent::MouseButton {
            button: MouseButton::PRIMARY,
            state,
            modifiers: Modifiers::NONE,
        };
        Ok(vec![
            NormalizedInputEvent::PointerMoved {
                position: PointerPosition::new(
                    PointerCoordinate::new(px)?,
                    PointerCoordinate::new(py)?,
                ),
                motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
            },
            button(ButtonState::Pressed),
            button(ButtonState::Released),
        ])
    };
    match command {
        LineCommand::Key(code) => Ok(vec![
            key(code, ButtonState::Pressed),
            key(code, ButtonState::Released),
        ]),
        LineCommand::Click(px, py) => click(px, py),
        LineCommand::UseDoor => match tile_centre_pixel(view, model.actor, DOOR_TILE) {
            Some((px, py)) => click(px, py),
            None => Ok(Vec::new()),
        },
        LineCommand::Quit => Ok(Vec::new()),
    }
}

/// The admission material as the server expects it: a compact JWS never ends in a line break, so
/// the trailing `\n`/`\r\n` an editor or `echo` leaves in a grant file (or an env value) is removed.
pub fn grant_material(mut raw: Vec<u8>) -> Vec<u8> {
    while matches!(raw.last(), Some(b'\n' | b'\r')) {
        raw.pop();
    }
    raw
}

/// Connects, then runs the terminal loop until `quit` or end of input.
///
/// # Errors
///
/// Bad arguments, an unreadable CA or grant, a failed join, or any session failure.
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let parsed = parse_args(args, &|name| std::env::var(name).ok())?;
    let root = load_root_certificate(&parsed.ca_path)?;
    let grant = grant_material(match &parsed.grant {
        GrantSource::File(path) => std::fs::read(path)
            .map_err(|error| format!("read grant {}: {error}", path.display()))?,
        GrantSource::Inline(text) => text.clone().into_bytes(),
    });
    let character_id = parse_character_id(&parsed.character_id)?;
    let view = Viewport::configure(parsed.surface.0, parsed.surface.1)?;
    let input = LiveInput::new()?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let session = connect_session(JoinRequest {
                address: parsed.address,
                server_name: &parsed.server_name,
                root_certificate: &root,
                schema_revision: 1,
                character_id,
                admission_material: &grant,
                client_build_id: CLIENT_BUILD_ID,
                deadline: JOIN_DEADLINE,
            })
            .await?;
            let mut controller = LiveController::new(session, view, input);
            println!("{USAGE}\n\n{}", render_text(view, controller.model()));

            let (lines, receiver) = std::sync::mpsc::channel::<String>();
            std::thread::spawn(move || {
                let stdin = std::io::stdin();
                let mut line = String::new();
                while matches!(stdin.read_line(&mut line), Ok(count) if count > 0) {
                    if lines.send(std::mem::take(&mut line)).is_err() {
                        break;
                    }
                }
            });

            loop {
                match receiver.try_recv() {
                    Ok(line) => match parse_line(&line) {
                        Some(LineCommand::Quit) => break,
                        Some(command) => {
                            for event in events_for(command, controller.view(), controller.model())?
                            {
                                if controller.handle_event(&event).await? {
                                    println!("{}", render_text(view, controller.model()));
                                }
                            }
                        }
                        None if line.trim().is_empty() => {}
                        None => println!("unrecognised input\n{USAGE}"),
                    },
                    Err(TryRecvError::Empty) => {
                        if controller.idle(IDLE_SLICE).await? {
                            println!("{}", render_text(view, controller.model()));
                        }
                    }
                    Err(TryRecvError::Disconnected) => break,
                }
            }
            Ok::<(), Box<dyn Error>>(())
        })
}

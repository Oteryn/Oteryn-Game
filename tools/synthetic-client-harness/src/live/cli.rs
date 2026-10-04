//! Live mode entry: argument/env parsing, line-command -> input-event translation, and the
//! terminal loop around [`LiveController`]. Everything except `run` is pure.
//!
//! The grant is a secret: it is only ever read from a file or the environment, never a flag.

use super::controller::LiveController;
use super::input::LiveInput;
use super::model::{DOOR_TILE, LiveCommand, RenderModel, Viewport, render_text, tile_centre_pixel};
use oteryn_dev_client::{
    ChatIntent, ChatRoom, ChatSpeechMode, JoinRequest, MAX_CHAT_NAME_BYTES, MAX_CHAT_TEXT_BYTES,
    connect_session,
};
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

/// The chat help, shown only when the server selected capability 7.
pub const CHAT_USAGE: &str = "\
chat lines:  say TEXT | yell TEXT | whisper TEXT | pm NAME TEXT | pm \"NAME WITH SPACES\" TEXT
             room N TEXT | open N | close N (rooms: 1 World, 2 English, 3 Help, 4 Advertising)";

/// The item help, shown only when the server selected capability 4.
pub const ITEM_USAGE: &str = "\
item lines:  click a corpse to open it | loot N (move entry N of the open corpse to the backpack)";

/// The help for what the server selected: the chat and item lines appear only with their
/// capability.
#[must_use]
pub fn usage_for(model: &RenderModel) -> String {
    let mut text = USAGE.to_owned();
    if model.chat.is_some() {
        text.push('\n');
        text.push_str(CHAT_USAGE);
    }
    if model.items.is_some() {
        text.push('\n');
        text.push_str(ITEM_USAGE);
    }
    text
}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineCommand {
    Key(KeyCode),
    /// One chat intent (`say`, `yell`, `whisper`, `pm`, `room`, `open`, `close`).
    Chat(ChatIntent),
    /// Move entry N (1-based) of the open corpse to the backpack.
    Loot(usize),
    /// Click the door tile.
    UseDoor,
    Click(i32, i32),
    Quit,
}

fn room_for(word: &str) -> Option<ChatRoom> {
    match word {
        "1" => Some(ChatRoom::World),
        "2" => Some(ChatRoom::English),
        "3" => Some(ChatRoom::Help),
        "4" => Some(ChatRoom::Advertising),
        _ => None,
    }
}

/// `word` and the rest of `text` after it, trimmed (the chat text keeps its inner spacing).
fn split_word(text: &str) -> Option<(&str, &str)> {
    let text = text.trim();
    let end = text.find(char::is_whitespace).unwrap_or(text.len());
    let (word, rest) = text.split_at(end);
    (!word.is_empty()).then_some((word, rest.trim()))
}

/// The `pm` recipient and the text after it: a name in double quotes (`"Al Dric"`, which may hold
/// spaces) or one unquoted word. An empty or over-long name is no recipient.
fn split_recipient(text: &str) -> Option<(&str, &str)> {
    let text = text.trim();
    let (name, body) = match text.strip_prefix('"') {
        Some(quoted) => {
            let (name, rest) = quoted.split_once('"')?;
            (name, rest.trim())
        }
        None => split_word(text)?,
    };
    (!name.is_empty() && name.len() <= MAX_CHAT_NAME_BYTES).then_some((name, body))
}

/// A chat line, or `None` when it is not one (or its text is empty or over the wire bound).
fn parse_chat(line: &str) -> Option<ChatIntent> {
    let (verb, rest) = split_word(line)?;
    let text = |text: &str| {
        (!text.is_empty() && text.len() <= MAX_CHAT_TEXT_BYTES).then(|| text.to_owned())
    };
    let say = |mode| text(rest).map(|text| ChatIntent::Say { mode, text });
    match verb {
        "say" => say(ChatSpeechMode::Say),
        "yell" => say(ChatSpeechMode::Yell),
        "whisper" => say(ChatSpeechMode::Whisper),
        "pm" => {
            let (name, body) = split_recipient(rest)?;
            Some(ChatIntent::Private {
                recipient_name: name.to_owned(),
                text: text(body)?,
            })
        }
        "room" => {
            let (room, body) = split_word(rest)?;
            Some(ChatIntent::Room {
                room: room_for(room)?,
                text: text(body)?,
            })
        }
        "open" => Some(ChatIntent::OpenRoom(room_for(rest)?)),
        "close" => Some(ChatIntent::CloseRoom(room_for(rest)?)),
        _ => None,
    }
}

#[must_use]
pub fn parse_line(line: &str) -> Option<LineCommand> {
    if let Some(intent) = parse_chat(line) {
        return Some(LineCommand::Chat(intent));
    }
    let mut words = line.split_whitespace();
    let command = match words.next()? {
        "up" | "w" => LineCommand::Key(KeyCode::ARROW_UP),
        "down" | "s" => LineCommand::Key(KeyCode::ARROW_DOWN),
        "left" | "a" => LineCommand::Key(KeyCode::ARROW_LEFT),
        "right" | "d" => LineCommand::Key(KeyCode::ARROW_RIGHT),
        "use" => LineCommand::UseDoor,
        "loot" => {
            let entry: usize = words.next()?.parse().ok()?;
            (entry > 0).then_some(LineCommand::Loot(entry))?
        }
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
        // Chat and loot are not input events: the loop dispatches them as commands.
        LineCommand::Chat(_) | LineCommand::Loot(_) | LineCommand::Quit => Ok(Vec::new()),
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
            println!(
                "{}\n\n{}",
                usage_for(controller.model()),
                render_text(view, controller.model())
            );

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
                        Some(LineCommand::Chat(intent)) => {
                            controller.dispatch(LiveCommand::Chat(intent)).await?;
                            println!("{}", render_text(view, controller.model()));
                        }
                        Some(LineCommand::Loot(entry)) => {
                            controller.dispatch(LiveCommand::Loot { entry }).await?;
                            println!("{}", render_text(view, controller.model()));
                        }
                        Some(command) => {
                            for event in events_for(command, controller.view(), controller.model())?
                            {
                                if controller.handle_event(&event).await? {
                                    println!("{}", render_text(view, controller.model()));
                                }
                            }
                        }
                        None if line.trim().is_empty() => {}
                        None => {
                            println!("unrecognised input\n{}", usage_for(controller.model()));
                        }
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

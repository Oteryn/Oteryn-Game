//! Live mode entry: argument/env parsing, line-command -> input-event translation, and the
//! terminal loop around [`LiveController`]. Everything except `run` is pure.
//!
//! The grant is a secret: it is only ever read from a file or the environment, never a flag.

use super::controller::LiveController;
use super::input::LiveInput;
use super::model::{DOOR_TILE, LiveCommand, RenderModel, Viewport, render_text, tile_centre_pixel};
use oteryn_dev_client::{
    ChatIntent, ChatRoom, ChatSpeechMode, JoinRequest, MAX_CHAT_TEXT_BYTES, connect_session,
};
use oteryn_input_actions::{
    ButtonState, InputError, KeyCode, Modifiers, MouseButton, NormalizedInputEvent,
    PointerCoordinate, PointerDelta, PointerMotion, PointerPosition,
};
use oteryn_protocol_oteryn::CharacterId;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget, SpellTargetPosition};
use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;
use std::error::Error;
use std::io::Read;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

const DEFAULT_SURFACE: (u32, u32) = (640, 384);
const IDLE_SLICE: Duration = Duration::from_millis(200);
const JOIN_DEADLINE: Duration = Duration::from_secs(30);
const CLIENT_BUILD_ID: &str = "oteryn-synthetic-client-harness-live";
const MAX_SCRIPT_BYTES: usize = 1_048_576;
const MAX_SCRIPT_LINES: usize = 4096;
const MAX_WAIT_MS: u64 = 30_000;
const MAX_SCRIPT_WAIT: Duration = Duration::from_secs(300);

pub const USAGE: &str = "\
live mode (dev/qualification only):
  synthetic-client-harness --live --addr HOST:PORT --ca CA.pem|der --character-id UUID \\
      (--grant-file FILE | env OTERYN_LIVE_GRANT) [--server-name NAME] [--surface WxH] [--script FILE]
  env fallbacks: OTERYN_LIVE_ADDR OTERYN_LIVE_CA OTERYN_LIVE_CHARACTER_ID
                 OTERYN_LIVE_SERVER_NAME OTERYN_LIVE_GRANT_FILE OTERYN_LIVE_GRANT
input lines: up down left right (or w a s d) | use | click PX PY | quit
             cast SPELL_INDEX self|none|attack|position X Y FLOOR [aim]
             wait MS (0..30000; connection remains serviced)
             expect Cast|CoolingDown|LevelTooLow|MagicLevelTooLow|NotEnoughMana|NotEnoughSoul|NotAvailable|TargetRequired|TargetIllegal|Rejected
chat lines:  say TEXT | yell TEXT | whisper TEXT | pm NAME TEXT | room N TEXT | open N | close N
             (rooms: 1 World, 2 English, 3 Help, 4 Advertising)
script: same commands, blank lines and # comments; at most 4096 lines / 1 MiB / 5 min waits";

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
    pub script_path: Option<PathBuf>,
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
    let mut script = None;
    let mut rest = args.iter().filter(|arg| arg.as_str() != "--live");
    while let Some(flag) = rest.next() {
        let slot = match flag.as_str() {
            "--addr" => &mut address,
            "--server-name" => &mut server_name,
            "--ca" => &mut ca,
            "--grant-file" => &mut grant_file,
            "--character-id" => &mut character_id,
            "--surface" => &mut surface,
            "--script" => &mut script,
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
        script_path: script.map(PathBuf::from),
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
    /// Click the door tile.
    UseDoor,
    Click(i32, i32),
    Cast {
        spell: NonZeroU32,
        target: SpellTarget,
        aim_at_target: bool,
    },
    Wait(Duration),
    Expect(SpellCastDisposition),
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
            let (name, body) = split_word(rest)?;
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
        "click" => LineCommand::Click(words.next()?.parse().ok()?, words.next()?.parse().ok()?),
        "cast" => {
            let spell = words.next()?.parse().ok()?;
            let target = match words.next()? {
                "self" | "none" => SpellTarget::None,
                "attack" => SpellTarget::AttackTarget,
                "position" => SpellTarget::Position(SpellTargetPosition {
                    x: words.next()?.parse().ok()?,
                    y: words.next()?.parse().ok()?,
                    floor: words.next()?.parse().ok()?,
                }),
                _ => return None,
            };
            let aim_at_target = match words.next() {
                Some("aim") => true,
                None => false,
                _ => return None,
            };
            LineCommand::Cast {
                spell,
                target,
                aim_at_target,
            }
        }
        "wait" => {
            let milliseconds: u64 = words.next()?.parse().ok()?;
            if milliseconds > MAX_WAIT_MS {
                return None;
            }
            LineCommand::Wait(Duration::from_millis(milliseconds))
        }
        "expect" => LineCommand::Expect(match words.next()? {
            "Cast" => SpellCastDisposition::Cast,
            "CoolingDown" => SpellCastDisposition::CoolingDown,
            "LevelTooLow" => SpellCastDisposition::LevelTooLow,
            "MagicLevelTooLow" => SpellCastDisposition::MagicLevelTooLow,
            "NotEnoughMana" => SpellCastDisposition::NotEnoughMana,
            "NotEnoughSoul" => SpellCastDisposition::NotEnoughSoul,
            "NotAvailable" => SpellCastDisposition::NotAvailable,
            "TargetRequired" => SpellCastDisposition::TargetRequired,
            "TargetIllegal" => SpellCastDisposition::TargetIllegal,
            "Rejected" => SpellCastDisposition::Rejected,
            _ => return None,
        }),
        "quit" | "q" => LineCommand::Quit,
        _ => return None,
    };
    words.next().is_none().then_some(command)
}

/// Validates an entire batch before any command is sent. Spell indices belong to the loaded
/// content generation; they are not source spell names or upstream numeric identifiers.
///
/// # Errors
/// An invalid command, commands after `quit`, or a script exceeding its size/wait limits.
pub fn parse_script(text: &str) -> Result<Vec<LineCommand>, String> {
    if text.len() > MAX_SCRIPT_BYTES {
        return Err("script exceeds 1 MiB".to_owned());
    }
    let mut commands = Vec::new();
    let mut total_wait = Duration::ZERO;
    for (index, line) in text.lines().enumerate() {
        if index >= MAX_SCRIPT_LINES {
            return Err("script exceeds 4096 lines".to_owned());
        }
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if commands.last() == Some(&LineCommand::Quit) {
            return Err(format!("script line {} follows quit", index + 1));
        }
        let command = parse_line(line)
            .ok_or_else(|| format!("invalid script command at line {}", index + 1))?;
        if let LineCommand::Wait(duration) = command {
            total_wait += duration;
            if total_wait > MAX_SCRIPT_WAIT {
                return Err("script waits exceed 5 minutes".to_owned());
            }
        }
        commands.push(command);
    }
    Ok(commands)
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
        // Chat is not an input event: the loop dispatches it as a command.
        LineCommand::Cast { .. }
        | LineCommand::Wait(_)
        | LineCommand::Expect(_)
        | LineCommand::Chat(_)
        | LineCommand::Quit => Ok(Vec::new()),
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
    let script = parsed
        .script_path
        .as_ref()
        .map(|path| {
            let mut text = String::new();
            std::fs::File::open(path)?
                .take((MAX_SCRIPT_BYTES + 1) as u64)
                .read_to_string(&mut text)?;
            Ok::<_, Box<dyn Error>>(parse_script(&text)?.into_iter())
        })
        .transpose()?;
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

            let scripted = script.is_some();
            let mut script = script;
            let mut command_count = 0_u32;
            let mut cast_count = 0_u32;
            let mut expectation_count = 0_u32;
            let receiver = if script.is_none() {
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
                Some(receiver)
            } else {
                None
            };

            loop {
                let command = if let Some(commands) = &mut script {
                    let Some(command) = commands.next() else {
                        break;
                    };
                    command
                } else {
                    let Some(receiver) = &receiver else { break };
                    match receiver.try_recv() {
                        Ok(line) => match parse_line(&line) {
                            Some(command) => command,
                            None if line.trim().is_empty() => continue,
                            None => {
                                println!("unrecognised input\n{USAGE}");
                                continue;
                            }
                        },
                        Err(TryRecvError::Empty) => {
                            if controller.idle(IDLE_SLICE).await? {
                                println!("{}", render_text(view, controller.model()));
                            }
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => break,
                    }
                };
                command_count += 1;
                match command {
                    LineCommand::Quit => break,
                    LineCommand::Chat(intent) => {
                        controller.dispatch(LiveCommand::Chat(intent)).await?;
                        println!("{}", render_text(view, controller.model()));
                    }
                    LineCommand::Wait(duration) => {
                        if controller.idle(duration).await? {
                            println!("{}", render_text(view, controller.model()));
                        }
                    }
                    LineCommand::Expect(expected) => {
                        let actual = controller.last_cast().map(|outcome| outcome.disposition);
                        if actual != Some(expected) {
                            return Err(format!(
                                "expected spell disposition {expected:?}, got {actual:?}"
                            )
                            .into());
                        }
                        expectation_count += 1;
                        println!("expect {expected:?}: passed");
                    }
                    LineCommand::Cast {
                        spell,
                        target,
                        aim_at_target,
                    } => {
                        println!("cast request: spell={spell} target={target:?} aim={aim_at_target}");
                        controller
                            .dispatch(LiveCommand::Cast {
                                spell,
                                target,
                                aim_at_target,
                            })
                            .await?;
                        cast_count += 1;
                        println!("{}", controller.status_text());
                    }
                    command => {
                        for event in events_for(command, controller.view(), controller.model())? {
                            if controller.handle_event(&event).await? {
                                println!("{}", render_text(view, controller.model()));
                            }
                        }
                    }
                }
            }
            if scripted {
                println!("scenario completed: commands={command_count} casts={cast_count} expectations={expectation_count}");
            }
            Ok::<(), Box<dyn Error>>(())
        })
}

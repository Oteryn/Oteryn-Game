//! Replay the original server bytes for the C12 equipped admission and spell exchange.

use super::*;
use oteryn_protocol_oteryn::actor_spell::{self, ActorVitals, SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::world_spatial::{
    ActorPosition, StepDirection, StepDisposition, decode_step_intent_profile,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_protocol_oteryn::{FrameLength, MessageType, decode_framed_envelope};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::io;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[path = "../../../../tools/qualification/c12_frame_capture.rs"]
mod frame_capture;

struct Witness {
    prior: [u8; 16],
    session: [u8; 16],
    attack_command: u64,
    command: u64,
    result_sequence: u64,
    delta_sequence: u64,
    spell: NonZeroU32,
    target: EntityRef,
    definition: NonZeroU32,
    mana_before: u32,
    damage: u64,
    before_sequence: u64,
    before_bytes: usize,
    builder: NonZeroU32,
    builder_damage: u64,
    builder_start: usize,
    builder_sequence: u64,
    clear_start: usize,
    clear_sequence: u64,
    logout_start: usize,
    logout_sequence: u64,
    vitals: ActorVitals,
    traveled: Vec<StepDirection>,
    cast_tile: ActorPosition,
}

fn hex<const N: usize>(value: &str) -> Result<[u8; N], BoxError> {
    if value.len() != 2 * N || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("invalid C12 identity".into());
    }
    let mut bytes = [0; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[2 * index..2 * index + 2], 16)?;
    }
    Ok(bytes)
}

fn read_witness(root: &Path, text: &str) -> Result<Witness, BoxError> {
    let (run, values) = text.split_once('\n').ok_or("C12 run provenance absent")?;
    if Path::new(run) != root {
        return Err("stale C12 capture from a different run".into());
    }
    let values: Vec<_> = values.split_whitespace().collect();
    let [
        session,
        prior,
        command,
        result,
        delta,
        attack,
        spell,
        target,
        generation,
        definition,
        before,
        health,
        max_health,
        mana,
        max_mana,
        soul,
        harmony,
        serene,
        damage,
        damage_before,
        damage_augmented,
        damage_critical,
        before_sequence,
        before_bytes,
        builder,
        builder_damage,
        builder_start,
        builder_sequence,
        clear_start,
        clear_sequence,
        logout_start,
        logout_sequence,
        traveled,
        x,
        y,
        floor,
    ] = values.as_slice()
    else {
        return Err("malformed C12 capture provenance".into());
    };
    if !(1..=512).contains(&traveled.len()) {
        return Err("C12 original movement count outside bounds".into());
    }
    let traveled = traveled
        .bytes()
        .map(|value| {
            let direction = value.checked_sub(b'0').ok_or("C12 invalid movement")?;
            decode_step_intent_profile(&[8, direction], true).map_err(|_| "C12 invalid movement")
        })
        .collect::<Result<_, _>>()?;
    let witness = Witness {
        prior: hex(prior)?,
        session: hex(session)?,
        attack_command: attack.parse()?,
        command: command.parse()?,
        result_sequence: result.parse()?,
        delta_sequence: delta.parse()?,
        spell: spell.parse()?,
        target: EntityRef {
            identity: hex(target)?,
            generation: generation.parse()?,
        },
        definition: definition.parse()?,
        mana_before: before.parse()?,
        damage: damage.parse()?,
        before_sequence: before_sequence.parse()?,
        before_bytes: before_bytes.parse()?,
        builder: builder.parse()?,
        builder_damage: builder_damage.parse()?,
        builder_start: builder_start.parse()?,
        builder_sequence: builder_sequence.parse()?,
        clear_start: clear_start.parse()?,
        clear_sequence: clear_sequence.parse()?,
        logout_start: logout_start.parse()?,
        logout_sequence: logout_sequence.parse()?,
        traveled,
        cast_tile: ActorPosition {
            x: x.parse()?,
            y: y.parse()?,
            floor: floor.parse()?,
        },
        vitals: ActorVitals {
            health: health.parse()?,
            max_health: max_health.parse()?,
            mana: mana.parse()?,
            max_mana: max_mana.parse()?,
            soul: soul.parse()?,
            harmony: harmony.parse()?,
            serene: match *serene {
                "0" => false,
                "1" => true,
                _ => return Err("invalid C12 Serene flag".into()),
            },
        },
    };
    if !exact_equipped_damage(
        damage_before.parse()?,
        damage_augmented.parse()?,
        damage_critical.parse()?,
        witness.damage,
    ) {
        return Err(
            "C12 replay needs exact equipped Greater Tiger Clash damage at 800 basis points".into(),
        );
    }
    if witness.session == witness.prior
        || witness.attack_command.checked_add(2) != Some(witness.command)
        || witness.before_sequence >= witness.result_sequence
        || witness.result_sequence >= witness.delta_sequence
        || witness.builder == witness.spell
        || witness.builder_start >= witness.before_bytes
        || witness.before_bytes >= witness.clear_start
        || witness.builder_sequence >= witness.before_sequence
        || witness.delta_sequence > witness.clear_sequence
        || witness.clear_start >= witness.logout_start
        || witness.clear_sequence >= witness.logout_sequence
        || witness.vitals.harmony != 0
        || witness.damage == 0
        || witness.builder_damage == 0
        || witness.mana_before.checked_sub(50) != Some(witness.vitals.mana)
    {
        return Err("C12 fresh-session or cast correlation mismatch".into());
    }
    Ok(witness)
}

fn exact_equipped_damage(before: u64, augmented: u64, critical: bool, committed: u64) -> bool {
    before.checked_mul(8).is_some_and(|scaled| {
        let increase = scaled / 100;
        increase != 0
            && before.checked_add(increase) == Some(augmented)
            && augmented.checked_add(if critical { augmented / 10 } else { 0 }) == Some(committed)
    })
}

fn capture_root(root: Option<PathBuf>) -> Result<PathBuf, BoxError> {
    Ok(root.ok_or("C12 capture directory absent")?.canonicalize()?)
}

/// Replay idle traffic only up to the byte boundary before the next real command result.
fn command_result_start(bytes: &[u8], command: u64) -> Result<usize, BoxError> {
    use oteryn_protocol_oteryn::{
        FrameLength, MessageType, decode_command_result, decode_framed_envelope,
    };

    let mut offset = 0;
    let mut matching = None;
    while offset < bytes.len() {
        let remaining = &bytes[offset..];
        let prefix = remaining.get(..4).ok_or("C12 truncated frame prefix")?;
        let size = FrameLength::from_prefix(prefix)?.get() as usize + 4;
        let envelope = decode_framed_envelope(remaining.get(..size).ok_or("C12 truncated frame")?)?;
        if envelope.message_type() == MessageType::CommandResult
            && decode_command_result(envelope.payload())?.command_id == command
            && matching.replace(offset).is_some()
        {
            return Err("duplicate C12 attack command result".into());
        }
        offset += size;
    }
    matching.ok_or_else(|| "C12 attack command result absent".into())
}

fn recorded_commands(bytes: &[u8]) -> Result<VecDeque<Vec<u8>>, BoxError> {
    let frames = frame_capture::command_frames(bytes)?;
    if frames.iter().map(|frame| frame.len()).sum::<usize>() != bytes.len() {
        return Err("C12 stored outgoing trace contains non-command frames".into());
    }
    Ok(frames.into_iter().map(<[u8]>::to_vec).collect())
}

fn verify_recorded_command(commands: &mut VecDeque<Vec<u8>>, frame: &[u8]) -> io::Result<()> {
    if commands.front().map(Vec::as_slice) != Some(frame) {
        return Err(io::Error::other(
            "outgoing C12 command differs from live capture",
        ));
    }
    commands.pop_front();
    Ok(())
}

fn require_complete_capture(status: &serde_json::Value) -> Result<(), BoxError> {
    if status["scenario"] != "C12.client" || status["status"] != "CAPTURED" {
        return Err(format!("C12 live capture incomplete: {status}").into());
    }
    Ok(())
}

#[test]
fn c12_failed_capture_cannot_pass_replay() {
    let failed = serde_json::json!({
        "scenario": "C12.client",
        "status": "FAIL",
        "server_wire_bytes": 0,
        "error": "Platform-issued row grant unavailable",
    });
    assert!(require_complete_capture(&failed).is_err());
}

async fn replay(root: &Path) -> Result<(), BoxError> {
    let status = std::fs::read(root.join("c12-capture-status.json"))?;
    require_complete_capture(&serde_json::from_slice(&status)?)?;
    let expected_path = root.join("c12-expected.txt");
    if !(1..=4096).contains(&expected_path.metadata()?.len()) {
        return Err("C12 provenance size outside bounds".into());
    }
    let expected = std::fs::read_to_string(expected_path)?;
    let witness = read_witness(root, &expected)?;
    let path = root.join("c12-wire.bin");
    let size = path.metadata()?.len();
    if !(1..=frame_capture::FRAME_LIMIT as u64).contains(&size) {
        return Err("C12 wire capture size outside bounds".into());
    }
    let bytes = std::fs::read(path)?;
    let frames = root.join("c12-frames.json");
    if !(1..=16 * frame_capture::FRAME_LIMIT as u64).contains(&frames.metadata()?.len()) {
        return Err("C12 #3521 envelope capture size outside bounds".into());
    }
    let captured: Vec<frame_capture::Envelope> = serde_json::from_slice(&std::fs::read(frames)?)?;
    let session = oteryn_protocol_oteryn::GameSessionId::decode(&witness.session)?;
    if captured != frame_capture::envelopes(&bytes, session)? {
        return Err("C12 #3521 envelopes do not match the original server frames".into());
    }
    let command_path = root.join("c12-client-wire.bin");
    if !(1..=frame_capture::FRAME_LIMIT as u64).contains(&command_path.metadata()?.len()) {
        return Err("C12 outgoing capture size outside bounds".into());
    }
    let recorded = Rc::new(RefCell::new(recorded_commands(&std::fs::read(
        command_path,
    )?)?));
    let length = bytes.len();
    let before_attack = command_result_start(&bytes, witness.attack_command)?;
    if before_attack >= witness.builder_start || witness.logout_start >= length {
        return Err("C12 command boundaries outside captured transcript".into());
    }
    let visible = Rc::new(Cell::new(before_attack));
    let consumed = Rc::new(Cell::new(0));
    let expected_cast = Rc::new(Cell::new(None));
    let outbound = Rc::new(RefCell::new(Vec::new()));
    let wire = ReplayWire {
        bytes,
        visible: Rc::clone(&visible),
        consumed: Rc::clone(&consumed),
        expected_cast: Rc::clone(&expected_cast),
        outbound: Rc::clone(&outbound),
        recorded: Rc::clone(&recorded),
    };
    let dictionary = root.join("c12-item-dictionary.json");
    if !(1..=4096).contains(&dictionary.metadata()?.len()) {
        return Err("C12 Item dictionary pin size outside bounds".into());
    }
    let pin: frame_capture::ItemDictionaryPin =
        serde_json::from_slice(&std::fs::read(dictionary)?)?;
    let refs = pin.sorted_refs()?;
    let admission = crate::Admission {
        appearance_items: Some(crate::AppearanceItemDictionary {
            content_generation: pin.generation,
            definition_refs: &refs,
        }),
        supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
        deadline: Duration::from_secs(2),
        ..admission()?
    };
    let mut session = Session::admit(wire, admission).await?;
    let steps_and_premium = u64::try_from(witness.traveled.len())? + 1;
    if session.join_snapshot().game_session_id.as_bytes() != &witness.session
        || session.next_command_id().checked_add(steps_and_premium) != Some(witness.attack_command)
    {
        return Err("original C12 admission does not match capture provenance".into());
    }
    let inventory = session.inventory().ok_or("captured C12 inventory absent")?;
    assert!(frame_capture::equipped(inventory, witness.definition));
    // The live transcript begins with the fresh account's Premium observation.
    use oteryn_protocol_oteryn::{character_info, premium_overview};
    let premium = session
        .gated_command(
            character_info::CAPABILITY_CHARACTER_INFO_V1,
            character_info::COMMAND_TYPE_CHARACTER_INFO,
            &[premium_overview::OPERATION],
        )
        .await?;
    assert_eq!(premium.status, CommandStatus::Accepted);
    let overview = premium_overview::Overview::decode(&premium.payload)
        .ok_or("captured C12 Premium response malformed")?;
    assert_eq!(overview.state, premium_overview::State::Active);
    assert!(overview.valid_for_us > 0);
    let catalog = session
        .spell_catalog()?
        .ok_or("captured C12 spell catalogue absent")?;
    assert!(
        catalog
            .entries
            .iter()
            .any(|entry| entry.words == "exori nia"
                && entry.available
                && entry.rune_item.is_none()
                && entry.cast_index == witness.spell.get())
    );
    assert!(
        catalog
            .entries
            .iter()
            .any(|entry| entry.words == "exori infir pug"
                && entry.available
                && entry.rune_item.is_none()
                && entry.cast_index == witness.builder.get())
    );
    assert_ne!(session.world_spatial().actor_position, witness.cast_tile);
    for &direction in &witness.traveled {
        let before = session.world_spatial().actor_position;
        let command = session.next_command_id();
        let step = session.step(direction).await?;
        assert_eq!(step.command_id, command);
        assert_eq!(step.status, CommandStatus::Accepted);
        assert_eq!(step.disposition, StepDisposition::Moved);
        let moved = step
            .world_spatial_delta
            .ok_or("captured C12 movement delta absent")?;
        assert_ne!(moved.value.actor_position, before);
        assert_eq!(
            session.world_spatial().actor_position,
            moved.value.actor_position
        );
    }
    assert_eq!(session.world_spatial().actor_position, witness.cast_tile);
    assert_eq!(session.next_command_id(), witness.attack_command);
    assert_eq!(
        session
            .combat_state()
            .ok_or("captured C12 combat state absent")?
            .target,
        None
    );
    let own = loop {
        let entities = session
            .world_entities()
            .ok_or("captured C12 entity projection absent")?;
        if entities.get(&witness.target).is_some_and(|entry| {
            let at = witness.cast_tile;
            entry.position.floor == at.floor
                && entry.position.x == at.x
                && entry.position.y.checked_add(1) == Some(at.y)
        }) {
            break entities
                .iter()
                .find(|entry| entry.entity.identity == *entities.own_identity())
                .ok_or("captured C12 caster entity absent")?
                .entity;
        }
        session.wait_for_events(Duration::from_secs(2)).await?;
    };
    visible.set(witness.builder_start);
    let attack = session.attack_target(Some(&witness.target)).await?;
    assert_eq!(attack.command_id, witness.attack_command);
    assert_eq!(attack.status, CommandStatus::Accepted);
    assert_eq!(attack.disposition, AttackIntentDisposition::Ok);
    drain_boundary(&mut session, &consumed, witness.builder_start).await?;
    assert_eq!(session.last_server_sequence(), witness.builder_sequence);
    check_cast_target(&session, &witness)?;
    assert_eq!(
        session
            .actor_vitals()
            .ok_or("C12 builder pre-state missing")?
            .harmony,
        0
    );
    session.take_events();

    let generation = session.connection_generation;
    expected_cast.set(Some(ExpectedCast {
        command: witness.attack_command + 1,
        spell: witness.builder,
        generation,
        visible_after: witness.before_bytes,
    }));
    let builder_before = *session
        .actor_vitals()
        .ok_or("C12 builder pre-state missing")?;
    let builder = session
        .cast_spell(witness.builder, SpellTarget::AttackTarget, true)
        .await?;
    assert!(expected_cast.get().is_none());
    assert_eq!(builder.status, CommandStatus::Accepted);
    assert_eq!(builder.disposition, SpellCastDisposition::Cast);
    assert_eq!(
        builder
            .actor_vitals_delta
            .as_ref()
            .ok_or("C12 builder delta missing")?
            .value
            .harmony,
        1
    );
    let builder_events = drain_boundary(&mut session, &consumed, witness.before_bytes).await?;
    let builder_delta = builder
        .actor_vitals_delta
        .ok_or("C12 builder vitals missing")?;
    let builder_mana = frame_capture::mana_before_cast(
        builder_events.iter().filter_map(|event| match event {
            SessionEvent::ActorVitals(update) => Some((update.server_sequence, update.value.mana)),
            _ => None,
        }),
        builder_before.mana,
        builder_before.max_mana,
        builder_delta.server_sequence,
    )
    .ok_or("C12 unexpected mana change before replay Swift Jab")?;
    assert_eq!(builder_mana.checked_sub(3), Some(builder_delta.value.mana));
    let mut builder_hits = Vec::new();
    for event in builder_events {
        if let SessionEvent::Presentation(delta) = event {
            frame_capture::attributed_hits(
                &delta.value,
                own,
                witness.target,
                witness.builder.get(),
                &mut builder_hits,
            );
        }
    }
    assert!(
        frame_capture::exact_hit(&builder_hits, witness.builder_damage),
        "C12 Swift Jab expected {}, observed {builder_hits:?}",
        witness.builder_damage
    );
    assert_eq!(session.last_server_sequence(), witness.before_sequence);
    check_cast_target(&session, &witness)?;
    let before = *session
        .actor_vitals()
        .ok_or("captured C12 pre-cast vitals absent")?;
    assert_eq!(before.harmony, 1);
    session.take_events();

    expected_cast.set(Some(ExpectedCast {
        command: witness.command,
        spell: witness.spell,
        generation,
        visible_after: witness.clear_start,
    }));
    let cast = session
        .cast_spell(witness.spell, SpellTarget::AttackTarget, true)
        .await?;
    assert!(expected_cast.get().is_none());
    assert_eq!(cast.command_id, witness.command);
    assert_eq!(cast.status, CommandStatus::Accepted);
    assert_eq!(cast.disposition, SpellCastDisposition::Cast);
    assert_eq!(cast.result_server_sequence, witness.result_sequence);
    let delta = cast
        .actor_vitals_delta
        .ok_or("captured C12 cast delta absent")?;
    assert_eq!(delta.server_sequence, witness.delta_sequence);
    assert_eq!(delta.value, witness.vitals);
    assert_eq!(session.actor_vitals(), Some(&witness.vitals));
    assert!(frame_capture::spent_harmony(
        before.harmony,
        delta.value.harmony
    ));
    let mut events = session.take_events();
    let mana_before = frame_capture::mana_before_cast(
        events.iter().filter_map(|event| match event {
            SessionEvent::ActorVitals(update) => Some((update.server_sequence, update.value.mana)),
            _ => None,
        }),
        before.mana,
        before.max_mana,
        delta.server_sequence,
    )
    .ok_or("unexpected mana change before the replayed cast delta")?;
    assert_eq!(mana_before, witness.mana_before);
    assert_eq!(mana_before.checked_sub(50), Some(delta.value.mana));

    events.extend(drain_boundary(&mut session, &consumed, witness.clear_start).await?);
    assert_eq!(session.last_server_sequence(), witness.clear_sequence);
    visible.set(witness.logout_start);
    let clear = session.attack_target(None).await?;
    assert_eq!(clear.command_id, witness.command + 1);
    assert_eq!(clear.status, CommandStatus::Accepted);
    assert_eq!(clear.disposition, AttackIntentDisposition::Ok);
    events.extend(drain_boundary(&mut session, &consumed, witness.logout_start).await?);
    assert_eq!(session.last_server_sequence(), witness.logout_sequence);
    assert!(
        session
            .combat_state()
            .is_some_and(|state| state.target.is_none() && !state.in_fight)
    );
    let unchanged = *session.actor_vitals().ok_or("C12 actor vitals absent")?;
    assert!(actor_spell::decode_actor_vitals(&[0x38, 0x02]).is_err());
    assert_eq!(session.actor_vitals(), Some(&unchanged));
    visible.set(length);
    let logout = session.logout().await?;
    assert_eq!(logout.command_id, witness.command + 2);
    assert_eq!(logout.status, CommandStatus::Accepted);
    assert_eq!(
        logout.result,
        Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    );
    assert_eq!(consumed.get(), length);
    events.extend(session.take_events());
    let mut hits = Vec::new();
    for event in events {
        if let SessionEvent::Presentation(delta) = event {
            frame_capture::attributed_hits(
                &delta.value,
                own,
                witness.target,
                witness.spell.get(),
                &mut hits,
            );
        }
    }
    assert!(
        frame_capture::exact_hit(&hits, witness.damage),
        "C12 attributed hits through logout: {hits:?}"
    );
    assert!(
        session
            .combat_state()
            .is_some_and(|state| state.target.is_none() && !state.in_fight)
    );
    assert!(
        outbound.borrow().is_empty(),
        "C12 trailing partial outgoing frame"
    );
    assert!(recorded.borrow().is_empty(), "C12 unconsumed live commands");
    Ok(())
}

async fn drain_boundary(
    session: &mut Session<ReplayWire>,
    consumed: &Cell<usize>,
    boundary: usize,
) -> Result<Vec<SessionEvent>, BoxError> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut events = session.take_events();
    while consumed.get() < boundary {
        let Some(remaining) = deadline.checked_duration_since(tokio::time::Instant::now()) else {
            break;
        };
        session
            .service_liveness(remaining.min(Duration::from_millis(50)))
            .await?;
        events.extend(session.take_events());
    }
    if consumed.get() != boundary {
        return Err(format!(
            "C12 did not replay the complete boundary: {} != {boundary}",
            consumed.get(),
        )
        .into());
    }
    Ok(events)
}

fn check_cast_target(session: &Session<ReplayWire>, witness: &Witness) -> Result<(), BoxError> {
    let target = session
        .world_entities()
        .and_then(|entities| entities.get(&witness.target))
        .ok_or("C12 replay target missing at CAST")?;
    assert!(frame_capture::exact_target(
        session.world_spatial().actor_position,
        target.position,
        witness.cast_tile
    ));
    assert_eq!(
        session.combat_state().and_then(|state| state.target),
        Some(witness.target)
    );
    Ok(())
}

#[test]
fn e2e_c12_replays_augmented_cast_payload_and_refuses_invalid_payload() -> Result<(), BoxError> {
    let directory = std::env::var_os("E2E_ALL_CAPTURE_DIR").map(PathBuf::from);
    if directory.is_none() {
        assert!(capture_root(directory).is_err());
        assert!(read_witness(Path::new("/current-run"), "/previous-run\n").is_err());
        assert!(actor_spell::decode_actor_vitals(&[0x38, 0x02]).is_err());
        return Ok(());
    }
    let root = capture_root(directory)?;
    block_on(replay(&root))?
}

#[test]
fn c12_capture_provenance_refuses_stale_or_mismatched_fields() -> Result<(), BoxError> {
    let run = Path::new("/current-run");
    let session = "01a218116e5d7a078802f8f24863df69";
    let prior = "01a218115a6578e2a6913d169380b741";
    let target = "00000000000000000000000000000009";
    let current = format!(
        "/current-run\n{session} {prior} 8 4 5 6 42 {target} 1 27000 120 305 305 70 210 100 0 0 48 45 48 false 3 26013 41 22 20000 2 28000 6 30000 9 1234 32295 32334 8\n"
    );
    let witness = read_witness(run, &current)?;
    assert_eq!(witness.command, 8);
    assert_eq!(witness.delta_sequence, 5);
    assert_eq!(witness.traveled.len(), 4);
    for invalid in [
        current.replace("/current-run", "/previous-run"),
        current.replace(prior, session),
        current.replace(" 8 4 5 6 ", " 8 5 4 6 "),
        current.replace(" 305 305 70 210 ", " 305 305 120 210 "),
        current.replace(" 48 45 48 false ", " 45 45 45 false "),
        current.replace(" 48 45 48 false ", " 45 45 48 false "),
        current.replace(" 48 45 48 false ", " 48 45 48 true "),
        current.replace(" 1234 32295 ", " 1934 32295 "),
    ] {
        assert!(read_witness(run, &invalid).is_err());
    }
    Ok(())
}

#[test]
fn c12_capture_requires_exact_envelope_keys() -> Result<(), BoxError> {
    let json = r#"{"command_id":7,"game_session_id":"01a21811-6e5d-7a07-8802-f8f24863df69","payload":[0,255],"server_sequence":9,"status":1}"#;
    let captured: frame_capture::Envelope = serde_json::from_str(json)?;
    assert_eq!(
        serde_json::to_value(&captured)?,
        serde_json::from_str::<serde_json::Value>(json)?
    );
    for invalid in [
        json.replace(r#","status":1"#, ""),
        json.replace(r#""status":1"#, r#""status":1,"status":1"#),
        json.replace(r#""status":1"#, r#""status":1,"extra":0"#),
    ] {
        assert!(serde_json::from_str::<frame_capture::Envelope>(&invalid).is_err());
    }
    Ok(())
}

struct ReplayWire {
    bytes: Vec<u8>,
    visible: Rc<Cell<usize>>,
    consumed: Rc<Cell<usize>>,
    expected_cast: Rc<Cell<Option<ExpectedCast>>>,
    outbound: Rc<RefCell<Vec<u8>>>,
    recorded: Rc<RefCell<VecDeque<Vec<u8>>>>,
}

impl AsyncRead for ReplayWire {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let offset = self.consumed.get();
        let end = (offset + buffer.remaining()).min(self.visible.get());
        if end == offset && buffer.remaining() != 0 {
            return Poll::Pending;
        }
        buffer.put_slice(&self.bytes[offset..end]);
        self.consumed.set(end);
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for ReplayWire {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut pending = self.outbound.borrow_mut();
        pending.extend_from_slice(buffer);
        while let Some(prefix) = pending.get(..4) {
            let size = FrameLength::from_prefix(prefix)
                .map_err(io::Error::other)?
                .get() as usize
                + 4;
            let Some(frame) = pending.get(..size) else {
                break;
            };
            let envelope = decode_framed_envelope(frame).map_err(io::Error::other)?;
            if envelope.message_type() == MessageType::ClientCommand {
                verify_recorded_command(&mut self.recorded.borrow_mut(), frame)?;
                if let Some(expected) = self.expected_cast.get() {
                    verify_cast_intent(frame, expected)?;
                    self.expected_cast.set(None);
                    self.visible.set(expected.visible_after);
                } else if envelope
                    .client_command(envelope.connection_generation())
                    .map_err(io::Error::other)?
                    .command_type
                    == actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT
                {
                    return Poll::Ready(Err(io::Error::other("unexpected outgoing C12 CAST")));
                }
            }
            pending.drain(..size);
        }
        Poll::Ready(Ok(buffer.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Clone, Copy)]
struct ExpectedCast {
    command: u64,
    spell: NonZeroU32,
    generation: u64,
    visible_after: usize,
}

fn verify_cast_intent(frame: &[u8], expected: ExpectedCast) -> io::Result<()> {
    let envelope = decode_framed_envelope(frame).map_err(io::Error::other)?;
    let command = envelope
        .client_command(expected.generation)
        .map_err(io::Error::other)?;
    let intent = actor_spell::SpellCastIntent {
        spell: expected.spell,
        target: SpellTarget::AttackTarget,
        aim_at_target: true,
    };
    if command.command_type != actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT
        || command.command_id != expected.command
        || actor_spell::decode_spell_cast_intent(command.payload).ok() != Some(intent)
        || command.payload != actor_spell::encode_spell_cast_intent(&intent)
    {
        return Err(io::Error::other("outgoing C12 CAST intent mismatch"));
    }
    Ok(())
}

#[test]
fn c12_replay_only_releases_response_for_exact_outgoing_cast() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::{ClientCommandValue, encode_client_command};
    use tokio::io::AsyncWriteExt;

    let expected = ExpectedCast {
        command: 11,
        spell: NonZeroU32::MIN,
        generation: 7,
        visible_after: 4,
    };
    let correct = actor_spell::SpellCastIntent {
        spell: expected.spell,
        target: SpellTarget::AttackTarget,
        aim_at_target: true,
    };
    let cast_type = actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT;
    block_on(async {
        for (generation, command, command_type, intent, accepted) in [
            (7, 11, cast_type, correct, true),
            (
                7,
                11,
                cast_type,
                actor_spell::SpellCastIntent {
                    spell: NonZeroU32::new(2).ok_or("spell")?,
                    ..correct
                },
                false,
            ),
            (
                7,
                11,
                cast_type,
                actor_spell::SpellCastIntent {
                    target: SpellTarget::None,
                    ..correct
                },
                false,
            ),
            (
                7,
                11,
                cast_type,
                actor_spell::SpellCastIntent {
                    aim_at_target: false,
                    ..correct
                },
                false,
            ),
            (8, 11, cast_type, correct, false),
            (7, 12, cast_type, correct, false),
            (7, 11, 9, correct, false),
        ] {
            let visible = Rc::new(Cell::new(0));
            let gate = Rc::new(Cell::new(Some(expected)));
            let mut wire = ReplayWire {
                bytes: vec![0; 4],
                visible: Rc::clone(&visible),
                consumed: Rc::new(Cell::new(0)),
                expected_cast: Rc::clone(&gate),
                outbound: Rc::new(RefCell::new(Vec::new())),
                recorded: Rc::new(RefCell::new(VecDeque::new())),
            };
            let payload = actor_spell::encode_spell_cast_intent(&intent);
            let body = encode_client_command(
                generation,
                &ClientCommandValue {
                    command_id: command,
                    command_type,
                    payload: &payload,
                },
            )?;
            let prefix = u32::try_from(body.len())?.to_be_bytes();
            let mut original = prefix.to_vec();
            original.extend_from_slice(&body);
            wire.recorded.borrow_mut().push_back(original);
            wire.write_all(&prefix[..2]).await?;
            wire.write_all(&prefix[2..]).await?;
            assert_eq!(visible.get(), 0);
            assert_eq!(wire.write_all(&body).await.is_ok(), accepted);
            assert_eq!(visible.get(), if accepted { 4 } else { 0 });
            assert_eq!(gate.get().is_none(), accepted);
        }
        Ok::<(), BoxError>(())
    })?
}

#[test]
fn c12_rejects_changed_noncast_commands() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::{ClientCommandValue, encode_client_command};
    use oteryn_protocol_oteryn::{attack, logout, world_spatial};
    let target = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    for (command_type, payload) in [
        (
            world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            world_spatial::encode_step_intent(StepDirection::North),
        ),
        (
            attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
            attack::encode_attack_target_intent(Some(&target)).map_err(|e| format!("{e:?}"))?,
        ),
        (
            attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
            attack::encode_attack_target_intent(None).map_err(|e| format!("{e:?}"))?,
        ),
        (
            logout::COMMAND_TYPE_LOGOUT_INTENT,
            logout::encode_logout_intent(),
        ),
    ] {
        let encode = |command_id, payload: &[u8]| -> Result<Vec<u8>, BoxError> {
            let body = encode_client_command(
                7,
                &ClientCommandValue {
                    command_id,
                    command_type,
                    payload,
                },
            )?;
            let mut frame = u32::try_from(body.len())?.to_be_bytes().to_vec();
            frame.extend(body);
            Ok(frame)
        };
        let matching = encode(11, &payload)?;
        let mut commands = recorded_commands(&matching)?;
        let mut changed = payload.clone();
        changed.push(0);
        assert!(verify_recorded_command(&mut commands, &encode(11, &changed)?).is_err());
        assert!(verify_recorded_command(&mut commands, &encode(12, &payload)?).is_err());
        verify_recorded_command(&mut commands, &matching)?;
        assert!(commands.is_empty());
    }
    Ok(())
}

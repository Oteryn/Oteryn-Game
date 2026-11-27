//! Replay the original server bytes for the C12 equipped admission and spell exchange.

use super::*;
use oteryn_protocol_oteryn::actor_spell::{self, ActorVitals, SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::item_view::EquipmentSlot;
use oteryn_protocol_oteryn::presentation::{PresentationEventKind, PresentationValueKind};
use oteryn_protocol_oteryn::world_spatial::{
    ActorPosition, StepDirection, StepDisposition, decode_step_intent_profile,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::cell::Cell;
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
        before_sequence,
        before_bytes,
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
    if witness.session == witness.prior
        || witness.attack_command.checked_add(1) != Some(witness.command)
        || witness.before_sequence >= witness.result_sequence
        || witness.result_sequence >= witness.delta_sequence
        || witness.damage == 0
        || witness.mana_before.checked_sub(50) != Some(witness.vitals.mana)
    {
        return Err("C12 fresh-session or cast correlation mismatch".into());
    }
    Ok(witness)
}

fn capture_root(root: Option<PathBuf>) -> Result<PathBuf, BoxError> {
    Ok(root.ok_or("C12 capture directory absent")?.canonicalize()?)
}

async fn replay(root: &Path) -> Result<(), BoxError> {
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
    if !(1..bytes.len()).contains(&witness.before_bytes) {
        return Err("C12 cast boundary outside the original transcript".into());
    }
    let length = bytes.len();
    let visible = Rc::new(Cell::new(witness.before_bytes));
    let wire = ReplayWire {
        bytes,
        offset: 0,
        visible: Rc::clone(&visible),
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
    let total: u64 = inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .filter(|entry| entry.item_definition_ref == witness.definition)
        .map(|entry| u64::from(entry.count.get()))
        .sum();
    assert_eq!(total, 1);
    assert!(inventory.equipment.iter().any(|entry| {
        entry.slot == EquipmentSlot::RightHand
            && entry.item.item_definition_ref == witness.definition
            && entry.item.count.get() == 1
    }));
    assert!(
        !inventory
            .entries
            .iter()
            .any(|entry| entry.item_definition_ref == witness.definition)
    );
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
    assert_ne!(session.world_spatial().actor_position, witness.cast_tile);
    for direction in witness.traveled {
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
                && entry
                    .position
                    .x
                    .abs_diff(at.x)
                    .max(entry.position.y.abs_diff(at.y))
                    == 1
        }) {
            break entities
                .iter()
                .find(|entry| entry.entity.identity == *entities.own_identity())
                .ok_or("captured C12 caster entity absent")?
                .entity;
        }
        session.wait_for_events(Duration::from_secs(2)).await?;
    };
    let attack = session.attack_target(Some(&witness.target)).await?;
    assert_eq!(attack.command_id, witness.attack_command);
    assert_eq!(attack.status, CommandStatus::Accepted);
    assert_eq!(attack.disposition, AttackIntentDisposition::Ok);
    while session.combat_state().and_then(|state| state.target) != Some(witness.target)
        || session.last_server_sequence() < witness.before_sequence
    {
        session.wait_for_events(Duration::from_secs(2)).await?;
    }
    assert_eq!(
        session
            .actor_vitals()
            .ok_or("captured C12 pre-cast vitals absent")?
            .mana,
        witness.mana_before
    );
    assert_eq!(session.last_server_sequence(), witness.before_sequence);
    session.take_events();
    visible.set(length);
    let cast = session
        .cast_spell(witness.spell, SpellTarget::AttackTarget, true)
        .await?;
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
    let mut events = session.take_events();
    loop {
        let damage: u64 = events
            .iter()
            .map(|event| damage_for_cast(event, own, witness.target))
            .sum();
        if damage != 0 {
            assert_eq!(damage, witness.damage);
            break;
        }
        events = session.wait_for_events(Duration::from_secs(2)).await?;
    }
    let unchanged = *session.actor_vitals().ok_or("C12 actor vitals absent")?;
    assert!(actor_spell::decode_actor_vitals(&[0x38, 0x02]).is_err());
    assert_eq!(session.actor_vitals(), Some(&unchanged));
    Ok(())
}

fn damage_for_cast(event: &SessionEvent, caster: EntityRef, target: EntityRef) -> u64 {
    let SessionEvent::Presentation(delta) = event else {
        return 0;
    };
    delta.value.iter().filter_map(|entry| match entry.kind {
        PresentationEventKind::ValueText { kind: PresentationValueKind::DamageDealt, target: victim, primary, .. }
            if victim == target && delta.value.iter().any(|part| {
                part.decision_ordinal == entry.decision_ordinal
                    && matches!(part.kind, PresentationEventKind::SpellWords { caster: source, .. } if source == caster)
            }) => Some(primary.value),
        _ => None,
    }).sum()
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
        "/current-run\n{session} {prior} 7 4 5 6 42 {target} 1 27000 120 305 305 70 210 100 0 0 48 3 26013 1234 32295 32334 8\n"
    );
    let witness = read_witness(run, &current)?;
    assert_eq!(witness.command, 7);
    assert_eq!(witness.delta_sequence, 5);
    assert_eq!(witness.traveled.len(), 4);
    for invalid in [
        current.replace("/current-run", "/previous-run"),
        current.replace(prior, session),
        current.replace(" 7 4 5 6 ", " 7 5 4 6 "),
        current.replace(" 305 305 70 210 ", " 305 305 120 210 "),
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
    offset: usize,
    visible: Rc<Cell<usize>>,
}

impl AsyncRead for ReplayWire {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let end = (self.offset + buffer.remaining()).min(self.visible.get());
        if end == self.offset && buffer.remaining() != 0 {
            return Poll::Pending;
        }
        buffer.put_slice(&self.bytes[self.offset..end]);
        self.offset = end;
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for ReplayWire {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        // Only server-to-client frames are stored. The client issues its own real commands.
        Poll::Ready(Ok(buffer.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

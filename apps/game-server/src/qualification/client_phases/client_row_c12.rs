//! Capture the fresh equipped admission and its actual spell exchange for the client replay.

use super::*;
use oteryn_dev_client::native_session::{Admission, Session};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::item_view::EquipmentSlot;
use std::io::{self, Write};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[path = "../../../../../tools/qualification/c12_frame_capture.rs"]
mod frame_capture;

const COMMAND_BUDGET: Duration = Duration::from_secs(8);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(360), verdict::observe(capture(clients)))
            .await
            .unwrap_or_else(|_| Err("C12.client capture deadline=360s".into()));
    match result {
        Ok(()) => evidence("e2e_capture scenario=C12.client status=CAPTURED"),
        Err(error) => evidence(&format!(
            "e2e_all scenario=C12.client status=FAIL evidence={error}"
        )),
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    use full_row_c12::hex_bytes;

    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("C12 capture directory absent")?;
    let root = Path::new(&root).canonicalize()?;
    let full_row_c12::ClientCaptureSetup {
        accounts,
        characters,
        definition,
        target,
        previous,
        position: prior_position,
    } = full_row_c12::prepare_client_capture(clients).await?;
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    wait_character_released(clients, 1).await?;
    let grant = platform_row_grant(clients, 1).await?;
    let stream = connector(clients.certificate, &EXACT)?
        .connect(
            ServerName::try_from("localhost")?,
            TcpStream::connect(clients.address).await?,
        )
        .await?;
    let received = Arc::new(Mutex::new(Vec::new()));
    let mut session = Session::admit(
        WireCapture {
            stream,
            received: Arc::clone(&received),
        },
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: CharacterId::decode(&characters[1])?,
            admission_material: grant.as_bytes(),
            client_build_id: "oteryn-dev-client/world-qualification",
            supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(20),
        },
    )
    .await?;
    let id = session.join_snapshot().game_session_id;
    let inventory = session.inventory().ok_or("C12 fresh inventory absent")?;
    let total: u64 = inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .filter(|entry| entry.item_definition_ref == definition)
        .map(|entry| u64::from(entry.count.get()))
        .sum();
    if total != 1
        || id == previous
        || !inventory.equipment.iter().any(|entry| {
            entry.slot == EquipmentSlot::RightHand
                && entry.item.item_definition_ref == definition
                && entry.item.count.get() == 1
        })
        || inventory
            .entries
            .iter()
            .any(|entry| entry.item_definition_ref == definition)
    {
        return Err(
            "C12 fresh admission did not retain exactly one weapon in the right hand".into(),
        );
    }
    full_row_c12::require_current_premium(&mut session).await?;
    let catalog = session
        .spell_catalog()?
        .ok_or("C12 fresh spell catalogue absent")?;
    let spell = catalog
        .entries
        .iter()
        .find(|entry| entry.words == "exori nia" && entry.rune_item.is_none())
        .ok_or("C12 Greater Tiger Clash absent from fresh catalogue")?;
    if !spell.available {
        return Err(format!("C12 Greater Tiger Clash unavailable after equipped relog: learned={} required_level={}", spell.learned, spell.level).into());
    }
    let spell = NonZeroU32::new(spell.cast_index).ok_or("C12 spell index is zero")?;
    let position = session.world_spatial().actor_position;
    if tile(position)? != prior_position {
        return Err("C12.client old position was not retained across relog".into());
    }
    let entities = session
        .world_entities()
        .ok_or("C12 fresh entity projection absent")?;
    let victim = entities
        .get(&target)
        .ok_or("C12 original target absent after fresh admission")?;
    let own = entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .ok_or("C12 fresh caster entity absent")?
        .entity;
    if victim.position.floor != position.floor
        || victim
            .position
            .x
            .abs_diff(position.x)
            .max(victim.position.y.abs_diff(position.y))
            != 1
    {
        return Err("C12 original target is not adjacent after fresh admission".into());
    }
    if session
        .combat_state()
        .ok_or("C12 fresh combat state absent")?
        .target
        .is_some()
    {
        return Err("C12 fresh combat target was already set".into());
    }
    let attack = session.attack_target(Some(&target)).await?;
    require_mechanic_acceptance(attack.status, "C12.client ATTACK")?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("C12.client attack reply={attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + COMMAND_BUDGET;
    while session.combat_state().and_then(|state| state.target) != Some(target) {
        if tokio::time::Instant::now() >= deadline {
            return Err("C12.client attack target change not served".into());
        }
        session.service_liveness(Duration::from_millis(25)).await?;
        session.take_events();
    }
    let before = session
        .actor_vitals()
        .ok_or("C12 actor vitals absent before cast")?
        .mana;
    let path = PathBuf::from(required("NODE_BOOT_CAST_EVIDENCE_LOG")?);
    let offset = std::fs::metadata(&path)?.len();
    session.take_events();
    let before_sequence = session.last_server_sequence();
    let before_bytes = received
        .lock()
        .map_err(|_| "C12 capture lock unavailable")?
        .len();
    let cast = session
        .cast_spell(spell, SpellTarget::AttackTarget, true)
        .await?;
    require_mechanic_acceptance(cast.status, "C12.client CAST")?;
    if cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("C12.client cast reply={cast:?}").into());
    }
    let delta = cast
        .actor_vitals_delta
        .ok_or("C12 cast actor-vitals delta absent")?;
    let vitals = delta.value;
    if before.checked_sub(50) != Some(vitals.mana) || session.actor_vitals() != Some(&vitals) {
        return Err(format!(
            "C12.client exact mana debit not applied: {before} -> {}",
            vitals.mana
        )
        .into());
    }
    let session_id = hex_bytes(id.as_bytes());
    let sequence = delta.server_sequence;
    let command = cast.command_id;
    let result_sequence = cast.result_server_sequence;
    let deadline = tokio::time::Instant::now() + COMMAND_BUDGET;
    let damage = loop {
        let dealt: u64 = session
            .take_events()
            .iter()
            .map(|event| full_row_c12::cast_damage_in(event, own, target))
            .sum();
        if dealt != 0 {
            break dealt;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C12.client cast did not serve attributed damage".into());
        }
        session.service_liveness(Duration::from_millis(25)).await?;
    };
    let target_identity = hex_bytes(&target.identity);
    let marker = format!(
        "SEAM_EVIDENCE c12_base_damage session={session_id} command={command} target={target_identity} generation={} ",
        target.generation
    );
    let (_, augmented) = full_row_c12::committed_base_damage(
        &path,
        offset,
        &marker,
        full_row_c12::content_augment()?,
    )?;
    if damage != augmented {
        return Err(format!(
            "C12.client served damage {damage} differs from committed augmented damage {augmented}"
        )
        .into());
    }
    let earlier = hex_bytes(previous.as_bytes());
    let expected = format!(
        "{}\n{session_id} {earlier} {command} {result_sequence} {sequence} {attack_id} {spell} {target_identity} {target_generation} {definition} {before} {health} {max_health} {mana} {max_mana} {soul} {harmony} {serene} {damage} {before_sequence} {before_bytes}\n",
        root.display(),
        attack_id = attack.command_id,
        target_generation = target.generation,
        health = vitals.health,
        max_health = vitals.max_health,
        mana = vitals.mana,
        max_mana = vitals.max_mana,
        soul = vitals.soul,
        harmony = vitals.harmony,
        serene = u8::from(vitals.serene),
    );
    let bytes = received
        .lock()
        .map_err(|_| "C12 capture lock unavailable")?;
    let frames = frame_capture::envelopes(&bytes, id)
        .map_err(|error| error as Box<dyn std::error::Error>)?;
    let frames = serde_json::to_vec(&frames)?;
    write_packet(&root, "c12-wire.bin", &bytes)?;
    write_packet(&root, "c12-frames.json", &frames)?;
    write_packet(&root, "c12-expected.txt", expected.as_bytes())?;
    evidence(&format!(
        "C12_CAPTURE session={session_id} command={command} result_sequence={result_sequence} delta_sequence={sequence}"
    ));
    Ok(())
}

fn write_packet(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::File::create_new(root.join(name))?.write_all(bytes)?;
    Ok(())
}

#[test]
fn c12_missing_capture_directory_is_an_error() {
    let directory = Path::new(file!()).join("cannot-create-child-under-source");
    assert!(write_packet(&directory, "capture.bin", b"frame").is_err());
}

struct WireCapture<S> {
    stream: S,
    received: Arc<Mutex<Vec<u8>>>,
}

impl<S: AsyncRead + Unpin> AsyncRead for WireCapture<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let old_len = buffer.filled().len();
        let read = Pin::new(&mut self.stream).poll_read(cx, buffer);
        if matches!(read, Poll::Ready(Ok(()))) {
            let bytes = &buffer.filled()[old_len..];
            let mut received = self
                .received
                .lock()
                .map_err(|_| io::Error::other("C12 capture lock unavailable"))?;
            if bytes.len() > frame_capture::FRAME_LIMIT.saturating_sub(received.len()) {
                return Poll::Ready(Err(io::Error::other("C12 capture exceeds bounds")));
            }
            received.extend_from_slice(bytes);
        }
        read
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for WireCapture<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, bytes)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

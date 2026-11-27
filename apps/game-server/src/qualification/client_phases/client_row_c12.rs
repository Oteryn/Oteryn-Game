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
pub(super) mod frame_capture;

const COMMAND_BUDGET: Duration = Duration::from_secs(8);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let received = Arc::new(Mutex::new(Vec::new()));
    let result = verdict::indexed_timeout(
        Duration::from_secs(360),
        verdict::observe(capture(clients, Arc::clone(&received))),
    )
    .await
    .unwrap_or_else(|_| Err("C12.client capture deadline=360s".into()));
    match result {
        Ok(()) => evidence("e2e_capture scenario=C12.client status=CAPTURED"),
        Err(error) => {
            if let (Some(root), Ok(bytes)) =
                (std::env::var_os("E2E_ALL_CAPTURE_DIR"), received.lock())
            {
                if !bytes.is_empty() {
                    let name = "c12-failed-wire.bin";
                    match write_packet(Path::new(&root), name, &bytes) {
                        Ok(()) => evidence(&format!(
                            "C12_FAILED_WIRE file={name} bytes={}",
                            bytes.len()
                        )),
                        Err(save_error) => {
                            evidence(&format!("C12_FAILED_WIRE save_error={save_error}"))
                        }
                    }
                }
            }
            evidence(&format!(
                "e2e_all scenario=C12.client status=FAIL evidence={error}"
            ));
        }
    }
}

async fn capture(clients: &SeamClients<'_>, received: Arc<Mutex<Vec<u8>>>) -> TestResult {
    use full_row_c12::hex_bytes;

    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("C12 capture directory absent")?;
    let root = Path::new(&root).canonicalize()?;
    let full_row_c12::ClientCaptureSetup {
        accounts,
        characters,
        definition,
        route,
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
    let item_pin = full_row_c12::item_dictionary_pin(clients)?;
    let item_refs = item_pin.sorted_refs()?;
    let mut session = Session::admit(
        WireCapture {
            stream,
            received: Arc::clone(&received),
        },
        Admission {
            appearance_items: Some(oteryn_dev_client::AppearanceItemDictionary {
                content_generation: item_pin.generation,
                definition_refs: &item_refs,
            }),
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
    if tile(session.world_spatial().actor_position)? != prior_position {
        return Err("C12.client old position was not retained across relog".into());
    }
    evidence("C12_CLIENT_EQUIPPED item=50271 right_hand_count=1 fresh_session=true");
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
    let mut traveled = String::new();
    for (direction, expected) in route {
        let before = tile(session.world_spatial().actor_position)?;
        let step = session.step(direction).await.inspect_err(|error| {
            evidence(&format!("C12_CLIENT_STEP_ERROR from={before:?} expected={expected:?} direction={direction:?} error={error}"));
        })?;
        require_mechanic_acceptance(step.status, "C12.client STEP")?;
        let actual = tile(session.world_spatial().actor_position)?;
        if step.disposition != StepDisposition::Moved
            || step.world_spatial_delta.is_none()
            || actual != expected
            || actual == before
        {
            return Err(format!(
                "C12.client STEP {direction:?} from={before:?} expected={expected:?} actual={actual:?}: {step:?}"
            )
            .into());
        }
        traveled.push(char::from(b'0' + direction as u8));
        verdict::Harness::global().progress("client-hunt");
        session
            .service_liveness_discarding_events(Duration::from_millis(u64::from(step.step_wait_ms)))
            .await
            .inspect_err(|error| {
                evidence(&format!(
                    "C12_CLIENT_STEP_LIVENESS_ERROR at={actual:?} command={} error={error}",
                    step.command_id
                ));
            })?;
    }
    let position = session.world_spatial().actor_position;
    let target = full_row_c12::client_fixture_target(&mut session)
        .await
        .inspect_err(|error| {
            evidence(&format!(
                "C12_CLIENT_FIXTURE_TARGET_ERROR at={position:?} error={error}"
            ));
        })?;
    let entities = session
        .world_entities()
        .ok_or("C12 fresh entity projection absent")?;
    let own = entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .ok_or("C12 fresh caster entity absent")?
        .entity;
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
    let damage_before = inspect_damage(id, target).await;
    let cast_origin = session.world_spatial().actor_position;
    let target_before = session
        .world_entities()
        .and_then(|entities| entities.get(&target))
        .ok_or("C12.client selected target absent immediately before CAST")?;
    let target_position = target_before.position;
    let target_detail = target_before.detail;
    if target_position.floor != cast_origin.floor
        || target_position
            .x
            .abs_diff(cast_origin.x)
            .max(target_position.y.abs_diff(cast_origin.y))
            != 1
    {
        return Err(format!("C12.client selected target out of range at CAST: {target:?} origin={cast_origin:?} position={target_position:?}").into());
    }
    evidence(&format!(
        "C12_CLIENT_CAST_TARGET target={target:?} origin={cast_origin:?} position={target_position:?} detail={target_detail:?} owner_damage_before={damage_before:?}"
    ));
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
    let target_identity = hex_bytes(&target.identity);
    let marker = format!(
        "SEAM_EVIDENCE c12_base_damage session={session_id} command={command} target={target_identity} generation={} ",
        target.generation
    );
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
            let damage_after = inspect_damage(id, target).await;
            let owner_cast = full_row_c12::committed_base_damage(
                &path,
                offset,
                &marker,
                full_row_c12::content_augment()?,
            );
            let target_after = session
                .world_entities()
                .and_then(|entities| entities.get(&target));
            evidence(&format!(
                "C12_CLIENT_CAST_DIAGNOSTIC session={session_id} command={command} mana={before}->{} target_after={target_after:?} owner_damage_before={damage_before:?} owner_damage_after={damage_after:?} committed_cast={owner_cast:?}",
                vitals.mana
            ));
            return Err("C12.client cast did not serve attributed damage".into());
        }
        session.service_liveness(Duration::from_millis(25)).await?;
    };
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
        "{}\n{session_id} {earlier} {command} {result_sequence} {sequence} {attack_id} {spell} {target_identity} {target_generation} {definition} {before} {health} {max_health} {mana} {max_mana} {soul} {harmony} {serene} {damage} {before_sequence} {before_bytes} {traveled} {x} {y} {floor}\n",
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
        x = position.x,
        y = position.y,
        floor = position.floor,
    );
    let bytes = received
        .lock()
        .map_err(|_| "C12 capture lock unavailable")?
        .clone();
    verify_post_cast_relog(session, clients, id, before).await?;
    let frames = frame_capture::envelopes(&bytes, id)
        .map_err(|error| error as Box<dyn std::error::Error>)?;
    let frames = serde_json::to_vec(&frames)?;
    write_packet(
        &root,
        "c12-item-dictionary.json",
        &serde_json::to_vec(&item_pin)?,
    )?;
    write_packet(&root, "c12-wire.bin", &bytes)?;
    write_packet(&root, "c12-frames.json", &frames)?;
    write_packet(&root, "c12-expected.txt", expected.as_bytes())?;
    evidence(&format!(
        "C12_CAPTURE session={session_id} command={command} result_sequence={result_sequence} delta_sequence={sequence}"
    ));
    Ok(())
}

async fn verify_post_cast_relog<S: oteryn_dev_client::native_session::SessionStream>(
    mut session: Session<S>,
    clients: &SeamClients<'_>,
    old_id: oteryn_protocol_oteryn::GameSessionId,
    mana_before_cast: u32,
) -> TestResult {
    let clear = session.attack_target(None).await?;
    require_mechanic_acceptance(clear.status, "C12.client clear attack")?;
    if clear.disposition != AttackIntentDisposition::Ok {
        return Err(format!("C12.client clear attack refused: {clear:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(75);
    loop {
        let combat = session
            .combat_state()
            .ok_or("C12.client post-cast combat state absent")?;
        if combat.target.is_none() && !combat.in_fight {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C12.client post-cast combat lock did not expire".into());
        }
        session
            .service_liveness_discarding_events(Duration::from_millis(250))
            .await?;
    }
    let mana = session
        .actor_vitals()
        .ok_or("C12.client pre-logout vitals absent")?
        .mana;
    if mana >= mana_before_cast {
        return Err(format!("C12.client post-cast mana no longer distinguishes pre-state: {mana_before_cast}->{mana}").into());
    }
    let logout = session.logout().await?;
    require_mechanic_acceptance(logout.status, "C12.client LOGOUT")?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C12.client logout refused on first attempt: {logout:?}").into());
    }
    drop(session);
    let saved_mana = full_row_c12::saved_mana(clients).await?;
    if saved_mana < mana || saved_mana >= mana_before_cast {
        return Err(format!("C12.client terminal mana does not preserve the cast debit: before_cast={mana_before_cast} before_logout={mana} saved={saved_mana}").into());
    }
    let fresh = full_row_c12::connect_c12_character(clients).await?;
    let durable_mana = fresh
        .actor_vitals()
        .ok_or("C12.client post-relog vitals absent")?
        .mana;
    if fresh.join_snapshot().game_session_id == old_id || durable_mana != saved_mana {
        return Err(format!("C12.client relog did not retain exact post-cast mana: expected={saved_mana} observed={durable_mana}").into());
    }
    evidence(&format!(
        "C12_CLIENT_RELOG mana_before_cast={mana_before_cast} durable_mana={durable_mana} fresh_session=true"
    ));
    Ok(())
}

async fn inspect_damage(
    session: oteryn_protocol_oteryn::GameSessionId,
    target: oteryn_dev_client::EntityRef,
) -> Result<u64, String> {
    let answer = request_fixture_control(
        uuid_text(session.as_bytes()),
        "--inspect-damage",
        format!(
            "{} {}",
            full_row_c12::hex_bytes(&target.identity),
            target.generation
        ),
    )
    .await?;
    let mut fields = answer.split_ascii_whitespace();
    match (fields.next(), fields.next(), fields.next(), fields.next()) {
        (Some("damage"), Some(total), Some(_), None) => total
            .parse()
            .map_err(|error| format!("C12 damage inspection total: {error}")),
        _ => Err(format!("C12 damage inspection response: {answer}")),
    }
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

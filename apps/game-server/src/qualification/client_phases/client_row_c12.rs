//! Capture the fresh equipped admission and its actual spell exchange for the client replay.

use super::*;
use oteryn_dev_client::native_session::{Admission, Session, SessionStream};
use oteryn_dev_client::{EntityRef, SessionEvent};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::actor_spell::ActorVitals;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::item_view::{
    EquipmentSlot, ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome,
};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
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
            if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
                let saved = received
                    .lock()
                    .map_err(|_| "C12 failure capture lock unavailable".into())
                    .and_then(|bytes| {
                        write_failed_capture(Path::new(&root), &bytes, &error.to_string())
                    });
                match saved {
                    Ok(()) => evidence(
                        "C12_FAILED_CAPTURE file=c12-capture-status.json wire=c12-failed-wire.bin",
                    ),
                    Err(save_error) => {
                        evidence(&format!("C12_FAILED_CAPTURE save_error={save_error}"))
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
    let sent = Arc::new(Mutex::new(Vec::new()));
    let mut session = Session::admit(
        WireCapture {
            stream,
            received: Arc::clone(&received),
            sent: Arc::clone(&sent),
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
    if id == previous || !frame_capture::equipped(inventory, definition) {
        return Err("C12 fresh admission requires one equipped Fist and a vacant off hand".into());
    }
    if tile(session.world_spatial().actor_position)? != prior_position {
        return Err("C12.client old position was not retained across relog".into());
    }
    evidence("C12_CLIENT_EQUIPPED item=50271 right_hand_count=1 fresh_session=true");
    full_row_c12::require_current_premium(&mut session).await?;
    full_row_c12::require_fists_damage_perks(clients).await?;
    let spell = spell_index(&session, "exori nia")?;
    let builder = spell_index(&session, "exori infir pug")?;
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
    let own = select_target(&mut session, target, position).await?;
    let attack_id = session.next_command_id() - 1;
    let builder_start = wire_boundary(&received)?;
    let builder_sequence = session.last_server_sequence();
    let builder_damage = build_harmony(&mut session, own, target, position, builder).await?;
    let before_sequence = session.last_server_sequence();
    let before_bytes = wire_boundary(&received)?;
    let mut cast = cast_checked(
        &mut session,
        own,
        target,
        position,
        spell,
        full_row_c12::AUGMENT_BPS,
    )
    .await?;
    let clear_start = wire_boundary(&received)?;
    let clear_sequence = session.last_server_sequence();
    let (bytes, logout_start, logout_sequence, mut fresh) = verify_post_cast_relog(
        session, clients, id, &received, &mut cast, own, target, spell,
    )
    .await?;

    if !frame_capture::equipped(
        fresh.inventory().ok_or("C12 successor inventory absent")?,
        definition,
    ) {
        return Err("C12 successor lost equipped Fists or has an occupied off hand".into());
    }
    full_row_c12::require_fists_damage_perks(clients).await?;
    let fresh_spell = spell_index(fresh.session_mut(), "exori nia")?;
    let fresh_builder = spell_index(fresh.session_mut(), "exori infir pug")?;
    let fresh_own = select_target(fresh.session_mut(), target, position).await?;
    build_harmony(
        fresh.session_mut(),
        fresh_own,
        target,
        position,
        fresh_builder,
    )
    .await?;
    let mut durable_cast = cast_checked(
        fresh.session_mut(),
        fresh_own,
        target,
        position,
        fresh_spell,
        full_row_c12::AUGMENT_BPS,
    )
    .await?;
    // Keep the entire authored cooldown free of another Greater Tiger Clash.
    fresh.service_liveness(Duration::from_secs(8)).await?;
    collect_hits(
        &fresh.take_events(),
        fresh_own,
        target,
        fresh_spell,
        &mut durable_cast.hits,
    );
    durable_cast.verify_hits()?;
    evidence(&format!(
        "C12_CLIENT_DURABLE_AUGMENT xp=100000 perks=1,3 augment_bps=800 damage={}",
        durable_cast.damage
    ));
    let handle = fresh
        .inventory()
        .and_then(|inventory| {
            inventory
                .equipment
                .iter()
                .find(|entry| entry.slot == EquipmentSlot::RightHand)
        })
        .ok_or("C12 equipped Fists handle missing")?
        .item
        .handle;
    let moved = fresh
        .move_item(&ItemMoveIntent {
            source: handle,
            destination: ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != ItemMoveOutcome::Moved || moved.status != CommandStatus::Accepted {
        return Err(format!("C12 unequip failed on first attempt: {moved:?}").into());
    }
    let until = tokio::time::Instant::now() + COMMAND_BUDGET;
    while !frame_capture::carried(
        fresh
            .inventory()
            .ok_or("C12 unequipped inventory missing")?,
        definition,
    ) {
        if tokio::time::Instant::now() >= until {
            return Err("C12 control cast requires one copy of the weapon in the backpack".into());
        }
        fresh.service_liveness(Duration::from_millis(25)).await?;
        fresh.take_events();
    }
    let baseline_spell = spell_index(fresh.session_mut(), "exori nia")?;
    let baseline_builder = spell_index(fresh.session_mut(), "exori infir pug")?;
    let own = select_target(fresh.session_mut(), target, position).await?;
    build_harmony(fresh.session_mut(), own, target, position, baseline_builder).await?;
    let mut baseline = cast_checked(
        fresh.session_mut(),
        own,
        target,
        position,
        baseline_spell,
        0,
    )
    .await?;
    fresh.service_liveness(Duration::from_secs(8)).await?;
    collect_hits(
        &fresh.take_events(),
        own,
        target,
        baseline_spell,
        &mut baseline.hits,
    );
    baseline.verify_hits()?;
    evidence(&format!(
        "C12_CLIENT_UNEQUIPPED command={} damage={} augment_bps=0 harmony=1->0",
        baseline.command, baseline.damage
    ));

    let CheckedCast {
        command,
        sequence,
        result_sequence,
        vitals,
        mana_before: before,
        damage,
        damage_before,
        damage_augmented,
        damage_critical,
        ..
    } = cast;
    let session_id = hex_bytes(id.as_bytes());
    let target_identity = hex_bytes(&target.identity);
    let earlier = hex_bytes(previous.as_bytes());
    let expected = format!(
        "{}\n{session_id} {earlier} {command} {result_sequence} {sequence} {attack_id} {spell} {target_identity} {target_generation} {definition} {before} {health} {max_health} {mana} {max_mana} {soul} {harmony} {serene} {damage} {damage_before} {damage_augmented} {damage_critical} {before_sequence} {before_bytes} {builder} {builder_damage} {builder_start} {builder_sequence} {clear_start} {clear_sequence} {logout_start} {logout_sequence} {traveled} {x} {y} {floor}\n",
        root.display(),
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
    let frames = frame_capture::envelopes(&bytes, id)
        .map_err(|error| error as Box<dyn std::error::Error>)?;
    let frames = serde_json::to_vec(&frames)?;
    write_packet(
        &root,
        "c12-item-dictionary.json",
        &serde_json::to_vec(&item_pin)?,
    )?;
    write_packet(&root, "c12-wire.bin", &bytes)?;
    let sent = sent
        .lock()
        .map_err(|_| "C12 outgoing capture lock unavailable")?;
    let commands = frame_capture::command_frames(&sent)
        .map_err(|error| error as Box<dyn std::error::Error>)?
        .concat();
    write_packet(&root, "c12-client-wire.bin", &commands)?;
    write_packet(&root, "c12-frames.json", &frames)?;
    write_packet(&root, "c12-expected.txt", expected.as_bytes())?;
    write_packet(
        &root,
        "c12-capture-status.json",
        br#"{"scenario":"C12.client","status":"CAPTURED"}"#,
    )?;
    evidence(&format!(
        "C12_CAPTURE session={session_id} command={command} result_sequence={result_sequence} delta_sequence={sequence}"
    ));
    Ok(())
}

fn wire_boundary(received: &Mutex<Vec<u8>>) -> TestResult<usize> {
    Ok(received
        .lock()
        .map_err(|_| "C12 capture lock unavailable")?
        .len())
}

fn spell_index<S: SessionStream>(session: &Session<S>, words: &str) -> TestResult<NonZeroU32> {
    let catalog = session
        .spell_catalog()?
        .ok_or("C12 spell catalogue absent")?;
    catalog
        .entries
        .iter()
        .find(|entry| entry.words == words && entry.available && entry.rune_item.is_none())
        .and_then(|entry| NonZeroU32::new(entry.cast_index))
        .ok_or_else(|| format!("C12 available spell absent: {words}").into())
}

fn check_target<S: SessionStream>(
    session: &Session<S>,
    target: EntityRef,
    position: ActorPosition,
) -> TestResult {
    let origin = session.world_spatial().actor_position;
    let actual = session
        .world_entities()
        .and_then(|entities| entities.get(&target))
        .ok_or("C12 selected target absent immediately before CAST")?
        .position;
    if !frame_capture::exact_target(origin, actual, position)
        || session.combat_state().and_then(|state| state.target) != Some(target)
    {
        return Err(format!("C12 exact north attack target required: origin={origin:?} target={actual:?} expected_origin={position:?}").into());
    }
    Ok(())
}

async fn select_target<S: SessionStream>(
    session: &mut Session<S>,
    target: EntityRef,
    position: ActorPosition,
) -> TestResult<EntityRef> {
    let entities = session
        .world_entities()
        .ok_or("C12 entity projection absent")?;
    let own = entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .ok_or("C12 caster entity absent")?
        .entity;
    if session
        .combat_state()
        .ok_or("C12 combat state absent")?
        .target
        .is_some()
    {
        return Err("C12 combat target already set".into());
    }
    let attack = session.attack_target(Some(&target)).await?;
    require_mechanic_acceptance(attack.status, "C12.client ATTACK")?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("C12.client attack reply={attack:?}").into());
    }
    let until = tokio::time::Instant::now() + COMMAND_BUDGET;
    while session.combat_state().and_then(|state| state.target) != Some(target) {
        if tokio::time::Instant::now() >= until {
            return Err("C12 attack target change not served".into());
        }
        session
            .service_liveness_discarding_events(Duration::from_millis(25))
            .await?;
    }
    check_target(session, target, position)?;
    Ok(own)
}

pub(super) fn collect_hits(
    events: &[SessionEvent],
    own: EntityRef,
    target: EntityRef,
    spell: NonZeroU32,
    hits: &mut Vec<u64>,
) {
    for event in events {
        if let SessionEvent::Presentation(delta) = event {
            frame_capture::attributed_hits(&delta.value, own, target, spell.get(), hits);
        }
    }
}

async fn build_harmony<S: SessionStream>(
    session: &mut Session<S>,
    own: EntityRef,
    target: EntityRef,
    position: ActorPosition,
    spell: NonZeroU32,
) -> TestResult<u64> {
    check_target(session, target, position)?;
    if session
        .actor_vitals()
        .ok_or("C12 builder pre-state missing")?
        .harmony
        != 0
    {
        return Err("C12 Swift Jab requires zero Harmony before cast".into());
    }
    let path = PathBuf::from(required("NODE_BOOT_CAST_EVIDENCE_LOG")?);
    let offset = std::fs::metadata(&path)?.len();
    session.take_events();
    let reply = session
        .cast_spell(spell, SpellTarget::AttackTarget, true)
        .await?;
    require_mechanic_acceptance(reply.status, "C12.client Swift Jab")?;
    if reply.disposition != SpellCastDisposition::Cast
        || reply
            .actor_vitals_delta
            .is_none_or(|delta| delta.value.harmony != 1)
    {
        return Err("C12 Swift Jab did not publish Harmony 0 -> 1".into());
    }
    let mut hits = Vec::new();
    collect_hits(&session.take_events(), own, target, spell, &mut hits);
    // Swift Jab's authored attack-group cooldown is two seconds.
    session.service_liveness(Duration::from_secs(2)).await?;
    collect_hits(&session.take_events(), own, target, spell, &mut hits);
    let damage = committed_damage(session, target, reply.command_id, &path, offset, 0)?.committed;
    if !frame_capture::exact_hit(&hits, damage) {
        return Err(format!(
            "C12 Swift Jab expected one unaugmented hit of {damage}, observed {hits:?}"
        )
        .into());
    }
    Ok(damage)
}

struct CheckedCast {
    command: u64,
    result_sequence: u64,
    sequence: u64,
    mana_before: u32,
    vitals: ActorVitals,
    damage: u64,
    damage_before: u64,
    damage_augmented: u64,
    damage_critical: bool,
    hits: Vec<u64>,
}

impl CheckedCast {
    fn verify_hits(&self) -> TestResult {
        if !frame_capture::exact_hit(&self.hits, self.damage) {
            return Err(format!(
                "C12 command {} expected exactly one attributed hit of {}, observed {:?}",
                self.command, self.damage, self.hits
            )
            .into());
        }
        Ok(())
    }
}

async fn cast_checked<S: SessionStream>(
    session: &mut Session<S>,
    own: EntityRef,
    target: EntityRef,
    position: ActorPosition,
    spell: NonZeroU32,
    bps: u64,
) -> TestResult<CheckedCast> {
    let path = PathBuf::from(required("NODE_BOOT_CAST_EVIDENCE_LOG")?);
    let offset = std::fs::metadata(&path)?.len();
    check_target(session, target, position)?;
    let before = *session.actor_vitals().ok_or("C12 cast pre-state missing")?;
    if before.harmony != 1 {
        return Err("C12 Greater Tiger Clash requires one Harmony before CAST".into());
    }
    session.take_events();
    let reply = session
        .cast_spell(spell, SpellTarget::AttackTarget, true)
        .await?;
    require_mechanic_acceptance(reply.status, "C12.client CAST")?;
    let delta = reply.actor_vitals_delta.ok_or("C12 cast vitals missing")?;
    let vitals = delta.value;
    let events = session.take_events();
    let mana_before = frame_capture::mana_before_cast(
        events.iter().filter_map(|event| match event {
            SessionEvent::ActorVitals(update) => Some((update.server_sequence, update.value.mana)),
            _ => None,
        }),
        before.mana,
        before.max_mana,
        delta.server_sequence,
    )
    .ok_or("C12 unexpected mana change before the cast delta")?;
    if reply.disposition != SpellCastDisposition::Cast
        || mana_before.checked_sub(50) != Some(vitals.mana)
        || !frame_capture::spent_harmony(before.harmony, vitals.harmony)
        || session.actor_vitals() != Some(&vitals)
    {
        return Err(format!("C12 cast must apply exact mana and Harmony debits: before={before:?} mana_at_cast={mana_before} after={vitals:?}").into());
    }
    let damage = committed_damage(session, target, reply.command_id, &path, offset, bps)?;
    let mut hits = Vec::new();
    collect_hits(&events, own, target, spell, &mut hits);
    let until = tokio::time::Instant::now() + COMMAND_BUDGET;
    while hits.is_empty() {
        if tokio::time::Instant::now() >= until {
            return Err("C12 attributed damage missing after CAST".into());
        }
        session.service_liveness(Duration::from_millis(25)).await?;
        collect_hits(&session.take_events(), own, target, spell, &mut hits);
    }
    let cast = CheckedCast {
        command: reply.command_id,
        result_sequence: reply.result_server_sequence,
        sequence: delta.server_sequence,
        mana_before,
        vitals,
        damage: damage.committed,
        damage_before: damage.before,
        damage_augmented: damage.augmented,
        damage_critical: damage.critical,
        hits,
    };
    cast.verify_hits()?;
    Ok(cast)
}

fn committed_damage<S: SessionStream>(
    session: &Session<S>,
    target: EntityRef,
    command: u64,
    path: &Path,
    offset: u64,
    bps: u64,
) -> TestResult<full_row_c12::CommittedDamage> {
    let id = full_row_c12::hex_bytes(session.join_snapshot().game_session_id.as_bytes());
    let target_id = full_row_c12::hex_bytes(&target.identity);
    let marker = format!(
        "SEAM_EVIDENCE c12_base_damage session={id} command={} target={target_id} generation={} ",
        command, target.generation
    );
    full_row_c12::committed_base_damage(path, offset, &marker, bps)
}

#[allow(clippy::too_many_arguments)]
async fn verify_post_cast_relog<S: SessionStream>(
    mut session: Session<S>,
    clients: &SeamClients<'_>,
    old_id: oteryn_protocol_oteryn::GameSessionId,
    received: &Mutex<Vec<u8>>,
    cast: &mut CheckedCast,
    own: EntityRef,
    target: EntityRef,
    spell: NonZeroU32,
) -> TestResult<(Vec<u8>, usize, u64, oteryn_dev_client::DevClientSession)> {
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
        session.service_liveness(Duration::from_millis(250)).await?;
        collect_hits(&session.take_events(), own, target, spell, &mut cast.hits);
    }
    let prior_mana = session
        .actor_vitals()
        .ok_or("C12 pre-logout vitals absent")?
        .mana;
    let until = tokio::time::Instant::now() + Duration::from_secs(12);
    let mana = loop {
        session.service_liveness(Duration::from_millis(50)).await?;
        collect_hits(&session.take_events(), own, target, spell, &mut cast.hits);
        let mana = session
            .actor_vitals()
            .ok_or("C12 logout regen vitals absent")?
            .mana;
        if matches!(mana.checked_sub(prior_mana), Some(1 | 2)) {
            break mana;
        }
        if mana != prior_mana || tokio::time::Instant::now() >= until {
            return Err(format!("C12 expected one final mana tick: {prior_mana} -> {mana}").into());
        }
    };
    let logout_start = wire_boundary(received)?;
    let logout_sequence = session.last_server_sequence();
    let logout = session.logout().await?;
    require_mechanic_acceptance(logout.status, "C12.client LOGOUT")?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C12.client logout refused on first attempt: {logout:?}").into());
    }
    collect_hits(&session.take_events(), own, target, spell, &mut cast.hits);
    cast.verify_hits()?;
    let bytes = received
        .lock()
        .map_err(|_| "C12 capture lock unavailable")?
        .clone();
    drop(session);
    let saved = full_row_c12::saved_mana(clients).await?;
    let fresh = full_row_c12::connect_c12_character(clients).await?;
    let reloaded = fresh
        .actor_vitals()
        .ok_or("C12 post-relog vitals absent")?
        .mana;
    if fresh.join_snapshot().game_session_id == old_id
        || !frame_capture::exact_saved_mana(cast.mana_before, mana, saved, reloaded)
    {
        return Err(format!("C12 exact mana save mismatch: before_cast={} before_logout={mana} saved={saved} reloaded={reloaded}", cast.mana_before).into());
    }
    evidence(&format!(
        "C12_CLIENT_RELOG mana_before_cast={} before_logout={mana} durable_mana={reloaded} fresh_session=true",
        cast.mana_before
    ));
    Ok((bytes, logout_start, logout_sequence, fresh))
}

fn write_failed_capture(root: &Path, wire: &[u8], error: &str) -> TestResult {
    let failure = serde_json::json!({
        "scenario": "C12.client",
        "status": "FAIL",
        "server_wire_bytes": wire.len(),
        "error": error,
    });
    write_packet(root, "c12-failed-wire.bin", wire)?;
    write_packet(
        root,
        "c12-capture-status.json",
        &serde_json::to_vec(&failure)?,
    )
}

#[test]
fn c12_admission_failure_records_a_zero_frame_capture() -> TestResult {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
    let root = std::env::temp_dir().join(format!(
        "oteryn-c12-{}-{}",
        std::process::id(),
        stamp.as_nanos()
    ));
    std::fs::create_dir(&root)?;
    let result = (|| -> TestResult {
        let error = "Platform-issued row grant unavailable";
        write_failed_capture(&root, &[], error)?;
        let diagnostic: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("c12-capture-status.json"))?)?;
        assert_eq!(diagnostic["status"], "FAIL");
        assert_eq!(diagnostic["server_wire_bytes"], 0);
        assert_eq!(diagnostic["error"], error);
        assert_eq!(root.join("c12-failed-wire.bin").metadata()?.len(), 0);
        assert!(!root.join("c12-expected.txt").exists());
        Ok(())
    })();
    std::fs::remove_dir_all(&root)?;
    result
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
    sent: Arc<Mutex<Vec<u8>>>,
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
        let result = Pin::new(&mut self.stream).poll_write(cx, bytes);
        if let Poll::Ready(Ok(written)) = result {
            let mut sent = self
                .sent
                .lock()
                .map_err(|_| io::Error::other("C12 outgoing capture lock unavailable"))?;
            if written > frame_capture::FRAME_LIMIT.saturating_sub(sent.len()) {
                return Poll::Ready(Err(io::Error::other("C12 outgoing capture exceeds bounds")));
            }
            sent.extend_from_slice(&bytes[..written]);
        }
        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

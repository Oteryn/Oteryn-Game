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
use oteryn_protocol_oteryn::weapon_proficiency as proficiency;
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
        Duration::from_secs(1100),
        verdict::observe(capture(clients, Arc::clone(&received))),
    )
    .await
    .unwrap_or_else(|_| Err("C12.client capture deadline=1100s".into()));
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
    let builder_damage =
        build_harmony(&mut session, own, target, position, builder, 0, false).await?;
    let before_sequence = session.last_server_sequence();
    let before_bytes = wire_boundary(&received)?;
    let mut cast = cast_checked(
        &mut session,
        own,
        target,
        position,
        spell,
        1,
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
    let catalyst = provisioning::item_reference(
        &provisioning::item_index()?,
        full_row_c12::GREATER_PROFICIENCY_CATALYST,
    )?;
    require_consumed_catalyst(fresh.session_mut(), catalyst)?;
    require_fists_browse(fresh.session_mut(), definition, SELECTED_DAMAGE_PERKS).await?;
    evidence(
        "C12_CLIENT_PROFICIENCY_BROWSE fresh_session=true level=3 xp=100000 perks=1,3 catalyst=absent",
    );
    let fresh_spell = spell_index(fresh.session_mut(), "exori nia")?;
    let fresh_builder = spell_index(fresh.session_mut(), "exori infir pug")?;
    let fresh_own = select_target(fresh.session_mut(), target, position).await?;
    build_harmony(
        fresh.session_mut(),
        fresh_own,
        target,
        position,
        fresh_builder,
        0,
        true,
    )
    .await?;
    let mut durable_cast = cast_checked(
        fresh.session_mut(),
        fresh_own,
        target,
        position,
        fresh_spell,
        1,
        full_row_c12::AUGMENT_BPS,
    )
    .await?;
    // Once the two-second attack group ends, the eight-second spell cooldown still applies.
    fresh.service_liveness(Duration::from_secs(2)).await?;
    require_cooldown(fresh.session_mut(), fresh_spell).await?;
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
    // The unequipped control uses the same selected creature in the same session.
    build_harmony(
        fresh.session_mut(),
        fresh_own,
        target,
        position,
        fresh_builder,
        0,
        false,
    )
    .await?;
    let mut baseline = cast_checked(
        fresh.session_mut(),
        fresh_own,
        target,
        position,
        fresh_spell,
        1,
        0,
    )
    .await?;
    fresh.service_liveness(Duration::from_secs(8)).await?;
    collect_hits(
        &fresh.take_events(),
        fresh_own,
        target,
        fresh_spell,
        &mut baseline.hits,
    );
    baseline.verify_hits()?;
    evidence(&format!(
        "C12_CLIENT_UNEQUIPPED command={} damage={} augment_bps=0 harmony=1->0",
        baseline.command, baseline.damage
    ));

    let world = load_world(
        std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
        &serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let combat_tile = tile(position)?;
    let target_tile = TilePos {
        y: combat_tile
            .y
            .checked_sub(1)
            .ok_or("C12 target tile overflow")?,
        ..combat_tile
    };
    let is_protection_zone = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone())
    };
    if is_protection_zone(combat_tile) {
        return Err("C12 refusal must be attempted outside a protection zone".into());
    }
    let mut perks = SELECTED_DAMAGE_PERKS;
    for level in [0, 2] {
        require_perk_reply(
            fresh.session_mut(),
            definition,
            level,
            None,
            proficiency::SelectPerkResult::NotInProtectionZone,
            perks,
        )
        .await?;
    }
    full_row_c12::require_fists_damage_perks(clients).await?;
    evidence("C12_CLIENT_PERK_CLEAR_REFUSED reason=NotInProtectionZone perks=1,3");
    let to_protection_zone = route_avoiding(&world, combat_tile, is_protection_zone, |at| {
        at != target_tile
    })
    .ok_or("C12 no authored route to a protection zone")?;
    full_row_c12::equip_right(&mut fresh, definition).await?;
    for (changes, active_level) in [
        (&[(0, None)][..], 3),
        (&[(0, Some(0)), (2, None)][..], 1),
        (&[(0, None)][..], 0),
    ] {
        let clear = fresh.session_mut().attack_target(None).await?;
        require_mechanic_acceptance(clear.status, "C12.client clear target before perk changes")?;
        if clear.disposition != AttackIntentDisposition::Ok {
            return Err(format!("C12.client clear target failed: {clear:?}").into());
        }
        full_row_c12::wait_combat_target(&mut fresh, None).await?;
        full_row_c12::walk_exact_route(&mut fresh, &world, &to_protection_zone).await?;
        let protection_tile = tile(fresh.world_spatial().actor_position)?;
        if !is_protection_zone(protection_tile) {
            return Err("C12 perk control did not reach a protection zone".into());
        }
        for &(level, perk) in changes {
            perks[usize::from(level)] = perk;
            require_perk_reply(
                fresh.session_mut(),
                definition,
                level,
                perk,
                proficiency::SelectPerkResult::Accepted,
                perks,
            )
            .await?;
        }
        if active_level == 0 {
            require_fists_browse(fresh.session_mut(), definition, [None; 4]).await?;
            let previous = fresh.join_snapshot().game_session_id;
            full_row_c12::logout(&mut fresh, false).await?;
            drop(fresh);
            fresh = full_row_c12::connect_c12_character(clients).await?;
            if fresh.join_snapshot().game_session_id == previous
                || tile(fresh.world_spatial().actor_position)? != protection_tile
            {
                return Err("C12 perk removal requires a fresh admission at the saved tile".into());
            }
            require_fists_browse(fresh.session_mut(), definition, [None; 4]).await?;
            require_consumed_catalyst(fresh.session_mut(), catalyst)?;
            evidence(
                "C12_CLIENT_PERKS_CLEARED fresh_session=true xp=100000 perks=none catalyst=absent",
            );
        }
        let back_to_target = route_avoiding(
            &world,
            protection_tile,
            |at| at == combat_tile,
            |at| at != target_tile,
        )
        .ok_or("C12 no authored route back to the same combat tile")?;
        full_row_c12::walk_exact_route(&mut fresh, &world, &back_to_target).await?;
        if !frame_capture::equipped(
            fresh
                .inventory()
                .ok_or("C12 equipped control inventory absent")?,
            definition,
        ) {
            return Err(
                "C12 proficiency control requires the Fists equipped and off hand vacant".into(),
            );
        }
        let own = select_target(fresh.session_mut(), target, position).await?;
        let spell = spell_index(fresh.session_mut(), "exori nia")?;
        let builder = spell_index(fresh.session_mut(), "exori infir pug")?;
        // A five-Harmony roll makes each 4% contribution observable after rounding.
        let harmony = if active_level == 0 { 1 } else { 5 };
        for before in 0..harmony {
            build_harmony(
                fresh.session_mut(),
                own,
                target,
                position,
                builder,
                before,
                false,
            )
            .await?;
        }
        let bps = if active_level == 0 { 0 } else { 400 };
        let mut control = cast_checked(
            fresh.session_mut(),
            own,
            target,
            position,
            spell,
            harmony,
            bps,
        )
        .await?;
        fresh.service_liveness(Duration::from_secs(8)).await?;
        collect_hits(&fresh.take_events(), own, target, spell, &mut control.hits);
        control.verify_hits()?;
        evidence(&format!(
            "C12_CLIENT_PERK_CONTROL active_level={active_level} augment_bps={bps} command={} damage={}",
            control.command, control.damage
        ));
    }

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
    harmony_before: u32,
    probe_cooldown: bool,
) -> TestResult<u64> {
    check_target(session, target, position)?;
    let before = *session
        .actor_vitals()
        .ok_or("C12 builder pre-state missing")?;
    if before.harmony != harmony_before {
        return Err(format!("C12 Swift Jab requires Harmony {harmony_before} before cast").into());
    }
    let path = PathBuf::from(required("NODE_BOOT_CAST_EVIDENCE_LOG")?);
    let offset = std::fs::metadata(&path)?.len();
    session.take_events();
    let reply = session
        .cast_spell(spell, SpellTarget::AttackTarget, true)
        .await?;
    require_mechanic_acceptance(reply.status, "C12.client Swift Jab")?;
    let delta = reply
        .actor_vitals_delta
        .ok_or("C12 Swift Jab vitals missing")?;
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
    .ok_or("C12 unexpected mana change before Swift Jab")?;
    if reply.disposition != SpellCastDisposition::Cast
        || delta.value.harmony != harmony_before + 1
        || mana_before.checked_sub(3) != Some(delta.value.mana)
    {
        return Err(format!(
            "C12 Swift Jab expected 3 mana and Harmony {harmony_before} -> {}, observed mana={mana_before}->{} Harmony={}",
            harmony_before + 1, delta.value.mana, delta.value.harmony
        ).into());
    }
    let mut hits = Vec::new();
    collect_hits(&events, own, target, spell, &mut hits);
    if probe_cooldown {
        require_cooldown(session, spell).await?;
    }
    // Swift Jab's authored attack-group cooldown is two seconds.
    session.service_liveness(Duration::from_secs(2)).await?;
    let trailing = session.take_events();
    if !(validate_cast_timers(session, &events, spell, 2_000)?
        || validate_cast_timers(session, &trailing, spell, 2_000)?)
    {
        return Err("C12 Swift Jab cooldown projection missing".into());
    }
    collect_hits(&trailing, own, target, spell, &mut hits);
    let damage = committed_damage(session, target, reply.command_id, &path, offset, 0)?.committed;
    if !frame_capture::exact_hit(&hits, damage) {
        return Err(format!(
            "C12 Swift Jab expected one unaugmented hit of {damage}, observed {hits:?}"
        )
        .into());
    }
    Ok(damage)
}

/// Exercise the original authored hunt before the isolated damage controls.
pub(super) async fn cast_at_authored_spawn<S: SessionStream>(
    session: &mut Session<S>,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + COMMAND_BUDGET;
    let target = loop {
        let visible = session
            .world_entities()
            .ok_or("C12 authored entity projection absent")?;
        let mut matching = visible.iter().filter(|entry| {
            entry.kind == EntityKind::Creature
                && entry.position.x == 32294
                && entry.position.y == 32333
                && entry.position.floor == 8
                && entry.display_name.is_some_and(|name| {
                    name.as_str()
                        .is_ok_and(|n| n == full_row_c12::CLIENT_TARGET_NAME)
                })
        });
        if let Some(entry) = matching.next() {
            if matching.next().is_some() {
                return Err("C12 authored spawn is ambiguous".into());
            }
            break entry.entity;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C12 Tarantula absent at the original authored spawn".into());
        }
        session
            .service_liveness_discarding_events(Duration::from_millis(50))
            .await?;
    };
    let spell = spell_index(session, "exori nia")?;
    let attack = session.attack_target(Some(&target)).await?;
    require_mechanic_acceptance(attack.status, "C12 authored Tarantula target")?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("C12 authored Tarantula target failed: {attack:?}").into());
    }
    let before = *session
        .actor_vitals()
        .ok_or("C12 authored cast pre-state missing")?;
    let outcome = session
        .cast_spell(spell, SpellTarget::AttackTarget, true)
        .await?;
    require_mechanic_acceptance(outcome.status, "C12 authored Tarantula cast")?;
    let update = outcome
        .actor_vitals_delta
        .ok_or("C12 authored cast vitals missing")?;
    let events = session.take_events();
    let mana = frame_capture::mana_before_cast(
        events.iter().filter_map(|event| match event {
            SessionEvent::ActorVitals(update) => Some((update.server_sequence, update.value.mana)),
            _ => None,
        }),
        before.mana,
        before.max_mana,
        update.server_sequence,
    );
    if outcome.disposition != SpellCastDisposition::Cast
        || mana.and_then(|mana| mana.checked_sub(50)) != Some(update.value.mana)
    {
        return Err(
            format!("C12 authored cast did not commit its 50 mana debit: {outcome:?}").into(),
        );
    }
    evidence("C12_CLIENT_AUTHORED_CAST spawn=32294,32333,-8 cast=32295,32334,-8 mana_debit=50");
    Ok(())
}

fn validate_cast_timers<S: SessionStream>(
    session: &Session<S>,
    events: &[SessionEvent],
    spell: NonZeroU32,
    spell_ms: u64,
) -> TestResult<bool> {
    let catalog = session
        .spell_catalog()?
        .ok_or("C12 cooldown catalogue absent")?;
    let [group] = catalog
        .entries
        .iter()
        .find(|entry| entry.cast_index == spell.get())
        .ok_or("C12 cooldown spell is missing")?
        .cooldown_groups
        .as_slice()
    else {
        return Err("C12 expects exactly one attack group".into());
    };
    let timers = events.iter().find_map(|event| match event {
        SessionEvent::ActorCooldowns(update) => Some(&update.value),
        _ => None,
    });
    match timers {
        None => Ok(false),
        Some(timers)
            if frame_capture::exact_cast_cooldowns(timers, spell.get(), *group, spell_ms) =>
        {
            Ok(true)
        }
        Some(timers) => Err(format!("C12 incorrect spell or group deadlines: {timers:?}").into()),
    }
}

async fn require_cooldown<S: SessionStream>(
    session: &mut Session<S>,
    spell: NonZeroU32,
) -> TestResult {
    let before = *session
        .actor_vitals()
        .ok_or("C12 cooldown pre-state missing")?;
    let reply = session
        .cast_spell(spell, SpellTarget::AttackTarget, true)
        .await?;
    require_mechanic_acceptance(reply.status, "C12 cooldown probe")?;
    let after = session
        .actor_vitals()
        .ok_or("C12 cooldown post-state missing")?;
    if reply.disposition != SpellCastDisposition::CoolingDown
        || reply.actor_vitals_delta.is_some()
        || after.harmony != before.harmony
        || !matches!(after.mana.checked_sub(before.mana), Some(0..=2))
    {
        return Err(format!("C12 CAST during cooldown must refuse without cost: {reply:?}").into());
    }
    evidence(&format!(
        "C12_CLIENT_COOLDOWN spell={spell} disposition=CoolingDown"
    ));
    Ok(())
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
    expected_harmony: u32,
    bps: u64,
) -> TestResult<CheckedCast> {
    let path = PathBuf::from(required("NODE_BOOT_CAST_EVIDENCE_LOG")?);
    let offset = std::fs::metadata(&path)?.len();
    check_target(session, target, position)?;
    let before = *session.actor_vitals().ok_or("C12 cast pre-state missing")?;
    if before.harmony != expected_harmony {
        return Err(format!(
            "C12 Greater Tiger Clash requires Harmony {expected_harmony} before CAST"
        )
        .into());
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
        || vitals.harmony != 0
        || session.actor_vitals() != Some(&vitals)
    {
        return Err(format!("C12 cast must apply exact mana and Harmony debits: before={before:?} mana_at_cast={mana_before} after={vitals:?}").into());
    }
    let damage = committed_damage(session, target, reply.command_id, &path, offset, bps)?;
    if expected_harmony == 5 && !frame_capture::five_harmony_damage(damage.before) {
        return Err(format!(
            "C12 five Harmony damage outside independent fixture range: {}",
            damage.before
        )
        .into());
    }
    let mut hits = Vec::new();
    collect_hits(&events, own, target, spell, &mut hits);
    let until = tokio::time::Instant::now() + COMMAND_BUDGET;
    let mut timers = validate_cast_timers(session, &events, spell, 8_000)?;
    while hits.is_empty() || !timers {
        if tokio::time::Instant::now() >= until {
            return Err("C12 attributed damage missing after CAST".into());
        }
        session.service_liveness(Duration::from_millis(25)).await?;
        let trailing = session.take_events();
        timers |= validate_cast_timers(session, &trailing, spell, 8_000)?;
        collect_hits(&trailing, own, target, spell, &mut hits);
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

const SELECTED_DAMAGE_PERKS: [Option<u8>; 4] = [Some(0), None, Some(0), None];

async fn require_fists_browse<S: SessionStream>(
    session: &mut Session<S>,
    definition: NonZeroU32,
    selections: [Option<u8>; 4],
) -> TestResult {
    let mut offset = 0_u32;
    let mut total = None;
    let mut found = None;
    loop {
        let reply = session
            .gated_command(
                proficiency::CAPABILITY_WEAPON_PROFICIENCY_V1,
                proficiency::COMMAND_TYPE_PROFICIENCY_BROWSE,
                &offset.to_be_bytes(),
            )
            .await?;
        require_mechanic_acceptance(reply.status, "C12.client proficiency browse")?;
        let page = proficiency::ProficiencyPage::decode(&reply.payload)
            .ok_or("C12 malformed proficiency browse reply")?;
        if page.offset != offset || total.is_some_and(|total| total != page.total) {
            return Err(
                format!("C12 inconsistent proficiency page at offset {offset}: {page:?}").into(),
            );
        }
        total = Some(page.total);
        let count = u32::try_from(page.rows.len())?;
        for row in page
            .rows
            .into_iter()
            .filter(|row| row.definition == definition)
        {
            if found.replace(row).is_some() {
                return Err("C12 duplicate Fists row in proficiency browse".into());
            }
        }
        offset = offset
            .checked_add(count)
            .ok_or("C12 proficiency browse offset overflow")?;
        if offset == page.total {
            break;
        }
        if count == 0 {
            return Err("C12 proficiency browse stopped before its advertised total".into());
        }
    }
    let row = found.ok_or("C12 Fists missing from proficiency browse")?;
    require_fists_browse_row(&row, selections)
}

fn require_fists_browse_row(
    row: &proficiency::ProficiencyRow,
    selections: [Option<u8>; 4],
) -> TestResult {
    if row.name != "fists of enlightenment"
        || row.level != 3
        || row.experience != 100_000
        || row.selections != selections
    {
        return Err(format!(
            "C12 proficiency browse expected fists of enlightenment, level 3, 100000 XP and {selections:?}: {row:?}"
        )
        .into());
    }
    Ok(())
}

fn require_consumed_catalyst<S: SessionStream>(
    session: &Session<S>,
    catalyst: NonZeroU32,
) -> TestResult {
    let inventory = session
        .inventory()
        .ok_or("C12 fresh catalyst inventory absent")?;
    if frame_capture::item_units(inventory, catalyst) != 0 {
        return Err("C12 consumed Greater Proficiency Catalyst reappeared after relog".into());
    }
    Ok(())
}

async fn require_perk_reply<S: SessionStream>(
    session: &mut Session<S>,
    definition: NonZeroU32,
    level: u8,
    perk: Option<u8>,
    expected_result: proficiency::SelectPerkResult,
    expected_selections: [Option<u8>; 4],
) -> TestResult {
    let request = proficiency::SelectPerkRequest {
        definition,
        level,
        perk,
    };
    let reply = session
        .gated_command(
            proficiency::CAPABILITY_WEAPON_PROFICIENCY_V1,
            proficiency::COMMAND_TYPE_PROFICIENCY_SELECT_PERK,
            &request.encode(),
        )
        .await?;
    require_mechanic_acceptance(reply.status, "C12.client change damage perk")?;
    let reply = proficiency::SelectPerkReply::decode(&reply.payload)
        .ok_or("C12 malformed perk selection reply")?;
    if reply.result != expected_result || reply.selections != expected_selections {
        return Err(format!(
            "C12 perk {} requested {perk:?} expected {expected_result:?} {expected_selections:?}: {reply:?}",
            level + 1
        ).into());
    }
    Ok(())
}

#[test]
fn c12_wire_proficiency_rejects_stale_xp_or_missing_perks() -> TestResult {
    let mut row = proficiency::ProficiencyRow {
        definition: NonZeroU32::new(50271).ok_or("Fists definition is zero")?,
        name: "fists of enlightenment".into(),
        experience: 100_000,
        level: 3,
        selections: SELECTED_DAMAGE_PERKS.to_vec(),
    };
    require_fists_browse_row(&row, SELECTED_DAMAGE_PERKS)?;
    row.level = 4;
    assert!(require_fists_browse_row(&row, SELECTED_DAMAGE_PERKS).is_err());
    row.level = 3;
    row.name = "incorrect weapon".into();
    assert!(require_fists_browse_row(&row, SELECTED_DAMAGE_PERKS).is_err());
    row.name = "fists of enlightenment".into();
    assert!(require_fists_browse_row(&row, [None; 4]).is_err());
    row.experience -= 1;
    assert!(require_fists_browse_row(&row, SELECTED_DAMAGE_PERKS).is_err());
    row.experience += 1;
    row.selections[2] = None;
    assert!(require_fists_browse_row(&row, SELECTED_DAMAGE_PERKS).is_err());
    row.selections.fill(None);
    require_fists_browse_row(&row, [None; 4])
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

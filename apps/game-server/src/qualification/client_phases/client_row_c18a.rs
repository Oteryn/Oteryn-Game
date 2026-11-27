//! Captures the real C18a combat-state and spell-result payloads consumed by
//! `crates/session/src/domain_dispatch.rs::Session::apply_domain_delta` and
//! `crates/client-runtime/src/features/local-spell-action-bar/protocol.rs::run_cast`.

use super::*;
use crate::qualification_wild_spawn::{C18A_RAT_FIXTURE, C18A_RAT_POSITION};
use oteryn_dev_client::client_runtime::combat::{CombatCommand, CombatReport, run_combat};
use oteryn_dev_client::client_runtime::spell::SpellFeedback;
use oteryn_dev_client::{EntityDetail, SessionEvent};
use oteryn_protocol_oteryn::{
    actor_spell::{self, SpellCastDisposition}, attack,
    item_view as items, presentation, target_move, world_clock,
};

const SCORCH_MANA: u32 = 8;
const MEAT: u32 = 3577;
const MEAT_SATIETY: Duration = Duration::from_secs(180);
const RAT_MAX_HP: u64 = 20;
const RETREAT_STEPS: usize = 64;
// Keep the Rat outside melee reach while its attack target is selected.
const CAST_TILE: TilePos = TilePos {
    x: 32489,
    y: 32169,
    floor: -7,
};
const RAT_TILE: TilePos = TilePos {
    x: C18A_RAT_POSITION.x as u16,
    y: C18A_RAT_POSITION.y as u16,
    floor: -(C18A_RAT_POSITION.floor as i8),
};
const WORDS: &str = "exevo infir flam hur";

pub(super) async fn run_packet(clients: &SeamClients<'_>, scenario: &str) {
    match verdict::observe(capture(clients, scenario)).await {
        Ok(()) => evidence(&format!(
            "SEAM_EVIDENCE e2e_capture scenario={scenario} status=CAPTURED evidence=live-client-directional-cast",
        )),
        Err(error) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario={scenario} status=FAIL evidence={error}"
        )),
    }
}

async fn capture(clients: &SeamClients<'_>, scenario: &str) -> TestResult {
    let wrapper = oteryn_dev_client::qualification_runner_identity()?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let book = native_spell_book()?;
    let spell = client_spell_journey::spell_index(book, WORDS)?;
    let definition = book.indexed(spell).ok_or("C18a served spell absent")?;
    if definition.mana != crate::spell::ManaCost::Fixed(SCORCH_MANA) || definition.soul != 0 {
        return Err("C18a Scorch must cost exactly 8 mana and 0 soul".into());
    }
    let profile = definition
        .authored
        .as_ref()
        .ok_or("C18a Scorch lacks an authored profile")?;
    let ability = profile
        .ability
        .as_ref()
        .ok_or("C18a Scorch lacks an authored ability")?;
    let matrix = &ability
        .area
        .as_ref()
        .ok_or("C18a Scorch lacks a cone")?
        .matrix;
    if matrix.north != ["xxxxx", ".xxx.", ".xxx.", "..C.."]
        || matrix.diagonal.as_deref().is_none_or(|matrix| {
            matrix != ["....x.", "...xx.", "..xxx.", ".xxxx.", "xxxxx.", ".....C"]
        })
        || !profile.dependencies.effects.iter().any(|effect| {
            effect.identity.key == "candidate:ability/spell/scorch/effect"
                && effect.operation == "damage"
                && effect.damage_type.as_deref() == Some("fire")
                && ability.effects.iter().any(|reference| {
                    reference.family == "Effect"
                        && reference.key == effect.identity.key
                        && reference.revision == effect.identity.revision
                })
        })
    {
        return Err(
            "C18a authored Scorch cone or fire effect differs from the expected profile".into(),
        );
    }
    let spawn_cells = metadata
        .spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().map(|point| TilePos {
                x: point.x,
                y: point.y,
                floor: source.floor,
            })
        })
        .collect();
    let account = if scenario == "C18a.full" {
        "NODE_BOOT_Q_C18A_FULL_ACCOUNT"
    } else {
        "NODE_BOOT_Q_C18A_ACCOUNT"
    };
    let dedicated = character_prep::dedicated_character(clients, account).await?;
    let scoped = dedicated.clients(clients);
    let clients = &scoped;
    character_prep::declare_seeded(
        scenario,
        "level 15 Sorcerer and one blank rune",
        "A real soul debit makes the fresh-admission check discriminate a reset",
    );
    let mut client = character_prep::Seeded {
        profile: "q_c18a",
        reason: "Level 15 unlocks a soul-spending conjure without a combat lock",
    }
    .connect(clients, 0xca, 0)
    .await?;
    let settled = client.actor_vitals().ok_or("C18a seeded vitals absent")?;
    if (settled.max_health, settled.max_mana, settled.soul) != (220, 300, 100) {
        return Err(format!("C18a expected level-15 Sorcerer stats: {settled:?}").into());
    }
    character_prep::grant_item(&client, 3147, 1).await?;
    character_prep::grant_item(&client, MEAT, 1).await?;
    let item_index = provisioning::item_index()?;
    // A held attack target would otherwise let the starter wand fire at the Rat
    // from two tiles away. Stow it before selecting any target.
    let wand = provisioning::item_reference(&item_index, 21348)?;
    let inventory = client.inventory().ok_or("C18a starter inventory absent")?;
    let wand_handle = inventory
        .equipment
        .iter()
        .find(|row| {
            row.slot == items::EquipmentSlot::RightHand && row.item.item_definition_ref == wand
        })
        .ok_or("C18a starter wand missing from the right hand")?
        .item
        .handle;
    let wand_count = item_count(&client, wand)?;
    let moved = client
        .move_item(&items::ItemMoveIntent {
            source: wand_handle,
            destination: items::ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("C18a first starter wand stow refused: {moved:?}").into());
    }
    let stow_deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        let inventory = client
            .inventory()
            .ok_or("C18a inventory absent after stow")?;
        if inventory
            .equipment
            .iter()
            .all(|row| row.slot != items::EquipmentSlot::RightHand)
            && item_count(&client, wand)? == wand_count + 1
        {
            break;
        }
        if tokio::time::Instant::now() >= stow_deadline {
            return Err(
                "C18a starter wand did not move from the right hand into the backpack".into(),
            );
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }

    let blank = provisioning::item_reference(&item_index, 3147)?;
    let missile = provisioning::item_reference(&item_index, 3174)?;

    let conjure = client_spell_journey::spell_index(book, "adori min vis")?;
    let conjure_definition = book
        .indexed(conjure)
        .ok_or("C18a conjure definition absent")?;
    if conjure_definition.mana != crate::spell::ManaCost::Fixed(120)
        || conjure_definition.soul != 1
        || conjure_definition.aggressive
    {
        return Err(
            "C18a soul preparation spell must cost 120 mana and one soul without a combat lock"
                .into(),
        );
    }

    if tile(client.world_spatial().actor_position)? != TOWN {
        return Err("C18a expected the initial town tile".into());
    }
    let mut clock_before = client
        .session_mut()
        .gated_snapshot(world_clock::STATE_DOMAIN_WORLD_CLOCK)
        .map(|snapshot| (snapshot.revision, snapshot.payload.clone()));
    let path = fixed_cast_route(&client, world, &spawn_cells, CAST_TILE)?;
    let (&last_step, approach) = path
        .split_last()
        .ok_or("C18a cast route has no final step")?;
    let approach_goal = approach
        .iter()
        .try_fold(TOWN, |at, &step| destination(world, at, step))
        .ok_or("C18a cast approach has an invalid landing")?;
    walk_path(
        &mut client,
        world,
        &spawn_cells,
        approach,
        approach_goal,
        None,
        clock_before.as_mut(),
    )
    .await?;
    let meat = provisioning::item_reference(&item_index, MEAT)?;
    if item_count(&client, meat)? != 1 {
        return Err("C18a expected one meat before the combat setup".into());
    }
    let meat_handle = client
        .inventory()
        .and_then(|view| {
            view.entries
                .iter()
                .find(|entry| entry.item_definition_ref == meat)
        })
        .ok_or("C18a missing meat handle")?
        .handle;
    let fed_since = tokio::time::Instant::now();
    let food_command = client.next_command_id();
    evidence(&format!(
        "c18a_food_send command={food_command} last_frame={}",
        client.last_server_sequence()
    ));
    let (eaten, exchange) =
        oteryn_dev_client::capture_qualification_exchange(client.use_item(meat_handle)).await;
    let eaten = match eaten {
        Ok(eaten) => eaten,
        Err(error) => {
            let last_sent = last_sent_command(&exchange.outbound).await;
            log_client_close(&client, &error, "food", food_command, last_sent, fed_since);
            return Err(error.into());
        }
    };
    if eaten.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("C18a first meat use refused: {eaten:?}").into());
    }
    let food_deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while item_count(&client, meat)? != 0 {
        if tokio::time::Instant::now() >= food_deadline {
            return Err("C18a first meat use did not consume exactly one".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        collect_events(&mut client, None, clock_before.as_mut())?;
    }
    // Let travel combat expire before placing the live Rat.
    let quiet_deadline = tokio::time::Instant::now() + Duration::from_secs(65);
    while client.session_mut().combat_state().ok_or("C18a missing approach combat state")?.in_fight {
        let frame = collect_events(&mut client, None, clock_before.as_mut())?;
        evidence(&format!("c18a_precast_lock_wait last_frame={} combat_frame={frame:?}", client.last_server_sequence()));
        if tokio::time::Instant::now() >= quiet_deadline {
            return Err("C18a pre-cast combat lock did not clear after travel".into());
        }
        client.service_liveness(Duration::from_secs(1)).await?;
    }
    let target = place_exact_rat(&mut client, clock_before.as_mut()).await?;
    walk_path(
        &mut client,
        world,
        &spawn_cells,
        &[last_step],
        CAST_TILE,
        None,
        clock_before.as_mut(),
    )
    .await?;
    if !world
        .base()
        .tile(CAST_TILE.x, CAST_TILE.y, CAST_TILE.floor)
        .is_some_and(|tile| !tile.protection_zone())
    {
        return Err("C18a fixed cast tile is not outdoors".into());
    }
    let here = client.world_spatial().actor_position;
    let initial_combat = *client
        .session_mut()
        .combat_state()
        .ok_or("C18a missing combat state")?;
    if initial_combat.target.is_some() {
        return Err("C18a directional cast requires no active attack target".into());
    }
    let selection = client.session_mut().attack_target(Some(&target)).await?;
    require_mechanic_acceptance(selection.status, "C18a first Rat selection")?;
    if selection.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("C18a selection refused: {selection:?}").into());
    }
    let facing = StepDirection::West;
    let intent =
        target_move::encode_target_move_intent(target_move::TargetMoveIntent::Turn(facing))
            .map_err(|error| format!("C18a turn encoding failed: {error:?}"))?;
    let mut aim = None;
    for attempt in 1..=12 {
        let rat_before = client
            .world_entities()
            .and_then(|view| view.get(&target))
            .ok_or("C18a exact Rat left visibility before re-aim")?
            .position;
        evidence(&format!(
            "c18a_reaim attempt={attempt} caster={here:?} rat={rat_before:?} target={target:?}"
        ));
        if !rat_in_scorch_cone(here, rat_before) {
            client.service_liveness(Duration::from_millis(500)).await?;
            continue;
        }
        let turn_id = client.next_command_id();
        let turn_before = client.last_server_sequence();
        let (turn, mut turn_capture) = oteryn_dev_client::capture_qualification_exchange(
            client.session_mut().gated_command(
                target_move::CAPABILITY_TARGET_MOVE_V1,
                target_move::COMMAND_TYPE_TARGET_MOVE_INTENT,
                &intent,
            )
        ).await;
        let turn = turn?;
        let turn_reply = turn.server_sequence;
        evidence(&format!("c18a_turn command={turn_id} before={turn_before} reply={turn_reply}"));
        require_mechanic_acceptance(turn.status, "C18a turn away from the selected Rat")?;
        if turn.payload != [8, 1] {
            return Err("C18a turn away from the selected Rat was refused".into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            let entities = client
                .world_entities()
                .ok_or("C18a has no post-turn entity projection")?;
            let own = entities
                .iter()
                .find(|entry| &entry.entity.identity == entities.own_identity())
                .ok_or("C18a has no own entity projection")?;
            let selected = entities
                .get(&target)
                .ok_or("C18a selected Rat generation left visibility during the turn")?;
            if own.position != here {
                return Err(format!(
                    "C18a caster moved during turn: before={here:?} after={:?}", own.position
                ).into());
            }
            if selected.position != rat_before {
                evidence(&format!(
                    "c18a_rat_moved_during_turn target={target:?} before={rat_before:?} after={:?}",
                    selected.position
                ));
            }
            if matches!(actor_state(own.detail), Some((direction, _)) if direction == facing) {
                if rat_in_scorch_cone(here, selected.position) {
                    require_unique_rat_tile(
                        entities.iter().map(|entity| (tile(entity.position).ok(), entity.detail)),
                        tile(selected.position)?,
                    )?;
                    let Some((_, health_percent)) = actor_state(selected.detail) else {
                        return Err("C18a selected Rat has no creature health".into());
                    };
                    if health_percent != 100 {
                        return Err(format!(
                            "C18a selected Rat was damaged before Scorch: health={health_percent}"
                        ).into());
                    }
                    trace_turn_frames(&turn_capture.inbound, own.entity, client.selected_capabilities()).await?;
                    aim = Some((own.entity, selected.position, health_percent));
                }
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                trace_turn_frames(&turn_capture.inbound, own.entity, client.selected_capabilities()).await?;
                return Err(format!(
                    "C18a TURN direction unobserved: command={turn_id} reply={turn_reply} last_frame={} expected={facing:?} actual={:?}",
                    client.last_server_sequence(), actor_state(own.detail)
                ).into());
            }
            let (result, slice) = oteryn_dev_client::capture_qualification_exchange(
                client.service_liveness(Duration::from_millis(20))
            ).await;
            turn_capture.inbound.extend_from_slice(&slice.inbound);
            result?;
        }
        if aim.is_some() {
            break;
        }
    }
    let (caster, rat_position, health_before) =
        aim.ok_or("C18a exact Rat did not enter the Scorch cone within 12 observations")?;
    let combat_before = *client
        .session_mut()
        .combat_state()
        .ok_or("C18a missing selected combat state")?;
    if combat_before.target != Some(target) || combat_before.modes != initial_combat.modes {
        return Err("C18a did not retain the exact selected Rat while facing away".into());
    }
    let precast_frame = collect_events(&mut client, None, clock_before.as_mut())?;
    let cast_session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    evidence(&format!(
        "c18a_precast_lock session={cast_session} last_frame={} combat_frame={precast_frame:?} state={combat_before:?}",
        client.last_server_sequence()
    ));
    let Some((_, projected_combat)) = precast_frame else {
        return Err("C18a pre-cast combat condition frame absent".into());
    };
    if projected_combat != combat_before || combat_before.in_fight {
        return Err(format!("C18a pre-cast lock must be absent: frame={precast_frame:?}").into());
    }
    let pre_cast_sequence = client.last_server_sequence();
    let before = *client.actor_vitals().ok_or("C18a caster vitals absent")?;
    let mut feedback = SpellFeedback::default();
    let command_id = client.next_command_id();
    let cast_started = tokio::time::Instant::now();
    let (report, mut exchange) = oteryn_dev_client::capture_qualification_exchange(run_combat(
        client.session_mut(),
        &mut feedback,
        CombatCommand::Cast(spell),
    ))
    .await;
    let report = report?;
    let (result_server_sequence, status, cast_payload) =
        received_cast_payload(&exchange.inbound, command_id).await?;
    require_mechanic_acceptance(status, "C18a aimed Scorch")?;
    if report != CombatReport::Cast(SpellCastDisposition::Cast)
        || feedback.text() != Some("Spell cast")
    {
        return Err(format!("C18a Scorch cast refused: {report:?}").into());
    }
    let (cast_intent, cast_send_offset) = sent_cast_intent(&exchange, command_id).await?;
    let cast_send_sequence = verify_pre_send_frames(
        &exchange.inbound[..cast_send_offset], pre_cast_sequence, before,
    ).await?;
    let vitals_deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let (vitals_sequence, vitals_payload, after) = loop {
        if let Some((sequence, payload, after)) =
            first_cast_vitals(&exchange.inbound[cast_send_offset..], cast_send_sequence, before).await?
        {
            break (sequence, payload, after);
        }
        if tokio::time::Instant::now() >= vitals_deadline {
            return Err("C18a first cast stats change absent".into());
        }
        let (result, read) = oteryn_dev_client::capture_qualification_exchange(
            client.service_liveness(Duration::from_millis(20)),
        ).await;
        result?;
        exchange.inbound.extend(read.inbound);
    };
    let cast_exchange_len = exchange.inbound.len();
    let mut observation = CastObservation {
        started: cast_started,
        preceding_sequence: cast_send_sequence,
        lock_sequence: None,
        last_combat: None,
        charge_sequence: vitals_sequence,
        regeneration: Vec::new(),
        first_unlock: None,
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let mut health = None;
    let mut damage = None;
    let (health_sequence, health_after, damage_sequence, damage_value) = loop {
        for event in client.take_events() {
            observation.record(&event);
            match event {
                SessionEvent::WorldSpatialEntities(delta)
                    if delta.server_sequence > result_server_sequence =>
                {
                    if delta.value.leave.contains(&target) {
                        return Err("C18a Rat left visibility after the cast".into());
                    }
                    for entry in delta
                        .value
                        .update
                        .iter()
                        .filter(|entry| entry.entity == target)
                    {
                        if let Some((_, health_percent)) = actor_state(entry.detail)
                            && health_percent != health_before
                            && health.replace((delta.server_sequence, health_percent)).is_some()
                        {
                            return Err("C18a observed multiple Rat health changes".into());
                        }
                    }
                }
                SessionEvent::Presentation(batch)
                    if batch.server_sequence > result_server_sequence =>
                {
                    let impact = oteryn_dev_client::qualification_scorch_impact(
                        &batch.value,
                        here,
                        caster,
                        rat_position,
                        target,
                        spell.get(),
                    );
                    let impact = match impact {
                        Ok(impact) => impact,
                        Err(error) => {
                            let sequence = batch.server_sequence;
                            let raw = received_state_payload(&exchange.inbound, sequence,
                                presentation::STATE_DOMAIN_WORLD_PRESENTATION,
                                presentation::DELTA_TYPE_WORLD_PRESENTATION_BATCH_V1).await;
                            let wire = raw.as_ref().map(|payload| {
                                presentation::decode_presentation_batch(payload)
                            });
                            evidence(&format!(
                                "c18a_scorch_rejected command={command_id} reply={result_server_sequence} frame={sequence} expected_origin={here:?} expected_rat={rat_position:?} error={error} raw_payload={raw:?} wire={wire:?} applied={:?}",
                                batch.value
                            ));
                            return Err(error.into());
                        }
                    };
                    if let Some(value) = impact
                        && damage.replace((batch.server_sequence, value)).is_some()
                    {
                        return Err("C18a observed multiple Scorch decisions for one cast".into());
                    }
                }
                _ => {}
            }
        }
        if let (Some((sequence, remaining)), Some((damage_sequence, value))) = (health, damage) {
            let loss = u64::from(health_before).checked_sub(u64::from(remaining));
            if remaining == 0 || loss != value.checked_mul(100).map(|percent| percent / RAT_MAX_HP)
            {
                return Err(format!("C18a fire damage {value} disagrees with Rat health {health_before} -> {remaining}").into());
            }
            break (sequence, remaining, damage_sequence, value);
        }
        if tokio::time::Instant::now() >= deadline {
            log_damage_diagnostic(&mut client, target, result_server_sequence, damage).await;
            return Err(format!(
                "C18a missing correlated Scorch damage: command={} reply_sequence={} target={target:?} before={health_before} health={health:?} damage={damage:?}",
                command_id, result_server_sequence
            ).into());
        }
        let (pumped, read) = oteryn_dev_client::capture_qualification_exchange(
            client.service_liveness(Duration::from_millis(20)),
        )
        .await;
        pumped?;
        exchange.inbound.extend(read.inbound);
    };
    let lock_deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let combat_after = loop {
        let state = *client.session_mut().combat_state()
            .ok_or("C18a missing post-cast combat state")?;
        evidence(&format!(
            "c18a_postcast_lock session={cast_session} command={command_id} reply={result_server_sequence} damage_frame={damage_sequence} last_frame={} lock_frame={:?} state={state:?}",
            client.last_server_sequence(), observation.lock_sequence,
        ));
        if state.target != Some(target) || state.modes != combat_before.modes {
            return Err("C18a aimed cast changed the attack target or fight modes".into());
        }
        if state.in_fight && observation.lock_sequence.is_some() {
            break state;
        }
        if tokio::time::Instant::now() >= lock_deadline {
            return Err(format!(
                "C18a post-cast lock absent: session={cast_session} command={command_id} reply={result_server_sequence} damage_frame={damage_sequence} last_frame={} combat_frame={:?} state={state:?}",
                client.last_server_sequence(), observation.last_combat,
            ).into());
        }
        let (result, read) = oteryn_dev_client::capture_qualification_exchange(
            client.service_liveness(Duration::from_millis(100)),
        ).await;
        result?;
        exchange.inbound.extend(read.inbound);
        collect_events(&mut client, Some(&mut observation), None)?;
    };
    evidence(&format!(
        "SEAM_EVIDENCE c18a_health command={} reply_sequence={} delta_sequence={health_sequence} target={target:?} before={health_before} after={health_after} damage={damage_value}",
        command_id, result_server_sequence
    ));
    let session = client.join_snapshot().game_session_id;
    let capabilities = client.selected_capabilities().to_vec();
    let health_payload = client
        .session_mut()
        .raw_world_entity_deltas()
        .iter()
        .find(|delta| delta.server_sequence == health_sequence)
        .ok_or("C18a original health delta payload absent")?
        .payload
        .clone();
    let damage_payload = received_state_payload(&exchange.inbound, damage_sequence,
        presentation::STATE_DOMAIN_WORLD_PRESENTATION,
        presentation::DELTA_TYPE_WORLD_PRESENTATION_BATCH_V1).await?;
    let lock_sequence = observation.lock_sequence.ok_or("C18a missing combat lock sequence")?;
    let lock_payload = received_state_payload(&exchange.inbound, lock_sequence,
        attack::STATE_DOMAIN_ACTOR_COMBAT_STATE, attack::DELTA_TYPE_ACTOR_COMBAT_STATE_V1).await?;

    let clear = client.session_mut().attack_target(None).await?;
    require_mechanic_acceptance(clear.status, "C18a first target clear")?;
    if clear.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("C18a target clear refused: {clear:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.target.is_some())
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("C18a target clear did not remove the selected Rat".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        collect_events(&mut client, Some(&mut observation), None)?;
    }

    let cleared_combat = *client
        .session_mut()
        .combat_state()
        .ok_or("C18a cleared combat absent")?;
    if cleared_combat.modes != combat_before.modes {
        return Err("C18a clearing the Rat changed fight modes".into());
    }
    evidence(&format!(
        "SEAM_EVIDENCE c18a_target_cleared command={} reply_sequence={}",
        clear.command_id, clear.result_server_sequence
    ));

    // Use a fixed prefix of the return route. The caster stays outdoors until
    // the ordinary combat lock clears.
    let retreat = route_avoiding(
        world,
        CAST_TILE,
        |at| at == TOWN,
        |at| !spawn_cells.contains(&at),
    )
    .ok_or("C18a fixed retreat route unavailable")?;
    let retreat = retreat
        .get(..RETREAT_STEPS)
        .ok_or("C18a retreat route too short")?;
    let rest = retreat
        .iter()
        .try_fold(CAST_TILE, |from, &direction| {
            destination(world, from, direction).filter(|at| {
                world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| !tile.protection_zone())
            })
        })
        .ok_or("C18a fixed retreat crosses a protection tile")?;
    walk_path(
        &mut client,
        world,
        &spawn_cells,
        retreat,
        rest,
        Some(&mut observation),
        None,
    )
    .await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let state = client
            .session_mut()
            .combat_state()
            .ok_or("C18a missing outdoor combat state")?;
        if !state.in_fight {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "C18a combat lock did not clear at the fixed outdoor tile {rest:?}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(200)).await?;
        collect_events(&mut client, Some(&mut observation), None)?;
        verdict::Harness::global().progress("wait-for-combat-lock");
    }
    require_combat_lock_duration(observation.first_unlock)?;
    evidence(&format!(
        "SEAM_EVIDENCE c18a_combat_lock first_unlock={:?}",
        observation.first_unlock
    ));
    if fed_since.elapsed() >= MEAT_SATIETY {
        return Err("C18a food regeneration window expired before conjure".into());
    }
    let regeneration = observation.regeneration;
    let before_conjure = *client
        .actor_vitals()
        .ok_or("C18a pre-conjure vitals absent")?;
    let conjure_sequence = client.last_server_sequence();
    if !oteryn_dev_client::qualification_mana_history(
        (vitals_sequence, after.mana, after.soul),
        &regeneration,
        after.max_mana,
        (
            conjure_sequence.saturating_add(1),
            before_conjure.mana,
            before_conjure.soul,
        ),
    ) {
        return Err(
            format!("C18a unexplained mana or soul before conjure: {regeneration:?}").into(),
        );
    }
    let blank_before = item_count(&client, blank)?;
    let missile_before = item_count(&client, missile)?;
    let blank_after = blank_before
        .checked_sub(1)
        .ok_or("C18a prepared blank rune absent")?;
    let missile_after = missile_before
        .checked_add(10)
        .ok_or("C18a missile count overflow")?;
    let paid = client
        .cast_spell(conjure, actor_spell::SpellTarget::None, false)
        .await?;
    require_mechanic_acceptance(paid.status, "C18a first soul-spending conjure")?;
    let conjure_debit = paid
        .actor_vitals_delta
        .ok_or("C18a conjure has no authoritative debit")?;
    if paid.disposition != SpellCastDisposition::Cast
        || before_conjure.mana.checked_sub(120) != Some(conjure_debit.value.mana)
        || before_conjure.soul.checked_sub(1) != Some(conjure_debit.value.soul)
    {
        return Err(format!("C18a conjure did not charge 120 mana and one soul: {paid_disposition:?}: {conjure_debit:?}", paid_disposition=paid.disposition).into());
    }
    let inventory_deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while item_count(&client, blank)? != blank_after
        || item_count(&client, missile)? != missile_after
    {
        if tokio::time::Instant::now() >= inventory_deadline {
            return Err("C18a soul preparation did not consume one blank rune and produce ten missile charges".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
    let (result, detail, _, _) = logout_once(&mut client).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("C18a first logout refused: {detail}").into());
    }
    let logout_vitals = *client
        .actor_vitals()
        .ok_or("C18a missing final logout vitals")?;
    let logout_sequence = client.last_server_sequence();
    if (logout_vitals.mana, logout_vitals.soul)
        != (conjure_debit.value.mana, conjure_debit.value.soul)
    {
        return Err("C18a logout state differs from the authoritative conjure debit".into());
    }

    drop(client);
    let mut fresh = connect_character(clients, 0xcb, 0).await?;
    if fresh.join_snapshot().game_session_id == session {
        return Err("C18a relog reused the cast session".into());
    }
    if tile(fresh.world_spatial().actor_position)? != rest {
        return Err("C18a relog did not retain the fixed outdoor tile".into());
    }
    if item_count(&fresh, blank)? != blank_after || item_count(&fresh, missile)? != missile_after {
        return Err("C18a conjure inventory changed across relog".into());
    }
    let fresh_vitals = *fresh.actor_vitals().ok_or("C18a missing relogged vitals")?;
    if (fresh_vitals.mana, fresh_vitals.soul) != (logout_vitals.mana, logout_vitals.soul) {
        return Err(format!("C18a relog vitals differ after Scorch: expected mana={} soul={}, observed mana={} soul={}", logout_vitals.mana, logout_vitals.soul, fresh_vitals.mana, fresh_vitals.soul).into());
    }
    if logout_vitals.mana >= logout_vitals.max_mana || logout_vitals.soul >= 100 {
        return Err(format!(
            "C18a logout mana {} equals capacity {}; a full-mana reset would pass",
            logout_vitals.mana, logout_vitals.max_mana
        )
        .into());
    }
    evidence(&format!(
        "SEAM_EVIDENCE c18a_mana before={} pre_cast_sequence={pre_cast_sequence} cast_sequence={result_server_sequence} charge={} vitals_sequence={vitals_sequence} logout={} logout_sequence={logout_sequence} relog={}",
        before.mana, after.mana, logout_vitals.mana, fresh_vitals.mana
    ));
    walk_once(&mut fresh, world, &spawn_cells, CAST_TILE, None).await?;
    let relogged = fresh
        .world_entities()
        .and_then(|entities| entities.get(&target))
        .ok_or("C18a relog did not observe the same Rat generation")?;
    if !matches!(actor_state(relogged.detail), Some((_, health_percent)) if health_percent == health_after)
    {
        return Err(format!(
            "C18a relog Rat health differs from {health_after}: {:?}",
            relogged.detail
        )
        .into());
    }

    let mut root = std::path::PathBuf::from(required("E2E_ALL_CAPTURE_DIR")?);
    if scenario == "C18a.full" {
        root.push("C18a.full");
        std::fs::create_dir(&root)?;
    }
    std::fs::write(
        root.join("c18a-exchange.bin"),
        &exchange.inbound[..cast_exchange_len],
    )?;
    std::fs::write(root.join("c18a-intent.bin"), cast_intent)?;
    use std::os::unix::fs::MetadataExt;
    let capture_dir = std::fs::metadata(&root)?;
    let session = uuid_text(session.as_bytes());
    for (name, sequence, payload) in [
        (
            "c18a-cast.json",
            result_server_sequence,
            cast_payload.as_slice(),
        ),
        (
            "c18a-vitals.json",
            vitals_sequence,
            vitals_payload.as_slice(),
        ),
        ("c18a-health.json", health_sequence, health_payload.as_ref()),
        ("c18a-lock.json", lock_sequence, lock_payload.as_slice()),
        (
            "c18a-damage.json",
            damage_sequence,
            damage_payload.as_slice(),
        ),
    ] {
        std::fs::write(
            root.join(name),
            serde_json::to_vec(&serde_json::json!({
                "command_id": command_id,
                "game_session_id": session,
                "payload": payload,
                "server_sequence": sequence,
                "status": status as u32,
            }))?,
        )?;
    }
    let mut proof = serde_json::json!({
            "game_session_id": session,
            "capture_device": capture_dir.dev(),
            "scenario": scenario,
            "capture_inode": capture_dir.ino(),
            "spell": spell.get(),
            "damage": damage_value,
            "damage_sequence": damage_sequence,
            "caster_identity": caster.identity,
            "rat_position": [rat_position.x, rat_position.y, i32::from(rat_position.floor)],
            "caster_generation": caster.generation,
            "mana_before": before.mana,
            "vitals_before": actor_spell::encode_actor_vitals(&before).map_err(|error| format!("{error:?}"))?,
            "mana_after": after.mana,
            "pre_cast_sequence": pre_cast_sequence,
            "vitals_sequence": vitals_sequence,
            "logout_sequence": logout_sequence,
            "wrapper": wrapper,
            "regeneration": regeneration,
            "conjure_sequence": conjure_sequence,
            "mana_before_conjure": before_conjure.mana,
            "soul_before_conjure": before_conjure.soul,
            "mana_logout": logout_vitals.mana,
            "soul_logout": logout_vitals.soul,
            "mana_relog": fresh_vitals.mana,
            "soul_before": before.soul,
            "soul_after": after.soul,
            "soul_relog": fresh_vitals.soul,
            "clear_sequence": clear.result_server_sequence,
            "cleared_combat": oteryn_protocol_oteryn::attack::encode_actor_combat_state(&cleared_combat).map_err(|error| format!("{error:?}"))?,
            "clock_before": clock_before,
            "combat_before": oteryn_protocol_oteryn::attack::encode_actor_combat_state(&combat_before).map_err(|error| format!("{error:?}"))?,
            "combat_after": oteryn_protocol_oteryn::attack::encode_actor_combat_state(&combat_after).map_err(|error| format!("{error:?}"))?,
            "relog_session_id": uuid_text(fresh.join_snapshot().game_session_id.as_bytes()),
            "command_id": command_id,
            "cast_sequence": result_server_sequence,
            "health_sequence": health_sequence,
            "identity": target.identity,
            "generation": target.generation,
            "before": health_before,
            "after": health_after,
            "capabilities": capabilities,
    });
    proof["cast_send_offset"] = serde_json::json!(cast_send_offset);
    std::fs::write(root.join("c18a-proof.json"), serde_json::to_vec(&proof)?)?;
    let capture_name = root
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or("C18a capture directory lacks a run name")?;
    evidence(&format!(
        "SEAM_EVIDENCE c18a_capture_binding directory={capture_name} session={session}"
    ));
    Ok(())
}

async fn trace_turn_frames(
    mut inbound: &[u8],
    caster: oteryn_dev_client::EntityRef,
    capabilities: &[u32],
) -> TestResult {
    use oteryn_protocol_oteryn::{MessageType, decode_state_delta, decode_wire_envelope};
    use oteryn_protocol_oteryn::world_spatial_entities as entities;
    while !inbound.is_empty() {
        let frame = oteryn_dev_client::native_session::read_frame(&mut inbound).await?;
        let envelope = decode_wire_envelope(&frame)?;
        let sequence = envelope.server_sequence();
        let kind = envelope.message_type();
        if kind != MessageType::StateDelta {
            evidence(&format!("c18a_turn_frame sequence={sequence} kind={kind:?}"));
            continue;
        }
        let delta = decode_state_delta(envelope.payload())?;
        evidence(&format!("c18a_turn_frame sequence={sequence} domain={} delta={}", delta.domain_id, delta.delta_type));
        if (delta.domain_id, delta.delta_type) == (
            oteryn_protocol_oteryn::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            entities::DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
        ) {
            let (changes, _, _) = entities::decode_world_spatial_entities_delta_with_sidecar(
                delta.payload, entities::EntityFields::selected(capabilities),
            ).map_err(|error| format!("C18a TURN entity frame {sequence} cannot decode: {error:?}"))?;
            let own = changes.enter.iter().chain(&changes.update)
                .find(|entry| entry.entity == caster);
            evidence(&format!(
                "c18a_turn_entity_frame sequence={sequence} caster={caster:?} own={own:?}"
            ));
        }
    }
    Ok(())
}

async fn received_cast_payload(
    mut inbound: &[u8],
    command_id: u64,
) -> TestResult<(u64, CommandStatus, Vec<u8>)> {
    use oteryn_protocol_oteryn::{MessageType, decode_command_result, decode_wire_envelope};
    while !inbound.is_empty() {
        let frame = oteryn_dev_client::native_session::read_frame(&mut inbound).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let received = decode_command_result(envelope.payload())?;
            if received.command_id != command_id {
                return Err("C18a original cast reply does not match the exchange".into());
            }
            return Ok((
                envelope.server_sequence(),
                received.status,
                received.payload.to_vec(),
            ));
        }
    }
    Err("C18a original cast reply absent from inbound capture".into())
}

#[derive(Debug)]
enum RatPlacementRefusal {
    Fixture(String),
    InvalidReceipt,
    ProjectionDeadline,
}

impl std::fmt::Display for RatPlacementRefusal {
    fn fmt(&self, format: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixture(reason) => {
                write!(format, "C18a_exact_rat_placement_refused reason={reason}")
            }
            Self::InvalidReceipt => format.write_str("C18a_exact_rat_placement_invalid_receipt"),
            Self::ProjectionDeadline => {
                format.write_str("C18a_exact_rat_placement_projection_deadline")
            }
        }
    }
}

impl std::error::Error for RatPlacementRefusal {}

async fn place_exact_rat(
    client: &mut oteryn_dev_client::DevClientSession,
    mut clock: Option<&mut (u64, Vec<u8>)>,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let receipt = request_spawn(session, C18A_RAT_FIXTURE.into())
        .await
        .map_err(RatPlacementRefusal::Fixture)?;
    let target = parse_spawned_entity(&receipt).map_err(|_| RatPlacementRefusal::InvalidReceipt)?;
    let fields = oteryn_protocol_oteryn::world_spatial_entities::EntityFields::selected(
        client.selected_capabilities(),
    );
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let at_fixed_tile = client
            .world_entities()
            .and_then(|view| view.get(&target))
            .is_some_and(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Creature
                    && tile(entity.position).ok() == Some(RAT_TILE)
                    && entity.display_name.is_some_and(|name| {
                        name.as_str()
                            .is_ok_and(|name| name.eq_ignore_ascii_case("rat"))
                    })
            });
        if at_fixed_tile {
            use oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_delta_with_sidecar;
            let frame = client
                .session_mut()
                .raw_world_entity_deltas()
                .iter()
                .rev()
                .find(|frame| {
                    decode_world_spatial_entities_delta_with_sidecar(&frame.payload, fields)
                        .is_ok_and(|(delta, _, _)| {
                            delta.enter.iter().chain(&delta.update).any(|entity| {
                                entity.entity == target
                                    && tile(entity.position).ok() == Some(RAT_TILE)
                            })
                        })
                });
            if let Some(frame) = frame {
                evidence(&format!(
                    "SEAM_EVIDENCE c18a_rat_placement target={target:?} frame_sequence={}",
                    frame.server_sequence
                ));
                return Ok(target);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(RatPlacementRefusal::ProjectionDeadline.into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        collect_events(client, None, clock.as_deref_mut())?;
    }
}

fn fixed_cast_route(
    client: &oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawn_cells: &std::collections::BTreeSet<TilePos>,
    goal: TilePos,
) -> TestResult<Vec<StepDirection>> {
    let start = tile(client.world_spatial().actor_position)?;
    route_avoiding(
        world,
        start,
        |at| at == goal,
        |at| !spawn_cells.contains(&at),
    )
    .ok_or_else(|| format!("C18a fixed route unavailable: {start:?} -> {goal:?}").into())
}

async fn walk_once(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawn_cells: &std::collections::BTreeSet<TilePos>,
    goal: TilePos,
    clock: Option<&mut (u64, Vec<u8>)>,
) -> TestResult {
    let path = fixed_cast_route(client, world, spawn_cells, goal)?;
    walk_path(client, world, spawn_cells, &path, goal, None, clock).await
}

async fn walk_path(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawn_cells: &std::collections::BTreeSet<TilePos>,
    path: &[StepDirection],
    goal: TilePos,
    mut vitals: Option<&mut CastObservation>,
    mut clock: Option<&mut (u64, Vec<u8>)>,
) -> TestResult {
    let started = tokio::time::Instant::now();
    let mut path = VecDeque::from(path.to_vec());
    let mut detours = 0;
    let mut index = 0;
    while let Some(direction) = path.pop_front() {
        let from = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, from, direction).ok_or("C18a route lost its planned landing")?;
        let entities = client.world_entities().ok_or("C18a walk tile occupancy unavailable")?;
        let occupied: BTreeMap<_, _> = entities.iter()
            .filter(|entity| &entity.entity.identity != entities.own_identity()
                && actor_state(entity.detail).is_some())
            .filter_map(|entity| tile(entity.position).ok().map(|at| (at, entity.entity)))
            .collect();
        if detours < 8
            && let Some(blocker) = occupied.get(&expected)
            && let Some(detour) = route_avoiding(world, from, |at| at == goal, |at| {
                !occupied.contains_key(&at)
                    && !spawn_cells.contains(&at)
                    && (vitals.is_none() || world.base().tile(at.x, at.y, at.floor)
                        .is_some_and(|tile| !tile.protection_zone()))
            })
        {
            detours += 1;
            evidence(&format!("c18a_walk_detour attempt={detours} from={from:?} blocked={expected:?} blocker={blocker:?} goal={goal:?} route={detour:?}"));
            path = detour.into();
            continue;
        }
        wait_for_walk_tile(client, expected, vitals.as_deref_mut(), clock.as_deref_mut()).await?;
        let command = client.next_command_id();
        let frame_before = client.last_server_sequence();
        let trace_boundary = matches!(command, 209 | 210);
        if trace_boundary {
            evidence(&format!(
                "c18a_walk_send command={command} step={} goal={goal:?} from={from:?} to={expected:?} direction={direction:?} last_frame={frame_before} walk_elapsed_ms={}",
                index + 1,
                started.elapsed().as_millis()
            ));
        }
        let (result, exchange) = if trace_boundary {
            let (result, exchange) =
                oteryn_dev_client::capture_qualification_exchange(client.step(direction)).await;
            (result, Some(exchange))
        } else {
            (client.step(direction).await, None)
        };
        let last_sent = match exchange {
            Some(exchange) => last_sent_command(&exchange.outbound).await,
            None => result.as_ref().ok().map(|outcome| outcome.command_id),
        };
        let outcome = match result {
            Ok(outcome) => outcome,
            Err(error) => {
                log_client_close(client, &error, "step", command, last_sent, started);
                return Err(error.into());
            }
        };
        if trace_boundary {
            evidence(&format!(
                "c18a_walk_reply command={command} sent_command={last_sent:?} result_frame={} last_frame={}",
                outcome.result_server_sequence,
                client.last_server_sequence()
            ));
        }
        require_mechanic_acceptance(outcome.status, "C18a first step attempt")?;
        let actual = tile(client.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || actual != expected
        {
            let occupants: Vec<_> = client
                .world_entities()
                .into_iter()
                .flat_map(|view| view.iter())
                .filter(|entity| tile(entity.position).ok() == Some(expected))
                .map(|entity| (entity.entity, entity.kind, entity.display_name))
                .collect();
            return Err(format!("C18a first step failed: from={from:?} direction={direction:?} expected={expected:?} actual={actual:?} outcome={outcome:?} observed_destination_occupants={occupants:?}").into());
        }
        index += 1;
        verdict::Harness::global().progress("walk");
        collect_events(client, vitals.as_deref_mut(), clock.as_deref_mut())?;
        if outcome.step_wait_ms != 0
            && let Err(error) = client
                .service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms)))
                .await
            {
                log_client_close(
                    client,
                    &error,
                    "pacing",
                    command,
                    Some(outcome.command_id),
                    started,
                );
                return Err(error.into());
        }
    }
    collect_events(client, vitals, clock)?;
    if tile(client.world_spatial().actor_position)? != goal {
        return Err("C18a fixed route did not reach its goal".into());
    }
    Ok(())
}

async fn last_sent_command(mut outbound: &[u8]) -> Option<u64> {
    use oteryn_protocol_oteryn::{MessageType, decode_wire_envelope};
    let mut last = None;
    while !outbound.is_empty() {
        let Ok(frame) = oteryn_dev_client::native_session::read_frame(&mut outbound).await else {
            break;
        };
        let Ok(envelope) = decode_wire_envelope(&frame) else {
            break;
        };
        if envelope.message_type() == MessageType::ClientCommand {
            last = Some(
                envelope
                    .client_command(envelope.connection_generation())
                    .ok()?
                    .command_id,
            );
        }
    }
    last
}

fn log_client_close(
    client: &oteryn_dev_client::DevClientSession,
    error: &oteryn_dev_client::DevClientError,
    phase: &str,
    command: u64,
    last_sent: Option<u64>,
    started: tokio::time::Instant,
) {
    use oteryn_dev_client::{DevClientError, SessionError};
    let cause = match error {
        DevClientError::Context { source, .. } => source.as_ref(),
        error => error,
    };
    let (observed_close, timeout) = match cause {
        DevClientError::Session(SessionError::Timeout(phase)) | DevClientError::Timeout(phase) => {
            ("client_deadline", Some(*phase))
        }
        DevClientError::Session(SessionError::ServerProtocolError { .. }) => {
            ("server_protocol_error", None)
        }
        DevClientError::Session(SessionError::Io(error)) | DevClientError::Io(error)
            if error.kind() == std::io::ErrorKind::UnexpectedEof =>
        {
            ("peer_eof", None)
        }
        _ => ("undetermined", None),
    };
    evidence(&format!(
        "c18a_client_close observed_close={observed_close} phase={phase} attempted_command={command} sent_command={last_sent:?} last_frame={} client_timeout={timeout:?} frame_deadline_ms=20000 phase_elapsed_ms={} error={error}",
        client.last_server_sequence(),
        started.elapsed().as_millis()
    ));
}

async fn sent_cast_intent(
    exchange: &oteryn_dev_client::QualificationExchange,
    command_id: u64,
) -> TestResult<(Vec<u8>, usize)> {
    use oteryn_protocol_oteryn::{MessageType, decode_wire_envelope};
    let mut intent = None;
    let mut outbound = exchange.outbound.as_slice();
    while !outbound.is_empty() {
        let offset = exchange.outbound.len() - outbound.len();
        let frame = oteryn_dev_client::native_session::read_frame(&mut outbound).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::ClientCommand {
            let command = envelope.client_command(envelope.connection_generation())?;
            if command.command_id != command_id
                || command.command_type != oteryn_protocol_oteryn::actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT
                || intent.is_some()
            {
                return Err("C18a captured an unexpected outbound command".into());
            }
            let &(_, inbound) = exchange.write_offsets.iter().rev()
                .find(|&&(written, _)| written <= offset)
                .ok_or("C18a cast send missing transport offset")?;
            intent = Some((command.payload.to_vec(), inbound));
        }
    }
    intent.ok_or_else(|| "C18a original outbound cast intent absent".into())
}

async fn log_damage_diagnostic(
    client: &mut oteryn_dev_client::DevClientSession,
    target: oteryn_dev_client::EntityRef,
    after_sequence: u64,
    damage: Option<(u64, u64)>,
) {
    use oteryn_protocol_oteryn::world_spatial_entities as wire;
    let fields = wire::EntityFields::selected(client.selected_capabilities());
    for frame in client.session_mut().raw_world_entity_deltas() {
        if frame.server_sequence <= after_sequence {
            continue;
        }
        let decoded =
            wire::decode_world_spatial_entities_delta_with_sidecar(&frame.payload, fields);
        let detail = decoded.as_ref().ok().and_then(|(delta, _, _)| {
            delta
                .enter
                .iter()
                .chain(&delta.update)
                .find(|entity| entity.entity == target)
        });
        evidence(&format!(
            "SEAM_EVIDENCE c18a_health_frame sequence={} target={detail:?} decoded={}",
            frame.server_sequence,
            decoded.is_ok()
        ));
    }
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let identity: String = target
        .identity
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let authority = tokio::time::timeout(
        Duration::from_secs(2),
        request_fixture_control(
            session,
            "--inspect-damage",
            format!("{identity} {}", target.generation),
        ),
    )
    .await;
    evidence(&format!(
        "SEAM_EVIDENCE c18a_damage_diagnostic presentation={damage:?} server={authority:?}"
    ));
}

async fn received_state_payload(mut inbound: &[u8], sequence: u64, domain: u32, kind: u32) -> TestResult<Vec<u8>> {
    use oteryn_protocol_oteryn::{MessageType, decode_state_delta, decode_wire_envelope};
    while !inbound.is_empty() {
        let frame = oteryn_dev_client::native_session::read_frame(&mut inbound).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta
            && envelope.server_sequence() == sequence
        {
            let delta = decode_state_delta(envelope.payload())?;
            if (delta.domain_id, delta.delta_type)
                == (domain, kind)
            {
                return Ok(delta.payload.to_vec());
            }
        }
    }
    Err(format!("C18a original state frame absent: sequence={sequence} domain={domain} delta={kind}").into())
}

async fn verify_pre_send_frames(
    mut inbound: &[u8],
    mut sequence: u64,
    vitals: actor_spell::ActorVitals,
) -> TestResult<u64> {
    use oteryn_protocol_oteryn::{MessageType, decode_state_delta, decode_wire_envelope};
    while !inbound.is_empty() {
        let frame = oteryn_dev_client::native_session::read_frame(&mut inbound).await?;
        let envelope = decode_wire_envelope(&frame)?;
        sequence = sequence.max(envelope.server_sequence());
        if envelope.message_type() != MessageType::StateDelta {
            continue;
        }
        let delta = decode_state_delta(envelope.payload())?;
        if delta.domain_id == actor_spell::STATE_DOMAIN_ACTOR_VITALS {
            let observed = actor_spell::decode_actor_vitals(delta.payload)
                .map_err(|error| format!("C18a malformed pre-send stats: {error:?}"))?;
            if observed != vitals {
                return Err(format!("C18a stats changed before cast was sent: frame={sequence} expected={vitals:?} actual={observed:?}").into());
            }
        }
        if delta.domain_id == attack::STATE_DOMAIN_ACTOR_COMBAT_STATE {
            let state = attack::decode_actor_combat_state(delta.payload)
                .map_err(|error| format!("C18a malformed pre-send combat state: {error:?}"))?;
            if state.in_fight {
                return Err(format!("C18a combat lock appeared before cast send: frame={sequence} actual={state:?}").into());
            }
        }
    }
    Ok(sequence)
}

async fn first_cast_vitals(
    mut inbound: &[u8],
    preceding_sequence: u64,
    before: actor_spell::ActorVitals,
) -> TestResult<Option<(u64, Vec<u8>, actor_spell::ActorVitals)>> {
    use oteryn_protocol_oteryn::{MessageType, decode_state_delta, decode_wire_envelope};
    while !inbound.is_empty() {
        let frame = oteryn_dev_client::native_session::read_frame(&mut inbound).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload())?;
            if delta.domain_id == actor_spell::STATE_DOMAIN_ACTOR_VITALS {
                if delta.delta_type != actor_spell::DELTA_TYPE_ACTOR_VITALS_V1 {
                    return Err("C18a unexpected stats delta type".into());
                }
                let sequence = envelope.server_sequence();
                let after = actor_spell::decode_actor_vitals(delta.payload)
                    .map_err(|error| format!("C18a malformed cast vitals: {error:?}"))?;
                if sequence <= preceding_sequence || after != before {
                    if !oteryn_dev_client::qualification_scorch_charge(preceding_sequence, sequence, before, after) {
                        return Err(format!("C18a wrong first cast stats change: pre_send={preceding_sequence} frame={sequence} before={before:?} actual={after:?}").into());
                    }
                    return Ok(Some((sequence, delta.payload.to_vec(), after)));
                }
            }
        }
    }
    Ok(None)
}

fn collect_events(
    client: &mut oteryn_dev_client::DevClientSession,
    mut history: Option<&mut CastObservation>,
    mut clock: Option<&mut (u64, Vec<u8>)>,
) -> TestResult<Option<(u64, attack::ActorCombatState)>> {
    let mut combat = None;
    for event in client.take_events() {
        if let Some(history) = history.as_deref_mut() {
            history.record(&event);
        }
        match event {
            SessionEvent::WorldClock(delta) => {
                if let Some(clock) = clock.as_deref_mut() {
                    *clock = (
                        delta.new_revision,
                        world_clock::encode_world_clock(delta.value)
                            .map_err(|error| format!("C18a invalid applied clock: {error:?}"))?,
                    );
                }
            }
            SessionEvent::ActorCombatState(delta) => {
                combat = Some((delta.server_sequence, delta.value));
            }
            _ => {}
        }
    }
    Ok(combat)
}

async fn wait_for_walk_tile(
    client: &mut oteryn_dev_client::DevClientSession,
    landing: TilePos,
    mut vitals: Option<&mut CastObservation>,
    mut clock: Option<&mut (u64, Vec<u8>)>,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let mut attempt = 0;
    loop {
        let entities = client
            .world_entities()
            .ok_or("C18a walk tile occupancy unavailable")?;
        let occupant = entities.iter().find(|entity| {
            matches!(
                entity.kind,
                oteryn_dev_client::EntityKind::Creature
                    | oteryn_dev_client::EntityKind::Npc
                    | oteryn_dev_client::EntityKind::Player
            ) && tile(entity.position).ok() == Some(landing)
        });
        let Some(occupant) = occupant else {
            return Ok(());
        };
        attempt += 1;
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        evidence(&format!(
            "c18a_walk_tile_occupied attempt={attempt} landing={landing:?} occupant={:?} remaining_ms={}",
            occupant.entity,
            remaining.as_millis()
        ));
        if remaining.is_zero() {
            return Err(format!(
                "C18a walk tile occupancy deadline: landing={landing:?} occupant={:?}",
                occupant.entity
            )
            .into());
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(250)))
            .await?;
        collect_events(client, vitals.as_deref_mut(), clock.as_deref_mut())?;
    }
}

fn item_count(
    client: &oteryn_dev_client::DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("C18a inventory absent")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == reference)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

fn rat_in_scorch_cone(
    origin: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    rat: oteryn_protocol_oteryn::world_spatial::ActorPosition,
) -> bool {
    origin.floor == rat.floor
        && matches!((rat.x - origin.x, rat.y - origin.y), (1, 0) | (2 | 3, -1..=1))
}

fn require_unique_rat_tile(
    entities: impl IntoIterator<Item = (Option<TilePos>, EntityDetail)>,
    rat_tile: TilePos,
) -> TestResult {
    let occupants = entities
        .into_iter()
        .filter(|&(position, detail)| position == Some(rat_tile) && actor_state(detail).is_some())
        .count();
    if occupants != 1 {
        return Err(
            format!("C18a expected one actor on the Rat tile, observed {occupants}").into(),
        );
    }
    Ok(())
}

struct CastObservation {
    started: tokio::time::Instant,
    preceding_sequence: u64,
    lock_sequence: Option<u64>,
    last_combat: Option<(u64, attack::ActorCombatState)>,
    charge_sequence: u64,
    regeneration: Vec<(u64, u32, u32, u64)>,
    first_unlock: Option<Duration>,
}

impl CastObservation {
    fn record(&mut self, event: &SessionEvent) {
        match event {
            SessionEvent::ActorVitals(delta) if delta.server_sequence > self.charge_sequence => {
                let elapsed_ms = self.started.elapsed().as_millis() as u64;
                self.regeneration.push((
                    delta.server_sequence,
                    delta.value.mana,
                    delta.value.soul,
                    elapsed_ms,
                ));
            }
            SessionEvent::ActorCombatState(delta) if delta.server_sequence > self.preceding_sequence => {
                self.last_combat = Some((delta.server_sequence, delta.value));
                if delta.value.in_fight {
                    self.lock_sequence.get_or_insert(delta.server_sequence);
                } else if self.lock_sequence.is_some() {
                    self.first_unlock.get_or_insert_with(|| self.started.elapsed());
                }
            }
            _ => {}
        }
    }
}

// Global outdoor monster combat applies a 60-second logout block.
fn require_combat_lock_duration(first_unlock: Option<Duration>) -> TestResult {
    if !first_unlock.is_some_and(|elapsed| elapsed >= Duration::from_secs(60)) {
        return Err(
            format!("C18a missing or early outdoor combat unlock: {first_unlock:?}").into(),
        );
    }
    Ok(())
}

fn actor_state(detail: EntityDetail) -> Option<(StepDirection, u8)> {
    match detail {
        EntityDetail::Actor {
            direction,
            health_percent,
            ..
        }
        | EntityDetail::ActorWithIcons {
            direction,
            health_percent,
            ..
        } => Some((direction, health_percent)),
        EntityDetail::Object { .. } | EntityDetail::ObjectWithAppearance { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c18a_rejects_multiple_actors_on_the_rat_tile() {
        let rat = EntityDetail::Actor {
            direction: StepDirection::West,
            appearance_ref: 1,
            health_percent: 100,
        };
        let object = EntityDetail::Object {
            item_definition_ref: 1,
            quantity: 1,
            item_handle: None,
        };
        assert!(require_unique_rat_tile([(Some(RAT_TILE), rat), (Some(RAT_TILE), object)], RAT_TILE).is_ok());
        assert!(require_unique_rat_tile([(Some(RAT_TILE), rat), (Some(RAT_TILE), rat)], RAT_TILE).is_err());
        assert!(require_unique_rat_tile([(Some(RAT_TILE), object)], RAT_TILE).is_err());
    }

    #[test]
    fn c18a_rejects_missing_or_short_combat_lock() {
        assert!(require_combat_lock_duration(None).is_err());
        assert!(require_combat_lock_duration(Some(Duration::ZERO)).is_err());
        assert!(require_combat_lock_duration(Some(Duration::from_millis(59_999))).is_err());
        assert!(require_combat_lock_duration(Some(Duration::from_secs(60))).is_ok());
    }

    #[test]
    fn c18a_actor_state_reads_direction_and_health_with_or_without_icons() -> TestResult {
        use oteryn_protocol_oteryn::world_spatial_entities::{
            NativeCreatureIcon, NativeCreatureIcons,
        };

        let icons = NativeCreatureIcons::new(&[NativeCreatureIcon {
            category: 1,
            id: 6,
            count: 1,
        }])
        .map_err(|error| format!("C18a invalid creature icon: {error:?}"))?;
        assert_eq!(
            actor_state(EntityDetail::Actor {
                direction: StepDirection::West,
                appearance_ref: 1,
                health_percent: 100,
            }),
            Some((StepDirection::West, 100)),
        );
        assert_eq!(
            actor_state(EntityDetail::ActorWithIcons {
                direction: StepDirection::East,
                appearance_ref: 1,
                health_percent: 70,
                icons,
            }),
            Some((StepDirection::East, 70)),
        );
        assert_eq!(
            actor_state(EntityDetail::Object {
                item_definition_ref: 1,
                quantity: 1,
                item_handle: None,
            }),
            None,
        );
        Ok(())
    }
}

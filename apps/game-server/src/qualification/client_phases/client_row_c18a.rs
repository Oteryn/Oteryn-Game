//! Captures the real C18a combat-state and spell-result payloads consumed by
//! `crates/session/src/domain_dispatch.rs::Session::apply_domain_delta` and
//! `crates/client-runtime/src/features/local-spell-action-bar/protocol.rs::run_cast`.

use super::*;
use oteryn_dev_client::client_runtime::spell::{SpellFeedback, cast_selected};
use oteryn_dev_client::{EntityDetail, SessionEvent};
use oteryn_protocol_oteryn::{
    actor_spell::{self, SpellCastDisposition},
    attack, target_move,
};

const WORDS: &str = "exevo infir flam hur";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::observe(capture(clients)).await {
        Ok(()) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=C18a.client status=PASS evidence=live-client-directional-cast",
        ),
        Err(error) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C18a.client status=FAIL evidence={error}"
        )),
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let book = native_spell_book()?;
    let spell = client_spell_journey::spell_index(book, WORDS)?;
    let definition = book.indexed(spell).ok_or("C18a served spell absent")?;
    let crate::spell::ManaCost::Fixed(mana) = definition.mana else {
        return Err("C18a Scorch must have a fixed mana cost".into());
    };
    let mut client = connect_character(clients, 0xca, 1).await?;
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let here = client.world_spatial().actor_position;
    let rat = {
        let mut rats = client
            .world_entities()
            .ok_or("C18a capture has no entity projection")?
            .iter()
            .filter(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Creature
                    && entry.position.floor == here.floor
                    && entry
                        .position
                        .x
                        .abs_diff(here.x)
                        .max(entry.position.y.abs_diff(here.y))
                        == 1
                    && entry.display_name.is_some_and(|name| {
                        name.as_str()
                            .is_ok_and(|name| name.eq_ignore_ascii_case("rat"))
                    })
            });
        let rat = *rats.next().ok_or("C18a capture has no adjacent Rat")?;
        if rats.next().is_some() {
            return Err("C18a capture has multiple adjacent Rats".into());
        }
        rat
    };
    let target = rat.entity;
    let attack = client.session_mut().attack_target(Some(&target)).await?;
    require_mechanic_acceptance(attack.status, "C18a first Rat selection")?;
    if attack.disposition != attack::AttackIntentDisposition::Ok {
        return Err(format!(
            "C18a first Rat selection refused: {attack:?}; caster={here:?} rat={rat:?}"
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let combat = loop {
        let combat = *client
            .session_mut()
            .combat_state()
            .ok_or("C18a capture has no combat state")?;
        if combat.target == Some(target) {
            break combat;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "C18a server did not retain selected target after {attack:?}; observed={combat:?}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    };
    let facing = if rat.position.x < here.x {
        StepDirection::East
    } else if rat.position.x > here.x {
        StepDirection::West
    } else if rat.position.y < here.y {
        StepDirection::South
    } else {
        StepDirection::North
    };
    let intent =
        target_move::encode_target_move_intent(target_move::TargetMoveIntent::Turn(facing))
            .map_err(|error| format!("C18a turn encoding failed: {error:?}"))?;
    let turn = client
        .session_mut()
        .gated_command(
            target_move::CAPABILITY_TARGET_MOVE_V1,
            target_move::COMMAND_TYPE_TARGET_MOVE_INTENT,
            &intent,
        )
        .await?;
    require_mechanic_acceptance(turn.status, "C18a turn away from the selected Rat")?;
    if turn.payload != [8, 1] {
        return Err("C18a turn away from the selected Rat was refused".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let health_before = loop {
        let entities = client
            .world_entities()
            .ok_or("C18a has no post-turn entity projection")?;
        let own = entities
            .iter()
            .find(|entry| &entry.entity.identity == entities.own_identity())
            .ok_or("C18a has no own entity projection")?;
        let selected = entities
            .iter()
            .find(|entry| entry.entity == target)
            .ok_or("C18a selected Rat generation left visibility during the turn")?;
        if own.position != here || selected.position != rat.position {
            return Err("C18a caster or selected Rat moved during the turn".into());
        }
        if matches!(own.detail, EntityDetail::Actor { direction, .. } if direction == facing) {
            let EntityDetail::Actor { health_percent, .. } = selected.detail else {
                return Err("C18a selected Rat has no creature health".into());
            };
            if health_percent == 0 {
                return Err("C18a selected Rat has zero health before the cast".into());
            }
            break health_percent;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C18a did not observe the caster facing away from the selected Rat".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    };
    client.take_events();
    if client
        .session_mut()
        .combat_state()
        .and_then(|state| state.target)
        != Some(target)
    {
        return Err("C18a cast requires the same selected Rat generation after turning".into());
    }
    let before = *client.actor_vitals().ok_or("C18a caster vitals absent")?;
    let mut feedback = SpellFeedback::default();
    let cast = cast_selected(client.session_mut(), &mut feedback, spell, true).await?;
    require_mechanic_acceptance(cast.status, "C18a aimed Scorch")?;
    if cast.disposition != SpellCastDisposition::Cast || feedback.text() != Some("Spell cast") {
        return Err(format!("C18a Scorch cast refused: {:?}", cast.disposition).into());
    }
    let after = *client
        .actor_vitals()
        .ok_or("C18a post-cast vitals absent")?;
    if before.mana.checked_sub(mana) != Some(after.mana)
        || before.soul.checked_sub(definition.soul) != Some(after.soul)
    {
        return Err("C18a Scorch did not apply its exact mana and soul costs".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    'observe: loop {
        for event in client.take_events() {
            let SessionEvent::WorldSpatialEntities(delta) = event else {
                continue;
            };
            if delta.server_sequence <= cast.result_server_sequence {
                continue;
            }
            if delta.value.leave.contains(&target) {
                return Err(format!(
                    "C18a selected Rat left visibility without a health-loss delta: command={} reply_sequence={} delta_sequence={} target={target:?}",
                    cast.command_id, cast.result_server_sequence, delta.server_sequence
                ).into());
            }
            if let Some(updated) = delta
                .value
                .update
                .iter()
                .find(|entry| entry.entity == target)
                && let EntityDetail::Actor { health_percent, .. } = updated.detail
                && health_percent < health_before
            {
                evidence(&format!(
                    "SEAM_EVIDENCE c18a_health command={} reply_sequence={} delta_sequence={} target={target:?} before={health_before} after={health_percent}",
                    cast.command_id, cast.result_server_sequence, delta.server_sequence
                ));
                break 'observe;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "C18a no WorldSpatialEntities health-loss delta after accepted Scorch: command={} reply_sequence={} target={target:?} before={health_before}",
                cast.command_id, cast.result_server_sequence
            ).into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let write = || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let root = Path::new(&root);
        std::fs::write(
            root.join("c18a-combat.bin"),
            attack::encode_actor_combat_state(&combat).map_err(|error| format!("{error:?}"))?,
        )?;
        std::fs::write(
            root.join("c18a-result.bin"),
            actor_spell::encode_spell_cast_result(SpellCastDisposition::Cast),
        )?;
        std::fs::write(
            root.join("c18a-vitals-before.bin"),
            actor_spell::encode_actor_vitals(&before).map_err(|error| format!("{error:?}"))?,
        )?;
        std::fs::write(
            root.join("c18a-vitals.bin"),
            actor_spell::encode_actor_vitals(&after).map_err(|error| format!("{error:?}"))?,
        )?;
        std::fs::write(root.join("c18a-expected.txt"), format!("{spell} Cast\n"))?;
        Ok(())
    };
    if let Err(error) = write() {
        return Err(format!("C18a capture write failed: {error}").into());
    }
    Ok(())
}

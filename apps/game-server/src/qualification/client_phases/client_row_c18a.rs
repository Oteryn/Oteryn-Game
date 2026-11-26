//! Captures the real C18a combat-state and spell-result payloads consumed by
//! `crates/session/src/domain_dispatch.rs::Session::apply_domain_delta` and
//! `crates/client-runtime/src/features/local-spell-action-bar/protocol.rs::run_cast`.

use super::*;
use oteryn_dev_client::client_runtime::{
    combat::{CombatCommand, CombatReport, run_combat},
    spell::SpellFeedback,
};
use oteryn_dev_client::{EntityDetail, SessionEvent};
use oteryn_protocol_oteryn::{
    actor_spell::{self, SpellCastDisposition},
    attack,
    presentation::{PresentationEventKind, PresentationValueKind},
    target_move,
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
    client.session_mut().attack_target(Some(&target)).await?;
    client.service_liveness(Duration::from_millis(25)).await?;
    let combat = *client
        .session_mut()
        .combat_state()
        .ok_or("C18a capture has no combat state")?;
    if combat.target != Some(target) {
        return Err("C18a server did not retain selected target".into());
    }
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
    if turn.status != CommandStatus::Accepted || turn.payload != [8, 1] {
        return Err("C18a turn away from the selected Rat was refused".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let own = loop {
        let entities = client
            .world_entities()
            .ok_or("C18a has no post-turn entity projection")?;
        let own = entities
            .iter()
            .find(|entry| &entry.entity.identity == entities.own_identity())
            .ok_or("C18a has no own entity projection")?;
        if own.position != here
            || !entities
                .iter()
                .any(|entry| entry.entity == target && entry.position == rat.position)
        {
            return Err("C18a caster or selected Rat moved during the turn".into());
        }
        if matches!(own.detail, EntityDetail::Actor { direction, .. } if direction == facing) {
            break own.entity;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C18a did not observe the caster facing away from the selected Rat".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    };
    client.take_events();
    let sequence = client.last_server_sequence();
    let before = *client.actor_vitals().ok_or("C18a caster vitals absent")?;
    let mut feedback = SpellFeedback::default();
    let report = run_combat(
        client.session_mut(),
        &mut feedback,
        CombatCommand::Cast(spell),
    )
    .await?;
    if !matches!(&report, CombatReport::Cast(SpellCastDisposition::Cast))
        || feedback.text() != Some("Spell cast")
    {
        return Err(format!("C18a Scorch cast refused: {report:?}").into());
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
    loop {
        if client.take_events().iter().any(|event| {
            let SessionEvent::Presentation(delta) = event else { return false };
            delta.server_sequence > sequence && delta.value.iter().any(|damage| {
                matches!(damage.kind, PresentationEventKind::ValueText {
                    kind: PresentationValueKind::DamageDealt, target: victim, primary, ..
                } if victim == target && primary.value > 0)
                    && delta.value.iter().any(|words| {
                        words.decision_ordinal == damage.decision_ordinal
                            && matches!(words.kind, PresentationEventKind::SpellWords { caster, .. } if caster == own)
                    })
            })
        }) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "C18a aimed Scorch did not damage the selected Rat in the spell presentation"
                    .into(),
            );
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

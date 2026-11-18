//! Captures the real C18a combat-state and spell-result payloads consumed by
//! `crates/session/src/domain_dispatch.rs::Session::apply_domain_delta` and
//! `crates/client-runtime/src/features/local-spell-action-bar/protocol.rs::run_cast`.

use super::*;
use oteryn_protocol_oteryn::{actor_spell, attack};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::observe(capture(clients)).await {
        Ok(()) => evidence("e2e_all scenario=C18a.client status=PASS evidence=accepted-directional-attack-target-cast"),
        Err(error) => evidence(&format!("e2e_all scenario=C18a.client status=FAIL evidence={error}")),
    }
}

fn aimed_cast_matches(status: CommandStatus, disposition: actor_spell::SpellCastDisposition) -> bool {
    status == CommandStatus::Accepted && disposition == actor_spell::SpellCastDisposition::Cast
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
    let mut client = connect_character(clients, 0xca, 1).await?;
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.rat",
    )
    .await?;
    let target = client
        .world_entities()
        .ok_or("C18a capture has no entity projection")?
        .iter()
        .find(|entry| entry.kind == oteryn_dev_client::EntityKind::Creature)
        .map(|entry| entry.entity)
        .ok_or("C18a capture has no visible creature")?;
    client.session_mut().attack_target(Some(&target)).await?;
    client.service_liveness(Duration::from_millis(25)).await?;
    let combat = *client
        .session_mut()
        .combat_state()
        .ok_or("C18a capture has no combat state")?;
    if combat.target != Some(target) {
        return Err("C18a server did not retain selected target".into());
    }
    let catalog = client.session_mut().spell_catalog()?.ok_or("C18a served spell catalog absent")?;
    let spell = catalog.entries.iter().find(|entry| {
        entry.available && matches!(entry.words.as_str(), "exevo vis lux" | "exevo flam hur")
    }).ok_or("C18a no available directional spell")?;
    let index = std::num::NonZeroU32::new(spell.cast_index).ok_or("C18a zero spell index")?;
    let result = client
        .cast_spell(index, actor_spell::SpellTarget::AttackTarget, true)
        .await?;
    if !aimed_cast_matches(result.status, result.disposition) {
        return Err(format!("C18a aim cast failed: {result:?}").into());
    }
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let write = || -> TestResult {
        let root = Path::new(&root);
        std::fs::write(
            root.join("c18a-combat.bin"),
            attack::encode_actor_combat_state(&combat).map_err(|error| format!("{error:?}"))?,
        )?;
        std::fs::write(
            root.join("c18a-result.bin"),
            actor_spell::encode_spell_cast_result(result.disposition),
        )?;
        std::fs::write(
            root.join("c18a-expected.txt"),
            format!("{:?}\n", result.disposition),
        )?;
        Ok(())
    };
    write().map_err(|error| verdict::context(format!("C18a capture output: {error}"), error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aim_capture_requires_a_successful_cast() {
        assert!(!aimed_cast_matches(CommandStatus::Rejected, actor_spell::SpellCastDisposition::Rejected));
        assert!(!aimed_cast_matches(CommandStatus::Accepted, actor_spell::SpellCastDisposition::TargetIllegal));
        assert!(aimed_cast_matches(CommandStatus::Accepted, actor_spell::SpellCastDisposition::Cast));
    }
}

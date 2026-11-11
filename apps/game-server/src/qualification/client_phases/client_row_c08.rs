//! C08 client replay capture: let a real poison spider poison the character, dispel it with the
//! served `exana pox` spell, and capture the ACTOR_VITALS values applied by
//! `crates/session/src/domain_dispatch.rs` through `Session::apply_state_delta` and exposed by
//! `Session::actor_vitals` in `session_access.rs`. The session owns domain 3 and does not retain
//! raw delta bytes, so each capture is the applied vitals value encoded by the production codec.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{
    ActorVitals, SpellCastDisposition, SpellTarget, encode_actor_vitals,
};
use oteryn_protocol_oteryn::actor_spell_catalog as catalogue;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn line(vitals: &ActorVitals) -> String {
    format!(
        "{} {} {} {} {} {} {}\n",
        vitals.health,
        vitals.max_health,
        vitals.mana,
        vitals.max_mana,
        vitals.soul,
        vitals.harmony,
        u8::from(vitals.serene)
    )
}

/// The next ActorVitals delta whose health is strictly below `above`.
async fn next_damage(
    client: &mut DevClientSession,
    above: u32,
    seconds: u64,
) -> TestResult<ActorVitals> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event
                && delta.value.health < above
            {
                return Ok(delta.value);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no poison damage delta below health {above}").into());
        }
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0x30).await?;
    let row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        travel_to_spawn(
            &mut client,
            world,
            &metadata.spawns,
            "oteryn:creature.poison_spider",
        )
        .await?;
        let before = *client.actor_vitals().ok_or("C08 initial vitals absent")?;
        client.session_mut().attack_target(None).await?;
        client.take_events();
        let direct = next_damage(&mut client, before.health, 30).await?;
        let delayed = next_damage(&mut client, direct.health, 15).await?;

        let snapshot = client
            .session_mut()
            .gated_snapshot(catalogue::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
            .ok_or("C08 spell catalogue absent")?;
        let entry = catalogue::decode_catalog(&snapshot.payload)
            .map_err(|error| format!("C08 catalogue decode: {error:?}"))?
            .into_iter()
            .find(|entry| entry.words == "exana pox" && entry.available)
            .ok_or("C08 exana pox not served as available")?;
        let mana_before = client
            .actor_vitals()
            .ok_or("C08 pre-cast vitals absent")?
            .mana;
        let cast = client
            .cast_spell(
                std::num::NonZeroU32::new(entry.cast_index).ok_or("C08 zero cast index")?,
                SpellTarget::None,
                false,
            )
            .await?;
        if cast.disposition != SpellCastDisposition::Cast {
            return Err(format!("C08 exana pox refused: {cast:?}").into());
        }
        let dispelled = *client.actor_vitals().ok_or("C08 post-cast vitals absent")?;
        if dispelled.mana.checked_add(entry.mana) != Some(mana_before) {
            return Err(format!(
                "C08 exana pox mana {mana_before}->{} does not match cost {}",
                dispelled.mana, entry.mana
            )
            .into());
        }

        let mut expected = String::new();
        for (name, vitals) in [
            ("before", &before),
            ("direct", &direct),
            ("delayed", &delayed),
            ("dispelled", &dispelled),
        ] {
            match encode_actor_vitals(vitals) {
                Ok(payload) => capture(&format!("c08-{name}.bin"), &payload)?,
                Err(error) => return Err(format!("C08 {name} encode: {error:?}").into()),
            }
            expected.push_str(&line(vitals));
        }
        capture("c08-expected.txt", expected.as_bytes())?;
        // Leave the saved character out of combat for the phases that follow.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
        while tokio::time::Instant::now() < deadline {
            let reply = client.session_mut().logout().await?;
            if reply.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
                break;
            }
            client.service_liveness(Duration::from_millis(250)).await?;
            client.take_events();
        }
        Ok(format!(
            "health={}>{}>{} mana={}>{}",
            before.health, direct.health, delayed.health, mana_before, dispelled.mana
        ))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        row_logout(&mut client).await?;
    }
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=C08.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C08.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=C08.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vitals_text_preserves_boundaries_and_boolean() {
        assert_eq!(
            line(&ActorVitals {
                health: 0,
                max_health: u32::MAX,
                mana: 1,
                max_mana: 2,
                soul: 3,
                harmony: 4,
                serene: true
            }),
            "0 4294967295 1 2 3 4 1\n"
        );
    }
}

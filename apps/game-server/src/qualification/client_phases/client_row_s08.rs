//! S08 client replay capture: an unjustified player attack gives the attacker a white skull. Two
//! admitted players (Character one attacks Character zero) meet outside protection zones, wait out
//! the admission immunity, the attacker selects the Red Fist expert mode (the server answers with
//! an EXPERT_FIGHT_MODES delta) and attacks, and the victim's PARTY SKULL query returns the raw
//! icon payload that lists the attacker with a white skull. That payload is written byte for byte
//! (`s08-skull.bin`); `s08-expected.txt` holds the attacker's entity (identity hex and generation)
//! and the skull value. The attacker's pz lock is waited out before logout.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{
    attack as attacks, party_skull, pvp_expert, world_spatial_entities as spatial,
};

fn entity_of(client: &DevClientSession) -> TestResult<spatial::EntityRef> {
    let entities = client.world_entities().ok_or("own entities absent")?;
    entities
        .iter()
        .find(|e| e.entity.identity == *entities.own_identity())
        .map(|e| e.entity)
        .ok_or_else(|| "own entity absent".into())
}

async fn poll(a: &mut DevClientSession, b: &mut DevClientSession) -> TestResult {
    let (x, y) = join(
        a.service_liveness(Duration::from_millis(100)),
        b.service_liveness(Duration::from_millis(100)),
    )
    .await;
    x?;
    y?;
    a.take_events();
    b.take_events();
    Ok(())
}

async fn observe(clients: &SeamClients<'_>) -> TestResult<(Vec<u8>, String)> {
    flow(clients).await
}

async fn flow(clients: &SeamClients<'_>) -> TestResult<(Vec<u8>, String)> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut victim = connect_character(clients, 0x67, 0).await?;
    let mut attacker = connect_character(clients, 0x68, 1).await?;
    let open = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|t| !t.protection_zone())
    };
    let path = route(world, tile(attacker.world_spatial().actor_position)?, open)
        .ok_or("step=walk no outside-PZ tile for the attacker")?;
    for direction in path {
        let (moved, serviced) = join(
            walk(&mut attacker, world, &[direction]),
            victim.service_liveness(Duration::from_millis(10)),
        )
        .await;
        moved?;
        serviced?;
        attacker.take_events();
        victim.take_events();
    }
    let spot = tile(attacker.world_spatial().actor_position)?;
    let path = route(world, tile(victim.world_spatial().actor_position)?, |at| {
        at != spot
            && at.floor == spot.floor
            && at.x.abs_diff(spot.x).max(at.y.abs_diff(spot.y)) <= 1
            && open(at)
    })
    .ok_or("step=walk no outside-PZ tile beside the attacker")?;
    for direction in path {
        let (moved, serviced) = join(
            walk(&mut victim, world, &[direction]),
            attacker.service_liveness(Duration::from_millis(10)),
        )
        .await;
        moved?;
        serviced?;
        attacker.take_events();
        victim.take_events();
    }
    // The natural admission immunity must elapse while both sessions are serviced.
    let until = tokio::time::Instant::now() + Duration::from_secs(11);
    while tokio::time::Instant::now() < until {
        poll(&mut attacker, &mut victim).await?;
    }
    let attacker_ref = entity_of(&attacker)?;
    let victim_ref = entity_of(&victim)?;
    let expected = pvp_expert::ExpertFightModes {
        modes: attacks::FightModes {
            secure: false,
            ..attacks::FightModes::DEFAULT
        },
        expert: pvp_expert::ExpertMode::RedFist,
    };
    let reply = attacker
        .session_mut()
        .gated_command(
            pvp_expert::CAPABILITY_PVP_EXPERT_V1,
            pvp_expert::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
            &pvp_expert::encode_expert_fight_modes(&expected),
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=expert refused {reply:?}").into());
    }
    let delta = attacker
        .session_mut()
        .read_gated_delta(pvp_expert::STATE_DOMAIN_EXPERT_FIGHT_MODES, 1)
        .await?;
    if pvp_expert::decode_expert_fight_modes(&delta.payload).ok() != Some(expected) {
        return Err("step=expert the served mode differs from Red Fist".into());
    }
    let health = victim.actor_vitals().map(|v| v.health).ok_or("victim vitals absent")?;
    let attack = attacker.session_mut().attack_target(Some(&victim_ref)).await?;
    if attack.disposition != attacks::AttackIntentDisposition::Ok {
        return Err(format!("step=attack {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        poll(&mut attacker, &mut victim).await?;
        if victim.actor_vitals().is_some_and(|v| v.health < health) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=damage the victim took no damage".into());
        }
    }
    let reply = victim
        .session_mut()
        .gated_command(
            party_skull::CAPABILITY_PARTY_SKULL_V1,
            party_skull::COMMAND_TYPE_PARTY_SKULL_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=skull refused {reply:?}").into());
    }
    let icon = party_skull::decode_party_icons(&reply.payload)
        .map_err(|e| format!("skull codec {e:?}"))?
        .into_iter()
        .find(|icon| icon.entity == attacker_ref)
        .ok_or("step=skull the attacker is absent from the icons")?;
    if icon.skull != 3 {
        return Err(format!("step=skull attacker skull {} is not white", icon.skull).into());
    }
    let expected = format!(
        "identity={}\ngeneration={}\nskull={}\n",
        attacker_ref
            .identity
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        attacker_ref.generation,
        icon.skull
    );
    // Wait out the attacker's pz lock so later rows can log out.
    let release = tokio::time::Instant::now() + Duration::from_secs(70);
    while tokio::time::Instant::now() < release {
        poll(&mut attacker, &mut victim).await?;
        if let Ok(outcome) = attacker.session_mut().logout().await
            && outcome.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            break;
        }
    }
    let _ = victim.session_mut().logout().await;
    Ok((reply.payload, expected))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match verdict::observe(observe(clients)).await {
        Ok((payload, expected)) => {
            if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
                let root = Path::new(&root);
                for (name, bytes) in [
                    ("s08-skull.bin", payload.as_slice()),
                    ("s08-expected.txt", expected.as_bytes()),
                ] {
                    if let Err(error) = std::fs::write(root.join(name), bytes) {
                        evidence(&format!("S08_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
                    }
                }
            }
            evidence(&format!(
                "e2e_all scenario=S08.client status=CAPTURED evidence=skull_bytes={}",
                payload.len()
            ));
        }
        Err(error) => evidence(&format!(
            "e2e_all scenario=S08.client status=FAIL evidence={error}"
        )),
    }
}

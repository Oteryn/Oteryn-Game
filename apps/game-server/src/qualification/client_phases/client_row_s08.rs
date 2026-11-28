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
use sha2::{Digest, Sha256};

struct SkullObservation {
    icon: party_skull::PartyIcon,
    payload: Vec<u8>,
    command_id: u64,
    server_sequence: u64,
    status: CommandStatus,
}

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

async fn prepare_player(
    clients: &SeamClients<'_>,
    index: usize,
    tag: u8,
    profile: &'static str,
) -> TestResult<DevClientSession> {
    let mut client = character_prep::Seeded {
        profile,
        reason: "Open PvP minimum level",
    }
    .connect_at(clients, tag, index, "NODE_BOOT_PVP_CONTROL_SOCKET")
    .await?;
    let level = full_e2ef7::player_level(&mut client).await?;
    let vitals = client
        .actor_vitals()
        .ok_or("step=seeded-level vitals absent")?;
    if (level, vitals.max_health, vitals.max_mana) != (8, 185, 90) {
        return Err(format!(
            "step=seeded-level player={index} level={level} vitals={vitals:?} expected=8/185/90"
        )
        .into());
    }
    evidence(&format!(
        "S08_CHECK PASS step=seeded-level player={index} level={level} max_health={} max_mana={}",
        vitals.max_health, vitals.max_mana
    ));
    Ok(client)
}

async fn observe(clients: &SeamClients<'_>) -> TestResult<(Vec<u8>, String)> {
    let url = required("NODE_BOOT_PVP_DATABASE_URL")?;
    let address = required("NODE_BOOT_PVP_ADDRESS")?.parse()?;
    let world = crate::node::uuid_bytes(&required("NODE_BOOT_PVP_WORLD_ID")?)
        .ok_or("invalid Open PvP World")?;
    let channel = crate::node::uuid_bytes(&required("NODE_BOOT_PVP_CHANNEL_ID")?)
        .ok_or("invalid Open PvP Channel")?;
    let accounts = [
        required("NODE_BOOT_PVP_ACCOUNT_ID")?,
        required("NODE_BOOT_PVP_SECOND_ACCOUNT_ID")?,
    ];
    let mut database = sqlx::PgConnection::connect(&url).await?;
    let world_type: i16 = sqlx::query_scalar(
        "SELECT world_type FROM game_spell_field_world_policies
         WHERE world_id=encode($1,'hex')::uuid AND world_policy_revision='policy-s3b-1'",
    )
    .bind(world.as_slice())
    .fetch_one(&mut database)
    .await?;
    if world_type != 2 {
        return Err(format!("step=open-pvp-fixture world_type={world_type} expected=2").into());
    }
    let mut characters = [[0; 16]; 2];
    for (account, character) in accounts.iter().zip(&mut characters) {
        let found: Vec<Vec<u8>> = sqlx::query_scalar(
            "SELECT uuid_send(character_id) FROM game_character_roots
             WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid LIMIT 2",
        )
        .bind(account)
        .bind(world.as_slice())
        .fetch_all(&mut database)
        .await?;
        let [id] = found.as_slice() else {
            return Err("step=open-pvp-fixture expected exactly one Character per Account".into());
        };
        *character = id.as_slice().try_into()?;
    }
    let scope_generation: String = sqlx::query_scalar(
        "SELECT ownership_generation::text FROM game_runtime_scope_assignments
         WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1",
    )
    .bind(world.as_slice())
    .bind(channel.as_slice())
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    let scope = SeamClients {
        address,
        accounts: &accounts,
        characters: &characters,
        world,
        channel,
        scope_generation: scope_generation.parse()?,
        url: &url,
        runtime: None,
        ..*clients
    };
    flow(&scope).await
}

async fn walk_pair(
    client: &mut DevClientSession,
    peer: &mut DevClientSession,
    world: &BundleWorld,
    path: Vec<StepDirection>,
) -> TestResult {
    for direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("step=walk landing absent")?;
        let (moved, serviced) = join(
            client.step(direction),
            peer.service_liveness(Duration::from_millis(10)),
        )
        .await;
        let outcome = moved?;
        serviced?;
        let at = tile(client.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || at != expected
        {
            return Err(format!(
                "step=walk direction={direction:?} from={from:?} expected={expected:?} actual={at:?} outcome={outcome:?}"
            ).into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
        peer.take_events();
        let next_step =
            tokio::time::Instant::now() + Duration::from_millis(u64::from(outcome.step_wait_ms));
        while tokio::time::Instant::now() < next_step {
            poll(client, peer).await?;
        }
    }
    Ok(())
}

async fn flow(clients: &SeamClients<'_>) -> TestResult<(Vec<u8>, String)> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    if adventurer_forfeited(&mut database, &clients.characters[1]).await? {
        return Err("step=adventurer-protection attacker already forfeited protection".into());
    }
    let data = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let loaded = boot_world(
        data,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let world = &loaded.bundle;
    character_prep::declare_seeded(
        "S08.client",
        "two Open PvP characters at level 8",
        "ordinary leveling exceeds the client capture budget",
    );
    let mut victim = prepare_player(clients, 0, 0x67, "s08-knight").await?;
    let mut attacker = prepare_player(clients, 1, 0x68, "s08-sorcerer").await?;
    let open = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|t| !t.protection_zone())
    };
    let path = route(world, tile(attacker.world_spatial().actor_position)?, open)
        .ok_or("step=walk no outside-PZ tile for the attacker")?;
    walk_pair(&mut attacker, &mut victim, world, path).await?;
    let spot = tile(attacker.world_spatial().actor_position)?;
    let path = route_avoiding(
        world,
        tile(victim.world_spatial().actor_position)?,
        |at| {
            at != spot
                && at.floor == spot.floor
                && at.x.abs_diff(spot.x).max(at.y.abs_diff(spot.y)) <= 1
                && open(at)
        },
        |at| at != spot,
    )
    .ok_or("step=walk no outside-PZ tile beside the attacker")?;
    walk_pair(&mut victim, &mut attacker, world, path).await?;
    // The natural admission immunity must elapse while both sessions are serviced.
    let until = tokio::time::Instant::now() + Duration::from_secs(11);
    while tokio::time::Instant::now() < until {
        poll(&mut attacker, &mut victim).await?;
    }
    let attacker_ref = entity_of(&attacker)?;
    let victim_ref = entity_of(&victim)?;
    if attacker_ref == victim_ref {
        return Err("step=neutrality participants share an entity".into());
    }
    for (observer, target) in [(&mut attacker, victim_ref), (&mut victim, attacker_ref)] {
        let icon = query_icon(observer, target).await?.icon;
        if (icon.shield, icon.skull) != (0, 0) {
            return Err(format!("step=neutrality participants already marked {icon:?}").into());
        }
    }
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
    let health = victim
        .actor_vitals()
        .map(|v| v.health)
        .ok_or("victim vitals absent")?;
    let equipped = |player: &DevClientSession| {
        player.inventory().map(|inventory| {
            inventory
                .equipment
                .iter()
                .map(|equipped| (equipped.slot, equipped.item.appearance_id))
                .collect::<Vec<_>>()
        })
    };
    evidence(&format!(
        "S08_ATTACK_DIAGNOSTIC attacker={:?} victim={:?} attacker_equipment={:?} victim_equipment={:?}",
        attacker.world_spatial().actor_position,
        victim.world_spatial().actor_position,
        equipped(&attacker),
        equipped(&victim),
    ));
    let attack = attacker
        .session_mut()
        .attack_target(Some(&victim_ref))
        .await?;
    if attack.disposition != attacks::AttackIntentDisposition::Ok {
        return Err(format!("step=attack {attack:?}").into());
    }
    evidence(&format!("S08_ATTACK_DIAGNOSTIC result={attack:?}"));
    verdict::Harness::global().progress("damage");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        poll(&mut attacker, &mut victim).await?;
        if victim.actor_vitals().is_some_and(|v| v.health < health) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            let combat = attacker.session_mut().combat_state().copied();
            let vitals = victim.actor_vitals();
            evidence(&format!(
                "S08_ATTACK_DIAGNOSTIC timeout combat={combat:?} initial_health={health} final_vitals={vitals:?}"
            ));
            return Err("step=damage the victim took no damage".into());
        }
    }
    let cancelled = attacker.session_mut().attack_target(None).await?;
    if cancelled.disposition != attacks::AttackIntentDisposition::Ok {
        return Err(format!("step=cancel-attack {cancelled:?}").into());
    }
    let SkullObservation { payload, icon, command_id, server_sequence, status } =
        query_icon(&mut victim, attacker_ref).await?;
    if icon.skull != 3 {
        return Err(format!("step=skull attacker skull {} is not white", icon.skull).into());
    }
    let expected = format!(
        "identity={}\ngeneration={}\nskull={}\nsession={}\ncommand_id={command_id}\nserver_sequence={server_sequence}\ncommand_status={}\n",
        attacker_ref
            .identity
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        attacker_ref.generation,
        icon.skull,
        uuid_text(victim.join_snapshot().game_session_id.as_bytes()),
        status as u32,
    );
    release(&mut attacker, &mut victim, &mut database, clients).await?;
    let previous = attacker.join_snapshot().game_session_id;
    drop(attacker);
    let mut attacker = connect_character(clients, 0x69, 1).await?;
    if attacker.join_snapshot().game_session_id == previous
        || !adventurer_forfeited(&mut database, &clients.characters[1]).await?
    {
        return Err(
            "step=relog expected a new session with forfeited adventurer protection".into(),
        );
    }
    evidence("S08_CHECK PASS step=relog adventurer_forfeited=true");
    let relog_logout = attacker.session_mut().logout().await?;
    if (relog_logout.status, relog_logout.result)
        != (
            CommandStatus::Accepted,
            Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted),
        )
    {
        return Err(format!("step=relog-logout {relog_logout:?}").into());
    }
    let victim_logout = victim.session_mut().logout().await?;
    if victim_logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        || victim_logout.status != CommandStatus::Accepted
    {
        return Err(format!("step=victim-logout {victim_logout:?}").into());
    }
    Ok((payload, expected))
}

async fn query_icon(
    observer: &mut DevClientSession,
    target: spatial::EntityRef,
) -> TestResult<SkullObservation> {
    let reply = tokio::time::timeout(Duration::from_secs(10), async {
        const ATTEMPTS: usize = 5;
        for number in 1..=ATTEMPTS {
            let reply = observer
                .session_mut()
                .gated_command(
                    party_skull::CAPABILITY_PARTY_SKULL_V1,
                    party_skull::COMMAND_TYPE_PARTY_SKULL_QUERY,
                    &[],
                )
                .await?;
            if reply.status != CommandStatus::Rejected
                || reply.payload != oteryn_protocol_oteryn::COMMAND_BUSY_REFUSAL
            {
                return TestResult::Ok(reply);
            }
            // PARTY SKULL QUERY only reads visible icons. Its typed Busy has no state effect.
            evidence(&format!("S08_SKULL_QUERY_BUSY attempt={number} frame={reply:?}"));
            if number < ATTEMPTS {
                observer.service_liveness(Duration::from_millis(500)).await?;
            }
        }
        Err("step=skull Busy attempt limit".into())
    })
    .await
    .map_err(|_| "step=skull query deadline")??;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=skull refused {reply:?}").into());
    }
    let icon = party_skull::decode_party_icons(&reply.payload)
        .map_err(|e| format!("skull codec {e:?}"))?
        .into_iter()
        .find(|icon| icon.entity == target)
        .ok_or("step=skull the attacker is absent from the icons")?;
    Ok(SkullObservation {
        icon,
        payload: reply.payload,
        command_id: reply.command_id,
        server_sequence: reply.server_sequence,
        status: reply.status,
    })
}

async fn adventurer_forfeited(
    database: &mut sqlx::PgConnection,
    character: &[u8; 16],
) -> TestResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_character_pvp_state
         WHERE character_id=encode($1,'hex')::uuid AND adventurer_forfeited)",
    )
    .bind(character.as_slice())
    .fetch_one(database)
    .await?)
}

async fn release(
    client: &mut DevClientSession,
    peer: &mut DevClientSession,
    database: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    verdict::Harness::global().bounded_wait(deadline, "S08 combat logout lock");
    loop {
        let locked: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM game_character_pvp_state
             WHERE character_id IN (encode($1,'hex')::uuid, encode($2,'hex')::uuid)
               AND logout_block_until > clock_timestamp())",
        )
        .bind(clients.characters[0].as_slice())
        .bind(clients.characters[1].as_slice())
        .fetch_one(&mut *database)
        .await?;
        if !locked {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=logout-lock server deadline did not clear".into());
        }
        poll(client, peer).await?;
    }
    let reply = client.session_mut().logout().await?;
    if (reply.status, reply.result) != (CommandStatus::Accepted, Some(LogoutResult::Accepted)) {
        return Err(format!("step=logout {reply:?}").into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::observe(async {
        let root = std::env::var_os("NODE_BOOT_S08_CAPTURE_DIR")
            .ok_or("step=capture no capture directory")?;
        let root = Path::new(&root);
        let (payload, expected) = observe(clients).await?;
        for (name, bytes) in [
            ("s08-skull.bin", payload.as_slice()),
            ("s08-expected.txt", expected.as_bytes()),
        ] {
            use std::io::Write as _;
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(root.join(name))?
                .write_all(bytes)?;
        }
        let binding = expected.lines().skip(3).collect::<Vec<_>>().join(" ");
        let digest = Sha256::digest(&payload)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Ok::<_, Box<dyn std::error::Error>>((payload.len(), digest, binding))
    })
    .await;
    match result {
        Ok((length, digest, binding)) => evidence(&format!(
            "e2e_all scenario=S08.client status=CAPTURED evidence=skull_bytes={length} payload_sha256={digest} {binding}"
        )),
        Err(error) => evidence(&format!(
            "e2e_all scenario=S08.client status=FAIL evidence={error}"
        )),
    }
}

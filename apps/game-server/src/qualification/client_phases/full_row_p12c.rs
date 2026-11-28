//! P12c — Cyclopedia character information and recent death/PvP-kill history.
//!
//! Concrete player scenario derived from the matrix, the re-audit, and
//! `gameplay_transport/character_info.rs`:
//! 1. Admit the Open PvP World pair. On a fresh shard, prepare level eight on each
//!    character and record an actual unjustified kill before querying its history.
//! 2. Send the paged `QuerySelf(offset=1, limit=1)` player command and require an accepted,
//!    strictly decoded bounded page (the history counts may be zero at that offset).
//! 3. Send `QueryVisible` for the other provisioned character without Friends/VIP consent;
//!    require the server's refusal and no disclosed payload.
//! 4. Walk the admitted character through the compiled world to a poison-spider spawn, receive
//!    real combat damage until lethal respawn, then send `QuerySelf` again and require exactly
//!    one additional recent-death entry while the PvP-kill count remains present.
//! 5. Log out normally, obtain a fresh admission, send `QuerySelf` once more, and require the
//!    same summary, death count, and PvP-kill count, proving the histories are durable.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::character_info as info;
use oteryn_protocol_oteryn::logout::LogoutResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Page {
    level: u64,
    experience: u64,
    counters: [u64; 6],
    deaths: u16,
    pvp_kills: u16,
}

fn decode_page(bytes: &[u8], limit: u8) -> TestResult<Page> {
    if bytes.first() != Some(&0) || bytes.len() < 71 {
        return Err(format!(
            "step character-info decode: malformed header/length={}",
            bytes.len()
        )
        .into());
    }
    let mut values = [0u64; 8];
    for (index, value) in values.iter_mut().enumerate() {
        let raw = i64::from_le_bytes(bytes[1 + index * 8..9 + index * 8].try_into()?);
        *value =
            u64::try_from(raw).map_err(|_| "step character-info decode: negative summary value")?;
    }
    let deaths = u16::from_le_bytes(bytes[65..67].try_into()?);
    let pvp_kills = u16::from_le_bytes(bytes[67..69].try_into()?);
    let title_count = usize::from(bytes[69]);
    if deaths > u16::from(limit)
        || pvp_kills > u16::from(limit)
        || title_count > info::TITLE_COUNT
        || bytes.len() != 71 + title_count
    {
        return Err(format!("step character-info decode: unbounded/trailing reply deaths={deaths} kills={pvp_kills} titles={title_count} len={}", bytes.len()).into());
    }
    let earned = &bytes[70..70 + title_count];
    if earned
        .iter()
        .any(|raw| info::CharacterTitle::from_id(*raw).is_none())
        || earned
            .iter()
            .enumerate()
            .any(|(at, raw)| earned[..at].contains(raw))
    {
        return Err("step character-info decode: invalid/duplicate title".into());
    }
    let selected = bytes[70 + title_count];
    if selected != 0 && !earned.contains(&selected) {
        return Err("step character-info decode: selected title is not earned".into());
    }
    Ok(Page {
        level: values[0],
        experience: values[1],
        counters: values[2..8].try_into()?,
        deaths,
        pvp_kills,
    })
}

fn encode(operation: info::CharacterInfoOperation) -> Vec<u8> {
    let mut bytes = Vec::new();
    match operation {
        info::CharacterInfoOperation::QuerySelf(page) => {
            bytes.push(0);
            bytes.extend_from_slice(&page.offset.to_le_bytes());
            bytes.push(page.limit);
        }
        info::CharacterInfoOperation::QueryVisible { character, page } => {
            bytes.push(1);
            bytes.extend_from_slice(&character);
            bytes.extend_from_slice(&page.offset.to_le_bytes());
            bytes.push(page.limit);
        }
        _ => {}
    }
    bytes
}

async fn query(client: &mut DevClientSession, offset: u16, limit: u8) -> TestResult<Page> {
    let operation = info::CharacterInfoOperation::QuerySelf(
        info::PageRequest::new(offset, limit).ok_or("step query: invalid page")?,
    );
    let reply = client
        .session_mut()
        .gated_command(
            info::CAPABILITY_CHARACTER_INFO_V1,
            info::COMMAND_TYPE_CHARACTER_INFO,
            &encode(operation),
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step query-self: observed status={:?} payload={:?}; need accepted Character Info reply", reply.status, reply.payload).into());
    }
    decode_page(&reply.payload, limit)
}

pub(super) async fn logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now()
        + Duration::from_millis(oteryn_gameplay_combat_rules::pvp::KILL_BLOCK_MS);
    while client
        .session_mut()
        .combat_state()
        .ok_or("missing combat state before logout")?
        .in_fight
    {
        verdict::Harness::global().bounded_wait(deadline, "open-pvp-logout-lock");
        if tokio::time::Instant::now() >= deadline {
            return Err("Open PvP combat lock did not clear before logout".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    let outcome = client.session_mut().logout().await?;
    if outcome.status != CommandStatus::Accepted || outcome.result != Some(LogoutResult::Accepted) {
        return Err(format!("step logout: observed {outcome:?}; need accepted logout").into());
    }
    Ok(())
}

fn one_pve_death(before: Page, after: Page) -> bool {
    after.deaths == before.deaths.saturating_add(1) && after.pvp_kills == before.pvp_kills
}

struct OpenWorld {
    url: String,
    address: std::net::SocketAddr,
    accounts: [String; 2],
    characters: [[u8; 16]; 2],
    world: [u8; 16],
    channel: [u8; 16],
    scope_generation: u64,
}

impl OpenWorld {
    async fn read() -> TestResult<Self> {
        use sqlx::Connection as _;
        let url = required("NODE_BOOT_PVP_DATABASE_URL")?;
        let address = required("NODE_BOOT_PVP_ADDRESS")?.parse()?;
        let accounts = [
            required("NODE_BOOT_PVP_ACCOUNT_ID")?,
            required("NODE_BOOT_PVP_SECOND_ACCOUNT_ID")?,
        ];
        let world = crate::node::uuid_bytes(&required("NODE_BOOT_PVP_WORLD_ID")?)
            .ok_or("invalid Open PvP World id")?;
        let channel = crate::node::uuid_bytes(&required("NODE_BOOT_PVP_CHANNEL_ID")?)
            .ok_or("invalid Open PvP Channel id")?;
        let mut db = sqlx::PgConnection::connect(&url).await?;
        let open: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_spell_field_world_policies WHERE world_id=encode($1,'hex')::uuid AND world_policy_revision='policy-s3b-1' AND world_type=2)",
        ).bind(world.as_slice()).fetch_one(&mut db).await?;
        if !open {
            return Err("bootstrapped PvP World does not have the Open PvP policy".into());
        }
        let scope: String = sqlx::query_scalar(
            "SELECT ownership_generation::text FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1",
        ).bind(world.as_slice()).bind(channel.as_slice()).fetch_one(&mut db).await?;
        let mut characters = [[0; 16]; 2];
        for (account, character) in accounts.iter().zip(&mut characters) {
            let found: Vec<Vec<u8>> = sqlx::query_scalar(
                "SELECT uuid_send(character_id) FROM game_character_roots WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid LIMIT 2",
            ).bind(account).bind(world.as_slice()).fetch_all(&mut db).await?;
            let [id] = found.as_slice() else {
                return Err("Open PvP Account needs exactly one bootstrapped character".into());
            };
            *character = id.as_slice().try_into()?;
        }
        db.close().await?;
        Ok(Self {
            url,
            address,
            accounts,
            characters,
            world,
            channel,
            scope_generation: scope.parse()?,
        })
    }
}

async fn prepare_pvp_player(
    clients: &SeamClients<'_>,
    profile: &'static str,
    tag: u8,
    index: usize,
) -> TestResult<DevClientSession> {
    let prep = character_prep::Seeded {
        profile,
        reason: "The rat hunt exceeded the row budget. Level eight is a PvP input.",
    };
    let mut player = prep
        .connect_at(clients, tag, index, "NODE_BOOT_PVP_CONTROL_SOCKET")
        .await?;
    let page = query(&mut player, 0, 50).await?;
    let vitals = player.actor_vitals().ok_or("missing seeded PvP vitals")?;
    if (
        page.level,
        page.experience,
        vitals.max_health,
        vitals.max_mana,
    ) != (8, 4_200, 185, 90)
        || (page.deaths, page.pvp_kills) != (0, 0)
    {
        return Err(format!(
            "Open PvP profile {profile} requires level-8 derived stats and empty history: page={page:?} vitals={vitals:?}"
        )
        .into());
    }
    Ok(player)
}

async fn equip_pvp_axe(client: &mut DevClientSession, item: std::num::NonZeroU32) -> TestResult {
    use oteryn_protocol_oteryn::item_view::{EquipmentSlot, ItemMoveOutcome};
    let inventory = client.inventory().ok_or("missing attacker inventory")?;
    if !inventory
        .entries
        .iter()
        .any(|entry| entry.item_definition_ref == item)
    {
        return Err("fixture axe missing from attacker backpack".into());
    }
    let reply = client.session_mut().equip_item_definition(item).await?;
    if reply.status != CommandStatus::Accepted || reply.outcome != ItemMoveOutcome::Moved {
        return Err(format!("PvP axe equip refused: {reply:?}").into());
    }
    full_row_p05::wait_for_inventory(client, "p12c-equip-axe", |client| {
        client.inventory().is_some_and(|inventory| {
            inventory.equipment.iter().any(|entry| {
                entry.slot == EquipmentSlot::RightHand && entry.item.item_definition_ref == item
            })
        })
    })
    .await
}

async fn stow_victim_equipment(client: &mut DevClientSession) -> TestResult {
    use oteryn_dev_client::client_runtime::inventory::MoveRequest;
    use oteryn_dev_client::{ItemMoveDestination, ItemMoveOutcome};
    let equipment = client
        .inventory()
        .ok_or("missing victim inventory")?
        .equipment
        .clone();
    for entry in equipment {
        let reply = client
            .session_mut()
            .move_item_counted(MoveRequest {
                source: entry.item.handle,
                destination: ItemMoveDestination::MainBackpack,
                count: None,
            })
            .await?;
        if reply.outcome != ItemMoveOutcome::Moved {
            return Err(format!("PvP victim could not stow {:?}: {reply:?}", entry.slot).into());
        }
    }
    full_row_p05::wait_for_inventory(client, "p12c-victim-equipment", |client| {
        client
            .inventory()
            .is_some_and(|inventory| inventory.equipment.is_empty())
    })
    .await
}

async fn earn_pvp_kill(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult<DevClientSession> {
    use oteryn_protocol_oteryn::{attack, pvp_expert};
    let mut attacker = prepare_pvp_player(clients, "s08-knight", 0xca, 0).await?;
    let axe_id = 3266;
    let axe = provisioning::item_reference(&provisioning::item_index()?, axe_id)?;
    request_fixture_control_at(
        "NODE_BOOT_PVP_CONTROL_SOCKET",
        uuid_text(attacker.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        format!("item-{axe_id}-1"),
    )
    .await?;
    evidence(&format!(
        "P12C_FIXTURE role=attacker item={axe_id} count=1 purpose=level-eight-pvp-damage"
    ));
    logout(&mut attacker).await?;
    drop(attacker);
    let mut victim = prepare_pvp_player(clients, "s08-sorcerer", 0xcd, 1).await?;
    stow_victim_equipment(&mut victim).await?;
    let unprotected = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    };
    let path = route(
        world,
        tile(victim.world_spatial().actor_position)?,
        unprotected,
    )
    .ok_or("no walkable Open PvP tile for the victim")?;
    walk(&mut victim, world, &path).await?;
    let victim_tile = tile(victim.world_spatial().actor_position)?;
    let victim_identity = *victim
        .world_entities()
        .ok_or("victim visibility absent")?
        .own_identity();

    let moving = async {
        let mut attacker = connect_character(clients, 0xce, 0).await?;
        equip_pvp_axe(&mut attacker, axe).await?;
        let path = route(
            world,
            tile(attacker.world_spatial().actor_position)?,
            |at| {
                unprotected(at)
                    && at != victim_tile
                    && at.floor == victim_tile.floor
                    && at
                        .x
                        .abs_diff(victim_tile.x)
                        .max(at.y.abs_diff(victim_tile.y))
                        == 1
            },
        )
        .ok_or("no walkable adjacent Open PvP tile for the attacker")?;
        walk(&mut attacker, world, &path).await?;
        TestResult::Ok(attacker)
    };
    tokio::pin!(moving);
    let mut attacker = loop {
        tokio::select! {
            ready = &mut moving => break ready?,
            result = victim.service_liveness(Duration::from_millis(100)) => {
                result?;
                victim.take_events();
            }
        }
    };
    let before = query(&mut attacker, 0, 50).await?;
    let before_victim = query(&mut victim, 0, 50).await?;
    let modes = pvp_expert::ExpertFightModes {
        expert: pvp_expert::ExpertMode::RedFist,
        modes: attack::FightModes {
            fight_mode: attack::FightMode::Offensive,
            secure: false,
            ..attack::FightModes::DEFAULT
        },
    };
    let reply = attacker
        .session_mut()
        .gated_command(
            pvp_expert::CAPABILITY_PVP_EXPERT_V1,
            pvp_expert::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
            &pvp_expert::encode_expert_fight_modes(&modes),
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || attack::decode_attack_intent_result(&reply.payload)
            != Ok(attack::AttackIntentDisposition::Ok)
    {
        return Err(format!("Open PvP aggressive modes refused: {reply:?}").into());
    }
    let immune_until = tokio::time::Instant::now() + Duration::from_secs(11);
    while tokio::time::Instant::now() < immune_until {
        attacker
            .service_liveness(Duration::from_millis(100))
            .await?;
        victim.service_liveness(Duration::from_millis(100)).await?;
        attacker.take_events();
        victim.take_events();
    }
    let target = attacker
        .world_entities()
        .and_then(|view| {
            view.iter()
                .find(|entry| entry.entity.identity == victim_identity)
        })
        .ok_or("victim missing from attacker visibility")?
        .entity;
    let initial_health = victim
        .actor_vitals()
        .ok_or("missing pre-fight victim vitals")?
        .health;
    let mut lowest_health = initial_health;
    let attack = attacker.session_mut().attack_target(Some(&target)).await?;
    if attack.disposition != attack::AttackIntentDisposition::Ok {
        return Err(format!("Open PvP attack refused: {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    verdict::Harness::global().bounded_wait(deadline, "open-pvp-kill-credit");
    loop {
        attacker
            .service_liveness(Duration::from_millis(250))
            .await?;
        victim.service_liveness(Duration::from_millis(100)).await?;
        attacker.take_events();
        for event in victim.take_events() {
            if let SessionEvent::ActorVitals(delta) = event {
                lowest_health = lowest_health.min(delta.value.health);
            }
        }
        if lowest_health < initial_health
            && tile(victim.world_spatial().actor_position)? != victim_tile
            && victim
                .actor_vitals()
                .is_some_and(|vitals| vitals.health == vitals.max_health)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no live PvP respawn before deadline; victim-health={initial_health}->{lowest_health} victim-position={:?}", victim.world_spatial().actor_position).into());
        }
    }
    let after = query(&mut attacker, 0, 50).await?;
    let victim_page = query(&mut victim, 0, 50).await?;
    if after.pvp_kills != before.pvp_kills + 1 || victim_page.deaths != before_victim.deaths + 1 {
        return Err(format!("PvP respawn lacked exact history credit; killer-history={}->{} victim-deaths={}->{} victim-health={initial_health}->{lowest_health}", before.pvp_kills, after.pvp_kills, before_victim.deaths, victim_page.deaths).into());
    }
    attacker.session_mut().attack_target(None).await?;
    let releasing = logout(&mut victim);
    tokio::pin!(releasing);
    loop {
        tokio::select! {
            result = &mut releasing => { result?; break; }
            result = attacker.service_liveness(Duration::from_millis(100)) => {
                result?;
                attacker.take_events();
            }
        }
    }
    Ok(attacker)
}

pub(super) async fn die_to_spider(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    travel_to_spawn(client, world, spawns, "oteryn:creature.poison_spider")
        .await
        .map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            verdict::context(
                format!(
                    "step 4 walk-to-combat: observed={error}; need reachable authored lethal spawn"
                ),
                error,
            )
        })?;
    client.session_mut().attack_target(None).await?;
    let encounter = client.world_spatial().actor_position;
    let health = client
        .actor_vitals()
        .ok_or("step 4 combat: missing entry vitals")?
        .health;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut damaged = false;
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event {
                damaged |= delta.value.health < health;
            }
        }
        if damaged
            && client.world_spatial().actor_position != encounter
            && client
                .actor_vitals()
                .is_some_and(|vitals| vitals.health == vitals.max_health)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step 4 combat: timeout damaged={damaged} position={:?}; need lethal damage and respawn deltas", client.world_spatial().actor_position).into());
        }
    }
    Ok(())
}

async fn journey(core: &SeamClients<'_>) -> TestResult {
    let open = OpenWorld::read().await.map_err(|error| format!("step=open-pvp-admission need=bootstrapped-Open-PvP-World observed={error}; source:tools/qualification/node_boot/run.sh:1619"))?;
    let clients = &SeamClients {
        address: open.address,
        url: &open.url,
        accounts: &open.accounts,
        characters: &open.characters,
        world: open.world,
        channel: open.channel,
        scope_generation: open.scope_generation,
        runtime: None,
        ..*core
    };
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    // Both nodes serve this bundle. Reuse the core map for route planning and
    // keep every player connection scoped to the Open PvP node.
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&core.world)?,
        ChannelId::decode(&core.channel)?,
    )?;
    character_prep::declare_seeded(
        "P12c.full",
        "Open-PvP level-8 knight and sorcerer",
        "ordinary leveling exceeds the row budget",
    );
    let mut client = earn_pvp_kill(clients, world).await.map_err(|error| {
        verdict::context(format!("step=pvp-kill-history need=credited-unjustified-Open-PvP-kill observed={error}; source:crates/gameplay/durability/src/durability/character_title.rs:309"), error)
    })?;
    let baseline = query(&mut client, 0, 50).await?;
    if (baseline.deaths, baseline.pvp_kills) != (0, 1) {
        return Err(format!(
            "step=character-summary observed={baseline:?}; need one new PvP kill and no deaths"
        )
        .into());
    }
    let _paged = query(&mut client, 1, 1).await?;

    let visible = info::CharacterInfoOperation::QueryVisible {
        character: clients.characters[1],
        page: info::PageRequest::new(0, 1).ok_or("step refusal page")?,
    };
    let refused = client
        .session_mut()
        .gated_command(
            info::CAPABILITY_CHARACTER_INFO_V1,
            info::COMMAND_TYPE_CHARACTER_INFO,
            &encode(visible),
        )
        .await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!(
            "step 3 refusal: observed status={:?} payload={:?}; need rejected empty disclosure",
            refused.status, refused.payload
        )
        .into());
    }

    die_to_spider(&mut client, world, &metadata.spawns).await?;
    let after = query(&mut client, 0, 50).await?;
    if !one_pve_death(baseline, after) {
        return Err(format!("step 4 history delta: before={baseline:?} after={after:?}; need one death and retained PvP kill").into());
    }
    let old_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    client = connect_character(clients, 0xcb, 0).await?;
    if client.join_snapshot().game_session_id == old_session {
        return Err("step 5 fresh admission: server reused GameSession".into());
    }
    let durable = query(&mut client, 0, 50).await?;
    if durable != after {
        return Err(format!("step 5 durable recheck: before-logout={after:?} fresh={durable:?}; need identical summary/history counts").into());
    }
    logout(&mut client).await?;
    if durable.pvp_kills == 0 {
        return Err("step=pvp-kill-history observed=0 need=credited-unjustified-Open-PvP-kill-on-this-character; source:crates/gameplay/durability/src/durability/character_title.rs:309".into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(
        Duration::from_secs(1800),
        verdict::observe(journey(clients)),
    )
    .await;
    let failure = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("step=character-history-journey observed=1800s-timeout".into()),
    };
    if let Some(error) = &failure {
        evidence(&format!("P12C_GAP {error}"));
    }
    evidence(&format!(
        "e2e_all scenario=P12c.full status={} seconds={:.3}",
        if failure.is_none() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pve_history_can_advance_before_pvp_and_cannot_credit_a_frag() {
        let entry = Page {
            level: 8,
            experience: 4200,
            counters: [0; 6],
            deaths: 0,
            pvp_kills: 0,
        };
        let after = Page { deaths: 1, ..entry };
        assert!(one_pve_death(entry, after));
        assert!(!one_pve_death(
            entry,
            Page {
                pvp_kills: 1,
                ..after
            }
        ));
    }

    #[test]
    fn page_decoder_is_bounded_and_strict() -> TestResult {
        let mut bytes = vec![0];
        bytes.extend_from_slice(&8i64.to_le_bytes());
        bytes.extend_from_slice(&4_200i64.to_le_bytes());
        for value in 1..=6i64 {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&[
            1,
            info::CharacterTitle::AdeptArmorer.id(),
            info::CharacterTitle::AdeptArmorer.id(),
        ]);
        let page = decode_page(&bytes, 1)?;
        assert_eq!(
            (page.level, page.experience, page.deaths, page.pvp_kills),
            (8, 4_200, 1, 1)
        );
        for end in 0..bytes.len() {
            assert!(decode_page(&bytes[..end], 1).is_err());
        }
        bytes.push(0);
        assert!(decode_page(&bytes, 1).is_err());
        Ok(())
    }
}

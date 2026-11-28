//! C09 client replay capture: create real HP/mana deficits, consume fixture food, then
//! capture the ACTOR_VITALS payload applied by `crates/session/src/domain_dispatch.rs` through
//! `Session::apply_domain_delta` and exposed by `Session::actor_vitals` in `session_access.rs`.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::ActorVitals;
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
            .await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=C09.client status=PASS seconds={:.3} evidence=live-consumption-durable-debit-original-frame-replay",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C09.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=C09.client status=FAIL evidence=journey-timeout"),
    }
}

fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .filter(|item| item.item_definition_ref == reference)
            .map(|item| u64::from(item.count.get()))
            .sum()
    })
}

async fn replay_capture(
    character_id: oteryn_protocol_oteryn::CharacterId,
    capture: &oteryn_dev_client::InboundCapture,
    session_id: &oteryn_protocol_oteryn::GameSessionId,
    sequences: (u64, u64, u64),
    expected: (ActorVitals, ActorVitals),
) -> TestResult {
    use oteryn_dev_client::native_session::{Admission, Session};
    use std::io::Write as _;
    let root = if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        std::path::PathBuf::from(root)
    } else {
        let log = std::path::PathBuf::from(required("NODE_BOOT_CAST_EVIDENCE_LOG")?);
        log.parent()
            .ok_or("node log directory missing")?
            .to_path_buf()
    };
    let path = root.join("c09-inbound.frames");
    let bytes = capture.bytes().ok_or("C09 inbound transcript overflow")?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?
        .write_all(&bytes)?;
    let transport = tokio::io::join(
        std::io::Cursor::new(std::fs::read(path)?),
        tokio::io::sink(),
    );
    let mut replay = Session::admit(
        transport,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id,
            admission_material: b"offline-c09-replay",
            client_build_id: "oteryn-dev-client/c09-replay",
            supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(2),
        },
    )
    .await?;
    if &replay.join_snapshot().game_session_id != session_id
        || replay
            .replay_food_vitals(sequences.0, sequences.1, sequences.2)
            .await?
            != expected
    {
        return Err("C09 raw replay disagrees with the live session or vitals".into());
    }
    evidence(&format!(
        "C09_REPLAY verified before={} use={} regenerated={}",
        sequences.0, sequences.1, sequences.2
    ));
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    const ACCOUNT: &str = "NODE_BOOT_C09_ACCOUNT";
    const MEAT: u32 = 3577;
    const AXE: u32 = 3276;
    const LEVEL: u32 = 20;
    if !requires_platform_grant(&required(ACCOUNT)?) {
        return Err("C09 dedicated account lacks Platform admission".into());
    }
    let dedicated = character_prep::dedicated_character(clients, ACCOUNT).await?;
    let clients = dedicated.clients(clients);
    let character_id = oteryn_protocol_oteryn::CharacterId::decode(&clients.characters[0])?;
    let index = provisioning::item_index()?;
    let meat = provisioning::item_reference(&index, MEAT)?;
    let axe = provisioning::item_reference(&index, AXE)?;
    let mut preparation = connect_character(&clients, 0xc9, 0).await?;
    if quantity(&preparation, meat) != 0 || preparation.inventory().is_none() {
        return Err("C09 initial fixture inventory did not prove zero meat".into());
    }
    character_prep::declare_seeded(
        "C09.client",
        "level=20,meat=1,axe=1",
        "inputs for live regeneration",
    );
    let before_session = preparation.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(before_session.as_bytes()),
        "--profile",
        "c09-sorcerer".into(),
    )
    .await?;
    character_prep::grant_item(&preparation, MEAT, 1).await?;
    character_prep::grant_item(&preparation, AXE, 1).await?;
    logout_once(&mut preparation).await?;
    drop(preparation);
    let limit = std::num::NonZeroUsize::MIN.saturating_add(32 * 1024 * 1024 - 1);
    let (mut client, capture) =
        connect_character_with_capture(&clients, 0xc9, 0, Some(limit)).await?;
    let capture = capture.ok_or("C09 capture disabled")?;
    let actual = client.actor_vitals().ok_or("C09 seeded vitals absent")?;
    let (hp, mp, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer",
        LEVEL,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("C09 seeded maxima absent")?;
    let food_session = client.join_snapshot().game_session_id;
    if food_session == before_session
        || quantity(&client, meat) != 1
        || (actual.max_health, actual.max_mana) != (hp, mp)
    {
        return Err(
            "C09 fresh fixture admission did not prove level-derived vitals and one meat".into(),
        );
    }
    if super::full_row_c09::food_remaining(&mut client, None).await? != 0 {
        return Err("C09 fresh fixture started fed".into());
    }
    let axe_equipped = |client: &DevClientSession| {
        client.inventory().is_some_and(|inventory| {
            inventory
                .equipment
                .iter()
                .any(|slot| slot.item.item_definition_ref == axe && slot.item.count.get() == 1)
        })
    };
    if axe_equipped(&client) || quantity(&client, axe) != 1 {
        return Err("C09 fixture did not supply one unequipped axe".into());
    }
    let equip = client.session_mut().equip_item_definition(axe).await?;
    if equip.status != CommandStatus::Accepted
        || equip.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
    {
        return Err(format!("C09 axe equip refused: {equip:?}").into());
    }
    let equip_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !axe_equipped(&client) {
        if tokio::time::Instant::now() >= equip_deadline {
            return Err("C09 axe equip did not publish equipment".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    verdict::Harness::global().progress("c09-combat");
    use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightModes};
    let modes = FightModes {
        chase: ChaseMode::Chase,
        ..FightModes::DEFAULT
    };
    let reply = client.session_mut().set_fight_modes(&modes).await?;
    if reply.status != CommandStatus::Accepted || reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("C09 chase mode refused: {reply:?}").into());
    }
    let mode_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.modes != modes)
    {
        if tokio::time::Instant::now() >= mode_deadline {
            return Err("C09 chase mode state delta missing".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    let full = *client.actor_vitals().ok_or("C09 initial vitals absent")?;
    for _ in 0..3 {
        verdict::Harness::global().bounded_wait(
            tokio::time::Instant::now() + Duration::from_secs(125),
            "c09-combat",
        );
        client_hunt::hunt(&mut client).await.map_err(|error| {
            let combat = client.session_mut().combat_state().copied();
            let target = combat.and_then(|state| state.target).and_then(|target| {
                client.world_entities().and_then(|entities| entities.get(&target))
            });
            evidence(&format!(
                "C09_CLIENT_COMBAT_GAP observed={error} actor={:?} combat={combat:?} target={target:?}",
                client.world_spatial().actor_position,
            ));
            verdict::record_error(verdict::IndexedError::as_error(&error));
            verdict::context(format!("C09 combat: {error}"), error)
        })?;
        if client
            .actor_vitals()
            .is_some_and(|v| v.health < full.health)
        {
            break;
        }
    }
    super::full_row_c09::spend_mana(&mut client).await?;
    let before = *client.actor_vitals().ok_or("deficit vitals absent")?;
    if before.health >= full.health || before.mana >= full.mana {
        return Err(format!("HP/mana deficits missing full={full:?} before={before:?}").into());
    }
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == meat)
        })
        .map(|item| item.handle)
        .ok_or("meat handle absent")?;
    let before_meat = quantity(&client, meat);
    let baseline = client.last_server_sequence();
    let use_reply = client.use_item(handle).await?;
    if use_reply.status != CommandStatus::Accepted
        || use_reply.disposition != UseDisposition::Committed
    {
        return Err("food use was not committed".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event
                && delta.server_sequence > use_reply.result_server_sequence
                && delta.value.health > before.health
                && delta.value.mana > before.mana
            {
                if before_meat.checked_sub(1) != Some(quantity(&client, meat)) {
                    return Err("C09 captured regeneration without exactly one meat debit".into());
                }
                let remaining = super::full_row_c09::food_remaining(
                    &mut client,
                    Some(use_reply.result_server_sequence),
                )
                .await?;
                if !(1..=180_000).contains(&remaining) {
                    return Err(format!("C09 invalid live meat credit {remaining}").into());
                }
                let lock_deadline = tokio::time::Instant::now() + Duration::from_secs(60);
                loop {
                    let combat = client
                        .session_mut()
                        .combat_state()
                        .ok_or("C09 combat state absent")?;
                    if !combat.in_fight && combat.target.is_none() {
                        break;
                    }
                    if tokio::time::Instant::now() >= lock_deadline {
                        return Err("C09 combat lock did not clear before one-shot logout".into());
                    }
                    client.service_liveness(Duration::from_millis(50)).await?;
                    client.take_events();
                }
                logout_once(&mut client).await?;
                drop(client);
                let mut fresh = connect_character(&clients, 0xca, 0).await?;
                if fresh.join_snapshot().game_session_id == food_session
                    || fresh.inventory().is_none()
                    || quantity(&fresh, meat) != 0
                {
                    return Err("C09 fresh session did not retain the exact meat debit".into());
                }
                let restored = super::full_row_c09::food_remaining(&mut fresh, None).await?;
                if !(1..=remaining).contains(&restored) {
                    return Err(
                        format!("C09 durable food credit invalid {remaining}->{restored}").into(),
                    );
                }
                logout_once(&mut fresh).await?;
                replay_capture(
                    character_id,
                    &capture,
                    &food_session,
                    (
                        baseline,
                        use_reply.result_server_sequence,
                        delta.server_sequence,
                    ),
                    (before, delta.value),
                )
                .await?;
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "food regeneration timeout before={before:?} current={:?}",
                client.actor_vitals()
            )
            .into());
        }
    }
}

async fn logout_once(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C09 logout first request refused: {reply:?}").into());
    }
    Ok(())
}

#[test]
fn c09_missing_capture_directory_is_an_error() {
    let directory = Path::new(file!()).join("cannot-create-child-under-source");
    assert!(write_packet(&directory, "capture.bin", b"frame").is_err());
}
fn write_packet(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::write(root.join(name), bytes)
        .map_err(|error| format!("capture-write-error {name}: {error}").into())
}

//! I17c: hotkey hand displacement and ammunition transfer into the worn quiver.
use super::*;
use oteryn_dev_client::{DevClientSession, ItemMoveOutcome};
use oteryn_protocol_oteryn::item_view::EquipmentSlot;
use sqlx::Connection;

const SHIELD: u32 = 3412;
const BOW: u32 = 3350;
const QUIVER: u32 = 35848;
const ARROW: u32 = 3447;
const ARROWS: u32 = 10;
const LEVEL: u32 = 100;
const VOCATION: &str = "paladin";
const TAG: u8 = 0xe3;
const INPUTS: [(u32, u32); 4] = [(SHIELD, 1), (BOW, 1), (QUIVER, 1), (ARROW, ARROWS)];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(600), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I17c.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Custody {
    carried: u64,
    worn: Option<EquipmentSlot>,
}

fn custody(client: &DevClientSession, definition: std::num::NonZeroU32) -> TestResult<Custody> {
    let inventory = client.inventory().ok_or("inventory snapshot absent")?;
    Ok(Custody {
        carried: inventory
            .entries
            .iter()
            .filter(|item| item.item_definition_ref == definition)
            .map(|item| u64::from(item.count.get()))
            .sum(),
        worn: inventory
            .equipment
            .iter()
            .find(|entry| entry.item.item_definition_ref == definition)
            .map(|entry| entry.slot),
    })
}

fn reference(id: u32) -> TestResult<std::num::NonZeroU32> {
    provisioning::item_reference(&provisioning::item_index()?, id)
}

fn require_custody(
    client: &DevClientSession,
    definition: std::num::NonZeroU32,
    expected: Custody,
) -> TestResult<()> {
    let seen = custody(client, definition)?;
    if seen != expected {
        return Err(
            format!("definition={definition} observed={seen:?}; expected={expected:?}").into(),
        );
    }
    Ok(())
}

async fn await_custody(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    expected: Custody,
) -> TestResult<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while custody(client, definition)? != expected {
        if tokio::time::Instant::now() >= deadline {
            return require_custody(client, definition, expected);
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
    }
    Ok(())
}

async fn hotkey(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    expected: ItemMoveOutcome,
) -> TestResult<()> {
    let seen = oteryn_dev_client::client_runtime::hotkey_equip::equip_definition(
        client.session_mut(),
        definition,
    )
    .await?;
    if seen != expected {
        return Err(format!(
            "hotkey definition={definition} observed={seen:?}; expected={expected:?}"
        )
        .into());
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused".into());
    }
    Ok(())
}

async fn relog(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<DevClientSession> {
    let previous = client.join_snapshot().game_session_id;
    logout(client).await?;
    let fresh = connect_character(clients, TAG, 0).await?;
    if fresh.join_snapshot().game_session_id == previous {
        return Err("I17c relog reused the previous session".into());
    }
    Ok(fresh)
}

async fn assert_build(client: &mut DevClientSession) -> TestResult<()> {
    use oteryn_protocol_oteryn::{actor_spell, cyclopedia_character as general};
    let selected = general_stats::selected(client);
    let stats = client
        .session_mut()
        .gated_command(
            general::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            general::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[1, 0, 0, 1],
        )
        .await?;
    if stats.status != CommandStatus::Accepted {
        return Err("I17c GeneralStats refused".into());
    }
    let vocation = general_stats::validate(&stats.payload, selected)?;
    let level = u32::from_le_bytes(stats.payload[9..13].try_into()?);
    let vitals = client.actor_vitals().ok_or("I17c actor vitals absent")?;
    let (health, mana, _) = oteryn_gameplay_characters::vitals::maxima(
        VOCATION,
        LEVEL,
        actor_spell::MAX_VITAL_POOL,
        actor_spell::MAX_SOUL,
    )
    .ok_or("I17c seeded build has no source vitals")?;
    if vocation != general::vocation_id(VOCATION)
        || level != LEVEL
        || (vitals.max_health, vitals.max_mana) != (health, mana)
    {
        return Err(format!("I17c build observed={vocation:?}/{level}/{vitals:?}").into());
    }
    evidence(&format!(
        "I17C_CHECK build={VOCATION} level={level} max_health={health} max_mana={mana}"
    ));
    Ok(())
}

async fn prepared_character(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    evidence("row=I17c.full mode=seeded prerequisites=paladin,shield,bow,quiver,arrows");
    let mut client = connect_character(clients, TAG, 0).await?;
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "i17c-paladin".into(),
    )
    .await?;
    let mut client = relog(&mut client, clients).await?;
    assert_build(&mut client).await?;
    for (id, count) in INPUTS {
        require_custody(
            &client,
            reference(id)?,
            Custody {
                carried: u64::from(count),
                worn: None,
            },
        )?;
    }
    Ok(client)
}

// Scope the nested credit to the equipped quiver of the character driven over the wire.
async fn quiver_arrows(clients: &SeamClients<'_>) -> TestResult<i64> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let count = sqlx::query_scalar(
        "SELECT COALESCE(sum(a.quantity), 0)::bigint
         FROM game_character_equipment_slots s
         JOIN game_item_instances q ON q.item_instance_id = s.item_instance_id
         JOIN game_item_nested_entries e ON e.parent_item_instance_id = q.item_instance_id
         JOIN game_item_instances a ON a.item_instance_id = e.item_instance_id
         WHERE s.character_id = $1::uuid
           AND q.definition_production_key = $2 AND q.lifecycle = 1
           AND a.definition_production_key = $3 AND a.lifecycle = 1",
    )
    .bind(uuid_text(&clients.characters[0]))
    .bind(format!("oteryn:item.tibia.i{QUIVER}"))
    .bind(format!("oteryn:item.tibia.i{ARROW}"))
    .fetch_one(&mut connection)
    .await?;
    Ok(count)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use EquipmentSlot::{LeftHand, RightHand};
    use ItemMoveOutcome::{Moved, SlotMismatch};

    let account = required("NODE_BOOT_I17C_ACCOUNT")?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let rows: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(character_id) FROM game_character_roots
         WHERE account_id=$1::uuid AND world_id=$2::uuid LIMIT 2",
    )
    .bind(&account)
    .bind(uuid_text(&clients.world))
    .fetch_all(&mut database)
    .await?;
    database.close().await?;
    let [character] = rows.as_slice() else {
        return Err("I17c requires exactly one dedicated character in this world".into());
    };
    let characters = [character.as_slice().try_into()?];
    let accounts = [account];
    let clients = &SeamClients { accounts: &accounts, characters: &characters, ..*clients };
    let [shield, bow, quiver, arrow] = [
        reference(SHIELD)?,
        reference(BOW)?,
        reference(QUIVER)?,
        reference(ARROW)?,
    ];
    let mut client = prepared_character(clients).await?;

    let shield_carried = Custody {
        carried: 1,
        worn: None,
    };
    let off_hand = Custody {
        carried: 0,
        worn: Some(LeftHand),
    };
    let weapon_hand = Custody {
        carried: 0,
        worn: Some(RightHand),
    };
    let arrows_carried = Custody {
        carried: u64::from(ARROWS),
        worn: None,
    };
    let arrows_nested = Custody {
        carried: 0,
        worn: None,
    };

    hotkey(&mut client, shield, Moved).await?;
    await_custody(&mut client, shield, off_hand).await?;
    hotkey(&mut client, bow, Moved).await?;
    await_custody(&mut client, bow, weapon_hand).await?;
    await_custody(&mut client, shield, shield_carried).await?;

    let before = client
        .inventory()
        .ok_or("I17c pre-refusal inventory absent")?
        .clone();
    hotkey(&mut client, arrow, SlotMismatch).await?;
    if client.inventory() != Some(&before) {
        return Err("I17c ammo refusal changed inventory or equipment".into());
    }
    require_custody(&client, quiver, shield_carried)?;
    hotkey(&mut client, quiver, Moved).await?;
    await_custody(&mut client, quiver, off_hand).await?;
    require_custody(&client, bow, weapon_hand)?;
    require_custody(&client, arrow, arrows_carried)?;
    if quiver_arrows(clients).await? != 0 {
        return Err("I17c quiver already contained arrows".into());
    }

    hotkey(&mut client, arrow, Moved).await?;
    await_custody(&mut client, arrow, arrows_nested).await?;
    let credited = quiver_arrows(clients).await?;
    if credited != i64::from(ARROWS) {
        return Err(format!("I17c quiver credit observed={credited}; expected={ARROWS}").into());
    }

    let mut client = relog(&mut client, clients).await?;
    for (item, expected) in [
        (shield, shield_carried),
        (bow, weapon_hand),
        (quiver, off_hand),
        (arrow, arrows_nested),
    ] {
        require_custody(&client, item, expected)?;
    }
    let reloaded = quiver_arrows(clients).await?;
    if reloaded != credited {
        return Err(
            format!("I17c reloaded quiver observed={reloaded}; expected={credited}").into(),
        );
    }
    logout(&mut client).await?;
    Ok(format!(
        "shield-displaced;missing-quiver=SlotMismatch;quiver-arrows={reloaded};relog=verified"
    ))
}

//! Capture the protection receipt from this batch's own Open PvP death.
use super::*;
use super::{client_row_p20, full_e2ef6};
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::death_protection::{
    CAPABILITY_DEATH_PROTECTION_V1, COMMAND_TYPE_DEATH_PROTECTION_QUERY, DeathProtectionReply,
};
use std::io::Write as _;

const ROW: &str = "C20c.client";
const SHIELDING: &str = "oteryn:blessing.the_spiritual_shielding";
const TWIST: &str = "oteryn:blessing.twist_of_fate";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    character_prep::declare_seeded(
        ROW,
        "level-21-sorcerer,level-100-knight,purchase-gold",
        "isolate Twist consumption on a legal Open PvP death",
    );
    match verdict::indexed_timeout(Duration::from_secs(540), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(())) => evidence("C20c_CHECK packet-and-fresh-admission PASS"),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario={ROW} status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=C20c.client status=FAIL evidence=journey-deadline"),
    }
}

async fn journey(core: &SeamClients<'_>) -> TestResult {
    let url = required("NODE_BOOT_PVP_DATABASE_URL")?;
    let accounts = [
        required("NODE_BOOT_PVP_ACCOUNT_ID")?,
        required("NODE_BOOT_C20C_PVP_ACCOUNT")?,
    ];
    let world_id = crate::node::uuid_bytes(&required("NODE_BOOT_PVP_WORLD_ID")?)
        .ok_or("invalid Open PvP world id")?;
    let channel = crate::node::uuid_bytes(&required("NODE_BOOT_PVP_CHANNEL_ID")?)
        .ok_or("invalid Open PvP channel id")?;
    let mut db = sqlx::PgConnection::connect(&url).await?;
    let open: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_spell_field_world_policies WHERE world_id=encode($1,'hex')::uuid AND world_policy_revision='policy-s3b-1' AND world_type=2)",
    ).bind(world_id.as_slice()).fetch_one(&mut db).await?;
    if !open {
        return Err("C20c requires the Open PvP world".into());
    }
    let generation: String = sqlx::query_scalar(
        "SELECT ownership_generation::text FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1",
    ).bind(world_id.as_slice()).bind(channel.as_slice()).fetch_one(&mut db).await?;
    let mut characters = [[0; 16]; 2];
    for (account, character) in accounts.iter().zip(&mut characters) {
        let found: Vec<Vec<u8>> = sqlx::query_scalar(
            "SELECT uuid_send(character_id) FROM game_character_roots WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid LIMIT 2",
        ).bind(account).bind(world_id.as_slice()).fetch_all(&mut db).await?;
        let [id] = found.as_slice() else {
            return Err("C20c requires exactly one character per Open PvP account".into());
        };
        *character = id.as_slice().try_into()?;
    }
    db.close().await?;
    let clients = &SeamClients {
        address: required("NODE_BOOT_PVP_ADDRESS")?.parse()?,
        url: &url,
        accounts: &accounts,
        characters: &characters,
        world: world_id,
        channel,
        scope_generation: generation.parse()?,
        runtime: None,
        ..*core
    };
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&world_id)?,
        ChannelId::decode(&channel)?,
    )?;

    let socket = "NODE_BOOT_PVP_CONTROL_SOCKET";
    let mut victim = prepare_build(clients, 0xc2, 1, "c20c-sorcerer").await?;
    assert_build(&mut victim, 21, 250, 480).await?;
    let (before, _) = protection(&mut victim).await?;
    if before.0.is_some() || !full_e2ef6::blessings(&mut victim).await?.is_empty() {
        return Err("C20c dedicated character already has a death or blessings".into());
    }
    request_fixture_control_at(
        socket,
        uuid_text(victim.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "item-3043-1".into(),
    )
    .await?;
    relog(&mut victim, clients, 0xc3, 1).await?;
    buy(
        &mut victim,
        world,
        &metadata.npcs,
        "oteryn:npc.norf",
        "The Spiritual Shielding",
    )
    .await?;
    if full_e2ef6::blessings(&mut victim).await? != [SHIELDING] {
        return Err("C20c regular blessing purchase did not change ownership exactly".into());
    }
    buy(
        &mut victim,
        world,
        &metadata.npcs,
        "oteryn:npc.quentin",
        "Twist of Fate",
    )
    .await?;
    if full_e2ef6::blessings(&mut victim).await? != [SHIELDING, TWIST] {
        return Err("C20c must hold both regular and Twist protections before the kill".into());
    }

    let mut attacker = prepare_build(clients, 0xc4, 0, "knight").await?;
    assert_build(&mut attacker, 100, 1565, 550).await?;
    let (before, after) =
        full_e2ef6::drive_pvp_death(&mut victim, &mut attacker, world, &metadata.spawns, true)
            .await?;
    if after.0 >= before.0 || full_e2ef6::blessings(&mut victim).await? != [SHIELDING] {
        return Err(format!(
            "C20c PvP death did not lose XP and retain only Shielding: {before:?} -> {after:?}"
        )
        .into());
    }
    let (live, _) = protection(&mut victim).await?;
    let live = live
        .0
        .ok_or("death-protection server returned no latest death")?;
    if live.consumed != [TWIST] {
        return Err(format!(
            "C20c latest real death consumed unexpected protection: {:?}",
            live.consumed
        )
        .into());
    }
    relog(&mut victim, clients, 0xc5, 1).await?;
    let (saved, bytes) = protection(&mut victim).await?;
    if saved.0.as_ref() != Some(&live)
        || full_e2ef6::blessings(&mut victim).await? != [SHIELDING]
        || full_e2ef6::progress(&mut victim).await? != after
    {
        return Err(
            "C20c relog did not preserve the exact death receipt, blessings and progression".into(),
        );
    }
    logout(&mut victim).await?;
    let capture = required("NODE_BOOT_C20C_CAPTURE")?;
    for (name, payload) in [
        ("c20c-death-protection.bin", bytes.as_slice()),
        ("c20c-expected.txt", TWIST.as_bytes()),
    ] {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(Path::new(&capture).join(name))?
            .write_all(payload)?;
    }
    Ok(())
}

async fn prepare_build(
    clients: &SeamClients<'_>,
    tag: u8,
    index: usize,
    profile: &str,
) -> TestResult<DevClientSession> {
    let mut client = connect_character(clients, tag, index).await?;
    request_fixture_control_at(
        "NODE_BOOT_PVP_CONTROL_SOCKET",
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        profile.into(),
    )
    .await?;
    relog(&mut client, clients, tag, index).await?;
    Ok(client)
}

async fn assert_build(
    client: &mut DevClientSession,
    level: u32,
    health: u32,
    mana: u32,
) -> TestResult {
    let build = full_e2ef6::progress(client).await?;
    let vitals = client
        .actor_vitals()
        .ok_or("C20c prepared vitals missing")?;
    if build.1 != level
        || (
            vitals.health,
            vitals.max_health,
            vitals.mana,
            vitals.max_mana,
        ) != (health, health, mana, mana)
    {
        return Err(format!("C20c prepared build differs: level={level} health={health} mana={mana} observed={build:?}/{vitals:?}").into());
    }
    Ok(())
}

async fn buy(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    seller: &str,
    name: &str,
) -> TestResult {
    let npc = client_row_p20::approach_npc(client, world, npcs, seller).await?;
    client_row_p20::talk(client, npc, "hi").await?;
    let (_, quote) = client_row_p20::talk(client, npc, name).await?;
    if quote
        != [format!(
            "Do you want to buy {name} at the price for your current level?"
        )]
    {
        return Err(format!("C20c {seller} quote differs: {quote:?}").into());
    }
    let (_, confirm) = client_row_p20::talk(client, npc, "yes").await?;
    if confirm != ["You have received the blessing."] {
        return Err(format!("C20c {seller} purchase refused: {confirm:?}").into());
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let outcome = client.session_mut().logout().await?;
    if outcome.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C20c first logout refused: {outcome:?}").into());
    }
    Ok(())
}

async fn relog(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    tag: u8,
    index: usize,
) -> TestResult {
    let previous = client.join_snapshot().game_session_id;
    logout(client).await?;
    *client = connect_character(clients, tag, index).await?;
    if client.join_snapshot().game_session_id == previous {
        return Err("C20c relog reused the original session".into());
    }
    Ok(())
}

async fn protection(client: &mut DevClientSession) -> TestResult<(DeathProtectionReply, Vec<u8>)> {
    let result = client
        .session_mut()
        .gated_command(
            CAPABILITY_DEATH_PROTECTION_V1,
            COMMAND_TYPE_DEATH_PROTECTION_QUERY,
            &[],
        )
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("death-protection query refused: {:?}", result.status).into());
    }
    let reply = DeathProtectionReply::decode(&result.payload)
        .ok_or("death-protection server payload was not canonical")?;
    Ok((reply, result.payload))
}

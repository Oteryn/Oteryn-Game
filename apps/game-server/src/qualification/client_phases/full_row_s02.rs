//! S02 full-row player scenario, derived from the matrix, crystal re-audit, and
//! `gameplay_transport::{party_command,party_skull}` owners:
//!
//! 1. Admit two distinct provisioned characters, walk both from town to adjacent
//!    unprotected tiles, and query their native party/icon state; both start outside a party.
//! 2. The first player sends PARTY Invite for the visible second player.  Assert an accepted
//!    party reply/current-view delta and the observer-relative yellow/blue invitation shields.
//! 3. Through ordinary combat controls, the first player selects Open-PvP Red Fist with secure
//!    mode off and sends ATTACK for the second player.  Assert accepted mode and attack replies,
//!    a victim-health delta, and the victim's observer-relative white skull for the aggressor.
//! 4. While that live aggression exists, the invited player sends PARTY Accept.  Assert the
//!    `Rejected`/`AGGRESSION_CONFLICT` refusal, an unchanged non-member party view, and the
//!    still-present invitation (the aggression join guard must not consume durable state).
//! 5. After the player-visible combat/skull lock naturally expires, send PARTY Accept again.
//!    Assert the accepted two-member roster, leader/member shields, and green party skull
//!    exemption rather than a yellow aggression skull.
//! 6. Log both players out normally, freshly admit both characters, walk them back into
//!    visibility, and re-query.  Assert new game-session ids and the durable two-member party,
//!    leader/member shields, and green party skull exemption; then leave and log out.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityRef};
use oteryn_protocol_oteryn::{CommandStatus, party_skull};
use std::collections::BTreeSet;

const STEPS: &[&str] = &[
    "admit-walk-neutral",
    "invite-shields",
    "aggression-white-skull",
    "join-guard-refusal",
    "join-party-exemption",
    "fresh-admission-durable",
];

struct Row {
    started: std::time::Instant,
    passed: BTreeSet<&'static str>,
}

impl Row {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            passed: BTreeSet::new(),
        }
    }
    fn pass(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        if STEPS.contains(&step) && self.passed.insert(step) {
            evidence(&format!(
                "S02_CHECK step={step} status=PASS observed={observed:?}"
            ));
        }
    }
    fn finish(self, result: &TestResult) {
        let complete = result.is_ok() && STEPS.iter().all(|step| self.passed.contains(step));
        if let Err(error) = result {
            evidence(&format!("S02_GAP observed={error}"));
        }
        evidence(&format!(
            "e2e_all scenario=S02.full status={} seconds={:.3} evidence={}",
            if complete { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            if complete {
                "party-shields-skulls-guard-relogin"
            } else {
                "step-observation-in-S02_GAP"
            },
        ));
    }
}

type PartyResult = oteryn_dev_client::client_runtime::party::party::PartyResult;
use oteryn_dev_client::client_runtime::party::party::{PartyDisposition, PartyIntent};

fn own(client: &DevClientSession) -> TestResult<EntityRef> {
    let entities = client
        .world_entities()
        .ok_or("S02 own entity view absent")?;
    entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "S02 own entity absent".into())
}

async fn party(
    client: &mut DevClientSession,
    intent: Option<PartyIntent>,
) -> TestResult<(PartyDisposition, PartyResult)> {
    use oteryn_dev_client::client_runtime::party as runtime;
    let command = match intent {
        Some(intent) => {
            runtime::PartyCommand::intent(intent).map_err(|e| format!("S02 party encode: {e:?}"))?
        }
        None => runtime::PartyCommand::query(),
    };
    let reply = runtime::send(client.session_mut(), &command).await?;
    let operation = runtime::party::decode_party_result(&reply.operation)
        .map_err(|e| format!("S02 party operation: {e:?}"))?;
    let view =
        runtime::party::decode_party_result(reply.view.as_deref().ok_or("S02 party view absent")?)
            .map_err(|e| format!("S02 party view: {e:?}"))?;
    Ok((operation.disposition, view))
}

async fn icons(client: &mut DevClientSession) -> TestResult<Vec<party_skull::PartyIcon>> {
    let reply = client
        .session_mut()
        .gated_command(
            party_skull::CAPABILITY_PARTY_SKULL_V1,
            party_skull::COMMAND_TYPE_PARTY_SKULL_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step icon-query observed reply={reply:?}").into());
    }
    party_skull::decode_party_icons(&reply.payload)
        .map_err(|e| format!("S02 icon reply: {e:?}").into())
}

async fn icon(client: &mut DevClientSession, target: EntityRef) -> TestResult<(u8, u8)> {
    icons(client)
        .await?
        .into_iter()
        .find(|entry| entry.entity == target)
        .map(|entry| (entry.shield, entry.skull))
        .ok_or_else(|| format!("step icon-query missing visible target={target:?}").into())
}

async fn pump(a: &mut DevClientSession, b: &mut DevClientSession) -> TestResult {
    a.service_liveness(Duration::from_millis(25)).await?;
    b.service_liveness(Duration::from_millis(25)).await?;
    a.take_events();
    b.take_events();
    Ok(())
}

fn distance(a: TilePos, b: TilePos) -> u16 {
    if a.floor == b.floor {
        a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
    } else {
        u16::MAX
    }
}

async fn place_pair(
    a: &mut DevClientSession,
    b: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    let path = route(world, tile(a.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|t| !t.protection_zone())
            && [
                StepDirection::North,
                StepDirection::East,
                StepDirection::South,
                StepDirection::West,
            ]
            .into_iter()
            .any(|d| {
                destination(world, at, d).is_some_and(|p| {
                    world
                        .base()
                        .tile(p.x, p.y, p.floor)
                        .is_some_and(|t| !t.protection_zone())
                })
            })
    })
    .ok_or("step walk-a: no reachable unprotected pair")?;
    walk(a, world, &path).await?;
    let target = tile(a.world_spatial().actor_position)?;
    let path = route(world, tile(b.world_spatial().actor_position)?, |at| {
        at != target
            && distance(at, target) <= 1
            && world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|t| !t.protection_zone())
    })
    .ok_or("step walk-b: no reachable adjacent unprotected tile")?;
    walk(b, world, &path).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    while tokio::time::Instant::now() < deadline {
        pump(a, b).await?;
    }
    Ok(())
}

async fn set_open_attack(client: &mut DevClientSession) -> TestResult {
    use oteryn_dev_client::client_runtime::pvp_expert as pvp;
    let expected = pvp::ExpertFightModes {
        modes: pvp::attack::FightModes {
            secure: false,
            ..pvp::attack::FightModes::DEFAULT
        },
        expert: pvp::ExpertMode::RedFist,
    };
    let reply = client
        .session_mut()
        .gated_command(
            pvp::CAPABILITY_PVP_EXPERT_V1,
            pvp::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
            &pvp::encode_expert_fight_modes(&expected),
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step aggression expert-mode reply={reply:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(pvp::STATE_DOMAIN_EXPERT_FIGHT_MODES, 1)
        .await?;
    let observed = pvp::decode_expert_fight_modes(&delta.payload)
        .map_err(|e| format!("S02 expert delta: {e:?}"))?;
    if observed != expected {
        return Err(format!("step aggression expert delta={observed:?}").into());
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession, budget: Duration) -> TestResult {
    let deadline = tokio::time::Instant::now() + budget;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    loop {
        let reply = client.session_mut().logout().await?;
        match reply.result {
            Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) => return Ok(()),
            Some(
                oteryn_protocol_oteryn::logout::LogoutResult::InFight { .. }
                | oteryn_protocol_oteryn::logout::LogoutResult::Busy,
            ) if tokio::time::Instant::now() < deadline => {
                client.service_liveness(Duration::from_millis(250)).await?;
                client.take_events();
            }
            _ => return Err(format!("step logout observed reply={reply:?}").into()),
        }
    }
}

fn invitation_retained(view: &PartyResult, handle: u64) -> bool {
    view.invitations.iter().any(|entry| entry.handle == handle)
}

struct PvpWorld {
    address: std::net::SocketAddr,
    url: String,
    accounts: [String; 2],
    characters: Vec<[u8; 16]>,
    world: [u8; 16],
    channel: [u8; 16],
    scope_generation: u64,
}

async fn pvp_world() -> TestResult<PvpWorld> {
    let url = required("NODE_BOOT_PVP_DATABASE_URL")?;
    let accounts = [
        required("NODE_BOOT_S02_KNIGHT_ACCOUNT")?,
        required("NODE_BOOT_S02_SORCERER_ACCOUNT")?,
    ];
    let world = crate::node::uuid_bytes(&required("NODE_BOOT_PVP_WORLD_ID")?)
        .ok_or("invalid NODE_BOOT_PVP_WORLD_ID")?;
    let channel = crate::node::uuid_bytes(&required("NODE_BOOT_PVP_CHANNEL_ID")?)
        .ok_or("invalid NODE_BOOT_PVP_CHANNEL_ID")?;
    // Same bounded, read-only PgConnection pattern as the node-boot fixture read.
    let mut database = sqlx::PgConnection::connect(&url).await?;
    let world_type: i16 = sqlx::query_scalar("SELECT world_type FROM game_spell_field_world_policies WHERE world_id=encode($1,'hex')::uuid AND world_policy_revision='policy-s3b-1'")
        .bind(world.as_slice()).fetch_one(&mut database).await?;
    if world_type != 2 {
        return Err(format!("PvP World policy world_type={world_type};expected=2 (Open PvP);source:tools/qualification/node_boot/run.sh:1531").into());
    }
    let mut characters = Vec::new();
    for account in &accounts {
        let found: Vec<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(character_id) FROM game_character_roots WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid LIMIT 2")
            .bind(account).bind(world.as_slice()).fetch_all(&mut database).await?;
        let [character] = found.as_slice() else {
            return Err(format!(
                "PvP World Account must own exactly one Character: account={account} count={}",
                found.len()
            )
            .into());
        };
        characters.push(<[u8; 16]>::try_from(character.as_slice())?);
    }
    let scope_generation: String = sqlx::query_scalar("SELECT ownership_generation::text FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1")
        .bind(world.as_slice()).bind(channel.as_slice()).fetch_one(&mut database).await?;
    database.close().await?;
    Ok(PvpWorld {
        address: required("NODE_BOOT_PVP_ADDRESS")?.parse()?,
        url,
        accounts,
        characters,
        world,
        channel,
        scope_generation: scope_generation.parse()?,
    })
}

async fn prepare_fighter(
    clients: &SeamClients<'_>,
    tag: u8,
    index: usize,
    profile: &'static str,
) -> TestResult<DevClientSession> {
    use oteryn_protocol_oteryn::cyclopedia_character as stats;

    let mut fighter = character_prep::Seeded {
        profile,
        reason: "Open PvP level floor",
    }
    .connect_at(clients, tag, index, "NODE_BOOT_PVP_CONTROL_SOCKET")
    .await?;
    let reply = fighter
        .session_mut()
        .gated_command(
            stats::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            stats::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[stats::CharacterTab::GeneralStats as u8, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("S02 player {index} GeneralStats refused: {:?}", reply.status).into());
    }
    general_stats::validate(&reply.payload, general_stats::selected(&fighter))?;
    let level = u32::from_le_bytes(reply.payload[9..13].try_into()?);
    if level != 8 {
        return Err(format!("S02 player {index} served level {level}, expected 8").into());
    }
    evidence(&format!("S02_CHECK step=seeded-level status=PASS player={index} level={level}"));
    Ok(fighter)
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let loaded = boot_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let world = &loaded.bundle;
    character_prep::declare_seeded(
        "S02",
        "two-dedicated-open-pvp-characters-at-level-8",
        "Open PvP requires level 8 before exercising the party aggression guard",
    );
    let mut a = prepare_fighter(clients, 0xb0, 0, "s02-knight").await?;
    let mut b = prepare_fighter(clients, 0xb1, 1, "s02-sorcerer").await?;
    place_pair(&mut a, &mut b, world).await?;
    let (ar, av) = party(&mut a, None).await?;
    let (br, bv) = party(&mut b, None).await?;
    if ar != PartyDisposition::Ok
        || br != PartyDisposition::Ok
        || !av.members.is_empty()
        || !bv.members.is_empty()
    {
        return Err(
            format!("step 1 neutral party replies={ar:?}/{br:?} views={av:?}/{bv:?}").into(),
        );
    }
    row.pass(
        "admit-walk-neutral",
        (
            a.world_spatial().actor_position,
            b.world_spatial().actor_position,
        ),
    );

    let a_ref = own(&a)?;
    let b_ref = own(&b)?;
    let (reply, _) = party(&mut a, Some(PartyIntent::Invite(b_ref))).await?;
    if reply != PartyDisposition::Ok {
        return Err(format!("step 2 invite reply={reply:?}").into());
    }
    let (_, invitation) = party(&mut b, None).await?;
    let handle = invitation
        .invitations
        .first()
        .ok_or("step 2 invite missing peer view delta")?
        .handle;
    let ai = icon(&mut a, b_ref).await?;
    let bi = icon(&mut b, a_ref).await?;
    if ai.0 != 2 || bi.0 != 1 {
        return Err(format!("step 2 invitation shields attacker={ai:?} invitee={bi:?}").into());
    }
    row.pass("invite-shields", (reply, ai, bi));

    let immune_until = tokio::time::Instant::now() + Duration::from_secs(11);
    while tokio::time::Instant::now() < immune_until {
        pump(&mut a, &mut b).await?;
    }
    set_open_attack(&mut a).await?;
    let before = *b.actor_vitals().ok_or("step 3 victim vitals absent")?;
    let attack = a.session_mut().attack_target(Some(&b_ref)).await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!(
            "step=aggression-white-skull observed={attack:?} need=level-eight-combat-eligible-open-pvp-players"
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    while b.actor_vitals().is_none_or(|v| v.health >= before.health) {
        if tokio::time::Instant::now() >= deadline {
            return Err(
                format!("step 3 accepted attack missing health delta before={before:?}").into(),
            );
        }
        pump(&mut a, &mut b).await?;
    }
    a.session_mut().attack_target(None).await?;
    let observed = icon(&mut b, a_ref).await?;
    if observed.1 != 3 {
        return Err(format!("step 3 expected white skull observed={observed:?}").into());
    }
    row.pass(
        "aggression-white-skull",
        (attack, before, b.actor_vitals(), observed),
    );

    let (refused, refused_view) = party(&mut b, Some(PartyIntent::Accept(handle))).await?;
    let (_, retained) = party(&mut b, None).await?;
    if refused != PartyDisposition::Rejected
        || !refused_view.members.is_empty()
        || !invitation_retained(&retained, handle)
    {
        return Err(format!(
            "step 4 join guard reply={refused:?} view={refused_view:?} retained={retained:?}"
        )
        .into());
    }
    row.pass("join-guard-refusal", (refused, retained.invitations.len()));

    // The production aggression/skull lock is intentionally real.  Poll the same Accept command;
    // every refused reply is an observed negative path, and only server expiry permits progress.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(920);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let joined = loop {
        pump(&mut a, &mut b).await?;
        let (_, state) = party(&mut b, None).await?;
        let current = state
            .invitations
            .first()
            .map(|entry| entry.handle)
            .ok_or("step 5 invitation disappeared while guard active")?;
        let (reply, state) = party(&mut b, Some(PartyIntent::Accept(current))).await?;
        if reply == PartyDisposition::Ok {
            break state;
        }
        if reply != PartyDisposition::Rejected || tokio::time::Instant::now() >= deadline {
            return Err(
                format!("step 5 retry accept observed reply={reply:?} state={state:?}").into(),
            );
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    };
    if joined.members.len() != 2 {
        return Err(format!("step 5 accepted roster={joined:?}").into());
    }
    let ai = icon(&mut a, b_ref).await?;
    let bi = icon(&mut b, a_ref).await?;
    if ai != (3, 2) || bi != (4, 2) {
        return Err(format!("step 5 party icons leader-view={ai:?} member-view={bi:?}").into());
    }
    row.pass("join-party-exemption", (ai, bi));

    let sessions = (
        a.join_snapshot().game_session_id,
        b.join_snapshot().game_session_id,
    );
    logout(&mut a, Duration::from_secs(920)).await?;
    logout(&mut b, Duration::from_secs(920)).await?;
    drop((a, b));
    let mut a = connect_character(clients, 0xb2, 0).await?;
    let mut b = connect_character(clients, 0xb3, 1).await?;
    if sessions
        == (
            a.join_snapshot().game_session_id,
            b.join_snapshot().game_session_id,
        )
    {
        return Err("step 6 fresh admission reused sessions".into());
    }
    place_pair(&mut a, &mut b, world).await?;
    let (_, durable) = party(&mut a, None).await?;
    let a_ref = own(&a)?;
    let b_ref = own(&b)?;
    let ai = icon(&mut a, b_ref).await?;
    let bi = icon(&mut b, a_ref).await?;
    if durable.members.len() != 2 || ai != (3, 2) || bi != (4, 2) {
        return Err(format!("step 6 durable roster={durable:?} icons={ai:?}/{bi:?}").into());
    }
    row.pass("fresh-admission-durable", (durable.members.len(), ai, bi));
    let _ = party(&mut b, Some(PartyIntent::Leave)).await?;
    logout(&mut a, Duration::from_secs(70)).await?;
    logout(&mut b, Duration::from_secs(70)).await?;
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(Duration::from_secs(2_100), async {
        let fixture = pvp_world()
            .await
            .map_err(|error| format!("step=open-pvp-fixture observed={error}"))?;
        let pvp = SeamClients {
            address: fixture.address,
            accounts: &fixture.accounts,
            characters: &fixture.characters,
            world: fixture.world,
            channel: fixture.channel,
            scope_generation: fixture.scope_generation,
            url: &fixture.url,
            runtime: None,
            ..*clients
        };
        let outcome = journey(&mut row, &pvp).await;
        let released = release_row_characters(&pvp).await;
        outcome.and(released)
    })
        .await
        .unwrap_or_else(|_| Err("step lifecycle timeout observed=no-reply-or-delta".into()));
    row.finish(&result);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guard_must_retain_the_invitation_that_was_refused() {
        let mut view = PartyResult::refused(PartyDisposition::Ok);
        view.invitations
            .push(oteryn_protocol_oteryn::party::PartyEntry {
                handle: 17,
                name: "Peer".into(),
                leader: false,
                list: Vec::new(),
                expires_at: 1,
                party_spell_remaining_ms: None,
                channel: None,
            });
        assert!(invitation_retained(&view, 17));
        assert!(!invitation_retained(&view, 18));
    }

    #[test]
    fn row_requires_every_numbered_step() {
        assert_eq!(STEPS.len(), 6);
        assert_eq!(
            STEPS.iter().copied().collect::<BTreeSet<_>>().len(),
            STEPS.len()
        );
    }
}

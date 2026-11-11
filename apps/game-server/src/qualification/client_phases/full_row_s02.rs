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

async fn logout(client: &mut RowClient, budget: Duration) -> TestResult {
    let deadline = tokio::time::Instant::now() + budget;
    loop {
        let reply = client.row_logout().await?;
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

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut a = row_connect_character(clients, 0xb0, 0).await?;
    let mut b = row_connect_character(clients, 0xb1, 1).await?;
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

    set_open_attack(&mut a).await?;
    let before = *b.actor_vitals().ok_or("step 3 victim vitals absent")?;
    let attack = a.session_mut().attack_target(Some(&b_ref)).await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!(
            "step 3 attack observed reply={attack:?}; need Open-PvP eligible provisioned players"
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
        || retained.invitations.is_empty()
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
    let mut a = row_connect_character(clients, 0xb2, 0).await?;
    let mut b = row_connect_character(clients, 0xb3, 1).await?;
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
    let result = row_complete_with_timeout(Duration::from_secs(2_100), journey(&mut row, clients))
        .await
        .unwrap_or_else(|_| Err("step lifecycle timeout observed=no-reply-or-delta".into()));
    row.finish(&result);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn row_requires_every_numbered_step() {
        assert_eq!(STEPS.len(), 6);
        assert_eq!(
            STEPS.iter().copied().collect::<BTreeSet<_>>().len(),
            STEPS.len()
        );
    }
}

//! S01 through player commands and observed combat/XP frames.
use super::*;
use oteryn_dev_client::{EntityKind, SessionEvent};
use oteryn_protocol_oteryn::{
    cyclopedia_character::{self as character, CharacterTab},
    presentation::{PresentationEventKind, PresentationValueKind},
};

pub(super) async fn run(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let mut task = std::pin::pin!(journey(row, client, clients, world));
    let mut pulse = tokio::time::interval(Duration::from_secs(5));
    loop {
        tokio::select! {
            biased;
            result = &mut task => return result,
            _ = pulse.tick() => verdict::Harness::global().progress("e2ef4-s01"),
        }
    }
}

// Like mp/bots.rs: service the other socket only between polls of the
// in-flight operation; never cancel an operation halfway through a frame.
async fn keeping_alive<T>(
    idle: &mut DevClientSession,
    step: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    let mut step = std::pin::pin!(step);
    let mut slice = tokio::time::interval(Duration::from_millis(250));
    loop {
        tokio::select! {
            biased;
            result = &mut step => return result,
            _ = slice.tick() => {
                verdict::Harness::global().progress("e2ef4-s01-peer");
                idle.service_liveness(Duration::from_millis(20)).await?;
            }
        }
    }
}

struct Stats {
    xp: u64,
    level: u32,
    stamina: u16,
}

async fn stats(client: &mut DevClientSession) -> TestResult<Stats> {
    let reply = client
        .session_mut()
        .gated_command(
            character::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            character::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[CharacterTab::GeneralStats as u8, 0, 0, 50],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("S01 GeneralStats refused: {reply:?}").into());
    }
    let vocation = general_stats::validate(&reply.payload, general_stats::selected(client))?;
    // Both harness starters have no vocation. Prove that from the served view,
    // so the single-vocation Crystal bonus is not assumed from a fixture label.
    if vocation != character::vocation_id("none") {
        return Err(format!("S01 requires observed starter vocation, got {vocation:?}").into());
    }
    Ok(Stats {
        xp: u64::from_le_bytes(reply.payload[1..9].try_into()?),
        level: u32::from_le_bytes(reply.payload[9..13].try_into()?),
        stamina: u16::from_le_bytes(reply.payload[29..31].try_into()?),
    })
}

async fn align_levels(
    client: &mut DevClientSession,
    peer: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    let bytes = std::fs::read(std::env::var("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    loop {
        let a = stats(client).await?;
        let b = stats(peer).await?;
        if u64::from(a.level.min(b.level)) * 3 >= u64::from(a.level.max(b.level)) * 2 {
            return Ok(());
        }
        let (lower, idle) = if a.level < b.level {
            (&mut *client, &mut *peer)
        } else {
            (&mut *peer, &mut *client)
        };
        keeping_alive(idle, async {
            super::super::travel_to_spawn(lower, world, &metadata.spawns, "oteryn:creature.rat")
                .await?;
            client_hunt::hunt(lower).await
        })
        .await?;
    }
}

fn hits(
    client: &mut DevClientSession,
    target: EntityRef,
    last_position: &mut oteryn_protocol_oteryn::world_spatial::ActorPosition,
) -> u64 {
    client
        .take_events()
        .iter()
        .filter_map(|event| {
            if let SessionEvent::WorldSpatialEntities(delta) = event
                && let Some(updated) = delta
                    .value
                    .update
                    .iter()
                    .find(|entry| entry.entity == target)
            {
                *last_position = updated.position;
            }
            let SessionEvent::Presentation(delta) = event else {
                return None;
            };
            Some(
                delta
                    .value
                    .iter()
                    .filter_map(|event| {
                        if let PresentationEventKind::ValueText {
                            kind: PresentationValueKind::DamageDealt,
                            target: victim,
                            primary,
                            ..
                        } = &event.kind
                            && *victim == target
                            && primary.value > 0
                        {
                            Some(primary.value)
                        } else {
                            None
                        }
                    })
                    .sum::<u64>(),
            )
        })
        .sum()
}

async fn shared_kill(
    row: &mut Row,
    client: &mut DevClientSession,
    peer: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    let bytes = std::fs::read(std::env::var("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    // A troll (20 XP) exercises the 20% bonus threshold without a
    // poison condition extending combat after the kill.
    keeping_alive(
        peer,
        super::super::travel_to_spawn(client, world, &metadata.spawns, "oteryn:creature.troll"),
    )
    .await?;
    let here = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry.display_name.is_some_and(|name| {
                        name.as_str()
                            .is_ok_and(|name| name.eq_ignore_ascii_case("troll"))
                    })
                    && entry.position.floor == here.floor
                    && entry
                        .position
                        .x
                        .abs_diff(here.x)
                        .max(entry.position.y.abs_diff(here.y))
                        <= 1
            })
        })
        .copied()
        .ok_or("S01 no adjacent named troll at authored spawn")?;
    if !matches!(
        target.detail,
        oteryn_dev_client::EntityDetail::Actor {
            health_percent: 100,
            ..
        } | oteryn_dev_client::EntityDetail::ActorWithIcons {
            health_percent: 100,
            ..
        }
    ) {
        return Err("S01 target must be an undamaged troll".into());
    }
    let profiles: serde_json::Value = serde_json::from_slice(&std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/test-packs/spells/r25/creature_profiles.json"),
    )?)?;
    let profile = profiles["records"]
        .as_array()
        .ok_or("S01 creature profiles absent")?
        .iter()
        .find(|record| record["profile"]["target"]["key"] == "oteryn:creature.troll")
        .ok_or("S01 authored troll profile absent")?;
    if profile["profile"]["data"]["profile"]["experience"].as_u64() != Some(20) {
        return Err("S01 authored troll XP differs from the 20 XP bonus oracle".into());
    }
    let at = tile(target.position)?;
    let occupied = tile(here)?;
    let path = route(world, tile(peer.world_spatial().actor_position)?, |p| {
        p != occupied
            && p != at
            && p.floor == at.floor
            && p.x.abs_diff(at.x).max(p.y.abs_diff(at.y)) <= 1
    })
    .ok_or("S01 peer has no adjacent troll route")?;
    keeping_alive(client, walk(peer, world, &path)).await?;
    let before = [stats(client).await?, stats(peer).await?];
    if u64::from(before[0].level.min(before[1].level)) * 3
        < u64::from(before[0].level.max(before[1].level)) * 2
    {
        return Err("S01 party outside shared XP level range".into());
    }
    let corpses_before: BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entry| entry.kind == EntityKind::Corpse)
        .map(|entry| entry.entity)
        .collect();
    client.take_events();
    peer.take_events();
    for fighter in [&mut *client, &mut *peer] {
        let reply = fighter
            .session_mut()
            .attack_target(Some(&target.entity))
            .await?;
        if reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
            return Err(format!("S01 attack refused: {reply:?}").into());
        }
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let mut damage = [0_u64; 2];
    let mut last_position = [target.position; 2];
    loop {
        for (i, fighter) in [&mut *client, &mut *peer].into_iter().enumerate() {
            fighter.service_liveness(Duration::from_millis(50)).await?;
            damage[i] += hits(fighter, target.entity, &mut last_position[i]);
        }
        verdict::Harness::global().progress("e2ef4-s01-fight");
        let removed = [&*client, &*peer].iter().all(|fighter| {
            fighter
                .world_entities()
                .is_some_and(|entities| entities.get(&target.entity).is_none())
        });
        let corpse = client.world_entities().is_some_and(|entities| {
            entities.iter().any(|entry| {
                entry.kind == EntityKind::Corpse
                    && !corpses_before.contains(&entry.entity)
                    && last_position.contains(&entry.position)
            })
        });
        if removed && corpse {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("S01 troll kill deadline; own damage={damage:?}").into());
        }
    }
    for fighter in [&mut *client, &mut *peer] {
        fighter.session_mut().attack_target(None).await?;
    }
    if damage.contains(&0) {
        return Err(format!("S01 both members must damage this troll: {damage:?}").into());
    }
    // Crystal: ceil(20 * 1.20 / 2) = 12 per member, before individual
    // low-level/stamina modifiers. These non-premium starter accounts have
    // no purchased boosts or prey selected; this journey never activates any.
    let mut expected = [0_u64; 2];
    for (i, observed) in before.iter().enumerate() {
        let award = oteryn_gameplay_characters::hunting_experience::hunting_experience(
            12,
            0,
            0,
            i32::from(observed.stamina),
            false,
            observed.level,
        )
        .map_err(|_| "S01 hunting XP oracle rejected served stats")?;
        expected[i] = u64::try_from(award)?;
        if expected[i] == 0 {
            return Err("S01 positive XP required for both members".into());
        }
    }
    loop {
        let after = [stats(client).await?, stats(peer).await?];
        let gained = [
            after[0]
                .xp
                .checked_sub(before[0].xp)
                .ok_or("S01 XP regressed")?,
            after[1]
                .xp
                .checked_sub(before[1].xp)
                .ok_or("S01 peer XP regressed")?,
        ];
        if gained == expected {
            row.pass("shared-xp-split", format!("troll=20 bonus=20% members=2 share=12 before={:?} after={:?} gained={gained:?} damage={damage:?}", [before[0].xp,before[1].xp], [after[0].xp,after[1].xp]));
            return Ok(());
        }
        if gained.iter().zip(expected).any(|(got, want)| *got > want)
            || tokio::time::Instant::now() >= deadline
        {
            return Err(
                format!("S01 shared XP mismatch: gained={gained:?} expected={expected:?}").into(),
            );
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        peer.service_liveness(Duration::from_millis(50)).await?;
        verdict::Harness::global().progress("e2ef4-s01-xp");
    }
}

async fn journey(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_dev_client::client_runtime::party as runtime;
    use runtime::party::{PartyDisposition, PartyIntent};
    async fn send(
        client: &mut DevClientSession,
        intent: Option<PartyIntent>,
    ) -> TestResult<runtime::party::PartyResult> {
        let command = match intent {
            Some(intent) => {
                runtime::PartyCommand::intent(intent).map_err(|e| format!("party input: {e:?}"))?
            }
            None => runtime::PartyCommand::query(),
        };
        let reply = runtime::send(client.session_mut(), &command).await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("S01 party command status: {:?}", reply.status).into());
        }
        let operation = runtime::party::decode_party_result(&reply.operation)
            .map_err(|e| format!("party operation: {e:?}"))?;
        if operation.disposition != PartyDisposition::Ok {
            return Err(format!("party refused: {:?}", operation.disposition).into());
        }
        runtime::party::decode_party_result(
            reply.view.as_deref().ok_or("party current view absent")?,
        )
        .map_err(|e| format!("party view: {e:?}").into())
    }
    let mut peer = connect_character(clients, 0x82, 0).await?;
    let result: TestResult = async {
        align_levels(client, &mut peer, world).await?;
        let other = tile(peer.world_spatial().actor_position)?;
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            at != other
                && at.floor == other.floor
                && at.x.abs_diff(other.x).max(at.y.abs_diff(other.y)) <= 1
        })
        .ok_or("S01 party invitation approach unavailable")?;
        keeping_alive(&mut peer, walk(client, world, &path)).await?;
        send(client, Some(PartyIntent::Invite(own(&peer)?))).await?;
        let invitations = send(&mut peer, None).await?;
        let handle = invitations
            .invitations
            .first()
            .ok_or("party invitation not visible to peer")?
            .handle;
        row.pass("invite", "peer-current-query-has-invitation");
        let state = send(&mut peer, Some(PartyIntent::Accept(handle))).await?;
        if state.members.len() != 2 {
            return Err("party join did not produce two members".into());
        }
        row.pass("join", "two-current-members");
        let state = send(client, None).await?;
        let member = state
            .members
            .iter()
            .find(|m| !m.leader)
            .ok_or("no nonleader member")?;
        let target_name = member.name.clone();
        let state = send(client, Some(PartyIntent::PassLeadership(member.handle))).await?;
        if !state
            .members
            .iter()
            .any(|m| m.leader && m.name == target_name)
        {
            return Err("leadership did not pass to selected member".into());
        }
        row.pass("leadership", "current-query-selected-member-is-leader");
        let result = peer
            .chat(&oteryn_dev_client::ChatIntent::Say {
                mode: oteryn_dev_client::ChatSpeechMode::Say,
                text: "/sharedxp on".into(),
            })
            .await?;
        if result.disposition != oteryn_dev_client::ChatDisposition::Ok
            || !send(&mut peer, None).await?.shared_xp_enabled
            || !send(client, None).await?.shared_xp_enabled
        {
            return Err("shared XP toggle not reflected by party owner".into());
        }
        row.pass(
            "shared-xp-toggle",
            "leader-chat-toggle-current-state-enabled",
        );
        shared_kill(row, client, &mut peer, world).await?;
        // Party leave is intentionally refused during the real combat lock.
        // Retry the native action, servicing both sockets until it is accepted.
        let leave_deadline = tokio::time::Instant::now() + Duration::from_secs(70);
        loop {
            let command = runtime::PartyCommand::intent(PartyIntent::Leave)
                .map_err(|e| format!("S01 leave input: {e:?}"))?;
            let reply = runtime::send(client.session_mut(), &command).await?;
            let operation = runtime::party::decode_party_result(&reply.operation)
                .map_err(|e| format!("S01 leave operation: {e:?}"))?;
            match operation.disposition {
                PartyDisposition::Ok if reply.status == CommandStatus::Accepted => break,
                PartyDisposition::LogoutBlocked if tokio::time::Instant::now() < leave_deadline => {
                    client.service_liveness(Duration::from_millis(100)).await?;
                    peer.service_liveness(Duration::from_millis(100)).await?;
                    verdict::Harness::global().progress("e2ef4-s01-leave-lock");
                }
                other => return Err(format!("S01 leave refused: {other:?}").into()),
            }
        }
        if !send(client, None).await?.members.is_empty()
            || !send(&mut peer, None).await?.members.is_empty()
        {
            return Err("party leave retained membership".into());
        }
        row.pass("leave", "leaver-current-query-empty");
        Ok(())
    }
    .await;
    for fighter in [&mut *client, &mut peer] {
        let _ = fighter.session_mut().attack_target(None).await;
    }
    let _ = send(client, Some(PartyIntent::Leave)).await;
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        keeping_alive(client, super::logout_retrying_busy(&mut peer)).await?;
    if cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("party peer cleanup logout refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={result:?}").into());
    }
    result
}

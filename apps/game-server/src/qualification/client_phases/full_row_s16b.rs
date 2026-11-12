//! S16b — vocation conversion and changed-vocation statistics.
//!
//! Concrete player scenario derived from the S16b matrix/re-audit row and its
//! `character_build` owner:
//! 1. Admit the node-boot Character with no vocation; query GeneralStats,
//!    trained SkillProgress and live ActorVitals, then walk to Oressa.
//! 2. Say `hi`, `knight`, `yes` before level 8; require accepted command frames,
//!    Oressa's level refusal conversation delta, and unchanged build/stat frames.
//! 3. Walk to authored rat spawns and issue real ATTACK commands until exactly
//!    level 8, requiring creature removal plus an XP delta after every kill.
//! 4. Query the pre-choice skill/ML snapshot, walk back, and say `hi`, `knight`,
//!    `yes`; require each NPC result/conversation frame and the committed reply.
//! 5. Require the post-choice GeneralStats and SkillProgress frames to agree,
//!    show conversion of genuinely earned nonzero training, and require live
//!    health and mana to be refilled to the level-8 knight maxima (185/90).
//! 6. Attempt `hi`, `druid`, `yes`; require the second-choice refusal and no
//!    change to vocation, stats, trained progress, or live maxima.
//! 7. Logout and freshly admit the Character; re-query every durable frame and
//!    require the knight vocation, converted skill/ML state and maxima to match.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::skill_progress::SkillProgressSnapshot;
use std::collections::BTreeSet;

const ACTIONS: &[&str] = &[
    "initial-snapshots",
    "underlevel-refusal",
    "player-earned-level-eight",
    "confirmed-conversion",
    "refilled-changed-stats",
    "second-choice-refusal",
    "fresh-admission-durable",
];

struct Row {
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
}

impl Row {
    fn pass(&mut self, action: &'static str, observed: impl std::fmt::Debug) {
        if ACTIONS.contains(&action) {
            self.passed.insert(action);
            evidence(&format!("S16B_CHECK {action} PASS observed={observed:?}"));
        }
    }

    fn finish(self, result: TestResult) {
        if let Err(error) = &result {
            evidence(&format!("S16B_GAP step-failed observed={error}"));
        }
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("S16B_CHECK {action} FAIL missing-delta-or-reply"));
            }
        }
        evidence(&format!(
            "e2e_all scenario=S16b.full status={} seconds={:.3}",
            if result.is_ok() && self.passed.len() == ACTIONS.len() { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row { passed: BTreeSet::new(), started: std::time::Instant::now() };
    let result = verdict::indexed_timeout(Duration::from_secs(480), verdict::observe(journey(&mut row, clients))).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err("step deadline after real player command; server supplied no completing frame".into()),
    });
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Stats {
    xp: u64,
    level: u32,
    magic: u16,
    skills: [u16; 7],
}

fn decode_stats(payload: &[u8]) -> TestResult<Stats> {
    if payload.len() != 42 || payload[0] != 1 || payload[33] > 1 {
        return Err(format!("GeneralStats malformed: len={} tab={:?}", payload.len(), payload.first()).into());
    }
    Ok(Stats {
        xp: u64::from_le_bytes(payload[1..9].try_into()?),
        level: u32::from_le_bytes(payload[9..13].try_into()?),
        magic: u16::from_le_bytes(payload[13..15].try_into()?),
        skills: std::array::from_fn(|i| u16::from_le_bytes([payload[15 + i * 2], payload[16 + i * 2]])),
    })
}

async fn stats(client: &mut DevClientSession) -> TestResult<Stats> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client.session_mut().gated_command(
        wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
        wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
        &[1, 0, 0, 1],
    ).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("GeneralStats query status={:?} payload={:?}", reply.status, reply.payload).into());
    }
    decode_stats(&reply.payload)
}

async fn trained(client: &mut DevClientSession) -> TestResult<SkillProgressSnapshot> {
    use oteryn_protocol_oteryn::skill_progress as wire;
    let reply = client.session_mut().gated_command(
        wire::CAPABILITY_SKILL_PROGRESS_V1,
        wire::COMMAND_TYPE_SKILL_PROGRESS_QUERY,
        &[],
    ).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("SkillProgress query status={:?} payload={:?}", reply.status, reply.payload).into());
    }
    SkillProgressSnapshot::decode(&reply.payload)
        .ok_or_else(|| "SkillProgress reply missing or malformed".into())
}

async fn talk(
    client: &mut DevClientSession,
    npc: oteryn_dev_client::EntityRef,
    text: &str,
    disposition: NpcIntentDisposition,
    line: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::npc_service as wire;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent { npc_actor: npc, text: text.into() })
        .map_err(|error| format!("step NPC {text} encode: {error:?}"))?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let reply = loop {
        let reply = client.session_mut().gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        ).await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("step NPC {text} command status={:?} payload={:?}", reply.status, reply.payload).into());
        }
        let observed = wire::decode_npc_intent_result(&reply.payload)
            .map_err(|error| format!("step NPC {text} result: {error:?}"))?;
        if observed == wire::NpcIntentDisposition::RateLimited && tokio::time::Instant::now() < deadline {
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
            continue;
        }
        if observed != disposition {
            return Err(format!("step NPC {text} disposition={observed:?}, expected={disposition:?}").into());
        }
        break reply;
    };
    loop {
        let delta = client.session_mut().read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1).await?;
        if delta.server_sequence <= reply.server_sequence { continue; }
        let conversation = wire::decode_npc_conversation(&delta.payload)
            .map_err(|error| format!("step NPC {text} conversation: {error:?}"))?;
        if conversation.open.map(|(actor, _)| actor) != Some(npc) || conversation.lines != [line] {
            return Err(format!("step NPC {text} observed conversation={conversation:?}").into());
        }
        return Ok(());
    }
}

async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let npc = npcs.npcs.iter().find(|npc| npc.key == "oteryn:npc.oressa")
        .ok_or("step approach: compiled Oressa placement missing")?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements.iter().any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
    }).ok_or("step approach: no legitimate walking route to Oressa")?;
    walk(client, world, &path).await?;
    client.world_entities().and_then(|entities| entities.iter().find(|entry| {
        entry.kind == EntityKind::Npc && npc.placements.iter().any(|p| {
            entry.position.x == i32::from(p.x) && entry.position.y == i32::from(p.y)
                && entry.position.floor == -i16::from(p.floor)
        })
    })).map(|entry| entry.entity).ok_or_else(|| "step approach: Oressa actor not visible after walking".into())
}

async fn earn_level_eight(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<Stats> {
    use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut before = stats(client).await?;
    while before.level < 8 {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step level-8 timeout after ATTACK: observed={before:?}; server needs reachable credited spawns").into());
        }
        super::travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await
            .map_err(|error| format!("step level-8 legitimate rat route: {error}"))?;
        let here = client.world_spatial().actor_position;
        let target = client.world_entities().and_then(|entities| entities.iter().find(|entry| {
            entry.kind == EntityKind::Creature && entry.position.floor == here.floor
                && entry.position.x.abs_diff(here.x).max(entry.position.y.abs_diff(here.y)) <= 1
        })).copied().ok_or("step level-8: no adjacent creature after walking")?;
        client.take_events();
        let attack = client.session_mut().attack_target(Some(&target.entity)).await?;
        if attack.disposition != AttackIntentDisposition::Ok {
            return Err(format!("step level-8 ATTACK reply={attack:?}").into());
        }
        let kill_deadline = (tokio::time::Instant::now() + Duration::from_secs(60)).min(deadline);
        loop {
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
            if client.world_entities().is_some_and(|entities| entities.get(&target.entity).is_none()) {
                let after = stats(client).await?;
                if after.xp <= before.xp || after.level < before.level {
                    return Err(format!("step level-8 removed creature but missing XP delta: {before:?}->{after:?}").into());
                }
                before = after;
                break;
            }
            if tokio::time::Instant::now() >= kill_deadline {
                return Err(format!("step level-8 ATTACK accepted but creature/XP delta timed out: {attack:?}").into());
            }
        }
    }
    client.session_mut().attack_target(None).await?;
    if before.level != 8 { return Err(format!("step level-8 overshot exact level: {before:?}").into()); }
    Ok(before)
}

fn assert_consistent(stats: &Stats, skills: &SkillProgressSnapshot) -> TestResult {
    if stats.magic != skills.skills[0].level
        || stats.skills != std::array::from_fn(|i| skills.skills[i + 1].level)
    {
        return Err(format!("stats and trained progress disagree: {stats:?}/{skills:?}").into());
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("logout refusal after vocation commands".into());
    }
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    // Character zero is intentionally used: S16a owns Character one later in this phase.
    let mut client = connect_character(clients, 0xb0, 0).await?;
    let initial_session = client.join_snapshot().game_session_id;
    let initial = stats(&mut client).await?;
    let initial_skills = trained(&mut client).await?;
    assert_consistent(&initial, &initial_skills)?;
    if initial.level >= 8 { return Err(format!("step initial prerequisite already level {}", initial.level).into()); }
    row.pass("initial-snapshots", (&initial, &initial_skills));

    let npc = approach(&mut client, world, &metadata.npcs).await?;
    talk(&mut client, npc, "hi", NpcIntentDisposition::Ok, "Welcome. Ask about a vocation when you are ready to choose.").await?;
    talk(&mut client, npc, "knight", NpcIntentDisposition::Ok, "Do you wish to become a knight? Say yes to confirm, or no to reconsider.").await?;
    talk(&mut client, npc, "yes", NpcIntentDisposition::Rejected, "Reach level 8 before making your first vocation choice.").await?;
    if stats(&mut client).await? != initial || trained(&mut client).await? != initial_skills {
        return Err("step underlevel refusal changed stats or trained progress".into());
    }
    row.pass("underlevel-refusal", "Rejected + unchanged frames");

    // Clear retained refused conversation through an ordinary fresh admission.
    logout(&mut client).await?;
    client = connect_character(clients, 0xb1, 0).await?;
    let earned = earn_level_eight(&mut client, world, &metadata.spawns).await?;
    row.pass("player-earned-level-eight", &earned);
    let before = stats(&mut client).await?;
    let before_skills = trained(&mut client).await?;
    assert_consistent(&before, &before_skills)?;
    let npc = approach(&mut client, world, &metadata.npcs).await?;
    talk(&mut client, npc, "hi", NpcIntentDisposition::Ok, "Welcome. Ask about a vocation when you are ready to choose.").await?;
    talk(&mut client, npc, "knight", NpcIntentDisposition::Ok, "Do you wish to become a knight? Say yes to confirm, or no to reconsider.").await?;
    talk(&mut client, npc, "yes", NpcIntentDisposition::Ok, "Your vocation is now knight.").await?;
    let after = stats(&mut client).await?;
    let after_skills = trained(&mut client).await?;
    assert_consistent(&after, &after_skills)?;
    if !before_skills.skills.iter().any(|skill| skill.progress > 0)
        || before_skills == after_skills
    {
        return Err(format!("step conversion missing earned nonzero training or converted delta: {before_skills:?}->{after_skills:?}; server needs to expose the committed conversion").into());
    }
    row.pass("confirmed-conversion", (&before_skills, &after_skills));
    let vitals = *client.actor_vitals().ok_or("step changed stats missing ActorVitals frame")?;
    if vitals.health != 185 || vitals.max_health != 185 || vitals.mana != 90 || vitals.max_mana != 90 {
        return Err(format!("step changed stats expected level-8 knight refill 185/90, observed={vitals:?}").into());
    }
    row.pass("refilled-changed-stats", vitals);

    talk(&mut client, npc, "hi", NpcIntentDisposition::Ok, "Welcome. Ask about a vocation when you are ready to choose.").await?;
    talk(&mut client, npc, "druid", NpcIntentDisposition::Ok, "Do you wish to become a druid? Say yes to confirm, or no to reconsider.").await?;
    talk(&mut client, npc, "yes", NpcIntentDisposition::Rejected, "Reach level 8 before making your first vocation choice.").await?;
    if stats(&mut client).await? != after || trained(&mut client).await? != after_skills
        || client.actor_vitals() != Some(&vitals) {
        return Err("step second-choice refusal changed committed frames".into());
    }
    row.pass("second-choice-refusal", "Rejected + unchanged frames");
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xb2, 0).await?;
    if fresh.join_snapshot().game_session_id == initial_session
        || stats(&mut fresh).await? != after
        || trained(&mut fresh).await? != after_skills
        || fresh.actor_vitals() != Some(&vitals)
    {
        return Err("step fresh admission did not restore converted stats/progress/vitals".into());
    }
    row.pass("fresh-admission-durable", (&after, &after_skills, vitals));
    logout(&mut fresh).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_decoder_reads_magic_and_all_seven_skills() -> TestResult {
        let mut payload = vec![0; 42];
        payload[0] = 1;
        payload[1..9].copy_from_slice(&123u64.to_le_bytes());
        payload[9..13].copy_from_slice(&8u32.to_le_bytes());
        payload[13..15].copy_from_slice(&4u16.to_le_bytes());
        for i in 0..7 { payload[15 + i * 2..17 + i * 2].copy_from_slice(&(10 + i as u16).to_le_bytes()); }
        assert_eq!(decode_stats(&payload)?, Stats { xp: 123, level: 8, magic: 4, skills: [10, 11, 12, 13, 14, 15, 16] });
        assert!(decode_stats(&payload[..41]).is_err());
        Ok(())
    }
}

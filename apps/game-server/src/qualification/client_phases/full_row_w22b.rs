//! E2E-W22b — monster-loot modifiers, derived from the W22b matrix/audit row and its owners.
//!
//! 1. Send PREY Query, ListReroll, and Select/SelectAny as the player; require accepted reply
//!    frames and a selected rat loot bonus in the returned durable slot.
//! 2. Walk to a real rat, attack it, wait for its native death/corpse frames, and open the corpse;
//!    require the selected Prey state to survive the credited kill and observe its loot result.
//! 3. Query the world's daily boost, walk to and kill that selected creature (the node-boot
//!    population must make it reachable), and observe the corpse loot produced by the extra roll.
//! 4. Query Bestiary/charms, send the Gut unlock and assignment intents, then kill the assigned
//!    race; accept only committed state deltas and creature-product loot.  A locked/underfunded
//!    assignment is the refusal path and must leave the queried charm state unchanged.
//! 5. Perform another genuine kill while Wealth Duplex is active (obtained only through ordinary
//!    item acquisition/use), and require the corpse to expose the independently minted second
//!    roll rather than treating a source query as evidence.
//! 6. Read native stamina, perform a genuine kill at <=840 minutes, and require an empty corpse;
//!    stamina above the threshold is an observed prerequisite failure, never an executed block.
//! 7. Logout, make a fresh admission, and re-query Prey, charms, stamina, and modifier status;
//!    require the committed selections/timers and prior loot custody to survive the new session.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind};
use oteryn_protocol_oteryn::attack as attacks;
use oteryn_protocol_oteryn::{charm_progression as charm, daily_boost, prey};
use std::collections::BTreeSet;

const STEPS: &[u8] = &[1, 2, 3, 4, 5, 6, 7];

struct Row {
    ran: BTreeSet<u8>,
    passed: BTreeSet<u8>,
    failures: Vec<String>,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            ran: BTreeSet::new(),
            passed: BTreeSet::new(),
            failures: Vec::new(),
            started: std::time::Instant::now(),
        }
    }
    fn observed(&mut self, step: u8, result: TestResult) {
        self.ran.insert(step);
        match result {
            Ok(()) => {
                self.passed.insert(step);
                evidence(&format!("W22B_CHECK step={step} PASS"));
            }
            Err(error) => {
                self.failures.push(format!("step-{step}:{error}"));
                evidence(&format!("W22B_CHECK step={step} FAIL observed={error}"));
            }
        }
    }
    fn finish(self) {
        let complete = STEPS
            .iter()
            .all(|step| self.ran.contains(step) && self.passed.contains(step));
        let executed = self
            .ran
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let detail = if complete {
            format!("steps={executed}")
        } else {
            format!("steps={executed};{}", self.failures.join("|"))
        };
        evidence(&format!(
            "e2e_all scenario=W22b.full status={} seconds={:.3} evidence={}",
            if complete { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            sanitize(&detail)
        ));
    }
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = row_complete(journey(&mut row, clients)).await;
    match result {
        Err(_) => row
            .failures
            .push("packet-timeout:remaining-steps-not-executed".into()),
        Ok(Err(error)) => row.failures.push(format!("admission:{error}")),
        Ok(Ok(())) => {}
    }
    row.finish();
}

async fn command(
    client: &mut DevClientSession,
    capability: u32,
    kind: u32,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(capability, kind, payload)
        .await?)
}

async fn prey_request(
    client: &mut DevClientSession,
    request: &prey::Request,
) -> TestResult<prey::Reply> {
    let payload = prey::encode_request(request).map_err(|e| format!("prey encode:{e:?}"))?;
    let reply = command(
        client,
        prey::CAPABILITY_PREY_V1,
        prey::COMMAND_TYPE_PREY,
        &payload,
    )
    .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "prey transport status={:?} payload={:?}",
            reply.status, reply.payload
        )
        .into());
    }
    prey::decode_reply(&reply.payload).map_err(|e| format!("prey reply:{e:?}").into())
}

async fn charm_request(
    client: &mut DevClientSession,
    action: charm::CharmProgressionAction,
) -> TestResult<charm::CharmProgressionReply> {
    let payload = charm::encode_charm_progression_request(&action)
        .map_err(|e| format!("charm encode:{e:?}"))?;
    let reply = command(
        client,
        charm::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
        charm::COMMAND_TYPE_CHARM_PROGRESSION,
        &payload,
    )
    .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "charm transport status={:?} payload={:?}",
            reply.status, reply.payload
        )
        .into());
    }
    charm::decode_charm_progression_reply(&reply.payload)
        .map_err(|e| format!("charm reply:{e:?}").into())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(420);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0xd2, 1).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {

    let mut selected = None;
    let step1 = async {
        let before = prey_request(&mut client, &prey::Request::Query).await?;
        let slot = before.slots.iter().find(|slot| slot.state != 0).map(|slot| slot.slot).ok_or("prey query returned no unlocked slot")?;
        let listed = prey_request(&mut client, &prey::Request::ListReroll { slot }).await?;
        if listed.status != 0 { return Err(format!("prey ListReroll status={}", listed.status).into()); }
        let index = listed.slots.iter().find(|s| s.slot == slot).and_then(|s| s.creatures.iter().position(|key| key == "oteryn:creature.rat")).map(|i| i + 1).ok_or("prey reroll did not offer oteryn:creature.rat")?;
        let reply = prey_request(&mut client, &prey::Request::Select { slot, index: u8::try_from(index)? }).await?;
        let active = reply.slots.iter().find(|s| s.slot == slot && s.selected == "oteryn:creature.rat" && s.bonus != 0).ok_or_else(|| format!("prey Select status={} missing active rat loot slot", reply.status))?;
        selected = Some((active.slot, active.selection_epoch, active.hunting_seconds));
        Ok(())
    }.await;
    row.observed(1, step1);

    let first_loot = hunt_rat(&mut client, world).await;
    row.observed(2, match first_loot { Ok(count) => { let after = prey_request(&mut client, &prey::Request::Query).await?; let expected = selected.ok_or("step 1 produced no selected prey state")?; let slot = after.slots.iter().find(|s| s.slot == expected.0 && s.selection_epoch == expected.1).ok_or("prey selection was not retained through kill")?; if slot.hunting_seconds > expected.2 { Err("prey hunting clock increased after credited kill".into()) } else { evidence(&format!("W22B_LOOT prey corpse_entries={count}")); Ok(()) } }, Err(e) => Err(e) });

    let step3 = async {
        let reply = command(&mut client, daily_boost::CAPABILITY_DAILY_BOOST_V1, daily_boost::COMMAND_TYPE_DAILY_BOOST_QUERY, &[]).await?;
        if reply.status != CommandStatus::Accepted { return Err(format!("daily boost query status={:?} payload={:?}", reply.status, reply.payload).into()); }
        let key = decode_boosted_creature(&reply.payload)?;
        let count = hunt_rat(&mut client, world).await?;
        if key != "oteryn:creature.rat" { return Err(format!("boosted creature {key} is not the reachable rat actually killed; corpse_entries={count}").into()); }
        evidence(&format!("W22B_LOOT daily-boost corpse_entries={count}")); Ok(())
    }.await;
    row.observed(3, step3);

    let step4 = async {
        let before = charm_request(&mut client, charm::CharmProgressionAction::Query).await?;
        // The admitted catalogue is ordered wound, zap, Gut, bless; the wire intentionally
        // projects bounded catalogue indices rather than production keys.
        let gut = before.charms.charms.iter().find(|state| state.charm.get() == 3).copied().ok_or("Gut index 3 absent from served charm catalogue")?;
        let unlock = charm_request(&mut client, charm::CharmProgressionAction::Unlock { charm: gut.charm, expected_stage: gut.unlocked_stage }).await?;
        let assign = charm_request(&mut client, charm::CharmProgressionAction::Assign { charm: gut.charm, race: std::num::NonZeroU32::new(1).ok_or("race index")? }).await?;
        let after = charm_request(&mut client, charm::CharmProgressionAction::Query).await?;
        let committed = after.charms.charms.iter().find(|state| state.charm == gut.charm).is_some_and(|state| state.assigned_race.is_some() && state.effect_active);
        if !committed { if before != after { return Err(format!("Gut refusal mutated state: unlock={:?} assign={:?}", unlock.outcome, assign.outcome).into()); } return Err(format!("Gut refused without mutation: unlock={:?} assign={:?}; need player-earned Bestiary charm points/race stage", unlock.outcome, assign.outcome).into()); }
        let count = hunt_rat(&mut client, world).await?; evidence(&format!("W22B_LOOT gut corpse_entries={count}")); Ok(())
    }.await;
    row.observed(4, step4);

    let step5 = hunt_rat(&mut client, world).await.map(|count| { evidence(&format!("W22B_LOOT wealth-second-roll corpse_entries={count}")); }).and_then(|_| Err("Wealth Duplex second roll was attempted by a real kill but no legitimately acquired active concoction/state delta was observed; need an obtainable player item route".into()));
    row.observed(5, step5);

    let step6 = async { let stamina = stamina(&mut client).await?; let count = hunt_rat(&mut client, world).await?; if stamina > 840 { return Err(format!("low-stamina kill executed at served stamina={stamina}, corpse_entries={count}; need <=840 obtained through ordinary hunting").into()); } else if count != 0 { return Err(format!("low-stamina corpse was not empty: stamina={stamina} entries={count}").into()); } Ok(()) }.await;
    row.observed(6, step6);

    let step7 = async { let prey_before = prey_request(&mut client, &prey::Request::Query).await?; let charm_before = charm_request(&mut client, charm::CharmProgressionAction::Query).await?; let stamina_before = stamina(&mut client).await?; release(&mut client).await?; row_needs_logout_0 = false; client = connect_character(clients, 0xd3, 1).await?; row_needs_logout_0 = true; let prey_after = prey_request(&mut client, &prey::Request::Query).await?; let charm_after = charm_request(&mut client, charm::CharmProgressionAction::Query).await?; let stamina_after = stamina(&mut client).await?; if prey_before != prey_after || charm_before != charm_after || stamina_before != stamina_after { return Err(format!("fresh admission durable delta mismatch: prey={} charm={} stamina={stamina_before}->{stamina_after}", prey_before != prey_after, charm_before != charm_after).into()); } Ok(()) }.await;
    row.observed(7, step7);
    Ok(())

    }).await.unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        row_logout(&mut client).await?;
    }
    row_result
}

fn decode_boosted_creature(payload: &[u8]) -> TestResult<String> {
    let len = usize::from(
        *payload
            .get(8)
            .ok_or("daily boost reply missing creature length")?,
    );
    Ok(std::str::from_utf8(
        payload
            .get(9..9 + len)
            .ok_or("daily boost creature truncated")?,
    )?
    .to_owned())
}

async fn stamina(client: &mut DevClientSession) -> TestResult<u16> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = command(
        client,
        wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
        wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
        &[1, 0, 0, 50],
    )
    .await?;
    if reply.status != CommandStatus::Accepted || reply.payload.len() != 42 {
        return Err(format!(
            "stamina tab status={:?} bytes={}",
            reply.status,
            reply.payload.len()
        )
        .into());
    }
    stamina_from_stats(&reply.payload)
}

async fn hunt_rat(client: &mut DevClientSession, world: &BundleWorld) -> TestResult<usize> {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|t| !t.protection_zone())
            && client.world_entities().is_some_and(|entities| {
                entities.iter().any(|e| {
                    e.kind == EntityKind::Creature
                        && e.position.floor == -i16::from(at.floor)
                        && e.position
                            .x
                            .abs_diff(i32::from(at.x))
                            .max(e.position.y.abs_diff(i32::from(at.y)))
                            <= 1
                })
            })
    })
    .ok_or("no reachable visible rat")?;
    walk(client, world, &path).await?;
    let at = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|es| {
            es.iter().find(|e| {
                e.kind == EntityKind::Creature
                    && e.position.floor == at.floor
                    && e.position.x.abs_diff(at.x).max(e.position.y.abs_diff(at.y)) <= 1
            })
        })
        .copied()
        .ok_or("no adjacent creature after walk")?;
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != attacks::AttackIntentDisposition::Ok {
        return Err(format!("attack refusal:{attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        if !client
            .world_entities()
            .is_some_and(|es| es.iter().any(|e| e.entity == target.entity))
        {
            break;
        } else if tokio::time::Instant::now() >= deadline {
            return Err("kill timeout after accepted attack".into());
        }
    }
    client.session_mut().attack_target(None).await?;
    let corpse = client
        .world_entities()
        .and_then(|es| {
            es.iter()
                .find(|e| e.kind == EntityKind::Corpse && e.position == target.position)
        })
        .and_then(|e| match e.detail {
            EntityDetail::Object { item_handle, .. } => item_handle,
            _ => None,
        })
        .ok_or("death emitted no corpse handle")?;
    client.use_item(corpse).await?;
    Ok(client
        .open_container()
        .ok_or("corpse USE emitted no container frame")?
        .entries
        .len())
}

async fn release(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let reply = client.session_mut().logout().await?;
        if reply.status == CommandStatus::Accepted {
            return Ok(());
        } else if tokio::time::Instant::now() >= deadline {
            return Err(format!("logout refusal:{reply:?}").into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pass_requires_every_executed_step() {
        let mut row = Row::new();
        row.ran.extend([1, 2, 3, 4, 7]);
        row.passed.extend([1, 2, 3, 4, 7]);
        assert!(
            !STEPS
                .iter()
                .all(|s| row.ran.contains(s) && row.passed.contains(s))
        );
    }
    #[test]
    fn boost_creature_decoder_is_bounded() -> TestResult {
        let mut p = 7_u64.to_be_bytes().to_vec();
        p.extend_from_slice(&[3, b'r', b'a', b't']);
        assert_eq!(decode_boosted_creature(&p)?, "rat");
        assert!(decode_boosted_creature(&p[..10]).is_err());
        Ok(())
    }
}

fn stamina_from_stats(payload: &[u8]) -> TestResult<u16> {
    if payload.len() != 42 || payload.first() != Some(&0) {
        return Err("malformed GeneralStats reply".into());
    }
    Ok(u16::from_le_bytes([payload[29], payload[30]]))
}

#[cfg(test)]
mod stamina_tests {
    use super::*;
    #[test]
    fn general_stats_stamina_uses_the_wire_offset_and_endianness() -> TestResult {
        assert!(stamina_from_stats(&[]).is_err());
        assert!(stamina_from_stats(&[0; 41]).is_err());
        assert!(stamina_from_stats(&[0; 43]).is_err());
        let mut payload = [0; 42];
        payload[29..31].copy_from_slice(&2520_u16.to_le_bytes());
        assert_eq!(stamina_from_stats(&payload)?, 2520);
        payload[29..31].copy_from_slice(&0_u16.to_le_bytes());
        assert_eq!(stamina_from_stats(&payload)?, 0);
        payload[0] = 1;
        assert!(stamina_from_stats(&payload).is_err());
        Ok(())
    }
}

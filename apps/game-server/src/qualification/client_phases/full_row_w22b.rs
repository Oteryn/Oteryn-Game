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
use oteryn_dev_client::DevClientSession;
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
        Self { ran: BTreeSet::new(), passed: BTreeSet::new(), failures: Vec::new(), started: std::time::Instant::now() }
    }
    fn observed(&mut self, step: u8, result: TestResult) {
        self.ran.insert(step);
        match result {
            Ok(()) => { self.passed.insert(step); evidence(&format!("W22B_CHECK step={step} PASS")); }
            Err(error) => { self.failures.push(format!("step-{step}:{error}")); evidence(&format!("W22B_CHECK step={step} FAIL observed={error}")); }
        }
    }
    fn finish(self) {
        let complete = STEPS.iter().all(|step| self.ran.contains(step) && self.passed.contains(step));
        let executed = self.ran.iter().map(u8::to_string).collect::<Vec<_>>().join(",");
        let detail = if complete { format!("steps={executed}") } else { format!("steps={executed};{}", self.failures.join("|")) };
        for failure in &self.failures {
            evidence(&format!("W22B_GAP {failure}"));
        }
        evidence(&format!("e2e_all scenario=W22b.full status={} seconds={:.3} evidence={}", if complete { "PASS" } else { "FAIL" }, self.started.elapsed().as_secs_f64(), sanitize(&detail)));
    }
}

fn sanitize(value: &str) -> String { value.chars().map(|c| if c.is_whitespace() { '_' } else { c }).collect() }

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(&mut row, clients))).await;
    match result {
        Err(_) => row.failures.push("packet-timeout:remaining-steps-not-executed".into()),
        Ok(Err(error)) => row.failures.push(format!("admission:{error}")),
        Ok(Ok(())) => {}
    }
    row.finish();
}

async fn command(client: &mut DevClientSession, capability: u32, kind: u32, payload: &[u8]) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client.session_mut().gated_command(capability, kind, payload).await?)
}

async fn prey_request(client: &mut DevClientSession, request: &prey::Request) -> TestResult<prey::Reply> {
    let payload = prey::encode_request(request).map_err(|e| format!("prey encode:{e:?}"))?;
    let reply = command(client, prey::CAPABILITY_PREY_V1, prey::COMMAND_TYPE_PREY, &payload).await?;
    if reply.status != CommandStatus::Accepted { return Err(format!("prey transport status={:?} payload={:?}", reply.status, reply.payload).into()); }
    prey::decode_reply(&reply.payload).map_err(|e| format!("prey reply:{e:?}").into())
}

async fn charm_request(client: &mut DevClientSession, action: charm::CharmProgressionAction) -> TestResult<charm::CharmProgressionReply> {
    let payload = charm::encode_charm_progression_request(&action).map_err(|e| format!("charm encode:{e:?}"))?;
    let reply = command(client, charm::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1, charm::COMMAND_TYPE_CHARM_PROGRESSION, &payload).await?;
    if reply.status != CommandStatus::Accepted { return Err(format!("charm transport status={:?} payload={:?}", reply.status, reply.payload).into()); }
    charm::decode_charm_progression_reply(&reply.payload).map_err(|e| format!("charm reply:{e:?}").into())
}

fn rat_loot_bonus(creature: &str, bonus: u8, percent: u8, seconds: u32) -> bool {
    creature == "oteryn:creature.rat" && bonus == 4 && percent > 0 && seconds > 0
}

fn gut_and_rat_indexes() -> TestResult<(std::num::NonZeroU32, std::num::NonZeroU32)> {
    let catalogue = crate::content::charm_source::CanonicalCharmCatalogue::embedded()
        .map_err(|error| format!("Gut catalogue: {error:?}"))?;
    let gut = catalogue.catalogue().definitions()
        .position(|definition| definition.key.as_str() == "oteryn:charm.gut")
        .ok_or("Gut absent from the canonical Charm catalogue")?;
    let races = crate::content::canonical_bestiary_rows(crate::content::ProjectEvidenceLimits {
        max_documents: 5,
        max_document_bytes: 2_500_000,
        max_total_bytes: 8_000_000,
        max_json_depth: 10,
        max_decoded_fields: 65_000,
        max_string_bytes: 1_000_000,
        max_locator_bytes: 128,
        max_locator_segments: 5,
        max_reference_records: 4096,
        max_import_records: 1,
        max_reimport_states: 1,
    }).map_err(|error| format!("Rat Bestiary catalogue: {error:?}"))?;
    let rat = races.iter().position(|(race, _)| race.key() == "oteryn:creature.rat")
        .ok_or("Rat absent from the canonical Bestiary catalogue")?;
    Ok((
        std::num::NonZeroU32::try_from(u32::try_from(gut + 1)?)?,
        std::num::NonZeroU32::try_from(u32::try_from(rat + 1)?)?,
    ))
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let spawns = &metadata.spawns;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut client = connect_character(clients, 0xd2, 1).await?;

    let mut selected = None;
    let step1 = async {
        let before = prey_request(&mut client, &prey::Request::Query).await?;
        let slot = before.slots.iter().find(|slot| slot.state != 0).map(|slot| slot.slot).ok_or("prey query returned no unlocked slot")?;
        let listed = prey_request(&mut client, &prey::Request::ListReroll { slot }).await?;
        if listed.status != 0 { return Err(format!("prey ListReroll status={}", listed.status).into()); }
        let index = listed.slots.iter().find(|s| s.slot == slot).and_then(|s| s.creatures.iter().position(|key| key == "oteryn:creature.rat")).map(|i| i + 1).ok_or("prey reroll did not offer oteryn:creature.rat")?;
        let reply = prey_request(&mut client, &prey::Request::Select { slot, index: u8::try_from(index)? }).await?;
        let active = reply.slots.iter().find(|s| {
            s.slot == slot
                && s.state == 2
                && rat_loot_bonus(&s.selected, s.bonus, s.percent, s.hunting_seconds)
        }).filter(|_| reply.status == 0).ok_or_else(|| {
            format!("prey Select status={} missing active rat loot bonus", reply.status)
        })?;
        selected = Some((active.slot, active.selection_epoch, active.hunting_seconds));
        Ok(())
    }.await;
    row.observed(1, step1);

    let step2 = async {
        let count = hunt_rat(&mut client, world, spawns).await?;
        let after = prey_request(&mut client, &prey::Request::Query).await?;
        let (id, epoch, seconds) = selected.ok_or("step 1 produced no selected prey state")?;
        let slot = after.slots.iter().find(|slot| {
            slot.slot == id && slot.selection_epoch == epoch
        }).ok_or("prey selection was not retained through kill")?;
        if slot.hunting_seconds > seconds {
            return Err("prey hunting clock increased after credited kill".into());
        }
        evidence(&format!("W22B_LOOT prey corpse_entries={count}"));
        Ok(())
    }.await;
    row.observed(2, step2);

    let step3 = async {
        let reply = command(&mut client, daily_boost::CAPABILITY_DAILY_BOOST_V1, daily_boost::COMMAND_TYPE_DAILY_BOOST_QUERY, &[]).await?;
        if reply.status != CommandStatus::Accepted { return Err(format!("daily boost query status={:?} payload={:?}", reply.status, reply.payload).into()); }
        let key = decode_boosted_creature(&reply.payload)?;
        travel_to_spawn(&mut client, world, spawns, &key).await
            .map_err(|error| format!("daily boost selects {key}; authored spawn route failed: {error}"))?;
        let profiles: serde_json::Value = serde_json::from_slice(&native_input()?.creature_profiles.bytes)?;
        let name = profiles["records"].as_array().and_then(|records| {
            records.iter().find_map(|record| {
                let profile = &record["profile"];
                if profile["target"]["key"].as_str() != Some(key.as_str()) {
                    return None;
                }
                profile["data"]["profile"]["details"]["display_name"].as_str()
            })
        }).ok_or_else(|| format!("daily boost selects {key}; pinned creature display name absent"))?;
        let count = super::full_row_w22a::kill_and_open(&mut client, name).await?.len();
        evidence(&format!("W22B_LOOT daily-boost creature={key} corpse_entries={count}"));
        Ok(())
    }.await;
    row.observed(3, step3);

    let step4 = async {
        let before = charm_request(&mut client, charm::CharmProgressionAction::Query).await?;
        let (gut, rat) = gut_and_rat_indexes()?;
        let state = before.charms.charms.iter().find(|state| state.charm == gut)
            .ok_or("Gut missing from the served Charm progression view")?;
        let unlock = charm_request(&mut client, charm::CharmProgressionAction::Unlock {
            charm: gut,
            expected_stage: state.unlocked_stage,
        }).await?;
        if unlock.outcome != charm::CharmProgressionOutcome::Unlock(
            oteryn_protocol_oteryn::charm::CharmUnlockDisposition::Unlocked,
        ) && (unlock.charms != before.charms || unlock.bestiary != before.bestiary) {
            return Err(format!("Gut unlock refusal mutated state: unlock={:?}", unlock.outcome).into());
        }
        let assign = charm_request(&mut client, charm::CharmProgressionAction::Assign {
            charm: gut,
            race: rat,
        }).await?;
        let after = charm_request(&mut client, charm::CharmProgressionAction::Query).await?;
        let committed = after.charms.charms.iter().find(|state| state.charm == gut)
            .is_some_and(|state| state.assigned_race == Some(rat) && state.effect_active);
        if !committed {
            if unlock.charms != after.charms || unlock.bestiary != after.bestiary {
                return Err(format!("Gut assignment refusal mutated state: assign={:?}", assign.outcome).into());
            }
            return Err(format!(
                "Gut could not be activated on Rat: unlock={:?} assign={:?}; need player-earned Minor Charm Echoes and the Rat Bestiary discovery stage",
                unlock.outcome, assign.outcome
            ).into());
        }
        let count = hunt_rat(&mut client, world, spawns).await?;
        evidence(&format!("W22B_LOOT gut corpse_entries={count}"));
        Ok(())
    }.await;
    row.observed(4, step4);

    row.observed(5, Err("need a starter acquisition and USE route for Wealth Duplex (oteryn:item.tibia.i36727) before its independent loot roll can be observed".into()));

    let step6 = async { let stamina = stamina(&mut client).await?; let count = hunt_rat(&mut client, world, spawns).await?; if stamina > 840 { return Err(format!("low-stamina kill executed at served stamina={stamina}, corpse_entries={count}; need <=840 obtained through ordinary hunting").into()); } else if count != 0 { return Err(format!("low-stamina corpse was not empty: stamina={stamina} entries={count}").into()); } Ok(()) }.await;
    row.observed(6, step6);

    let step7 = async { let prey_before = prey_request(&mut client, &prey::Request::Query).await?; let charm_before = charm_request(&mut client, charm::CharmProgressionAction::Query).await?; let stamina_before = stamina(&mut client).await?; release(&mut client).await?; client = connect_character(clients, 0xd3, 1).await?; let prey_after = prey_request(&mut client, &prey::Request::Query).await?; let charm_after = charm_request(&mut client, charm::CharmProgressionAction::Query).await?; let stamina_after = stamina(&mut client).await?; if prey_before != prey_after || charm_before != charm_after || stamina_before != stamina_after { return Err(format!("fresh admission durable delta mismatch: prey={} charm={} stamina={stamina_before}->{stamina_after}", prey_before != prey_after, charm_before != charm_after).into()); } Ok(()) }.await;
    row.observed(7, step7);
    Ok(())
}

fn decode_boosted_creature(payload: &[u8]) -> TestResult<String> {
    let len = usize::from(*payload.get(8).ok_or("daily boost reply missing creature length")?);
    Ok(std::str::from_utf8(payload.get(9..9 + len).ok_or("daily boost creature truncated")?)?.to_owned())
}

async fn stamina(client: &mut DevClientSession) -> TestResult<u16> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = command(client, wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1, wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, &[1, 0, 0, 50]).await?;
    if reply.status != CommandStatus::Accepted || reply.payload.len() < 43 { return Err(format!("stamina tab status={:?} bytes={}", reply.status, reply.payload.len()).into()); }
    Ok(u16::from_be_bytes([reply.payload[41], reply.payload[42]]))
}

async fn hunt_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<usize> {
    travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
    Ok(super::full_row_w22a::kill_and_open(client, "Rat").await?.len())
}

async fn release(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop { let reply = client.session_mut().logout().await?; if reply.status == CommandStatus::Accepted { return Ok(()); } else if tokio::time::Instant::now() >= deadline { return Err(format!("logout refusal:{reply:?}").into()); } client.service_liveness(Duration::from_millis(250)).await?; }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gut_assignment_uses_current_canonical_indices() -> TestResult {
        let (gut, rat) = gut_and_rat_indexes()?;
        assert_ne!(gut.get(), 3);
        assert_ne!(rat.get(), 1);
        Ok(())
    }

    #[test]
    fn journey_regression_requires_a_live_rat_loot_bonus() {
        assert!(rat_loot_bonus("oteryn:creature.rat", 4, 20, 60));
        assert!(!rat_loot_bonus("oteryn:creature.rat", 3, 20, 60));
        assert!(!rat_loot_bonus("oteryn:creature.rat", 4, 20, 0));
    }

    #[test]
    fn pass_requires_every_executed_step() { let mut row = Row::new(); row.ran.extend([1, 2, 3, 4, 7]); row.passed.extend([1, 2, 3, 4, 7]); assert!(!STEPS.iter().all(|s| row.ran.contains(s) && row.passed.contains(s))); }
    #[test]
    fn boost_creature_decoder_is_bounded() -> TestResult { let mut p = 7_u64.to_be_bytes().to_vec(); p.extend_from_slice(&[3, b'r', b'a', b't']); assert_eq!(decode_boosted_creature(&p)?, "rat"); assert!(decode_boosted_creature(&p[..10]).is_err()); Ok(()) }
}

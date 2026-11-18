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
//! 5. Establish Wealth Duplex custody through the fenced fixture, then require real player USE
//!    and a genuine kill while the effect is active. Require the corpse's independently minted second
//!    roll rather than treating a source query as evidence.
//! 6. Read native stamina, perform a genuine kill at <=840 minutes, and require an empty corpse;
//!    the explicit fenced fixture establishes exhaustion before the real kill, never its result.
//! 7. Logout, make a fresh admission, and re-query Prey, charms, stamina, and modifier status;
//!    require the committed selections/timers and prior loot custody to survive the new session.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind};
use oteryn_protocol_oteryn::attack as attacks;
use oteryn_protocol_oteryn::{self as native, charm_progression as charm, daily_boost, prey};
use std::collections::{BTreeMap, BTreeSet};

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
    let result = verdict::indexed_timeout(
        Duration::from_secs(420),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
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
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    reward_row(clients, "oteryn:creature.rat")?;
    let gut_target = product_target(clients, &metadata.spawns)?;
    let mut client = connect_character(clients, 0xd2, 1).await?;

    let mut selected = None;
    let step1 = async {
        let before = prey_request(&mut client, &prey::Request::Query).await?;
        let slot = before
            .slots
            .iter()
            .find(|slot| slot.state != 0)
            .map(|slot| slot.slot)
            .ok_or("prey query returned no unlocked slot")?;
        let listed = prey_request(&mut client, &prey::Request::ListReroll { slot }).await?;
        if listed.status != 0 {
            return Err(format!("prey ListReroll status={}", listed.status).into());
        }
        let index = listed
            .slots
            .iter()
            .find(|s| s.slot == slot)
            .and_then(|s| {
                s.creatures
                    .iter()
                    .position(|key| key == "oteryn:creature.rat")
            })
            .map(|i| i + 1)
            .ok_or("prey reroll did not offer oteryn:creature.rat")?;
        let reply = prey_request(
            &mut client,
            &prey::Request::Select {
                slot,
                index: u8::try_from(index)?,
            },
        )
        .await?;
        let active = reply
            .slots
            .iter()
            .find(|s| s.slot == slot && s.selected == "oteryn:creature.rat" && s.bonus == 4)
            .ok_or_else(|| {
                format!(
                    "prey Select status={} missing active rat loot slot",
                    reply.status
                )
            })?;
        selected = Some((active.slot, active.selection_epoch, active.hunting_seconds));
        Ok(())
    }
    .await;
    row.observed(1, step1);

    let step2 = async {
        let expected = selected.ok_or("step 1 produced no selected loot prey")?;
        prove_modifier(
            &mut client,
            world,
            &metadata.spawns,
            clients,
            "oteryn:creature.rat",
            Modifier::Prey,
        )
        .await?;
        let after = prey_request(&mut client, &prey::Request::Query).await?;
        let slot = after
            .slots
            .iter()
            .find(|slot| slot.slot == expected.0 && slot.selection_epoch == expected.1)
            .ok_or("prey selection changed through credited kill")?;
        if slot.hunting_seconds >= expected.2 {
            return Err("prey hunting clock did not decrease after credited kill".into());
        }
        Ok(())
    }
    .await;
    row.observed(2, step2);
    let step3 = async {
        let key = boosted(&mut client).await?;
        prove_modifier(
            &mut client,
            world,
            &metadata.spawns,
            clients,
            &key,
            Modifier::Daily,
        )
        .await
    }
    .await;
    row.observed(3, step3);
    let step4 = async {
        use oteryn_gameplay_domain::domain::charm_data::{CharmCatalogueRead, CharmEffect};
        let catalogue = crate::content::charm_source::CanonicalCharmCatalogue::embedded()?;
        let (index, _) = catalogue
            .catalogue()
            .definitions()
            .enumerate()
            .find(|(_, definition)| {
                catalogue
                    .charm(definition.key.as_str())
                    .is_some_and(|charm| {
                        matches!(charm.effect(), CharmEffect::CreatureProductBonus)
                    })
            })
            .ok_or("source Gut charm absent")?;
        let index = std::num::NonZeroU32::new(u32::try_from(index + 1)?).ok_or("Gut index")?;
        let before = charm_request(&mut client, charm::CharmProgressionAction::Query).await?;
        let gut = before
            .charms
            .charms
            .iter()
            .find(|state| state.charm == index)
            .ok_or("source Gut absent from live catalogue")?;
        if gut.unlocked_stage == 0 {
            charm_request(
                &mut client,
                charm::CharmProgressionAction::Unlock {
                    charm: index,
                    expected_stage: 0,
                },
            )
            .await?;
        }
        let race = source_race(&gut_target)?;
        charm_request(
            &mut client,
            charm::CharmProgressionAction::Assign { charm: index, race },
        )
        .await?;
        prove_modifier(
            &mut client,
            world,
            &metadata.spawns,
            clients,
            &gut_target,
            Modifier::Gut,
        )
        .await
    }
    .await;
    row.observed(4, step4);
    let step5 = async {
        if wealth_seconds(&mut client).await? == 0 {
            let reference = provisioning::item_reference(&provisioning::item_index()?, 36727)?;
            if !client.inventory().is_some_and(|view| {
                view.entries
                    .iter()
                    .any(|item| item.item_definition_ref == reference)
            }) {
                fixture_reentry(&mut client, clients, "w22b-wealth", [0xd4, 0xd5]).await?;
            }
            let handle = client
                .inventory()
                .and_then(|view| {
                    view.entries
                        .iter()
                        .find(|item| item.item_definition_ref == reference)
                })
                .map(|item| item.handle)
                .ok_or("Wealth Duplex setup did not produce concoction custody")?;
            let used = client.use_item(handle).await?;
            if used.status != CommandStatus::Accepted
                || used.disposition
                    != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            {
                return Err(format!("Wealth Duplex ordinary USE refused: {used:?}").into());
            }
        }
        let active = wealth_seconds(&mut client).await? > 0;
        let proof = prove_modifier(
            &mut client,
            world,
            &metadata.spawns,
            clients,
            "oteryn:creature.rat",
            Modifier::Wealth,
        )
        .await;
        if !wealth_proven(active, proof.is_ok()) {
            proof?;
            return Err("Wealth Duplex not active at observed second roll".into());
        }
        Ok(())
    }
    .await;
    row.observed(5, step5);
    let step6 = async {
        if oteryn_gameplay_characters::stamina::hunting_loot_allowed(stamina(&mut client).await?) {
            fixture_reentry(&mut client, clients, "w22b-stamina", [0xd6, 0xd7]).await?;
        }
        prove_modifier(
            &mut client,
            world,
            &metadata.spawns,
            clients,
            "oteryn:creature.rat",
            Modifier::Stamina,
        )
        .await
    }
    .await;
    row.observed(6, step6);

    let step7 = async { let prey_before = prey_request(&mut client, &prey::Request::Query).await?; let charm_before = charm_request(&mut client, charm::CharmProgressionAction::Query).await?; let stamina_before = stamina(&mut client).await?; release(&mut client).await?; client = connect_character(clients, 0xd3, 1).await?; let prey_after = prey_request(&mut client, &prey::Request::Query).await?; let charm_after = charm_request(&mut client, charm::CharmProgressionAction::Query).await?; let stamina_after = stamina(&mut client).await?; if prey_before != prey_after || charm_before != charm_after || stamina_before != stamina_after { return Err(format!("fresh admission durable delta mismatch: prey={} charm={} stamina={stamina_before}->{stamina_after}", prey_before != prey_after, charm_before != charm_after).into()); } Ok(()) }.await;
    row.observed(7, step7);
    Ok(())
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

fn decode_stamina(payload: &[u8]) -> TestResult<u16> {
    Ok(u16::from_le_bytes(
        payload.get(29..31).ok_or("stamina truncated")?.try_into()?,
    ))
}

fn corpse_open_proven(accepted: bool, committed: bool, same_handle: bool, fresh: bool) -> bool {
    accepted && committed && same_handle && fresh
}

fn modifier_proven(
    observed: &BTreeMap<u32, u64>,
    enabled: &BTreeMap<u32, u64>,
    disabled: &BTreeMap<u32, u64>,
) -> bool {
    enabled != disabled && observed == enabled
}

fn wealth_proven(active: bool, effect: bool) -> bool {
    active && effect
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
    if reply.status != CommandStatus::Accepted
        || general_stats::validate(&reply.payload, general_stats::selected(client)).is_err()
    {
        return Err(format!(
            "stamina tab status={:?} bytes={}",
            reply.status,
            reply.payload.len()
        )
        .into());
    }
    decode_stamina(&reply.payload)
}

pub(super) struct Hunt {
    target: oteryn_dev_client::EntityRef,
    corpse: oteryn_dev_client::EntityRef,
    items: BTreeMap<u32, u64>,
    stamina: u16,
}

pub(super) async fn hunt_creature(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    key: &str,
) -> TestResult<Hunt> {
    travel_to_spawn(client, world, spawns, key).await?;
    let name = creature_display_name(key)?;
    let at = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                creature_matches(entry, name)
                    && entry.position.floor == at.floor
                    && entry
                        .position
                        .x
                        .abs_diff(at.x)
                        .max(entry.position.y.abs_diff(at.y))
                        <= 1
            })
        })
        .copied()
        .ok_or("requested creature not adjacent after spawn travel")?;
    let old_corpses: BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|view| view.iter())
        .filter(|entry| entry.kind == EntityKind::Corpse)
        .map(|entry| entry.entity.identity)
        .collect();
    let stamina = stamina(client).await?; // Travel can consume stamina; sample at the attack.
    client.take_events();
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != attacks::AttackIntentDisposition::Ok {
        return Err(format!("attack refusal:{attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut last_position = target.position;
    let corpse = loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) = event
                && let Some(entry) = delta
                    .value
                    .update
                    .iter()
                    .find(|entry| entry.entity == target.entity)
            {
                last_position = entry.position;
            }
        }
        let removed = !client
            .world_entities()
            .is_some_and(|view| view.iter().any(|entry| entry.entity == target.entity));
        if removed
            && let Some(corpse) = client
                .world_entities()
                .and_then(|view| {
                    view.iter().find(|entry| {
                        entry.kind == EntityKind::Corpse
                            && entry.position == last_position
                            && !old_corpses.contains(&entry.entity.identity)
                    })
                })
                .copied()
        {
            break corpse;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("selected target produced no fresh corpse".into());
        }
    };
    client.session_mut().attack_target(None).await?;
    let handle = match corpse.detail {
        EntityDetail::Object {
            item_handle: Some(handle),
            ..
        } => handle,
        _ => return Err("fresh corpse has no item handle".into()),
    };
    client.take_events();
    let used = client.use_item(handle).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut fresh = false;
    loop {
        fresh |= client.take_events().into_iter().any(|event| matches!(event,
            oteryn_dev_client::SessionEvent::OpenContainer(delta)
            if delta.server_sequence > used.result_server_sequence && delta.value.container_handle == Some(handle)));
        let same = client
            .open_container()
            .is_some_and(|view| view.container_handle == Some(handle));
        if corpse_open_proven(
            used.status == CommandStatus::Accepted,
            used.disposition == oteryn_protocol_oteryn::world_object::UseDisposition::Committed,
            same,
            fresh,
        ) {
            break;
        }
        if used.status != CommandStatus::Accepted
            || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        {
            return Err(format!("selected corpse USE refused: {used:?}").into());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("selected corpse USE produced no fresh bound container".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    let mut items = BTreeMap::new();
    for item in &client
        .open_container()
        .ok_or("bound corpse missing")?
        .entries
    {
        *items.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(Hunt {
        target: target.entity,
        corpse: corpse.entity,
        items,
        stamina,
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Modifier {
    Prey,
    Daily,
    Gut,
    Wealth,
    Stamina,
}

async fn boosted(client: &mut DevClientSession) -> TestResult<String> {
    let reply = command(
        client,
        daily_boost::CAPABILITY_DAILY_BOOST_V1,
        daily_boost::COMMAND_TYPE_DAILY_BOOST_QUERY,
        &[],
    )
    .await?;
    if reply.status != CommandStatus::Accepted {
        return Err("daily boost query refused".into());
    }
    decode_boosted_creature(&reply.payload)
}

async fn wealth_seconds(client: &mut DevClientSession) -> TestResult<u64> {
    use oteryn_protocol_oteryn::cyclopedia_character::{
        self as wire,
        combat::{CombatStat, CombatStats},
    };
    let reply = command(
        client,
        wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
        wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
        &[10, 0, 0, 50],
    )
    .await?;
    if reply.status != CommandStatus::Accepted {
        return Err("Wealth Duplex timer query refused".into());
    }
    let stats = CombatStats::decode(&reply.payload)
        .map_err(|error| format!("concoction stats: {error:?}"))?;
    u64::try_from(
        stats
            .get(CombatStat::WealthDuplexSeconds)
            .ok_or("Wealth Duplex timer not observed")?,
    )
    .map_err(Into::into)
}

fn source_race(key: &str) -> TestResult<std::num::NonZeroU32> {
    let rows = crate::content::canonical_bestiary_rows(crate::content::ProjectEvidenceLimits {
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
    })?;
    let index = rows
        .iter()
        .position(|(race, _)| race.key() == key)
        .ok_or("Rat absent from source Bestiary")?;
    std::num::NonZeroU32::new(u32::try_from(index + 1)?).ok_or_else(|| "Rat race index zero".into())
}

fn product_target(
    clients: &SeamClients<'_>,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<String> {
    let document: serde_json::Value =
        serde_json::from_slice(&native_input()?.creature_profiles.bytes)?;
    let health: BTreeMap<_, _> = document["records"]
        .as_array()
        .ok_or("source creature profiles absent")?
        .iter()
        .filter_map(|record| {
            let profile = record.get("profile")?;
            let name = profile["target"]["key"]
                .as_str()?
                .strip_prefix("canary:creature/")?;
            Some((
                format!("oteryn:creature.{name}"),
                profile["data"]["profile"]["health"].as_u64()?,
            ))
        })
        .collect();
    spawns
        .creatures
        .iter()
        .filter_map(|creature| {
            let row = reward_row(clients, &creature.key).ok()?;
            row.loot_table
                .entries
                .iter()
                .any(|entry| entry.details.creature_product)
                .then(|| {
                    health
                        .get(&creature.key)
                        .map(|health| (*health, creature.key.clone()))
                })
                .flatten()
        })
        .min()
        .map(|(_, key)| key)
        .ok_or_else(|| "no authored creature-product spawn for Gut".into())
}

fn reward_row(
    clients: &SeamClients<'_>,
    key: &str,
) -> TestResult<std::sync::Arc<crate::content::creature_reward::CreatureRewardRow>> {
    static NATIVE: std::sync::OnceLock<
        Result<crate::content::native_gameplay::NativeGameplayState, String>,
    > = std::sync::OnceLock::new();
    let native = NATIVE
        .get_or_init(|| {
            let load = || -> TestResult<_> {
                let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../content/spells.manifest.json");
                let room = crate::content::qualify_native_entry_room_from_gameplay_manifest(
                    WorldId::decode(&clients.world)?,
                    &manifest,
                )?;
                Ok(crate::content::qualified_native_gameplay_test_state(
                    room.compiled(),
                )?)
            };
            load().map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| format!("loot oracle source: {error}"))?;
    native
        .reward_table()
        .row(key)
        .map_err(|error| format!("loot source absent: {error:?}").into())
}

// Read historical receipts only. Reconstructing their opaque actor identity grants no
// authority and never creates a death, Item, reward, or modifier in the running server.
async fn death_seed(
    database: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
    hunt: &Hunt,
) -> TestResult<(oteryn_gameplay_items::loot_plan::LootPlanDeathKey, i64)> {
    use crate::foundation::{
        ActorLocalGeneration, ActorLocalId, ActorRef, ExactActorRef, ScopeOwnershipGeneration,
    };
    use sqlx::Row;
    let rows = sqlx::query("SELECT death_scope_ownership_generation::text AS scope_generation, death_actor_local_id, death_actor_local_generation::text AS actor_generation, materialized_at FROM game_item_mint_receipts WHERE death_world_id=encode($1,'hex')::uuid AND corpse_top_damage_character_id=encode($2,'hex')::uuid AND death_scope_id=(decode('01','hex') || $3::bytea) AND loot_purpose_key='CORPSE_MATERIALIZATION' ORDER BY materialized_at DESC LIMIT 64")
        .bind(clients.world.as_slice()).bind(clients.characters[1].as_slice()).bind(clients.channel.as_slice()).fetch_all(&mut *database).await?;
    let world = WorldId::decode(&clients.world)?;
    let channel = ChannelId::decode(&clients.channel)?;
    for row in rows {
        let scope: u64 = row.try_get::<String, _>("scope_generation")?.parse()?;
        let id = u32::try_from(row.try_get::<i64, _>("death_actor_local_id")?)?;
        let generation: u64 = row.try_get::<String, _>("actor_generation")?.parse()?;
        let actor = ExactActorRef(ActorRef::from_owner_parts(
            world,
            channel,
            ScopeOwnershipGeneration::new(scope)?,
            ActorLocalId(id),
            ActorLocalGeneration(generation),
        ));
        if actor.placement_identity() == hunt.target.identity
            && actor.corpse_identity() == hunt.corpse.identity
        {
            return Ok((
                oteryn_gameplay_items::loot_plan::LootPlanDeathKey::new(
                    clients.world,
                    clients.channel,
                    scope,
                    id,
                    generation,
                ),
                row.try_get("materialized_at")?,
            ));
        }
    }
    Err("no retained death binds the selected target to the opened corpse".into())
}

#[derive(Clone)]
struct LootContext {
    daily: bool,
    prey_percent: u8,
    gut: u32,
    wealth: bool,
    suppressed: bool,
    event_rate: u32,
    occurrence: [u8; 16],
}

fn expected_loot(
    row: &crate::content::creature_reward::CreatureRewardRow,
    seed: oteryn_gameplay_items::loot_plan::LootPlanDeathKey,
    context: &LootContext,
) -> TestResult<BTreeMap<u32, u64>> {
    use oteryn_gameplay_items::loot_plan::{self as loot, LootSelectionAlgorithm};
    use sha2::{Digest, Sha256};
    let mut table = row.loot_table.clone();
    if row.boss.is_some() {
        return Err("loot modifier oracle requires an ordinary creature".into());
    }
    if context.daily {
        table.algorithm = LootSelectionAlgorithm::DailyBoosted {
            extra_rolls: 1,
            boosted_entries: table.entries.len(),
        };
    }
    if context.prey_percent > 0 {
        if row.loot_table.entries.len() > 7 {
            return Err("Prey extra roll exceeds native source bound".into());
        }
        let mut hash = Sha256::new();
        hash.update(b"oteryn.prey.loot.v1");
        hash.update(context.occurrence);
        let hash = hash.finalize();
        let draw = ((u128::from(u64::from_be_bytes(hash[..8].try_into()?)) * 100) >> 64) as u8;
        if draw < context.prey_percent {
            table.entries.extend(row.loot_table.entries.iter().cloned());
        }
    }
    fn gut(entries: &mut [loot::LootTableEntry], bonus: u32) {
        for entry in entries {
            if entry.details.creature_product {
                entry.details.gut_bonus_hundredths = bonus;
            }
            gut(&mut entry.details.children, bonus);
        }
    }
    gut(&mut table.entries, context.gut);
    if context.suppressed {
        table.entries.clear();
    }
    if context.event_rate >= 200 && context.event_rate.is_multiple_of(100) {
        let multiplier = u8::try_from(context.event_rate / 100)?;
        let (rolls, boosted_entries) = match table.algorithm {
            LootSelectionAlgorithm::DailyBoosted {
                extra_rolls,
                boosted_entries,
            } => (
                (extra_rolls + 1)
                    .checked_mul(multiplier)
                    .ok_or("loot rate rolls overflow")?
                    - 1,
                boosted_entries,
            ),
            _ => (multiplier - 1, table.entries.len()),
        };
        table.algorithm = LootSelectionAlgorithm::DailyBoosted {
            extra_rolls: rolls,
            boosted_entries,
        };
    } else {
        loot::apply_configured_loot_rate(&mut table, context.event_rate);
    }
    if context.wealth && !table.entries.is_empty() {
        table.entries.extend(row.loot_table.entries.iter().cloned());
    }
    let plan = loot::plan_creature_loot(
        seed,
        row.loot_table_ref.as_ref().ok_or("loot reference absent")?,
        &table,
    )
    .map_err(|error| format!("source loot plan: {error:?}"))?;
    let index = provisioning::item_index()?;
    let mut quantities = BTreeMap::new();
    for entry in plan.entries {
        if entry.parent.is_some() {
            return Err("nested loot needs an opened child container before comparison".into());
        }
        let id: u32 = entry
            .item
            .production_key
            .strip_prefix("oteryn:item.tibia.i")
            .ok_or("loot item not canonically indexed")?
            .parse()?;
        let reference = provisioning::item_reference(&index, id)?;
        *quantities.entry(reference.get()).or_default() += u64::from(entry.quantity);
    }
    Ok(quantities)
}

async fn prove_modifier(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    clients: &SeamClients<'_>,
    key: &str,
    modifier: Modifier,
) -> TestResult {
    use oteryn_gameplay_domain::domain::charm_data::{CharmCatalogueRead, CharmEffect};
    use sqlx::Row;
    let row = reward_row(clients, key)?;
    let catalogue = crate::content::charm_source::CanonicalCharmCatalogue::embedded()?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    loop {
        let mut database = sqlx::PgConnection::connect(clients.url).await?;
        let cursor: i64 = sqlx::query_scalar("SELECT coalesce(max(committed_character_revision),0)::bigint FROM game_character_xp_receipts WHERE character_id=encode($1,'hex')::uuid")
            .bind(clients.characters[1].as_slice()).fetch_one(&mut database).await?;
        let prey = prey_request(client, &prey::Request::Query).await?;
        let prey_percent = prey
            .slots
            .iter()
            .find(|slot| slot.selected == key && slot.bonus == 4 && slot.hunting_seconds > 0)
            .map_or(0, |slot| slot.percent);
        let daily = boosted(client).await? == key;
        let wealth = wealth_seconds(client).await? > 0;
        let charms = sqlx::query("SELECT a.charm_key,u.unlocked_stage FROM game_character_charm_assignments a JOIN game_character_charm_unlocks u USING(character_id,charm_key) WHERE a.character_id=encode($1,'hex')::uuid AND a.race_key=$2")
            .bind(clients.characters[1].as_slice()).bind(key).fetch_all(&mut database).await?;
        let mut gut = 0;
        for charm in charms {
            if let Some(definition) = catalogue.charm(&charm.try_get::<String, _>("charm_key")?)
                && matches!(definition.effect(), CharmEffect::CreatureProductBonus)
            {
                gut = definition
                    .stage(u8::try_from(charm.try_get::<i16, _>("unlocked_stage")?)?)
                    .ok_or("Gut stage absent")?
                    .hundredths();
            }
        }
        let hunt = hunt_creature(client, world, spawns, key).await?;
        let (seed, materialized_at) = death_seed(&mut database, clients, &hunt).await?;
        let occurrences: Vec<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(reward_occurrence_id) FROM game_character_xp_receipts WHERE character_id=encode($1,'hex')::uuid AND committed_character_revision>$2::bigint ORDER BY committed_character_revision LIMIT 2")
            .bind(clients.characters[1].as_slice()).bind(cursor).fetch_all(&mut database).await?;
        if occurrences.len() != 1 {
            return Err("kill did not produce exactly one new credited XP occurrence".into());
        }
        let event_rate = oteryn_gameplay_foundation::scheduled_events::content_schedule()
            .rates_at(materialized_at / 1000)
            .loot;
        let context = LootContext {
            daily,
            prey_percent,
            gut,
            wealth,
            suppressed: hunt.stamina <= 840,
            event_rate,
            occurrence: occurrences[0].as_slice().try_into()?,
        };
        let mut disabled = context.clone();
        let active = match modifier {
            Modifier::Prey => {
                disabled.prey_percent = 0;
                prey_percent > 0
            }
            Modifier::Daily => {
                disabled.daily = false;
                daily
            }
            Modifier::Gut => {
                disabled.gut = 0;
                gut > 0
            }
            Modifier::Wealth => {
                disabled.wealth = false;
                wealth
            }
            Modifier::Stamina => {
                disabled.suppressed = false;
                context.suppressed
            }
        };
        if !active {
            return Err("requested loot modifier was not active at the observed kill".into());
        }
        let enabled = expected_loot(&row, seed, &context)?;
        let without = expected_loot(&row, seed, &disabled)?;
        if modifier_proven(&hunt.items, &enabled, &without) {
            evidence(&format!(
                "W22B_LOOT exact-death-counterfactual=proven actual={:?} without={without:?} stamina={}",
                hunt.items, hunt.stamina
            ));
            return Ok(());
        }
        if hunt.items != enabled {
            return Err(format!(
                "native loot differs from exact death oracle: observed={:?} expected={enabled:?}",
                hunt.items
            )
            .into());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("no discriminating loot roll before modifier deadline".into());
        }
        client.take_events();
        client.service_liveness(Duration::from_millis(250)).await?;
    }
}

async fn release_with<T, L, P>(state: &mut T, mut logout: L, mut poll: P) -> TestResult
where
    L: AsyncFnMut(&mut T) -> TestResult<bool>,
    P: AsyncFnMut(&mut T) -> TestResult,
{
    let deadline = tokio::time::Instant::now()
        + Duration::from_millis(u64::from(
            oteryn_protocol_oteryn::logout::MAX_LOGOUT_RETRY_AFTER_MS,
        ))
        + Duration::from_secs(30);
    let harness = verdict::Harness::global();
    harness.bounded_wait(deadline, "w22b-terminal-release");
    let result: Result<TestResult, _> = tokio::time::timeout_at(deadline, async {
        loop {
            if logout(state).await? {
                return Ok(());
            }
            poll(state).await?;
        }
    })
    .await;
    harness.bounded_wait(
        tokio::time::Instant::now(),
        "w22b-terminal-release-complete",
    );
    result.map_err(|_| {
        verdict::HarnessError::boxed(
            verdict::Cause::Deadline {
                phase: "w22b-terminal-release",
            },
            "native combat/release deadline",
        )
    })?
}

async fn settled_fixture<T, S, A>(state: &mut T, mut settle: S, mut apply: A) -> TestResult
where
    S: AsyncFnMut(&mut T) -> TestResult,
    A: AsyncFnMut(&mut T) -> TestResult,
{
    // Terminal settlement must drain the old controller's pending stamina before
    // an independently fenced fixture changes its durable before-state.
    settle(state).await?;
    apply(state).await?;
    settle(state).await
}

async fn fixture_reentry(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    profile: &'static str,
    tags: [u8; 2],
) -> TestResult {
    let mut tags = tags.into_iter();
    settled_fixture(
        client,
        async |client: &mut DevClientSession| {
            release(client).await?;
            let tag = tags.next().ok_or("fixture exceeded its two readmissions")?;
            *client = connect_character(clients, tag, 1).await?;
            Ok(())
        },
        async |client: &mut DevClientSession| {
            let deadline = tokio::time::Instant::now() + Duration::from_secs(65);
            verdict::Harness::global().bounded_wait(deadline, "w22b-fixture");
            let result = verdict::indexed_timeout(
                Duration::from_secs(65),
                full_e2ef5::apply_fixture(client, profile),
            )
            .await
            .map_err(|_| "W22 fixture deadline")?;
            verdict::Harness::global()
                .bounded_wait(tokio::time::Instant::now(), "w22b-fixture-complete");
            result
        },
    )
    .await
}

async fn release(client: &mut DevClientSession) -> TestResult {
    release_with(
        client,
        async |client: &mut DevClientSession| {
            let reply = client.session_mut().logout().await?;
            match reply.result {
                Some(native::logout::LogoutResult::Accepted)
                    if reply.status == CommandStatus::Accepted =>
                {
                    Ok(true)
                }
                Some(
                    native::logout::LogoutResult::InFight { .. }
                    | native::logout::LogoutResult::Busy,
                ) if reply.status == CommandStatus::Rejected => Ok(false),
                _ => Err(format!("logout refusal:{reply:?}").into()),
            }
        },
        async |client: &mut DevClientSession| {
            // A registered TURN keeps ordinary gameplay activity alive without
            // refreshing the fight lock. Only accepted LOGOUT completes release.
            let mut payload = Vec::new();
            bytes_field(&mut payload, 3, &encode_step_intent(StepDirection::North));
            let reply = client
                .session_mut()
                .gated_command(
                    native::target_move::CAPABILITY_TARGET_MOVE_V1,
                    native::target_move::COMMAND_TYPE_TARGET_MOVE_INTENT,
                    &payload,
                )
                .await?;
            if !matches!(
                (reply.status, reply.payload.as_slice()),
                (CommandStatus::Accepted, [8, 1]) | (CommandStatus::Rejected, [8, 3])
            ) {
                return Err(format!("release activity refused:{reply:?}").into());
            }
            client.service_liveness(Duration::from_millis(250)).await?;
            client.take_events();
            Ok(())
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn r4_release_allows_the_native_pve_combat_lock_to_expire() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()?
            .block_on(async {
                tokio::time::pause();
                let started = tokio::time::Instant::now();
                let mut until = started
                    + Duration::from_millis(u64::from(
                        oteryn_protocol_oteryn::logout::MAX_LOGOUT_RETRY_AFTER_MS,
                    ));
                let result = release_with(
                    &mut until,
                    async |until: &mut tokio::time::Instant| {
                        Ok(tokio::time::Instant::now() >= *until)
                    },
                    async |_: &mut tokio::time::Instant| {
                        tokio::time::sleep(Duration::from_millis(250)).await;
                        Ok(())
                    },
                )
                .await;
                assert!(
                    result.is_ok(),
                    "release abandoned a legitimate native combat lock: {result:?}"
                );
            });
        Ok(())
    }

    #[test]
    fn r4_fixture_runs_only_after_terminal_stamina_settlement() -> TestResult {
        struct State {
            durable: u16,
            pending_before: Option<u16>,
            trace: Vec<&'static str>,
        }
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()?
            .block_on(async {
                let mut state = State {
                    durable: 2520,
                    pending_before: Some(2520),
                    trace: Vec::new(),
                };
                let result = settled_fixture(
                    &mut state,
                    async |state: &mut State| {
                        if let Some(before) = state.pending_before.take() {
                            if before != state.durable {
                                return Err("BuildStateMismatch".into());
                            }
                            state.durable -= 1;
                        }
                        state.trace.push("settled-and-readmitted");
                        Ok(())
                    },
                    async |state: &mut State| {
                        state.durable = 840;
                        state.trace.push("fixture");
                        Ok(())
                    },
                )
                .await;
                assert!(
                    result.is_ok(),
                    "fixture overtook an unflushed native debit: {result:?}"
                );
                assert_eq!(state.durable, 840);
                assert_eq!(
                    state.trace,
                    [
                        "settled-and-readmitted",
                        "fixture",
                        "settled-and-readmitted"
                    ]
                );
            });
        Ok(())
    }

    #[test]
    fn review_stamina_uses_general_stats_little_endian_field() -> TestResult {
        use oteryn_protocol_oteryn::cyclopedia_character::{GeneralStats, TabView};
        let bytes = TabView::GeneralStats(GeneralStats {
            stamina_minutes: 839,
            ..Default::default()
        })
        .encode()
        .map_err(|_| "encode stats")?;
        assert_eq!(decode_stamina(&bytes)?, 839);
        Ok(())
    }
    #[test]
    fn review_refused_or_stale_corpse_is_not_loot() {
        assert!(!corpse_open_proven(false, false, true, false));
        assert!(!corpse_open_proven(true, true, false, true));
        assert!(!corpse_open_proven(true, true, true, false));
        assert!(corpse_open_proven(true, true, true, true));
    }
    #[test]
    fn review_modifier_requires_a_discriminating_exact_outcome() {
        let base = BTreeMap::from([(3031, 2)]);
        let modified = BTreeMap::from([(3031, 4)]);
        assert!(!modifier_proven(&base, &modified, &base));
        assert!(!modifier_proven(&base, &base, &base));
        assert!(modifier_proven(&modified, &modified, &base));
    }
    #[test]
    fn review_wealth_has_a_passing_path_only_with_active_effect() {
        assert!(wealth_proven(true, true));
        assert!(!wealth_proven(false, true));
        assert!(!wealth_proven(true, false));
    }
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

//! P20: Premium/promotion gates and paid-region expiry relocation.
//!
//! Concrete player scenario derived from matrix P20, the parity audit, and the
//! Premium purchase/gate owners:
//! 1. Walk to Oressa, say `hi`, `yes`, `knight`, and `yes`; assert the unquoted
//!    confirmation and under-level confirmation are refused without a vocation delta.
//! 2. Hunt native rats to level 8, walk back, say `hi`, `knight`, `no`, `yes`,
//!    `knight`, and `yes`; assert decline/refusal leave the build unchanged and the
//!    final confirmation changes the server-reported vocation to knight.
//! 3. Hunt native rats to level 20 and loot 20,000 gold, walk to King Tibianus,
//!    say `hi`, `promotion`, and `yes`; assert the quote and either the Premium
//!    refusal reply or the promoted-vocation/inventory deltas returned by the server.
//! 4. On a successful purchase, repeat `promotion` and `yes`; assert the duplicate
//!    request is refused and does not charge or change the durable build again.
//! 5. Logout and obtain a fresh admission; assert a new game-session id and the
//!    same XP, level, promoted vocation, inventory, and position.
//! 6. Observe the current entitlement expire, then admit again and require the native
//!    Premium-hometown relocation to Thais and the durable change of citizenship.

use super::*;

use oteryn_dev_client::DevClientSession;
use std::collections::BTreeSet;

const STEPS: &[&str] = &["step-1", "step-2", "step-3", "step-4", "step-5", "step-6"];

struct Row {
    completed: BTreeSet<&'static str>,
}

impl Row {
    fn complete(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        self.completed.insert(step);
        evidence(&format!("P20_CHECK {step} PASS observed={observed:?}"));
    }

    fn finish(&self, result: &TestResult) -> &'static str {
        for step in STEPS {
            if !self.completed.contains(step) {
                evidence(&format!(
                    "P20_CHECK {step} FAIL observed=no asserted server reply/delta"
                ));
            }
        }
        if result.is_ok() && self.completed.len() == STEPS.len() {
            "PASS"
        } else {
            "FAIL"
        }
    }
}

async fn progress(client: &mut DevClientSession) -> TestResult<(u64, u32)> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[1, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted || general_stats::validate(&reply.payload, general_stats::selected(client)).is_err() {
        return Err(format!("P20 GeneralStats observed={reply:?}").into());
    }
    Ok((
        u64::from_le_bytes(reply.payload[1..9].try_into()?),
        u32::from_le_bytes(reply.payload[9..13].try_into()?),
    ))
}

fn is_expected_promotion(before: &str, after: &str) -> bool {
    let Some((identity, vocation)) = before.strip_suffix(").").and_then(|text| text.rsplit_once(", ")) else {
        return false;
    };
    let promoted = match vocation {
        "knight" => "elite knight",
        "paladin" => "royal paladin",
        "druid" => "elder druid",
        "sorcerer" => "master sorcerer",
        "monk" => "exalted monk",
        _ => return false,
    };
    after == format!("{identity}, {promoted}).")
}

async fn vocation(client: &mut DevClientSession) -> TestResult<String> {
    use oteryn_protocol_oteryn::item_look as wire;
    let entities = client.world_entities().ok_or("P20 own actor view absent")?;
    let own = entities
        .iter()
        .find(|entry| &entry.entity.identity == entities.own_identity())
        .ok_or("P20 own actor identity absent")?
        .entity;
    let selection = wire::ItemLookSelection { extended: true };
    let payload = wire::encode_item_look_intent_for(
        wire::ItemLookIntent {
            target: wire::ItemLookTarget::Creature(own),
        },
        selection,
    )
    .map_err(|error| format!("P20 self LOOK encode observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::ITEM_LOOK_CAPABILITY_ID.ok_or("P20 look capability absent")?,
            wire::ITEM_LOOK_COMMAND_ID.ok_or("P20 look command absent")?,
            &payload,
        )
        .await?;
    match wire::decode_item_look_result_for(&reply.payload, selection)
        .map_err(|error| format!("P20 self LOOK decode observed={error:?}"))?
    {
        wire::ItemLookResult::Text(text) if reply.status == CommandStatus::Accepted => Ok(text),
        observed => Err(format!(
            "P20 self LOOK observed={observed:?} status={:?}",
            reply.status
        )
        .into()),
    }
}

pub(super) async fn approach_king(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let king = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.king_tibianus")
        .ok_or("step-3 compiled world has no King Tibianus placement")?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        king.placements.iter().any(|place| {
            place.floor == at.floor && at.x.abs_diff(place.x).max(at.y.abs_diff(place.y)) <= 3
        })
    })
    .ok_or("step-3 no legitimate walking route to King Tibianus")?;
    walk(client, world, &path).await?;
    client
        .world_entities()
        .and_then(|view| {
            view.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Npc
                    && king.placements.iter().any(|place| {
                        entry.position
                            == ActorPosition {
                                x: i32::from(place.x),
                                y: i32::from(place.y),
                                floor: -i16::from(place.floor),
                            }
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or_else(|| "step-3 King Tibianus actor absent after walking to him".into())
}

pub(super) async fn talk(
    client: &mut DevClientSession,
    npc: oteryn_dev_client::EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    use oteryn_protocol_oteryn::npc_service as wire;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("step-3 {text} encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("step-3 {text} reply decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "step-3 player command={text:?} status={:?} disposition={disposition:?}; need=server acceptance of the native NPC command",
            reply.status
        )
        .into());
    }
    loop {
        let delta = client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        if delta.server_sequence <= reply.server_sequence {
            continue;
        }
        let conversation = wire::decode_npc_conversation(&delta.payload)
            .map_err(|error| format!("step-3 {text} conversation decode: {error:?}"))?;
        if conversation.open.map(|(actor, _)| actor) != Some(npc) {
            return Err(format!("step-3 {text} delta belonged to another NPC").into());
        }
        return Ok(conversation.lines);
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step-5 logout observed={reply:?}").into());
    }
    Ok(())
}

fn expiry_relocation_matches(
    expired: bool, fresh_admission: bool, before: ActorPosition, after: ActorPosition, temple: ActorPosition,
    destination_town: u32,
) -> bool {
    expired && fresh_admission && after != before && after == temple && destination_town == 8
}

async fn premium(client: &mut DevClientSession) -> TestResult<oteryn_protocol_oteryn::premium_overview::Overview> {
    use oteryn_protocol_oteryn::{character_info as info, premium_overview as wire};
    let reply = client.session_mut().gated_command(
        info::CAPABILITY_CHARACTER_INFO_V1, info::COMMAND_TYPE_CHARACTER_INFO, &[wire::OPERATION],
    ).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=premium-query observed={:?}", reply.status).into());
    }
    wire::Overview::decode(&reply.payload).ok_or_else(|| "step=premium-query observed=malformed-entitlement".into())
}

async fn hometown(clients: &SeamClients<'_>) -> TestResult<u32> {
    use sqlx::Connection;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let town: i32 = sqlx::query_scalar(
        "SELECT town_id FROM game_character_home_towns WHERE character_id=encode($1,'hex')::uuid",
    ).bind(clients.characters[1].as_slice()).fetch_one(&mut database).await?;
    Ok(u32::try_from(town)?)
}

async fn prepare_king_purchase(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<u64> {
    let mut build = progress(client).await?;
    while build.1 < 20 {
        travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt_named(client, "Rat").await?;
        let next = progress(client).await?;
        if next.0 <= build.0 || next.1 < build.1 {
            return Err(format!("step=3-leveling observed={build:?}->{next:?};needed=earned-experience").into());
        }
        build = next;
    }
    full_gold::earn(client, world, spawns, "P20 promotion", 20_000).await
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let mut row = Row {
        completed: BTreeSet::new(),
    };
    evidence("P20_CHECK steps-1-2 required-source-vocation-journey-outside-permitted-fixture-route");
    let result: TestResult = verdict::indexed_timeout(Duration::from_secs(1800) + full_gold::allowance(20_000), async {
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
        let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
        let world = load_world(
            bytes,
            &pin,
            WorldId::decode(&clients.world)?,
            ChannelId::decode(&clients.channel)?,
        )?;
        let mut client = connect_world(clients, 0xd0).await?;
        let result: TestResult = verdict::observe(async {
            prepare_king_purchase(&mut client, world, &metadata.spawns).await?;
            let npc = approach_king(&mut client, world, &metadata.npcs).await?;
            let greeting = talk(&mut client, npc, "hi").await?;
            if greeting.is_empty() {
                return Err("step-3 hi produced an empty conversation delta".into());
            }
            let unpromoted = vocation(&mut client).await?;
            let uncharged = client
                .inventory()
                .ok_or("step-3 inventory observation absent")?
                .clone();
            let carried_gold = full_gold::coin_value(&client)?;
            let quote = talk(&mut client, npc, "promotion").await?;
            if quote != ["Do you want to be promoted for 20000 gold coins?"] {
                return Err(format!("step-3 promotion quote observed={quote:?}").into());
            }
            let confirmation = talk(&mut client, npc, "yes").await?;
            if confirmation != ["You are now promoted."] {
                return Err(format!(
                    "step-3 promotion attempt quote={quote:?} confirmation={confirmation:?}; need=a player-reachable Premium entitlement and level/gold prerequisites before the positive promotion path"
                )
                .into());
            }
            let promoted = vocation(&mut client).await?;
            let inventory = client
                .inventory()
                .ok_or("step-4 inventory observation absent")?
                .clone();
            let paid = full_gold::coin_value(&client)?;
            if !is_expected_promotion(&unpromoted, &promoted) || inventory == uncharged || paid.checked_add(20_000) != Some(carried_gold) {
                return Err(format!(
                    "step-3 successful purchase missing vocation/inventory delta vocation={unpromoted:?}->{promoted:?} inventory={uncharged:?}->{inventory:?}"
                )
                .into());
            }
            row.complete("step-3", (&quote, &confirmation, &promoted, carried_gold, paid));

            let duplicate_quote = talk(&mut client, npc, "promotion").await?;
            let duplicate = talk(&mut client, npc, "yes").await?;
            let duplicate_vocation = vocation(&mut client).await?;
            if duplicate != ["You already have this blessing."]
                || duplicate_vocation != promoted
                || client.inventory() != Some(&inventory)
            {
                return Err(format!(
                    "step-4 duplicate observed quote={duplicate_quote:?} reply={duplicate:?} vocation={promoted:?}->{duplicate_vocation:?} inventory_unchanged={}",
                    client.inventory() == Some(&inventory)
                )
                .into());
            }
            row.complete(
                "step-4",
                (&duplicate_quote, &duplicate, &duplicate_vocation),
            );

            let build = progress(&mut client).await?;
            let before = client.world_spatial().actor_position;
            let old_session = client.join_snapshot().game_session_id;
            logout(&mut client).await?;
            client = connect_world(clients, 0xd1).await?;
            if client.join_snapshot().game_session_id == old_session {
                return Err("step-5 fresh admission reused the old game session".into());
            }
            let fresh_build = progress(&mut client).await?;
            let fresh_vocation = vocation(&mut client).await?;
            if fresh_build != build
                || fresh_vocation != promoted
                || client.inventory() != Some(&inventory)
                || client.world_spatial().actor_position != before
            {
                return Err(format!("step-5 fresh admission observed build={fresh_build:?} vocation={fresh_vocation:?} inventory={:?} position={:?}; expected build={build:?} vocation={promoted:?} inventory={inventory:?} position={before:?}", client.inventory(), client.world_spatial().actor_position).into());
            }
            row.complete(
                "step-5",
                (old_session, build, &promoted, &inventory, before),
            );

            use oteryn_protocol_oteryn::premium_overview::State;
            let current = premium(&mut client).await?;
            let expires = current.expires_at_us.filter(|_| current.state == State::Active && current.valid_for_us > 0)
                .ok_or("step-6 needed=current-Premium-with-observable-expiry")?;
            let source_town = hometown(clients).await?;
            if ![10, 11, 12, 13, 14, 15, 16, 18, 20, 21, 22].contains(&source_town) {
                return Err(format!("step-6 observed=hometown:{source_town};needed=Premium-hometown-fixture").into());
            }
            let temple = world.base().subareas.town(8).and_then(|town| town.temple)
                .map(|at| ActorPosition { x: i32::from(at.x), y: i32::from(at.y), floor: -i16::from(at.floor) })
                .ok_or("step-6 compiled Thais temple absent")?;
            let expired = tokio::time::timeout(Duration::from_secs(120), async {
                loop {
                    let view = premium(&mut client).await?;
                    if view.valid_for_us == 0 && view.observed_at_us >= expires
                        && view.expires_at_us == Some(expires)
                        && matches!(view.state, State::Expired | State::Active)
                    {
                        return Ok::<_, Box<dyn std::error::Error>>(view);
                    }
                    client.service_liveness(Duration::from_millis(250)).await?;
                    client.take_events();
                }
            }).await.map_err(|_| "step-6 needed=observed-Premium-expiry-before-admission")??;
            let pre_expiry_session = client.join_snapshot().game_session_id;
            logout(&mut client).await?;
            client = connect_world(clients, 0xd2).await?;
            let after = client.world_spatial().actor_position;
            let town = hometown(clients).await?;
            if !expiry_relocation_matches(expired.valid_for_us == 0, client.join_snapshot().game_session_id != pre_expiry_session,
                before, after, temple, town)
            {
                return Err(format!("step-6 observed=expiry:{expired:?},position:{before:?}->{after:?},town:{town};needed=Thais-temple:{temple:?}").into());
            }
            row.complete("step-6", (before, after, temple, source_town, town));
            Ok(())
        })
        .await;
        let _ = logout(&mut client).await;
        result
    })
    .await
    .unwrap_or_else(|_| Err("P20 journey deadline after native mechanic commands".into()));

    let status = row.finish(&result);
    let evidence_text = if result.is_ok() {
        "observed-all-required-actions"
    } else {
        "runtime-step-failure-see-P20_GAP"
    };
    if let Err(error) = result {
        evidence(&format!("P20_GAP {error}"));
    }
    evidence(&format!(
        "e2e_all scenario=P20.full status={status} seconds={:.3} evidence={evidence_text}",
        started.elapsed().as_secs_f64()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promotion_expiry_relocation_requires_actual_expiry_and_a_later_admission() {
        let before = ActorPosition { x: 100, y: 100, floor: 0 };
        let temple = ActorPosition { x: 200, ..before };
        assert!(!expiry_relocation_matches(false, true, before, temple, temple, 8));
        assert!(!expiry_relocation_matches(true, false, before, temple, temple, 8));
        assert!(!expiry_relocation_matches(true, true, before, temple, temple, 11));
        assert!(expiry_relocation_matches(true, true, before, temple, temple, 8));
    }

    #[test]
    fn verdict_requires_every_numbered_step() {
        for missing in STEPS {
            let mut row = Row {
                completed: BTreeSet::new(),
            };
            for step in STEPS {
                if step != missing {
                    row.completed.insert(step);
                }
            }
            assert_eq!(row.finish(&Ok(())), "FAIL");
        }

        let row = Row {
            completed: STEPS.iter().copied().collect(),
        };
        assert_eq!(row.finish(&Ok(())), "PASS");
        assert_eq!(row.finish(&Err("observed server refusal".into())), "FAIL");
    }
}

#[cfg(test)]
mod delta_tests {
    use super::*;

    #[test]
    fn wrong_promoted_vocation_does_not_complete_purchase() {
        let before = "You see yourself (Level 20, knight).";
        assert!(!is_expected_promotion(before, "You see yourself (Level 20, royal paladin)."));
        assert!(is_expected_promotion(before, "You see yourself (Level 20, elite knight)."));
    }
}

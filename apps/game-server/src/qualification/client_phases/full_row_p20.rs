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
//! 6. That fresh admission is the paid-region-expiry relocation attempt. Assert a
//!    server relocation delta to the free-town temple. The current server has no
//!    player command/content mapping that places a character on a Premium special
//!    tile and no admission relocation consumer; after the preceding real commands,
//!    report that runtime-observed missing path rather than inventing a relocation.

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

async fn approach_king(
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

async fn talk(
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

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let mut row = Row {
        completed: BTreeSet::new(),
    };
    let result: TestResult = verdict::indexed_timeout(Duration::from_secs(90), async {
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
            if promoted == unpromoted || inventory == uncharged {
                return Err(format!(
                    "step-3 successful purchase missing vocation/inventory delta vocation={unpromoted:?}->{promoted:?} inventory={uncharged:?}->{inventory:?}"
                )
                .into());
            }
            evidence(&format!(
                "P20_CHECK step-3 FAIL observed=quote-{quote:?}-confirmation-{confirmation:?}-vocation-{promoted:?}-inventory-{inventory:?}; native level-20 rat hunt and 20000-gold loot actions did not run"
            ));

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

            let towns = &world.base().subareas;
            let temple = towns
                .default_town
                .and_then(|id| towns.town(id))
                .and_then(|town| town.temple)
                .map(|at| ActorPosition {
                    x: i32::from(at.x),
                    y: i32::from(at.y),
                    floor: -i16::from(at.floor),
                })
                .ok_or("step-6 compiled free-town temple absent")?;
            let after = client.world_spatial().actor_position;
            if after == before || after != temple {
                return Err(format!("step-6 paid-region expiry relocation observed={before:?}->{after:?} expected_free_town_temple={temple:?}; need=Premium-area special-tile mapping and free-town admission relocation consumer; source:crates/gameplay/dispatch/src/gameplay_transport/spell_premium_coordinator.rs:525").into());
            }
            row.complete("step-6", (before, after, temple));
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

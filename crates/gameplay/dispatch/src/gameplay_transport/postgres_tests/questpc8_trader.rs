//! Selected six-leg Trader journey through production NPC_TALK, fee, MINT and Quest owners.
use super::fixture::{Harness, debug, id};
use super::{TestResult, quest_world_callbacks};
use crate::foundation::{
    MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority, UseCommand},
};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
#[path = "questpc8_trader/world.rs"]
mod fixture_world;
const STAGE: &str = "oteryn:quest-progress/chosen/travelling-trader/stage";
const QUEST: &str = "oteryn:quest.the_travelling_trader_quest";
async fn sell_intent(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    npc: EntityRef,
) -> TestResult<oteryn_protocol_oteryn::npc_service::NpcTradeIntent> {
    use oteryn_protocol_oteryn::npc_service::{NpcTradeIntent, NpcTradeSide};
    let book = owner.states.chat.npc_talk().ok_or("NPC book")?;
    let entry = book.get("oteryn:npc.rashid").ok_or("Rashid")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let eligible: Vec<_> = entry
        .offers()
        .iter()
        .filter(|offer| {
            crate::gameplay_transport::item_ref_admission::generation_item_facts(
                native,
                &crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: offer.item.key.clone(),
                    revision_ref: offer.item.revision.clone(),
                },
            )
            .is_ok_and(|facts| {
                facts.weight.is_some()
                    && !matches!(
                        facts.stack,
                        crate::durability::item_transfer::ItemStackClass::Unknown
                    )
            })
        })
        .cloned()
        .collect();
    let runtime = owner.runtime.lock().await;
    let index = crate::gameplay_transport::item_ref_admission::pinned_index(
        &runtime,
        owner.active_generation,
    )
    .ok_or("item index")?;
    let (offer_index, (shown, _)) = oteryn_gameplay_npc::npc_command::projected(&eligible, index)
        .enumerate()
        .find(|(_, (_, source))| source.item.key == "oteryn:item.tibia.i3364")
        .ok_or("Golden Legs source SELL offer")?;
    assert_eq!(shown.side, NpcTradeSide::Sell);
    assert_eq!(shown.unit_price, 30000);
    Ok(NpcTradeIntent {
        npc_actor: npc,
        catalogue_revision: book.catalogue_revision(),
        offer_index: u32::try_from(offer_index)?,
        side: shown.side,
        quantity: 1,
        expected_unit_price: shown.unit_price,
    })
}
async fn sell(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    intent: oteryn_protocol_oteryn::npc_service::NpcTradeIntent,
    number: u64,
    expected: bool,
) -> TestResult {
    use crate::gameplay_transport::npc_trade_command::NpcTradeStep;
    let result = owner
        .npc_trade(
            admitted.runtime_actor.ok_or("actor")?,
            command(owner, admitted, number).await?,
            intent,
        )
        .await;
    match result {
        NpcTradeStep::Traded => assert!(expected, "locked quest must refuse sale"),
        NpcTradeStep::Refused(refusal) => {
            assert!(!expected, "completed journey must admit source sale");
            assert_eq!(refusal.disposition, NpcIntentDisposition::Rejected);
        }
        NpcTradeStep::Unknown => return Err("SELL outcome must be retained".into()),
    }
    Ok(())
}

const NPCS: [(&str, u16, u16, i8); 5] = [
    ("willard", 33214, 31793, -6),
    ("snake_eye", 32657, 32189, -8),
    ("miraia", 33238, 32482, -7),
    ("briasol", 32636, 31667, -8),
    ("uzgod", 32664, 31894, -9),
];
pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    fixture_world::world()
}

fn production_placements(rashid: NpcPlacement) -> TestResult<Vec<NpcPlacements>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/npc-placements/index.json"),
    )?)?;
    let mut found = std::collections::BTreeMap::new();
    for shard in index["shards"].as_array().ok_or("NPC shards")? {
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join(shard.as_str().ok_or("NPC shard")?),
        )?)?;
        for record in document["records"].as_array().ok_or("NPC records")? {
            let declaration = &record["declaration"];
            let key = declaration["npc"].as_str().ok_or("NPC key")?;
            let selected = if key == "oteryn:npc.rashid" {
                Some((rashid.x, rashid.y, rashid.floor))
            } else {
                NPCS.iter()
                    .find(|(name, _, _, _)| key == format!("oteryn:npc.{name}"))
                    .map(|(_, x, y, f)| (*x, *y, *f))
            };
            let Some(expected) = selected else {
                continue;
            };
            let c = &declaration["cell"];
            let actual = (
                u16::try_from(c["x"].as_u64().ok_or("NPC x")?)?,
                u16::try_from(c["y"].as_u64().ok_or("NPC y")?)?,
                -i8::try_from(c["floor"].as_u64().ok_or("NPC floor")?)?,
            );
            if actual != expected {
                continue;
            }
            let p = NpcPlacement {
                x: actual.0,
                y: actual.1,
                floor: actual.2,
                direction: if key == "oteryn:npc.rashid" {
                    MovementFacing::South
                } else {
                    MovementFacing::North
                },
            };
            assert!(
                found
                    .insert(
                        key.to_owned(),
                        NpcPlacements {
                            key: key.into(),
                            placements: vec![p]
                        }
                    )
                    .is_none(),
                "one exact source placement per selected NPC"
            );
        }
    }
    assert_eq!(
        found.len(),
        6,
        "all six source NPCs must be present in production placement authoring"
    );
    Ok(found.into_values().collect())
}

async fn stage(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)")
        .bind(id(41).as_slice()).bind(STAGE).fetch_one(&h.pool).await?)
}
async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3 AND i.lifecycle=1")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn gold(h: &Harness) -> TestResult<i64> {
    Ok(quantity(h, 3031).await?
        + 100 * quantity(h, 3035).await?
        + 10_000 * quantity(h, 3043).await?)
}
async fn fee_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_fees")
            .fetch_one(&h.pool)
            .await?,
    )
}
async fn command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    number: u64,
) -> TestResult<UseCommand> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    Ok(UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: number,
        item_fence: Some(fence.item_fence()),
    })
}
async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    number: u64,
    text: &str,
    expected: NpcIntentDisposition,
) -> TestResult {
    // Literal world callbacks schedule a source read; run the ordinary cadence
    // before the next NPC selector so it observes the committed delivery graph.
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            command(owner, admitted, number).await?,
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert!(
        !outcome.unknown,
        "{number}:{text} must have a retained outcome"
    );
    assert_eq!(
        outcome.disposition, expected,
        "{number}:{text}; {:?}",
        outcome.conversation
    );
    Ok(())
}
async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    next: &mut u64,
    texts: &[&str],
) -> TestResult {
    for text in texts {
        talk(owner, admitted, npc, *next, text, NpcIntentDisposition::Ok).await?;
        *next += 1;
    }
    Ok(())
}
async fn teleport(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    destination: MovementLocalPosition,
) -> TestResult {
    let _permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        admitted.game_session_id,
        before,
        destination,
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}
async fn visit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
) -> TestResult<EntityRef> {
    let (entity, at) = {
        let runtime = owner.runtime.lock().await;
        let npc = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|n| runtime.npc_key(n.actor) == Some(key))
            .ok_or("source NPC")?;
        (
            EntityRef {
                identity: npc.actor.placement_identity(),
                generation: npc.generation,
            },
            runtime
                .read_scene_actor_position(npc.actor)
                .map_err(debug)?
                .position(),
        )
    };
    teleport(owner, admitted, MovementLocalPosition { x: at.x - 1, ..at }).await?;
    Ok(entity)
}
async fn prepare_item(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u32,
    count: u16,
    ordinal: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: u16::try_from(at.x)?,
                y: u16::try_from(at.y)?,
                floor: -i8::try_from(at.floor)?,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count,
        },
        ordinal,
        true,
    )
    .await
}
async fn drop_goldfish(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    next: &mut u64,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
    let item:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key='oteryn:item.tibia.i5929' AND i.lifecycle=1 ORDER BY i.item_instance_id LIMIT 1")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?;
    let item: [u8; 16] = item.try_into().map_err(|_| "item id")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let (at, digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime
                .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
                .map_err(debug)?
                .position(),
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let map = owner.bundle_map().ok_or("map")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let result = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence.item_fence(),
            crate::foundation::CommandRef::new(
                admitted.game_session_id,
                crate::foundation::CommandId::new(*next).map_err(debug)?,
            ),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            LoopMoveRequest {
                item,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            u16::try_from(at.x)?,
                            u16::try_from(at.y)?,
                            -i8::try_from(at.floor)?,
                        ),
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: format!(
                        "sha256:{}",
                        digest
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                    ),
                    placement_context: frame.to_vec(),
                }),
            },
        )
        .await
        .map_err(debug)?;
    *next += 1;
    assert!(result.is_ok(), "drop carried filler: {result:?}");
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let model = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?
    .service_model_for_project()
    .map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model(&model),
    ));
    owner.states.chat.admit(
        admitted.game_session_id,
        "Travelling Hero",
        owner.owner_now(),
    );
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?;
    let rashid = oteryn_gameplay_entities::rashid_schedule::destination_at(now).placement;
    let placements = production_placements(rashid)?;
    owner
        .runtime
        .lock()
        .await
        .place_npcs(&NpcPlacementTable { npcs: placements }, |_| true, true)
        .map_err(debug)?;
    // Direct NPC dispatch receives the same native Character lease binding that
    // ordinary connection admission installs before a mutation can be selected.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "native NPC lease fence")?
        .ok_or("native NPC lease fence")?;
    {
        let actor = admitted.runtime_actor.ok_or("actor")?;
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(
                actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let bound = runtime
            .bound_attacker_lease(actor, admitted.game_session_id)
            .map_err(debug)?;
        assert_eq!(
            bound.character_id().as_bytes(),
            fence.character_id.as_bytes()
        );
        assert_eq!(bound.generation(), fence.character_lease_generation);
    }
    let mut next = 1;
    let rashid = visit(owner, admitted, "oteryn:npc.rashid").await?;
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    say(owner, admitted, rashid, &mut next, &["hi", "mission"]).await?;
    talk(
        owner,
        admitted,
        rashid,
        next,
        "yes",
        NpcIntentDisposition::Rejected,
    )
    .await?;
    next += 1;
    assert_eq!(stage(h).await?, -1);
    owner.premium_coordinator = premium;
    assert!(
        owner.premium_gate(id(40)).await.premium_current(),
        "restored native Premium delivery must settle before the transaction-scoped quest gate"
    );
    // External carrying capacity comes from the ordinary XP owner; no quest track is seeded.
    super::questpc8_magic_sword::raise_level(owner, admitted, 100, 201).await?;
    prepare_item(owner, admitted, 3364, 1, 209).await?;
    prepare_item(owner, admitted, 3364, 1, 208).await?;
    say(owner, admitted, rashid, &mut next, &["hi", "trade"]).await?;
    let early_sell = sell_intent(owner, rashid).await?;
    sell(owner, admitted, early_sell, next, false).await?;
    next += 1;
    assert_eq!(quantity(h, 3364).await?, 2);
    assert_eq!(gold(h).await?, 0);

    say(owner, admitted, rashid, &mut next, &["hi", "mission"]).await?;
    talk(
        owner,
        admitted,
        rashid,
        next,
        "yes",
        NpcIntentDisposition::Rejected,
    )
    .await?;
    next += 1;
    assert_eq!(stage(h).await?, -1, "missing trophy cannot advance");
    prepare_item(owner, admitted, 7397, 1, 210).await?;
    prepare_item(owner, admitted, 5880, 3, 211).await?;
    prepare_item(owner, admitted, 5929, 1, 212).await?;
    assert_eq!(
        quantity(h, 7397).await?,
        1,
        "the native trophy hand-in needs actual carried custody"
    );
    assert_eq!(quantity(h, 5880).await?, 3);
    assert_eq!(quantity(h, 5929).await?, 1);
    assert_eq!(stage(h).await?, -1);
    eprintln!("QUESTPC8_TRADER_FIRST_HANDIN trophy=1 ore=3 goldfish=1 native_premium_current=true");

    say(
        owner,
        admitted,
        rashid,
        &mut next,
        &["hi", "mission", "yes"],
    )
    .await?;
    assert_eq!(stage(h).await?, 1);
    assert_eq!(quantity(h, 7397).await?, 0);
    let briasol = visit(owner, admitted, "oteryn:npc.briasol").await?;
    say(owner, admitted, briasol, &mut next, &["hi", "fine vase"]).await?;
    assert_eq!(stage(h).await?, 1, "out-of-order vase offer cannot advance");
    let willard = visit(owner, admitted, "oteryn:npc.willard").await?;
    say(
        owner,
        admitted,
        willard,
        &mut next,
        &["hi", "package for rashid"],
    )
    .await?;
    assert_eq!(stage(h).await?, 2);
    let snake = visit(owner, admitted, "oteryn:npc.snake_eye").await?;
    say(
        owner,
        admitted,
        snake,
        &mut next,
        &["hi", "package for rashid"],
    )
    .await?;
    assert_eq!(stage(h).await?, 3);
    teleport(
        owner,
        admitted,
        MovementLocalPosition {
            x: 32660,
            y: 32192,
            floor: 8,
        },
    )
    .await?;
    let target = super::questpc8_authored_programmes::object_target(
        owner,
        TilePos {
            x: 32661,
            y: 32192,
            floor: -8,
        },
        1,
    )?;
    let result = owner
        .use_object(
            admitted.runtime_actor.ok_or("actor")?,
            command(owner, admitted, next).await?,
            target.clone(),
        )
        .await;
    next += 1;
    assert_eq!(result.disposition, UseDisposition::Committed);
    assert_eq!(quantity(h, 145).await?, 1);
    assert_eq!(stage(h).await?, 4);
    let result = owner
        .use_object(
            admitted.runtime_actor.ok_or("actor")?,
            command(owner, admitted, next).await?,
            target,
        )
        .await;
    next += 1;
    assert_ne!(result.disposition, UseDisposition::Committed);
    assert_eq!(quantity(h, 145).await?, 1);
    let rashid = visit(owner, admitted, "oteryn:npc.rashid").await?;
    say(owner, admitted, rashid, &mut next, &["hi", "package"]).await?;
    assert_eq!(stage(h).await?, 5);
    assert_eq!(quantity(h, 145).await?, 0);
    let miraia = visit(owner, admitted, "oteryn:npc.miraia").await?;
    say(owner, admitted, miraia, &mut next, &["hi", "scarab cheese"]).await?;
    talk(
        owner,
        admitted,
        miraia,
        next,
        "yes",
        NpcIntentDisposition::Rejected,
    )
    .await?;
    next += 1;
    assert_eq!(stage(h).await?, 5);
    assert_eq!(fee_count(h).await?, 0);
    prepare_item(owner, admitted, 3035, 20, 213).await?;
    let slots:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    for ordinal in 0..20 - slots {
        prepare_item(owner, admitted, 5929, 1, 220 + u64::try_from(ordinal)?).await?;
    }
    say(owner, admitted, miraia, &mut next, &["hi", "scarab cheese"]).await?;
    talk(
        owner,
        admitted,
        miraia,
        next,
        "yes",
        NpcIntentDisposition::Rejected,
    )
    .await?;
    next += 1;
    assert_eq!(gold(h).await?, 2000, "full output rolls back fee");
    assert_eq!(fee_count(h).await?, 0);
    assert_eq!(stage(h).await?, 5);
    drop_goldfish(owner, admitted, h, &mut next).await?;
    // Command replay belongs to ingress, before NPC topic selection. Exercise
    // the paid purchase and its duplicate on that production wire path.
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[3],
        )
        .ok_or("NPC capability")?;
    admitted.continuity.next_command_id = next;
    super::grizzly_npc::wire::talk(
        owner,
        admitted,
        miraia,
        &[(next, "hi"), (next + 1, "scarab cheese"), (next + 2, "yes")],
    )
    .await?;
    next += 3;
    assert_eq!(gold(h).await?, 1900);
    assert_eq!(stage(h).await?, 6);
    assert_eq!(quantity(h, 169).await?, 1);
    assert_eq!(fee_count(h).await?, 1);
    let replay = next - 1;
    super::grizzly_npc::wire::talk(owner, admitted, miraia, &[(replay, "yes")]).await?;
    assert_eq!(gold(h).await?, 1900);
    assert_eq!(fee_count(h).await?, 1);
    // Canonical recipe adaptation explicitly omits cheese decay. No deadline or paid refresh is part of this programme.
    // Leave room for source subsidy and later fee change; use the ordinary item-loop owner.
    drop_goldfish(owner, admitted, h, &mut next).await?;
    drop_goldfish(owner, admitted, h, &mut next).await?;
    let rashid = visit(owner, admitted, "oteryn:npc.rashid").await?;
    say(owner, admitted, rashid, &mut next, &["hi", "cheese"]).await?;
    assert_eq!(stage(h).await?, 7);
    assert_eq!(quantity(h, 169).await?, 0);
    assert_eq!(gold(h).await?, 2900);
    let briasol = visit(owner, admitted, "oteryn:npc.briasol").await?;
    say(
        owner,
        admitted,
        briasol,
        &mut next,
        &["hi", "fine vase", "yes"],
    )
    .await?;
    assert_eq!(stage(h).await?, 8);
    assert_eq!(quantity(h, 227).await?, 1);
    assert_eq!(gold(h).await?, 1900);
    let rashid = visit(owner, admitted, "oteryn:npc.rashid").await?;
    say(owner, admitted, rashid, &mut next, &["hi", "vase"]).await?;
    assert_eq!(stage(h).await?, 9);
    assert_eq!(quantity(h, 227).await?, 0);
    let uzgod = visit(owner, admitted, "oteryn:npc.uzgod").await?;
    say(
        owner,
        admitted,
        uzgod,
        &mut next,
        &["hi", "crimson sword", "yes"],
    )
    .await?;
    assert_eq!(stage(h).await?, 10);
    assert_eq!(quantity(h, 5880).await?, 0);
    assert_eq!(quantity(h, 7385).await?, 1);
    assert_eq!(gold(h).await?, 1650);
    let rashid = visit(owner, admitted, "oteryn:npc.rashid").await?;
    say(owner, admitted, rashid, &mut next, &["hi", "sword"]).await?;
    assert_eq!(stage(h).await?, 11);
    assert_eq!(quantity(h, 7385).await?, 0);
    let fish = quantity(h, 5929).await?;
    say(owner, admitted, rashid, &mut next, &["goldfish"]).await?;
    assert_eq!(stage(h).await?, 12);
    assert_eq!(quantity(h, 5929).await?, fish - 1);
    say(owner, admitted, rashid, &mut next, &["mission"]).await?;
    assert_eq!(stage(h).await?, 13);
    let done:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert!(done);
    let achievement:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/recognised_trader'").fetch_one(&h.pool).await?;
    assert_eq!(achievement, 1);
    assert_eq!(fee_count(h).await?, 3);
    let exact: Vec<i64> = sqlx::query_scalar(
        "SELECT fee_gold_units FROM game_quest_reward_fees ORDER BY fee_gold_units",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(exact, [100, 250, 1000]);
    say(owner, admitted, rashid, &mut next, &["hi", "trade"]).await?;
    let sale = sell_intent(owner, rashid).await?;
    let sale_command = next;
    next += 1;
    sell(owner, admitted, sale, sale_command, true).await?;
    assert_eq!(quantity(h, 3364).await?, 1);
    assert_eq!(gold(h).await?, 31650);
    sell(owner, admitted, sale, sale_command, true).await?;
    assert_eq!(
        gold(h).await?,
        31650,
        "replayed SELL cannot mint payout twice"
    );
    let trades: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(trades, 1);
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(debug)?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    say(owner, admitted, rashid, &mut next, &["hi", "mission"]).await?;
    assert_eq!(stage(h).await?, 13);
    assert_eq!(gold(h).await?, 31650);
    assert_eq!(fee_count(h).await?, 3);
    say(owner, admitted, rashid, &mut next, &["hi", "trade"]).await?;
    sell(
        owner,
        admitted,
        sell_intent(owner, rashid).await?,
        next,
        true,
    )
    .await?;
    assert_eq!(quantity(h, 3364).await?, 0);
    assert_eq!(gold(h).await?, 61650);
    let trades: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(trades, 2, "cold session retains native trading eligibility");
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_trader six_legs exact_fees premium atomic_full_output cold_replay"
    );
    Ok(())
}

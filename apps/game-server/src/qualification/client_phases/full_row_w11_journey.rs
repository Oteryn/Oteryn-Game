use super::*;
use oteryn_dev_client::{EntityKind, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::{
    activity_scope::ActivityScope,
    attack::{AttackIntentDisposition, ChaseMode, FightModes},
    npc_service,
    world_object::UseDisposition,
};

const SOUL: &[&str] = &[
    "oteryn:creature.goshnar_s_malice",
    "oteryn:creature.goshnar_s_greed",
    "oteryn:creature.goshnar_s_spite",
    "oteryn:creature.goshnar_s_cruelty",
    "oteryn:creature.goshnar_s_hatred",
    "oteryn:creature.goshnar_s_megalomania_purple",
];
const ROTTEN: &[&str] = &[
    "oteryn:creature.murcion",
    "oteryn:creature.vemiath",
    "oteryn:creature.chagorz",
    "oteryn:creature.ichgahal",
    "oteryn:creature.bakragore",
];

pub(super) struct Rewards {
    soul_item: std::num::NonZeroU32,
    boss_chest: Vec<wire::BossRewardChestEntry>,
}

pub(super) async fn complete(
    clients: &SeamClients<'_>,
    client: &mut DevClientSession,
) -> TestResult<Rewards> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;

    speak(
        client,
        world,
        &metadata.npcs,
        "oteryn:npc.flickering_soul",
        &["hi", "task", "yes"],
    )
    .await?;
    check_access(clients, true).await?;
    chain(clients, client, world, SOUL, true).await?;
    let soul_item = claim_soul_reward(client, world).await?;

    speak(
        client,
        world,
        &metadata.npcs,
        "oteryn:npc.bloodshade_sacrifice",
        &["hi", "quest", "detour", "yes", "yes"],
    )
    .await?;
    check_access(clients, false).await?;
    chain(clients, client, world, ROTTEN, false).await?;
    let rewards = Rewards {
        soul_item,
        boss_chest: rewards_page(client).await?,
    };
    verify_rewards(clients, client, &rewards).await?;
    Ok(rewards)
}

async fn check_access(clients: &SeamClients<'_>, soul: bool) -> TestResult {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let statement = if soul {
        "SELECT EXISTS (SELECT 1 FROM game_character_soul_war
         WHERE character_id=encode($1,'hex')::uuid AND teleport_access)"
    } else {
        "SELECT EXISTS (SELECT 1 FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/rotten-blood/access' AND value=4)"
    };
    let granted: bool = sqlx::query_scalar(statement)
        .bind(clients.characters[1].as_slice())
        .fetch_one(&mut db)
        .await?;
    if !granted {
        return Err(if soul {
            "Flickering Soul dialogue did not grant access"
        } else {
            "Bloodshade dialogue did not complete the paid detour"
        }
        .into());
    }
    Ok(())
}

async fn speak(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    key: &str,
    lines: &[&str],
) -> TestResult {
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == key)
        .ok_or_else(|| format!("missing published NPC {key}"))?;
    walk_to(client, world, key, |at| {
        npc.placements.iter().any(|p| {
            let distance = at.x.abs_diff(p.x).max(at.y.abs_diff(p.y));
            p.floor == at.floor && (1..=3).contains(&distance)
        })
    })
    .await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        tile(entity.position)
                            .is_ok_and(|at| (at.x, at.y, at.floor) == (p.x, p.y, p.floor))
                    })
            })
        })
        .ok_or_else(|| format!("published NPC {key} absent from entity frames"))?
        .entity;
    for text in lines {
        let disposition = oteryn_dev_client::client_runtime::npc_shop::talk(
            client.session_mut(),
            &npc_service::NpcTalkIntent {
                npc_actor: actor,
                text: (*text).into(),
            },
        )
        .await?;
        if disposition != npc_service::NpcIntentDisposition::Ok {
            return Err(format!("{key} {text}: {disposition:?}").into());
        }
    }
    Ok(())
}

async fn walk_to(
    client: &mut DevClientSession,
    world: &BundleWorld,
    label: &str,
    goal: impl Fn(TilePos) -> bool,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let budget = std::cell::Cell::new(500_000_usize);
    let path = find_route(from, goal, |at, direction| {
        budget.set(budget.get().checked_sub(1)?);
        destination(world, at, direction)
    })
    .map_err(|visited| {
        format!("W11 route to {label} from {from:?} exhausted after {visited} cells")
    })?;
    walk(client, world, &path).await
}

async fn lever(
    client: &mut DevClientSession,
    world: &BundleWorld,
    key: &str,
) -> TestResult<(CommandStatus, wire::BossActivityReply)> {
    let candidates = crate::content::bosses::activity_template::captured_boss_levers()
        .map_err(|error| format!("pinned lever capture: {error:?}"))?;
    let pads = candidates
        .iter()
        .filter(|row| row.creature_key.as_deref() == Some(key))
        .flat_map(|row| row.lever_positions.iter())
        .collect::<Vec<_>>();
    if pads.is_empty() {
        return Err(format!("no pinned lever pads for {key}").into());
    }
    walk_to(client, world, key, |at| {
        pads.iter()
            .any(|p| **p == [i32::from(at.x), i32::from(at.y), -i32::from(at.floor)])
    })
    .await?;
    let payload =
        wire::encode_boss_key(key).map_err(|error| format!("{key} encoding: {error:?}"))?;
    let reply = command(client, wire::COMMAND_TYPE_BOSS_LEVER, &payload).await?;
    let result = wire::decode_boss_activity_reply(&reply.payload)
        .map_err(|error| format!("{key} lever result {:?}: {error:?}", reply.status))?;
    Ok((reply.status, result))
}

async fn chain(
    clients: &SeamClients<'_>,
    client: &mut DevClientSession,
    world: &BundleWorld,
    keys: &[&str],
    soul: bool,
) -> TestResult {
    let final_key = *keys.last().ok_or("W11 final boss")?;
    let before = client.join_snapshot().game_session_id;
    let refusal = lever(client, world, final_key).await?;
    if refusal
        != (
            CommandStatus::Rejected,
            wire::BossActivityReply::Refused(wire::BossActivityRefusal::AuthorityRejected),
        )
        || client.join_snapshot().game_session_id != before
    {
        return Err(
            format!("{final_key}: expected premature chain refusal, got {refusal:?}").into(),
        );
    }
    for (index, &key) in keys.iter().enumerate() {
        enter_and_defeat(clients, client, world, key).await?;
        let state = taints(client).await?;
        let final_boss = key == final_key;
        let count = if final_boss {
            0
        } else {
            u8::try_from(index + 1)?
        };
        let matches = if soul {
            state.soul_war == count
        } else {
            state.rotten_blood == count
                && state.rotten_blood_flags.count_ones() == u32::from(count)
                && state.rotten_blood_final == final_boss
        };
        if !matches {
            return Err(format!("{key}: unexpected authoritative taints {state:?}").into());
        }
    }
    Ok(())
}

async fn enter_and_defeat(
    clients: &SeamClients<'_>,
    client: &mut DevClientSession,
    world: &BundleWorld,
    key: &str,
) -> TestResult {
    let origin = client.join_snapshot().game_session_id;
    let result = lever(client, world, key).await?;
    let (
        CommandStatus::Accepted,
        wire::BossActivityReply::Accepted {
            handoff,
            deadline_unix_ms: Some(deadline_ms),
        },
    ) = result
    else {
        return Err(format!("{key}: expected lever handoff, got {result:?}").into());
    };
    let deadline = tokio::time::Instant::now()
        + Duration::from_millis(
            deadline_ms.saturating_sub(u64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis(),
            )?),
        );
    verdict::Harness::global().bounded_wait(deadline, "W11 native combat");
    while client.join_snapshot().game_session_id == origin {
        pump(client, deadline, "instance handoff").await?;
    }
    let instance = client.join_snapshot().game_session_id;
    let scope = client
        .session_mut()
        .activity_scope()
        .ok_or("missing activity scope")?;
    if scope.handoff != handoff
        || scope.source_session != *origin.as_bytes()
        || scope.session != *instance.as_bytes()
        || !matches!(scope.scope, ActivityScope::Instance { .. })
    {
        return Err(format!("{key}: wrong activity scope {scope:?}").into());
    }
    let modes = FightModes {
        chase: ChaseMode::Chase,
        ..FightModes::DEFAULT
    };
    if client
        .session_mut()
        .set_fight_modes(&modes)
        .await?
        .disposition
        != AttackIntentDisposition::Ok
    {
        return Err(format!("{key}: chase mode refused").into());
    }
    let megalomania = key == SOUL[SOUL.len() - 1];
    if megalomania {
        // Global Soul War Quest/Spoiler: four Aspects of Power open vulnerability.
        for _ in 0..4 {
            attack(client, deadline, "Aspect of Power").await?;
        }
    }
    let candidate = crate::content::bosses::activity_template::captured_boss_levers()
        .map_err(|error| format!("native boss names: {error:?}"))?;
    let name = if megalomania {
        "Goshnar's Megalomania"
    } else {
        candidate
            .iter()
            .find(|row| row.creature_key.as_deref() == Some(key))
            .and_then(|row| row.boss_name.as_deref())
            .ok_or("published boss name")?
    };
    attack(client, deadline, name).await?;
    // Only the production terminal receipt certifies victory. Leaving the
    // entity viewport or receiving a timeout return would not satisfy it.
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let return_deadline = deadline + Duration::from_secs(70);
    verdict::Harness::global().bounded_wait(return_deadline, "W11 victorious room return");
    while client.join_snapshot().game_session_id == instance {
        pump(client, return_deadline, "victory return").await?;
    }
    let victory: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_boss_room_activities
         WHERE admission=encode($1,'hex')::uuid AND state=4 AND defeat_id IS NOT NULL AND terminal_at<=deadline)"
    ).bind(handoff.as_slice()).fetch_one(&mut db).await?;
    if !victory {
        return Err(format!("{key}: channel return without a committed victory").into());
    }
    verdict::Harness::global().progress("W11 room victory");
    Ok(())
}

async fn pump(
    client: &mut DevClientSession,
    deadline: tokio::time::Instant,
    stage: &str,
) -> TestResult {
    if tokio::time::Instant::now() >= deadline {
        return Err(format!("W11 timeout waiting for {stage}").into());
    }
    client.service_liveness(Duration::from_millis(100)).await?;
    Ok(())
}

async fn attack(
    client: &mut DevClientSession,
    deadline: tokio::time::Instant,
    name: &str,
) -> TestResult {
    let target: EntityRef = loop {
        if let Some(entity) = client.world_entities().and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Creature
                    && entity
                        .display_name
                        .is_some_and(|display| display.as_str().is_ok_and(|text| text == name))
            })
        }) {
            break entity.entity;
        }
        pump(client, deadline, name).await?;
        client.take_events();
    };
    let result = client.session_mut().attack_target(Some(&target)).await?;
    if result.status != CommandStatus::Accepted || result.disposition != AttackIntentDisposition::Ok
    {
        return Err(format!("{name}: native attack refused: {result:?}").into());
    }
    loop {
        if client.take_events().into_iter().any(|event| {
            matches!(event, SessionEvent::WorldSpatialEntities(delta) if delta.value.leave.contains(&target))
        }) {
            return Ok(());
        }
        pump(client, deadline, name).await?;
    }
}

async fn claim_soul_reward(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<std::num::NonZeroU32> {
    // Crystal (Global silent), action-reward_soul_war.lua:4–28.
    let chest = TilePos {
        x: 33620,
        y: 31400,
        floor: -10,
    };
    walk_to(client, world, "Brass-Shod Chest", |at| {
        at.floor == chest.floor && at.x.abs_diff(chest.x).max(at.y.abs_diff(chest.y)) == 1
    })
    .await?;
    use crate::map::view::{EntrySource, MapFacts, compose_tile};
    // Tibia Fandom archive, Brass-Shod Chest: itemid 23740.
    let tile = compose_tile(world.overlay(), world.facts(), chest)
        .map_err(|error| format!("published Soul War chest tile: {error:?}"))?
        .ok_or("published Soul War chest tile missing")?;
    let key = tile
        .entries
        .iter()
        .find_map(|entry| match entry.source {
            EntrySource::Base { placement_key, .. } if entry.facts.appearance_id == 23740 => {
                Some(placement_key)
            }
            _ => None,
        })
        .ok_or("published Brass-Shod Chest placement")?;
    let revision = world.facts().object_revision(key);
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let index = provisioning::item_index()?;
    let refs = oteryn_gameplay_quests::quest::soul_war::FINAL_REWARD_ITEM_IDS
        .into_iter()
        .map(|id| provisioning::item_reference(&index, id))
        .collect::<TestResult<Vec<_>>>()?;
    let before = soul_inventory(client, &refs);
    let claimed = client.use_object(&placement, revision).await?;
    let replay = client.use_object(&placement, revision).await?;
    if claimed.disposition != UseDisposition::Committed
        || replay.disposition != UseDisposition::Rejected
    {
        return Err(format!("Soul War claim dispositions: {claimed:?} and {replay:?}").into());
    }
    let mut added = soul_inventory(client, &refs);
    for old in before {
        if let Some(index) = added.iter().position(|entry| *entry == old) {
            added.swap_remove(index);
        }
    }
    match added.as_slice() {
        &[item] => Ok(item),
        _ => Err(
            format!("expected one new Soul Set item in the carried inventory, got {added:?}")
                .into(),
        ),
    }
}

fn soul_inventory(
    client: &DevClientSession,
    refs: &[std::num::NonZeroU32],
) -> Vec<std::num::NonZeroU32> {
    client
        .inventory()
        .into_iter()
        .flat_map(|inventory| inventory.entries.iter())
        .map(|item| item.item_definition_ref)
        .filter(|reference| refs.contains(reference))
        .collect()
}

async fn rewards_page(
    client: &mut DevClientSession,
) -> TestResult<Vec<wire::BossRewardChestEntry>> {
    let reply = command(client, wire::COMMAND_TYPE_BOSS_REWARD_OPEN, &[]).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("boss reward view refused: {reply:?}").into());
    }
    Ok(client_boss::decode_rewards(&reply.payload)
        .ok_or("private reward view")?
        .entries)
}

pub(super) async fn verify_rewards(
    clients: &SeamClients<'_>,
    client: &mut DevClientSession,
    rewards: &Rewards,
) -> TestResult {
    if soul_inventory(client, &[rewards.soul_item]).is_empty()
        || rewards_page(client).await? != rewards.boss_chest
    {
        return Err("changed Soul War item or boss reward view".into());
    }
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let rows: (i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT count(*) FROM game_reward_claim_mint_receipts
          WHERE character_id=encode($1,'hex')::uuid AND claim_production_key='oteryn:reward-claim.soul-war-final'),
         (SELECT count(*) FROM game_account_appearance_unlock_receipts
          WHERE granting_character_id=encode($1,'hex')::uuid AND unlock_key='oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base'),
         (SELECT count(*) FROM game_character_rotten_blood
          WHERE character_id=encode($1,'hex')::uuid AND completed AND final_taint)"
    ).bind(clients.characters[1].as_slice()).fetch_one(&mut db).await?;
    if rows != (1, 1, 1) {
        return Err(format!(
            "expected one Soul War claim, one outfit grant and completed Rotten Blood, got {rows:?}"
        )
        .into());
    }
    Ok(())
}

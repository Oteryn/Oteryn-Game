//! W06 drives Crystal 201189f8's Echo Warden journey through ordinary attack and STEP.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind};
use oteryn_protocol_oteryn::charm_progression::{
    CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1, COMMAND_TYPE_CHARM_PROGRESSION, catalogue,
};
use oteryn_protocol_oteryn::world_spatial_entities::{EntityRef, WorldSpatialEntity};
use std::collections::BTreeSet;
use tokio::time::Instant;

const PORTAL_WAIT: Duration = Duration::from_secs(35);
const KILLS: usize = 3;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "portal-raid-and-first-species-charm".to_owned()),
        Ok(Err(error)) => ("FAIL", error.to_string()),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=W06.full status={status} seconds={:.3} evidence={}",
        started.elapsed().as_secs_f64(),
        detail.split_whitespace().collect::<Vec<_>>().join("-")
    ));
}

fn visible(client: &DevClientSession) -> impl Iterator<Item = &WorldSpatialEntity> {
    client
        .world_entities()
        .into_iter()
        .flat_map(|view| view.iter())
}

fn is_rat(entity: &WorldSpatialEntity) -> bool {
    entity.kind == EntityKind::Creature
        && entity
            .display_name
            .is_some_and(|name| name.as_str() == Ok("Rat"))
}

fn portal_position(client: &DevClientSession, portal: u32) -> TestResult<Option<TilePos>> {
    visible(client)
        .find(|entity| {
            entity.kind == EntityKind::GroundItem
                && matches!(
                    entity.detail,
                    EntityDetail::Object { item_definition_ref, .. }
                    | EntityDetail::ObjectWithAppearance { item_definition_ref, .. }
                        if item_definition_ref == portal
                )
        })
        .map(|entity| tile(entity.position))
        .transpose()
}

async fn wait_for_portal(
    client: &mut DevClientSession,
    portal: u32,
) -> TestResult<Option<TilePos>> {
    let deadline = Instant::now() + PORTAL_WAIT;
    loop {
        if let Some(position) = portal_position(client, portal)? {
            return Ok(Some(position));
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(100)))
            .await?;
        client.take_events();
    }
}

async fn bestiary_page(
    client: &mut DevClientSession,
    after: u16,
    race: u16,
) -> TestResult<catalogue::Page> {
    let request = catalogue::encode_echo_request(catalogue::Request {
        after,
        race,
        category: 0,
    })
    .map_err(|error| format!("W06 Bestiary request: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
            COMMAND_TYPE_CHARM_PROGRESSION,
            &request,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("W06 Bestiary reply: {:?}", reply.status).into());
    }
    catalogue::decode_page(&reply.payload)
        .map_err(|error| format!("W06 Bestiary page: {error:?}").into())
}

async fn rat_card(client: &mut DevClientSession) -> TestResult<catalogue::Card> {
    let mut after = 0;
    loop {
        let page = bestiary_page(client, after, 0).await?;
        if let Some(card) = page.cards.into_iter().find(|card| card.name == "Rat") {
            return Ok(card);
        }
        after = page
            .next
            .filter(|next| *next > after)
            .ok_or("W06 Rat missing from the native Bestiary catalogue")?;
    }
}

async fn completed(client: &mut DevClientSession, race: u16) -> TestResult<bool> {
    bestiary_page(client, 0, race)
        .await?
        .cards
        .into_iter()
        .find(|card| card.race == race && card.name == "Rat")
        .and_then(|card| card.echo_warden_completed)
        .ok_or_else(|| "W06 Rat card omitted the Echo Warden completion flag".into())
}

fn in_melee(from: TilePos, target: TilePos) -> bool {
    from.floor == target.floor && from.x.abs_diff(target.x).max(from.y.abs_diff(target.y)) <= 1
}

async fn kill(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: WorldSpatialEntity,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let goal = tile(target.position)?;
    let path =
        route(world, from, |at| in_melee(at, goal)).ok_or("W06 no route to the exact Rat actor")?;
    walk(client, world, &path).await?;
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("W06 attack refused: {attack:?}").into());
    }
    let deadline = Instant::now() + Duration::from_secs(75);
    let mut damaged = false;
    let mut last_position = target.position;
    let mut pursuit_steps = 0_u32;
    loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) = event
                && let Some(updated) = delta
                    .value
                    .update
                    .iter()
                    .find(|entity| entity.entity == target.entity)
            {
                last_position = updated.position;
                damaged |= matches!(updated.detail,
                    EntityDetail::Actor { health_percent, .. }
                    | EntityDetail::ActorWithIcons { health_percent, .. } if health_percent < 100);
            }
        }
        let alive = visible(client)
            .find(|entity| entity.entity == target.entity)
            .copied();
        let corpse = visible(client)
            .any(|entity| entity.kind == EntityKind::Corpse && entity.position == last_position);
        if damaged && alive.is_none() && corpse {
            verdict::Harness::global().progress("echo-kill");
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "W06 exact Rat kill evidence missing: damaged={damaged} live={} corpse={corpse} pursuit_steps={pursuit_steps}",
                alive.is_some()
            ).into());
        }
        if let Some(actor) = alive {
            last_position = actor.position;
            let from = tile(client.world_spatial().actor_position)?;
            let goal = tile(actor.position)?;
            if !in_melee(from, goal) {
                let path = route(world, from, |at| in_melee(at, goal))
                    .ok_or("W06 no STEP route to the fleeing Rat")?;
                let step = path
                    .first()
                    .ok_or("W06 pursuit route empty outside melee")?;
                tokio::time::timeout_at(deadline, walk(client, world, &[*step])).await??;
                pursuit_steps += 1;
            }
        }
    }
}

async fn raid(
    client: &mut DevClientSession,
    world: &BundleWorld,
    portal: u32,
    position: TilePos,
    race: u16,
) -> TestResult {
    let mut known: BTreeSet<EntityRef> = visible(client)
        .filter(|entity| is_rat(entity))
        .map(|entity| entity.entity)
        .collect();
    let start = tile(client.world_spatial().actor_position)?;
    // Walking off first is necessary when the corpse and the portal appear under the player.
    if start == position {
        let exit = route(world, start, |at| at.floor == start.floor && at != start)
            .ok_or("W06 cannot step off the portal tile")?;
        walk(client, world, &exit).await?;
    }
    let from = tile(client.world_spatial().actor_position)?;
    let path =
        route(world, from, |at| at == position).ok_or("W06 portal tile has no STEP route")?;
    walk(client, world, &path).await?;
    let deadline = Instant::now() + Duration::from_secs(8);
    let first = loop {
        let fresh = visible(client)
            .find(|entity| {
                is_rat(entity)
                    && !known.contains(&entity.entity)
                    && tile(entity.position).is_ok_and(|at| {
                        at.floor == position.floor
                            && at.x.abs_diff(position.x).max(at.y.abs_diff(position.y)) <= 2
                    })
            })
            .copied();
        if let Some(actor) =
            fresh.filter(|_| portal_position(client, portal).is_ok_and(|at| at.is_none()))
        {
            break actor;
        }
        if Instant::now() >= deadline {
            return Err("W06 STEP did not consume the portal and expose a raid actor".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    };
    evidence("W06_CHECK native-portal-step-and-raid-spawn");
    verdict::Harness::global().progress("echo-raid");
    // Crystal gives the Warden and the first minion the same spawn time. Use
    // Bestiary's exact species flag as the authoritative variant/reward proof.
    let mut target = first;
    loop {
        known.insert(target.entity);
        kill(client, world, target).await?;
        if completed(client, race).await? {
            evidence("W06_CHECK first-species-rat-charm-reward");
            return Ok(());
        }
        target = visible(client).find(|entity| is_rat(entity) && !known.contains(&entity.entity)
            && tile(entity.position).is_ok_and(|at| at.floor == position.floor
                && at.x.abs_diff(position.x).max(at.y.abs_diff(position.y)) <= 5))
            .copied()
            .ok_or("W06 raid finished without a Warden reward (needs ECHO_WARDEN_OUTCOME <= 10 for an incomplete Rat Bestiary)")?;
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    use oteryn_gameplay_domain::domain::echo_warden::PORTAL_ITEM_ID;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let index = provisioning::item_index()?;
    let portal = provisioning::item_reference(&index, u32::from(PORTAL_ITEM_ID))?.get();
    let mut client = connect_world(clients, 0xe3).await?;
    let card = rat_card(&mut client).await?;
    let race = card.race;
    if card.echo_warden_completed != Some(false) {
        return Err("W06 Rat Bestiary already has the first-species Warden reward".into());
    }
    for attempt in 1..=KILLS {
        travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        let rat = visible(&client)
            .find(|entity| {
                is_rat(entity)
                    && tile(entity.position).is_ok_and(|at| {
                        tile(client.world_spatial().actor_position).is_ok_and(|here| {
                            at.floor == here.floor
                                && at.x.abs_diff(here.x).max(at.y.abs_diff(here.y)) <= 1
                        })
                    })
            })
            .copied()
            .ok_or("W06 authored route did not expose an adjacent Rat")?;
        kill(&mut client, world, rat).await?;
        if let Some(position) = wait_for_portal(&mut client, portal).await? {
            raid(&mut client, world, portal, position, race).await?;
            if client.session_mut().logout().await?.result
                != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
            {
                return Err("W06 logout refused".into());
            }
            return Ok(());
        }
        evidence(&format!(
            "W06_CHECK kill={attempt} portal-window-elapsed=35s"
        ));
    }
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("W06 logout refused after portal draw miss".into());
    }
    Err("W06 no portal after three real Rat kills and their 35 s observation windows (needs ECHO_WARDEN_PORTAL <= 100)".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pursuit_keeps_the_exact_target_in_same_floor_melee() {
        let from = TilePos {
            x: 100,
            y: 100,
            floor: -7,
        };
        assert!(in_melee(
            from,
            TilePos {
                x: 101,
                y: 101,
                ..from
            }
        ));
        assert!(!in_melee(from, TilePos { x: 102, ..from }));
        assert!(!in_melee(from, TilePos { floor: -8, ..from }));
    }

    #[test]
    fn portal_poll_spans_the_donor_delay_and_keeps_time_to_enter() {
        use oteryn_gameplay_domain::domain::echo_warden::{PORTAL_DELAY_MS, PORTAL_TTL_MS};
        assert!(PORTAL_WAIT >= Duration::from_millis(PORTAL_DELAY_MS + 5_000));
        assert!(PORTAL_WAIT < Duration::from_millis(PORTAL_DELAY_MS + PORTAL_TTL_MS));
    }
}

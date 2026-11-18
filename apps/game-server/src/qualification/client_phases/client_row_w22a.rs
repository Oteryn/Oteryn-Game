//! W22a client replay capture: a player kills a rat, opens its corpse and moves the first rolled
//! stack into the backpack. The corpse view the client holds (`Session::open_container`) before and
//! after the move is session-owned, so (the C17c practice) both views are re-encoded with the
//! production `item_view::encode_open_container`; `crates/session/src/tests/e2e_w22a.rs` replays
//! them through `Session::admit` and `Session::apply_state_delta`. The expected file lists the
//! server rat table range for each rolled definition.
use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityKind, ItemMoveDestination, SessionEvent,
};
use oteryn_protocol_oteryn::item_view::{self, ItemEntry, ItemMoveOutcome, OpenContainer};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "W22A_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

fn rat_ranges() -> TestResult<BTreeMap<u32, (u32, u32)>> {
    let index = provisioning::item_index()?;
    [
        ("oteryn:item.tibia.i3607", 1, 1),
        ("oteryn:item.tibia.i3031", 1, 4),
    ]
    .into_iter()
    .map(|(key, min, max)| {
        let revision = index
            .revision(key)
            .ok_or("step=table rat item revision missing")?;
        let reference = index
            .definition_ref("Item", key, revision)
            .ok_or("step=table rat item reference missing")?;
        Ok((reference.get(), (min, max)))
    })
    .collect()
}

fn listed(entries: &[ItemEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.item_definition_ref, entry.count))
        .collect::<Vec<_>>()
        .join(";")
}

fn target_corpse(
    target_removed: bool,
    already_visible: bool,
    at_last_position: bool,
    exact_death: bool,
) -> bool {
    target_removed && !already_visible && at_last_position && exact_death
}

#[test]
fn corpse_must_be_new_and_follow_target_removal() {
    assert!(!target_corpse(false, false, true, true));
    assert!(!target_corpse(true, true, true, true));
    assert!(!target_corpse(true, false, false, true));
    assert!(!target_corpse(true, false, true, false));
    assert!(target_corpse(true, false, true, true));
}

pub(super) struct DeathProbe<'a> {
    clients: &'a SeamClients<'a>,
    connection: sqlx::PgConnection,
    since: i64,
}

impl<'a> DeathProbe<'a> {
    pub(super) async fn start(clients: &'a SeamClients<'a>) -> TestResult<Self> {
        use sqlx::Connection as _;
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        let since = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint",
        )
        .fetch_one(&mut connection)
        .await?;
        Ok(Self {
            clients,
            connection,
            since,
        })
    }

    pub(super) async fn corpse_identity(
        &mut self,
        target: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
        exclusive: bool,
    ) -> TestResult<Option<[u8; 16]>> {
        use sha2::{Digest, Sha256};
        let clients = self.clients;
        let deaths: Vec<(i64, String)> = sqlx::query_as(
            "SELECT death_actor_local_id, death_actor_local_generation::text              FROM game_item_mint_receipts WHERE death_world_id=encode($1,'hex')::uuid              AND death_channel_id=encode($2,'hex')::uuid              AND death_scope_ownership_generation=$3::text::numeric              AND corpse_top_damage_character_id=encode($4,'hex')::uuid              AND materialized_at >= $5 AND loot_purpose_key='CORPSE_MATERIALIZATION'              ORDER BY materialized_at LIMIT 32",
        )
        .bind(clients.world.as_slice())
        .bind(clients.channel.as_slice())
        .bind(clients.scope_generation.to_string())
        .bind(clients.characters[1].as_slice())
        .bind(self.since)
        .fetch_all(&mut self.connection)
        .await?;
        if exclusive && deaths.len() > 1 {
            return Err("multiple actor-specific corpse receipts during the analyser fight".into());
        }
        for (id, generation) in deaths {
            let id = u32::try_from(id)?;
            let generation: u64 = generation.parse()?;
            let hash = |domain: &[u8]| -> [u8; 16] {
                let digest = Sha256::new()
                    .chain_update(domain)
                    .chain_update(clients.world)
                    .chain_update(clients.channel)
                    .chain_update(clients.scope_generation.to_be_bytes())
                    .chain_update(id.to_be_bytes())
                    .chain_update(generation.to_be_bytes())
                    .finalize();
                let mut identity = [0; 16];
                identity.copy_from_slice(&digest[..16]);
                identity
            };
            if generation == target.generation
                && hash(b"oteryn:runtime-player-placement:v1") == target.identity
            {
                return Ok(Some(hash(b"oteryn:runtime-corpse-placement:v1")));
            }
        }
        Ok(None)
    }
}

async fn kill_and_open(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<OpenContainer> {
    let position = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry.display_name.is_some_and(|name| {
                        name.as_str()
                            .is_ok_and(|name| name.eq_ignore_ascii_case("Rat"))
                    })
                    && entry.position.floor == position.floor
                    && entry
                        .position
                        .x
                        .abs_diff(position.x)
                        .max(entry.position.y.abs_diff(position.y))
                        <= 1
            })
        })
        .copied()
        .ok_or("step=attack no adjacent rat after the walk")?;
    let mut existing_corpses: BTreeSet<_> = client
        .world_entities()
        .ok_or("step=attack no entities")?
        .iter()
        .filter(|entry| entry.kind == EntityKind::Corpse)
        .map(|entry| entry.entity)
        .collect();
    client.take_events();
    let mut death = DeathProbe::start(clients).await?;
    let reply = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("step=attack refused {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut last_position = target.position;
    let mut target_removed = false;
    let mut exact = None;
    let corpse = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event {
                target_removed |= delta.value.leave.contains(&target.entity);
                if !target_removed {
                    existing_corpses.extend(
                        delta
                            .value
                            .enter
                            .iter()
                            .filter(|entity| entity.kind == EntityKind::Corpse)
                            .map(|entity| entity.entity),
                    );
                }
                if let Some(update) = delta
                    .value
                    .update
                    .iter()
                    .find(|entry| entry.entity == target.entity)
                {
                    last_position = update.position;
                }
            }
        }
        if target_removed && exact.is_none() {
            exact = death.corpse_identity(target.entity, false).await?;
        }
        if let Some(corpse) = client.world_entities().and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Corpse
                    && target_corpse(
                        target_removed,
                        existing_corpses.contains(&entry.entity),
                        entry.position == last_position,
                        exact == Some(entry.entity.identity),
                    )
            })
        }) {
            break *corpse;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=death no corpse frame".into());
        }
    };
    let EntityDetail::Object {
        item_handle: Some(handle),
        ..
    } = corpse.detail
    else {
        return Err("step=corpse the corpse has no item handle".into());
    };
    let used = client.use_item(handle).await?;
    if used.status != CommandStatus::Accepted
        || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("step=open corpse USE refused {used:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(view) = client
            .open_container()
            .filter(|view| view.container_handle == Some(handle))
        {
            return Ok(view.clone());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=open no container delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

fn looted_into_backpack(corpse_removed: bool, expected: u64, observed: u64) -> bool {
    corpse_removed && expected == observed
}

#[test]
fn corpse_removal_alone_does_not_prove_loot_delivery() {
    assert!(!looted_into_backpack(true, 4, 3));
    assert!(!looted_into_backpack(false, 4, 4));
    assert!(looted_into_backpack(true, 4, 4));
}

fn backpack_count(client: &DevClientSession, entry: ItemEntry) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("step=move missing backpack inventory")?
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == entry.item_definition_ref)
        .map(|item| u64::from(item.count.get()))
        .sum())
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let ranges = rat_ranges()?;
    let mut client = connect_character(clients, 0x98, 1).await?;
    let mut before = OpenContainer::default();
    for _attempt in 0..4 {
        travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        before = kill_and_open(&mut client, clients).await?;
        if !before.entries.is_empty() {
            break;
        }
    }
    let first = *before
        .entries
        .first()
        .ok_or("step=roll every rat corpse was empty after four kills")?;
    for entry in &before.entries {
        let (min, max) = ranges
            .get(&entry.item_definition_ref.get())
            .ok_or_else(|| {
                format!(
                    "step=table unexpected definition {}",
                    entry.item_definition_ref
                )
            })?;
        if !(*min..=*max).contains(&entry.count.get()) {
            return Err(format!(
                "step=table count {}:{} outside {min}..={max}",
                entry.item_definition_ref, entry.count
            )
            .into());
        }
    }
    let expected_backpack = backpack_count(&client, first)?
        .checked_add(u64::from(first.count.get()))
        .ok_or("step=move backpack count overflow")?;
    let moved = client
        .move_item(&oteryn_dev_client::ItemMoveIntent {
            source: first.handle,
            destination: ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("step=move {moved:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let after = loop {
        if let Some(view) = client.open_container()
            && view.container_handle == before.container_handle
            && looted_into_backpack(
                view.entries
                    .iter()
                    .all(|entry| entry.handle != first.handle),
                expected_backpack,
                backpack_count(&client, first)?,
            )
        {
            break view.clone();
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=move no correlated corpse removal and backpack gain".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    let before_payload =
        item_view::encode_open_container(&before).map_err(|error| format!("encode {error:?}"))?;
    let after_payload =
        item_view::encode_open_container(&after).map_err(|error| format!("encode {error:?}"))?;
    let table = ranges
        .iter()
        .map(|(reference, (min, max))| format!("{reference}:{min}:{max}"))
        .collect::<Vec<_>>()
        .join(";");
    capture("w22a-before.bin", &before_payload);
    capture("w22a-after.bin", &after_payload);
    capture(
        "w22a-expected.txt",
        format!(
            "before={}\nafter={}\ntable={table}\n",
            listed(&before.entries),
            listed(&after.entries)
        )
        .as_bytes(),
    );
    let _ = client.session_mut().logout().await;
    Ok(format!(
        "before={} after={}",
        listed(&before.entries),
        listed(&after.entries)
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    match verdict::indexed_timeout(
        Duration::from_secs(300),
        verdict::observe(journey(clients, world)),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W22a.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W22a.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W22a.client status=FAIL evidence=timeout"),
    }
}

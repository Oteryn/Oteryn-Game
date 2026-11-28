//! C20b client inventory capture for the separate session replay.
//!
//! A fresh admission supplies the exact inventory snapshot bytes consumed by
//! `crates/session/src/snapshot_state.rs::SnapshotState::install_domains`; subsequent inventory
//! deltas are consumed by `crates/session/src/domain_dispatch.rs::Session::apply_state_delta`,
//! and `crates/session/src/session_access.rs::Session::inventory` exposes the resulting state.

use super::*;

mod journey {
    use super::*;

    pub(super) async fn run_and_capture(
        clients: &SeamClients<'_>,
        world: &BundleWorld,
        spawns: &oteryn_world_bundle::spawn::Table,
    ) -> TestResult {
        use oteryn_protocol_oteryn::{
            CommandStatus, cyclopedia_character as stats, logout::LogoutResult,
        };
        character_prep::declare_seeded(
            "C20b.client",
            "sorcerer level 9",
            "eligible for death item loss",
        );
        let mut client = character_prep::Seeded {
            profile: "magic-rope",
            reason: "eligible for death item loss",
        }
        .connect(clients, 0xcb, 0)
        .await?;
        let stats_reply = client
            .session_mut()
            .gated_command(
                stats::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
                stats::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
                &[stats::CharacterTab::GeneralStats as u8, 0, 0, 1],
            )
            .await?;
        evidence(&format!("C20b general stats result={stats_reply:?}"));
        if stats_reply.status != CommandStatus::Accepted
            || stats_reply.payload.get(9..13) != Some(9_u32.to_le_bytes().as_slice())
        {
            return Err(format!(
                "C20b level-9 fixture was not observed: status={:?} level_bytes={:?}",
                stats_reply.status,
                stats_reply.payload.get(9..13)
            )
            .into());
        }
        let replacement = provisioning::item_reference(&provisioning::item_index()?, 2853)?;
        let initial = client
            .inventory()
            .ok_or("C20b pre-death inventory absent")?;
        let original = initial
            .main_backpack
            .as_ref()
            .ok_or("C20b requires a pre-death backpack")?;
        if original.item_definition_ref == replacement || initial.entries.is_empty() {
            return Err("C20b pre-state must contain a distinct, nonempty backpack".into());
        }
        let effects = |inventory: &oteryn_dev_client::CharacterInventory| {
            inventory.entries.is_empty()
                && inventory
                    .main_backpack
                    .as_ref()
                    .is_some_and(|bag| bag.item_definition_ref == replacement && bag.count.get() == 1)
        };

        let home = world
            .base()
            .subareas
            .default_town
            .and_then(|id| world.base().subareas.town(id))
            .and_then(|town| town.temple)
            .ok_or("C20b default temple missing")?;
        let temple = spatial_basis::current(&mut client)?.position(TilePos { x: home.x, y: home.y, floor: home.floor });
        travel_to_spawn(&mut client, world, spawns, "oteryn:creature.poison_spider").await?;
        let encounter = client.world_spatial().actor_position;
        if encounter == temple {
            return Err("C20b lethal encounter coincides with temple".into());
        }
        let health = client
            .actor_vitals()
            .ok_or("C20b encounter vitals missing")?
            .health;
        client.take_events();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
        verdict::Harness::global().bounded_wait(deadline, "C20b lethal encounter");
        let mut damaged = false;
        loop {
            client.service_liveness(Duration::from_millis(100)).await?;
            for event in client.take_events() {
                match event {
                    oteryn_dev_client::SessionEvent::ActorVitals(delta) => {
                        if delta.value.health < health || damaged {
                            evidence(&format!(
                                "C20b vitals sequence={} revision={} health={} max_health={}",
                                delta.server_sequence,
                                delta.new_revision,
                                delta.value.health,
                                delta.value.max_health
                            ));
                        }
                        damaged |= delta.value.health < health;
                    }
                    oteryn_dev_client::SessionEvent::Inventory(delta) => {
                        evidence(&format!(
                            "C20b inventory sequence={} revision={} backpack={:?} entries={}",
                            delta.server_sequence,
                            delta.new_revision,
                            delta.value.main_backpack,
                            delta.value.entries.len()
                        ));
                    }
                    _ => {}
                }
            }
            if damaged
                && client.world_spatial().actor_position == temple
                && client
                    .actor_vitals()
                    .is_some_and(|v| v.health > 0 && v.health == v.max_health)
                && client.inventory().is_some_and(effects)
            {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                match tokio::time::timeout(
                    Duration::from_secs(5),
                    report_durable_death(clients),
                )
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => evidence(&format!("C20b death diagnostic failed: {error}")),
                    Err(_) => evidence("C20b death diagnostic timed out"),
                }
                return Err(format!(
                    "C20b death outcome missing: damaged={damaged} position={:?} inventory={:?}",
                    client.world_spatial().actor_position,
                    client.inventory()
                )
                .into());
            }
        }

        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        while !client
            .session_mut()
            .combat_state()
            .is_some_and(|state| !state.in_fight)
        {
            if tokio::time::Instant::now() >= deadline {
                return Err("C20b combat lock did not clear after temple respawn".into());
            }
            client.service_liveness(Duration::from_millis(100)).await?;
        }
        let previous = client.join_snapshot().game_session_id;
        let logout = client.session_mut().logout().await?;
        if logout.result != Some(LogoutResult::Accepted) {
            return Err(format!("C20b first logout refused: {logout:?}").into());
        }
        drop(client);
        let mut client = super::super::connect_character(clients, 0xcc, 0).await?;
        let session = client.join_snapshot().game_session_id;
        if session == previous || client.world_spatial().actor_position != temple {
            return Err(
                "C20b relog did not establish a fresh session at the respawn temple".into(),
            );
        }
        let client_session = client.session_mut();
        let (sequence, revision, payload) = client_session
            .raw_inventory_snapshot()
            .ok_or("C20b fresh admission omitted the inventory snapshot")?;
        let inventory = client_session
            .inventory()
            .ok_or("C20b fresh admission did not install inventory")?;
        if !effects(inventory) {
            return Err("C20b fresh admission did not retain the empty replacement bag".into());
        }
        capture_and_replay(
            uuid_text(session.as_bytes()),
            sequence,
            revision,
            payload,
            inventory,
            client_session.selected_capabilities(),
        )
    }

    async fn report_durable_death(clients: &SeamClients<'_>) -> TestResult {
        let mut database = sqlx::PgConnection::connect(clients.url).await?;
        let records: Vec<(String, bool, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT r.death_occurrence_id::text, p.death_occurrence_id IS NOT NULL, \
                    i.destination_corpse_id::text, i.replacement_bag_id::text \
             FROM game_character_death_receipts r \
             LEFT JOIN game_character_pending_respawns p USING (death_occurrence_id) \
             LEFT JOIN game_player_death_item_records i USING (death_occurrence_id) \
             WHERE r.character_id = encode($1, 'hex')::uuid \
               AND r.death_world_id = encode($2, 'hex')::uuid \
               AND r.death_channel_id = encode($3, 'hex')::uuid \
             ORDER BY r.committed_at DESC, r.death_occurrence_id, i.item_instance_id LIMIT 16",
        )
        .bind(clients.characters[0].as_slice())
        .bind(clients.world.as_slice())
        .bind(clients.channel.as_slice())
        .fetch_all(&mut database)
        .await?;
        database.close().await?;
        evidence(&format!(
            "C20b durable death (receipt, pending_respawn, corpse, replacement_bag)={records:?}"
        ));
        Ok(())
    }

    fn capture_and_replay(
        session: String,
        sequence: u64,
        revision: u64,
        payload: &[u8],
        inventory: &oteryn_dev_client::CharacterInventory,
        selected: &[u32],
    ) -> TestResult {
        let directory =
            std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("C20b capture directory disappeared")?;
        let directory = std::path::Path::new(&directory);
        let expected = format!(
            "session={session}\nsequence={sequence}\nrevision={revision}\nbackpack={}\nentries={}\nequipment={}\n",
            usize::from(inventory.main_backpack.is_some()),
            inventory.entries.len(),
            inventory.equipment.len()
        );
        for (name, bytes) in [
            ("c20b-inventory.bin", payload),
            ("c20b-expected.txt", expected.as_bytes()),
        ] {
            let path = directory.join(name);
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            std::io::Write::write_all(&mut file, bytes)?;
        }
        use oteryn_protocol_oteryn::item_view;
        let replay_payload = std::fs::read(directory.join("c20b-inventory.bin"))?;
        let replay_expected = std::fs::read_to_string(directory.join("c20b-expected.txt"))?;
        if replay_expected != expected || replay_payload != payload {
            return Err("C20b capture differs from this admission".into());
        }
        let replay = item_view::decode_character_inventory_with_appearances(
            &replay_payload,
            selected.contains(&item_view::CAPABILITY_ITEM_EQUIP_DROP_V1),
            selected.contains(&item_view::CAPABILITY_ITEM_APPEARANCE_V1),
        )
        .map_err(|error| format!("C20b fresh inventory replay rejected: {error:?}"))?;
        if &replay != inventory {
            return Err("C20b replay custody differs from the admitted client".into());
        }
        evidence(&format!(
            "C20b replay session={session} sequence={sequence} revision={revision}"
        ));
        Ok(())
    }
}

pub(super) async fn run_packet(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        evidence(
            "SEAM_EVIDENCE e2e_all scenario=C20b.client status=FAIL seconds=0.000 evidence=capture-directory-required",
        );
        return;
    }
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(
        Duration::from_secs(480),
        journey::run_and_capture(clients, world, spawns),
    )
    .await
    .map_err(|_| "C20b client capture timed out".into())
    .and_then(|result| result);
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C20b.client status={} seconds={:.3} evidence=fresh-inventory-snapshot",
        if result.is_ok() { "CAPTURED" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
    if let Err(error) = result {
        evidence(&format!("C20b client capture error={error}"));
    }
}

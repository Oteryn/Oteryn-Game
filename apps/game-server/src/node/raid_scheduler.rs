//! World raid scheduling from the verified generated catalogue. Channel effects
//! are consumed by the existing dispatch/monster owner after durable admission.
pub(super) mod world_changes;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::raid_schedule::{
    CalendarRaidScheduleCandidate, LuaRaidScheduleCandidate, RaidScheduleCandidate,
    RaidSchedulerCheck,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{ChannelId, ChannelRuntimeV1, ScopeOwnershipGeneration, WorldId};
use oteryn_foundation::CancellationToken;
use oteryn_gameplay_encounter_rules::raid::{
    CHECK_INTERVAL_MS, CatalogueRaid, CatalogueSchedule, LuaRaidPolicy, RaidCatalogue, RaidError,
};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::sync::Mutex;

pub(super) struct RaidScheduler<'a, 'f, 's> {
    pub root: &'a DurabilityRoot,
    pub character: &'a ReconciledCharacterAuthority<'f, 's>,
    pub node: &'a NodeIncarnationProof,
    pub runtime: &'a Mutex<ChannelRuntimeV1>,
    pub map: Option<&'a crate::map::boot::BundleMap>,
    pub world: WorldId,
    pub channel: ChannelId,
    pub generation: u64,
    pub reset_epoch: u64,
    pub bundle_digest: Option<[u8; 32]>,
    pub active_layers_epoch: Option<u64>,
    pub active_layers: Vec<String>,
}

/// Read only the actual composed movement map, never exit-only compatibility choices.
pub(super) fn world_change_layers(
    cells: &crate::content::NativeEntryMovementCells,
) -> (Option<u64>, Vec<String>) {
    use crate::map::boot::BundleCollisionIndexExt;
    match cells.index() {
        crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) => {
            let base = index.map().overlay.base();
            (base.layer_epoch(), base.active_layers().to_vec())
        }
        _ => (None, Vec::new()),
    }
}

/// A proposal, not a firing grant. Every channel proposes the same identity for
/// this World/key/check window; the existing durable World lock selects once.
fn firing_identity(
    world: WorldId,
    key: &str,
    epoch: u64,
    now_ms: i64,
) -> Result<[u8; 16], RaidError> {
    let now = u64::try_from(now_ms).map_err(|_| RaidError::InvalidTime)?;
    let slot_ms = now / 60_000 * 60_000;
    let mut hash = Sha256::new();
    hash.update(b"oteryn:raids-global:firing:v1\0");
    hash.update(world.as_bytes());
    hash.update(epoch.to_be_bytes());
    hash.update(slot_ms.to_be_bytes());
    hash.update(
        u64::try_from(key.len())
            .map_err(|_| RaidError::Exhausted)?
            .to_be_bytes(),
    );
    hash.update(key.as_bytes());
    let digest = hash.finalize();
    let mut id = [0_u8; 16];
    id.copy_from_slice(&digest[..16]);
    // UUIDv7 framing follows the repository's existing identity owner.
    id[..6].copy_from_slice(&slot_ms.to_be_bytes()[2..]);
    id[6] = 0x70 | (id[6] & 0x0f);
    id[8] = 0x80 | (id[8] & 0x3f);
    Ok(id)
}

fn source_order(raid: &CatalogueRaid) -> (u8, u16) {
    match raid.schedule {
        CatalogueSchedule::LegacyXml { source_order, .. } => (0, source_order),
        CatalogueSchedule::Lua { .. } => (1, 0),
        CatalogueSchedule::Calendar { .. } => (2, 0),
        CatalogueSchedule::Unknown => (3, 0),
    }
}

fn candidates(
    catalogue: &RaidCatalogue,
    scheduler: &RaidScheduler<'_, '_, '_>,
    now_ms: i64,
    bundle: [u8; 32],
) -> Result<Vec<RaidScheduleCandidate>, RaidError> {
    let mut records = catalogue.active().collect::<Vec<_>>();
    records.sort_by_key(|raid| source_order(raid));
    records
        .into_iter()
        .map(|raid| {
            let firing_id =
                firing_identity(scheduler.world, &raid.key, scheduler.reset_epoch, now_ms)?;
            let selected = raid.select(bundle, firing_id)?;
            let mut candidate = RaidScheduleCandidate {
                key: raid.production_key()?,
                definition: selected.definition,
                eligible: raid.requires_world_change.as_ref().is_none_or(|required| {
                    scheduler.active_layers_epoch == Some(scheduler.reset_epoch)
                        && required.satisfied_by(&scheduler.active_layers)
                }),
                calendar: None,
                firing_id,
                global_margin_ms: 0,
                lua: None,
                calendar_window: None,
            };
            match &raid.schedule {
                CatalogueSchedule::LegacyXml { margin_ms, .. } => {
                    candidate.global_margin_ms =
                        u32::try_from(*margin_ms).map_err(|_| RaidError::Exhausted)?;
                }
                CatalogueSchedule::Lua {
                    allowed_weekdays,
                    min_active_players,
                    min_gap_ms,
                    max_checks_per_day,
                    initial_chance_milli_percent,
                    target_chance_per_day_milli_percent,
                    max_chance_per_check_milli_percent,
                    ..
                } => {
                    candidate.lua = Some(LuaRaidScheduleCandidate {
                        policy: LuaRaidPolicy {
                            initial_chance: Some(
                                f64::from(*initial_chance_milli_percent) / 1_000.0,
                            ),
                            target_chance_per_day: f64::from(*target_chance_per_day_milli_percent)
                                / 1_000.0,
                            max_chance_per_check: f64::from(*max_chance_per_check_milli_percent)
                                / 1_000.0,
                            min_gap_ms: *min_gap_ms,
                            allowed_weekdays: allowed_weekdays.clone(),
                            min_active_players: *min_active_players,
                            max_checks_per_day: max_checks_per_day.map(u64::from),
                        },
                    });
                }
                CatalogueSchedule::Calendar {
                    window,
                    interval_ms,
                    repeat,
                } => {
                    candidate.calendar_window = Some(CalendarRaidScheduleCandidate {
                        window: window.ok_or(RaidError::InvalidCalendar)?.window(),
                        interval_ms: interval_ms.ok_or(RaidError::InvalidCalendar)?,
                        repeat: *repeat,
                    });
                }
                CatalogueSchedule::Unknown => return Err(RaidError::InvalidDefinition),
            }
            Ok(candidate)
        })
        .collect()
}

pub(super) async fn run(scheduler: RaidScheduler<'_, '_, '_>, stop: &CancellationToken) {
    let Some(bundle_digest) = scheduler.bundle_digest else {
        return;
    };
    if scheduler.reset_epoch == 0 {
        return;
    }
    let Ok(generation) = ScopeOwnershipGeneration::new(scheduler.generation) else {
        return;
    };
    let catalogue = match RaidCatalogue::embedded() {
        Ok(catalogue) => catalogue,
        Err(error) => {
            eprintln!("event=raid_catalogue state=refused error={error}");
            return;
        }
    };
    let mut next_check = 0_i64;
    loop {
        let now_ms = i64::try_from(oteryn_error_codes::unix_ms()).unwrap_or(i64::MAX);
        world_changes::observe(&scheduler, generation).await;
        if now_ms >= next_check {
            {
                let runtime = scheduler.runtime.lock().await;
                if runtime.binding().scope_generation() != generation
                    || runtime.owner_fence().is_err()
                {
                    return;
                }
            };
            match candidates(&catalogue, &scheduler, now_ms, bundle_digest) {
                Err(error) => eprintln!("event=raid_catalogue state=refused error={error}"),
                Ok(candidates) => {
                    let request = RaidSchedulerCheck {
                        world: scheduler.world,
                        channel: scheduler.channel,
                        generation,
                        reset_epoch: scheduler.reset_epoch,
                        now_ms,
                        bundle_digest,
                        candidates,
                    };
                    match scheduler
                        .root
                        .check_raid_schedules(scheduler.character, scheduler.node, request)
                        .await
                    {
                        Ok(outcome) => {
                            next_check = now_ms.saturating_add(if outcome.retry_activity {
                                1_000
                            } else {
                                CHECK_INTERVAL_MS
                            });
                            for (_, key) in outcome.firings {
                                eprintln!("event=raid_firing key={} state=admitted", key.as_str());
                            }
                        }
                        Err(error) => eprintln!("event=raid_schedule state=retry error={error}"),
                    }
                }
            }
        }
        if super::cancellation::first(tokio::time::sleep(Duration::from_secs(1)), stop.cancelled())
            .await
            .is_none()
        {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> Result<WorldId, Box<dyn std::error::Error>> {
        let mut bytes = [11; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        Ok(WorldId::decode(&bytes)?)
    }

    #[test]
    fn raids_global_firing_identity_is_framed_and_replay_stable()
    -> Result<(), Box<dyn std::error::Error>> {
        let world = world()?;
        let first = firing_identity(world, "raid:badger", 7, 1_800_000)?;
        assert_eq!(first, firing_identity(world, "raid:badger", 7, 1_850_000)?);
        assert!(WorldId::decode(&first).is_ok());
        assert_ne!(first, firing_identity(world, "raid:badger", 8, 1_800_000)?);
        assert_ne!(first, firing_identity(world, "raid:other", 7, 1_800_000)?);
        assert_ne!(first, firing_identity(world, "raid:badger", 7, 1_860_000)?);
        assert!(firing_identity(world, "raid:badger", 7, -1).is_err());
        Ok(())
    }

    #[test]
    fn world_src_furyosa_production_candidates_require_active_epoch_layer()
    -> Result<(), Box<dyn std::error::Error>> {
        let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
            return Ok(());
        };
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || -> Result<(), String> {
                crate::bestiary_postgres_harness::runtime()
                    .map_err(|e| e.to_string())?
                    .block_on(async {
                        let h = crate::bestiary_postgres_harness::Harness::create(
                            admin,
                            "world_src_furyosa",
                            false,
                        )
                        .await?;
                        let result = exercise_furyosa_source(&h).await;
                        let cleanup = h.cleanup().await;
                        result?;
                        cleanup
                    })
                    .map_err(|e| e.to_string())
            })?
            .join()
            .map_err(|_| "furyosa case thread")?
            .map_err(Box::<dyn std::error::Error>::from)?;
        eprintln!("POSTGRES_CASE_COMPLETED WORLD-SRC furyosa source-layer admission");
        Ok(())
    }

    // A tiny pinned test map isolates admission metadata; production geometry remains sourced.
    fn furyosa_test_cells(
        world: WorldId,
        channel: ChannelId,
        epoch: Option<u64>,
        active: bool,
    ) -> crate::bestiary_postgres_harness::TestResult<(
        crate::content::NativeEntryMovementCells,
        [u8; 32],
    )> {
        use oteryn_world_bundle_compiler::bundle as codec;
        let (bytes, mut pins) = crate::map::boot::tests::bundle();
        let mut decoded = codec::read(&bytes)?;
        let mut sector = decoded.sectors[0].clone();
        sector.tiles.retain(|tile| tile.x == 2);
        decoded.layers.push(oteryn_world_bundle::layers::Layer {
            key: "fury_gates/thais".into(),
            palette: decoded.manifest.palette.clone(),
            sectors: vec![sector],
        });
        let bytes = codec::write_with_layers(
            &decoded.manifest,
            &decoded.sectors,
            &decoded.spawns,
            &decoded.npcs,
            &decoded.layers,
        )?;
        pins.digest.copy_from_slice(&bytes[bytes.len() - 32..]);
        let boot = crate::map::boot::BootPins {
            map_revision: format!(
                "sha256:{}",
                pins.digest
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            ),
            bundle: pins.clone(),
            start: crate::map::overlay::TilePos {
                x: 3,
                y: 0,
                floor: -7,
            },
        };
        let checked = crate::map::boot::check(bytes, boot)?;
        let keys: &[&str] = if active { &["fury_gates/thais"] } else { &[] };
        let checked = if let Some(epoch) = epoch {
            checked.compose_epoch_layers(epoch, keys, &["fury_gates/thais"])?
        } else {
            checked.compose_layers_with_exits(keys, &["fury_gates/thais"])?
        };
        let map = checked.boot(world, channel, crate::map::boot::tests::items)?;
        assert_eq!(map.base().exit_layers(), &["fury_gates/thais"]);
        let room = crate::content::qualify_native_entry_room(world)?;
        Ok((map.movement_cells(room.movement_cells())?, pins.digest))
    }

    async fn exercise_furyosa_source(
        h: &crate::bestiary_postgres_harness::Harness,
    ) -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::{CHANNEL, WORLD, debug, id};
        use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
        use oteryn_gameplay_encounter_rules::raid::RaidPolicy;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
        let owner = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            h.node.fact().node_id(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::test(world),
        )
        .map_err(debug)?;
        let runtime = Mutex::new(owner);
        let canonical = RaidCatalogue::embedded().map_err(debug)?;
        let key = crate::content::ProductionKey::new("raid:furyfate.furyosa").map_err(debug)?;
        let source = canonical.get(&key).ok_or("Furyosa source")?;
        assert!(matches!(
            source.schedule,
            CatalogueSchedule::LegacyXml {
                interval_s: 86400,
                margin_ms: 21600000,
                ..
            }
        ));
        assert!(matches!(
            source
                .select([7; 32], id(81))
                .map_err(debug)?
                .definition
                .policy,
            RaidPolicy::Random {
                mean_interval_s: 86400,
                ..
            }
        ));
        // Test-clock amplification gives a certain first due check; canonical values are asserted above.
        let raw: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../content/encounters/raids/catalogue.json"
        ))?;
        let mut record = raw["records"]
            .as_array()
            .ok_or("records")?
            .iter()
            .find(|r| r["key"] == key.as_str())
            .ok_or("source")?
            .clone();
        record["schedule"]["interval_s"] = serde_json::json!(60);
        record["schedule"]["margin_ms"] = serde_json::json!(0);
        let catalogue = RaidCatalogue::from_json(
            &serde_json::json!({"schema":"OTERYN_RAID_CATALOGUE/v1","records":[record]})
                .to_string(),
        )
        .map_err(debug)?;
        let current = (2..10_000_u64)
            .find(|epoch| {
                crate::durability::world_event::changes::roll(world, *epoch, 0)
                    .is_ok_and(|day| day.fury_gates == Some("fury_gates/thais"))
            })
            .ok_or("active Thais Fury epoch")?;
        let previous = current - 1;
        let (old_cells, bundle) = furyosa_test_cells(world, channel, Some(previous), true)?;
        let (active_layers_epoch, active_layers) = world_change_layers(&old_cells);
        let mut scheduler = RaidScheduler {
            root: &h.root,
            character: &authority,
            node: &h.node,
            runtime: &runtime,
            map: None,
            world,
            channel,
            generation: 1,
            reset_epoch: previous,
            bundle_digest: Some(bundle),
            active_layers_epoch,
            active_layers,
        };
        for epoch in [previous, current] {
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')").bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(bundle.as_slice()).execute(&h.pool).await?;
        }
        sqlx::query(
            "INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric)",
        )
        .bind(world.as_bytes().as_slice())
        .bind(current.to_string())
        .execute(&h.pool)
        .await?;
        h.root
            .world_changes(world, current)
            .await?
            .ok_or("current Fury day")?;
        // Yesterday's boot still has an active Fury, but independently current epoch2 rejects it.
        for now_ms in [0, 60_000] {
            let request = RaidSchedulerCheck {
                world,
                channel,
                generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
                reset_epoch: previous,
                now_ms,
                bundle_digest: bundle,
                candidates: candidates(&catalogue, &scheduler, now_ms, bundle).map_err(debug)?,
            };
            assert!(
                h.root
                    .check_raid_schedules(&authority, &h.node, request)
                    .await?
                    .firings
                    .is_empty()
            );
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_schedules")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 0, "stale boot cannot create schedule state");
        scheduler.reset_epoch = current;
        assert!(
            !candidates(&catalogue, &scheduler, 120_000, bundle).map_err(debug)?[0].eligible,
            "old geometry cannot be rebound to the new epoch"
        );
        let (offday, off_digest) = furyosa_test_cells(world, channel, Some(current), false)?;
        assert_eq!(off_digest, bundle);
        (scheduler.active_layers_epoch, scheduler.active_layers) = world_change_layers(&offday);
        for now_ms in [120_000, 180_000] {
            let proposals = candidates(&catalogue, &scheduler, now_ms, bundle).map_err(debug)?;
            assert!(
                !proposals[0].eligible,
                "exit-only metadata never admits Furyosa"
            );
            let request = RaidSchedulerCheck {
                world,
                channel,
                generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
                reset_epoch: current,
                now_ms,
                bundle_digest: bundle,
                candidates: proposals,
            };
            assert!(
                h.root
                    .check_raid_schedules(&authority, &h.node, request)
                    .await?
                    .firings
                    .is_empty()
            );
        }
        let (undated, _) = furyosa_test_cells(world, channel, None, true)?;
        (scheduler.active_layers_epoch, scheduler.active_layers) = world_change_layers(&undated);
        assert!(!candidates(&catalogue, &scheduler, 240_000, bundle).map_err(debug)?[0].eligible);
        let (active, _) = furyosa_test_cells(world, channel, Some(current), true)?;
        (scheduler.active_layers_epoch, scheduler.active_layers) = world_change_layers(&active);
        let proposals = candidates(&catalogue, &scheduler, 240_000, bundle).map_err(debug)?;
        assert!(proposals[0].eligible);
        let request = RaidSchedulerCheck {
            world,
            channel,
            generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
            reset_epoch: current,
            now_ms: 240_000,
            bundle_digest: bundle,
            candidates: proposals,
        };
        assert_eq!(
            h.root
                .check_raid_schedules(&authority, &h.node, request.clone())
                .await?
                .firings
                .len(),
            1
        );
        assert!(
            h.root
                .check_raid_schedules(&authority, &h.node, request)
                .await?
                .firings
                .is_empty()
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_firings")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 1);
        Ok(())
    }

    #[test]
    fn raids_global_source_order_keeps_xml_before_independent_lua()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalogue = RaidCatalogue::embedded()?;
        let mut records = catalogue.active().collect::<Vec<_>>();
        records.sort_by_key(|raid| source_order(raid));
        let mut previous = (0, 0);
        for record in records {
            let next = source_order(record);
            assert!(next >= previous);
            previous = next;
        }
        assert_eq!(previous.0, 2);
        Ok(())
    }
}

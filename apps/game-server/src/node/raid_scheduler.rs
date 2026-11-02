//! World raid scheduling from the verified generated catalogue. Channel effects
//! are consumed by the existing dispatch/monster owner after durable admission.
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
    pub world: WorldId,
    pub channel: ChannelId,
    pub generation: u64,
    pub reset_epoch: u64,
    pub bundle_digest: Option<[u8; 32]>,
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
                eligible: true,
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
                } => {
                    candidate.calendar_window = Some(CalendarRaidScheduleCandidate {
                        window: window.ok_or(RaidError::InvalidCalendar)?.window(),
                        interval_ms: interval_ms.ok_or(RaidError::InvalidCalendar)?,
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
        assert_eq!(previous.0, 1);
        Ok(())
    }
}

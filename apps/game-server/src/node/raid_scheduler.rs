//! Production raid channel driver. Catalogue values are admitted Canary data;
//! absent raids remain explicit parity gaps rather than invented definitions.
use crate::durability::{DurabilityRoot, raid_runner::CommittedRaidAction};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_foundation::CancellationToken;
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{RaidDefinition, RaidPolicy, Wave, WaveEntry};
use std::time::Duration;

fn admitted(key: &ProductionKey) -> Option<RaidDefinition> {
    // Canary 47dfd51f... raids.xml + abdendriel/badger.xml. The rectangle is retained
    // as a content anchor identity; placement within it is intentionally still a gap.
    if key.as_str() != "raid:badger" {
        return None;
    }
    Some(RaidDefinition {
        policy: RaidPolicy::Random {
            mean_interval_s: 500 * 60,
            min_gap_s: 60 * 60,
        },
        despawn_after_ms: None,
        announcements: vec![],
        waves: vec![Wave {
            delay_ms: 1_000,
            entries: vec![WaveEntry {
                creature: ProductionKey::new("creature:badger").ok()?,
                count: 100,
                area_anchor: ProductionKey::new("raid-anchor:32633,31645,7-32715,31708,7").ok()?,
            }],
        }],
    })
}

pub(super) async fn run(
    root: &DurabilityRoot,
    channel: ChannelId,
    generation: u64,
    stop: &CancellationToken,
) {
    let Ok(generation) = ScopeOwnershipGeneration::new(generation) else {
        return;
    };
    loop {
        let now_ms = i64::try_from(oteryn_error_codes::unix_ms()).unwrap_or(i64::MAX);
        if let Ok(firings) = root.due_raid_firings(channel).await {
            for (firing, key) in firings {
                let Some(definition) = admitted(&key) else {
                    continue;
                };
                match root
                    .advance_raid_run(firing, channel, generation, definition, now_ms)
                    .await
                {
                    Ok(Some(CommittedRaidAction::Announcement { text, advance })) => eprintln!(
                        "event=raid_announcement kind={} text={text:?}",
                        if advance { "advance" } else { "warning" }
                    ),
                    Ok(Some(CommittedRaidAction::Wave { wave, inserted })) => {
                        eprintln!("event=raid_wave wave={wave} durable_creatures={inserted}")
                    }
                    Ok(Some(CommittedRaidAction::Complete)) => {
                        eprintln!("event=raid_run state=done")
                    }
                    Ok(Some(CommittedRaidAction::Lost)) => eprintln!("event=raid_run state=lost"),
                    Ok(None) => {}
                    Err(error) => eprintln!("event=raid_run state=retry error={error}"),
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

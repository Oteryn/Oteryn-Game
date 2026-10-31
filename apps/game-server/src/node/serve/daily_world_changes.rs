//! W05b: the server save rolls the day's world changes at Channel boot and applies the admitted
//! ones to the live map (`oteryn_gameplay_world_change::daily`).
//!
//! The draws hang off the committed reset epoch and the World, so a restart within the day rolls
//! the same day again; no extra durable state is needed.

use super::bundle_npcs::NpcBoot;
use super::event;
use crate::foundation::{ChannelRuntimeV1, MovementFacing, NpcPlacement, WorldId};
use oteryn_gameplay_world_change::daily::DailyWorldChanges;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

/// The day after the reset `reset_epoch` of `world`, on the UTC calendar at `unix_ms`.
pub(super) fn roll(world: WorldId, reset_epoch: u64, unix_ms: i64) -> DailyWorldChanges {
    let mut hash = Sha256::new();
    hash.update(b"oteryn.world_change.daily.v1\0");
    hash.update(world.as_bytes());
    let root = GameplayDecisionRoot::from_bytes(hash.finalize().into());
    let mut occurrence = [0; 16];
    occurrence[8..].copy_from_slice(&reset_epoch.to_be_bytes());
    let (month, day) = utc_month_day(unix_ms);
    DailyWorldChanges::roll(reset_epoch, month, day, |purpose| {
        // A purpose is never empty, so the draw cannot fail; a failure would read as "no".
        deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(occurrence),
            purpose,
            0,
        )
        .unwrap_or(u64::MAX)
    })
}

/// Places the day's NPCs the loaded catalogue admits; one that cannot stand is logged.
pub(super) fn apply(
    runtime: &mut ChannelRuntimeV1,
    npc_boot: Option<&NpcBoot>,
    changes: &DailyWorldChanges,
) -> usize {
    let mut placed = 0;
    for (key, (x, y, z)) in changes.npcs() {
        if !npc_boot.is_some_and(|boot| boot.admits(key)) {
            continue;
        }
        let at = NpcPlacement {
            floor: -z,
            y,
            x,
            direction: MovementFacing::South,
        };
        match runtime.place_dynamic_npc(key, at) {
            Ok(_) => placed += 1,
            Err(error) => event(&format!(
                "event=daily_world_change_npc state=refused npc_key={key} reason={error:?}"
            )),
        }
    }
    placed
}

/// The UTC (month, day) of `unix_ms` (days-from-civil inverted).
fn utc_month_day(unix_ms: i64) -> (u8, u8) {
    let days = unix_ms.div_euclid(86_400_000) + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (
        u8::try_from(month).unwrap_or(1),
        u8::try_from(day).unwrap_or(1),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures")]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, NodeId, NpcPlacementTable};
    use crate::gameplay_transport::NpcTalkBook;
    use std::sync::Arc;

    fn world() -> WorldId {
        WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).expect("world")
    }

    fn channel_runtime() -> ChannelRuntimeV1 {
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let node =
            NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
        ChannelRuntimeV1::from_committed_assignment(
            world(),
            channel,
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::test(world()),
        )
        .expect("runtime")
    }

    fn boot(keys: &[&str]) -> NpcBoot {
        NpcBoot::Place {
            table: NpcPlacementTable::default(),
            catalogue_sha256: String::new(),
            admitted: keys.iter().map(|key| (*key).to_owned()).collect(),
            production: true,
            talk: Arc::new(NpcTalkBook::default()),
        }
    }

    fn npc_keys(runtime: &ChannelRuntimeV1) -> Vec<String> {
        runtime
            .visible_entities()
            .npcs
            .iter()
            .filter_map(|npc| runtime.npc_key(npc.actor).map(str::to_owned))
            .collect()
    }

    /// 2031-06-13T12:00:00Z, 2031-07-01T00:00:00Z and 2031-05-31T23:59:59.999Z.
    const JUNE_13: i64 = 1_939_118_400_000;
    const JULY_1: i64 = 1_940_630_400_000;
    const MAY_31_LAST: i64 = 1_938_038_399_999;

    #[test]
    fn w05b_the_boot_roll_is_fixed_per_reset_epoch_and_follows_the_calendar() {
        assert_eq!(utc_month_day(JUNE_13), (6, 13));
        assert_eq!(utc_month_day(JULY_1), (7, 1));
        assert_eq!(utc_month_day(MAY_31_LAST), (5, 31));
        // A restart within the day rolls the same day again.
        assert_eq!(
            roll(world(), 41, JUNE_13),
            roll(world(), 41, JUNE_13 + 3_600_000)
        );
        let june = roll(world(), 41, JUNE_13);
        assert!(june.flower_month && june.grimvale_full_moon);
        let july = roll(world(), 41, JULY_1);
        assert!(!july.flower_month && !july.grimvale_full_moon);
        // Across many epochs the chance rolls land near the donor rates (10/20/33%).
        let days: Vec<_> = (1..=3_000)
            .map(|epoch| roll(world(), epoch, JULY_1))
            .collect();
        let rate = |hit: fn(&DailyWorldChanges) -> bool| days.iter().filter(|d| hit(d)).count();
        assert!((240..=360).contains(&rate(|d| d.thawing)), "thawing");
        assert!(
            (510..=690).contains(&rate(|d| d.their_masters_voice)),
            "voice"
        );
        assert!(
            (880..=1_100).contains(&rate(|d| d.oriental_trader.is_some())),
            "trader"
        );
    }

    #[test]
    fn w05b_rosemarie_stands_on_the_live_map_only_in_june() {
        let admitted = boot(&["oteryn:npc.rosemarie"]);
        let mut runtime = channel_runtime();
        let june = roll(world(), 41, JUNE_13);
        assert_eq!(apply(&mut runtime, Some(&admitted), &june), 1);
        assert_eq!(npc_keys(&runtime), ["oteryn:npc.rosemarie"]);
        let position = runtime
            .visible_entities()
            .npcs
            .first()
            .map(|npc| (npc.position.x, npc.position.y, npc.position.floor));
        assert_eq!(position, Some((32_534, 32_827, 7)));
        for unix_ms in [MAY_31_LAST, JULY_1] {
            let mut runtime = channel_runtime();
            let day = roll(world(), 41, unix_ms);
            assert_eq!(apply(&mut runtime, Some(&admitted), &day), 0);
            assert!(npc_keys(&runtime).is_empty());
        }
        // A catalogue without Rosemarie places nobody.
        let mut runtime = channel_runtime();
        assert_eq!(apply(&mut runtime, Some(&boot(&[])), &june), 0);
        assert_eq!(apply(&mut runtime, None, &june), 0);
        assert!(npc_keys(&runtime).is_empty());
    }
}

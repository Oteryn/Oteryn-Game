//! Display the same equipment bonus rules used by the frozen-death settlement owner.
use super::*;
use oteryn_protocol_oteryn::bosstiary_slots::SlotDetails;
pub(super) fn slot_details(
    bosses: &[String; 2],
    catalogue: &[BosstiaryBoss],
    rows: &[(String, i64, i64)],
    points: u32,
    boosted: &str,
    now: u64,
) -> SlotDetails {
    let mut bonuses = [0; 2];
    for (i, key) in bosses.iter().enumerate() {
        if key.is_empty() || key == boosted {
            continue;
        }
        let mastered = catalogue
            .iter()
            .find(|b| &b.key == key)
            .and_then(|b| {
                rows.iter()
                    .find(|r| &r.0 == key)
                    .and_then(|r| u64::try_from(r.1).ok())
                    .and_then(|n| progress(&b.category, n))
            })
            .is_some_and(|p| p.stage == 3);
        bonuses[i] = loot_bonus_percent(points, mastered);
    }
    SlotDetails {
        base_bonus: loot_bonus_percent(points, false),
        bonuses,
        boosted: boosted.into(),
        now,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detailed_slot_bonuses_use_actual_mastery_and_exclude_empty_and_daily_boosted_bosses() {
        let bosses: [String; 2] = [
            "oteryn:creature.oberon".into(),
            "oteryn:creature.drume".into(),
        ];
        let catalogue = vec![
            BosstiaryBoss {
                key: bosses[0].clone(),
                category: "archfoe".into(),
            },
            BosstiaryBoss {
                key: bosses[1].clone(),
                category: "archfoe".into(),
            },
        ];
        let rows = vec![(bosses[0].clone(), 60, 100), (bosses[1].clone(), 5, 10)];
        let d = slot_details(&bosses, &catalogue, &rows, 1500, "", 1000);
        assert_eq!(d.base_bonus, 106);
        assert_eq!(d.bonuses, [131, 106]);
        let d = slot_details(&bosses, &catalogue, &rows, 1500, &bosses[1], 1000);
        assert_eq!(d.bonuses, [131, 0]);
        let d = slot_details(
            &[String::new(), String::new()],
            &catalogue,
            &rows,
            1500,
            "",
            1000,
        );
        assert_eq!(d.bonuses, [0, 0]);
    }
}

use super::TestResult;

pub(super) fn table_override() -> TestResult<Option<Vec<u8>>> {
    let rate = oteryn_gameplay_foundation::scheduled_events::current_rates().loot;
    if rate < 200 || !rate.is_multiple_of(100) {
        return super::w22a_loot_fixtures::table_override();
    }
    let mut section: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../../../tools/content-schema/native-gameplay/loot-tables.json"
    ))?;
    let table = section["tables"]
        .as_array_mut()
        .ok_or("tables")?
        .iter_mut()
        .find(|row| row["identity"]["key"] == "oteryn:loot.creature.rat")
        .ok_or("rat table")?;
    table["entries"] = serde_json::json!([
        {"item":{"family":"Item","key":"oteryn:item.tibia.i3031","revision":"definition-r1"},
         "min_count":1,"max_count":1,"probability_ppm":1000000},
        {"item":{"family":"Item","key":"oteryn:item.tibia.i3035","revision":"definition-r1"},
         "min_count":1,"max_count":1,"probability_ppm":500000}
    ]);
    Ok(Some(serde_json::to_vec(&section)?))
}

pub(super) fn expected(
    table: &mut crate::combat::LootTableDefinition,
    base: &crate::combat::LootTableDefinition,
    rate: u32,
) {
    if rate >= 200 && rate.is_multiple_of(100) {
        *table = base.clone();
        table.algorithm = crate::combat::LootSelectionAlgorithm::DailyBoosted {
            extra_rolls: u8::try_from(rate / 100 - 1).unwrap_or(u8::MAX),
            boosted_entries: table.entries.len(),
        };
    }
}

pub(super) fn verify(actual: &[(String, i64)]) {
    let rate = oteryn_gameplay_foundation::scheduled_events::current_rates().loot;
    if rate >= 200 && rate.is_multiple_of(100) {
        assert_eq!(
            actual
                .iter()
                .filter(|(key, _)| key == "oteryn:item.tibia.i3031")
                .count(),
            usize::try_from(rate / 100).unwrap_or(usize::MAX),
            "Each complete loot roll mints its guaranteed drop"
        );
    }
}

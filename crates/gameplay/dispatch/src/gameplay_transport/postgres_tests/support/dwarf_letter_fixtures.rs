use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, id},
};

const LETTER: &str = "oteryn:item.tibia.i3505";

pub(super) fn force_letter_roll(creature: &str) -> TestResult<Vec<u8>> {
    let mut section: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../../../tools/content-schema/native-gameplay/loot-tables.json"
    ))?;
    let loot = section["creature_loot"]
        .as_array()
        .ok_or("creature bindings")?
        .iter()
        .find(|row| row["creature"]["key"] == creature)
        .ok_or("Dwarf loot binding")?["loot"]
        .clone();
    let table = section["tables"]
        .as_array_mut()
        .ok_or("loot tables")?
        .iter_mut()
        .find(|row| row["identity"] == loot)
        .ok_or("Dwarf loot table")?;
    let entry = table["entries"]
        .as_array_mut()
        .ok_or("Dwarf loot entries")?
        .iter_mut()
        .find(|entry| entry["item"]["key"] == LETTER)
        .ok_or("native Letter roll")?;
    // Preserve the source table and quantity, forcing only the required roll.
    entry["probability_ppm"] = 1_000_000.into();
    Ok(serde_json::to_vec(&section)?)
}

pub(super) async fn loot_letter(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    creature: &str,
) -> TestResult {
    let corpse = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .next()
        .ok_or_else(|| format!("{creature}: lethal auto-attack materialized no corpse"))?;
    assert_eq!(corpse.item.production_key, "oteryn:item.tibia.i6007");
    let looted = super::l6_tomb_native_fixtures::collect_loot(
        owner,
        admitted,
        h,
        corpse.item.item_instance_id,
        LETTER,
        1400,
    )
    .await?;
    let (carried, left): (i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT COALESCE(sum(i.quantity),0)::bigint
             FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id)
             WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$2),
            (SELECT count(*)
             FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id)
             WHERE i.definition_production_key=$2)",
    )
    .bind(id(41).as_slice())
    .bind(LETTER)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!((looted, carried, left), (1, 1, 0), "{creature}: loot exactly one Letter");
    eprintln!("DWARF_LETTER_POSTGRES creature={creature} carried={carried}");
    Ok(())
}

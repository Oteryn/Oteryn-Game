//! Ordinary production loot sources; geometry and donor drop probabilities stay unchanged.
use super::*;
pub(super) fn creature() -> Option<&'static str> {
    match super::cosmetic_source_loot::natural_mode() {
        5 => Some("bear"),
        6 => Some("chicken"),
        7 => Some("hellspawn"),
        8 => Some("minotaur"),
        9 => Some("rotworm"),
        10 => Some("wasp"),
        11 => Some("wolf"),
        12 => Some("demon_skeleton"),
        13 => Some("stampor"),
        _ => None,
    }
}
pub(super) fn target_item() -> Option<u32> {
    match creature()? {
        "bear" => Some(5896),
        "chicken" => Some(5890),
        "hellspawn" => Some(9034),
        "minotaur" => Some(5878),
        "rotworm" => Some(3374),
        "wasp" => Some(5902),
        "wolf" => Some(5897),
        _ => None,
    }
}
pub(super) fn positions() -> TestResult<
    Option<(
        crate::foundation::MovementLocalPosition,
        crate::foundation::MovementLocalPosition,
    )>,
> {
    if super::stampor_natural_death::active() {
        return super::stampor_natural_death::positions();
    }
    if super::spike_natural_death::active() {
        return super::spike_natural_death::positions();
    }
    let Some(name) = creature() else {
        return Ok(None);
    };
    let mut path = std::env::var_os("OTERYN_SRV7_ACQUISITION_BUNDLE")
        .ok_or("actual producer compiler Bundle")?;
    path.push(".cells.json");
    let data: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let row = &data[name];
    assert_eq!(row["creature"], format!("oteryn:creature.{name}"));
    let decode = |field: &str| -> TestResult<crate::foundation::MovementLocalPosition> {
        let a = row[field].as_array().ok_or("actual source cell")?;
        Ok(crate::foundation::MovementLocalPosition {
            x: i32::try_from(a[0].as_i64().ok_or("x")?)?,
            y: i32::try_from(a[1].as_i64().ok_or("y")?)?,
            floor: i16::try_from(a[2].as_i64().ok_or("floor")?)?,
        })
    };
    let a = decode("player")?;
    let b = decode("target")?;
    assert_eq!(a.floor, b.floor);
    assert_eq!(a.x.abs_diff(b.x) + a.y.abs_diff(b.y), 1);
    assert!(
        row["source"]["path"]
            .as_str()
            .is_some_and(|s| s.starts_with("content/world/spawns/spawns-"))
    );
    Ok(Some((a, b)))
}
pub(super) async fn qualify(
    h: &super::fixture::Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    _admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    if super::stampor_natural_death::active() {
        return super::stampor_natural_death::qualify(h, owner).await;
    }
    if super::spike_natural_death::active() {
        return super::spike_natural_death::qualify(h, owner).await;
    }
    let id = target_item().ok_or("natural acquisition source item")?;
    let key = format!("oteryn:item.tibia.i{id}");
    let source = creature().ok_or("natural producer")?;
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native source")?;
    let reward = native
        .reward_table()
        .row(&format!("oteryn:creature.{source}"))
        .map_err(super::fixture::debug)?;
    assert!(
        reward
            .loot_table
            .entries
            .iter()
            .any(|e| e.item.production_key == key),
        "unchanged donor producer loot contains the actual operand"
    );
    let quantity:i64=sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id,world_id) JOIN game_item_mint_receipts r ON r.item_instance_id=i.item_instance_id AND r.death_world_id=i.world_id WHERE i.definition_production_key=$1")
  .bind(&key).fetch_one(&h.pool).await?;
    assert!(
        quantity > 0,
        "actual native producer death must materialize its source operand"
    );
    let expected_body = match source {
        "bear" => 5975,
        "chicken" => 6042,
        "hellspawn" => 9009,
        "minotaur" => 5969,
        "rotworm" => 5967,
        "wasp" => 5989,
        "wolf" => 5968,
        _ => return Err("unexpected producer".into()),
    };
    assert_eq!(
        reward.corpse_item.production_key,
        format!("oteryn:item.tibia.i{expected_body}")
    );
    let policy = native
        .item_policy(
            &reward.corpse_item.production_key,
            &reward.corpse_item.revision_ref,
        )
        .ok_or("actual corpse native policy")?;
    let crate::content::ReferenceItemField::Known(container) = &policy.record().semantics.container
    else {
        return Err("source body container".into());
    };
    let crate::content::ReferenceItemField::Known(capacity) = container.capacity else {
        return Err("source physical capacity".into());
    };
    let expected_capacity = match source {
        "bear" => 20,
        "chicken" => 5,
        "hellspawn" => 44,
        "minotaur" | "rotworm" => 10,
        _ => 5,
    };
    assert_eq!(
        capacity, expected_capacity,
        "physicalsourcecapacity is never clamped to the native entry budget"
    );
    let corpse:(Vec<u8>,Vec<u8>,String,i64,String,Vec<u8>)=sqlx::query_as("SELECT uuid_send(r.death_world_id),uuid_send(r.death_channel_id),r.death_scope_ownership_generation::text,r.death_actor_local_id,r.death_actor_local_generation::text,uuid_send(r.item_instance_id) FROM game_item_mint_receipts r JOIN game_item_instances i USING(item_instance_id) WHERE r.loot_purpose_key='CORPSE_MATERIALIZATION' AND i.definition_production_key=$1")
        .bind(&reward.corpse_item.production_key).fetch_one(&h.pool).await?;
    let seed = oteryn_gameplay_items::loot_plan::LootPlanDeathKey::new(
        corpse
            .0
            .as_slice()
            .try_into()
            .map_err(super::fixture::debug)?,
        corpse
            .1
            .as_slice()
            .try_into()
            .map_err(super::fixture::debug)?,
        corpse.2.parse()?,
        u32::try_from(corpse.3)?,
        corpse.4.parse()?,
    );
    let plan = oteryn_gameplay_items::loot_plan::plan_creature_loot(
        seed,
        &reward.loot_table_ref,
        &reward.loot_table,
    )?;
    let expected: Vec<(String, i64, i64)> = plan
        .entries
        .iter()
        .map(|e| {
            (
                e.item.production_key.clone(),
                i64::from(e.quantity),
                i64::from(e.draw_ordinal),
            )
        })
        .collect();
    let actual:Vec<(String,i64,i64)>=sqlx::query_as("SELECT i.definition_production_key,i.quantity,r.draw_ordinal FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id,world_id) JOIN game_item_mint_receipts r ON r.item_instance_id=i.item_instance_id WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY r.draw_ordinal")
        .bind(corpse.5.as_slice()).fetch_all(&h.pool).await?;
    let mut expected = expected;
    expected.sort_by_key(|e| e.2);
    assert_eq!(
        actual, expected,
        "every selected unchanged source plan operand has its actual same-death MINT quantity/ordinal/corpse parent"
    );
    assert!(
        actual.len() <= 30,
        "physical44doesnotwidenaccepted30entrybudget"
    );
    assert!(crate::combat::check_corpse_container_capacity(30).is_ok());
    assert!(crate::combat::check_corpse_container_capacity(31).is_err());
    eprintln!(
        "SRV7_NATURAL_ACQUISITION_PRODUCER_POSTGRES_EXECUTED creature={source} source={id} quantity={quantity} body={expected_body} physical_capacity={capacity} actual_mints={} entry_budget=30",
        actual.len()
    );
    Ok(())
}

pub(super) fn source_xp(
    native: &crate::content::native_gameplay::NativeGameplayState,
    key: &str,
) -> TestResult<i64> {
    let record = native
        .creature_profiles()
        .records
        .iter()
        .find(|r| r.profile.target.key == key)
        .ok_or("actual native source Creature profile")?;
    let crate::content::ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data
    else {
        return Err("source Creature data".into());
    };
    Ok(i64::try_from(profile.experience.unwrap_or(0))?)
}
